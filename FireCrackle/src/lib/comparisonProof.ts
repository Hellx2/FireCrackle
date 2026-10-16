import { inTauri } from "./api";

export interface ProofMask { svg: SVGSVGElement; description: string; opacity: number; details?: string }
export const proofOpacity = (value: number) => value > 0 ? Math.min(1, value) : 1;

export function proofLayout(width: number, height: number) {
  if (![width, height].every(Number.isFinite) || width <= 0 || height <= 0) throw Error("The picture is not ready yet.");
  const scale = Math.min(4096 / width, 4096 / height, Math.max(1, 360 / width));
  const panelWidth = Math.round(width * scale), panelHeight = Math.round(height * scale);
  const font = Math.max(14, Math.min(48, Math.round(panelWidth / 36)));
  return { panelWidth, panelHeight, font, header: font * 5, footer: font * 2, gap: Math.max(8, Math.round(panelWidth / 120)) };
}

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(Error("Could not render the comparison mask."));
    image.src = url;
  });
}

function dataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(Error("Could not read the compressed mask."));
    reader.readAsDataURL(blob);
  });
}

/** Snapshot the rendered mask, including compression, but never its editing controls. */
export function cleanProofMask(mask: ProofMask, width: number, height: number): SVGSVGElement {
  if (mask.svg.querySelector('[role="status"], [role="alert"]')) throw Error("Wait for the mask preview to finish loading before making a comparison proof.");
  const clone = mask.svg.cloneNode(true) as SVGSVGElement;
  clone.querySelectorAll(".fire-quad-guide, .fire-quad-handle, .quad-transform, title").forEach(node => node.remove());
  if (!clone.querySelector("polygon, image")) throw Error("The mask texture is still loading. Try again in a moment.");
  clone.removeAttribute("class");
  clone.removeAttribute("style");
  clone.querySelectorAll("[opacity]").forEach(node => node.removeAttribute("opacity"));
  clone.setAttribute("opacity", String(proofOpacity(mask.opacity)));
  clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  clone.setAttribute("width", String(width));
  clone.setAttribute("height", String(height));
  return clone;
}

async function rasterMask(svg: SVGSVGElement): Promise<{ image: HTMLImageElement; dispose: () => void }> {
  // Blob-backed compressed PNGs must be embedded before the SVG becomes an image.
  for (const image of svg.querySelectorAll("image")) {
    const href = image.getAttribute("href");
    if (href && !href.startsWith("data:")) {
      const response = await fetch(href);
      if (!response.ok) throw Error("Could not read the mask preview.");
      image.setAttribute("href", await dataUrl(await response.blob()));
    }
  }
  const url = URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(svg)], { type: "image/svg+xml" }));
  try { return { image: await loadImage(url), dispose: () => URL.revokeObjectURL(url) }; }
  catch (error) { URL.revokeObjectURL(url); throw error; }
}

export async function createComparisonProof(picture: HTMLImageElement, masks: ProofMask[], source: string): Promise<Blob> {
  if (!picture.complete || !picture.naturalWidth || !picture.naturalHeight) throw Error("Wait for the current picture to load.");
  if (!masks.length) throw Error("Turn on a fire mask before making a comparison proof.");
  const { naturalWidth: width, naturalHeight: height } = picture;
  const { panelWidth: w, panelHeight: h, font, header, footer, gap } = proofLayout(width, height);
  // Clone geometry and pixels synchronously: animation/frame navigation cannot change this proof later.
  const layers = masks.map(mask => ({ ...mask, svg: cleanProofMask(mask, width, height) }));
  const canvas = document.createElement("canvas");
  canvas.width = w * 2 + gap; canvas.height = header + h + footer;
  const context = canvas.getContext("2d");
  if (!context) throw Error("Image rendering is unavailable.");
  context.fillStyle = "#111722"; context.fillRect(0, 0, canvas.width, canvas.height);
  context.imageSmoothingEnabled = false;
  context.drawImage(picture, 0, header, w, h);
  context.drawImage(picture, w + gap, header, w, h);
  for (const layer of layers) {
    const raster = await rasterMask(layer.svg);
    try { context.drawImage(raster.image, w + gap, header, w, h); }
    finally { raster.dispose(); }
  }
  context.fillStyle = "#ffffff"; context.font = `600 ${font}px sans-serif`;
  context.fillText("Original", font, font * 1.5, w - font * 2);
  context.fillText("After · mask comparison", w + gap + font, font * 1.5, w - font * 2);
  context.font = `${font}px sans-serif`;
  context.fillText(source.slice(0, 140), font, font * 3, w - font * 2);
  // The viewer offers one active mask mode; record the displayed sprite, never the candidate/guess.
  layers.forEach((mask, index) => {
    context.fillText(mask.description, w + gap + font, font * (3 + index), w - font * 2);
  });
  context.fillStyle = "#bdc8da";
  context.fillText(layers.map(mask => `Opacity ${Math.round(proofOpacity(mask.opacity) * 100)}%${mask.details ? ` · ${mask.details}` : ""}`).join(" / "), w + gap + font, font * 4.3, w - font * 2);
  context.fillText("Comparison proof · manually aligned reference overlay", font, header + h + font * 1.35, canvas.width - font * 2);
  return new Promise((resolve, reject) => canvas.toBlob(blob => blob ? resolve(blob) : reject(Error("Could not create the comparison PNG.")), "image/png"));
}

/** Copy only: never open a save dialog or silently download a file. */
export async function copyComparisonProof(proof: Blob | Promise<Blob>): Promise<void> {
  if (inTauri()) {
    const blob = await proof;
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("copy_comparison_proof", { png: Array.from(new Uint8Array(await blob.arrayBuffer())) });
    return;
  }
  if (typeof ClipboardItem === "undefined" || !navigator.clipboard?.write) {
    // Observe render failures even when the browser cannot copy images.
    void Promise.resolve(proof).catch(() => {});
    throw Error("Image clipboard is unavailable. Use the desktop app to copy the comparison proof.");
  }
  // Start the clipboard write in the click handler so rendering retains user activation.
  await navigator.clipboard.write([new ClipboardItem({ "image/png": Promise.resolve(proof) })]);
}
