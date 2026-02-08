//! UART 16550 Emulation for SBC-Compatible System
//!
//! This module emulates the 16550 UART used by the target board for serial I/O.
//! The UART is memory-mapped at $A00000 with 16-bit aligned registers.
//!
//! ## Register Map (offsets from $A00000)
//!
//! | Offset | Read         | Write        | Notes                    |
//! |--------|--------------|--------------|--------------------------|
//! | 0      | RHR (data)   | THR (data)   | When DLAB=0              |
//! | 0      | DLL          | DLL          | When DLAB=1 (divisor lo) |
//! | 2      | IER          | IER          | When DLAB=0              |
//! | 2      | DLM          | DLM          | When DLAB=1 (divisor hi) |
//! | 4      | ISR          | FCR          | Interrupt/FIFO control   |
//! | 6      | LCR          | LCR          | Line control             |
//! | 8      | MCR          | MCR          | Modem control            |
//! | 10     | LSR          | -            | Line status (read-only)  |
//! | 12     | MSR          | -            | Modem status             |
//! | 14     | SPR          | SPR          | Scratchpad               |
//!
//! ## Special Uses on the Target Board
//!
//! The modem control lines are used for:
//! - MCR bit 0 (DTR): SPI COPI (Controller Out, Peripheral In)
//! - MCR bit 1 (RTS): Status LED
//! - MCR bit 2 (OUT1): SPI clock
//! - MCR bit 3 (OUT2): SPI chip select (/SS, directly directly active low)
//!
//! The modem status lines are used for:
//! - MSR bit 7 (DCD): SPI CIPO (Controller In, Peripheral Out)
//! - MSR bit 6 (RI): Button input
//! - MSR bit 5 (DSR): RTC square wave output

// Allow dead code - this module is exercised through the CLI
#![allow(dead_code)]

use std::collections::VecDeque;

/// Base address of the UART in the system memory map
pub const UART_BASE: u32 = 0x00A0_0000;

/// UART register offsets (byte offsets, though accessed as words)
pub mod regs {
    /// Receive Holding Register / Transmit Holding Register / Divisor Latch Low
    pub const RHR_THR_DLL: u32 = 0;
    /// Interrupt Enable Register / Divisor Latch High
    pub const IER_DLM: u32 = 2;
    /// Interrupt Status Register (read) / FIFO Control Register (write)
    pub const ISR_FCR: u32 = 4;
    /// Line Control Register
    pub const LCR: u32 = 6;
    /// Modem Control Register
    pub const MCR: u32 = 8;
    /// Line Status Register
    pub const LSR: u32 = 10;
    /// Modem Status Register
    pub const MSR: u32 = 12;
    /// Scratchpad Register
    pub const SPR: u32 = 14;
}

/// Line Status Register bit flags
pub mod lsr {
    /// Data Ready - RX FIFO has data
    pub const DR: u8 = 0x01;
    /// Overrun Error
    pub const OE: u8 = 0x02;
    /// Parity Error
    pub const PE: u8 = 0x04;
    /// Framing Error
    pub const FE: u8 = 0x08;
    /// Break Interrupt
    pub const BI: u8 = 0x10;
    /// Transmitter Holding Register Empty
    pub const THRE: u8 = 0x20;
    /// Transmitter Empty
    pub const TEMT: u8 = 0x40;
    /// FIFO Data Error
    pub const FIFO_ERR: u8 = 0x80;
}

/// Modem Control Register bit flags (board-specific meanings)
pub mod mcr {
    /// SPI COPI (Data out to peripherals)
    pub const COPI: u8 = 0x01;
    /// Status LED
    pub const LED: u8 = 0x02;
    /// SPI Clock
    pub const CLK: u8 = 0x04;
    /// SPI Chip Select (/SS, directly active low)
    pub const NSS: u8 = 0x08;
    /// Loopback mode
    pub const LOOP: u8 = 0x10;
}

