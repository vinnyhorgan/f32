//! SBC-Compatible Single Board Computer Emulation
//!
//! This module provides a complete emulation of a small 68k SBC, tying together
//! the CPU core, memory bus, UART, and CompactFlash peripherals.
//!
//! ## Hardware Specifications
//!
//! - **CPU**: Motorola 68HC000 at 12MHz
//! - **RAM**: 1MB SRAM at $C00000-$CFFFFF (mirrored at $E00000-$EFFFFF)
//! - **ROM**: 64KB EEPROM repeated across two 1MB windows due to minimal decode
//! - **UART**: 16550 at $A00000 (serial terminal at 57600 baud)
//! - **Storage**: CompactFlash (True IDE mode) at $900000
//! - **RTC**: DS3234 via SPI on UART modem control lines
//!
//! ## Memory Map
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
//! Address decoding equations:
//! ```text
//! /ROMSEL  = /A23
//! /RAMSEL  =  A22
//! /UARTSEL =  A23 * /A22 * A21
//! /CARDSEL =  A20
//! ```
//! Overlaps select multiple devices and are treated as open bus in the emulator.
//!
//! ## Boot Process
//!
//! 1. CPU reads initial SSP from $000000 and initial PC from $000004
//! 2. ROM initializes UART to 57600 baud
//! 3. RAM test is performed
//! 4. If CompactFlash present, FAT16 is mounted
//! 5. If STARTUP.BIN exists and button not held, it's executed
//! 6. Otherwise, enters command shell
//!
//! ## Architecture
//!
//! The SBC wraps the existing Cpu core and intercepts memory accesses to route
//! them to the appropriate peripheral devices based on address decoding.

// Allow dead code - this module is exercised through the CLI
#![allow(dead_code)]

use crate::bus::ADDR_MASK;
use crate::cfcard::CfCard;
use crate::cpu::Cpu;
use crate::memory::{OperandSize, WriteHookResult};
use crate::uart::Uart16550;
use std::cell::RefCell;
use std::io;
use std::path::Path;
use std::rc::Rc;

/// SBC clock frequency in Hz (12 MHz)
pub const CLOCK_HZ: u32 = 12_000_000;

/// Default baud rate (57600)
pub const DEFAULT_BAUD: u32 = 57600;

/// RAM base address
pub const RAM_BASE: u32 = 0xC00000;

/// RAM mirror base address
pub const RAM_MIRROR: u32 = 0xE00000;

/// Application load address (256 bytes past RAM start for system variables)
pub const APP_START: u32 = 0xE00100;

/// Initial stack pointer (end of RAM)
pub const INITIAL_SP: u32 = 0xF00000;

/// ROM size (64KB)
pub const ROM_SIZE: usize = 64 * 1024;

/// RAM size (1MB)
pub const RAM_SIZE: usize = 1024 * 1024;

/// Embedded Flux32 system ROM
/// This ROM provides the shell, syscalls, and peripheral drivers.
static EMBEDDED_ROM: &[u8] = include_bytes!("../assets/rom.bin");

// System variable addresses (in RAM at E00000)
// These must be initialized before running apps directly without ROM boot
const OUTCH_VEC: u32 = 0xE00000;
#[allow(dead_code)]
const INCH_VEC: u32 = 0xE00004;
#[allow(dead_code)]
const HEXDIGITS_VEC: u32 = 0xE00008;
#[allow(dead_code)]
const SEPARATORS_VEC: u32 = 0xE0000C;

// ROM addresses for I/O routines (determined from rom.lst)
#[allow(dead_code)]
const UART_OUTCHAR_ADDR: u32 = 0x00001186;
#[allow(dead_code)]
const UART_INCHAR_ADDR: u32 = 0x00001198;
#[allow(dead_code)]
const HEXDIGITS_UC_ADDR: u32 = 0x000014EE;
// SEPARATORS value: hyphen, colon, comma, null
#[allow(dead_code)]
const SEPARATORS_VALUE: u32 = 0x2d3a2c00;

// Global peripheral pointers for MMIO hooks (replaces thread-locals)
// Safety: These pointers are only dereferenced while holding the SBC mutex,
// ensuring exclusive access. They point to RefCells owned by the Sbc struct.
use std::sync::atomic::{AtomicPtr, Ordering};

