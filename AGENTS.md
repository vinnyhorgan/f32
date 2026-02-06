# flux-32 — Agent Instructions

Educational M68K emulator playground (Tauri + React). Use this file for quick orientation.

## Essential Reading
- **CLAUDE.md** — Full project spec, architecture, non‑negotiable rules
- **.cursor/rules/** — Scoped rules (Rust, TypeScript, architecture)

## Key Constraints
- **Backend**: Flat `src-tauri/src/`, minimal deps, Tauri IPC in `lib.rs`
- **Frontend**: React + Tailwind + shadcn/ui, EmulatorAPI for all backend calls
- **Tests**: Musashi test suite (60 binaries), `cargo test` in src-tauri

## Workflow
```bash
npm run tauri dev    # Run app
cargo test          # In src-tauri — run tests
```

## Adding Features
- **Tauri command**: `lib.rs` → `invoke_handler!` → `emulator-api.ts` → `emulator-types.ts`
- **UI component**: `src/components/` or `npx shadcn@latest add [component]`
