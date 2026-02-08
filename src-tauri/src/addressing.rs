//! M68K Addressing Modes
//!
//! This module implements all M68K addressing modes. The M68K has a rich
//! set of addressing modes that make assembly programming flexible and
//! expressive.
//!
//! The addressing modes are:
//! - Data Register Direct
//! - Address Register Direct
//! - Address Register Indirect
//! - Address Register Indirect with Postincrement
//! - Address Register Indirect with Predecrement
//! - Address Register Indirect with Displacement
//! - Address Register Indirect with Index (8-bit displacement)
//! - Absolute Short Addressing
//! - Absolute Long Addressing
//! - Immediate Data
//! - Program Counter Relative with Displacement
//! - Program Counter Relative with Index (8-bit displacement)
//!
//! Addressing modes are encoded in the instruction word using the mode
//! and register fields.

use crate::memory::Memory;
use crate::registers::RegisterFile;

/// Size of an operand in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperandSize {
    Byte = 1,
    Word = 2,
    Long = 4,
}

impl OperandSize {
    /// Returns the size in bytes.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn bytes(self) -> usize {
        self as usize
    }

    /// Returns the size as a u32.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }

    /// Returns the number of bits for this size.
    ///
    /// - Byte: 8
    /// - Word: 16
    /// - Long: 32
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn bits(self) -> usize {
        match self {
            Self::Byte => 8,
            Self::Word => 16,
            Self::Long => 32,
        }
    }

    /// Returns the sign bit mask for this size.
    ///
    /// - Byte: 0x80
    /// - Word: 0x8000
    /// - Long: 0x80000000
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn sign_bit(self) -> u32 {
        match self {
            Self::Byte => 0x80,
            Self::Word => 0x8000,
            Self::Long => 0x8000_0000,
        }
    }

    /// Returns the appropriate mask for this size.
    ///
    /// - Byte: 0xFF
    /// - Word: 0xFFFF
    /// - Long: 0xFFFFFFFF
    #[must_use]
    pub const fn mask(self) -> u32 {
        match self {
            Self::Byte => 0xFF,
            Self::Word => 0xFFFF,
            Self::Long => 0xFFFF_FFFF,
        }
    }

    /// Sign-extends a value to 32 bits based on the size.
    #[must_use]
    pub const fn sign_extend(self, value: u32) -> i32 {
        match self {
            Self::Byte => (value as i8) as i32,
            Self::Word => (value as i16) as i32,
            Self::Long => value as i32,
        }
    }
}

/// M68K addressing modes.
///
/// These represent the various ways to specify operands in M68K instructions.
/// Each mode encodes how the effective address is calculated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressingMode {
    /// Data register direct: Dn
    DataRegisterDirect,

    /// Address register direct: An
    AddressRegisterDirect,

    /// Address register indirect: (An)
    AddressRegisterIndirect,

    /// Address register indirect with postincrement: (An)+
    AddressRegisterIndirectPostincrement,

    /// Address register indirect with predecrement: -(An)
    AddressRegisterIndirectPredecrement,

    /// Address register indirect with displacement: d16(An)
    AddressRegisterIndirectWithDisplacement,

    /// Address register indirect with index: d8(An, Xn)
    AddressRegisterIndirectWithIndex,

    /// Absolute short addressing: (xxx).W
    AbsoluteShort,

    /// Absolute long addressing: (xxx).L
    AbsoluteLong,

    /// Immediate data: #data
    Immediate,

    /// Program counter relative with displacement: d16(PC)
    ProgramCounterRelativeWithDisplacement,

    /// Program counter relative with index: d8(PC, Xn)
    ProgramCounterRelativeWithIndex,
}

