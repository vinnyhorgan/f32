# F32 - Flux32 Desktop App

This is the desktop GUI version of Flux32, a Motorola 68k educational playground environment.

## Project Structure

```
f32/
├── src/                      # Frontend (React + TypeScript)
│   ├── lib/
│   │   ├── emulator-types.ts # TypeScript types for emulator
│   │   ├── emulator-api.ts   # API wrapper for Tauri commands
│   │   └── utils.ts          # Utility functions
│   ├── components/
│   │   └── ui/               # shadcn/ui components
│   ├── App.tsx               # Main app component
│   └── main.tsx              # Entry point
│
├── src-tauri/                # Backend (Rust + Tauri)
│   ├── src/
│   │   ├── lib.rs            # Tauri commands and emulator wrapper
│   │   ├── addressing.rs     # M68K addressing modes
│   │   ├── assembler.rs      # Built-in M68K assembler
│   │   ├── bus.rs            # Memory bus architecture
│   │   ├── cfcard.rs         # CompactFlash card emulation
│   │   ├── cpu.rs            # M68K CPU core
│   │   ├── instructions.rs   # All M68K instructions
│   │   ├── memory.rs         # Memory model
│   │   ├── registers.rs      # Register file
│   │   ├── sbc.rs            # Single Board Computer integration
│   │   ├── uart.rs           # 16550 UART emulation
│   │   └── test_runner.rs    # Musashi test harness
│   ├── assets/
│   │   └── rom.bin           # Embedded system ROM
│   ├── rom/                  # ROM assembly source and examples
│   ├── test/                 # Musashi test binaries
│   └── Cargo.toml            # Rust dependencies
│
└── flux32/                   # Original CLI project (reference)
    └── src/                  # CLI source code
```

## Available Tauri Commands

The following commands are exposed from the Rust backend to the frontend:

- `emulator_init()` - Initialize a new emulator instance
- `emulator_step()` - Execute a single instruction step
- `emulator_reset()` - Reset the emulator to initial state
- `emulator_run()` - Run the emulator continuously
- `emulator_get_registers()` - Get the current CPU register state
- `emulator_read_byte(address)` - Read a byte from memory
- `emulator_write_byte(address, value)` - Write a byte to memory
- `emulator_assemble(code)` - Assemble M68K assembly code

## Development Setup

### Prerequisites
- Rust toolchain (stable)
- Node.js 18+
- pnpm or npm

### Installation

```bash
# Install dependencies
npm install

# Run development server
npm run dev

# Build the app
npm run build
```

### Development Workflow

1. **Backend (Rust) Development**
   - Modify files in `src-tauri/src/`
   - The emulator core is directly copied from the flux32 CLI project
   - All the M68K emulation code is already integrated

2. **Frontend (React) Development**
   - Use `src/lib/emulator-api.ts` to interact with the emulator
   - The `useEmulator()` hook provides state management
   - Build UI components using the existing shadcn/ui setup

3. **Adding New Tauri Commands**
   - Add the command function in `src-tauri/src/lib.rs`
   - Annotate with `#[tauri::command]`
   - Register in the `invoke_handler!` macro
   - Add TypeScript types in `src/lib/emulator-types.ts`
   - Add API wrapper methods in `src/lib/emulator-api.ts`

## Next Steps for GUI Development

1. **Register View Component**
   - Display all 8 data registers (D0-D7) and 7 address registers (A0-A6)
   - Show PC (Program Counter) and SR (Status Register)
   - Highlight changed registers after each step
   - Show flags: X, N, Z, V, C

2. **Memory View Component**
   - Hex dump display with ASCII representation
   - Scrollable memory window
   - Support for editing memory values
   - Go-to-address functionality

3. **Disassembly View Component**
   - Show disassembled instructions at current PC
   - Highlight current instruction
   - Support for setting breakpoints
   - Step-by-step execution highlighting

4. **Code Editor Component**
   - M68K assembly editor with syntax highlighting
   - Assemble button to compile code
   - Load binary into memory
   - Show assembly errors

5. **Terminal/UART Component**
   - Virtual UART output display
   - Input for sending characters to UART
   - Scrollback buffer
   - Clear terminal function

6. **Control Panel Component**
   - Step, Run, Reset buttons
   - Speed control
   - Breakpoint management
   - Load/save state

## Architecture Notes

- The emulator uses a `static mut` global for state management (simple for now)
- The SBC (Single Board Computer) wraps the CPU and peripherals
- Memory-mapped I/O for UART and CompactFlash
- ROM is embedded in the binary at compile time
- The emulator passes all 60 Musashi M68K tests

## Porting from CLI

The CLI code was originally designed for single-threaded execution. Key differences:

1. **Thread Safety**: The SBC uses `Rc<RefCell<>>` internally, which is wrapped in `Arc<Mutex<>>` for Tauri
2. **State Management**: CLI used direct struct access, Tauri uses command pattern
3. **UI**: CLI had terminal output, GUI will have React components

## Reference Material

- Original CLI project: `flux32/` directory
- Motorola 68k Programmer's Reference Manual
- Musashi test suite in `src-tauri/test/`
- ROM assembly source in `src-tauri/rom/`