/// Modem Status Register bit flags (board-specific meanings)
pub mod msr {
    /// Delta CTS
    pub const DCTS: u8 = 0x01;
    /// Delta DSR (SQW changed)
    pub const DDSR: u8 = 0x02;
    /// Trailing Edge RI (button press detected)
    pub const TERI: u8 = 0x04;
    /// Delta DCD
    pub const DDCD: u8 = 0x08;
    /// CTS
    pub const CTS: u8 = 0x10;
    /// DSR - RTC square wave output
    pub const SQW: u8 = 0x20;
    /// RI - Button input (active high, 1 = pressed)
    pub const BTN: u8 = 0x40;
    /// DCD - SPI CIPO (Data in from peripherals)
    pub const CIPO: u8 = 0x80;
}

/// Interrupt Enable Register bits
pub mod ier {
    /// Enable Received Data Available Interrupt
    pub const ERBFI: u8 = 0x01;
    /// Enable Transmitter Holding Register Empty Interrupt
    pub const ETBEI: u8 = 0x02;
    /// Enable Receiver Line Status Interrupt
    pub const ELSI: u8 = 0x04;
    /// Enable Modem Status Interrupt
    pub const EDSSI: u8 = 0x08;
}

/// Line Control Register bits
pub mod lcr {
    /// Word Length Select bit 0
    pub const WLS0: u8 = 0x01;
    /// Word Length Select bit 1
    pub const WLS1: u8 = 0x02;
    /// Number of Stop Bits
    pub const STB: u8 = 0x04;
    /// Parity Enable
    pub const PEN: u8 = 0x08;
    /// Even Parity Select
    pub const EPS: u8 = 0x10;
    /// Stick Parity
    pub const STICK: u8 = 0x20;
    /// Set Break
    pub const SBRK: u8 = 0x40;
    /// Divisor Latch Access Bit
    pub const DLAB: u8 = 0x80;
}

/// FIFO size for TX and RX buffers
const FIFO_SIZE: usize = 16;

/// Number of LSR reads to keep a break pulse asserted.
const BREAK_PULSE_READS: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpiMode {
    Idle,
    Read,
    Write,
}

/// Minimal DS3234 RTC SPI emulation used by the ROM.
#[derive(Clone)]
struct RtcSpi {
    regs: [u8; 0x20],
    active: bool,
    mode: SpiMode,
    addr: u8,
    bit_index: u8,
    shift_in: u8,
    shift_out: u8,
}

impl RtcSpi {
    const fn bcd(value: u8) -> u8 {
        ((value / 10) << 4) | (value % 10)
    }

    const fn new() -> Self {
        let mut regs = [0u8; 0x20];

        // Default to a valid, stable date/time (2020-07-07 19:14:36, weekday=2).
        regs[0x00] = Self::bcd(36); // seconds
        regs[0x01] = Self::bcd(14); // minutes
        regs[0x02] = Self::bcd(19); // hours (24-hour)
        regs[0x03] = Self::bcd(2); // weekday (1-7)
        regs[0x04] = Self::bcd(7); // date
        regs[0x05] = Self::bcd(7); // month (century bit cleared)
        regs[0x06] = Self::bcd(20); // year
        regs[0x0F] = 0x00; // control/status (OSF clear)

        Self {
            regs,
            active: false,
            mode: SpiMode::Idle,
            addr: 0,
            bit_index: 0,
            shift_in: 0,
            shift_out: 0xFF,
        }
    }

    const fn start_transfer(&mut self) {
        self.active = true;
        self.mode = SpiMode::Idle;
        self.addr = 0;
        self.bit_index = 0;
        self.shift_in = 0;
        self.shift_out = 0xFF;
    }

    const fn end_transfer(&mut self) {
        self.active = false;
        self.mode = SpiMode::Idle;
        self.addr = 0;
        self.bit_index = 0;
        self.shift_in = 0;
        self.shift_out = 0xFF;
    }

    const fn on_clock_rising(&mut self, copi_bit: u8) -> u8 {
        if !self.active {
            return 1;
        }

        let out_bit = (self.shift_out >> (7 - self.bit_index)) & 1;
        self.shift_in = (self.shift_in << 1) | (copi_bit & 1);
        self.bit_index += 1;

        if self.bit_index == 8 {
            self.finish_byte();
        }

        out_bit
    }

