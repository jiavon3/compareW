# Diff Pin Prev/Next Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the two decorative gold dots on the center rail into previous/next difference-hunk buttons on text, Excel, and folder pages.

**Architecture:** Cluster existing dirty-row indexes in `src/lib/diffNav.ts` (frontend does not compute diffs). Pages pass mapped `hasClusters` / `prevRow` / `nextRow` into `DiffRail`, which renders two buttons and still jumps to the nearest red mark on rail-body click.

**Tech Stack:** Tauri 2, React, TypeScript, Vite, Vitest (new, only for `diffNav.ts`)

## Global Constraints

- UI language: zh-CN
- Frontend never computes a diff, never hashes, never reads zip
- Pin look stays `--pin` gold radial gradient, visual size 8px
- No keyboard shortcuts this version
- No wrap-around at ends
- Folder rail does not gain red minimap marks
- Do not restyle the app chrome or compare paper
- Do not restore gold folder icons or frontend-design palettes
- Spec: `docs/superpowers/specs/2026-08-31-diff-pin-nav-design.md`

## File map

- Create: `src/lib/diffNav.ts` — `clusterMarks`, `pinTargets`, `mapStartToFilteredIndex`, `folderStopRows`, `remapClusterStarts`, `pinNav`
- Create: `src/lib/diffNav.test.ts` — Vitest for those functions
- Modify: `package.json` — add `vitest` and `test` script
- Modify: `vite.config.ts` — Vitest `test` block
- Modify: `src/components/DiffRail.tsx` — pin buttons
- Modify: `src/styles.css` — replace `.rail::before/::after` with `.rail-pin`
- Modify: `src/features/text-compare/TextComparePage.tsx` — `pinNav` + new DiffRail props
- Modify: `src/features/excel-compare/ExcelComparePage.tsx` — same
- Modify: `src/features/folder-compare/FolderComparePage.tsx` — stop rows, jump, DiffRail
- Modify: `docs/PROJECT_MEMORY.md` — document pin navigation

---

### Task 1: Vitest + failing `diffNav` tests

**Files:**
- Modify: `package.json`
- Modify: `vite.config.ts`
- Create: `src/lib/diffNav.test.ts`

**Interfaces:**
- Consumes: nothing
- Produces: failing tests that import from `./diffNav` (file not created yet)

- [ ] **Step 1: Add Vitest**

Run:

```bash
npm install -D vitest
```

In `package.json` scripts, add `"test": "vitest run"` next to `"preview"`.

In `vite.config.ts`, change the first two lines to:

```ts
/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
```

Inside the object passed to `defineConfig`, after `plugins: [react()],` add:

```ts
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
```

Leave the existing Tauri `server` / `clearScreen` config unchanged.

- [ ] **Step 2: Write the failing tests**

Create `src/lib/diffNav.test.ts` with this exact content:

