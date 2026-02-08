# Flux-32 Codebase Audit Workflow

A comprehensive, systematic audit workflow for the flux-32 M68K emulator codebase. This workflow ensures code quality, consistency, security, and maintainability across the entire Tauri + React application.

---

## Overview

This audit workflow examines all aspects of the flux-32 codebase:

- **Backend**: Rust (Tauri v2)
- **Frontend**: TypeScript/React 19 + Tailwind v4 + shadcn/ui
- **Communication**: Tauri IPC commands
- **Testing**: Vitest (frontend) + Cargo test (backend)
- **Build**: Vite 7 + Cargo

---

## Phase 1: Project Structure & Configuration Audit

### 1.1 Root Configuration Files

**Audit Checklist:**

- [ ] `package.json` - Verify dependencies, scripts, and metadata
- [ ] `tsconfig.json` - Ensure strict TypeScript configuration
- [ ] `vite.config.ts` - Validate build configuration
- [ ] `vitest.config.ts` - Check test configuration
- [ ] `eslint.config.mjs` - Verify linting rules
- [ ] `.prettierrc` - Check formatting configuration
- [ ] `.editorconfig` - Ensure editor consistency
- [ ] `.gitignore` - Verify ignored patterns
- [ ] `commitlint.config.mjs` - Check commit message rules
- [ ] `.lintstagedrc.js` - Verify pre-commit hooks
- [ ] `AGENTS.md` - Ensure agent instructions are current

**Validation Commands:**

```bash
# Check package.json validity
cat package.json | jq empty

# Verify TypeScript config
npx tsc --showConfig

# Check ESLint config validity
npx eslint --print-config src/App.tsx

# Verify Prettier config
npx prettier --check .prettierrc
```

### 1.2 Tauri Configuration

**Audit Checklist:**

- [ ] `src-tauri/Cargo.toml` - Verify dependencies, features, and metadata
- [ ] `src-tauri/tauri.conf.json` - Validate app configuration
- [ ] `src-tauri/build.rs` - Check build script
- [ ] `src-tauri/capabilities/default.json` - Verify IPC capabilities
- [ ] `src-tauri/.gitignore` - Ensure proper exclusions

**Validation Commands:**

```bash
# Check Cargo.toml validity
cd src-tauri && cargo check --message-format=json

# Validate Tauri config
cd src-tauri && npx tauri info

# Check capabilities schema
cd src-tauri && cat capabilities/default.json | jq empty
```

### 1.3 Directory Structure

**Audit Checklist:**

- [ ] Verify flat structure in `src-tauri/src/` (no subdirectories)
- [ ] Check `src/components/` organization
- [ ] Verify `src/components/ui/` contains shadcn/ui primitives
- [ ] Ensure `src/lib/` contains utilities and API layer
- [ ] Check `src/test/` for test setup
- [ ] Verify `src-tauri/rom/` structure
- [ ] Check `src-tauri/test/` for Musashi test binaries
- [ ] Verify `src-tauri/assets/` structure
- [ ] Check `scripts/` directory contents
- [ ] Verify `.husky/` for Git hooks

**Validation Commands:**

```bash
# Check for subdirectories in src-tauri/src/
find src-tauri/src -mindepth 2 -type d

# List all component files
ls -la src/components/

# List all UI primitives
ls -la src/components/ui/

# List all lib files
ls -la src/lib/
```

---

## Phase 2: Rust Backend Audit (`src-tauri/src/`)

### 2.1 File Structure & Organization

**Audit Checklist:**

- [ ] Verify flat structure (no subdirectories)
- [ ] Check all `.rs` files are present:
  - [ ] `lib.rs` - Tauri commands and app setup
  - [ ] `main.rs` - Entry point
  - [ ] `cpu.rs` - CPU implementation
  - [ ] `memory.rs` - Memory management
  - [ ] `instructions.rs` - M68K instruction set
  - [ ] `registers.rs` - CPU registers
  - [ ] `assembler.rs` - Assembler integration
  - [ ] `uart.rs` - UART/terminal support
  - [ ] `cfcard.rs` - CompactFlash card emulation
  - [ ] `bus.rs` - System bus
  - [ ] `addressing.rs` - Addressing modes
  - [ ] `sbc.rs` - SBC implementation
  - [ ] `test_runner.rs` - Test runner

**Validation Commands:**

```bash
# List all Rust source files
ls -1 src-tauri/src/*.rs

# Check for subdirectories (should be empty)
find src-tauri/src -mindepth 2 -type d
```

### 2.2 Code Style & Formatting

**Audit Checklist:**

- [ ] Run `cargo fmt --check` - Verify formatting
- [ ] Run `cargo clippy -- -D warnings` - Verify no warnings
- [ ] Check for `#[allow(...)]` attributes - Ensure they're explained
- [ ] Verify all public items have doc comments (`///`)
- [ ] Check for proper error handling (`Result<T, String>`)
- [ ] Verify consistent naming conventions (snake_case for functions/vars, PascalCase for types)

