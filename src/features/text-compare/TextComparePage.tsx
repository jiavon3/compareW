import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import DiffRail from "../../components/DiffRail";
import PathEditor, { normalizePath } from "../../components/PathEditor";
import { pinNav } from "../../lib/diffNav";
import { compareTexts, getDiffRows, readTextFile } from "../../lib/tauri";
import SessionTabs from "../folder-compare/SessionTabs";
import type { Session } from "../folder-compare/SessionTabs";
import ComparePane from "./ComparePane";
import type { RowFilter } from "./filterRows";
import {
  IconAll,
  IconClear,
  IconCompare,
  IconDiff,
  IconFile,
  IconRefresh,
  IconSame,
  IconUp,
} from "./icons";
import ToolButton from "./ToolButton";
import {
  AUTO_COMPARE_MAX_CHARS,
  LINE_BOX_PX,
  emptySummary,
  type CompareSummary,
  type DiffRow,
} from "./types";

function errorMessage(cause: unknown): string {
  if (typeof cause === "string") return cause;
  if (cause && typeof cause === "object" && "message" in cause) {
    return String((cause as { message: unknown }).message);
  }
  return String(cause);
}

function isLarge(left: string, right: string): boolean {
  return left.length + right.length > AUTO_COMPARE_MAX_CHARS;
}

