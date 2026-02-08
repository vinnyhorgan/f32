/**
 * Flux32 Main Application
 *
 * M68K Emulator IDE — desktop-style layout with:
 * - MenuBar (top)
 * - Toolbar (debug controls + assemble button)
 * - Code Editor (left main area)
 * - Right panel: tabs for Registers / Memory / UART
 * - StatusBar (bottom)
 */

import { useEffect, useCallback, useState, useRef } from "react";
import { AppMenuBar } from "./components/AppMenuBar";
import { Toolbar } from "./components/Toolbar";
import { StatusBar } from "./components/StatusBar";
import { RegisterDisplay } from "./components/RegisterDisplay";
import { MemoryViewer } from "./components/MemoryViewer";
import { CodeEditor, type CodeEditorRef } from "./components/CodeEditor";
import { UartTerminal } from "./components/UartTerminal";
import { useEmulatorStore } from "./lib/emulator-store";
import { EmulatorAPI } from "./lib/emulator-api";
import { ScrollArea } from "./components/ui/scroll-area";
import { getCurrentWindow } from "@tauri-apps/api/window";

type RightTab = "registers" | "memory" | "uart";

/** Example programs available from File > Examples */
const EXAMPLES: Record<string, string> = {
  hello: `; Hello World — prints a message via UART
    include "app.inc"

start:
    lea     msg(pc),a0
    sys     OutStr
    moveq   #10,d0
    sys     OutChar
    sys     Exit

msg:
    dc.b    "Hello from Flux32!",0
    even
`,
  chars: `; Print Characters — demonstrates OutChar syscall
    include "app.inc"

start:
    ; Print uppercase alphabet A-Z
    moveq   #26,d2          ; counter
    moveq   #'A',d0         ; start character
.loop:
    sys     OutChar         ; print character
    addq.b  #1,d0           ; next char
    subq.w  #1,d2           ; decrement counter
    bne.s   .loop           ; loop until done

    ; Print newline
    moveq   #13,d0
    sys     OutChar
    moveq   #10,d0
    sys     OutChar

    sys     Exit
`,
  count: `; Count to 10 — demonstrates looping and OutChar
    include "app.inc"

start:
    moveq   #1,d3           ; counter

.loop:
    ; Print counter digit (1-9)
    move.l  d3,d0
    add.b   #'0',d0         ; make ASCII
    sys     OutChar

    ; Print space
    moveq   #' ',d0
    sys     OutChar

    ; Next
    addq.w  #1,d3
    cmpi.w  #9,d3
    ble.s   .loop

    ; Print "10" as two characters
    moveq   #'1',d0
    sys     OutChar
    moveq   #'0',d0
    sys     OutChar

    ; Newline and exit
    moveq   #10,d0
    sys     OutChar
    sys     Exit
`,
  memory: `; Memory Operations — demonstrates data in RAM
    include "app.inc"

start:
    ; Store values in memory
    lea     buffer(pc),a0
    move.l  #$DEADBEEF,(a0)+
    move.l  #$CAFEBABE,(a0)+
    move.l  #$12345678,(a0)+

    ; Print confirmation
    lea     msg(pc),a0
    sys     OutStr
    moveq   #10,d0
    sys     OutChar

    sys     Exit

msg:
    dc.b    "Memory written at buffer!",0
    even
buffer:
    ds.l    4               ; reserve 4 longs
`,
};

