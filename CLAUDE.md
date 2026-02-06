# flux-32

## Project Overview

**flux-32** is an educational M68K (Motorola 68000) playground — a desktop GUI application
built with Tauri that includes a fully featured M68K emulator. The goal is to teach systems
programming from the ground up, inspired by Zachtronics and PICO-8. Users will learn from
basic instruction execution all the way up to writing simple preemptive operating systems.

**Current focus: Building the best possible GUI debugger and emulator experience.**

---

## Development Priority

**We are 100% focused on the GUI experience.** The emulator core (ported from the CLI version)
is solid and passing all Musashi tests. Current priorities:

1. **GUI polish** — Build an intuitive, responsive, and powerful visual debugger
2. **Frontend-backend integration** — Seamless communication between React UI and Rust emulator
3. **Test coverage** — Maintain and expand comprehensive tests as the primary guard rail
4. **Code quality** — Clean, readable, well-documented code that's a pleasure to work with

---

## Architecture Overview

This is a **Tauri desktop application** with two distinct parts:

### Frontend (TypeScript + React)
- **Location**: `src/`
- **Framework**: React 18 with TypeScript
- **Styling**: Tailwind CSS v4 with shadcn/ui components
- **State Management**: Zustand for global state
- **Build Tool**: Vite

### Backend (Rust)
- **Location**: `src-tauri/src/`
- **Framework**: Tauri v2
- **Core**: Complete M68K emulator (CPU, memory, peripherals, assembler)
- **Communication**: Tauri IPC commands (`invoke()`)

**The frontend and backend communicate exclusively through Tauri commands defined in
`src-tauri/src/lib.rs`.** All UI operations must go through this API layer.

---

## Non-Negotiable Rules

These are hard constraints. Do not deviate from them under any circumstances.

### 1. Minimal Dependencies (Backend)

The Rust backend (`src-tauri/Cargo.toml`) should remain as dependency-free as possible.
The core emulator modules have zero external dependencies beyond Tauri essentials.

**For the emulator core (cpu.rs, instructions.rs, memory.rs, etc.):**
- Write it yourself. The standard library is powerful — use it.
- If functionality genuinely exceeds ~200 lines AND a crate exists that fits perfectly,
  evaluate it seriously. The bar is very high.
- Prefer vendoring or pure-Rust implementations over pulling in transitive dependencies.

**For Tauri integration layer:**
- Tauri framework dependencies are necessary and acceptable
- `serde` and `serde_json` for IPC serialization are acceptable
- Keep it minimal — every dependency increases binary size and attack surface

### 2. Frontend Dependencies

The frontend uses React + Tailwind + shadcn/ui. New npm dependencies should be evaluated
carelessly:

- Prefer the existing ecosystem (Radix UI primitives, Lucide icons)
- Don't add UI component libraries — build with shadcn/ui components
- Keep the bundle size small

### 3. Flat `src-tauri/src/` Directory — No Subdirectories

All Rust source files live directly in `src-tauri/src/`. No nested modules via directories.
Module declarations in `lib.rs` point to sibling files only.

Prefer fewer, larger, well-organized files over many small ones. A file should be split only
when it genuinely becomes unwieldy (roughly 800+ lines of actual code).

### 4. Frontend Organization

The frontend uses a standard structure:
- `src/components/` — React components (grouped by feature with subdirectories)
- `src/lib/` — Utility modules, API clients, type definitions
- `src/App.tsx` — Root component

**Component guidelines:**
- Keep components focused and single-purpose
- Extract reusable UI patterns to `src/components/ui/` (shadcn/ui pattern)
- Use composition over complex prop drilling

---

## Code Style & Readability (Priority #1)

**Readability is the single most important quality of this codebase.** This project is
learning material. Every file should be a pleasure to read, understand, and debug.

### Rust Backend (General Principles)

- Write code as if the reader is a talented Rust developer who has never seen this project
- Stable, readable patterns > clever one-liners
- When two approaches are roughly equivalent in performance, pick the more readable one
- Avoid unnecessary abstractions. A simple `match` is almost always better than a trait
  object for this kind of project

### TypeScript Frontend (General Principles)

- Use TypeScript strictly — no `any` types without explicit justification
- Prefer functional components with hooks
- Keep components pure — side effects in hooks, not in render
- Use proper TypeScript types for all Tauri command responses

### Comments & Documentation

**Comments are not optional.**

- Every public Rust item (struct, enum, fn, mod) must have a doc comment (`///`)
- Every Tauri command must document its parameters, return values, and error conditions
- Complex React components should have JSDoc comments describing their purpose
- Complex logic blocks need inline comments explaining _why_, not just _what_
- For M68K instructions: each opcode should have a comment block with the instruction
  name, its effect on flags, and edge cases

### Naming

**Rust:**
- Be explicit: `execute_instruction` > `exec`, `AddressMode` > `AddrMode`
- Enum variants read naturally: `Instruction::Add`, `AddressMode::Immediate`
- M68K domain abbreviations are fine: `PC`, `SR`, `SP`

