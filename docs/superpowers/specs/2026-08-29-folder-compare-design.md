# CompareW Phase 2: Folder and Archive Compare

Date: 2026-08-29  
Status: Approved

## Problem

Phase 1 only compares two UTF-8 texts. Release work needs a Beyond Compare-style folder session: download a production jar and an upgrade jar (or their extracted trees), see which paths changed, drill into nested jars (including Spring Boot `BOOT-INF/lib`), and optionally decompile `.class` files when a JDK/JRE is installed.

## Goal

A user can pick two roots — each a directory or a `.jar` / `.zip` / `.war` — run a compare, and see a path-aligned list of same / different / left-only / right-only entries. They can enter folders and nested archives, and open UTF-8 files in the existing text compare. `.class` entries are binary-equal or binary-different by default; if `java` is on PATH, the user can decompile both sides and open the existing text compare.

## Non-goals

- Merge, copy-to-other-side, or three-way compare
- Bundled JRE (no decompile UI when `java` is missing)
- tar, 7z, or other non-zip archives
- Following directory symlinks (skip them). File symlinks are read as ordinary files when the target is readable.
- Ignore-whitespace, method-level class navigation, or intra-line highlight
- Auto-expanding every nested jar to hash inner files (nested jars are files until the user enters them)
- Session save / recent files

Phase 1 text compare stays as a separate session. Its 64 MiB UTF-8 rules still apply when a folder entry is opened as text.

## Tech stack

Unchanged from Phase 1: Tauri 2, Vite, React, TypeScript, Rust.

Additions in Rust:

- Zip reading for `.jar` / `.zip` / `.war` (and `.ear` if the file is a zip)
- Streaming SHA-256 of uncompressed entry bytes (or raw file bytes)
- A folder-compare store parallel to `DiffStore`

Decompile (optional):

- Ship a small CFR jar with the app
- Invoke system `java -jar cfr.jar` on extracted class bytes
- Do not ship a JRE

The frontend never hashes, never reads zip bytes, and never diffs. It only displays rows and invokes commands.

## Architecture

```
App
  文本比对 → existing TextComparePage
  文件夹比对 → FolderComparePage

FolderComparePage
  → dialog open file-or-folder (left / right)
  → invoke start_folder_compare
  → invoke list_folder_rows (windowed)
  → invoke folder_enter / folder_up
  → invoke cancel_folder_compare
  → drill to text: read both sides → existing compare_texts
  → drill to class: binary status; optional decompile_class then compare_texts

Rust
  FolderStore: roots, current virtual path, cached listing + rollup
  domain::folder: align names, kinds, hashes
  domain::archive: zip as a virtual directory, nested zip as a file until entered
```

Rules:

- `align_diff` still never reads disk.
- Folder listing and hashing never live in React.
- Archive roots are not fully extracted to disk. Nested jars that the user enters may be written to a temp file for zip parsing and must be deleted when the session ends or the user leaves that archive.

### Virtual paths

User-visible path bar uses Beyond Compare style:

```
app.jar!/BOOT-INF/lib/
app.jar!/BOOT-INF/lib/foo.jar!/com/example/A.class
```

When a root is a plain directory, the path bar is a relative path under that directory (`BOOT-INF/lib/`). Left and right roots may differ in type (extracted folder vs jar) and still align on the same relative virtual path.

Nested archive delimiter is `!/`. Archive-internal names are always case-sensitive. Real filesystem names follow the host OS (case-insensitive on default macOS, case-sensitive on typical Linux/UOS).

## Data contract

JSON camelCase. Rust `#[serde(rename_all = "camelCase")]`.

```ts
type FolderKind = "dir" | "archive" | "file";

type FolderStatus =
  | "equal"
  | "different"
  | "leftOnly"
  | "rightOnly"
  | "typeConflict";

type FolderRow = {
  name: string;
  kind: FolderKind;
  status: FolderStatus;
  leftSize: number | null;
  rightSize: number | null;
};

type FolderSummary = {
  pathBar: string;
  canGoUp: boolean;
  rowCount: number;
  equal: number;
  different: number;
};

type FolderWindow = {
  rows: FolderRow[];
  total: number;
  offset: number;
};
```

`leftSize` / `rightSize` are uncompressed byte lengths for files and nested archives; `null` when that side is missing. Directory rows may use `null` sizes.

`FolderSummary.different` counts rows whose status is `different`, `leftOnly`, `rightOnly`, or `typeConflict`.

`kind: "archive"` is a nested zip/jar/war that can be entered. The session roots themselves are not listed as rows.

## Comparison rules

At the current virtual directory, left and right children are aligned by relative name into one row each.

| Status | Meaning |
|---|---|
| `equal` | Both sides exist; content matches |
| `different` | Both sides exist; content does not match |
| `leftOnly` | Only on the left root |
| `rightOnly` | Only on the right root |
| `typeConflict` | One side is a directory (or enterable archive treated as container) and the other is a non-archive file |

**Files** (including nested jars not yet entered): compare SHA-256 of uncompressed bytes. Zip timestamps, extra fields, and compressed streams are ignored. Same content after recompression is `equal`.

**Directories**: status is a rollup of descendants visible in that archive/folder tree without entering nested archives. Any descendant `different` / `leftOnly` / `rightOnly` / `typeConflict` makes the directory `different`. All descendants `equal` (including two empty dirs) makes it `equal`. Nested jars contribute as single files (hash of the nested jar bytes), not their inner entries.