**Validation Commands:**

```bash
# Check formatting
cd src-tauri && cargo fmt --check

# Run clippy
cd src-tauri && cargo clippy -- -D warnings

# Check for allow attributes
grep -r "#\[allow" src-tauri/src/

# Check for public items without docs
grep -r "^pub " src-tauri/src/ | grep -v "///"
```

### 2.3 `lib.rs` - Tauri Commands Audit

**Audit Checklist:**

- [ ] Verify all `#[tauri::command]` functions are documented
- [ ] Check command registration in `invoke_handler!`
- [ ] Verify all commands return `Result<T, String>`
- [ ] Check for proper error messages
- [ ] Verify command naming follows `snake_case`
- [ ] Check for proper parameter types
- [ ] Verify no unsafe code without justification
- [ ] Check for proper use of `Mutex` or `Arc` for shared state

**Key Commands to Audit:**

```rust
// Expected pattern:
/// Doc comment explaining the command
#[tauri::command]
fn command_name(param: Type) -> Result<ReturnType, String> {
    // Implementation
    Ok(result)
}
```

**Validation Commands:**

```bash
# List all Tauri commands
grep -n "#\[tauri::command\]" src-tauri/src/lib.rs

# Check command registration
grep -A 100 "invoke_handler!" src-tauri/src/lib.rs

# Check for Result return types
grep -A 2 "#\[tauri::command\]" src-tauri/src/lib.rs | grep "-> Result"
```

### 2.4 `cpu.rs` - CPU Implementation Audit

**Audit Checklist:**

- [ ] Verify CPU struct is well-documented
- [ ] Check for proper register management
- [ ] Verify instruction execution logic
- [ ] Check for proper flag handling (CCR, SR)
- [ ] Verify exception handling
- [ ] Check for proper state management
- [ ] Verify interrupt handling
- [ ] Check for proper cycle counting (if implemented)

**Key Areas to Examine:**

- CPU state structure
- Register access methods
- Instruction fetch/decode/execute cycle
- Flag update logic
- Exception handling
- Interrupt handling

**Validation Commands:**

```bash
# Check CPU struct definition
grep -A 20 "^pub struct Cpu" src-tauri/src/cpu.rs

# Check for unsafe code
grep -n "unsafe" src-tauri/src/cpu.rs

# Check for TODO/FIXME comments
grep -n "TODO\|FIXME" src-tauri/src/cpu.rs
```

### 2.5 `memory.rs` - Memory Management Audit

**Audit Checklist:**

- [ ] Verify memory size and layout
- [ ] Check for proper bounds checking
- [ ] Verify read/write operations
- [ ] Check for memory-mapped I/O handling
- [ ] Verify ROM/RAM separation
- [ ] Check for proper error handling on invalid access
- [ ] Verify alignment requirements (if any)
- [ ] Check for memory initialization

**Key Areas to Examine:**

- Memory structure
- Read/write methods
- Bounds checking
- Memory-mapped devices
- ROM/RAM regions

**Validation Commands:**

```bash
# Check memory struct
grep -A 20 "^pub struct Memory" src-tauri/src/memory.rs

# Check for bounds checking
grep -n "bounds\|range\|check" src-tauri/src/memory.rs

# Check for panic/unwrap usage
grep -n "panic!\|unwrap()" src-tauri/src/memory.rs
```

### 2.6 `instructions.rs` - Instruction Set Audit

**Audit Checklist:**

- [ ] Verify instruction coverage (M68K instruction set)
- [ ] Check for proper instruction decoding
- [ ] Verify addressing mode support
- [ ] Check for proper flag updates
- [ ] Verify instruction timing (if implemented)
- [ ] Check for proper operand handling
- [ ] Verify exception handling for illegal instructions
- [ ] Check for proper documentation of each instruction

**Key Areas to Examine:**

- Instruction enumeration/structs
- Decode logic
- Execute logic
- Flag update logic
- Addressing mode handling

**Validation Commands:**

```bash
# Count instruction implementations
grep -c "^pub fn" src-tauri/src/instructions.rs

# Check for undocumented instructions
grep -B 1 "^pub fn" src-tauri/src/instructions.rs | grep -v "///"

# Check for match statement completeness
grep -A 50 "match opcode" src-tauri/src/instructions.rs
```

### 2.7 `registers.rs` - Registers Audit

**Audit Checklist:**

- [ ] Verify register structure (D0-D7, A0-A7, PC, SR)
- [ ] Check for proper register access methods
- [ ] Verify stack pointer management (USP, SSP)
- [ ] Check for proper flag bit handling (CCR)
- [ ] Verify status register (SR) management
- [ ] Check for proper register initialization
- [ ] Verify register file organization

**Key Areas to Examine:**

- Register struct/enum
- Access methods
- Stack pointer handling
- Flag bit operations
- Status register operations

**Validation Commands:**

```bash
# Check register structure
grep -A 30 "^pub struct Registers" src-tauri/src/registers.rs

# Check for flag operations
grep -n "flag\|CCR\|SR" src-tauri/src/registers.rs
```

