/**
 * ControlPanel Component
 *
 * Provides emulator control buttons with keyboard shortcuts
 * Modern, sleek design with visual feedback
 */

import React from "react";
import { Button } from "./ui/button";
import {
  Play,
  SkipForward,
  RotateCcw,
  RefreshCw,
  Zap,
} from "lucide-react";

export interface ControlPanelProps {
  /** Whether the emulator is running */
  isRunning?: boolean;
  /** Whether the emulator is halted */
  isHalted?: boolean;
  /** Callback when Step button is clicked */
  onStep: () => void | Promise<void>;
  /** Callback when Run button is clicked */
  onRun: () => void | Promise<void>;
  /** Callback when Reset button is clicked */
  onReset: () => void | Promise<void>;
  /** Callback when Refresh button is clicked */
  onRefresh?: () => void | Promise<void>;
  /** Optional class name for styling */
  className?: string;
}

/**
 * Main ControlPanel component
 */
export function ControlPanel({
  isRunning = false,
  isHalted = false,
  onStep,
  onRun,
  onReset,
  onRefresh,
  className,
}: ControlPanelProps) {
  return (
    <div className={className}>
      <div className="rounded-xl border border-white/5 bg-white/[0.02] overflow-hidden">
        {/* Header */}
        <div className="flex items-center gap-2 px-5 py-3 border-b border-white/5 bg-white/[0.02]">
          <Zap className="w-4 h-4 text-amber-400" />
          <h2 className="text-sm font-semibold text-slate-200">Controls</h2>
        </div>

        {/* Status */}
        <div className="px-5 py-3 border-b border-white/5">
          <p className="text-xs text-slate-500">
            {isHalted ? (
              <span className="flex items-center gap-2">
                <span className="w-1.5 h-1.5 rounded-full bg-amber-500 animate-pulse" />
                Halted - Ready to step
              </span>
            ) : isRunning ? (
              <span className="flex items-center gap-2">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                Running
              </span>
            ) : (
              <span className="flex items-center gap-2">
                <span className="w-1.5 h-1.5 rounded-full bg-slate-500" />
                Ready
              </span>
            )}
          </p>
        </div>

        {/* Control Buttons */}
        <div className="p-4 grid grid-cols-2 gap-2">
          {/* Step */}
          <Button
            onClick={onStep}
            disabled={isRunning}
            className="h-auto flex-col gap-1.5 py-3 bg-gradient-to-br from-blue-500/20 to-cyan-500/20 hover:from-blue-500/30 hover:to-cyan-500/30 border border-blue-500/30 text-blue-400 hover:text-blue-300"
            variant="outline"
          >
            <SkipForward className="w-4 h-4" />
            <span className="text-xs font-semibold">Step</span>
            <span className="text-[9px] opacity-60 font-mono">F10</span>
          </Button>

          {/* Run */}
          <Button
            onClick={onRun}
            disabled={isRunning}
            className="h-auto flex-col gap-1.5 py-3 bg-gradient-to-br from-emerald-500/20 to-green-500/20 hover:from-emerald-500/30 hover:to-green-500/30 border border-emerald-500/30 text-emerald-400 hover:text-emerald-300"
            variant="outline"
          >
            <Play className="w-4 h-4" />
            <span className="text-xs font-semibold">Run</span>
            <span className="text-[9px] opacity-60 font-mono">F5</span>
          </Button>

          {/* Reset */}
          <Button
            onClick={onReset}
            disabled={isRunning}
            className="h-auto flex-col gap-1.5 py-3 bg-gradient-to-br from-amber-500/20 to-orange-500/20 hover:from-amber-500/30 hover:to-orange-500/30 border border-amber-500/30 text-amber-400 hover:text-amber-300"
            variant="outline"
          >
            <RotateCcw className="w-4 h-4" />
            <span className="text-xs font-semibold">Reset</span>
            <span className="text-[9px] opacity-60 font-mono">F6</span>
          </Button>

          {/* Refresh */}
          <Button
            onClick={onRefresh}
            disabled={isRunning}
            className="h-auto flex-col gap-1.5 py-3 bg-gradient-to-br from-violet-500/20 to-purple-500/20 hover:from-violet-500/30 hover:to-purple-500/30 border border-violet-500/30 text-violet-400 hover:text-violet-300"
            variant="outline"
          >
            <RefreshCw className="w-4 h-4" />
            <span className="text-xs font-semibold">Refresh</span>
            <span className="text-[9px] opacity-60 font-mono">F9</span>
          </Button>
        </div>
      </div>
    </div>
  );
}
