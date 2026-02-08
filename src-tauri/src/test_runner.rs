//! Test runner for Musashi M68K test suite integration.
//!
//! This module provides functionality to load and execute Musashi test binaries,
//! providing the expected memory-mapped I/O interface for test pass/fail reporting.
//!
//! ## Memory-Mapped I/O Interface
//!
//! Musashi tests communicate with the test harness via writes to specific memory
//! addresses. The test framework monitors these addresses via a write hook.
//!
//! Note: This module is primarily used for testing and is not used by the CLI runtime.

// Allow dead code: test runner is compiled into the CLI binary but only used in tests.
#![allow(dead_code)]

use crate::cpu::Cpu;
use crate::memory::WriteHookResult;
use std::fs;
use std::path::Path;

/// Memory-mapped I/O addresses used by Musashi tests.
///
/// These constants document the MMIO addresses expected by Musashi test binaries.
/// Tests write to these addresses to signal pass/fail status and request debug output.
const TEST_FAIL_REG: u32 = 0x0100_0000;
const TEST_PASS_REG: u32 = 0x0100_0004;
const PRINT_REG_REG: u32 = 0x0100_0008;
const INTERRUPT_REG: u32 = 0x0100_000C;
const STDOUT_REG: u32 = 0x0100_0014;
const PRINT_FP_REG: u32 = 0x0100_0020;

/// Initial stack pointer for test execution
const STACK_BASE: u32 = 0x3F0;

/// Entry point for test code
const TEST_ENTRY: u32 = 0x10000;

/// Test execution result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestResult {
    Pass,
    Fail,
    Timeout,
    Error(&'static str),
}

/// Test runner for Musashi test binaries
pub struct TestRunner {
    cpu: Cpu,
    max_cycles: u64,
}

// Global state for MMIO callbacks (since function pointers can't capture)
use std::sync::atomic::{AtomicBool, Ordering};
static TEST_PASSED_FLAG: AtomicBool = AtomicBool::new(false);
static TEST_FAILED_FLAG: AtomicBool = AtomicBool::new(false);

/// MMIO write hook for Musashi test registers
fn musashi_write_hook(
    address: u32,
    value: u32,
    _size: crate::memory::OperandSize,
) -> WriteHookResult {
    match address {
        TEST_PASS_REG => {
            TEST_PASSED_FLAG.store(true, Ordering::SeqCst);
            WriteHookResult::Handled
        }
        TEST_FAIL_REG => {
            TEST_FAILED_FLAG.store(true, Ordering::SeqCst);
            WriteHookResult::Handled
        }
        STDOUT_REG => {
            // Print character to stdout
            print!("{}", (value & 0xFF) as u8 as char);
            std::io::Write::flush(&mut std::io::stdout()).ok();
            WriteHookResult::Handled
        }
        PRINT_REG_REG => {
            eprintln!("PRINT_REG: CPU state requested (value=0x{value:08X})");
            WriteHookResult::Handled
        }
        INTERRUPT_REG => {
            eprintln!("INTERRUPT: Interrupt requested (value=0x{value:08X})");
            WriteHookResult::Handled
        }
        PRINT_FP_REG => {
            eprintln!("PRINT_FP: FP registers requested (value=0x{value:08X})");
            WriteHookResult::Handled
        }
        _ => WriteHookResult::Unhandled,
    }
}

impl TestRunner {
    /// Print register state for debugging test failures
    fn print_register_state(&self, prefix: &str) {
        eprintln!("{} at PC={:08X}", prefix, self.cpu.registers.pc());
        for i in 0..8 {
            eprintln!(
                "  D{}: {:08X}  A{}: {:08X}",
                i,
                self.cpu.registers.d(i),
                i,
                self.cpu.registers.a(i)
            );
        }
    }

    /// Create a new test runner
    pub fn new() -> Self {
        // Reset global flags
        TEST_PASSED_FLAG.store(false, Ordering::SeqCst);
        TEST_FAILED_FLAG.store(false, Ordering::SeqCst);

        // Musashi tests require a large memory space:
        // 0x0-0x10000: RAM (64KB)
        // 0x10000-0x50000: ROM (256KB for code)
        // 0x100000-0x110000: Test device registers (64KB)
        // 0x300000-0x310000: Extra RAM (64KB)
        // Total: ~3MB minimum, but we'll use 4MB for safety
        let mut cpu = Cpu::with_memory_size(4 * 1024 * 1024); // 4MB

        // Install MMIO write hook
        cpu.memory_mut().set_write_hook(musashi_write_hook);

        Self {
            cpu,
            max_cycles: 10_000_000, // Default: 10M cycles should be enough
        }
    }

    /// Load a Musashi test binary from file
    pub fn load_test(&mut self, path: &Path) -> Result<(), String> {
        // Read the test binary
        let rom_data = fs::read(path).map_err(|e| format!("Failed to read test file: {e}"))?;

        // Reset CPU first to clear any previous state
        // This also clears memory, so we load ROM after this
        self.cpu.reset();

        // Setup memory layout according to Musashi test expectations:
        // 0x0-0x10000: RAM (stack and vectors)
        // 0x10000-...: ROM (test code)
        // 0x100000-0x110000: Test device registers
        // 0x300000-0x310000: Extra RAM

        // Write test ROM starting at 0x10000
        for (i, &byte) in rom_data.iter().enumerate() {
            let addr = TEST_ENTRY + i as u32;
            self.cpu
                .memory_mut()
                .write_byte(addr, byte)
                .map_err(|e| format!("Failed to write ROM at {addr:08X}: {e}"))?;
        }

        // Setup boot vectors
        self.setup_vectors()?;

        // Load initial PC and SP from reset vectors (like real M68K hardware)
        let initial_sp = self
            .cpu
            .memory_mut()
            .read_long(0)
            .map_err(|e| format!("Failed to read initial SP: {e}"))?;
        let initial_pc = self
            .cpu
            .memory_mut()
            .read_long(4)
            .map_err(|e| format!("Failed to read initial PC: {e}"))?;

        self.cpu.registers.set_sp(initial_sp);
        self.cpu.set_pc(initial_pc);

        Ok(())
    }

