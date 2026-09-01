# CompareW

轻量桌面比对工具。左右对照 UTF-8 文本，或比对两个文件夹 / `.jar` / `.zip` / `.war`，界面为简体中文。

CompareW is a lightweight, Beyond Compare–style desktop app for two-pane UTF-8 text and folder/archive comparison. UI is Simplified Chinese.

**栈：** [Tauri 2](https://tauri.app/) + React + TypeScript + Vite。比对、哈希、读 zip 全部在 Rust 完成，前端只负责展示。

当前版本 **0.1.0**。安装包未签名。

## 能做什么

### 文本比对

- 左右两侧打开文件或直接粘贴，按行对齐。
- 不一致行内用红色标出不同文字。
- 筛选：全部 / 差别 / 相同。
- 有文件路径时可刷新，从磁盘重读后再比对。
- 单侧文件上限 64 MiB；两侧合计超过约 20 万字符时不自动比对（可手动点比对）。

### 文件夹比对

- 根可以是目录，也可以是 `.jar` / `.zip` / `.war`。
- 按未压缩内容的 SHA-256 比对，忽略 zip 时间戳。
- 嵌套 jar 可展开（例如 Spring Boot `BOOT-INF/lib`）。
- 只选一侧也能列出，另一侧显示空档。
- 树就地展开；双击文件夹展开，双击文件进入文本比对。
- `.class` 默认按二进制判定同/不同；本机有 `java` 时可用内置 [CFR](https://github.com/leibnitz27/cfr) 反编译后再做文本比对（不内置 JRE）。

### 会话

- **文本** 与 **文件夹** 两个会话同时存在，切换不丢结果。
- 从文件夹里打开文件是叠层，点「返回文件夹」关掉叠层，不影响独立的文本会话。
- **清空** 只清当前这次比对。结果只在内存里，关掉应用即丢失。

## 不做

- 合并、拷到对侧、三路比对
- 忽略空白、语法高亮、方法级 class 导航
- tar / 7z，以及跟随目录符号链接
- 会话保存、最近文件
- 自动展开所有嵌套 jar 做全量哈希（嵌套 jar 当作文件，直到你展开）

## 安装

从 [Releases](https://github.com/wangjiafeng93/compareW/releases) 下载：

| 平台 | 包 |
|---|---|
| Windows x64 | NSIS 安装包（`.exe`） |
| UOS / Debian amd64 | `.deb`（自带 WebKit，不依赖系统 `libwebkit2gtk-4.1-0`） |

推送 `v*` 标签或在 Actions 里手动跑工作流即可打包：

- [Package Windows x64](.github/workflows/windows-x64.yml)
- [Package UOS amd64](.github/workflows/uos-amd64.yml)

`.class` 反编译依赖系统 `PATH` 上的 `java`。没有 Java 时文件夹比对仍可用，只是不能反编译。

统信 UOS 默认源没有 `libwebkit2gtk-4.1-0`，所以 UOS 包会把 WebKit 打进 `/opt/comparew`。能装上。若启动时提示 glibc 过旧，说明系统是 UOS V20（glibc 2.28）；需要 Deepin 23 / 较新 UOS，或 Ubuntu 22.04 / Debian 12。

## 本地开发

需要 **Node 20+** 和 **Rust stable**。Linux 还要有 webkit2gtk 4.1、GTK 3 等 Tauri 依赖。

```bash
npm install
npm run tauri dev
```

发布构建：

```bash
# 当前平台（Windows 会打 NSIS 等）
npx tauri build

# 在 Docker 里打 UOS/Debian amd64 .deb（自带 WebKit 4.1，不依赖系统 libwebkit2gtk-4.1-0）
npm run build:deb
```

推荐编辑器：[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)。

## 架构

| 位置 | 职责 |
|---|---|
| `src-tauri/src/domain/` | 文本行对齐、文件夹对齐、扫目录/zip（不依赖 UI） |
| `src-tauri/src/commands/` | Tauri 命令 |
| `src/features/text-compare/` | 文本比对界面 |
| `src/features/folder-compare/` | 文件夹比对界面 |
| `src/lib/tauri.ts` | 前端 `invoke` |

规则：前端不算 diff、不哈希、不读 zip。

更细的产品约定见 [docs/PROJECT_MEMORY.md](docs/PROJECT_MEMORY.md)。