function App() {
  const {
    initialized,
    cpuState,
    status,
    error,
    assemblyError,
    loading,
    memoryAddress,
    uartOutput,
    ledState,
    init,
    step,
    run,
    reset,
    refresh,
    clearError,
    pollUart,
    sendUartChar,
    setSourceCode,
    assembleAndRun,
    clearUart,
  } = useEmulatorStore();

  const [rightTab, setRightTab] = useState<RightTab>("registers");
  const editorRef = useRef<CodeEditorRef>(null);
  const isHalted = status?.halted ?? false;

  const handleRun = useCallback(() => run(100000), [run]);

  const handleAssembleAndRun = useCallback(() => {
    assembleAndRun();
  }, [assembleAndRun]);

  const handleStep = useCallback(async () => {
    await step();
    await pollUart();
  }, [step, pollUart]);

  const handleLoadExample = useCallback(
    (name: string) => {
      const code = EXAMPLES[name];
      if (code && editorRef.current) {
        editorRef.current.setContent(code);
        setSourceCode(code);
      }
    },
    [setSourceCode],
  );

  /** Initialize emulator on mount */
  useEffect(() => {
    // Start backend initialization
    if (!initialized) {
      init().catch(console.error);
    }
  }, [initialized, init]);

  /** Show window after brief delay to ensure content is painted */
  useEffect(() => {
    // Show window safely
    const win = getCurrentWindow();
    const showWindow = async () => {
      try {
        await win.show();
        await win.setFocus();
      } catch (err) {
        console.error("Failed to show window:", err);
      }
    };

    // Small delay to ensure React has painted
    const timer = setTimeout(showWindow, 50);
    return () => clearTimeout(timer);
  }, []);

  /** Global keyboard shortcuts */
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (loading) return;

      switch (e.key) {
        case "F5":
          e.preventDefault();
          handleAssembleAndRun();
          break;
        case "F10":
          e.preventDefault();
          handleStep();
          break;
        case "F6":
          e.preventDefault();
          reset();
          break;
        case "F8":
          e.preventDefault();
          handleRun();
          break;
        case "F9":
          e.preventDefault();
          refresh();
          break;
        case "Escape":
          if (error || assemblyError) {
            e.preventDefault();
            clearError();
          }
          break;
        default:
          if (e.ctrlKey && e.shiftKey) {
            if (e.code === "Digit1") {
              e.preventDefault();
              setRightTab("registers");
            }
            if (e.code === "Digit2") {
              e.preventDefault();
              setRightTab("memory");
            }
            if (e.code === "Digit3") {
              e.preventDefault();
              setRightTab("uart");
            }
          }
          break;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [
    loading,
    handleAssembleAndRun,
    handleStep,
    handleRun,
    reset,
    refresh,
    error,
    assemblyError,
    clearError,
  ]);

  const handleEditorChange = useCallback(
    (value: string) => {
      setSourceCode(value);
    },
    [setSourceCode],
  );

  const displayError = assemblyError || error;

  return (
    <div className="bg-background text-foreground flex h-screen w-full flex-col overflow-hidden">
      {/* Menu bar */}
      <AppMenuBar
        isRunning={loading}
        onStep={handleStep}
        onRun={handleRun}
        onReset={reset}
        onRefresh={refresh}
        onAssembleAndRun={handleAssembleAndRun}
        onLoadExample={handleLoadExample}
        onSwitchTab={(tab) => setRightTab(tab as RightTab)}
      />

      {/* Toolbar */}
      <Toolbar
        isRunning={loading}
        isHalted={isHalted}
        onStep={handleStep}
        onRun={handleRun}
        onReset={reset}
        onRefresh={refresh}
        onAssembleAndRun={handleAssembleAndRun}
        ledState={ledState}
      />

      {/* Error banner */}
      {displayError && (
        <div className="bg-destructive/10 border-destructive/20 text-destructive flex shrink-0 items-center justify-between border-b px-3 py-1.5 font-mono text-xs">
          <span className="truncate">{displayError}</span>
          <button
            onClick={clearError}
            className="text-muted-foreground hover:text-foreground ml-2 shrink-0 text-[10px] tracking-wider transition-colors"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Main content: Editor (left) + Panels (right) */}
      <div className="flex min-h-0 flex-1">
        {/* Left — Code Editor */}
        <div className="flex min-w-0 flex-1 flex-col">
          <div
            data-no-select
            className="border-border bg-muted/40 flex h-7 shrink-0 items-center border-b px-3"
          >
            <span className="text-muted-foreground text-[11px] font-semibold tracking-wider">
              Editor
            </span>
            <span className="text-muted-foreground/60 ml-2 text-[10px]">
              M68K Assembly
            </span>
          </div>
          <div className="flex min-h-0 flex-1 flex-col">
            <CodeEditor ref={editorRef} onChange={handleEditorChange} />
          </div>
        </div>

        {/* Right panel — tabbed */}
        <div className="border-border bg-card flex w-[340px] shrink-0 flex-col border-l">
          {/* Tab bar */}
          <div
            data-no-select
            className="border-border bg-muted/40 flex h-7 shrink-0 items-center border-b"
          >
            {(["registers", "memory", "uart"] as RightTab[]).map((tab) => (
              <button
                key={tab}
                onClick={() => setRightTab(tab)}
                className={`h-full border-b-2 px-3 text-[11px] font-semibold tracking-wider transition-colors ${
                  rightTab === tab
                    ? "text-primary border-primary bg-background/50"
                    : "text-muted-foreground hover:text-foreground hover:bg-muted/30 border-transparent"
                }`}
              >
                {tab}
                {tab === "uart" && uartOutput && (
                  <span className="ml-1.5 inline-block h-1.5 w-1.5 rounded-full bg-[oklch(0.65_0.15_145)]" />
                )}
              </button>
            ))}
          </div>

          {/* Tab content */}
          <div className="flex min-h-0 flex-1 flex-col">
            {rightTab === "registers" && (
              <ScrollArea className="flex-1">
                <RegisterDisplay cpuState={cpuState} />
              </ScrollArea>
            )}

            {rightTab === "memory" && (
              <MemoryViewer
                className="min-h-0 flex-1"
                onReadMemory={async (address, length) => {
                  const result = await EmulatorAPI.readMemory(address, length);
                  if (result.status === "success") return result.data;
                  throw new Error(result.error);
                }}
                initialAddress={memoryAddress}
                displayLength={192}
              />
            )}

            {rightTab === "uart" && (
              <div className="flex min-h-0 flex-1 flex-col">
                <UartTerminal
                  output={uartOutput}
                  onInput={sendUartChar}
                  className="min-h-0 flex-1"
                />
                <div
                  data-no-select
                  className="border-border bg-muted/30 flex h-6 shrink-0 items-center justify-between border-t px-2"
                >
                  <span className="text-muted-foreground text-[10px]">
                    {uartOutput.length} chars
                  </span>
                  <button
                    onClick={clearUart}
                    className="text-muted-foreground hover:text-foreground text-[10px] transition-colors"
                  >
                    Clear
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Status bar */}
      <StatusBar
        status={status}
        cpuState={cpuState}
        initialized={initialized}
        error={displayError}
        loading={loading}
        ledState={ledState}
      />
    </div>
  );
}

export default App;
