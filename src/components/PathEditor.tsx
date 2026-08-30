import type { KeyboardEvent, ReactNode } from "react";
import { IconFolder } from "../features/text-compare/icons";

export function normalizePath(raw: string): string {
  let value = raw.trim();
  if (
    (value.startsWith('"') && value.endsWith('"')) ||
    (value.startsWith("'") && value.endsWith("'"))
  ) {
    value = value.slice(1, -1).trim();
  }
  return value;
}

export default function PathEditor({
  value,
  placeholder,
  onChange,
  onSubmit,
  onBrowse,
  browseLabel = "选择文件夹",
  icon,
}: {
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  onBrowse: () => void;
  browseLabel?: string;
  icon?: ReactNode;
}) {
  function handleKey(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key !== "Enter") return;
    event.preventDefault();
    onSubmit();
  }

  return (
    <div className="path-side">
      <input
        className="path-input"
        value={value}
        placeholder={placeholder}
        spellCheck={false}
        autoCorrect="off"
        autoCapitalize="off"
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={handleKey}
      />
      <button
        type="button"
        className="path-browse"
        aria-label={browseLabel}
        title={browseLabel}
        onClick={onBrowse}
      >
        {icon ?? <IconFolder />}
      </button>
    </div>
  );
}
