import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// Check if cleanup is needed (vitest usually handles it if imported properly)
// If we use globals: true, afterEach is available globally.
// Otherwise:
afterEach(() => {
  cleanup();
});
