/**
 * RegisterDisplay Component
 *
 * Compact, tabular display of M68K CPU registers.
 * Shows data registers, address registers, special registers, and flags
 * in a dense, monospace layout suitable for a desktop debugger.
 */

import type { CpuState } from "../lib/emulator-types";
import { cn } from "../lib/utils";

interface RegisterDisplayProps {
  cpuState: CpuState | null;
  className?: string;
}

/** Format a number as hexadecimal with zero-padding */
function formatHex(value: number, digits: number = 8): string {
  return value.toString(16).toUpperCase().padStart(digits, "0");
}

/** Parse SR flags into individual flag booleans */
function parseFlags(sr: number) {
  return {
    C: (sr & 0x0001) !== 0,
    V: (sr & 0x0002) !== 0,
    Z: (sr & 0x0004) !== 0,
    N: (sr & 0x0008) !== 0,
    X: (sr & 0x0010) !== 0,
    S: (sr & 0x2000) !== 0,
  };
}

/** Single flag badge */
function FlagBadge({ label, active }: { label: string; active: boolean }) {
  return (
    <span
      className={cn(
        "inline-flex items-center justify-center w-5 h-5 rounded text-[10px] font-bold font-mono transition-colors",
        active
          ? "bg-primary/20 text-primary border border-primary/30"
          : "bg-muted/50 text-muted-foreground/40 border border-transparent"
      )}
    >
      {label}
    </span>
  );
}

/** A single register row: label + hex value */
function RegisterRow({
  label,
  value,
  changed,
}: {
  label: string;
  value: string;
  changed?: boolean;
}) {
  return (
    <div className="flex items-center gap-2 px-2 py-[3px] rounded-sm hover:bg-accent/50 transition-colors group">
      <span className="text-[11px] font-mono font-semibold text-muted-foreground w-7 shrink-0">
        {label}
      </span>
      <span
        className={cn(
          "text-[12px] font-mono tracking-wider",
          changed ? "text-primary" : "text-foreground/80"
        )}
      >
        {value}
      </span>
    </div>
  );
}

/** Compact section header */
function SectionHeader({ children }: { children: React.ReactNode }) {
  return (
    <div className="px-2 pt-2 pb-1 text-[10px] font-semibold text-muted-foreground uppercase tracking-widest">
      {children}
    </div>
  );
}

/** Main RegisterDisplay component */
export function RegisterDisplay({ cpuState, className }: RegisterDisplayProps) {
  if (!cpuState) {
    return (
      <div className={cn("p-4 text-xs text-muted-foreground", className)}>
        No CPU state available
      </div>
    );
  }

  const flags = parseFlags(cpuState.sr);
  const interruptMask = (cpuState.sr >> 8) & 0x7;

  return (
    <div className={cn("flex flex-col min-w-0 overflow-auto", className)}>
      {/* Data Registers */}
      <SectionHeader>Data</SectionHeader>
      <div className="grid grid-cols-2 gap-x-1">
        {cpuState.d.map((value, i) => (
          <RegisterRow
            key={`D${i}`}
            label={`D${i}`}
            value={formatHex(value)}
            changed={value !== 0}
          />
        ))}
      </div>

      {/* Address Registers */}
      <SectionHeader>Address</SectionHeader>
      <div className="grid grid-cols-2 gap-x-1">
        {cpuState.a.map((value, i) => (
          <RegisterRow
            key={`A${i}`}
            label={`A${i}`}
            value={formatHex(value)}
            changed={value !== 0}
          />
        ))}
      </div>

      {/* Special Registers */}
      <SectionHeader>System</SectionHeader>
      <div className="grid grid-cols-1 gap-x-1">
        <RegisterRow label="PC" value={formatHex(cpuState.pc)} changed />
        <RegisterRow label="SR" value={formatHex(cpuState.sr, 4)} />
        <RegisterRow label="USP" value={formatHex(cpuState.usp)} changed={cpuState.usp !== 0} />
        <RegisterRow label="SSP" value={formatHex(cpuState.ssp)} changed={cpuState.ssp !== 0} />
      </div>

      {/* Flags */}
      <SectionHeader>Flags</SectionHeader>
      <div className="flex items-center gap-1 px-2 pb-1">
        {(["X", "N", "Z", "V", "C"] as const).map((flag) => (
          <FlagBadge key={flag} label={flag} active={flags[flag]} />
        ))}
        <div className="ml-auto flex items-center gap-1.5 text-[10px] text-muted-foreground font-mono">
          <span>IPL</span>
          <span className={cn(
            "font-semibold",
            interruptMask > 0 ? "text-amber-400" : "text-muted-foreground/50"
          )}>
            {interruptMask}
          </span>
        </div>
      </div>

      {/* Mode badge */}
      <div className="px-2 pb-2 pt-1">
        <span
          className={cn(
            "inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-bold uppercase tracking-wider",
            flags.S
              ? "bg-purple-500/15 text-purple-400 border border-purple-500/20"
              : "bg-muted/50 text-muted-foreground/60 border border-transparent"
          )}
        >
          {flags.S ? "Supervisor" : "User"}
        </span>
      </div>
    </div>
  );
}