impl AddressingMode {
    /// Returns the addressing mode from the mode and register fields.
    ///
    /// The mode field is 3 bits, the register field is 3 bits.
    /// Together they encode the addressing mode.
    #[must_use]
    pub const fn from_mode_reg(mode: u8, reg: u8) -> Option<Self> {
        match (mode, reg) {
            // Mode 000: Data register direct
            (0b000, _) => Some(Self::DataRegisterDirect),

            // Mode 001: Address register direct
            (0b001, _) => Some(Self::AddressRegisterDirect),

            // Mode 010: Address register indirect
            (0b010, _) => Some(Self::AddressRegisterIndirect),

            // Mode 011: Address register indirect with postincrement
            (0b011, _) => Some(Self::AddressRegisterIndirectPostincrement),

            // Mode 100: Address register indirect with predecrement
            (0b100, _) => Some(Self::AddressRegisterIndirectPredecrement),

            // Mode 101: Address register indirect with displacement
            (0b101, _) => Some(Self::AddressRegisterIndirectWithDisplacement),

            // Mode 110: Address register indirect with index
            (0b110, _) => Some(Self::AddressRegisterIndirectWithIndex),

            // Mode 111: Depends on register field
            (0b111, 0b000) => Some(Self::AbsoluteShort),
            (0b111, 0b001) => Some(Self::AbsoluteLong),
            (0b111, 0b100) => Some(Self::Immediate),
            (0b111, 0b010) => Some(Self::ProgramCounterRelativeWithDisplacement),
            (0b111, 0b011) => Some(Self::ProgramCounterRelativeWithIndex),
            (0b111, _) => None, // Other modes are not valid
            _ => None,
        }
    }

    /// Returns the mode and register fields for this addressing mode.
    ///
    /// This is used for encoding instructions.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    // Allow clippy::wrong_self_convention: encoding helpers read better as mode/register builders.
    #[allow(clippy::wrong_self_convention)]
    pub const fn to_mode_reg(&self, reg: u8) -> Option<(u8, u8)> {
        match self {
            Self::DataRegisterDirect => Some((0b000, reg)),
            Self::AddressRegisterDirect => Some((0b001, reg)),
            Self::AddressRegisterIndirect => Some((0b010, reg)),
            Self::AddressRegisterIndirectPostincrement => Some((0b011, reg)),
            Self::AddressRegisterIndirectPredecrement => Some((0b100, reg)),
            Self::AddressRegisterIndirectWithDisplacement => Some((0b101, reg)),
            Self::AddressRegisterIndirectWithIndex => Some((0b110, reg)),
            Self::AbsoluteShort => Some((0b111, 0b000)),
            Self::AbsoluteLong => Some((0b111, 0b001)),
            Self::Immediate => Some((0b111, 0b100)),
            Self::ProgramCounterRelativeWithDisplacement => Some((0b111, 0b010)),
            Self::ProgramCounterRelativeWithIndex => Some((0b111, 0b011)),
        }
    }
}

/// Extension word format for indexed addressing modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexExtension {
    /// Whether the index register is an address register (true) or data register (false).
    pub is_address_register: bool,
    /// The index register number (0-7).
    pub register: u8,
    /// The size of the index register (Word or Long).
    pub size: IndexSize,
    /// The 8-bit displacement (sign-extended).
    pub displacement: i8,
}

/// Size of the index register in indexed addressing modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexSize {
    Word,
    Long,
}

/// Represents the effective address of an operand.
///
/// This is the result of resolving an addressing mode to an actual
/// value or memory location.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectiveAddress {
    /// Register direct - the value is in a register.
    DataRegister(u8),
    AddressRegister(u8),

    /// Memory address - the value is in memory at this address.
    Memory(u32),

    /// Immediate value.
    Immediate(u32),
}

impl EffectiveAddress {
    /// Returns true if this is a register direct address.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn is_register_direct(&self) -> bool {
        matches!(self, Self::DataRegister(_) | Self::AddressRegister(_))
    }

    /// Returns true if this is a memory address.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn is_memory(&self) -> bool {
        matches!(self, Self::Memory(_))
    }

    /// Returns true if this is an immediate value.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn is_immediate(&self) -> bool {
        matches!(self, Self::Immediate(_))
    }
}