export default function TextComparePage({
  session = "text",
  active = true,
  onSession,
  drillIn,
  onBackToFolder,
  onNewSession,
}: {
  session?: Session;
  active?: boolean;
  onSession?: (session: Session) => void;
  drillIn?: { left: string; right: string };
  onBackToFolder?: () => void;
  onNewSession?: () => void;
}) {
  const [left, setLeft] = useState(drillIn?.left ?? "");
  const [right, setRight] = useState(drillIn?.right ?? "");
  const [leftPath, setLeftPath] = useState("");
  const [rightPath, setRightPath] = useState("");
  const [summary, setSummary] = useState<CompareSummary>(emptySummary);
  const [windowRows, setWindowRows] = useState<DiffRow[]>([]);
  const [windowStart, setWindowStart] = useState(0);
  const [windowTotal, setWindowTotal] = useState(0);
  const [mode, setMode] = useState<"edit" | "result">("edit");
  const [filter, setFilter] = useState<RowFilter>("all");
  const [error, setError] = useState("");
  const leftDirty = useRef(new Set<number>());
  const rightDirty = useRef(new Set<number>());
  const [, setDirtyTick] = useState(0);
  const leftScroll = useRef<HTMLDivElement>(null);
  const rightScroll = useRef<HTMLDivElement>(null);
  const syncing = useRef(false);
  const viewReq = useRef({ filter: "all", start: -1, count: 0 });
  const suppressAuto = useRef(false);
  const pendingJump = useRef<number | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewHeight, setViewHeight] = useState(0);

  async function loadWindow(nextFilter: RowFilter, start: number, count: number) {
    if (
      viewReq.current.filter === nextFilter &&
      viewReq.current.start === start &&
      viewReq.current.count === count
    ) {
      return;
    }
    viewReq.current = { filter: nextFilter, start, count };
    const page = await getDiffRows(nextFilter, start, count);
    setWindowRows(page.rows);
    setWindowStart(page.offset);
    setWindowTotal(page.total);
  }

  async function runDiff(nextLeft = left, nextRight = right, showResult = false) {
    const next = await compareTexts(nextLeft, nextRight);
    setSummary(next);
    leftDirty.current = new Set(next.dirtyLeft);
    rightDirty.current = new Set(next.dirtyRight);
    setDirtyTick((value) => value + 1);
    setError("");
    viewReq.current = { filter, start: -1, count: 0 };
    await loadWindow(filter, 0, 80);
    if (showResult) {
      setMode("result");
    }
  }

  useEffect(() => {
    if (!active) return;
    if (drillIn) {
      void runDiff(drillIn.left, drillIn.right, true);
      return;
    }
    if (left || right) {
      void runDiff(left, right, mode === "result");
    }
  }, [active]);

  useEffect(() => {
    if (drillIn) return;
    if (suppressAuto.current) {
      suppressAuto.current = false;
      return;
    }
    if (isLarge(left, right)) return;
    const timer = window.setTimeout(() => {
      void runDiff(left, right);
    }, 300);
    return () => window.clearTimeout(timer);
  }, [left, right]);

  useEffect(() => {
    const node = leftScroll.current;
    if (!node || mode !== "result") return;
    const frame = () => setViewHeight(node.clientHeight);
    frame();
    const observer = new ResizeObserver(frame);
    observer.observe(node);
    return () => observer.disconnect();
  }, [mode, windowTotal]);

  function editLeft(value: string) {
    setLeft(value);
    setMode("edit");
  }

  function editRight(value: string) {
    setRight(value);
    setMode("edit");
  }

  async function loadFile(side: "left" | "right", raw: string) {
    const path = normalizePath(raw);
    if (!path) return;
    if (side === "left") setLeftPath(path);
    else setRightPath(path);
    try {
      const text = await readTextFile(path);
      setError("");
      const nextLeft = side === "left" ? text : left;
      const nextRight = side === "right" ? text : right;
      if (side === "left") setLeft(text);
      else setRight(text);
      setMode("edit");
      if (isLarge(nextLeft, nextRight) && nextLeft && nextRight) {
        void runDiff(nextLeft, nextRight, true);
      }
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }

  async function browseFile(side: "left" | "right") {
    const path = await open({
      multiple: false,
      title: "选择文件",
    });
    if (typeof path !== "string") return;
    await loadFile(side, path);
  }

  function applyFilter(next: RowFilter) {
    setFilter(next);
    if (summary.rowCount > 0) {
      setMode("result");
      viewReq.current = { filter: next, start: -1, count: 0 };
      void loadWindow(next, 0, 80);
      setScrollTop(0);
      if (leftScroll.current) leftScroll.current.scrollTop = 0;
      if (rightScroll.current) rightScroll.current.scrollTop = 0;
    }
  }

  function syncFrom(source: "left" | "right") {
    if (syncing.current) return;
    const from = source === "left" ? leftScroll.current : rightScroll.current;
    const to = source === "left" ? rightScroll.current : leftScroll.current;
    if (!from || !to) return;
    syncing.current = true;
    to.scrollTop = from.scrollTop;
    setScrollTop(from.scrollTop);
    setViewHeight(from.clientHeight);
    syncing.current = false;
  }

  function handleViewport(start: number, count: number) {
    void loadWindow(filter, start, count);
  }

  function jumpToRow(row: number) {
    if (mode !== "result") {
      pendingJump.current = row;
      setMode("result");
      return;
    }
    const top = Math.max(0, row) * LINE_BOX_PX;
    if (leftScroll.current) leftScroll.current.scrollTop = top;
    if (rightScroll.current) rightScroll.current.scrollTop = top;
    setScrollTop(top);
    const start = Math.max(0, row - 20);
    viewReq.current = { filter, start: -1, count: 0 };
    void loadWindow(filter, start, 80);
  }

  useEffect(() => {
    if (mode !== "result" || pendingJump.current == null) return;
    const row = pendingJump.current;
    pendingJump.current = null;
    jumpToRow(row);
  }, [mode]);

  async function refresh() {
    suppressAuto.current = true;
    try {
      let nextLeft = left;
      let nextRight = right;
      if (leftPath) {
        nextLeft = await readTextFile(normalizePath(leftPath));
        setLeft(nextLeft);
      }
      if (rightPath) {
        nextRight = await readTextFile(normalizePath(rightPath));
        setRight(nextRight);
      }
      await runDiff(nextLeft, nextRight, Boolean(nextLeft || nextRight));
      if (!leftPath && !rightPath) suppressAuto.current = false;
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }

  function resetSession() {
    suppressAuto.current = true;
    setLeft("");
    setRight("");
    setLeftPath("");
    setRightPath("");
    setSummary(emptySummary);
    setWindowRows([]);
    setWindowStart(0);
    setWindowTotal(0);
    setMode("edit");
    setFilter("all");
    setError("");
    leftDirty.current = new Set();
    rightDirty.current = new Set();
    viewReq.current = { filter: "all", start: -1, count: 0 };
    void compareTexts("", "");
    onNewSession?.();
  }

  const topRow = Math.floor(scrollTop / LINE_BOX_PX);
  const pins = pinNav(summary.diffMarks, topRow, filter);

  return (
    <div className="bench">
      <header className="chrome">
        <div className="chrome-lead">
          <h1>CompareW</h1>
          {onSession ? (
            <SessionTabs session={session} onSession={onSession} />
          ) : null}
          <div className="tool-group chrome-session">
            <ToolButton
              icon={<IconClear />}
              label="清空"
              onClick={resetSession}
            />
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
          {onBackToFolder ? (
            <div className="tool-group">
              <ToolButton
                icon={<IconUp />}
                label="返回文件夹"
                onClick={onBackToFolder}
              />
            </div>
          ) : null}
          <div className="tool-group">
            <ToolButton
              kind="compare"
              icon={<IconCompare />}
              label="比对"
              onClick={() => {
                void runDiff(left, right, true);
              }}
            />
            <ToolButton
              icon={<IconRefresh />}
              label="重载"
              onClick={() => void refresh()}
            />
          </div>
        </div>
      </header>
      {error ? <div className="banner">{error}</div> : null}
      <div className="path-bar path-bar-split">
        <PathEditor
          value={leftPath}
          placeholder="左侧文件路径，回车打开"
          browseLabel="选择文件"
          icon={<IconFile />}
          onChange={setLeftPath}
          onSubmit={() => void loadFile("left", leftPath)}
          onBrowse={() => void browseFile("left")}
        />
        <div className="path-rail" aria-hidden="true" />
        <PathEditor
          value={rightPath}
          placeholder="右侧文件路径，回车打开"
          browseLabel="选择文件"
          icon={<IconFile />}
          onChange={setRightPath}
          onSubmit={() => void loadFile("right", rightPath)}
          onBrowse={() => void browseFile("right")}
        />
      </div>
      <main className="panes">
        <section className="pane">
          <ComparePane
            side="left"
            mode={mode}
            text={left}
            rows={windowRows}
            totalRows={windowTotal}
            windowStart={windowStart}
            onChange={editLeft}
            onStartEdit={() => setMode("edit")}
            dirtyLines={leftDirty.current}
            scrollRef={leftScroll}
            onScroll={() => syncFrom("left")}
            onViewportChange={handleViewport}
          />
        </section>
        <DiffRail
          totalRows={windowTotal}
          marks={filter === "all" ? summary.diffMarks : []}
          scrollTop={scrollTop}
          viewHeight={viewHeight}
          linePx={LINE_BOX_PX}
          onJump={jumpToRow}
          hasClusters={pins.hasClusters}
          prevRow={pins.prevRow}
          nextRow={pins.nextRow}
        />
        <section className="pane">
          <ComparePane
            side="right"
            mode={mode}
            text={right}
            rows={windowRows}
            totalRows={windowTotal}
            windowStart={windowStart}
            onChange={editRight}
            onStartEdit={() => setMode("edit")}
            dirtyLines={rightDirty.current}
            scrollRef={rightScroll}
            onScroll={() => syncFrom("right")}
            onViewportChange={handleViewport}
          />
        </section>
      </main>
      <footer className="status">
        <span className="stat">
          <span className="stat-mark">*</span>
          {summary.rowCount}
        </span>
        <span className="stat">
          <span className="stat-mark">=</span>
          {summary.stats.equal}
        </span>
        <span className="stat">
          <span className="stat-mark">≠</span>
          {summary.stats.insert + summary.stats.delete + summary.stats.replace}
        </span>
      </footer>
    </div>
  );
}
