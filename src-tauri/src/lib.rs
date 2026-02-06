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
mod uart;
mod test_runner;

use sbc::Sbc;
use std::sync::{Arc, Mutex};

/// The Flux32 emulator state - wrapped in Arc<Mutex<>> for thread safety
pub struct Flux32Emulator {
    sbc: Arc<Mutex<Sbc>>,
}

impl Flux32Emulator {
    fn new() -> Self {
        Self { sbc: Arc::new(Mutex::new(Sbc::new())) }
    }

    /// Execute a single instruction step
    fn step(&mut self) -> Result<(), String> {
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

// SAFETY: We assert that Flux32Emulator is Send + Sync since it's Arc<Mutex<Sbc>>
// The SBC itself uses Rc<RefCell<>> internally but is only accessed on the main thread
unsafe impl Send for Flux32Emulator {}
unsafe impl Sync for Flux32Emulator {}

/// Global emulator state - use once_cell sync or a simpler approach
static mut EMULATOR: Option<Flux32Emulator> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// Initialize a new emulator instance
#[tauri::command]
fn emulator_init() -> Result<String, String> {
    INIT.call_once(|| {
        unsafe {
            EMULATOR = Some(Flux32Emulator::new());
        }
    });
    Ok("Emulator initialized".to_string())
}

/// Execute a single instruction step
#[tauri::command]
fn emulator_step() -> Result<String, String> {
    unsafe {
        if let Some(emulator) = EMULATOR.as_mut() {
            emulator.step()?;
            Ok("Step executed".to_string())
        } else {
            Err("Emulator not initialized".to_string())
        }
    }
}

/// Get the current CPU register state
#[tauri::command]
fn emulator_get_registers() -> Result<CpuState, String> {
    unsafe {
        if let Some(emulator) = EMULATOR.as_ref() {
            Ok(emulator.get_cpu_state())
        } else {
            Err("Emulator not initialized".to_string())
        }
    }
}

/// Read a byte from memory at the given address
#[tauri::command]
fn emulator_read_byte(address: u32) -> Result<u8, String> {
    unsafe {
        if let Some(emulator) = EMULATOR.as_ref() {
            let sbc = emulator.sbc.lock().unwrap();
            sbc.cpu().memory.read_byte(address).map_err(|e| e.to_string())
        } else {
            Err("Emulator not initialized".to_string())
        }
    }
}

/// Read a block of bytes from memory
#[tauri::command]
fn emulator_read_memory(address: u32, length: usize) -> Result<Vec<u8>, String> {
    unsafe {
        if let Some(emulator) = EMULATOR.as_ref() {
            let sbc = emulator.sbc.lock().unwrap();
            let mut data = vec![0u8; length];
            for i in 0..length {
                data[i] = sbc.cpu().memory.read_byte(address + i as u32).map_err(|e| e.to_string())?;
            }
            Ok(data)
        } else {
            Err("Emulator not initialized".to_string())
        }
    }
}

/// Write a byte to memory at the given address
#[tauri::command]
fn emulator_write_byte(address: u32, value: u8) -> Result<(), String> {
    unsafe {
        if let Some(emulator) = EMULATOR.as_mut() {
            let mut sbc = emulator.sbc.lock().unwrap();
            sbc.cpu_mut().memory.write_byte(address, value).map_err(|e| e.to_string())
        } else {
            Err("Emulator not initialized".to_string())
        }
    }
}

/// Assemble M68K assembly code
#[tauri::command]
fn emulator_assemble(code: String) -> Result<Vec<u8>, String> {
    // TODO: Implement assembler integration
    Ok(vec![])
}

/// Reset the emulator to initial state
#[tauri::command]
fn emulator_reset() -> Result<String, String> {
    unsafe {
        EMULATOR = Some(Flux32Emulator::new());
        Ok("Emulator reset".to_string())
    }
}

/// Run the emulator continuously
#[tauri::command]
fn emulator_run(max_cycles: Option<u64>) -> Result<EmulatorStatus, String> {
    unsafe {
        if let Some(emulator) = EMULATOR.as_mut() {
            let cycles = max_cycles.unwrap_or(100000);
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
}

/// Get emulator status (halted, cycles, etc.)
#[tauri::command]
fn emulator_get_status() -> Result<EmulatorStatus, String> {
    unsafe {
        if let Some(emulator) = EMULATOR.as_ref() {
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
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();
    builder = builder.plugin(tauri_plugin_opener::init());

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_mcp_bridge::init());
    }

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
