/**
 * RegisterDisplay Component
 *
 * Displays the M68K CPU register state with a modern, sleek design
 */

import React from "react";
import type { CpuState } from "../lib/emulator-types";
import { cn } from "../lib/utils";
import { Cpu, Shield, Flag } from "lucide-react";

interface RegisterDisplayProps {
  cpuState: CpuState | null;
  className?: string;
}

/**
 * Format a number as hexadecimal
 */
function formatHex(value: number, digits: number = 8): string {
  return value.toString(16).toUpperCase().padStart(digits, "0");
}

/**
 * Parse SR flags into individual flag states
 */
function parseFlags(sr: number) {
  return {
    C: (sr & 0x0001) !== 0,
    V: (sr & 0x0002) !== 0,
    Z: (sr & 0x0004) !== 0,
    N: (sr & 0x0008) !== 0,
    X: (sr & 0x0010) !== 0,
    S: (sr & 0x2000) !== 0,
    I0: (sr & 0x0100) !== 0,
    I1: (sr & 0x0200) !== 0,
    I2: (sr & 0x0400) !== 0,
  };
}

/**
 * Single register display cell
 */
function RegisterCell({
  label,
  value,
  highlight,
}: {
  label: string;
  value: string;
  highlight?: boolean;
}) {
  return (
    <div
      className={cn(
        "group relative flex items-center gap-2 px-3 py-2 rounded-lg transition-all duration-200",
        highlight
          ? "bg-blue-500/10 border border-blue-500/20"
          : "bg-white/[0.02] border border-white/5 hover:border-white/10"
      )}
    >
      <span className="text-[10px] font-bold text-slate-500 w-6 uppercase">
        {label}
      </span>
      <span className="font-mono text-sm text-slate-300 tracking-wider">
        {value}
      </span>
    </div>
  );
}

/**
 * Flag indicator component
 */
function FlagIndicator({
  label,
  set,
}: {
  label: string;
  set: boolean;
}) {
  return (
    <div
      className={cn(
        "flex items-center justify-center w-6 h-6 rounded text-[10px] font-bold transition-all duration-200",
        set
          ? "bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 shadow-sm shadow-emerald-500/10"
          : "bg-white/[0.02] text-slate-600 border border-white/5"
      )}
    >
      {label}
    </div>
  );
}

/**
 * Main RegisterDisplay component
 */
export function RegisterDisplay({ cpuState, className }: RegisterDisplayProps) {
  if (!cpuState) {
    return (
      <div
        className={cn(
          "p-5 rounded-xl border border-white/5 bg-white/[0.02]",
          className,
        )}
      >
        <div className="flex items-center gap-2 text-slate-500 text-xs">
          <Cpu className="w-4 h-4" />
          <span>No CPU state available</span>
        </div>
      </div>
    );
  }

  const flags = parseFlags(cpuState.sr);
  const interruptMask = ((cpuState.sr >> 8) & 0x7).toString();

  return (
    <div
      className={cn(
        "rounded-xl border border-white/5 bg-white/[0.02] overflow-hidden",
        className,
      )}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-3 border-b border-white/5 bg-white/[0.02]">
        <div className="flex items-center gap-2">
          <Cpu className="w-4 h-4 text-blue-400" />
          <h2 className="text-sm font-semibold text-slate-200">CPU Registers</h2>
        </div>
        <div
          className={cn(
            "flex items-center gap-1.5 px-2 py-1 rounded-md text-[10px] font-semibold border transition-all duration-200",
            flags.S
              ? "bg-purple-500/20 text-purple-400 border-purple-500/30"
              : "bg-white/[0.02] text-slate-500 border-white/5",
          )}
        >
          <Shield className="w-3 h-3" />
          {flags.S ? "SUPERVISOR" : "USER"}
        </div>
      </div>

      <div className="p-4 space-y-4">
        {/* Data Registers */}
        <div>
          <div className="text-[10px] font-semibold text-slate-500 mb-2 uppercase tracking-wider flex items-center gap-1.5">
            <div className="w-1.5 h-1.5 rounded-full bg-blue-400" />
            Data Registers
          </div>
          <div className="grid grid-cols-4 gap-2">
            {cpuState.d.map((value, i) => (
              <RegisterCell
                key={`D${i}`}
                label={`D${i}`}
                value={formatHex(value)}
                highlight={value !== 0}
              />
            ))}
          </div>
        </div>

        {/* Address Registers */}
        <div>
          <div className="text-[10px] font-semibold text-slate-500 mb-2 uppercase tracking-wider flex items-center gap-1.5">
            <div className="w-1.5 h-1.5 rounded-full bg-cyan-400" />
            Address Registers
          </div>
          <div className="grid grid-cols-4 gap-2">
            {cpuState.a.map((value, i) => (
              <RegisterCell
                key={`A${i}`}
                label={`A${i}`}
                value={formatHex(value)}
                highlight={value !== 0}
              />
            ))}
          </div>
        </div>

        {/* Special Registers */}
        <div>
          <div className="text-[10px] font-semibold text-slate-500 mb-2 uppercase tracking-wider flex items-center gap-1.5">
            <div className="w-1.5 h-1.5 rounded-full bg-amber-400" />
            Special Registers
          </div>
          <div className="grid grid-cols-2 gap-2">
            <RegisterCell label="PC" value={formatHex(cpuState.pc)} highlight />
            <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-white/[0.02] border border-white/5">
              <span className="text-[10px] font-bold text-slate-500 w-6 uppercase">
                SR
              </span>
              <span className="font-mono text-sm text-slate-300 tracking-wider">
                {formatHex(cpuState.sr, 4)}
              </span>
            </div>
            <RegisterCell
              label="USP"
              value={formatHex(cpuState.usp)}
              highlight={cpuState.usp !== 0}
            />
            <RegisterCell
              label="SSP"
              value={formatHex(cpuState.ssp)}
              highlight={cpuState.ssp !== 0}
            />
          </div>
        </div>

        {/* Condition Codes & Interrupt */}
        <div className="grid grid-cols-2 gap-4">
          {/* Flags */}
          <div>
            <div className="text-[10px] font-semibold text-slate-500 mb-2 uppercase tracking-wider flex items-center gap-1.5">
              <Flag className="w-3 h-3" />
              Condition Codes
            </div>
            <div className="flex flex-wrap gap-1">
              {["X", "N", "Z", "V", "C"].map((flag) => {
                const flagSet = flags[flag as keyof typeof flags];
                return (
                  <div
                    key={flag}
                    className="flex items-center gap-1"
                  >
                    <FlagIndicator label={flag} set={flagSet} />
                  </div>
                );
              })}
            </div>
          </div>

          {/* Interrupt Priority */}
          <div>
            <div className="text-[10px] font-semibold text-slate-500 mb-2 uppercase tracking-wider">
              Interrupt Priority
            </div>
            <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-white/[0.02] border border-white/5">
              <span className="text-[10px] text-slate-500">IPL</span>
              <span className="font-mono text-sm text-slate-300">
                {interruptMask}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