### 2.8 `assembler.rs` - Assembler Audit

**Audit Checklist:**

- [ ] Verify assembly parsing logic
- [ ] Check for proper instruction encoding
- [ ] Verify label handling
- [ ] Check for proper error reporting
- [ ] Verify symbol table management
- [ ] Check for proper handling of directives
- [ ] Verify output binary format
- [ ] Check for proper handling of constants

**Key Areas to Examine:**

- Parser implementation
- Instruction encoding
- Label resolution
- Error handling
- Symbol table

**Validation Commands:**

```bash
# Check assembler struct
grep -A 20 "^pub struct Assembler" src-tauri/src/assembler.rs

# Check for error handling
grep -n "Result\|Error" src-tauri/src/assembler.rs
```

### 2.9 `uart.rs` - UART Audit

**Audit Checklist:**

- [ ] Verify UART register layout
- [ ] Check for proper transmit/receive logic
- [ ] Verify interrupt handling
- [ ] Check for proper baud rate handling
- [ ] Verify buffer management
- [ ] Check for proper error handling
- [ ] Verify terminal integration
- [ ] Check for proper state management

**Key Areas to Examine:**

- UART register structure
- TX/RX logic
- Interrupt handling
- Buffer management

**Validation Commands:**

```bash
# Check UART struct
grep -A 20 "^pub struct Uart" src-tauri/src/uart.rs

# Check for interrupt handling
grep -n "interrupt" src-tauri/src/uart.rs
```

### 2.10 `cfcard.rs` - CompactFlash Audit

**Audit Checklist:**

- [ ] Verify CF card register layout
- [ ] Check for proper read/write operations
- [ ] Verify sector handling
- [ ] Check for proper error handling
- [ ] Verify card detection
- [ ] Check for proper state management
- [ ] Verify file system integration (if any)

**Key Areas to Examine:**

- CF card structure
- Register operations
- Sector operations
- Error handling

**Validation Commands:**

```bash
# Check CF card struct
grep -A 20 "^pub struct CfCard" src-tauri/src/cfcard.rs

# Check for sector operations
grep -n "sector" src-tauri/src/cfcard.rs
```

### 2.11 `bus.rs` - System Bus Audit

**Audit Checklist:**

- [ ] Verify bus architecture
- [ ] Check for proper device mapping
- [ ] Verify address decoding
- [ ] Check for proper arbitration (if needed)
- [ ] Verify interrupt routing
- [ ] Check for proper error handling
- [ ] Verify DMA support (if any)

**Key Areas to Examine:**

- Bus structure
- Device mapping
- Address decoding
- Interrupt routing

**Validation Commands:**

```bash
# Check bus struct
grep -A 20 "^pub struct Bus" src-tauri/src/bus.rs

# Check for device mapping
grep -n "map\|device" src-tauri/src/bus.rs
```

### 2.12 `addressing.rs` - Addressing Modes Audit

**Audit Checklist:**

- [ ] Verify all M68K addressing modes are implemented
- [ ] Check for proper effective address calculation
- [ ] Verify mode decoding
- [ ] Check for proper error handling
- [ ] Verify documentation for each mode

**Expected Addressing Modes:**

- Data register direct
- Address register direct
- Address register indirect
- Address register indirect with postincrement
- Address register indirect with predecrement
- Address register indirect with displacement
- Address register indirect with index
- Absolute short
- Absolute long
- PC with displacement
- PC with index
- Immediate

**Validation Commands:**

```bash
# Check for addressing mode implementations
grep -n "addressing\|mode" src-tauri/src/addressing.rs

# Check for effective address calculation
grep -n "effective\|ea" src-tauri/src/addressing.rs
```

### 2.13 `test_runner.rs` - Test Runner Audit

**Audit Checklist:**

- [ ] Verify test loading logic
- [ ] Check for proper test execution
- [ ] Verify result comparison
- [ ] Check for proper error reporting
- [ ] Verify Musashi test compatibility
- [ ] Check for proper cleanup

**Key Areas to Examine:**

- Test loading
- Test execution
- Result comparison
- Error reporting

**Validation Commands:**

```bash
# Check test runner struct
grep -A 20 "^pub struct TestRunner" src-tauri/src/test_runner.rs

# Check for test execution
grep -n "execute\|run" src-tauri/src/test_runner.rs
```

### 2.14 Rust Testing Audit

**Audit Checklist:**

- [ ] Verify all modules have `#[cfg(test)] mod tests {}`
- [ ] Run `cargo test` - Ensure all tests pass
- [ ] Run `cargo test -- --test-threads=1` - Verify sequential execution
- [ ] Check test coverage (if available)
- [ ] Verify Musashi tests pass (60 tests in `src-tauri/test/`)
- [ ] Check for proper test naming
- [ ] Verify test isolation

**Validation Commands:**