**Nested jars**: at the parent listing they are files. Equal hash → do not need to enter. Different hash → `different`; double-click enters `name.jar!/` as the next current path.

**Session roots**: if the user picked two jars, the first listing is the zip entries of those jars, not a single row named `app.jar`.

## Drill-in

- **Directory** or **archive**: enter that path. Toolbar 上一级 returns one level (`canGoUp` is false at the session root).
- **UTF-8 text**: both sides exist and are readable as UTF-8 without a NUL byte, or the name matches a text extension (`.xml` `.yml` `.yaml` `.properties` `.html` `.htm` `.md` `.txt` `.json` `.js` `.ts` `.css` `.java` `.kt` `.sql` `.conf` `.ini` `.csv` `.gradle`). Load via a read command (64 MiB limit) and switch to the text session with `compare_texts`. Toolbar 返回文件夹 restores the folder session (same window, not a new window). Missing-side files are not opened as text.
- **`.class`**: listing uses binary hash only. If `java` is on PATH, show 反编译比对. That extracts both class files, runs bundled CFR, then opens the text session on the two Java sources. Failure keeps the binary conclusion and sets the banner to `反编译失败`. If `java` is missing, do not show 反编译比对.
- **Other binary**: show equal/different only; do not open text compare.

## Commands

### `start_folder_compare`

```
start_folder_compare(left: String, right: String) -> Result<FolderSummary, String>
```

Validates both paths, detects directory vs zip, replaces `FolderStore`, compares the root listing. Errors: `无法打开文件`, `无法打开文件夹`, `无法作为压缩包打开`.

### `list_folder_rows`

```
list_folder_rows(filter: String, offset: u32, limit: u32) -> FolderWindow
```

`filter`: `all` | `same` | `diff` (diff = not `equal`). `limit` clamped to 1–300, same idea as text windows.

### `folder_enter` / `folder_up`

```
folder_enter(name: String) -> Result<FolderSummary, String>
folder_up() -> Result<FolderSummary, String>
```

`folder_enter` works on any directory or archive row, including `equal` nested jars (the user can still look inside). `folder_up` at root is not offered (`canGoUp` is false).

### `cancel_folder_compare`

```
cancel_folder_compare() -> ()
```

Aborts an in-flight scan. Store listing becomes empty; UI shows an empty list.

### `read_folder_entry`

```
read_folder_entry(side: "left" | "right", virtualPath: String) -> Result<String, String>
```

Reads one file (disk or zip entry) as UTF-8 with the same 64 MiB and UTF-8 errors as `read_text_file`.

### `java_available` / `decompile_class`

```
java_available() -> bool
decompile_class(side: "left" | "right", virtualPath: String) -> Result<String, String>
```

`decompile_class` fails with `反编译失败` if java or CFR fails.

Scanning runs off the UI thread. While running, the UI shows `正在比对…` and ignores repeat 比对 clicks until done or cancelled.

## UI

Single window. Header session switch: `文本比对` | `文件夹比对`.

Folder toolbar (zh-CN, sentence case, same icon-group chrome as Phase 1):

- `打开左侧` `打开右侧` `比对`
- `上一级`
- `*` 全部  `=` 相同  `≠` 差别

Path bar under the toolbar. One aligned table: 名称, 状态, 左侧大小, 右侧大小. Virtual-scrolled. Inconsistent rows use the same purple wash as text compare (`#e4d4f2` / `#5c3d86`). Equal rows have no fill.

Open dialogs accept a file or a folder. `.jar` `.zip` `.war` (zip-format `.ear` included) are archive roots; anything else that is a directory is a folder root.

Status bar: `* rowCount`  `= equal`  `≠ different` (the four non-equal statuses).

Errors: one-line banner under the toolbar, replaced on the next success (same as Phase 1). Per-entry read failures mark that row failed and set the banner; other rows still complete.

Copy:

- Empty state: `打开左侧和右侧的文件夹或 jar`
- Scanning: `正在比对…`
- Decompile button: `反编译比对`
- Back from text: `返回文件夹`

## Permissions

Tauri capabilities stay read-oriented:

- Dialog open for files and folders
- Filesystem read for user-selected roots (recursive for directories; zip bytes for archives)
- No blanket write of user data. Temp files for nested-zip parse and CFR input are app temp and deleted afterwards.

## Testing

Rust unit tests with temp dirs and fixture zips (including one nested jar), no JDK required:

- Identical trees → all `equal`
- Left-only and right-only names
- Same name, different bytes → `different`
- File vs directory → `typeConflict`
- Zip entry timestamp change only → `equal`
- Nested `lib/foo.jar` same bytes → `equal` without enter; different bytes → `different` and enter lists inner entries
- Directory rollup: dirty child ⇒ parent `different`
- Empty directory and empty zip
- Corrupt zip → open error, no partial success tree
- Text drill-in over 64 MiB → `文件超过 64MB`
- Window + `diff` filter offset/limit

## Constraints

- Product name CompareW, identifier `com.comparew.app`, UI zh-CN
- Node.js 20+, Rust stable
- Text drill-in max 64 MiB (`67108864` bytes)
- Folder scan may walk larger trees; hash files by streaming (do not load a fat jar into one buffer)
- First implementation keeps the current stack; Linux/UOS packaging is unchanged (build on the target OS)
