import { useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, UIEvent } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import PathEditor, { normalizePath } from "../../components/PathEditor";
import type { RowFilter } from "../text-compare/filterRows";
import {
  IconAll,
  IconArchive,
  IconClear,
  IconCollapseAll,
  IconCompare,
  IconDiff,
  IconExpandAll,
  IconFile,
  IconFolder,
  IconRefresh,
  IconSame,
} from "../text-compare/icons";
import ToolButton from "../text-compare/ToolButton";
import {
  decompileClass,
  javaAvailable,
  listFolderChildren,
  readFolderEntry,
  startFolderCompare,
} from "../../lib/tauri";
import SessionTabs from "./SessionTabs";
import {
  emptyFolderSummary,
  FOLDER_ROW_PX,
  type FolderRow,
  type FolderStatus,
  type FolderSummary,
} from "./types";

type Props = {
  session: "text" | "folder";
  onSession: (session: "text" | "folder") => void;
  onOpenText: (left: string, right: string) => void;
  onClearDrillIn?: () => void;
  resetToken?: number;
};

type VisibleRow = {
  row: FolderRow;
  path: string[];
  depth: number;
  lastAt: boolean[];
};

function formatSize(value: number | null, kind: FolderRow["kind"]): string {
  if (value == null || kind === "dir") return "—";
  return value.toLocaleString("en-US");
}