    const fn finish_byte(&mut self) {
        match self.mode {
            SpiMode::Idle => {
                let cmd = self.shift_in;
                self.addr = cmd & 0x7F;
                if cmd & 0x80 != 0 {
                    self.mode = SpiMode::Write;
                    self.shift_out = 0xFF;
                } else {
                    self.mode = SpiMode::Read;
                    self.shift_out = self.regs[self.addr as usize & 0x1F];
                    self.addr = self.addr.wrapping_add(1);
                }
            }
            SpiMode::Read => {
                self.shift_out = self.regs[self.addr as usize & 0x1F];
                self.addr = self.addr.wrapping_add(1);
            }
            SpiMode::Write => {
                self.regs[self.addr as usize & 0x1F] = self.shift_in;
                self.addr = self.addr.wrapping_add(1);
                self.shift_out = 0xFF;
            }
        }

        self.bit_index = 0;
        self.shift_in = 0;
    }
}

/// 16550 UART emulation state
#[derive(Clone)]
pub struct Uart16550 {
    /// Receive FIFO
    rx_fifo: VecDeque<u8>,
    /// Transmit FIFO
    tx_fifo: VecDeque<u8>,

    /// Interrupt Enable Register
    ier: u8,
    /// FIFO Control Register (write-only, but we track state)
    fcr: u8,
    /// Line Control Register
    lcr: u8,
    /// Modem Control Register
    mcr: u8,
    /// Scratchpad Register
    spr: u8,

    /// Divisor latch low byte
    dll: u8,
    /// Divisor latch high byte
    dlm: u8,

    /// Break condition active (emulated as a short pulse)
    break_active: bool,
    /// Remaining LSR reads before a break pulse deasserts
    break_reads_remaining: u8,
    /// Button state (false = not pressed, true = pressed)
    button_pressed: bool,
    /// Button press edge detector
    button_edge: bool,

    /// LED state (derived from MCR)
    led_on: bool,

    /// Interrupt pending flag
    interrupt_pending: bool,

    /// RTC SPI emulation
    spi: RtcSpi,
    /// Current CIPO line level as seen by the UART (inverted)
    spi_cipo_inverted: bool,
}

impl Default for Uart16550 {
    fn default() -> Self {
        Self::new()
    }
}

impl Uart16550 {
    /// Creates a new UART in reset state
    #[must_use]
    pub fn new() -> Self {
        Self {
            rx_fifo: VecDeque::with_capacity(FIFO_SIZE),
            tx_fifo: VecDeque::with_capacity(FIFO_SIZE),
            ier: 0,
            fcr: 0,
            lcr: 0,
            mcr: 0,
            spr: 0,
            dll: 0,
            dlm: 0,
            break_active: false,
            break_reads_remaining: 0,
            button_pressed: false,
            button_edge: false,
            led_on: false,
            interrupt_pending: false,
            spi: RtcSpi::new(),
            spi_cipo_inverted: true,
        }
    }

    /// Resets the UART to power-on state
    pub fn reset(&mut self) {
        self.rx_fifo.clear();
        self.tx_fifo.clear();
        self.ier = 0;
        self.fcr = 0;
        self.lcr = 0;
        self.mcr = 0;
        self.spr = 0;
        self.dll = 0;
        self.dlm = 0;
        self.break_active = false;
        self.break_reads_remaining = 0;
        self.button_edge = false;
        self.interrupt_pending = false;
        self.spi = RtcSpi::new();
        self.spi_cipo_inverted = true;
    }

    /// Returns true if the status LED should be on
    ///
    /// On the target board, the LED is directly controlled by MCR bit 1 (RTS).
    /// A bit value of 1 drives the output low, which turns the LED on.
    #[must_use]
    pub const fn led_state(&self) -> bool {
        self.led_on
    }

    /// Returns true if an interrupt is pending
    #[must_use]
    pub const fn interrupt_pending(&self) -> bool {
        self.interrupt_pending
    }

    /// Clears the interrupt pending flag (call after servicing)
    pub const fn clear_interrupt(&mut self) {
        self.interrupt_pending = false;
    }

