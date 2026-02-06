/**
 * Flux32 Main Application
 *
 * M68K Emulator GUI — desktop-style layout with:
 * - MenuBar (top)
 * - Toolbar (debug controls)
 * - Sidebar (registers) + Main area (memory viewer)
 * - StatusBar (bottom)
 */

import { useEffect, useCallback } from "react";
import { AppMenuBar } from "./components/AppMenuBar";
import { Toolbar } from "./components/Toolbar";
import { StatusBar } from "./components/StatusBar";
import { RegisterDisplay } from "./components/RegisterDisplay";
import { MemoryViewer } from "./components/MemoryViewer";
import { useEmulatorStore } from "./lib/emulator-store";
import { EmulatorAPI } from "./lib/emulator-api";
import { ScrollArea } from "./components/ui/scroll-area";
import { Separator } from "./components/ui/separator";

function App() {
  const {
    initialized,
    cpuState,
    status,
    error,
    loading,
    memoryAddress,
    init,
    step,
    run,
    reset,
    refresh,
    clearError,
  } = useEmulatorStore();

  const isHalted = status?.halted ?? false;

  const handleRun = useCallback(() => run(100000), [run]);

  /** Initialize emulator on mount */
  useEffect(() => {
    if (!initialized) init();
  }, [initialized, init]);

  /** Global keyboard shortcuts */
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (loading) return;
      switch (e.key) {
        case "F5":
          e.preventDefault();
          handleRun();
          break;
        case "F10":
          e.preventDefault();
          step();
          break;
        case "F6":
          e.preventDefault();
          reset();
          break;
        case "F9":
          e.preventDefault();
          refresh();
          break;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [loading, handleRun, step, reset, refresh]);

  /** Dismiss error on Escape */
  useEffect(() => {
    if (!error) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") clearError();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [error, clearError]);

  return (
    <div className="h-screen w-full flex flex-col overflow-hidden bg-background text-foreground">
      {/* Menu bar */}
      <AppMenuBar
        isRunning={loading}
        onStep={step}
        onRun={handleRun}
        onReset={reset}
        onRefresh={refresh}
      />

      {/* Toolbar */}
      <Toolbar
        isRunning={loading}
        isHalted={isHalted}
        onStep={step}
        onRun={handleRun}
        onReset={reset}
        onRefresh={refresh}
      />

      {/* Error banner */}
      {error && (
        <div className="shrink-0 flex items-center justify-between px-3 py-1.5 bg-destructive/10 border-b border-destructive/20 text-destructive text-xs">
          <span className="truncate">{error}</span>
          <button
            onClick={clearError}
            className="ml-2 shrink-0 text-[10px] text-muted-foreground hover:text-foreground transition-colors"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Main content area: sidebar + center panel */}
      <div className="flex-1 flex min-h-0">
        {/* Left sidebar — Registers */}
        <aside className="w-[260px] shrink-0 border-r border-border bg-card flex flex-col">
          <div
            data-no-select
            className="shrink-0 flex items-center h-7 px-3 border-b border-border bg-muted/40"
          >
            <span className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">
              Registers
            </span>
          </div>
          <ScrollArea className="flex-1">
            <RegisterDisplay cpuState={cpuState} />
          </ScrollArea>
        </aside>

        <Separator orientation="vertical" className="bg-border" />

        {/* Main panel — Memory Viewer */}
        <main className="flex-1 flex flex-col min-w-0 bg-background">
          <div
            data-no-select
            className="shrink-0 flex items-center h-7 px-3 border-b border-border bg-muted/40"
          >
            <span className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">
              Memory
            </span>
          </div>
          <MemoryViewer
            className="flex-1 min-h-0"
            onReadMemory={async (address, length) => {
              const result = await EmulatorAPI.readMemory(address, length);
              if (result.status === "success") return result.data;
              throw new Error(result.error);
            }}
            initialAddress={memoryAddress}
            displayLength={512}
          />
        </main>
      </div>

      {/* Status bar */}
      <StatusBar
        status={status}
        initialized={initialized}
        error={error}
      />
    </div>
  );
}

export default App;