```bash
# Run all tests
cd src-tauri && cargo test

# Run tests with single thread
cd src-tauri && cargo test -- --test-threads=1

# List test binaries
ls -1 src-tauri/test/*.bin

# Check for test modules
grep -r "#\[cfg(test)\]" src-tauri/src/
```

### 2.15 Rust Dependencies Audit

**Audit Checklist:**

- [ ] Review `src-tauri/Cargo.toml` dependencies
- [ ] Check for outdated dependencies
- [ ] Verify no unnecessary dependencies
- [ ] Check for security vulnerabilities
- [ ] Verify license compatibility
- [ ] Check for dependency version pinning

**Validation Commands:**

```bash
# Check for outdated dependencies
cd src-tauri && cargo outdated

# Check for security vulnerabilities
cd src-tauri && cargo audit

# Check dependency tree
cd src-tauri && cargo tree
```

---

## Phase 3: TypeScript Frontend Audit (`src/`)

### 3.1 File Structure & Organization

**Audit Checklist:**

- [ ] Verify `src/components/` structure
- [ ] Check `src/components/ui/` for shadcn/ui primitives
- [ ] Verify `src/lib/` structure
- [ ] Check `src/test/` for test setup
- [ ] Verify all `.tsx` and `.ts` files are present

**Expected Files:**

- `src/App.tsx` - Main application component
- `src/main.tsx` - Entry point
- `src/index.css` - Global styles
- `src/components/CodeEditor.tsx` - Assembly code editor
- `src/components/RegisterDisplay.tsx` - CPU register view
- `src/components/MemoryViewer.tsx` - Memory inspection
- `src/components/Toolbar.tsx` - Control buttons
- `src/components/StatusBar.tsx` - Status display
- `src/components/UartTerminal.tsx` - UART terminal
- `src/components/ControlPanel.tsx` - Control panel
- `src/components/AppMenuBar.tsx` - Menu bar
- `src/lib/emulator-api.ts` - Frontend API wrapper
- `src/lib/emulator-store.ts` - Application state (Zustand)
- `src/lib/emulator-types.ts` - Shared type definitions
- `src/lib/m68k-lang.ts` - M68K language support
- `src/lib/editor-theme.ts` - Editor theme
- `src/lib/utils.ts` - Utility functions

**Validation Commands:**

```bash
# List all TypeScript files
find src -name "*.ts" -o -name "*.tsx"

# List all component files
ls -1 src/components/*.tsx

# List all UI primitives
ls -1 src/components/ui/*.tsx
```

### 3.2 TypeScript Configuration & Strictness

**Audit Checklist:**

- [ ] Verify `tsconfig.json` has `"strict": true`
- [ ] Check `"noImplicitAny": true`
- [ ] Verify `"strictNullChecks": true`
- [ ] Check `"noUnusedLocals": true`
- [ ] Verify `"noUnusedParameters": true`
- [ ] Check `"noImplicitReturns": true`
- [ ] Verify `"noFallthroughCasesInSwitch": true`
- [ ] Run `npx tsc --noEmit` - Verify no type errors

**Validation Commands:**

```bash
# Check TypeScript config
cat tsconfig.json

# Run TypeScript compiler
npx tsc --noEmit
```

### 3.3 ESLint Configuration & Quality

**Audit Checklist:**

- [ ] Verify `eslint.config.mjs` configuration
- [ ] Run `pnpm lint` - Ensure zero warnings
- [ ] Check for proper React rules
- [ ] Verify TypeScript-specific rules
- [ ] Check for accessibility rules
- [ ] Verify no `any` types in code

**Validation Commands:**

```bash
# Run ESLint
pnpm lint

# Check for any types
grep -r ": any" src/ --include="*.ts" --include="*.tsx"

# Check for eslint-disable comments
grep -r "eslint-disable" src/
```

### 3.4 Prettier Configuration & Formatting

**Audit Checklist:**

- [ ] Verify `.prettierrc` configuration
- [ ] Run `pnpm format` - Ensure proper formatting
- [ ] Check for consistent code style
- [ ] Verify line length limits
- [ ] Check for consistent quotes

**Validation Commands:**

```bash
# Check formatting
pnpm format

# Run format check
pnpm format:check
```

### 3.5 `emulator-api.ts` - API Layer Audit

**Audit Checklist:**

- [ ] Verify all Tauri commands are wrapped
- [ ] Check for proper `invoke()` usage
- [ ] Verify async/await pattern
- [ ] Check for proper error handling
- [ ] Verify type safety (no `any`)
- [ ] Check for proper return types
- [ ] Verify all methods are static
- [ ] Check for proper parameter types

**Expected Pattern:**

```typescript
static async commandName(param: Type): Promise<ReturnType> {
    return await invoke('command_name', { param });
}
```

**Validation Commands:**

```bash
# Check for invoke usage
grep -n "invoke" src/lib/emulator-api.ts

# Check for async methods
grep -n "static async" src/lib/emulator-api.ts

# Check for any types
grep -n ": any" src/lib/emulator-api.ts
```

