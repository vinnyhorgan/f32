//! M68K Register File and Status Register
//!
//! This module defines the complete M68K register file:
//! - Data registers D0-D7 (general-purpose, 32-bit)
//! - Address registers A0-A7 (32-bit, A7 is the stack pointer)
//! - Program Counter (PC, 32-bit)
//! - Status Register (SR, 16-bit)
//!
//! It also provides helpers for manipulating the condition code flags
//! (Negative, Zero, Overflow, Carry) and other status bits.

use std::fmt;

/// The M68K register file.
///
/// Contains all user-visible registers: data registers, address registers,
/// the program counter, and the status register.
///
/// The M68K has two stack pointers: USP (user) and SSP (supervisor).
/// A7 refers to one or the other depending on the S bit in SR:
///
/// - S=1 (supervisor mode): A7 = SSP
/// - S=0 (user mode): A7 = USP
///
/// Internally we track both and swap A7 when the mode changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisterFile {
    /// Data registers D0-D7 (32-bit each)
    pub d: [u32; 8],
    /// Address registers A0-A7 (32-bit each)
    /// Note: A7 serves as the stack pointer (SP), which is either USP or SSP
    pub a: [u32; 8],
    /// Program Counter (32-bit)
    pub pc: u32,
    /// Status Register (16-bit)
    pub sr: u16,
    /// User Stack Pointer (USP) - stored here when in supervisor mode
    usp: u32,
    /// Supervisor Stack Pointer (SSP) - stored here when in user mode
    ssp: u32,
}

impl Default for RegisterFile {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterFile {
    /// Creates a new register file with all registers initialized to zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            d: [0; 8],
            a: [0; 8],
            pc: 0,
            sr: 0,
            usp: 0,
            ssp: 0,
        }
    }

    /// Reads a data register (D0-D7).
    ///
    /// # Panics
    /// Panics if `reg` is >= 8.
    #[must_use]
    #[inline]
    pub const fn d(&self, reg: usize) -> u32 {
        self.d[reg]
    }

    /// Writes to a data register (D0-D7).
    ///
    /// # Panics
    /// Panics if `reg` is >= 8.
    #[inline]
    pub const fn set_d(&mut self, reg: usize, value: u32) {
        self.d[reg] = value;
    }

    /// Reads an address register (A0-A7).
    ///
    /// # Panics
    /// Panics if `reg` is >= 8.
    #[must_use]
    #[inline]
    pub const fn a(&self, reg: usize) -> u32 {
        self.a[reg]
    }

    /// Writes to an address register (A0-A7).
    ///
    /// # Panics
    /// Panics if `reg` is >= 8.
    #[inline]
    pub const fn set_a(&mut self, reg: usize, value: u32) {
        self.a[reg] = value;
    }

    /// Reads the stack pointer (A7).
    #[must_use]
    #[inline]
    pub const fn sp(&self) -> u32 {
        self.a[7]
    }

    /// Writes to the stack pointer (A7).
    #[inline]
    pub const fn set_sp(&mut self, value: u32) {
        self.a[7] = value;
    }

    /// Reads the user stack pointer (USP).
    /// Returns the actual USP regardless of current mode.
    #[must_use]
    #[inline]
    pub const fn usp(&self) -> u32 {
        let supervisor = (self.sr & 0x2000) != 0;
        if supervisor {
            self.usp // In supervisor mode, USP is stored separately
        } else {
            self.a[7] // In user mode, A7 is USP
        }
    }

    /// Writes to the user stack pointer (USP).
    /// Sets the actual USP regardless of current mode.
    #[inline]
    pub const fn set_usp(&mut self, value: u32) {
        let supervisor = (self.sr & 0x2000) != 0;
        if supervisor {
            self.usp = value; // In supervisor mode, USP is stored separately
        } else {
            self.a[7] = value; // In user mode, A7 is USP
        }
    }

    /// Reads the program counter.
    #[must_use]
    #[inline]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn pc(&self) -> u32 {
        self.pc
    }

    /// Writes to the program counter.
    #[inline]
    pub const fn set_pc(&mut self, value: u32) {
        self.pc = value;
    }

    /// Reads the status register.
    #[must_use]
    #[inline]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn sr(&self) -> u16 {
        self.sr
    }

    /// Writes to the status register.
    ///
    /// This properly handles mode switching between supervisor and user mode.
    /// When the S bit (bit 13) changes:
    /// - Supervisor -> User: save current A7 to SSP (internally stored), load USP into A7
    /// - User -> Supervisor: save current A7 to USP, load SSP into A7
    #[inline]
    pub const fn set_sr(&mut self, value: u16) {
        let old_supervisor = (self.sr & 0x2000) != 0;
        let new_supervisor = (value & 0x2000) != 0;

        if old_supervisor != new_supervisor {
            if old_supervisor {
                // Transitioning from supervisor to user mode
                // Save current A7 (SSP) to internal storage, load USP into A7
                self.ssp = self.a[7];
                self.a[7] = self.usp;
            } else {
                // Transitioning from user to supervisor mode
                // Save current A7 (USP) to internal storage, load SSP into A7
                self.usp = self.a[7];
                self.a[7] = self.ssp;
            }
        }

        self.sr = value;
    }

    /// Gets the SSP (supervisor stack pointer) value.
    /// Returns the actual SSP regardless of current mode.
    #[must_use]
    #[inline]
    pub const fn get_ssp(&self) -> u32 {
        let supervisor = (self.sr & 0x2000) != 0;
        if supervisor {
            self.a[7] // In supervisor mode, A7 is SSP
        } else {
            self.ssp // In user mode, SSP is stored separately
        }
    }
}

