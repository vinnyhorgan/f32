import { describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/react";
import App from "./App";

// Mock the module
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve()), // Return a promise
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    show: vi.fn(),
    setFocus: vi.fn(),
  }),
}));

describe("App", () => {
  it("renders without crashing", async () => {
    expect.assertions(1); // Ensure 1 assertion runs
    render(<App />);
    // Just check if something renders, e.g. the main container or title
    expect(document.body).toBeInTheDocument();
  });
});
