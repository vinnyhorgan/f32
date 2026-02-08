#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
// Common patterns that make code more readable
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
// Legacy Codebase Exemptions (Strict Mode)
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::similar_names)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::significant_drop_in_scrutinee)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::fn_params_excessive_bools)]
#![allow(clippy::needless_pass_by_ref_mut)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::non_send_fields_in_send_ty)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::match_bool)]
// Style allowances - keep code readable
#![allow(clippy::too_many_lines)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::let_underscore_untyped)]
#![allow(clippy::unnecessary_literal_unwrap)]
#![allow(clippy::ref_patterns)]
#![allow(clippy::inconsistent_struct_constructor)]
#![allow(clippy::bool_to_int_with_if)]
#![allow(clippy::useless_let_if_seq)]
#![allow(clippy::if_not_else)]
#![allow(clippy::single_match)]
#![allow(clippy::single_match_else)]
#![allow(clippy::ignored_unit_patterns)]
#![allow(clippy::trait_duplication_in_bounds)]
#![allow(clippy::type_repetition_in_bounds)]
// Performance style - prefer explicitness over micro-optimizations
#![allow(clippy::ptr_arg)]
#![allow(clippy::used_underscore_binding)]
#![allow(clippy::assigning_clones)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::inline_always)]
#![allow(clippy::default_numeric_fallback)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::let_and_return)]
#![allow(clippy::map_flatten)]
#![allow(clippy::map_identity)]
#![allow(clippy::needless_late_init)]
#![allow(clippy::redundant_else)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::wildcard_in_or_patterns)]
// Additional style allowances
#![allow(clippy::items_after_statements)]
#![allow(clippy::no_effect_underscore_binding)]
#![allow(clippy::branches_sharing_code)]
#![allow(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::unused_self)]
// Nursery Exemptions
#![allow(clippy::redundant_pub_crate)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::use_self)]
#![allow(clippy::cognitive_complexity)]
// Allow for error handling in emulator code
#![allow(clippy::verbose_bit_mask)]
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Core emulator modules
mod addressing;
mod assembler;
mod bus;
mod cfcard;
mod cpu;
mod instructions;
mod memory;
mod registers;
mod sbc;
mod test_runner;
mod uart;

use sbc::Sbc;
use std::sync::{Arc, Mutex};

/// The Flux32 emulator state - wrapped in `Arc<Mutex<>>` for thread safety
///
/// The SBC now uses `Arc<Mutex<>>` internally, making it `Send + Sync` automatically.
/// `Flux32Emulator` wraps this in another `Arc<Mutex<>>` for Tauri IPC access.
pub struct Flux32Emulator {
    sbc: Arc<Mutex<Sbc>>,
}

impl Flux32Emulator {
    fn new() -> Self {
        Self {
            sbc: Arc::new(Mutex::new(Sbc::new())),
        }
    }

    /// Execute a single instruction step
    fn step(&self) -> Result<(), String> {
        self.sbc.lock().unwrap().step();
        Ok(())
    }

    /// Get the CPU state as a JSON-serializable structure
    fn get_cpu_state(&self) -> CpuState {
        let sbc = self.sbc.lock().unwrap();
        let regs = sbc.registers();
        CpuState {
            d: regs.d.to_vec(),
            a: regs.a[0..7].to_vec(), // A0-A6 (A7 is SP)
            pc: regs.pc,
            sr: regs.sr,
            usp: regs.usp(),
            ssp: regs.get_ssp(),
        }
    }
}

impl Default for Flux32Emulator {
    fn default() -> Self {
        Self::new()
    }
}

/// CPU register state for serialization
#[derive(serde::Serialize)]
pub struct CpuState {
    d: Vec<u32>,
    a: Vec<u32>,
    pc: u32,
    sr: u16,
    usp: u32,
    ssp: u32,
}

/// Emulator status information
#[derive(serde::Serialize)]
pub struct EmulatorStatus {
    halted: bool,
    cycles: u64,
    executed: u64,
}