```ts
import { describe, expect, it } from "vitest";
import {
  clusterMarks,
  folderStopRows,
  mapStartToFilteredIndex,
  pinNav,
  pinTargets,
  remapClusterStarts,
} from "./diffNav";

describe("clusterMarks", () => {
  it("returns no clusters for an empty list", () => {
    expect(clusterMarks([])).toEqual([]);
  });

  it("treats a single row as one cluster", () => {
    expect(clusterMarks([7])).toEqual([{ start: 7, end: 7 }]);
  });

  it("merges consecutive dirty rows", () => {
    expect(clusterMarks([10, 11, 12])).toEqual([{ start: 10, end: 12 }]);
  });

  it("merges when one equal row sits between marks", () => {
    expect(clusterMarks([10, 12])).toEqual([{ start: 10, end: 12 }]);
  });

  it("merges when two equal rows sit between marks", () => {
    expect(clusterMarks([10, 11, 12, 15, 20])).toEqual([
      { start: 10, end: 15 },
      { start: 20, end: 20 },
    ]);
  });

  it("splits when three equal rows sit between marks", () => {
    expect(clusterMarks([10, 14])).toEqual([
      { start: 10, end: 10 },
      { start: 14, end: 14 },
    ]);
  });

  it("sorts and dedupes before clustering", () => {
    expect(clusterMarks([12, 10, 10, 11])).toEqual([{ start: 10, end: 12 }]);
  });
});

describe("pinTargets", () => {
  const clusters = [
    { start: 10, end: 15 },
    { start: 40, end: 42 },
    { start: 80, end: 80 },
  ];

  it("disables prev on the first cluster start", () => {
    expect(pinTargets(clusters, 10)).toEqual({ prev: null, next: 40 });
  });

  it("disables next on the last cluster start", () => {
    expect(pinTargets(clusters, 80)).toEqual({ prev: 40, next: null });
  });

  it("picks neighbors when the viewport is between clusters", () => {
    expect(pinTargets(clusters, 20)).toEqual({ prev: 10, next: 40 });
  });

  it("skips the current hunk when the viewport is inside it", () => {
    expect(pinTargets(clusters, 12)).toEqual({ prev: 10, next: 40 });
  });

  it("disables both pins when sitting on the only cluster", () => {
    expect(pinTargets([{ start: 5, end: 8 }], 5)).toEqual({
      prev: null,
      next: null,
    });
  });
});

describe("mapStartToFilteredIndex", () => {
  it("counts marks strictly before start", () => {
    expect(mapStartToFilteredIndex([10, 11, 12, 20], 20)).toBe(3);
    expect(mapStartToFilteredIndex([10, 11, 12, 20], 10)).toBe(0);
  });
});

describe("pinNav", () => {
  const marks = [10, 11, 12, 20];

  it("hides pins for the same-only filter", () => {
    expect(pinNav(marks, 0, "same")).toEqual({
      hasClusters: false,
      prevRow: null,
      nextRow: null,
    });
  });

  it("uses unfiltered indexes for the all filter", () => {
    expect(pinNav(marks, 10, "all")).toEqual({
      hasClusters: true,
      prevRow: null,
      nextRow: 20,
    });
  });

  it("maps hunk starts into the diff-only list", () => {
    expect(pinNav(marks, 0, "diff")).toEqual({
      hasClusters: true,
      prevRow: null,
      nextRow: 3,
    });
  });

  it("hides pins when there are no marks", () => {
    expect(pinNav([], 0, "all")).toEqual({
      hasClusters: false,
      prevRow: null,
      nextRow: null,
    });
  });
});

describe("folderStopRows", () => {
  it("stops on a collapsed dirty directory", () => {
    expect(
      folderStopRows([
        { status: "different", kind: "dir", expanded: false },
      ]),
    ).toEqual([0]);
  });

  it("skips an expanded dirty directory and stops on dirty files", () => {
    expect(
      folderStopRows([
        { status: "different", kind: "dir", expanded: true },
        { status: "different", kind: "file", expanded: false },
        { status: "equal", kind: "file", expanded: false },
      ]),
    ).toEqual([1]);
  });

  it("does not stop on equal files", () => {
    expect(
      folderStopRows([{ status: "equal", kind: "file", expanded: false }]),
    ).toEqual([]);
  });

  it("stops on left-only, right-only, and type-conflict rows", () => {
    expect(
      folderStopRows([
        { status: "leftOnly", kind: "file", expanded: false },
        { status: "rightOnly", kind: "file", expanded: false },
        { status: "typeConflict", kind: "file", expanded: false },
      ]),
    ).toEqual([0, 1, 2]);
  });
});

describe("remapClusterStarts", () => {
  it("drops clusters whose start is not visible", () => {
    expect(
      remapClusterStarts(
        [
          { start: 0, end: 0 },
          { start: 2, end: 2 },
        ],
        (start) => (start === 2 ? 0 : null),
      ),
    ).toEqual([{ start: 0, end: 0 }]);
  });
});
```

- [ ] **Step 3: Run tests and confirm they fail to import**

Run: `npm test`

Expected: FAIL, module `./diffNav` not found (or named exports missing).

- [ ] **Step 4: Commit**

```bash
git add package.json package-lock.json vite.config.ts src/lib/diffNav.test.ts
git commit -m "Add Vitest coverage for difference-hunk pin navigation."
```

---

### Task 2: Implement `diffNav.ts`

**Files:**
- Create: `src/lib/diffNav.ts`
- Test: `src/lib/diffNav.test.ts`

**Interfaces:**
- Consumes: failing tests from Task 1
- Produces:

```ts
export type Cluster = { start: number; end: number };

export type RowFilter = "all" | "same" | "diff";

export type FolderNavRow = {
  status: "equal" | "different" | "leftOnly" | "rightOnly" | "typeConflict";
  kind: "dir" | "archive" | "file";
  expanded: boolean;
};

export function clusterMarks(marks: number[]): Cluster[];
export function pinTargets(
  clusters: Cluster[],
  topRow: number,
): { prev: number | null; next: number | null };
export function mapStartToFilteredIndex(marks: number[], start: number): number;
export function pinNav(
  marks: number[],
  topRow: number,
  filter: RowFilter,
): { hasClusters: boolean; prevRow: number | null; nextRow: number | null };
export function folderStopRows(rows: FolderNavRow[]): number[];
export function remapClusterStarts(
  clusters: Cluster[],
  mapStart: (start: number) => number | null,
): Cluster[];
```

