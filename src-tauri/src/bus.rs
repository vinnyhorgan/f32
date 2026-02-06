//! Memory Bus Architecture for SBC-Compatible System
//!
//! This module provides a flexible memory bus implementation that supports
//! multiple memory regions with different behaviors (ROM, RAM, peripherals).
//!
//! The target SBC has the following memory map:
//!
//! ```text
//! $000000-$0FFFFF  ROM (64KB repeated 16×)
//! $100000-$1FFFFF  Forbidden (ROM + CF overlap)
//! $200000-$2FFFFF  ROM mirror (64KB repeated 16×)
//! $300000-$7FFFFF  Forbidden (overlaps from minimal decode)
//! $800000-$8FFFFF  Open bus (expansion)
//! $900000-$9FFFFF  CompactFlash card
//! $A00000-$AFFFFF  UART (16550)
//! $B00000-$BFFFFF  Forbidden (UART + CF overlap)
//! $C00000-$CFFFFF  RAM (1MB)
//! $D00000-$DFFFFF  Forbidden (RAM + CF overlap)
//! $E00000-$EFFFFF  RAM mirror
//! $F00000-$FFFFFF  Forbidden (RAM + CF overlap)
//! ```
//!
//! Address decoding equations from the hardware:
//! ```text
//! /ROMSEL  = /A23
//! /RAMSEL  =  A22
//! /UARTSEL =  A23 * /A22 * A21
//! /CARDSEL =  A20
//! ```

// Allow dead code - this module is exercised through the CLI
#![allow(dead_code)]

use std::fmt;

/// M68K uses 24-bit addresses
pub const ADDR_MASK: u32 = 0x00FF_FFFF;

/// Result type for bus operations
pub type BusResult<T> = Result<T, BusError>;

/// Errors that can occur during bus operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusError {
    /// Address is not mapped to any device
    Unmapped(u32),
    /// Write to read-only memory (ROM)
    ReadOnly(u32),
    /// Alignment error
    Alignment(u32),
}

impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unmapped(addr) => write!(f, "Unmapped address: ${:06X}", addr),
            Self::ReadOnly(addr) => write!(f, "Write to ROM at ${:06X}", addr),
            Self::Alignment(addr) => write!(f, "Alignment error at ${:06X}", addr),
        }
    }
}

impl std::error::Error for BusError {}

/// 64KB ROM region (read-only)
///
/// Stores the firmware. In hardware, this is two 32KB AT28C256 EEPROMs
/// (one for odd bytes, one for even bytes). The ROM is mirrored throughout the
/// lower 2MB of address space when A23 is low.
#[derive(Clone)]
pub struct RomRegion {
    /// ROM data (64KB)
    data: Vec<u8>,
}

impl Default for RomRegion {
    fn default() -> Self {
        Self::new()
    }
}

impl RomRegion {
    /// ROM size: 64KB
    pub const SIZE: usize = 64 * 1024;

    /// Creates a new empty ROM region
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: vec![0xFF; Self::SIZE], // Unprogrammed EEPROM reads as 0xFF
        }
    }

    /// Loads ROM data from bytes.
    ///
    /// If the data is smaller than 64KB, it's zero-padded.
    /// If larger, it's truncated.
    pub fn load(&mut self, data: &[u8]) {
        let len = data.len().min(Self::SIZE);
        self.data[..len].copy_from_slice(&data[..len]);
    }

    /// Loads split ROM images (rom-l.bin and rom-u.bin).
    ///
    /// The target board uses two 8-bit EEPROMs for the 16-bit data bus:
    /// - rom-l.bin: Lower bytes (odd addresses in M68K terms, but D0-D7)
    /// - rom-u.bin: Upper bytes (even addresses, D8-D15)
    ///
    /// This interleaves them into a single 64KB image.
    pub fn load_split(&mut self, rom_l: &[u8], rom_u: &[u8]) {
        let len = rom_l.len().min(rom_u.len()).min(Self::SIZE / 2);
        for i in 0..len {
            self.data[i * 2] = rom_u[i]; // High byte first (big-endian)
            self.data[i * 2 + 1] = rom_l[i]; // Low byte second
        }
    }

    /// Reads a byte from ROM.
    #[inline]
    pub fn read_byte(&self, offset: u32) -> u8 {
        let idx = (offset as usize) & (Self::SIZE - 1);
        self.data[idx]
    }

    /// Reads a word from ROM (big-endian).
    #[inline]
    pub fn read_word(&self, offset: u32) -> u16 {
        let idx = (offset as usize) & (Self::SIZE - 1);
        let hi = self.data[idx] as u16;
        let lo = self.data[(idx + 1) & (Self::SIZE - 1)] as u16;
        (hi << 8) | lo
    }

    /// Reads a long word from ROM (big-endian).
    #[inline]
    pub fn read_long(&self, offset: u32) -> u32 {
        let hi = self.read_word(offset) as u32;
        let lo = self.read_word(offset.wrapping_add(2)) as u32;
        (hi << 16) | lo
    }
}

