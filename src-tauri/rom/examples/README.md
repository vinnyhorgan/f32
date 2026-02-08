# Flux32 Example Programs

This directory contains example assembly programs demonstrating various features of the Flux32 system.

## Quick Start

### Assemble and Run

```bash
# Using embedded toolchain
cd rom/examples
flux32 --vasm -m68000 -Fbin -o hello.bin hello.asm

# Run in terminal mode
flux32 --run ../rom.bin --app hello.bin

# Run in debugger
flux32 hello.bin
```

### From ROM Shell

If you have a CompactFlash image with these programs:

```bash
# Boot the system
flux32 --run ../rom.bin --app ../disk.img

# At the shell prompt:
> HELLO.BIN      # Run the program
```

## Example Programs

### hello.asm - Basic UART Output

**Difficulty**: ⭐ Beginner

A minimal "Hello World" program demonstrating:

- Including the system headers (`app.inc`)
- Using system calls (`sys` macro)
- Waiting for button input (`WaitBtn`)
- Toggling the LED (`led_tgl`)
- Printing strings (`OutStr`)
- Infinite loops

**Source**:

```asm
        include    "../app.inc"

start:  sys        WaitBtn                     ; Wait for button press
        led_tgl                                ; Toggle LED
        lea.l      str,a0                      ; Load string address
        sys        OutStr                      ; Print string
        bra        start                       ; Loop forever

str:    dc.b       "Hello from Flux32!\n",0
```

**What it does**:

1. Waits for you to press the virtual button (Ctrl+B in terminal mode)
2. Toggles the LED indicator
3. Prints "Hello from Flux32!" to the UART
4. Loops forever

**Learning Points**:

- System call invocation via TRAP
- LEA instruction for loading addresses
- Infinite loops with BRA
- Null-terminated strings

**Try It**:

```bash
flux32 --vasm -m68000 -Fbin -o hello.bin hello.asm
flux32 --run ../rom.bin --app hello.bin
# Press Ctrl+B to trigger the button
```

---

### fizzbuzz.asm - Control Flow and Formatting

**Difficulty**: ⭐⭐ Intermediate

The classic FizzBuzz programming challenge, demonstrating:

- Integer arithmetic (DIVU)
- Modulo operations (remainder via SWAP)
- Conditional logic (branches)
- Formatted output (`OutFmt` syscall)
- Inline string literals (`litstr` macro)
- Delay loops

**Source Walkthrough**:

```asm
start:  sys        WaitBtn
        moveq      #1,d3           ; Counter starting at 1

.loop:  moveq      #0,d2           ; Flag for "printed something"

        ; Check divisible by 3
        move.l     d3,d0
        divu.w     #3,d0           ; Divide by 3
        swap       d0              ; Remainder now in low word
        tst.w      d0
        bne        .1              ; Skip if not divisible
        litstr     "Fizz"
        sys        OutStr
        addq.w     #1,d2           ; Mark that we printed

.1:     ; Check divisible by 5
        move.l     d3,d0
        divu.w     #5,d0           ; Divide by 5
        swap       d0              ; Remainder now in low word
        tst.w      d0
        bne        .2              ; Skip if not divisible
        litstr     "Buzz"
        sys        OutStr
        addq       #1,d2           ; Mark that we printed

.2:     ; If flag not set, print the number
        tst.w      d2
        bne        .3
        move.w     d3,-(sp)        ; Push number
        litstr     FMT_U16,0       ; Push format string
        sys        OutFmt          ; Print number
        addq       #2,sp           ; Clean up stack

.3:     moveq      #$0a,d0         ; Newline
        sys        OutChar
        addq       #1,d3           ; Increment counter
        led_tgl                    ; Visual feedback
        move.l     #$40000,d0      ; Delay
        bsr        delay
        bra        .loop           ; Continue forever
```

**What it does**:

1. Counts from 1 to infinity
2. Prints "Fizz" for multiples of 3
3. Prints "Buzz" for multiples of 5
4. Prints "FizzBuzz" for multiples of 15
5. Prints the number otherwise
6. Toggles LED and delays between iterations

**Learning Points**:

- DIVU (unsigned division) instruction
- Using SWAP to access remainder from DIVU
- TST instruction for testing zero
- Conditional branches (BNE)
- Stack manipulation for syscall arguments
- Format specifiers (FMT_U16)
- BSR/RTS for subroutines

**Try It**:

```bash
flux32 --vasm -m68000 -Fbin -o fizzbuzz.bin fizzbuzz.asm
flux32 --run ../rom.bin --app fizzbuzz.bin
# Watch it count: 1, 2, Fizz, 4, Buzz, Fizz, 7, 8, Fizz, Buzz, 11, Fizz...
```

---

### idle.asm - PWM LED Animation

**Difficulty**: ⭐⭐⭐ Advanced

A beautiful LED fade animation using pulse-width modulation (PWM), demonstrating:

- Nested loops
- LED control macros
- PWM technique (duty cycle)
- Waveform generation
- DBRA instruction (decrement and branch)

**Source Walkthrough**:

