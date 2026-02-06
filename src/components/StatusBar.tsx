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
export function StatusBar({ status, cpuState, initialized, error, loading, ledState }: StatusBarProps) {
    const halted = status?.halted ?? true;
    const cycles = status?.cycles ?? 0;
    const isRunning = loading ?? false;

    return (
        <div
            data-no-select
            className={cn(
                "shrink-0 flex items-center h-6 px-2 text-[11px] border-t",
                error
                    ? "bg-destructive/15 border-destructive/30 text-destructive"
                    : "bg-muted/40 border-border text-muted-foreground"
            )}
        >
            {/* Left section */}
            <div className="flex items-center gap-1.5 min-w-0 flex-1">
                {/* Connection status */}
                <div className="flex items-center gap-1.5 px-1.5">
                    <div
                        className={cn(
                            "w-1.5 h-1.5 rounded-full",
                            !initialized
                                ? "bg-muted-foreground/50"
                                : error
                                    ? "bg-destructive"
                                    : isRunning
                                        ? "bg-emerald-500 animate-pulse"
                                        : halted
                                            ? "bg-amber-500"
                                            : "bg-muted-foreground/60"
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

                <Separator orientation="vertical" className="h-3.5 bg-border/60" />

                {/* Cycles */}
                {status && (
                    <span className="font-mono text-[10px] px-1.5">
                        {cycles.toLocaleString()} cycles
                    </span>
                )}

                {/* PC value */}
                {cpuState && (
                    <>
                        <Separator orientation="vertical" className="h-3.5 bg-border/60" />
                        <span className="font-mono text-[10px] px-1.5 text-emerald-500/80">
                            PC ${cpuState.pc.toString(16).toUpperCase().padStart(6, "0")}
                        </span>
                    </>
                )}

                {/* Error message */}
                {error && (
                    <>
                        <Separator orientation="vertical" className="h-3.5 bg-destructive/30" />
                        <span className="text-[10px] truncate px-1.5">{error}</span>
                    </>
                )}
            </div>

            {/* Right section */}
            <div className="flex items-center gap-1.5 shrink-0">
                <span className="font-mono text-[10px] px-1.5">
                    ROM $000000
                </span>
                <Separator orientation="vertical" className="h-3.5 bg-border/60" />
                <span className="font-mono text-[10px] px-1.5">
                    RAM $C00000
                </span>
                <Separator orientation="vertical" className="h-3.5 bg-border/60" />
                <span className="font-mono text-[10px] px-1.5">
                    UART $A00000
                </span>
                <Separator orientation="vertical" className="h-3.5 bg-border/60" />
                <span className="text-[10px] px-1.5">
                    Flux32 v0.1.0
                </span>
            </div>
        </div>
    );
}
