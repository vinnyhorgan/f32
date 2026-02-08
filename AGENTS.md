# flux-32 — Agent Configuration

**flux-32** is an educational M68K (Motorola 68000) emulator desktop app (Tauri v2 + React).

## Mission

Build the best possible GUI debugger and emulator experience to teach systems programming.

---

## Architecture

- **Backend (Rust)**: `src-tauri/src/`. Flat structure. No subdirectories. Minimal deps.
- **Frontend (TS/React)**: `src/`. React 19, Tailwind v4, shadcn/ui, Vite 7.
- **Communication**: Exclusive use of Tauri IPC commands (`invoke()`) defined in `lib.rs` and wrapped in `EmulatorAPI`.

---

## Commands

### Development

```bash
pnpm tauri dev          # Start dev server with hot reload
pnpm dev                # Frontend only
```

### Building

```bash
pnpm tauri build        # Build release binary
```

### Testing

```bash
pnpm test               # Run frontend tests (watch mode)
pnpm test:run           # Run frontend tests once
cd src-tauri && cargo test -- --test-threads=1  # Backend tests
```

### Quality

```bash
pnpm check              # Full quality check (lint + format + test)
pnpm lint               # ESLint + TypeScript check
pnpm format             # Prettier + Taplo formatting
```

---

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

---

## Standard Procedures

### Adding a New Tauri Command

1. **Backend (`src-tauri/src/lib.rs`)**: Define `#[tauri::command] fn my_command(...) -> Result<T, String>`. Register in `invoke_handler!`.
2. **Frontend API (`src/lib/emulator-api.ts`)**: Add static method `EmulatorAPI.myCommand(...)`. Use `invoke`.
3. **Types (`src/lib/emulator-types.ts`)**: Define shared interfaces.

```rust
/// Doc comment explaining the command
#[tauri::command]
fn my_new_command(arg: String) -> Result<String, String> {
    // Implementation
    Ok("success".to_string())
}
```

```typescript
static async myNewCommand(arg: string): Promise<string> {
    return await invoke('my_new_command', { arg });
}
```

### Adding a UI Component

- **Primitive**: Use `npx shadcn@latest add [name]`.
- **Feature**: Create `src/components/[Name].tsx`. Use Tailwind classes.
- **State**: Use `useEmulatorStore` (Zustand) for global state.

---

## Agent Workflow & Tool Usage

### Testing & Verification

- Use the **`tauri` MCP server** to test the running application.
- Validate application state using `driver_session`, `webview_dom_snapshot`, and other Tauri driver tools.
- Verification should involve actual interaction with the app when possible.

### Documentation & Research

- Always pull the **latest documentation** using the **`context7` MCP server** (`query-docs`) before implementing new features or using unfamiliar libraries.
- Don't assume API knowledge for rapidly evolving libraries; check the docs first.

### Problem Solving

- Use the **`sequential-thinking` tool** to break down **complex problems** into manageable steps.
- Before jumping into implementation on difficult tasks, outline your logic and potential pitfalls using sequential thinking.
- Revisit your thinking process if you encounter unexpected roadblocks.

---

## Key Files

### Backend (Rust)

- `src-tauri/src/lib.rs` - Tauri commands and app setup
- `src-tauri/src/cpu.rs` - CPU implementation
- `src-tauri/src/memory.rs` - Memory management
- `src-tauri/src/instructions.rs` - M68K instruction set
- `src-tauri/src/registers.rs` - CPU registers
- `src-tauri/src/assembler.rs` - Assembler integration
- `src-tauri/src/uart.rs` - UART/terminal support
- `src-tauri/src/cfcard.rs` - CompactFlash card emulation
- `src-tauri/src/bus.rs` - System bus

### Frontend (TypeScript/React)

- `src/lib/emulator-api.ts` - Frontend API wrapper
- `src/lib/emulator-store.ts` - Application state (Zustand)
- `src/lib/emulator-types.ts` - Shared type definitions
- `src/components/CodeEditor.tsx` - Assembly code editor
- `src/components/RegisterDisplay.tsx` - CPU register view
- `src/components/MemoryViewer.tsx` - Memory inspection
- `src/components/Toolbar.tsx` - Control buttons
- `src/components/StatusBar.tsx` - Status display
- `src/components/UartTerminal.tsx` - UART terminal
- `src/components/ControlPanel.tsx` - Control panel

---

## Tech Stack

- **Desktop Framework**: Tauri 2 (Rust backend + webview frontend)
- **Frontend**: React 19, TypeScript, Tailwind CSS v4
- **State Management**: Zustand
- **UI Components**: Radix UI primitives + Shadcn/ui
- **Code Editor**: CodeMirror 6 with M68K syntax highlighting
- **Build Tool**: Vite 7 (Rolldown-powered)
- **Testing**: Vitest + Testing Library

---

## Code Style

### Rust

- Run `cargo fmt` before committing
- Must pass `cargo clippy -- -D warnings`
- Public items require doc comments
- Unit tests in `#[cfg(test)] mod tests {}`
- No unexplained `#[allow(...)]` attributes

### TypeScript

- Strict TypeScript (no `any` types)
- ESLint with zero warnings tolerance
- Prettier formatting enforced
- Conventional Commits required
- Functional components + hooks only

---

## Project Structure

```
flux-32/
├── src-tauri/           # Rust backend
│   ├── src/            # Flat structure, no subdirectories
│   ├── test/           # Musashi test binaries (60 tests)
│   ├── rom/            # System ROM and examples
│   └── assets/         # Application assets
├── src/                # TypeScript/React frontend
│   ├── components/     # React components
│   │   └── ui/        # shadcn/ui primitives
│   ├── lib/           # Utilities and API layer
│   └── test/          # Test setup
├── scripts/            # Build and format scripts
└── AGENTS.md          # This file - Agent configuration
```

---

## ROM Examples

Located in `src-tauri/rom/examples/`:

- `hello.asm` - Hello World program
- `fizzbuzz.asm` - FizzBuzz implementation
- `idle.asm` - Idle loop

---

## MCP Servers Used

- **`tauri`** - For testing the running Tauri application
- **`context7`** - For fetching latest documentation

---

## Legacy Configuration Notes

The following files have been consolidated into this AGENTS.md:

- `CLAUDE.md` - Project context (merged here)
- `litellm.yaml` - Model configurations (merged here)
- `llms.txt` - Project overview (merged here)
- `.claude/settings.local.json` - Claude settings (merged here)
- `.agent/` directory - Agent rules, skills, and workflows (merged here)
- `proxy.bat` - Proxy configuration script (removed)

These legacy files have been removed. This AGENTS.md file is designed to work with any LLM provider and provides the best possible instructions to the agent.