### 3.6 `emulator-types.ts` - Type Definitions Audit

**Audit Checklist:**

- [ ] Verify types match Rust structs
- [ ] Check for proper interface definitions
- [ ] Verify type exports
- [ ] Check for proper enum definitions
- [ ] Verify no `any` types
- [ ] Check for proper documentation
- [ ] Verify type consistency across files

**Key Types to Verify:**

- CPU state types
- Register types
- Memory types
- Instruction types
- UART types
- CF card types
- Bus types

**Validation Commands:**

```bash
# Check for interface definitions
grep -n "^export interface" src/lib/emulator-types.ts

# Check for type definitions
grep -n "^export type" src/lib/emulator-types.ts

# Check for any types
grep -n ": any" src/lib/emulator-types.ts
```

### 3.7 `emulator-store.ts` - State Management Audit

**Audit Checklist:**

- [ ] Verify Zustand store structure
- [ ] Check for proper state initialization
- [ ] Verify action definitions
- [ ] Check for proper selectors
- [ ] Verify no `any` types
- [ ] Check for proper TypeScript typing
- [ ] Verify state immutability
- [ ] Check for proper middleware (if any)

**Expected Pattern:**

```typescript
interface EmulatorState {
  // State properties
  cpuState: CpuState;
  memory: MemoryState;
  // ...
}

interface EmulatorActions {
  // Actions
  setCpuState: (state: CpuState) => void;
  // ...
}

type EmulatorStore = EmulatorState & EmulatorActions;
```

**Validation Commands:**

```bash
# Check store structure
grep -n "interface.*State" src/lib/emulator-store.ts

# Check for actions
grep -n "interface.*Actions" src/lib/emulator-store.ts

# Check for any types
grep -n ": any" src/lib/emulator-store.ts
```

### 3.8 React Components Audit

#### 3.8.1 `App.tsx` - Main Component Audit

**Audit Checklist:**

- [ ] Verify proper component structure
- [ ] Check for proper hooks usage
- [ ] Verify no `any` types
- [ ] Check for proper error boundaries
- [ ] Verify proper TypeScript typing
- [ ] Check for proper accessibility
- [ ] Verify proper state management usage

**Validation Commands:**

```bash
# Check for any types
grep -n ": any" src/App.tsx

# Check for hooks usage
grep -n "use" src/App.tsx
```

#### 3.8.2 `CodeEditor.tsx` - Code Editor Audit

**Audit Checklist:**

- [ ] Verify CodeMirror 6 integration
- [ ] Check for M68K syntax highlighting
- [ ] Verify proper editor configuration
- [ ] Check for proper event handling
- [ ] Verify no `any` types
- [ ] Check for proper TypeScript typing
- [ ] Verify proper accessibility
- [ ] Check for proper performance (debouncing, etc.)

**Validation Commands:**

```bash
# Check for CodeMirror imports
grep -n "codemirror" src/components/CodeEditor.tsx

# Check for any types
grep -n ": any" src/components/CodeEditor.tsx
```

#### 3.8.3 `RegisterDisplay.tsx` - Register View Audit

**Audit Checklist:**

- [ ] Verify proper register display
- [ ] Check for proper state updates
- [ ] Verify no `any` types
- [ ] Check for proper TypeScript typing
- [ ] Verify proper accessibility
- [ ] Check for proper formatting (hex display)

**Validation Commands:**

```bash
# Check for any types
grep -n ": any" src/components/RegisterDisplay.tsx

# Check for hex formatting
grep -n "toString(16)\|0x" src/components/RegisterDisplay.tsx
```

#### 3.8.4 `MemoryViewer.tsx` - Memory Inspection Audit

**Audit Checklist:**

- [ ] Verify proper memory display
- [ ] Check for proper pagination
- [ ] Verify no `any` types
- [ ] Check for proper TypeScript typing
- [ ] Verify proper accessibility
- [ ] Check for proper hex display
- [ ] Verify proper scrolling

**Validation Commands:**

```bash
# Check for any types
grep -n ": any" src/components/MemoryViewer.tsx

# Check for hex formatting
grep -n "toString(16)\|0x" src/components/MemoryViewer.tsx
```

#### 3.8.5 `Toolbar.tsx` - Control Buttons Audit

**Audit Checklist:**

- [ ] Verify proper button layout
- [ ] Check for proper event handlers
- [ ] Verify no `any` types
- [ ] Check for proper TypeScript typing
- [ ] Verify proper accessibility (aria labels)
- [ ] Check for proper disabled states

**Validation Commands:**

```bash
# Check for any types
grep -n ": any" src/components/Toolbar.tsx

# Check for aria labels
grep -n "aria-" src/components/Toolbar.tsx
```

#### 3.8.6 `StatusBar.tsx` - Status Display Audit

**Audit Checklist:**

- [ ] Verify proper status display
- [ ] Check for proper state updates
- [ ] Verify no `any` types
- [ ] Check for proper TypeScript typing
- [ ] Verify proper accessibility

