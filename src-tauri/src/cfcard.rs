//! CompactFlash Card Emulation for SBC-Compatible System
//!
//! This module emulates an IDE/ATA CompactFlash card in True IDE 16-bit mode.
//! The CF card is memory-mapped at $900000 with the standard IDE task file registers.
//!
//! ## Register Map (offsets from $900000)
//!
//! The target board uses odd byte addresses for 8-bit registers due to the way
//! the 16-bit data bus is wired to the 8-bit M68K data lines.
//!
//! | Offset | Read          | Write         |
//! |--------|---------------|---------------|
//! | 0      | Data (16-bit) | Data (16-bit) |
//! | 3      | Error         | Feature       |
//! | 5      | Sector Count  | Sector Count  |
//! | 7      | LBA0          | LBA0          |
//! | 9      | LBA1          | LBA1          |
//! | 11     | LBA2          | LBA2          |
//! | 13     | Drive/Head    | Drive/Head    |
//! | 15     | Status        | Command       |
//!
//! ## Supported Commands
//!
//! - $EC: Identify Device (returns 512-byte identification block)
//! - $20: Read Sector(s) (reads from loaded disk image)
//!
//! ## Disk Image Format
//!
//! The emulator loads raw disk images (typically FAT16 formatted).
//! Each sector is 512 bytes. The image should be a multiple of 512 bytes.

// Allow dead code - this module is exercised through the CLI
#![allow(dead_code)]

use std::fs;
use std::io::{self, Read};
use std::path::Path;

/// Base address of the CF card in the system memory map
pub const CF_BASE: u32 = 0x900000;

/// Sector size in bytes
pub const SECTOR_SIZE: usize = 512;

/// Number of status reads to keep the card busy after a command.
const BUSY_READS: u8 = 2;

/// CF card register offsets (byte offsets)
pub mod regs {
    /// Data register (16-bit)
    pub const DATA: u32 = 0;
    /// Error register (read) / Feature register (write)
    pub const ERROR_FEATURE: u32 = 3;
    /// Sector count register
    pub const SECTOR_COUNT: u32 = 5;
    /// LBA bits 0-7 / Sector number
    pub const LBA0: u32 = 7;
    /// LBA bits 8-15 / Cylinder low
    pub const LBA1: u32 = 9;
    /// LBA bits 16-23 / Cylinder high
    pub const LBA2: u32 = 11;
    /// LBA bits 24-27 + Drive select / Drive/Head
    pub const DRIVE_HEAD: u32 = 13;
    /// Status register (read) / Command register (write)
    pub const STATUS_COMMAND: u32 = 15;
}

/// Status register bits
pub mod status {
    /// Error occurred
    pub const ERR: u8 = 0x01;
    /// Index (always 0)
    pub const IDX: u8 = 0x02;
    /// Corrected data (always 0)
    pub const CORR: u8 = 0x04;
    /// Data request - ready for data transfer
    pub const DRQ: u8 = 0x08;
    /// Drive seek complete
    pub const DSC: u8 = 0x10;
    /// Drive write fault
    pub const DWF: u8 = 0x20;
    /// Drive ready
    pub const DRDY: u8 = 0x40;
    /// Busy - command in progress
    pub const BSY: u8 = 0x80;
}

/// Error register bits
pub mod error {
    /// Address mark not found
    pub const AMNF: u8 = 0x01;
    /// Track 0 not found
    pub const TK0NF: u8 = 0x02;
    /// Command aborted
    pub const ABRT: u8 = 0x04;
    /// Media change requested
    pub const MCR: u8 = 0x08;
    /// ID not found
    pub const IDNF: u8 = 0x10;
    /// Media changed
    pub const MC: u8 = 0x20;
    /// Uncorrectable data error
    pub const UNC: u8 = 0x40;
    /// Bad block detected
    pub const BBK: u8 = 0x80;
}

