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
        className="border-border bg-muted/30 flex h-9 shrink-0 items-center gap-0.5 border-b px-2"
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
                  className="text-emerald-500 hover:bg-emerald-500/10 hover:text-emerald-400"
                >
                  <Rocket className="size-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom" className="text-xs">
                <p>
                  Assemble &amp; Run{" "}
                  <kbd className="text-muted-foreground bg-muted ml-1.5 rounded px-1 py-0.5 font-mono text-[10px]">
                    F5
                  </kbd>
                </p>
              </TooltipContent>
            </Tooltip>

            <Separator
              orientation="vertical"
              className="bg-border/60 mx-1 h-4"
            />
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
              className="text-sky-400 hover:bg-sky-500/10 hover:text-sky-300"
            >
              {isRunning ? (
                <Square className="size-3.5" />
              ) : (
                <FastForward className="size-3.5" />
              )}
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom" className="text-xs">
            <p>
              Continue{" "}
              <kbd className="text-muted-foreground bg-muted ml-1.5 rounded px-1 py-0.5 font-mono text-[10px]">
                F8
              </kbd>
            </p>
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
              className="text-blue-400 hover:bg-blue-500/10 hover:text-blue-300"
            >
              <StepForward className="size-3.5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom" className="text-xs">
            <p>
              Step{" "}
              <kbd className="text-muted-foreground bg-muted ml-1.5 rounded px-1 py-0.5 font-mono text-[10px]">
                F10
              </kbd>
            </p>
          </TooltipContent>
        </Tooltip>

        <Separator orientation="vertical" className="bg-border/60 mx-1 h-4" />

        {/* Reset */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={onReset}
              disabled={isRunning}
              className="text-amber-500 hover:bg-amber-500/10 hover:text-amber-400"
            >
              <RotateCcw className="size-3.5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom" className="text-xs">
            <p>
              Reset{" "}
              <kbd className="text-muted-foreground bg-muted ml-1.5 rounded px-1 py-0.5 font-mono text-[10px]">
                F6
              </kbd>
            </p>
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
            <p>
              Refresh{" "}
              <kbd className="text-muted-foreground bg-muted ml-1.5 rounded px-1 py-0.5 font-mono text-[10px]">
                F9
              </kbd>
            </p>
          </TooltipContent>
        </Tooltip>

        <Separator orientation="vertical" className="bg-border/60 mx-1 h-4" />

        {/* Status indicator */}
        <div className="text-muted-foreground flex items-center gap-1.5 px-2 text-xs">
          <div
            className={
              isRunning
                ? "h-2 w-2 animate-pulse rounded-full bg-emerald-500"
                : isHalted
                  ? "h-2 w-2 rounded-full bg-amber-500"
                  : "bg-muted-foreground/40 h-2 w-2 rounded-full"
            }
          />
          <span className="text-[11px] font-medium">
            {isRunning ? "Running" : isHalted ? "Halted" : "Ready"}
          </span>
        </div>

        {/* LED indicator */}
        {ledState !== undefined && (
          <div className="ml-auto flex items-center gap-1.5 px-2">
            <div
              className={`h-2.5 w-2.5 rounded-full transition-all duration-150 ${
                ledState
                  ? "bg-red-500 shadow-[0_0_6px_rgba(239,68,68,0.7)]"
                  : "bg-muted-foreground/20"
              }`}
            />
            <span className="text-muted-foreground text-[10px] font-medium">
              LED
            </span>
          </div>
        )}
      </div>
    </TooltipProvider>
  );
}
