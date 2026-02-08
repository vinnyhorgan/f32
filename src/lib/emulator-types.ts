/**
 * Flux32 Emulator Type Definitions
 *
 * This file contains TypeScript types for the Flux32 M68K emulator.
 * These types correspond to the Tauri commands defined in src-tauri/src/lib.rs
 */

/**
 * CPU register state
 */
export interface CpuState {
  /** Data registers D0-D7 */
  d: number[];
  /** Address registers A0-A6 (A7 is SP) */
  a: number[];
  /** Program Counter */
  pc: number;
  /** Status Register */
  sr: number;
  /** User Stack Pointer */
  usp: number;
  /** Supervisor Stack Pointer */
  ssp: number;
}

/**
 * Emulator status information
 */
export interface EmulatorStatus {
  /** Whether the CPU is halted */
  halted: boolean;
  /** Total cycles executed */
  cycles: number;
  /** Cycles executed in last run */
  executed: number;
}

/**
 * Result type for emulator operations
 */
export type EmulatorResult<T> =
  | { status: "success"; data: T }
  | { status: "error"; error: string };

/**
 * Emulator command responses
 */
export interface EmulatorCommands {
  /** Initialize a new emulator instance */
  emulator_init: () => Promise<EmulatorResult<string>>;

  /** Execute a single instruction step */
  emulator_step: () => Promise<EmulatorResult<string>>;

  /** Reset the emulator to initial state */
  emulator_reset: () => Promise<EmulatorResult<string>>;

  /** Run the emulator continuously */
  emulator_run: () => Promise<EmulatorResult<string>>;

  /** Get the current CPU register state */
  emulator_get_registers: () => Promise<EmulatorResult<CpuState>>;

  /** Read a byte from memory at the given address */
  emulator_read_byte: (address: number) => Promise<EmulatorResult<number>>;

  /** Write a byte to memory at the given address */
  emulator_write_byte: (
    address: number,
    value: number,
  ) => Promise<EmulatorResult<null>>;

  /** Assemble M68K assembly code */
  emulator_assemble: (code: string) => Promise<EmulatorResult<number[]>>;
}

/**
 * Memory view options
 */
export interface MemoryViewOptions {
  /** Start address */
  address: number;
  /** Number of bytes to display */
  length: number;
  /** Whether to show as hex, decimal, or both */
  format: "hex" | "decimal" | "both";
}

/**
 * Disassembly result
 */
export interface DisassemblyLine {
  /** Address of the instruction */
  address: number;
  /** Opcode bytes in hex */
  bytes: string;
  /** Instruction mnemonic and operands */
  instruction: string;
}

/**
 * Breakpoint information
 */
export interface Breakpoint {
  /** Address of the breakpoint */
  address: number;
  /** Whether the breakpoint is enabled */
  enabled: boolean;
  /** Optional label/description */
  label?: string;
}
