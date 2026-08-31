import type { RowFilter } from "../features/text-compare/filterRows";

export type { RowFilter };

export type Cluster = {
  start: number;
  end: number;
};

export type FolderNavRow = {
  status: "equal" | "different" | "leftOnly" | "rightOnly" | "typeConflict";
  kind: "dir" | "archive" | "file";
  expanded: boolean;
};

export function clusterMarks(marks: number[]): Cluster[] {
  const unique = [...new Set(marks)].sort((a, b) => a - b);
  if (unique.length === 0) return [];
  const first = unique[0];
  const out: Cluster[] = [{ start: first, end: first }];
  for (let i = 1; i < unique.length; i += 1) {
    const row = unique[i];
    const last = out[out.length - 1];
    if (row - last.end <= 3) {
      last.end = row;
    } else {
      out.push({ start: row, end: row });
    }
  }
  return out;
}

export function pinTargets(
  clusters: Cluster[],
  topRow: number,
): { prev: number | null; next: number | null } {
  let prev: number | null = null;
  let next: number | null = null;
  for (const cluster of clusters) {
    if (cluster.start < topRow) prev = cluster.start;
    if (cluster.start > topRow && next == null) next = cluster.start;
  }
  return { prev, next };
}

export function mapStartToFilteredIndex(marks: number[], start: number): number {
  let count = 0;
  for (const mark of marks) {
    if (mark < start) count += 1;
  }
  return count;
}

export function pinNav(
  marks: number[],
  topRow: number,
  filter: RowFilter,
): { hasClusters: boolean; prevRow: number | null; nextRow: number | null } {
  if (filter === "same" || marks.length === 0) {
    return { hasClusters: false, prevRow: null, nextRow: null };
  }
  const clusters = clusterMarks(marks);
  const mapped =
    filter === "diff"
      ? clusters.map((cluster) => ({
          start: mapStartToFilteredIndex(marks, cluster.start),
          end: mapStartToFilteredIndex(marks, cluster.end),
        }))
      : clusters;
  const { prev, next } = pinTargets(mapped, topRow);
  return { hasClusters: true, prevRow: prev, nextRow: next };
}

export function folderStopRows(rows: FolderNavRow[]): number[] {
  const stops: number[] = [];
  rows.forEach((row, index) => {
    if (row.status === "equal") return;
    if ((row.kind === "dir" || row.kind === "archive") && row.expanded) return;
    stops.push(index);
  });
  return stops;
}

export function remapClusterStarts(
  clusters: Cluster[],
  mapStart: (start: number) => number | null,
): Cluster[] {
  const out: Cluster[] = [];
  for (const cluster of clusters) {
    const start = mapStart(cluster.start);
    if (start == null) continue;
    out.push({ start, end: start });
  }
  return out;
}
