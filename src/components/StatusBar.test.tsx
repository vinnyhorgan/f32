import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { StatusBar } from "./StatusBar";

// Suppress React act() console noise during tests
const originalError = console.error;
beforeEach(() => {
  console.error = (...args: unknown[]) => {
    if (typeof args[0] === "string" && args[0].includes("not wrapped in act")) {
      return;
    }
    originalError(...args);
  };
});

afterEach(() => {
  console.error = originalError;
  cleanup();
});

// Helper to create mock status with all required fields
function mockStatus(
  overrides: Partial<{ halted: boolean; cycles: number }> = {},
) {
  return {
    halted: overrides.halted ?? false,
    cycles: overrides.cycles ?? 0,
    executed: 0,
  };
}

// Helper to create mock CPU state with all required fields
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

describe("StatusBar", () => {
  const defaultProps = {
    status: null,
    cpuState: null,
    initialized: false,
    error: null,
  };

  it("shows Not Initialized when not initialized", () => {
    render(<StatusBar {...defaultProps} />);

    expect(screen.getByText("Not Initialized")).toBeInTheDocument();
  });

  it("shows Ready when initialized and not halted", () => {
    render(
      <StatusBar {...defaultProps} initialized={true} status={mockStatus()} />,
    );

    expect(screen.getByText("Ready")).toBeInTheDocument();
  });

  it("shows Halted when initialized and halted", () => {
    render(
      <StatusBar
        {...defaultProps}
        initialized={true}
        status={mockStatus({ halted: true })}
      />,
    );

    expect(screen.getByText("Halted")).toBeInTheDocument();
  });

  it("shows Running when loading", () => {
    render(
      <StatusBar
        {...defaultProps}
        initialized={true}
        loading={true}
        status={mockStatus()}
      />,
    );

    expect(screen.getByText("Running")).toBeInTheDocument();
  });

  it("shows Error when error is present", () => {
    render(
      <StatusBar
        {...defaultProps}
        initialized={true}
        error="Something went wrong"
      />,
    );

    expect(screen.getByText("Error")).toBeInTheDocument();
    expect(screen.getByText("Something went wrong")).toBeInTheDocument();
  });

  it("displays cycle count when status is available", () => {
    render(
      <StatusBar
        {...defaultProps}
        initialized={true}
        status={mockStatus({ cycles: 12345 })}
      />,
    );

    expect(screen.getByText("12,345 cycles")).toBeInTheDocument();
  });

  it("displays PC value when cpuState is available", () => {
    render(
      <StatusBar
        {...defaultProps}
        initialized={true}
        status={mockStatus()}
        cpuState={mockCpuState({ pc: 0x001234 })}
      />,
    );

    expect(screen.getByText("PC $001234")).toBeInTheDocument();
  });

  it("displays memory map information", () => {
    render(<StatusBar {...defaultProps} initialized={true} />);

    expect(screen.getByText("ROM $000000")).toBeInTheDocument();
    expect(screen.getByText("RAM $C00000")).toBeInTheDocument();
    expect(screen.getByText("UART $A00000")).toBeInTheDocument();
  });

  it("displays version info", () => {
    render(<StatusBar {...defaultProps} />);

    expect(screen.getByText("Flux32 v0.1.0")).toBeInTheDocument();
  });
});
