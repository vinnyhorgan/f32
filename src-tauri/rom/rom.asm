; vim:noet:sw=8:ts=8:sts=8:ai:syn=asm68k
;
; Flux32 System ROM
; A complete monitor/shell for the Flux32 M68K educational platform
;
; This ROM provides:
; - UART serial I/O with 16550 compatibility
; - CompactFlash card FAT16 filesystem support
; - Real-time clock via SPI (DS3234)
; - Interactive shell with file operations
; - Serial loader for uploading programs
; - Exception handlers with register dump
; - System calls via TRAP instructions

                 include    "flux32.inc"

;-------------------------------------------------------------------------------
; ROM Version Information
;-------------------------------------------------------------------------------
ROM_VER_MAJ    equ $0001
ROM_VER_MIN    equ $0000
ROM_DATE_YEAR  equ $2026
ROM_DATE_MONTH equ $02
ROM_DATE_DAY   equ $05

;-------------------------------------------------------------------------------
; Hardware Configuration
;-------------------------------------------------------------------------------
F_CPU          equ 8000000                                                                                 ; 8 MHz CPU clock
BAUD           equ 115200                                                                                  ; Serial baud rate
BAUD_DIV       equ (((F_CPU*10)/(16*BAUD))+5)/10
BAUD_DIV_L     equ (BAUD_DIV&$FF)
BAUD_DIV_U     equ ((BAUD_DIV>>8)&$FF)

