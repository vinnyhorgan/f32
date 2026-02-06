/**
 * Flux32 Main Application
 *
 * M68K Emulator GUI Debugger
 * Integrates all emulator components into a unified interface
 */

import { useEffect } from "react";
import { RegisterDisplay } from "./components/RegisterDisplay";
import { ControlPanel } from "./components/ControlPanel";
import { MemoryViewer } from "./components/MemoryViewer";
import { useEmulatorStore } from "./lib/emulator-store";
import { EmulatorAPI } from "./lib/emulator-api";
import { Cpu, Activity } from "lucide-react";

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

  // Initialize emulator on mount
  useEffect(() => {
    if (!initialized) {
      init();
    }
  }, []);

  return (
    <div className="h-screen w-full bg-gradient-to-br from-slate-950 via-slate-900 to-zinc-950 text-slate-100 flex flex-col overflow-hidden">
      {/* Glassmorphic Header */}
      <header className="shrink-0 border-b border-white/5 bg-white/[0.02] backdrop-blur-xl">
        <div className="flex items-center justify-between px-6 py-3">
          {/* Logo & Title */}
          <div className="flex items-center gap-3">
            <div className="flex items-center justify-center w-9 h-9 rounded-xl bg-gradient-to-br from-blue-500 to-cyan-400 shadow-lg shadow-blue-500/20">
              <Cpu className="w-5 h-5 text-white" />
            </div>
            <div>
              <h1 className="text-lg font-bold tracking-tight bg-gradient-to-r from-white to-slate-300 bg-clip-text text-transparent">
                Flux32
              </h1>
              <p className="text-[10px] text-slate-500 font-medium tracking-wide uppercase">
                M68K Debugger
              </p>
            </div>
          </div>

          {/* Status indicators */}
          <div className="flex items-center gap-4">
            {status && (
              <div className="flex items-center gap-4 px-4 py-1.5 rounded-full bg-black/20 border border-white/5">
                {/* Status */}
                <div className="flex items-center gap-2">
                  <div
                    className={`relative w-2 h-2 rounded-full ${
                      status.halted ? "bg-amber-500" : "bg-emerald-500"
                    }`}
                  >
                    <div
                      className={`absolute inset-0 rounded-full animate-ping ${
                        status.halted ? "bg-amber-500/30" : "bg-emerald-500/30"
                      }`}
                    />
                  </div>
                  <span className="text-xs font-medium text-slate-300">
                    {status.halted ? "HALTED" : "RUNNING"}
                  </span>
                </div>

                {/* Separator */}
                <div className="w-px h-3 bg-white/10" />

                {/* Cycles */}
                <div className="flex items-center gap-1.5 text-xs text-slate-400">
                  <Activity className="w-3 h-3" />
                  <span className="font-mono">{status.cycles.toLocaleString()}</span>
                </div>
              </div>
            )}
            {loading && (
              <div className="text-xs text-blue-400 font-medium animate-pulse">
                Loading...
              </div>
            )}
          </div>
        </div>
      </header>

      {/* Error banner */}
      {error && (
        <div className="shrink-0 mx-6 mt-3 px-4 py-2.5 bg-red-500/10 border border-red-500/20 rounded-lg flex items-center justify-between">
          <span className="text-red-400 text-xs font-medium">{error}</span>
          <button
            onClick={clearError}
            className="text-slate-400 hover:text-slate-200 text-xs transition-colors"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Main content - scrollable */}
      <main className="flex-1 overflow-auto px-6 py-4">
        <div className="max-w-[1600px] mx-auto grid grid-cols-1 lg:grid-cols-12 gap-4">
          {/* Left column: Controls and Registers */}
          <div className="lg:col-span-4 space-y-4">
            <ControlPanel
              isRunning={loading}
              isHalted={status?.halted ?? false}
              onStep={step}
              onRun={() => run(100000)}
              onReset={reset}
              onRefresh={refresh}
            />
            <RegisterDisplay cpuState={cpuState} />
          </div>

          {/* Right column: Memory Viewer */}
          <div className="lg:col-span-8">
            <MemoryViewer
              onReadMemory={async (address, length) => {
                const result = await EmulatorAPI.readMemory(address, length);
                if (result.status === "success") {
                  return result.data;
                }
                throw new Error(result.error);
              }}
              initialAddress={memoryAddress}
              displayLength={256}
            />
          </div>
        </div>
      </main>

      {/* Footer */}
      <footer className="shrink-0 border-t border-white/5 bg-black/20 backdrop-blur-sm px-6 py-2">
        <div className="flex items-center justify-between text-[10px] text-slate-600 font-medium">
          <span>Flux32 v0.1.0</span>
          <div className="flex items-center gap-3 font-mono">
            <span>RAM: $C00000-$CFFFFF</span>
            <span>ROM: $000000-$0FFFFF</span>
            <span>UART: $A00000</span>
          </div>
        </div>
      </footer>
    </div>
  );
}

export default App;