function formatMtime(value: number | null): string {
  if (value == null) return "—";
  const date = new Date(value * 1000);
  if (Number.isNaN(date.getTime())) return "—";
  const pad = (n: number) => String(n).padStart(2, "0");
  const time = `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
  const now = new Date();
  if (
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate()
  ) {
    return `今天, ${time}`;
  }
  return `${date.getFullYear()}年${date.getMonth() + 1}月${date.getDate()}日 ${time}`;
}

function errorMessage(cause: unknown): string {
  if (typeof cause === "string") return cause;
  if (cause && typeof cause === "object" && "message" in cause) {
    return String((cause as { message: unknown }).message);
  }
  return String(cause);
}

function statusLabel(status: FolderStatus): string {
  switch (status) {
    case "equal":
      return "相同";
    case "different":
      return "不同";
    case "leftOnly":
      return "仅左侧";
    case "rightOnly":
      return "仅右侧";
    case "typeConflict":
      return "类型冲突";
  }
}

function pathKey(path: string[]): string {
  return path.join("\0");
}

async function pickFolder(): Promise<string | null> {
  const folder = await open({
    multiple: false,
    directory: true,
    title: "选择文件夹",
  });
  return typeof folder === "string" ? folder : null;
}

function isClass(name: string): boolean {
  return name.toLowerCase().endsWith(".class");
}

function canExpand(row: FolderRow): boolean {
  return row.kind === "dir" || row.kind === "archive";
}

function flatten(
  nodes: FolderRow[],
  prefix: string[],
  depth: number,
  expanded: Set<string>,
  children: Map<string, FolderRow[]>,
  lastAt: boolean[],
): VisibleRow[] {
  const out: VisibleRow[] = [];
  nodes.forEach((row, index) => {
    const path = [...prefix, row.name];
    const nextLast = [...lastAt, index === nodes.length - 1];
    out.push({ row, path, depth, lastAt: nextLast });
    const key = pathKey(path);
    if (expanded.has(key) && canExpand(row)) {
      out.push(
        ...flatten(
          children.get(key) ?? [],
          path,
          depth + 1,
          expanded,
          children,
          nextLast,
        ),
      );
    }
  });
  return out;
}

function guideClass(item: VisibleRow, indent: number): string {
  if (indent < item.depth - 1) {
    return item.lastAt[indent] ? "is-blank" : "is-vert";
  }
  return item.lastAt[item.depth] ? "is-elbow" : "is-tee";
}

function matchesFilter(row: FolderRow, filter: RowFilter): boolean {
  if (filter === "same") return row.status === "equal";
  if (filter === "diff") return row.status !== "equal";
  return true;
}

export default function FolderComparePage({
  session,
  onSession,
  onOpenText,
  onClearDrillIn,
  resetToken = 0,
}: Props) {
  const [leftPath, setLeftPath] = useState("");
  const [rightPath, setRightPath] = useState("");
  const [summary, setSummary] = useState<FolderSummary>(emptyFolderSummary);
  const [rootRows, setRootRows] = useState<FolderRow[]>([]);
  const [childMap, setChildMap] = useState<Map<string, FolderRow[]>>(() => new Map());
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());
  const [filter, setFilter] = useState<RowFilter>("all");
  const [error, setError] = useState("");
  const [scanning, setScanning] = useState(false);
  const [hasJava, setHasJava] = useState(false);
  const [selected, setSelected] = useState("");
  const [viewStart, setViewStart] = useState(0);
  const [viewCount, setViewCount] = useState(80);
  const leftScroll = useRef<HTMLDivElement>(null);
  const rightScroll = useRef<HTMLDivElement>(null);
  const syncing = useRef(false);
  const scanningRef = useRef(false);
  const expandGen = useRef(0);

  useEffect(() => {
    void javaAvailable().then(setHasJava);
  }, []);

  const visible = useMemo(() => {
    return flatten(rootRows, [], 0, expanded, childMap, []).filter((item) =>
      matchesFilter(item.row, filter),
    );
  }, [rootRows, expanded, childMap, filter]);

  const windowRows = visible.slice(viewStart, viewStart + viewCount);
  const selectedItem = visible.find((item) => pathKey(item.path) === selected);

  async function expandFirstLevel(roots: FolderRow[]) {
    const nextMap = new Map<string, FolderRow[]>();
    const nextExpanded = new Set<string>();
    let firstError = "";
    await Promise.all(
      roots.filter(canExpand).map(async (row) => {
        const path = [row.name];
        try {
          const kids = await listFolderChildren(path);
          const key = pathKey(path);
          nextMap.set(key, kids);
          nextExpanded.add(key);
        } catch (cause) {
          if (!firstError) firstError = errorMessage(cause);
        }
      }),
    );
    setChildMap(nextMap);
    setExpanded(nextExpanded);
    if (firstError) setError(firstError);
  }

  async function restoreExpanded(roots: FolderRow[], want: Set<string>) {
    const nextMap = new Map<string, FolderRow[]>();
    const nextExpanded = new Set<string>();
    let level: { rows: FolderRow[]; prefix: string[] }[] = [
      { rows: roots, prefix: [] },
    ];
    while (level.length > 0) {
      const nextLevel: { rows: FolderRow[]; prefix: string[] }[] = [];
      await Promise.all(
        level.flatMap(({ rows, prefix }) =>
          rows.filter(canExpand).map(async (row) => {
            const path = [...prefix, row.name];
            const key = pathKey(path);
            if (!want.has(key)) return;
            try {
              const kids = await listFolderChildren(path);
              nextMap.set(key, kids);
              nextExpanded.add(key);
              nextLevel.push({ rows: kids, prefix: path });
            } catch {
              /* path may have disappeared after refresh */
            }
          }),
        ),
      );
      level = nextLevel;
    }
    setChildMap(nextMap);
    setExpanded(nextExpanded);
  }

  async function runCompare(
    nextLeft = leftPath,
    nextRight = rightPath,
    keepExpanded = false,
  ) {
    const left = normalizePath(nextLeft);
    const right = normalizePath(nextRight);
    if ((!left && !right) || scanningRef.current) return;
    if (left !== leftPath) setLeftPath(left);
    if (right !== rightPath) setRightPath(right);
    const prevExpanded = keepExpanded ? new Set(expanded) : null;
    scanningRef.current = true;
    setScanning(true);
    setError("");
    try {
      const next = await startFolderCompare(left, right);
      const roots = await listFolderChildren([]);
      setSummary(next);
      setRootRows(roots);
      setSelected("");
      setViewStart(0);
      if (leftScroll.current) leftScroll.current.scrollTop = 0;
      if (rightScroll.current) rightScroll.current.scrollTop = 0;
      if (prevExpanded && prevExpanded.size > 0) {
        await restoreExpanded(roots, prevExpanded);
      } else {
        await expandFirstLevel(roots);
      }
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      scanningRef.current = false;
      setScanning(false);
    }
  }

  async function openFolder(side: "left" | "right") {
    const path = await pickFolder();
    if (!path) return;
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

  function resetSession() {
    onClearDrillIn?.();
    setLeftPath("");
    setRightPath("");
    setSummary(emptyFolderSummary);
    setRootRows([]);
    setChildMap(new Map());
    setExpanded(new Set());
    setSelected("");
    setError("");
    setViewStart(0);
    setFilter("all");
  }

  useEffect(() => {
    if (resetToken === 0) return;
    resetSession();
  }, [resetToken]);

  async function expandAll() {
    if (rootRows.length === 0 || scanningRef.current) return;
    const gen = ++expandGen.current;
    scanningRef.current = true;
    setScanning(true);
    try {
      const nextMap = new Map(childMap);
      const nextExpanded = new Set(expanded);
      let level: { path: string[] }[] = rootRows
        .filter(canExpand)
        .map((row) => ({ path: [row.name] }));
      while (level.length > 0) {
        if (gen !== expandGen.current) return;
        await Promise.all(
          level.map(async ({ path }) => {
            const key = pathKey(path);
            if (!nextMap.has(key)) {
              try {
                nextMap.set(key, await listFolderChildren(path));
              } catch (cause) {
                setError(errorMessage(cause));
                nextMap.set(key, []);
              }
            }
            nextExpanded.add(key);
          }),
        );
        const nextLevel: { path: string[] }[] = [];
        for (const { path } of level) {
          for (const row of nextMap.get(pathKey(path)) ?? []) {
            if (canExpand(row)) nextLevel.push({ path: [...path, row.name] });
          }
        }
        level = nextLevel;
      }
      if (gen !== expandGen.current) return;
      setChildMap(nextMap);
      setExpanded(nextExpanded);
    } finally {
      if (gen === expandGen.current) {
        scanningRef.current = false;
        setScanning(false);
      }
    }
  }

  function collapseAll() {
    expandGen.current += 1;
    scanningRef.current = false;
    setScanning(false);
    setExpanded(new Set());
  }

  async function toggleExpand(row: FolderRow, path: string[]) {
    if (!canExpand(row)) return;
    const key = pathKey(path);
    if (expanded.has(key)) {
      setExpanded((prev) => {
        const next = new Set(prev);
        next.delete(key);
        return next;
      });
      return;
    }
    try {
      let kids = childMap.get(key);
      if (!kids) {
        kids = await listFolderChildren(path);
        setChildMap((prev) => {
          const map = new Map(prev);
          map.set(key, kids ?? []);
          return map;
        });
      }
      setExpanded((prev) => {
        const next = new Set(prev);
        next.add(key);
        return next;
      });
      setError("");
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }

  async function openTextPair(path: string[]) {
    try {
      const left = await readFolderEntry("left", path);
      const right = await readFolderEntry("right", path);
      setError("");
      onOpenText(left, right);
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }

  async function openDecompile(path: string[]) {
    try {
      const left = await decompileClass("left", path);
      const right = await decompileClass("right", path);
      setError("");
      onOpenText(left, right);
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }

  async function activate(row: FolderRow, path: string[]) {
    if (canExpand(row)) {
      await toggleExpand(row, path);
      return;
    }
    if (isClass(row.name)) {
      if (hasJava && row.status !== "leftOnly" && row.status !== "rightOnly") {
        await openDecompile(path);
      } else {
        setError(row.status === "equal" ? "class 内容相同" : "class 仅按二进制比对");
      }
      return;
    }
    if (row.status === "leftOnly" || row.status === "rightOnly" || row.status === "typeConflict") {
      setError("无法作为文本打开");
      return;
    }
    await openTextPair(path);
  }

  function handleScroll(source: "left" | "right") {
    return (event: UIEvent<HTMLDivElement>) => {
      const node = event.currentTarget;
      const start = Math.max(0, Math.floor(node.scrollTop / FOLDER_ROW_PX) - 10);
      const count = Math.ceil(node.clientHeight / FOLDER_ROW_PX) + 20;
      setViewStart(start);
      setViewCount(Math.max(count, 40));
      if (syncing.current) return;
      const other = source === "left" ? rightScroll.current : leftScroll.current;
      if (!other) return;
      syncing.current = true;
      other.scrollTop = node.scrollTop;
      syncing.current = false;
    };
  }

  function renderSide(side: "left" | "right") {
    const emptyStatus = side === "left" ? "rightOnly" : "leftOnly";
    return (
      <div
        className="folder-list"
        ref={side === "left" ? leftScroll : rightScroll}
        onScroll={handleScroll(side)}
      >
        {rootRows.length === 0 ? (
          <div className="folder-empty">
            {scanning ? "正在比对…" : "输入路径后回车，或选择文件夹"}
          </div>
        ) : (
          <div className="virt-space" style={{ height: visible.length * FOLDER_ROW_PX }}>
            <div
              className="virt-window"
              style={{ transform: `translateY(${viewStart * FOLDER_ROW_PX}px)` }}
            >
              {windowRows.map((item, index) => {
                const empty = item.row.status === emptyStatus;
                const key = pathKey(item.path);
                const twist = canExpand(item.row);
                const open = expanded.has(key);
                const kindIcon =
                  item.row.kind === "archive" ? (
                    <IconArchive />
                  ) : item.row.kind === "dir" ? (
                    <IconFolder />
                  ) : (
                    <IconFile />
                  );
                return (
                  <div
                    key={`${side}-${key}`}
                    className={[
                      "folder-side-row",
                      item.row.status === "equal" ? "" : "is-diff",
                      empty ? "is-gap" : "",
                      (viewStart + index) % 2 === 1 ? "is-alt" : "",
                      selected === key ? "is-selected" : "",
                    ]
                      .filter(Boolean)
                      .join(" ")}
                    style={
                      {
                        "--tree-y": `${(viewStart + index) * FOLDER_ROW_PX}px`,
                      } as CSSProperties
                    }
                    onClick={() => setSelected(key)}
                    onDoubleClick={() => {
                      if (empty && !twist) return;
                      void activate(item.row, item.path);
                    }}
                    title={statusLabel(item.row.status)}
                  >
                    <span className="folder-lead">
                      {Array.from({ length: item.depth }, (_, indent) => (
                        <span
                          key={indent}
                          className={`tree-guide ${guideClass(item, indent)}`}
                        />
                      ))}
                      {empty ? (
                        <span
                          className={[
                            "folder-kind is-spacer",
                            open ? "is-open" : "",
                          ]
                            .filter(Boolean)
                            .join(" ")}
                        />
                      ) : (
                        <span
                          className={[
                            "folder-kind",
                            twist ? "is-dir" : "is-file",
                            open ? "is-open" : "",
                          ]
                            .filter(Boolean)
                            .join(" ")}
                        >
                          {kindIcon}
                        </span>
                      )}
                      <span className="folder-name">
                        {empty ? "" : item.row.name}
                      </span>
                    </span>
                    <span className="folder-size">
                      {empty
                        ? ""
                        : formatSize(
                            side === "left" ? item.row.leftSize : item.row.rightSize,
                            item.row.kind,
                          )}
                    </span>
                    <span className="folder-mtime">
                      {empty
                        ? ""
                        : formatMtime(
                            side === "left" ? item.row.leftMtime : item.row.rightMtime,
                          )}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="bench">
      <header className="chrome">
        <div className="chrome-lead">
          <h1>CompareW</h1>
          <SessionTabs session={session} onSession={onSession} />
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
              onClick={() => setFilter("all")}
            />
            <ToolButton
              kind="scope"
              icon={<IconDiff />}
              label="差别"
              pressed={filter === "diff"}
              onClick={() => setFilter("diff")}
            />
            <ToolButton
              kind="scope"
              icon={<IconSame />}
              label="相同"
              pressed={filter === "same"}
              onClick={() => setFilter("same")}
            />
          </div>
        </div>
        <div className="toolbar">
          <div className="tool-group">
            <ToolButton
              kind="compare"
              icon={<IconCompare />}
              label={scanning ? "比对中" : "比对"}
              onClick={() => void runCompare()}
            />
            <ToolButton
              icon={<IconRefresh />}
              label="刷新"
              onClick={() => void runCompare(leftPath, rightPath, true)}
            />
          </div>
          <div className="tool-group">
            <ToolButton
              icon={<IconExpandAll />}
              label="展开"
              onClick={() => void expandAll()}
            />
            <ToolButton
              icon={<IconCollapseAll />}
              label="折叠"
              onClick={collapseAll}
            />
          </div>
          {hasJava && selectedItem && isClass(selectedItem.row.name) ? (
            <div className="tool-group">
              <ToolButton
                kind="compare"
                icon={<IconCompare />}
                label="反编译比对"
                onClick={() => void openDecompile(selectedItem.path)}
              />
            </div>
          ) : null}
        </div>
      </header>
      {error ? <div className="banner">{error}</div> : null}
      <div className="path-bar path-bar-split">
        <PathEditor
          value={leftPath}
          placeholder="左侧路径，回车打开"
          onChange={setLeftPath}
          onSubmit={() => submitPath("left")}
          onBrowse={() => void openFolder("left")}
        />
        <div className="path-rail" aria-hidden="true" />
        <PathEditor
          value={rightPath}
          placeholder="右侧路径，回车打开"
          onChange={setRightPath}
          onSubmit={() => submitPath("right")}
          onBrowse={() => void openFolder("right")}
        />
      </div>
      <main className="panes">
        <section className="pane">
          <div className="folder-pane-head">
            <span>名称</span>
            <span>大小</span>
            <span>已修改</span>
          </div>
          {renderSide("left")}
        </section>
        <div className="rail" aria-hidden="true" />
        <section className="pane">
          <div className="folder-pane-head">
            <span>名称</span>
            <span>大小</span>
            <span>已修改</span>
          </div>
          {renderSide("right")}
        </section>
      </main>
      <footer className="status">
        <span className="stat">
          <span className="stat-mark">*</span>
          {summary.rowCount}
        </span>
        <span className="stat">
          <span className="stat-mark">=</span>
          {summary.equal}
        </span>
        <span className="stat">
          <span className="stat-mark">≠</span>
          {summary.different}
        </span>
      </footer>
    </div>
  );
}