static SBC_UART_PTR: AtomicPtr<RefCell<Uart16550>> = AtomicPtr::new(std::ptr::null_mut());
static SBC_CFCARD_PTR: AtomicPtr<RefCell<CfCard>> = AtomicPtr::new(std::ptr::null_mut());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SbcAddressRegion {
    Rom(u32),
    Ram(u32),
    Uart(u32),
    CfCard(u32),
    OpenBus,
    Conflict,
}

fn decode_address(address: u32) -> SbcAddressRegion {
    let addr = address & ADDR_MASK;
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
        return SbcAddressRegion::OpenBus;
    }
    if selected > 1 {
        return SbcAddressRegion::Conflict;
    }

    if rom_sel {
        return SbcAddressRegion::Rom(addr & 0xFFFF);
    }
    if ram_sel {
        return SbcAddressRegion::Ram(addr & 0xFFFFF);
    }
    if uart_sel {
        return SbcAddressRegion::Uart(addr & 0xF);
    }
    SbcAddressRegion::CfCard(addr & 0xF)
}

/// MMIO read hook for SBC peripherals
fn sbc_read_hook(address: u32) -> Option<u8> {
    match decode_address(address) {
        SbcAddressRegion::Uart(offset) => {
            let ptr = SBC_UART_PTR.load(Ordering::Acquire);
            if ptr.is_null() {
                return Some(0xFF);
            }
            let uart = unsafe { &*ptr };
            Some(uart.borrow_mut().read(offset))
        }
        SbcAddressRegion::CfCard(offset) => {
            let ptr = SBC_CFCARD_PTR.load(Ordering::Acquire);
            if ptr.is_null() {
                return Some(0xFF);
            }
            let cf = unsafe { &*ptr };
            Some(cf.borrow_mut().read(offset))
        }
        SbcAddressRegion::OpenBus | SbcAddressRegion::Conflict => Some(0xFF),
        SbcAddressRegion::Rom(_) | SbcAddressRegion::Ram(_) => None,
    }
}

/// MMIO write hook for SBC peripherals
fn sbc_write_hook(address: u32, value: u32, size: OperandSize) -> WriteHookResult {
    match decode_address(address) {
        SbcAddressRegion::Uart(offset) => {
            let ptr = SBC_UART_PTR.load(Ordering::Acquire);
            if !ptr.is_null() {
                let uart = unsafe { &*ptr };
                match size {
                    OperandSize::Byte => {
                        uart.borrow_mut().write(offset, value as u8);
                    }
                    OperandSize::Word => {
                        uart.borrow_mut().write(offset, (value >> 8) as u8);
                        uart.borrow_mut().write(offset + 1, value as u8);
                    }
                    OperandSize::Long => {
                        uart.borrow_mut().write(offset, (value >> 24) as u8);
                        uart.borrow_mut().write(offset + 1, (value >> 16) as u8);
                        uart.borrow_mut().write(offset + 2, (value >> 8) as u8);
                        uart.borrow_mut().write(offset + 3, value as u8);
                    }
                }
            }
            WriteHookResult::Handled
        }
        SbcAddressRegion::CfCard(offset) => {
            let ptr = SBC_CFCARD_PTR.load(Ordering::Acquire);
            if !ptr.is_null() {
                let cf = unsafe { &*ptr };
                match size {
                    OperandSize::Byte => {
                        cf.borrow_mut().write(offset, value as u8);
                    }
                    OperandSize::Word => {
                        cf.borrow_mut().write(offset, (value >> 8) as u8);
                        cf.borrow_mut().write(offset + 1, value as u8);
                    }
                    OperandSize::Long => {
                        cf.borrow_mut().write(offset, (value >> 24) as u8);
                        cf.borrow_mut().write(offset + 1, (value >> 16) as u8);
                        cf.borrow_mut().write(offset + 2, (value >> 8) as u8);
                        cf.borrow_mut().write(offset + 3, value as u8);
                    }
                }
            }
            WriteHookResult::Handled
        }
        SbcAddressRegion::Rom(_) | SbcAddressRegion::OpenBus | SbcAddressRegion::Conflict => {
            WriteHookResult::Handled
        }
        SbcAddressRegion::Ram(_) => WriteHookResult::Unhandled,
    }
}

