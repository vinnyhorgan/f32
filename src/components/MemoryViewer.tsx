/**
 * MemoryViewer Component
 *
 * Displays a hex dump of memory with a modern, sleek design
 * - Address column
 * - Hex bytes (16 per line)
 * - ASCII representation
 * - Navigation controls
 */

import React, { useState, useEffect } from "react";
import { Button } from "./ui/button";
import { ChevronUp, ChevronDown, ArrowUp, ArrowDown, Database } from "lucide-react";
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

const BYTES_PER_LINE = 16;
const LINE_COUNT = 16;

interface MemoryLine {
  address: number;
  bytes: number[];
  ascii: string;
}

/**
 * Format a number as hex with zero-padding
 */
function formatHex(value: number, digits: number = 8): string {
  return value.toString(16).toUpperCase().padStart(digits, "0");
}

/**
 * Convert bytes to ASCII representation (or . for non-printable)
 */
function bytesToAscii(bytes: number[]): string {
  return bytes
    .map((b) => (b >= 32 && b < 127 ? String.fromCharCode(b) : "."))
    .join("");
}

/**
 * Parse address input string to number
 */
function parseAddressInput(input: string): number | null {
  const trimmed = input.trim();
  let address: number;

  if (trimmed.startsWith("$") || trimmed.startsWith("0x")) {
    address = parseInt(trimmed.slice(1), 16);
  } else {
    address = parseInt(trimmed, 10);
  }

  if (isNaN(address) || address < 0 || address > 0xffffff) {
    return null;
  }

  return address;
}

/**
 * Main MemoryViewer component
 */
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

  /**
   * Load memory at current address
   */
  const loadMemory = async (address: number) => {
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
  };

  /**
   * Navigation handlers
   */
  const goToPreviousPage = () => {
    const newAddress = Math.max(0, currentAddress - displayLength);
    loadMemory(newAddress);
  };

  const goToNextPage = () => {
    const newAddress = currentAddress + displayLength;
    if (newAddress <= 0xffffff - displayLength) {
      loadMemory(newAddress);
    }
  };

  const goToPreviousLine = () => {
    const newAddress = Math.max(0, currentAddress - BYTES_PER_LINE);
    loadMemory(newAddress);
  };

  const goToNextLine = () => {
    const newAddress = currentAddress + BYTES_PER_LINE;
    if (newAddress <= 0xffffff - displayLength) {
      loadMemory(newAddress);
    }
  };

  const handleAddressSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const address = parseAddressInput(addressInput);
    if (address !== null) {
      loadMemory(address);
    } else {
      setError("Invalid address. Use $hex or 0xhex format, e.g., $1000");
    }
  };

  // Load initial memory
  useEffect(() => {
    loadMemory(initialAddress);
  }, [initialAddress]);

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
          <Database className="w-4 h-4 text-purple-400" />
          <h2 className="text-sm font-semibold text-slate-200">Memory Viewer</h2>
        </div>

        {/* Address input & navigation */}
        <div className="flex items-center gap-2">
          <form onSubmit={handleAddressSubmit} className="flex items-center gap-2">
            <input
              type="text"
              value={addressInput}
              onChange={(e) => setAddressInput(e.target.value)}
              className="w-28 px-2.5 py-1.5 text-xs font-mono bg-black/20 border border-white/10 rounded-lg text-slate-300 focus:outline-none focus:border-purple-500/50 focus:ring-2 focus:ring-purple-500/20 transition-all"
              placeholder="Address"
            />
            <Button type="submit" size="sm" variant="outline" className="h-8 px-3">
              Go
            </Button>
          </form>

          {/* Navigation */}
          <div className="flex gap-1 ml-2">
            <Button
              size="sm"
              variant="ghost"
              onClick={goToPreviousPage}
              disabled={loading || currentAddress === 0}
              title="Previous page (Page Up)"
              className="h-8 w-8 p-0"
            >
              <ChevronUp className="h-4 w-4" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={goToPreviousLine}
              disabled={loading || currentAddress === 0}
              title="Previous line (Up)"
              className="h-8 w-8 p-0"
            >
              <ArrowUp className="h-4 w-4" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={goToNextLine}
              disabled={loading || currentAddress > 0xffffff - displayLength}
              title="Next line (Down)"
              className="h-8 w-8 p-0"
            >
              <ArrowDown className="h-4 w-4" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={goToNextPage}
              disabled={loading || currentAddress > 0xffffff - displayLength}
              title="Next page (Page Down)"
              className="h-8 w-8 p-0"
            >
              <ChevronDown className="h-4 w-4" />
            </Button>
          </div>

          {/* Refresh */}
          <Button
            size="sm"
            variant="outline"
            onClick={() => loadMemory(currentAddress)}
            disabled={loading}
            className="h-8 px-3"
          >
            {loading ? "..." : "Refresh"}
          </Button>
        </div>
      </div>

      {/* Error display */}
      {error && (
        <div className="mx-5 mt-3 px-3 py-2 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400 text-xs">
          {error}
        </div>
      )}

      {/* Memory display */}
      <div className="p-4">
        <div className="bg-black/40 rounded-lg border border-white/5 p-3 font-mono text-xs overflow-auto max-h-[500px]">
          {lines.length === 0 ? (
            <div className="text-slate-600 text-center py-8">
              {loading ? "Loading memory..." : "No memory data"}
            </div>
          ) : (
            <div className="space-y-0.5">
              {lines.map((line) => (
                <div
                  key={line.address}
                  className="flex gap-4 hover:bg-white/[0.02] px-2 py-0.5 rounded transition-colors"
                >
                  {/* Address */}
                  <span className="text-purple-400 select-none shrink-0 w-20">
                    {formatHex(line.address)}:
                  </span>

                  {/* Hex bytes */}
                  <div className="flex gap-1 flex-wrap flex-1">
                    {line.bytes.map((byte, i) => (
                      <span
                        key={i}
                        className={cn(
                          "w-5 text-center transition-colors",
                          byte === 0
                            ? "text-slate-700"
                            : byte === 0xff
                              ? "text-slate-600"
                              : "text-slate-300",
                        )}
                      >
                        {formatHex(byte, 2)}
                      </span>
                    ))}
                  </div>

                  {/* ASCII */}
                  <span className="text-emerald-400 select-none shrink-0 ml-auto">
                    |{line.ascii}|
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Status bar */}
      <div className="px-5 py-2 border-t border-white/5 bg-white/[0.02]">
        <div className="flex items-center justify-between text-[10px] text-slate-600 font-mono">
          <span>
            ${formatHex(currentAddress)} - ${formatHex(currentAddress + displayLength - 1)}
          </span>
          <span>{displayLength} bytes</span>
        </div>
      </div>
    </div>
  );
}
