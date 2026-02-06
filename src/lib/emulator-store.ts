/**
 * Emulator Store (Zustand)
 *
 * Global state management for the M68K emulator.
 * Handles CPU state, execution control, memory, UART, and assembly.
 */

import { create } from "zustand";
import type { CpuState, EmulatorStatus } from "./emulator-types";
import { EmulatorAPI } from "./emulator-api";

/**
 * Emulator store state
 */
interface EmulatorState {
  /** Whether the emulator has been initialized */
  initialized: boolean;
  /** Current CPU register state */
  cpuState: CpuState | null;
  /** Emulator status (halted, cycles) */
  status: EmulatorStatus | null;
  /** Current error message, if any */
  error: string | null;
  /** Whether an operation is in progress */
  loading: boolean;
  /** Memory viewer address */
  memoryAddress: number;
  /** UART output text */
  uartOutput: string;
  /** LED state */
  ledState: boolean;
  /** Assembly source code */
  sourceCode: string;
  /** Assembly errors */
  assemblyError: string | null;
}

/**
 * Emulator store actions
 */
interface EmulatorActions {
  /** Initialize the emulator */
  init: () => Promise<void>;
  /** Execute one instruction step */
  step: () => Promise<void>;
  /** Run the emulator for max cycles */
  run: (maxCycles?: number) => Promise<void>;
  /** Reset the emulator */
  reset: () => Promise<void>;
  /** Refresh CPU state from backend */
  refresh: () => Promise<void>;
  /** Clear error message */
  clearError: () => void;
  /** Set memory viewer address */
  setMemoryAddress: (address: number) => void;
  /** Poll UART output */
  pollUart: () => Promise<void>;
  /** Send a character to UART */
  sendUartChar: (byte: number) => Promise<void>;
  /** Set source code */
  setSourceCode: (code: string) => void;
  /** Assemble and load the current source code */
  assembleAndRun: () => Promise<void>;
  /** Clear UART output */
  clearUart: () => void;
}

/**
 * Combined store type
 */
type EmulatorStore = EmulatorState & EmulatorActions;

/**
 * Create the emulator store
 */
export const useEmulatorStore = create<EmulatorStore>((set, get) => ({
  // Initial state
  initialized: false,
  cpuState: null,
  status: null,
  error: null,
  loading: false,
  memoryAddress: 0,
  uartOutput: "",
  ledState: false,
  sourceCode: "",
  assemblyError: null,

  // Initialize the emulator
  init: async () => {
    set({ loading: true, error: null });
    try {
      const result = await EmulatorAPI.init();
      if (result.status === "success") {
        set({ initialized: true });
        // Load initial CPU state
        await get().refresh();
      } else {
        set({ error: result.error });
      }
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
      });
    } finally {
      set({ loading: false });
    }
  },

  // Execute one instruction step
  step: async () => {
    set({ loading: true, error: null });
    try {
      const result = await EmulatorAPI.step();
      if (result.status === "error") {
        set({ error: result.error });
        return;
      }
      await get().refresh();
      await get().pollUart();
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
      });
    } finally {
      set({ loading: false });
    }
  },

  // Run the emulator
  run: async (maxCycles) => {
    set({ loading: true, error: null });
    try {
      const result = await EmulatorAPI.run(maxCycles);
      if (result.status === "success") {
        set({ status: result.data });
        await get().refresh();
        await get().pollUart();
      } else {
        set({ error: result.error });
      }
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
      });
    } finally {
      set({ loading: false });
    }
  },

  // Reset the emulator
  reset: async () => {
    set({ loading: true, error: null, uartOutput: "", assemblyError: null });
    try {
      const result = await EmulatorAPI.reset();
      if (result.status === "error") {
        set({ error: result.error });
        return;
      }
      await get().refresh();
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
      });
    } finally {
      set({ loading: false });
    }
  },

  // Refresh CPU state from backend
  refresh: async () => {
    try {
      const [cpuResult, statusResult, ledResult] = await Promise.all([
        EmulatorAPI.getRegisters(),
        EmulatorAPI.getStatus(),
        EmulatorAPI.getLed(),
      ]);

      if (cpuResult.status === "success") {
        set({ cpuState: cpuResult.data });
      } else {
        set({ error: cpuResult.error });
      }

      if (statusResult.status === "success") {
        set({ status: statusResult.data });
      }

      if (ledResult.status === "success") {
        set({ ledState: ledResult.data });
      }
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
      });
    }
  },

  // Poll UART output
  pollUart: async () => {
    try {
      const result = await EmulatorAPI.readUart();
      if (result.status === "success" && result.data.length > 0) {
        const text = result.data.map((b) => String.fromCharCode(b)).join("");
        set((state) => ({ uartOutput: state.uartOutput + text }));
      }
    } catch {
      // Silently ignore UART poll errors
    }
  },

  // Send a character to UART
  sendUartChar: async (byte: number) => {
    try {
      await EmulatorAPI.writeUart(byte);
    } catch {
      // Silently ignore
    }
  },

  // Set source code
  setSourceCode: (code: string) => set({ sourceCode: code }),

  // Assemble and load the current source code
  assembleAndRun: async () => {
    const { sourceCode } = get();
    if (!sourceCode.trim()) {
      set({ assemblyError: "No source code to assemble" });
      return;
    }
    set({ loading: true, assemblyError: null, error: null, uartOutput: "" });
    try {
      // Reset emulator first
      const resetResult = await EmulatorAPI.reset();
      if (resetResult.status === "error") {
        set({ error: resetResult.error });
        return;
      }

      // Assemble and load
      const result = await EmulatorAPI.assembleAndLoad(sourceCode);
      if (result.status === "error") {
        set({ assemblyError: result.error });
        return;
      }

      // Run the program
      const runResult = await EmulatorAPI.run(1000000);
      if (runResult.status === "success") {
        set({ status: runResult.data });
      }
      await get().refresh();
      await get().pollUart();
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
      });
    } finally {
      set({ loading: false });
    }
  },

  // Clear UART output
  clearUart: () => set({ uartOutput: "" }),

  // Clear error
  clearError: () => set({ error: null, assemblyError: null }),

  // Set memory viewer address
  setMemoryAddress: (address) => set({ memoryAddress: address }),
}));
