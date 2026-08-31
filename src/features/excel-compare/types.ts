export type ExcelSheetStatus = "equal" | "different" | "leftOnly" | "rightOnly";

export type ExcelCell = {
  left: string;
  right: string;
  changed: boolean;
};

export type ExcelRow = {
  index: number;
  dirty: boolean;
  cells: ExcelCell[];
};

export type ExcelSheetInfo = {
  name: string;
  status: ExcelSheetStatus;
  width: number;
  height: number;
  dirtyRows: number[];
  changedCells: number;
};

export type ExcelSummary = {
  sheets: ExcelSheetInfo[];
  changedCells: number;
  dirtySheets: number;
};

export type ExcelWindow = {
  rows: ExcelRow[];
  total: number;
  offset: number;
  width: number;
};

export const emptyExcelSummary: ExcelSummary = {
  sheets: [],
  changedCells: 0,
  dirtySheets: 0,
};

export const EXCEL_ROW_PX = 20;
export const EXCEL_COL_PX = 88;
export const EXCEL_GUTTER_PX = 44;
export const EXCEL_OVERSCAN = 12;

export function columnLabel(index: number): string {
  let n = index + 1;
  let label = "";
  while (n > 0) {
    n -= 1;
    label = String.fromCharCode(65 + (n % 26)) + label;
    n = Math.floor(n / 26);
  }
  return label;
}