- [ ] **Step 1: Write `src/lib/diffNav.ts`**

```ts
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
```

Note: `pinNav` returns `hasClusters: true` whenever `marks.length > 0`. After clustering that is always at least one cluster. Empty marks already returned false.

Do not import `FolderNavRow` status from the feature folder module; keep the union in this file so tests stay isolated.

- [ ] **Step 2: Run tests**

Run: `npm test`

Expected: all tests PASS.

If `tsc` complains about `unique[0]` possibly undefined, keep the `first` local as above.

- [ ] **Step 3: Commit**

```bash
git add src/lib/diffNav.ts
git commit -m "Cluster dirty rows into prev/next hunk targets."
```

---

### Task 3: Pin buttons on `DiffRail`

**Files:**
- Modify: `src/components/DiffRail.tsx`
- Modify: `src/styles.css` (`.rail::before` / `.rail::after` around lines 670–691)

**Interfaces:**
- Consumes: none of the `diffNav` functions (pages will compute targets)
- Produces: `DiffRail` extra props

```ts
hasClusters?: boolean;
prevRow?: number | null;
nextRow?: number | null;
```

Default `hasClusters` to `false` so existing call sites keep decorative-less pins until wired.

- [ ] **Step 1: Replace CSS pseudo dots with `.rail-pin`**

Delete the `.rail::before, .rail::after` rule and the `.rail::before` / `.rail::after` position rules.

Add immediately after `.rail-thumb`:

```css
.rail-pin {
  appearance: none;
  position: absolute;
  left: 50%;
  z-index: 3;
  width: 8px;
  height: 8px;
  padding: 0;
  border: 0;
  border-radius: 50%;
  transform: translateX(-50%);
  background: radial-gradient(circle at 35% 30%, #f0d59a, var(--pin) 55%, #7a5c24);
  box-shadow: 0 0 0 1px #2a3038;
  cursor: pointer;
}

.rail-pin.is-prev {
  top: 18%;
}

.rail-pin.is-next {
  bottom: 18%;
}

.rail-pin:hover:not(:disabled) {
  filter: brightness(1.12);
}

.rail-pin:disabled {
  opacity: 0.35;
  cursor: default;
}
```

- [ ] **Step 2: Render buttons in `DiffRail`**

Update the props type and destructure:

```ts
type Props = {
  totalRows: number;
  marks: number[];
  scrollTop: number;
  viewHeight: number;
  linePx: number;
  onJump: (row: number) => void;
  hasClusters?: boolean;
  prevRow?: number | null;
  nextRow?: number | null;
};
```

```ts
export default function DiffRail({
  totalRows,
  marks,
  scrollTop,
  viewHeight,
  linePx,
  onJump,
  hasClusters = false,
  prevRow = null,
  nextRow = null,
}: Props) {
```

Add a jump helper that stops the rail-body click from firing:

```ts
  function jumpPin(event: MouseEvent<HTMLButtonElement>, row: number | null) {
    event.stopPropagation();
    if (row == null) return;
    onJump(row);
  }
```

Inside the rail `div`, after the `bands.map(...)` block, add:

```tsx
      {hasClusters ? (
        <>
          <button
            type="button"
            className="rail-pin is-prev"
            aria-label="上一条差异"
            title={prevRow == null ? "没有上一条差异" : "上一条差异"}
            disabled={prevRow == null}
            onClick={(event) => jumpPin(event, prevRow)}
          />
          <button
            type="button"
            className="rail-pin is-next"
            aria-label="下一条差异"
            title={nextRow == null ? "没有下一条差异" : "下一条差异"}
            disabled={nextRow == null}
            onClick={(event) => jumpPin(event, nextRow)}
          />
        </>
      ) : null}
```

Do not change `handleClick` / red-mark snapping.

- [ ] **Step 3: Typecheck**

Run: `npx tsc --noEmit`

Expected: no errors. Existing `DiffRail` call sites still compile because new props are optional.

- [ ] **Step 4: Commit**

```bash
git add src/components/DiffRail.tsx src/styles.css
git commit -m "Make center-rail pins clickable prev/next buttons."
```