/// Global emulator state using Mutex for thread-safe access.
///
/// The Mutex provides thread-safe access to the emulator instance. All access
/// is single-threaded through the main Tauri event loop, but Mutex ensures
/// safety if the pattern changes in the future.
static EMULATOR: std::sync::Mutex<Option<Flux32Emulator>> = std::sync::Mutex::new(None);

/// Initialize a new emulator instance
#[tauri::command]
fn emulator_init() -> Result<String, String> {
    let mut emulator = EMULATOR.lock().unwrap();
    if emulator.is_none() {
        *emulator = Some(Flux32Emulator::new());
    }
    Ok("Emulator initialized".to_string())
}

/// Execute a single instruction step
#[tauri::command]
fn emulator_step() -> Result<String, String> {
    let emulator = EMULATOR.lock().unwrap();
    if let Some(emulator) = emulator.as_ref() {
        emulator.step()?;
        Ok("Step executed".to_string())
    } else {
        Err("Emulator not initialized".to_string())
    }
}

/// Get the current CPU register state
#[tauri::command]
fn emulator_get_registers() -> Result<CpuState, String> {
    let emulator = EMULATOR.lock().unwrap();
    if let Some(emulator) = emulator.as_ref() {
        Ok(emulator.get_cpu_state())
    } else {
        Err("Emulator not initialized".to_string())
    }
}

/// Read a byte from memory at the given address
#[tauri::command]
fn emulator_read_byte(address: u32) -> Result<u8, String> {
    let emulator = EMULATOR.lock().unwrap();
    if let Some(emulator) = emulator.as_ref() {
        let sbc = emulator.sbc.lock().unwrap();
        sbc.cpu()
            .memory
            .read_byte(address)
            .map_err(|e| e.to_string())
    } else {
        Err("Emulator not initialized".to_string())
    }
}

/// Read a block of bytes from memory
#[tauri::command]
fn emulator_read_memory(address: u32, length: usize) -> Result<Vec<u8>, String> {
    let emulator = EMULATOR.lock().unwrap();
    if let Some(emulator) = emulator.as_ref() {
        let sbc = emulator.sbc.lock().unwrap();
        (0..length)
            .map(|i| {
                sbc.cpu()
                    .memory
                    .read_byte(address + i as u32)
                    .map_err(|e| e.to_string())
            })
            .collect()
    } else {
        Err("Emulator not initialized".to_string())
    }
}

/// Write a byte to memory at the given address
#[tauri::command]
fn emulator_write_byte(address: u32, value: u8) -> Result<(), String> {
    let emulator = EMULATOR.lock().unwrap();
    if let Some(emulator) = emulator.as_ref() {
        let mut sbc = emulator.sbc.lock().unwrap();
        sbc.cpu_mut()
            .memory
            .write_byte(address, value)
            .map_err(|e| e.to_string())
    } else {
        Err("Emulator not initialized".to_string())
    }
}

/// Assemble M68K assembly code and return the binary
#[tauri::command]
fn emulator_assemble(code: String) -> Result<Vec<u8>, String> {
    let mut asm = assembler::Assembler::new();
    // Add the rom directory as an include path so app.inc etc. can be found
    let rom_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rom");
    asm.include_paths.push(rom_dir);
    let path = std::path::Path::new("<editor>");
    asm.assemble_source(&code, path)
}

/// Assemble code, load it into RAM at `APP_START`, and start execution
#[tauri::command]
fn emulator_assemble_and_load(code: String) -> Result<String, String> {
    let binary = emulator_assemble(code)?;
    if binary.is_empty() {
        return Err("Assembly produced no output".to_string());
    }
    let emulator = EMULATOR.lock().unwrap();
    if let Some(emulator) = emulator.as_ref() {
        let mut sbc = emulator.sbc.lock().unwrap();
        sbc.load_app(&binary);
        sbc.run_app();
        Ok(format!("Loaded {} bytes at $E00100", binary.len()))
    } else {
        Err("Emulator not initialized".to_string())
    }
}

