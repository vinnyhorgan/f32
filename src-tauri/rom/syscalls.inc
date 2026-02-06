; vim:noet:sw=8:ts=8:sts=8:ai:syn=asm68k
;
; Flux32 System Call Definitions
; TRAP-based syscall interface

;-------------------------------------------------------------------------------
; Calling conventions
;-------------------------------------------------------------------------------
; D0-D1 - integer arguments
; A0-A1 - pointer arguments
; Additional arguments passed on the stack
;
; D0-D1 - integer return values
; A0-A1 - pointer return values
; D2-D7 - preserved
; A2-A7 - preserved
; SR    - preserved
;
; Registers D0-D1/A0-A1 not used for return values may have undefined
; contents upon return (callee may use as scratch registers).
;
; Caller is responsible for removing stack arguments.

;-------------------------------------------------------------------------------
; System call numbers (TRAP vector offsets)
;-------------------------------------------------------------------------------
Exit            equ     0               ; Return to system
WaitBtn         equ     1               ; Wait for button press/release
OutChar         equ     2               ; Write single character
OutStr          equ     3               ; Write null-terminated string
OutFmt          equ     4               ; Formatted string output
InChar          equ     5               ; Read single character
PromptStr       equ     6               ; Prompt for string input
ReadSector      equ     7               ; Read CF card sector
ListDirectory   equ     8               ; Iterate directory entries
FindFile        equ     9               ; Find named file
ReadFile        equ     10              ; Read file into memory
GetDateTime     equ     11              ; Read from RTC
SetDateTime     equ     12              ; Set RTC time
GetSysInfo      equ     13              ; Get system info pointer
; 14 - reserved
Breakpoint      equ     15              ; Enter debugger

;-------------------------------------------------------------------------------
; Syscall macro
;-------------------------------------------------------------------------------
sys             macro
                trap    #\1
                endm

;-------------------------------------------------------------------------------
; Format specifiers for OutFmt
;-------------------------------------------------------------------------------
FMT_BASE        equ     $E0
FMT_C           equ     FMT_BASE+0      ; Pop word, print LSB as ASCII character
FMT_C2          equ     FMT_BASE+1      ; Pop word, print as two ASCII characters
FMT_C4          equ     FMT_BASE+2      ; Pop longword, print as four ASCII characters
FMT_H8          equ     FMT_BASE+3      ; Pop word, print LSB as two hex digits
FMT_H16         equ     FMT_BASE+4      ; Pop word, print as four hex digits
FMT_H32         equ     FMT_BASE+5      ; Pop longword, print as eight hex digits
FMT_S           equ     FMT_BASE+6      ; Pop longword, print null-terminated string at address
FMT_U8          equ     FMT_BASE+7      ; Pop word, print LSB as 1-3 decimal digits (unsigned)
FMT_U16         equ     FMT_BASE+8      ; Pop word, print as 1-5 decimal digits (unsigned)
FMT_U32         equ     FMT_BASE+9      ; Pop longword, print as 1-10 decimal digits (unsigned)
FMT_D8          equ     FMT_BASE+10     ; Pop word, print LSB as 1-3 decimal digits (signed)
FMT_D16         equ     FMT_BASE+11     ; Pop word, print as 1-5 decimal digits (signed)
FMT_D32         equ     FMT_BASE+12     ; Pop longword, print as 1-10 decimal digits (signed)
FMT_Z8          equ     FMT_BASE+13     ; Pop word, print LSB as 3 digits with leading zeros
FMT_Z16         equ     FMT_BASE+14     ; Pop word, print as 5 digits with leading zeros
FMT_Z32         equ     FMT_BASE+15     ; Pop longword, print as 10 digits with leading zeros
FMT_SRFLAGS     equ     FMT_BASE+16     ; Pop word, print as status register flags
FMT_FAULTFLAGS  equ     FMT_BASE+17     ; Pop word, print as bus/address error flags
FMT_DATE        equ     FMT_BASE+18     ; Pop longword, print as date (YYYY/MM/DD)
FMT_TIME        equ     FMT_BASE+19     ; Pop longword, print as time (HH:MM:SS)
FMT_HEXDUMP     equ     FMT_BASE+20     ; Pop length and address, print as hex dump
FMT_BUF         equ     FMT_BASE+21     ; Pop length and address, print as ASCII
FMT_FNAME       equ     FMT_BASE+22     ; Pop address, print 11 chars as decoded filename
FMT_ERR         equ     FMT_BASE+23     ; Pop word, print error description

;-------------------------------------------------------------------------------
; PromptStr option flags
;-------------------------------------------------------------------------------
PRbNOECHO       equ     8
PRbNOCTRLCHARS  equ     9

PR_NOECHO       equ     (1<<PRbNOECHO)          ; Don't echo received characters
PR_NOCTRLCHARS  equ     (1<<PRbNOCTRLCHARS)     ; Don't interpret control characters
