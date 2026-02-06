//! M68K Memory Model
//!
//! This module implements a flat memory model for the M68K.
//!
//! The M68K has a 24-bit address space (16MB maximum), but addresses are
//! manipulated as 32-bit values with the upper 8 bits ignored (or used for
//! address error detection in some implementations).
//!
//! This implementation uses a simple byte vector with bounds checking.
//! Bus errors are generated for out-of-bounds accesses.

use std::fmt;

/// Default memory size: 100KB for flux32
///
/// This provides ample space for educational programs while keeping the
/// memory view manageable.
pub const DEFAULT_MEMORY_SIZE: usize = 100 * 1024;

/// Maximum memory size: 16MB (full M68K address space)
pub const MAX_MEMORY_SIZE: usize = 16 * 1024 * 1024;

/// M68K uses 24-bit addresses (address bus is 24 bits wide)
/// All addresses must be masked to 24 bits before accessing memory
pub const ADDR_MASK: u32 = 0x00FFFFFF;

/// Error type for memory operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryError {
    /// Attempted to read or write outside the valid address range.
    AddressOutOfRange { address: u32, size: usize },
}

impl std::error::Error for MemoryError {}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AddressOutOfRange { address, size } => write!(
                f,
                "Address out of range: 0x{:08X} (size: {} bytes)",
                address, size
            ),
        }
    }
}

/// Result of a write hook decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteHookResult {
    /// The hook handled the write; the memory array should not be modified.
    Handled,
    /// The hook did not handle the write; the memory array should be updated.
    Unhandled,
}

/// Callback function type for write hooks.
///
/// Returning `Handled` prevents the write from touching the backing memory.
pub type WriteHook = fn(address: u32, value: u32, size: OperandSize) -> WriteHookResult;

/// Callback function type for read hooks
pub type ReadHook = fn(address: u32) -> Option<u8>;

/// Operand size for write hooks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandSize {
    Byte,
    Word,
    Long,
}

/// The M68K memory bus.
///
/// Provides a flat address space with bounds checking and alignment checking.
/// All memory operations return a `Result` for proper error handling.
#[derive(Clone)]
pub struct Memory {
    /// The memory backing store.
    data: Vec<u8>,
    /// Size of the memory in bytes.
    size: usize,
    /// Write hook for memory-mapped I/O
    write_hook: Option<WriteHook>,
    /// Read hook for memory-mapped I/O
    read_hook: Option<ReadHook>,
}

impl Default for Memory {
    fn default() -> Self {
        Self::new(DEFAULT_MEMORY_SIZE)
    }
}

impl Memory {
    /// Creates a new memory with the specified size.
    ///
    /// # Panics
    /// Panics if `size` exceeds `MAX_MEMORY_SIZE`.
    #[must_use]
    pub fn new(size: usize) -> Self {
        assert!(
            size <= MAX_MEMORY_SIZE,
            "Memory size exceeds maximum of 16MB"
        );
        Self {
            data: vec![0; size],
            size,
            write_hook: None,
            read_hook: None,
        }
    }

    /// Sets a write hook for memory-mapped I/O.
    ///
    /// The hook will be called for all write operations. If the hook returns
    /// `WriteHookResult::Handled`, the write will not modify the backing memory.
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn set_write_hook(&mut self, hook: WriteHook) {
        self.write_hook = Some(hook);
    }

    /// Clears the write hook.
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn clear_write_hook(&mut self) {
        self.write_hook = None;
    }

    /// Sets a read hook for memory-mapped I/O.
    ///
    /// The hook will be called for all read operations. If the hook returns
    /// `Some(value)`, that value is used and memory is not accessed.
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn set_read_hook(&mut self, hook: ReadHook) {
        self.read_hook = Some(hook);
    }

    /// Clears the read hook.
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn clear_read_hook(&mut self) {
        self.read_hook = None;
    }

    /// Creates a new memory with the default size (64KB).
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn with_default_size() -> Self {
        Self::default()
    }

    /// Returns the size of the memory in bytes.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Checks if an address is valid for a given access size.
    ///
    /// Returns `Ok(())` if valid, `Err(MemoryError)` if not.
    fn check_bounds(&self, address: u32, size: usize) -> Result<(), MemoryError> {
        // M68K uses 24-bit addresses - mask to 24 bits
        let addr = (address & ADDR_MASK) as usize;
        if addr.saturating_add(size) > self.size {
            Err(MemoryError::AddressOutOfRange { address, size })
        } else {
            Ok(())
        }
    }