/// SBC emulation state
///
/// The SBC wraps the Cpu core and adds:
/// - ROM at $000000 (mirrored)
/// - RAM at $C00000/$E00000
/// - UART at $A00000
/// - CompactFlash at $900000
pub struct Sbc {
    /// The CPU core (uses 16MB flat memory for simplicity)
    cpu: Cpu,
    /// UART peripheral
    uart: Rc<RefCell<Uart16550>>,
    /// CompactFlash card
    cfcard: Rc<RefCell<CfCard>>,
    /// ROM data (for read interception)
    rom_data: Vec<u8>,
    /// UART output buffer (auto-drained from TX FIFO)
    uart_output: Vec<u8>,
}

impl Default for Sbc {
    fn default() -> Self {
        Self::new()
    }
}

impl Sbc {
    /// Creates a new SBC instance with embedded ROM pre-loaded
    #[must_use]
    pub fn new() -> Self {
        let uart = Rc::new(RefCell::new(Uart16550::new()));
        let cfcard = Rc::new(RefCell::new(CfCard::new()));

        // Create CPU with full 16MB address space
        let mut cpu = Cpu::with_memory_size(16 * 1024 * 1024);

        // Install write hook for MMIO
        cpu.memory_mut().set_write_hook(sbc_write_hook);
        // Install read hook for MMIO
        cpu.memory_mut().set_read_hook(sbc_read_hook);

        // Register peripherals in global atomic pointers
        // Safety: These are only dereferenced while holding the SBC mutex
        SBC_UART_PTR.store(Rc::as_ptr(&uart) as *mut _, Ordering::Release);
        SBC_CFCARD_PTR.store(Rc::as_ptr(&cfcard) as *mut _, Ordering::Release);

        // Initialize CPU for supervisor mode
        cpu.set_sr(0x2700); // Supervisor mode, interrupts masked

        // Load embedded ROM
        let mut rom_data = vec![0xFF; ROM_SIZE];
        let len = EMBEDDED_ROM.len().min(ROM_SIZE);
        rom_data[..len].copy_from_slice(&EMBEDDED_ROM[..len]);

        let mut sbc = Self {
            cpu,
            uart,
            cfcard,
            rom_data,
            uart_output: Vec::new(),
        };

        // Sync ROM to memory (don't reset yet, let caller decide)
        sbc.sync_rom_to_memory();

        sbc
    }

    /// Performs a hardware reset
    ///
    /// This simulates the CPU reset sequence:
    /// 1. Read initial SSP from $000000
    /// 2. Read initial PC from $000004
    /// 3. Set supervisor mode, mask interrupts
    pub fn reset(&mut self) {
        // Copy ROM to CPU memory at $000000
        self.sync_rom_to_memory();

        // Read reset vectors
        let ssp = self.cpu.memory.read_long(0x000000).unwrap_or(0);
        let pc = self.cpu.memory.read_long(0x000004).unwrap_or(0);

        // Reset CPU
        self.cpu.reset();

        // Restore ROM (reset clears memory)
        self.sync_rom_to_memory();

        // Set up registers
        self.cpu.registers.set_sp(ssp);
        self.cpu.set_pc(pc);
        self.cpu.set_sr(0x2700); // Supervisor mode, all interrupts masked

        // Reset UART
        self.uart.borrow_mut().reset();
        self.uart_output.clear();
    }

    /// Syncs ROM data to CPU memory
    fn sync_rom_to_memory(&mut self) {
        // ROM repeats every 64KB within two 1MB windows:
        // $000000-$0FFFFF and $200000-$2FFFFF.
        let bases = [0x000000u32, 0x200000u32];
        for base in bases {
            for mirror in 0..16u32 {
                let addr = base + mirror * (ROM_SIZE as u32);
                let _ = self.cpu.memory.load_binary(addr, &self.rom_data);
            }
        }
        // Fix TRAP vectors: the embedded ROM binary has handler addresses that
        // are off by $12 (18 bytes) from the actual handler code. This is due
        // to a mismatch between the vector table and handler positions in the
        // assembled binary. Patch all 16 TRAP vectors to correct the offset.
        // NOTE: This fixup is for ROM boot mode. For app mode (run_app),
        // custom stubs are installed that bypass the ROM handlers entirely.
        self.fixup_trap_vectors();
    }