    /// Returns true if there is data waiting to be transmitted
    #[must_use]
    pub fn has_tx_data(&self) -> bool {
        !self.tx_fifo.is_empty()
    }

    /// Gets the next byte from the transmit FIFO
    ///
    /// Call this from the terminal to get characters to display.
    pub fn pop_tx(&mut self) -> Option<u8> {
        self.tx_fifo.pop_front()
    }

    /// Pushes a byte into the receive FIFO
    ///
    /// Call this from the terminal when the user types a character.
    pub fn push_rx(&mut self, byte: u8) {
        if self.rx_fifo.len() < FIFO_SIZE {
            self.rx_fifo.push_back(byte);
            // Check if RX interrupt should fire
            if self.ier & ier::ERBFI != 0 {
                self.interrupt_pending = true;
            }
        }
        // If FIFO full, character is dropped (overrun)
    }

    /// Sends a break condition (enters the serial loader)
    pub fn send_break(&mut self) {
        self.break_active = true;
        self.break_reads_remaining = BREAK_PULSE_READS;
        // A break condition pushes a zero byte into the RX FIFO.
        if self.rx_fifo.len() < FIFO_SIZE {
            self.rx_fifo.push_back(0);
        }
        // Break triggers line status interrupt if enabled
        if self.ier & ier::ELSI != 0 {
            self.interrupt_pending = true;
        }
    }

    /// Sets the button state
    ///
    /// The button is read via MSR bit 6 (RI). On the target board, the UART sees
    /// a high level when the button is pressed.
    pub const fn set_button(&mut self, pressed: bool) {
        if pressed && !self.button_pressed {
            // Rising edge - button just pressed
            self.button_edge = true;
            if self.ier & ier::EDSSI != 0 {
                self.interrupt_pending = true;
            }
        }
        self.button_pressed = pressed;
    }

    /// Reads from a UART register
    ///
    /// `offset` is the byte offset from the UART base address.
    pub fn read(&mut self, offset: u32) -> u8 {
        match offset & 0xF {
            0 => {
                // RHR or DLL
                if self.lcr & lcr::DLAB != 0 {
                    self.dll
                } else {
                    // Read from RX FIFO
                    self.rx_fifo.pop_front().unwrap_or(0)
                }
            }
            2 => {
                // IER or DLM
                if self.lcr & lcr::DLAB != 0 {
                    self.dlm
                } else {
                    self.ier
                }
            }
            4 => {
                // ISR (Interrupt Status Register)
                self.read_isr()
            }
            6 => {
                // LCR
                self.lcr
            }
            8 => {
                // MCR
                self.mcr
            }
            10 => {
                // LSR
                self.read_lsr()
            }
            12 => {
                // MSR
                self.read_msr()
            }
            14 => {
                // SPR
                self.spr
            }
            _ => 0xFF,
        }
    }

    /// Writes to a UART register
    ///
    /// `offset` is the byte offset from the UART base address.
    pub fn write(&mut self, offset: u32, value: u8) {
        match offset & 0xF {
            0 => {
                // THR or DLL
                if self.lcr & lcr::DLAB != 0 {
                    self.dll = value;
                } else {
                    // Write to TX FIFO
                    if self.tx_fifo.len() < FIFO_SIZE {
                        self.tx_fifo.push_back(value);
                    }
                }
            }
            2 => {
                // IER or DLM
                if self.lcr & lcr::DLAB != 0 {
                    self.dlm = value;
                } else {
                    self.ier = value & 0x0F; // Only bits 0-3 are valid
                }
            }
            4 => {
                // FCR (write-only)
                self.write_fcr(value);
            }
            6 => {
                // LCR
                self.lcr = value;
            }
            8 => {
                // MCR
                let new_mcr = value & 0x1F; // Only bits 0-4 are valid
                let prev_mcr = self.mcr;
                self.mcr = new_mcr;
                // Update LED state
                self.led_on = (new_mcr & mcr::LED) != 0;
                self.handle_mcr_change(prev_mcr, new_mcr);
            }
            14 => {
                // SPR
                self.spr = value;
            }
            _ => {
                // LSR and MSR are read-only, ignore writes
            }
        }
    }

