import type { ReactNode } from "react";

type IconProps = {
  className?: string;
};

function Mark({ children, className }: IconProps & { children: ReactNode }) {
  return (
    <svg
      className={className}
      viewBox="0 0 16 16"
      width="15"
      height="15"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

export function IconOpenLeft({ className }: IconProps) {
  return (
    <Mark className={className}>
      <rect x="5" y="2.5" width="8" height="11" rx="1.2" />
      <path d="M2 8h6.5M5.5 5.8 2.8 8l2.7 2.2" />
    </Mark>
  );
}

export function IconOpenRight({ className }: IconProps) {
  return (
    <Mark className={className}>
      <rect x="3" y="2.5" width="8" height="11" rx="1.2" />
      <path d="M14 8H7.5M10.5 5.8 13.2 8l-2.7 2.2" />
    </Mark>
  );
}

export function IconCompare({ className }: IconProps) {
  return (
    <Mark className={className}>
      <rect x="1.8" y="3" width="5.2" height="10" rx="1" />
      <rect x="9" y="3" width="5.2" height="10" rx="1" />
      <circle cx="8" cy="8" r="1.15" fill="currentColor" stroke="none" />
    </Mark>
  );
}

export function IconAll({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M8 3.2v9.6M3.2 8h9.6M4.6 4.6l6.8 6.8M11.4 4.6l-6.8 6.8" />
    </Mark>
  );
}

export function IconSame({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M3.2 6h9.6M3.2 10h9.6" />
    </Mark>
  );
}

export function IconDiff({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M3.2 6h9.6M3.2 10h9.6M11.2 4.2 4.8 11.8" />
    </Mark>
  );
}

export function IconFolder({ className }: IconProps) {
  return (
    <svg
      className={className}
      viewBox="0 0 16 16"
      width="16"
      height="16"
      aria-hidden="true"
    >
      <path fill="currentColor" d="M1.4 5.1h4.4l1.05 1.25H14.6v7.15H1.4z" />
      <path fill="currentColor" d="M1.4 3.2h4.15l.95 1.2H1.4z" opacity="0.7" />
    </svg>
  );
}

export function IconFile({ className }: IconProps) {
  return (
    <svg
      className={className}
      viewBox="0 0 16 16"
      width="15"
      height="15"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.3"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M4.2 2.6h5.1L12 5.4v8H4.2z" />
      <path d="M9.2 2.6V5.4H12" />
    </svg>
  );
}

export function IconArchive({ className }: IconProps) {
  return (
    <svg
      className={className}
      viewBox="0 0 16 16"
      width="16"
      height="16"
      aria-hidden="true"
    >
      <path fill="currentColor" d="M1.4 5.1h4.4l1.05 1.25H14.6v7.15H1.4z" />
      <path fill="currentColor" d="M1.4 3.2h4.15l.95 1.2H1.4z" opacity="0.7" />
      <path
        fill="none"
        stroke="currentColor"
        strokeWidth="1.2"
        d="M8 7.4v4.2M6.7 8.6h2.6"
        opacity="0.95"
      />
    </svg>
  );
}

export function IconUp({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M8 12.5V3.8M4.6 7.2 8 3.8l3.4 3.4" />
    </Mark>
  );
}

export function IconText({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M4 3.5h8M8 3.5v9M5.5 12.5h5" />
    </Mark>
  );
}

export function IconClear({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M3.5 10.1 9 4.1a1.35 1.35 0 0 1 1.95 0l1 1.05a1.35 1.35 0 0 1 0 1.95L6.4 13.1H3.5z" />
      <path d="M5.5 8.4 9.8 12.2" />
      <path d="M3.2 13.2h9.6" />
    </Mark>
  );
}

export function IconRefresh({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M12.4 8A4.4 4.4 0 1 1 10.6 4.3" />
      <path d="M10.2 2.6 10.6 4.4 12.4 4.1" />
    </Mark>
  );
}

export function IconExpandAll({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M3.2 6.2 8 10.6l4.8-4.4" />
      <path d="M3.2 3.6 8 8l4.8-4.4" />
    </Mark>
  );
}

export function IconCollapseAll({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M3.2 9.8 8 5.4l4.8 4.4" />
      <path d="M3.2 12.4 8 8l4.8 4.4" />
    </Mark>
  );
}
