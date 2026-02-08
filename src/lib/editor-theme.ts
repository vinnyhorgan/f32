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
      color: "#c0caf5",
      backgroundColor: "#16161e",
      fontFamily:
        "'JetBrains Mono', 'Cascadia Code', 'Fira Code', 'SF Mono', ui-monospace, monospace",
      fontSize: "13px",
      lineHeight: "1.55",
    },
    ".cm-content": {
      caretColor: "#7aa2f7",
      padding: "8px 0",
    },
    ".cm-cursor, .cm-dropCursor": {
      borderLeftColor: "#7aa2f7",
      borderLeftWidth: "2px",
    },
    "&.cm-focused .cm-cursor": {
      borderLeftColor: "#7aa2f7",
    },
    ".cm-activeLine": {
      backgroundColor: "#292e4280", // 50% opacity
    },
    ".cm-selectionMatch": {
      backgroundColor: "#515c7e4d", // 30% opacity
    },
    "&.cm-focused .cm-selectionBackground, ::selection": {
      backgroundColor: "#515c7e66", // 40% opacity
    },
    ".cm-gutters": {
      backgroundColor: "#16161e",
      color: "#565f89",
      borderRight: "1px solid #1a1b26",
      minWidth: "48px",
    },
    ".cm-activeLineGutter": {
      backgroundColor: "#292e4280",
      color: "#7aa2f7",
    },
    ".cm-lineNumbers .cm-gutterElement": {
      padding: "0 8px 0 12px",
      minWidth: "28px",
    },
    ".cm-foldGutter .cm-gutterElement": {
      padding: "0 4px",
    },
    ".cm-tooltip": {
      backgroundColor: "#1a1b26",
      border: "1px solid #292e42",
      color: "#c0caf5",
    },
    ".cm-tooltip-autocomplete": {
      "& > ul > li": {
        padding: "2px 8px",
      },
      "& > ul > li[aria-selected]": {
        backgroundColor: "#7aa2f7",
        color: "#16161e",
      },
    },
    ".cm-panels": {
      backgroundColor: "#1a1b26",
      color: "#c0caf5",
    },
    ".cm-panels.cm-panels-top": {
      borderBottom: "1px solid #292e42",
    },
    ".cm-panels.cm-panels-bottom": {
      borderTop: "1px solid #292e42",
    },
    ".cm-searchMatch": {
      backgroundColor: "#515c7e4d",
      outline: "1px solid #7aa2f7",
    },
    ".cm-searchMatch.cm-searchMatch-selected": {
      backgroundColor: "#515c7e80",
    },
    ".cm-matchingBracket": {
      backgroundColor: "#515c7e4d",
      outline: "1px solid #7aa2f7",
    },
  },
  { dark: true },
);

/**
 * Syntax highlighting colors
 */
export const flux32Highlight = HighlightStyle.define([
  // Keywords / instructions / directives — Magenta / Purple
  { tag: tags.keyword, color: "#bb9af7", fontWeight: "500" },
  // Comments — dim muted
  { tag: tags.lineComment, color: "#565f89", fontStyle: "italic" },
  { tag: tags.blockComment, color: "#565f89", fontStyle: "italic" },
  // Strings — Green
  { tag: tags.string, color: "#9ece6a" },
  // Numbers — Orange
  { tag: tags.number, color: "#ff9e64" },
  // Registers — Yellow/Gold
  { tag: tags.special(tags.variableName), color: "#e0af68", fontWeight: "500" },
  // Labels/identifiers — Blue/Foreground
  { tag: tags.variableName, color: "#c0caf5" },
  { tag: tags.labelName, color: "#7dcfff" }, // Cyan for labels
  // Operators — Cyan
  { tag: tags.operator, color: "#89ddff" },
  // Punctuation — Foreground
  { tag: tags.punctuation, color: "#c0caf5" },
  { tag: tags.paren, color: "#c0caf5" },
  { tag: tags.separator, color: "#c0caf5" },
]);

/**
 * Combined theme extension (structure + syntax)
 */
export const flux32ThemeExtension = [
  flux32Theme,
  syntaxHighlighting(flux32Highlight),
];
