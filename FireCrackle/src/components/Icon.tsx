import type { ButtonHTMLAttributes, ReactNode } from "react";

const shapes = {
  folder: <><path d="M3 7V5h6l2 2h10v12H3Z" /><path d="M3 11h18" /></>,
  save: <><path d="M4 3h13l3 3v15H4Z" /><path d="M8 3v6h8V3M8 21v-8h8v8" /></>,
  import: <><path d="M12 3v12m-4-4 4 4 4-4M4 15v6h16v-6" /></>,
  export: <><path d="M12 16V4m-4 4 4-4 4 4M4 15v6h16v-6" /></>,
  undo: <><path d="m8 4-5 5 5 5M3 9h11a6 6 0 0 1 0 12" /></>,
  play: <path d="m8 4 12 8-12 8Z" />,
  pause: <><path d="M7 4h3v16H7ZM15 4h3v16h-3Z" /></>,
  keyboard: <><rect x="2" y="5" width="20" height="14" rx="2" /><path d="M6 9h.01M10 9h.01M14 9h.01M18 9h.01M6 12h.01M10 12h.01M14 12h.01M18 12h.01M7 16h10" /></>,
  settings: <><path d="M4 7h16M4 17h16" /><circle cx="9" cy="7" r="3" /><circle cx="15" cy="17" r="3" /></>,
  layers: <><path d="m12 3 10 5-10 5L2 8ZM3 12l9 5 9-5M3 16l9 5 9-5" /></>,
  mirror: <><path d="M12 3v18M3 7l5-3v16l-5-3ZM21 7l-5-3v16l5-3Z" /></>,
  noFire: <><path d="M14 3c0 4-5 6-5 9 0 1 1 2 2 2M7 7c-2 3-3 5-3 8a8 8 0 0 0 13 6M17 7c2 3 3 5 3 8M3 3l18 18" /></>,
  skip: <><path d="m5 5 10 7-10 7ZM19 5v14" /></>,
  erase: <><path d="m4 14 9-11 9 7-9 11H8l-4-4ZM9 8l9 7M13 21h9" /></>,
  label: <><path d="M3 3h8l10 10-8 8L3 11Z" /><circle cx="7.5" cy="7.5" r="1" /></>,
  chart: <><path d="M3 3v18h18M6 15l4-5 4 3 6-8" /></>,
  left: <path d="m15 5-7 7 7 7" />,
  right: <path d="m9 5 7 7-7 7" />,
  reset: <><path d="M3 4v6h6M3 10a9 9 0 1 1 1 8" /></>,
  parallel: <path d="M7 5h15l-5 14H2Z" />,
  ruler: <><path d="m3 16 13-13 5 5L8 21ZM7 12l3 3M10 9l2 2M13 6l3 3" /></>,
  copy: <><rect x="8" y="8" width="13" height="13" rx="2" /><path d="M16 8V3H3v13h5" /></>,
  zoomIn: <><circle cx="10" cy="10" r="7" /><path d="m15 15 6 6M7 10h6M10 7v6" /></>,
  zoomOut: <><circle cx="10" cy="10" r="7" /><path d="m15 15 6 6M7 10h6" /></>,
  fit: <><path d="M8 3H3v5M16 3h5v5M3 16v5h5M21 16v5h-5" /><rect x="7" y="7" width="10" height="10" rx="1" /></>,
  check: <path d="m5 12 4 4L19 6" />,
  info: <><circle cx="12" cy="12" r="9" /><path d="M12 11v6M12 7.5h.01" /></>,
  alert: <><path d="m12 3 10 18H2ZM12 9v5M12 17h.01" /></>,
} satisfies Record<string, ReactNode>;

export type IconName = keyof typeof shapes;
export function Icon({ name }: { name: IconName }) {
  return <svg className="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">{shapes[name]}</svg>;
}

export function IconButton({ icon, label, children, className = "", title, ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { icon: IconName; label: string }) {
  return <button type="button" {...props} className={`icon-button ${children ? "with-text" : "icon-only"} ${className}`} aria-label={label} title={title ?? label}><Icon name={icon} />{children}</button>;
}
