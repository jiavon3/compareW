import type { ReactNode } from "react";

type Props = {
  icon: ReactNode;
  label: string;
  onClick: () => void;
  kind?: "ghost" | "compare" | "scope";
  pressed?: boolean;
};

export default function ToolButton({
  icon,
  label,
  onClick,
  kind = "ghost",
  pressed = false,
}: Props) {
  const className = ["tool", `tool-${kind}`, pressed ? "is-on" : ""]
    .filter(Boolean)
    .join(" ");

  return (
    <button
      type="button"
      className={className}
      onClick={onClick}
      aria-pressed={kind === "scope" ? pressed : undefined}
    >
      <span className="tool-icon">{icon}</span>
      <span className="tool-label">{label}</span>
    </button>
  );
}
