import { describe, it, expect, vi, beforeEach } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useEmulatorStore } from "./emulator-store";
import { EmulatorAPI } from "./emulator-api";

// Mock the EmulatorAPI module
vi.mock("./emulator-api", () => ({
  EmulatorAPI: {
    init: vi.fn(),
    step: vi.fn(),
    run: vi.fn(),
    reset: vi.fn(),
    getRegisters: vi.fn(),
    getStatus: vi.fn(),
    getLed: vi.fn(),
    readUart: vi.fn(),
    writeUart: vi.fn(),
    assembleAndLoad: vi.fn(),
  },
}));

// Helper to create a mock CpuState with all required fields
function mockCpuState(overrides: Partial<{ pc: number; sr: number }> = {}) {
  return {
    pc: overrides.pc ?? 0,
    sr: overrides.sr ?? 0,
    d: [0, 0, 0, 0, 0, 0, 0, 0],
    a: [0, 0, 0, 0, 0, 0, 0, 0],
    usp: 0,
    ssp: 0,
  };
}

// Helper to create a mock EmulatorStatus with all required fields
function mockStatus(
  overrides: Partial<{
    halted: boolean;
    cycles: number;
    executed: number;
  }> = {},
) {
  return {
    halted: overrides.halted ?? false,
    cycles: overrides.cycles ?? 0,
    executed: overrides.executed ?? 0,
  };
}