```asm
fadespeed equ 6

animate_led:
        moveq      #0,d1            ; Duty cycle (0-255)
        moveq      #fadespeed,d2    ; Periods per duty cycle
        led_on

.cycle: move.l     #255,d0          ; 255 iterations per period
        led_tgl                     ; Start period with toggle

.loop:  cmp.b      d0,d1            ; Is count == duty cycle?
        bne        .1
        led_tgl                     ; Invert LED at duty cycle point

.1:     dbra       d0,.loop         ; 255 iterations (one PWM period)
        dbra       d2,.cycle        ; Repeat period several times

        ; Move to next duty cycle
        addq       #1,d1

        ; At 0, invert waveform (fade up -> fade down)
        cmp.b      #0,d1
        bne        .2
        led_tgl

.2:     moveq      #fadespeed,d2
        bra        .cycle
```

**What it does**:

1. Generates a PWM waveform on the LED pin
2. Gradually increases duty cycle (0% → 100%)
3. LED appears to smoothly fade brighter
4. When duty cycle wraps to 0, waveform inverts
5. LED then fades darker (100% → 0%)
6. Repeats forever in a breathing pattern

**Learning Points**:

- PWM technique for analog-like output from digital pins
- DBRA instruction (very efficient for loops)
- Nested loop structures
- Waveform generation and inversion
- LED macros (led_on, led_off, led_tgl)
- Using duty cycle to control brightness

**How PWM Works**:

Each PWM period (256 iterations):

```
Duty Cycle 25%:  ░░░░░░██████████████████████
Duty Cycle 50%:  ░░░░░░░░░░░░░░██████████████
Duty Cycle 75%:  ░░░░░░░░░░░░░░░░░░░░████████
```

By repeating this pattern quickly, human eyes perceive varying brightness.

**Try It**:

```bash
flux32 --vasm -m68000 -Fbin -o idle.bin idle.asm
flux32 --run ../rom.bin --app idle.bin
# Watch the LED breathe in and out
```

## Writing Your Own Programs

### Program Template

All programs should follow this structure:

```asm
        include    "../app.inc"    ; System headers

start:  ; Your code here
        sys        Exit            ; Return to shell

; Data section
data:   dc.b       "Hello!",0
```

### Key Macros

**System Calls**:

```asm
sys <name>              ; Invoke syscall (TRAP)
```

**Inline Strings**:

```asm
litstr "text"           ; Embed string, A0 points to it
litstr FMT_U16,0        ; Format specifier
```

**LED Control**:

```asm
led_on                  ; Turn LED on (DTR low)
led_off                 ; Turn LED off (DTR high)
led_tgl                 ; Toggle LED state
```

**Register Push/Pop**:

```asm
pushm d0-d2/a0-a1       ; Save registers
popm d0-d2/a0-a1        ; Restore registers
```

**Branch and Link**:

```asm
bl subroutine           ; Call subroutine (BSR)
rl                      ; Return from subroutine (RTS)
```

### Available System Calls

| Syscall | Macro               | Arguments | Description                   |
| ------- | ------------------- | --------- | ----------------------------- |
| TRAP 0  | `sys Exit`          | -         | Exit to shell                 |
| TRAP 1  | `sys WaitBtn`       | -         | Wait for button press/release |
| TRAP 2  | `sys OutChar`       | D0.B      | Write character to UART       |
| TRAP 3  | `sys OutStr`        | A0        | Write null-terminated string  |
| TRAP 4  | `sys OutFmt`        | A0, stack | Printf-style formatting       |
| TRAP 5  | `sys InChar`        | → D0.B    | Read character from UART      |
| TRAP 6  | `sys PromptStr`     | A0, D0.W  | Prompt and read string        |
| TRAP 7  | `sys ReadSector`    | D0.L, A0  | Read CF sector                |
| TRAP 8  | `sys ListDirectory` | A0, D0.L  | Iterate directory             |
| TRAP 9  | `sys FindFile`      | A0, A1    | Find file by name             |
| TRAP 10 | `sys ReadFile`      | A0, A1    | Read entire file              |
| TRAP 11 | `sys GetDateTime`   | A0        | Read RTC                      |
| TRAP 12 | `sys SetDateTime`   | A0        | Set RTC                       |
| TRAP 13 | `sys GetSysInfo`    | → A0      | Get system info               |
| TRAP 15 | `sys Breakpoint`    | -         | Trigger debugger              |

See [../README.md](../README.md) for complete syscall documentation.

### Format Specifiers

For `OutFmt` syscall:

```asm
; Print unsigned word
move.w     #1234,-(sp)         ; Push value
litstr     FMT_U16,0           ; Push format
sys        OutFmt
addq       #2,sp               ; Clean stack

; Print string and number
move.w     #42,-(sp)
pea        str
litstr     FMT_STR," = ",FMT_U16,0
sys        OutFmt
addq       #6,sp               ; 2+4 bytes
```

Available formats:

