//! M68K Instruction Implementations
//!
//! This module contains implementations of all M68K instructions.
//! Instructions are grouped logically by category:
//!
//! - **Data Movement**: MOVE, MOVEA, MOVEQ, MOVEM, LEA, PEA, etc.
//! - **Integer Arithmetic**: ADD, SUB, MUL, DIV, NEG, CMP, etc.
//! - **Logical Operations**: AND, OR, EOR, NOT, TST, etc.
//! - **Shifts and Rotates**: ASL, ASR, LSL, LSR, ROL, ROR, ROXL, ROXR
//! - **Bit Operations**: BTST, BSET, BCLR, BCHG
//! - **Binary Coded Decimal (BCD)**: ABCD, SBCD, NBCD
//! - **Program Control**: BRA, BSR, Bcc, JMP, JSR, RTS, RTR, etc.
//! - **Condition Codes**: various conditional branches
//! - **System Control**: TRAP, CHK, RESET, STOP, RTE, etc.
//!
//! Each instruction is implemented as a function that takes the CPU
//! state, instruction word, and performs the operation.
//!
//! # Flag Effects
//!
//! Every instruction documents its effect on the condition code flags:
//! - N: Negative
//! - Z: Zero
//! - V: Overflow
//! - C: Carry
//! - X: Extend (for chained arithmetic operations)
//!
//! Instructions cross-reference the M68K instruction set reference.

use crate::addressing::{AddressingMode, EaResolver, EffectiveAddress, OperandSize};
use crate::memory::Memory;
use crate::registers::{CcrFlags, FlagOps, RegisterFile};

/// Instruction execution result.
///
/// Contains the updated PC and any additional cycles consumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstructionResult {
    /// The updated program counter.
    pub pc: u32,
    /// The number of instruction cycles consumed (approximate).
    pub cycles: u8,
    /// Exception vector to trigger (0 = no exception).
    pub exception: u8,
    /// Whether the CPU should halt (STOP instruction).
    pub halt: bool,
}

impl InstructionResult {
    /// Creates a new instruction result.
    #[must_use]
    pub const fn new(pc: u32, cycles: u8) -> Self {
        Self {
            pc,
            cycles,
            exception: 0,
            halt: false,
        }
    }

    /// Creates an instruction result that triggers an exception.
    #[must_use]
    pub const fn with_exception(pc: u32, cycles: u8, vector: u8) -> Self {
        Self {
            pc,
            cycles,
            exception: vector,
            halt: false,
        }
    }

    /// Creates an instruction result that halts the CPU.
    #[must_use]
    pub const fn with_halt(pc: u32, cycles: u8) -> Self {
        Self {
            pc,
            cycles,
            exception: 0,
            halt: true,
        }
    }
}

/// M68K Instruction Set.
///
/// Each instruction is implemented as a method that operates on the CPU state.
pub struct Instructions;

impl Instructions {
    /// Sets logical operation flags (N, Z, V=0, C=0) based on size.
    /// Used by AND, OR, EOR, NOT, and other logical instructions.
    #[inline]
    fn set_logic_flags(registers: &mut RegisterFile, result: u32, size: OperandSize) {
        let sign_bit = size.sign_bit();
        registers.set_n((result & sign_bit) != 0);
        registers.set_z(result == 0);
        registers.set_v(false);
        registers.set_c(false);
        // X is unchanged for logical operations
    }