    /// Reads a single byte from memory.
    pub fn read_byte(&self, address: u32) -> Result<u8, MemoryError> {
        // Call read hook FIRST (for MMIO, even if address is out of bounds)
        if let Some(hook) = self.read_hook {
            if let Some(value) = hook(address) {
                return Ok(value);
            }
        }

        self.check_bounds(address, 1)?;
        // M68K uses 24-bit addresses - mask to 24 bits
        Ok(self.data[(address & ADDR_MASK) as usize])
    }

    /// Writes a single byte to memory.
    pub fn write_byte(&mut self, address: u32, value: u8) -> Result<(), MemoryError> {
        // Call write hook FIRST (for MMIO, even if address is out of bounds)
        if let Some(hook) = self.write_hook {
            if matches!(
                hook(address, value as u32, OperandSize::Byte),
                WriteHookResult::Handled
            ) {
                return Ok(());
            }
        }

        self.check_bounds(address, 1)?;
        // M68K uses 24-bit addresses - mask to 24 bits
        self.data[(address & ADDR_MASK) as usize] = value;
        Ok(())
    }

    /// Reads a word (16 bits) from memory.
    ///
    /// Words must be aligned to even addresses on the M68K.
    /// This emulator performs byte-wise reads for unaligned addresses instead
    /// of raising an address error, which keeps the core simple for now.
    /// The data is stored in big-endian format (Motorola convention).
    pub fn read_word(&self, address: u32) -> Result<u16, MemoryError> {
        if self.read_hook.is_some() {
            let high = self.read_byte(address)? as u16;
            let low = self.read_byte(address + 1)? as u16;
            return Ok((high << 8) | low);
        }

        // For unaligned accesses, do two byte reads
        if address & 1 != 0 {
            // Unaligned: read high byte at address, low byte at address+1
            let high = self.read_byte(address)? as u16;
            let low = self.read_byte(address + 1)? as u16;
            Ok((high << 8) | low)
        } else {
            // Aligned: fast path
            self.check_bounds(address, 2)?;
            // M68K uses 24-bit addresses - mask to 24 bits
            let addr = (address & ADDR_MASK) as usize;
            let high = self.data[addr] as u16;
            let low = self.data[addr + 1] as u16;
            Ok((high << 8) | low)
        }
    }

    /// Writes a word (16 bits) to memory.
    ///
    /// Words must be aligned to even addresses on the M68K.
    /// This emulator performs byte-wise writes for unaligned addresses instead
    /// of raising an address error, which keeps the core simple for now.
    /// The data is stored in big-endian format (Motorola convention).
    pub fn write_word(&mut self, address: u32, value: u16) -> Result<(), MemoryError> {
        // Call write hook FIRST (for MMIO, even if address is out of bounds)
        if let Some(hook) = self.write_hook {
            if matches!(
                hook(address, value as u32, OperandSize::Word),
                WriteHookResult::Handled
            ) {
                return Ok(());
            }
        }

        // For unaligned accesses, do two byte writes
        if address & 1 != 0 {
            self.check_bounds(address, 2)?;
            let addr = (address & ADDR_MASK) as usize;
            self.data[addr] = (value >> 8) as u8;
            self.data[addr + 1] = value as u8;
            Ok(())
        } else {
            // Aligned: fast path
            self.check_bounds(address, 2)?;
            // M68K uses 24-bit addresses - mask to 24 bits
            let addr = (address & ADDR_MASK) as usize;
            self.data[addr] = (value >> 8) as u8;
            self.data[addr + 1] = (value & 0xFF) as u8;
            Ok(())
        }
    }

    /// Reads a long word (32 bits) from memory.
    ///
    /// Long words must be aligned to even addresses (word-aligned) on the M68K.
    /// This emulator performs byte-wise reads for unaligned addresses instead
    /// of raising an address error, which keeps the core simple for now.
    /// The data is stored in big-endian format (Motorola convention).
    pub fn read_long(&self, address: u32) -> Result<u32, MemoryError> {
        if self.read_hook.is_some() {
            let b0 = self.read_byte(address)? as u32;
            let b1 = self.read_byte(address + 1)? as u32;
            let b2 = self.read_byte(address + 2)? as u32;
            let b3 = self.read_byte(address + 3)? as u32;
            return Ok((b0 << 24) | (b1 << 16) | (b2 << 8) | b3);
        }

        // For unaligned accesses (odd address), do byte + word + byte reads
        if address & 1 != 0 {
            let b0 = self.read_byte(address)? as u32;
            let w1 = self.read_word(address + 1)? as u32;
            let b3 = self.read_byte(address + 3)? as u32;
            Ok((b0 << 24) | (w1 << 8) | b3)
        } else {
            // Even address: can do aligned longword read
            self.check_bounds(address, 4)?;
            // M68K uses 24-bit addresses - mask to 24 bits
            let addr = (address & ADDR_MASK) as usize;
            let b0 = self.data[addr] as u32;
            let b1 = self.data[addr + 1] as u32;
            let b2 = self.data[addr + 2] as u32;
            let b3 = self.data[addr + 3] as u32;
            Ok((b0 << 24) | (b1 << 16) | (b2 << 8) | b3)
        }
    }

