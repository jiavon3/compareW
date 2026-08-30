import type { DiffRow } from "./types";

export type RowFilter = "all" | "same" | "diff";

export function dirtyLineNumbers(
  rows: DiffRow[],
  side: "left" | "right",
): Set<number> {
  const lines = new Set<number>();
  for (const row of rows) {
    if (row.kind === "equal") continue;
    const line = side === "left" ? row.leftLine : row.rightLine;
    if (line != null) {
      lines.add(line);
    }
  }
  return lines;
}

export function filterRows(rows: DiffRow[], filter: RowFilter): DiffRow[] {
  if (filter === "same") {
    return rows.filter((row) => row.kind === "equal");
  }
  if (filter === "diff") {
    return rows.filter((row) => row.kind !== "equal");
  }
  return rows;
}