;===============================================================================
; VECTOR TABLE
;===============================================================================
                 org        ROM
                 dc.l       INITIAL_SP                                                                     ; Initial SSP
                 dc.l       RESET                                                                          ; Initial PC
                 dc.l       VEC_BUSFAULT                                                                   ; Bus error
                 dc.l       VEC_ADRERROR                                                                   ; Address error
                 dc.l       VEC_ILLINSTR                                                                   ; Illegal instruction
                 dc.l       VEC_DIVBY0                                                                     ; Zero divide
                 dc.l       VEC_CHK                                                                        ; CHK instruction
                 dc.l       VEC_TRAPV                                                                      ; TRAPV instruction
                 dc.l       VEC_PRIVVIOL                                                                   ; Privilege violation
                 dc.l       VEC_TRACE                                                                      ; Trace
                 dc.l       VEC_LINE1010                                                                   ; Line 1010 emulator
                 dc.l       VEC_LINE1111                                                                   ; Line 1111 emulator
                 dc.l       VEC_RESERVED                                                                   ; 12 - Reserved
                 dc.l       VEC_RESERVED                                                                   ; 13 - Reserved
                 dc.l       VEC_RESERVED                                                                   ; 14 - Reserved
                 dc.l       VEC_UNINIVEC                                                                   ; Uninitialized interrupt
                 rept       8
                 dc.l       VEC_RESERVED                                                                   ; 16-23 - Reserved
                 endr
                 dc.l       VEC_SPURIOUS                                                                   ; Spurious interrupt
                 dc.l       VEC_AUTOVEC1                                                                   ; Autovector 1 (serial loader)
                 dc.l       VEC_AUTOVEC2                                                                   ; Autovector 2
                 dc.l       VEC_AUTOVEC3                                                                   ; Autovector 3
                 dc.l       VEC_AUTOVEC4                                                                   ; Autovector 4
                 dc.l       VEC_AUTOVEC5                                                                   ; Autovector 5
                 dc.l       VEC_AUTOVEC6                                                                   ; Autovector 6
                 dc.l       VEC_AUTOVEC7                                                                   ; Autovector 7

        ; System calls (TRAP #n)
                 dc.l       SYS_Exit                                                                       ; 0  - Return to system
                 dc.l       SYS_WaitBtn                                                                    ; 1  - Wait for button press/release
                 dc.l       SYS_OutChar                                                                    ; 2  - Single character output
                 dc.l       SYS_OutStr                                                                     ; 3  - String output
                 dc.l       SYS_OutFmt                                                                     ; 4  - Formatted string output
                 dc.l       SYS_InChar                                                                     ; 5  - Single character input
                 dc.l       SYS_PromptStr                                                                  ; 6  - Prompt for string input
                 dc.l       SYS_ReadSector                                                                 ; 7  - Read sector from CF card
                 dc.l       SYS_ListDirectory                                                              ; 8  - Iterate through directory
                 dc.l       SYS_FindFile                                                                   ; 9  - Find named file
                 dc.l       SYS_ReadFile                                                                   ; 10 - Read file into memory
                 dc.l       SYS_GetDateTime                                                                ; 11 - Read from RTC
                 dc.l       SYS_SetDateTime                                                                ; 12 - Set RTC time
                 dc.l       SYS_GetSysInfo                                                                 ; 13 - Get system info pointer
                 dc.l       $FFFFFFFF                                                                      ; 14 - Reserved
                 dc.l       VEC_BREAKPT                                                                    ; 15 - Breakpoint (debugger)

;===============================================================================
; RESET HANDLER
;===============================================================================
RESET:
        ; Initialize UART
                 lea.l      UART,a1
                 move.b     #%00001101,FCR(a1)                                                             ; Enable FIFO
                 move.b     #%10000011,LCR(a1)                                                             ; 8N1, DLAB=1
                 move.b     #BAUD_DIV_L,DLL(a1)                                                            ; Divisor latch low
                 move.b     #BAUD_DIV_U,DLM(a1)                                                            ; Divisor latch high
                 bclr.b     #7,LCR(a1)                                                                     ; DLAB=0
                 clr.b      SCR(a1)                                                                        ; Clear scratchpad
                 move.b     #(1<<MCR_COPI),MCR(a1)                                                         ; SPI COPI idles low

        ; Save button state at boot
                 move.b     MSR(a1),SCR(a1)

        ; Welcome message
                 led_on
                 lea.l      str_startup,a0
                 bl         _printstr
                 move.l     rom_version,d0
                 bl         _printhexl
                 moveq      #' ',d0
                 tx_char    d0,a1
                 moveq      #'(',d0
                 tx_char    d0,a1
                 move.l     rom_date,d0
                 bl         _printhexl
                 lea.l      str_credits,a0
                 bl         _printstr

;-------------------------------------------------------------------------------
; Power-on self-test: RAM test
;-------------------------------------------------------------------------------
                 lea.l      str_ramtest,a0
                 bl         _printstr

        ; Write test pattern
                 lea.l      RAM,a0
                 lea.l      RAMEND,a3
                 move.l     #$A5C99C5A,d0                                                                  ; Test pattern 1
                 move.l     d0,d1
                 not.l      d1                                                                             ; Test pattern 2
.write_loop:
                 rept       8
                 move.l     d0,(a0)+
                 move.l     d1,(a0)+
                 endr
                 cmp.l      a0,a3
                 bne        .write_loop

        ; Read back and verify
                 lea.l      RAM,a0
                 lea.l      RAMEND,a3
.read_loop:
                 rept       8
                 cmp.l      (a0)+,d0
                 bne        testfail
                 cmp.l      (a0)+,d1
                 bne        testfail
                 endr
                 cmp.l      a0,a3
                 bne        .read_loop

testpass:        lea.l      str_testpass,a0
                 bl         _printstr
                 bra        ready

testfail:        lea.l      -4(a0),a2                                                                      ; Address of failure
                 lea.l      str_testfail,a0
                 bl         _printstr
                 move.l     a2,d0
                 bl         _printhexl

        ; Flash LED to indicate failure
lockup:          led_tgl
                 move.l     #$8000,d0
.1:              dbra       d0,.1
                 bra        lockup

;-------------------------------------------------------------------------------
; Startup strings
;-------------------------------------------------------------------------------
str_startup:     asciz      "\n\nFLUX32 - ROM VERSION "
str_credits:     asciz      ")\nFLUX32 M68K EDUCATIONAL PLATFORM\n"
str_ramtest:     asciz      "TESTING RAM..."
str_testpass:    asciz      "PASSED\n"
str_testfail:    asciz      "FAILED AT "
                 even

;-------------------------------------------------------------------------------
; Early boot print routines (no stack required)
;-------------------------------------------------------------------------------

; Print D0 as 8 hex digits
; Clobbers D1, D2
_printhexl:      moveq      #7,d1
.loop:           rol.l      #4,d0
                 move.w     d0,d2
                 and.w      #%1111,d2
                 move.b     (hexdigits,pc,d2),d2
                 tx_wait    a1
                 move.b     d2,THR(a1)
                 dbra       d1,.loop
                 rl

hexdigits:       dc.b       "0123456789ABCDEF"

; Print null-terminated string in A0
; Clobbers D0
_printstr:       move.b     (a0)+,d0
                 beq        .done
.1:              btst.b     #5,LSR(a1)
                 beq        .1
                 move.b     d0,THR(a1)
                 bra        _printstr
.done:           rl

;===============================================================================
; SYSTEM READY - Memory is usable
;===============================================================================
ready:
                 move.l     #INITIAL_SP,sp                                                                 ; Reset stack pointer
                 move.l     #uart_outchar,OUTCH_VEC                                                        ; Default I/O is serial
                 move.l     #uart_inchar,INCH_VEC
                 move.l     #hexdigits_uc,HEXDIGITS
                 move.l     #$2d3a2c00,SEPARATORS                                                          ; Hyphen, colon, comma

        ; Print system info
                 sys        GetSysInfo
                 move.l     0(a0),-(sp)                                                                    ; Clock speed
                 move.l     8(a0),-(sp)                                                                    ; ROM size
                 move.l     4(a0),-(sp)                                                                    ; RAM size
                 lea.l      fmt_sysinfo,a0
                 sys        OutFmt
                 lea.l      12(sp),sp

        ; Check for RTC
                 bsr        printtime

        ; Enable break interrupt for serial loader
                 move.b     #%00000100,IER(a1)
                 move.w     #$2000,sr                                                                      ; Enable interrupts

        ; Try to mount filesystem
                 bsr        fs_mount
                 beq        .foundcard

        ; Mount error
                 bsr        fs_errorstr
                 move.w     d0,-(sp)
                 litstr     FMT_ERR,"\n"
                 sys        OutFmt
                 addq       #2,sp
                 bra        idle

.printcardmsg:   sys        OutStr
                 bra        idle

.foundcard:      litstr     "CARD DETECTED: ",FMT_U32," KB '",FMT_S,"'\n"
                 pea        VOLNAME
                 move.l     PARTSIZE,d0
                 lsr.l      #1,d0                                                                          ; Convert sectors to KB
                 move.l     d0,-(sp)
                 sys        OutFmt
                 addq       #8,sp

        ; Check for startup bypass (button held at boot)
                 btst.b     #MSR_BTN1,UART+SCR
                 bne        .skipstartup

        ; Try to run STARTUP.BIN
                 moveq      #-1,d0
                 lea.l      APPMEMSTART,a1
                 litstr     "STARTUP.BIN"
                 sys        ReadFile
                 tst.b      d0
                 bne        .nostartup

        ; Launch startup file
                 litstr     "RUNNING STARTUP.BIN\n"
                 sys        OutStr
                 bra        launchapp

.nostartup:      move.w     d0,-(sp)
                 litstr     "CANNOT LOAD STARTUP.BIN - ",FMT_ERR,"\n"
                 sys        OutFmt
                 addq       #2,sp
                 bra        idle

.skipstartup:    litstr     "BYPASSING STARTUP.BIN\n"
                 sys        OutStr

idle:            bra        startshell

;===============================================================================
; SYSTEM CALLS
;===============================================================================

; Exit - Return to system
SYS_Exit:        bra        ready

; WaitBtn - Wait for button press and release
SYS_WaitBtn:     lea.l      UART,a1
.waitpress:      btst       #MSR_BTN1,MSR(a1)
                 beq        .waitpress
                 moveq      #-1,d0
.debounce:       dbra       d0,.debounce
.waitrelease:    btst       #MSR_BTN1,MSR(a1)
                 bne        .waitrelease
                 rte

; OutChar - Write one character
; D0.B = character
SYS_OutChar:     move.l     OUTCH_VEC,a1
                 jsr        (a1)
                 rte

; OutStr - Write null-terminated string
; A0.L = pointer to string
SYS_OutStr:      pushm      a2-a3
                 move.l     a0,a2
                 move.l     OUTCH_VEC,a3
.1:              move.b     (a2)+,d0
                 beq        .2
                 jsr        (a3)
                 bra        .1
.2:              popm       a2-a3
                 rte

; OutFmt - Formatted string output
; A0.L = format string, arguments on stack
SYS_OutFmt:      pushm      d2/a2-a6
                 lea.l      30(sp),a6                                                                      ; Point to arguments
                 move.l     a0,a2
                 move.l     OUTCH_VEC,a3
                 move.l     HEXDIGITS,a5
fmtchar:         moveq      #0,d0
                 move.b     (a2)+,d0
                 add.w      d0,d0
                 move.w     fmt_jumptable(pc,d0.w),d1
                 jmp        fmt_jumptable(pc,d1.w)

fmt_jumptable:   dc.w       .fmt_nullbyte-fmt_jumptable
                 rept       FMT_BASE-1
                 dc.w       .fmt_literalchar-fmt_jumptable
                 endr
                 dc.w       .fmt_char-fmt_jumptable
                 dc.w       .fmt_char2-fmt_jumptable
                 dc.w       .fmt_char4-fmt_jumptable
                 dc.w       .fmt_hex8-fmt_jumptable
                 dc.w       .fmt_hex16-fmt_jumptable
                 dc.w       .fmt_hex32-fmt_jumptable
                 dc.w       .fmt_str-fmt_jumptable
                 dc.w       .fmt_u8-fmt_jumptable
                 dc.w       .fmt_u16-fmt_jumptable
                 dc.w       .fmt_u32-fmt_jumptable
                 dc.w       .fmt_d8-fmt_jumptable
                 dc.w       .fmt_d16-fmt_jumptable
                 dc.w       .fmt_d32-fmt_jumptable
                 dc.w       .fmt_z8-fmt_jumptable
                 dc.w       .fmt_z16-fmt_jumptable
                 dc.w       .fmt_z32-fmt_jumptable
                 dc.w       .fmt_srbits-fmt_jumptable
                 dc.w       .fmt_fltbits-fmt_jumptable
                 dc.w       .fmt_date-fmt_jumptable
                 dc.w       .fmt_time-fmt_jumptable
                 dc.w       .fmt_hexdump-fmt_jumptable
                 dc.w       .fmt_buf-fmt_jumptable
                 dc.w       .fmt_fname-fmt_jumptable
                 dc.w       .fmt_err-fmt_jumptable

.fmt_nullbyte:   popm       d2/a2-a6
                 rte

.fmt_literalchar:
                 lsr.w      #1,d0
                 jsr        (a3)
                 bra        fmtchar

.fmt_char:       move.w     (a6)+,d0
                 jsr        (a3)
                 bra        fmtchar

.fmt_char2:      move.w     (a6)+,d0
.1:              ror.w      #8,d0
                 jsr        (a3)
                 ror.w      #8,d0
                 jsr        (a3)
                 bra        fmtchar

.fmt_char4:      move.l     (a6)+,d0
                 swap       d0
                 ror.w      #8,d0
                 jsr        (a3)
                 ror.w      #8,d0
                 jsr        (a3)
                 swap       d0
                 bra        .1

.fmt_hex8:       move.w     (a6)+,d0
                 pea        fmtchar
                 bra        printhexbyte

.fmt_hex32:      move.w     (a6)+,d0
                 bsr        printhexword
.fmt_hex16:      move.w     (a6)+,d0
                 pea        fmtchar
                 bra        printhexword

.fmt_date:       move.w     (a6)+,d0
                 bclr.l     #15,d0
                 lea.l      DATE_SEP,a4
                 bsr        printhexword
                 move.b     (a4),d0
                 jsr        (a3)
                 move.b     (a6)+,d0
                 bsr        printhexbyte
                 move.b     (a4),d0
                 jsr        (a3)
                 move.b     (a6)+,d0
                 pea        fmtchar
                 bra        printhexbyte

.fmt_time:       move.w     (a6)+,d0
                 lea.l      TIME_SEP,a4
                 bsr        printhexbyte
                 move.b     (a4),d0
                 jsr        (a3)
                 move.b     (a6)+,d0
                 bsr        printhexbyte
                 move.b     (a4),d0
                 jsr        (a3)
                 move.b     (a6)+,d0
                 pea        fmtchar
                 bra        printhexbyte

.fmt_hexdump:    move.l     (a6)+,a4
                 moveq      #0,d2
.hexdumploop:    cmp.l      (a6),d2
                 beq        .dumpend
                 moveq      #$0F,d0
                 and.b      d2,d0
                 bne        .midline
                 moveq      #$0a,d0
                 jsr        (a3)
                 move.l     d2,d0
                 swap       d0
                 bsr        printhexword
                 move.w     d2,d0
                 bsr        printhexword
                 moveq      #':',d0
                 jsr        (a3)
                 moveq      #' ',d0
                 jsr        (a3)
.midline:        move.b     (a4)+,d0
                 bsr        printhexbyte
                 moveq      #' ',d0
                 jsr        (a3)
                 addq.l     #1,d2
                 bra        .hexdumploop
.dumpend:        neg.w      d2
                 and.w      #$000F,d2
                 beq        .nopad
                 subq       #1,d2
.padloop:        moveq      #' ',d0
                 jsr        (a3)
                 jsr        (a3)
                 jsr        (a3)
                 dbra       d2,.padloop
.nopad:          addq       #4,a6
                 moveq      #$0a,d0
                 pea        fmtchar
                 jmp        (a3)

.fmt_buf:        move.l     (a6)+,a4
                 move.l     (a6)+,d2
                 beq        fmtchar
.bufloop:        move.b     (a4)+,d0
                 jsr        (a3)
                 subq.l     #1,d2
                 bne        .bufloop
                 bra        fmtchar

.fmt_str:        move.l     (a6)+,a4
.strloop:        move.b     (a4)+,d0
                 beq        fmtchar
                 jsr        (a3)
                 bra        .strloop

.fmt_fname:      move.l     (a6)+,a0
                 lea.l      -14(sp),sp
                 move.l     sp,a1
                 bsr        fname_decode
                 move.l     a1,a4
.fnameloop:      move.b     (a4)+,d0
                 beq        .fnamedone
                 jsr        (a3)
                 bra        .fnameloop
.fnamedone:      lea.l      14(sp),sp
                 bra        fmtchar

.fmt_err:        move.w     (a6)+,d0
                 bsr        fs_errorstr
                 beq        .othererr
                 move.l     a0,a4
                 bra        .strloop
.othererr:       pea        fmtchar
                 bra        printhexbyte

.fmt_d8:         move.w     (a6)+,d2
                 ext.w      d2
                 bpl        .dec8
                 moveq      #'-',d0
                 jsr        (a3)
                 neg.b      d2
                 and.l      #$FF,d2
                 bra        .dec8

.fmt_u8:         move.w     (a6)+,d2
                 andi.w     #$00FF,d2
                 bra        .dec8

.fmt_z8:         move.w     (a6)+,d2
                 andi.w     #$00FF,d2
                 bra        .hundreds

.fmt_z16:        move.w     (a6)+,d2
                 bra        .tthousands

.fmt_u32:        move.l     (a6)+,d2
.dec32:          cmp.l      #$FFFF,d2
                 bls        .dec16
                 cmp.l      #1000000000,d2
                 bcc        .billions
                 cmp.l      #100000000,d2
                 bcc        .hmillions
                 cmp.l      #10000000,d2
                 bcc        .tmillions
                 cmp.l      #1000000,d2
                 bcc        .millions
                 cmp.l      #100000,d2
                 bcc        .hthousands
                 bra        .tthousands_l

.fmt_d16:        move.w     (a6)+,d2
                 bpl        .dec16
                 moveq      #'-',d0
                 jsr        (a3)
                 neg.w      d2
                 bra        .dec16

.fmt_u16:        move.w     (a6)+,d2
.dec16:          cmp.w      #10000,d2
                 bcc        .tthousands
                 cmp.w      #1000,d2
                 bcc        .thousands
.dec8:           cmp.w      #100,d2
                 bcc        .hundreds
                 cmp.w      #10,d2
                 bcc        .tens
                 bra        .ones

.fmt_d32:        move.l     (a6)+,d2
                 bpl        .dec32
                 moveq      #'-',d0
                 jsr        (a3)
                 neg.l      d2
                 bra        .dec32

.fmt_z32:        move.l     (a6)+,d2
                 cmp.l      #$FFFF,d2
                 bhi        .billions
                 moveq      #'0',d0
                 jsr        (a3)
                 jsr        (a3)
                 jsr        (a3)
                 jsr        (a3)

.tthousands:     move.l     #10000,d1
                 moveq      #'/',d0
.loop10000:      addq       #1,d0
                 sub.w      d1,d2
                 bcc        .loop10000
                 add.w      d1,d2
                 jsr        (a3)

.thousands:      move.w     #1000,d1
                 moveq      #'/',d0
.loop1000:       addq       #1,d0
                 sub.w      d1,d2
                 bcc        .loop1000
                 add.w      d1,d2
                 jsr        (a3)
                 move.b     THOUSANDS_SEP,d0
                 beq        .hundreds
                 jsr        (a3)

.hundreds:       moveq      #100,d1
                 moveq      #'/',d0
.loop100:        addq       #1,d0
                 sub.w      d1,d2
                 bcc        .loop100
                 add.w      d1,d2
                 jsr        (a3)

.tens:           moveq      #10,d1
                 moveq      #'/',d0
.loop10:         addq       #1,d0
                 sub.w      d1,d2
                 bcc        .loop10
                 add.w      d1,d2
                 jsr        (a3)

.ones:           moveq      #'0',d0
                 add.b      d2,d0
                 jsr        (a3)
                 bra        fmtchar

.billions:       move.l     #1000000000,d1
                 moveq      #'/',d0
.loop1e9:        addq       #1,d0
                 sub.l      d1,d2
                 bcc        .loop1e9
                 add.l      d1,d2
                 jsr        (a3)
                 move.b     THOUSANDS_SEP,d0
                 beq        .hmillions
                 jsr        (a3)

.hmillions:      move.l     #100000000,d1
                 moveq      #'/',d0
.loop1e8:        addq       #1,d0
                 sub.l      d1,d2
                 bcc        .loop1e8
                 add.l      d1,d2
                 jsr        (a3)

.tmillions:      move.l     #10000000,d1
                 moveq      #'/',d0
.loop1e7:        addq       #1,d0
                 sub.l      d1,d2
                 bcc        .loop1e7
                 add.l      d1,d2
                 jsr        (a3)

.millions:       move.l     #1000000,d1
                 moveq      #'/',d0
.loop1e6:        addq       #1,d0
                 sub.l      d1,d2
                 bcc        .loop1e6
                 add.l      d1,d2
                 jsr        (a3)
                 move.b     THOUSANDS_SEP,d0
                 beq        .hthousands
                 jsr        (a3)

.hthousands:     move.l     #100000,d1
                 moveq      #'/',d0
.loop1e5:        addq       #1,d0
                 sub.l      d1,d2
                 bcc        .loop1e5
                 add.l      d1,d2
                 jsr        (a3)

.tthousands_l:   move.l     #10000,d1
                 moveq      #'/',d0
.loop1e4:        addq       #1,d0
                 sub.l      d1,d2
                 bcc        .loop1e4
                 add.l      d1,d2
                 jsr        (a3)
                 bra        .thousands

.fmt_srbits:     move.w     (a6)+,d2
                 lea.l      .srflagchars,a4
                 btst.l     #15,d2
                 bsr        printflag
                 btst.l     #13,d2
                 bsr        printflag
                 move.w     d2,d0
                 lsr.w      #8,d0
                 and.b      #7,d0
                 add.b      #'0',d0
                 jsr        (a3)
                 btst.l     #4,d2
                 bsr        printflag
                 btst.l     #3,d2
                 bsr        printflag
                 btst.l     #2,d2
                 bsr        printflag
                 btst.l     #1,d2
                 bsr        printflag
                 btst.l     #0,d2
                 bsr        printflag
                 bra        fmtchar

.fmt_fltbits:    move.w     (a6)+,d2
                 lea.l      .fltflagchars,a4
                 btst.l     #4,d2
                 bsr        printflag
                 btst.l     #3,d2
                 bsr        printflag
                 and.b      #7,d2
                 moveq      #'0',d0
                 add.b      d2,d0
                 jsr        (a3)
                 bra        fmtchar

.srflagchars:    dc.b       "-T-S-X-N-Z-V-C"
.fltflagchars:   dc.b       "WRIN"
                 even

printflag:       beq        .flag0
.flag1:          move.w     (a4)+,d0
                 jmp        (a3)
.flag0:          move.b     (a4)+,d0
                 addq       #1,a4
                 jmp        (a3)

printhexbyte:    andi.l     #$00FF,d0
                 ror.l      #8,d0
_lsb:            rol.l      #4,d0
                 move.b     (a5,d0.w),d0
                 jsr        (a3)
                 clr.w      d0
                 rol.l      #4,d0
                 move.b     (a5,d0.w),d0
                 jmp        (a3)

printhexword:    swap       d0
                 clr.w      d0
                 rol.l      #4,d0
                 move.b     (a5,d0.w),d0
                 jsr        (a3)
                 clr.w      d0
                 rol.l      #4,d0
                 move.b     (a5,d0.w),d0
                 jsr        (a3)
                 clr.w      d0
                 bra        _lsb

; InChar - Read one character
; Returns: D0.B = character
SYS_InChar:      move.l     INCH_VEC,a1
                 jsr        (a1)
                 rte

; PromptStr - Prompt for string input
; A0.L = destination buffer
; D0.L = maximum length
; D1.W = delimiter and flags
; Returns: A0.L = buffer start, D0.L = string length
SYS_PromptStr:   pushm      a0/a2-a3/d2
                 move.l     a0,a2
                 lea.l      (a0,d0),a3
                 move.w     d1,d2
.prompt:         move.l     INCH_VEC,a1
                 jsr        (a1)
.tstchar:        cmp.b      d0,d2
                 beq        .founddelim
                 btst.l     #PRbNOCTRLCHARS,d2
                 bne        .noctrlchar
                 cmp.b      #$08,d0
                 beq        .backspace
                 cmp.b      #$7f,d0
                 beq        .backspace
                 cmp.b      #$03,d0
                 beq        .abort
                 cmp.b      #$0d,d0
                 bne        .noctrlchar
                 moveq      #$0a,d0
                 bra        .tstchar
.noctrlchar:     cmp.l      a2,a3
                 beq        .prompt
                 move.b     d0,(a2)+
                 btst.l     #PRbNOECHO,d2
                 bne        .prompt
                 sys        OutChar
                 bra        .prompt
.founddelim:     move.b     #0,(a2)
                 move.l     a2,d0
                 popm       a0/a2-a3/d2
                 sub.l      a0,d0
                 rte
.backspace:      cmp.l      4(sp),a2
                 beq        .prompt
                 subq       #1,a2
                 btst.l     #PRbNOECHO,d2
                 bne        .prompt
                 lea.l      .backspacestr,a0
                 sys        OutStr
                 bra        .prompt
.abort:          popm       a0/a2-a3/d2
                 move.b     #0,(a0)
                 clr.b      d0
                 rte
.backspacestr:   dc.b       $08,$20,$08,0

; ReadSector - Read one sector from CF card
; D0.L = LBA sector number
; D1.L = buffer size (>= 512 for full sector)
; A0.L = destination buffer (word-aligned)
; Returns: D0.B = error code
SYS_ReadSector:  move.l     d1,-(sp)
                 lea.l      CFCARD,a1
                 rol.w      #8,d0
                 swap       d0
                 rol.w      #8,d0
                 or.b       #$E0,d0
                 movep.l    d0,CF_LBA0(a1)
                 move.b     #1,CF_COUNT(a1)
                 moveq      #CFCMD_RDSECTOR,d0
                 bsr        _cfcard_sendcmd
                 bne        .done
                 bsr        _cfcard_waitfordata
                 bne        .done
                 move.l     #SECTORSIZE,d0
                 cmp.l      d0,d1
                 bls        .1
                 move.l     d0,d1
.1:              sub.l      d1,(sp)
                 lsr.w      #1,d1
                 beq        .2
                 bra        .3
.readloop:       move.w     CF_DATA(a1),d0
                 ror.w      #8,d0
                 move.w     d0,(a0)+
.3:              dbra       d1,.readloop
.2:              roxr.b     #1,d1
                 bpl        .flush
                 move.w     CF_DATA(a1),d0
                 move.b     d0,(a0)+
.flush:          tst.w      CF_DATA(a1)
                 btst.b     #3,CF_STATUS(a1)
                 bne        .flush
                 moveq      #0,d0
.done:           move.l     (sp)+,d1
                 rte

; ListDirectory - Traverse root directory
; A0.L = buffer (>= DIRBUFSIZE bytes)
; D0.L = nonzero for first call, zero for subsequent
; Returns: A1.L = directory entry, D0.W = status
SYS_ListDirectory:
                 tst.l      d0
                 beq        .1
                 bsr        fs_rdirlist
                 bne        .error
.1:              bsr        fs_dirnext
.error:          rte

; FindFile - Find named file
; A0.L = filename string
; A1.L = 32-byte buffer for directory entry
; Returns: D0.B = error code
SYS_FindFile:    bsr        fs_findfile
                 rte

; ReadFile - Read file into memory
; A0.L = filename string
; A1.L = destination buffer
; D0.L = maximum size
; Returns: D0.B = error, D1.L = bytes read
SYS_ReadFile:    bsr        fs_loadfile
                 rte

; GetDateTime - Read from RTC
; Returns: D0.L = date, D1.L = time
SYS_GetDateTime:
                 link       a6,#-8
                 bsr        spi_startxfer
                 moveq      #0,d0
                 bsr        spi_shiftbyte
                 bsr        spi_shiftbyte
                 move.b     d0,-(a6)
                 bmi        .rtc_error
                 bsr        spi_shiftbyte
                 move.b     d0,-(a6)
                 bsr        spi_shiftbyte
                 move.b     d0,-(a6)
                 bsr        spi_shiftbyte
                 move.b     d0,-(a6)
                 bsr        spi_shiftbyte
                 move.b     d0,-(a6)
                 bsr        spi_shiftbyte
                 tst.b      d0
                 bmi        .twenty_second_century
                 move.b     d0,-(a6)
                 bsr        spi_shiftbyte
                 move.b     d0,-(a6)
                 move.b     #$20,-(a6)
                 bra        .1
.twenty_second_century:
                 bclr.l     #7,d0
                 move.b     d0,-(a6)
                 bsr        spi_shiftbyte
                 move.b     d0,-(a6)
                 move.b     #$21,-(a6)
.1:              bsr        spi_endxfer
                 bsr        spi_startxfer
                 moveq      #$0F,d0
                 bsr        spi_shiftbyte
                 bsr        spi_shiftbyte
                 tst.b      d0
                 bpl        .nopwrloss
                 bset.b     #7,(a6)
.nopwrloss:      bsr        spi_endxfer
                 popm       d0-d1/a6
                 rte
.rtc_error:      bsr        spi_endxfer
                 moveq      #0,d0
                 moveq      #0,d1
                 addq       #8,sp
                 pop        a6
                 rte

; SetDateTime - Set RTC time
; D0.L = date, D1.L = time
SYS_SetDateTime:
                 link       a6,#0
                 movem.l    d0-d1,-(sp)
                 bsr        spi_startxfer
                 move.l     #$80,d0
                 bsr        spi_shiftbyte
                 move.b     -(a6),d0
                 bsr        spi_shiftbyte
                 move.b     -(a6),d0
                 bsr        spi_shiftbyte
                 move.b     -(a6),d0
                 bsr        spi_shiftbyte
                 move.b     -(a6),d0
                 bsr        spi_shiftbyte
                 move.b     -(a6),d0
                 bsr        spi_shiftbyte
                 move.b     -(a6),d0
                 btst.b     #0,-2(a6)
                 beq        .1
                 bset.l     #7,d0
.1:              bsr        spi_shiftbyte
                 move.b     -(a6),d0
                 bsr        spi_shiftbyte
                 bsr        spi_endxfer
                 bsr        spi_startxfer
                 move.l     #$8F,d0
                 bsr        spi_shiftbyte
                 moveq      #$40,d0
                 bsr        spi_shiftbyte
                 bsr        spi_endxfer
                 addq       #8,sp
                 pop        a6
                 rte

; GetSysInfo - Get system info pointer
; Returns: A0.L = pointer to system info structure
SYS_GetSysInfo:  lea.l      sysinfo,a0
                 rte

;===============================================================================
; CF CARD AND FILESYSTEM
;===============================================================================

cfcard_init:     moveq      #-1,d0
.1:              cmp.b      #$50,CFCARD+CF_STATUS
                 beq        .found
                 dbeq       d0,.1
.notfound:       bclr.b     #7,UART+SCR
                 bra        .2
.found:          bset.b     #7,UART+SCR
.2:              rts

cfcard_sendcmd:
                 lea.l      CFCARD,a1
_cfcard_sendcmd:
                 move.b     d0,CF_COMMAND(a1)
                 moveq      #-1,d0
.busy:           btst.b     #7,CF_STATUS(a1)
                 beq        .notbusy
                 dbra       d0,.busy
.timeout:        moveq      #FSERR_TIMEOUT,d0
                 rts
.notbusy:        moveq      #-1,d0
.notready:       btst.b     #4,CF_STATUS(a1)
                 bne        .ready
                 dbra       d0,.notready
.rdytimeout:     btst.b     #0,CF_STATUS(a1)
                 bne        .err
                 bra        .timeout
.ready:          btst.b     #0,CF_STATUS(a1)
                 bne        .err
                 moveq      #0,d0
                 rts
.err:            moveq      #0,d0
                 move.b     CF_ERROR(a1),d0
                 rts

cfcard_waitfordata:
                 lea.l      CFCARD,a1
_cfcard_waitfordata:
                 moveq      #-1,d0
.nodata:         btst.b     #3,CF_STATUS(a1)
                 bne        .gotdata
                 dbra       d0,.nodata
.datatimeout:    btst.b     #0,CF_STATUS(a1)
                 bne        .err
                 moveq      #FSERR_TIMEOUT,d0
                 rts
.gotdata:        btst.b     #0,CF_STATUS(a1)
                 bne        .err
                 moveq      #0,d0
                 rts
.err:            moveq      #0,d0
                 move.b     CF_ERROR(a1),d0
                 rts

; Mount FAT16 filesystem
fs_mount:        move.w     #(FSVARLEN/2)-1,d0
                 lea.l      FSVARSTART,a0
.1:              clr.w      (a0)+
                 dbra       d0,.1
                 link       a6,#-SECTORSIZE
                 moveq      #0,d0
                 move.l     sp,a0
                 moveq      #-1,d1
                 sys        ReadSector
                 tst        d0
                 bne        .error
                 lea.l      $1BE(sp),a0
                 move.b     4(a0),d0
                 cmp.b      #$04,d0
                 beq        .isfat16
                 cmp.b      #$06,d0
                 beq        .isfat16
                 cmp.b      #$0B,d0
                 beq        .isfat16
.notfat16:       moveq      #FSERR_WRONGTYPE,d0
                 bra        .error
.isfat16:        lea.l      16(a0),a0
                 lea.l      PARTSIZE,a1
                 move.b     -(a0),(a1)+
                 move.b     -(a0),(a1)+
                 move.b     -(a0),(a1)+
                 move.b     -(a0),(a1)+
                 move.b     -(a0),(a1)+
                 move.b     -(a0),(a1)+
                 move.b     -(a0),(a1)+
                 move.b     -(a0),(a1)+
                 move.l     BPBSECTOR,d0
                 move.l     sp,a0
                 moveq      #-1,d1
                 sys        ReadSector
                 tst        d0
                 bne        .error
                 lea.l      24(sp),a0
                 lea.l      FATSIZE,a1
                 move.b     -(a0),(a1)+
                 move.b     -(a0),(a1)+
                 subq       #3,a0
                 move.b     -(a0),(a1)+
                 move.b     -(a0),(a1)+
                 clr.b      (a1)+
                 move.b     -(a0),(a1)+
                 move.b     -(a0),(a1)+
                 move.b     -(a0),(a1)+
                 clr.b      (a1)+
                 move.b     -(a0),(a1)+
                 move.b     -(a0),d0
                 lsl.w      #8,d0
                 move.b     -(a0),d0
                 cmp.w      #SECTORSIZE,d0
                 bne        .not512bps
                 tst.w      MAXRDIRENTS
                 beq        .notfat16
                 lea.l      32(a0),a0
                 lea.l      VOLNAME,a1
                 moveq      #10,d0
.namecopy:       move.b     (a0)+,(a1)+
                 dbra       d0,.namecopy
                 moveq      #0,d0
                 move.w     RSVDSECTORS,d0
                 add.l      BPBSECTOR,d0
                 move.l     d0,FATSECTOR
                 move.w     FATCOPIES,d1
                 mulu.w     FATSIZE,d1
                 add.l      d1,d0
                 move.l     d0,RDIRSECTOR
                 move.w     MAXRDIRENTS,d1
                 lsr.w      #4,d1
                 ext.l      d1
                 add.l      d1,d0
                 move.l     d0,DATASTART
                 moveq      #0,d0
.2:              unlk       a6
                 rts
.not512bps:      moveq      #FSERR_BPS,d0
.error:          clr.l      PARTSIZE
                 tst        d0
                 bra        .2

fs_errorstr:     moveq      #0,d1
                 move.b     d0,d1
                 lsl.w      #2,d1
                 lea.l      fserrtable,a0
                 move.l     (a0,d1.w),a0
                 rts

fs_rdirlist:     tst.l      PARTSIZE
                 beq        .notmounted
                 move.w     MAXRDIRENTS,(a0)
                 move.l     RDIRSECTOR,4(a0)
                 moveq      #0,d0
                 rts
.notmounted:     moveq      #FSERR_NMOUNTED,d0
                 rts

fs_dirnext:      tst.w      (a0)
                 beq        .iterdone
                 moveq      #$0F,d0
                 and.w      (a0),d0
                 bne        .noload
                 push       a0
                 move.l     4(a0),d0
                 lea.l      8(a0),a0
                 moveq      #-1,d1
                 sys        ReadSector
                 pop        a0
                 tst.b      d0
                 bne        .done
                 addq.l     #1,4(a0)
.noload:         neg.b      d0
                 and.b      #$0F,d0
                 lsl.w      #5,d0
                 lea.l      8(a0,d0.w),a1
                 subq.w     #1,(a0)
                 beq        .iterdone
                 tst.b      FNAME(a1)
                 beq        fs_dirnext
                 cmp.b      #$E5,FNAME(a1)
                 beq        fs_dirnext
                 cmp.b      #$0F,FATTRS(a1)
                 beq        fs_dirnext
                 btst.b     #1,FATTRS(a1)
                 bne        fs_dirnext
                 btst.b     #3,FATTRS(a1)
                 bne        fs_dirnext
                 move.w     FCLUSTER(a1),d0
                 ror.w      #8,d0
                 move.w     d0,FCLUSTER(a1)
                 move.l     $1c(a1),d0
                 ror.w      #8,d0
                 swap       d0
                 ror.w      #8,d0
                 move.l     d0,$1c(a1)
                 moveq      #0,d0
                 rts
.iterdone:       moveq      #-1,d0
.done:           rts

fs_findfile:     pushm      a2-a3
                 move.l     a1,a3
                 link       a6,#-(DIRBUFSIZE+12)
                 lea.l      DIRBUFSIZE(sp),a1
                 bsr        fname_encode
                 bne        .ret
                 move.l     a1,a2
                 move.l     sp,a0
                 bsr        fs_rdirlist
                 bne        .ret
.diriter:        move.l     sp,a0
                 bsr        fs_dirnext
                 bmi        .notfound
                 bne        .ret
                 move.l     a2,a0
                 moveq      #FNAMELEN-1,d0
.namecmp:        cmpm.b     (a0)+,(a1)+
                 bne        .diriter
                 dbra       d0,.namecmp
                 lea.l      -FNAMELEN(a1),a1
                 moveq      #DIRENTLEN-1,d0
.direntcmp:      move.b     (a1)+,(a3)+
                 dbra       d0,.direntcmp
                 lea.l      -DIRENTLEN(a3),a1
                 moveq      #0,d0
                 bra        .ret
.notfound:       moveq      #FSERR_NOTFOUND,d0
.ret:            unlk       a6
                 popm       a2-a3
                 rts

fs_loadcluster:  pushm      d2-d3
                 cmp.w      #$FFF7,d0
                 beq        .badsector
                 cmp.w      #$FFF0,d0
                 bcc        .invclstr
                 subq       #2,d0
                 bmi        .invclstr
                 move.w     CLUSTERSIZE,d2
                 mulu       d2,d0
                 add.l      DATASTART,d0
                 move.l     d0,d3
                 subq       #1,d2
.readloop:       sys        ReadSector
                 tst.b      d0
                 bne        .error
                 tst.l      d1
                 beq        .bufferfull
                 addq.l     #1,d3
                 move.l     d3,d0
                 dbra       d2,.readloop
.bufferfull:     moveq      #0,d0
.error:          popm       d2-d3
                 rts
.badsector:      moveq      #FSERR_BADSECTOR,d0
                 bra        .error
.invclstr:       moveq      #FSERR_INVCLSTR,d0
                 bra        .error

fs_loadfileat:   move.w     d0,-(sp)
                 bsr        fs_loadcluster
                 bne        .error
                 tst.l      d1
                 beq        .done
                 move.w     (sp)+,d0
                 bsr        fs_nextcluster
                 bmi        .error
                 cmp.w      #$FFF8,d0
                 bcs        fs_loadfileat
                 moveq      #0,d0
                 rts
.done:           moveq      #0,d0
.error:          addq       #2,sp
                 rts

fs_nextcluster:  pushm      d1-d2/a0
                 move.w     d0,d2
                 move.l     FATSECTOR,d1
                 moveq      #0,d0
                 move.w     d2,d0
                 lsr.w      #8,d0
                 add.l      d1,d0
                 link       a6,#-SECTORSIZE
                 move.l     sp,a0
                 moveq      #-1,d1
                 sys        ReadSector
                 tst        d0
                 bne        .error
                 moveq      #0,d0
                 move.b     d2,d0
                 add.w      d0,d0
                 move.w     (sp,d0.w),d0
                 ror.w      #8,d0
.ret:            unlk       a6
                 popm       d1-d2/a0
                 tst.l      d0
                 rts
.error:          bset.l     #31,d0
                 bra        .ret

fs_loadfile:     pushm      d0/a1
                 link       a6,#-DIRENTLEN
                 move.l     sp,a1
                 bsr        fs_findfile
                 bne        .finderror
                 btst.b     #4,FATTRS(a1)
                 bne        .isdir
                 move.w     FCLUSTER(a1),d0
                 move.l     FSIZE(a1),d1
                 beq        .emptyfile
                 unlk       a6
                 cmp.l      (sp),d1
                 bcc        .1
                 move.l     d1,(sp)
.1:              pop        d1
                 move.l     (sp),a0
                 bsr        fs_loadfileat
                 sub.l      (sp),a0
                 move.l     a0,d1
                 pop        a0
                 tst.b      d0
                 rts
.finderror:      unlk       a6
                 addq       #8,sp
                 rts
.isdir:          moveq      #FSERR_ISDIR,d0
                 bra        .finderror
.emptyfile:      unlk       a6
                 popm       d0/a1
                 moveq      #0,d0
                 rts

; Filename encoding/decoding
fname_encode:    pushm      a0-a1
                 moveq      #FNAMELEN-1,d0
                 moveq      #$20,d1
.1:              move.b     d1,(a1)+
                 dbra       d0,.1
                 lea.l      -FNAMELEN(a1),a1
                 tst.b      (a0)
                 beq        .invalid
                 moveq      #0,d1
.filename:       moveq      #7,d0
.fnameloop:      move.b     (a0)+,d1
                 beq        .done
                 cmp.b      #'.',d1
                 beq        .extension
                 move.b     (validchartable,pc,d1.w),d1
                 beq        .invalid
                 move.b     d1,(a1)+
                 dbra       d0,.fnameloop
                 tst.b      (a0)
                 beq        .done
                 cmp.b      #'.',(a0)+
                 bne        .invalid
.extension:      move.l     4(sp),a1
                 addq       #8,a1
                 moveq      #2,d0
.extloop:        move.b     (a0)+,d1
                 beq        .done
                 move.b     (validchartable,pc,d1.w),d1
                 beq        .invalid
                 move.b     d1,(a1)+
                 dbra       d0,.extloop
                 tst.b      (a0)
                 bne        .invalid
.done:           moveq      #0,d0
.ret:            popm       a0-a1
                 rts
.invalid:        moveq      #FSERR_INVNAME,d0
                 bra        .ret

validchartable:
                 dc.b       0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0
                 dc.b       0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0
                 dc.b       0,  '!',0,  '#','$','%','&',$27,'(',')',0,  0,  0,  '-',0,  0
                 dc.b       '0','1','2','3','4','5','6','7','8','9',0,  0,  0,  0,  0,  0
                 dc.b       '@','A','B','C','D','E','F','G','H','I','J','K','L','M','N','O'
                 dc.b       'P','Q','R','S','T','U','V','W','X','Y','Z',0,  0,  0,  '^','_'
                 dc.b       '`','A','B','C','D','E','F','G','H','I','J','K','L','M','N','O'
                 dc.b       'P','Q','R','S','T','U','V','W','X','Y','Z','{',0,  '}','~',0
                 dc.b       $80,$81,$82,$83,$84,$85,$86,$87,$88,$89,$8A,$8B,$8C,$8D,$8E,$8F
                 dc.b       $90,$91,$92,$93,$94,$95,$96,$97,$98,$99,$9A,$9B,$9C,$9D,$9E,$9F
                 dc.b       $A0,$A1,$A2,$A3,$A4,$A5,$A6,$A7,$A8,$A9,$AA,$AB,$AC,$AD,$AE,$AF
                 dc.b       $B0,$B1,$B2,$B3,$B4,$B5,$B6,$B7,$B8,$B9,$BA,$BB,$BC,$BD,$BE,$BF
                 dc.b       $C0,$C1,$C2,$C3,$C4,$C5,$C6,$C7,$C8,$C9,$CA,$CB,$CC,$CD,$CE,$CF
                 dc.b       $D0,$D1,$D2,$D3,$D4,$D5,$D6,$D7,$D8,$D9,$DA,$DB,$DC,$DD,$DE,$DF
                 dc.b       $E0,$E1,$E2,$E3,$E4,0,  $E6,$E7,$E8,$E9,$EA,$EB,$EC,$ED,$EE,$EF
                 dc.b       $F0,$F1,$F2,$F3,$F4,$F5,$F6,$F7,$F8,$F9,$FA,$FB,$FC,$FD,$FE,$FF

fname_decode:    pushm      a0-a1
                 addq       #8,a0
                 moveq      #$20,d1
                 moveq      #7,d0
.1:              cmp.b      -(a0),d1
                 bne        .2
                 dbra       d0,.1
                 bra        .extension
.2:              move.l     (sp),a0
.3:              move.b     (a0)+,(a1)+
                 dbra       d0,.3
.extension:      move.l     (sp),a0
                 lea.l      FNAMELEN(a0),a0
                 moveq      #$20,d1
                 moveq      #2,d0
.4:              cmp.b      -(a0),d1
                 bne        .5
                 dbra       d0,.4
                 bra        .done
.5:              move.b     #'.',(a1)+
                 move.l     (sp),a0
                 addq       #8,a0
.6:              move.b     (a0)+,(a1)+
                 dbra       d0,.6
.done:           move.b     #0,(a1)+
                 popm       a0-a1
                 rts

;===============================================================================
; SPI INTERFACE (for RTC)
;===============================================================================

spi_startxfer:   lea.l      UART+MCR,a1
                 bclr.b     #MCR_CLK,(a1)
                 bset.b     #MCR_nSS,(a1)
                 rts

spi_endxfer:     lea.l      UART+MCR,a1
                 bclr.b     #MCR_nSS,(a1)
                 rts

spi_shiftbyte:   push       d2
                 lea.l      UART+MCR,a1
                 move.b     (a1),d2
                 lsr.b      #1,d2
                 not.b      d0
                 moveq      #7,d1
.bitloop:        roxl.b     #1,d0
                 roxl.b     #1,d2
                 addq.b     #(1<<MCR_CLK),d2
                 move.b     d2,(a1)
                 subq.b     #(1<<MCR_CLK),d2
                 move.b     d2,(a1)
                 lsr.b      #1,d2
                 swap       d2
                 move.b     4(a1),d2
                 lsl.b      #1,d2
                 swap       d2
                 dbra       d1,.bitloop
                 roxl.b     #1,d0
                 not.b      d0
                 pop        d2
                 rts

printtime:       sys        GetDateTime
                 tst.l      d0
                 bmi        .timenotset
                 beq        .no_rtc
                 lea.l      fmt_date,a0
                 movem.l    d0-d1,-(sp)
                 sys        OutFmt
                 addq       #8,sp
                 rts
.no_rtc:         lea.l      str_noclock,a0
                 sys        OutStr
                 rts
.timenotset:     lea.l      str_timenotset,a0
                 sys        OutStr
                 rts

;===============================================================================
; SERIAL I/O
;===============================================================================

uart_outchar:    lea.l      UART,a1
                 tx_char    d0,a1
                 rts

uart_inchar:     lea.l      UART,a1
.1:              btst.b     #0,LSR(a1)
                 beq        .1
                 move.b     RHR(a1),d0
                 rts

;===============================================================================
; SERIAL LOADER (via UART break)
;===============================================================================

VEC_AUTOVEC1:    lea.l      UART,a1
                 btst.b     #4,LSR(a1)
                 bne        .1
                 rte
.1:              btst.b     #4,LSR(a1)
                 bne        .1
                 move.b     RHR(a1),d0
                 tx_wait    a1
                 move.b     #'U',THR(a1)
                 lea.l      APPMEMSTART,a0
                 moveq      #0,d1
.byteloop:       move.b     LSR(a1),d0
                 btst.l     #4,d0
                 bne        .done
                 btst.l     #0,d0
                 beq        .byteloop
                 move.b     RHR(a1),(a0)+
                 addq.b     #1,d1
                 bne        .byteloop
                 bchg.b     #MCR_LED,MCR(a1)
                 bra        .byteloop
.done:           btst.b     #4,LSR(a1)
                 bne        .done
                 move.b     RHR(a1),d0

launchapp:       bclr.b     #MCR_LED,MCR(a1)
                 moveq      #0,d0
                 move.l     d0,d1
                 move.l     d0,d2
                 move.l     d0,d3
                 move.l     d0,d4
                 move.l     d0,d5
                 move.l     d0,d6
                 move.l     d0,d7
                 move.l     d0,a0
                 move.l     d0,a1
                 move.l     d0,a2
                 move.l     d0,a3
                 move.l     d0,a4
                 move.l     d0,a5
                 move.l     d0,a6
                 move.l     #INITIAL_SP,sp
                 move.w     #$2000,sr
                 jmp        APPMEMSTART

;===============================================================================
; EXCEPTION HANDLERS
;===============================================================================

vecstub          macro
                 movem.l    d0-d7/a0-a7,-(sp)
                 lea.l      (.str\@,pc),a0
                 bra        crash
                 dc.w       $0A2A
.str\@:          asciz      \1
                 even
                 endm

g0stub           macro
                 movem.l    d0-d7/a0-a7,-(sp)
                 lea.l      (.str\@,pc),a0
                 bra        grp0_crash
                 dc.w       $0A2A
.str\@:          asciz      \1
                 even
                 endm

VEC_BUSFAULT:    g0stub     "BUS FAULT"
VEC_ADRERROR:    g0stub     "ADDRESS ERROR"
VEC_ILLINSTR:    vecstub    "ILLEGAL INSTRUCTION"
VEC_DIVBY0:      vecstub    "ZERO DIVIDE"
VEC_CHK:         vecstub    "CHK"
VEC_TRAPV:       vecstub    "TRAPV"
VEC_PRIVVIOL:    vecstub    "PRIVILEGE VIOLATION"
VEC_TRACE:       vecstub    "TRACE"
VEC_LINE1010:    vecstub    "LINE 1010 EMULATOR"
VEC_LINE1111:    vecstub    "LINE 1111 EMULATOR"
VEC_RESERVED:    vecstub    "RESERVED VECTOR"
VEC_UNINIVEC:    vecstub    "UNINITIALIZED INTERRUPT VECTOR"
VEC_SPURIOUS:    vecstub    "SPURIOUS INTERRUPT"
VEC_AUTOVEC2:    vecstub    "AUTOVECTOR 2"
VEC_AUTOVEC3:    vecstub    "AUTOVECTOR 3"
VEC_AUTOVEC4:    vecstub    "AUTOVECTOR 4"
VEC_AUTOVEC5:    vecstub    "AUTOVECTOR 5"
VEC_AUTOVEC6:    vecstub    "AUTOVECTOR 6"
VEC_AUTOVEC7:    vecstub    "AUTOVECTOR 7"
VEC_BREAKPT:     vecstub    "BREAKPOINT"

crash:           sys        OutStr
                 lea.l      fmt_regs,a0
                 moveq      #64,d7
                 bra        dumpregs

grp0_crash:      sys        OutStr
                 lea.l      fmt_grp0regs,a0
                 moveq      #72,d7

dumpregs:        sys        OutFmt
                 move.l     usp,a0
                 move.l     a0,-(sp)
                 lea.l      fmt_usp,a0
                 sys        OutFmt
                 addq       #4,sp
                 bra        prompt

prompt:          lea.l      debugprompt,a0
                 sys        OutStr
.waitchar:       btst.b     #0,UART+LSR
                 bne        .gotchar
                 led_tgl
                 spin       $8000
                 bra        .waitchar
.gotchar:        move.b     UART+RHR,d0
                 sys        OutChar
                 cmp.b      #'A',d0
                 beq        ready
                 cmp.b      #'a',d0
                 beq        ready
                 cmp.b      #'C',d0
                 beq        resume
                 cmp.b      #'c',d0
                 beq        resume
                 cmp.b      #'S',d0
                 beq        resume_traceon
                 cmp.b      #'s',d0
                 bne        prompt
resume_traceon:  add.l      d7,sp
                 or.w       #$8000,(sp)
                 rte
resume:          add.l      d7,sp
                 and.w      #$7FFF,(sp)
                 rte

hexdigits_uc:    dc.b       "0123456789ABCDEF"
hexdigits_lc:    dc.b       "0123456789abcdef"
debugprompt:     dc.b       "\n[A]BORT/[C]ONTINUE/[S]TEP? ",0

fmt_regs:        dc.b       "\nD0=",FMT_H32,"  D1=",FMT_H32,"  D2=",FMT_H32,"  D3=",FMT_H32,"\n"
                 dc.b       "D4=",FMT_H32,"  D5=",FMT_H32,"  D6=",FMT_H32,"  D7=",FMT_H32,"\n"
                 dc.b       "A0=",FMT_H32,"  A1=",FMT_H32,"  A2=",FMT_H32,"  A3=",FMT_H32,"\n"
                 dc.b       "A4=",FMT_H32,"  A5=",FMT_H32,"  A6=",FMT_H32,"  A7=",FMT_H32,"\n"
                 dc.b       "SR=",FMT_SRFLAGS,"  PC=",FMT_H32,"              ", 0

fmt_grp0regs:    dc.b       "\nD0=",FMT_H32,"  D1=",FMT_H32,"  D2=",FMT_H32,"  D3=",FMT_H32,"\n"
                 dc.b       "D4=",FMT_H32,"  D5=",FMT_H32,"  D6=",FMT_H32,"  D7=",FMT_H32,"\n"
                 dc.b       "A0=",FMT_H32,"  A1=",FMT_H32,"  A2=",FMT_H32,"  A3=",FMT_H32,"\n"
                 dc.b       "A4=",FMT_H32,"  A5=",FMT_H32,"  A6=",FMT_H32,"  A7=",FMT_H32,"\n"
                 dc.b       "FLAGS=",FMT_FAULTFLAGS,"  ADDR=",FMT_H32,"  IR=",FMT_H16,"\n"
                 dc.b       "SR=",FMT_SRFLAGS,"  PC=",FMT_H32,"              ", 0

fmt_usp:         dc.b       "USP=",FMT_H32,"\n",0
fmt_sysinfo:     dc.b       "\nRAM: ",FMT_U32," BYTES\nROM: ",FMT_U32," BYTES\nCPU: ",FMT_U32," HZ\n",0
fmt_date:        dc.b       "TIME IS ",FMT_DATE," ",FMT_TIME,"\n",0
str_noclock:     asciz      "NO REAL-TIME CLOCK DETECTED\n"
str_timenotset:  asciz      "DATE/TIME NOT SET\n"
                 even

;===============================================================================
; ERROR STRINGS
;===============================================================================

fserr_noerror:   asciz      "NO ERROR"
fserr_amnf:      asciz      "CARD ERROR: ADDRESS MARK NOT FOUND"
fserr_abrt:      asciz      "CARD ERROR: COMMAND ABORTED"
fserr_idnf:      asciz      "CARD ERROR: INVALID SECTOR ID"
fserr_unc:       asciz      "CARD ERROR: UNCORRECTABLE ERROR DETECTED"
fserr_bbk:       asciz      "CARD ERROR: BAD BLOCK DETECTED"
fserr_nocard:    asciz      "NO CARD DETECTED"
fserr_notfat16:  asciz      "INVALID CARD FORMAT (NOT FAT16)"
fserr_not512:    asciz      "INVALID CARD FORMAT (LARGE SECTORS NOT SUPPORTED)"
fserr_nmounted:  asciz      "CARD NOT MOUNTED"
fserr_notfound:  asciz      "FILE NOT FOUND"
fserr_invclstr:  asciz      "INVALID CLUSTER NUMBER"
fserr_badsectr:  asciz      "BAD SECTOR"
fserr_invname:   asciz      "INVALID FILENAME"
fserr_isdir:     asciz      "IS A DIRECTORY"
fserr_other:     asciz      "CARD ERROR: OTHER"

fserrtable:      dc.l       fserr_noerror
                 dc.l       fserr_amnf
                 dc.l       fserr_other
                 dc.l       fserr_other
                 dc.l       fserr_abrt
                 dcb.l      16-((*-fserrtable)/4),fserr_other
                 dc.l       fserr_idnf
                 dcb.l      FSERR_TIMEOUT-((*-fserrtable)/4)
                 dc.l       fserr_nocard
                 dc.l       fserr_notfat16
                 dc.l       fserr_not512
                 dc.l       fserr_nmounted
                 dc.l       fserr_notfound
                 dc.l       fserr_invclstr
                 dc.l       fserr_badsectr
                 dc.l       fserr_invname
                 dc.l       fserr_isdir
                 dcb.l      64-((*-fserrtable)/4),fserr_other
                 dc.l       fserr_unc
                 dcb.l      128-((*-fserrtable)/4),fserr_other
                 dc.l       fserr_bbk
                 dcb.l      256-((*-fserrtable)/4),fserr_other

;===============================================================================
; SHELL (COMMAND INTERPRETER)
;===============================================================================

loadaddr       equ APPMEMSTART
loadlen        equ RAMEND-loadaddr-256

startshell:      litstr     "\nTYPE ? [ENTER] FOR HELP."
                 sys        OutStr

shell:           litstr     "\n> "
                 sys        OutStr
                 lea.l      INPUTBUF,a0
                 moveq      #INPUTBUFLEN,d0
                 moveq      #$0a,d1
                 sys        PromptStr
                 tst.l      d0
                 beq        shell

        ; Help?
                 move.b     (a0)+,d0
                 cmp.b      #'?',d0
                 beq        help

        ; Internal command?
                 cmp.b      #'.',d0
                 bne        runfile

        ; Parse command
                 move.b     (a0)+,d0
.1:              cmp.b      #$20,(a0)+
                 beq        .1
                 subq       #1,a0
                 lea.l      commands,a1

                 cmp.b      (a1)+,d0
                 beq        debug
                 cmp.b      (a1)+,d0
                 beq        debug
                 cmp.b      (a1)+,d0
                 beq        hexdumpfile
                 cmp.b      (a1)+,d0
                 beq        hexdumpfile
                 cmp.b      (a1)+,d0
                 beq        cardinfo
                 cmp.b      (a1)+,d0
                 beq        cardinfo
                 cmp.b      (a1)+,d0
                 beq        listfiles
                 cmp.b      (a1)+,d0
                 beq        listfiles
                 cmp.b      (a1)+,d0
                 beq        printfile
                 cmp.b      (a1)+,d0
                 beq        printfile
                 cmp.b      (a1)+,d0
                 beq        time
                 cmp.b      (a1)+,d0
                 beq        time

runfile:         subq       #1,a0
                 lea.l      loadaddr,a1
                 move.l     #loadlen,d0
                 sys        ReadFile
                 tst.b      d0
                 bne        error
                 litstr     "\nRUNNING.\n"
                 sys        OutStr
                 bra        launchapp

printfile:       lea.l      fmt_printfile,a2
printfile_:      lea.l      loadaddr,a1
                 move.l     #loadlen,d0
                 sys        ReadFile
                 tst.b      d0
                 bne        error
                 move.l     d1,-(sp)
                 move.l     a0,-(sp)
                 move.l     a2,a0
                 sys        OutFmt
                 lea.l      8(sp),sp
                 bra        shell

hexdumpfile:     lea.l      fmt_hexdump,a2
                 bra        printfile_

error:           litstr     "\n",FMT_ERR
                 move.w     d0,-(sp)
                 sys        OutFmt
                 addq       #2,sp
                 bra        shell

debug:           brk
                 bra        shell

commands:        dc.b       "DdHhIiLlPpTt"

listfiles:       moveq      #$0a,d0
                 sys        OutChar
                 link       a6,#-DIRBUFSIZE
                 move.l     sp,a0
                 moveq      #-1,d0
.list:           sys        ListDirectory
                 tst.w      d0
                 bmi        .listdone
                 bne        .listerror
                 move.l     a0,a2
                 btst.b     #4,FATTRS(a1)
                 bne        .isdir
                 litstr     FMT_FNAME," - ",FMT_U32," BYTE(S)\n"
                 move.l     FSIZE(a1),-(sp)
                 move.l     a1,-(sp)
                 sys        OutFmt
                 addq       #8,sp
                 bra        .1
.isdir:          litstr     FMT_FNAME," - DIRECTORY\n"
                 move.l     a1,-(sp)
                 sys        OutFmt
                 addq       #4,sp
.1:              move.l     a2,a0
                 moveq      #0,d0
                 bra        .list
.listdone:       unlk       a6
                 bra        shell
.listerror:      unlk       a6
                 bra        error

time:            tst.b      (a0)
                 beq        showtime
                 movep.l    0(a0),d0
                 and.l      #$0F0F0F0F,d0
                 lsl.l      #4,d0
                 movep.l    1(a0),d1
                 and.l      #$0F0F0F0F,d1
                 or.l       d1,d0
                 movep.l    8(a0),d1
                 and.l      #$0F0F0F0F,d1
                 lsl.l      #4,d1
                 movep.l    9(a0),d2
                 and.l      #$0F0F0F0F,d2
                 or.l       d2,d1
                 sys        SetDateTime

showtime:        sys        GetDateTime
                 tst.l      d0
                 bmi        .timenotset
                 beq        .no_rtc
                 litstr     "\n",FMT_DATE," ",FMT_TIME
                 movem.l    d0-d1,-(sp)
                 sys        OutFmt
                 addq       #8,sp
                 bra        shell
.no_rtc:         litstr     "\nNO REAL-TIME CLOCK DETECTED"
                 sys        OutStr
                 bra        shell
.timenotset:     litstr     "\nTIME NOT SET"
                 sys        OutStr
                 bra        shell

help:            lea.l      helpstr,a0
                 sys        OutStr
                 bra        shell

cardinfo:        tst.l      PARTSIZE
                 beq        .nocard
                 link       a6,#0
                 pea        VOLNAME
                 move.w     MAXRDIRENTS,-(sp)
                 move.w     CLUSTERSIZE,-(sp)
                 move.w     FATSIZE,-(sp)
                 move.w     RSVDSECTORS,-(sp)
                 move.w     FATCOPIES,-(sp)
                 move.l     DATASTART,-(sp)
                 move.l     RDIRSECTOR,-(sp)
                 move.l     FATSECTOR,-(sp)
                 move.l     BPBSECTOR,-(sp)
                 move.l     PARTSIZE,-(sp)
                 lea.l      fmt_cardinfo,a0
                 sys        OutFmt
                 unlk       a6
                 bra        shell
.nocard:         litstr     "\nNO CARD INSERTED OR NO VALID FILESYSTEM ON CARD"
                 sys        OutStr
                 bra        shell

fmt_hexdump:     dc.b       "\n",FMT_HEXDUMP,"\n",0
fmt_printfile:   dc.b       "\n",FMT_BUF,0

helpstr:         dc.b       "\nCOMMANDS:\n"
                 dc.b       "?          PRINT THIS HELP MESSAGE\n"
                 dc.b       "<FILE>     RUN <FILE>\n"
                 dc.b       ".L         LIST FILES\n"
                 dc.b       ".I         PRINT CARD INFO\n"
                 dc.b       ".P <FILE>  PRINT CONTENTS OF <FILE>\n"
                 dc.b       ".H <FILE>  HEXDUMP CONTENTS OF <FILE>\n"
                 dc.b       ".T         PRINT DATE AND TIME\n"
                 dc.b       ".T <DATE>  SET DATE AND TIME\n"
                 dc.b       "             (<DATE> FORMAT IS YYYYMMDDWWhhmmss)\n"
                 dc.b       ".D         ENTER DEBUGGER\n"
                 dc.b       0

fmt_cardinfo:    dc.b       "\nPARTITION SIZE:                 ",FMT_U32," SECTORS\n"
                 dc.b       "BIOS PARAMETER BLOCK AT SECTOR  ",FMT_U32,"\n"
                 dc.b       "FILE ALLOCATION TABLE AT SECTOR ",FMT_U32,"\n"
                 dc.b       "ROOT DIRECTORY AT SECTOR        ",FMT_U32,"\n"
                 dc.b       "START OF DATA REGION AT SECTOR  ",FMT_U32,"\n"
                 dc.b       "COPIES OF FAT:                  ",FMT_U16,"\n"
                 dc.b       "RESERVED SECTORS:               ",FMT_U16,"\n"
                 dc.b       "FILE ALLOCATION TABLE SIZE:     ",FMT_U16," SECTORS\n"
                 dc.b       "CLUSTER SIZE:                   ",FMT_U16," SECTORS\n"
                 dc.b       "MAX ROOT DIRECTORY ENTRIES:     ",FMT_U16,"\n"
                 dc.b       "VOLUME NAME:                    '",FMT_S,"'\n"
                 dc.b       0
                 even
                 bne        error
                 move.l     d1,-(sp)
                 move.l     a0,-(sp)
                 move.l     a2,a0
                 sys        OutFmt
                 lea.l      8(sp),sp
                 bra        shell

hexdumpfile:     lea.l      fmt_hexdump,a2
                 bra        printfile_

error:           litstr     "\n",FMT_ERR
                 move.w     d0,-(sp)
                 sys        OutFmt
                 addq       #2,sp
                 bra        shell

debug:           brk
                 bra        shell

commands:        dc.b       "DdHhIiLlPpTt"

listfiles:       moveq      #$0a,d0
                 sys        OutChar
                 link       a6,#-DIRBUFSIZE
                 move.l     sp,a0
                 moveq      #-1,d0
.list:           sys        ListDirectory
                 tst.w      d0
                 bmi        .listdone
                 bne        .listerror
                 move.l     a0,a2
                 btst.b     #4,FATTRS(a1)
                 bne        .isdir
                 litstr     FMT_FNAME," - ",FMT_U32," BYTE(S)\n"
                 move.l     FSIZE(a1),-(sp)
                 move.l     a1,-(sp)
                 sys        OutFmt
                 addq       #8,sp
                 bra        .1
.isdir:          litstr     FMT_FNAME," - DIRECTORY\n"
                 move.l     a1,-(sp)
                 sys        OutFmt
                 addq       #4,sp
.1:              move.l     a2,a0
                 moveq      #0,d0
                 bra        .list
.listdone:       unlk       a6
                 bra        shell
.listerror:      unlk       a6
                 bra        error

time:            tst.b      (a0)
                 beq        showtime
                 movep.l    0(a0),d0
                 and.l      #$0F0F0F0F,d0
                 lsl.l      #4,d0
                 movep.l    1(a0),d1
                 and.l      #$0F0F0F0F,d1
                 or.l       d1,d0
                 movep.l    8(a0),d1
                 and.l      #$0F0F0F0F,d1
                 lsl.l      #4,d1
                 movep.l    9(a0),d2
                 and.l      #$0F0F0F0F,d2
                 or.l       d2,d1
                 sys        SetDateTime

showtime:        sys        GetDateTime
                 tst.l      d0
                 bmi        .timenotset
                 beq        .no_rtc
                 litstr     "\n",FMT_DATE," ",FMT_TIME
                 movem.l    d0-d1,-(sp)
                 sys        OutFmt
                 addq       #8,sp
                 bra        shell
.no_rtc:         litstr     "\nNO REAL-TIME CLOCK DETECTED"
                 sys        OutStr
                 bra        shell
.timenotset:     litstr     "\nTIME NOT SET"
                 sys        OutStr
                 bra        shell

help:            lea.l      helpstr,a0
                 sys        OutStr
                 bra        shell

cardinfo:        tst.l      PARTSIZE
                 beq        .nocard
                 link       a6,#0
                 pea        VOLNAME
                 move.w     MAXRDIRENTS,-(sp)
                 move.w     CLUSTERSIZE,-(sp)
                 move.w     FATSIZE,-(sp)
                 move.w     RSVDSECTORS,-(sp)
                 move.w     FATCOPIES,-(sp)
                 move.l     DATASTART,-(sp)
                 move.l     RDIRSECTOR,-(sp)
                 move.l     FATSECTOR,-(sp)
                 move.l     BPBSECTOR,-(sp)
                 move.l     PARTSIZE,-(sp)
                 lea.l      fmt_cardinfo,a0
                 sys        OutFmt
                 unlk       a6
                 bra        shell
.nocard:         litstr     "\nNO CARD INSERTED OR NO VALID FILESYSTEM ON CARD"
                 sys        OutStr
                 bra        shell

fmt_hexdump:     dc.b       "\n",FMT_HEXDUMP,"\n",0
fmt_printfile:   dc.b       "\n",FMT_BUF,0

helpstr:         dc.b       "\nFLUX32 SHELL COMMANDS:\n"
                 dc.b       "?          PRINT THIS HELP MESSAGE\n"
                 dc.b       "<FILE>     RUN <FILE>\n"
                 dc.b       ".L         LIST FILES\n"
                 dc.b       ".I         PRINT CARD INFO\n"
                 dc.b       ".P <FILE>  PRINT CONTENTS OF <FILE>\n"
                 dc.b       ".H <FILE>  HEXDUMP CONTENTS OF <FILE>\n"
                 dc.b       ".T         PRINT DATE AND TIME\n"
                 dc.b       ".T <DATE>  SET DATE AND TIME\n"
                 dc.b       "             (<DATE> FORMAT IS YYYYMMDDWWhhmmss)\n"
                 dc.b       0

fmt_cardinfo:    dc.b       "\nPARTITION SIZE:                 ",FMT_U32," SECTORS\n"
                 dc.b       "BIOS PARAMETER BLOCK AT SECTOR  ",FMT_U32,"\n"
                 dc.b       "FILE ALLOCATION TABLE AT SECTOR ",FMT_U32,"\n"
                 dc.b       "ROOT DIRECTORY AT SECTOR        ",FMT_U32,"\n"
                 dc.b       "START OF DATA REGION AT SECTOR  ",FMT_U32,"\n"
                 dc.b       "COPIES OF FAT:                  ",FMT_U16,"\n"
                 dc.b       "RESERVED SECTORS:               ",FMT_U16,"\n"
                 dc.b       "FILE ALLOCATION TABLE SIZE:     ",FMT_U16," SECTORS\n"
                 dc.b       "CLUSTER SIZE:                   ",FMT_U16," SECTORS\n"
                 dc.b       "MAX ROOT DIRECTORY ENTRIES:     ",FMT_U16,"\n"
                 dc.b       "VOLUME NAME:                    '",FMT_S,"'\n"
                 dc.b       0
                 even

;===============================================================================
; ROM FOOTER
;===============================================================================

                 printt     "Total ROM size:"
                 printv     *
                 dcb.b      ROMSIZE-20-*,$FF

sysinfo:
clockspeed:      dc.l       F_CPU
ramsize:         dc.l       RAMSIZE
romsize:         dc.l       ROMSIZE
rom_date:        dc.w       ROM_DATE_YEAR
                 dc.b       ROM_DATE_MONTH
                 dc.b       ROM_DATE_DAY
rom_version:     dc.w       ROM_VER_MAJ
                 dc.w       ROM_VER_MIN