/// IDE/ATA commands
pub mod commands {
    /// Identify Device - returns 512 bytes of device info
    pub const IDENTIFY: u8 = 0xEC;
    /// Read Sector(s) with retry
    pub const READ_SECTORS: u8 = 0x20;
    /// Read Sector(s) without retry
    pub const READ_SECTORS_NR: u8 = 0x21;
    /// Write Sector(s) with retry
    pub const WRITE_SECTORS: u8 = 0x30;
    /// Write Sector(s) without retry
    pub const WRITE_SECTORS_NR: u8 = 0x31;
}

/// CompactFlash card emulation
#[derive(Clone)]
pub struct CfCard {
    /// Disk image data
    data: Vec<u8>,
    /// Total number of sectors
    total_sectors: u32,
    /// Whether a card is inserted
    inserted: bool,
    /// Volume label (11 chars, space-padded)
    label: [u8; 11],

    // Task file registers
    /// Error register (read)
    error: u8,
    /// Feature register (write)
    feature: u8,
    /// Sector count register
    sector_count: u8,
    /// LBA0 / Sector number
    lba0: u8,
    /// LBA1 / Cylinder low
    lba1: u8,
    /// LBA2 / Cylinder high
    lba2: u8,
    /// Drive/Head register
    drive_head: u8,
    /// Status register
    status: u8,
    /// Busy countdown (status reads remaining)
    busy_reads_remaining: u8,

    /// Data transfer buffer (512 bytes for sector data)
    buffer: Vec<u8>,
    /// Current position in the buffer for reads
    buffer_pos: usize,
    /// Number of bytes remaining in the buffer
    buffer_remaining: usize,
}

impl Default for CfCard {
    fn default() -> Self {
        Self::new()
    }
}

