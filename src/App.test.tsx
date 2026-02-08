import { describe, it, expect, vi, afterEach } from "vitest";
import { render, waitFor, cleanup } from "@testing-library/react";
import { act } from "react";
import App from "./App";

// Mock the Tauri core module
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve()),
}));

// Mock the Tauri window module
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    show: vi.fn(() => Promise.resolve()),
    setFocus: vi.fn(() => Promise.resolve()),
  }),
}));

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

describe("App", () => {
  it("renders without crashing", async () => {
    expect.assertions(1);

    await act(async () => {
      render(<App />);
    });

    await waitFor(() => {
      expect(document.body).toBeInTheDocument();
    });
  });
});
