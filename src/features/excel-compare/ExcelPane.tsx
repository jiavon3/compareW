import type { RefObject, UIEvent } from "react";
import { EXCEL_COL_PX, EXCEL_GUTTER_PX, EXCEL_ROW_PX, columnLabel } from "./types";
import type { ExcelRow } from "./types";

type Side = "left" | "right";

type Props = {
  side: Side;
  rows: ExcelRow[];
  totalRows: number;
  width: number;
  windowStart: number;
  scrollRef: RefObject<HTMLDivElement | null>;
  onScroll: (event: UIEvent<HTMLDivElement>) => void;
};

export default function ExcelPane({
  side,
  rows,
  totalRows,
  width,
  windowStart,
  scrollRef,
  onScroll,
}: Props) {
  const columns = Math.max(width, 1);
  const innerWidth = EXCEL_GUTTER_PX + columns * EXCEL_COL_PX;
  const grid = {
    gridTemplateColumns: `${EXCEL_GUTTER_PX}px repeat(${columns}, ${EXCEL_COL_PX}px)`,
  };

  return (
    <div className="pane-frame">
      <div className="excel-scroll" ref={scrollRef} onScroll={onScroll}>
        <div className="excel-inner" style={{ width: innerWidth }}>
          <div className="excel-colhead" style={grid}>
            <span className="excel-gutter" />
            {Array.from({ length: columns }, (_, index) => (
              <span className="excel-col" key={index}>
                {columnLabel(index)}
              </span>
            ))}
          </div>
          {totalRows === 0 ? null : (
            <div
              className="virt-space"
              style={{ height: Math.max(totalRows, 1) * EXCEL_ROW_PX }}
            >
              <div
                className="virt-window"
                style={{ transform: `translateY(${windowStart * EXCEL_ROW_PX}px)` }}
              >
                {rows.map((row, index) => {
                  const line = side === "left" ? row.leftIndex : row.rightIndex;
                  return (
                    <div
                      key={`${side}-${windowStart + index}`}
                      className={row.dirty ? "excel-row is-diff" : "excel-row"}
                      style={grid}
                    >
                      <span className="excel-gutter">{line ?? ""}</span>
                      {Array.from({ length: columns }, (_, col) => {
                        const cell = row.cells[col];
                        const value = side === "left" ? (cell?.left ?? "") : (cell?.right ?? "");
                        const changed = Boolean(cell?.changed);
                        return (
                          <span
                            key={col}
                            className={changed ? "excel-cell is-diff" : "excel-cell"}
                            title={value}
                          >
                            {value}
                          </span>
                        );
                      })}
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
