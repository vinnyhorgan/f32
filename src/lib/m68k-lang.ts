/**
 * M68K Assembly Language Support for CodeMirror 6
 *
 * Provides syntax highlighting for Motorola 68000 assembly.
 * Uses StreamLanguage for simplicity with a hand-written tokenizer.
 */

import { StreamLanguage, StringStream } from "@codemirror/language";
import { tags } from "@lezer/highlight";

// M68K instruction mnemonics (uppercase forms; we match case-insensitively)
const INSTRUCTIONS = new Set([
    // Data Movement
    "MOVE", "MOVEA", "MOVEQ", "MOVEM", "MOVEP", "LEA", "PEA", "EXG", "SWAP",
    "LINK", "UNLK",
    // Arithmetic
    "ADD", "ADDA", "ADDI", "ADDQ", "ADDX", "SUB", "SUBA", "SUBI", "SUBQ", "SUBX",
    "MULS", "MULU", "DIVS", "DIVU", "NEG", "NEGX", "CLR", "CMP", "CMPA", "CMPI",
    "CMPM", "EXT", "EXTB", "TST",
    // Logical
    "AND", "ANDI", "OR", "ORI", "EOR", "EORI", "NOT",
    // Shift/Rotate
    "ASL", "ASR", "LSL", "LSR", "ROL", "ROR", "ROXL", "ROXR",
    // Bit Operations
    "BTST", "BSET", "BCLR", "BCHG",
    // BCD
    "ABCD", "SBCD", "NBCD",
    // Program Control
    "BRA", "BSR", "BCC", "BCS", "BEQ", "BNE", "BGE", "BGT", "BHI", "BLE",
    "BLS", "BLT", "BMI", "BPL", "BVC", "BVS", "JMP", "JSR", "RTS", "RTR", "RTE",
    "DBRA", "DBF", "DBCC", "DBCS", "DBEQ", "DBNE", "DBGE", "DBGT", "DBHI",
    "DBLE", "DBLS", "DBLT", "DBMI", "DBPL", "DBVC", "DBVS", "DBT",
    // Set
    "SCC", "SCS", "SEQ", "SF", "SGE", "SGT", "SHI", "SLE", "SLS", "SLT",
    "SMI", "SNE", "SPL", "ST", "SVC", "SVS",
    // System
    "TRAP", "TRAPV", "CHK", "NOP", "RESET", "STOP", "TAS", "ILLEGAL",
    // Privileged
    "ANDI_TO_SR", "EORI_TO_SR", "ORI_TO_SR", "MOVE_TO_SR", "MOVE_FROM_SR",
    "MOVE_TO_CCR", "MOVE_USP",
]);

// Assembler directives
const DIRECTIVES = new Set([
    "ORG", "DC", "DS", "EQU", "SET", "EVEN", "ALIGN", "INCLUDE", "INCBIN",
    "SECTION", "END", "MACRO", "ENDM", "REPT", "ENDR", "IF", "ELSE", "ENDIF",
    "IFND", "IFD", "IFC", "IFNC", "CNOP", "RS", "RSRESET", "RSSET",
    // With size suffixes
    "DC.B", "DC.W", "DC.L", "DS.B", "DS.W", "DS.L", "RS.B", "RS.W", "RS.L",
]);

// Register names
const REGISTERS = new Set([
    "D0", "D1", "D2", "D3", "D4", "D5", "D6", "D7",
    "A0", "A1", "A2", "A3", "A4", "A5", "A6", "A7",
    "SP", "USP", "SSP", "SR", "CCR", "PC",
]);

interface M68kState {
    inComment: boolean;
}

const m68kLanguage = StreamLanguage.define<M68kState>({
    name: "m68k",
    startState: () => ({ inComment: false }),
    token(stream: StringStream, _state: M68kState): string | null {
        // Skip whitespace
        if (stream.eatSpace()) return null;

        // Comments: ; or * at start of line
        if (stream.match(/^[;]/) || (stream.sol() && stream.match(/^\*/))) {
            stream.skipToEnd();
            return "lineComment";
        }

        // String literals
        if (stream.match(/^"[^"]*"/)) return "string";
        if (stream.match(/^'[^']*'/)) return "string";

        // Hex numbers: $xxxx or 0xXXXX
        if (stream.match(/^\$[0-9A-Fa-f]+/)) return "number";
        if (stream.match(/^0x[0-9A-Fa-f]+/i)) return "number";

        // Binary numbers: %xxxx
        if (stream.match(/^%[01]+/)) return "number";

        // Decimal numbers
        if (stream.match(/^[0-9]+/)) return "number";

        // Hash for immediate values
        if (stream.eat("#")) return "operator";

        // Operators
        if (stream.match(/^[+\-*/&|^~<>=!]+/)) return "operator";

        // Parentheses (for indirect addressing)
        if (stream.eat("(") || stream.eat(")")) return "paren";

        // Comma
        if (stream.eat(",")) return "separator";

        // Identifiers (instructions, labels, registers, directives)
        if (stream.match(/^\.?[A-Za-z_][A-Za-z0-9_.]*/)) {
            const word = stream.current();
            const upper = word.toUpperCase();

            // Check if it's a size suffix on instruction (e.g., MOVE.L)
            const dotIdx = upper.indexOf(".");
            const base = dotIdx >= 0 ? upper.substring(0, dotIdx) : upper;
            const suffix = dotIdx >= 0 ? upper.substring(dotIdx) : "";

            // Check for directives (with or without size)
            if (DIRECTIVES.has(upper) || DIRECTIVES.has(base)) {
                return "keyword";
            }

            // Check for registers
            if (REGISTERS.has(upper)) return "variableName.special";

            // Check for instructions (with optional .B/.W/.L suffix)
            if (INSTRUCTIONS.has(base) && (suffix === "" || suffix === ".B" || suffix === ".W" || suffix === ".L" || suffix === ".S")) {
                return "keyword";
            }

            // Check for label definition (followed by colon or at start of line)
            if (stream.peek() === ":") {
                return "labelName";
            }

            return "variableName";
        }

        // Label colon
        if (stream.eat(":")) return "punctuation";

        // Anything else, advance
        stream.next();
        return null;
    },
    languageData: {
        commentTokens: { line: ";" },
    },
    tokenTable: {
        lineComment: tags.lineComment,
        string: tags.string,
        number: tags.number,
        keyword: tags.keyword,
        operator: tags.operator,
        paren: tags.paren,
        separator: tags.separator,
        "variableName.special": tags.special(tags.variableName),
        variableName: tags.variableName,
        labelName: tags.labelName,
        punctuation: tags.punctuation,
    },
});

export function m68kAsm() {
    return m68kLanguage;
}
