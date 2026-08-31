import { useEffect, useRef, useState } from "react";
import type { UIEvent } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import DiffRail from "../../components/DiffRail";
import PathEditor, { normalizePath } from "../../components/PathEditor";
import { pinNav } from "../../lib/diffNav";
import { compareExcel, getExcelRows } from "../../lib/tauri";
import SessionTabs from "../folder-compare/SessionTabs";
import type { Session } from "../folder-compare/SessionTabs";
import type { RowFilter } from "../text-compare/filterRows";
import {
  IconAll,
  IconClear,
  IconCompare,
  IconDiff,
  IconFile,
  IconRefresh,
  IconSame,
} from "../text-compare/icons";
import ToolButton from "../text-compare/ToolButton";
import ExcelPane from "./ExcelPane";
import {
  emptyExcelSummary,
  EXCEL_OVERSCAN,
  EXCEL_ROW_PX,
  type ExcelRow,
  type ExcelSummary,
} from "./types";

function errorMessage(cause: unknown): string {
  if (typeof cause === "string") return cause;
  if (cause && typeof cause === "object" && "message" in cause) {
    return String((cause as { message: unknown }).message);
  }
  return String(cause);
}

export default function ExcelComparePage({
  session,
  onSession,
}: {
  session: Session;
  onSession: (session: Session) => void;
}) {
  const [leftPath, setLeftPath] = useState("");
  const [rightPath, setRightPath] = useState("");
  const [summary, setSummary] = useState<ExcelSummary>(emptyExcelSummary);
  const [sheetIndex, setSheetIndex] = useState(0);
  const [filter, setFilter] = useState<RowFilter>("all");
  const [windowRows, setWindowRows] = useState<ExcelRow[]>([]);
  const [windowStart, setWindowStart] = useState(0);
  const [windowTotal, setWindowTotal] = useState(0);
  const [windowWidth, setWindowWidth] = useState(0);
  const [error, setError] = useState("");
  const [scrollTop, setScrollTop] = useState(0);
  const [viewHeight, setViewHeight] = useState(0);
  const leftScroll = useRef<HTMLDivElement>(null);
  const rightScroll = useRef<HTMLDivElement>(null);
  const syncing = useRef(false);
  const viewReq = useRef({ sheet: -1, filter: "all", start: -1, count: 0 });

  const sheet = summary.sheets[sheetIndex];

  async function loadWindow(
    nextSheet: number,
    nextFilter: RowFilter,
    start: number,
    count: number,
  ) {
    if (
      viewReq.current.sheet === nextSheet &&
      viewReq.current.filter === nextFilter &&
      viewReq.current.start === start &&
      viewReq.current.count === count
    ) {
      return;
    }
    viewReq.current = {
      sheet: nextSheet,
      filter: nextFilter,
      start,
      count,
    };
    const page = await getExcelRows(nextSheet, nextFilter, start, count);
    setWindowRows(page.rows);
    setWindowStart(page.offset);
    setWindowTotal(page.total);
    setWindowWidth(page.width);
  }

  async function runCompare(nextLeft = leftPath, nextRight = rightPath) {
    const left = normalizePath(nextLeft);
    const right = normalizePath(nextRight);
    if (!left && !right) {
      setSummary(emptyExcelSummary);
      setWindowRows([]);
      setWindowStart(0);
      setWindowTotal(0);
      setWindowWidth(0);
      setSheetIndex(0);
      setError("");
      await compareExcel("", "");
      return;
    }
    try {
      const next = await compareExcel(left, right);
      setSummary(next);
      setError("");
      const firstDirty = next.sheets.findIndex((item) => item.status !== "equal");
      const index = firstDirty >= 0 ? firstDirty : 0;
      setSheetIndex(index);
      setScrollTop(0);
      if (leftScroll.current) leftScroll.current.scrollTop = 0;
      if (rightScroll.current) rightScroll.current.scrollTop = 0;
      viewReq.current = { sheet: -1, filter, start: -1, count: 0 };
      if (next.sheets.length > 0) {
        await loadWindow(index, filter, 0, 80);
      } else {
        setWindowRows([]);
        setWindowTotal(0);
        setWindowWidth(0);
      }
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }

  useEffect(() => {
    const node = leftScroll.current;
    if (!node) return;
    const frame = () => setViewHeight(node.clientHeight);
    frame();
    const observer = new ResizeObserver(frame);
    observer.observe(node);
    return () => observer.disconnect();
  }, [sheetIndex, windowTotal]);

  async function browseFile(side: "left" | "right") {
    const path = await open({
      multiple: false,
      title: "选择 Excel",
      filters: [{ name: "Excel", extensions: ["xlsx", "xlsm", "xls", "xlsb", "ods"] }],
    });
    if (typeof path !== "string") return;
    if (side === "left") setLeftPath(path);
    else setRightPath(path);
    await runCompare(side === "left" ? path : leftPath, side === "right" ? path : rightPath);
  }

  function submitPath(side: "left" | "right") {
    const left = side === "left" ? normalizePath(leftPath) : leftPath;
    const right = side === "right" ? normalizePath(rightPath) : rightPath;
    if (side === "left") setLeftPath(left);
    else setRightPath(right);
    void runCompare(left, right);
  }

  function applyFilter(next: RowFilter) {
    setFilter(next);
    if (summary.sheets.length === 0) return;
    viewReq.current = { sheet: -1, filter: next, start: -1, count: 0 };
    void loadWindow(sheetIndex, next, 0, 80);
    setScrollTop(0);
    if (leftScroll.current) {
      leftScroll.current.scrollTop = 0;
      leftScroll.current.scrollLeft = 0;
    }
    if (rightScroll.current) {
      rightScroll.current.scrollTop = 0;
      rightScroll.current.scrollLeft = 0;
    }
  }

  function selectSheet(index: number) {
    setSheetIndex(index);
    viewReq.current = { sheet: -1, filter, start: -1, count: 0 };
    void loadWindow(index, filter, 0, 80);
    setScrollTop(0);
    if (leftScroll.current) leftScroll.current.scrollTop = 0;
    if (rightScroll.current) rightScroll.current.scrollTop = 0;
  }

  function handleScroll(source: "left" | "right") {
    return (event: UIEvent<HTMLDivElement>) => {
      const node = event.currentTarget;
      const start = Math.max(
        0,
        Math.floor(node.scrollTop / EXCEL_ROW_PX) - EXCEL_OVERSCAN,
      );
      const count = Math.ceil(node.clientHeight / EXCEL_ROW_PX) + EXCEL_OVERSCAN * 2;
      void loadWindow(sheetIndex, filter, start, Math.max(count, 40));
      if (syncing.current) return;
      const other = source === "left" ? rightScroll.current : leftScroll.current;
      if (!other) return;
      syncing.current = true;
      other.scrollTop = node.scrollTop;
      other.scrollLeft = node.scrollLeft;
      setScrollTop(node.scrollTop);
      setViewHeight(node.clientHeight);
      syncing.current = false;
    };
  }

  function jumpToRow(row: number) {
    const top = Math.max(0, row) * EXCEL_ROW_PX;
    if (leftScroll.current) leftScroll.current.scrollTop = top;
    if (rightScroll.current) rightScroll.current.scrollTop = top;
    setScrollTop(top);
    const start = Math.max(0, row - EXCEL_OVERSCAN);
    viewReq.current = { sheet: -1, filter, start: -1, count: 0 };
    void loadWindow(sheetIndex, filter, start, 80);
  }

  function resetSession() {
    setLeftPath("");
    setRightPath("");
    setSummary(emptyExcelSummary);
    setSheetIndex(0);
    setFilter("all");
    setWindowRows([]);
    setWindowStart(0);
    setWindowTotal(0);
    setWindowWidth(0);
    setError("");
    setScrollTop(0);
    viewReq.current = { sheet: -1, filter: "all", start: -1, count: 0 };
    void compareExcel("", "");
  }

  const marks = filter === "all" && sheet ? sheet.dirtyRows : [];
  const topRow = Math.floor(scrollTop / EXCEL_ROW_PX);
  const pins = pinNav(sheet ? sheet.dirtyRows : [], topRow, filter);
  const emptyText =
    summary.sheets.length === 0 ? "输入路径后回车，或选择 Excel" : "没有可显示的行";

  return (
    <div className="bench">
      <header className="chrome">
        <div className="chrome-lead">
          <h1>CompareW</h1>
          <SessionTabs session={session} onSession={onSession} />
          <div className="tool-group chrome-session">
            <ToolButton icon={<IconClear />} label="清空" onClick={resetSession} />
          </div>
          <div className="scope chrome-filters" role="group" aria-label="显示范围">
            <ToolButton
              kind="scope"
              icon={<IconAll />}
              label="全部"
              pressed={filter === "all"}
              onClick={() => applyFilter("all")}
            />
            <ToolButton
              kind="scope"
              icon={<IconDiff />}
              label="差别"
              pressed={filter === "diff"}
              onClick={() => applyFilter("diff")}
            />
            <ToolButton
              kind="scope"
              icon={<IconSame />}
              label="相同"
              pressed={filter === "same"}
              onClick={() => applyFilter("same")}
            />
          </div>
        </div>
        <div className="toolbar">
          <div className="tool-group">
            <ToolButton
              kind="compare"
              icon={<IconCompare />}
              label="比对"
              onClick={() => void runCompare()}
            />
            <ToolButton
              icon={<IconRefresh />}
              label="刷新"
              onClick={() => void runCompare()}
            />
          </div>
        </div>
      </header>
      {error ? <div className="banner">{error}</div> : null}
      <div className="path-bar path-bar-split">
        <PathEditor
          value={leftPath}
          placeholder="左侧 Excel 路径，回车打开"
          browseLabel="选择 Excel"
          icon={<IconFile />}
          onChange={setLeftPath}
          onSubmit={() => submitPath("left")}
          onBrowse={() => void browseFile("left")}
        />
        <div className="path-rail" aria-hidden="true" />
        <PathEditor
          value={rightPath}
          placeholder="右侧 Excel 路径，回车打开"
          browseLabel="选择 Excel"
          icon={<IconFile />}
          onChange={setRightPath}
          onSubmit={() => submitPath("right")}
          onBrowse={() => void browseFile("right")}
        />
      </div>
      {summary.sheets.length > 0 ? (
        <div className="sheet-tabs" role="tablist" aria-label="工作表">
          {summary.sheets.map((item, index) => (
            <button
              key={`${item.name}-${index}`}
              type="button"
              className={[
                "sheet-tab",
                item.status === "equal" ? "" : "is-diff",
                index === sheetIndex ? "is-on" : "",
              ]
                .filter(Boolean)
                .join(" ")}
              onClick={() => selectSheet(index)}
            >
              {item.name}
            </button>
          ))}
        </div>
      ) : null}
      <main className="panes">
        <section className="pane">
          <ExcelPane
            side="left"
            rows={windowRows}
            totalRows={windowTotal}
            width={windowWidth}
            windowStart={windowStart}
            emptyText={emptyText}
            scrollRef={leftScroll}
            onScroll={handleScroll("left")}
          />
        </section>
        <DiffRail
          totalRows={windowTotal}
          marks={marks}
          scrollTop={scrollTop}
          viewHeight={viewHeight}
          linePx={EXCEL_ROW_PX}
          onJump={jumpToRow}
          hasClusters={pins.hasClusters}
          prevRow={pins.prevRow}
          nextRow={pins.nextRow}
        />
        <section className="pane">
          <ExcelPane
            side="right"
            rows={windowRows}
            totalRows={windowTotal}
            width={windowWidth}
            windowStart={windowStart}
            emptyText={emptyText}
            scrollRef={rightScroll}
            onScroll={handleScroll("right")}
          />
        </section>
      </main>
      <footer className="status">
        <span className="stat">
          <span className="stat-mark">*</span>
          {summary.sheets.length}
        </span>
        <span className="stat">
          <span className="stat-mark">≠</span>
          {summary.changedCells}
        </span>
      </footer>
    </div>
  );
}