/// The effective address resolver.
///
/// This struct resolves addressing modes to effective addresses using the
/// current CPU state (registers and memory). It advances the PC as needed
/// when fetching extension words.
pub struct EaResolver {
    // This is a stateless resolver - all state comes from the CPU
}

impl EaResolver {
    /// Resolves an addressing mode to an effective address.
    ///
    /// # Arguments
    /// * `mode` - The addressing mode to resolve
    /// * `reg` - The register number (0-7)
    /// * `size` - The operand size
    /// * `registers` - The CPU register file
    /// * `memory` - The memory bus
    /// * `pc` - The current program counter (will be updated)
    ///
    /// # Returns
    /// The effective address and the updated PC.
    ///
    /// # Panics
    /// Panics if the addressing mode is invalid or the register number is out of range.
    #[must_use]
    pub fn resolve(
        mode: AddressingMode,
        reg: u8,
        size: OperandSize,
        registers: &mut RegisterFile,
        memory: &Memory,
        mut pc: u32,
    ) -> (EffectiveAddress, u32) {
        match mode {
            AddressingMode::DataRegisterDirect => (EffectiveAddress::DataRegister(reg), pc),

            AddressingMode::AddressRegisterDirect => (EffectiveAddress::AddressRegister(reg), pc),

            AddressingMode::AddressRegisterIndirect => {
                let addr = registers.a(reg as usize);
                (EffectiveAddress::Memory(addr), pc)
            }

            AddressingMode::AddressRegisterIndirectPostincrement => {
                let addr = registers.a(reg as usize);
                // A7 (stack pointer) must always move by at least 2, even for byte operations
                let increment = if reg == 7 && size == OperandSize::Byte {
                    2
                } else {
                    size.as_u32()
                };
                let new_addr = addr.wrapping_add(increment);
                registers.set_a(reg as usize, new_addr);
                (EffectiveAddress::Memory(addr), pc)
            }

            AddressingMode::AddressRegisterIndirectPredecrement => {
                // A7 (stack pointer) must always move by at least 2, even for byte operations
                let decrement = if reg == 7 && size == OperandSize::Byte {
                    2
                } else {
                    size.as_u32()
                };
                let addr = registers.a(reg as usize).wrapping_sub(decrement);
                registers.set_a(reg as usize, addr);
                (EffectiveAddress::Memory(addr), pc)
            }

            AddressingMode::AddressRegisterIndirectWithDisplacement => {
                // Fetch the displacement word (sign-extended)
                let disp = memory.read_word_unchecked(pc) as i16;
                pc += 2;

                let base = registers.a(reg as usize);
                let addr = (base as i32).wrapping_add(i32::from(disp)) as u32;
                (EffectiveAddress::Memory(addr), pc)
            }

            AddressingMode::AddressRegisterIndirectWithIndex => {
                // Fetch the extension word
                let ext = memory.read_word_unchecked(pc);
                pc += 2;

                let index_ext = Self::parse_index_extension(ext);

                // Calculate the effective address
                let base = registers.a(reg as usize);
                let index_val = if index_ext.is_address_register {
                    registers.a(index_ext.register as usize)
                } else {
                    registers.d(index_ext.register as usize)
                };

                // Sign-extend or truncate index based on size
                let index = match index_ext.size {
                    IndexSize::Word => i32::from(index_val as i16),
                    IndexSize::Long => index_val as i32,
                };

                let displacement = i32::from(index_ext.displacement);
                let addr = (base as i32).wrapping_add(index).wrapping_add(displacement) as u32;

                (EffectiveAddress::Memory(addr), pc)
            }

            AddressingMode::AbsoluteShort => {
                // Fetch the absolute address (sign-extended)
                let addr = i32::from(memory.read_word_unchecked(pc) as i16) as u32;
                pc += 2;
                (EffectiveAddress::Memory(addr), pc)
            }

            AddressingMode::AbsoluteLong => {
                // Fetch the absolute long address
                let addr = memory.read_long_unchecked(pc);
                pc += 4;
                (EffectiveAddress::Memory(addr), pc)
            }

            AddressingMode::Immediate => {
                // Fetch the immediate data
                let value = match size {
                    OperandSize::Byte => {
                        // Immediate data is always word-sized for bytes, MSB is ignored
                        let data = u32::from(memory.read_word_unchecked(pc));
                        pc += 2;
                        data & 0xFF
                    }
                    OperandSize::Word => {
                        let data = u32::from(memory.read_word_unchecked(pc));
                        pc += 2;
                        data
                    }
                    OperandSize::Long => {
                        let data = memory.read_long_unchecked(pc);
                        pc += 4;
                        data
                    }
                };
                (EffectiveAddress::Immediate(value), pc)
            }

            AddressingMode::ProgramCounterRelativeWithDisplacement => {
                // Fetch the displacement word (sign-extended)
                let disp = memory.read_word_unchecked(pc) as i16;

                // Calculate address relative to PC at the displacement word
                // M68K semantics: displacement is relative to the PC pointing at the displacement word
                let addr = (pc as i32).wrapping_add(i32::from(disp)) as u32;

                // Advance PC past the displacement word
                pc += 2;

                (EffectiveAddress::Memory(addr), pc)
            }

            AddressingMode::ProgramCounterRelativeWithIndex => {
                // Fetch the extension word
                // For PC-relative modes, the base PC is the address of the extension word
                let base_pc = pc;
                let ext = memory.read_word_unchecked(pc);
                pc += 2;

                let index_ext = Self::parse_index_extension(ext);

                // Calculate the effective address
                // Base is the address of the extension word (before reading it)
                let base = base_pc as i32;

                let index_val = if index_ext.is_address_register {
                    registers.a(index_ext.register as usize)
                } else {
                    registers.d(index_ext.register as usize)
                };

                // Sign-extend or truncate index based on size
                let index = match index_ext.size {
                    IndexSize::Word => i32::from(index_val as i16),
                    IndexSize::Long => index_val as i32,
                };

                let displacement = i32::from(index_ext.displacement);
                let addr = base.wrapping_add(index).wrapping_add(displacement) as u32;

                (EffectiveAddress::Memory(addr), pc)
            }
        }
    }

