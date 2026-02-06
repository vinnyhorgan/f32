//! M68K Assembler for Flux32
//!
//! A complete M68000 assembler with Motorola syntax support. This module provides:
//! - Full M68000 instruction set encoding
//! - Macro processor with REPT, IF/ELSE/ENDIF, and parameterized macros
//! - Expression evaluator for complex constant expressions
//! - Include file handling
//! - Two-pass assembly for forward reference resolution
//!
//! The assembler produces raw binary output suitable for direct execution
//! on the Flux32 emulator or real M68K hardware.

use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// TOKEN TYPES
// ============================================================================

/// Source location for error reporting.
#[derive(Debug, Clone, Default)]
pub struct SourceLoc {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for SourceLoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

/// Token types produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Some variants reserved for future expression features
pub enum Token {
    /// Identifier (label, instruction, register name)
    Ident(String),
    /// Numeric literal (already parsed to value)
    Number(i64),
    /// String literal (without quotes)
    String(String),
    /// Single character literal
    Char(char),
    /// Newline (significant for line-based parsing)
    Newline,
    /// Comma separator
    Comma,
    /// Colon (label terminator)
    Colon,
    /// Hash (immediate prefix)
    Hash,
    /// Dot (size suffix separator)
    Dot,
    /// Left parenthesis
    LParen,
    /// Right parenthesis
    RParen,
    /// Plus operator
    Plus,
    /// Minus operator
    Minus,
    /// Asterisk (multiply or current PC)
    Star,
    /// Slash (divide)
    Slash,
    /// Ampersand (bitwise AND)
    Ampersand,
    /// Pipe (bitwise OR)
    Pipe,
    /// Caret (bitwise XOR)
    Caret,
    /// Tilde (bitwise NOT)
    Tilde,
    /// Left shift (<<)
    LShift,
    /// Right shift (>>)
    RShift,
    /// Equals (comparison or assignment)
    Equals,
    /// Not equals (<>)
    NotEquals,
    /// Less than
    Less,
    /// Greater than
    Greater,
    /// Less than or equal (<=)
    LessEq,
    /// Greater than or equal (>=)
    GreaterEq,
    /// Percent (binary number prefix when standalone, modulo otherwise)
    Percent,
    /// Dollar (hex number prefix)
    Dollar,
    /// Backslash (macro parameter prefix)
    Backslash,
    /// At sign (unique label suffix in macros)
    At,
    /// End of file
    Eof,
}

/// A token with its source location.
#[derive(Debug, Clone)]
pub struct LocatedToken {
    pub token: Token,
    pub loc: SourceLoc,
}

// ============================================================================
// LEXER
// ============================================================================

/// Lexer for M68K assembly source code.
pub struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    file: String,
    line: usize,
    line_start: usize,
    current_pos: usize,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer for the given source code.
    pub fn new(source: &'a str, file: impl Into<String>) -> Self {
        Self {
            chars: source.char_indices().peekable(),
            file: file.into(),
            line: 1,
            line_start: 0,
            current_pos: 0,
        }
    }

    /// Returns the current source location (of the next character to be read).
    fn loc(&self) -> SourceLoc {
        // Get the position of the next character to be read
        let next_pos = self
            .chars
            .clone()
            .next()
            .map(|(p, _)| p)
            .unwrap_or(self.current_pos + 1);
        SourceLoc {
            file: self.file.clone(),
            line: self.line,
            column: next_pos.saturating_sub(self.line_start) + 1,
        }
    }

    /// Peeks at the next character without consuming it.
    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    /// Consumes and returns the next character.
    fn next_char(&mut self) -> Option<char> {
        if let Some((pos, c)) = self.chars.next() {
            self.current_pos = pos;
            if c == '\n' {
                self.line += 1;
                self.line_start = pos + 1;
            }
            Some(c)
        } else {
            None
        }
    }

    /// Skips whitespace (but not newlines).
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c == ' ' || c == '\t' || c == '\r' {
                self.next_char();
            } else {
                break;
            }
        }
    }

    /// Skips a comment (from ; or * to end of line).
    fn skip_comment(&mut self) {
        while let Some(c) = self.peek_char() {
            if c == '\n' {
                break;
            }
            self.next_char();
        }
    }

    /// Reads an identifier or keyword.
    fn read_ident(&mut self, first: char) -> String {
        let mut s = String::new();
        s.push(first);
        while let Some(c) = self.peek_char() {
            if c.is_ascii_alphanumeric() || c == '_' {
                s.push(c);
                self.next_char();
            } else {
                break;
            }
        }
        s
    }

    /// Reads a decimal number.
    fn read_decimal(&mut self, first: char) -> Result<i64, String> {
        let mut s = String::new();
        s.push(first);
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                s.push(c);
                self.next_char();
            } else {
                break;
            }
        }
        s.parse::<i64>()
            .map_err(|_| format!("invalid decimal number: {}", s))
    }

    /// Reads a hexadecimal number (after $ prefix).
    fn read_hex(&mut self) -> Result<i64, String> {
        let mut s = String::new();
        while let Some(c) = self.peek_char() {
            if c.is_ascii_hexdigit() {
                s.push(c);
                self.next_char();
            } else {
                break;
            }
        }
        if s.is_empty() {
            return Err("expected hexadecimal digits after $".to_string());
        }
        i64::from_str_radix(&s, 16).map_err(|_| format!("invalid hex number: ${}", s))
    }

    /// Reads a binary number (after % prefix).
    fn read_binary(&mut self) -> Result<i64, String> {
        let mut s = String::new();
        while let Some(c) = self.peek_char() {
            if c == '0' || c == '1' {
                s.push(c);
                self.next_char();
            } else {
                break;
            }
        }
        if s.is_empty() {
            return Err("expected binary digits after %".to_string());
        }
        i64::from_str_radix(&s, 2).map_err(|_| format!("invalid binary number: %{}", s))
    }

    /// Reads a string literal (after opening quote).
    fn read_string(&mut self, quote: char) -> Result<String, String> {
        let mut s = String::new();
        loop {
            match self.next_char() {
                None => return Err("unterminated string literal".to_string()),
                Some(c) if c == quote => break,
                Some('\\') => {
                    // Handle escape sequences
                    match self.next_char() {
                        Some('n') => s.push('\n'),
                        Some('r') => s.push('\r'),
                        Some('t') => s.push('\t'),
                        Some('0') => s.push('\0'),
                        Some('\\') => s.push('\\'),
                        Some(c) if c == quote => s.push(c),
                        Some(c) => s.push(c),
                        None => return Err("unterminated escape sequence".to_string()),
                    }
                }
                Some(c) => s.push(c),
            }
        }
        Ok(s)
    }

    /// Reads the next token from the source.
    pub fn next_token(&mut self) -> Result<LocatedToken, String> {
        self.skip_whitespace();

        let loc = self.loc();
        let c = match self.next_char() {
            None => {
                return Ok(LocatedToken {
                    token: Token::Eof,
                    loc,
                })
            }
            Some(c) => c,
        };

        let token = match c {
            '\n' => Token::Newline,
            ',' => Token::Comma,
            ':' => Token::Colon,
            '#' => Token::Hash,
            '.' => {
                // Could be .label (local label) or just dot for size suffix
                // Local labels can start with a digit: .1, .2, etc.
                if let Some(c2) = self.peek_char() {
                    if c2.is_ascii_alphanumeric() || c2 == '_' {
                        // Peek ahead to see if it's a single-letter size suffix
                        // or a local label
                        let first = self.next_char().unwrap();
                        let rest = self.read_ident(first);

                        // If just a single letter (b, w, l, s), treat as potential size suffix
                        let lower = rest.to_ascii_lowercase();
                        if lower == "b" || lower == "w" || lower == "l" || lower == "s" {
                            // Return it as .X identifier - parser will decide
                            return Ok(LocatedToken {
                                token: Token::Ident(format!(".{}", rest)),
                                loc,
                            });
                        } else {
                            // Multi-character or starts with digit: local label
                            return Ok(LocatedToken {
                                token: Token::Ident(format!(".{}", rest)),
                                loc,
                            });
                        }
                    }
                }
                Token::Dot
            }
            '(' => Token::LParen,
            ')' => Token::RParen,
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => {
                // Could be comment at start of line or multiply
                if loc.column == 1 {
                    self.skip_comment();
                    Token::Newline
                } else {
                    Token::Star
                }
            }
            '/' => Token::Slash,
            '&' => Token::Ampersand,
            '|' => Token::Pipe,
            '^' => Token::Caret,
            '~' => Token::Tilde,
            '<' => {
                if self.peek_char() == Some('<') {
                    self.next_char();
                    Token::LShift
                } else if self.peek_char() == Some('=') {
                    self.next_char();
                    Token::LessEq
                } else if self.peek_char() == Some('>') {
                    self.next_char();
                    Token::NotEquals
                } else {
                    Token::Less
                }
            }
            '>' => {
                if self.peek_char() == Some('>') {
                    self.next_char();
                    Token::RShift
                } else if self.peek_char() == Some('=') {
                    self.next_char();
                    Token::GreaterEq
                } else {
                    Token::Greater
                }
            }
            '=' => Token::Equals,
            '%' => {
                // Binary number
                let n = self.read_binary()?;
                Token::Number(n)
            }
            '$' => {
                // Hex number
                let n = self.read_hex()?;
                Token::Number(n)
            }
            '\\' => Token::Backslash,
            '@' => Token::At,
            ';' => {
                self.skip_comment();
                Token::Newline
            }
            '"' => {
                let s = self.read_string('"')?;
                Token::String(s)
            }
            '\'' => {
                // Character literal or string
                let s = self.read_string('\'')?;
                if s.len() == 1 {
                    Token::Char(s.chars().next().unwrap())
                } else {
                    Token::String(s)
                }
            }
            c if c.is_ascii_digit() => {
                let n = self.read_decimal(c)?;
                Token::Number(n)
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let s = self.read_ident(c);
                Token::Ident(s)
            }
            c => return Err(format!("unexpected character: '{}'", c)),
        };

        Ok(LocatedToken { token, loc })
    }

    /// Tokenizes the entire source into a vector of tokens.
    pub fn tokenize(&mut self) -> Result<Vec<LocatedToken>, String> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.token == Token::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }
}

// ============================================================================
// EXPRESSION EVALUATOR
// ============================================================================

/// Expression AST node.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal number
    Number(i64),
    /// Symbol reference
    Symbol(String),
    /// Current program counter (*)
    CurrentPc,
    /// Unary negation
    Neg(Box<Expr>),
    /// Bitwise NOT
    Not(Box<Expr>),
    /// Binary operation
    BinOp(Box<Expr>, BinOp, Box<Expr>),
}

/// Binary operators in expressions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

impl BinOp {
    /// Returns operator precedence (higher = tighter binding).
    fn precedence(self) -> u8 {
        match self {
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => 1,
            BinOp::Or => 2,
            BinOp::Xor => 3,
            BinOp::And => 4,
            BinOp::Shl | BinOp::Shr => 5,
            BinOp::Add | BinOp::Sub => 6,
            BinOp::Mul | BinOp::Div | BinOp::Mod => 7,
        }
    }
}

/// Expression parser operating on a token slice.
pub struct ExprParser<'a> {
    tokens: &'a [LocatedToken],
    pos: usize,
}

impl<'a> ExprParser<'a> {
    pub fn new(tokens: &'a [LocatedToken]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|t| &t.token)
    }

    fn advance(&mut self) -> Option<&Token> {
        if self.pos < self.tokens.len() {
            let t = &self.tokens[self.pos].token;
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }

    fn current_loc(&self) -> SourceLoc {
        self.tokens
            .get(self.pos)
            .map(|t| t.loc.clone())
            .unwrap_or_default()
    }

    /// Parses an expression with operator precedence.
    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_binary(0)
    }

    fn parse_binary(&mut self, min_prec: u8) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                Some(Token::Star) => BinOp::Mul,
                Some(Token::Slash) => BinOp::Div,
                Some(Token::Percent) => BinOp::Mod,
                Some(Token::Ampersand) => BinOp::And,
                Some(Token::Pipe) => BinOp::Or,
                Some(Token::Caret) => BinOp::Xor,
                Some(Token::LShift) => BinOp::Shl,
                Some(Token::RShift) => BinOp::Shr,
                Some(Token::Equals) => BinOp::Eq,
                Some(Token::NotEquals) => BinOp::Ne,
                Some(Token::Less) => BinOp::Lt,
                Some(Token::Greater) => BinOp::Gt,
                Some(Token::LessEq) => BinOp::Le,
                Some(Token::GreaterEq) => BinOp::Ge,
                _ => break,
            };

            let prec = op.precedence();
            if prec < min_prec {
                break;
            }

            self.advance();
            let right = self.parse_binary(prec + 1)?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Token::Minus) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Neg(Box::new(expr)))
            }
            Some(Token::Tilde) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Not(Box::new(expr)))
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Token::Number(n)) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Some(Token::Char(c)) => {
                self.advance();
                Ok(Expr::Number(c as i64))
            }
            Some(Token::Ident(s)) => {
                self.advance();
                Ok(Expr::Symbol(s))
            }
            Some(Token::Star) => {
                self.advance();
                Ok(Expr::CurrentPc)
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expr()?;
                if self.peek() != Some(&Token::RParen) {
                    return Err("expected ')' in expression".to_string());
                }
                self.advance();
                Ok(expr)
            }
            _ => Err(format!(
                "unexpected token in expression at {}",
                self.current_loc()
            )),
        }
    }
}

/// Evaluates an expression given a symbol table, current PC, and optional local label scope.
pub fn eval_expr(expr: &Expr, symbols: &HashMap<String, i64>, pc: u32) -> Result<i64, String> {
    eval_expr_scoped(expr, symbols, pc, None)
}

/// Evaluates an expression with local label scope support.
pub fn eval_expr_scoped(
    expr: &Expr,
    symbols: &HashMap<String, i64>,
    pc: u32,
    scope: Option<&str>,
) -> Result<i64, String> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Symbol(name) => {
            // First try the name as-is
            if let Some(&v) = symbols.get(name) {
                return Ok(v);
            }
            // If it's a local label and we have a scope, try expanded form
            if name.starts_with('.') {
                if let Some(scope) = scope {
                    let expanded = format!("{}{}", scope, name);
                    if let Some(&v) = symbols.get(&expanded) {
                        return Ok(v);
                    }
                }
            }
            Err(format!("undefined symbol: {}", name))
        }
        Expr::CurrentPc => Ok(pc as i64),
        Expr::Neg(e) => Ok(-eval_expr_scoped(e, symbols, pc, scope)?),
        Expr::Not(e) => Ok(!eval_expr_scoped(e, symbols, pc, scope)?),
        Expr::BinOp(l, op, r) => {
            let lv = eval_expr_scoped(l, symbols, pc, scope)?;
            let rv = eval_expr_scoped(r, symbols, pc, scope)?;
            Ok(match op {
                BinOp::Add => lv.wrapping_add(rv),
                BinOp::Sub => lv.wrapping_sub(rv),
                BinOp::Mul => lv.wrapping_mul(rv),
                BinOp::Div => {
                    if rv == 0 {
                        return Err("division by zero".to_string());
                    }
                    lv / rv
                }
                BinOp::Mod => {
                    if rv == 0 {
                        return Err("modulo by zero".to_string());
                    }
                    lv % rv
                }
                BinOp::And => lv & rv,
                BinOp::Or => lv | rv,
                BinOp::Xor => lv ^ rv,
                BinOp::Shl => lv << (rv & 63),
                BinOp::Shr => ((lv as u64) >> (rv & 63)) as i64,
                BinOp::Eq => (lv == rv) as i64,
                BinOp::Ne => (lv != rv) as i64,
                BinOp::Lt => (lv < rv) as i64,
                BinOp::Gt => (lv > rv) as i64,
                BinOp::Le => (lv <= rv) as i64,
                BinOp::Ge => (lv >= rv) as i64,
            })
        }
    }
}

// ============================================================================
// SYMBOL TABLE
// ============================================================================

/// Symbol table for the assembler.
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// Symbol values (labels, EQU constants).
    symbols: HashMap<String, i64>,
    /// Current local label scope (most recent global label).
    local_scope: String,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets a symbol value, resolving local labels to their full name.
    #[cfg(test)]
    pub fn get(&self, name: &str) -> Option<i64> {
        // First try as-is (for full names)
        if let Some(&v) = self.symbols.get(name) {
            return Some(v);
        }
        // For local labels, try with current scope
        if name.starts_with('.') && !self.local_scope.is_empty() {
            let full_name = format!("{}{}", self.local_scope, name);
            return self.symbols.get(&full_name).copied();
        }
        None
    }

    /// Defines a symbol. Returns error if already defined with different value.
    pub fn define(&mut self, name: &str, value: i64) -> Result<(), String> {
        let full_name = if name.starts_with('.') {
            format!("{}{}", self.local_scope, name)
        } else {
            // Update local scope for global labels
            self.local_scope = name.to_string();
            name.to_string()
        };

        // In two-pass assembly, pass 2 redefines all symbols - allow this
        // The pass 2 values are the correct ones
        self.symbols.insert(full_name, value);
        Ok(())
    }

    /// Returns reference to inner HashMap for expression evaluation.
    pub fn as_map(&self) -> &HashMap<String, i64> {
        &self.symbols
    }
}

// ============================================================================
// OPERAND SIZE
// ============================================================================

/// Operand size for instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Size {
    Byte,
    #[default]
    Word,
    Long,
}

impl Size {
    /// Parses size from suffix string (B, W, L, or S for short).
    pub fn from_suffix(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "B" => Some(Size::Byte),
            "W" => Some(Size::Word),
            "L" => Some(Size::Long),
            "S" => Some(Size::Word), // Short is word for branches
            _ => None,
        }
    }

    /// Returns size in bytes.
    pub fn bytes(self) -> usize {
        match self {
            Size::Byte => 1,
            Size::Word => 2,
            Size::Long => 4,
        }
    }
}

// ============================================================================
// ADDRESSING MODES
// ============================================================================

/// M68K addressing modes.
#[derive(Debug, Clone)]
pub enum AddrMode {
    /// Data register direct: Dn
    DataReg(u8),
    /// Address register direct: An
    AddrReg(u8),
    /// Address register indirect: (An)
    AddrInd(u8),
    /// Post-increment: (An)+
    PostInc(u8),
    /// Pre-decrement: -(An)
    PreDec(u8),
    /// Displacement: d(An)
    Disp(Expr, u8),
    /// Index: d(An,Xn.s)
    Index(Expr, u8, u8, Size, bool), // disp, An, Xn, size, is_addr_reg
    /// Absolute short: addr.W
    AbsShort(Expr),
    /// Absolute long: addr.L
    AbsLong(Expr),
    /// PC relative: d(PC)
    PcDisp(Expr),
    /// PC relative with index: d(PC,Xn.s)
    PcIndex(Expr, u8, Size, bool), // disp, Xn, size, is_addr_reg
    /// Immediate: #imm
    Immediate(Expr),
    /// Status register
    Sr,
    /// Condition code register
    Ccr,
    /// User stack pointer
    Usp,
}

