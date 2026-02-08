import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Toolbar } from "./Toolbar";

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

describe("Toolbar", () => {
  const defaultProps = {
    isRunning: false,
    isHalted: false,
    onStep: vi.fn(),
    onRun: vi.fn(),
    onReset: vi.fn(),
    onRefresh: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders all debug buttons", () => {
    render(<Toolbar {...defaultProps} />);

    // Check for buttons by their accessible roles
    const buttons = screen.getAllByRole("button");
    expect(buttons.length).toBeGreaterThanOrEqual(4);
  });

  it("shows Ready status when not running and not halted", () => {
    render(<Toolbar {...defaultProps} />);

    expect(screen.getByText("Ready")).toBeInTheDocument();
  });

  it("shows Running status when isRunning is true", () => {
    render(<Toolbar {...defaultProps} isRunning={true} />);

    expect(screen.getByText("Running")).toBeInTheDocument();
  });

  it("shows Halted status when isHalted is true", () => {
    render(<Toolbar {...defaultProps} isHalted={true} />);

    expect(screen.getByText("Halted")).toBeInTheDocument();
  });

  it("disables buttons when running", () => {
    render(<Toolbar {...defaultProps} isRunning={true} />);

    const buttons = screen.getAllByRole("button");
    buttons.forEach((button) => {
      expect(button).toBeDisabled();
    });
  });

  it("calls onStep when step button is clicked", async () => {
    const user = userEvent.setup();
    const onStep = vi.fn();
    render(<Toolbar {...defaultProps} onStep={onStep} />);

    // Find the step button (usually the second or third button)
    const buttons = screen.getAllByRole("button");
    // The step button has StepForward icon - click it
    await user.click(buttons[1]); // Step button is typically second

    expect(onStep).toHaveBeenCalledTimes(1);
  });

  it("calls onReset when reset button is clicked", async () => {
    const user = userEvent.setup();
    const onReset = vi.fn();
    render(<Toolbar {...defaultProps} onReset={onReset} />);

    const buttons = screen.getAllByRole("button");
    await user.click(buttons[2]); // Reset button

    expect(onReset).toHaveBeenCalledTimes(1);
  });

  it("shows Assemble & Run button when onAssembleAndRun is provided", () => {
    render(<Toolbar {...defaultProps} onAssembleAndRun={vi.fn()} />);

    // Should have one more button
    const buttons = screen.getAllByRole("button");
    expect(buttons.length).toBeGreaterThanOrEqual(5);
  });

  it("shows LED indicator when ledState is provided", () => {
    render(<Toolbar {...defaultProps} ledState={true} />);

    expect(screen.getByText("LED")).toBeInTheDocument();
  });
});
