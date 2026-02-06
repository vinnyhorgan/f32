/**
 * Flux32 Dark Theme for CodeMirror 6
 *
 * A custom dark theme that matches the Flux32 desktop application aesthetic.
 */

import { EditorView } from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags } from "@lezer/highlight";

/**
 * Editor theme (structural styles)
 */
export const flux32Theme = EditorView.theme(
    {
        "&": {
            color: "oklch(0.9 0.005 260)",
            backgroundColor: "oklch(0.13 0.005 260)",
            fontFamily: "'JetBrains Mono', 'Cascadia Code', 'Fira Code', 'SF Mono', ui-monospace, monospace",
            fontSize: "13px",
            lineHeight: "1.55",
        },
        ".cm-content": {
            caretColor: "oklch(0.65 0.18 250)",
            padding: "8px 0",
        },
        ".cm-cursor, .cm-dropCursor": {
            borderLeftColor: "oklch(0.65 0.18 250)",
            borderLeftWidth: "2px",
        },
        "&.cm-focused .cm-cursor": {
            borderLeftColor: "oklch(0.75 0.18 250)",
        },
        ".cm-activeLine": {
            backgroundColor: "oklch(0.18 0.008 260 / 0.5)",
        },
        ".cm-selectionMatch": {
            backgroundColor: "oklch(0.35 0.05 250 / 0.3)",
        },
        "&.cm-focused .cm-selectionBackground, ::selection": {
            backgroundColor: "oklch(0.3 0.08 250 / 0.4)",
        },
        ".cm-gutters": {
            backgroundColor: "oklch(0.14 0.005 260)",
            color: "oklch(0.45 0.01 260)",
            borderRight: "1px solid oklch(0.22 0.008 260)",
            minWidth: "48px",
        },
        ".cm-activeLineGutter": {
            backgroundColor: "oklch(0.18 0.008 260 / 0.5)",
            color: "oklch(0.7 0.01 260)",
        },
        ".cm-lineNumbers .cm-gutterElement": {
            padding: "0 8px 0 12px",
            minWidth: "28px",
        },
        ".cm-foldGutter .cm-gutterElement": {
            padding: "0 4px",
        },
        ".cm-tooltip": {
            backgroundColor: "oklch(0.17 0.008 260)",
            border: "1px solid oklch(0.25 0.008 260)",
            color: "oklch(0.9 0.005 260)",
        },
        ".cm-tooltip-autocomplete": {
            "& > ul > li": {
                padding: "2px 8px",
            },
            "& > ul > li[aria-selected]": {
                backgroundColor: "oklch(0.25 0.02 250)",
                color: "oklch(0.95 0.005 260)",
            },
        },
        ".cm-panels": {
            backgroundColor: "oklch(0.16 0.005 260)",
            color: "oklch(0.85 0.005 260)",
        },
        ".cm-panels.cm-panels-top": {
            borderBottom: "1px solid oklch(0.25 0.008 260)",
        },
        ".cm-panels.cm-panels-bottom": {
            borderTop: "1px solid oklch(0.25 0.008 260)",
        },
        ".cm-searchMatch": {
            backgroundColor: "oklch(0.45 0.15 80 / 0.3)",
            outline: "1px solid oklch(0.55 0.15 80 / 0.5)",
        },
        ".cm-searchMatch.cm-searchMatch-selected": {
            backgroundColor: "oklch(0.45 0.15 80 / 0.5)",
        },
        ".cm-matchingBracket": {
            backgroundColor: "oklch(0.3 0.05 250 / 0.3)",
            outline: "1px solid oklch(0.5 0.1 250 / 0.5)",
        },
    },
    { dark: true },
);

/**
 * Syntax highlighting colors
 */
export const flux32Highlight = HighlightStyle.define([
    // Keywords / instructions / directives — blue
    { tag: tags.keyword, color: "oklch(0.72 0.17 250)", fontWeight: "500" },
    // Comments — dim muted
    { tag: tags.lineComment, color: "oklch(0.5 0.02 260)", fontStyle: "italic" },
    { tag: tags.blockComment, color: "oklch(0.5 0.02 260)", fontStyle: "italic" },
    // Strings — warm orange
    { tag: tags.string, color: "oklch(0.75 0.15 55)" },
    // Numbers — teal/cyan
    { tag: tags.number, color: "oklch(0.78 0.14 175)" },
    // Registers — special gold
    { tag: tags.special(tags.variableName), color: "oklch(0.80 0.15 85)", fontWeight: "500" },
    // Labels/identifiers — light foreground
    { tag: tags.variableName, color: "oklch(0.85 0.03 260)" },
    { tag: tags.labelName, color: "oklch(0.80 0.12 300)" },
    // Operators
    { tag: tags.operator, color: "oklch(0.7 0.1 250)" },
    // Punctuation
    { tag: tags.punctuation, color: "oklch(0.6 0.02 260)" },
    { tag: tags.paren, color: "oklch(0.7 0.05 250)" },
    { tag: tags.separator, color: "oklch(0.6 0.02 260)" },
]);

/**
 * Combined theme extension (structure + syntax)
 */
export const flux32ThemeExtension = [
    flux32Theme,
    syntaxHighlighting(flux32Highlight),
];