    /// Reads the Line Status Register
    fn read_lsr(&mut self) -> u8 {
        let mut lsr = 0u8;

        // THRE (bit 5): TX Holding Register Empty - set if FIFO has space
        // TEMT (bit 6): TX Empty - set if FIFO is completely empty
        if self.tx_fifo.len() < FIFO_SIZE {
            lsr |= lsr::THRE; // Ready to accept more data
        }
        if self.tx_fifo.is_empty() {
            lsr |= lsr::TEMT; // Transmitter completely empty
        }

        if !self.rx_fifo.is_empty() {
            lsr |= lsr::DR; // Data ready
        }

        if self.break_active {
            lsr |= lsr::BI; // Break interrupt
            if self.break_reads_remaining > 0 {
                self.break_reads_remaining -= 1;
                if self.break_reads_remaining == 0 {
                    self.break_active = false;
                }
            }
        }

        lsr
    }

    /// Reads the Modem Status Register
    const fn read_msr(&mut self) -> u8 {
        let mut msr = 0u8;

        // Button (RI) - active high
        if self.button_pressed {
            msr |= msr::BTN;
        }

        // Button press edge (TERI)
        if self.button_edge {
            msr |= msr::TERI;
            self.button_edge = false; // Clear on read
        }

        // CTS/DSR/DCD - not connected, default high
        msr |= msr::CTS;
        if self.spi_cipo_inverted {
            msr |= msr::CIPO;
        }

        msr
    }

    /// Reads the Interrupt Status Register
    fn read_isr(&mut self) -> u8 {
        // Bit 0 = 0 means interrupt pending, 1 means no interrupt
        // For now, simple implementation
        if self.interrupt_pending {
            // Return interrupt source (simplified)
            if !self.rx_fifo.is_empty() && (self.ier & ier::ERBFI != 0) {
                0x04 // Received data available
            } else if self.break_active && (self.ier & ier::ELSI != 0) {
                0x06 // Line status (break)
            } else {
                0x01 // No interrupt (clear pending)
            }
        } else {
            0x01 // No interrupt pending
        }
    }

    fn handle_mcr_change(&mut self, prev: u8, new: u8) {
        let prev_nss_active = (prev & mcr::NSS) != 0;
        let new_nss_active = (new & mcr::NSS) != 0;

        if new_nss_active && !prev_nss_active {
            self.spi.start_transfer();
            self.spi_cipo_inverted = true;
        } else if !new_nss_active && prev_nss_active {
            self.spi.end_transfer();
            self.spi_cipo_inverted = true;
        }

        // Actual clock line is inverted (bit cleared means line high).
        let prev_clk_high = (prev & mcr::CLK) == 0;
        let new_clk_high = (new & mcr::CLK) == 0;

        if !prev_clk_high && new_clk_high && new_nss_active {
            // Rising edge: sample COPI and update CIPO.
            let copi_bit = u8::from((new & mcr::COPI) == 0);
            let out_bit = self.spi.on_clock_rising(copi_bit);
            self.spi_cipo_inverted = out_bit == 0;
        }
    }

    /// Writes to the FIFO Control Register
    fn write_fcr(&mut self, value: u8) {
        self.fcr = value;

        // Bit 1: Clear RX FIFO
        if value & 0x02 != 0 {
            self.rx_fifo.clear();
        }

        // Bit 2: Clear TX FIFO
        if value & 0x04 != 0 {
            self.tx_fifo.clear();
        }
    }

    /// Returns the current baud rate divisor
    #[must_use]
    pub fn divisor(&self) -> u16 {
        (u16::from(self.dlm) << 8) | u16::from(self.dll)
    }