impl fmt::Display for RegisterFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Data Registers:")?;
        for i in 0..8 {
            writeln!(f, "  D{} = 0x{:08X}", i, self.d[i])?;
        }
        writeln!(f, "Address Registers:")?;
        for i in 0..7 {
            writeln!(f, "  A{} = 0x{:08X}", i, self.a[i])?;
        }
        writeln!(f, "  A7 (SP) = 0x{:08X}", self.a[7])?;
        writeln!(f, "PC = 0x{:08X}", self.pc)?;
        writeln!(f, "SR = 0x{:04X}", self.sr)?;
        Ok(())
    }
}

/// Condition code flags in the Status Register.
///
/// These are the lower 5 bits of the SR (bits 0-4 of the byte
/// accessed in user mode).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// Allow dead code: kept for tests, completeness, or CLI-only usage.
#[allow(dead_code)]
pub struct CcrFlags {
    /// Carry flag (bit 0)
    pub c: bool,
    /// Overflow flag (bit 1)
    pub v: bool,
    /// Zero flag (bit 2)
    pub z: bool,
    /// Negative flag (bit 3)
    pub n: bool,
    /// Extended flag (bit 4)
    pub x: bool,
}

impl CcrFlags {
    /// Creates a new `CcrFlags` with all flags cleared.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn new() -> Self {
        Self {
            c: false,
            v: false,
            z: false,
            n: false,
            x: false,
        }
    }

    /// Extracts CCR flags from a 16-bit status register value.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn from_sr(sr: u16) -> Self {
        Self {
            c: (sr & 0x0001) != 0,
            v: (sr & 0x0002) != 0,
            z: (sr & 0x0004) != 0,
            n: (sr & 0x0008) != 0,
            x: (sr & 0x0010) != 0,
        }
    }

    /// Converts CCR flags to a 16-bit status register value.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn to_sr(self) -> u16 {
        let mut result = 0u16;
        if self.c {
            result |= 0x0001;
        }
        if self.v {
            result |= 0x0002;
        }
        if self.z {
            result |= 0x0004;
        }
        if self.n {
            result |= 0x0008;
        }
        if self.x {
            result |= 0x0010;
        }
        result
    }
}

impl Default for CcrFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for easy flag manipulation on `RegisterFile`.
// Allow dead code: kept for tests, completeness, or CLI-only usage.
#[allow(dead_code)]
pub trait FlagOps {
    /// Gets the current condition code flags.
    fn get_ccr(&self) -> CcrFlags;

    /// Sets condition code flags from a `CcrFlags` struct.
    fn set_ccr(&mut self, flags: CcrFlags);

    /// Gets the carry flag.
    fn get_c(&self) -> bool;

    /// Sets the carry flag.
    fn set_c(&mut self, value: bool);

    /// Gets the overflow flag.
    fn get_v(&self) -> bool;

    /// Sets the overflow flag.
    fn set_v(&mut self, value: bool);

    /// Gets the zero flag.
    fn get_z(&self) -> bool;

    /// Sets the zero flag.
    fn set_z(&mut self, value: bool);

    /// Gets the negative flag.
    fn get_n(&self) -> bool;

    /// Sets the negative flag.
    fn set_n(&mut self, value: bool);

    /// Gets the extend flag.
    fn get_x(&self) -> bool;