**TypeScript:**
- Use PascalCase for components: `RegisterDisplay`, `MemoryViewer`
- Use camelCase for functions and variables: `formatHexValue`, `cpuState`
- Types and interfaces in PascalCase: `CpuState`, `EmulatorResult`

### Formatting & Linting (Non-Negotiable)

**Rust:**
- Always run `cargo fmt` before committing
- Fix all `cargo clippy -- -D warnings` warnings
- No dead code — if it's not used, it doesn't belong
- No `#[allow(...)]` without a comment explaining why

**TypeScript:**
- Use ESLint and Prettier (configured in project)
- Run `npm run lint` before committing
- Fix all linting errors
- Format with Prettier (automatic on save in VSCode)

---

## Testing (Guard Rails for Humans and AI)

**Tests are the primary guard rail that keeps this project maintainable.**

### Test Strategy: Musashi Test Suite

The **Musashi M68K test suite** is the official standard for instruction correctness.
These are comprehensive binary test files that verify every implemented instruction.
The project currently passes **all 60 Musashi tests**.

Test binaries live in `src-tauri/test/*.bin` as black boxes — no source files, no assembly.
They communicate via memory-mapped I/O to signal pass/fail status.

**To run the Musashi test suite from the GUI:**
```bash
# Run via Tauri command (not yet implemented in GUI)
cargo test --package f32_lib
```

**Or run directly with the old CLI-style test runner:**
```bash
cd src-tauri
cargo test -- --test-runner
```

### Backend Tests (Rust)

- **Unit tests**: In the same file as the code, in a `#[cfg(test)] mod tests {}` block
- **Instruction tests**: Musashi test binaries in `src-tauri/test/*.bin` (60 tests)
- **Integration tests**: Could live in `src-tauri/tests/` if needed for end-to-end flows

**What to test:**
- Instructions: All tested via Musashi suite
- Core modules: Addressing, memory, registers, CPU state machine must have comprehensive
  unit tests covering all code paths, edge cases, and error conditions
- Tauri commands: Test command handlers with various inputs

### Frontend Tests (TypeScript)

Frontend testing should focus on:
- **Component tests**: Critical UI components (register display, memory viewer, etc.)
- **API integration**: Mock Tauri invoke calls to test error handling
- **User interactions**: Step/run/reset workflows, state updates

**Test framework**: Add testing setup (Vitest + React Testing Library) when needed.

### Test-Driven Development

**Write tests first.** When implementing a new feature:
1. Write failing tests defining expected behavior
2. Implement minimum code to make tests pass
3. Refactor for clarity while keeping tests green

This gives clear goals, prevents scope creep, and ensures test coverage from day one.

### Running Tests

```bash
# Rust backend tests
cd src-tauri
cargo test                           # Run all unit tests
cargo test -- --nocapture           # See stdout during tests
cargo clippy -- -D warnings         # Check for issues

# Frontend tests (when test setup is added)
npm test                            # Run frontend tests
npm run lint                        # Lint TypeScript/React code
```

---

## File Layout & Architecture

### Backend (`src-tauri/src/`)

All source files live directly in `src-tauri/src/` (flat structure):

| File              | Responsibility                                                                          |
| ----------------- | --------------------------------------------------------------------------------------- |
| `lib.rs`          | Tauri commands, IPC interface, emulator state management. The bridge to frontend.       |
| `main.rs`         | Entry point, Tauri builder setup, plugin initialization.                                |
| `cpu.rs`          | The M68K CPU core: registers, state, step logic, instruction dispatch.                  |
| `instructions.rs` | Implementation of all M68K instructions. Grouped logically with clear section comments. |
| `addressing.rs`   | All addressing mode resolution and encoding/decoding logic.                             |
| `memory.rs`       | Memory model: address space, read/write, MMIO hooks.                                    |
| `registers.rs`    | Register file definition, flag/status register helpers.                                 |
| `test_runner.rs`  | Musashi test binary loader and execution harness.                                       |
| `bus.rs`          | Memory bus architecture for SBC-compatible system.                                      |
| `uart.rs`         | 16550 UART emulation for serial I/O.                                                    |
| `cfcard.rs`       | CompactFlash card emulation (IDE/ATA in True IDE mode).                                 |
| `sbc.rs`          | Single Board Computer emulation tying CPU, bus, and peripherals together.               |
| `assembler.rs`    | Complete M68K assembler with macro processor and two-pass assembly.                     |

### Frontend (`src/`)

| Path                        | Responsibility                                                      |
| --------------------------- | ------------------------------------------------------------------- |
| `App.tsx`                   | Root component, application layout.                                 |
| `main.tsx`                  | React entry point, Tauri API import.                                |
| `lib/emulator-api.ts`       | Tauri command wrappers, typed IPC interface.                        |
| `lib/emulator-types.ts`     | TypeScript types matching Rust structures.                          |
| `lib/utils.ts`              | Utility functions (cn for classnames, etc.).                       |
| `components/ui/`            | shadcn/ui primitive components (button, input, etc.).               |
| `components/`               | Feature components (register view, memory viewer, disassembly, etc.). |

