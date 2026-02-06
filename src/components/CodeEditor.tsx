/**
 * CodeEditor — CodeMirror 6 based M68K Assembly Editor
 *
 * Main code editor component for writing M68K assembly programs.
 * Features: syntax highlighting, line numbers, bracket matching, search.
 */

import { useRef, useEffect, useCallback, forwardRef, useImperativeHandle } from "react";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection, rectangularSelection } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { bracketMatching, indentOnInput, foldGutter, foldKeymap } from "@codemirror/language";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { closeBrackets, closeBracketsKeymap } from "@codemirror/autocomplete";
import { m68kAsm } from "../lib/m68k-lang";
import { flux32ThemeExtension } from "../lib/editor-theme";

interface CodeEditorProps {
    /** Initial source code */
    initialValue?: string;
    /** Callback when content changes */
    onChange?: (value: string) => void;
    /** Additional CSS class */
    className?: string;
}

// Default M68K assembly example
const DEFAULT_CODE = `; ──────────────────────────────────────────
; Flux32 Assembly Program
; ──────────────────────────────────────────
; Write your M68K assembly code here.
; Use the toolbar to Assemble & Run (F5).
;
; Memory Map:
;   $000000-$0FFFFF  ROM (system)
;   $A00000          UART (serial I/O)
;   $C00000-$CFFFFF  RAM
;   $E00000-$EFFFFF  RAM mirror
;   $E00100          App load address
;
; System calls (TRAP #n):
;   TRAP #0  — Exit
;   TRAP #2  — OutChar (D0.B = char)
;   TRAP #3  — OutStr  (A0 = string ptr)
;   TRAP #4  — OutFmt  (formatted output)
;   TRAP #5  — InChar  (→ D0.B)
; ──────────────────────────────────────────

    include "app.inc"

start:
    ; Print hello message
    lea     msg(pc),a0
    sys     OutStr

    ; Print newline
    moveq   #10,d0
    sys     OutChar

    ; Exit cleanly
    sys     Exit

msg:
    dc.b    "Hello from Flux32!",0
    even
`;

/** Handle exposed by CodeEditor via ref */
export interface CodeEditorRef {
    setContent: (content: string) => void;
    getContent: () => string;
}

export const CodeEditor = forwardRef<CodeEditorRef, CodeEditorProps>(function CodeEditor({ initialValue, onChange, className }, ref) {
    const containerRef = useRef<HTMLDivElement>(null);
    const viewRef = useRef<EditorView | null>(null);
    const onChangeRef = useRef(onChange);

    // Keep onChange ref current
    onChangeRef.current = onChange;

    // Get current editor content
    const getContent = useCallback(() => {
        return viewRef.current?.state.doc.toString() ?? "";
    }, []);

    // Set content programmatically
    const setContent = useCallback((content: string) => {
        const view = viewRef.current;
        if (!view) return;
        view.dispatch({
            changes: {
                from: 0,
                to: view.state.doc.length,
                insert: content,
            },
        });
    }, []);

    // Expose methods via ref
    useImperativeHandle(ref, () => ({
        setContent,
        getContent,
    }), [setContent, getContent]);

    useEffect(() => {
        if (!containerRef.current) return;

        const updateListener = EditorView.updateListener.of((update) => {
            if (update.docChanged && onChangeRef.current) {
                onChangeRef.current(update.state.doc.toString());
            }
        });

        const state = EditorState.create({
            doc: initialValue ?? DEFAULT_CODE,
            extensions: [
                lineNumbers(),
                highlightActiveLineGutter(),
                highlightActiveLine(),
                drawSelection(),
                rectangularSelection(),
                indentOnInput(),
                bracketMatching(),
                closeBrackets(),
                foldGutter(),
                highlightSelectionMatches(),
                history(),
                EditorState.tabSize.of(8),
                EditorView.lineWrapping,
                m68kAsm(),
                ...flux32ThemeExtension,
                keymap.of([
                    ...closeBracketsKeymap,
                    ...defaultKeymap,
                    ...searchKeymap,
                    ...historyKeymap,
                    ...foldKeymap,
                    indentWithTab,
                ]),
                updateListener,
            ],
        });

        const view = new EditorView({
            state,
            parent: containerRef.current,
        });

        viewRef.current = view;

        // Emit initial content so the store is synced
        if (onChangeRef.current) {
            onChangeRef.current(view.state.doc.toString());
        }

        return () => {
            view.destroy();
            viewRef.current = null;
        };
        // Only run on mount
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    return (
        <div
            ref={containerRef}
            className={`flex-1 min-h-0 overflow-hidden [&_.cm-editor]:h-full [&_.cm-scroller]:overflow-auto ${className ?? ""}`}
        />
    );
});
