# CompareW 项目记忆

换路径或新开 Cursor 会话时先读这份。聊天历史不会跟着文件夹走，约定写在仓库里。

## 产品

轻量 Beyond Compare 风格桌面工具。**CompareW**，identifier `com.comparew.app`。界面 **zh-CN**。

栈必须保持：**Tauri 2 + React + TypeScript + Vite**，比对引擎在 Rust（`similar`）。不要换成 Electron / Java。不要反复确认后再动手，直接做完给结果。

## 已做成的能力

### 文本比对

- 左右 UTF-8 文本：打开文件或粘贴，行对齐。
- 不一致行内用红色标出不同文字（Rust 算 span，前端只渲染）。
- 中间分隔栏缩略标记不一致行，点击跳到该行。
- 中间栏上下两颗圆点：上一条 / 下一条差异团（中间隔 ≤2 行相同仍算一团）。到头变暗不绕圈。筛选「相同」时隐藏。
- 大文件：Rust `DiffStore` + 虚拟滚动；64 MiB 上限；`left.length + right.length > 200_000` 时不自动比对。
- 路径栏：可编辑，回车打开，右侧浏览按钮。占位：「在此粘贴文本，或打开文件」。
- 筛选：全部 / 差别 / 相同。
- 刷新：有路径则从磁盘重读再比对。

### 文件夹比对

- 根可以是文件夹或 `.jar` / `.zip` / `.war`。嵌套 jar 可展开（Spring Boot `BOOT-INF/lib`）。
- 按未压缩 SHA-256 比对，忽略 zip 时间戳。
- `.class` 默认同/不同按二进制；可选 CFR 反编译（`src-tauri/resources/cfr.jar` + 系统 `java`）。
- 只选一侧也能列出，另一侧是空档（`leftOnly` / `rightOnly`）。
- 树就地展开，无「进入/上一级」。双击文件夹展开，双击文件进文本比对。
- 列：名称 / 大小 / 已修改。行高 24px。
- 展开 / 折叠（工具栏）、刷新（尽量保持已展开路径）。
- 中间栏圆点同样上一条 / 下一条。已展开的红目录不停，落到不同文件 / 未展开子目录 / 仅一侧。不画红条缩略。

### Excel 比对

- 左右 `.xlsx` / `.xlsm` / `.xls` / `.xlsb` / `.ods`。Rust `calamine` 读表，前端不算 diff。
- 按工作表名对齐（忽略大小写）；表内按单元格显示值比对（公式看缓存值）。
- 不一致单元格红色 `#c62828`；工作表标签不同则标红。
- 筛选：全部 / 差别 / 相同（按行是否含不一致单元格）。
- 中间分隔栏同样可跳到不一致行。单表最多约 50 万格；文件 64 MiB 上限。
- 中间栏圆点与文本页相同：按当前表不一致行成团，上一条 / 下一条。

### 会话

- 文本页、文件夹页和 Excel 页**同时挂着**，切换不丢结果（`src/App.tsx`）。
- 文件夹里打开文件是 drill-in 叠层，不影响独立的文本会话。「返回文件夹」关掉叠层。
- **清空**：清当前这次比对。drill-in 上点清空会关掉叠层并重置文件夹会话。
- 比对结果只在内存里，关应用即丢。没有会话落盘。

## 界面约定（不要重做皮肤）

曾经用 frontend-design 改过两版（奶油金、深色头+强紫），用户明确说难看，**已撤回**。不要再套那套调色。

当前观感：

- 顶栏深色 `#232830` / `#2a3038`。
- 比对区纸色 `#f4f1ea`，名称栏 20px、上下居中，三列分隔线要能看清。
- 行间有底部分隔 + 深浅交替；选中 `#c5e0f7`。
- **一致：黑色**（`#111111`），图标也跟行色。
- **不一致：红色** `#c62828`（含子项不同的父文件夹，后端 `rollup`）。
- 文件夹图标是带页签的文件夹形，文件是折角文档。不要恢复金色文件夹、不要恢复向下箭头。
- 虚线对齐文件夹图标正下方，末子不向下穿；虚线样式统一。
- 左侧：会话切换 → **清空**（橡皮擦图标）→ **全部 / 差别 / 相同**。
- 右侧：比对、刷新；文件夹另有展开、折叠。
- 文件夹空白提示与文本占位同一套样式：「输入路径后回车，或选择文件夹」。
- Excel 空白：「输入路径后回车，或选择 Excel」。

## 架构要点

| 位置 | 职责 |
|---|---|
| `src-tauri/src/domain/align.rs` | 文本行对齐与行内 span，不读盘 |
| `src-tauri/src/domain/excel.rs` | Excel 工作表/单元格对齐，不读盘 |
| `src-tauri/src/domain/folder.rs` | 文件夹对齐与 rollup |
| `src-tauri/src/domain/scan.rs` | 扫目录/zip |
| `src-tauri/src/commands/*` | Tauri 命令 |
| `src/lib/tauri.ts` | 前端 invoke |
| `src/features/text-compare/` | 文本 UI |
| `src/features/excel-compare/` | Excel UI |
| `src/features/folder-compare/` | 文件夹 UI |
| `src/components/PathEditor.tsx` | 路径栏 |

规则：前端不算 diff、不哈希、不读 zip。空路径对文件夹比对合法。

macOS 选文件夹/jar：`src-tauri/src/picker.rs`（objc2，仅 macOS）。Windows/Linux 走 dialog 插件。

## 打包

- GitHub：`wangjiafeng93/compareW`
- Windows x64 NSIS：`.github/workflows/windows-x64.yml`（Actions 里 Package Windows x64）
- UOS amd64 `.deb`：`.github/workflows/uos-amd64.yml`（Package UOS amd64），与本地 `Dockerfile.deb` / `scripts/build-deb.sh` 一致（Ubuntu 22.04 + webkit2gtk 4.1）
- 安装包未签名

本地开发：`npm run tauri dev`（需 Node 20+、Rust stable）。

## 明确不做

- 合并、拷到对侧、三路比对
- 内置 JRE
- tar / 7z
- 跟随目录符号链接
- 忽略空白、方法级 class 导航
- 自动展开所有嵌套 jar 做全量哈希（嵌套 jar 是文件，直到用户展开）
- 会话保存 / 最近文件（尚未做）

## 规格原文

- `docs/superpowers/specs/2026-08-23-text-compare-design.md`
- `docs/superpowers/specs/2026-08-29-folder-compare-design.md`
