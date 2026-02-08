#!/usr/bin/env node
/**
 * M68K Assembly Formatter
 *
 * Formats .asm and .inc files with consistent style:
 * - Labels at column 0
 * - Instructions at column 17 (after tab)
 * - Operands at column 28
 * - Comments at column 80 (or after operand with padding)
 * - Consistent spacing around operators
 *
 * Usage: node scripts/format-asm.mjs [files...]
 * With --check flag: exits with error if files need formatting
 */

import fs from "fs";
import path from "path";

const INSTR_COL = 17;
const OPERAND_COL = 28;
const COMMENT_COL = 80;

// Directives
const DIRECTIVES = new Set([
  "org",
  "dc.b",
  "dc.w",
  "dc.l",
  "ds.b",
  "ds.w",
  "ds.l",
  "equ",
  "set",
  "include",
  "incbin",
  "macro",
  "endm",
  "rept",
  "endr",
  "if",
  "ifeq",
  "ifne",
  "ifgt",
  "ifge",
  "iflt",
  "ifle",
  "ifd",
  "ifnd",
  "else",
  "elseif",
  "endif",
  "endc",
  "section",
  "rsreset",
  "rsset",
  "rs.b",
  "rs.w",
  "rs.l",
  "even",
  "odd",
  "cnop",
  "align",
  "asciz",
  "ascii",
  "fail",
  "opt",
  "end",
  "litstr",
  "pushm",
  "popm",
  "bl",
  "rl",
  "sys",
  "led_on",
  "led_off",
  "led_tgl",
  "tx_char",
  "tx_wait",
]);

/**
 * Parse a line into its components
 */
function parseLine(line) {
  const result = {
    label: "",
    instruction: "",
    operand: "",
    comment: "",
    isBlank: false,
    isCommentOnly: false,
    preserveWhitespace: false,
  };

  const trimmed = line.trim();

  // Blank line
  if (trimmed === "") {
    result.isBlank = true;
    return result;
  }

  // Comment-only line
  if (trimmed.startsWith(";")) {
    result.isCommentOnly = true;
    result.comment = trimmed;
    return result;
  }

  // Check for vim modeline or special comments at start
  if (line.startsWith("; vim:") || line.match(/^;\s*[-=]+/)) {
    result.isCommentOnly = true;
    result.comment = line;
    result.preserveWhitespace = true;
    return result;
  }

  let remaining = line;
  let commentStart = -1;

  // Find comment (but not inside quotes)
  let inQuote = false;
  let quoteChar = "";
  for (let i = 0; i < remaining.length; i++) {
    const ch = remaining[i];
    if (!inQuote && (ch === '"' || ch === "'")) {
      inQuote = true;
      quoteChar = ch;
    } else if (inQuote && ch === quoteChar) {
      inQuote = false;
    } else if (!inQuote && ch === ";") {
      commentStart = i;
      break;
    }
  }

  if (commentStart >= 0) {
    result.comment = remaining.substring(commentStart).trim();
    remaining = remaining.substring(0, commentStart);
  }

  remaining = remaining.trimEnd();

  // Check if line starts with whitespace (no label)
  const hasLabel = remaining.length > 0 && !/^\s/.test(remaining);

  // Split by whitespace
  const parts = remaining.split(/\s+/).filter((p) => p !== "");

  if (parts.length === 0) {
    return result;
  }

  let idx = 0;

  // First part could be a label
  if (hasLabel) {
    const first = parts[0];
    // Labels end with : or are at column 0 followed by instruction
    if (first.endsWith(":") || first.startsWith(".")) {
      result.label = first;
      idx = 1;
    } else if (
      !isInstruction(first.toLowerCase()) &&
      !DIRECTIVES.has(first.toLowerCase())
    ) {
      // Assume it's a label if not an instruction
      result.label = first;
      idx = 1;
    }
  }

  // Next should be instruction
  if (idx < parts.length) {
    result.instruction = parts[idx];
    idx++;
  }

  // Rest is operand
  if (idx < parts.length) {
    // Reconstruct operand preserving internal spacing for strings
    const instrEnd = remaining
      .toLowerCase()
      .indexOf(result.instruction.toLowerCase());
    if (instrEnd >= 0) {
      const afterInstr = remaining.substring(
        instrEnd + result.instruction.length,
      );
      result.operand = afterInstr.trim();
    } else {
      result.operand = parts.slice(idx).join(" ");
    }
  }

  return result;
}

/**
 * Check if a string is a known instruction
 */
