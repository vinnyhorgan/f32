//! M68K CPU Core
//!
//! This module implements the M68K CPU state machine. The CPU is responsible for:
//!
//! - Maintaining register state (data, address, PC, SR)
//! - Fetching instructions from memory
//! - Decoding instructions
//! - Dispatching to the appropriate instruction handler
//! - Managing the fetch-decode-execute cycle
//!
//! The M68K is a big-endian, 32-bit processor with a 24-bit address space.
//!
//! # Instruction Cycle
//!
//! 1. **Fetch**: Read the instruction word from the address in PC
//! 2. **Decode**: Parse the opcode to determine the operation and operands
//! 3. **Execute**: Perform the operation and update flags/registers
//! 4. **Repeat**: PC is updated by the instruction handler

use crate::instructions::{InstructionResult, Instructions};
use crate::memory::Memory;
use crate::registers::{FlagOps, RegisterFile};
use std::fmt;

/// M68K CPU state.
///
/// The complete CPU state including registers and memory interface.
pub struct Cpu {
    /// The register file (D0-D7, A0-A7, PC, SR)
    pub registers: RegisterFile,
    /// The memory bus.
    pub memory: Memory,
    /// Whether the CPU is halted.
    halted: bool,
    /// Total number of cycles executed.
    cycles: u64,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    /// Creates a new CPU with default memory size (64KB).
    #[must_use]
    pub fn new() -> Self {
        Self::with_memory_size(crate::memory::DEFAULT_MEMORY_SIZE)
    }

    /// Creates a new CPU with the specified memory size.
    ///
    /// # Panics
    /// Panics if `size` exceeds 16MB.
    #[must_use]
    pub fn with_memory_size(size: usize) -> Self {
        Self {
            registers: RegisterFile::new(),
            memory: Memory::new(size),
            halted: false,
            cycles: 0,
        }
    }

    /// Resets the CPU to initial state.
    ///
    /// - All registers are cleared to zero
    ///   PC is set to 0 (in real hardware, this would be loaded from the reset vector)
    /// - SR is set to supervisor mode (S bit = 1) as per M68K reset behavior
    /// - Memory is cleared to zero for test isolation
    pub fn reset(&mut self) {
        self.registers = RegisterFile::new();
        // M68K starts in supervisor mode after reset
        self.registers.set_sr(0x2000); // Set S bit (supervisor mode)
        self.memory.clear();
        self.halted = false;
        self.cycles = 0;
    }

    /// Returns the current program counter.
    #[must_use]
    pub const fn pc(&self) -> u32 {
        self.registers.pc
    }

    /// Sets the program counter.
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn set_pc(&mut self, value: u32) {
        self.registers.set_pc(value);
    }