/// 1MB RAM region (read-write)
///
/// The target board has 1MB of SRAM at $C00000-$CFFFFF, mirrored at $E00000-$EFFFFF.
/// Applications are loaded at $E00100 (256 bytes past RAM start for system variables).
#[derive(Clone)]
pub struct RamRegion {
    /// RAM data (1MB)
    data: Vec<u8>,
}

impl Default for RamRegion {
    fn default() -> Self {
        Self::new()
    }
}

impl RamRegion {
    /// RAM size: 1MB
    pub const SIZE: usize = 1024 * 1024;

    /// Creates a new RAM region initialized to zero
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: vec![0; Self::SIZE],
        }
    }

    /// Clears RAM to zero
    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    /// Reads a byte from RAM.
    #[inline]
    pub fn read_byte(&self, offset: u32) -> u8 {
        let idx = (offset as usize) & (Self::SIZE - 1);
        self.data[idx]
    }

    /// Writes a byte to RAM.
    #[inline]
    pub fn write_byte(&mut self, offset: u32, value: u8) {
        let idx = (offset as usize) & (Self::SIZE - 1);
        self.data[idx] = value;
    }

    /// Reads a word from RAM (big-endian).
    #[inline]
    pub fn read_word(&self, offset: u32) -> u16 {
        let idx = (offset as usize) & (Self::SIZE - 1);
        let hi = self.data[idx] as u16;
        let lo = self.data[(idx + 1) & (Self::SIZE - 1)] as u16;
        (hi << 8) | lo
    }

    /// Writes a word to RAM (big-endian).
    #[inline]
    pub fn write_word(&mut self, offset: u32, value: u16) {
        let idx = (offset as usize) & (Self::SIZE - 1);
        self.data[idx] = (value >> 8) as u8;
        self.data[(idx + 1) & (Self::SIZE - 1)] = value as u8;
    }

    /// Reads a long word from RAM (big-endian).
    #[inline]
    pub fn read_long(&self, offset: u32) -> u32 {
        let hi = self.read_word(offset) as u32;
        let lo = self.read_word(offset.wrapping_add(2)) as u32;
        (hi << 16) | lo
    }

    /// Writes a long word to RAM (big-endian).
    #[inline]
    pub fn write_long(&mut self, offset: u32, value: u32) {
        self.write_word(offset, (value >> 16) as u16);
        self.write_word(offset.wrapping_add(2), value as u16);
    }

    /// Loads binary data into RAM at the specified offset.
    pub fn load(&mut self, offset: u32, data: &[u8]) {
        let start = (offset as usize) & (Self::SIZE - 1);
        let end = (start + data.len()).min(Self::SIZE);
        let len = end - start;
        self.data[start..end].copy_from_slice(&data[..len]);
    }
}

/// Memory bus for the SBC-compatible system.
///
/// Routes memory accesses to the appropriate device based on address:
/// - ROM: $000000-$0FFFF (mirrored when A23=0)
/// - RAM: $C00000-$CFFFFF and $E00000-$EFFFFF
/// - UART: $A00000 region
/// - CF Card: $900000 region
pub struct MemoryBus {
    /// ROM region (64KB)
    pub rom: RomRegion,
    /// RAM region (1MB)
    pub ram: RamRegion,
    /// UART read callback
    uart_read: Option<fn(u32) -> u8>,
    /// UART write callback
    uart_write: Option<fn(u32, u8)>,
    /// CF card read callback
    cf_read: Option<fn(u32) -> u8>,
    /// CF card write callback
    cf_write: Option<fn(u32, u8)>,
}