describe("useEmulatorStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset store state
    useEmulatorStore.setState({
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
    });
  });

  describe("initial state", () => {
    it("has correct initial values", () => {
      const { result } = renderHook(() => useEmulatorStore());

      expect(result.current.initialized).toBe(false);
      expect(result.current.cpuState).toBeNull();
      expect(result.current.status).toBeNull();
      expect(result.current.error).toBeNull();
      expect(result.current.loading).toBe(false);
      expect(result.current.uartOutput).toBe("");
    });
  });

  describe("init", () => {
    it("initializes successfully", async () => {
      vi.mocked(EmulatorAPI.init).mockResolvedValue({
        status: "success",
        data: "initialized",
      });
      vi.mocked(EmulatorAPI.getRegisters).mockResolvedValue({
        status: "success",
        data: mockCpuState(),
      });
      vi.mocked(EmulatorAPI.getStatus).mockResolvedValue({
        status: "success",
        data: mockStatus(),
      });
      vi.mocked(EmulatorAPI.getLed).mockResolvedValue({
        status: "success",
        data: false,
      });

      const { result } = renderHook(() => useEmulatorStore());

      await act(async () => {
        await result.current.init();
      });

      expect(result.current.initialized).toBe(true);
      expect(result.current.error).toBeNull();
    });

    it("handles initialization error", async () => {
      vi.mocked(EmulatorAPI.init).mockResolvedValue({
        status: "error",
        error: "Failed to initialize",
      });

      const { result } = renderHook(() => useEmulatorStore());

      await act(async () => {
        await result.current.init();
      });

      expect(result.current.initialized).toBe(false);
      expect(result.current.error).toBe("Failed to initialize");
    });
  });

  describe("step", () => {
    it("executes a step successfully", async () => {
      vi.mocked(EmulatorAPI.step).mockResolvedValue({
        status: "success",
        data: "stepped",
      });
      vi.mocked(EmulatorAPI.getRegisters).mockResolvedValue({
        status: "success",
        data: mockCpuState({ pc: 4 }),
      });
      vi.mocked(EmulatorAPI.getStatus).mockResolvedValue({
        status: "success",
        data: mockStatus({ cycles: 1 }),
      });
      vi.mocked(EmulatorAPI.getLed).mockResolvedValue({
        status: "success",
        data: false,
      });
      vi.mocked(EmulatorAPI.readUart).mockResolvedValue({
        status: "success",
        data: [],
      });

      const { result } = renderHook(() => useEmulatorStore());

      await act(async () => {
        await result.current.step();
      });

      expect(EmulatorAPI.step).toHaveBeenCalled();
      expect(result.current.cpuState?.pc).toBe(4);
    });
  });

  describe("run", () => {
    it("runs with specified cycles", async () => {
      vi.mocked(EmulatorAPI.run).mockResolvedValue({
        status: "success",
        data: mockStatus({ halted: true, cycles: 1000, executed: 1000 }),
      });
      vi.mocked(EmulatorAPI.getRegisters).mockResolvedValue({
        status: "success",
        data: mockCpuState({ pc: 100 }),
      });
      vi.mocked(EmulatorAPI.getStatus).mockResolvedValue({
        status: "success",
        data: mockStatus({ halted: true, cycles: 1000, executed: 1000 }),
      });
      vi.mocked(EmulatorAPI.getLed).mockResolvedValue({
        status: "success",
        data: true,
      });
      vi.mocked(EmulatorAPI.readUart).mockResolvedValue({
        status: "success",
        data: [],
      });

      const { result } = renderHook(() => useEmulatorStore());

      await act(async () => {
        await result.current.run(1000);
      });

      expect(EmulatorAPI.run).toHaveBeenCalledWith(1000);
      expect(result.current.status?.halted).toBe(true);
      expect(result.current.status?.cycles).toBe(1000);
    });
  });

  describe("reset", () => {
    it("resets state correctly", async () => {
      vi.mocked(EmulatorAPI.reset).mockResolvedValue({
        status: "success",
        data: "reset",
      });
      vi.mocked(EmulatorAPI.getRegisters).mockResolvedValue({
        status: "success",
        data: mockCpuState(),
      });
      vi.mocked(EmulatorAPI.getStatus).mockResolvedValue({
        status: "success",
        data: mockStatus(),
      });
      vi.mocked(EmulatorAPI.getLed).mockResolvedValue({
        status: "success",
        data: false,
      });

      const { result } = renderHook(() => useEmulatorStore());

      // Set some state first
      act(() => {
        useEmulatorStore.setState({
          uartOutput: "some output",
          assemblyError: "some error",
        });
      });

      await act(async () => {
        await result.current.reset();
      });

      expect(result.current.uartOutput).toBe("");
      expect(result.current.assemblyError).toBeNull();
    });
  });

  describe("UART", () => {
    it("polls UART and appends output", async () => {
      vi.mocked(EmulatorAPI.readUart).mockResolvedValue({
        status: "success",
        data: [72, 105], // "Hi"
      });

      const { result } = renderHook(() => useEmulatorStore());

      await act(async () => {
        await result.current.pollUart();
      });

      expect(result.current.uartOutput).toBe("Hi");
    });

    it("clears UART output", () => {
      const { result } = renderHook(() => useEmulatorStore());

      act(() => {
        useEmulatorStore.setState({ uartOutput: "Hello World" });
      });

      act(() => {
        result.current.clearUart();
      });

      expect(result.current.uartOutput).toBe("");
    });
  });

  describe("source code", () => {
    it("sets source code", () => {
      const { result } = renderHook(() => useEmulatorStore());

      act(() => {
        result.current.setSourceCode("MOVE.L #$1234,D0");
      });

      expect(result.current.sourceCode).toBe("MOVE.L #$1234,D0");
    });
  });

  describe("assembleAndRun", () => {
    it("does nothing with empty source", async () => {
      const { result } = renderHook(() => useEmulatorStore());

      await act(async () => {
        await result.current.assembleAndRun();
      });

      expect(result.current.assemblyError).toBe("No source code to assemble");
    });

    it("assembles and runs successfully", async () => {
      vi.mocked(EmulatorAPI.reset).mockResolvedValue({
        status: "success",
        data: "reset",
      });
      vi.mocked(EmulatorAPI.assembleAndLoad).mockResolvedValue({
        status: "success",
        data: "loaded",
      });
      vi.mocked(EmulatorAPI.run).mockResolvedValue({
        status: "success",
        data: mockStatus({ halted: true, cycles: 100, executed: 100 }),
      });
      vi.mocked(EmulatorAPI.getRegisters).mockResolvedValue({
        status: "success",
        data: mockCpuState(),
      });
      vi.mocked(EmulatorAPI.getStatus).mockResolvedValue({
        status: "success",
        data: mockStatus({ halted: true, cycles: 100, executed: 100 }),
      });
      vi.mocked(EmulatorAPI.getLed).mockResolvedValue({
        status: "success",
        data: false,
      });
      vi.mocked(EmulatorAPI.readUart).mockResolvedValue({
        status: "success",
        data: [],
      });

      const { result } = renderHook(() => useEmulatorStore());

      act(() => {
        result.current.setSourceCode("NOP");
      });

      await act(async () => {
        await result.current.assembleAndRun();
      });

      expect(EmulatorAPI.assembleAndLoad).toHaveBeenCalledWith("NOP");
      expect(result.current.assemblyError).toBeNull();
    });

    it("handles assembly errors", async () => {
      vi.mocked(EmulatorAPI.reset).mockResolvedValue({
        status: "success",
        data: "reset",
      });
      vi.mocked(EmulatorAPI.assembleAndLoad).mockResolvedValue({
        status: "error",
        error: "Syntax error at line 1",
      });

      const { result } = renderHook(() => useEmulatorStore());

      act(() => {
        result.current.setSourceCode("INVALID_INSTRUCTION");
      });

      await act(async () => {
        await result.current.assembleAndRun();
      });

      expect(result.current.assemblyError).toBe("Syntax error at line 1");
    });
  });

  describe("clearError", () => {
    it("clears both error and assemblyError", () => {
      const { result } = renderHook(() => useEmulatorStore());

      act(() => {
        useEmulatorStore.setState({
          error: "General error",
          assemblyError: "Assembly error",
        });
      });

      act(() => {
        result.current.clearError();
      });

      expect(result.current.error).toBeNull();
      expect(result.current.assemblyError).toBeNull();
    });
  });

  describe("setMemoryAddress", () => {
    it("updates memory address", () => {
      const { result } = renderHook(() => useEmulatorStore());

      act(() => {
        result.current.setMemoryAddress(0xc00000);
      });

      expect(result.current.memoryAddress).toBe(0xc00000);
    });
  });
});