    /// Returns the calculated baud rate given a clock frequency
    #[must_use]
    pub fn baud_rate(&self, clock_hz: u32) -> u32 {
        let div = self.divisor();
        if div == 0 {
            0
        } else {
            clock_hz / (16 * u32::from(div))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uart_new() {
        let uart = Uart16550::new();
        assert!(!uart.led_state());
        assert!(!uart.interrupt_pending());
        assert!(!uart.has_tx_data());
    }

    #[test]
    fn test_uart_tx_fifo() {
        let mut uart = Uart16550::new();

        // Write to THR (DLAB must be 0)
        uart.write(regs::RHR_THR_DLL, b'H');
        uart.write(regs::RHR_THR_DLL, b'i');

        assert!(uart.has_tx_data());
        assert_eq!(uart.pop_tx(), Some(b'H'));
        assert_eq!(uart.pop_tx(), Some(b'i'));
        assert_eq!(uart.pop_tx(), None);
    }

    #[test]
    fn test_uart_rx_fifo() {
        let mut uart = Uart16550::new();

        uart.push_rx(b'A');
        uart.push_rx(b'B');

        // LSR should show data ready
        let lsr = uart.read(regs::LSR);
        assert!(lsr & lsr::DR != 0);

        // Read from RHR
        assert_eq!(uart.read(regs::RHR_THR_DLL), b'A');
        assert_eq!(uart.read(regs::RHR_THR_DLL), b'B');
        assert_eq!(uart.read(regs::RHR_THR_DLL), 0); // Empty
    }

    #[test]
    fn test_uart_divisor_latch() {
        let mut uart = Uart16550::new();

        // Set DLAB
        uart.write(regs::LCR, lcr::DLAB);

        // Write divisor
        uart.write(regs::RHR_THR_DLL, 0x0C); // DLL
        uart.write(regs::IER_DLM, 0x00); // DLM

        assert_eq!(uart.divisor(), 0x000C);

        // Clear DLAB
        uart.write(regs::LCR, 0);

        // Now writes go to THR
        uart.write(regs::RHR_THR_DLL, b'X');
        assert_eq!(uart.pop_tx(), Some(b'X'));
    }

    #[test]
    fn test_uart_led_control() {
        let mut uart = Uart16550::new();

        assert!(!uart.led_state());

        // Set LED bit in MCR
        uart.write(regs::MCR, mcr::LED);
        assert!(uart.led_state());

        // Clear LED bit
        uart.write(regs::MCR, 0);
        assert!(!uart.led_state());
    }

    #[test]
    fn test_uart_button() {
        let mut uart = Uart16550::new();

        // Button not pressed - RI bit should be clear (active high)
        let msr = uart.read(regs::MSR);
        assert!(msr & msr::BTN == 0);

        // Press button
        uart.set_button(true);
        let msr = uart.read(regs::MSR);
        assert!(msr & msr::BTN != 0); // Active high
        assert!(msr & msr::TERI != 0); // Edge detected

        // Read again - edge should be cleared
        let msr = uart.read(regs::MSR);
        assert!(msr & msr::TERI == 0);
    }

    #[test]
    fn test_uart_lsr_tx_ready() {
        let mut uart = Uart16550::new();

        // TX should always be ready
        let lsr = uart.read_lsr();
        assert!(lsr & lsr::THRE != 0);
        assert!(lsr & lsr::TEMT != 0);
    }

    #[test]
    fn test_uart_scratchpad() {
        let mut uart = Uart16550::new();

        uart.write(regs::SPR, 0x42);
        assert_eq!(uart.read(regs::SPR), 0x42);

        uart.write(regs::SPR, 0xAB);
        assert_eq!(uart.read(regs::SPR), 0xAB);
    }

    #[test]
    fn test_uart_break_pulse() {
        let mut uart = Uart16550::new();
        uart.write(regs::IER_DLM, ier::ELSI);

        uart.send_break();
        assert!(uart.interrupt_pending());

        let lsr = uart.read(regs::LSR);
        assert!(lsr & lsr::BI != 0);

        // A break inserts a zero byte into RX.
        assert_eq!(uart.read(regs::RHR_THR_DLL), 0);

        // After a few LSR reads, the break should deassert.
        for _ in 0..(BREAK_PULSE_READS + 1) {
            let _ = uart.read(regs::LSR);
        }
        let lsr = uart.read(regs::LSR);
        assert!(lsr & lsr::BI == 0);
    }
}
