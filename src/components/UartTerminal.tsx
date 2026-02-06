/**
 * UartTerminal — Serial I/O display for the Flux32 UART
 *
 * Shows UART TX output as a scrolling terminal, with input capability.
 */

import { useRef, useEffect, useCallback, useState } from "react";

interface UartTerminalProps {
    /** UART output text buffer */
    output: string;
    /** Callback when user types a character */
    onInput?: (char: number) => void;
    /** CSS class */
    className?: string;
}

export function UartTerminal({ output, onInput, className }: UartTerminalProps) {
    const termRef = useRef<HTMLDivElement>(null);
    const inputRef = useRef<HTMLInputElement>(null);
    const [focused, setFocused] = useState(false);

    // Auto-scroll to bottom when output changes
    useEffect(() => {
        if (termRef.current) {
            termRef.current.scrollTop = termRef.current.scrollHeight;
        }
    }, [output]);

    // Handle keyboard input
    const handleKeyDown = useCallback(
        (e: React.KeyboardEvent) => {
            if (!onInput) return;

            if (e.key === "Enter") {
                e.preventDefault();
                onInput(13); // CR
                onInput(10); // LF
            } else if (e.key === "Backspace") {
                e.preventDefault();
                onInput(8);
            } else if (e.key === "Escape") {
                e.preventDefault();
                onInput(27);
            } else if (e.key.length === 1) {
                e.preventDefault();
                onInput(e.key.charCodeAt(0));
            }
        },
        [onInput],
    );

    // Focus the hidden input when clicking the terminal
    const handleClick = useCallback(() => {
        inputRef.current?.focus();
    }, []);

    // Convert output to rendered lines, handling \r\n and basic control chars
    const renderedOutput = output
        .replace(/\r\n/g, "\n")
        .replace(/\r/g, "\n");

    return (
        <div
            className={`relative flex flex-col bg-[oklch(0.10_0.005_260)] ${className ?? ""}`}
            onClick={handleClick}
        >
            <div
                ref={termRef}
                className="flex-1 min-h-0 overflow-auto p-3 font-mono text-[13px] leading-[1.5] text-[oklch(0.85_0.12_145)] whitespace-pre-wrap break-all select-text"
            >
                {renderedOutput || (
                    <span className="text-[oklch(0.4_0.01_260)] italic">
                        UART output will appear here...
                    </span>
                )}
                {focused && (
                    <span className="inline-block w-[7px] h-[15px] bg-[oklch(0.85_0.12_145)] animate-pulse ml-[1px] align-text-bottom" />
                )}
            </div>
            {/* Hidden input for capturing keystrokes */}
            <input
                ref={inputRef}
                className="absolute opacity-0 w-0 h-0 pointer-events-none"
                onKeyDown={handleKeyDown}
                onFocus={() => setFocused(true)}
                onBlur={() => setFocused(false)}
                aria-label="UART terminal input"
            />
        </div>
    );
}