/// Parses a register name, returns (register number, is_address_reg).
fn parse_register(name: &str) -> Option<(u8, bool)> {
    let upper = name.to_ascii_uppercase();
    if upper == "SP" || upper == "A7" {
        return Some((7, true));
    }
    if upper.len() == 2 {
        let reg_num = upper.chars().nth(1)?.to_digit(10)? as u8;
        if reg_num > 7 {
            return None;
        }
        match upper.chars().next()? {
            'D' => Some((reg_num, false)),
            'A' => Some((reg_num, true)),
            _ => None,
        }
    } else {
        None
    }
}

/// Parses a register list like "d0-d3/a0-a2" into a bitmask.
fn parse_register_list(s: &str) -> Result<u16, String> {
    let mut mask = 0u16;
    for part in s.split('/') {
        let part = part.trim();
        if part.contains('-') {
            // Range like d0-d3
            let mut iter = part.split('-');
            let start = iter.next().ok_or("invalid register range")?;
            let end = iter.next().ok_or("invalid register range")?;
            let (start_num, start_is_addr) =
                parse_register(start).ok_or_else(|| format!("invalid register: {}", start))?;
            let (end_num, end_is_addr) =
                parse_register(end).ok_or_else(|| format!("invalid register: {}", end))?;
            if start_is_addr != end_is_addr {
                return Err("register range must be same type".to_string());
            }
            let base = if start_is_addr { 8 } else { 0 };
            for i in start_num..=end_num {
                mask |= 1 << (base + i);
            }
        } else {
            // Single register
            let (num, is_addr) =
                parse_register(part).ok_or_else(|| format!("invalid register: {}", part))?;
            let bit = if is_addr { 8 + num } else { num };
            mask |= 1 << bit;
        }
    }
    Ok(mask)
}

// ============================================================================
// INSTRUCTION ENCODING
// ============================================================================

/// Try to evaluate an expression, returning 0 if symbols are undefined (for pass 1).
/// Returns the value if successful, or 0 if a symbol is undefined.
fn try_eval_expr(
    expr: &Expr,
    symbols: &HashMap<String, i64>,
    pc: u32,
    pass: u8,
    scope: Option<&str>,
) -> Result<i64, String> {
    if pass == 1 {
        // In pass 1, return 0 for any undefined symbols - we just need sizes
        match eval_expr_scoped(expr, symbols, pc, scope) {
            Ok(v) => Ok(v),
            Err(_) => Ok(0), // Use placeholder
        }
    } else {
        eval_expr_scoped(expr, symbols, pc, scope)
    }
}

/// Encodes an effective address into mode/reg fields and extension words.
pub fn encode_ea(
    mode: &AddrMode,
    symbols: &HashMap<String, i64>,
    pc: u32,
    pass: u8,
    scope: Option<&str>,
) -> Result<(u8, u8, Vec<u16>), String> {
    match mode {
        AddrMode::DataReg(r) => Ok((0b000, *r, vec![])),
        AddrMode::AddrReg(r) => Ok((0b001, *r, vec![])),
        AddrMode::AddrInd(r) => Ok((0b010, *r, vec![])),
        AddrMode::PostInc(r) => Ok((0b011, *r, vec![])),
        AddrMode::PreDec(r) => Ok((0b100, *r, vec![])),
        AddrMode::Disp(expr, r) => {
            let disp = try_eval_expr(expr, symbols, pc, pass, scope)? as i16;
            Ok((0b101, *r, vec![disp as u16]))
        }
        AddrMode::Index(expr, an, xn, sz, is_addr) => {
            let disp = try_eval_expr(expr, symbols, pc, pass, scope)? as i8;
            let xr = if *is_addr { 0x8000 } else { 0 };
            let xs = if *sz == Size::Long { 0x0800 } else { 0 };
            let ext = xr | xs | ((*xn as u16) << 12) | ((disp as u8) as u16);
            Ok((0b110, *an, vec![ext]))
        }
        AddrMode::AbsShort(expr) => {
            let addr = try_eval_expr(expr, symbols, pc, pass, scope)? as i16;
            Ok((0b111, 0b000, vec![addr as u16]))
        }
        AddrMode::AbsLong(expr) => {
            let addr = try_eval_expr(expr, symbols, pc, pass, scope)? as u32;
            Ok((0b111, 0b001, vec![(addr >> 16) as u16, addr as u16]))
        }
        AddrMode::PcDisp(expr) => {
            let target = try_eval_expr(expr, symbols, pc, pass, scope)?;
            let disp = (target - (pc as i64 + 2)) as i16;
            Ok((0b111, 0b010, vec![disp as u16]))
        }
        AddrMode::PcIndex(expr, xn, sz, is_addr) => {
            let target = try_eval_expr(expr, symbols, pc, pass, scope)?;
            let disp = (target - (pc as i64 + 2)) as i8;
            let xr = if *is_addr { 0x8000 } else { 0 };
            let xs = if *sz == Size::Long { 0x0800 } else { 0 };
            let ext = xr | xs | ((*xn as u16) << 12) | ((disp as u8) as u16);
            Ok((0b111, 0b011, vec![ext]))
        }
        AddrMode::Immediate(_) => {
            // Immediate mode encoding varies by instruction size, handled separately
            Err("immediate mode must be handled by instruction encoder".to_string())
        }
        _ => Err("cannot encode addressing mode as EA".to_string()),
    }
}

/// Parses an addressing mode from tokens.
pub fn parse_operand(
    tokens: &[LocatedToken],
    symbols: &HashMap<String, i64>,
) -> Result<AddrMode, String> {
    if tokens.is_empty() {
        return Err("empty operand".to_string());
    }

    // Check for immediate mode: #expr
    if tokens[0].token == Token::Hash {
        let mut parser = ExprParser::new(&tokens[1..]);
        let expr = parser.parse_expr()?;
        return Ok(AddrMode::Immediate(expr));
    }

    // Check for pre-decrement: -(An)
    if tokens.len() >= 4 && tokens[0].token == Token::Minus && tokens[1].token == Token::LParen {
        if let Token::Ident(ref reg) = tokens[2].token {
            if let Some((n, true)) = parse_register(reg) {
                if tokens[3].token == Token::RParen {
                    return Ok(AddrMode::PreDec(n));
                }
            }
        }
    }

    // Check for register direct or special registers
    if let Token::Ident(ref name) = tokens[0].token {
        let upper = name.to_ascii_uppercase();

        // Special registers
        if upper == "SR" {
            return Ok(AddrMode::Sr);
        }
        if upper == "CCR" {
            return Ok(AddrMode::Ccr);
        }
        if upper == "USP" {
            return Ok(AddrMode::Usp);
        }
        if upper == "PC" && tokens.len() == 1 {
            // Just PC alone - treat as symbol
            return Ok(AddrMode::AbsLong(Expr::Symbol("PC".to_string())));
        }

        // Data or address register direct
        if let Some((n, is_addr)) = parse_register(name) {
            if tokens.len() == 1 {
                return if is_addr {
                    Ok(AddrMode::AddrReg(n))
                } else {
                    Ok(AddrMode::DataReg(n))
                };
            }
        }
    }

    // Check for (An), (An)+, d(An), d(An,Xn), (PC), d(PC), d(PC,Xn)
    if tokens[0].token == Token::LParen {
        return parse_indirect(tokens);
    }

    // Check for expression possibly followed by (An) or (PC)
    // e.g., 4(a0) or label(pc)
    let paren_pos = tokens.iter().position(|t| t.token == Token::LParen);
    if let Some(pp) = paren_pos {
        if pp > 0 {
            // Expression before the parenthesis
            let mut parser = ExprParser::new(&tokens[..pp]);
            let disp_expr = parser.parse_expr()?;
            return parse_indirect_with_disp(&tokens[pp..], disp_expr);
        }
    }

    // Default: absolute address (expression)
    let mut parser = ExprParser::new(tokens);
    let expr = parser.parse_expr()?;

    // Try to determine if short or long based on value
    if let Ok(val) = eval_expr(&expr, symbols, 0) {
        if (-32768..=32767).contains(&val) {
            return Ok(AddrMode::AbsShort(expr));
        }
    }
    Ok(AddrMode::AbsLong(expr))
}