---

### Task 4: Wire text compare

**Files:**
- Modify: `src/features/text-compare/TextComparePage.tsx`

**Interfaces:**
- Consumes: `pinNav(marks, topRow, filter)` from `src/lib/diffNav.ts`
- Produces: text page passes `hasClusters` / `prevRow` / `nextRow`

- [ ] **Step 1: Import and compute pin nav**

Add import:

```ts
import { pinNav } from "../../lib/diffNav";
```

After `viewHeight` state (near the other hooks), the rail already has `scrollTop`. Just before the `return (`, compute:

```ts
  const topRow = Math.floor(scrollTop / LINE_BOX_PX);
  const pins = pinNav(summary.diffMarks, topRow, filter);
```

- [ ] **Step 2: Pass props into `DiffRail`**

Replace the existing `DiffRail` JSX with:

```tsx
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
```

Keep red marks only for filter `"all"`. Keep `jumpToRow` unchanged (it already switches edit → result via `pendingJump`).

- [ ] **Step 3: Typecheck**

Run: `npx tsc --noEmit`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/features/text-compare/TextComparePage.tsx
git commit -m "Jump text difference hunks from the rail pins."
```

---

### Task 5: Wire Excel compare

**Files:**
- Modify: `src/features/excel-compare/ExcelComparePage.tsx`

**Interfaces:**
- Consumes: `pinNav(marks: number[], topRow: number, filter: RowFilter)` from `src/lib/diffNav.ts`
- Produces: Excel `DiffRail` pin props from current sheet `dirtyRows`

- [ ] **Step 1: Import and compute**

Add:

```ts
import { pinNav } from "../../lib/diffNav";
```

`sheet` is already `summary.sheets[sheetIndex]`. Next to the existing `marks` line:

```ts
  const marks = filter === "all" && sheet ? sheet.dirtyRows : [];
  const topRow = Math.floor(scrollTop / EXCEL_ROW_PX);
  const pins = pinNav(sheet ? sheet.dirtyRows : [], topRow, filter);
```

- [ ] **Step 2: Pass props**

Replace the `DiffRail` JSX with:

```tsx
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
```

- [ ] **Step 3: Typecheck**

Run: `npx tsc --noEmit`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/features/excel-compare/ExcelComparePage.tsx
git commit -m "Jump Excel difference hunks from the rail pins."
```

---

### Task 6: Wire folder compare

**Files:**
- Modify: `src/features/folder-compare/FolderComparePage.tsx`

**Interfaces:**
- Consumes: `clusterMarks`, `folderStopRows`, `pinTargets`, `remapClusterStarts` from `src/lib/diffNav.ts`
- Produces: folder page uses `DiffRail` instead of a dummy `.rail` div; `jumpToRow` scrolls both lists and selects the row

- [ ] **Step 1: Imports and scroll state**

Add:

```ts
import DiffRail from "../../components/DiffRail";
import {
  clusterMarks,
  folderStopRows,
  pinTargets,
  remapClusterStarts,
} from "../../lib/diffNav";
```

Next to `viewStart` / `viewCount` state add:

```ts
  const [scrollTop, setScrollTop] = useState(0);
  const [viewHeight, setViewHeight] = useState(0);
```

- [ ] **Step 2: Split flatten from filter**

Replace the current `visible` memo with:

```ts
  const tree = useMemo(
    () => flatten(rootRows, [], 0, expanded, childMap, []),
    [rootRows, expanded, childMap],
  );

  const visible = useMemo(
    () => tree.filter((item) => matchesFilter(item.row, filter)),
    [tree, filter],
  );
```

- [ ] **Step 3: Compute pins**

After `visible` / `windowRows`:

```ts
  const topRow = Math.floor(scrollTop / FOLDER_ROW_PX);
  const pins = useMemo(() => {
    const stops = folderStopRows(
      tree.map((item) => ({
        status: item.row.status,
        kind: item.row.kind,
        expanded: expanded.has(pathKey(item.path)),
      })),
    );
    const clusters = remapClusterStarts(clusterMarks(stops), (start) => {
      const item = tree[start];
      if (!item) return null;
      const key = pathKey(item.path);
      const index = visible.findIndex((row) => pathKey(row.path) === key);
      return index < 0 ? null : index;
    });
    if (clusters.length === 0) {
      return { hasClusters: false, prevRow: null as number | null, nextRow: null as number | null };
    }
    const { prev, next } = pinTargets(clusters, topRow);
    return { hasClusters: true, prevRow: prev, nextRow: next };
  }, [tree, visible, expanded, topRow]);
```

