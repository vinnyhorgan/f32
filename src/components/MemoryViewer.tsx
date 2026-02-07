/**
 * MemoryViewer Component
 *
 * Hex dump display modeled after professional hex editors.
 * Dense monospace layout with address, hex bytes, and ASCII columns.
 */

import React, { useState, useEffect, useCallback } from "react";
import { Button } from "./ui/button";
import { ChevronUp, ChevronDown, ArrowUp, ArrowDown } from "lucide-react";
import { cn } from "../lib/utils";

interface MemoryViewerProps {
  /** Function to read memory bytes */
  onReadMemory: (address: number, length: number) => Promise<number[]>;
  /** Starting address to display */
  initialAddress?: number;
  /** Number of bytes to display (multiple of 16) */
  displayLength?: number;
  /** Optional class name */
  className?: string;
}

const BYTES_PER_LINE = 8;
const LINE_COUNT = 24;

interface MemoryLine {
  address: number;
  bytes: number[];
  ascii: string;
}

/** Format a number as hex with zero-padding */
function formatHex(value: number, digits: number = 8): string {
  return value.toString(16).toLowerCase().padStart(digits, "0");
}

/** Convert bytes to ASCII representation (or . for non-printable) */
function bytesToAscii(bytes: number[]): string {
  return bytes
    .map((b) => (b >= 32 && b < 127 ? String.fromCharCode(b) : "."))
    .join("");
}

/** Parse address input string to number (supports $hex, 0xhex, decimal) */
function parseAddressInput(input: string): number | null {
  const trimmed = input.trim();
  let address: number;
  if (trimmed.startsWith("$") || trimmed.startsWith("0x")) {
    address = parseInt(trimmed.replace("$", "").replace("0x", ""), 16);
  } else {
    address = parseInt(trimmed, 16);
  }
  if (isNaN(address) || address < 0 || address > 0xffffff) return null;
  return address;
}

