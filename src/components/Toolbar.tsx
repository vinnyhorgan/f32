/**
 * Toolbar Component
 *
 * Horizontal toolbar with debug action buttons and keyboard shortcuts.
 * Compact, icon-forward design inspired by IDE debug toolbars.
 */

import { Button } from "./ui/button";
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from "./ui/tooltip";
import { Separator } from "./ui/separator";
import {
    Play,
    StepForward,
    RotateCcw,
    RefreshCw,
    Square,
    Rocket,
    FastForward,
} from "lucide-react";

interface ToolbarProps {
    isRunning: boolean;
    isHalted: boolean;
    onStep: () => void;
    onRun: () => void;
    onReset: () => void;
    onRefresh: () => void;
    onAssembleAndRun?: () => void;
    ledState?: boolean;
}

/** A compact, icon-centric debug toolbar */
export function Toolbar({
    isRunning,
    isHalted,
    onStep,
    onRun,
    onReset,
    onRefresh,
    onAssembleAndRun,
    ledState,
}: ToolbarProps) {
    return (
        <TooltipProvider delayDuration={400}>
            <div
                data-no-select
                className="shrink-0 flex items-center gap-0.5 h-9 px-2 border-b border-border bg-muted/30"
            >
                {/* Assemble & Run */}
                {onAssembleAndRun && (
                    <>
                        <Tooltip>
                            <TooltipTrigger asChild>
                                <Button
                                    variant="ghost"
                                    size="icon-sm"
                                    onClick={onAssembleAndRun}
                                    disabled={isRunning}
                                    className="text-emerald-500 hover:text-emerald-400 hover:bg-emerald-500/10"
                                >
                                    <Rocket className="size-3.5" />
                                </Button>
                            </TooltipTrigger>
                            <TooltipContent side="bottom" className="text-xs">
                                <p>Assemble &amp; Run <kbd className="ml-1.5 font-mono text-[10px] text-muted-foreground bg-muted px-1 py-0.5 rounded">F5</kbd></p>
                            </TooltipContent>
                        </Tooltip>

                        <Separator orientation="vertical" className="h-4 mx-1 bg-border/60" />
                    </>
                )}

                {/* Continue running */}
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Button
                            variant="ghost"
                            size="icon-sm"
                            onClick={onRun}
                            disabled={isRunning}
                            className="text-sky-400 hover:text-sky-300 hover:bg-sky-500/10"
                        >
                            {isRunning ? (
                                <Square className="size-3.5" />
                            ) : (
                                <FastForward className="size-3.5" />
                            )}
                        </Button>
                    </TooltipTrigger>
                    <TooltipContent side="bottom" className="text-xs">
                        <p>Continue <kbd className="ml-1.5 font-mono text-[10px] text-muted-foreground bg-muted px-1 py-0.5 rounded">F8</kbd></p>
                    </TooltipContent>
                </Tooltip>

                {/* Step */}
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Button
                            variant="ghost"
                            size="icon-sm"
                            onClick={onStep}
                            disabled={isRunning}
                            className="text-blue-400 hover:text-blue-300 hover:bg-blue-500/10"
                        >
                            <StepForward className="size-3.5" />
                        </Button>
                    </TooltipTrigger>
                    <TooltipContent side="bottom" className="text-xs">
                        <p>Step <kbd className="ml-1.5 font-mono text-[10px] text-muted-foreground bg-muted px-1 py-0.5 rounded">F10</kbd></p>
                    </TooltipContent>
                </Tooltip>

                <Separator orientation="vertical" className="h-4 mx-1 bg-border/60" />

                {/* Reset */}
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Button
                            variant="ghost"
                            size="icon-sm"
                            onClick={onReset}
                            disabled={isRunning}
                            className="text-amber-500 hover:text-amber-400 hover:bg-amber-500/10"
                        >
                            <RotateCcw className="size-3.5" />
                        </Button>
                    </TooltipTrigger>
                    <TooltipContent side="bottom" className="text-xs">
                        <p>Reset <kbd className="ml-1.5 font-mono text-[10px] text-muted-foreground bg-muted px-1 py-0.5 rounded">F6</kbd></p>
                    </TooltipContent>
                </Tooltip>

                {/* Refresh */}
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Button
                            variant="ghost"
                            size="icon-sm"
                            onClick={onRefresh}
                            disabled={isRunning}
                            className="text-muted-foreground hover:text-foreground hover:bg-accent"
                        >
                            <RefreshCw className="size-3.5" />
                        </Button>
                    </TooltipTrigger>
                    <TooltipContent side="bottom" className="text-xs">
                        <p>Refresh <kbd className="ml-1.5 font-mono text-[10px] text-muted-foreground bg-muted px-1 py-0.5 rounded">F9</kbd></p>
                    </TooltipContent>
                </Tooltip>

                <Separator orientation="vertical" className="h-4 mx-1 bg-border/60" />

                {/* Status indicator */}
                <div className="flex items-center gap-1.5 px-2 text-xs text-muted-foreground">
                    <div
                        className={
                            isRunning
                                ? "w-2 h-2 rounded-full bg-emerald-500 animate-pulse"
                                : isHalted
                                    ? "w-2 h-2 rounded-full bg-amber-500"
                                    : "w-2 h-2 rounded-full bg-muted-foreground/40"
                        }
                    />
                    <span className="font-medium text-[11px]">
                        {isRunning ? "Running" : isHalted ? "Halted" : "Ready"}
                    </span>
                </div>

                {/* LED indicator */}
                {ledState !== undefined && (
                    <div className="flex items-center gap-1.5 px-2 ml-auto">
                        <div
                            className={`w-2.5 h-2.5 rounded-full transition-all duration-150 ${ledState
                                    ? "bg-red-500 shadow-[0_0_6px_rgba(239,68,68,0.7)]"
                                    : "bg-muted-foreground/20"
                                }`}
                        />
                        <span className="text-[10px] text-muted-foreground font-medium">LED</span>
                    </div>
                )}
            </div>
        </TooltipProvider>
    );
}