impl CfCard {
    /// Creates a new empty CF card (no card inserted)
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            total_sectors: 0,
            inserted: false,
            label: *b"NO NAME    ",
            error: 0,
            feature: 0,
            sector_count: 0,
            lba0: 0,
            lba1: 0,
            lba2: 0,
            drive_head: 0,
            status: 0,
            busy_reads_remaining: 0,
            buffer: vec![0; SECTOR_SIZE],
            buffer_pos: 0,
            buffer_remaining: 0,
        }
    }

    /// Loads a disk image from a file
    ///
    /// The image should be a raw disk image (e.g., created with `dd`).
    /// FAT16 images are typically 16MB-2GB in size.
    pub fn load_image(&mut self, path: &Path) -> io::Result<()> {
        let mut file = fs::File::open(path)?;
        self.data.clear();
        file.read_to_end(&mut self.data)?;

        // Ensure size is a multiple of sector size
        let remainder = self.data.len() % SECTOR_SIZE;
        if remainder != 0 {
            self.data
                .resize(self.data.len() + SECTOR_SIZE - remainder, 0);
        }

        self.total_sectors = (self.data.len() / SECTOR_SIZE) as u32;
        self.inserted = true;
        self.status = status::DRDY | status::DSC;
        self.error = 0;
        self.busy_reads_remaining = 0;

        // Try to read volume label from FAT16 BPB
        self.read_volume_label();

        Ok(())
    }

    /// Loads a disk image from bytes
    pub fn load_bytes(&mut self, data: &[u8]) {
        self.data = data.to_vec();

        // Ensure size is a multiple of sector size
        let remainder = self.data.len() % SECTOR_SIZE;
        if remainder != 0 {
            self.data
                .resize(self.data.len() + SECTOR_SIZE - remainder, 0);
        }

        self.total_sectors = (self.data.len() / SECTOR_SIZE) as u32;
        self.inserted = true;
        self.status = status::DRDY | status::DSC;
        self.error = 0;
        self.busy_reads_remaining = 0;

        self.read_volume_label();
    }

    /// Ejects the current disk image
    pub fn eject(&mut self) {
        self.data.clear();
        self.total_sectors = 0;
        self.inserted = false;
        self.status = 0;
        self.error = 0;
        self.buffer_remaining = 0;
        self.busy_reads_remaining = 0;
    }

    /// Returns true if a card is inserted
    #[must_use]
    pub const fn is_inserted(&self) -> bool {
        self.inserted
    }

    /// Returns the total capacity in bytes
    #[must_use]
    pub fn capacity(&self) -> u64 {
        self.data.len() as u64
    }

    /// Returns the total number of sectors
    #[must_use]
    pub const fn sector_count(&self) -> u32 {
        self.total_sectors
    }

    /// Returns the volume label as a string
    #[must_use]
    pub fn volume_label(&self) -> &str {
        std::str::from_utf8(&self.label).unwrap_or("???????????")
    }

    /// Attempts to read the FAT16 volume label from the BPB
    fn read_volume_label(&mut self) {
        // Volume label is at offset 0x2B in the BPB (first sector)
        if self.data.len() >= SECTOR_SIZE {
            let label_offset = 0x2B;
            if self.data.len() > label_offset + 11 {
                self.label
                    .copy_from_slice(&self.data[label_offset..label_offset + 11]);
            }
        }
    }

    /// Reads from a CF card register
    ///
    /// `offset` is the byte offset from the CF base address.
    pub fn read(&mut self, offset: u32) -> u8 {
        if !self.inserted {
            return 0xFF; // No card - open bus
        }

        match offset & 0xF {
            0 | 1 => {
                // Data register (16-bit, but we handle byte-by-byte)
                self.read_data()
            }
            3 => {
                // Error register
                self.error
            }
            5 => {
                // Sector count
                self.sector_count
            }
            7 => {
                // LBA0
                self.lba0
            }
            9 => {
                // LBA1
                self.lba1
            }
            11 => {
                // LBA2
                self.lba2
            }
            13 => {
                // Drive/Head
                self.drive_head
            }
            15 => {
                // Status
                let mut status = self.status;
                if self.busy_reads_remaining > 0 {
                    status |= status::BSY;
                    status &= !status::DRQ;
                    self.busy_reads_remaining -= 1;
                }
                status
            }
            _ => 0xFF,
        }
    }

    /// Writes to a CF card register
    ///
    /// `offset` is the byte offset from the CF base address.
    pub fn write(&mut self, offset: u32, value: u8) {
        if !self.inserted {
            return; // No card
        }

        match offset & 0xF {
            0 | 1 => {
                // Data register - ignore writes for now (read-only disk)
            }
            3 => {
                // Feature register
                self.feature = value;
            }
            5 => {
                // Sector count
                self.sector_count = value;
            }
            7 => {
                // LBA0
                self.lba0 = value;
            }
            9 => {
                // LBA1
                self.lba1 = value;
            }
            11 => {
                // LBA2
                self.lba2 = value;
            }
            13 => {
                // Drive/Head
                self.drive_head = value;
            }
            15 => {
                // Command register
                self.execute_command(value);
            }
            _ => {}
        }
    }

    /// Reads a 16-bit word from the data register
    pub fn read_data_word(&mut self) -> u16 {
        let hi = self.read_data() as u16;
        let lo = self.read_data() as u16;
        (hi << 8) | lo
    }

    /// Reads a byte from the data buffer
    fn read_data(&mut self) -> u8 {
        if self.buffer_remaining == 0 {
            return 0;
        }

        let byte = self.buffer[self.buffer_pos];
        self.buffer_pos += 1;
        self.buffer_remaining -= 1;

        // If buffer is exhausted, clear DRQ
        if self.buffer_remaining == 0 {
            self.status &= !status::DRQ;

            // If more sectors to read, set up next sector
            if self.sector_count > 0 {
                self.sector_count -= 1;
                if self.sector_count > 0 {
                    // Increment LBA
                    let lba = self.get_lba() + 1;
                    self.set_lba(lba);
                    self.setup_read_sector(lba);
                }
            }
        }

        byte
    }

    /// Gets the current LBA from the task file registers
    fn get_lba(&self) -> u32 {
        let lba0 = self.lba0 as u32;
        let lba1 = self.lba1 as u32;
        let lba2 = self.lba2 as u32;
        let lba3 = (self.drive_head & 0x0F) as u32;
        lba0 | (lba1 << 8) | (lba2 << 16) | (lba3 << 24)
    }

    /// Sets the LBA in the task file registers
    fn set_lba(&mut self, lba: u32) {
        self.lba0 = lba as u8;
        self.lba1 = (lba >> 8) as u8;
        self.lba2 = (lba >> 16) as u8;
        self.drive_head = (self.drive_head & 0xF0) | ((lba >> 24) as u8 & 0x0F);
    }

    /// Executes an ATA command
    fn execute_command(&mut self, cmd: u8) {
        self.error = 0;
        self.busy_reads_remaining = BUSY_READS;

        match cmd {
            commands::IDENTIFY => {
                self.execute_identify();
            }
            commands::READ_SECTORS | commands::READ_SECTORS_NR => {
                let lba = self.get_lba();
                self.setup_read_sector(lba);
            }
            commands::WRITE_SECTORS | commands::WRITE_SECTORS_NR => {
                // Write is not supported (read-only disk)
                self.error = error::ABRT;
                self.status = status::DRDY | status::ERR;
            }
            _ => {
                // Unknown command
                self.error = error::ABRT;
                self.status = status::DRDY | status::ERR;
            }
        }
    }

    /// Executes the IDENTIFY DEVICE command
    fn execute_identify(&mut self) {
        // Build the 512-byte identification block
        self.buffer.fill(0);

        // Word 0: General configuration
        self.buffer[0] = 0x84; // Removable, not MFM
        self.buffer[1] = 0x8A; // Hard sectored, etc.

        // Word 1: Number of cylinders (obsolete, but fill in)
        let cyls = (self.total_sectors / (16 * 63)).min(16383) as u16;
        self.buffer[2] = cyls as u8;
        self.buffer[3] = (cyls >> 8) as u8;

        // Word 3: Number of heads
        self.buffer[6] = 16;
        self.buffer[7] = 0;

        // Word 6: Number of sectors per track
        self.buffer[12] = 63;
        self.buffer[13] = 0;

        // Words 10-19: Serial number (20 ASCII chars)
        let serial = b"FLUX32-CFCARD-001   ";
        self.buffer[20..40].copy_from_slice(serial);

        // Words 23-26: Firmware revision (8 ASCII chars)
        let firmware = b"1.00    ";
        self.buffer[46..54].copy_from_slice(firmware);

        // Words 27-46: Model number (40 ASCII chars)
        let model = b"FLUX32 Virtual CompactFlash Card        ";
        self.buffer[54..94].copy_from_slice(model);

        // Word 49: Capabilities
        self.buffer[98] = 0x00;
        self.buffer[99] = 0x02; // LBA supported

        // Words 60-61: Total addressable sectors (LBA)
        self.buffer[120] = self.total_sectors as u8;
        self.buffer[121] = (self.total_sectors >> 8) as u8;
        self.buffer[122] = (self.total_sectors >> 16) as u8;
        self.buffer[123] = (self.total_sectors >> 24) as u8;

        self.buffer_pos = 0;
        self.buffer_remaining = SECTOR_SIZE;
        self.status = status::DRDY | status::DRQ | status::DSC;
    }

    /// Sets up a sector read operation
    fn setup_read_sector(&mut self, lba: u32) {
        if lba >= self.total_sectors {
            // Invalid sector
            self.error = error::IDNF;
            self.status = status::DRDY | status::ERR;
            return;
        }

        // Copy sector data to buffer
        let offset = (lba as usize) * SECTOR_SIZE;
        self.buffer
            .copy_from_slice(&self.data[offset..offset + SECTOR_SIZE]);
        self.buffer_pos = 0;
        self.buffer_remaining = SECTOR_SIZE;
        self.status = status::DRDY | status::DRQ | status::DSC;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_status_ready(cf: &mut CfCard) -> u8 {
        for _ in 0..4 {
            let status = cf.read(regs::STATUS_COMMAND);
            if status & status::BSY == 0 {
                return status;
            }
        }
        cf.read(regs::STATUS_COMMAND)
    }

    #[test]
    fn test_cfcard_new() {
        let cf = CfCard::new();
        assert!(!cf.is_inserted());
        assert_eq!(cf.capacity(), 0);
    }

    #[test]
    fn test_cfcard_load_bytes() {
        let mut cf = CfCard::new();

        // Create a minimal disk image (1 sector)
        let mut data = vec![0u8; SECTOR_SIZE];
        data[0] = 0xEB; // JMP short
        data[0x1FE] = 0x55; // Boot signature
        data[0x1FF] = 0xAA;

        cf.load_bytes(&data);

        assert!(cf.is_inserted());
        assert_eq!(cf.capacity(), SECTOR_SIZE as u64);
        assert_eq!(cf.sector_count(), 1);
    }

    #[test]
    fn test_cfcard_identify() {
        let mut cf = CfCard::new();
        cf.load_bytes(&vec![0u8; SECTOR_SIZE * 100]); // 100 sectors

        // Write IDENTIFY command
        cf.write(regs::STATUS_COMMAND, commands::IDENTIFY);

        // Status should have DRQ set
        let status = read_status_ready(&mut cf);
        assert!(status & status::DRQ != 0);
        assert!(status & status::DRDY != 0);

        // Read first few bytes of identify data
        let b0 = cf.read(regs::DATA);
        let b1 = cf.read(regs::DATA);
        assert_eq!(b0, 0x84);
        assert_eq!(b1, 0x8A);
    }

    #[test]
    fn test_cfcard_read_sector() {
        let mut cf = CfCard::new();

        // Create a disk image with known data
        let mut data = vec![0u8; SECTOR_SIZE * 10];
        data[0] = 0xAA; // Sector 0, byte 0
        data[SECTOR_SIZE] = 0xBB; // Sector 1, byte 0
        data[SECTOR_SIZE * 2] = 0xCC; // Sector 2, byte 0

        cf.load_bytes(&data);

        // Read sector 0
        cf.write(regs::LBA0, 0);
        cf.write(regs::LBA1, 0);
        cf.write(regs::LBA2, 0);
        cf.write(regs::DRIVE_HEAD, 0xE0); // LBA mode
        cf.write(regs::SECTOR_COUNT, 1);
        cf.write(regs::STATUS_COMMAND, commands::READ_SECTORS);

        // Should have DRQ
        assert!(read_status_ready(&mut cf) & status::DRQ != 0);

        // Read first byte
        assert_eq!(cf.read(regs::DATA), 0xAA);

        // Read sector 1
        cf.write(regs::LBA0, 1);
        cf.write(regs::STATUS_COMMAND, commands::READ_SECTORS);
        assert_eq!(cf.read(regs::DATA), 0xBB);
    }

    #[test]
    fn test_cfcard_invalid_sector() {
        let mut cf = CfCard::new();
        cf.load_bytes(&vec![0u8; SECTOR_SIZE]); // Only 1 sector

        // Try to read sector 10 (doesn't exist)
        cf.write(regs::LBA0, 10);
        cf.write(regs::STATUS_COMMAND, commands::READ_SECTORS);

        // Should have error
        let status = read_status_ready(&mut cf);
        assert!(status & status::ERR != 0);
        assert!(cf.read(regs::ERROR_FEATURE) & error::IDNF != 0);
    }

    #[test]
    fn test_cfcard_no_card() {
        let mut cf = CfCard::new();

        // Reading with no card should return 0xFF
        assert_eq!(cf.read(regs::STATUS_COMMAND), 0xFF);
        assert_eq!(cf.read(regs::DATA), 0xFF);
    }
}
