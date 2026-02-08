/**
 * RegisterDisplay Component
 *
 * Compact, tabular display of M68K CPU registers.
 * Shows data registers, address registers, special registers, and flags
 * in a dense, monospace layout suitable for a desktop debugger.
 * Highlights registers that changed since last update.
 */

import { useRef, useEffect } from "react";
import type { CpuState } from "../lib/emulator-types";
import { cn } from "../lib/utils";

interface RegisterDisplayProps {
  cpuState: CpuState | null;
  className?: string;
}

/** Format a number as hexadecimal with zero-padding */
function formatHex(value: number, digits: number = 8): string {
  return value.toString(16).toLowerCase().padStart(digits, "0");
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
        "inline-flex h-5 w-5 items-center justify-center rounded font-mono text-[10px] font-bold transition-colors",
        active
          ? "bg-primary/20 text-primary border-primary/30 border"
          : "bg-muted/50 text-muted-foreground/40 border border-transparent",
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
  highlight,
}: {
  label: string;
  value: string;
  changed?: boolean;
  highlight?: "primary" | "amber" | "green";
}) {
  const colorClass = changed
    ? highlight === "amber"
      ? "text-amber-400"
      : highlight === "green"
        ? "text-emerald-400"
        : "text-primary"
    : "text-foreground/60";

  return (
    <div className="hover:bg-accent/50 group flex items-center gap-2 rounded-sm px-2 py-[3px] transition-colors">
      <span className="text-muted-foreground w-7 shrink-0 font-mono text-[11px] font-semibold">
        {label}
      </span>
      <span
        className={cn(
          "font-mono text-[12px] tracking-wider transition-colors duration-300",
          colorClass,
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
    <div className="text-muted-foreground px-2 pt-2 pb-1 text-[10px] font-semibold tracking-widest">
      {children}
    </div>
  );
}

/** Track which registers changed between updates */
function getChangedSet(prev: CpuState | null, curr: CpuState): Set<string> {
  const changed = new Set<string>();
  if (!prev) return changed;

  for (let i = 0; i < 8; i++) {
    if ((prev.d[i] ?? 0) !== (curr.d[i] ?? 0)) changed.add(`D${i}`);
  }
  for (let i = 0; i < 7; i++) {
    if ((prev.a[i] ?? 0) !== (curr.a[i] ?? 0)) changed.add(`A${i}`);
  }
  if (prev.pc !== curr.pc) changed.add("PC");
  if (prev.sr !== curr.sr) changed.add("SR");
  if (prev.usp !== curr.usp) changed.add("USP");
  if (prev.ssp !== curr.ssp) changed.add("SSP");

  return changed;
}

/** Main RegisterDisplay component */
export function RegisterDisplay({ cpuState, className }: RegisterDisplayProps) {
  const prevStateRef = useRef<CpuState | null>(null);
  const changedRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    if (cpuState) {
      changedRef.current = getChangedSet(prevStateRef.current, cpuState);
      prevStateRef.current = {
        ...cpuState,
        d: [...cpuState.d],
        a: [...cpuState.a],
      };
    }
  }, [cpuState]);

  if (!cpuState) {
    return (
      <div className={cn("text-muted-foreground p-4 text-xs", className)}>
        No CPU state available
      </div>
    );
  }

  const flags = parseFlags(cpuState.sr);
  const interruptMask = (cpuState.sr >> 8) & 0x7;
  const changed = changedRef.current;

  return (
    <div className={cn("flex min-w-0 flex-col overflow-auto", className)}>
      {/* Data Registers */}
      <SectionHeader>Data</SectionHeader>
      <div className="grid grid-cols-2 gap-x-1">
        {cpuState.d.map((value, i) => (
          <RegisterRow
            key={`D${i}`}
            label={`D${i}`}
            value={formatHex(value)}
            changed={changed.has(`D${i}`)}
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
            changed={changed.has(`A${i}`)}
          />
        ))}
      </div>

      {/* Special Registers */}
      <SectionHeader>System</SectionHeader>
      <div className="grid grid-cols-1 gap-x-1">
        <RegisterRow
          label="PC"
          value={formatHex(cpuState.pc)}
          changed={changed.has("PC")}
          highlight="green"
        />
        <RegisterRow
          label="SR"
          value={formatHex(cpuState.sr, 4)}
          changed={changed.has("SR")}
          highlight="amber"
        />
        <RegisterRow
          label="USP"
          value={formatHex(cpuState.usp)}
          changed={changed.has("USP")}
        />
        <RegisterRow
          label="SSP"
          value={formatHex(cpuState.ssp)}
          changed={changed.has("SSP")}
        />
      </div>

      {/* Flags */}
      <SectionHeader>Flags</SectionHeader>
      <div className="flex items-center gap-1 px-2 pb-1">
        {(["X", "N", "Z", "V", "C"] as const).map((flag) => (
          <FlagBadge key={flag} label={flag} active={flags[flag]} />
        ))}
        <div className="text-muted-foreground ml-auto flex items-center gap-1.5 font-mono text-[10px]">
          <span>IPL</span>
          <span
            className={cn(
              "font-semibold",
              interruptMask > 0 ? "text-amber-400" : "text-muted-foreground/50",
            )}
          >
            {interruptMask}
          </span>
        </div>
      </div>

      {/* Mode badge */}
      <div className="px-2 pt-1 pb-2">
        <span
          className={cn(
            "inline-flex items-center rounded px-1.5 py-0.5 text-[9px] font-bold tracking-wider",
            flags.S
              ? "border border-purple-500/20 bg-purple-500/15 text-purple-400"
              : "bg-muted/50 text-muted-foreground/60 border border-transparent",
          )}
        >
          {flags.S ? "Supervisor" : "User"}
        </span>
      </div>
    </div>
  );
}