- [ ] **Step 4: `jumpToRow` and scroll sync**

In `handleScroll`, after computing `start` / `count`, also:

```ts
      setScrollTop(node.scrollTop);
      setViewHeight(node.clientHeight);
```

Add:

```ts
  function jumpToRow(row: number) {
    const target = Math.max(0, Math.min(row, Math.max(visible.length, 1) - 1));
    const top = target * FOLDER_ROW_PX;
    if (leftScroll.current) leftScroll.current.scrollTop = top;
    if (rightScroll.current) rightScroll.current.scrollTop = top;
    setScrollTop(top);
    const start = Math.max(0, target - 10);
    const count = Math.max(viewCount, 40);
    setViewStart(start);
    setViewCount(count);
    const item = visible[target];
    if (item) setSelected(pathKey(item.path));
  }
```

- [ ] **Step 5: Replace the dummy rail**

Replace:

```tsx
        <div className="rail" aria-hidden="true" />
```

with:

```tsx
        <DiffRail
          totalRows={visible.length}
          marks={[]}
          scrollTop={scrollTop}
          viewHeight={viewHeight}
          linePx={FOLDER_ROW_PX}
          onJump={jumpToRow}
          hasClusters={pins.hasClusters}
          prevRow={pins.prevRow}
          nextRow={pins.nextRow}
        />
```

Leave folder `marks` as `[]` (no red minimap). Rail-body click still maps Y to a visible row via existing `rowAt` when `totalRows > 0`.

- [ ] **Step 6: Typecheck**

Run: `npx tsc --noEmit`

Expected: PASS. `noUnusedLocals` must still pass; do not leave unused imports.

- [ ] **Step 7: Commit**

```bash
git add src/features/folder-compare/FolderComparePage.tsx
git commit -m "Jump folder difference hunks from the rail pins."
```

---

### Task 7: Project memory and verify

**Files:**
- Modify: `docs/PROJECT_MEMORY.md`

**Interfaces:**
- Consumes: behavior shipped in Tasks 3–6
- Produces: memory that pins are prev/next hunk navigation

- [ ] **Step 1: Update memory**

In `### 文本比对`, after the bullet about the center rail jumping to inconsistent rows, add:

```
- 中间栏上下两颗圆点：上一条 / 下一条差异团（中间隔 ≤2 行相同仍算一团）。到头变暗不绕圈。筛选「相同」时隐藏。
```

In `### 文件夹比对`, add:

```
- 中间栏圆点同样上一条 / 下一条。已展开的红目录不停，落到不同文件 / 未展开子目录 / 仅一侧。不画红条缩略。
```

In `### Excel 比对`, after the rail bullet, add:

```
- 中间栏圆点与文本页相同：按当前表不一致行成团，上一条 / 下一条。
```

- [ ] **Step 2: Run automated checks**

Run:

```bash
npm test
npx tsc --noEmit
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: all three succeed. Rust tests are unchanged; run them anyway so nothing in the dirty tree was accidentally edited.

- [ ] **Step 3: Manual check (app)**

Run: `npm run tauri dev`

- Text: two hunks far apart → bottom pin jumps to the second, top pin returns, both dim on the only remaining direction at each end.
- Text filter 「相同」: pins hidden. Filter 「差别」: pins still jump by original hunks, not every adjacent filtered row.
- Excel: same on a sheet with two dirty regions.
- Folder: collapsed red dir is a stop; expand it, pins skip the parent and land on dirty files. Pins do not auto-expand.

- [ ] **Step 4: Commit**

```bash
git add docs/PROJECT_MEMORY.md
git commit -m "Note rail pin hunk navigation in project memory."
```

---

## Spec coverage

| Spec section | Task |
|---|---|
| Clustering `b - a <= 3` | 1–2 |
| pinTargets / no wrap / dim at ends | 1–2, 3 |
| `hasClusters` hide vs disable | 2–3 |
| Filter mapping for 差别 / 相同 | 1–2, 4–5 |
| Folder stop rules | 1–2, 6 |
| DiffRail buttons + stopPropagation | 3 |
| Text / Excel / folder wiring | 4–6 |
| No keyboard, no folder red marks, no extra Rust marks | 3, 6 (explicit non-work) |
| Vitest for `diffNav` only | 1–2 |
| PROJECT_MEMORY | 7 |