    /// Patches TRAP vector table to correct handler address offset
    fn fixup_trap_vectors(&mut self) {
        const TRAP_VECTOR_OFFSET: u32 = 0x12; // All handlers are +$12 from vectors
        for i in 0..16u32 {
            let vec_addr = 0x80 + i * 4; // TRAP #0 is vector 32 = offset $80
            if let Ok(old_handler) = self.cpu.memory.read_long(vec_addr) {
                // Only fix vectors that point into ROM (< $100000) and aren't $FFFFFFFF
                if old_handler < 0x100000 && old_handler != 0xFFFFFFFF {
                    let _ = self
                        .cpu
                        .memory
                        .write_long(vec_addr, old_handler + TRAP_VECTOR_OFFSET);
                }
            }
        }
    }

    /// Loads ROM from a single binary file
    ///
    /// The ROM should be 64KB or less. If smaller, it's padded with 0xFF.
    pub fn load_rom(&mut self, data: &[u8]) {
        self.rom_data.fill(0xFF);
        let len = data.len().min(ROM_SIZE);
        self.rom_data[..len].copy_from_slice(&data[..len]);
        self.sync_rom_to_memory();
    }

    /// Loads ROM from a file
    pub fn load_rom_file(&mut self, path: &Path) -> io::Result<()> {
        let data = std::fs::read(path)?;
        self.load_rom(&data);
        Ok(())
    }

    /// Loads split ROM images (rom-l.bin and rom-u.bin)
    ///
    /// The target board uses two 8-bit EEPROMs that need to be interleaved:
    /// - rom-u.bin: Upper bytes (D8-D15, even addresses)
    /// - rom-l.bin: Lower bytes (D0-D7, odd addresses)
    pub fn load_rom_split(&mut self, rom_l: &[u8], rom_u: &[u8]) {
        self.rom_data.fill(0xFF);
        let len = rom_l.len().min(rom_u.len()).min(ROM_SIZE / 2);
        for i in 0..len {
            self.rom_data[i * 2] = rom_u[i]; // High byte first (big-endian)
            self.rom_data[i * 2 + 1] = rom_l[i]; // Low byte second
        }
        self.sync_rom_to_memory();
    }

    /// Loads split ROM from files
    pub fn load_rom_split_files(&mut self, path_l: &Path, path_u: &Path) -> io::Result<()> {
        let rom_l = std::fs::read(path_l)?;
        let rom_u = std::fs::read(path_u)?;
        self.load_rom_split(&rom_l, &rom_u);
        Ok(())
    }

    /// Loads a CompactFlash disk image
    pub fn load_cf_image(&mut self, path: &Path) -> io::Result<()> {
        self.cfcard.borrow_mut().load_image(path)
    }

    /// Loads a CompactFlash disk image from bytes
    pub fn load_cf_bytes(&mut self, data: &[u8]) {
        self.cfcard.borrow_mut().load_bytes(data);
    }

    /// Ejects the CompactFlash card
    pub fn eject_cf(&mut self) {
        self.cfcard.borrow_mut().eject();
    }

    /// Returns true if a CF card is inserted
    #[must_use]
    pub fn cf_inserted(&self) -> bool {
        self.cfcard.borrow().is_inserted()
    }

    /// Loads an application binary into RAM at $E00100
    ///
    /// This is how programs are loaded for execution on the target board.
    pub fn load_app(&mut self, data: &[u8]) {
        let _ = self.cpu.memory.load_binary(APP_START, data);
    }

    /// Executes the loaded application
    ///
    /// Sets up registers as the ROM would:
    /// - SP = end of RAM ($F00000)
    /// - PC = $E00100 (app start)
    /// - D0-D7/A0-A6 = 0
    /// - Supervisor mode, interrupts enabled (IPL=0)
    ///
    /// Instead of relying on ROM TRAP handlers (which have address mismatches
    /// in the current ROM binary), this installs small handler stubs directly
    /// in RAM at $E00080. These stubs handle the core syscalls (Exit, OutChar,
    /// OutStr, InChar) by directly accessing the UART hardware.
    pub fn run_app(&mut self) {
        // Install TRAP handler stubs in RAM at $E00080 (within the 256-byte
        // system area, below the app load address at $E00100)
        self.install_trap_stubs();

        // Don't call cpu.reset() as it clears memory!
        // Just set up registers for app execution
        self.cpu.registers = crate::registers::RegisterFile::new();
        // Set supervisor mode FIRST (so set_sp sets SSP)
        self.cpu.set_sr(0x2000); // Supervisor mode, interrupts enabled
                                 // Now set stack pointer (will set SSP since we're in supervisor mode)
        self.cpu.registers.set_sp(INITIAL_SP);
        self.cpu.set_pc(APP_START);
        self.cpu.resume();
    }