/** Main MemoryViewer component */
export function MemoryViewer({
  onReadMemory,
  initialAddress = 0,
  displayLength = BYTES_PER_LINE * LINE_COUNT,
  className,
}: MemoryViewerProps) {
  const [currentAddress, setCurrentAddress] = useState(initialAddress);
  const [lines, setLines] = useState<MemoryLine[]>([]);
  const [addressInput, setAddressInput] = useState(formatHex(initialAddress));
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadMemory = useCallback(
    async (address: number) => {
      setLoading(true);
      setError(null);
      try {
        const alignedAddress = address & ~(BYTES_PER_LINE - 1);
        const bytes = await onReadMemory(alignedAddress, displayLength);
        const newLines: MemoryLine[] = [];
        for (let i = 0; i < bytes.length; i += BYTES_PER_LINE) {
          const lineBytes = bytes.slice(i, i + BYTES_PER_LINE);
          newLines.push({
            address: alignedAddress + i,
            bytes: lineBytes,
            ascii: bytesToAscii(lineBytes),
          });
        }
        setLines(newLines);
        setCurrentAddress(alignedAddress);
        setAddressInput(formatHex(alignedAddress));
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        setLines([]);
      } finally {
        setLoading(false);
      }
    },
    [onReadMemory, displayLength]
  );

  const goToPreviousPage = () => loadMemory(Math.max(0, currentAddress - displayLength));
  const goToNextPage = () => {
    const next = currentAddress + displayLength;
    if (next <= 0xffffff - displayLength) loadMemory(next);
  };
  const goToPreviousLine = () => loadMemory(Math.max(0, currentAddress - BYTES_PER_LINE));
  const goToNextLine = () => {
    const next = currentAddress + BYTES_PER_LINE;
    if (next <= 0xffffff - displayLength) loadMemory(next);
  };

  const handleAddressSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const address = parseAddressInput(addressInput);
    if (address !== null) {
      loadMemory(address);
    } else {
      setError("Invalid address");
    }
  };

  useEffect(() => {
    loadMemory(initialAddress);
  }, [initialAddress, loadMemory]);

  return (
    <div className={cn("flex flex-col h-full min-w-0", className)}>
      {/* Toolbar row */}
      <div
        data-no-select
        className="flex flex-col gap-1 px-2 py-1 border-b border-border bg-muted/30 shrink-0"
      >
        <div className="flex items-center gap-1.5">
          <form onSubmit={handleAddressSubmit} className="flex items-center gap-1">
            <span className="text-[10px] text-muted-foreground font-mono">$</span>
            <input
              type="text"
              value={addressInput}
              onChange={(e) => setAddressInput(e.target.value)}
              className="w-20 px-1.5 py-0.5 text-[11px] font-mono bg-background border border-border rounded text-foreground focus:outline-none focus:ring-1 focus:ring-ring"
              placeholder="Address"
            />
            <Button type="submit" size="xs" variant="secondary" className="h-5 px-1.5 text-[10px]">
              Go
            </Button>
          </form>

          <div className="flex items-center gap-0.5 ml-auto">
            <Button
              size="icon-xs"
              variant="ghost"
              onClick={goToPreviousPage}
              disabled={loading || currentAddress === 0}
              title="Page Up"
            >
              <ChevronUp className="size-3" />
            </Button>
            <Button
              size="icon-xs"
              variant="ghost"
              onClick={goToPreviousLine}
              disabled={loading || currentAddress === 0}
              title="Line Up"
            >
              <ArrowUp className="size-3" />
            </Button>
            <Button
              size="icon-xs"
              variant="ghost"
              onClick={goToNextLine}
              disabled={loading || currentAddress > 0xffffff - displayLength}
              title="Line Down"
            >
              <ArrowDown className="size-3" />
            </Button>
            <Button
              size="icon-xs"
              variant="ghost"
              onClick={goToNextPage}
              disabled={loading || currentAddress > 0xffffff - displayLength}
              title="Page Down"
            >
              <ChevronDown className="size-3" />
            </Button>
          </div>
        </div>
        {/* Quick-jump region buttons */}
        <div className="flex items-center gap-1">
          {[
            { label: "ROM", addr: 0x000000 },
            { label: "Vectors", addr: 0x000080 },
            { label: "UART", addr: 0xA00000 },
            { label: "RAM", addr: 0xC00000 },
            { label: "App", addr: 0xE00100 },
          ].map(({ label, addr }) => (
            <Button
              key={label}
              size="xs"
              variant={currentAddress === addr ? "secondary" : "ghost"}
              className="h-4 px-1.5 text-[9px] font-mono"
              onClick={() => loadMemory(addr)}
              disabled={loading}
            >
              {label}
            </Button>
          ))}
        </div>
      </div>

      {/* Error banner */}
      {error && (
        <div className="px-2 py-1 bg-destructive/10 text-destructive text-[10px] border-b border-destructive/20">
          {error}
        </div>
      )}

      {/* Hex dump table */}
      <div className="flex-1 overflow-auto bg-background">
        {lines.length === 0 ? (
          <div className="flex items-center justify-center h-full text-xs text-muted-foreground">
            {loading ? "Loading..." : "No data"}
          </div>
        ) : (
          <table className="w-full border-collapse">
            <thead className="sticky top-0 bg-muted/60 backdrop-blur-sm z-10">
              <tr className="text-[9px] font-mono text-muted-foreground">
                <th className="text-left px-2 py-1 font-medium w-[72px]">Address</th>
                <th className="text-left px-1 py-1 font-medium">
                  {/* Hex column headers */}
                  <div className="flex">
                    {Array.from({ length: BYTES_PER_LINE }, (_, i) => (
                      <span
                        key={i}
                        className={cn(
                          "w-[18px] text-center",
                          i > 0 && i % 4 === 0 ? "ml-2" : "ml-[3px]"
                        )}
                      >
                        {i.toString(16).toLowerCase()}
                      </span>
                    ))}
                  </div>
                </th>
                <th className="text-left px-2 py-1 font-medium">ASCII</th>
              </tr>
            </thead>
            <tbody>
              {lines.map((line) => (
                <tr
                  key={line.address}
                  className="hover:bg-accent/40 transition-colors"
                >
                  <td className="px-2 py-px font-mono text-[11px] text-primary/80 whitespace-nowrap">
                    {formatHex(line.address)}
                  </td>
                  <td className="px-1 py-px">
                    <div className="flex">
                      {line.bytes.map((byte, i) => (
                        <span
                          key={i}
                          className={cn(
                            "w-[18px] text-center font-mono text-[11px]",
                            i > 0 && i % 4 === 0 ? "ml-2" : "ml-[3px]",
                            byte === 0
                              ? "text-muted-foreground/20"
                              : byte === 0xff
                                ? "text-muted-foreground/40"
                                : "text-foreground/90"
                          )}
                        >
                          {formatHex(byte, 2)}
                        </span>
                      ))}
                    </div>
                  </td>
                  <td className="px-2 py-px font-mono text-[11px] text-emerald-500/70 whitespace-nowrap">
                    {line.ascii}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Bottom address range */}
      <div
        data-no-select
        className="shrink-0 flex items-center justify-between px-2 py-0.5 border-t border-border bg-muted/30 text-[10px] text-muted-foreground font-mono"
      >
        <span>
          ${formatHex(currentAddress)}-${formatHex(currentAddress + displayLength - 1)}
        </span>
        <span>{displayLength} bytes</span>
      </div>
    </div>
  );
}