- `FMT_U8`, `FMT_U16`, `FMT_U32` - Unsigned decimal
- `FMT_S8`, `FMT_S16`, `FMT_S32` - Signed decimal
- `FMT_X8`, `FMT_X16`, `FMT_X32` - Hexadecimal
- `FMT_CHR` - Character
- `FMT_STR` - String pointer

### Memory Layout

Your program loads at `$E00100`:

```
$E00000-$E000FF   System variables (256 bytes)
$E00100           Your code starts here (PC)
$E00100+          Your data follows code
$F00000           Initial stack pointer (grows down)
```

Stack grows downward from `$F00000`. Keep data structures small to avoid collision.

### Calling Convention

Functions should follow these rules:

- **D0-D1**: Integer arguments and return values
- **A0-A1**: Pointer arguments and return values
- **D2-D7/A2-A7**: Preserved (callee-saved)
- **SR**: Preserved
- Arguments pushed right-to-left
- Caller cleans up stack

Example:

```asm
; Call myfunc(10, 20)
move.w     #20,-(sp)           ; Push right arg
move.w     #10,-(sp)           ; Push left arg
bsr        myfunc
addq       #4,sp               ; Clean up (2+2)

; In myfunc:
myfunc: move.w     4(sp),d0        ; Get first arg
        move.w     6(sp),d1        ; Get second arg
        add.w      d1,d0           ; Compute result
        rts                        ; D0 = return value
```

## Common Patterns

### Print Number in Decimal

```asm
move.w     value,-(sp)
litstr     FMT_U16,10,0        ; Format + newline
sys        OutFmt
addq       #2,sp
```

### Print Multiple Values

```asm
move.w     d2,-(sp)
move.w     d1,-(sp)
move.w     d0,-(sp)
litstr     FMT_U16," ",FMT_U16," ",FMT_U16,10,0
sys        OutFmt
addq       #6,sp
```

### Wait and Blink LED

```asm
loop:   sys        WaitBtn
        led_tgl
        ; ... do something ...
        bra        loop
```

### Delay Loop

```asm
delay:  move.l     #$100000,d0
.loop:  subq.l     #1,d0
        bne        .loop
        rts
```

### Read Character

```asm
        sys        InChar
        cmp.b      #'q',d0
        beq        quit
        ; ... process character ...
```

## Debugging Tips

### Use Breakpoints

```asm
        sys        Breakpoint      ; Enter debugger here
```

### Print Debug Values

```asm
        move.w     d0,-(sp)
        litstr     "D0=",FMT_X16,10,0
        sys        OutFmt
        addq       #2,sp
```

### Step Through in Debugger

```bash
flux32 myprogram.bin
# In debugger:
s              # Step one instruction
reg            # View registers
mem e00100     # View memory
dis            # Disassemble
```

## Going Further

### Ideas for New Programs

1. **Calculator** - Parse input, perform arithmetic, print result
2. **Text Editor** - Read lines, store in buffer, allow edits
3. **File Lister** - Use `ListDirectory` to show CF card contents
4. **Memory Game** - Random numbers, user guesses
5. **Benchmark** - Time instruction sequences, report performance
6. **Graphics Demo** - Use LED as 1-bit display with patterns
7. **Serial Terminal** - Echo characters with processing
8. **Tiny Shell** - Parse commands, execute actions

### Study the ROM

The ROM source (`../rom.asm`) shows advanced techniques:

- FAT16 filesystem implementation
- Command parser
- Formatted output engine
- Error handling
- Interrupt management

### Read Instruction Documentation

M68K Programmer's Reference Manual is invaluable:

- All instruction encodings
- Addressing modes
- Flag behaviors
- Timing information

## Tips and Tricks

### Efficient Loops

Use `DBRA` instead of manual decrement:

```asm
; Good
        moveq      #99,d0
.loop:  ; ... loop body ...
        dbra       d0,.loop

; Avoid
        moveq      #100,d0
.loop:  ; ... loop body ...
        subq       #1,d0
        bne        .loop
```

### Clear Registers Fast

```asm
moveq      #0,d0               ; Fast, 2 bytes
; vs
clr.l      d0                  ; Slower, 4 bytes
```

### Test Flags Efficiently

```asm
tst.w      d0                  ; Set flags based on D0
beq        zero                ; Branch if zero
bmi        negative            ; Branch if negative
```

### Inline Constants

```asm
moveq      #42,d0              ; Fast (-128 to 127)
move.w     #1234,d0            ; For larger values
```

## Assembler Invocation

### Basic Assembly

```bash
flux32 --vasm -m68000 -Fbin -o output.bin input.asm
```

### With Listing File

```bash
flux32 --vasm -m68000 -Fbin -o output.bin -L output.lst input.asm
```

### Verbose Errors

```bash
flux32 --vasm -m68000 -Fbin -wfail -o output.bin input.asm
```

### Common VASM Options

- `-m68000` - Target M68000 CPU
- `-Fbin` - Output flat binary
- `-o <file>` - Output filename
- `-L <file>` - Generate listing
- `-wfail` - Treat warnings as errors
- `-I<dir>` - Include search path

---

**Have fun exploring the M68K! 🚀**