/// Read UART output (drain output buffer)
#[tauri::command]
fn emulator_read_uart() -> Result<Vec<u8>, String> {
    let emulator = EMULATOR.lock().unwrap();
    if let Some(emulator) = emulator.as_ref() {
        let mut sbc = emulator.sbc.lock().unwrap();
        Ok(sbc.drain_output())
    } else {
        Err("Emulator not initialized".to_string())
    }
}

/// Write a character to UART RX (simulate keyboard input)
#[tauri::command]
fn emulator_write_uart(byte: u8) -> Result<(), String> {
    let emulator = EMULATOR.lock().unwrap();
    if let Some(emulator) = emulator.as_ref() {
        let mut sbc = emulator.sbc.lock().unwrap();
        sbc.send_char(byte);
        Ok(())
    } else {
        Err("Emulator not initialized".to_string())
    }
}

/// Get the LED state
#[tauri::command]
fn emulator_get_led() -> Result<bool, String> {
    let emulator = EMULATOR.lock().unwrap();
    if let Some(emulator) = emulator.as_ref() {
        let sbc = emulator.sbc.lock().unwrap();
        Ok(sbc.led_state())
    } else {
        Err("Emulator not initialized".to_string())
    }
}

/// Reset the emulator to initial state
#[tauri::command]
fn emulator_reset() -> Result<String, String> {
    let mut emulator = EMULATOR.lock().unwrap();
    *emulator = Some(Flux32Emulator::new());
    Ok("Emulator reset".to_string())
}

/// Run the emulator continuously
#[tauri::command]
fn emulator_run(max_cycles: Option<u64>) -> Result<EmulatorStatus, String> {
    let emulator = EMULATOR.lock().unwrap();
    if let Some(emulator) = emulator.as_ref() {
        let cycles = max_cycles.unwrap_or(100_000);
        let mut sbc = emulator.sbc.lock().unwrap();
        let executed = sbc.run(cycles);
        Ok(EmulatorStatus {
            halted: sbc.is_halted(),
            cycles: sbc.cycles(),
            executed,
        })
    } else {
        Err("Emulator not initialized".to_string())
    }
}

/// Get emulator status (halted, cycles, etc.)
#[tauri::command]
fn emulator_get_status() -> Result<EmulatorStatus, String> {
    let emulator = EMULATOR.lock().unwrap();
    if let Some(emulator) = emulator.as_ref() {
        let sbc = emulator.sbc.lock().unwrap();
        Ok(EmulatorStatus {
            halted: sbc.is_halted(),
            cycles: sbc.cycles(),
            executed: 0,
        })
    } else {
        Err("Emulator not initialized".to_string())
    }
}

fn prevent_default() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri_plugin_prevent_default::Flags;

    let mut builder = tauri_plugin_prevent_default::Builder::new();

    #[cfg(debug_assertions)]
    {
        builder = builder.with_flags(Flags::all().difference(Flags::DEV_TOOLS | Flags::RELOAD));
    }

    #[cfg(not(debug_assertions))]
    {
        builder = builder.with_flags(Flags::all());
    }

    builder.build()
}

/// Main entry point for the Tauri application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();
    builder = builder.plugin(tauri_plugin_opener::init());

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_mcp_bridge::init());
    }

    builder = builder.plugin(prevent_default());

    builder
        .invoke_handler(tauri::generate_handler![
            emulator_init,
            emulator_step,
            emulator_reset,
            emulator_run,
            emulator_get_registers,
            emulator_get_status,
            emulator_read_byte,
            emulator_read_memory,
            emulator_write_byte,
            emulator_assemble,
            emulator_assemble_and_load,
            emulator_read_uart,
            emulator_write_uart,
            emulator_get_led,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
