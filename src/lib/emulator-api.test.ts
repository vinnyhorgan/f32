import { describe, it, expect, vi, beforeEach, type Mock } from "vitest";
import { EmulatorAPI } from "./emulator-api";
import { invoke } from "@tauri-apps/api/core";

// Mock the Tauri invoke function
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("EmulatorAPI", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("init calls invoke with correct command", async () => {
    (invoke as unknown as Mock).mockResolvedValue("Emulator initialized");

    const result = await EmulatorAPI.init();

    expect(invoke).toHaveBeenCalledWith("emulator_init");
    expect(result).toEqual({ status: "success", data: "Emulator initialized" });
  });

  it("handles errors gracefully", async () => {
    (invoke as unknown as Mock).mockRejectedValue(new Error("Failed to init"));

    const result = await EmulatorAPI.init();

    expect(invoke).toHaveBeenCalledWith("emulator_init");
    expect(result).toEqual({ status: "error", error: "Failed to init" });
  });
});