**Validation Commands:**

```bash
# Check for any types
grep -n ": any" src/components/StatusBar.tsx
```

#### 3.8.7 `UartTerminal.tsx` - UART Terminal Audit

**Audit Checklist:**

- [ ] Verify proper terminal display
- [ ] Check for proper scrolling
- [ ] Verify no `any` types
- [ ] Check for proper TypeScript typing
- [ ] Verify proper accessibility
- [ ] Check for proper input handling

**Validation Commands:**

```bash
# Check for any types
grep -n ": any" src/components/UartTerminal.tsx
```

#### 3.8.8 `ControlPanel.tsx` - Control Panel Audit

**Audit Checklist:**

- [ ] Verify proper control layout
- [ ] Check for proper event handlers
- [ ] Verify no `any` types
- [ ] Check for proper TypeScript typing
- [ ] Verify proper accessibility

**Validation Commands:**

```bash
# Check for any types
grep -n ": any" src/components/ControlPanel.tsx
```

#### 3.8.9 `AppMenuBar.tsx` - Menu Bar Audit

**Audit Checklist:**

- [ ] Verify proper menu structure
- [ ] Check for proper menu items
- [ ] Verify no `any` types
- [ ] Check for proper TypeScript typing
- [ ] Verify proper accessibility
- [ ] Check for proper keyboard shortcuts

**Validation Commands:**

```bash
# Check for any types
grep -n ": any" src/components/AppMenuBar.tsx

# Check for keyboard shortcuts
grep -n "shortcut\|hotkey" src/components/AppMenuBar.tsx
```

### 3.9 shadcn/ui Components Audit

**Audit Checklist:**

- [ ] Verify all UI primitives are present:
  - [ ] `badge.tsx`
  - [ ] `button.tsx`
  - [ ] `dropdown-menu.tsx`
  - [ ] `menubar.tsx`
  - [ ] `scroll-area.tsx`
  - [ ] `separator.tsx`
  - [ ] `tabs.tsx`
  - [ ] `tooltip.tsx`
- [ ] Check for proper Radix UI usage
- [ ] Verify proper Tailwind classes
- [ ] Verify no `any` types
- [ ] Check for proper TypeScript typing
- [ ] Verify proper accessibility

**Validation Commands:**

```bash
# List all UI primitives
ls -1 src/components/ui/*.tsx

# Check for any types
grep -r ": any" src/components/ui/

# Check for Radix imports
grep -r "@radix-ui" src/components/ui/
```

### 3.10 Frontend Testing Audit

**Audit Checklist:**

- [ ] Verify test files exist:
  - [ ] `src/App.test.tsx`
  - [ ] `src/components/StatusBar.test.tsx`
  - [ ] `src/components/Toolbar.test.tsx`
  - [ ] `src/lib/emulator-api.test.ts`
  - [ ] `src/lib/emulator-store.test.ts`
- [ ] Run `pnpm test` - Ensure all tests pass
- [ ] Run `pnpm test:run` - Ensure single execution works
- [ ] Check test coverage (if available)
- [ ] Verify proper test setup in `src/test/setup.ts`
- [ ] Check for proper use of Testing Library
- [ ] Verify test isolation

**Validation Commands:**

```bash
# Run tests
pnpm test

# Run tests once
pnpm test:run

# List test files
find src -name "*.test.ts" -o -name "*.test.tsx"

# Check test setup
cat src/test/setup.ts
```

### 3.11 Frontend Dependencies Audit

**Audit Checklist:**

- [ ] Review `package.json` dependencies
- [ ] Check for outdated dependencies
- [ ] Verify no unnecessary dependencies
- [ ] Check for security vulnerabilities
- [ ] Verify license compatibility
- [ ] Check for peer dependencies

**Validation Commands:**

```bash
# Check for outdated dependencies
pnpm outdated

# Check for security vulnerabilities
pnpm audit

# Check dependency tree
pnpm why <package-name>
```

---

## Phase 4: IPC Communication Audit

### 4.1 Command Registration Audit

**Audit Checklist:**

- [ ] Verify all commands are registered in `invoke_handler!`
- [ ] Check for command naming consistency
- [ ] Verify no duplicate commands
- [ ] Check for proper command availability in capabilities

**Validation Commands:**

```bash
# Check invoke_handler registration
grep -A 100 "invoke_handler!" src-tauri/src/lib.rs

# Check capabilities
cat src-tauri/capabilities/default.json | jq .commands
```

### 4.2 Type Synchronization Audit

**Audit Checklist:**

- [ ] Verify Rust structs match TypeScript interfaces
- [ ] Check for consistent field names
- [ ] Verify consistent field types
- [ ] Check for consistent enum variants
- [ ] Verify no type mismatches

**Key Types to Verify:**

- CPU state
- Registers
- Memory state
- Instruction results
- UART state
- CF card state

**Validation Commands:**

```bash
# Compare Rust and TypeScript types
# (Manual verification required)
```

### 4.3 Error Handling Audit