    /// Parses an index extension word.
    #[must_use]
    const fn parse_index_extension(ext: u16) -> IndexExtension {
        // Bit 15: D/A (0 = data register, 1 = address register)
        let is_address_register = (ext & 0x8000) != 0;

        // Bits 12-11: Register number
        let register = ((ext >> 12) & 0x7) as u8;

        // Bit 11: Size (0 = Word, 1 = Long)
        let size = if (ext & 0x0800) != 0 {
            IndexSize::Long
        } else {
            IndexSize::Word
        };

        // Bits 0-7: Displacement (sign-extended)
        let displacement = (ext & 0xFF) as i8;

        IndexExtension {
            is_address_register,
            register,
            size,
            displacement,
        }
    }

    /// Reads an operand value from an effective address.
    ///
    /// # Arguments
    /// * `ea` - The effective address
    /// * `size` - The operand size
    /// * `registers` - The CPU register file
    /// * `memory` - The memory bus
    ///
    /// # Returns
    /// The operand value, sign-extended to 32 bits.
    ///
    /// # Panics
    /// Panics if the memory access fails or the register number is invalid.
    #[must_use]
    pub fn read_operand(
        ea: EffectiveAddress,
        size: OperandSize,
        registers: &RegisterFile,
        memory: &Memory,
    ) -> u32 {
        match ea {
            EffectiveAddress::DataRegister(reg) => {
                let value = registers.d(reg as usize);
                match size {
                    OperandSize::Byte => value & 0xFF,
                    OperandSize::Word => value & 0xFFFF,
                    OperandSize::Long => value,
                }
            }

            EffectiveAddress::AddressRegister(reg) => {
                let value = registers.a(reg as usize);
                match size {
                    OperandSize::Byte => value & 0xFF,
                    OperandSize::Word => value & 0xFFFF,
                    OperandSize::Long => value,
                }
            }

            EffectiveAddress::Memory(addr) => match size {
                OperandSize::Byte => u32::from(memory.read_byte(addr).unwrap_or(0)),
                OperandSize::Word => u32::from(memory.read_word(addr).unwrap_or(0)),
                OperandSize::Long => memory.read_long(addr).unwrap_or(0),
            },

            EffectiveAddress::Immediate(value) => value,
        }
    }