function isInstruction(str) {
  const base = str.replace(/\.[bwlsBWLS]$/, "").toLowerCase();
  const instrs = [
    "move",
    "moveq",
    "movea",
    "movem",
    "movep",
    "lea",
    "pea",
    "add",
    "addi",
    "addq",
    "adda",
    "addx",
    "sub",
    "subi",
    "subq",
    "suba",
    "subx",
    "muls",
    "mulu",
    "divs",
    "divu",
    "and",
    "andi",
    "or",
    "ori",
    "eor",
    "eori",
    "not",
    "neg",
    "negx",
    "clr",
    "cmp",
    "cmpi",
    "cmpa",
    "cmpm",
    "tst",
    "bra",
    "bsr",
    "bcc",
    "bcs",
    "beq",
    "bne",
    "bge",
    "bgt",
    "ble",
    "blt",
    "bhi",
    "bls",
    "bmi",
    "bpl",
    "bvc",
    "bvs",
    "jmp",
    "jsr",
    "rts",
    "rte",
    "rtr",
    "dbra",
    "dbcc",
    "dbcs",
    "dbeq",
    "dbne",
    "dbf",
    "dbt",
    "lsl",
    "lsr",
    "asl",
    "asr",
    "rol",
    "ror",
    "roxl",
    "roxr",
    "swap",
    "ext",
    "exg",
    "link",
    "unlk",
    "trap",
    "trapv",
    "nop",
    "reset",
    "stop",
    "illegal",
    "btst",
    "bset",
    "bclr",
    "bchg",
    "scc",
    "scs",
    "seq",
    "sne",
    "sge",
    "sgt",
    "sle",
    "slt",
    "shi",
    "sls",
    "smi",
    "spl",
    "svc",
    "svs",
    "sf",
    "st",
    "chk",
    "tas",
    "abcd",
    "sbcd",
    "nbcd",
  ];
  return instrs.includes(base);
}

/**
 * Format a parsed line back to string
 */
function formatLine(parsed) {
  if (parsed.isBlank) {
    return "";
  }

  if (parsed.isCommentOnly) {
    return parsed.comment;
  }

  let line = "";

  // Label
  if (parsed.label) {
    line = parsed.label;
  }

  // Instruction
  if (parsed.instruction) {
    // Pad to instruction column
    const targetCol = INSTR_COL;
    while (line.length < targetCol) {
      line += " ";
    }
    if (line.length > 0 && line.length < targetCol) {
      line += " ";
    }
    line += parsed.instruction;
  }

  // Operand
  if (parsed.operand) {
    // Pad to operand column or just add space
    const targetCol = Math.max(line.length + 1, OPERAND_COL);
    while (line.length < targetCol) {
      line += " ";
    }
    line += parsed.operand;
  }

  // Comment
  if (parsed.comment) {
    // Pad to comment column or add 2 spaces
    const targetCol = Math.max(line.length + 2, COMMENT_COL);
    while (line.length < targetCol) {
      line += " ";
    }
    line += parsed.comment;
  }

  return line.trimEnd();
}

/**
 * Format an entire file
 */
function formatFile(content) {
  const lines = content.split(/\r?\n/);
  const formatted = lines.map((line) => {
    const parsed = parseLine(line);
    if (parsed.preserveWhitespace) {
      return line;
    }
    return formatLine(parsed);
  });
  return formatted.join("\n");
}

/**
 * Get all assembly files from a path (file or directory)
 */
function getAsmFiles(inputPath) {
  const fullPath = path.resolve(inputPath);
  if (!fs.existsSync(fullPath)) {
    return [];
  }

  const stat = fs.statSync(fullPath);
  if (stat.isDirectory()) {
    // Get all .asm and .inc files in directory
    const entries = fs.readdirSync(fullPath);
    return entries
      .filter((f) => f.endsWith(".asm") || f.endsWith(".inc"))
      .map((f) => path.join(fullPath, f));
  }
  return [fullPath];
}

/**
 * Main entry point
 */
function main() {
  const args = process.argv.slice(2);
  const checkOnly = args.includes("--check");
  const inputs = args.filter((a) => !a.startsWith("--"));

  if (inputs.length === 0) {
    process.stderr.write(
      "Usage: format-asm.mjs [--check] <files or directories...>\n",
    );
    process.exit(1);
  }

  // Expand inputs to actual files
  const files = inputs.flatMap((input) => getAsmFiles(input));

  if (files.length === 0) {
    process.stderr.write("No .asm or .inc files found\n");
    process.exit(1);
  }

  let hasErrors = false;

  for (const fullPath of files) {
    const relativePath = path.relative(process.cwd(), fullPath);
    const content = fs.readFileSync(fullPath, "utf-8");
    const formatted = formatFile(content);

    if (checkOnly) {
      if (content !== formatted) {
        process.stderr.write(`${relativePath} needs formatting\n`);
        hasErrors = true;
      }
    } else {
      if (content !== formatted) {
        fs.writeFileSync(fullPath, formatted, "utf-8");
        process.stdout.write(`Formatted: ${relativePath}\n`);
      }
    }
  }

  if (hasErrors) {
    process.exit(1);
  }
}

main();
