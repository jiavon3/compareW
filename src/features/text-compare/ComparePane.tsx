import { useMemo, useState } from "react";
import type { RefObject, UIEvent } from "react";
import type { DiffKind, DiffRow } from "./types";
import { LINE_BOX_PX, VIEW_OVERSCAN } from "./types";

type Side = "left" | "right";

type Props = {
  side: Side;
  mode: "edit" | "result";
  text: string;
  rows: DiffRow[];
  totalRows: number;
  windowStart: number;
  onChange: (value: string) => void;
  onStartEdit: () => void;
  dirtyLines: Set<number>;
  scrollRef: RefObject<HTMLDivElement | null>;
  onScroll: () => void;
  onViewportChange: (start: number, count: number) => void;
};

function cellClass(kind: DiffKind, empty: boolean): string {
  if (kind === "equal") return "cell";
  return empty ? "cell cell-diff cell-gap" : "cell cell-diff";
}

function lineCount(text: string): number {
  return Math.max(1, text.split("\n").length);
}

function visibleRange(scrollTop: number, height: number, total: number) {
  const start = Math.max(0, Math.floor(scrollTop / LINE_BOX_PX) - VIEW_OVERSCAN);
  const count = Math.ceil(height / LINE_BOX_PX) + VIEW_OVERSCAN * 2;
  return {
    start,
    count: Math.min(count, Math.max(0, total - start)),
  };
}

export default function ComparePane({
  side,
  mode,
  text,
  rows,
  totalRows,
  windowStart,
  onChange,
  onStartEdit,
  dirtyLines,
  scrollRef,
  onScroll,
  onViewportChange,
}: Props) {
  const [editScroll, setEditScroll] = useState(0);
  const [editHeight, setEditHeight] = useState(400);
  const count = lineCount(text);
  const editVis = useMemo(
    () => visibleRange(editScroll, editHeight, count),
    [editScroll, editHeight, count],
  );

  function handleResultScroll(event: UIEvent<HTMLDivElement>) {
    const node = event.currentTarget;
    const range = visibleRange(node.scrollTop, node.clientHeight, totalRows);
    onViewportChange(range.start, Math.max(range.count, 40));
    onScroll();
  }

  function syncEditChrome(event: UIEvent<HTMLTextAreaElement>) {
    const node = event.currentTarget;
    setEditScroll(node.scrollTop);
    setEditHeight(node.clientHeight);
  }

  if (mode === "edit") {
    return (
      <div className="pane-frame">
        <div className="highlight-layer" aria-hidden="true">
          <div
            className="virt-window"
            style={{
              transform: `translateY(${editVis.start * LINE_BOX_PX - editScroll}px)`,
            }}
          >
            {Array.from({ length: editVis.count }, (_, index) => {
              const lineNo = editVis.start + index + 1;
              return (
                <div
                  key={lineNo}
                  className={
                    dirtyLines.has(lineNo) ? "highlight-row is-diff" : "highlight-row"
                  }
                />
              );
            })}
          </div>
        </div>
        <div className="edit-gutter" aria-hidden="true">
          <div
            className="virt-window"
            style={{
              transform: `translateY(${editVis.start * LINE_BOX_PX - editScroll}px)`,
            }}
          >
            {Array.from({ length: editVis.count }, (_, index) => {
              const lineNo = editVis.start + index + 1;
              return (
                <div className="gutter-cell" key={lineNo}>
                  {lineNo}
                </div>
              );
            })}
          </div>
        </div>
        <textarea
          className="pane-editor"
          value={text}
          placeholder="在此粘贴文本，或打开文件"
          onChange={(event) => onChange(event.target.value)}
          onScroll={syncEditChrome}
          spellCheck={false}
        />
      </div>
    );
  }

  return (
    <div className="pane-frame">
      <div
        className="pane-result"
        ref={scrollRef}
        onScroll={handleResultScroll}
        onClick={onStartEdit}
        title="点击回到编辑"
      >
        <div
          className="virt-space"
          style={{ height: Math.max(totalRows, 1) * LINE_BOX_PX }}
        >
          <div
            className="virt-window"
            style={{ transform: `translateY(${windowStart * LINE_BOX_PX}px)` }}
          >
            {rows.map((row, index) => {
              const line = side === "left" ? row.leftLine : row.rightLine;
              const value = side === "left" ? row.leftText : row.rightText;
              const empty = value.length === 0 && line === null;
              return (
                <div
                  key={`${side}-${windowStart + index}`}
                  className={cellClass(row.kind, empty)}
                >
                  <span className="gutter">{line ?? ""}</span>
                  <pre className="line">{value}</pre>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
