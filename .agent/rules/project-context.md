---
description: Global project context and architecture for flux-32
alwaysApply: true
---

# flux-32 — Project Context

**flux-32** is an educational M68K (Motorola 68000) emulator desktop app (Tauri v2 + React).

## Mission

Build the best possible GUI debugger and emulator experience to teach systems programming.

## Architecture

- **Backend (Rust)**: `src-tauri/src/`. Flat structure. No subdirectories. Minimal deps.
- **Frontend (TS/React)**: `src/`. React 18, Tailwind v4, shadcn/ui.
- **Communication**: Exclusive use of Tauri IPC commands (`invoke()`) defined in `lib.rs` and wrapped in `EmulatorAPI`.

## Core Rules

1. **Minimal Dependencies**: Verify every new crate/npm package.
2. **Flat Backend**: Keep all Rust source files in `src-tauri/src/`.
3. **Strict Typing**: No `any` in TypeScript.
4. **Testing**: Maintain passing Musashi tests (60 binaries).
