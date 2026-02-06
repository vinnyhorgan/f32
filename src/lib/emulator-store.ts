/**
 * Emulator Store (Zustand)
 *
 * Global state management for the M68K emulator.
 * Handles CPU state, execution control, memory, and disassembly.
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
    set({ loading: true, error: null });
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
      const [cpuResult, statusResult] = await Promise.all([
        EmulatorAPI.getRegisters(),
        EmulatorAPI.getStatus(),
      ]);

      if (cpuResult.status === "success") {
        set({ cpuState: cpuResult.data });
      } else {
        set({ error: cpuResult.error });
      }

      if (statusResult.status === "success") {
        set({ status: statusResult.data });
      }
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
      });
    }
  },

  // Clear error
  clearError: () => set({ error: null }),

  // Set memory viewer address
  setMemoryAddress: (address) => set({ memoryAddress: address }),
}));
