/**
 * StatusBar Component
 *
 * Bottom status bar displaying emulator state, memory map, cycle count,
 * and version info. Modeled after VS Code / desktop IDE status bars.
 */

import type { EmulatorStatus } from "../lib/emulator-types";
import type { CpuState } from "../lib/emulator-types";
import { cn } from "../lib/utils";
import { Separator } from "./ui/separator";

interface StatusBarProps {
  status: EmulatorStatus | null;
  cpuState: CpuState | null;
  initialized: boolean;
  error: string | null;
  loading?: boolean;
  ledState?: boolean;
}

/** Desktop-style status bar pinned to the bottom of the window */
export function StatusBar({
  status,
  cpuState,
  initialized,
  error,
  loading,
}: StatusBarProps) {
  const halted = status?.halted ?? true;
  const cycles = status?.cycles ?? 0;
  const isRunning = loading ?? false;

  return (
    <div
      data-no-select
      className={cn(
        "flex h-6 shrink-0 items-center border-t px-2 text-[11px]",
        error
          ? "bg-destructive/15 border-destructive/30 text-destructive"
          : "bg-muted/40 border-border text-muted-foreground",
      )}
    >
      {/* Left section */}
      <div className="flex min-w-0 flex-1 items-center gap-1.5">
        {/* Connection status */}
        <div className="flex items-center gap-1.5 px-1.5">
          <div
            className={cn(
              "h-1.5 w-1.5 rounded-full",
              !initialized
                ? "bg-muted-foreground/50"
                : error
                  ? "bg-destructive"
                  : isRunning
                    ? "animate-pulse bg-emerald-500"
                    : halted
                      ? "bg-amber-500"
                      : "bg-muted-foreground/60",
            )}
          />
          <span className="font-medium">
            {!initialized
              ? "Not Initialized"
              : error
                ? "Error"
                : isRunning
                  ? "Running"
                  : halted
                    ? "Halted"
                    : "Ready"}
          </span>
        </div>

        <Separator orientation="vertical" className="bg-border/60 h-3.5" />

        {/* Cycles */}
        {status && (
          <span className="px-1.5 font-mono text-[10px]">
            {cycles.toLocaleString()} cycles
          </span>
        )}

        {/* PC value */}
        {cpuState && (
          <>
            <Separator orientation="vertical" className="bg-border/60 h-3.5" />
            <span className="px-1.5 font-mono text-[10px] text-emerald-500/80">
              PC ${cpuState.pc.toString(16).toLowerCase().padStart(6, "0")}
            </span>
          </>
        )}

        {/* Error message */}
        {error && (
          <>
            <Separator
              orientation="vertical"
              className="bg-destructive/30 h-3.5"
            />
            <span className="truncate px-1.5 text-[10px]">{error}</span>
          </>
        )}
      </div>

      {/* Right section */}
      <div className="flex shrink-0 items-center gap-1.5">
        <span className="px-1.5 font-mono text-[10px]">ROM $000000</span>
        <Separator orientation="vertical" className="bg-border/60 h-3.5" />
        <span className="px-1.5 font-mono text-[10px]">RAM $C00000</span>
        <Separator orientation="vertical" className="bg-border/60 h-3.5" />
        <span className="px-1.5 font-mono text-[10px]">UART $A00000</span>
        <Separator orientation="vertical" className="bg-border/60 h-3.5" />
        <span className="px-1.5 text-[10px]">Flux32 v0.1.0</span>
      </div>
    </div>
  );
}