impl Default for MemoryBus {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryBus {
    /// Creates a new memory bus with empty ROM and zeroed RAM
    #[must_use]
    pub fn new() -> Self {
        Self {
            rom: RomRegion::new(),
            ram: RamRegion::new(),
            uart_read: None,
            uart_write: None,
            cf_read: None,
            cf_write: None,
        }
    }

    /// Sets the UART I/O callbacks
    pub fn set_uart_handlers(&mut self, read: fn(u32) -> u8, write: fn(u32, u8)) {
        self.uart_read = Some(read);
        self.uart_write = Some(write);
    }

    /// Sets the CompactFlash I/O callbacks
    pub fn set_cf_handlers(&mut self, read: fn(u32) -> u8, write: fn(u32, u8)) {
        self.cf_read = Some(read);
        self.cf_write = Some(write);
    }

    /// Decodes an address and returns which device it maps to.
    ///
    /// Based on the target board address decoding:
    /// - /ROMSEL  = /A23 (ROM selected when A23=0)
    /// - /RAMSEL  = A22 (RAM selected when A22=1)
    /// - /UARTSEL = A23 * /A22 * A21 (UART at $A00000)
    /// - /CARDSEL = A20 (CF at $900000, but overlaps elsewhere)
    #[inline]
    fn decode_address(&self, addr: u32) -> AddressRegion {
        let addr = addr & ADDR_MASK;
        let a23 = (addr >> 23) & 1;
        let a22 = (addr >> 22) & 1;
        let a21 = (addr >> 21) & 1;
        let a20 = (addr >> 20) & 1;

        let rom_sel = a23 == 0;
        let ram_sel = a22 == 1;
        let uart_sel = a23 == 1 && a22 == 0 && a21 == 1;
        let card_sel = a20 == 1;

        let selected = rom_sel as u8 + ram_sel as u8 + uart_sel as u8 + card_sel as u8;
        if selected == 0 {
            return AddressRegion::OpenBus;
        }
        if selected > 1 {
            return AddressRegion::Conflict;
        }

        if rom_sel {
            // ROM is 64KB mirrored within allowed windows.
            AddressRegion::Rom(addr & 0xFFFF)
        } else if ram_sel {
            // RAM is 1MB mirrored wherever RAM is selected.
            AddressRegion::Ram(addr & 0xFFFFF)
        } else if uart_sel {
            AddressRegion::Uart(addr & 0xF)
        } else {
            AddressRegion::CfCard(addr & 0xF)
        }
    }

    /// Reads a byte from the bus.
    pub fn read_byte(&self, addr: u32) -> BusResult<u8> {
        match self.decode_address(addr) {
            AddressRegion::Rom(offset) => Ok(self.rom.read_byte(offset)),
            AddressRegion::Ram(offset) => Ok(self.ram.read_byte(offset)),
            AddressRegion::Uart(offset) => {
                if let Some(read) = self.uart_read {
                    Ok(read(offset))
                } else {
                    Ok(0xFF) // No UART handler, return open bus
                }
            }
            AddressRegion::CfCard(offset) => {
                if let Some(read) = self.cf_read {
                    Ok(read(offset))
                } else {
                    Ok(0xFF)
                }
            }
            AddressRegion::OpenBus | AddressRegion::Conflict => Ok(0xFF),
        }
    }

    /// Writes a byte to the bus.
    pub fn write_byte(&mut self, addr: u32, value: u8) -> BusResult<()> {
        match self.decode_address(addr) {
            AddressRegion::Rom(_) => {
                // Silently ignore writes to ROM (like real hardware)
                Ok(())
            }
            AddressRegion::Ram(offset) => {
                self.ram.write_byte(offset, value);
                Ok(())
            }
            AddressRegion::Uart(offset) => {
                if let Some(write) = self.uart_write {
                    write(offset, value);
                }
                Ok(())
            }
            AddressRegion::CfCard(offset) => {
                if let Some(write) = self.cf_write {
                    write(offset, value);
                }
                Ok(())
            }
            AddressRegion::OpenBus | AddressRegion::Conflict => Ok(()), // Writes ignored
        }
    }