    /// Invalid instruction (illegal).
    ///
    /// Triggers an illegal instruction exception.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub const fn illegal(
        _registers: &mut RegisterFile,
        _memory: &Memory,
        _opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Illegal instruction exception (vector 4).
        InstructionResult::with_exception(pc, 34, 4)
    }

    // ==================== DATA MOVEMENT INSTRUCTIONS ====================

    /// MOVE instruction.
    ///
    /// Moves data from source to destination.
    ///
    /// # Encoding
    /// ```text
    /// 15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
    /// | size |   0 | 0 | 1 |           register             |  mode  |
    /// |              destination (mode + register)             |
    /// |              source (mode + register)                  |
    /// ```text
    ///
    /// # Flags
    /// - N: Set if result is negative
    /// - Z: Set if result is zero
    /// - V: Always cleared
    /// - C: Always cleared
    /// - X: Not affected
    ///
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn move_(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Parse the instruction
        // MOVE encoding: bits 15-12 should be 0001 (byte), 0010 (long), or 0011 (word)
        let size_field = (opcode >> 12) & 0x0F;
        let size = match size_field {
            0b0001 => OperandSize::Byte,
            0b0010 => OperandSize::Long,
            0b0011 => OperandSize::Word,
            _ => {
                // Invalid size (including 0xFFCC which has 1111)
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        let dst_mode = ((opcode >> 6) & 0x07) as u8;
        let dst_reg = ((opcode >> 9) & 0x07) as u8;
        let src_mode = ((opcode >> 3) & 0x07) as u8; // Bits 5-3
        let src_reg = (opcode & 0x07) as u8; // Bits 2-0

        // Resolve source effective address (PC is at extension words)
        let src_addr_mode = match AddressingMode::from_mode_reg(src_mode, src_reg) {
            Some(mode) => mode,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (src_ea, pc) = EaResolver::resolve(src_addr_mode, src_reg, size, registers, memory, pc);

        // Resolve destination effective address
        let dst_addr_mode = match AddressingMode::from_mode_reg(dst_mode, dst_reg) {
            Some(mode) => mode,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (dst_ea, pc) = EaResolver::resolve(dst_addr_mode, dst_reg, size, registers, memory, pc);

        // Read source operand
        let value = EaResolver::read_operand(src_ea, size, registers, memory);

        // Write to destination
        EaResolver::write_operand(dst_ea, size, value, registers, memory);

        // Set flags
        Self::set_logic_flags(registers, value, size);

        InstructionResult::new(pc, 4)
    }

    /// MOVEQ instruction - Move Quick.
    ///
    /// Moves a sign-extended 8-bit immediate to a data register.
    ///
    /// # Encoding
    /// ```text
    /// 15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
    /// | 0 | 1 | 1 | 1 |               data                |  0 | reg |
    /// ```text
    /// Pattern: 0111 0xxx xxxx xxxx
    ///
    /// # Flags
    /// - N: Set if result is negative
    /// - Z: Set if result is zero
    /// - V: Always cleared
    /// - C: Always cleared
    /// - X: Always cleared
    ///
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn moveq(
        registers: &mut RegisterFile,
        _memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Extract data and register
        let data = i32::from((opcode & 0xFF) as i8) as u32; // Sign-extended
        let reg = (opcode >> 9) & 0x07;

        // Move to data register
        registers.set_d(reg as usize, data);

        // Set flags - MOVEQ always uses long size
        Self::set_logic_flags(registers, data, OperandSize::Long);

        // MOVEQ is 2 bytes only - pc already points past the opcode
        InstructionResult::new(pc, 4)
    }

    /// LEA instruction - Load Effective Address.
    ///
    /// Loads an effective address into an address register.
    ///
    /// # Encoding
    /// 15  14  13  12  11   9   5   3   2   0
    /// | 0 | 1 | 0 | 0 | An |  mode  |  reg  |
    /// |             source effective address (mode + register)       |
    ///
    /// # Flags
    /// No flags are affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn lea(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // For LEA, the source effective address is in bits 5-0
        // Bits 5-3 = mode field
        // Bits 2-0 = register field
        let src_mode = ((opcode >> 3) & 0x07) as u8;
        let src_reg = (opcode & 0x07) as u8;
        let dst_reg = ((opcode >> 9) & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(src_mode, src_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, src_reg, OperandSize::Long, registers, memory, pc);

        match ea {
            EffectiveAddress::Memory(addr) => {
                registers.set_a(dst_reg as usize, addr);
            }
            _ => {
                panic!("LEA must resolve to a memory address");
            }
        }

        InstructionResult::new(new_pc, 4)
    }

    // ==================== INTEGER ARITHMETIC INSTRUCTIONS ====================

    /// ADD instruction - Add.
    ///
    /// Adds source to destination.
    ///
    /// # Flags
    /// - N: Set if result is negative
    /// - Z: Set if result is zero
    /// - V: Set if overflow occurred
    /// - C: Set if carry occurred
    /// - X: Set to same as C
    ///
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn add(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Determine operation direction
        // Bit 8: 0 = <ea> + Dn -> Dn, 1 = Dn + <ea> -> <ea>
        let result_in_dn = (opcode >> 8) & 0x01 == 0;
        let size = match (opcode >> 6) & 0x03 {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => {
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;
        let d_reg = ((opcode >> 9) & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        if result_in_dn {
            // <ea> + Dn -> Dn (bit 8 = 0)
            let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);
            let src = EaResolver::read_operand(ea, size, registers, memory);
            let dst = registers.d(d_reg as usize);

            let result = Self::add_with_carry(dst, src, false, size, registers);

            // Merge result with destination, preserving upper bits for byte/word ops
            let mask = size.mask();
            let merged = (dst & !mask) | (result & mask);

            registers.set_d(d_reg as usize, merged);

            InstructionResult::new(new_pc, 4)
        } else {
            // Dn + <ea> -> <ea> (bit 8 = 1)
            let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);
            let src = registers.d(d_reg as usize);
            let dst = EaResolver::read_operand(ea, size, registers, memory);

            let result = Self::add_with_carry(src, dst, false, size, registers);

            EaResolver::write_operand(ea, size, result, registers, memory);
            InstructionResult::new(new_pc, 6)
        }
    }

    /// ADDA instruction - Add Address.
    ///
    /// Adds to an address register (no sign extension, no flags affected).
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn adda(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let size = if (opcode >> 8) & 0x01 == 0 {
            OperandSize::Word
        } else {
            OperandSize::Long
        };

        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;
        let a_reg = (opcode >> 9) & 0x07;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => {
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);
        let src = EaResolver::read_operand(ea, size, registers, memory);

        // Sign-extend if word size
        let src = if size == OperandSize::Word {
            i32::from(src as i16) as u32
        } else {
            src
        };

        let dst = registers.a(a_reg as usize);
        registers.set_a(a_reg as usize, dst.wrapping_add(src));

        InstructionResult::new(new_pc, 6)
    }

    /// SUBA instruction - Subtract Address.
    ///
    /// Subtracts from an address register. The source operand is sign-extended
    /// if word-sized. No flags are affected.
    ///
    /// # Syntax
    /// SUBA.W <ea>, An
    /// SUBA.L <ea>, An
    ///
    /// # Flags
    /// No flags are affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn suba(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Bit 8 determines size: 0 = word, 1 = long
        let size = if (opcode >> 8) & 0x01 == 0 {
            OperandSize::Word
        } else {
            OperandSize::Long
        };

        // EA field: mode in bits 5-3, register in bits 2-0
        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;
        // Destination address register in bits 11-9
        let a_reg = (opcode >> 9) & 0x07;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);
        let src = EaResolver::read_operand(ea, size, registers, memory);

        // Sign-extend if word size
        let src = if size == OperandSize::Word {
            i32::from(src as i16) as u32
        } else {
            src
        };

        let dst = registers.a(a_reg as usize);
        registers.set_a(a_reg as usize, dst.wrapping_sub(src));

        InstructionResult::new(new_pc, 6)
    }

    /// EXT instruction - Sign Extend.
    ///
    /// Sign-extends a data register:
    /// - EXT.W: Extends a byte to a word (copies bit 7 to bits 8-15)
    /// - EXT.L: Extends a word to a long (copies bit 15 to bits 16-31)
    ///
    /// # Syntax
    /// EXT.W Dn
    /// EXT.L Dn
    ///
    /// # Flags
    /// N and Z are set according to the result. V and C are cleared.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn ext(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Register is in bits 2-0
        let reg = (opcode & 0x07) as usize;

        // Bits 7-6 determine the operation: 10 = byte to word, 11 = word to long
        let op = (opcode >> 6) & 0x03;

        match op {
            0b10 => {
                // EXT.W: Byte to word
                // Opcode: 0100 1000 10 000 rrr

                // Extract the byte from bits 0-7, sign-extend to 16 bits
                let byte_value = i16::from((registers.d(reg) & 0xFF) as i8) as u32;
                // Clear the lower 16 bits and set the sign-extended word
                let current = registers.d(reg) & 0xFFFF_0000;
                registers.set_d(reg, current | (byte_value & 0xFFFF));

                // Set flags according to the result
                let result = byte_value as u16;
                registers.set_n((result & 0x8000) != 0);
                registers.set_z(result == 0);
                registers.set_v(false);
                registers.set_c(false);

                InstructionResult::new(pc, 4)
            }
            0b11 => {
                // EXT.L: Word to long
                // Opcode: 0100 1000 11 000 rrr
                // Extract the word from bits 0-15, sign-extend to 32 bits
                let word_value = i32::from((registers.d(reg) & 0xFFFF) as i16) as u32;
                registers.set_d(reg, word_value);

                // Set flags according to the result
                registers.set_n((word_value & 0x8000_0000) != 0);
                registers.set_z(word_value == 0);
                registers.set_v(false);
                registers.set_c(false);

                InstructionResult::new(pc, 4)
            }
            _ => {
                // Invalid operation
                Self::illegal(registers, memory, opcode, pc)
            }
        }
    }

    /// CLR instruction - Clear.
    ///
    /// Clears the destination to zero.
    ///
    /// # Syntax
    /// CLR.B <ea>
    /// CLR.W <ea>
    /// CLR.L <ea>
    ///
    /// # Flags
    /// N and V are cleared. Z is set. C is cleared.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn clr(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let size = match (opcode >> 6) & 0x03 {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => {
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        // EA field: mode in bits 5-3, register in bits 2-0
        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);

        // Write zero to the destination
        EaResolver::write_operand(ea, size, 0, registers, memory);

        // Set flags: N and V are cleared, Z is set, C is cleared
        registers.set_n(false);
        registers.set_z(true);
        registers.set_v(false);
        registers.set_c(false);

        InstructionResult::new(new_pc, if size == OperandSize::Long { 6 } else { 4 })
    }

    /// ADDI instruction - Add Immediate.
    ///
    /// Adds immediate data to destination.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn addi(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let size = match (opcode >> 6) & 0x03 {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => {
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        // Read immediate (PC is at immediate data)
        let imm = match size {
            OperandSize::Byte => {
                let imm = u32::from(memory.read_word_unchecked(pc) as u8);
                (pc + 2, imm)
            }
            OperandSize::Word => {
                let imm = u32::from(memory.read_word_unchecked(pc));
                (pc + 2, imm)
            }
            OperandSize::Long => {
                let imm = memory.read_long_unchecked(pc);
                (pc + 4, imm)
            }
        };

        let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, imm.0);

        let dst = EaResolver::read_operand(ea, size, registers, memory);
        let result = Self::add_with_carry(dst, imm.1, false, size, registers);

        EaResolver::write_operand(ea, size, result, registers, memory);

        InstructionResult::new(new_pc, 8)
    }

    /// ADDQ instruction - Add Quick.
    ///
    /// Adds a small immediate (1-8) to destination.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn addq(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let data = u32::from((opcode >> 9) & 0x07);
        let data = if data == 0 { 8 } else { data }; // 0 means 8

        let size = match (opcode >> 6) & 0x03 {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => {
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        // For address register operations, flags are NOT affected
        let is_address_reg = mode == 1;

        if is_address_reg {
            // Address register - add directly, no flags
            // When destination is address register, size is always long (word is sign-extended)
            let dst = registers.a(reg as usize);
            let src = if size == OperandSize::Word {
                // Sign-extend word to long for address register
                i32::from(data as i16) as u32
            } else {
                data
            };
            let result = dst.wrapping_add(src);
            registers.set_a(reg as usize, result);
            InstructionResult::new(pc, 4)
        } else {
            let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);
            let dst = EaResolver::read_operand(ea, size, registers, memory);
            let result = Self::add_with_carry(dst, data, false, size, registers);
            EaResolver::write_operand(ea, size, result, registers, memory);
            InstructionResult::new(new_pc, 4)
        }
    }

    /// SUB instruction - Subtract.
    ///
    /// Subtracts source from destination.
    ///
    /// # Flags
    /// - N: Set if result is negative
    /// - Z: Set if result is zero
    /// - V: Set if overflow occurred
    /// - C: Set if borrow occurred
    /// - X: Set to same as C
    ///
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn sub(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Bit 8: 0 = <ea> - Dn -> Dn, 1 = Dn - <ea> -> <ea>
        let reverse = (opcode >> 8) & 0x01 == 1;
        let size = match (opcode >> 6) & 0x03 {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => {
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        // M68K EA encoding: bits 5-3 are mode, bits 2-0 are register
        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;
        let d_reg = ((opcode >> 9) & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        if reverse {
            // SUB Dn,<ea>: <ea> - Dn -> <ea>
            let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);
            let src = registers.d(d_reg as usize);
            let dst = EaResolver::read_operand(ea, size, registers, memory);

            let result = Self::sub_with_borrow(dst, src, false, size, registers, true);

            EaResolver::write_operand(ea, size, result, registers, memory);
            InstructionResult::new(new_pc, 6)
        } else {
            // <ea> - Dn -> Dn (actually: subtract <ea> FROM Dn, so Dn - <ea> -> Dn)
            let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);
            let src = EaResolver::read_operand(ea, size, registers, memory);
            let dst = registers.d(d_reg as usize);

            let result = Self::sub_with_borrow(dst, src, false, size, registers, true);

            // Merge result with destination, preserving upper bits for byte/word ops
            let mask = size.mask();
            let merged = (dst & !mask) | (result & mask);
            registers.set_d(d_reg as usize, merged);
            InstructionResult::new(new_pc, 4)
        }
    }

    /// CMP instruction - Compare.
    ///
    /// Compares destination with source (subtracts without storing).
    ///
    /// # Flags
    /// - N: Set if result is negative
    /// - Z: Set if result is zero
    /// - V: Set if overflow occurred
    /// - C: Set if borrow occurred
    /// - X: Not affected
    ///
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn cmp(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let size = match (opcode >> 6) & 0x03 {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => {
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;
        let d_reg = ((opcode >> 9) & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);
        let src = EaResolver::read_operand(ea, size, registers, memory);
        let dst = registers.d(d_reg as usize);

        // CMP does not affect X flag
        let _ = Self::sub_with_borrow(dst, src, false, size, registers, false);

        InstructionResult::new(new_pc, 4)
    }

    /// NEG instruction - Negate.
    ///
    /// Negates the destination operand (0 - operand -> operand).
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn neg(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let size = match (opcode >> 6) & 0x03 {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => {
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);

        let operand = EaResolver::read_operand(ea, size, registers, memory);
        let result = Self::sub_with_borrow(0, operand, false, size, registers, true);

        // Write result - write_operand handles masking and upper bit preservation
        EaResolver::write_operand(ea, size, result, registers, memory);

        InstructionResult::new(new_pc, 4)
    }

    // ==================== LOGICAL INSTRUCTIONS ====================

    /// AND instruction - Logical AND.
    ///
    /// Performs bitwise AND of source and destination.
    ///
    /// # Flags
    /// - N: Set if result is negative
    /// - Z: Set if result is zero
    /// - V: Always cleared
    /// - C: Always cleared
    /// - X: Not affected
    ///
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn and(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let direction = (opcode >> 8) & 0x01 == 0;
        let size = match (opcode >> 6) & 0x03 {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => {
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;
        let d_reg = ((opcode >> 9) & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let mask = size.mask();

        if direction {
            // <ea> & Dn -> Dn
            let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);
            let src = EaResolver::read_operand(ea, size, registers, memory);
            let dst = registers.d(d_reg as usize);

            let result = (src & dst) & mask;
            // Preserve upper bits for byte/word operations
            let new_value = (dst & !mask) | result;
            registers.set_d(d_reg as usize, new_value);
            Self::set_logic_flags(registers, result, size);

            InstructionResult::new(new_pc, 4)
        } else {
            // Dn & <ea> -> <ea>
            let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);
            let src = registers.d(d_reg as usize);
            let dst = EaResolver::read_operand(ea, size, registers, memory);

            let result = (src & dst) & mask;
            EaResolver::write_operand(ea, size, result, registers, memory);
            Self::set_logic_flags(registers, result, size);

            InstructionResult::new(new_pc, 6)
        }
    }

    /// OR instruction - Logical OR.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn or(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let direction = (opcode >> 8) & 0x01 == 0;
        let size = match (opcode >> 6) & 0x03 {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => {
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;
        let d_reg = ((opcode >> 9) & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let mask = size.mask();

        if direction {
            // <ea> | Dn -> Dn
            let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);
            let src = EaResolver::read_operand(ea, size, registers, memory);
            let dst = registers.d(d_reg as usize);

            let result = (src | dst) & mask;
            // Preserve upper bits for byte/word operations
            let new_value = (dst & !mask) | result;
            registers.set_d(d_reg as usize, new_value);
            Self::set_logic_flags(registers, result, size);

            InstructionResult::new(new_pc, 4)
        } else {
            // Dn | <ea> -> <ea>
            let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);
            let src = registers.d(d_reg as usize);
            let dst = EaResolver::read_operand(ea, size, registers, memory);

            let result = (src | dst) & mask;
            EaResolver::write_operand(ea, size, result, registers, memory);
            Self::set_logic_flags(registers, result, size);

            InstructionResult::new(new_pc, 6)
        }
    }

    /// EOR instruction - Logical Exclusive OR.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn eor(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let size = match (opcode >> 6) & 0x03 {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => {
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;
        let d_reg = ((opcode >> 9) & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);
        let src = registers.d(d_reg as usize);
        let dst = EaResolver::read_operand(ea, size, registers, memory);

        let result = (src ^ dst) & size.mask();
        EaResolver::write_operand(ea, size, result, registers, memory);
        Self::set_logic_flags(registers, result, size);

        InstructionResult::new(new_pc, 6)
    }

    /// NOT instruction - Logical NOT (complement).
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn not(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let size = match (opcode >> 6) & 0x03 {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => {
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);

        let operand = EaResolver::read_operand(ea, size, registers, memory);
        let result = !operand & size.mask();

        EaResolver::write_operand(ea, size, result, registers, memory);
        Self::set_logic_flags(registers, result, size);

        InstructionResult::new(new_pc, 4)
    }

    /// TST instruction - Test.
    ///
    /// Tests an operand and sets flags accordingly (performs a compare with zero).
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn tst(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let size = match (opcode >> 6) & 0x03 {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => {
                return Self::illegal(registers, memory, opcode, pc);
            }
        };

        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) = EaResolver::resolve(addr_mode, reg, size, registers, memory, pc);

        let operand = EaResolver::read_operand(ea, size, registers, memory);
        // Set flags based on the operand with correct size
        Self::set_logic_flags(registers, operand, size);

        InstructionResult::new(new_pc, 4)
    }

    // ==================== SHIFT AND ROTATE INSTRUCTIONS ====================

    /// ASL/ASR instructions - Arithmetic Shift Left/Right.
    ///
    /// Arithmetic shift preserves the sign bit for right shifts.
    /// Handles both register shifts and memory shifts.
    ///
    /// Register format: 1110 ccc d ss i 00 rrr
    ///   - ccc = count or count register, d = direction, ss = size, i = immediate/register, rrr = data reg
    ///
    /// Memory format: 1110 000 d 11 MMMRRR
    ///   - d = direction, MMMRRR = EA (always word size, always count of 1)
    ///
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn asx(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Determine direction: bit 8 = 0 for right, 1 for left
        let is_left = (opcode >> 8) & 0x01 == 1;

        // Check for memory shift: bits 7-6 = 11
        let size_bits = (opcode >> 6) & 0x03;
        if size_bits == 0b11 {
            // Memory shift - always word size, always count of 1
            let mode = ((opcode >> 3) & 0x07) as u8;
            let reg = (opcode & 0x07) as u8;

            let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };

            let (ea, new_pc) =
                EaResolver::resolve(addr_mode, reg, OperandSize::Word, registers, memory, pc);
            let value = EaResolver::read_operand(ea, OperandSize::Word, registers, memory);

            return Self::shift_arithmetic_memory(
                registers,
                memory,
                ea,
                value,
                1,
                OperandSize::Word,
                is_left,
                new_pc,
            );
        }

        // Register shift
        let size = match size_bits {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => unreachable!(), // Already handled 0b11 above
        };

        // Determine shift type: bit 5 = 0 for immediate count, 1 for register count
        let is_register = (opcode >> 5) & 0x01 == 1;

        if is_register {
            // Shift count in Dn
            // With register count: bits 11-9 = count register, bits 2-0 = destination register
            let count_reg = ((opcode >> 9) & 0x07) as u8;
            let reg = (opcode & 0x07) as u8;
            let count = registers.d(count_reg as usize) & 0x3F; // Only 6 bits used

            Self::shift_arithmetic(registers, memory, reg, count, size, is_left, true, pc)
        } else {
            // Immediate shift count
            // With immediate count: bits 11-9 = count (0=8), bits 2-0 = destination register
            let reg = (opcode & 0x07) as u8;
            let mut count = u32::from((opcode >> 9) & 0x07);
            if count == 0 {
                count = 8;
            }

            Self::shift_arithmetic(registers, memory, reg, count, size, is_left, false, pc)
        }
    }

    // ==================== MULTIPLY AND DIVIDE INSTRUCTIONS ====================

    /// MULU/MULS instructions - Multiply Unsigned/Signed.
    ///
    /// Multiplies two 16-bit values to produce a 32-bit result.
    ///
    /// # Flags
    /// - N: Set if result is negative
    /// - Z: Set if result is zero
    /// - V: Always cleared
    /// - C: Always cleared
    /// - X: Not affected
    ///
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn mul(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // MULU/MULS format: 1100 rrr s 11 MMMRRR
        // rrr = destination data register (bits 11-9)
        // s = 0 for MULU (unsigned), 1 for MULS (signed) (bit 8)
        // MMMRRR = source effective address (bits 5-0)
        let is_signed = (opcode & 0x0100) != 0; // Bit 8
        let d_reg = ((opcode >> 9) & 0x07) as u8; // Bits 11-9
        let mode = ((opcode >> 3) & 0x07) as u8;
        let src_reg = (opcode & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, src_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, src_reg, OperandSize::Word, registers, memory, pc);
        let src = EaResolver::read_operand(ea, OperandSize::Word, registers, memory);
        let dst = registers.d(d_reg as usize) & 0xFFFF;

        let result = if is_signed {
            // Signed multiply: sign-extend both operands to i32, multiply
            let src_signed = OperandSize::Word.sign_extend(src);
            let dst_signed = OperandSize::Word.sign_extend(dst);
            src_signed.wrapping_mul(dst_signed) as u32
        } else {
            // Unsigned multiply - only use lower 16 bits
            src.wrapping_mul(dst)
        };

        registers.set_d(d_reg as usize, result);
        Self::set_logic_flags(registers, result, OperandSize::Long);

        InstructionResult::new(new_pc, 54) // MULU/MULS take many cycles
    }

    /// DIVU/DIVS instructions - Divide Unsigned/Signed.
    ///
    /// Divides a 32-bit dividend by a 16-bit divisor.
    /// Produces a 16-bit quotient and 16-bit remainder.
    ///
    /// # Flags
    /// - N: Set if quotient is negative
    /// - Z: Set if quotient is zero
    /// - V: Set if division overflow (quotient > 16 bits)
    /// - C: Always cleared
    /// - X: Not affected
    ///
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn div(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // DIVU/DIVS format: 1000 rrr s 11 MMMRRR
        // rrr = destination data register (bits 11-9)
        // s = 0 for DIVU (unsigned), 1 for DIVS (signed) (bit 8)
        // MMMRRR = source effective address (bits 5-0)
        let is_signed = (opcode & 0x0100) != 0; // Bit 8
        let d_reg = ((opcode >> 9) & 0x07) as u8; // Bits 11-9
        let mode = ((opcode >> 3) & 0x07) as u8;
        let src_reg = (opcode & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, src_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, src_reg, OperandSize::Word, registers, memory, pc);
        let divisor = EaResolver::read_operand(ea, OperandSize::Word, registers, memory);
        let dividend = registers.d(d_reg as usize);

        // Check for division by zero - triggers trap
        if divisor == 0 {
            // Division by zero triggers exception vector 5
            return InstructionResult::with_exception(pc, 0, 5);
        }

        // C is always cleared, even on overflow
        registers.set_c(false);

        if is_signed {
            // Signed division - full 32-bit dividend divided by 16-bit divisor
            let dividend_signed = dividend as i32;
            let divisor_signed = i32::from(divisor as i16);

            // Check for overflow: -2^31 / -1 can't be represented
            if dividend_signed == i32::MIN && divisor_signed == -1 {
                registers.set_v(true);
                // N and Z are undefined on overflow
                return InstructionResult::new(new_pc, 68);
            }

            let quotient = dividend_signed / divisor_signed;
            let remainder = dividend_signed % divisor_signed;

            // Check if quotient fits in 16 bits
            if quotient < i32::from(i16::MIN) || quotient > i32::from(i16::MAX) {
                registers.set_v(true);
                // N and Z are undefined on overflow
                return InstructionResult::new(new_pc, 68);
            }

            // Store quotient in lower word, remainder in upper word
            let result = ((remainder as u32) << 16) | u32::from(quotient as u16);
            registers.set_d(d_reg as usize, result);
            // Flags are based on 16-bit quotient
            Self::set_logic_flags(registers, u32::from(quotient as u16), OperandSize::Word);
        } else {
            // Unsigned division - full 32-bit dividend divided by 16-bit divisor
            let quotient = dividend / divisor;
            let remainder = dividend % divisor;

            // Check if quotient fits in 16 bits
            if quotient > 0xFFFF {
                registers.set_v(true);
                // N and Z are undefined on overflow
                return InstructionResult::new(new_pc, 68);
            }

            // Store quotient in lower word, remainder in upper word
            let result = (remainder << 16) | quotient;
            registers.set_d(d_reg as usize, result);
            // Flags are based on 16-bit quotient
            Self::set_logic_flags(registers, quotient, OperandSize::Word);
        }

        InstructionResult::new(new_pc, 68) // DIVU/DIVS take many cycles
    }

    // ==================== PROGRAM CONTROL INSTRUCTIONS ====================

    /// BRA instruction - Branch Always.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn bra(
        _registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let displacement = Self::parse_branch_displacement(opcode, pc, memory);
        // Target = PC + displacement (pc is already past the instruction word)
        let new_pc = pc.wrapping_add(displacement as u32);
        InstructionResult::new(new_pc, 10)
    }

    /// BSR instruction - Branch to Subroutine.
    ///
    /// Pushes the return address (PC of the next instruction) onto the stack
    /// and branches to the target address.
    ///
    /// # Flags
    /// No flags are affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn bsr(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let displacement = Self::parse_branch_displacement(opcode, pc, memory);
        // Target = PC + displacement (pc is already past the instruction word)
        let new_pc = pc.wrapping_add(displacement as u32);

        // Push return address onto stack
        // The return address is the address of the instruction following BSR
        // For 8-bit displacement: return_addr = pc (already past opcode)
        // For 16-bit displacement: return_addr = pc + 2 (past extension word)
        let has_ext_word = (opcode & 0xFF) == 0;
        let return_addr = if has_ext_word { pc + 2 } else { pc };

        let sp = registers.sp();
        let new_sp = sp.wrapping_sub(4);
        registers.set_sp(new_sp);
        // Write return address to stack
        let _ = memory.write_long(new_sp, return_addr);

        InstructionResult::new(new_pc, 10)
    }

    /// Bcc instructions - Branch Conditionally.
    ///
    /// Condition codes:
    /// - 0000: Bcc - Branch if Carry Clear
    /// - 0001: BCS - Branch if Carry Set
    /// - 0010: BEQ - Branch if Equal (Zero)
    /// - 0011: BNE - Branch if Not Equal
    /// - 0100: BPL - Branch if Plus (Negative clear)
    /// - 0101: BMI - Branch if Minus (Negative set)
    /// - 0110: BVS - Branch if Overflow Set
    /// - 0111: BVC - Branch if Overflow Clear
    /// - 1000: BHI - Branch if Higher (C clear and Z clear)
    /// - 1001: BLS - Branch if Lower or Same (C set or Z set)
    /// - 1010: BGE - Branch if Greater or Equal (N=V)
    /// - 1011: BLT - Branch if Less Than (N≠V)
    /// - 1100: BGT - Branch if Greater (Z clear and N=V)
    /// - 1101: BLE - Branch if Less or Equal (Z set or N≠V)
    ///
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn bcc(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        let condition = (opcode >> 8) & 0x0F;
        let should_branch = Self::evaluate_condition(registers, condition);

        // Check if instruction has 8-bit or 16-bit displacement
        let has_ext_word = (opcode & 0xFF) == 0;
        let next_pc = if has_ext_word { pc + 2 } else { pc };

        if should_branch {
            let displacement = Self::parse_branch_displacement(opcode, pc, memory);
            // Target = PC + displacement (pc is already past the instruction word)
            let new_pc = pc.wrapping_add(displacement as u32);
            InstructionResult::new(new_pc, 10)
        } else {
            // Branch not taken - advance PC past any extension word
            InstructionResult::new(next_pc, 4)
        }
    }

    /// JMP instruction - Jump.
    ///
    /// Unconditional jump to an effective address.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn jmp(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // JMP encoding: 0100 1110 11xx xxxx
        // The addressing mode is in bits 5-0 (EA field)
        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, _new_pc) =
            EaResolver::resolve(addr_mode, reg, OperandSize::Long, registers, memory, pc);

        match ea {
            EffectiveAddress::Memory(addr) => InstructionResult::new(addr, 4),
            EffectiveAddress::AddressRegister(r) => {
                InstructionResult::new(registers.a(r as usize), 4)
            }
            _ => panic!("JMP must resolve to a memory address or address register"),
        }
    }

    /// JSR instruction - Jump to Subroutine.
    ///
    /// Pushes the return address (PC of the next instruction) onto the stack
    /// and jumps to the target address.
    ///
    /// # Flags
    /// No flags are affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn jsr(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // JSR encoding: 0100 1110 10xx xxxx
        // The addressing mode is in bits 5-0 (EA field)
        let mode = ((opcode >> 3) & 0x07) as u8;
        let reg = (opcode & 0x07) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(mode, reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, _new_pc) =
            EaResolver::resolve(addr_mode, reg, OperandSize::Long, registers, memory, pc);

        let target_addr = match ea {
            EffectiveAddress::Memory(addr) => addr,
            EffectiveAddress::AddressRegister(r) => registers.a(r as usize),
            _ => panic!("JSR must resolve to a memory address or address register"),
        };

        // Push return address onto stack
        // The return address is the address of the instruction following JSR
        // We need to calculate the size based on the addressing mode
        let ext_size = match addr_mode {
            AddressingMode::AddressRegisterDirect => 2,
            AddressingMode::AddressRegisterIndirect => 2,
            AddressingMode::AddressRegisterIndirectWithDisplacement => 4,
            AddressingMode::AddressRegisterIndirectWithIndex => 4,
            AddressingMode::AbsoluteShort => 4,
            AddressingMode::AbsoluteLong => 6,
            AddressingMode::ProgramCounterRelativeWithDisplacement => 4,
            AddressingMode::ProgramCounterRelativeWithIndex => 4,
            _ => 2,
        };
        // The PC passed to JSR is current_pc + 2 (pointing past the opcode)
        // The return address should be current_pc + instruction_size
        // For modes without extension words (size=2), return_addr = pc
        // For modes with extension words (size>2), return_addr = pc + (ext_size - 2)
        let return_addr = if ext_size == 2 {
            pc
        } else {
            pc.wrapping_add(ext_size as u32).wrapping_sub(2)
        };

        let sp = registers.sp();
        let new_sp = sp.wrapping_sub(4);
        registers.set_sp(new_sp);
        // Write return address to stack
        let _ = memory.write_long(new_sp, return_addr);

        InstructionResult::new(target_addr, 8)
    }

    /// RTS instruction - Return from Subroutine.
    ///
    /// Pops the return address from the stack and continues execution there.
    ///
    /// # Flags
    /// No flags are affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn rts(
        registers: &mut RegisterFile,
        memory: &Memory,
        _opcode: u16,
        _pc: u32,
    ) -> InstructionResult {
        // Pop return address from stack
        let sp = registers.sp();
        let return_addr = memory.read_long(sp).unwrap_or(0);
        registers.set_sp(sp.wrapping_add(4));

        InstructionResult::new(return_addr, 4)
    }

    // ==================== HELPER FUNCTIONS ====================

    /// Test condition code based on condition bits.
    ///
    /// Returns true if the condition is met based on current flags.
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    fn test_condition(condition: u8, registers: &RegisterFile) -> bool {
        match condition {
            0x0 => true,                                     // T (true)
            0x1 => false,                                    // F (false)
            0x2 => !registers.get_c() && !registers.get_z(), // HI (high)
            0x3 => registers.get_c() || registers.get_z(),   // LS (low or same)
            0x4 => !registers.get_c(),                       // CC/HS (carry clear/high or same)
            0x5 => registers.get_c(),                        // CS/LO (carry set/low)
            0x6 => !registers.get_z(),                       // NE (not equal)
            0x7 => registers.get_z(),                        // EQ (equal)
            0x8 => !registers.get_v(),                       // VC (overflow clear)
            0x9 => registers.get_v(),                        // VS (overflow set)
            0xA => !registers.get_n(),                       // PL (plus)
            0xB => registers.get_n(),                        // MI (minus)
            0xC => {
                (registers.get_n() && registers.get_v())
                    || (!registers.get_n() && !registers.get_v())
            } // GE (greater or equal)
            0xD => {
                (registers.get_n() && !registers.get_v())
                    || (!registers.get_n() && registers.get_v())
            } // LT (less than)
            0xE => {
                (registers.get_n() && registers.get_v() && !registers.get_z())
                    || (!registers.get_n() && !registers.get_v() && !registers.get_z())
            } // GT (greater than)
            0xF => {
                registers.get_z()
                    || (registers.get_n() && !registers.get_v())
                    || (!registers.get_n() && registers.get_v())
            } // LE (less or equal)
            _ => false,
        }
    }

    /// Adds two values with carry, setting flags appropriately.
    /// Returns the masked result (NOT sign-extended) - caller must handle merge with destination.
    fn add_with_carry(
        dst: u32,
        src: u32,
        carry_in: bool,
        size: OperandSize,
        registers: &mut RegisterFile,
    ) -> u32 {
        let mask = size.mask();
        let dst_masked = dst & mask;
        let src_masked = src & mask;

        let carry = u32::from(carry_in);
        let result = dst_masked.wrapping_add(src_masked).wrapping_add(carry);
        let result_masked = result & mask;

        // Calculate overflow and carry flags
        let overflow = Self::check_add_overflow(dst_masked, src_masked, carry, size);
        let carry_out = Self::check_add_carry(dst_masked, src_masked, carry, size);

        // Set flags based on the size-appropriate result
        let sign_bit = size.sign_bit();
        registers.set_n((result_masked & sign_bit) != 0);
        registers.set_z(result_masked == 0);
        registers.set_v(overflow);
        registers.set_c(carry_out);
        registers.set_x(carry_out);

        // Return masked result - caller handles merging with destination for byte/word ops
        result_masked
    }

    /// Subtracts with borrow, setting flags appropriately.
    /// Returns the masked result (NOT sign-extended) - caller must handle merge with destination.
    ///
    /// # Parameters
    /// - `update_x`: If true, X flag is set to match C (used by SUB, SUBX, NEG, NEGX).
    ///   If false, X flag is unchanged (used by CMP, CMPI, CMPM, CHK).
    fn sub_with_borrow(
        dst: u32,
        src: u32,
        borrow_in: bool,
        size: OperandSize,
        registers: &mut RegisterFile,
        update_x: bool,
    ) -> u32 {
        let mask = size.mask();
        let dst_masked = dst & mask;
        let src_masked = src & mask;

        let borrow = u32::from(borrow_in);
        let result = dst_masked.wrapping_sub(src_masked).wrapping_sub(borrow);
        let result_masked = result & mask;

        // Calculate overflow and carry flags
        let overflow = Self::check_sub_overflow(dst_masked, src_masked, borrow, size);
        let carry_out = Self::check_sub_carry(dst_masked, src_masked, borrow, size);

        // Set flags based on the size-appropriate result
        let sign_bit = size.sign_bit();
        registers.set_n((result_masked & sign_bit) != 0);
        registers.set_z(result_masked == 0);
        registers.set_v(overflow);
        registers.set_c(carry_out);
        if update_x {
            registers.set_x(carry_out);
        }

        // Return masked result - caller handles merging with destination for byte/word ops
        result_masked
    }

    /// Checks for signed overflow in addition.
    fn check_add_overflow(a: u32, b: u32, carry: u32, size: OperandSize) -> bool {
        let _mask = size.mask();
        let sign_bit = if size == OperandSize::Byte {
            0x80
        } else if size == OperandSize::Word {
            0x8000
        } else {
            0x8000_0000_u32
        };

        let a_neg = (a & sign_bit) != 0;
        let b_neg = (b & sign_bit) != 0;
        let r_neg = ((a.wrapping_add(b).wrapping_add(carry)) & sign_bit) != 0;

        // Overflow if operands have same sign but result has different sign
        (a_neg == b_neg) && (a_neg != r_neg)
    }

    /// Checks for unsigned carry in addition.
    fn check_add_carry(a: u32, b: u32, carry: u32, size: OperandSize) -> bool {
        if size == OperandSize::Long {
            // For long size, use u64 to properly detect overflow
            u64::from(a) + u64::from(b) + u64::from(carry) > u64::from(u32::MAX)
        } else {
            let mask = size.mask();
            a.wrapping_add(b).wrapping_add(carry) & !mask != 0
        }
    }

    /// Checks for signed overflow in subtraction.
    fn check_sub_overflow(a: u32, b: u32, borrow: u32, size: OperandSize) -> bool {
        let _mask = size.mask();
        let sign_bit = if size == OperandSize::Byte {
            0x80
        } else if size == OperandSize::Word {
            0x8000
        } else {
            0x8000_0000_u32
        };

        let a_neg = (a & sign_bit) != 0;
        let b_neg = (b & sign_bit) != 0;
        let r_neg = ((a.wrapping_sub(b).wrapping_sub(borrow)) & sign_bit) != 0;

        // Overflow if operands have different signs and result has opposite sign of minuend
        (a_neg != b_neg) && (a_neg != r_neg)
    }

    /// Checks for unsigned borrow in subtraction.
    fn check_sub_carry(a: u32, b: u32, borrow: u32, size: OperandSize) -> bool {
        if size == OperandSize::Long {
            // For long size, use u64 to properly detect borrow
            u64::from(b) + u64::from(borrow) > u64::from(a)
        } else {
            let mask = size.mask();
            a.wrapping_sub(b).wrapping_sub(borrow) & !mask != 0
        }
    }

    /// Performs an arithmetic shift operation.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    fn shift_arithmetic(
        registers: &mut RegisterFile,
        _memory: &mut Memory,
        reg: u8,
        count: u32,
        size: OperandSize,
        is_left: bool,
        _is_register: bool,
        pc: u32, // Added: actual next PC value
    ) -> InstructionResult {
        // Read value directly from data register
        let value = registers.d(reg as usize) & size.mask();
        let sign_bit = size.bits() as u32 - 1;
        let sign_mask = 1 << sign_bit;

        if count == 0 {
            // No shift, just set flags (value unchanged)
            // C is cleared on count==0, V is cleared, X is unchanged
            let result = value;
            registers.set_n((result & sign_mask) != 0);
            registers.set_z(result == 0);
            registers.set_v(false);
            registers.set_c(false);
            // X is not affected when shift count is 0
            return InstructionResult::new(pc, 6);
        }

        if is_left {
            // Arithmetic shift left (same as logical shift left)
            // Track if sign bit ever changes during the shift
            let original_sign = (value >> sign_bit) & 1;

            // Calculate the final result
            let (result, last_shifted_out, overflow) = if count >= size.bits() as u32 {
                // All bits shift out - result is 0
                // V is set if the MSB changes at ANY point during shifting.
                // This happens if:
                // 1. Any bit position that will become MSB differs from original MSB
                //    (i.e., any bit in position 0 to size-2 differs from bit size-1)
                // 2. OR the final result (0) has different sign than original
                //
                // For shift count >= size, the final result is always 0.
                // If original sign was 1, then final sign is 0 → V=1
                // If original sign was 0, then we need to check if any 1 bit got shifted into MSB
                let v = if original_sign == 1 {
                    // Original was negative, final is 0 (positive) → sign changed
                    true
                } else {
                    // Original was positive, check if any bit is 1 (would become MSB during shift)
                    let mut any_one = false;
                    for i in 0..size.bits() as u32 {
                        if (value >> i) & 1 == 1 {
                            any_one = true;
                            break;
                        }
                    }
                    any_one
                };

                // The last bit shifted out for count >= size
                // For ASL (left shift), bits shift out from the MSB position
                // After size shifts, the last bit shifted out is the original LSB
                // (it's been shifted all the way from position 0 to position size-1)
                // For count > size, we're shifting zeros, so last bit is 0
                let last_bit = if count == size.bits() as u32 {
                    value & 1 // Original LSB
                } else {
                    0 // Only zeros after size shifts
                };
                (0u32, last_bit != 0, v)
            } else {
                // Normal shift within range
                let result = (value << count) & size.mask();
                let _new_sign = (result >> sign_bit) & 1;

                // Check if any bit that will become the sign bit differs from original sign
                // This happens if we shift a bit that differs from the sign into the sign position
                // We need to check all intermediate steps
                let mut v = false;
                for i in 1..=count {
                    let intermediate = (value << i) & size.mask();
                    let intermediate_sign = (intermediate >> sign_bit) & 1;
                    if intermediate_sign != original_sign {
                        v = true;
                        break;
                    }
                }

                // The last bit shifted out is at position (sign_bit - count + 1)
                // Actually, it's the bit that was at position sign_bit after count-1 shifts
                // which is the original bit at position (sign_bit - count + 1)
                let last_bit = if sign_bit >= count - 1 {
                    (value >> (sign_bit - count + 1)) & 1
                } else {
                    // count > sign_bit+1, so we've shifted past all original bits
                    0
                };

                (result, last_bit != 0, v)
            };

            let masked_result = result & size.mask();

            // Write result to register, preserving upper bits for byte/word
            let mask = size.mask();
            let current = registers.d(reg as usize);
            let merged = (current & !mask) | (masked_result & mask);
            registers.set_d(reg as usize, merged);

            // Set flags
            registers.set_n((masked_result & sign_mask) != 0);
            registers.set_z(masked_result == 0);
            registers.set_v(overflow);
            registers.set_c(last_shifted_out);
            registers.set_x(last_shifted_out);

            InstructionResult::new(pc, 6)
        } else {
            // Arithmetic shift right (sign-extended)
            let value_signed = size.sign_extend(value);

            let (result, last_shifted_out) = if count >= size.bits() as u32 {
                // All bits shift out - result is all sign bits
                let final_result = if value_signed < 0 {
                    size.mask() // All 1s (sign-extended)
                } else {
                    0
                };
                // The last bit shifted out: for ASR, when count >= size,
                // after we've shifted out all original bits, we're shifting
                // copies of the sign bit. The last bit shifted is:
                // - For count == size: the MSB (sign bit) of original value
                // - For count > size: still the sign bit (which is now replicated)
                let sign_bit = (value >> (size.bits() as u32 - 1)) & 1;
                (final_result, sign_bit != 0)
            } else {
                let result = (value_signed >> count) as u32;
                // The last bit shifted out is at position (count - 1)
                let last_bit = (value >> (count - 1)) & 1;
                (result & size.mask(), last_bit != 0)
            };

            let masked_result = result & size.mask();

            // Write result to register, preserving upper bits for byte/word
            let mask = size.mask();
            let current = registers.d(reg as usize);
            let merged = (current & !mask) | (masked_result & mask);
            registers.set_d(reg as usize, merged);

            // Set flags - ASR never sets V
            registers.set_n((masked_result & sign_mask) != 0);
            registers.set_z(masked_result == 0);
            registers.set_v(false);
            registers.set_c(last_shifted_out);
            registers.set_x(last_shifted_out);

            InstructionResult::new(pc, 6)
        }
    }

    /// Arithmetic shift for memory operands.
    /// Memory shifts are always word size and always shift by 1.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    fn shift_arithmetic_memory(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        ea: EffectiveAddress,
        value: u32,
        count: u32,
        size: OperandSize,
        is_left: bool,
        pc: u32,
    ) -> InstructionResult {
        let sign_bit = size.bits() as u32 - 1;
        let sign_mask = 1 << sign_bit;
        let original_sign = (value >> sign_bit) & 1;

        let (result, last_shifted_out, overflow) = if is_left {
            // ASL - shift left
            let result = (value << count) & size.mask();
            let new_sign = (result >> sign_bit) & 1;
            let last_bit = (value >> sign_bit) & 1; // For count=1, MSB shifts out
            let overflow = original_sign != new_sign;
            (result, last_bit != 0, overflow)
        } else {
            // ASR - arithmetic shift right (sign-extended)
            let value_signed = size.sign_extend(value);
            let result = ((value_signed >> count) as u32) & size.mask();
            let last_bit = value & 1; // For count=1, LSB shifts out
            (result, last_bit != 0, false) // ASR never sets V
        };

        // Write result back to memory
        EaResolver::write_operand(ea, size, result, registers, memory);

        // Set flags
        registers.set_n((result & sign_mask) != 0);
        registers.set_z(result == 0);
        registers.set_v(overflow);
        registers.set_c(last_shifted_out);
        registers.set_x(last_shifted_out);

        InstructionResult::new(pc, 8) // Memory shifts take longer
    }

    /// Parses a branch displacement from the opcode and extension words.
    ///
    /// # Arguments
    /// * `opcode` - The branch instruction opcode
    /// * `pc` - The PC pointing past the opcode word (`current_pc` + 2)
    /// * `memory` - Memory reference for reading extension words
    ///
    /// # Returns
    /// The signed displacement value
    fn parse_branch_displacement(opcode: u16, pc: u32, memory: &Memory) -> i32 {
        let offset = (opcode & 0xFF) as i8;

        if offset != 0 {
            // 8-bit displacement embedded in opcode
            i32::from(offset)
        } else {
            // 16-bit displacement - read extension word from pc (which points right after opcode)
            let ext = memory.read_word_unchecked(pc) as i16;
            i32::from(ext)
        }
    }

    /// Evaluates a branch condition.
    ///
    /// M68K Bcc condition encoding (bits 11-8 of opcode):
    /// 0010: BHI  - Higher (C=0 and Z=0)
    /// 0011: BLS  - Lower or Same (C=1 or Z=1)
    /// 0100: BCC/BHS - Carry Clear (C=0)
    /// 0101: BCS/BLO - Carry Set (C=1)
    /// 0110: BNE  - Not Equal (Z=0)
    /// 0111: BEQ  - Equal (Z=1)
    /// 1000: BVC  - Overflow Clear (V=0)
    /// 1001: BVS  - Overflow Set (V=1)
    /// 1010: BPL  - Plus (N=0)
    /// 1011: BMI  - Minus (N=1)
    /// 1100: BGE  - Greater or Equal (N=V)
    /// 1101: BLT  - Less Than (N≠V)
    /// 1110: BGT  - Greater Than (Z=0 and N=V)
    /// 1111: BLE  - Less or Equal (Z=1 or N≠V)
    fn evaluate_condition(registers: &RegisterFile, condition: u16) -> bool {
        let n = registers.get_n();
        let z = registers.get_z();
        let c = registers.get_c();
        let v = registers.get_v();

        match condition {
            0x2 => !c && !z,       // BHI - Higher
            0x3 => c || z,         // BLS - Lower or Same
            0x4 => !c,             // BCC - Carry Clear
            0x5 => c,              // BCS - Carry Set
            0x6 => !z,             // BNE - Not Equal
            0x7 => z,              // BEQ - Equal
            0x8 => !v,             // BVC - Overflow Clear
            0x9 => v,              // BVS - Overflow Set
            0xA => !n,             // BPL - Plus
            0xB => n,              // BMI - Minus
            0xC => n == v,         // BGE - Greater or Equal
            0xD => n != v,         // BLT - Less Than
            0xE => !z && (n == v), // BGT - Greater Than
            0xF => z || (n != v),  // BLE - Less or Equal
            _ => true,             // 0, 1 would be BRA/BSR but those have their own handlers
        }
    }

    // ==================== ADDITIONAL CRITICAL INSTRUCTIONS ====================

    /// MOVEA - Move to Address Register.
    ///
    /// Moves data from source to an address register. Unlike MOVE, this instruction
    /// does not affect condition codes. Word operations are sign-extended to 32 bits.
    ///
    /// Reference: m68k-instruction-set.txt - MOVEA
    ///
    /// # Encoding
    /// Similar to MOVE but with destination mode bits indicating address register direct.
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn movea(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // MOVEA encoding: 00 size dest_reg 001 src_mode src_reg
        // size: 11=word (sign-extended), 10=long
        let size_bits = (opcode >> 12) & 0x3;
        let size = if size_bits == 0b11 {
            OperandSize::Word // Will be sign-extended to long
        } else if size_bits == 0b10 {
            OperandSize::Long
        } else {
            // Byte not valid for MOVEA
            return Self::illegal(registers, memory, opcode, pc);
        };

        let dest_reg = ((opcode >> 9) & 0x7) as u8;
        let src_mode = ((opcode >> 3) & 0x7) as u8;
        let src_reg = (opcode & 0x7) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(src_mode, src_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (src_ea, new_pc) = EaResolver::resolve(addr_mode, src_reg, size, registers, memory, pc);

        let value = EaResolver::read_operand(src_ea, size, registers, memory);

        // Sign-extend word to long if needed
        let final_value = if size == OperandSize::Word {
            i32::from(value as i16) as u32
        } else {
            value
        };

        registers.set_a(dest_reg as usize, final_value);

        InstructionResult::new(new_pc, 4)
    }

    /// SUBI - Subtract Immediate.
    ///
    /// Subtracts an immediate value from the destination operand.
    ///
    /// Reference: m68k-instruction-set.txt - SUBI
    ///
    /// # Flags
    /// X: Set according to carry
    /// N: Set if result is negative
    /// Z: Set if result is zero
    /// V: Set if overflow occurs
    /// C: Set if borrow occurs
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn subi(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // SUBI encoding: 0000 0100 size ea
        let size_bits = (opcode >> 6) & 0x3;
        let size = match size_bits {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => return Self::illegal(registers, memory, opcode, pc),
        };

        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        // Read immediate value first (PC+2)
        let (immediate, pc_after_imm) = if size == OperandSize::Long {
            (memory.read_long_unchecked(pc), pc + 4)
        } else if size == OperandSize::Word {
            (u32::from(memory.read_word_unchecked(pc)), pc + 2)
        } else {
            (u32::from(memory.read_word_unchecked(pc)) & 0xFF, pc + 2)
        };

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc_after_imm);

        let dest_value = EaResolver::read_operand(ea, size, registers, memory);

        let result = Self::sub_with_borrow(dest_value, immediate, false, size, registers, true);

        // Write back result
        EaResolver::write_operand(ea, size, result, registers, memory);

        InstructionResult::new(new_pc, 8)
    }

    /// SUBQ - Subtract Quick.
    ///
    /// Subtracts a small immediate value (1-8) from the destination.
    ///
    /// Reference: m68k-instruction-set.txt - SUBQ
    ///
    /// # Flags
    /// X, N, Z, V, C: Set according to result (unless destination is address register)
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn subq(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // SUBQ encoding: 0101 data 1 size ea
        let data = u32::from((opcode >> 9) & 0x7);
        let immediate = if data == 0 { 8 } else { data };

        let size_bits = (opcode >> 6) & 0x3;
        let size = match size_bits {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => return Self::illegal(registers, memory, opcode, pc),
        };

        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        // For address register operations, flags are NOT affected
        let is_address_reg = ea_mode == 1;

        if is_address_reg {
            // Address register - subtract directly, no flags
            // When destination is address register, operation is always on full 32 bits
            let dst = registers.a(ea_reg as usize);
            let result = dst.wrapping_sub(immediate);
            registers.set_a(ea_reg as usize, result);
            InstructionResult::new(pc, 4)
        } else {
            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) = EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc);
            let dest_value = EaResolver::read_operand(ea, size, registers, memory);
            let result = Self::sub_with_borrow(dest_value, immediate, false, size, registers, true);
            EaResolver::write_operand(ea, size, result, registers, memory);
            InstructionResult::new(new_pc, 4)
        }
    }

    /// CMPA - Compare Address.
    ///
    /// Compares an address register with source operand by subtraction.
    /// The address register is not modified, only flags are affected.
    ///
    /// Reference: m68k-instruction-set.txt - CMPA
    ///
    /// # Flags
    /// N, Z, V, C: Set according to result
    /// X: Not affected
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn cmpa(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // CMPA encoding: 1011 An size 11 ea
        let dest_reg = ((opcode >> 9) & 0x7) as usize;
        let size_bit = (opcode >> 8) & 0x1;
        let size = if size_bit == 0 {
            OperandSize::Word // Sign-extended to long
        } else {
            OperandSize::Long
        };

        let src_mode = ((opcode >> 3) & 0x7) as u8;
        let src_reg = (opcode & 0x7) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(src_mode, src_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (src_ea, new_pc) = EaResolver::resolve(addr_mode, src_reg, size, registers, memory, pc);

        let src_value = EaResolver::read_operand(src_ea, size, registers, memory);

        // Sign-extend if word
        let src_extended = if size == OperandSize::Word {
            i32::from(src_value as i16) as u32
        } else {
            src_value
        };

        let dest_value = registers.a(dest_reg);

        // Perform comparison (subtract without storing) - CMPA does not affect X
        let _ = Self::sub_with_borrow(
            dest_value,
            src_extended,
            false,
            OperandSize::Long,
            registers,
            false,
        );

        InstructionResult::new(new_pc, 6)
    }

    /// CMPI - Compare Immediate.
    ///
    /// Compares immediate data with destination by subtraction.
    /// The destination is not modified, only flags are affected.
    ///
    /// Reference: m68k-instruction-set.txt - CMPI
    ///
    /// # Flags
    /// N, Z, V, C: Set according to result
    /// X: Not affected
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn cmpi(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // CMPI encoding: 0000 1100 size ea
        let size_bits = (opcode >> 6) & 0x3;
        let size = match size_bits {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => return Self::illegal(registers, memory, opcode, pc),
        };

        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        // Read immediate value (PC+2)
        let (immediate, pc_after_imm) = if size == OperandSize::Long {
            (memory.read_long_unchecked(pc), pc + 4)
        } else if size == OperandSize::Word {
            (u32::from(memory.read_word_unchecked(pc)), pc + 2)
        } else {
            (u32::from(memory.read_word_unchecked(pc)) & 0xFF, pc + 2)
        };

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc_after_imm);

        let dest_value = EaResolver::read_operand(ea, size, registers, memory);

        // Perform comparison (subtract without storing result) - CMPI does not affect X
        let _ = Self::sub_with_borrow(dest_value, immediate, false, size, registers, false);

        InstructionResult::new(new_pc, 8)
    }

    /// ORI - Logical OR Immediate.
    ///
    /// ORs an immediate value with the destination.
    ///
    /// Reference: m68k-instruction-set.txt - ORI
    ///
    /// # Flags
    /// - N: Set if result is negative
    /// - Z: Set if result is zero
    /// - V: Always cleared
    /// - C: Always cleared
    /// - X: Not affected
    ///
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn ori(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // ORI encoding: 0000 0000 ssxx xxxx
        let size_bits = (opcode >> 6) & 0x3;
        let size = match size_bits {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => return Self::illegal(registers, memory, opcode, pc),
        };

        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        // Read immediate value
        let (immediate, pc_after_imm) = if size == OperandSize::Long {
            (memory.read_long_unchecked(pc), pc + 4)
        } else if size == OperandSize::Word {
            (u32::from(memory.read_word_unchecked(pc)), pc + 2)
        } else {
            (u32::from(memory.read_word_unchecked(pc)) & 0xFF, pc + 2)
        };

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc_after_imm);

        let dest_value = EaResolver::read_operand(ea, size, registers, memory);
        let result = dest_value | immediate;

        EaResolver::write_operand(ea, size, result, registers, memory);

        // Set flags: N, Z; clear V, C; X unaffected
        Self::set_logic_flags(registers, result, size);

        InstructionResult::new(new_pc, 8)
    }

    /// ANDI - Logical AND Immediate.
    ///
    /// ANDs an immediate value with the destination.
    ///
    /// Reference: m68k-instruction-set.txt - ANDI
    ///
    /// # Flags
    /// - N: Set if result is negative
    /// - Z: Set if result is zero
    /// - V: Always cleared
    /// - C: Always cleared
    /// - X: Not affected
    ///
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn andi(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // ANDI encoding: 0000 0010 ssxx xxxx
        let size_bits = (opcode >> 6) & 0x3;
        let size = match size_bits {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => return Self::illegal(registers, memory, opcode, pc),
        };

        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        // Read immediate value
        let (immediate, pc_after_imm) = if size == OperandSize::Long {
            (memory.read_long_unchecked(pc), pc + 4)
        } else if size == OperandSize::Word {
            (u32::from(memory.read_word_unchecked(pc)), pc + 2)
        } else {
            (u32::from(memory.read_word_unchecked(pc)) & 0xFF, pc + 2)
        };

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc_after_imm);

        let dest_value = EaResolver::read_operand(ea, size, registers, memory);
        let result = dest_value & immediate;

        EaResolver::write_operand(ea, size, result, registers, memory);

        // Set flags: N, Z; clear V, C; X unaffected
        Self::set_logic_flags(registers, result, size);

        InstructionResult::new(new_pc, 8)
    }

    /// EORI - Logical Exclusive OR Immediate.
    ///
    /// XORs an immediate value with the destination.
    ///
    /// Reference: m68k-instruction-set.txt - EORI
    ///
    /// # Flags
    /// - N: Set if result is negative
    /// - Z: Set if result is zero
    /// - V: Always cleared
    /// - C: Always cleared
    /// - X: Not affected
    ///
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn eori(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // EORI encoding: 0000 1010 ssxx xxxx
        let size_bits = (opcode >> 6) & 0x3;
        let size = match size_bits {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => return Self::illegal(registers, memory, opcode, pc),
        };

        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        // Read immediate value
        let (immediate, pc_after_imm) = if size == OperandSize::Long {
            (memory.read_long_unchecked(pc), pc + 4)
        } else if size == OperandSize::Word {
            (u32::from(memory.read_word_unchecked(pc)), pc + 2)
        } else {
            (u32::from(memory.read_word_unchecked(pc)) & 0xFF, pc + 2)
        };

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };

        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc_after_imm);

        let dest_value = EaResolver::read_operand(ea, size, registers, memory);
        let result = dest_value ^ immediate;

        EaResolver::write_operand(ea, size, result, registers, memory);

        // Set flags: N, Z; clear V, C; X unaffected
        Self::set_logic_flags(registers, result, size);

        InstructionResult::new(new_pc, 8)
    }

    /// NOP - No Operation.
    ///
    /// Does nothing, advances PC by 2.
    ///
    /// Reference: m68k-instruction-set.txt - NOP
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn nop(
        _registers: &mut RegisterFile,
        _memory: &Memory,
        _opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        InstructionResult::new(pc, 4)
    }

    /// ADDX - Add Extended.
    ///
    /// Adds source, destination, and the X flag. Used for multi-precision arithmetic.
    /// Zero flag is only cleared if result is non-zero (preserves across multiple operations).
    ///
    /// Reference: m68k-instruction-set.txt - ADDX
    ///
    /// # Flags
    /// X: Set according to carry
    /// N: Set if result is negative
    /// Z: Cleared if result is non-zero, unchanged otherwise
    /// V: Set if overflow
    /// C: Set if carry
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn addx(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // ADDX encoding: 1101 Rx 1 size 00 M Ry
        // M bit (bit 3): 0 = data register, 1 = memory (predecrement)
        let rx = ((opcode >> 9) & 0x7) as u8;
        let ry = (opcode & 0x7) as u8;
        let size_bits = (opcode >> 6) & 0x3;
        let size = match size_bits {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => return Self::illegal(registers, memory, opcode, pc),
        };
        let memory_mode = (opcode >> 3) & 0x1 == 1;

        let extend = registers.get_x();

        // Save current Z flag for special handling
        let old_z = registers.get_z();

        if memory_mode {
            // ADDX memory mode: -(Ay) + -(Ax) + X -> -(Ax)
            // Operation: destination + source + X -> destination
            // where source is Ay and destination is Ax
            // A7 (stack pointer) must always move by at least 2, even for byte operations
            let bytes_y = if ry == 7 && size == OperandSize::Byte {
                2
            } else {
                match size {
                    OperandSize::Byte => 1,
                    OperandSize::Word => 2,
                    OperandSize::Long => 4,
                }
            };
            let bytes_x = if rx == 7 && size == OperandSize::Byte {
                2
            } else {
                match size {
                    OperandSize::Byte => 1,
                    OperandSize::Word => 2,
                    OperandSize::Long => 4,
                }
            };

            // Predecrement source (Ay)
            let ay_val = registers.a(ry as usize).wrapping_sub(bytes_y);
            registers.set_a(ry as usize, ay_val);
            let src = match size {
                OperandSize::Byte => u32::from(memory.read_byte_unchecked(ay_val)),
                OperandSize::Word => u32::from(memory.read_word_unchecked(ay_val)),
                OperandSize::Long => memory.read_long_unchecked(ay_val),
            };

            // Predecrement destination (Ax)
            let ax_val = registers.a(rx as usize).wrapping_sub(bytes_x);
            registers.set_a(rx as usize, ax_val);
            let dst = match size {
                OperandSize::Byte => u32::from(memory.read_byte_unchecked(ax_val)),
                OperandSize::Word => u32::from(memory.read_word_unchecked(ax_val)),
                OperandSize::Long => memory.read_long_unchecked(ax_val),
            };

            let result = Self::add_with_carry(dst, src, extend, size, registers);

            // ADDX special Z flag: only clear if result is non-zero, unchanged otherwise
            if result != 0 {
                registers.set_z(false);
            } else {
                registers.set_z(old_z);
            }

            // Write result to destination (Ax)
            match size {
                OperandSize::Byte => {
                    let _ = memory.write_byte(ax_val, result as u8);
                }
                OperandSize::Word => {
                    let _ = memory.write_word(ax_val, result as u16);
                }
                OperandSize::Long => {
                    let _ = memory.write_long(ax_val, result);
                }
            }

            InstructionResult::new(pc, 18)
        } else {
            // Dy + Dx + X -> Dx
            let src = registers.d(ry as usize);
            let dst = registers.d(rx as usize);

            let result = Self::add_with_carry(dst, src, extend, size, registers);

            // ADDX special Z flag: only clear if result is non-zero, unchanged otherwise
            if result != 0 {
                registers.set_z(false);
            } else {
                registers.set_z(old_z);
            }

            // Merge result with destination, preserving upper bits for byte/word ops
            let mask = size.mask();
            let merged = (dst & !mask) | (result & mask);
            registers.set_d(rx as usize, merged);

            InstructionResult::new(pc, 4)
        }
    }

    /// SUBX - Subtract Extended.
    ///
    /// Subtracts source and X flag from destination. Used for multi-precision arithmetic.
    /// Zero flag is only cleared if result is non-zero (preserves across multiple operations).
    ///
    /// Reference: m68k-instruction-set.txt - SUBX
    ///
    /// # Flags
    /// X: Set according to borrow
    /// N: Set if result is negative
    /// Z: Cleared if result is non-zero, unchanged otherwise
    /// V: Set if overflow
    /// C: Set if borrow
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn subx(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // SUBX encoding: 1001 Rx 1 size 00 M Ry
        let rx = ((opcode >> 9) & 0x7) as u8;
        let ry = (opcode & 0x7) as u8;
        let size_bits = (opcode >> 6) & 0x3;
        let size = match size_bits {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => return Self::illegal(registers, memory, opcode, pc),
        };
        let memory_mode = (opcode >> 3) & 0x1 == 1;

        let extend = registers.get_x();

        // Save current Z flag for special handling
        let old_z = registers.get_z();

        if memory_mode {
            // SUBX memory mode: -(Ax) - -(Ay) - X -> -(Ax)
            // Operation: destination - source - X -> destination
            // where source is Ay and destination is Ax
            // A7 (stack pointer) must always move by at least 2, even for byte operations
            let bytes_y = if ry == 7 && size == OperandSize::Byte {
                2
            } else {
                match size {
                    OperandSize::Byte => 1,
                    OperandSize::Word => 2,
                    OperandSize::Long => 4,
                }
            };
            let bytes_x = if rx == 7 && size == OperandSize::Byte {
                2
            } else {
                match size {
                    OperandSize::Byte => 1,
                    OperandSize::Word => 2,
                    OperandSize::Long => 4,
                }
            };

            // Predecrement source (Ay)
            let ay_val = registers.a(ry as usize).wrapping_sub(bytes_y);
            registers.set_a(ry as usize, ay_val);
            let src = match size {
                OperandSize::Byte => u32::from(memory.read_byte_unchecked(ay_val)),
                OperandSize::Word => u32::from(memory.read_word_unchecked(ay_val)),
                OperandSize::Long => memory.read_long_unchecked(ay_val),
            };

            // Predecrement destination (Ax)
            let ax_val = registers.a(rx as usize).wrapping_sub(bytes_x);
            registers.set_a(rx as usize, ax_val);
            let dst = match size {
                OperandSize::Byte => u32::from(memory.read_byte_unchecked(ax_val)),
                OperandSize::Word => u32::from(memory.read_word_unchecked(ax_val)),
                OperandSize::Long => memory.read_long_unchecked(ax_val),
            };

            let result = Self::sub_with_borrow(dst, src, extend, size, registers, true);

            // SUBX special Z flag: only clear if result is non-zero, unchanged otherwise
            if result != 0 {
                registers.set_z(false);
            } else {
                registers.set_z(old_z);
            }

            // Write result to destination (Ax)
            match size {
                OperandSize::Byte => {
                    let _ = memory.write_byte(ax_val, result as u8);
                }
                OperandSize::Word => {
                    let _ = memory.write_word(ax_val, result as u16);
                }
                OperandSize::Long => {
                    let _ = memory.write_long(ax_val, result);
                }
            }

            InstructionResult::new(pc, 18)
        } else {
            // Dx - Dy - X -> Dx
            let src = registers.d(ry as usize);
            let dst = registers.d(rx as usize);

            let result = Self::sub_with_borrow(dst, src, extend, size, registers, true);

            // SUBX special Z flag: only clear if result is non-zero, unchanged otherwise
            if result != 0 {
                registers.set_z(false);
            } else {
                registers.set_z(old_z);
            }

            // Merge result with destination, preserving upper bits for byte/word ops
            let mask = size.mask();
            let merged = (dst & !mask) | (result & mask);
            registers.set_d(rx as usize, merged);

            InstructionResult::new(pc, 4)
        }
    }

    /// SWAP - Swap Register Halves.
    ///
    /// Exchanges the upper and lower 16-bit words of a data register.
    ///
    /// Reference: m68k-instruction-set.txt - SWAP
    ///
    /// # Flags
    /// N: Set if result is negative (bit 31 set)
    /// Z: Set if result is zero
    /// V: Always cleared
    /// C: Always cleared
    /// X: Not affected
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn swap(
        registers: &mut RegisterFile,
        _memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // SWAP encoding: 0100 1000 0100 0 Dn
        let reg = (opcode & 0x7) as usize;

        let value = registers.d(reg);
        let swapped = ((value & 0xFFFF) << 16) | ((value >> 16) & 0xFFFF);

        registers.set_d(reg, swapped);

        // Set flags
        registers.set_n((swapped & 0x8000_0000) != 0);
        registers.set_z(swapped == 0);
        registers.set_v(false);
        registers.set_c(false);

        InstructionResult::new(pc, 4)
    }

    /// LSL - Logical Shift Left.
    ///
    /// Shifts bits left, filling with zeros from the right.
    ///
    /// Reference: m68k-instruction-set.txt - LSL
    ///
    /// # Flags
    /// X: Set to last bit shifted out
    /// N: Set if result is negative
    /// Z: Set if result is zero
    /// V: Always cleared
    /// C: Set to last bit shifted out
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn lsl(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // LSL encoding: 1110 count/reg 1 size i 01 mode/reg
        // i bit (bit 5): 0 = immediate count, 1 = register count
        let is_reg = (opcode >> 5) & 0x1 == 1;

        if (opcode & 0xC0) == 0xC0 {
            // Memory shift (always 1 bit)
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) =
                EaResolver::resolve(addr_mode, ea_reg, OperandSize::Word, registers, memory, pc);

            let value = EaResolver::read_operand(ea, OperandSize::Word, registers, memory) as u16;
            let result = value << 1;
            let carry = (value & 0x8000) != 0;

            EaResolver::write_operand(ea, OperandSize::Word, u32::from(result), registers, memory);

            registers.set_n((result & 0x8000) != 0);
            registers.set_z(result == 0);
            registers.set_v(false);
            registers.set_c(carry);
            registers.set_x(carry);

            InstructionResult::new(new_pc, 8)
        } else {
            // Register shift
            let count_reg = ((opcode >> 9) & 0x7) as usize;
            let data_reg = (opcode & 0x7) as usize;
            let size_bits = (opcode >> 6) & 0x3;
            let size = match size_bits {
                0b00 => OperandSize::Byte,
                0b01 => OperandSize::Word,
                0b10 => OperandSize::Long,
                _ => return Self::illegal(registers, memory, opcode, pc),
            };

            let count = if is_reg {
                registers.d(count_reg) & 0x3F // Modulo 64
            } else {
                let c = count_reg as u32;
                if c == 0 {
                    8
                } else {
                    c
                } // 0 means 8
            };

            let value = registers.d(data_reg);
            let mask = size.mask();
            let masked_value = value & mask;

            let (result, carry) = if count == 0 {
                (masked_value, false)
            } else if count < size.bits() as u32 {
                let shifted = masked_value << count;
                let last_bit = (masked_value >> (size.bits() as u32 - count)) & 1;
                (shifted & mask, last_bit != 0)
            } else {
                (
                    0,
                    if count == size.bits() as u32 {
                        (masked_value & 1) != 0
                    } else {
                        false
                    },
                )
            };

            // Preserve upper bits for byte/word operations
            let new_value = (value & !mask) | (result & mask);
            registers.set_d(data_reg, new_value);

            let sign_bit = size.sign_bit();
            registers.set_n((result & sign_bit) != 0);
            registers.set_z((result & mask) == 0);
            registers.set_v(false);
            registers.set_c(carry);
            if count > 0 {
                registers.set_x(carry);
            }

            InstructionResult::new(pc, 6 + (count * 2) as u8)
        }
    }

    /// LSR - Logical Shift Right.
    ///
    /// Shifts bits right, filling with zeros from the left.
    ///
    /// Reference: m68k-instruction-set.txt - LSR
    ///
    /// # Flags
    /// X: Set to last bit shifted out
    /// N: Always cleared (zeros shifted in)
    /// Z: Set if result is zero
    /// V: Always cleared
    /// C: Set to last bit shifted out
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn lsr(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // LSR encoding: 1110 count/reg 0 size i 01 mode/reg
        let is_reg = (opcode >> 5) & 0x1 == 1;

        if (opcode & 0xC0) == 0xC0 {
            // Memory shift (always 1 bit)
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) =
                EaResolver::resolve(addr_mode, ea_reg, OperandSize::Word, registers, memory, pc);

            let value = EaResolver::read_operand(ea, OperandSize::Word, registers, memory) as u16;
            let result = value >> 1;
            let carry = (value & 1) != 0;

            EaResolver::write_operand(ea, OperandSize::Word, u32::from(result), registers, memory);

            registers.set_n(false); // Always 0 for LSR
            registers.set_z(result == 0);
            registers.set_v(false);
            registers.set_c(carry);
            registers.set_x(carry);

            InstructionResult::new(new_pc, 8)
        } else {
            // Register shift
            let count_reg = ((opcode >> 9) & 0x7) as usize;
            let data_reg = (opcode & 0x7) as usize;
            let size_bits = (opcode >> 6) & 0x3;
            let size = match size_bits {
                0b00 => OperandSize::Byte,
                0b01 => OperandSize::Word,
                0b10 => OperandSize::Long,
                _ => return Self::illegal(registers, memory, opcode, pc),
            };

            let count = if is_reg {
                registers.d(count_reg) & 0x3F
            } else {
                let c = count_reg as u32;
                if c == 0 {
                    8
                } else {
                    c
                }
            };

            let value = registers.d(data_reg);
            let mask = size.mask();
            let masked_value = value & mask;

            let (result, carry) = if count == 0 {
                (masked_value, false)
            } else if count < size.bits() as u32 {
                let shifted = masked_value >> count;
                let last_bit = (masked_value >> (count - 1)) & 1;
                (shifted, last_bit != 0)
            } else {
                (
                    0,
                    if count == size.bits() as u32 {
                        ((masked_value >> (count - 1)) & 1) != 0
                    } else {
                        false
                    },
                )
            };

            // Preserve upper bits for byte/word operations
            let new_value = (value & !mask) | (result & mask);
            registers.set_d(data_reg, new_value);

            // N is set if result MSB is set (only possible when count=0)
            let msb = match size {
                OperandSize::Byte => result & 0x80 != 0,
                OperandSize::Word => result & 0x8000 != 0,
                OperandSize::Long => result & 0x8000_0000 != 0,
            };
            registers.set_n(msb);
            registers.set_z((result & mask) == 0);
            registers.set_v(false);
            registers.set_c(carry);
            if count > 0 {
                registers.set_x(carry);
            }

            InstructionResult::new(pc, 6 + (count * 2) as u8)
        }
    }

    /// ROL - Rotate Left (no extend).
    ///
    /// Rotates bits left, with bits shifting out on the left coming back in on the right.
    ///
    /// Reference: m68k-instruction-set.txt - ROL
    ///
    /// # Flags
    /// N: Set if result is negative
    /// Z: Set if result is zero
    /// V: Always cleared
    /// C: Set to last bit rotated out
    /// X: Not affected
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn rol(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // ROL encoding: 1110 count/reg 1 size i 11 mode/reg
        let is_reg = (opcode >> 5) & 0x1 == 1;

        if (opcode & 0xC0) == 0xC0 {
            // Memory rotate (always 1 bit)
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) =
                EaResolver::resolve(addr_mode, ea_reg, OperandSize::Word, registers, memory, pc);

            let value = EaResolver::read_operand(ea, OperandSize::Word, registers, memory) as u16;
            let msb = (value & 0x8000) != 0;
            let result = (value << 1) | u16::from(msb);

            EaResolver::write_operand(ea, OperandSize::Word, u32::from(result), registers, memory);

            registers.set_n((result & 0x8000) != 0);
            registers.set_z(result == 0);
            registers.set_v(false);
            registers.set_c(msb);

            InstructionResult::new(new_pc, 8)
        } else {
            // Register rotate
            let count_reg = ((opcode >> 9) & 0x7) as usize;
            let data_reg = (opcode & 0x7) as usize;
            let size_bits = (opcode >> 6) & 0x3;
            let size = match size_bits {
                0b00 => OperandSize::Byte,
                0b01 => OperandSize::Word,
                0b10 => OperandSize::Long,
                _ => return Self::illegal(registers, memory, opcode, pc),
            };

            let count = if is_reg {
                registers.d(count_reg) & 0x3F
            } else {
                let c = count_reg as u32;
                if c == 0 {
                    8
                } else {
                    c
                }
            };

            let value = registers.d(data_reg);
            let mask = size.mask();
            let bits = size.bits() as u32;
            let masked_value = value & mask;

            let effective_count = count % bits;
            let (result, carry) = if effective_count == 0 {
                if count == 0 {
                    // No rotation at all, C is cleared
                    (masked_value, false)
                } else {
                    // Full rotation(s) - result unchanged, carry is the LSB (last bit rotated out)
                    // because after bits rotations, the last bit shifted out was bit 0
                    (masked_value, (masked_value & 1) != 0)
                }
            } else {
                let rotated = ((masked_value << effective_count)
                    | (masked_value >> (bits - effective_count)))
                    & mask;
                let last_bit = (rotated & 1) != 0;
                (rotated, last_bit)
            };

            // Preserve upper bits for byte/word operations
            let new_value = (value & !mask) | (result & mask);
            registers.set_d(data_reg, new_value);

            let sign_bit = size.sign_bit();
            registers.set_n((result & sign_bit) != 0);
            registers.set_z((result & mask) == 0);
            registers.set_v(false);
            // C is set to last bit rotated out, or cleared if count is 0
            if count > 0 {
                registers.set_c(carry);
            } else {
                registers.set_c(false);
            }

            InstructionResult::new(pc, 6 + (count * 2) as u8)
        }
    }

    /// ROR - Rotate Right (no extend).
    ///
    /// Rotates bits right, with bits shifting out on the right coming back in on the left.
    ///
    /// Reference: m68k-instruction-set.txt - ROR
    ///
    /// # Flags
    /// N: Set if result is negative
    /// Z: Set if result is zero
    /// V: Always cleared
    /// C: Set to last bit rotated out
    /// X: Not affected
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn ror(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // ROR encoding: 1110 count/reg 0 size i 11 mode/reg
        let is_reg = (opcode >> 5) & 0x1 == 1;

        if (opcode & 0xC0) == 0xC0 {
            // Memory rotate (always 1 bit)
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) =
                EaResolver::resolve(addr_mode, ea_reg, OperandSize::Word, registers, memory, pc);

            let value = EaResolver::read_operand(ea, OperandSize::Word, registers, memory) as u16;
            let lsb = (value & 1) != 0;
            let result = (value >> 1) | if lsb { 0x8000 } else { 0 };

            EaResolver::write_operand(ea, OperandSize::Word, u32::from(result), registers, memory);

            registers.set_n((result & 0x8000) != 0);
            registers.set_z(result == 0);
            registers.set_v(false);
            registers.set_c(lsb);

            InstructionResult::new(new_pc, 8)
        } else {
            // Register rotate
            let count_reg = ((opcode >> 9) & 0x7) as usize;
            let data_reg = (opcode & 0x7) as usize;
            let size_bits = (opcode >> 6) & 0x3;
            let size = match size_bits {
                0b00 => OperandSize::Byte,
                0b01 => OperandSize::Word,
                0b10 => OperandSize::Long,
                _ => return Self::illegal(registers, memory, opcode, pc),
            };

            let count = if is_reg {
                registers.d(count_reg) & 0x3F
            } else {
                let c = count_reg as u32;
                if c == 0 {
                    8
                } else {
                    c
                }
            };

            let value = registers.d(data_reg);
            let mask = size.mask();
            let bits = size.bits() as u32;
            let masked_value = value & mask;

            let effective_count = count % bits;
            let (result, carry) = if effective_count == 0 {
                if count == 0 {
                    // No rotation at all, C is cleared
                    (masked_value, false)
                } else {
                    // Full rotation(s) - result unchanged, carry is the MSB (last bit rotated out)
                    // because after bits rotations, the last bit shifted out was bit (bits-1)
                    let sign_bit = size.sign_bit();
                    (masked_value, (masked_value & sign_bit) != 0)
                }
            } else {
                let rotated = ((masked_value >> effective_count)
                    | (masked_value << (bits - effective_count)))
                    & mask;
                let last_bit = ((masked_value >> (effective_count - 1)) & 1) != 0;
                (rotated, last_bit)
            };

            // Preserve upper bits for byte/word operations
            let new_value = (value & !mask) | (result & mask);
            registers.set_d(data_reg, new_value);

            let sign_bit = size.sign_bit();
            registers.set_n((result & sign_bit) != 0);
            registers.set_z((result & mask) == 0);
            registers.set_v(false);
            // C is set to last bit rotated out, or cleared if count is 0
            if count > 0 {
                registers.set_c(carry);
            } else {
                registers.set_c(false);
            }

            InstructionResult::new(pc, 6 + (count * 2) as u8)
        }
    }

    /// BTST - Bit Test.
    ///
    /// Tests a bit in the destination operand and sets the Z flag accordingly.
    /// The bit is NOT modified.
    ///
    /// Reference: m68k-instruction-set.txt - BTST
    ///
    /// # Flags
    /// Z: Set if tested bit is zero
    /// N, V, C, X: Not affected
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn btst(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // BTST encoding: 0000 reg 100 ea (dynamic) or 0000 1000 00 ea (static)
        // Distinguish by bit 11: 0 = dynamic (register), 1 = static (immediate)
        let is_dynamic = (opcode >> 11) & 0x1 == 0;

        if is_dynamic {
            // BTST Dn, <ea>
            let bit_reg = ((opcode >> 9) & 0x7) as usize;
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let bit_num = registers.d(bit_reg);

            // Size depends on whether destination is register or memory
            let is_reg = ea_mode == 0;
            let size = if is_reg {
                OperandSize::Long
            } else {
                OperandSize::Byte
            };
            let modulo = if is_reg { 32 } else { 8 };
            let bit_num = (bit_num % modulo) as u8;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) = EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc);

            let value = EaResolver::read_operand(ea, size, registers, memory);
            let bit_set = (value & (1 << bit_num)) != 0;

            registers.set_z(!bit_set);

            InstructionResult::new(new_pc, if is_reg { 6 } else { 4 })
        } else {
            // BTST #imm, <ea>
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let bit_num = (memory.read_word_unchecked(pc) & 0xFF) as u8;
            let pc = pc + 2;

            let is_reg = ea_mode == 0;
            let size = if is_reg {
                OperandSize::Long
            } else {
                OperandSize::Byte
            };
            let modulo = if is_reg { 32 } else { 8 };
            let bit_num = bit_num % modulo;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) = EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc);

            let value = EaResolver::read_operand(ea, size, registers, memory);
            let bit_set = (value & (1 << bit_num)) != 0;

            registers.set_z(!bit_set);

            InstructionResult::new(new_pc, if is_reg { 10 } else { 8 })
        }
    }

    /// BSET - Bit Set.
    ///
    /// Tests a bit in the destination operand (sets Z flag), then sets the bit to 1.
    ///
    /// Reference: m68k-instruction-set.txt - BSET
    ///
    /// # Flags
    /// Z: Set if tested bit was zero (before modification)
    /// N, V, C, X: Not affected
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn bset(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // BSET encoding: 0000 reg 111 ea (dynamic) or 0000 1000 11 ea (static)
        // Distinguish by bit 8: 1 = dynamic (register), 0 = static (immediate)
        let is_dynamic = (opcode >> 8) & 0x1 == 1;

        if is_dynamic {
            // BSET Dn, <ea>
            let bit_reg = ((opcode >> 9) & 0x7) as usize;
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let bit_num = registers.d(bit_reg);

            let is_reg = ea_mode == 0;
            let size = if is_reg {
                OperandSize::Long
            } else {
                OperandSize::Byte
            };
            let modulo = if is_reg { 32 } else { 8 };
            let bit_num = (bit_num % modulo) as u8;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) = EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc);

            let value = EaResolver::read_operand(ea, size, registers, memory);
            let bit_set = (value & (1 << bit_num)) != 0;
            let new_value = value | (1 << bit_num);

            EaResolver::write_operand(ea, size, new_value, registers, memory);
            registers.set_z(!bit_set);

            InstructionResult::new(new_pc, 8)
        } else {
            // BSET #imm, <ea>
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let bit_num = (memory.read_word_unchecked(pc) & 0xFF) as u8;
            let pc = pc + 2;

            let is_reg = ea_mode == 0;
            let size = if is_reg {
                OperandSize::Long
            } else {
                OperandSize::Byte
            };
            let modulo = if is_reg { 32 } else { 8 };
            let bit_num = bit_num % modulo;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) = EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc);

            let value = EaResolver::read_operand(ea, size, registers, memory);
            let bit_set = (value & (1 << bit_num)) != 0;
            let new_value = value | (1 << bit_num);

            EaResolver::write_operand(ea, size, new_value, registers, memory);
            registers.set_z(!bit_set);

            InstructionResult::new(new_pc, 12)
        }
    }

    /// BCLR - Bit Clear.
    ///
    /// Tests a bit in the destination operand (sets Z flag), then clears the bit to 0.
    ///
    /// Reference: m68k-instruction-set.txt - BCLR
    ///
    /// # Flags
    /// Z: Set if tested bit was zero (before modification)
    /// N, V, C, X: Not affected
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn bclr(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // BCLR encoding: 0000 reg 110 ea (dynamic) or 0000 1000 10 ea (static)
        // Distinguish by bit 8: 1 = dynamic (register), 0 = static (immediate)
        let is_dynamic = (opcode >> 8) & 0x1 == 1;

        if is_dynamic {
            // BCLR Dn, <ea>
            let bit_reg = ((opcode >> 9) & 0x7) as usize;
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let bit_num = registers.d(bit_reg);

            let is_reg = ea_mode == 0;
            let size = if is_reg {
                OperandSize::Long
            } else {
                OperandSize::Byte
            };
            let modulo = if is_reg { 32 } else { 8 };
            let bit_num = (bit_num % modulo) as u8;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) = EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc);

            let value = EaResolver::read_operand(ea, size, registers, memory);
            let bit_set = (value & (1 << bit_num)) != 0;
            let new_value = value & !(1 << bit_num);

            EaResolver::write_operand(ea, size, new_value, registers, memory);
            registers.set_z(!bit_set);

            InstructionResult::new(new_pc, 8)
        } else {
            // BCLR #imm, <ea>
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let bit_num = (memory.read_word_unchecked(pc) & 0xFF) as u8;
            let pc = pc + 2;

            let is_reg = ea_mode == 0;
            let size = if is_reg {
                OperandSize::Long
            } else {
                OperandSize::Byte
            };
            let modulo = if is_reg { 32 } else { 8 };
            let bit_num = bit_num % modulo;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) = EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc);

            let value = EaResolver::read_operand(ea, size, registers, memory);
            let bit_set = (value & (1 << bit_num)) != 0;
            let new_value = value & !(1 << bit_num);

            EaResolver::write_operand(ea, size, new_value, registers, memory);
            registers.set_z(!bit_set);

            InstructionResult::new(new_pc, 12)
        }
    }

    /// BCHG - Bit Change (toggle).
    ///
    /// Tests a bit in the destination operand (sets Z flag), then toggles the bit.
    ///
    /// Reference: m68k-instruction-set.txt - BCHG
    ///
    /// # Flags
    /// Z: Set if tested bit was zero (before modification)
    /// N, V, C, X: Not affected
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn bchg(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // BCHG encoding: 0000 reg 101 ea (dynamic) or 0000 1000 01 ea (static)
        // Distinguish by bit 8: 1 = dynamic (register), 0 = static (immediate)
        let is_dynamic = (opcode >> 8) & 0x1 == 1;

        if is_dynamic {
            // BCHG Dn, <ea>
            let bit_reg = ((opcode >> 9) & 0x7) as usize;
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let bit_num = registers.d(bit_reg);

            let is_reg = ea_mode == 0;
            let size = if is_reg {
                OperandSize::Long
            } else {
                OperandSize::Byte
            };
            let modulo = if is_reg { 32 } else { 8 };
            let bit_num = (bit_num % modulo) as u8;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) = EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc);

            let value = EaResolver::read_operand(ea, size, registers, memory);
            let bit_set = (value & (1 << bit_num)) != 0;
            let new_value = value ^ (1 << bit_num);

            EaResolver::write_operand(ea, size, new_value, registers, memory);
            registers.set_z(!bit_set);

            InstructionResult::new(new_pc, 8)
        } else {
            // BCHG #imm, <ea>
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let bit_num = (memory.read_word_unchecked(pc) & 0xFF) as u8;
            let pc = pc + 2;

            let is_reg = ea_mode == 0;
            let size = if is_reg {
                OperandSize::Long
            } else {
                OperandSize::Byte
            };
            let modulo = if is_reg { 32 } else { 8 };
            let bit_num = bit_num % modulo;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) = EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc);

            let value = EaResolver::read_operand(ea, size, registers, memory);
            let bit_set = (value & (1 << bit_num)) != 0;
            let new_value = value ^ (1 << bit_num);

            EaResolver::write_operand(ea, size, new_value, registers, memory);
            registers.set_z(!bit_set);

            InstructionResult::new(new_pc, 12)
        }
    }

    /// ROXL - Rotate Left Through Extend.
    ///
    /// Rotates bits left through the X flag. X flag becomes the LSB, MSB goes to X flag.
    ///
    /// Reference: m68k-instruction-set.txt - ROXL
    ///
    /// # Flags
    /// X: Set to last bit rotated out
    /// N: Set if result is negative
    /// Z: Set if result is zero
    /// V: Always cleared
    /// C: Set to same as X
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn roxl(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // ROXL encoding: 1110 count/reg 1 size i 10 mode/reg
        let is_reg = (opcode >> 5) & 0x1 == 1;

        if (opcode & 0xC0) == 0xC0 {
            // Memory rotate (always 1 bit)
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) =
                EaResolver::resolve(addr_mode, ea_reg, OperandSize::Word, registers, memory, pc);

            let value = EaResolver::read_operand(ea, OperandSize::Word, registers, memory) as u16;
            let x_bit = u16::from(registers.get_x());
            let msb = (value & 0x8000) != 0;
            let result = (value << 1) | x_bit;

            EaResolver::write_operand(ea, OperandSize::Word, u32::from(result), registers, memory);

            registers.set_n((result & 0x8000) != 0);
            registers.set_z(result == 0);
            registers.set_v(false);
            registers.set_c(msb);
            registers.set_x(msb);

            InstructionResult::new(new_pc, 8)
        } else {
            // Register rotate
            let count_reg = ((opcode >> 9) & 0x7) as usize;
            let data_reg = (opcode & 0x7) as usize;
            let size_bits = (opcode >> 6) & 0x3;
            let size = match size_bits {
                0b00 => OperandSize::Byte,
                0b01 => OperandSize::Word,
                0b10 => OperandSize::Long,
                _ => return Self::illegal(registers, memory, opcode, pc),
            };

            let count = if is_reg {
                registers.d(count_reg) & 0x3F
            } else {
                let c = count_reg as u32;
                if c == 0 {
                    8
                } else {
                    c
                }
            };

            let value = registers.d(data_reg);
            let mask = size.mask();
            let bits = size.bits() as u32;
            let mut result = value & mask;
            let mut x = registers.get_x();

            for _ in 0..count {
                let msb = (result & (1 << (bits - 1))) != 0;
                result = (result << 1) | u32::from(x);
                x = msb;
            }

            result &= mask;
            // Preserve upper bits for byte/word operations
            let new_value = (value & !mask) | (result & mask);
            registers.set_d(data_reg, new_value);

            let sign_bit = size.sign_bit();
            registers.set_n((result & sign_bit) != 0);
            registers.set_z((result & mask) == 0);
            registers.set_v(false);
            if count > 0 {
                registers.set_c(x);
                registers.set_x(x);
            } else {
                // When count is 0, C is set to X (X is unchanged)
                registers.set_c(registers.get_x());
            }

            InstructionResult::new(pc, 6 + (count * 2) as u8)
        }
    }

    /// ROXR - Rotate Right Through Extend.
    ///
    /// Rotates bits right through the X flag. X flag becomes the MSB, LSB goes to X flag.
    ///
    /// Reference: m68k-instruction-set.txt - ROXR
    ///
    /// # Flags
    /// X: Set to last bit rotated out
    /// N: Set if result is negative
    /// Z: Set if result is zero
    /// V: Always cleared
    /// C: Set to same as X
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn roxr(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // ROXR encoding: 1110 count/reg 0 size i 10 mode/reg
        let is_reg = (opcode >> 5) & 0x1 == 1;

        if (opcode & 0xC0) == 0xC0 {
            // Memory rotate (always 1 bit)
            let ea_mode = ((opcode >> 3) & 0x7) as u8;
            let ea_reg = (opcode & 0x7) as u8;

            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, new_pc) =
                EaResolver::resolve(addr_mode, ea_reg, OperandSize::Word, registers, memory, pc);

            let value = EaResolver::read_operand(ea, OperandSize::Word, registers, memory) as u16;
            let x_bit = if registers.get_x() { 0x8000u16 } else { 0 };
            let lsb = (value & 1) != 0;
            let result = (value >> 1) | x_bit;

            EaResolver::write_operand(ea, OperandSize::Word, u32::from(result), registers, memory);

            registers.set_n((result & 0x8000) != 0);
            registers.set_z(result == 0);
            registers.set_v(false);
            registers.set_c(lsb);
            registers.set_x(lsb);

            InstructionResult::new(new_pc, 8)
        } else {
            // Register rotate
            let count_reg = ((opcode >> 9) & 0x7) as usize;
            let data_reg = (opcode & 0x7) as usize;
            let size_bits = (opcode >> 6) & 0x3;
            let size = match size_bits {
                0b00 => OperandSize::Byte,
                0b01 => OperandSize::Word,
                0b10 => OperandSize::Long,
                _ => return Self::illegal(registers, memory, opcode, pc),
            };

            let count = if is_reg {
                registers.d(count_reg) & 0x3F
            } else {
                let c = count_reg as u32;
                if c == 0 {
                    8
                } else {
                    c
                }
            };

            let value = registers.d(data_reg);
            let mask = size.mask();
            let bits = size.bits() as u32;
            let mut result = value & mask;
            let mut x = registers.get_x();

            for _ in 0..count {
                let lsb = (result & 1) != 0;
                result = (result >> 1) | if x { 1 << (bits - 1) } else { 0 };
                x = lsb;
            }

            result &= mask;
            // Preserve upper bits for byte/word operations
            let new_value = (value & !mask) | (result & mask);
            registers.set_d(data_reg, new_value);

            let sign_bit = size.sign_bit();
            registers.set_n((result & sign_bit) != 0);
            registers.set_z((result & mask) == 0);
            registers.set_v(false);
            if count > 0 {
                registers.set_c(x);
                registers.set_x(x);
            } else {
                // When count is 0, C is set to X (X is unchanged)
                registers.set_c(registers.get_x());
            }

            InstructionResult::new(pc, 6 + (count * 2) as u8)
        }
    }

    /// EXG - Exchange Registers.
    ///
    /// Exchanges the contents of two registers (data-data, addr-addr, or data-addr).
    ///
    /// Reference: m68k-instruction-set.txt - EXG
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn exg(
        registers: &mut RegisterFile,
        _memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // EXG encoding: 1100 Rx 1 opmode Ry
        // opmode: 01000 = Dx,Dy; 01001 = Ax,Ay; 10001 = Dx,Ay
        let rx = ((opcode >> 9) & 0x7) as usize;
        let ry = (opcode & 0x7) as usize;
        let opmode = (opcode >> 3) & 0x1F;

        match opmode {
            0b01000 => {
                // Data register to data register
                let temp = registers.d(rx);
                registers.set_d(rx, registers.d(ry));
                registers.set_d(ry, temp);
            }
            0b01001 => {
                // Address register to address register
                let temp = registers.a(rx);
                registers.set_a(rx, registers.a(ry));
                registers.set_a(ry, temp);
            }
            0b10001 => {
                // Data register to address register
                let temp = registers.d(rx);
                registers.set_d(rx, registers.a(ry));
                registers.set_a(ry, temp);
            }
            _ => return Self::illegal(registers, _memory, opcode, pc),
        }

        InstructionResult::new(pc, 6)
    }

    /// NEGX - Negate with Extend.
    ///
    /// Negates destination with extend: 0 - dest - X -> dest.
    /// Zero flag only cleared if result is non-zero.
    ///
    /// Reference: m68k-instruction-set.txt - NEGX
    ///
    /// # Flags
    /// X: Set according to borrow
    /// N: Set if result is negative
    /// Z: Cleared if result is non-zero, unchanged otherwise
    /// V: Set if overflow
    /// C: Set if borrow
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn negx(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // NEGX encoding: 0100 0000 size ea
        let size_bits = (opcode >> 6) & 0x3;
        let size = match size_bits {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => return Self::illegal(registers, memory, opcode, pc),
        };

        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };
        let (ea, new_pc) = EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, pc);

        let operand = EaResolver::read_operand(ea, size, registers, memory);

        // Use current X flag value for the extend
        let extend = registers.get_x();

        // Save the current Z flag for special handling
        let old_z = registers.get_z();

        let result = Self::sub_with_borrow(0, operand, extend, size, registers, true);

        // NEGX special Z flag handling: only CLEAR Z if result is non-zero
        // If result is zero, Z remains unchanged (keep the old value)
        if result == 0 {
            registers.set_z(old_z);
        }
        // If result != 0, Z was already set to false by sub_with_borrow

        EaResolver::write_operand(ea, size, result, registers, memory);

        InstructionResult::new(new_pc, 4)
    }

    /// PEA - Push Effective Address.
    ///
    /// Calculates an effective address and pushes it onto the stack.
    ///
    /// Reference: m68k-instruction-set.txt - PEA
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn pea(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // PEA encoding: 0100 1000 01 ea
        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };
        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, ea_reg, OperandSize::Long, registers, memory, pc);

        // Get the effective address value
        let address = match ea {
            EffectiveAddress::Memory(addr) => addr,
            _ => return Self::illegal(registers, memory, opcode, pc),
        };

        // Push onto stack (predecrement SP, then write)
        let sp = registers.sp().wrapping_sub(4);
        registers.set_sp(sp);
        let _ = memory.write_long(sp, address);

        InstructionResult::new(new_pc, 12)
    }

    /// LINK - Link and Allocate.
    ///
    /// Creates a stack frame: pushes An onto stack, loads SP into An, then adds displacement to SP.
    ///
    /// Reference: m68k-instruction-set.txt - LINK
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn link(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // LINK encoding: 0100 1110 0101 0 An
        let an = (opcode & 0x7) as usize;

        // Read 16-bit displacement (sign extended)
        let displacement = i32::from(memory.read_word(pc).unwrap_or(0) as i16) as u32;

        // Push An onto stack
        let sp = registers.sp().wrapping_sub(4);
        registers.set_sp(sp);
        let _ = memory.write_long(sp, registers.a(an));

        // Load SP into An
        registers.set_a(an, sp);

        // Add displacement to SP
        let new_sp = sp.wrapping_add(displacement);
        registers.set_sp(new_sp);

        InstructionResult::new(pc + 2, 16)
    }

    /// UNLK - Unlink.
    ///
    /// Destroys stack frame: loads An into SP, then pops An from stack.
    ///
    /// Reference: m68k-instruction-set.txt - UNLK
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn unlk(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // UNLK encoding: 0100 1110 0101 1 An
        let an = (opcode & 0x7) as usize;

        // Load An into SP
        let sp = registers.a(an);
        registers.set_sp(sp);

        // Pop An from stack
        let value = memory.read_long(sp).unwrap_or(0);
        registers.set_a(an, value);
        registers.set_sp(sp.wrapping_add(4));

        InstructionResult::new(pc, 12)
    }

    /// TAS - Test and Set.
    ///
    /// Tests a byte operand (sets flags), then sets bit 7 of the operand.
    /// Atomic on real M68K hardware (important for multiprocessing).
    ///
    /// Reference: m68k-instruction-set.txt - TAS
    ///
    /// # Flags
    /// N: Set if bit 7 is set
    /// Z: Set if operand is zero
    /// V: Always cleared
    /// C: Always cleared
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn tas(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // TAS encoding: 0100 1010 11 ea
        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };
        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, ea_reg, OperandSize::Byte, registers, memory, pc);

        let value = EaResolver::read_operand(ea, OperandSize::Byte, registers, memory) as u8;

        // Set flags based on original value
        registers.set_n((value & 0x80) != 0);
        registers.set_z(value == 0);
        registers.set_v(false);
        registers.set_c(false);

        // Set bit 7
        let result = value | 0x80;
        EaResolver::write_operand(ea, OperandSize::Byte, u32::from(result), registers, memory);

        InstructionResult::new(new_pc, 4)
    }

    /// CMPM - Compare Memory to Memory.
    ///
    /// Compares memory at (Ay)+ to memory at (Ax)+ with postincrement.
    /// Sets flags based on (Ax)+ - (Ay)+.
    ///
    /// Reference: m68k-instruction-set.txt - CMPM
    ///
    /// # Flags
    /// X: Not affected
    /// N: Set if result is negative
    /// Z: Set if result is zero
    /// V: Set if overflow
    /// C: Set if borrow
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn cmpm(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // CMPM encoding: 1011 Ax 1 size 001 Ay
        let ax = ((opcode >> 9) & 0x7) as usize;
        let ay = (opcode & 0x7) as usize;
        let size_bits = (opcode >> 6) & 0x3;
        let size = match size_bits {
            0b00 => OperandSize::Byte,
            0b01 => OperandSize::Word,
            0b10 => OperandSize::Long,
            _ => return Self::illegal(registers, memory, opcode, pc),
        };

        // A7 (stack pointer) must always move by at least 2, even for byte operations
        let increment_y = if ay == 7 && size == OperandSize::Byte {
            2
        } else {
            match size {
                OperandSize::Byte => 1,
                OperandSize::Word => 2,
                OperandSize::Long => 4,
            }
        };
        let increment_x = if ax == 7 && size == OperandSize::Byte {
            2
        } else {
            match size {
                OperandSize::Byte => 1,
                OperandSize::Word => 2,
                OperandSize::Long => 4,
            }
        };

        // Read from (Ay)+ then (Ax)+
        let addr_y = registers.a(ay);
        let src = match size {
            OperandSize::Byte => u32::from(memory.read_byte(addr_y).unwrap_or(0)),
            OperandSize::Word => u32::from(memory.read_word(addr_y).unwrap_or(0)),
            OperandSize::Long => memory.read_long(addr_y).unwrap_or(0),
        };
        registers.set_a(ay, addr_y.wrapping_add(increment_y));

        let addr_x = registers.a(ax);
        let dst = match size {
            OperandSize::Byte => u32::from(memory.read_byte(addr_x).unwrap_or(0)),
            OperandSize::Word => u32::from(memory.read_word(addr_x).unwrap_or(0)),
            OperandSize::Long => memory.read_long(addr_x).unwrap_or(0),
        };
        registers.set_a(ax, addr_x.wrapping_add(increment_x));

        // Perform subtraction dst - src for flags - CMPM does not affect X
        let _ = Self::sub_with_borrow(dst, src, false, size, registers, false);

        InstructionResult::new(pc, 12)
    }

    /// ABCD - Add Binary-Coded Decimal with Extend.
    ///
    /// Adds two BCD digits with extend: src + dst + X -> dst.
    /// Zero flag only cleared if result is non-zero.
    ///
    /// Reference: m68k-instruction-set.txt - ABCD
    ///
    /// # Flags
    /// X: Set if decimal carry
    /// N: Undefined
    /// Z: Cleared if result is non-zero, unchanged otherwise
    /// V: Undefined
    /// C: Set if decimal carry
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn abcd(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // ABCD encoding: 1100 Dx 10000 R/M Dy
        let dx = ((opcode >> 9) & 0x7) as usize;
        let dy = (opcode & 0x7) as usize;
        let rm_bit = (opcode >> 3) & 0x1;

        let (src, dst, result_addr) = if rm_bit == 0 {
            // Register to register
            let s = registers.d(dy) as u8;
            let d = registers.d(dx) as u8;
            (s, d, None)
        } else {
            // Memory (predecrement) to memory
            // A7 must decrement by 2 for byte operations to maintain word alignment
            let dec_y = if dy == 7 { 2 } else { 1 };
            let addr_y = registers.a(dy).wrapping_sub(dec_y);
            registers.set_a(dy, addr_y);
            let s = memory.read_byte(addr_y).unwrap_or(0);

            let dec_x = if dx == 7 { 2 } else { 1 };
            let addr_x = registers.a(dx).wrapping_sub(dec_x);
            registers.set_a(dx, addr_x);
            let d = memory.read_byte(addr_x).unwrap_or(0);

            (s, d, Some(addr_x))
        };

        // Save old Z for special extend operation handling
        let old_z = registers.get_z();

        // BCD addition with extend using UAE-style algorithm
        let x = i32::from(registers.get_x());
        let src_i = i32::from(src);
        let dst_i = i32::from(dst);

        // Full binary addition
        let res = src_i + dst_i + x;
        let raw_msb = (res >> 7) & 1;

        // Low nibble addition (for half-carry detection)
        let lo = (src_i & 0xf) + (dst_i & 0xf) + x;

        // Apply BCD corrections
        let mut result = res;
        if lo > 9 {
            result += 6;
        }
        let carry = if result > 0x99 {
            result -= 0xa0;
            true
        } else {
            false
        };

        let result = (result & 0xFF) as u8;
        let result_msb = (result >> 7) & 1;

        // Write result
        if let Some(addr) = result_addr {
            let _ = memory.write_byte(addr, result);
        } else {
            registers.set_d(dx, (registers.d(dx) & 0xFFFF_FF00) | u32::from(result));
        }

        // Update flags
        // N is set based on MSB of final BCD result
        registers.set_n((result & 0x80) != 0);

        // V flag: Per M68K docs, V is "undefined" for BCD operations.
        // MAME's behavior: V is set if the MSB changed from 0 to 1 (positive to negative)
        // due to BCD correction. This is: raw_sum MSB == 0 AND result MSB == 1.
        registers.set_v(raw_msb == 0 && result_msb == 1);

        // Z is only cleared if result is non-zero, otherwise unchanged
        if result != 0 {
            registers.set_z(false);
        } else {
            registers.set_z(old_z);
        }

        // C and X are set if there was a decimal carry
        registers.set_c(carry);
        registers.set_x(carry);

        InstructionResult::new(pc, if rm_bit == 0 { 6 } else { 18 })
    }

    /// SBCD - Subtract Binary-Coded Decimal with Extend.
    ///
    /// Subtracts two BCD digits with extend: dst - src - X -> dst.
    /// Zero flag only cleared if result is non-zero.
    ///
    /// Reference: m68k-instruction-set.txt - SBCD
    ///
    /// # Flags
    /// X: Set if decimal borrow
    /// N: Undefined
    /// Z: Cleared if result is non-zero, unchanged otherwise
    /// V: Undefined
    /// C: Set if decimal borrow
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn sbcd(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // SBCD encoding: 1000 Dx 10000 R/M Dy
        let dx = ((opcode >> 9) & 0x7) as usize;
        let dy = (opcode & 0x7) as usize;
        let rm_bit = (opcode >> 3) & 0x1;

        let (src, dst, result_addr) = if rm_bit == 0 {
            // Register to register
            let s = registers.d(dy) as u8;
            let d = registers.d(dx) as u8;
            (s, d, None)
        } else {
            // Memory (predecrement) to memory
            // A7 must decrement by 2 for byte operations to maintain word alignment
            let dec_y = if dy == 7 { 2 } else { 1 };
            let addr_y = registers.a(dy).wrapping_sub(dec_y);
            registers.set_a(dy, addr_y);
            let s = memory.read_byte(addr_y).unwrap_or(0);

            let dec_x = if dx == 7 { 2 } else { 1 };
            let addr_x = registers.a(dx).wrapping_sub(dec_x);
            registers.set_a(dx, addr_x);
            let d = memory.read_byte(addr_x).unwrap_or(0);

            (s, d, Some(addr_x))
        };

        // Save old Z for special extend operation handling
        let old_z = registers.get_z();

        // BCD subtraction with extend: dst - src - X
        // The algorithm matches Musashi/real M68K behavior for both valid and invalid BCD.
        let x = i32::from(registers.get_x());
        let dst_i = i32::from(dst);
        let src_i = i32::from(src);

        // Full binary subtraction (raw, before BCD correction)
        let mut result = dst_i - src_i - x;
        let raw_msb = i32::from((result & 0x80) != 0);

        // Low nibble subtraction (for half-borrow detection)
        let lo = (dst_i & 0xf) - (src_i & 0xf) - x;

        // Low nibble adjustment: if borrowed OR result nibble > 9
        if (result & 0xF) > 9 || lo < 0 {
            result -= 6;
        }

        // High nibble correction: if high nibble > 9 OR result went negative
        // Borrow is set when high correction is needed
        let borrow = (result & 0xF0) > 0x90 || result < 0;
        if borrow {
            result -= 0x60;
        }

        let result = (result & 0xFF) as u8;
        let result_msb = (result >> 7) & 1;

        // Write result
        if let Some(addr) = result_addr {
            let _ = memory.write_byte(addr, result);
        } else {
            registers.set_d(dx, (registers.d(dx) & 0xFFFF_FF00) | u32::from(result));
        }

        // Update flags
        // N and V are officially "undefined" but real hardware (and MAME) has specific behavior
        registers.set_n((result & 0x80) != 0);

        // V flag: Per M68K docs, V is "undefined" for BCD operations.
        // MAME's behavior: V is set if the MSB changed from 1 to 0 (negative to positive)
        // due to BCD correction. This is: raw_sub MSB == 1 AND result MSB == 0.
        registers.set_v(raw_msb == 1 && result_msb == 0);

        // Z is only cleared if result is non-zero, otherwise unchanged
        if result != 0 {
            registers.set_z(false);
        } else {
            registers.set_z(old_z);
        }

        // C and X are set if there was a decimal borrow
        registers.set_c(borrow);
        registers.set_x(borrow);

        InstructionResult::new(pc, if rm_bit == 0 { 6 } else { 18 })
    }

    /// NBCD - Negate Binary-Coded Decimal with Extend.
    ///
    /// Negates a BCD value with extend: 0 - dst - X -> dst.
    /// Zero flag only cleared if result is non-zero.
    ///
    /// Reference: m68k-instruction-set.txt - NBCD
    ///
    /// # Flags
    /// X: Set if decimal borrow
    /// N: Undefined
    /// Z: Cleared if result is non-zero, unchanged otherwise
    /// V: Undefined
    /// C: Set if decimal borrow
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn nbcd(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // NBCD encoding: 0100 1000 00 ea
        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };
        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, ea_reg, OperandSize::Byte, registers, memory, pc);

        let dst = EaResolver::read_operand(ea, OperandSize::Byte, registers, memory) as u8;

        // Save old Z for special extend operation handling
        let old_z = registers.get_z();

        // NBCD: 0 - dst - X (ten's complement negation in BCD)
        // Same algorithm as SBCD with src=dst, dst=0
        let x = i32::from(registers.get_x());
        let dst_i = i32::from(dst);

        // Full binary subtraction from 0 (raw, before BCD correction)
        let mut result = 0 - dst_i - x;
        let raw_msb = i32::from((result & 0x80) != 0);

        // Low nibble subtraction (for half-borrow detection)
        let lo = 0 - (dst_i & 0xf) - x;

        // Low nibble adjustment: if borrowed OR result nibble > 9
        if (result & 0xF) > 9 || lo < 0 {
            result -= 6;
        }

        // High nibble correction: if high nibble > 9 OR result went negative
        // Borrow is set when high correction is needed
        let borrow = (result & 0xF0) > 0x90 || result < 0;
        if borrow {
            result -= 0x60;
        }

        let result = (result & 0xFF) as u8;
        let result_msb = (result >> 7) & 1;

        EaResolver::write_operand(ea, OperandSize::Byte, u32::from(result), registers, memory);

        // Update flags
        // N and V are officially "undefined" but real hardware (and MAME) has specific behavior
        registers.set_n((result & 0x80) != 0);

        // V flag: Per M68K docs, V is "undefined" for BCD operations.
        // MAME's behavior: V is set if the MSB changed from 1 to 0 (negative to positive)
        // due to BCD correction. This is: raw_sub MSB == 1 AND result MSB == 0.
        registers.set_v(raw_msb == 1 && result_msb == 0);

        // Z is only cleared if result is non-zero, otherwise unchanged
        if result != 0 {
            registers.set_z(false);
        } else {
            registers.set_z(old_z);
        }

        // C and X are set if there was a decimal borrow
        registers.set_c(borrow);
        registers.set_x(borrow);

        InstructionResult::new(new_pc, 6)
    }

    /// `DBcc` - Test Condition, Decrement and Branch.
    ///
    /// Tests condition; if false, decrements Dn and branches if Dn != -1.
    /// Used for counted loops.
    ///
    /// Reference: m68k-instruction-set.txt - `DBcc`
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn dbcc(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // DBcc encoding: 0101 cccc 11001 Dn
        // pc points to the displacement word
        let dn = (opcode & 0x7) as usize;
        let condition = ((opcode >> 8) & 0x0F) as u8;

        // Read 16-bit displacement (from current pc location)
        let displacement = i32::from(memory.read_word(pc).unwrap_or(0) as i16);

        // Test condition
        if Self::test_condition(condition, registers) {
            // Condition true - don't branch, don't decrement
            // Skip past the displacement word
            InstructionResult::new(pc + 2, 12)
        } else {
            // Condition false - decrement and test
            let count = (registers.d(dn) as u16).wrapping_sub(1);
            // Only modify the low word of the register
            let current = registers.d(dn);
            registers.set_d(dn, (current & 0xFFFF_0000) | u32::from(count));

            if count == 0xFFFF {
                // Counter expired (-1) - don't branch
                // Skip past the displacement word
                InstructionResult::new(pc + 2, 14)
            } else {
                // Branch - displacement is from the displacement word address
                let target = (pc as i32 + displacement) as u32;
                InstructionResult::new(target, 10)
            }
        }
    }

    /// Scc - Set According to Condition.
    ///
    /// Tests condition; if true, sets all bits of destination byte to 1, otherwise clears to 0.
    ///
    /// Reference: m68k-instruction-set.txt - Scc
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn scc(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Scc encoding: 0101 cccc 11 ea
        let condition = ((opcode >> 8) & 0x0F) as u8;
        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };
        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, ea_reg, OperandSize::Byte, registers, memory, pc);

        let result = if Self::test_condition(condition, registers) {
            0xFF
        } else {
            0x00
        };

        EaResolver::write_operand(ea, OperandSize::Byte, result, registers, memory);

        InstructionResult::new(new_pc, 4)
    }

    /// MOVEM - Move Multiple Registers.
    ///
    /// Moves multiple registers to/from memory using a register mask.
    ///
    /// Reference: m68k-instruction-set.txt - MOVEM
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn movem(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // MOVEM encoding: 0100 1 d 00 1 sz ea, followed by 16-bit register mask
        let direction = (opcode >> 10) & 0x1; // 0 = registers to memory, 1 = memory to registers
        let size = if (opcode >> 6) & 0x1 == 1 {
            OperandSize::Long
        } else {
            OperandSize::Word
        };
        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        // Read register mask
        let mask = match memory.read_word(pc) {
            Ok(m) => m,
            Err(_) => return Self::illegal(registers, memory, opcode, pc),
        };
        let new_pc = pc + 2;

        let is_predecrement = ea_mode == 4; // Predecrement mode -(An)
        let is_postincrement = ea_mode == 3; // Postincrement mode (An)+
        let increment = if size == OperandSize::Word { 2 } else { 4 };

        // For predecrement and postincrement, we handle address register updates specially
        // and don't use the normal EA resolver which would modify the register
        let (mut addr, final_pc) = if is_predecrement || is_postincrement {
            // Just get the current address register value without modifying it
            (registers.a(ea_reg as usize), new_pc)
        } else {
            let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
                Some(am) => am,
                None => return Self::illegal(registers, memory, opcode, pc),
            };
            let (ea, final_pc) =
                EaResolver::resolve(addr_mode, ea_reg, size, registers, memory, new_pc);
            match ea {
                EffectiveAddress::Memory(a) => (a, final_pc),
                _ => return Self::illegal(registers, memory, opcode, pc),
            }
        };

        if direction == 0 {
            // Registers to memory
            if is_predecrement {
                // For predecrement mode, the register mask is reversed:
                // Bits 0-7 = A7...A0, Bits 8-15 = D7...D0
                // And we process in reverse order (A7 first, then A6, ..., then D7, D6, ..., D0)
                for i in 0..8 {
                    // Bit i corresponds to A(7-i)
                    if (mask & (1 << i)) != 0 {
                        addr = addr.wrapping_sub(increment);
                        let reg_idx = 7 - i;
                        let value = registers.a(reg_idx);
                        if size == OperandSize::Word {
                            let _ = memory.write_word(addr, value as u16);
                        } else {
                            let _ = memory.write_long(addr, value);
                        }
                    }
                }
                for i in 0..8 {
                    // Bit (8+i) corresponds to D(7-i)
                    if (mask & (1 << (8 + i))) != 0 {
                        addr = addr.wrapping_sub(increment);
                        let reg_idx = 7 - i;
                        let value = registers.d(reg_idx);
                        if size == OperandSize::Word {
                            let _ = memory.write_word(addr, value as u16);
                        } else {
                            let _ = memory.write_long(addr, value);
                        }
                    }
                }
                // Update the address register for predecrement mode
                registers.set_a(ea_reg as usize, addr);
            } else {
                // Normal order: D0...D7, A0...A7
                for i in 0..8 {
                    if (mask & (1 << i)) != 0 {
                        let value = registers.d(i);
                        if size == OperandSize::Word {
                            let _ = memory.write_word(addr, value as u16);
                        } else {
                            let _ = memory.write_long(addr, value);
                        }
                        addr = addr.wrapping_add(increment);
                    }
                }
                for i in 0..8 {
                    if (mask & (1 << (8 + i))) != 0 {
                        let value = registers.a(i);
                        if size == OperandSize::Word {
                            let _ = memory.write_word(addr, value as u16);
                        } else {
                            let _ = memory.write_long(addr, value);
                        }
                        addr = addr.wrapping_add(increment);
                    }
                }
            }
        } else {
            // Memory to registers (always in order: D0...D7, A0...A7)
            for i in 0..8 {
                if (mask & (1 << i)) != 0 {
                    let value = if size == OperandSize::Word {
                        i32::from(memory.read_word(addr).unwrap_or(0) as i16) as u32
                    } else {
                        memory.read_long(addr).unwrap_or(0)
                    };
                    registers.set_d(i, value);
                    addr = addr.wrapping_add(increment);
                }
            }
            for i in 0..8 {
                if (mask & (1 << (8 + i))) != 0 {
                    let value = if size == OperandSize::Word {
                        i32::from(memory.read_word(addr).unwrap_or(0) as i16) as u32
                    } else {
                        memory.read_long(addr).unwrap_or(0)
                    };
                    registers.set_a(i, value);
                    addr = addr.wrapping_add(increment);
                }
            }
            // Update address register for postincrement mode
            if is_postincrement {
                registers.set_a(ea_reg as usize, addr);
            }
        }

        InstructionResult::new(final_pc, 12) // Approximate timing
    }

    /// CHK - Check Register Against Bounds.
    ///
    /// Checks if Dn is less than 0 or greater than upper bound.
    /// Triggers exception if out of bounds.
    ///
    /// Reference: m68k-instruction-set.txt - CHK
    ///
    /// # Flags
    /// N: Set if Dn < 0, cleared if Dn > source, undefined otherwise
    /// Z, V, C: Undefined
    /// X: Not affected
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn chk(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // CHK encoding: 0100 Dn 110 ea (size is always word)
        let dn = ((opcode >> 9) & 0x7) as usize;
        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };
        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, ea_reg, OperandSize::Word, registers, memory, pc);

        let upper_bound = EaResolver::read_operand(ea, OperandSize::Word, registers, memory) as i16;
        let value = registers.d(dn) as i16;

        if value < 0 {
            registers.set_n(true);
            // Trigger CHK exception (vector 6)
            InstructionResult::with_exception(new_pc, 40, 6)
        } else if value > upper_bound {
            registers.set_n(false);
            // Trigger CHK exception (vector 6)
            InstructionResult::with_exception(new_pc, 40, 6)
        } else {
            // Within bounds - continue
            InstructionResult::new(new_pc, 10)
        }
    }

    /// TRAP - Trap.
    ///
    /// Triggers a trap exception with vector number 32 + #vector (0-15).
    ///
    /// Reference: m68k-instruction-set.txt - TRAP
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn trap(
        _registers: &RegisterFile,
        _memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // TRAP encoding: 0100 1110 0100 vector
        let vector = (opcode & 0x0F) as u8;

        // Trigger TRAP exception (vector 32 + vector)
        InstructionResult::with_exception(pc, 34, 32 + vector)
    }

    /// TRAPV - Trap on Overflow.
    ///
    /// Triggers a trap if the V flag is set.
    ///
    /// Reference: m68k-instruction-set.txt - TRAPV
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn trapv(
        registers: &RegisterFile,
        _memory: &Memory,
        _opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        if registers.get_v() {
            // Trigger TRAPV exception (vector 7)
            InstructionResult::with_exception(pc, 34, 7)
        } else {
            InstructionResult::new(pc, 4)
        }
    }

    /// RTR - Return and Restore Condition Codes.
    ///
    /// Pops CCR from stack, then pops PC from stack.
    ///
    /// Reference: m68k-instruction-set.txt - RTR
    ///
    /// # Flags
    /// All flags restored from stack.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn rtr(
        registers: &mut RegisterFile,
        memory: &Memory,
        _opcode: u16,
        _pc: u32,
    ) -> InstructionResult {
        // Pop CCR (word)
        let sp = registers.sp();
        let ccr = memory.read_word(sp).unwrap_or(0);
        registers.set_sp(sp.wrapping_add(2));

        // Restore flags from CCR (lower byte)
        registers.set_ccr(CcrFlags::from_sr(ccr));

        // Pop PC (long)
        let sp = registers.sp();
        let return_addr = memory.read_long(sp).unwrap_or(0);
        registers.set_sp(sp.wrapping_add(4));

        InstructionResult::new(return_addr, 20)
    }

    /// RTE - Return from Exception.
    ///
    /// Privileged instruction: pops SR from stack, then pops PC from stack.
    /// Triggers privilege violation if in user mode.
    ///
    /// Reference: m68k-instruction-set.txt - RTE
    ///
    /// # Flags
    /// All flags restored from stack.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn rte(
        registers: &mut RegisterFile,
        memory: &Memory,
        _opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Check for privilege violation (must be in supervisor mode)
        if (registers.sr & 0x2000) == 0 {
            return InstructionResult::with_exception(pc - 2, 34, 8); // Privilege violation, vector 8
        }

        // Pop SR (word) from supervisor stack
        let sp = registers.sp();
        let sr = memory.read_word(sp).unwrap_or(0);
        registers.set_sp(sp.wrapping_add(2));

        // Pop PC (long) from supervisor stack
        let sp = registers.sp();
        let return_addr = memory.read_long(sp).unwrap_or(0);
        registers.set_sp(sp.wrapping_add(4));

        // Restore the full SR (this will handle mode switching if S bit changes)
        registers.set_sr(sr);

        InstructionResult::new(return_addr, 20)
    }

    /// STOP - Stop and Wait.
    ///
    /// Privileged instruction: loads immediate value into SR and halts the processor.
    /// Triggers privilege violation if in user mode.
    ///
    /// Reference: m68k-instruction-set.txt - STOP
    ///
    /// # Flags
    /// All flags set according to immediate value.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn stop(
        registers: &mut RegisterFile,
        memory: &Memory,
        _opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Check for privilege violation (must be in supervisor mode)
        if (registers.sr & 0x2000) == 0 {
            return InstructionResult::with_exception(pc - 2, 34, 8); // Privilege violation, vector 8
        }

        // STOP encoding: 0100 1110 0111 0010, followed by immediate word
        let sr_value = memory.read_word(pc).unwrap_or(0);

        // Load the full SR value (handles mode switching)
        registers.set_sr(sr_value);

        // Processor halts until an interrupt occurs with priority > new IPL
        // PC advances past the immediate word
        InstructionResult::with_halt(pc + 2, 4)
    }

    /// RESET - Reset External Devices.
    ///
    /// Privileged instruction: asserts RESET line to reset external devices.
    /// Triggers privilege violation if in user mode.
    ///
    /// Reference: m68k-instruction-set.txt - RESET
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn reset(
        registers: &RegisterFile,
        _memory: &Memory,
        _opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Check for privilege violation (must be in supervisor mode)
        if (registers.sr & 0x2000) == 0 {
            return InstructionResult::with_exception(pc - 2, 34, 8); // Privilege violation, vector 8
        }

        // RESET encoding: 0100 1110 0111 0000
        // In emulator, this is a no-op (would reset peripherals on real hardware)
        InstructionResult::new(pc, 132)
    }

    /// ANDI to CCR - AND Immediate to Condition Code Register.
    ///
    /// ANDs immediate byte with CCR.
    ///
    /// Reference: m68k-instruction-set.txt - ANDI to CCR
    ///
    /// # Flags
    /// All flags `ANDed` with immediate value.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn andi_to_ccr(
        registers: &mut RegisterFile,
        memory: &Memory,
        _opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // ANDI to CCR encoding: 0000 0010 0011 1100, followed by immediate byte (as word)
        let imm = memory.read_word(pc).unwrap_or(0) as u8;

        let ccr = registers.get_ccr();
        let ccr_byte = ccr.to_sr() as u8;
        let new_ccr_byte = ccr_byte & imm;
        registers.set_ccr(CcrFlags::from_sr(u16::from(new_ccr_byte)));

        InstructionResult::new(pc + 2, 20)
    }

    /// EORI to CCR - Exclusive OR Immediate to Condition Code Register.
    ///
    /// XORs immediate byte with CCR.
    ///
    /// Reference: m68k-instruction-set.txt - EORI to CCR
    ///
    /// # Flags
    /// All flags `XORed` with immediate value.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn eori_to_ccr(
        registers: &mut RegisterFile,
        memory: &Memory,
        _opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // EORI to CCR encoding: 0000 1010 0011 1100, followed by immediate byte (as word)
        let imm = memory.read_word(pc).unwrap_or(0) as u8;

        let ccr = registers.get_ccr();
        let ccr_byte = ccr.to_sr() as u8;
        let new_ccr_byte = ccr_byte ^ imm;
        registers.set_ccr(CcrFlags::from_sr(u16::from(new_ccr_byte)));

        InstructionResult::new(pc + 2, 20)
    }

    /// ORI to CCR - OR Immediate to Condition Code Register.
    ///
    /// ORs immediate byte with CCR.
    ///
    /// Reference: m68k-instruction-set.txt - ORI to CCR
    ///
    /// # Flags
    /// All flags `ORed` with immediate value.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn ori_to_ccr(
        registers: &mut RegisterFile,
        memory: &Memory,
        _opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // ORI to CCR encoding: 0000 0000 0011 1100, followed by immediate byte (as word)
        let imm = memory.read_word(pc).unwrap_or(0) as u8;

        let ccr = registers.get_ccr();
        let ccr_byte = ccr.to_sr() as u8;
        let new_ccr_byte = ccr_byte | imm;
        registers.set_ccr(CcrFlags::from_sr(u16::from(new_ccr_byte)));

        InstructionResult::new(pc + 2, 20)
    }

    /// ANDI to SR - AND Immediate to Status Register.
    ///
    /// ANDs immediate word with SR (full 16-bit status register).
    /// Privileged instruction - triggers privilege violation if in user mode.
    ///
    /// Reference: m68k-instruction-set.txt - ANDI to SR
    ///
    /// # Flags
    /// All bits `ANDed` with immediate value.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn andi_to_sr(
        registers: &mut RegisterFile,
        memory: &Memory,
        _opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Check for privilege violation (must be in supervisor mode)
        if (registers.sr & 0x2000) == 0 {
            return InstructionResult::with_exception(pc - 2, 34, 8); // Privilege violation, vector 8
        }

        // ANDI to SR encoding: 0000 0010 0111 1100, followed by immediate word
        let imm = memory.read_word(pc).unwrap_or(0);

        let sr = registers.sr;
        let new_sr = sr & imm;
        registers.set_sr(new_sr);

        InstructionResult::new(pc + 2, 20)
    }

    /// EORI to SR - Exclusive OR Immediate to Status Register.
    ///
    /// XORs immediate word with SR (full 16-bit status register).
    /// Privileged instruction - triggers privilege violation if in user mode.
    ///
    /// Reference: m68k-instruction-set.txt - EORI to SR
    ///
    /// # Flags
    /// All bits `XORed` with immediate value.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn eori_to_sr(
        registers: &mut RegisterFile,
        memory: &Memory,
        _opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Check for privilege violation (must be in supervisor mode)
        if (registers.sr & 0x2000) == 0 {
            return InstructionResult::with_exception(pc - 2, 34, 8); // Privilege violation, vector 8
        }

        // EORI to SR encoding: 0000 1010 0111 1100, followed by immediate word
        let imm = memory.read_word(pc).unwrap_or(0);

        let sr = registers.sr;
        let new_sr = sr ^ imm;
        registers.set_sr(new_sr);

        InstructionResult::new(pc + 2, 20)
    }

    /// ORI to SR - OR Immediate to Status Register.
    ///
    /// ORs immediate word with SR (full 16-bit status register).
    /// Privileged instruction - triggers privilege violation if in user mode.
    ///
    /// Reference: m68k-instruction-set.txt - ORI to SR
    ///
    /// # Flags
    /// All bits `ORed` with immediate value.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn ori_to_sr(
        registers: &mut RegisterFile,
        memory: &Memory,
        _opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Check for privilege violation (must be in supervisor mode)
        if (registers.sr & 0x2000) == 0 {
            return InstructionResult::with_exception(pc - 2, 34, 8); // Privilege violation, vector 8
        }

        // ORI to SR encoding: 0000 0000 0111 1100, followed by immediate word
        let imm = memory.read_word(pc).unwrap_or(0);

        let sr = registers.sr;
        let new_sr = sr | imm;
        registers.set_sr(new_sr);

        InstructionResult::new(pc + 2, 20)
    }

    /// MOVE from SR - Move from Status Register.
    ///
    /// Copies SR to destination. Privileged in 68010+, not in 68000.
    ///
    /// Reference: m68k-instruction-set.txt - MOVE from SR
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn move_from_sr(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // MOVE from SR encoding: 0100 0000 11 ea
        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };
        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, ea_reg, OperandSize::Word, registers, memory, pc);

        // Read SR and mask reserved bits
        // Valid bits: 0-4 (CCR flags), 8-10 (interrupt mask), 13 (supervisor)
        // Reserved bits 5-7 and 11 always read as 0
        let sr = u32::from(registers.sr & 0x271F);

        EaResolver::write_operand(ea, OperandSize::Word, sr, registers, memory);

        InstructionResult::new(new_pc, 6)
    }

    /// MOVE to CCR - Move to Condition Code Register.
    ///
    /// Copies source to CCR (user mode accessible).
    ///
    /// Reference: m68k-instruction-set.txt - MOVE to CCR
    ///
    /// # Flags
    /// All flags set according to source.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn move_to_ccr(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // MOVE to CCR encoding: 0100 0100 11 ea
        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };
        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, ea_reg, OperandSize::Word, registers, memory, pc);

        let value = EaResolver::read_operand(ea, OperandSize::Word, registers, memory) as u16;
        registers.set_ccr(CcrFlags::from_sr(value));

        InstructionResult::new(new_pc, 12)
    }

    /// MOVE to SR - Move to Status Register.
    ///
    /// Privileged instruction: copies source to SR.
    /// Triggers privilege violation if in user mode.
    ///
    /// Reference: m68k-instruction-set.txt - MOVE to SR
    ///
    /// # Flags
    /// All flags set according to source.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn move_to_sr(
        registers: &mut RegisterFile,
        memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Check for privilege violation (must be in supervisor mode)
        if (registers.sr & 0x2000) == 0 {
            return InstructionResult::with_exception(pc - 2, 34, 8); // Privilege violation, vector 8
        }

        // MOVE to SR encoding: 0100 0110 11 ea
        let ea_mode = ((opcode >> 3) & 0x7) as u8;
        let ea_reg = (opcode & 0x7) as u8;

        let addr_mode = match AddressingMode::from_mode_reg(ea_mode, ea_reg) {
            Some(am) => am,
            None => return Self::illegal(registers, memory, opcode, pc),
        };
        let (ea, new_pc) =
            EaResolver::resolve(addr_mode, ea_reg, OperandSize::Word, registers, memory, pc);

        let value = EaResolver::read_operand(ea, OperandSize::Word, registers, memory) as u16;
        // Set the full SR value (16-bit)
        registers.set_sr(value);

        InstructionResult::new(new_pc, 12)
    }

    /// MOVE USP - Move to/from User Stack Pointer.
    ///
    /// Privileged instruction to move between an address register and USP.
    /// Triggers privilege violation if in user mode.
    /// MOVE An, USP: stores An into USP
    /// MOVE USP, An: loads USP into An
    ///
    /// Reference: m68k-instruction-set.txt - MOVE USP
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn move_usp(
        registers: &mut RegisterFile,
        _memory: &Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // Check for privilege violation (must be in supervisor mode)
        if (registers.sr & 0x2000) == 0 {
            return InstructionResult::with_exception(pc - 2, 34, 8); // Privilege violation, vector 8
        }

        // MOVE USP encoding: 0100 1110 0110 drrr
        // Bit 3: 0 = MOVE An, USP; 1 = MOVE USP, An
        // Bits 2-0: An register number
        let reg = (opcode & 0x7) as usize;
        let to_usp = (opcode & 0x8) == 0;

        if to_usp {
            // MOVE An, USP
            let value = registers.a(reg);
            registers.set_usp(value);
        } else {
            // MOVE USP, An
            let value = registers.usp();
            registers.set_a(reg, value);
        }

        InstructionResult::new(pc, 4)
    }

    /// MOVEP - Move Peripheral Data.
    ///
    /// Transfers data between a data register and alternate bytes in memory.
    /// Used for 8-bit peripherals connected to a 16-bit data bus.
    ///
    /// Reference: m68k-instruction-set.txt - MOVEP
    ///
    /// # Flags
    /// None affected.
    /// Edge cases: None beyond standard M68K addressing and size rules.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn movep(
        registers: &mut RegisterFile,
        memory: &mut Memory,
        opcode: u16,
        pc: u32,
    ) -> InstructionResult {
        // MOVEP encoding: 0000 rrr ooo 001 aaa
        // rrr = data register
        // ooo = opmode: 100 = word mem->reg, 101 = long mem->reg
        //               110 = word reg->mem, 111 = long reg->mem
        // aaa = address register
        let dn = ((opcode >> 9) & 0x7) as usize;
        let an = (opcode & 0x7) as usize;
        let opmode = (opcode >> 6) & 0x7;

        // Read 16-bit displacement (sign-extended)
        let displacement = i32::from(memory.read_word(pc).unwrap_or(0) as i16);
        let new_pc = pc + 2;

        let base_addr = (registers.a(an) as i32).wrapping_add(displacement) as u32;

        match opmode {
            0b100 => {
                // Memory to register, word
                let b0 = u32::from(memory.read_byte(base_addr).unwrap_or(0));
                let b1 = u32::from(memory.read_byte(base_addr.wrapping_add(2)).unwrap_or(0));
                let value = (b0 << 8) | b1;
                // Only modify low word of register
                registers.set_d(dn, (registers.d(dn) & 0xFFFF_0000) | value);
            }
            0b101 => {
                // Memory to register, long
                let b0 = u32::from(memory.read_byte(base_addr).unwrap_or(0));
                let b1 = u32::from(memory.read_byte(base_addr.wrapping_add(2)).unwrap_or(0));
                let b2 = u32::from(memory.read_byte(base_addr.wrapping_add(4)).unwrap_or(0));
                let b3 = u32::from(memory.read_byte(base_addr.wrapping_add(6)).unwrap_or(0));
                let value = (b0 << 24) | (b1 << 16) | (b2 << 8) | b3;
                registers.set_d(dn, value);
            }
            0b110 => {
                // Register to memory, word
                let value = registers.d(dn);
                let _ = memory.write_byte(base_addr, ((value >> 8) & 0xFF) as u8);
                let _ = memory.write_byte(base_addr.wrapping_add(2), (value & 0xFF) as u8);
            }
            0b111 => {
                // Register to memory, long
                let value = registers.d(dn);
                let _ = memory.write_byte(base_addr, ((value >> 24) & 0xFF) as u8);
                let _ = memory.write_byte(base_addr.wrapping_add(2), ((value >> 16) & 0xFF) as u8);
                let _ = memory.write_byte(base_addr.wrapping_add(4), ((value >> 8) & 0xFF) as u8);
                let _ = memory.write_byte(base_addr.wrapping_add(6), (value & 0xFF) as u8);
            }
            _ => return Self::illegal(registers, memory, opcode, pc),
        }

        InstructionResult::new(new_pc, 16)
    }
}
