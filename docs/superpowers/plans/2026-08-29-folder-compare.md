# Folder Compare Implementation Plan

> **For agentic workers:** Execute inline in this session. User asked not to pause for review. Do not commit unless asked.

**Goal:** Add a folder/archive compare session (directories and zip/jar/war, nested jars, class as binary with optional decompile) while keeping Phase 1 text compare.

**Architecture:** Rust builds a virtual tree (filesystem or zip, uncompressed SHA-256), aligns children by name, windows the listing. React adds a folder session and drills UTF-8 files into the existing text compare.

**Tech Stack:** Tauri 2, React, TypeScript, `zip`, `sha2`, existing `similar` text diff. Optional system `java` + bundled CFR or `javap`.

## Global Constraints

- UI zh-CN, product CompareW, `com.comparew.app`
- Text drill-in 64 MiB UTF-8; hash files by streaming
- Nested jars are files until entered; compare zip uncompressed bytes, ignore timestamps
- No bundled JRE; no merge; no tar/7z
- Do not commit unless the user asks

## Files

- Create: `src-tauri/src/domain/folder.rs` (types, align, filter window)
- Create: `src-tauri/src/domain/scan.rs` (dir walk, zip tree, nested extract)
- Create: `src-tauri/src/commands/folder.rs` (store + commands)
- Modify: `src-tauri/src/domain/mod.rs`, `commands/mod.rs`, `lib.rs`, `Cargo.toml`
- Create: `src/features/folder-compare/*`
- Modify: `src/App.tsx`, `src/lib/tauri.ts`, `src/styles.css`, `src/features/text-compare/TextComparePage.tsx`

## Tasks

1. Domain align + tests (no IO)
2. Scan dir/zip + nested jar tests
3. Tauri commands + FolderStore
4. Folder UI + session switch + text drill-in
5. Optional decompile when `java` exists
6. `cargo test --lib` and `npx tsc --noEmit`

Execute all in this session. Skip git commits.