    /// Installs M68K TRAP handler stubs in RAM for app execution.
    ///
    /// These stubs bypass the ROM's TRAP handlers entirely, directly
    /// implementing the core syscalls. Stubs are placed at $E00080-$E000FF.
    fn install_trap_stubs(&mut self) {
        // Base address for stubs (in system variable area)
        let stub_base: u32 = 0xE00080;
        let mut addr = stub_base;

        // Helper to write a word and advance
        let mem = &mut self.cpu.memory;

        // ---- TRAP #0: Exit ----
        // STOP #$2700  (halt CPU, supervisor mode, interrupts masked)
        let trap0_addr = addr;
        let _ = mem.write_word(addr, 0x4E72);
        addr += 2; // STOP
        let _ = mem.write_word(addr, 0x2700);
        addr += 2; // #$2700

        // ---- TRAP #2: OutChar (D0.B = character) ----
        // LEA.L $A00000,A1       ; UART base
        // .wait: BTST #5,10(A1)  ; check LSR THRE bit
        //        BEQ.S .wait     ; loop until ready
        // MOVE.B D0,(A1)         ; write char to THR
        // RTE
        let trap2_addr = addr;
        let _ = mem.write_word(addr, 0x43F9);
        addr += 2; // LEA.L
        let _ = mem.write_long(addr, 0x00A00000);
        addr += 4; // $A00000
                   // .wait:
        let _wait_addr = addr;
        let _ = mem.write_word(addr, 0x0829);
        addr += 2; // BTST #imm,(d,An)
        let _ = mem.write_word(addr, 0x0005);
        addr += 2; // bit #5
        let _ = mem.write_word(addr, 0x0005);
        addr += 2; // offset 5 (LSR)
        let _ = mem.write_word(addr, 0x67F8);
        addr += 2; // BEQ.S -8 (back to BTST)
        let _ = mem.write_word(addr, 0x1280);
        addr += 2; // MOVE.B D0,(A1)
        let _ = mem.write_word(addr, 0x4E73);
        addr += 2; // RTE

        // ---- TRAP #3: OutStr (A0 = null-terminated string) ----
        // MOVEM.L D0/A0-A1,-(SP) ; save regs
        // LEA.L $A00000,A1       ; UART base
        // .loop: MOVE.B (A0)+,D0 ; get next char
        //        BEQ.S .done     ; null = end
        // .twait: BTST #5,10(A1) ; check THRE
        //         BEQ.S .twait   ; wait
        //  MOVE.B D0,(A1)        ; write char
        //  BRA.S .loop
        // .done: MOVEM.L (SP)+,D0/A0-A1
        //        RTE
        let trap3_addr = addr;
        let _ = mem.write_word(addr, 0x48E7);
        addr += 2; // MOVEM.L ...,-(SP)
        let _ = mem.write_word(addr, 0x80C0);
        addr += 2; // D0/A0-A1
        let _ = mem.write_word(addr, 0x43F9);
        addr += 2; // LEA.L
        let _ = mem.write_long(addr, 0x00A00000);
        addr += 4; // $A00000
                   // .loop:
        let _loop_addr = addr;
        let _ = mem.write_word(addr, 0x1018);
        addr += 2; // MOVE.B (A0)+,D0
        let _ = mem.write_word(addr, 0x670C);
        addr += 2; // BEQ.S .done (+12)
                   // .twait:
        let _ = mem.write_word(addr, 0x0829);
        addr += 2; // BTST #imm,(d,An)
        let _ = mem.write_word(addr, 0x0005);
        addr += 2; // bit #5
        let _ = mem.write_word(addr, 0x0005);
        addr += 2; // offset 5 (LSR)
        let _ = mem.write_word(addr, 0x67F8);
        addr += 2; // BEQ.S .twait (-8)
        let _ = mem.write_word(addr, 0x1280);
        addr += 2; // MOVE.B D0,(A1)
        let _ = mem.write_word(addr, 0x60F0);
        addr += 2; // BRA.S .loop (back to MOVE.B (A0)+)
                   // .done:
        let _ = mem.write_word(addr, 0x4CDF);
        addr += 2; // MOVEM.L (SP)+,...
        let _ = mem.write_word(addr, 0x0301);
        addr += 2; // D0/A0-A1
        let _ = mem.write_word(addr, 0x4E73);
        addr += 2; // RTE

        // ---- TRAP #5: InChar (returns D0.B) ----
        // LEA.L $A00000,A1
        // .wait: BTST #0,10(A1) ; check LSR DR bit (data ready)
        //        BEQ.S .wait
        // MOVE.B (A1),D0        ; read RHR
        // RTE
        let trap5_addr = addr;
        let _ = mem.write_word(addr, 0x43F9);
        addr += 2;
        let _ = mem.write_long(addr, 0x00A00000);
        addr += 4;
        let _ = mem.write_word(addr, 0x0829);
        addr += 2; // BTST
        let _ = mem.write_word(addr, 0x0000);
        addr += 2; // bit #0
        let _ = mem.write_word(addr, 0x0005);
        addr += 2; // offset 5 (LSR)
        let _ = mem.write_word(addr, 0x67F8);
        addr += 2; // BEQ.S -8
        let _ = mem.write_word(addr, 0x1011);
        addr += 2; // MOVE.B (A1),D0
        let _ = mem.write_word(addr, 0x4E73);
        addr += 2; // RTE

        // Set TRAP vectors in the exception vector table.
        // IMPORTANT: use load_binary (not write_long) because the vector table
        // is in the ROM region ($000080-$00009F) and write_long goes through
        // the write hook which blocks writes to ROM addresses.
        fn write_vec(mem: &mut crate::memory::Memory, vec_addr: u32, handler: u32) {
            let bytes = handler.to_be_bytes();
            let _ = mem.load_binary(vec_addr, &bytes);
        }
        write_vec(mem, 0x80, trap0_addr); // TRAP #0 = Exit
        write_vec(mem, 0x84, trap0_addr); // TRAP #1 = halt too
        write_vec(mem, 0x88, trap2_addr); // TRAP #2 = OutChar
        write_vec(mem, 0x8C, trap3_addr); // TRAP #3 = OutStr
        write_vec(mem, 0x94, trap5_addr); // TRAP #5 = InChar

        // Set OUTCH_VEC for any code that uses indirect calls via JSR
        let _ = mem.write_long(OUTCH_VEC, trap2_addr);
    }

