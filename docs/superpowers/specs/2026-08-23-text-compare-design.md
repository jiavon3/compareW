# CompareW Phase 1: Two-Text Compare

Date: 2026-08-23  
Status: Approved

## Problem

Build a lightweight desktop compare tool (Beyond Compare-like, smaller). Phase 1 is only side-by-side comparison of two texts. Directory compare, merge, and binary compare are out of scope.

## Goal

A user can paste or open two UTF-8 texts, run a compare, and see aligned left/right rows with line-level coloring, line numbers, synced scroll, and a `+N  -M  ~K` status bar.

## Non-goals (Phase 1)

- Intra-line character highlight
- Ignore-whitespace options
- Copy hunk / merge editing
- Directory, binary, or 3-way compare
- Syntax highlighting
- Session save / recent files
- Non-UTF-8 encodings

## Tech stack

- Tauri 2
- Vite + React + TypeScript
- Rust crate `similar` (`TextDiff::from_lines`, Myers)
- `@tauri-apps/plugin-dialog` for the open-file picker
- File bytes are read in Rust, not in the frontend

## Architecture

```
React TextComparePage
  -> invoke("diff_texts", { left, right })
  -> invoke("read_text_file", { path })
  -> dialog.open()

Rust commands
  diff_texts   -> domain::align_diff
  read_text_file -> UTF-8 + 64 MiB guard

domain::align_diff
  similar::TextDiff::from_lines
  DiffOp -> Vec<DiffRow>
```

Rules:

- The frontend never computes a diff.
- `align_diff` never reads disk and never knows about Tauri.
- `read_text_file` never diffs.

## Data contract

JSON uses camelCase. Rust structs use `#[serde(rename_all = "camelCase")]`.

```ts
type DiffKind = "equal" | "delete" | "insert" | "replace";

type DiffRow = {
  leftLine: number | null;
  rightLine: number | null;
  leftText: string;
  rightText: string;
  kind: DiffKind;
};

type DiffResult = {
  rows: DiffRow[];
  stats: {
    equal: number;
    insert: number;
    delete: number;
    replace: number;
  };
};
```

Line numbers are 1-based. `null` means that side has no source line (a gap).

`stats` counts **rows**, not source lines:

- `equal`: rows whose `kind` is `equal`
- `insert`: rows whose `kind` is `insert`
- `delete`: rows whose `kind` is `delete`
- `replace`: rows whose `kind` is `replace`

## Alignment rules

`similar` yields `DiffOp` over line slices. Convert as follows. Newlines are stripped from stored `leftText` / `rightText`.

### Equal

For each line in the equal range, emit one `equal` row. Both sides get the same text and sequential line numbers.

### Delete

For each deleted left line, emit `kind: "delete"`, `rightLine: null`, `rightText: ""`.

### Insert

For each inserted right line, emit `kind: "insert"`, `leftLine: null`, `leftText: ""`.

### Replace

Zip left and right replacement ranges. If one side is shorter, pad it with `null` line numbers and empty text. Every emitted row is `kind: "replace"` (including the padded gap rows).

Example: left `["a", "b"]` vs right `["x"]` becomes:

| leftLine | leftText | rightLine | rightText | kind |
|---:|---|---:|---|---|
| 1 | a | 1 | x | replace |
| 2 | b | null |  | replace |

Empty inputs are valid. Two empty strings produce `{ rows: [], stats: { equal: 0, insert: 0, delete: 0, replace: 0 } }`.

A file or paste that has no trailing newline is still split by lines the same way `similar::TextDiff::from_lines` does.

## Commands

### `diff_texts`

```
diff_texts(left: String, right: String) -> DiffResult
```

Always succeeds. Empty strings are allowed.

### `read_text_file`

```
read_text_file(path: String) -> Result<String, String>
```

Behavior:

1. If the file size is greater than `67108864` bytes (64 MiB), return error `"文件超过 64MB"`.
2. If the file cannot be opened, return error `"无法打开文件"`.
3. If the bytes are not valid UTF-8, return error `"无法读取文件：不是有效的 UTF-8"`.
4. Otherwise return the file contents as a UTF-8 string (BOM `EF BB BF` is stripped if present).

The frontend obtains `path` from `@tauri-apps/plugin-dialog` `open({ multiple: false })`. It does not read file bytes itself.

## UI

Single window, single page.

```
[ 打开左侧 ]  [ 打开右侧 ]  [ 比对 ]
┌─────────────────────┬─────────────────────┐
│ 1  hello            │ 1  hallo            │
│ 2  world            │ 2  world            │
└─────────────────────┴─────────────────────┘
+1  -1  ~1
```

Copy (zh-CN, sentence case):

- Buttons: `打开左侧`, `打开右侧`, `比对`
- Status: `+{insert}  -{delete}  ~{replace}`
- Empty panes: `在此粘贴文本，或打开文件`
- File errors show as a one-line banner under the toolbar, replaced on the next success

Behavior:

- Each pane is a textarea-like editor before compare and a read-only aligned row list after compare. Editing a pane (paste or type) marks the current result stale and shows the editor content. Pressing `比对` or a successful file load of the other pane while both sides have content refreshes the result.
- Auto-compare: when both panes are non-empty, debounce 300ms and call `diff_texts`.
- `比对` calls `diff_texts` immediately (no debounce).
- Opening a file replaces that pane's text, then triggers compare if the other pane is non-empty.
- Vertical scroll of the two result lists is synchronized (setting one scrollTop sets the other).
- Horizontal scroll is independent.
- Line numbers are a gutter; they do not scroll away horizontally with long lines.

Row colors:

- `equal`: no fill
- any inconsistent row (`delete` / `insert` / `replace`, including padded gaps): purple wash on both cells (`#e4d4f2` / `#5c3d86`)

Visual direction: a proof bench / contact-print table. Quiet chrome, paper panes, a thin vertical registration rail between the two columns as the one signature element. Diff colors are muted inks, not neon.

## Permissions

Tauri 2 capabilities grant only:

- `dialog:default` (or the equivalent `dialog:allow-open`)
- filesystem read for user-selected files required by the dialog/fs plugin pairing

No blanket home-directory or disk-wide write permissions.

## Testing

Rust unit tests in `src-tauri/src/domain/align.rs` (or a sibling `align` test module) cover:

- identical texts → all `equal`
- insertion only
- deletion only
- replace with unequal line counts (padding)
- two empty strings → empty result
- stats match row kinds

`read_text_file` tests cover the 64 MiB limit, invalid UTF-8, and BOM strip using temp files.

## Constraints

- Node.js 20 or newer
- Rust stable via rustup
- First supported desktop: macOS (the current machine). Windows/Linux are not blocked by code, but are not a Phase 1 verification target.
- Max file size: 64 MiB (`67108864` bytes)
- Encoding: UTF-8 only
- UI language: zh-CN
- Product name: CompareW
- Bundle identifier: `com.comparew.app`