    /// Writes a long word (32 bits) to memory.
    ///
    /// Long words must be aligned to even addresses (word-aligned) on the M68K.
    /// This emulator performs byte-wise writes for unaligned addresses instead
    /// of raising an address error, which keeps the core simple for now.
    /// The data is stored in big-endian format (Motorola convention).
    pub fn write_long(&mut self, address: u32, value: u32) -> Result<(), MemoryError> {
        // Call write hook FIRST (for MMIO, even if address is out of bounds)
        if let Some(hook) = self.write_hook {
            if matches!(
                hook(address, value, OperandSize::Long),
                WriteHookResult::Handled
            ) {
                return Ok(());
            }
        }

        // For unaligned accesses (odd address), do byte + word + byte writes
        if address & 1 != 0 {
            self.check_bounds(address, 4)?;
            let addr = (address & ADDR_MASK) as usize;
            self.data[addr] = (value >> 24) as u8;
            self.data[addr + 1] = ((value >> 16) & 0xFF) as u8;
            self.data[addr + 2] = ((value >> 8) & 0xFF) as u8;
            self.data[addr + 3] = (value & 0xFF) as u8;
            Ok(())
        } else {
            // Even address: can do aligned longword write
            self.check_bounds(address, 4)?;
            // M68K uses 24-bit addresses - mask to 24 bits
            let addr = (address & ADDR_MASK) as usize;
            self.data[addr] = (value >> 24) as u8;
            self.data[addr + 1] = ((value >> 16) & 0xFF) as u8;
            self.data[addr + 2] = ((value >> 8) & 0xFF) as u8;
            self.data[addr + 3] = (value & 0xFF) as u8;
            Ok(())
        }
    }

    /// Reads a byte from memory without bounds checking.
    ///
    /// Used by instructions that need fast byte access.
    /// Returns 0 for out-of-bounds accesses.
    #[inline]
    pub(crate) fn read_byte_unchecked(&self, address: u32) -> u8 {
        let addr = (address & ADDR_MASK) as usize;
        if addr >= self.data.len() {
            return 0;
        }
        self.data[addr]
    }

    /// Reads an unaligned word from memory.
    ///
    /// Used by the instruction fetcher which may fetch at any address.
    /// Returns the word in big-endian format regardless of alignment.
    /// Returns 0 for out-of-bounds accesses.
    #[inline]
    pub(crate) fn read_word_unchecked(&self, address: u32) -> u16 {
        // M68K uses 24-bit addresses - mask to 24 bits
        let addr = (address & ADDR_MASK) as usize;
        if addr + 1 >= self.data.len() {
            return 0;
        }
        let high = self.data[addr] as u16;
        let low = self.data[addr + 1] as u16;
        (high << 8) | low
    }

    /// Reads an unaligned long word from memory.
    ///
    /// Used by the instruction fetcher which may fetch at any address.
    /// Returns the long word in big-endian format regardless of alignment.
    /// Returns 0 for out-of-bounds accesses.
    #[inline]
    pub(crate) fn read_long_unchecked(&self, address: u32) -> u32 {
        // M68K uses 24-bit addresses - mask to 24 bits
        let addr = (address & ADDR_MASK) as usize;
        if addr + 3 >= self.data.len() {
            return 0;
        }
        let b0 = self.data[addr] as u32;
        let b1 = self.data[addr + 1] as u32;
        let b2 = self.data[addr + 2] as u32;
        let b3 = self.data[addr + 3] as u32;
        (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
    }

    /// Loads a binary image into memory at the specified address.
    ///
    /// Returns the number of bytes written.
    pub fn load_binary(&mut self, address: u32, data: &[u8]) -> Result<usize, MemoryError> {
        // M68K uses 24-bit addresses - mask to 24 bits
        let start_addr = (address & ADDR_MASK) as usize;
        let end_addr = start_addr.saturating_add(data.len());

        if end_addr > self.size {
            return Err(MemoryError::AddressOutOfRange {
                address,
                size: data.len(),
            });
        }

        self.data[start_addr..end_addr].copy_from_slice(data);
        Ok(data.len())
    }

    /// Dumps memory as a hex string for debugging.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn dump_range(&self, start: u32, length: usize) -> String {
        let mut output = String::new();
        // M68K uses 24-bit addresses - mask to 24 bits
        let mut addr = (start & ADDR_MASK) as usize;
        let end = addr.saturating_add(length).min(self.size);

        while addr < end {
            // Print address
            output.push_str(&format!("{:08X}: ", addr));

            // Print hex bytes (16 bytes per line)
            for i in 0..16 {
                if addr + i < end {
                    output.push_str(&format!("{:02X} ", self.data[addr + i]));
                } else {
                    output.push_str("   ");
                }
                if i == 7 {
                    output.push(' ');
                }
            }

            // Print ASCII representation
            output.push_str(" |");
            for i in 0..16 {
                if addr + i < end {
                    let b = self.data[addr + i];
                    if b.is_ascii_graphic() || b == b' ' {
                        output.push(b as char);
                    } else {
                        output.push('.');
                    }
                }
            }
            output.push_str("|\n");

            addr += 16;
        }

        output
    }

