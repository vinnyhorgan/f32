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
   */
  static async run(): Promise<EmulatorResult<string>> {
    try {
      const result = await invoke<string>("emulator_run");
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
      const bytes: number[] = [];
      for (let i = 0; i < length; i++) {
        const result = await invoke<number>("emulator_read_byte", {
          address: address + i,
        });
        bytes.push(result);
      }
      return { status: "success", data: bytes };
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
        .map((b) => b.toString(16).toUpperCase().padStart(2, "0"))
        .join(" ");
      const asciiBytes = lineBytes
        .map((b) => (b >= 32 && b < 127 ? String.fromCharCode(b) : "."))
        .join("");

      lines.push(
        `${address.toString(16).toUpperCase().padStart(6, "0")}:  ${hexBytes.padEnd(bytesPerLine * 3 - 1, " ")}  |${asciiBytes}|`,
      );
    }

    return lines;
  }
}

/**
 * React hook for emulator state management
 *
 * This hook provides a convenient way to manage emulator state in React components.
 */
export function useEmulator() {
  const [isInitialized, setIsInitialized] = React.useState(false);
  const [cpuState, setCpuState] = React.useState<CpuState | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  const init = React.useCallback(async () => {
    const result = await EmulatorAPI.init();
    if (result.status === "success") {
      setIsInitialized(true);
      setError(null);
    } else {
      setError(result.error);
    }
    return result;
  }, []);

  const step = React.useCallback(async () => {
    const result = await EmulatorAPI.step();
    if (result.status === "error") {
      setError(result.error);
    } else {
      setError(null);
    }
    return result;
  }, []);

  const reset = React.useCallback(async () => {
    const result = await EmulatorAPI.reset();
    if (result.status === "success") {
      setCpuState(null);
      setError(null);
    } else {
      setError(result.error);
    }
    return result;
  }, []);

  const getRegisters = React.useCallback(async () => {
    const result = await EmulatorAPI.getRegisters();
    if (result.status === "success") {
      setCpuState(result.data);
      setError(null);
    } else {
      setError(result.error);
    }
    return result;
  }, []);

  return {
    isInitialized,
    cpuState,
    error,
    init,
    step,
    reset,
    getRegisters,
    api: EmulatorAPI,
  };
}

import React from "react";
