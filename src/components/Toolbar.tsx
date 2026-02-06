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
} from "lucide-react";

interface ToolbarProps {
    isRunning: boolean;
    isHalted: boolean;
    onStep: () => void;
    onRun: () => void;
    onReset: () => void;
    onRefresh: () => void;
}

/** A compact, icon-centric debug toolbar */
export function Toolbar({
    isRunning,
    isHalted,
    onStep,
    onRun,
    onReset,
    onRefresh,
}: ToolbarProps) {
    return (
        <TooltipProvider delayDuration={400}>
            <div
                data-no-select
                className="shrink-0 flex items-center gap-0.5 h-9 px-2 border-b border-border bg-muted/30"
            >
                {/* Debug controls */}
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Button
                            variant="ghost"
                            size="icon-sm"
                            onClick={onRun}
                            disabled={isRunning}
                            className="text-emerald-500 hover:text-emerald-400 hover:bg-emerald-500/10"
                        >
                            {isRunning ? (
                                <Square className="size-3.5" />
                            ) : (
                                <Play className="size-3.5" />
                            )}
                        </Button>
                    </TooltipTrigger>
                    <TooltipContent side="bottom" className="text-xs">
                        <p>Run <kbd className="ml-1.5 font-mono text-[10px] text-muted-foreground bg-muted px-1 py-0.5 rounded">F5</kbd></p>
                    </TooltipContent>
                </Tooltip>

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
            </div>
        </TooltipProvider>
    );
}