    /// Clears all memory to zero.
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    /// Reads a range of bytes from memory into a Vec.
    ///
    /// Returns as many bytes as possible up to `length`, truncating at memory bounds.
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)] // Useful API for future bulk reads (e.g., disassembly, debugging)
    pub fn read_range(&self, start: u32, length: usize) -> Vec<u8> {
        if self.read_hook.is_some() {
            let mut out = Vec::with_capacity(length);
            for i in 0..length {
                let addr = start.wrapping_add(i as u32);
                if let Ok(byte) = self.read_byte(addr) {
                    out.push(byte);
                } else {
                    break;
                }
            }
            return out;
        }

        let end = start.saturating_add(length as u32);
        let actual_end = end.min(self.size as u32);

        let start_idx = start as usize;
        let end_idx = actual_end as usize;

        if start_idx >= self.data.len() || start_idx >= end_idx {
            return Vec::new();
        }

        self.data[start_idx..end_idx].to_vec()
    }
}

impl fmt::Debug for Memory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Memory").field("size", &self.size).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_new() {
        let mem = Memory::new(1024);
        assert_eq!(mem.size(), 1024);
    }

    #[test]
    fn test_memory_default() {
        let mem = Memory::default();
        assert_eq!(mem.size(), DEFAULT_MEMORY_SIZE);
    }

    #[test]
    fn test_read_write_byte() {
        let mut mem = Memory::new(1024);
        mem.write_byte(0x100, 0xAB).unwrap();
        assert_eq!(mem.read_byte(0x100).unwrap(), 0xAB);
    }

    #[test]
    fn test_read_write_word() {
        let mut mem = Memory::new(1024);
        mem.write_word(0x100, 0xABCD).unwrap();
        assert_eq!(mem.read_word(0x100).unwrap(), 0xABCD);
    }

    #[test]
    fn test_read_write_long() {
        let mut mem = Memory::new(1024);
        mem.write_long(0x100, 0x12345678).unwrap();
        assert_eq!(mem.read_long(0x100).unwrap(), 0x12345678);
    }

    #[test]
    fn test_endianness_word() {
        let mut mem = Memory::new(1024);
        mem.write_byte(0x100, 0xAA).unwrap();
        mem.write_byte(0x101, 0xBB).unwrap();
        assert_eq!(mem.read_word(0x100).unwrap(), 0xAABB);
    }

    #[test]
    fn test_endianness_long() {
        let mut mem = Memory::new(1024);
        mem.write_byte(0x100, 0x11).unwrap();
        mem.write_byte(0x101, 0x22).unwrap();
        mem.write_byte(0x102, 0x33).unwrap();
        mem.write_byte(0x103, 0x44).unwrap();
        assert_eq!(mem.read_long(0x100).unwrap(), 0x11223344);
    }

    #[test]
    fn test_unaligned_access() {
        let mut mem = Memory::new(1024);

        // M68K supports unaligned access in many cases
        // Write some test data first
        mem.write_byte(0x100, 0xAA).unwrap();
        mem.write_byte(0x101, 0xBB).unwrap();
        mem.write_byte(0x102, 0xCC).unwrap();
        mem.write_byte(0x103, 0xDD).unwrap();
        mem.write_byte(0x104, 0xEE).unwrap();

        // Unaligned word read should work (reads from 0x101 and 0x102)
        let result = mem.read_word(0x101).unwrap();
        assert_eq!(result, 0xBBCC);

        // Unaligned long read should work (reads from 0x101, 0x102, 0x103, 0x104)
        let result = mem.read_long(0x101).unwrap();
        assert_eq!(result, 0xBBCCDDEE);

        // Aligned long write should succeed
        let result = mem.write_long(0x200, 0x12345678);
        assert!(result.is_ok());

        // Verify the aligned write
        let result = mem.read_long(0x200);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x12345678);

        // Test unaligned write
        let result = mem.write_word(0x301, 0xABCD);
        assert!(result.is_ok());
        assert_eq!(mem.read_byte(0x301).unwrap(), 0xAB);
        assert_eq!(mem.read_byte(0x302).unwrap(), 0xCD);
    }

    #[test]
    fn test_out_of_bounds_access() {
        let mut mem = Memory::new(1024);

        // Read beyond memory
        let result = mem.read_byte(0x1000);
        assert!(matches!(result, Err(MemoryError::AddressOutOfRange { .. })));

        // Write beyond memory
        let result = mem.write_byte(0x1000, 0xFF);
        assert!(matches!(result, Err(MemoryError::AddressOutOfRange { .. })));

        // Word read at last even address - should succeed (reads 0x3FE and 0x3FF)
        let result = mem.read_word(0x3FE);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_binary() {
        let mut mem = Memory::new(1024);
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let count = mem.load_binary(0x100, &data).unwrap();
        assert_eq!(count, 4);
        assert_eq!(mem.read_byte(0x100).unwrap(), 0x01);
        assert_eq!(mem.read_byte(0x101).unwrap(), 0x02);
        assert_eq!(mem.read_byte(0x102).unwrap(), 0x03);
        assert_eq!(mem.read_byte(0x103).unwrap(), 0x04);
    }

    #[test]
    fn test_load_binary_out_of_bounds() {
        let mut mem = Memory::new(1024);
        let data = vec![0u8; 100];
        let result = mem.load_binary(0xFF0, &data);
        assert!(matches!(result, Err(MemoryError::AddressOutOfRange { .. })));
    }

    #[test]
    fn test_clear() {
        let mut mem = Memory::new(1024);
        mem.write_word(0x100, 0x1234).unwrap();
        mem.write_long(0x200, 0x56789ABC).unwrap();
        mem.clear();
        assert_eq!(mem.read_word(0x100).unwrap(), 0);
        assert_eq!(mem.read_long(0x200).unwrap(), 0);
    }

    #[test]
    fn test_read_word_unchecked() {
        let mut mem = Memory::new(1024);
        mem.write_byte(0x100, 0xAA).unwrap();
        mem.write_byte(0x101, 0xBB).unwrap();

        // Unchecked read should work even if we don't care about alignment
        let value = mem.read_word_unchecked(0x100);
        assert_eq!(value, 0xAABB);
    }

    #[test]
    fn test_read_long_unchecked() {
        let mut mem = Memory::new(1024);
        mem.write_byte(0x100, 0x11).unwrap();
        mem.write_byte(0x101, 0x22).unwrap();
        mem.write_byte(0x102, 0x33).unwrap();
        mem.write_byte(0x103, 0x44).unwrap();

        let value = mem.read_long_unchecked(0x100);
        assert_eq!(value, 0x11223344);
    }

    #[test]
    fn test_dump_range() {
        let mut mem = Memory::new(1024);
        mem.write_byte(0x100, 0x41).unwrap(); // 'A'
        mem.write_byte(0x101, 0x42).unwrap(); // 'B'
        mem.write_byte(0x102, 0x43).unwrap(); // 'C'

        let dump = mem.dump_range(0x100, 16);
        assert!(dump.contains("00000100:"));
        assert!(dump.contains("41 42 43"));
        assert!(dump.contains("ABC"));
    }

    #[test]
    fn test_read_hook_overrides_memory() {
        let mut mem = Memory::new(1024);
        mem.write_byte(0x100, 0xAA).unwrap();

        mem.set_read_hook(|address| if address == 0x100 { Some(0x55) } else { None });

        assert_eq!(mem.read_byte(0x100).unwrap(), 0x55);
        assert_eq!(mem.read_byte(0x101).unwrap(), 0x00);
    }

    #[test]
    fn test_write_hook_blocks_memory_write() {
        let mut mem = Memory::new(1024);
        mem.set_write_hook(|address, _value, _size| {
            if address == 0x100 {
                WriteHookResult::Handled
            } else {
                WriteHookResult::Unhandled
            }
        });

        mem.write_byte(0x100, 0xAA).unwrap();
        mem.write_byte(0x101, 0xBB).unwrap();

        // The blocked write should not affect backing memory.
        assert_eq!(mem.read_byte(0x100).unwrap(), 0x00);
        assert_eq!(mem.read_byte(0x101).unwrap(), 0xBB);
    }
}