    /// Setup the M68K vector table for test execution
    fn setup_vectors(&mut self) -> Result<(), String> {
        // Only set the reset vectors (0 and 1).
        // The test binary itself sets up any exception vectors it needs.
        // Don't overwrite other vectors with garbage values.

        // Vector 0: Initial SSP (Supervisor Stack Pointer)
        self.cpu
            .memory_mut()
            .write_long(0, STACK_BASE)
            .map_err(|e| format!("Failed to write SSP: {e}"))?;

        // Vector 1: Initial PC (Program Counter - entry point)
        self.cpu
            .memory_mut()
            .write_long(4, TEST_ENTRY)
            .map_err(|e| format!("Failed to write PC: {e}"))?;

        Ok(())
    }

    /// Execute the loaded test
    pub fn run_test(&mut self) -> TestResult {
        // Reset global flags
        TEST_PASSED_FLAG.store(false, Ordering::SeqCst);
        TEST_FAILED_FLAG.store(false, Ordering::SeqCst);

        let mut cycle_count = 0u64;

        while cycle_count < self.max_cycles {
            // Check if test has passed or failed via MMIO
            if TEST_PASSED_FLAG.load(Ordering::SeqCst) {
                return TestResult::Pass;
            }
            if TEST_FAILED_FLAG.load(Ordering::SeqCst) {
                self.print_register_state("FAIL");
                return TestResult::Fail;
            }

            // Execute one instruction
            if !self.cpu.step() {
                // CPU halted

                // Check flags one more time after halt
                if TEST_PASSED_FLAG.load(Ordering::SeqCst) {
                    return TestResult::Pass;
                }
                if TEST_FAILED_FLAG.load(Ordering::SeqCst) {
                    self.print_register_state("FAIL");
                    return TestResult::Fail;
                }

                // Halted without pass/fail
                return TestResult::Error("CPU halted without test result");
            }

            cycle_count += 1;
        }

        eprintln!("Test TIMEOUT after {cycle_count} cycles");
        eprintln!("Last PC: 0x{:08X}", self.cpu.pc());
        for i in 0..8 {
            eprintln!(
                "  D{}: {:08X}  A{}: {:08X}",
                i,
                self.cpu.registers.d(i),
                i,
                self.cpu.registers.a(i)
            );
        }
        TestResult::Timeout
    }
}

/// Results from running a test suite
#[derive(Debug)]
pub struct TestSuiteResults {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub timeout: usize,
    pub error: usize,
}

impl TestSuiteResults {
    const fn new() -> Self {
        Self {
            total: 0,
            passed: 0,
            failed: 0,
            timeout: 0,
            error: 0,
        }
    }

    const fn record(&mut self, result: TestResult) {
        self.total += 1;
        match result {
            TestResult::Pass => self.passed += 1,
            TestResult::Fail => self.failed += 1,
            TestResult::Timeout => self.timeout += 1,
            TestResult::Error(_) => self.error += 1,
        }
    }
}

/// Run a suite of Musashi tests
pub fn run_test_suite(test_dir: &Path) -> TestSuiteResults {
    let mut results = TestSuiteResults::new();

    // Find all .bin files in the directory
    let paths = match fs::read_dir(test_dir) {
        Ok(entries) => entries
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("bin"))
            .collect::<Vec<_>>(),
        Err(e) => {
            eprintln!("Failed to read test directory: {e}");
            return results;
        }
    };

    if paths.is_empty() {
        eprintln!("No test binaries found in {}", test_dir.display());
        return results;
    }

    eprintln!(
        "Running {} tests from {}...",
        paths.len(),
        test_dir.display()
    );
    eprintln!();

    for path in paths {
        let test_name = path.file_name().unwrap().to_string_lossy();
        print!("  {test_name}: ");
        std::io::Write::flush(&mut std::io::stdout()).ok();

        let mut runner = TestRunner::new();
        match runner.load_test(&path) {
            Ok(()) => {
                let result = runner.run_test();
                match result {
                    TestResult::Pass => println!("PASS"),
                    TestResult::Fail => println!("FAIL"),
                    TestResult::Timeout => println!("TIMEOUT"),
                    TestResult::Error(msg) => println!("ERROR: {msg}"),
                }
                results.record(result);
            }
            Err(e) => {
                println!("ERROR loading: {e}");
                results.record(TestResult::Error("Load failure"));
            }
        }
    }

    eprintln!();
    eprintln!("Test Results:");
    eprintln!("  Total:   {}", results.total);
    eprintln!("  Passed:  {}", results.passed);
    eprintln!("  Failed:  {}", results.failed);
    eprintln!("  Timeout: {}", results.timeout);
    eprintln!("  Error:   {}", results.error);

    results
}