fn parse_indirect(tokens: &[LocatedToken]) -> Result<AddrMode, String> {
    // Starts with (, parse register/PC inside
    if tokens.len() < 3 {
        return Err("incomplete indirect addressing".to_string());
    }

    // Check for (expr,PC) or (expr,An) or (expr,PC,Xn) or (expr,An,Xn) form
    // First, check if second-to-last token before ) is a comma followed by a register
    if let Some(comma_pos) = tokens[1..].iter().position(|t| t.token == Token::Comma) {
        let comma_idx = comma_pos + 1; // Adjust for slice offset
                                       // Check if what follows the comma is PC or An
        if tokens.len() > comma_idx + 1 {
            if let Token::Ident(ref reg) = tokens[comma_idx + 1].token {
                let upper = reg.to_ascii_uppercase();
                // Parse the expression before the comma
                let expr_tokens = &tokens[1..comma_idx];
                if !expr_tokens.is_empty() {
                    let mut parser = ExprParser::new(expr_tokens);
                    if let Ok(disp_expr) = parser.parse_expr() {
                        // Check for PC-relative forms
                        if upper == "PC" {
                            // (expr,PC) - simple PC-relative
                            if tokens.len() > comma_idx + 2
                                && tokens[comma_idx + 2].token == Token::RParen
                            {
                                return Ok(AddrMode::PcDisp(disp_expr));
                            }
                            // (expr,PC,Xn) - PC-relative indexed
                            if tokens.len() > comma_idx + 4
                                && tokens[comma_idx + 2].token == Token::Comma
                            {
                                if let Token::Ident(ref xreg) = tokens[comma_idx + 3].token {
                                    let (xn, is_addr, sz) = parse_index_reg(xreg)?;
                                    return Ok(AddrMode::PcIndex(disp_expr, xn, sz, is_addr));
                                }
                            }
                        }
                        // Check for (expr,An) - displacement mode
                        if let Some((an, true)) = parse_register(&upper) {
                            if tokens.len() > comma_idx + 2
                                && tokens[comma_idx + 2].token == Token::RParen
                            {
                                return Ok(AddrMode::Disp(disp_expr, an));
                            }
                            // (expr,An,Xn) - index with displacement
                            if tokens.len() > comma_idx + 4
                                && tokens[comma_idx + 2].token == Token::Comma
                            {
                                if let Token::Ident(ref xreg) = tokens[comma_idx + 3].token {
                                    let (xn, is_addr, sz) = parse_index_reg(xreg)?;
                                    return Ok(AddrMode::Index(disp_expr, an, xn, sz, is_addr));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Token::Ident(ref reg) = tokens[1].token {
        let upper = reg.to_ascii_uppercase();

        // Check for (PC)
        if upper == "PC" && tokens.len() >= 3 && tokens[2].token == Token::RParen {
            return Ok(AddrMode::PcDisp(Expr::Number(0)));
        }
        // (PC,Xn) - parse index register
        // For now, simplified

        // Check for (An)
        if let Some((n, true)) = parse_register(reg) {
            // Check for (An)
            if tokens.len() >= 3 && tokens[2].token == Token::RParen {
                // Check for (An)+ post-increment
                if tokens.len() >= 4 && tokens[3].token == Token::Plus {
                    return Ok(AddrMode::PostInc(n));
                }
                return Ok(AddrMode::AddrInd(n));
            }

            // Check for (An,Xn) - index mode with 0 displacement
            if tokens.len() >= 5 && tokens[2].token == Token::Comma {
                if let Token::Ident(ref xreg) = tokens[3].token {
                    let (xn, is_addr, sz) = parse_index_reg(xreg)?;
                    return Ok(AddrMode::Index(Expr::Number(0), n, xn, sz, is_addr));
                }
            }
        }
    }

    Err("invalid indirect addressing".to_string())
}

fn parse_indirect_with_disp(tokens: &[LocatedToken], disp: Expr) -> Result<AddrMode, String> {
    // tokens starts with (
    if tokens.len() < 3 {
        return Err("incomplete indirect addressing".to_string());
    }

    if let Token::Ident(ref reg) = tokens[1].token {
        let upper = reg.to_ascii_uppercase();

        // d(PC)
        if upper == "PC" {
            if tokens.len() >= 3 && tokens[2].token == Token::RParen {
                return Ok(AddrMode::PcDisp(disp));
            }
            // d(PC,Xn)
            if tokens.len() >= 5 && tokens[2].token == Token::Comma {
                if let Token::Ident(ref xreg) = tokens[3].token {
                    let (xn, is_addr, sz) = parse_index_reg(xreg)?;
                    return Ok(AddrMode::PcIndex(disp, xn, sz, is_addr));
                }
            }
        }

        // d(An)
        if let Some((n, true)) = parse_register(reg) {
            if tokens.len() >= 3 && tokens[2].token == Token::RParen {
                return Ok(AddrMode::Disp(disp, n));
            }
            // d(An,Xn)
            if tokens.len() >= 5 && tokens[2].token == Token::Comma {
                if let Token::Ident(ref xreg) = tokens[3].token {
                    let (xn, is_addr, sz) = parse_index_reg(xreg)?;
                    return Ok(AddrMode::Index(disp, n, xn, sz, is_addr));
                }
            }
        }
    }

    Err("invalid indexed addressing".to_string())
}

fn parse_index_reg(name: &str) -> Result<(u8, bool, Size), String> {
    // Parse register like d0, a0, d0.w, a0.l
    let name = name.trim();
    let (reg_part, size) = if let Some(dot_pos) = name.find('.') {
        let sz = Size::from_suffix(&name[dot_pos + 1..]).unwrap_or(Size::Word);
        (&name[..dot_pos], sz)
    } else {
        (name, Size::Word)
    };

    if let Some((n, is_addr)) = parse_register(reg_part) {
        Ok((n, is_addr, size))
    } else {
        Err(format!("invalid index register: {}", name))
    }
}

/// Splits operands by comma, respecting parentheses.
/// Filters out Eof and Newline tokens.
pub fn split_operands(tokens: &[LocatedToken]) -> Vec<&[LocatedToken]> {
    // Filter out trailing Eof/Newline tokens first
    let end = tokens
        .iter()
        .position(|t| matches!(t.token, Token::Eof | Token::Newline))
        .unwrap_or(tokens.len());
    let tokens = &tokens[..end];

    let mut result = vec![];
    let mut start = 0;
    let mut paren_depth: usize = 0;

    for (i, tok) in tokens.iter().enumerate() {
        match tok.token {
            Token::LParen => paren_depth += 1,
            Token::RParen => paren_depth = paren_depth.saturating_sub(1),
            Token::Comma if paren_depth == 0 => {
                if i > start {
                    result.push(&tokens[start..i]);
                }
                start = i + 1;
            }
            _ => {}
        }
    }

    if start < tokens.len() {
        result.push(&tokens[start..]);
    }

    result
}

// ============================================================================
// INSTRUCTION DEFINITIONS
// ============================================================================

/// Condition codes for Bcc, Scc, DBcc instructions.
#[derive(Debug, Clone, Copy)]
pub enum Condition {
    True = 0,  // T
    False = 1, // F
    Hi = 2,    // HI (high)
    Ls = 3,    // LS (low or same)
    Cc = 4,    // CC (carry clear) / HS
    Cs = 5,    // CS (carry set) / LO
    Ne = 6,    // NE (not equal)
    Eq = 7,    // EQ (equal)
    Vc = 8,    // VC (overflow clear)
    Vs = 9,    // VS (overflow set)
    Pl = 10,   // PL (plus)
    Mi = 11,   // MI (minus)
    Ge = 12,   // GE (greater or equal)
    Lt = 13,   // LT (less than)
    Gt = 14,   // GT (greater than)
    Le = 15,   // LE (less or equal)
}

impl Condition {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_uppercase().as_str() {
            "T" | "RA" => Some(Condition::True),
            "F" | "SR" => Some(Condition::False),
            "HI" => Some(Condition::Hi),
            "LS" => Some(Condition::Ls),
            "CC" | "HS" => Some(Condition::Cc),
            "CS" | "LO" => Some(Condition::Cs),
            "NE" => Some(Condition::Ne),
            "EQ" => Some(Condition::Eq),
            "VC" => Some(Condition::Vc),
            "VS" => Some(Condition::Vs),
            "PL" => Some(Condition::Pl),
            "MI" => Some(Condition::Mi),
            "GE" => Some(Condition::Ge),
            "LT" => Some(Condition::Lt),
            "GT" => Some(Condition::Gt),
            "LE" => Some(Condition::Le),
            _ => None,
        }
    }
}

// ============================================================================
// ASSEMBLER STATE
// ============================================================================

/// Main assembler state.
pub struct Assembler {
    /// Symbol table.
    pub symbols: SymbolTable,
    /// Current program counter.
    pub pc: u32,
    /// Origin address.
    pub origin: u32,
    /// Output buffer.
    pub output: Vec<u8>,
    /// RS counter for structure definitions.
    pub rs_counter: u32,
    /// Include paths for resolving includes.
    pub include_paths: Vec<PathBuf>,
    /// Current pass (1 or 2).
    pub pass: u8,
    /// Current file being assembled.
    pub current_file: PathBuf,
    /// Pending EQU definitions with forward references.
    pending_equs: Vec<(String, Expr)>,
    /// Current global label scope for local labels.
    current_scope: String,
}

impl Assembler {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            pc: 0,
            origin: 0,
            output: Vec::new(),
            rs_counter: 0,
            include_paths: vec![],
            pass: 1,
            current_file: PathBuf::new(),
            pending_equs: Vec::new(),
            current_scope: String::new(),
        }
    }

    /// Emits a byte to the output.
    pub fn emit_byte(&mut self, b: u8) {
        if self.pass == 2 {
            // Extend output if needed
            let offset = (self.pc - self.origin) as usize;
            if offset >= self.output.len() {
                self.output.resize(offset + 1, 0);
            }
            self.output[offset] = b;
        }
        self.pc += 1;
    }

    /// Emits a word (big-endian) to the output.
    pub fn emit_word(&mut self, w: u16) {
        self.emit_byte((w >> 8) as u8);
        self.emit_byte(w as u8);
    }

    /// Emits a long (big-endian) to the output.
    pub fn emit_long(&mut self, l: u32) {
        self.emit_word((l >> 16) as u16);
        self.emit_word(l as u16);
    }

    /// Aligns PC to word boundary.
    pub fn align_word(&mut self) {
        if self.pc & 1 != 0 {
            self.emit_byte(0);
        }
    }

    /// Returns the current scope for local label resolution.
    fn scope(&self) -> Option<&str> {
        if self.current_scope.is_empty() {
            None
        } else {
            Some(&self.current_scope)
        }
    }

    /// Expands a local label (starting with .) to its fully-qualified name.
    /// Local labels are scoped to the most recent global label.
    fn expand_local_label(&self, name: &str) -> String {
        if name.starts_with('.') && !self.current_scope.is_empty() {
            format!("{}{}", self.current_scope, name)
        } else {
            name.to_string()
        }
    }

    /// Assembles a source file and returns the binary output.
    /// This is the main entry point for assembling.
    pub fn assemble_file(&mut self, path: &std::path::Path) -> Result<Vec<u8>, String> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
        self.current_file = path.to_path_buf();

        // Add the file's directory to include paths
        if let Some(parent) = path.parent() {
            if !self.include_paths.contains(&parent.to_path_buf()) {
                self.include_paths.push(parent.to_path_buf());
            }
        }

        self.assemble_source(&source, path)
    }

    /// Assembles source code and returns the binary output.
    pub fn assemble_source(
        &mut self,
        source: &str,
        file: &std::path::Path,
    ) -> Result<Vec<u8>, String> {
        // Preprocess
        let mut pp = Preprocessor::new();
        for inc_path in &self.include_paths {
            pp.add_include_path(inc_path.clone());
        }
        let processed = pp.preprocess(source, file)?;

        // Two-pass assembly
        for pass in 1..=2 {
            self.pass = pass;
            self.pc = self.origin;
            self.output.clear();

            // Tokenize and parse
            let mut lexer = Lexer::new(&processed, file.to_string_lossy().as_ref());
            let tokens = lexer.tokenize()?;
            let lines = split_lines(&tokens);

            for line_tokens in lines {
                if let Some(parsed) = parse_line(line_tokens) {
                    self.process_line(&parsed)?;
                }
            }

            // After pass 1, resolve any pending EQUs with forward references
            if pass == 1 {
                self.resolve_pending_equs()?;
            }
        }

        Ok(std::mem::take(&mut self.output))
    }
}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LINE PARSER
// ============================================================================

/// A parsed assembly line.
#[derive(Debug)]
pub struct ParsedLine {
    /// Optional label at the start of the line.
    pub label: Option<String>,
    /// The mnemonic/directive name (if any).
    pub mnemonic: Option<String>,
    /// Size suffix (.b, .w, .l) if present.
    pub size: Option<Size>,
    /// Operand tokens (everything after mnemonic).
    pub operands: Vec<LocatedToken>,
    /// Source location of the line.
    pub loc: SourceLoc,
}

/// Parses a single line from tokens.
/// Returns None if the line is empty/comment-only.
pub fn parse_line(tokens: &[LocatedToken]) -> Option<ParsedLine> {
    if tokens.is_empty() {
        return None;
    }

    let mut pos = 0;
    let loc = tokens[0].loc.clone();

    // Skip leading newlines
    while pos < tokens.len() && tokens[pos].token == Token::Newline {
        pos += 1;
    }
    if pos >= tokens.len() || tokens[pos].token == Token::Eof {
        return None;
    }

    let mut label = None;
    let mut mnemonic = None;
    let mut size = None;

    // Check for label (identifier followed by colon)
    // Note: We require explicit colon for labels. Column-1 heuristic is disabled for now.
    if let Token::Ident(ref s) = tokens[pos].token {
        // Explicit label with colon
        if pos + 1 < tokens.len() && tokens[pos + 1].token == Token::Colon {
            label = Some(s.clone());
            pos += 2; // Skip identifier and colon
        }
        // Handle EQU/RS syntax: LABEL EQU VALUE (no colon)
        else if pos + 1 < tokens.len() {
            if let Token::Ident(ref next) = tokens[pos + 1].token {
                let upper = next.to_ascii_uppercase();
                if upper == "EQU" || upper == "RS" || upper.starts_with("RS.") {
                    label = Some(s.clone());
                    pos += 1; // Skip label, mnemonic will be parsed next
                }
            }
        }
    }

    // Skip whitespace-equivalent (there isn't really any in tokens, just check for next ident)
    if pos >= tokens.len() || tokens[pos].token == Token::Newline || tokens[pos].token == Token::Eof
    {
        // Line with only a label
        return Some(ParsedLine {
            label,
            mnemonic: None,
            size: None,
            operands: vec![],
            loc,
        });
    }

    // Parse mnemonic
    if let Token::Ident(ref s) = tokens[pos].token {
        // Check if this is mnemonic with embedded size suffix like ".l" token after it
        mnemonic = Some(s.clone());
        pos += 1;

        // Check for size suffix (e.g., ".l" as separate token)
        if pos < tokens.len() {
            if let Token::Ident(ref sz) = tokens[pos].token {
                if sz.starts_with('.') && sz.len() == 2 {
                    size = Size::from_suffix(&sz[1..]);
                    if size.is_some() {
                        pos += 1;
                    }
                }
            }
        }
    }

    // Collect remaining tokens as operands (until newline/eof)
    let mut operands = vec![];
    while pos < tokens.len()
        && tokens[pos].token != Token::Newline
        && tokens[pos].token != Token::Eof
    {
        operands.push(tokens[pos].clone());
        pos += 1;
    }

    Some(ParsedLine {
        label,
        mnemonic,
        size,
        operands,
        loc,
    })
}

/// Splits tokens into lines (separated by Newline tokens).
pub fn split_lines(tokens: &[LocatedToken]) -> Vec<&[LocatedToken]> {
    let mut lines = vec![];
    let mut start = 0;

    for (i, tok) in tokens.iter().enumerate() {
        if tok.token == Token::Newline || tok.token == Token::Eof {
            if i > start {
                lines.push(&tokens[start..i]);
            }
            start = i + 1;
        }
    }

    lines
}

// ============================================================================
// PREPROCESSOR
// ============================================================================

/// Preprocessor state for handling macros, includes, conditionals.
pub struct Preprocessor {
    /// Macro definitions: name -> (param_names, body_lines).
    macros: HashMap<String, (Vec<String>, Vec<String>)>,
    /// Include search paths.
    include_paths: Vec<PathBuf>,
    /// Unique counter for local labels in macro expansions.
    unique_counter: u32,
    /// Stack of files being processed (for detecting circular includes).
    file_stack: Vec<PathBuf>,
    /// Symbol table for EQU definitions (needed for REPT expressions).
    symbols: HashMap<String, i64>,
}

impl Preprocessor {
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
            include_paths: vec![],
            unique_counter: 0,
            file_stack: vec![],
            symbols: HashMap::new(),
        }
    }

    /// Adds an include search path.
    pub fn add_include_path(&mut self, path: impl Into<PathBuf>) {
        self.include_paths.push(path.into());
    }

    /// Generates a unique suffix for macro-local labels.
    fn unique_suffix(&mut self) -> String {
        let n = self.unique_counter;
        self.unique_counter += 1;
        format!("{}", n)
    }

    /// Preprocesses source text, expanding includes, macros, rept, and conditionals.
    /// Returns the fully expanded source.
    pub fn preprocess(&mut self, source: &str, file: &std::path::Path) -> Result<String, String> {
        self.file_stack.push(file.to_path_buf());
        let result = self.preprocess_lines(source, file);
        self.file_stack.pop();
        result
    }

    fn preprocess_lines(&mut self, source: &str, file: &std::path::Path) -> Result<String, String> {
        let mut output = String::new();
        let lines: Vec<&str> = source.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();
            let upper = trimmed.to_ascii_uppercase();

            // Check for INCLUDE
            if upper.starts_with("INCLUDE") || trimmed.to_ascii_lowercase().starts_with("include") {
                let included = self.handle_include(trimmed, file)?;
                output.push_str(&included);
                output.push('\n');
                i += 1;
                continue;
            }

            // Check for MACRO definition
            if let Some(macro_line) = self.try_parse_macro_start(trimmed) {
                let (name, params) = macro_line;
                let (body, end_idx) = self.collect_until(&lines, i + 1, "ENDM")?;
                self.macros
                    .insert(name.to_ascii_uppercase(), (params, body));
                i = end_idx + 1;
                continue;
            }

            // Check for REPT
            if upper.starts_with("REPT") {
                let count = self.parse_rept_count(trimmed)?;
                let (body, end_idx) = self.collect_until(&lines, i + 1, "ENDR")?;
                for _ in 0..count {
                    for body_line in &body {
                        output.push_str(body_line);
                        output.push('\n');
                    }
                }
                i = end_idx + 1;
                continue;
            }

            // Check for IF/ELSE/ENDIF
            if upper.starts_with("IF")
                && !upper.starts_with("IFDEF")
                && !upper.starts_with("IFNDEF")
            {
                let (taken_body, end_idx) = self.handle_conditional(&lines, i)?;
                for body_line in taken_body {
                    output.push_str(&body_line);
                    output.push('\n');
                }
                i = end_idx + 1;
                continue;
            }

            // Check for FAIL
            if upper.starts_with("FAIL") {
                let msg = trimmed[4..].trim();
                return Err(format!("{}:{}: {}", file.display(), i + 1, msg));
            }

            // Check for EQU definitions and collect symbols for REPT expressions
            // Format: NAME EQU value  OR  NAME equ value
            if upper.contains(" EQU ")
                || upper.contains("\tEQU\t")
                || upper.contains("\tEQU ")
                || upper.contains(" EQU\t")
            {
                self.try_parse_equ(trimmed);
            }

            // Check for macro invocation
            if let Some(expanded) = self.try_expand_macro(trimmed)? {
                output.push_str(&expanded);
                output.push('\n');
                i += 1;
                continue;
            }

            // Regular line - pass through
            output.push_str(line);
            output.push('\n');
            i += 1;
        }

        Ok(output)
    }

    /// Try to parse an EQU definition and add to preprocessor's symbol table.
    fn try_parse_equ(&mut self, line: &str) {
        // Format: NAME EQU value
        let upper = line.to_ascii_uppercase();
        if let Some(equ_pos) = upper
            .find(" EQU ")
            .or_else(|| upper.find("\tEQU\t"))
            .or_else(|| upper.find("\tEQU "))
            .or_else(|| upper.find(" EQU\t"))
        {
            let name = line[..equ_pos].trim().to_ascii_uppercase();
            let value_str = line[equ_pos + 4..].trim();

            // Try to parse as number
            let value = if let Some(hex) = value_str.strip_prefix('$') {
                i64::from_str_radix(hex, 16).ok()
            } else if let Some(bin) = value_str.strip_prefix('%') {
                i64::from_str_radix(bin, 2).ok()
            } else {
                value_str.parse::<i64>().ok()
            };

            // If simple number, store it
            if let Some(v) = value {
                self.symbols.insert(name, v);
            } else {
                // Try to evaluate as expression with known symbols
                let mut lexer = Lexer::new(value_str, "equ");
                if let Ok(tokens) = lexer.tokenize() {
                    let expr_tokens: Vec<_> = tokens
                        .into_iter()
                        .filter(|t| !matches!(t.token, Token::Eof | Token::Newline))
                        .collect();
                    if !expr_tokens.is_empty() {
                        let mut parser = ExprParser::new(&expr_tokens);
                        if let Ok(expr) = parser.parse_expr() {
                            if let Ok(v) = eval_expr(&expr, &self.symbols, 0) {
                                self.symbols.insert(name, v);
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_include(
        &mut self,
        line: &str,
        current_file: &std::path::Path,
    ) -> Result<String, String> {
        // Parse: include "filename" or include <filename>
        let rest = line.trim();
        let rest = if rest.to_ascii_uppercase().starts_with("INCLUDE") {
            rest[7..].trim()
        } else {
            rest
        };

        let filename = if rest.starts_with('"') {
            rest.trim_matches('"')
        } else if rest.starts_with('<') && rest.ends_with('>') {
            &rest[1..rest.len() - 1]
        } else {
            rest.trim_matches('"')
        };

        // Resolve path relative to current file
        let include_path = if let Some(parent) = current_file.parent() {
            let relative = parent.join(filename);
            if relative.exists() {
                relative
            } else {
                // Try include paths
                self.include_paths
                    .iter()
                    .map(|p| p.join(filename))
                    .find(|p| p.exists())
                    .unwrap_or_else(|| PathBuf::from(filename))
            }
        } else {
            PathBuf::from(filename)
        };

        // Check for circular include
        if self.file_stack.contains(&include_path) {
            return Err(format!(
                "circular include detected: {}",
                include_path.display()
            ));
        }

        // Read and preprocess the included file
        let content = std::fs::read_to_string(&include_path)
            .map_err(|e| format!("cannot read {}: {}", include_path.display(), e))?;

        self.preprocess(&content, &include_path)
    }

    fn try_parse_macro_start(&self, line: &str) -> Option<(String, Vec<String>)> {
        // Format: name MACRO or name macro [params]
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let upper1 = parts[1].to_ascii_uppercase();
            if upper1 == "MACRO" {
                let name = parts[0].to_string();
                let params: Vec<String> = parts.iter().skip(2).map(|s| s.to_string()).collect();
                return Some((name, params));
            }
        }
        // Also check for: MACRO name (less common)
        if !parts.is_empty() && parts[0].eq_ignore_ascii_case("MACRO") {
            // Not the standard vasm format, skip for now
        }
        None
    }

    fn collect_until(
        &self,
        lines: &[&str],
        start: usize,
        end_directive: &str,
    ) -> Result<(Vec<String>, usize), String> {
        let mut body = vec![];
        let mut depth = 1;
        let mut i = start;

        while i < lines.len() {
            let trimmed = lines[i].trim().to_ascii_uppercase();

            // Track nesting
            if trimmed.contains("MACRO") || trimmed.starts_with("REPT") || trimmed.starts_with("IF")
            {
                depth += 1;
            }
            if trimmed == end_directive || trimmed.starts_with(&format!("{} ", end_directive)) {
                depth -= 1;
                if depth == 0 {
                    return Ok((body, i));
                }
            }
            if trimmed == "ENDM" || trimmed == "ENDR" || trimmed == "ENDIF" {
                depth -= 1;
                if depth == 0 && trimmed.starts_with(end_directive) {
                    return Ok((body, i));
                }
            }

            body.push(lines[i].to_string());
            i += 1;
        }

        Err(format!("unterminated {}", end_directive))
    }

    fn parse_rept_count(&self, line: &str) -> Result<usize, String> {
        // Format: REPT count (where count can be an expression)
        let rest = line[4..].trim();

        // Try simple numeric formats first
        if let Some(hex) = rest.strip_prefix('$') {
            if let Ok(n) = usize::from_str_radix(hex, 16) {
                return Ok(n);
            }
        } else if let Some(bin) = rest.strip_prefix('%') {
            if let Ok(n) = usize::from_str_radix(bin, 2) {
                return Ok(n);
            }
        } else if let Ok(n) = rest.parse::<usize>() {
            return Ok(n);
        }

        // Try to evaluate as expression using preprocessor's known symbols
        let mut lexer = Lexer::new(rest, "rept");
        if let Ok(tokens) = lexer.tokenize() {
            // Filter out Eof and Newline
            let expr_tokens: Vec<_> = tokens
                .into_iter()
                .filter(|t| !matches!(t.token, Token::Eof | Token::Newline))
                .collect();
            if !expr_tokens.is_empty() {
                let mut parser = ExprParser::new(&expr_tokens);
                if let Ok(expr) = parser.parse_expr() {
                    if let Ok(val) = eval_expr(&expr, &self.symbols, 0) {
                        if val >= 0 {
                            return Ok(val as usize);
                        }
                    }
                }
            }
        }

        Err(format!("invalid rept count: {}", rest))
    }

    fn handle_conditional(
        &mut self,
        lines: &[&str],
        start: usize,
    ) -> Result<(Vec<String>, usize), String> {
        // Parse IF expression
        let line = lines[start].trim();
        let upper = line.to_ascii_uppercase();
        let condition_str = if upper.starts_with("IF ") {
            line[3..].trim()
        } else {
            line[2..].trim() // Just "IF"
        };

        // Evaluate condition (simple: just check if it's non-zero or NARG comparison)
        let condition = self.eval_simple_condition(condition_str)?;

        let mut if_body = vec![];
        let mut else_body = vec![];
        let mut in_else = false;
        let mut depth = 1;
        let mut i = start + 1;

        while i < lines.len() {
            let trimmed = lines[i].trim().to_ascii_uppercase();

            if trimmed.starts_with("IF")
                && !trimmed.starts_with("IFDEF")
                && !trimmed.starts_with("IFNDEF")
            {
                depth += 1;
            }
            if trimmed == "ENDIF" {
                depth -= 1;
                if depth == 0 {
                    return Ok((if condition { if_body } else { else_body }, i));
                }
            }
            if trimmed == "ELSE" && depth == 1 {
                in_else = true;
                i += 1;
                continue;
            }

            if in_else {
                else_body.push(lines[i].to_string());
            } else {
                if_body.push(lines[i].to_string());
            }
            i += 1;
        }

        Err("unterminated IF".to_string())
    }

    fn eval_simple_condition(&self, expr: &str) -> Result<bool, String> {
        // Handle simple cases: NARG>1, number, symbol comparison
        let expr = expr.trim();

        // NARG comparisons (used in macros - for now return false outside macro)
        if expr.contains("NARG") {
            // In macro expansion context, we'd have NARG value
            // For preprocessing stage, default to false
            return Ok(false);
        }

        // Simple number check
        if let Ok(n) = expr.parse::<i64>() {
            return Ok(n != 0);
        }
        if let Some(hex) = expr.strip_prefix('$') {
            if let Ok(n) = i64::from_str_radix(hex, 16) {
                return Ok(n != 0);
            }
        }

        // For complex expressions, default to true (will be properly evaluated later)
        Ok(true)
    }

    fn try_expand_macro(&mut self, line: &str) -> Result<Option<String>, String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(None);
        }

        // The macro name could be the first word, or the second word if first is a label
        let (label_prefix, name, arg_start_idx) = if parts[0].ends_with(':') {
            // First word is a label
            if parts.len() > 1 {
                (Some(parts[0]), parts[1].to_ascii_uppercase(), 2)
            } else {
                return Ok(None);
            }
        } else {
            (None, parts[0].to_ascii_uppercase(), 1)
        };

        // Check if it's a defined macro
        if let Some((_params, body)) = self.macros.get(&name).cloned() {
            // Parse args: join remaining parts and split by comma
            let arg_text: String = parts[arg_start_idx..].join(" ");
            let args: Vec<&str> = if arg_text.is_empty() {
                vec![]
            } else {
                arg_text
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect()
            };

            let suffix = self.unique_suffix();
            let mut output = String::new();

            // Preserve label if present
            if let Some(label) = label_prefix {
                output.push_str(label);
                output.push('\n');
            }

            // Process macro body, handling nested REPT with \+ specially
            let mut i = 0;
            while i < body.len() {
                let line = &body[i];
                let upper = line.trim().to_ascii_uppercase();

                // Check for REPT \# pattern which iterates over arguments
                if upper.starts_with("REPT") && line.contains("\\#") {
                    // Find matching ENDR
                    let mut depth = 1;
                    let mut rept_body = Vec::new();
                    let mut j = i + 1;
                    while j < body.len() && depth > 0 {
                        let inner = body[j].trim().to_ascii_uppercase();
                        if inner.starts_with("REPT") {
                            depth += 1;
                        } else if inner == "ENDR" {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        rept_body.push(body[j].clone());
                        j += 1;
                    }

                    // Expand: for each argument, output the rept body with \+ replaced
                    for arg in args.iter() {
                        for rept_line in &rept_body {
                            let mut expanded = rept_line.clone();
                            // Replace \@ with unique suffix
                            expanded = expanded.replace("\\@", &suffix);
                            // Replace \+ with current argument
                            expanded = expanded.replace("\\+", arg);
                            // Replace \1, \2, etc.
                            for (idx, a) in args.iter().enumerate() {
                                expanded = expanded.replace(&format!("\\{}", idx + 1), a);
                            }
                            // Replace \# with arg count (for any remaining)
                            expanded = expanded.replace("\\#", &args.len().to_string());
                            output.push_str(&expanded);
                            output.push('\n');
                        }
                    }

                    i = j + 1; // Skip past ENDR
                    continue;
                }

                // Normal line processing
                let mut expanded = line.clone();

                // Replace \@@ with unique suffix + @ (vasm compatibility)
                expanded = expanded.replace("\\@@", &format!("{}@", suffix));

                // Replace \@ with unique suffix
                expanded = expanded.replace("\\@", &suffix);

                // Remove ! from label names (vasm local label syntax)
                // e.g., .t17!: becomes .t17:
                if expanded.contains('!') {
                    // Only remove ! if it's part of a label (before :)
                    if let Some(colon_pos) = expanded.find(':') {
                        let label_part = &expanded[..colon_pos];
                        if label_part.contains('!') {
                            let clean_label = label_part.replace('!', "");
                            expanded = format!("{}{}", clean_label, &expanded[colon_pos..]);
                        }
                    }
                }

                // Replace \# with argument count
                expanded = expanded.replace("\\#", &args.len().to_string());

                // Replace \1, \2, etc. with arguments
                for (idx, arg) in args.iter().enumerate() {
                    expanded = expanded.replace(&format!("\\{}", idx + 1), arg);
                }

                output.push_str(&expanded);
                output.push('\n');
                i += 1;
            }

            // Recursively preprocess the expanded output for nested macro calls
            // Use a temporary path for the expansion
            let expanded_path = std::path::Path::new("macro_expansion");
            let reprocessed = self.preprocess_lines(&output, expanded_path)?;
            return Ok(Some(reprocessed));
        }

        Ok(None)
    }
}

impl Default for Preprocessor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DIRECTIVE HANDLERS
// ============================================================================

impl Assembler {
    /// Processes an ORG directive.
    pub fn handle_org(&mut self, operands: &[LocatedToken], loc: &SourceLoc) -> Result<(), String> {
        if operands.is_empty() {
            return Err(format!("{}: org requires an address", loc));
        }
        let mut parser = ExprParser::new(operands);
        let expr = parser.parse_expr()?;
        let addr = eval_expr(&expr, self.symbols.as_map(), self.pc)?;
        self.pc = addr as u32;
        if self.pass == 1 && self.origin == 0 {
            self.origin = self.pc;
        }
        Ok(())
    }

    /// Processes an EQU directive.
    /// Note: If the expression can't be evaluated yet (forward reference),
    /// we'll try again later. The assemble_source function handles this.
    pub fn handle_equ(
        &mut self,
        label: &str,
        operands: &[LocatedToken],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if operands.is_empty() {
            return Err(format!("{}: equ requires a value", loc));
        }
        let mut parser = ExprParser::new(operands);
        let expr = parser.parse_expr()?;

        match eval_expr(&expr, self.symbols.as_map(), self.pc) {
            Ok(value) => {
                self.symbols.define(label, value)?;
            }
            Err(_) if self.pass == 1 => {
                // Forward reference - store as pending
                self.pending_equs.push((label.to_string(), expr));
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    /// Resolves pending EQU definitions that had forward references.
    fn resolve_pending_equs(&mut self) -> Result<(), String> {
        let mut made_progress = true;
        while made_progress && !self.pending_equs.is_empty() {
            made_progress = false;
            let pending = std::mem::take(&mut self.pending_equs);
            for (label, expr) in pending {
                match eval_expr(&expr, self.symbols.as_map(), self.pc) {
                    Ok(value) => {
                        self.symbols.define(&label, value)?;
                        made_progress = true;
                    }
                    Err(_) => {
                        // Still can't resolve, put back
                        self.pending_equs.push((label, expr));
                    }
                }
            }
        }

        // If any are still pending, that's an error
        if !self.pending_equs.is_empty() {
            let labels: Vec<_> = self.pending_equs.iter().map(|(l, _)| l.as_str()).collect();
            return Err(format!("unresolved symbols: {}", labels.join(", ")));
        }
        Ok(())
    }

    /// Processes an EVEN directive.
    pub fn handle_even(&mut self) {
        self.align_word();
    }

    /// Processes DC.B/W/L directive.
    pub fn handle_dc(
        &mut self,
        size: Size,
        operands: &[LocatedToken],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if operands.is_empty() {
            return Err(format!("{}: dc requires data", loc));
        }

        // Parse comma-separated values
        let mut pos = 0;
        while pos < operands.len() {
            // Check for string literal
            if let Token::String(ref s) = operands[pos].token {
                for byte in s.bytes() {
                    match size {
                        Size::Byte => self.emit_byte(byte),
                        Size::Word => self.emit_word(byte as u16),
                        Size::Long => self.emit_long(byte as u32),
                    }
                }
                pos += 1;
            } else {
                // Parse expression
                let expr_tokens: Vec<_> = operands[pos..]
                    .iter()
                    .take_while(|t| t.token != Token::Comma)
                    .cloned()
                    .collect();
                if expr_tokens.is_empty() {
                    pos += 1;
                    continue;
                }
                let mut parser = ExprParser::new(&expr_tokens);
                let expr = parser.parse_expr()?;
                // Use tolerant evaluation in pass 1 for forward references
                let value = if self.pass == 1 {
                    eval_expr_scoped(&expr, self.symbols.as_map(), self.pc, self.scope())
                        .unwrap_or(0)
                } else {
                    eval_expr_scoped(&expr, self.symbols.as_map(), self.pc, self.scope())?
                };
                match size {
                    Size::Byte => self.emit_byte(value as u8),
                    Size::Word => self.emit_word(value as u16),
                    Size::Long => self.emit_long(value as u32),
                }
                pos += expr_tokens.len();
            }

            // Skip comma
            if pos < operands.len() && operands[pos].token == Token::Comma {
                pos += 1;
            }
        }
        Ok(())
    }

    /// Processes DCB.B/W/L directive (define constant block - fill with repeated value).
    pub fn handle_dcb(
        &mut self,
        size: Size,
        operands: &[LocatedToken],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if operands.is_empty() {
            return Err(format!("{}: dcb requires count", loc));
        }

        // Parse count,value (comma-separated) or just count (fills with 0)
        let ops = split_operands(operands);

        // Parse count expression
        let mut parser = ExprParser::new(ops[0]);
        let count_expr = parser.parse_expr()?;
        let count = if self.pass == 1 {
            eval_expr_scoped(&count_expr, self.symbols.as_map(), self.pc, self.scope()).unwrap_or(0)
        } else {
            eval_expr_scoped(&count_expr, self.symbols.as_map(), self.pc, self.scope())?
        } as usize;

        // Parse value expression (optional, defaults to 0)
        let value = if ops.len() >= 2 {
            let mut parser = ExprParser::new(ops[1]);
            let value_expr = parser.parse_expr()?;
            if self.pass == 1 {
                eval_expr_scoped(&value_expr, self.symbols.as_map(), self.pc, self.scope())
                    .unwrap_or(0)
            } else {
                eval_expr_scoped(&value_expr, self.symbols.as_map(), self.pc, self.scope())?
            }
        } else {
            0
        };

        // Emit the repeated value
        for _ in 0..count {
            match size {
                Size::Byte => self.emit_byte(value as u8),
                Size::Word => self.emit_word(value as u16),
                Size::Long => self.emit_long(value as u32),
            }
        }
        Ok(())
    }

    /// Processes DS.B/W/L directive (reserve space).
    pub fn handle_ds(
        &mut self,
        size: Size,
        operands: &[LocatedToken],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if operands.is_empty() {
            return Err(format!("{}: ds requires a count", loc));
        }
        let mut parser = ExprParser::new(operands);
        let expr = parser.parse_expr()?;
        let count = eval_expr(&expr, self.symbols.as_map(), self.pc)? as usize;
        let bytes = count * size.bytes();
        for _ in 0..bytes {
            self.emit_byte(0);
        }
        Ok(())
    }

    /// Processes RSSET directive.
    pub fn handle_rsset(
        &mut self,
        operands: &[LocatedToken],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if operands.is_empty() {
            return Err(format!("{}: rsset requires an address", loc));
        }
        let mut parser = ExprParser::new(operands);
        let expr = parser.parse_expr()?;
        let value = eval_expr(&expr, self.symbols.as_map(), self.pc)?;
        self.rs_counter = value as u32;
        Ok(())
    }

    /// Processes RS.B/W/L directive.
    pub fn handle_rs(
        &mut self,
        size: Size,
        label: &str,
        operands: &[LocatedToken],
        _loc: &SourceLoc,
    ) -> Result<(), String> {
        // RS returns current RS counter, then advances it
        let current = self.rs_counter as i64;
        self.symbols.define(label, current)?;

        let count = if operands.is_empty() {
            1
        } else {
            let mut parser = ExprParser::new(operands);
            let expr = parser.parse_expr()?;
            eval_expr(&expr, self.symbols.as_map(), self.pc)? as usize
        };
        self.rs_counter += (count * size.bytes()) as u32;
        Ok(())
    }

    /// Processes a single parsed line.
    pub fn process_line(&mut self, line: &ParsedLine) -> Result<(), String> {
        // Handle label
        if let Some(ref label) = line.label {
            // Update scope: global labels set new scope, local labels use current scope
            let full_label = if label.starts_with('.') {
                self.expand_local_label(label)
            } else {
                // New global label - update scope
                self.current_scope = label.clone();
                label.clone()
            };

            // Don't define label for EQU (it's handled specially)
            let mnemonic = line.mnemonic.as_ref().map(|s| s.to_ascii_uppercase());
            if mnemonic.as_deref() != Some("EQU")
                && !mnemonic
                    .as_ref()
                    .map(|s| s.starts_with("RS"))
                    .unwrap_or(false)
            {
                self.symbols.define(&full_label, self.pc as i64)?;
            }
        }

        // Handle mnemonic/directive
        let mnemonic = match &line.mnemonic {
            Some(m) => m.to_ascii_uppercase(),
            None => return Ok(()), // Label-only line
        };

        let size = line.size.unwrap_or(Size::Word);

        match mnemonic.as_str() {
            "ORG" => self.handle_org(&line.operands, &line.loc),
            "EQU" => {
                let label = line
                    .label
                    .as_ref()
                    .ok_or_else(|| format!("{}: equ requires a label", line.loc))?;
                self.handle_equ(label, &line.operands, &line.loc)
            }
            "EVEN" => {
                self.handle_even();
                Ok(())
            }
            "DC" => self.handle_dc(size, &line.operands, &line.loc),
            "DCB" => self.handle_dcb(size, &line.operands, &line.loc),
            "DS" => self.handle_ds(size, &line.operands, &line.loc),
            "RSSET" => self.handle_rsset(&line.operands, &line.loc),
            "RS" => {
                let label = line
                    .label
                    .as_ref()
                    .ok_or_else(|| format!("{}: rs requires a label", line.loc))?;
                self.handle_rs(size, label, &line.operands, &line.loc)
            }
            // VASM diagnostic directives - ignore
            "PRINTT" | "PRINTV" | "PRINTI" | "ECHO" | "FAIL" | "WARN" => Ok(()),
            // Instructions
            _ => self.encode_instruction(&mnemonic, size, &line.operands, &line.loc),
        }
    }

    /// Encodes a single M68K instruction.
    fn encode_instruction(
        &mut self,
        mnemonic: &str,
        size: Size,
        operands: &[LocatedToken],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        let ops = split_operands(operands);

        match mnemonic {
            // Data movement
            "MOVE" => self.encode_move(size, &ops, loc),
            "MOVEA" => self.encode_movea(size, &ops, loc),
            "MOVEQ" => self.encode_moveq(&ops, loc),
            "LEA" => self.encode_lea(&ops, loc),
            "PEA" => self.encode_pea(&ops, loc),
            "CLR" => self.encode_clr(size, &ops, loc),
            "EXG" => self.encode_exg(&ops, loc),
            "SWAP" => self.encode_swap(&ops, loc),

            // Arithmetic
            "ADD" => self.encode_add(size, &ops, loc),
            "ADDA" => self.encode_adda(size, &ops, loc),
            "ADDI" => self.encode_addi(size, &ops, loc),
            "ADDQ" => self.encode_addq(size, &ops, loc),
            "ADDX" => self.encode_addx(size, &ops, loc),
            "SUB" => self.encode_sub(size, &ops, loc),
            "SUBA" => self.encode_suba(size, &ops, loc),
            "SUBI" => self.encode_subi(size, &ops, loc),
            "SUBQ" => self.encode_subq(size, &ops, loc),
            "SUBX" => self.encode_subx(size, &ops, loc),
            "NEG" => self.encode_neg(size, &ops, loc),
            "NEGX" => self.encode_negx(size, &ops, loc),
            "EXT" => self.encode_ext(size, &ops, loc),
            "MULU" => self.encode_mulu(&ops, loc),
            "MULS" => self.encode_muls(&ops, loc),
            "DIVU" => self.encode_divu(&ops, loc),
            "DIVS" => self.encode_divs(&ops, loc),
            "CMP" => self.encode_cmp(size, &ops, loc),
            "CMPA" => self.encode_cmpa(size, &ops, loc),
            "CMPI" => self.encode_cmpi(size, &ops, loc),
            "CMPM" => self.encode_cmpm(size, &ops, loc),
            "TST" => self.encode_tst(size, &ops, loc),

            // Logical
            "AND" => self.encode_and(size, &ops, loc),
            "ANDI" => self.encode_andi(size, &ops, loc),
            "OR" => self.encode_or(size, &ops, loc),
            "ORI" => self.encode_ori(size, &ops, loc),
            "EOR" => self.encode_eor(size, &ops, loc),
            "EORI" => self.encode_eori(size, &ops, loc),
            "NOT" => self.encode_not(size, &ops, loc),

            // Shifts and rotates
            "ASL" => self.encode_shift(size, &ops, loc, 0b100, true),
            "ASR" => self.encode_shift(size, &ops, loc, 0b000, true),
            "LSL" => self.encode_shift(size, &ops, loc, 0b101, true),
            "LSR" => self.encode_shift(size, &ops, loc, 0b001, true),
            "ROL" => self.encode_shift(size, &ops, loc, 0b111, false),
            "ROR" => self.encode_shift(size, &ops, loc, 0b011, false),
            "ROXL" => self.encode_shift(size, &ops, loc, 0b110, false),
            "ROXR" => self.encode_shift(size, &ops, loc, 0b010, false),

            // Bit manipulation
            "BTST" => self.encode_bit(0b00, &ops, loc),
            "BCHG" => self.encode_bit(0b01, &ops, loc),
            "BCLR" => self.encode_bit(0b10, &ops, loc),
            "BSET" => self.encode_bit(0b11, &ops, loc),

            // BCD
            "ABCD" => self.encode_bcd(0xC100, &ops, loc),
            "SBCD" => self.encode_bcd(0x8100, &ops, loc),
            "NBCD" => self.encode_nbcd(&ops, loc),

            // Branches
            "BRA" => self.encode_bra(&ops, loc),
            "BSR" => self.encode_bsr(&ops, loc),
            "BHI" | "BLS" | "BCC" | "BHS" | "BCS" | "BLO" | "BNE" | "BEQ" | "BVC" | "BVS"
            | "BPL" | "BMI" | "BGE" | "BLT" | "BGT" | "BLE" => self.encode_bcc(mnemonic, &ops, loc),

            // DBcc
            "DBRA" | "DBT" | "DBF" | "DBHI" | "DBLS" | "DBCC" | "DBCS" | "DBNE" | "DBEQ"
            | "DBVC" | "DBVS" | "DBPL" | "DBMI" | "DBGE" | "DBLT" | "DBGT" | "DBLE" => {
                self.encode_dbcc(mnemonic, &ops, loc)
            }

            // Scc
            "ST" | "SF" | "SHI" | "SLS" | "SCC" | "SCS" | "SNE" | "SEQ" | "SVC" | "SVS" | "SPL"
            | "SMI" | "SGE" | "SLT" | "SGT" | "SLE" => self.encode_scc(mnemonic, &ops, loc),

            // Control
            "JMP" => self.encode_jmp(&ops, loc),
            "JSR" => self.encode_jsr(&ops, loc),
            "RTS" => self.encode_simple(0x4E75),
            "RTE" => self.encode_simple(0x4E73),
            "RTR" => self.encode_simple(0x4E77),
            "NOP" => self.encode_simple(0x4E71),
            "RESET" => self.encode_simple(0x4E70),
            "TRAPV" => self.encode_simple(0x4E76),
            "ILLEGAL" => self.encode_simple(0x4AFC),
            "TRAP" => self.encode_trap(&ops, loc),
            "CHK" => self.encode_chk(&ops, loc),
            "LINK" => self.encode_link(&ops, loc),
            "UNLK" => self.encode_unlk(&ops, loc),
            "STOP" => self.encode_stop(&ops, loc),
            "TAS" => self.encode_tas(&ops, loc),
            "MOVEM" => self.encode_movem(size, &ops, loc),
            "MOVEP" => self.encode_movep(size, &ops, loc),

            _ => Err(format!("{}: unknown instruction: {}", loc, mnemonic)),
        }
    }

    // Helper to emit a simple instruction with no operands
    fn encode_simple(&mut self, opcode: u16) -> Result<(), String> {
        self.emit_word(opcode);
        Ok(())
    }

    // Encode MOVE instruction
    fn encode_move(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: move requires 2 operands", loc));
        }

        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        // Check for MOVE to/from SR, CCR
        if matches!(dst, AddrMode::Sr) {
            return self.encode_move_to_sr(&src, loc);
        }
        if matches!(src, AddrMode::Sr) {
            return self.encode_move_from_sr(&dst, loc);
        }
        if matches!(dst, AddrMode::Ccr) {
            return self.encode_move_to_ccr(&src, loc);
        }
        // Check for MOVE to/from USP
        if matches!(dst, AddrMode::Usp) {
            return self.encode_move_to_usp(&src, loc);
        }
        if matches!(src, AddrMode::Usp) {
            return self.encode_move_from_usp(&dst, loc);
        }

        let sz_bits = match size {
            Size::Byte => 0b01,
            Size::Word => 0b11,
            Size::Long => 0b10,
        };

        let (src_mode, src_reg, src_ext) = self.encode_ea_with_imm(&src, size)?;
        let (dst_mode, dst_reg, dst_ext) = encode_ea(
            &dst,
            self.symbols.as_map(),
            self.pc + 2 + (src_ext.len() * 2) as u32,
            self.pass,
            self.scope(),
        )?;

        // MOVE encoding: 00 size dst_reg dst_mode src_mode src_reg
        let opcode = (sz_bits << 12)
            | ((dst_reg as u16) << 9)
            | ((dst_mode as u16) << 6)
            | ((src_mode as u16) << 3)
            | (src_reg as u16);
        self.emit_word(opcode);
        for ext in src_ext {
            self.emit_word(ext);
        }
        for ext in dst_ext {
            self.emit_word(ext);
        }
        Ok(())
    }

    fn encode_movea(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: movea requires 2 operands", loc));
        }
        // MOVEA is just MOVE with address register destination
        self.encode_move(size, ops, loc)
    }

    fn encode_moveq(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: moveq requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let imm = match src {
            AddrMode::Immediate(ref expr) => eval_expr(expr, self.symbols.as_map(), self.pc)? as i8,
            _ => return Err(format!("{}: moveq source must be immediate", loc)),
        };
        let dreg = match dst {
            AddrMode::DataReg(r) => r,
            _ => return Err(format!("{}: moveq destination must be data register", loc)),
        };

        let opcode = 0x7000 | ((dreg as u16) << 9) | ((imm as u8) as u16);
        self.emit_word(opcode);
        Ok(())
    }

    fn encode_lea(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: lea requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let areg = match dst {
            AddrMode::AddrReg(r) => r,
            _ => return Err(format!("{}: lea destination must be address register", loc)),
        };

        let (mode, reg, ext) = encode_ea(
            &src,
            self.symbols.as_map(),
            self.pc + 2,
            self.pass,
            self.scope(),
        )?;
        let opcode = 0x41C0 | ((areg as u16) << 9) | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_pea(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: pea requires 1 operand", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let (mode, reg, ext) = encode_ea(
            &src,
            self.symbols.as_map(),
            self.pc + 2,
            self.pass,
            self.scope(),
        )?;
        let opcode = 0x4840 | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_clr(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: clr requires 1 operand", loc));
        }
        let dst = parse_operand(ops[0], self.symbols.as_map())?;
        let sz = match size {
            Size::Byte => 0,
            Size::Word => 1,
            Size::Long => 2,
        };
        let (mode, reg, ext) = encode_ea(
            &dst,
            self.symbols.as_map(),
            self.pc + 2,
            self.pass,
            self.scope(),
        )?;
        let opcode = 0x4200 | (sz << 6) | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_exg(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: exg requires 2 operands", loc));
        }
        let a = parse_operand(ops[0], self.symbols.as_map())?;
        let b = parse_operand(ops[1], self.symbols.as_map())?;

        let (rx, ry, mode) = match (&a, &b) {
            (AddrMode::DataReg(x), AddrMode::DataReg(y)) => (*x, *y, 0b01000),
            (AddrMode::AddrReg(x), AddrMode::AddrReg(y)) => (*x, *y, 0b01001),
            (AddrMode::DataReg(x), AddrMode::AddrReg(y)) => (*x, *y, 0b10001),
            (AddrMode::AddrReg(x), AddrMode::DataReg(y)) => (*y, *x, 0b10001),
            _ => return Err(format!("{}: exg requires register operands", loc)),
        };

        let opcode = 0xC100 | ((rx as u16) << 9) | (mode << 3) | (ry as u16);
        self.emit_word(opcode);
        Ok(())
    }

    fn encode_swap(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: swap requires 1 operand", loc));
        }
        let dst = parse_operand(ops[0], self.symbols.as_map())?;
        let dreg = match dst {
            AddrMode::DataReg(r) => r,
            _ => return Err(format!("{}: swap requires data register", loc)),
        };
        self.emit_word(0x4840 | (dreg as u16));
        Ok(())
    }

    // Helper to encode EA with immediate handling
    fn encode_ea_with_imm(
        &self,
        mode: &AddrMode,
        size: Size,
    ) -> Result<(u8, u8, Vec<u16>), String> {
        match mode {
            AddrMode::Immediate(ref expr) => {
                // In pass 1, use 0 for undefined symbols; in pass 2, require resolution
                let val = try_eval_expr(
                    expr,
                    self.symbols.as_map(),
                    self.pc,
                    self.pass,
                    self.scope(),
                )?;
                let ext = match size {
                    Size::Byte => vec![(val & 0xFF) as u16],
                    Size::Word => vec![val as u16],
                    Size::Long => vec![(val >> 16) as u16, val as u16],
                };
                Ok((0b111, 0b100, ext))
            }
            _ => encode_ea(
                mode,
                self.symbols.as_map(),
                self.pc + 2,
                self.pass,
                self.scope(),
            ),
        }
    }

    // Arithmetic encoders
    fn encode_add(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        // Check if destination is address register - use ADDA
        if ops.len() == 2 {
            let dst = parse_operand(ops[1], self.symbols.as_map())?;
            if matches!(dst, AddrMode::AddrReg(_)) {
                return self.encode_adda(size, ops, loc);
            }
        }
        self.encode_arith_op(0xD000, size, ops, loc)
    }

    fn encode_sub(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        // Check if destination is address register - use SUBA
        if ops.len() == 2 {
            let dst = parse_operand(ops[1], self.symbols.as_map())?;
            if matches!(dst, AddrMode::AddrReg(_)) {
                return self.encode_suba(size, ops, loc);
            }
        }
        self.encode_arith_op(0x9000, size, ops, loc)
    }

    fn encode_arith_op(
        &mut self,
        base: u16,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let sz = match size {
            Size::Byte => 0,
            Size::Word => 1,
            Size::Long => 2,
        };

        match (&src, &dst) {
            (_, AddrMode::DataReg(dreg)) => {
                // <ea>,Dn
                let (mode, reg, ext) = self.encode_ea_with_imm(&src, size)?;
                let opcode = base
                    | (((*dreg) as u16) << 9)
                    | (sz << 6)
                    | ((mode as u16) << 3)
                    | (reg as u16);
                self.emit_word(opcode);
                for e in ext {
                    self.emit_word(e);
                }
            }
            (AddrMode::DataReg(dreg), _) => {
                // Dn,<ea>
                let (mode, reg, ext) = encode_ea(
                    &dst,
                    self.symbols.as_map(),
                    self.pc + 2,
                    self.pass,
                    self.scope(),
                )?;
                let opmode = sz | 0b100;
                let opcode = base
                    | (((*dreg) as u16) << 9)
                    | (opmode << 6)
                    | ((mode as u16) << 3)
                    | (reg as u16);
                self.emit_word(opcode);
                for e in ext {
                    self.emit_word(e);
                }
            }
            _ => return Err(format!("{}: invalid operand combination", loc)),
        }
        Ok(())
    }

    fn encode_adda(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        self.encode_adda_suba(0xD0C0, size, ops, loc)
    }

    fn encode_suba(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        self.encode_adda_suba(0x90C0, size, ops, loc)
    }

    fn encode_adda_suba(
        &mut self,
        base: u16,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let areg = match dst {
            AddrMode::AddrReg(r) => r,
            _ => return Err(format!("{}: destination must be address register", loc)),
        };

        let opmode = if size == Size::Long { 0b111 } else { 0b011 };
        let (mode, reg, ext) = self.encode_ea_with_imm(&src, size)?;
        let opcode = (base & 0xF0C0)
            | ((areg as u16) << 9)
            | (opmode << 6)
            | ((mode as u16) << 3)
            | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_addi(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        self.encode_imm_op(0x0600, size, ops, loc)
    }

    fn encode_subi(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        self.encode_imm_op(0x0400, size, ops, loc)
    }

    fn encode_imm_op(
        &mut self,
        base: u16,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let imm = match src {
            AddrMode::Immediate(ref expr) => eval_expr(expr, self.symbols.as_map(), self.pc)?,
            _ => return Err(format!("{}: source must be immediate", loc)),
        };

        let sz = match size {
            Size::Byte => 0,
            Size::Word => 1,
            Size::Long => 2,
        };
        let (mode, reg, ext) = encode_ea(
            &dst,
            self.symbols.as_map(),
            self.pc + 4,
            self.pass,
            self.scope(),
        )?;
        let opcode = base | (sz << 6) | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);

        match size {
            Size::Byte | Size::Word => self.emit_word(imm as u16),
            Size::Long => self.emit_long(imm as u32),
        }
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_addq(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        self.encode_quick(0x5000, size, ops, loc)
    }

    fn encode_subq(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        self.encode_quick(0x5100, size, ops, loc)
    }

    fn encode_quick(
        &mut self,
        base: u16,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let imm = match src {
            AddrMode::Immediate(ref expr) => eval_expr(expr, self.symbols.as_map(), self.pc)? as u8,
            _ => return Err(format!("{}: source must be immediate 1-8", loc)),
        };
        let data = if imm == 8 { 0 } else { imm };

        let sz = match size {
            Size::Byte => 0,
            Size::Word => 1,
            Size::Long => 2,
        };
        let (mode, reg, ext) = encode_ea(
            &dst,
            self.symbols.as_map(),
            self.pc + 2,
            self.pass,
            self.scope(),
        )?;
        let opcode = base | ((data as u16) << 9) | (sz << 6) | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_addx(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        self.encode_x_op(0xD100, size, ops, loc)
    }

    fn encode_subx(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        self.encode_x_op(0x9100, size, ops, loc)
    }

    fn encode_x_op(
        &mut self,
        base: u16,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let sz = match size {
            Size::Byte => 0,
            Size::Word => 1,
            Size::Long => 2,
        };

        match (&src, &dst) {
            (AddrMode::DataReg(rx), AddrMode::DataReg(ry)) => {
                let opcode = base | (((*ry) as u16) << 9) | (sz << 6) | (*rx as u16);
                self.emit_word(opcode);
            }
            (AddrMode::PreDec(rx), AddrMode::PreDec(ry)) => {
                let opcode = base | (((*ry) as u16) << 9) | (sz << 6) | 0x08 | (*rx as u16);
                self.emit_word(opcode);
            }
            _ => {
                return Err(format!(
                    "{}: invalid operand combination for addx/subx",
                    loc
                ))
            }
        }
        Ok(())
    }

    fn encode_neg(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        self.encode_unary(0x4400, size, ops, loc)
    }

    fn encode_negx(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        self.encode_unary(0x4000, size, ops, loc)
    }

    fn encode_not(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        self.encode_unary(0x4600, size, ops, loc)
    }

    fn encode_tst(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        self.encode_unary(0x4A00, size, ops, loc)
    }

    fn encode_unary(
        &mut self,
        base: u16,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: requires 1 operand", loc));
        }
        let dst = parse_operand(ops[0], self.symbols.as_map())?;
        let sz = match size {
            Size::Byte => 0,
            Size::Word => 1,
            Size::Long => 2,
        };
        let (mode, reg, ext) = encode_ea(
            &dst,
            self.symbols.as_map(),
            self.pc + 2,
            self.pass,
            self.scope(),
        )?;
        let opcode = base | (sz << 6) | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_ext(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: ext requires 1 operand", loc));
        }
        let dst = parse_operand(ops[0], self.symbols.as_map())?;
        let dreg = match dst {
            AddrMode::DataReg(r) => r,
            _ => return Err(format!("{}: ext requires data register", loc)),
        };
        let opmode = if size == Size::Long { 0b011 } else { 0b010 };
        self.emit_word(0x4800 | (opmode << 6) | (dreg as u16));
        Ok(())
    }

    fn encode_mulu(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        self.encode_mul_div(0xC0C0, ops, loc)
    }

    fn encode_muls(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        self.encode_mul_div(0xC1C0, ops, loc)
    }

    fn encode_divu(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        self.encode_mul_div(0x80C0, ops, loc)
    }

    fn encode_divs(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        self.encode_mul_div(0x81C0, ops, loc)
    }

    fn encode_mul_div(
        &mut self,
        base: u16,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let dreg = match dst {
            AddrMode::DataReg(r) => r,
            _ => return Err(format!("{}: destination must be data register", loc)),
        };

        let (mode, reg, ext) = self.encode_ea_with_imm(&src, Size::Word)?;
        let opcode = base | ((dreg as u16) << 9) | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_cmp(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: cmp requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        // Auto-promote to CMPA if destination is address register
        if let AddrMode::AddrReg(areg) = dst {
            let opmode = if size == Size::Long { 0b111 } else { 0b011 };
            let (mode, reg, ext) = self.encode_ea_with_imm(&src, size)?;
            let opcode =
                0xB0C0 | ((areg as u16) << 9) | (opmode << 6) | ((mode as u16) << 3) | (reg as u16);
            self.emit_word(opcode);
            for e in ext {
                self.emit_word(e);
            }
            return Ok(());
        }

        // Auto-promote to CMPI if source is immediate and destination is not a register
        if matches!(src, AddrMode::Immediate(_)) && !matches!(dst, AddrMode::DataReg(_)) {
            // CMPI #imm,<ea>
            return self.encode_cmpi(size, ops, loc);
        }

        let dreg = match dst {
            AddrMode::DataReg(r) => r,
            _ => return Err(format!("{}: cmp destination must be data register", loc)),
        };

        let sz = match size {
            Size::Byte => 0,
            Size::Word => 1,
            Size::Long => 2,
        };
        let (mode, reg, ext) = self.encode_ea_with_imm(&src, size)?;
        let opcode =
            0xB000 | ((dreg as u16) << 9) | (sz << 6) | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_cmpa(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: cmpa requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let areg = match dst {
            AddrMode::AddrReg(r) => r,
            _ => {
                return Err(format!(
                    "{}: cmpa destination must be address register",
                    loc
                ))
            }
        };

        let opmode = if size == Size::Long { 0b111 } else { 0b011 };
        let (mode, reg, ext) = self.encode_ea_with_imm(&src, size)?;
        let opcode =
            0xB0C0 | ((areg as u16) << 9) | (opmode << 6) | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_cmpi(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        self.encode_imm_op(0x0C00, size, ops, loc)
    }

    fn encode_cmpm(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: cmpm requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let (ax, ay) = match (&src, &dst) {
            (AddrMode::PostInc(x), AddrMode::PostInc(y)) => (*x, *y),
            _ => return Err(format!("{}: cmpm requires (An)+,(An)+ operands", loc)),
        };

        let sz = match size {
            Size::Byte => 0,
            Size::Word => 1,
            Size::Long => 2,
        };
        let opcode = 0xB108 | ((ay as u16) << 9) | (sz << 6) | (ax as u16);
        self.emit_word(opcode);
        Ok(())
    }

    // Logical operations
    fn encode_and(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        // Auto-promote to ANDI if source is immediate and destination is not a data register
        if ops.len() == 2 {
            let src = parse_operand(ops[0], self.symbols.as_map())?;
            let dst = parse_operand(ops[1], self.symbols.as_map())?;
            if matches!(src, AddrMode::Immediate(_)) && !matches!(dst, AddrMode::DataReg(_)) {
                return self.encode_andi(size, ops, loc);
            }
        }
        self.encode_logical(0xC000, size, ops, loc)
    }

    fn encode_or(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        // Auto-promote to ORI if source is immediate and destination is not a data register
        if ops.len() == 2 {
            let src = parse_operand(ops[0], self.symbols.as_map())?;
            let dst = parse_operand(ops[1], self.symbols.as_map())?;
            if matches!(src, AddrMode::Immediate(_)) && !matches!(dst, AddrMode::DataReg(_)) {
                return self.encode_ori(size, ops, loc);
            }
        }
        self.encode_logical(0x8000, size, ops, loc)
    }

    fn encode_eor(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: eor requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let dreg = match src {
            AddrMode::DataReg(r) => r,
            _ => return Err(format!("{}: eor source must be data register", loc)),
        };

        let sz = match size {
            Size::Byte => 0,
            Size::Word => 1,
            Size::Long => 2,
        };
        let (mode, reg, ext) = encode_ea(
            &dst,
            self.symbols.as_map(),
            self.pc + 2,
            self.pass,
            self.scope(),
        )?;
        let opmode = sz | 0b100;
        let opcode =
            0xB000 | ((dreg as u16) << 9) | (opmode << 6) | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_logical(
        &mut self,
        base: u16,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let sz = match size {
            Size::Byte => 0,
            Size::Word => 1,
            Size::Long => 2,
        };

        match (&src, &dst) {
            (_, AddrMode::DataReg(dreg)) => {
                let (mode, reg, ext) = self.encode_ea_with_imm(&src, size)?;
                let opcode = base
                    | (((*dreg) as u16) << 9)
                    | (sz << 6)
                    | ((mode as u16) << 3)
                    | (reg as u16);
                self.emit_word(opcode);
                for e in ext {
                    self.emit_word(e);
                }
            }
            (AddrMode::DataReg(dreg), _) => {
                let (mode, reg, ext) = encode_ea(
                    &dst,
                    self.symbols.as_map(),
                    self.pc + 2,
                    self.pass,
                    self.scope(),
                )?;
                let opmode = sz | 0b100;
                let opcode = base
                    | (((*dreg) as u16) << 9)
                    | (opmode << 6)
                    | ((mode as u16) << 3)
                    | (reg as u16);
                self.emit_word(opcode);
                for e in ext {
                    self.emit_word(e);
                }
            }
            _ => return Err(format!("{}: invalid operand combination", loc)),
        }
        Ok(())
    }

    fn encode_andi(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        // Check for ANDI to CCR/SR
        if ops.len() == 2 {
            let dst = parse_operand(ops[1], self.symbols.as_map())?;
            if matches!(dst, AddrMode::Ccr) {
                return self.encode_imm_to_ccr(0x023C, ops, loc);
            }
            if matches!(dst, AddrMode::Sr) {
                return self.encode_imm_to_sr(0x027C, ops, loc);
            }
        }
        self.encode_imm_op(0x0200, size, ops, loc)
    }

    fn encode_ori(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() == 2 {
            let dst = parse_operand(ops[1], self.symbols.as_map())?;
            if matches!(dst, AddrMode::Ccr) {
                return self.encode_imm_to_ccr(0x003C, ops, loc);
            }
            if matches!(dst, AddrMode::Sr) {
                return self.encode_imm_to_sr(0x007C, ops, loc);
            }
        }
        self.encode_imm_op(0x0000, size, ops, loc)
    }

    fn encode_eori(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() == 2 {
            let dst = parse_operand(ops[1], self.symbols.as_map())?;
            if matches!(dst, AddrMode::Ccr) {
                return self.encode_imm_to_ccr(0x0A3C, ops, loc);
            }
            if matches!(dst, AddrMode::Sr) {
                return self.encode_imm_to_sr(0x0A7C, ops, loc);
            }
        }
        self.encode_imm_op(0x0A00, size, ops, loc)
    }

    fn encode_imm_to_ccr(
        &mut self,
        opcode: u16,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let imm = match src {
            AddrMode::Immediate(ref expr) => {
                eval_expr(expr, self.symbols.as_map(), self.pc)? as u16
            }
            _ => return Err(format!("{}: source must be immediate", loc)),
        };
        self.emit_word(opcode);
        self.emit_word(imm & 0xFF);
        Ok(())
    }

    fn encode_imm_to_sr(
        &mut self,
        opcode: u16,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let imm = match src {
            AddrMode::Immediate(ref expr) => {
                eval_expr(expr, self.symbols.as_map(), self.pc)? as u16
            }
            _ => return Err(format!("{}: source must be immediate", loc)),
        };
        self.emit_word(opcode);
        self.emit_word(imm);
        Ok(())
    }

    // Shift and rotate
    fn encode_shift(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
        kind: u16,
        _is_arith: bool,
    ) -> Result<(), String> {
        if ops.len() == 1 {
            // Memory shift
            let dst = parse_operand(ops[0], self.symbols.as_map())?;
            let (mode, reg, ext) = encode_ea(
                &dst,
                self.symbols.as_map(),
                self.pc + 2,
                self.pass,
                self.scope(),
            )?;
            let opcode = 0xE0C0 | (kind << 9) | ((mode as u16) << 3) | (reg as u16);
            self.emit_word(opcode);
            for e in ext {
                self.emit_word(e);
            }
        } else if ops.len() == 2 {
            let src = parse_operand(ops[0], self.symbols.as_map())?;
            let dst = parse_operand(ops[1], self.symbols.as_map())?;

            let dreg = match dst {
                AddrMode::DataReg(r) => r,
                _ => return Err(format!("{}: shift destination must be data register", loc)),
            };

            let sz = match size {
                Size::Byte => 0,
                Size::Word => 1,
                Size::Long => 2,
            };
            let direction = (kind >> 2) & 1; // Left=1, Right=0

            match src {
                AddrMode::DataReg(creg) => {
                    // Register count
                    let opcode = 0xE020
                        | ((creg as u16) << 9)
                        | (direction << 8)
                        | (sz << 6)
                        | ((kind & 3) << 3)
                        | (dreg as u16);
                    self.emit_word(opcode);
                }
                AddrMode::Immediate(ref expr) => {
                    let count = eval_expr(expr, self.symbols.as_map(), self.pc)? as u16;
                    let count = if count == 8 { 0 } else { count & 7 };
                    let opcode = 0xE000
                        | (count << 9)
                        | (direction << 8)
                        | (sz << 6)
                        | ((kind & 3) << 3)
                        | (dreg as u16);
                    self.emit_word(opcode);
                }
                _ => {
                    return Err(format!(
                        "{}: shift count must be immediate or data register",
                        loc
                    ))
                }
            }
        } else {
            return Err(format!("{}: shift requires 1 or 2 operands", loc));
        }
        Ok(())
    }

    // Bit operations
    fn encode_bit(
        &mut self,
        op: u16,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: bit operation requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        match src {
            AddrMode::DataReg(dreg) => {
                // Dynamic bit
                let (mode, reg, ext) = encode_ea(
                    &dst,
                    self.symbols.as_map(),
                    self.pc + 2,
                    self.pass,
                    self.scope(),
                )?;
                let opcode =
                    0x0100 | ((dreg as u16) << 9) | (op << 6) | ((mode as u16) << 3) | (reg as u16);
                self.emit_word(opcode);
                for e in ext {
                    self.emit_word(e);
                }
            }
            AddrMode::Immediate(ref expr) => {
                // Static bit
                let bit_num = eval_expr(expr, self.symbols.as_map(), self.pc)? as u16;
                let (mode, reg, ext) = encode_ea(
                    &dst,
                    self.symbols.as_map(),
                    self.pc + 4,
                    self.pass,
                    self.scope(),
                )?;
                let opcode = 0x0800 | (op << 6) | ((mode as u16) << 3) | (reg as u16);
                self.emit_word(opcode);
                self.emit_word(bit_num);
                for e in ext {
                    self.emit_word(e);
                }
            }
            _ => {
                return Err(format!(
                    "{}: bit number must be immediate or data register",
                    loc
                ))
            }
        }
        Ok(())
    }

    // BCD operations
    fn encode_bcd(
        &mut self,
        base: u16,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        match (&src, &dst) {
            (AddrMode::DataReg(rx), AddrMode::DataReg(ry)) => {
                let opcode = base | (((*ry) as u16) << 9) | (*rx as u16);
                self.emit_word(opcode);
            }
            (AddrMode::PreDec(rx), AddrMode::PreDec(ry)) => {
                let opcode = base | (((*ry) as u16) << 9) | 0x08 | (*rx as u16);
                self.emit_word(opcode);
            }
            _ => return Err(format!("{}: invalid operand combination for BCD", loc)),
        }
        Ok(())
    }

    fn encode_nbcd(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: nbcd requires 1 operand", loc));
        }
        let dst = parse_operand(ops[0], self.symbols.as_map())?;
        let (mode, reg, ext) = encode_ea(
            &dst,
            self.symbols.as_map(),
            self.pc + 2,
            self.pass,
            self.scope(),
        )?;
        let opcode = 0x4800 | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    // Branch instructions
    fn encode_bra(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        self.encode_branch(0x6000, ops, loc)
    }

    fn encode_bsr(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        self.encode_branch(0x6100, ops, loc)
    }

    fn encode_bcc(
        &mut self,
        mnemonic: &str,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        let cc = Condition::from_name(&mnemonic[1..])
            .ok_or_else(|| format!("{}: unknown condition", loc))?;
        let base = 0x6000 | ((cc as u16) << 8);
        self.encode_branch(base, ops, loc)
    }

    fn encode_branch(
        &mut self,
        base: u16,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: branch requires 1 operand", loc));
        }

        let mut parser = ExprParser::new(ops[0]);
        let expr = parser.parse_expr()?;

        // In pass 1, we may have forward references. Use placeholder.
        let target = match eval_expr_scoped(&expr, self.symbols.as_map(), self.pc, self.scope()) {
            Ok(t) => t,
            Err(_) if self.pass == 1 => self.pc as i64, // Placeholder
            Err(e) => return Err(e),
        };
        let disp = target - (self.pc as i64 + 2);

        if (-128..=127).contains(&disp) && disp != 0 {
            // Short branch
            let opcode = base | ((disp as u8) as u16);
            self.emit_word(opcode);
        } else {
            // Word displacement
            self.emit_word(base);
            self.emit_word(disp as u16);
        }
        Ok(())
    }

    fn encode_dbcc(
        &mut self,
        mnemonic: &str,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: dbcc requires 2 operands", loc));
        }

        let cc = if mnemonic == "DBRA" {
            Condition::False
        } else {
            let cc_name = &mnemonic[2..];
            Condition::from_name(cc_name).ok_or_else(|| format!("{}: unknown condition", loc))?
        };

        let dreg = match parse_operand(ops[0], self.symbols.as_map())? {
            AddrMode::DataReg(r) => r,
            _ => return Err(format!("{}: dbcc requires data register", loc)),
        };

        let mut parser = ExprParser::new(ops[1]);
        let expr = parser.parse_expr()?;
        // In pass 1, we may have forward references. Use placeholder.
        let target = match eval_expr_scoped(&expr, self.symbols.as_map(), self.pc, self.scope()) {
            Ok(t) => t,
            Err(_) if self.pass == 1 => self.pc as i64, // Placeholder
            Err(e) => return Err(e),
        };
        let disp = target - (self.pc as i64 + 2);

        let opcode = 0x50C8 | ((cc as u16) << 8) | (dreg as u16);
        self.emit_word(opcode);
        self.emit_word(disp as u16);
        Ok(())
    }

    fn encode_scc(
        &mut self,
        mnemonic: &str,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: scc requires 1 operand", loc));
        }

        let cc = Condition::from_name(&mnemonic[1..])
            .ok_or_else(|| format!("{}: unknown condition", loc))?;
        let dst = parse_operand(ops[0], self.symbols.as_map())?;
        let (mode, reg, ext) = encode_ea(
            &dst,
            self.symbols.as_map(),
            self.pc + 2,
            self.pass,
            self.scope(),
        )?;
        let opcode = 0x50C0 | ((cc as u16) << 8) | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    // Control instructions
    fn encode_jmp(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: jmp requires 1 operand", loc));
        }
        let dst = parse_operand(ops[0], self.symbols.as_map())?;
        let (mode, reg, ext) = encode_ea(
            &dst,
            self.symbols.as_map(),
            self.pc + 2,
            self.pass,
            self.scope(),
        )?;
        let opcode = 0x4EC0 | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_jsr(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: jsr requires 1 operand", loc));
        }
        let dst = parse_operand(ops[0], self.symbols.as_map())?;
        let (mode, reg, ext) = encode_ea(
            &dst,
            self.symbols.as_map(),
            self.pc + 2,
            self.pass,
            self.scope(),
        )?;
        let opcode = 0x4E80 | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_trap(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: trap requires 1 operand", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let vector = match src {
            AddrMode::Immediate(ref expr) => {
                eval_expr(expr, self.symbols.as_map(), self.pc)? as u16
            }
            _ => return Err(format!("{}: trap requires immediate vector", loc)),
        };
        if vector > 15 {
            return Err(format!("{}: trap vector must be 0-15", loc));
        }
        self.emit_word(0x4E40 | vector);
        Ok(())
    }

    fn encode_chk(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: chk requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let dreg = match dst {
            AddrMode::DataReg(r) => r,
            _ => return Err(format!("{}: chk destination must be data register", loc)),
        };

        let (mode, reg, ext) = self.encode_ea_with_imm(&src, Size::Word)?;
        let opcode = 0x4180 | ((dreg as u16) << 9) | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_link(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: link requires 2 operands", loc));
        }
        let areg_mode = parse_operand(ops[0], self.symbols.as_map())?;
        let disp_mode = parse_operand(ops[1], self.symbols.as_map())?;

        let areg = match areg_mode {
            AddrMode::AddrReg(r) => r,
            _ => return Err(format!("{}: link requires address register", loc)),
        };
        let disp = match disp_mode {
            AddrMode::Immediate(ref expr) => {
                eval_expr(expr, self.symbols.as_map(), self.pc)? as i16
            }
            _ => return Err(format!("{}: link requires immediate displacement", loc)),
        };

        self.emit_word(0x4E50 | (areg as u16));
        self.emit_word(disp as u16);
        Ok(())
    }

    fn encode_unlk(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: unlk requires 1 operand", loc));
        }
        let areg_mode = parse_operand(ops[0], self.symbols.as_map())?;
        let areg = match areg_mode {
            AddrMode::AddrReg(r) => r,
            _ => return Err(format!("{}: unlk requires address register", loc)),
        };
        self.emit_word(0x4E58 | (areg as u16));
        Ok(())
    }

    fn encode_stop(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: stop requires 1 operand", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let imm = match src {
            AddrMode::Immediate(ref expr) => {
                eval_expr(expr, self.symbols.as_map(), self.pc)? as u16
            }
            _ => return Err(format!("{}: stop requires immediate", loc)),
        };
        self.emit_word(0x4E72);
        self.emit_word(imm);
        Ok(())
    }

    fn encode_tas(&mut self, ops: &[&[LocatedToken]], loc: &SourceLoc) -> Result<(), String> {
        if ops.len() != 1 {
            return Err(format!("{}: tas requires 1 operand", loc));
        }
        let dst = parse_operand(ops[0], self.symbols.as_map())?;
        let (mode, reg, ext) = encode_ea(
            &dst,
            self.symbols.as_map(),
            self.pc + 2,
            self.pass,
            self.scope(),
        )?;
        let opcode = 0x4AC0 | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_movem(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: movem requires 2 operands", loc));
        }

        // Determine direction: register list to memory, or memory to register list
        // A register list contains only identifiers, minus (for ranges), and slash (for lists)
        // If first operand has parentheses or other tokens, it's a memory address
        let first_is_reglist = ops[0]
            .iter()
            .all(|t| matches!(t.token, Token::Ident(_) | Token::Minus | Token::Slash));

        let sz = if size == Size::Long { 1 } else { 0 };

        if first_is_reglist {
            // Try parsing first operand as register list
            let first_str: String = ops[0]
                .iter()
                .filter_map(|t| match &t.token {
                    Token::Ident(s) => Some(s.clone()),
                    Token::Minus => Some("-".to_string()),
                    Token::Slash => Some("/".to_string()),
                    _ => None,
                })
                .collect();

            if let Ok(mask) = parse_register_list(&first_str) {
                // Register list to memory
                let dst = parse_operand(ops[1], self.symbols.as_map())?;
                let (mode, reg, ext) = encode_ea(
                    &dst,
                    self.symbols.as_map(),
                    self.pc + 4,
                    self.pass,
                    self.scope(),
                )?;

                // For pre-decrement, reverse the mask
                let mask = if matches!(dst, AddrMode::PreDec(_)) {
                    reverse_bits_16(mask)
                } else {
                    mask
                };

                let opcode = 0x4880 | (sz << 6) | ((mode as u16) << 3) | (reg as u16);
                self.emit_word(opcode);
                self.emit_word(mask);
                for e in ext {
                    self.emit_word(e);
                }
                return Ok(());
            }
        }

        // Memory to register list
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let second_str: String = ops[1]
            .iter()
            .filter_map(|t| match &t.token {
                Token::Ident(s) => Some(s.clone()),
                Token::Minus => Some("-".to_string()),
                Token::Slash => Some("/".to_string()),
                _ => None,
            })
            .collect();
        let mask = parse_register_list(&second_str)?;

        let (mode, reg, ext) = encode_ea(
            &src,
            self.symbols.as_map(),
            self.pc + 4,
            self.pass,
            self.scope(),
        )?;
        let opcode = 0x4C80 | (sz << 6) | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        self.emit_word(mask);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_movep(
        &mut self,
        size: Size,
        ops: &[&[LocatedToken]],
        loc: &SourceLoc,
    ) -> Result<(), String> {
        if ops.len() != 2 {
            return Err(format!("{}: movep requires 2 operands", loc));
        }
        let src = parse_operand(ops[0], self.symbols.as_map())?;
        let dst = parse_operand(ops[1], self.symbols.as_map())?;

        let opmode = if size == Size::Long { 0b11 } else { 0b10 };

        match (&src, &dst) {
            (AddrMode::Disp(expr, areg), AddrMode::DataReg(dreg)) => {
                // Memory to register
                let disp = eval_expr(expr, self.symbols.as_map(), self.pc)? as u16;
                let opcode = 0x0108 | (((*dreg) as u16) << 9) | (opmode << 6) | (*areg as u16);
                self.emit_word(opcode);
                self.emit_word(disp);
            }
            (AddrMode::DataReg(dreg), AddrMode::Disp(expr, areg)) => {
                // Register to memory
                let disp = eval_expr(expr, self.symbols.as_map(), self.pc)? as u16;
                let opcode = 0x0188 | (((*dreg) as u16) << 9) | (opmode << 6) | (*areg as u16);
                self.emit_word(opcode);
                self.emit_word(disp);
            }
            _ => return Err(format!("{}: movep requires d(An),Dn or Dn,d(An)", loc)),
        }
        Ok(())
    }

    fn encode_move_to_sr(&mut self, src: &AddrMode, _loc: &SourceLoc) -> Result<(), String> {
        let (mode, reg, ext) = self.encode_ea_with_imm(src, Size::Word)?;
        let opcode = 0x46C0 | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_move_from_sr(&mut self, dst: &AddrMode, _loc: &SourceLoc) -> Result<(), String> {
        let (mode, reg, ext) = encode_ea(
            dst,
            self.symbols.as_map(),
            self.pc + 2,
            self.pass,
            self.scope(),
        )?;
        let opcode = 0x40C0 | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_move_to_ccr(&mut self, src: &AddrMode, _loc: &SourceLoc) -> Result<(), String> {
        let (mode, reg, ext) = self.encode_ea_with_imm(src, Size::Word)?;
        let opcode = 0x44C0 | ((mode as u16) << 3) | (reg as u16);
        self.emit_word(opcode);
        for e in ext {
            self.emit_word(e);
        }
        Ok(())
    }

    fn encode_move_to_usp(&mut self, src: &AddrMode, loc: &SourceLoc) -> Result<(), String> {
        // MOVE An,USP: 0100 1110 0110 0 An
        let areg = match src {
            AddrMode::AddrReg(r) => *r,
            _ => {
                return Err(format!(
                    "{}: move to USP source must be address register",
                    loc
                ))
            }
        };
        let opcode = 0x4E60 | (areg as u16);
        self.emit_word(opcode);
        Ok(())
    }

    fn encode_move_from_usp(&mut self, dst: &AddrMode, loc: &SourceLoc) -> Result<(), String> {
        // MOVE USP,An: 0100 1110 0110 1 An
        let areg = match dst {
            AddrMode::AddrReg(r) => *r,
            _ => {
                return Err(format!(
                    "{}: move from USP destination must be address register",
                    loc
                ))
            }
        };
        let opcode = 0x4E68 | (areg as u16);
        self.emit_word(opcode);
        Ok(())
    }
}

/// Reverses bits in a 16-bit value (for MOVEM pre-decrement).
fn reverse_bits_16(mut v: u16) -> u16 {
    let mut result = 0u16;
    for _ in 0..16 {
        result = (result << 1) | (v & 1);
        v >>= 1;
    }
    result
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // Lexer tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_lexer_basic_tokens() {
        let source = "move.l #$1234,d0";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();

        // move.l tokenizes as: "move", ".l" (identifier), "#", 0x1234, ",", "d0"
        assert!(matches!(tokens[0].token, Token::Ident(ref s) if s == "move"));
        assert!(matches!(tokens[1].token, Token::Ident(ref s) if s == ".l"));
        assert!(matches!(tokens[2].token, Token::Hash));
        assert!(matches!(tokens[3].token, Token::Number(0x1234)));
        assert!(matches!(tokens[4].token, Token::Comma));
        assert!(matches!(tokens[5].token, Token::Ident(ref s) if s == "d0"));
    }

    #[test]
    fn test_lexer_hex_numbers() {
        let source = "$FF $1234 $DEADBEEF";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(tokens[0].token, Token::Number(0xFF)));
        assert!(matches!(tokens[1].token, Token::Number(0x1234)));
        assert!(matches!(tokens[2].token, Token::Number(0xDEADBEEF)));
    }

    #[test]
    fn test_lexer_binary_numbers() {
        let source = "%1010 %11110000";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(tokens[0].token, Token::Number(0b1010)));
        assert!(matches!(tokens[1].token, Token::Number(0b11110000)));
    }

    #[test]
    fn test_lexer_strings() {
        let source = r#""hello" "world\n""#;
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(tokens[0].token, Token::String(ref s) if s == "hello"));
        assert!(matches!(tokens[1].token, Token::String(ref s) if s == "world\n"));
    }

    #[test]
    fn test_lexer_local_labels() {
        let source = ".loop .done";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(tokens[0].token, Token::Ident(ref s) if s == ".loop"));
        assert!(matches!(tokens[1].token, Token::Ident(ref s) if s == ".done"));
    }

    #[test]
    fn test_lexer_comments() {
        let source = "move.l d0,d1 ; this is a comment\nadd.l d2,d3";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();

        // Should have: move .l d0 , d1 newline add .l d2 , d3 eof
        let idents: Vec<_> = tokens
            .iter()
            .filter_map(|t| {
                if let Token::Ident(s) = &t.token {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            idents,
            vec!["move", ".l", "d0", "d1", "add", ".l", "d2", "d3"]
        );
    }

    #[test]
    fn test_lexer_operators() {
        let source = "+ - * / << >> & | ^ ~ < > <= >= = <>";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(tokens[0].token, Token::Plus));
        assert!(matches!(tokens[1].token, Token::Minus));
        assert!(matches!(tokens[2].token, Token::Star));
        assert!(matches!(tokens[3].token, Token::Slash));
        assert!(matches!(tokens[4].token, Token::LShift));
        assert!(matches!(tokens[5].token, Token::RShift));
        assert!(matches!(tokens[6].token, Token::Ampersand));
        assert!(matches!(tokens[7].token, Token::Pipe));
        assert!(matches!(tokens[8].token, Token::Caret));
        assert!(matches!(tokens[9].token, Token::Tilde));
        assert!(matches!(tokens[10].token, Token::Less));
        assert!(matches!(tokens[11].token, Token::Greater));
        assert!(matches!(tokens[12].token, Token::LessEq));
        assert!(matches!(tokens[13].token, Token::GreaterEq));
        assert!(matches!(tokens[14].token, Token::Equals));
        assert!(matches!(tokens[15].token, Token::NotEquals));
    }

    #[test]
    fn test_lexer_char_literal() {
        let source = "'A' 'x'";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(tokens[0].token, Token::Char('A')));
        assert!(matches!(tokens[1].token, Token::Char('x')));
    }

    // ------------------------------------------------------------------------
    // Expression parser tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_expr_simple_number() {
        let source = "42";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ExprParser::new(&tokens);
        let expr = parser.parse_expr().unwrap();

        let result = eval_expr(&expr, &HashMap::new(), 0).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_expr_addition() {
        let source = "10 + 20";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ExprParser::new(&tokens);
        let expr = parser.parse_expr().unwrap();

        let result = eval_expr(&expr, &HashMap::new(), 0).unwrap();
        assert_eq!(result, 30);
    }

    #[test]
    fn test_expr_complex() {
        let source = "(10 + 5) * 2 - 6 / 3";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ExprParser::new(&tokens);
        let expr = parser.parse_expr().unwrap();

        let result = eval_expr(&expr, &HashMap::new(), 0).unwrap();
        assert_eq!(result, 28); // (15 * 2) - 2 = 28
    }

    #[test]
    fn test_expr_bitwise() {
        let source = "$FF & $0F";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ExprParser::new(&tokens);
        let expr = parser.parse_expr().unwrap();

        let result = eval_expr(&expr, &HashMap::new(), 0).unwrap();
        assert_eq!(result, 0x0F);
    }

    #[test]
    fn test_expr_shift() {
        let source = "1 << 8";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ExprParser::new(&tokens);
        let expr = parser.parse_expr().unwrap();

        let result = eval_expr(&expr, &HashMap::new(), 0).unwrap();
        assert_eq!(result, 256);
    }

    #[test]
    fn test_expr_symbols() {
        let source = "BASE + OFFSET";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ExprParser::new(&tokens);
        let expr = parser.parse_expr().unwrap();

        let mut symbols = HashMap::new();
        symbols.insert("BASE".to_string(), 0x1000);
        symbols.insert("OFFSET".to_string(), 0x100);

        let result = eval_expr(&expr, &symbols, 0).unwrap();
        assert_eq!(result, 0x1100);
    }

    #[test]
    fn test_expr_current_pc() {
        // The * at column 1 is treated as a comment, so prefix with a space
        // In real assembly, expressions with * appear after a label/directive
        let source = " * + 4";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ExprParser::new(&tokens);
        let expr = parser.parse_expr().unwrap();

        let result = eval_expr(&expr, &HashMap::new(), 0x1000).unwrap();
        assert_eq!(result, 0x1004);
    }

    #[test]
    fn test_expr_negation() {
        let source = "-10 + 15";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ExprParser::new(&tokens);
        let expr = parser.parse_expr().unwrap();

        let result = eval_expr(&expr, &HashMap::new(), 0).unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn test_expr_comparison() {
        let source = "10 > 5";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ExprParser::new(&tokens);
        let expr = parser.parse_expr().unwrap();

        let result = eval_expr(&expr, &HashMap::new(), 0).unwrap();
        assert_eq!(result, 1);
    }

    // ------------------------------------------------------------------------
    // Symbol table tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_symbol_table_basic() {
        let mut st = SymbolTable::new();
        st.define("FOO", 100).unwrap();
        st.define("BAR", 200).unwrap();

        assert_eq!(st.get("FOO"), Some(100));
        assert_eq!(st.get("BAR"), Some(200));
        assert_eq!(st.get("BAZ"), None);
    }

    #[test]
    fn test_symbol_table_local_labels() {
        let mut st = SymbolTable::new();
        st.define("main", 0x1000).unwrap();
        st.define(".loop", 0x1010).unwrap();
        st.define(".done", 0x1020).unwrap();

        assert_eq!(st.get(".loop"), Some(0x1010));
        assert_eq!(st.get(".done"), Some(0x1020));

        // New global label changes scope
        st.define("other", 0x2000).unwrap();
        st.define(".loop", 0x2010).unwrap();

        // The new .loop is different from main's .loop
        assert_eq!(st.get(".loop"), Some(0x2010));
    }

    #[test]
    fn test_symbol_table_redefine_same_value() {
        let mut st = SymbolTable::new();
        st.define("FOO", 100).unwrap();
        // Same value is OK
        st.define("FOO", 100).unwrap();
        assert_eq!(st.get("FOO"), Some(100));
    }

    #[test]
    fn test_symbol_table_redefine_allowed() {
        let mut st = SymbolTable::new();
        st.define("FOO", 100).unwrap();
        // Redefinition is allowed for two-pass assembly (pass 2 updates values)
        st.define("FOO", 200).unwrap();
        assert_eq!(st.get("FOO"), Some(200));
    }

    // ------------------------------------------------------------------------
    // Register parsing tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_parse_register_data() {
        for i in 0..8 {
            let name = format!("d{}", i);
            assert_eq!(parse_register(&name), Some((i, false)));
            let upper = format!("D{}", i);
            assert_eq!(parse_register(&upper), Some((i, false)));
        }
    }

    #[test]
    fn test_parse_register_address() {
        for i in 0..8 {
            let name = format!("a{}", i);
            assert_eq!(parse_register(&name), Some((i, true)));
            let upper = format!("A{}", i);
            assert_eq!(parse_register(&upper), Some((i, true)));
        }
    }

    #[test]
    fn test_parse_register_sp() {
        assert_eq!(parse_register("sp"), Some((7, true)));
        assert_eq!(parse_register("SP"), Some((7, true)));
    }

    #[test]
    fn test_parse_register_invalid() {
        assert_eq!(parse_register("d8"), None);
        assert_eq!(parse_register("a8"), None);
        assert_eq!(parse_register("x0"), None);
        assert_eq!(parse_register("foo"), None);
    }

    // ------------------------------------------------------------------------
    // Register list parsing tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_parse_register_list_single() {
        assert_eq!(parse_register_list("d0").unwrap(), 0x0001);
        assert_eq!(parse_register_list("d7").unwrap(), 0x0080);
        assert_eq!(parse_register_list("a0").unwrap(), 0x0100);
        assert_eq!(parse_register_list("a7").unwrap(), 0x8000);
    }

    #[test]
    fn test_parse_register_list_range() {
        assert_eq!(parse_register_list("d0-d3").unwrap(), 0x000F);
        assert_eq!(parse_register_list("a0-a2").unwrap(), 0x0700);
    }

    #[test]
    fn test_parse_register_list_multiple() {
        assert_eq!(parse_register_list("d0/d2/d4").unwrap(), 0x0015);
        assert_eq!(parse_register_list("d0-d3/a0-a2").unwrap(), 0x070F);
    }

    // ------------------------------------------------------------------------
    // Size parsing tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_size_from_suffix() {
        assert_eq!(Size::from_suffix("b"), Some(Size::Byte));
        assert_eq!(Size::from_suffix("B"), Some(Size::Byte));
        assert_eq!(Size::from_suffix("w"), Some(Size::Word));
        assert_eq!(Size::from_suffix("W"), Some(Size::Word));
        assert_eq!(Size::from_suffix("l"), Some(Size::Long));
        assert_eq!(Size::from_suffix("L"), Some(Size::Long));
        assert_eq!(Size::from_suffix("s"), Some(Size::Word));
        assert_eq!(Size::from_suffix("x"), None);
    }

    #[test]
    fn test_size_bytes() {
        assert_eq!(Size::Byte.bytes(), 1);
        assert_eq!(Size::Word.bytes(), 2);
        assert_eq!(Size::Long.bytes(), 4);
    }

    // ------------------------------------------------------------------------
    // Condition code tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_condition_from_name() {
        assert!(matches!(Condition::from_name("eq"), Some(Condition::Eq)));
        assert!(matches!(Condition::from_name("NE"), Some(Condition::Ne)));
        assert!(matches!(Condition::from_name("bra"), None)); // BRA is not a condition
        assert!(matches!(Condition::from_name("ra"), Some(Condition::True)));
        assert!(matches!(Condition::from_name("cc"), Some(Condition::Cc)));
        assert!(matches!(Condition::from_name("hs"), Some(Condition::Cc)));
    }

    // ------------------------------------------------------------------------
    // Assembler emit tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_assembler_emit_byte() {
        let mut asm = Assembler::new();
        asm.pass = 2;
        asm.origin = 0;
        asm.pc = 0;

        asm.emit_byte(0x12);
        asm.emit_byte(0x34);

        assert_eq!(asm.output, vec![0x12, 0x34]);
        assert_eq!(asm.pc, 2);
    }

    #[test]
    fn test_assembler_emit_word() {
        let mut asm = Assembler::new();
        asm.pass = 2;
        asm.origin = 0;
        asm.pc = 0;

        asm.emit_word(0x1234);

        assert_eq!(asm.output, vec![0x12, 0x34]);
        assert_eq!(asm.pc, 2);
    }

    #[test]
    fn test_assembler_emit_long() {
        let mut asm = Assembler::new();
        asm.pass = 2;
        asm.origin = 0;
        asm.pc = 0;

        asm.emit_long(0x12345678);

        assert_eq!(asm.output, vec![0x12, 0x34, 0x56, 0x78]);
        assert_eq!(asm.pc, 4);
    }

    #[test]
    fn test_assembler_align_word() {
        let mut asm = Assembler::new();
        asm.pass = 2;
        asm.origin = 0;
        asm.pc = 0;

        asm.emit_byte(0xFF);
        assert_eq!(asm.pc, 1);

        asm.align_word();
        assert_eq!(asm.pc, 2);
        assert_eq!(asm.output, vec![0xFF, 0x00]);

        // Already aligned, should not add padding
        asm.align_word();
        assert_eq!(asm.pc, 2);
    }

    // ------------------------------------------------------------------------
    // Line parser tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_parse_line_label_only() {
        let source = "main:";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();

        assert_eq!(line.label.as_deref(), Some("main"));
        assert!(line.mnemonic.is_none());
    }

    #[test]
    fn test_parse_line_instruction() {
        let source = "  move.l d0,d1";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();

        assert!(line.label.is_none());
        assert_eq!(line.mnemonic.as_deref(), Some("move"));
        assert_eq!(line.size, Some(Size::Long));
    }

    #[test]
    fn test_parse_line_label_and_instruction() {
        let source = "loop: bra loop";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();

        assert_eq!(line.label.as_deref(), Some("loop"));
        assert_eq!(line.mnemonic.as_deref(), Some("bra"));
    }

    #[test]
    fn test_parse_line_equ() {
        let source = "SIZE equ 100";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();

        assert_eq!(line.label.as_deref(), Some("SIZE"));
        assert_eq!(line.mnemonic.as_deref(), Some("equ"));
    }

    // ------------------------------------------------------------------------
    // Directive tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_directive_org() {
        let source = "  org $1000";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();

        let mut asm = Assembler::new();
        asm.pass = 1;
        asm.process_line(&line).unwrap();

        assert_eq!(asm.pc, 0x1000);
        assert_eq!(asm.origin, 0x1000);
    }

    #[test]
    fn test_directive_equ() {
        let source = "RAMBASE equ $E00000";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();

        let mut asm = Assembler::new();
        asm.pass = 1;
        asm.process_line(&line).unwrap();

        assert_eq!(asm.symbols.get("RAMBASE"), Some(0xE00000));
    }

    #[test]
    fn test_directive_equ_expression() {
        // First define BASE, then use it in OFFSET
        let mut asm = Assembler::new();
        asm.pass = 1;

        let source1 = "BASE equ $1000";
        let mut lexer = Lexer::new(source1, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();
        asm.process_line(&line).unwrap();

        let source2 = "NEXT equ BASE+$100";
        let mut lexer = Lexer::new(source2, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();
        asm.process_line(&line).unwrap();

        assert_eq!(asm.symbols.get("BASE"), Some(0x1000));
        assert_eq!(asm.symbols.get("NEXT"), Some(0x1100));
    }

    #[test]
    fn test_directive_dc_byte() {
        let source = "  dc.b 1,2,3";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();

        let mut asm = Assembler::new();
        asm.pass = 2;
        asm.origin = 0;
        asm.pc = 0;
        asm.process_line(&line).unwrap();

        assert_eq!(asm.output, vec![1, 2, 3]);
        assert_eq!(asm.pc, 3);
    }

    #[test]
    fn test_directive_dc_word() {
        let source = "  dc.w $1234,$5678";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();

        let mut asm = Assembler::new();
        asm.pass = 2;
        asm.origin = 0;
        asm.pc = 0;
        asm.process_line(&line).unwrap();

        assert_eq!(asm.output, vec![0x12, 0x34, 0x56, 0x78]);
        assert_eq!(asm.pc, 4);
    }

    #[test]
    fn test_directive_dc_long() {
        let source = "  dc.l $12345678";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();

        let mut asm = Assembler::new();
        asm.pass = 2;
        asm.origin = 0;
        asm.pc = 0;
        asm.process_line(&line).unwrap();

        assert_eq!(asm.output, vec![0x12, 0x34, 0x56, 0x78]);
        assert_eq!(asm.pc, 4);
    }

    #[test]
    fn test_directive_dc_string() {
        let source = r#"  dc.b "Hi",0"#;
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();

        let mut asm = Assembler::new();
        asm.pass = 2;
        asm.origin = 0;
        asm.pc = 0;
        asm.process_line(&line).unwrap();

        assert_eq!(asm.output, vec![b'H', b'i', 0]);
    }

    #[test]
    fn test_directive_ds() {
        let source = "  ds.b 10";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();

        let mut asm = Assembler::new();
        asm.pass = 2;
        asm.origin = 0;
        asm.pc = 0;
        asm.process_line(&line).unwrap();

        assert_eq!(asm.output.len(), 10);
        assert_eq!(asm.pc, 10);
    }

    #[test]
    fn test_directive_rsset_and_rs() {
        let mut asm = Assembler::new();
        asm.pass = 1;

        // rsset $100
        let source1 = "  rsset $100";
        let mut lexer = Lexer::new(source1, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();
        asm.process_line(&line).unwrap();
        assert_eq!(asm.rs_counter, 0x100);

        // FIELD1 rs.l 1
        let source2 = "FIELD1 rs.l 1";
        let mut lexer = Lexer::new(source2, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();
        asm.process_line(&line).unwrap();

        assert_eq!(asm.symbols.get("FIELD1"), Some(0x100));
        assert_eq!(asm.rs_counter, 0x104);

        // FIELD2 rs.w 2
        let source3 = "FIELD2 rs.w 2";
        let mut lexer = Lexer::new(source3, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();
        asm.process_line(&line).unwrap();

        assert_eq!(asm.symbols.get("FIELD2"), Some(0x104));
        assert_eq!(asm.rs_counter, 0x108);
    }

    #[test]
    fn test_directive_even() {
        let source = "  even";
        let mut lexer = Lexer::new(source, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let line = parse_line(&tokens).unwrap();

        let mut asm = Assembler::new();
        asm.pass = 2;
        asm.origin = 0;
        asm.pc = 3; // Odd address
        asm.process_line(&line).unwrap();

        assert_eq!(asm.pc, 4); // Aligned to even
    }

    // ------------------------------------------------------------------------
    // Preprocessor tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_preprocessor_rept() {
        let source = "rept 3\n  nop\nendr";
        let mut pp = Preprocessor::new();
        let result = pp
            .preprocess(source, std::path::Path::new("test.asm"))
            .unwrap();

        // Should expand to 3 nop lines
        let lines: Vec<_> = result.lines().filter(|l| l.trim() == "nop").collect();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_preprocessor_macro_simple() {
        let source = "mymacro macro\n  move.l d0,d1\nendm\n  mymacro";
        let mut pp = Preprocessor::new();
        let result = pp
            .preprocess(source, std::path::Path::new("test.asm"))
            .unwrap();

        // Should contain the expanded macro
        assert!(result.contains("move.l d0,d1"));
    }

    #[test]
    fn test_preprocessor_macro_with_args() {
        let source = "swap2 macro\n  move.l \\1,\\2\nendm\n  swap2 d0,d1";
        let mut pp = Preprocessor::new();
        let result = pp
            .preprocess(source, std::path::Path::new("test.asm"))
            .unwrap();

        // Should substitute \1 and \2
        assert!(result.contains("move.l d0,d1"));
    }

    #[test]
    fn test_preprocessor_unique_labels() {
        let source = "test macro\n.loop\\@: bra .loop\\@\nendm\n  test\n  test";
        let mut pp = Preprocessor::new();
        let result = pp
            .preprocess(source, std::path::Path::new("test.asm"))
            .unwrap();

        // Should have two different unique suffixes
        assert!(result.contains(".loop0:"));
        assert!(result.contains(".loop1:"));
    }

    #[test]
    fn test_preprocessor_if_true() {
        let source = "if 1\n  included\nendif";
        let mut pp = Preprocessor::new();
        let result = pp
            .preprocess(source, std::path::Path::new("test.asm"))
            .unwrap();

        assert!(result.contains("included"));
    }

    #[test]
    fn test_preprocessor_if_false() {
        let source = "if 0\n  excluded\nendif";
        let mut pp = Preprocessor::new();
        let result = pp
            .preprocess(source, std::path::Path::new("test.asm"))
            .unwrap();

        assert!(!result.contains("excluded"));
    }

    #[test]
    fn test_preprocessor_if_else() {
        let source = "if 0\n  excluded\nelse\n  included\nendif";
        let mut pp = Preprocessor::new();
        let result = pp
            .preprocess(source, std::path::Path::new("test.asm"))
            .unwrap();

        assert!(!result.contains("excluded"));
        assert!(result.contains("included"));
    }

    // ------------------------------------------------------------------------
    // Operand parsing tests
    // ------------------------------------------------------------------------

    // Helper to tokenize and filter newlines/eof for operand tests
    fn tokenize_operand(source: &str) -> Vec<LocatedToken> {
        let mut lexer = Lexer::new(source, "test.asm");
        lexer
            .tokenize()
            .unwrap()
            .into_iter()
            .filter(|t| !matches!(t.token, Token::Newline | Token::Eof))
            .collect()
    }

    #[test]
    fn test_parse_operand_data_reg() {
        let tokens = tokenize_operand("d0");
        let mode = parse_operand(&tokens, &HashMap::new()).unwrap();
        assert!(matches!(mode, AddrMode::DataReg(0)));
    }

    #[test]
    fn test_parse_operand_addr_reg() {
        let tokens = tokenize_operand("a3");
        let mode = parse_operand(&tokens, &HashMap::new()).unwrap();
        assert!(matches!(mode, AddrMode::AddrReg(3)));
    }

    #[test]
    fn test_parse_operand_immediate() {
        let tokens = tokenize_operand("#$1234");
        let mode = parse_operand(&tokens, &HashMap::new()).unwrap();
        assert!(matches!(mode, AddrMode::Immediate(_)));
    }

    #[test]
    fn test_parse_operand_addr_indirect() {
        let tokens = tokenize_operand("(a0)");
        let mode = parse_operand(&tokens, &HashMap::new()).unwrap();
        assert!(matches!(mode, AddrMode::AddrInd(0)));
    }

    #[test]
    fn test_parse_operand_post_inc() {
        let tokens = tokenize_operand("(a1)+");
        let mode = parse_operand(&tokens, &HashMap::new()).unwrap();
        assert!(matches!(mode, AddrMode::PostInc(1)));
    }

    #[test]
    fn test_parse_operand_pre_dec() {
        let tokens = tokenize_operand("-(a7)");
        let mode = parse_operand(&tokens, &HashMap::new()).unwrap();
        assert!(matches!(mode, AddrMode::PreDec(7)));
    }

    #[test]
    fn test_parse_operand_disp() {
        let tokens = tokenize_operand("4(a0)");
        let mode = parse_operand(&tokens, &HashMap::new()).unwrap();
        assert!(matches!(mode, AddrMode::Disp(_, 0)));
    }

    // ------------------------------------------------------------------------
    // Instruction encoding tests
    // ------------------------------------------------------------------------

    fn assemble_line(line: &str) -> Vec<u8> {
        let mut asm = Assembler::new();
        asm.pass = 2; // Need to be in pass 2 to emit bytes
        let mut lexer = Lexer::new(line, "test.asm");
        let tokens = lexer.tokenize().unwrap();
        let parsed = parse_line(&tokens).unwrap();
        asm.process_line(&parsed).unwrap();
        asm.output
    }

    #[test]
    fn test_encode_nop() {
        let output = assemble_line("nop");
        assert_eq!(output, vec![0x4E, 0x71]);
    }

    #[test]
    fn test_encode_rts() {
        let output = assemble_line("rts");
        assert_eq!(output, vec![0x4E, 0x75]);
    }

    #[test]
    fn test_encode_moveq() {
        let output = assemble_line("moveq #0,d0");
        assert_eq!(output, vec![0x70, 0x00]);
    }

    #[test]
    fn test_encode_moveq_neg() {
        let output = assemble_line("moveq #-1,d3");
        // MOVEQ #-1,D3 = 0x76FF (D3 = 011, data = 0xFF)
        assert_eq!(output, vec![0x76, 0xFF]);
    }

    #[test]
    fn test_encode_clr_l() {
        let output = assemble_line("clr.l d0");
        // CLR.L D0 = 0x4280 (size=10, mode=000, reg=000)
        assert_eq!(output, vec![0x42, 0x80]);
    }

    #[test]
    fn test_encode_move_reg_to_reg() {
        let output = assemble_line("move.l d0,d1");
        // MOVE.L D0,D1: size=10, dst_reg=001, dst_mode=000, src_mode=000, src_reg=000
        // = 0010 001 000 000 000 = 0x2200
        assert_eq!(output, vec![0x22, 0x00]);
    }

    #[test]
    fn test_encode_lea() {
        let output = assemble_line("lea $1000,a0");
        // LEA $1000,A0 = 0x41F8 + 0x1000 (absolute short, since $1000 fits in 16 bits)
        assert_eq!(output, vec![0x41, 0xF8, 0x10, 0x00]);
    }

    #[test]
    fn test_encode_addq() {
        let output = assemble_line("addq.l #1,a0");
        // ADDQ.L #1,A0 = 0101 001 0 10 001 000 = 0x5288
        assert_eq!(output, vec![0x52, 0x88]);
    }

    #[test]
    fn test_encode_subq() {
        let output = assemble_line("subq.w #2,d3");
        // SUBQ.W #2,D3 = 0101 010 1 01 000 011 = 0x5543
        assert_eq!(output, vec![0x55, 0x43]);
    }

    #[test]
    fn test_encode_trap() {
        let output = assemble_line("trap #15");
        // TRAP #15 = 0x4E4F
        assert_eq!(output, vec![0x4E, 0x4F]);
    }

    #[test]
    fn test_encode_swap() {
        let output = assemble_line("swap d5");
        // SWAP D5 = 0x4845
        assert_eq!(output, vec![0x48, 0x45]);
    }

    #[test]
    fn test_encode_ext_w() {
        let output = assemble_line("ext.w d2");
        // EXT.W D2 = 0x4882
        assert_eq!(output, vec![0x48, 0x82]);
    }

    #[test]
    fn test_encode_ext_l() {
        let output = assemble_line("ext.l d2");
        // EXT.L D2 = 0x48C2
        assert_eq!(output, vec![0x48, 0xC2]);
    }

    #[test]
    fn test_encode_tst_b() {
        let output = assemble_line("tst.b d0");
        // TST.B D0 = 0x4A00
        assert_eq!(output, vec![0x4A, 0x00]);
    }

    #[test]
    fn test_encode_jmp() {
        let output = assemble_line("jmp (a0)");
        // JMP (A0) = 0x4ED0
        assert_eq!(output, vec![0x4E, 0xD0]);
    }

    #[test]
    fn test_encode_jsr() {
        let output = assemble_line("jsr (a1)");
        // JSR (A1) = 0x4E91
        assert_eq!(output, vec![0x4E, 0x91]);
    }

    #[test]
    fn test_encode_link() {
        let output = assemble_line("link a6,#-4");
        // LINK A6,#-4 = 0x4E56 0xFFFC
        assert_eq!(output, vec![0x4E, 0x56, 0xFF, 0xFC]);
    }

    #[test]
    fn test_encode_unlk() {
        let output = assemble_line("unlk a6");
        // UNLK A6 = 0x4E5E
        assert_eq!(output, vec![0x4E, 0x5E]);
    }

    #[test]
    fn test_register_list_single() {
        let mask = parse_register_list("d0").unwrap();
        assert_eq!(mask, 0x0001);
    }

    #[test]
    fn test_register_list_range() {
        let mask = parse_register_list("d0-d3").unwrap();
        assert_eq!(mask, 0x000F);
    }

    #[test]
    fn test_register_list_multiple() {
        let mask = parse_register_list("d0-d2/a0-a2").unwrap();
        assert_eq!(mask, 0x0707);
    }

    // ------------------------------------------------------------------------
    // Full assembly tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_assemble_simple_program() {
        let source = r#"
            org $1000
start:
            moveq #0,d0
            nop
            rts
"#;
        let mut asm = Assembler::new();
        let output = asm
            .assemble_source(source, std::path::Path::new("test.asm"))
            .unwrap();

        // moveq #0,d0 = 0x7000
        // nop = 0x4E71
        // rts = 0x4E75
        assert_eq!(output, vec![0x70, 0x00, 0x4E, 0x71, 0x4E, 0x75]);
        assert_eq!(asm.symbols.get("start"), Some(0x1000));
    }

    #[test]
    fn test_lea_pc_relative_local_label() {
        let source = r#"
    org $1000
start:
    lea.l (.str,pc),a0
    rts
.str:
    dc.b "Test",0
"#;
        let mut asm = Assembler::new();
        let output = asm
            .assemble_source(source, std::path::Path::new("test.asm"))
            .unwrap();
        // Should succeed - LEA with PC-relative addressing to local label
        assert!(!output.is_empty());
    }

    #[test]
    fn test_indexed_addressing_with_size() {
        let source = r#"
    org $1000
    move.b (a5,d0.w),d0
"#;
        let mut asm = Assembler::new();
        let output = asm
            .assemble_source(source, std::path::Path::new("test.asm"))
            .unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_pc_indexed_addressing() {
        let source = r#"
    org $1000
table:
    dc.w 0
    move.w table(pc,d0.w),d1
"#;
        let mut asm = Assembler::new();
        let output = asm
            .assemble_source(source, std::path::Path::new("test.asm"))
            .unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_pc_indexed_jmp() {
        let source = r#"
    org $1000
    move.w fmt_jumptable(pc,d0.w),d1
    jmp fmt_jumptable(pc,d1.w)
fmt_jumptable:
    dc.w 0
"#;
        let mut asm = Assembler::new();
        let output = asm
            .assemble_source(source, std::path::Path::new("test.asm"))
            .unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_index_no_size_suffix() {
        // Index mode without size suffix on index register
        let source = r#"
    org $1000
    lea.l (a0,d0),a3
"#;
        let mut asm = Assembler::new();
        let output = asm
            .assemble_source(source, std::path::Path::new("test.asm"))
            .unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_pc_index_3_component() {
        // PC-relative indexed: (label,pc,Xn)
        let source = r#"
    org $1000
hexdigits:
    dc.b "0123456789ABCDEF"
    move.b (hexdigits,pc,d2),d2
"#;
        let mut asm = Assembler::new();
        let output = asm
            .assemble_source(source, std::path::Path::new("test.asm"))
            .unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_index_with_disp() {
        // d(An,Xn) format
        let source = r#"
    org $1000
    lea.l 8(a0,d0.w),a1
"#;
        let mut asm = Assembler::new();
        let output = asm
            .assemble_source(source, std::path::Path::new("test.asm"))
            .unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_assemble_with_labels_and_branch() {
        let source = r#"
            org $1000
start:
            bra.s end
            nop
end:
            rts
"#;
        let mut asm = Assembler::new();
        let output = asm
            .assemble_source(source, std::path::Path::new("test.asm"))
            .unwrap();

        // bra.s end (displacement = 2) = 0x6002
        // nop = 0x4E71
        // rts = 0x4E75
        assert_eq!(output, vec![0x60, 0x02, 0x4E, 0x71, 0x4E, 0x75]);
    }

    #[test]
    fn test_bset_absolute_expr() {
        // Test bset.b with absolute address expression
        let source = r#"
UART equ $FF0000
MCR equ 8
    org $1000
    bset.b #1,UART+MCR
"#;
        let mut asm = Assembler::new();
        let output = asm
            .assemble_source(source, std::path::Path::new("test.asm"))
            .unwrap();

        // bset.b #1,$FF0008 - bit number immediate, absolute long destination
        // 08F9 0001 00FF 0008
        assert_eq!(output, vec![0x08, 0xF9, 0x00, 0x01, 0x00, 0xFF, 0x00, 0x08]);
    }

    #[test]
    fn test_local_labels() {
        // Test local labels like .loop
        let source = r#"
    org $1000
start:
    moveq #5,d0
.loop:
    dbra d0,.loop
    rts
"#;
        let mut asm = Assembler::new();
        let output = asm
            .assemble_source(source, std::path::Path::new("test.asm"))
            .unwrap();

        // moveq #5,d0 = 0x7005
        // dbra d0,.loop - d0 is reg 0, displacement is -4 (0xFFFC)
        // dbra = 0x51C8, with displacement 0xFFFE (-2) to loop back
        // rts = 0x4E75
        assert!(!output.is_empty());
    }

    #[test]
    fn test_assemble_with_equ() {
        let source = r#"
VALUE equ $42
            org $1000
            moveq #VALUE,d0
"#;
        let mut asm = Assembler::new();
        let output = asm
            .assemble_source(source, std::path::Path::new("test.asm"))
            .unwrap();

        // moveq #$42,d0 = 0x7042
        assert_eq!(output, vec![0x70, 0x42]);
    }

    #[test]
    fn test_assemble_with_dc() {
        let source = r#"
            org $1000
            dc.b "Hi",0
            even
            dc.w $1234
"#;
        let mut asm = Assembler::new();
        let output = asm
            .assemble_source(source, std::path::Path::new("test.asm"))
            .unwrap();

        // "Hi" = 0x48, 0x69
        // 0 = 0x00
        // even = 0x00 (pad to word boundary)
        // dc.w $1234 = 0x12, 0x34
        assert_eq!(output, vec![0x48, 0x69, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    #[ignore] // Enable when ready to test real files
    fn test_assemble_hello_asm() {
        let mut asm = Assembler::new();
        let hello_path = std::path::Path::new("rom/examples/hello.asm");
        match asm.assemble_file(hello_path) {
            Ok(output) => {
                assert!(!output.is_empty(), "assembled output should not be empty");
                eprintln!("Successfully assembled hello.asm: {} bytes", output.len());
            }
            Err(e) => {
                panic!("Failed to assemble hello.asm: {}", e);
            }
        }
    }

    #[test]
    #[ignore] // Enable when ready to test real files
    fn test_assemble_idle_asm() {
        let mut asm = Assembler::new();
        let path = std::path::Path::new("rom/examples/idle.asm");
        // Debug: dump preprocessed output on error
        let source = std::fs::read_to_string(path).unwrap();
        asm.include_paths.push(path.parent().unwrap().to_path_buf());
        let mut pp = Preprocessor::new();
        for inc_path in &asm.include_paths {
            pp.add_include_path(inc_path.clone());
        }
        let processed = pp.preprocess(&source, path).unwrap();

        match asm.assemble_source(&processed, path) {
            Ok(output) => {
                assert!(!output.is_empty(), "assembled output should not be empty");
                eprintln!("Successfully assembled idle.asm: {} bytes", output.len());
            }
            Err(e) => {
                // Dump context around line 376
                let lines: Vec<_> = processed.lines().collect();
                eprintln!("Context around line 376:");
                for i in 371usize..=380 {
                    if let Some(line) = lines.get(i.saturating_sub(1)) {
                        eprintln!("{}: {}", i, line);
                    }
                }
                panic!("Failed to assemble idle.asm: {}", e);
            }
        }
    }

    #[test]
    #[ignore] // Enable when ready to test real files
    fn test_assemble_fizzbuzz_asm() {
        let mut asm = Assembler::new();
        let path = std::path::Path::new("rom/examples/fizzbuzz.asm");
        let source = std::fs::read_to_string(path).unwrap();
        asm.include_paths.push(path.parent().unwrap().to_path_buf());
        let mut pp = Preprocessor::new();
        for inc_path in &asm.include_paths {
            pp.add_include_path(inc_path.clone());
        }
        let processed = pp.preprocess(&source, path).unwrap();

        // Write preprocessed output for debugging
        std::fs::write("target/fizzbuzz_pp.asm", &processed).ok();
        eprintln!("Wrote preprocessed output to target/fizzbuzz_pp.asm");

        match asm.assemble_source(&processed, path) {
            Ok(output) => {
                assert!(!output.is_empty(), "assembled output should not be empty");
                eprintln!(
                    "Successfully assembled fizzbuzz.asm: {} bytes",
                    output.len()
                );
            }
            Err(e) => {
                panic!("Failed to assemble fizzbuzz.asm: {}", e);
            }
        }
    }

    #[test]
    #[ignore] // Enable when ready to test real files
    fn test_assemble_rom_asm() {
        let mut asm = Assembler::new();
        let path = std::path::Path::new("rom/rom.asm");
        let source = std::fs::read_to_string(path).unwrap();
        asm.include_paths.push(path.parent().unwrap().to_path_buf());
        let mut pp = Preprocessor::new();
        for inc_path in &asm.include_paths {
            pp.add_include_path(inc_path.clone());
        }
        let processed = pp.preprocess(&source, path).unwrap();

        // Write preprocessed output for debugging
        std::fs::write("target/rom_pp.asm", &processed).ok();
        eprintln!(
            "Wrote preprocessed output to target/rom_pp.asm ({} lines)",
            processed.lines().count()
        );

        match asm.assemble_source(&processed, path) {
            Ok(output) => {
                assert!(!output.is_empty(), "assembled output should not be empty");
                eprintln!("Successfully assembled rom.asm: {} bytes", output.len());
            }
            Err(e) => {
                panic!("Failed to assemble rom.asm: {}", e);
            }
        }
    }
}