### Guiding Architectural Principles

**Backend (Rust):**
- The CPU is a clean state machine. `cpu.step()` fetches, decodes, executes, updates flags
- Addressing modes resolved in one place — instructions shouldn't understand their operands
- Keep memory model simple — bus abstraction handles routing to devices
- Peripheral emulation clearly isolated and well-tested
- Tauri commands are thin wrappers around emulator operations — no business logic in IPC layer

**Frontend (React):**
- Components communicate via props and state (Zustand for global state)
- All emulator operations go through `EmulatorAPI` class in `lib/emulator-api.ts`
- Keep UI responsive — use async/await properly, show loading states
- Keyboard shortcuts for debugger actions (step, run, reset)
- Real-time updates for register/memory displays during execution

---

## MCP Servers Available

This project has configured three MCP servers for Claude Code:

### 1. **tauri-mcp-server** (`hypothesi/tauri-mcp-server`)
**Purpose:** Direct interaction with the running Tauri app during development.

**Capabilities:**
- Connect to running app instance
- Inspect webview DOM (snapshots, find elements)
- Execute JavaScript in webview context
- Inspect computed CSS styles
- Read console/system logs
- Send keyboard/mouse events for testing UI interactions
- Manage windows (resize, list, info)
- Monitor and execute IPC commands

**Use for:**
- Testing the GUI without manual clicks
- Inspecting component state and styles
- Debugging IPC communication between frontend and backend
- Automated UI testing workflows

### 2. **sequential-thinking**
**Purpose:** Enhanced reasoning for complex problem-solving.

**Use for:**
- Breaking down complex architectural decisions
- Tracing through difficult emulator bugs
- Planning multi-step refactors

### 3. **context7**
**Purpose:** Enhanced context management for large codebases.

**Use for:**
- Maintaining context across multiple files
- Remembering patterns across the project

---

## Development Workflow

### Running the App

```bash
# Install dependencies (first time)
npm install

# Development mode (hot reload)
npm run tauri dev

# Production build
npm run tauri build

# Just frontend dev server
npm run dev
```

### Common Development Tasks

**Adding a new Tauri command:**
1. Add the function in `src-tauri/src/lib.rs` with `#[tauri::command]`
2. Add it to the `invoke_handler!` macro in `run()`
3. Add TypeScript wrapper in `src/lib/emulator-api.ts`
4. Add types to `src/lib/emulator-types.ts` if needed

**Adding a new UI component:**
1. Create component in `src/components/` or use shadcn/ui: `npx shadcn@latest add [component]`
2. Import and use in parent component or `App.tsx`
3. Style with Tailwind utility classes
4. Document with JSDoc if complex

**Testing emulator changes:**
1. Write unit test in `src-tauri/src/[module].rs`
2. Run `cargo test` in `src-tauri/`
3. Verify Musashi tests still pass
4. Update GUI if needed to expose new functionality

---

## Release Build Configuration

The release binary should be small and self-contained. `src-tauri/Cargo.toml` includes:

```toml
[profile.release]
opt-level = "s"           # Optimize for size
lto = true                # Full link-time optimization
codegen-units = 1         # Single codegen unit (helps LTO)
panic = "abort"           # No unwinding machinery
strip = true              # Strip debug symbols
```

**Goal:** `npm run tauri build` produces a small, fast desktop application.

---

## What NOT to Do

### Backend (Rust)
- Do not add dependencies without serious evaluation against the criteria above
- Do not create subdirectories inside `src-tauri/src/`
- Do not write "clever" code at the expense of clarity
- Do not leave TODOs without an associated comment explaining what's needed
- Do not implement features speculatively
- Do not ignore clippy warnings
- Do not commit code that hasn't been formatted with `cargo fmt`
- Do not skip writing tests for new code
- Do not optimize prematurely — get it correct and readable first
- Do not over-engineer

### Frontend (TypeScript/React)
- Do not add npm dependencies without evaluating necessity
- Do not bypass the `EmulatorAPI` layer — always use the typed wrappers
- Do not use `any` types — proper TypeScript typing is required
- Do not commit code that fails ESLint or Prettier checks
- Do not create overly complex components — break them down
- Do not put business logic in components — keep it in the API layer or custom hooks
- Do not ignore TypeScript errors
- Do not over-abstract — simple and correct beats flexible and complex

---

## Porting from CLI

This GUI version ports the complete CLI emulator codebase. Key differences from the original:

**Removed:**
- CLI debugger REPL (`cli.rs` — no longer needed)
- Command-line argument parsing

**Added:**
- Tauri IPC command layer in `lib.rs`
- TypeScript API wrappers in `src/lib/emulator-api.ts`
- React UI components for visual debugging
- Real-time state display and interaction

**Preserved:**
- All emulator core modules (CPU, memory, instructions, etc.)
- Musashi test suite compatibility
- Single-file, flat directory structure for backend
- Zero-dependency philosophy for the emulator core
