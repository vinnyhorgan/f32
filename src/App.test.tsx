import { describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/react";
import App from "./App";

// Mock the module
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    show: vi.fn(),
    setFocus: vi.fn(),
  }),
}));

describe("App", () => {
  it("renders without crashing", () => {
    render(<App />);
    // Just check if something renders, e.g. the main container or title
    // Since I don't know the exact text, I'll rely on it not throwing.
    expect(document.body).toBeInTheDocument();
  });
});