    /// Returns the current status register.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn sr(&self) -> u16 {
        self.registers.sr
    }

    /// Sets the status register.
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn set_sr(&mut self, value: u16) {
        self.registers.set_sr(value);
    }

    /// Returns true if the CPU is halted.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub const fn is_halted(&self) -> bool {
        self.halted
    }

    /// Halts the CPU.
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn halt(&mut self) {
        self.halted = true;
    }

    /// Resumes the CPU.
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn resume(&mut self) {
        self.halted = false;
    }

    /// Returns the total number of cycles executed.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)] // Used in unit tests
    pub const fn total_cycles(&self) -> u64 {
        self.cycles
    }

    /// Returns a mutable reference to the memory subsystem.
    #[must_use]
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn memory_mut(&mut self) -> &mut Memory {
        &mut self.memory
    }

    /// Executes a single CPU instruction.
    ///
    /// This performs the complete fetch-decode-execute cycle:
    /// 1. Fetch the instruction word from PC
    /// 2. Decode the opcode
    /// 3. Dispatch to the appropriate instruction handler
    /// 4. Update PC and cycle count
    ///
    /// # Returns
    /// `true` if an instruction was executed, `false` if the CPU is halted.
    ///
    /// # Panics
    /// Panics if the instruction fetch fails (address out of bounds).
    pub fn step(&mut self) -> bool {
        if self.halted {
            return false;
        }

        // Fetch the instruction word
        let current_pc = self.registers.pc;
        let opcode = self.memory.read_word_unchecked(current_pc);
        let initial_pc = current_pc;

        // Dispatch to instruction handler
        let result = self.execute(opcode);
        self.registers.set_pc(result.pc);
        self.cycles += result.cycles as u64;

        // Handle exceptions if triggered
        if result.exception != 0 {
            // For most exceptions, push the address of the NEXT instruction
            // This allows execution to continue after the exception is handled
            self.trigger_exception(result.exception, result.pc);
        }

        // Handle STOP instruction (halt flag set)
        if result.halt {
            self.halted = true;
            return true; // Instruction was executed, but CPU is now halted
        }

        // Check if PC didn't advance (illegal instruction or halt)
        if result.pc == initial_pc && result.cycles == 0 {
            self.halted = true;
            return false;
        }

        true
    }

    /// Triggers an exception by pushing state to stack and jumping to handler.
    ///
    /// This is the core M68K exception processing sequence:
    /// 1. Save current SR (for later restoration by RTE)
    /// 2. Enter supervisor mode (set S bit in SR)
    /// 3. Clear trace mode (clear T bit)
    /// 4. Push PC to supervisor stack
    /// 5. Push old SR to supervisor stack
    /// 6. Load new PC from exception vector table
    ///
    /// # Arguments
    /// * `vector` - The exception vector number (0-255)
    /// * `exception_pc` - The PC to push (usually current instruction address)
    fn trigger_exception(&mut self, vector: u8, exception_pc: u32) {
        // Save the old SR before modifying it
        let old_sr = self.registers.sr;

        // Get the SSP before any mode switch (using mode-aware getter)
        let ssp = self.registers.get_ssp();

        // Enter supervisor mode and clear trace
        // Set S bit (0x2000), clear T bit (0x8000)
        // Use set_sr to properly handle stack pointer swap
        let new_sr = (old_sr | 0x2000) & !0x8000;
        self.trigger_exception_with_sr(vector, exception_pc, old_sr, new_sr, ssp);
    }

    /// Triggers an exception using an explicit new SR value.
    ///
    /// This is used for autovector interrupts where the IPL should be updated.
    fn trigger_exception_with_sr(
        &mut self,
        vector: u8,
        exception_pc: u32,
        old_sr: u16,
        new_sr: u16,
        ssp: u32,
    ) {
        // Set SR (switch to supervisor, clear trace, update IPL as needed)
        self.registers.set_sr(new_sr);

        // Push exception frame onto SSP
        let mut new_ssp = ssp;

        // Push PC to stack (long) - the address of the instruction that caused the exception
        new_ssp = new_ssp.wrapping_sub(4);
        let _ = self.memory.write_long(new_ssp, exception_pc);

        // Push old SR to stack (word)
        new_ssp = new_ssp.wrapping_sub(2);
        let _ = self.memory.write_word(new_ssp, old_sr);

        // Update SSP (which is now A7 since we're in supervisor mode)
        self.registers.set_a(7, new_ssp);

        // Load new PC from vector table (vector * 4)
        let vector_addr = (vector as u32) * 4;
        let new_pc = self.memory.read_long(vector_addr).unwrap_or(0);

        self.registers.set_pc(new_pc);
    }

    /// Services an autovector interrupt at the given level (1-7).
    ///
    /// This clears the halted state, updates the IPL, and jumps to the
    /// corresponding autovector (vector 24 + level).
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn service_autovector_interrupt(&mut self, level: u8) {
        if !(1..=7).contains(&level) {
            return;
        }

        let old_sr = self.registers.sr;
        let ssp = self.registers.get_ssp();

        // Update SR: set supervisor, clear trace, and set IPL to interrupt level.
        let mut new_sr = (old_sr | 0x2000) & !0x8000;
        new_sr = (new_sr & !0x0700) | ((level as u16) << 8);

        // Autovector number is 24 + level.
        let vector = 24 + level;
        self.halted = false;
        self.trigger_exception_with_sr(vector, self.registers.pc, old_sr, new_sr, ssp);
    }

    /// Executes multiple instructions until a condition is met.
    ///
    /// # Arguments
    /// * `max_instructions` - Maximum number of instructions to execute.
    ///   Use `u64::MAX` for no limit.
    ///
    /// # Returns
    /// The number of instructions executed.
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn run(&mut self, max_instructions: u64) -> u64 {
        let mut count = 0;
        while count < max_instructions && !self.halted {
            if !self.step() {
                break;
            }
            count += 1;
        }
        count
    }

    /// Executes an instruction by opcode.
    ///
    /// This decodes the opcode and dispatches to the appropriate instruction
    /// handler in the `Instructions` module.
    // Allow clippy::too_many_arguments: instruction handlers mirror M68K operand shapes.
    #[allow(clippy::too_many_arguments)]
    fn execute(&mut self, opcode: u16) -> InstructionResult {
        // Save current PC for instructions that need it (e.g., branches)
        let current_pc = self.registers.pc;
        // Most instructions expect PC to point to the first extension word
        let pc = current_pc + 2;

        // Extract key bit fields for decoding
        let top_nibble = (opcode >> 12) & 0x0F;

        // ==================== OPCODE FAMILY 0: Bit Manipulation, MOVEP, Immediate ====================
        if top_nibble == 0x0 {
            // ORI to CCR: 0000 0000 0011 1100
            if opcode == 0x003C {
                return Instructions::ori_to_ccr(&mut self.registers, &self.memory, opcode, pc);
            }
            // ORI to SR: 0000 0000 0111 1100
            if opcode == 0x007C {
                return Instructions::ori_to_sr(&mut self.registers, &self.memory, opcode, pc);
            }
            // ORI: 0000 0000 ssxx xxxx (general form, not special cases above)
            if (opcode & 0xFF00) == 0x0000 {
                return Instructions::ori(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // ANDI to CCR: 0000 0010 0011 1100
            if opcode == 0x023C {
                return Instructions::andi_to_ccr(&mut self.registers, &self.memory, opcode, pc);
            }
            // ANDI to SR: 0000 0010 0111 1100
            if opcode == 0x027C {
                return Instructions::andi_to_sr(&mut self.registers, &self.memory, opcode, pc);
            }
            // ANDI: 0000 0010 ssxx xxxx (general form, not special cases above)
            if (opcode & 0xFF00) == 0x0200 {
                return Instructions::andi(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // EORI to CCR: 0000 1010 0011 1100
            if opcode == 0x0A3C {
                return Instructions::eori_to_ccr(&mut self.registers, &self.memory, opcode, pc);
            }
            // EORI to SR: 0000 1010 0111 1100
            if opcode == 0x0A7C {
                return Instructions::eori_to_sr(&mut self.registers, &self.memory, opcode, pc);
            }
            // EORI: 0000 1010 ssxx xxxx (general form, not special cases above)
            if (opcode & 0xFF00) == 0x0A00 {
                return Instructions::eori(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // MOVEP: 0000 rrr ooo 001 aaa (ooo = 100, 101, 110, 111)
            // Pattern: 0000 xxx1 xx00 1xxx
            if (opcode & 0xF138) == 0x0108 {
                return Instructions::movep(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // BTST (dynamic): 0000 xxx1 00xx xxxx
            if (opcode & 0xF1C0) == 0x0100 {
                return Instructions::btst(&mut self.registers, &self.memory, opcode, pc);
            }
            // BCHG (dynamic): 0000 xxx1 01xx xxxx
            if (opcode & 0xF1C0) == 0x0140 {
                return Instructions::bchg(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // BCLR (dynamic): 0000 xxx1 10xx xxxx
            if (opcode & 0xF1C0) == 0x0180 {
                return Instructions::bclr(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // BSET (dynamic): 0000 xxx1 11xx xxxx
            if (opcode & 0xF1C0) == 0x01C0 {
                return Instructions::bset(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // BTST (static): 0000 1000 00xx xxxx
            if (opcode & 0xFFC0) == 0x0800 {
                return Instructions::btst(&mut self.registers, &self.memory, opcode, pc);
            }
            // BCHG (static): 0000 1000 01xx xxxx
            if (opcode & 0xFFC0) == 0x0840 {
                return Instructions::bchg(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // BCLR (static): 0000 1000 10xx xxxx
            if (opcode & 0xFFC0) == 0x0880 {
                return Instructions::bclr(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // BSET (static): 0000 1000 11xx xxxx
            if (opcode & 0xFFC0) == 0x08C0 {
                return Instructions::bset(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // SUBI: 0000 0100 ssxx xxxx
            if (opcode & 0xFF00) == 0x0400 {
                return Instructions::subi(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // ADDI: 0000 0110 ssxx xxxx
            if (opcode & 0xFF00) == 0x0600 {
                return Instructions::addi(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // CMPI: 0000 1100 ssxx xxxx
            if (opcode & 0xFF00) == 0x0C00 {
                return Instructions::cmpi(&mut self.registers, &self.memory, opcode, pc);
            }

            // If we're still in family 0 and didn't match anything, it's illegal
            return Instructions::illegal(&mut self.registers, &self.memory, opcode, pc);
        }

        // ==================== OPCODE FAMILY 4: Miscellaneous ====================
        if top_nibble == 0x4 {
            // NOP: 0100 1110 0111 0001
            if opcode == 0x4E71 {
                return Instructions::nop(&mut self.registers, &self.memory, opcode, pc);
            }
            // RTS: 0100 1110 0111 0101
            if opcode == 0x4E75 {
                return Instructions::rts(&mut self.registers, &self.memory, opcode, pc);
            }
            // RTR: 0100 1110 0111 0111
            if opcode == 0x4E77 {
                return Instructions::rtr(&mut self.registers, &self.memory, opcode, pc);
            }
            // RTE: 0100 1110 0111 0011
            if opcode == 0x4E73 {
                return Instructions::rte(&mut self.registers, &self.memory, opcode, pc);
            }
            // TRAPV: 0100 1110 0111 0110
            if opcode == 0x4E76 {
                return Instructions::trapv(&self.registers, &self.memory, opcode, pc);
            }
            // RESET: 0100 1110 0111 0000
            if opcode == 0x4E70 {
                return Instructions::reset(&self.registers, &self.memory, opcode, pc);
            }
            // STOP: 0100 1110 0111 0010
            if opcode == 0x4E72 {
                return Instructions::stop(&mut self.registers, &self.memory, opcode, pc);
            }

            // TRAP: 0100 1110 0100 xxxx
            if (opcode & 0xFFF0) == 0x4E40 {
                return Instructions::trap(&self.registers, &self.memory, opcode, pc);
            }

            // LINK: 0100 1110 0101 0xxx
            if (opcode & 0xFFF8) == 0x4E50 {
                return Instructions::link(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // UNLK: 0100 1110 0101 1xxx
            if (opcode & 0xFFF8) == 0x4E58 {
                return Instructions::unlk(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // SWAP: 0100 1000 0100 0xxx
            if (opcode & 0xFFF8) == 0x4840 {
                return Instructions::swap(&mut self.registers, &self.memory, opcode, pc);
            }

            // PEA: 0100 1000 01xx xxxx
            if (opcode & 0xFFC0) == 0x4840 {
                return Instructions::pea(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // EXT: 0100 1000 1s00 0rrr (s=0 for word, s=1 for long, rrr=register)
            // EXT.W: 0x4880-0x4887, EXT.L: 0x48C0-0x48C7
            if (opcode & 0xFFF8) == 0x4880 || (opcode & 0xFFF8) == 0x48C0 {
                return Instructions::ext(&mut self.registers, &self.memory, opcode, pc);
            }

            // MOVEM: 0100 1d00 1sxx xxxx
            // Registers to memory (d=0): 0100 1000 1xxx xxxx = 0x4880-0x48FF
            // Memory to registers (d=1): 0100 1100 1xxx xxxx = 0x4C80-0x4CFF
            // EXT overlaps at 0x4880-0x4887 and 0x48C0-0x48C7, so check EXT first (above)
            if (opcode & 0xFB80) == 0x4880 {
                return Instructions::movem(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // LEA: 0100 xxx1 11xx xxxx
            if (opcode & 0xF1C0) == 0x41C0 {
                return Instructions::lea(&mut self.registers, &self.memory, opcode, pc);
            }

            // CHK: 0100 xxx1 10xx xxxx
            if (opcode & 0xF1C0) == 0x4180 {
                return Instructions::chk(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // TAS: 0100 1010 11xx xxxx
            if (opcode & 0xFFC0) == 0x4AC0 {
                return Instructions::tas(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // TST: 0100 1010 ssxx xxxx
            if (opcode & 0xFF00) == 0x4A00 {
                return Instructions::tst(&mut self.registers, &self.memory, opcode, pc);
            }

            // NBCD: 0100 1000 00xx xxxx
            if (opcode & 0xFFC0) == 0x4800 {
                return Instructions::nbcd(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // SWAP: 0100 1000 0100 0xxx
            if (opcode & 0xFFF8) == 0x4840 {
                return Instructions::swap(&mut self.registers, &self.memory, opcode, pc);
            }

            // MOVE from SR: 0100 0000 11xx xxxx (must check before NEGX)
            if (opcode & 0xFFC0) == 0x40C0 {
                return Instructions::move_from_sr(
                    &mut self.registers,
                    &mut self.memory,
                    opcode,
                    pc,
                );
            }
            // MOVE to CCR: 0100 0100 11xx xxxx (must check before NEG)
            if (opcode & 0xFFC0) == 0x44C0 {
                return Instructions::move_to_ccr(&mut self.registers, &self.memory, opcode, pc);
            }
            // MOVE to SR: 0100 0110 11xx xxxx (must check before NOT)
            if (opcode & 0xFFC0) == 0x46C0 {
                return Instructions::move_to_sr(&mut self.registers, &self.memory, opcode, pc);
            }
            // MOVE USP: 0100 0110 0110 xxxx (must check before NOT)
            if (opcode & 0xFFF0) == 0x4E60 {
                return Instructions::move_usp(&mut self.registers, &self.memory, opcode, pc);
            }

            // NEG: 0100 0100 ssxx xxxx (where ss != 11)
            if (opcode & 0xFF00) == 0x4400 {
                return Instructions::neg(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // NEGX: 0100 0000 ssxx xxxx (where ss != 11)
            if (opcode & 0xFF00) == 0x4000 {
                return Instructions::negx(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // CLR: 0100 0010 ssxx xxxx
            if (opcode & 0xFF00) == 0x4200 {
                return Instructions::clr(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // NOT: 0100 0110 ssxx xxxx (where ss != 11)
            if (opcode & 0xFF00) == 0x4600 {
                return Instructions::not(&mut self.registers, &mut self.memory, opcode, pc);
            }

            // JMP: 0100 1110 11xx xxxx
            if (opcode & 0xFFC0) == 0x4EC0 {
                return Instructions::jmp(&mut self.registers, &self.memory, opcode, pc);
            }
            // JSR: 0100 1110 10xx xxxx
            if (opcode & 0xFFC0) == 0x4E80 {
                return Instructions::jsr(&mut self.registers, &mut self.memory, opcode, pc);
            }
        }

        // ==================== OPCODE FAMILY 5: ADDQ/SUBQ/Scc/DBcc ====================
        if top_nibble == 0x5 {
            // DBcc: 0101 cccc 1100 1xxx
            if (opcode & 0xF0F8) == 0x50C8 {
                return Instructions::dbcc(&mut self.registers, &self.memory, opcode, pc);
            }
            // Scc: 0101 cccc 11xx xxxx (not DBcc pattern)
            if (opcode & 0xF0C0) == 0x50C0 {
                return Instructions::scc(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // ADDQ: 0101 xxx0 ssxx xxxx
            if (opcode & 0xF100) == 0x5000 {
                return Instructions::addq(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // SUBQ: 0101 xxx1 ssxx xxxx
            if (opcode & 0xF100) == 0x5100 {
                return Instructions::subq(&mut self.registers, &mut self.memory, opcode, pc);
            }
        }

        // ==================== OPCODE FAMILY 6: Bcc/BSR/BRA ====================
        if top_nibble == 0x6 {
            // Check for BSR (bits 11-8 = 0001)
            if (opcode & 0x0F00) == 0x0100 {
                return Instructions::bsr(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // Check for BRA (bits 11-8 = 0000)
            if (opcode & 0x0F00) == 0x0000 {
                return Instructions::bra(&mut self.registers, &self.memory, opcode, pc);
            }
            // Other conditional branches (Bcc)
            return Instructions::bcc(&mut self.registers, &self.memory, opcode, pc);
        }

        // ==================== OPCODE FAMILY 7: MOVEQ ====================
        if top_nibble == 0x7 && (opcode & 0x0100) == 0 {
            return Instructions::moveq(&mut self.registers, &self.memory, opcode, pc);
        }

        // ==================== OPCODE FAMILY 8: OR/DIV/SBCD ====================
        if top_nibble == 0x8 {
            // SBCD: 1000 xxx1 0000 0xxx (Dn to Dn)
            //       1000 xxx1 0000 1xxx (-(An) to -(An))
            // Mask 0xF1F8 isolates bits 15-12, 8, 6-3 to distinguish from OR
            if (opcode & 0xF1F8) == 0x8100 || (opcode & 0xF1F8) == 0x8108 {
                return Instructions::sbcd(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // DIVU/DIVS: 1000 xxx0 11xx xxxx (DIVU) or 1000 xxx1 11xx xxxx (DIVS)
            if (opcode & 0xF0C0) == 0x80C0 {
                return Instructions::div(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // OR
            return Instructions::or(&mut self.registers, &mut self.memory, opcode, pc);
        }

        // ==================== OPCODE FAMILY 9: SUB/SUBX ====================
        if top_nibble == 0x9 {
            // SUBX: 1001 xxx1 ss00 0xxx (Dx to Dx)
            //       1001 xxx1 ss00 1xxx (-(Ax) to -(Ax))
            // Note: bits 5-4 must be 00 for SUBX, distinguishing it from SUB
            if (opcode & 0xF130) == 0x9100 {
                return Instructions::subx(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // SUBA: 1001 xxx0 11xx xxxx (word) or 1001 xxx1 11xx xxxx (long)
            if (opcode & 0xF1C0) == 0x91C0 || (opcode & 0xF1C0) == 0x90C0 {
                return Instructions::suba(&mut self.registers, &self.memory, opcode, pc);
            }
            // Regular SUB
            return Instructions::sub(&mut self.registers, &mut self.memory, opcode, pc);
        }

        // ==================== OPCODE FAMILY B: CMP/EOR ====================
        if top_nibble == 0xB {
            // CMPA: 1011 xxx0 11xx xxxx (word) or 1011 xxx1 11xx xxxx (long)
            // Must check CMPA before CMPM because CMPM pattern could match CMPA
            if (opcode & 0xF0C0) == 0xB0C0 {
                return Instructions::cmpa(&mut self.registers, &self.memory, opcode, pc);
            }
            // CMPM: 1011 xxx1 ss00 1xxx (where ss != 11, which is CMPA)
            if (opcode & 0xF138) == 0xB108 {
                return Instructions::cmpm(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // EOR: 1011 xxx1 ssxx xxxx (but not CMPM or CMPA)
            if (opcode & 0xF100) == 0xB100 {
                return Instructions::eor(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // CMP: 1011 xxx0 ssxx xxxx
            return Instructions::cmp(&mut self.registers, &self.memory, opcode, pc);
        }

        // ==================== OPCODE FAMILY C: AND/MUL/ABCD/EXG ====================
        if top_nibble == 0xC {
            // ABCD: 1100 xxx1 0000 0xxx (Dn to Dn)
            //       1100 xxx1 0000 1xxx (-(An) to -(An))
            // Mask 0xF1F8 isolates bits 15-12, 8, 6-3 to distinguish from EXG
            if (opcode & 0xF1F8) == 0xC100 || (opcode & 0xF1F8) == 0xC108 {
                return Instructions::abcd(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // EXG: 1100 xxx1 0100 0xxx (data registers)
            //      1100 xxx1 0100 1xxx (address registers)
            //      1100 xxx1 1000 1xxx (data and address)
            if (opcode & 0xF130) == 0xC100
                && ((opcode & 0x00C8) == 0x0040
                    || (opcode & 0x00C8) == 0x0048
                    || (opcode & 0x00C8) == 0x0088)
            {
                return Instructions::exg(&mut self.registers, &self.memory, opcode, pc);
            }
            // MULU/MULS: 1100 xxx0 11xx xxxx (MULU) or 1100 xxx1 11xx xxxx (MULS)
            if (opcode & 0xF0C0) == 0xC0C0 {
                return Instructions::mul(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // AND
            return Instructions::and(&mut self.registers, &mut self.memory, opcode, pc);
        }

        // ==================== OPCODE FAMILY D: ADD/ADDX ====================
        if top_nibble == 0xD {
            // ADDX: 1101 xxx1 ss00 0xxx (Dx to Dx)
            //       1101 xxx1 ss00 1xxx (-(Ax) to -(Ax))
            if (opcode & 0xF130) == 0xD100 {
                return Instructions::addx(&mut self.registers, &mut self.memory, opcode, pc);
            }
            // ADDA: 1101 xxx0 11xx xxxx (word) or 1101 xxx1 11xx xxxx (long)
            if (opcode & 0xF1C0) == 0xD0C0 || (opcode & 0xF1C0) == 0xD1C0 {
                return Instructions::adda(&mut self.registers, &self.memory, opcode, pc);
            }
            // Regular ADD
            return Instructions::add(&mut self.registers, &mut self.memory, opcode, pc);
        }

        // ==================== OPCODE FAMILY E: Shift/Rotate ====================
        if top_nibble == 0xE {
            // Memory shifts/rotates: 1110 xxx0 11xx xxxx or 1110 xxx1 11xx xxxx
            if (opcode & 0xF8C0) == 0xE0C0 {
                // Dispatch based on bits 10-9 (shift type)
                let shift_type = (opcode >> 9) & 0x03;
                let direction = (opcode >> 8) & 0x01;
                match (shift_type, direction) {
                    (0, 0) => {
                        return Instructions::asx(&mut self.registers, &mut self.memory, opcode, pc)
                    } // ASR
                    (0, 1) => {
                        return Instructions::asx(&mut self.registers, &mut self.memory, opcode, pc)
                    } // ASL
                    (1, 0) => {
                        return Instructions::lsr(&mut self.registers, &mut self.memory, opcode, pc)
                    } // LSR
                    (1, 1) => {
                        return Instructions::lsl(&mut self.registers, &mut self.memory, opcode, pc)
                    } // LSL
                    (2, 0) => {
                        return Instructions::roxr(
                            &mut self.registers,
                            &mut self.memory,
                            opcode,
                            pc,
                        )
                    } // ROXR
                    (2, 1) => {
                        return Instructions::roxl(
                            &mut self.registers,
                            &mut self.memory,
                            opcode,
                            pc,
                        )
                    } // ROXL
                    (3, 0) => {
                        return Instructions::ror(&mut self.registers, &mut self.memory, opcode, pc)
                    } // ROR
                    (3, 1) => {
                        return Instructions::rol(&mut self.registers, &mut self.memory, opcode, pc)
                    } // ROL
                    _ => unreachable!(),
                }
            }
            // Register shifts/rotates: 1110 xxxd ss0i irrr
            // where d=direction, ss=size, i=count mode, rrr=register
            let direction = (opcode >> 8) & 0x01;
            let shift_type = (opcode >> 3) & 0x03;
            match (shift_type, direction) {
                (0, _) => {
                    return Instructions::asx(&mut self.registers, &mut self.memory, opcode, pc)
                } // AS
                (1, 0) => {
                    return Instructions::lsr(&mut self.registers, &mut self.memory, opcode, pc)
                } // LSR
                (1, 1) => {
                    return Instructions::lsl(&mut self.registers, &mut self.memory, opcode, pc)
                } // LSL
                (2, 0) => {
                    return Instructions::roxr(&mut self.registers, &mut self.memory, opcode, pc)
                } // ROXR
                (2, 1) => {
                    return Instructions::roxl(&mut self.registers, &mut self.memory, opcode, pc)
                } // ROXL
                (3, 0) => {
                    return Instructions::ror(&mut self.registers, &mut self.memory, opcode, pc)
                } // ROR
                (3, 1) => {
                    return Instructions::rol(&mut self.registers, &mut self.memory, opcode, pc)
                } // ROL
                _ => unreachable!(),
            }
        }

        // ==================== MOVE and MOVEA ====================
        // MOVE opcodes: 00ss xxxx xxxx xxxx where ss encodes size:
        //   01 = MOVE.B (Line-1: 0x1xxx)
        //   11 = MOVE.W (Line-3: 0x3xxx)
        //   10 = MOVE.L (Line-2: 0x2xxx)
        // MOVEA: destination mode = 001 (address register direct)
        //
        // Important: Only Line-1, Line-2, Line-3 are MOVE instructions.
        // Line-A (0xAxxx) and Line-F (0xFxxx) are reserved/illegal on 68000.
        if (0x1..=0x3).contains(&top_nibble) {
            let dst_mode_raw = (opcode >> 6) & 0x07;
            let dst_mode = dst_mode_raw as u8;

            // MOVEA: destination mode = 001 (address register direct)
            if dst_mode == 0b001 {
                return Instructions::movea(&mut self.registers, &self.memory, opcode, pc);
            }

            // Regular MOVE
            let src_mode = (opcode & 0x07) as u8;
            if src_mode <= 0b111 && dst_mode <= 0b111 {
                return Instructions::move_(&mut self.registers, &mut self.memory, opcode, pc);
            }
        }

        // ==================== Line-A and Line-F Exceptions ====================
        // Line-A (0xAxxx): Unimplemented instruction, triggers vector 10
        // Line-F (0xFxxx): Unimplemented instruction, triggers vector 11
        // These are used for emulators and coprocessors on later 68K models.
        if top_nibble == 0xA {
            return InstructionResult::with_exception(pc, 34, 10); // Line-A exception, vector 10
        }
        if top_nibble == 0xF {
            return InstructionResult::with_exception(pc, 34, 11); // Line-F exception, vector 11
        }

        // If we haven't matched any instruction, it's illegal (vector 4)
        InstructionResult::with_exception(pc, 34, 4) // Illegal instruction exception
    }

    /// Prints the current CPU state for debugging.
    // Allow dead code: kept for tests, completeness, or CLI-only usage.
    #[allow(dead_code)]
    pub fn dump_state(&self) -> String {
        format!(
            "PC={:08X} SR={:04X} [{}{}{}{}{}] Cycles={}\n{}",
            self.registers.pc,
            self.registers.sr,
            if self.registers.get_n() { "N" } else { "-" },
            if self.registers.get_z() { "Z" } else { "-" },
            if self.registers.get_c() { "C" } else { "-" },
            if self.registers.get_v() { "V" } else { "-" },
            if self.registers.get_x() { "X" } else { "-" },
            self.cycles,
            self.registers
        )
    }
}

impl fmt::Debug for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cpu")
            .field("registers", &self.registers)
            .field("memory", &self.memory)
            .field("halted", &self.halted)
            .field("cycles", &self.cycles)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_new() {
        let cpu = Cpu::new();
        assert_eq!(cpu.pc(), 0);
        assert_eq!(cpu.sr(), 0);
        assert!(!cpu.is_halted());
        assert_eq!(cpu.total_cycles(), 0);
    }

    #[test]
    fn test_cpu_reset() {
        let mut cpu = Cpu::new();
        cpu.registers.set_d(0, 0x12345678);
        cpu.set_pc(0x1000);
        cpu.cycles = 1000;

        cpu.reset();

        assert_eq!(cpu.pc(), 0);
        assert_eq!(cpu.registers.d(0), 0);
        assert!(!cpu.is_halted());
        assert_eq!(cpu.total_cycles(), 0);
    }

    #[test]
    fn test_halt_resume() {
        let mut cpu = Cpu::new();

        assert!(!cpu.is_halted());
        cpu.halt();
        assert!(cpu.is_halted());
        cpu.resume();
        assert!(!cpu.is_halted());
    }

    #[test]
    fn test_step_moveq() {
        let mut cpu = Cpu::new();
        // Write MOVEQ #0, D0 to memory at PC=0
        cpu.memory.write_word(0, 0x7000).unwrap();

        let executed = cpu.step();
        assert!(executed);
        assert_eq!(cpu.registers.d(0), 0);
        assert_eq!(cpu.pc(), 2);
        assert_eq!(cpu.total_cycles(), 4);
    }

    #[test]
    fn test_step_bra() {
        let mut cpu = Cpu::new();
        // Write BRA #10 to memory at PC=0
        // displacement = +10, target = PC + 2 + displacement = 0 + 2 + 10 = 12
        cpu.memory.write_word(0, 0x600A).unwrap();

        let executed = cpu.step();
        assert!(executed);
        assert_eq!(cpu.pc(), 12);
    }

    #[test]
    fn test_multiple_steps() {
        let mut cpu = Cpu::new();
        // Write a sequence: MOVEQ #1, D0; MOVEQ #2, D1; BRA #0
        cpu.memory.write_word(0, 0x7001).unwrap();
        cpu.memory.write_word(2, 0x7202).unwrap();
        cpu.memory.write_word(4, 0x6000).unwrap(); // BRA with displacement 0

        // Execute 3 instructions
        // 1. MOVEQ at PC=0 -> PC=2
        // 2. MOVEQ at PC=2 -> PC=4
        // 3. BRA at PC=4 -> PC=6 (target = 4+2+0 = 6)
        let count = cpu.run(3);
        assert_eq!(count, 3);
        assert_eq!(cpu.registers.d(0), 1);
        assert_eq!(cpu.registers.d(1), 2);
        assert_eq!(cpu.pc(), 6);
    }

    #[test]
    fn test_dump_state() {
        let cpu = Cpu::new();
        let dump = cpu.dump_state();
        assert!(dump.contains("PC=00000000"));
        assert!(dump.contains("SR=0000"));
        assert!(dump.contains("Cycles=0"));
    }
}
