export type DiffKind = "equal" | "delete" | "insert" | "replace";

export type TextSpan = {
  text: string;
  changed: boolean;
};

export type DiffRow = {
  leftLine: number | null;
  rightLine: number | null;
  leftText: string;
  rightText: string;
  kind: DiffKind;
  leftSpans?: TextSpan[];
  rightSpans?: TextSpan[];
};

export type DiffStats = {
  equal: number;
  insert: number;
  delete: number;
  replace: number;
};

export type CompareSummary = {
  stats: DiffStats;
  rowCount: number;
  dirtyLeft: number[];
  dirtyRight: number[];
  diffMarks: number[];
};

export type DiffWindow = {
  rows: DiffRow[];
  total: number;
  offset: number;
};

export const LINE_BOX_PX = 20;
export const VIEW_OVERSCAN = 20;
export const AUTO_COMPARE_MAX_CHARS = 200_000;

export const emptyStats: DiffStats = {
  equal: 0,
  insert: 0,
  delete: 0,
  replace: 0,
};

export const emptySummary: CompareSummary = {
  stats: emptyStats,
  rowCount: 0,
  dirtyLeft: [],
  dirtyRight: [],
  diffMarks: [],
};
