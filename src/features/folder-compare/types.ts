export type FolderKind = "dir" | "archive" | "file";

export type FolderStatus =
  | "equal"
  | "different"
  | "leftOnly"
  | "rightOnly"
  | "typeConflict";

export type FolderRow = {
  name: string;
  kind: FolderKind;
  status: FolderStatus;
  leftSize: number | null;
  rightSize: number | null;
  leftMtime: number | null;
  rightMtime: number | null;
};

export type FolderSummary = {
  pathBar: string;
  canGoUp: boolean;
  rowCount: number;
  equal: number;
  different: number;
};

export type FolderWindow = {
  rows: FolderRow[];
  total: number;
  offset: number;
};

export const emptyFolderSummary: FolderSummary = {
  pathBar: "",
  canGoUp: false,
  rowCount: 0,
  equal: 0,
  different: 0,
};

export const FOLDER_ROW_PX = 24;
