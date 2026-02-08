# flux-32 — Project Context (Shared with Antigravity)

**flux-32** is an educational M68K (Motorola 68000) emulator desktop app (Tauri v2 + React).

## Mission

Build the best possible GUI debugger and emulator experience to teach systems programming.

## Architecture

- **Backend (Rust)**: `src-tauri/src/`. Flat structure. No subdirectories. Minimal deps.
- **Frontend (TS/React)**: `src/`. React 19, Tailwind v4, shadcn/ui, Vite 7.
- **Communication**: Exclusive use of Tauri IPC commands (`invoke()`) defined in `lib.rs` and wrapped in `EmulatorAPI`.

## Commands

- **Dev Server**: `pnpm tauri dev`
- **Build Release**: `pnpm tauri build`
- **Test Backend**: `cd src-tauri && cargo test -- --test-threads=1`
- **Lint Frontend**: `pnpm lint`

## Core Rules

### Rust Backend (`src-tauri/src/`)

- **Flat Layout**: Do not create subdirectories. Keep modules flat.
- **Entry Point**: `lib.rs` contains all `#[tauri::command]` definitions.
- **Style**: Prioritize clarity. Public items must have doc comments.
- **Quality**: Must pass `cargo fmt` and `cargo clippy -- -D warnings`.
- **Testing**: Place unit tests in `#[cfg(test)] mod tests {}`. Maintain passing Musashi tests.

### TypeScript Frontend (`src/`)

- **API Layer**: All backend communication MUST go through `src/lib/emulator-api.ts`.
- **Types**: Sync types in `src/lib/emulator-types.ts` with Rust structs.
- **Strict TypeScript**: No `any` types allowed.
- **State**: Use Zustand for global state.
- **Async**: Handle async Tauri commands with proper loading states.

## Standard Procedures

### Adding a New Tauri Command

1.  **Backend (`src-tauri/src/lib.rs`)**: Define `#[context::command] fn my_command(...) -> Result<T, String>`. Register in `invoke_handler!`.
2.  **Frontend API (`src/lib/emulator-api.ts`)**: Add static method `EmulatorAPI.myCommand(...)`. Use `invoke`.
3.  **Types (`src/lib/emulator-types.ts`)**: Define shared interfaces.

### Adding a UI Component

- **Primitive**: Use `npx shadcn@latest add [name]`.
- **Feature**: Create `src/components/[Name].tsx`. Use Tailwind classes.
- **State**: Use `useEmulatorStore` (Zustand) for global state.

## Agent Workflow & Tool Usage

- **Testing**: Use the `tauri` MCP server to test the running application.
- **Documentation**: Pull latest docs via `context7` MCP (`query-docs`) before using new libs.
- **Problem Solving**: Use `sequential-thinking` for complex reasoning.