**Audit Checklist:**

- [ ] Verify all commands return `Result<T, String>`
- [ ] Check for proper error messages
- [ ] Verify frontend handles errors properly
- [ ] Check for proper error propagation
- [ ] Verify user-friendly error messages

**Validation Commands:**

```bash
# Check for Result return types
grep -A 2 "#\[tauri::command\]" src-tauri/src/lib.rs | grep "-> Result"

# Check for error handling in frontend
grep -n "catch\|error" src/lib/emulator-api.ts
```

---

## Phase 5: ROM & Examples Audit

### 5.1 ROM Structure Audit

**Audit Checklist:**

- [ ] Verify `src-tauri/rom/` structure
- [ ] Check for proper include files:
  - [ ] `app.inc`
  - [ ] `cfcard.inc`
  - [ ] `flux32.inc`
  - [ ] `macros.inc`
  - [ ] `memory.inc`
  - [ ] `syscalls.inc`
  - [ ] `uart.inc`
- [ ] Verify `rom.asm` is properly structured
- [ ] Check for proper syscall definitions
- [ ] Verify proper memory layout

**Validation Commands:**

```bash
# List ROM files
ls -1 src-tauri/rom/

# Check for include directives
grep -n "include" src-tauri/rom/rom.asm
```

### 5.2 ROM Examples Audit

**Audit Checklist:**

- [ ] Verify examples exist:
  - [ ] `hello.asm`
  - [ ] `fizzbuzz.asm`
  - [ ] `idle.asm`
- [ ] Check for proper assembly syntax
- [ ] Verify examples compile
- [ ] Check for proper documentation
- [ ] Verify examples work in emulator

**Validation Commands:**

```bash
# List examples
ls -1 src-tauri/rom/examples/

# Check example syntax
cat src-tauri/rom/examples/hello.asm
```

---

## Phase 6: Build & Release Audit

### 6.1 Build Configuration Audit

**Audit Checklist:**