    /// Writes an operand value to an effective address.
    ///
    /// # Arguments
    /// * `ea` - The effective address
    /// * `size` - The operand size
    /// * `value` - The value to write
    /// * `registers` - The CPU register file
    /// * `memory` - The memory bus
    ///
    /// # Panics
    /// Panics if the memory access fails, the register number is invalid,
    /// or the effective address is not writable (e.g., immediate).
    pub fn write_operand(
        ea: EffectiveAddress,
        size: OperandSize,
        value: u32,
        registers: &mut RegisterFile,
        memory: &mut Memory,
    ) {
        match ea {
            EffectiveAddress::DataRegister(reg) => {
                let current = registers.d(reg as usize);
                let masked_value = value & size.mask();
                let new_value = match size {
                    OperandSize::Byte => (current & 0xFFFF_FF00) | masked_value,
                    OperandSize::Word => (current & 0xFFFF_0000) | masked_value,
                    OperandSize::Long => masked_value,
                };
                registers.set_d(reg as usize, new_value);
            }

            EffectiveAddress::AddressRegister(reg) => {
                let current = registers.a(reg as usize);
                let masked_value = value & size.mask();
                let new_value = match size {
                    OperandSize::Byte => (current & 0xFFFF_FF00) | masked_value,
                    OperandSize::Word => (current & 0xFFFF_0000) | masked_value,
                    OperandSize::Long => masked_value,
                };
                registers.set_a(reg as usize, new_value);
            }

            EffectiveAddress::Memory(addr) => {
                // Silently ignore write errors (they may be to MMIO regions or out-of-bounds)
                // The MMIO hook will be called by the memory module before returning an error
                match size {
                    OperandSize::Byte => {
                        let _ = memory.write_byte(addr, value as u8);
                    }
                    OperandSize::Word => {
                        let _ = memory.write_word(addr, value as u16);
                    }
                    OperandSize::Long => {
                        let _ = memory.write_long(addr, value);
                    }
                }
            }

            EffectiveAddress::Immediate(_) => {
                // Silently ignore writes to immediate values (invalid operation)
                // This can happen with buggy code or during test scenarios
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operand_size_bytes() {
        assert_eq!(OperandSize::Byte.bytes(), 1);
        assert_eq!(OperandSize::Word.bytes(), 2);
        assert_eq!(OperandSize::Long.bytes(), 4);
    }

    #[test]
    fn test_operand_size_mask() {
        assert_eq!(OperandSize::Byte.mask(), 0xFF);
        assert_eq!(OperandSize::Word.mask(), 0xFFFF);
        assert_eq!(OperandSize::Long.mask(), 0xFFFFFFFF);
    }

    #[test]
    fn test_operand_size_sign_extend() {
        assert_eq!(OperandSize::Byte.sign_extend(0xFF), -1i32);
        assert_eq!(OperandSize::Word.sign_extend(0xFFFF), -1i32);
        assert_eq!(OperandSize::Long.sign_extend(0xFFFFFFFF), -1i32);
    }

    #[test]
    fn test_addressing_mode_from_mode_reg() {
        // Data register direct
        assert_eq!(
            AddressingMode::from_mode_reg(0b000, 0),
            Some(AddressingMode::DataRegisterDirect)
        );

        // Address register direct
        assert_eq!(
            AddressingMode::from_mode_reg(0b001, 0),
            Some(AddressingMode::AddressRegisterDirect)
        );

        // Address register indirect
        assert_eq!(
            AddressingMode::from_mode_reg(0b010, 0),
            Some(AddressingMode::AddressRegisterIndirect)
        );

        // Postincrement
        assert_eq!(
            AddressingMode::from_mode_reg(0b011, 0),
            Some(AddressingMode::AddressRegisterIndirectPostincrement)
        );

        // Predecrement
        assert_eq!(
            AddressingMode::from_mode_reg(0b100, 0),
            Some(AddressingMode::AddressRegisterIndirectPredecrement)
        );

        // Absolute short
        assert_eq!(
            AddressingMode::from_mode_reg(0b111, 0b000),
            Some(AddressingMode::AbsoluteShort)
        );

        // Absolute long
        assert_eq!(
            AddressingMode::from_mode_reg(0b111, 0b001),
            Some(AddressingMode::AbsoluteLong)
        );

        // Immediate
        assert_eq!(
            AddressingMode::from_mode_reg(0b111, 0b100),
            Some(AddressingMode::Immediate)
        );
    }

    #[test]
    fn test_parse_index_extension() {
        // Test case: D0 as word index with displacement 0x10
        let ext = 0x0010; // D/A=0, reg=0, size=0, disp=0x10
        let index = EaResolver::parse_index_extension(ext);
        assert!(!index.is_address_register);
        assert_eq!(index.register, 0);
        assert_eq!(index.size, IndexSize::Word);
        assert_eq!(index.displacement, 0x10);
    }

    #[test]
    fn test_ea_is_register_direct() {
        assert!(EffectiveAddress::DataRegister(0).is_register_direct());
        assert!(EffectiveAddress::AddressRegister(0).is_register_direct());
        assert!(!EffectiveAddress::Memory(0x1000).is_register_direct());
        assert!(!EffectiveAddress::Immediate(0x1234).is_register_direct());
    }

    #[test]
    fn test_ea_is_memory() {
        assert!(EffectiveAddress::Memory(0x1000).is_memory());
        assert!(!EffectiveAddress::DataRegister(0).is_memory());
    }

    #[test]
    fn test_ea_is_immediate() {
        assert!(EffectiveAddress::Immediate(0x1234).is_immediate());
        assert!(!EffectiveAddress::DataRegister(0).is_immediate());
    }

    #[test]
    fn test_resolve_data_register_direct() {
        let mut registers = RegisterFile::new();
        registers.set_d(3, 0x12345678);
        let memory = Memory::new(1024);

        let (ea, pc) = EaResolver::resolve(
            AddressingMode::DataRegisterDirect,
            3,
            OperandSize::Long,
            &mut registers,
            &memory,
            0x100,
        );

        assert_eq!(ea, EffectiveAddress::DataRegister(3));
        assert_eq!(pc, 0x100); // PC unchanged
    }

    #[test]
    fn test_resolve_address_register_indirect() {
        let mut registers = RegisterFile::new();
        registers.set_a(5, 0x1000);
        let memory = Memory::new(0x2000);

        let (ea, pc) = EaResolver::resolve(
            AddressingMode::AddressRegisterIndirect,
            5,
            OperandSize::Long,
            &mut registers,
            &memory,
            0x100,
        );

        assert_eq!(ea, EffectiveAddress::Memory(0x1000));
        assert_eq!(pc, 0x100); // PC unchanged
    }

    #[test]
    fn test_resolve_immediate_word() {
        let mut registers = RegisterFile::new();
        let mut memory = Memory::new(1024);
        memory.write_word(0x100, 0xABCD).unwrap();

        let (ea, pc) = EaResolver::resolve(
            AddressingMode::Immediate,
            0,
            OperandSize::Word,
            &mut registers,
            &memory,
            0x100,
        );

        assert_eq!(ea, EffectiveAddress::Immediate(0xABCD));
        assert_eq!(pc, 0x102); // PC advanced by 2
    }

    #[test]
    fn test_read_operand_data_register() {
        let mut registers = RegisterFile::new();
        registers.set_d(2, 0x12345678);
        let memory = Memory::new(1024);

        let ea = EffectiveAddress::DataRegister(2);

        assert_eq!(
            EaResolver::read_operand(ea, OperandSize::Long, &registers, &memory),
            0x12345678
        );
        assert_eq!(
            EaResolver::read_operand(ea, OperandSize::Word, &registers, &memory),
            0x5678
        );
        assert_eq!(
            EaResolver::read_operand(ea, OperandSize::Byte, &registers, &memory),
            0x78
        );
    }

    #[test]
    fn test_read_operand_memory() {
        let registers = RegisterFile::new();
        let mut memory = Memory::new(1024);
        memory.write_long(0x100, 0x12345678).unwrap();

        let ea = EffectiveAddress::Memory(0x100);

        assert_eq!(
            EaResolver::read_operand(ea, OperandSize::Long, &registers, &memory),
            0x12345678
        );
        assert_eq!(
            EaResolver::read_operand(ea, OperandSize::Word, &registers, &memory),
            0x1234
        );
        assert_eq!(
            EaResolver::read_operand(ea, OperandSize::Byte, &registers, &memory),
            0x12
        );
    }

    #[test]
    fn test_write_operand_data_register() {
        let mut registers = RegisterFile::new();
        let mut memory = Memory::new(1024);

        // Write byte
        let ea = EffectiveAddress::DataRegister(0);
        EaResolver::write_operand(ea, OperandSize::Byte, 0xAB, &mut registers, &mut memory);
        assert_eq!(registers.d(0), 0xAB);

        // Write word (should preserve upper bits)
        EaResolver::write_operand(ea, OperandSize::Word, 0x1234, &mut registers, &mut memory);
        assert_eq!(registers.d(0), 0x1234);

        // Write long
        EaResolver::write_operand(
            ea,
            OperandSize::Long,
            0x12345678,
            &mut registers,
            &mut memory,
        );
        assert_eq!(registers.d(0), 0x12345678);
    }

    #[test]
    fn test_write_operand_memory() {
        let mut registers = RegisterFile::new();
        let mut memory = Memory::new(1024);

        let ea = EffectiveAddress::Memory(0x100);
        EaResolver::write_operand(
            ea,
            OperandSize::Long,
            0x12345678,
            &mut registers,
            &mut memory,
        );

        assert_eq!(memory.read_long(0x100).unwrap(), 0x12345678);
    }

    #[test]
    fn test_resolve_postincrement() {
        let mut registers = RegisterFile::new();
        registers.set_a(3, 0x1000);
        let memory = Memory::new(0x2000);

        let (ea, _pc) = EaResolver::resolve(
            AddressingMode::AddressRegisterIndirectPostincrement,
            3,
            OperandSize::Long,
            &mut registers,
            &memory,
            0x100,
        );

        assert_eq!(ea, EffectiveAddress::Memory(0x1000));
        assert_eq!(registers.a(3), 0x1004); // Incremented by 4
    }

    #[test]
    fn test_resolve_predecrement() {
        let mut registers = RegisterFile::new();
        registers.set_a(3, 0x1000);
        let memory = Memory::new(0x2000);

        let (ea, _pc) = EaResolver::resolve(
            AddressingMode::AddressRegisterIndirectPredecrement,
            3,
            OperandSize::Long,
            &mut registers,
            &memory,
            0x100,
        );

        assert_eq!(ea, EffectiveAddress::Memory(0x0FFC));
        assert_eq!(registers.a(3), 0x0FFC); // Decremented by 4
    }
}