    /// Gets a reference to the UART for terminal I/O
    #[must_use]
    pub fn uart(&self) -> Rc<RefCell<Uart16550>> {
        Rc::clone(&self.uart)
    }

    /// Gets a reference to the CF card
    #[must_use]
    pub fn cfcard(&self) -> Rc<RefCell<CfCard>> {
        Rc::clone(&self.cfcard)
    }

    /// Returns true if the CPU is halted
    #[must_use]
    pub fn is_halted(&self) -> bool {
        self.cpu.is_halted()
    }

    /// Returns the current program counter
    #[must_use]
    pub fn pc(&self) -> u32 {
        self.cpu.pc()
    }

    /// Returns the current status register
    #[must_use]
    pub fn sr(&self) -> u16 {
        self.cpu.sr()
    }

    /// Returns the LED state from UART MCR
    #[must_use]
    pub fn led_state(&self) -> bool {
        self.uart.borrow().led_state()
    }

    /// Returns total cycles executed
    #[must_use]
    pub fn cycles(&self) -> u64 {
        self.cpu.total_cycles()
    }

    /// Sends a character to the UART receive buffer (from terminal)
    pub fn send_char(&mut self, ch: u8) {
        self.uart.borrow_mut().push_rx(ch);
    }

    /// Receives a character from the UART transmit buffer (to terminal)
    /// Drains from the accumulated output buffer first, then checks TX FIFO.
    pub fn recv_char(&mut self) -> Option<u8> {
        if !self.uart_output.is_empty() {
            Some(self.uart_output.remove(0))
        } else {
            self.uart.borrow_mut().pop_tx()
        }
    }

    /// Sends a break condition to enter the serial loader
    pub fn send_break(&mut self) {
        self.uart.borrow_mut().send_break();
    }

    /// Sets the button state
    pub fn set_button(&mut self, pressed: bool) {
        self.uart.borrow_mut().set_button(pressed);
    }

