# Two-Text Compare Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Tauri 2 desktop window that compares two UTF-8 texts side by side with aligned, colored rows.

**Architecture:** React renders `DiffRow[]` only. Rust `similar` computes the line diff; `domain::align_diff` turns `DiffOp` into aligned rows. File bytes are read in Rust after the dialog plugin returns a path.

**Tech Stack:** Tauri 2, Vite, React, TypeScript, `similar`, `@tauri-apps/plugin-dialog`

## Global Constraints

- Node.js 20 or newer
- Rust stable via rustup
- First supported desktop: macOS
- Max file size: 5 MiB (`5242880` bytes)
- Encoding: UTF-8 only
- UI language: zh-CN
- Product name: CompareW
- Bundle identifier: `com.comparew.app`
- Frontend never computes a diff
- `align_diff` never reads disk
- JSON field names are camelCase

See `docs/superpowers/specs/2026-08-23-text-compare-design.md` for the full contract.

## File map

- `src-tauri/src/domain/align.rs` — `align_diff`
- `src-tauri/src/commands/diff.rs` — `diff_texts`
- `src-tauri/src/commands/file.rs` — `read_text_file`
- `src/features/text-compare/*` — UI
- `src/lib/tauri.ts` — invoke wrappers

## Tasks

1. Scaffold Tauri 2 + React + TypeScript, identity `CompareW` / `com.comparew.app`
2. TDD `align_diff` (empty, equal, insert, delete, replace-pad)
3. TDD `read_text_file` (5 MiB, UTF-8, BOM, missing file) and register commands + dialog plugin
4. Frontend types and `diffTexts` / `readTextFile`
5. Proof-bench UI: dual pane, rail, zh-CN copy, debounce 300ms, sync scroll
6. Verify `cargo test --lib`, `npx tsc --noEmit`, and the manual compare path
