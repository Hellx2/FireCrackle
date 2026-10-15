import { isFire, type Label } from "./grid";

type Vec3 = [number, number, number];
type Point = [number, number];
type Quad = [Vec3, Vec3, Vec3, Vec3];

/** Same view-space transform and reversed UVs as fire_core::overlay. */
export function hudQuads(): Quad[] {
  const corners: Vec3[] = [[-0.5,-0.5,-0.5], [0.5,-0.5,-0.5], [0.5,0.5,-0.5], [-0.5,0.5,-0.5]];
  return [-1, 1].map((s) => {
    const angle = s * Math.PI / 18;
    return corners.map(([x,y,z]) => [x*Math.cos(angle)+z*Math.sin(angle)-s*0.24, y-0.3, -x*Math.sin(angle)+z*Math.cos(angle)] as Vec3) as Quad;
  });
}

export function projectHud([x,y,z]: Vec3, width: number, height: number, fov: number): Point {
  if (![x,y,z,width,height,fov].every(Number.isFinite) || width <= 0 || height <= 0 || fov <= 0 || fov >= 180 || z >= 0) {
    throw new Error("Invalid HUD projection");
  }
  const f = 1 / Math.tan(fov * Math.PI / 360);
  return [(1 + f * height / width * x / -z) * width / 2, (1 - f * y / -z) * height / 2];
}

export interface HudTexel {
  points: Point[];
  colour: string;
  alpha: number;
  quad: number;
}

/** Project each texel independently for true perspective, in game draw order. */
export function hudTexels(rgba: Uint8ClampedArray, row: number, width: number, height: number, fov: number): HudTexel[] {
  if (rgba.length !== 16 * 512 * 4 || !Number.isInteger(row) || row < 0 || row >= 32 || (fov !== 60 && fov !== 70)) throw new Error("Invalid HUD sprite row or FOV");
  projectHud([0, 0, -1], width, height, fov);
  const output: HudTexel[] = [];
  hudQuads().forEach((q, quad) => {
    const point = (u: number, v: number): Point => projectHud(q[0].map((c, k) => c + (1-u)*(q[1][k]-c) + (1-v)*(q[3][k]-c)) as Vec3, width, height, fov);
    for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) {
      const i = ((row*16+y)*16+x)*4;
      if (!rgba[i+3]) continue;
      output.push({
        points: [point(x/16,y/16), point((x+1)/16,y/16), point((x+1)/16,(y+1)/16), point(x/16,(y+1)/16)],
        colour: `rgb(${rgba[i]}, ${rgba[i+1]}, ${rgba[i+2]})`, alpha: 0.9*rgba[i+3]/255, quad,
      });
    }
  });
  return output;
}

export function hudRowFromLabel(label: Label): number | null {
  if (!isFire(label) || ![0,1].includes(label.fire.sheet) || !Number.isInteger(label.fire.row) || label.fire.row < 0 || label.fire.row >= 32) return null;
  return (label.fire.row + (label.fire.sheet === 0 ? 16 : 0)) % 32;
}

export const stepHudRow = (row: number, delta: number) => ((row + delta) % 32 + 32) % 32;
