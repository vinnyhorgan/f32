/**
 * Flux32 Emulator API
 *
 * This file provides a clean TypeScript API for interacting with the Flux32
 * M68K emulator through Tauri commands.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  CpuState,
  EmulatorResult,
  EmulatorStatus,
  MemoryViewOptions,
} from "./emulator-types";

/**
 * Flux32 Emulator API class
 *
 * Provides methods to interact with the M68K emulator backend.
 * All methods return promises that resolve to the operation result.
 */
export class EmulatorAPI {
  /**
   * Initialize a new emulator instance
   */
  static async init(): Promise<EmulatorResult<string>> {
    try {
      const result = await invoke<string>("emulator_init");
      return { status: "success", data: result };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Execute a single instruction step
   */
  static async step(): Promise<EmulatorResult<string>> {
    try {
      const result = await invoke<string>("emulator_step");
      return { status: "success", data: result };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Reset the emulator to initial state
   */
  static async reset(): Promise<EmulatorResult<string>> {
    try {
      const result = await invoke<string>("emulator_reset");
      return { status: "success", data: result };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Run the emulator continuously
   * @param maxCycles Maximum number of cycles to execute (default: 100000)
   */
  static async run(maxCycles?: number): Promise<EmulatorResult<EmulatorStatus>> {
    try {
      const result = await invoke<EmulatorStatus>("emulator_run", {
        maxCycles,
      });
      return { status: "success", data: result };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Get emulator status (halted state, cycle count)
   */
  static async getStatus(): Promise<EmulatorResult<EmulatorStatus>> {
    try {
      const result = await invoke<EmulatorStatus>("emulator_get_status");
      return { status: "success", data: result };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Get the current CPU register state
   */
  static async getRegisters(): Promise<EmulatorResult<CpuState>> {
    try {
      const result = await invoke<CpuState>("emulator_get_registers");
      return { status: "success", data: result };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Read a byte from memory at the given address
   */
  static async readByte(address: number): Promise<EmulatorResult<number>> {
    try {
      const result = await invoke<number>("emulator_read_byte", { address });
      return { status: "success", data: result };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Read multiple bytes from memory
   */
  static async readMemory(
    address: number,
    length: number,
  ): Promise<EmulatorResult<number[]>> {
    try {
      const result = await invoke<number[]>("emulator_read_memory", {
        address,
        length,
      });
      return { status: "success", data: result };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Write a byte to memory at the given address
   */
  static async writeByte(
    address: number,
    value: number,
  ): Promise<EmulatorResult<null>> {
    try {
      await invoke("emulator_write_byte", { address, value });
      return { status: "success", data: null };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Assemble M68K assembly code
   */
  static async assemble(code: string): Promise<EmulatorResult<number[]>> {
    try {
      const result = await invoke<number[]>("emulator_assemble", { code });
      return { status: "success", data: result };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Assemble code, load into RAM, and start execution
   */
  static async assembleAndLoad(code: string): Promise<EmulatorResult<string>> {
    try {
      const result = await invoke<string>("emulator_assemble_and_load", { code });
      return { status: "success", data: result };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Read UART output (drain TX buffer)
   */
  static async readUart(): Promise<EmulatorResult<number[]>> {
    try {
      const result = await invoke<number[]>("emulator_read_uart");
      return { status: "success", data: result };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Write a character to UART RX (simulate keyboard input)
   */
  static async writeUart(byte: number): Promise<EmulatorResult<null>> {
    try {
      await invoke("emulator_write_uart", { byte });
      return { status: "success", data: null };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Get LED state
   */
  static async getLed(): Promise<EmulatorResult<boolean>> {
    try {
      const result = await invoke<boolean>("emulator_get_led");
      return { status: "success", data: result };
    } catch (error) {
      return {
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Format a memory view for display
   */
  static formatMemoryView(
    bytes: number[],
    options: MemoryViewOptions,
  ): string[] {
    const lines: string[] = [];
    const bytesPerLine = 16;

    for (let i = 0; i < bytes.length; i += bytesPerLine) {
      const address = options.address + i;
      const lineBytes = bytes.slice(i, i + bytesPerLine);
      const hexBytes = lineBytes
        .map((b) => b.toString(16).toLowerCase().padStart(2, "0"))
        .join(" ");
      const asciiBytes = lineBytes
        .map((b) => (b >= 32 && b < 127 ? String.fromCharCode(b) : "."))
        .join("");

      lines.push(
        `${address.toString(16).toLowerCase().padStart(6, "0")}:  ${hexBytes.padEnd(bytesPerLine * 3 - 1, " ")}  |${asciiBytes}|`,
      );
    }

    return lines;
  }
}