    /// Reads a word (16-bit) from the bus.
    pub fn read_word(&self, addr: u32) -> BusResult<u16> {
        match self.decode_address(addr) {
            AddressRegion::Rom(offset) => Ok(self.rom.read_word(offset)),
            AddressRegion::Ram(offset) => Ok(self.ram.read_word(offset)),
            AddressRegion::Uart(_offset) => {
                // Word read from UART: read two consecutive bytes
                let hi = self.read_byte(addr)?;
                let lo = self.read_byte(addr + 1)?;
                Ok(((hi as u16) << 8) | (lo as u16))
            }
            AddressRegion::CfCard(offset) => {
                // CF card data register is 16-bit
                if offset == 0 {
                    // Data register - 16-bit read
                    if let Some(read) = self.cf_read {
                        let hi = read(0);
                        let lo = read(1);
                        Ok(((hi as u16) << 8) | (lo as u16))
                    } else {
                        Ok(0xFFFF)
                    }
                } else {
                    let hi = self.read_byte(addr)?;
                    let lo = self.read_byte(addr + 1)?;
                    Ok(((hi as u16) << 8) | (lo as u16))
                }
            }
            AddressRegion::OpenBus | AddressRegion::Conflict => Ok(0xFFFF),
        }
    }

    /// Writes a word (16-bit) to the bus.
    pub fn write_word(&mut self, addr: u32, value: u16) -> BusResult<()> {
        match self.decode_address(addr) {
            AddressRegion::Rom(_) => Ok(()), // Ignore ROM writes
            AddressRegion::Ram(offset) => {
                self.ram.write_word(offset, value);
                Ok(())
            }
            AddressRegion::Uart(_) | AddressRegion::CfCard(_) => {
                // Peripheral word writes: write two consecutive bytes
                self.write_byte(addr, (value >> 8) as u8)?;
                self.write_byte(addr + 1, value as u8)?;
                Ok(())
            }
            AddressRegion::OpenBus | AddressRegion::Conflict => Ok(()),
        }
    }

    /// Reads a long word (32-bit) from the bus.
    pub fn read_long(&self, addr: u32) -> BusResult<u32> {
        let hi = self.read_word(addr)? as u32;
        let lo = self.read_word(addr + 2)? as u32;
        Ok((hi << 16) | lo)
    }

    /// Writes a long word (32-bit) to the bus.
    pub fn write_long(&mut self, addr: u32, value: u32) -> BusResult<()> {
        self.write_word(addr, (value >> 16) as u16)?;
        self.write_word(addr + 2, value as u16)?;
        Ok(())
    }

    /// Reads a byte without error checking (returns 0 for unmapped).
    /// Used for instruction fetching where we want speed over error handling.
    #[inline]
    pub fn read_byte_unchecked(&self, addr: u32) -> u8 {
        self.read_byte(addr).unwrap_or(0)
    }

    /// Reads a word without error checking.
    #[inline]
    pub fn read_word_unchecked(&self, addr: u32) -> u16 {
        self.read_word(addr).unwrap_or(0)
    }