    /// Sets the extend flag.
    fn set_x(&mut self, value: bool);

    /// Clears all condition code flags.
    fn clear_flags(&mut self);

    /// Sets flags based on the result of an arithmetic operation.
    ///
    /// This is the standard flag-setting logic for most integer operations:
    /// - N: set if the result is negative (MSB set)
    /// - Z: set if the result is zero
    /// - V: set if signed overflow occurred
    /// - C: set if unsigned overflow occurred
    /// - X: set to same value as C (for most operations)
    fn set_flags_arith(&mut self, result: u32, overflow: bool, carry: bool);

    /// Sets flags based on the result of a logical operation.
    ///
    /// Logical operations (AND, OR, EOR, NOT) only affect N and Z.
    /// V and C are always cleared.
    fn set_flags_logical(&mut self, result: u32);

    /// Sets flags for a comparison operation (CMP, TST).
    ///
    /// Same as arithmetic but doesn't set X flag.
    fn set_flags_compare(&mut self, result: u32, overflow: bool, carry: bool);
}

impl FlagOps for RegisterFile {
    #[inline]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    fn get_ccr(&self) -> CcrFlags {
        CcrFlags::from_sr(self.sr)
    }

    #[inline]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    fn set_ccr(&mut self, flags: CcrFlags) {
        // Preserve upper byte of SR, set lower byte from flags
        self.sr = (self.sr & 0xFF00) | flags.to_sr();
    }

    #[inline]
    fn get_c(&self) -> bool {
        (self.sr & 0x0001) != 0
    }

    #[inline]
    fn set_c(&mut self, value: bool) {
        if value {
            self.sr |= 0x0001;
        } else {
            self.sr &= 0xFFFE;
        }
    }

    #[inline]
    fn get_v(&self) -> bool {
        (self.sr & 0x0002) != 0
    }

    #[inline]
    fn set_v(&mut self, value: bool) {
        if value {
            self.sr |= 0x0002;
        } else {
            self.sr &= 0xFFFD;
        }
    }

    #[inline]
    fn get_z(&self) -> bool {
        (self.sr & 0x0004) != 0
    }

    #[inline]
    fn set_z(&mut self, value: bool) {
        if value {
            self.sr |= 0x0004;
        } else {
            self.sr &= 0xFFFB;
        }
    }

    #[inline]
    fn get_n(&self) -> bool {
        (self.sr & 0x0008) != 0
    }

    #[inline]
    fn set_n(&mut self, value: bool) {
        if value {
            self.sr |= 0x0008;
        } else {
            self.sr &= 0xFFF7;
        }
    }

    #[inline]
    fn get_x(&self) -> bool {
        (self.sr & 0x0010) != 0
    }

    #[inline]
    fn set_x(&mut self, value: bool) {
        if value {
            self.sr |= 0x0010;
        } else {
            self.sr &= 0xFFEF;
        }
    }

    #[inline]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    fn clear_flags(&mut self) {
        self.sr &= 0xFFE0;
    }

    #[inline]
    fn set_flags_arith(&mut self, result: u32, overflow: bool, carry: bool) {
        self.set_n((result as i32) < 0);
        self.set_z(result == 0);
        self.set_v(overflow);
        self.set_c(carry);
        self.set_x(carry); // X follows C for most arithmetic operations
    }

    #[inline]
    fn set_flags_logical(&mut self, result: u32) {
        self.set_n((result as i32) < 0);
        self.set_z(result == 0);
        self.set_v(false);
        self.set_c(false);
        // X is unchanged for logical operations
    }

    #[inline]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    fn set_flags_compare(&mut self, result: u32, overflow: bool, carry: bool) {
        self.set_n((result as i32) < 0);
        self.set_z(result == 0);
        self.set_v(overflow);
        self.set_c(carry);
        // X is unchanged for compare operations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_file_new() {
        let rf = RegisterFile::new();
        assert_eq!(rf.d, [0; 8]);
        assert_eq!(rf.a, [0; 8]);
        assert_eq!(rf.pc, 0);
        assert_eq!(rf.sr, 0);
    }

    #[test]
    fn test_data_register_access() {
        let mut rf = RegisterFile::new();
        rf.set_d(0, 0xDEADBEEF);
        assert_eq!(rf.d(0), 0xDEADBEEF);

        rf.set_d(7, 0x12345678);
        assert_eq!(rf.d(7), 0x12345678);
    }

    #[test]
    fn test_address_register_access() {
        let mut rf = RegisterFile::new();
        rf.set_a(0, 0x00001000);
        assert_eq!(rf.a(0), 0x00001000);

        rf.set_a(7, 0x00002000);
        assert_eq!(rf.a(7), 0x00002000);
        assert_eq!(rf.sp(), 0x00002000);
    }

    #[test]
    fn test_pc_access() {
        let mut rf = RegisterFile::new();
        rf.set_pc(0x00001000);
        assert_eq!(rf.pc(), 0x00001000);
    }

    #[test]
    fn test_sr_access() {
        let mut rf = RegisterFile::new();
        rf.set_sr(0x2700); // Supervisor mode, all flags cleared
        assert_eq!(rf.sr(), 0x2700);
    }

    #[test]
    fn test_ccr_flags_new() {
        let flags = CcrFlags::new();
        assert!(!flags.c);
        assert!(!flags.v);
        assert!(!flags.z);
        assert!(!flags.n);
        assert!(!flags.x);
    }

    #[test]
    fn test_ccr_flags_from_sr() {
        let sr = 0x001F; // All flags set
        let flags = CcrFlags::from_sr(sr);
        assert!(flags.c);
        assert!(flags.v);
        assert!(flags.z);
        assert!(flags.n);
        assert!(flags.x);
    }

    #[test]
    fn test_ccr_flags_to_sr() {
        let flags = CcrFlags {
            c: true,
            v: false,
            z: true,
            n: false,
            x: true,
        };
        let sr = flags.to_sr();
        assert_eq!(sr, 0x0015); // Bits 0, 2, 4 set
    }

    #[test]
    fn test_flag_ops_get_set() {
        let mut rf = RegisterFile::new();

        rf.set_c(true);
        assert!(rf.get_c());

        rf.set_v(true);
        assert!(rf.get_v());

        rf.set_z(true);
        assert!(rf.get_z());

        rf.set_n(true);
        assert!(rf.get_n());

        rf.set_x(true);
        assert!(rf.get_x());
    }

    #[test]
    fn test_flag_ops_clear() {
        let mut rf = RegisterFile::new();
        rf.sr = 0x001F; // All flags set
        rf.clear_flags();
        assert_eq!(rf.sr & 0x1F, 0);
    }

    #[test]
    fn test_set_flags_arith() {
        let mut rf = RegisterFile::new();

        // Test positive result
        rf.set_flags_arith(0x00000001, false, false);
        assert!(!rf.get_n());
        assert!(!rf.get_z());
        assert!(!rf.get_v());
        assert!(!rf.get_c());
        assert!(!rf.get_x());

        // Test zero result
        rf.set_flags_arith(0x00000000, false, false);
        assert!(!rf.get_n());
        assert!(rf.get_z());

        // Test negative result
        rf.set_flags_arith(0xFFFFFFFF, false, false);
        assert!(rf.get_n());
        assert!(!rf.get_z());

        // Test overflow and carry
        rf.set_flags_arith(0x80000000, true, true);
        assert!(rf.get_n());
        assert!(!rf.get_z());
        assert!(rf.get_v());
        assert!(rf.get_c());
        assert!(rf.get_x());
    }

    #[test]
    fn test_set_flags_logical() {
        let mut rf = RegisterFile::new();

        rf.set_flags_logical(0x00000000);
        assert!(!rf.get_n());
        assert!(rf.get_z());
        assert!(!rf.get_v());
        assert!(!rf.get_c());

        rf.set_flags_logical(0xFFFFFFFF);
        assert!(rf.get_n());
        assert!(!rf.get_z());
        assert!(!rf.get_v());
        assert!(!rf.get_c());
    }

    #[test]
    fn test_set_flags_compare() {
        let mut rf = RegisterFile::new();

        rf.set_flags_compare(0x00000000, false, false);
        assert!(!rf.get_n());
        assert!(rf.get_z());
        assert!(!rf.get_v());
        assert!(!rf.get_c());
    }

    #[test]
    fn test_ccr_roundtrip() {
        let flags = CcrFlags {
            c: true,
            v: true,
            z: false,
            n: false,
            x: true,
        };
        let sr = flags.to_sr();
        let flags2 = CcrFlags::from_sr(sr);
        assert_eq!(flags.c, flags2.c);
        assert_eq!(flags.v, flags2.v);
        assert_eq!(flags.z, flags2.z);
        assert_eq!(flags.n, flags2.n);
        assert_eq!(flags.x, flags2.x);
    }
}