- [ ] Verify `vite.config.ts` configuration
- [ ] Check for proper build optimization
- [ [ ] Verify proper asset handling
- [ ] Check for proper environment variables
- [ ] Verify `src-tauri/build.rs` is correct
- [ ] Check for proper Tauri build configuration

**Validation Commands:**

```bash
# Check Vite config
cat vite.config.ts

# Check Tauri build script
cat src-tauri/build.rs

# Try building
pnpm tauri build
```

### 6.2 Release Audit

**Audit Checklist:**

- [ ] Verify release configuration
- [ ] Check for proper versioning
- [ ] Verify proper icon assets
- [ ] Check for proper signing (if applicable)
- [ ] Verify proper package metadata

**Validation Commands:**

```bash
# Check Tauri config
cat src-tauri/tauri.conf.json

# Check version
grep -n "version" package.json src-tauri/Cargo.toml
```

---

## Phase 7: Documentation Audit

### 7.1 Code Documentation Audit

**Rust Documentation:**

- [ ] Verify all public items have `///` doc comments
- [ ] Check for proper `#[doc]` attributes
- [ ] Verify documentation examples
- [ ] Check for proper module documentation

**TypeScript Documentation:**

- [ ] Verify complex functions have JSDoc comments
- [ ] Check for proper type documentation
- [ ] Verify component documentation

**Validation Commands:**

```bash
# Check for undocumented public items in Rust
grep -B 1 "^pub " src-tauri/src/*.rs | grep -v "///"

# Check for JSDoc in TypeScript
grep -B 1 "^export\|^function" src/lib/*.ts | grep -v "\/\*\*"
```

### 7.2 README & Documentation Files Audit

**Audit Checklist:**

- [ ] Verify `AGENTS.md` is current
- [ ] Check `src-tauri/rom/README.md`
- [ ] Check `src-tauri/rom/examples/README.md`
- [ ] Verify all documentation is accurate
- [ ] Check for proper formatting

**Validation Commands:**

```bash
# List documentation files
find . -name "README.md" -o -name "*.md"

# Check documentation links
grep -r "\.md" AGENTS.md
```

---

## Phase 8: Security Audit

### 8.1 Dependency Security Audit

**Rust Dependencies:**

- [ ] Run `cargo audit` - Check for vulnerabilities
- [ ] Review `src-tauri/Cargo.toml` dependencies
- [ ] Check for outdated dependencies

**TypeScript Dependencies:**

- [ ] Run `pnpm audit` - Check for vulnerabilities
- [ ] Review `package.json` dependencies
- [ ] Check for outdated dependencies

**Validation Commands:**

```bash
# Rust security audit
cd src-tauri && cargo audit

# TypeScript security audit
pnpm audit
```

### 8.2 Code Security Audit

**Rust Security:**

- [ ] Check for unsafe code - ensure it's justified
- [ ] Verify proper bounds checking
- [ ] Check for proper error handling
- [ ] Verify no hardcoded secrets

**TypeScript Security:**

- [ ] Check for unsafe patterns (eval, etc.)
- [ ] Verify proper input validation
- [ ] Check for proper error handling
- [ ] Verify no hardcoded secrets

**Validation Commands:**

```bash
# Check for unsafe code in Rust
grep -rn "unsafe" src-tauri/src/

# Check for eval in TypeScript
grep -rn "eval" src/
```

---

## Phase 9: Performance Audit

### 9.1 Backend Performance Audit

**Audit Checklist:**

- [ ] Check for unnecessary allocations
- [ ] Verify proper use of references
- [ ] Check for efficient data structures
- [ ] Verify no unnecessary clones
- [ ] Check for proper async handling (if any)

**Validation Commands:**

```bash
# Check for clone usage
grep -rn "\.clone()" src-tauri/src/

# Check for unnecessary allocations
grep -rn "String::new\|Vec::new" src-tauri/src/
```

### 9.2 Frontend Performance Audit

**Audit Checklist:**

- [ ] Check for unnecessary re-renders
- [ ] Verify proper memoization (useMemo, useCallback)
- [ ] Check for proper key usage in lists
- [ ] Verify no large bundle sizes
- [ ] Check for proper lazy loading (if any)

**Validation Commands:**

```bash
# Check for useMemo usage
grep -rn "useMemo" src/

# Check for useCallback usage
grep -rn "useCallback" src/

# Check bundle size
pnpm build
```

---

## Phase 10: Accessibility Audit

### 10.1 UI Accessibility Audit

**Audit Checklist:**

- [ ] Verify proper ARIA labels
- [ ] Check for keyboard navigation
- [ ] Verify proper focus management
- [ ] Check for proper color contrast
- [ ] Verify proper semantic HTML

**Validation Commands:**

```bash
# Check for ARIA labels
grep -rn "aria-" src/components/

# Check for semantic HTML
grep -rn "<button\|<input\|<label" src/components/
```

---

## Phase 11: Integration Testing Audit

### 11.1 End-to-End Testing Audit

**Audit Checklist:**

- [ ] Verify Tauri MCP server can connect
- [ ] Check for proper driver_session usage
- [ ] Verify webview tools work
- [ ] Check for proper IPC monitoring
- [ ] Verify proper state verification

**Validation Commands:**

```bash
# Start Tauri app
pnpm tauri dev

# Connect via MCP
# (Use Tauri MCP tools)
```

---

## Phase 12: Final Validation

### 12.1 Quality Check

**Audit Checklist:**

- [ ] Run `pnpm check` - Full quality check
- [ ] Run `pnpm lint` - ESLint + TypeScript check
- [ ] Run `pnpm format` - Prettier + Taplo formatting
- [ ] Run `pnpm test` - Frontend tests
- [ ] Run `cd src-tauri && cargo test -- --test-threads=1` - Backend tests
- [ ] Run `pnpm tauri build` - Build verification

**Validation Commands:**

```bash
# Full quality check
pnpm check

# Build verification
pnpm tauri build
```

### 12.2 Manual Verification

**Audit Checklist:**

- [ ] Launch application and verify UI
- [ ] Test all major features
- [ ] Verify code editor works
- [ ] Verify register display updates
- [ ] Verify memory viewer works
- [ ] Verify UART terminal works
- [ ] Verify control buttons work
- [ ] Verify menu bar works
- [ ] Test ROM examples
- [ ] Verify error handling

---

## Audit Report Template

After completing the audit, create a report with the following structure:

```markdown
# Flux-32 Codebase Audit Report

**Date:** [Date]
**Auditor:** [Name]
**Scope:** Full codebase audit

## Executive Summary

[Brief overview of audit findings]

## Phase Results

### Phase 1: Project Structure & Configuration

- [ ] Passed
- [ ] Issues Found: [List issues]

### Phase 2: Rust Backend Audit

- [ ] Passed
- [ ] Issues Found: [List issues]

[Continue for all phases...]

## Critical Issues

1. [Issue 1]
2. [Issue 2]
   ...

## High Priority Issues

1. [Issue 1]
2. [Issue 2]
   ...

## Medium Priority Issues

1. [Issue 1]
2. [Issue 2]
   ...

## Low Priority Issues

1. [Issue 1]
2. [Issue 2]
   ...

## Recommendations

1. [Recommendation 1]
2. [Recommendation 2]
   ...

## Conclusion

[Summary and next steps]
```

---

## Quick Audit Commands

For a quick audit, run these commands:

```bash
# Full quality check
pnpm check

# Rust backend check
cd src-tauri && cargo fmt --check && cargo clippy -- -D warnings && cargo test -- --test-threads=1

# Frontend check
pnpm lint && pnpm format:check && pnpm test

# Security check
pnpm audit && cd src-tauri && cargo audit

# Build check
pnpm tauri build
```

---

## Notes

- This workflow is designed to be thorough and systematic
- Each phase should be completed before moving to the next
- Document all findings in the audit report
- Prioritize issues based on severity
- Use the Tauri MCP server for integration testing
- Always verify that the application builds and runs correctly