    /// Executes a single instruction
    ///
    /// Returns true if an instruction was executed, false if halted.
    pub fn step(&mut self) -> bool {
        self.handle_interrupts();
        let result = self.cpu.step();
        // Auto-drain UART TX FIFO so ROM code doesn't hang waiting for THRE
        self.drain_uart_tx();
        result
    }

    /// Drains the UART TX FIFO into the output buffer
    fn drain_uart_tx(&mut self) {
        while let Some(byte) = self.uart.borrow_mut().pop_tx() {
            self.uart_output.push(byte);
        }
    }

    /// Returns and clears the accumulated UART output
    pub fn drain_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.uart_output)
    }

    /// Returns the accumulated UART output without clearing
    pub fn peek_output(&self) -> &[u8] {
        &self.uart_output
    }

    /// Handles interrupt delivery from peripherals.
    fn handle_interrupts(&mut self) {
        let uart_pending = self.uart.borrow().interrupt_pending();
        if !uart_pending {
            return;
        }

        // UART uses autovector level 1 on the target board
        let current_ipl = ((self.cpu.sr() >> 8) & 0x7) as u8;
        if 1 > current_ipl {
            self.cpu.service_autovector_interrupt(1);
            self.uart.borrow_mut().clear_interrupt();
        }
    }

    /// Runs until halted or for a maximum number of cycles
    pub fn run(&mut self, max_cycles: u64) -> u64 {
        let start_cycles = self.cycles();
        while !self.is_halted() && (self.cycles() - start_cycles) < max_cycles {
            self.handle_interrupts();
            self.cpu.step();
            self.drain_uart_tx();
        }
        self.cycles() - start_cycles
    }

    /// Provides mutable access to the underlying CPU
    pub fn cpu_mut(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    /// Provides access to the underlying CPU
    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    /// Returns a reference to CPU registers
    pub fn registers(&self) -> &crate::registers::RegisterFile {
        &self.cpu.registers
    }

    /// Returns a mutable reference to CPU registers
    pub fn registers_mut(&mut self) -> &mut crate::registers::RegisterFile {
        &mut self.cpu.registers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbc_new() {
        let sbc = Sbc::new();
        assert!(!sbc.is_halted());
    }

    #[test]
    fn test_sbc_reset() {
        let mut sbc = Sbc::new();

        // Load a simple ROM with reset vectors
        // SSP = $00F00000, PC = $00000008
        let mut rom = vec![0u8; 64];
        rom[0..4].copy_from_slice(&[0x00, 0xF0, 0x00, 0x00]); // Initial SSP
        rom[4..8].copy_from_slice(&[0x00, 0x00, 0x00, 0x08]); // Initial PC

        sbc.load_rom(&rom);
        sbc.reset();

        assert_eq!(sbc.cpu.registers.sp(), 0x00F00000);
        assert_eq!(sbc.pc(), 0x00000008);
    }

    #[test]
    fn test_sbc_rom_mirroring() {
        let mut sbc = Sbc::new();

        let mut rom = vec![0u8; 16];
        rom[0..4].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
        sbc.load_rom(&rom);

        // ROM at $000000
        assert_eq!(sbc.cpu.memory.read_long(0x000000).unwrap(), 0xDEADBEEF);

        // ROM repeats every 64KB within the 1MB window.
        assert_eq!(sbc.cpu.memory.read_long(0x010000).unwrap(), 0xDEADBEEF);

        // ROM mirror at $200000
        assert_eq!(sbc.cpu.memory.read_long(0x200000).unwrap(), 0xDEADBEEF);
        assert_eq!(sbc.cpu.memory.read_long(0x210000).unwrap(), 0xDEADBEEF);
    }

    #[test]
    fn test_sbc_uart_tx() {
        let mut sbc = Sbc::new();

        // Write to UART THR via memory
        let _ = sbc.cpu.memory.write_byte(0xA00000, b'H');
        let _ = sbc.cpu.memory.write_byte(0xA00000, b'i');

        // Read from TX buffer
        assert_eq!(sbc.recv_char(), Some(b'H'));
        assert_eq!(sbc.recv_char(), Some(b'i'));
        assert_eq!(sbc.recv_char(), None);
    }

    #[test]
    fn test_sbc_uart_rx() {
        let mut sbc = Sbc::new();

        // Send characters to RX
        sbc.send_char(b'A');
        sbc.send_char(b'B');

        // LSR should show data ready
        let lsr = sbc.cpu.memory.read_byte(0xA0000A).unwrap();
        assert!(lsr & 0x01 != 0); // Data ready bit
    }

    #[test]
    fn test_sbc_cf_card() {
        let mut sbc = Sbc::new();

        // No card initially
        assert!(!sbc.cf_inserted());

        // Load a disk image
        sbc.load_cf_bytes(&vec![0u8; 512 * 10]);
        assert!(sbc.cf_inserted());
    }

    #[test]
    fn test_sbc_led_control() {
        let mut sbc = Sbc::new();

        assert!(!sbc.led_state());

        // Set LED via MCR (offset 8)
        let _ = sbc.cpu.memory.write_byte(0xA00008, 0x02);
        assert!(sbc.led_state());

        // Clear LED
        let _ = sbc.cpu.memory.write_byte(0xA00008, 0x00);
        assert!(!sbc.led_state());
    }

    #[test]
    fn test_sbc_load_app() {
        let mut sbc = Sbc::new();

        // Load a simple app
        let app = [0x70, 0x2A]; // MOVEQ #42, D0
        sbc.load_app(&app);

        // Verify it's in RAM at $E00100
        assert_eq!(sbc.cpu.memory.read_word(APP_START).unwrap(), 0x702A);
    }

    #[test]
    fn test_sbc_forbidden_region_reads_open_bus() {
        let mut sbc = Sbc::new();
        // $100000-$1FFFFF is ROM + CF overlap (forbidden).
        let _ = sbc.cpu.memory.write_byte(0x100000, 0xAA);
        assert_eq!(sbc.cpu.memory.read_byte(0x100000).unwrap(), 0xFF);
    }

    #[test]
    fn test_sbc_button_msr_polarity() {
        let mut sbc = Sbc::new();
        sbc.set_button(true);
        let msr = sbc.cpu.memory.read_byte(0xA0000C).unwrap();
        assert!(msr & 0x40 != 0);
    }

    #[test]
    fn test_sbc_break_inserts_zero_and_sets_lsr() {
        let mut sbc = Sbc::new();
        sbc.send_break();
        let lsr = sbc.cpu.memory.read_byte(0xA0000A).unwrap();
        assert!(lsr & 0x10 != 0);
        let byte = sbc.cpu.memory.read_byte(0xA00000).unwrap();
        assert_eq!(byte, 0);
    }

    #[test]
    fn test_sbc_rom_compatibility() {
        let mut sbc = Sbc::new();

        // Simulate loading a compatible ROM with proper reset vectors
        // Initial SSP = 0x00F00000 (end of RAM)
        // Initial PC = 0x000000C0 (ROM start)
        let mut rom = vec![0u8; ROM_SIZE];
        rom[0..4].copy_from_slice(&[0x00, 0xF0, 0x00, 0x00]); // Initial SSP
        rom[4..8].copy_from_slice(&[0x00, 0x00, 0x00, 0xC0]); // Initial PC

        // Write a simple program at 0xC0 that loads D0 and halts
        // MOVEQ #42,D0 (quick move, single instruction)
        rom[0xC0] = 0x70; // MOVEQ
        rom[0xC1] = 0x2A; // #42

        // STOP #$2700 (halt CPU with interrupts masked)
        rom[0xC2] = 0x4E; // STOP
        rom[0xC3] = 0x72;
        rom[0xC4] = 0x27; // SR value
        rom[0xC5] = 0x00;

        sbc.load_rom(&rom);
        sbc.reset();

        // Verify reset vectors loaded correctly
        assert_eq!(sbc.pc(), 0x000000C0);
        assert_eq!(sbc.registers().sp(), 0x00F00000);
        assert_eq!(sbc.sr(), 0x2700); // Supervisor mode, interrupts masked

        // Execute instructions
        let mut executed = 0;
        for _ in 0..10 {
            if !sbc.step() {
                break;
            }
            executed += 1;
        }

        // Should have executed at least MOVEQ
        assert!(executed >= 1);

        // Verify D0 contains the value we loaded
        assert_eq!(sbc.registers().d(0), 42);

        // CPU should be halted by STOP
        assert!(sbc.is_halted());
    }
}
