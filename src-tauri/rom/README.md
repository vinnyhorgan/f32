# Flux32 ROM and Assembly Resources

This directory contains the Flux32 system ROM source code and assembly resources.

## Files

| File | Description |
|------|-------------|
| `rom.asm` | Complete system ROM source code |
| `rom.bin` | Compiled 64KB ROM binary (embedded in CLI) |
| `flux32.inc` | Master include file for assembly programs |
| `app.inc` | Application template (includes flux32.inc, sets org) |
| `memory.inc` | Memory map and system variable definitions |
| `uart.inc` | 16550 UART register definitions and macros |
| `cfcard.inc` | CompactFlash register and error definitions |
| `syscalls.inc` | System call numbers and format specifiers |
| `macros.inc` | Convenience macros (pushm/popm, bl/rl, etc.) |

## Memory Map

```
$000000-$0FFFFF  ROM (64KB repeated 16x)
$900000-$9FFFFF  CompactFlash (True IDE mode)
$A00000-$AFFFFF  UART 16550
$C00000-$CFFFFF  RAM (1MB)
$E00000-$EFFFFF  RAM mirror (apps load at $E00100)
```

## System Calls (TRAP #n)

| TRAP | Name | Description |
|------|------|-------------|
| 0 | Exit | Return to system |
| 1 | WaitBtn | Wait for button press/release |
| 2 | OutChar | Write single character (D0.B) |
| 3 | OutStr | Write null-terminated string (A0) |
| 4 | OutFmt | Formatted output (A0=format, stack=args) |
| 5 | InChar | Read single character -> D0.B |
| 6 | PromptStr | Prompt for string input |
| 7 | ReadSector | Read CF card sector |
| 8 | ListDirectory | Iterate directory entries |
| 9 | FindFile | Find named file |
| 10 | ReadFile | Read file into memory |
| 11 | GetDateTime | Read from RTC |
| 12 | SetDateTime | Set RTC time |
| 13 | GetSysInfo | Get system info pointer |
| 15 | Breakpoint | Enter debugger |

## Shell Commands

When booted without STARTUP.BIN, the ROM provides an interactive shell:

| Command | Description |
|---------|-------------|
| `?` | Print help |
| `<FILE>` | Run named file |
| `.L` | List files |
| `.I` | Print card info |
| `.P <FILE>` | Print file contents |
| `.H <FILE>` | Hexdump file contents |
| `.T` | Print date/time |
| `.T <DATE>` | Set date/time (YYYYMMDDWWhhmmss) |
| `.D` | Enter debugger |

## Writing Applications

Include `app.inc` at the start of your assembly file:

```asm
        include "app.inc"

start:  lea.l   message,a0
        sys     OutStr
        sys     Exit

message: dc.b   "Hello, World!\n",0
```

Assemble with vasm:
```
vasm -m68000 -Fbin -o program.bin program.asm
```

## Calling Convention

- `D0-D1` - Integer arguments and return values
- `A0-A1` - Pointer arguments and return values  
- `D2-D7/A2-A7/SR` - Preserved across calls
- Stack arguments are caller's responsibility to clean up

## Examples

See the `examples/` directory for sample programs:
- `hello.asm` - Basic UART output
- `fizzbuzz.asm` - Classic programming challenge
- `idle.asm` - LED fade animation using PWM