    /// Reads a long without error checking.
    #[inline]
    pub fn read_long_unchecked(&self, addr: u32) -> u32 {
        self.read_long(addr).unwrap_or(0)
    }
}

/// Memory region decoded from an address
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddressRegion {
    /// ROM at $000000 (with offset within 64KB)
    Rom(u32),
    /// RAM at $C00000 (with offset within 1MB)
    Ram(u32),
    /// UART at $A00000 (with register offset)
    Uart(u32),
    /// CompactFlash at $900000 (with register offset)
    CfCard(u32),
    /// Open bus (unmapped, returns 0xFF)
    OpenBus,
    /// Multiple devices selected (bus contention)
    Conflict,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rom_region_new() {
        let rom = RomRegion::new();
        // Unprogrammed ROM reads as 0xFF
        assert_eq!(rom.read_byte(0), 0xFF);
        assert_eq!(rom.read_byte(0xFFFF), 0xFF);
    }

    #[test]
    fn test_rom_region_load() {
        let mut rom = RomRegion::new();
        rom.load(&[0x00, 0x00, 0x10, 0x00]); // Reset vectors
        assert_eq!(rom.read_byte(0), 0x00);
        assert_eq!(rom.read_byte(2), 0x10);
        assert_eq!(rom.read_long(0), 0x00001000);
    }

    #[test]
    fn test_rom_region_split_load() {
        let mut rom = RomRegion::new();
        let rom_u = [0xAA, 0xBB]; // Upper bytes
        let rom_l = [0x11, 0x22]; // Lower bytes
        rom.load_split(&rom_l, &rom_u);
        // Interleaved: AA 11 BB 22
        assert_eq!(rom.read_word(0), 0xAA11);
        assert_eq!(rom.read_word(2), 0xBB22);
    }

    #[test]
    fn test_ram_region_read_write() {
        let mut ram = RamRegion::new();
        ram.write_byte(0, 0x42);
        assert_eq!(ram.read_byte(0), 0x42);

        ram.write_word(0x100, 0xABCD);
        assert_eq!(ram.read_word(0x100), 0xABCD);

        ram.write_long(0x200, 0x12345678);
        assert_eq!(ram.read_long(0x200), 0x12345678);
    }

    #[test]
    fn test_ram_region_mirroring() {
        let mut ram = RamRegion::new();
        ram.write_byte(0, 0x42);
        // Access wraps at 1MB boundary
        assert_eq!(ram.read_byte(0x100000), 0x42);
    }

    #[test]
    fn test_bus_rom_access() {
        let mut bus = MemoryBus::new();
        bus.rom
            .load(&[0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x08]);

        // ROM at $000000
        assert_eq!(bus.read_long(0x000000).unwrap(), 0x00001000);
        assert_eq!(bus.read_long(0x000004).unwrap(), 0x00000008);

        // ROM mirrored at $010000
        assert_eq!(bus.read_long(0x010000).unwrap(), 0x00001000);

        // ROM mirrored at $200000
        assert_eq!(bus.read_long(0x200000).unwrap(), 0x00001000);
    }

    #[test]
    fn test_bus_ram_access() {
        let mut bus = MemoryBus::new();

        // RAM at $C00000
        bus.write_long(0xC00000, 0xDEADBEEF).unwrap();
        assert_eq!(bus.read_long(0xC00000).unwrap(), 0xDEADBEEF);

        // RAM mirrored at $E00000
        assert_eq!(bus.read_long(0xE00000).unwrap(), 0xDEADBEEF);

        // Write via mirror
        bus.write_long(0xE00100, 0xCAFEBABE).unwrap();
        assert_eq!(bus.read_long(0xC00100).unwrap(), 0xCAFEBABE);
    }

    #[test]
    fn test_bus_rom_write_ignored() {
        let mut bus = MemoryBus::new();
        bus.rom.load(&[0xAA, 0xBB, 0xCC, 0xDD]);

        // Writes to ROM should be silently ignored
        bus.write_byte(0x000000, 0xFF).unwrap();
        assert_eq!(bus.read_byte(0x000000).unwrap(), 0xAA);
    }

    #[test]
    fn test_bus_open_bus() {
        let bus = MemoryBus::new();
        // $800000 region is open bus
        assert_eq!(bus.read_byte(0x800000).unwrap(), 0xFF);
    }

    #[test]
    fn test_bus_conflict_region_reads_open_bus() {
        let bus = MemoryBus::new();
        // $100000 region overlaps ROM and CF (conflict)
        assert_eq!(bus.read_byte(0x100000).unwrap(), 0xFF);
    }

    #[test]
    fn test_address_decode_regions() {
        let bus = MemoryBus::new();

        // Test various addresses decode correctly
        assert!(matches!(
            bus.decode_address(0x000000),
            AddressRegion::Rom(_)
        ));
        assert!(matches!(
            bus.decode_address(0x100000),
            AddressRegion::Conflict
        ));
        assert!(matches!(
            bus.decode_address(0x800000),
            AddressRegion::OpenBus
        ));
        assert!(matches!(
            bus.decode_address(0x900000),
            AddressRegion::CfCard(_)
        ));
        assert!(matches!(
            bus.decode_address(0xA00000),
            AddressRegion::Uart(_)
        ));
        assert!(matches!(
            bus.decode_address(0xB00000),
            AddressRegion::Conflict
        ));
        assert!(matches!(
            bus.decode_address(0xC00000),
            AddressRegion::Ram(_)
        ));
        assert!(matches!(
            bus.decode_address(0xE00000),
            AddressRegion::Ram(_)
        ));
    }
}
