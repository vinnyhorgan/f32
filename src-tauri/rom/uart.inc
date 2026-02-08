; vim:noet:sw=8:ts=8:sts=8:ai:syn=asm68k ; ; UART 16550 Register Definitions ;
For Flux32 SBC emulation
;-------------------------------------------------------------------------------
; Register offsets (word-aligned for M68K)
;-------------------------------------------------------------------------------
RHR equ 0 ; Receive holding register (read) THR equ 0 ; Transmit holding
register (write) IER equ 2 ; Interrupt enable register ISR equ 4 ; Interrupt
status register (read) FCR equ 4 ; FIFO control register (write) LCR equ 6 ;
Line control register MCR equ 8 ; Modem control register LSR equ 10 ; Line
status register MSR equ 12 ; Modem status register SPR equ 14 ; Scratchpad
register (system use) DLL equ 0 ; Divisor latch LSB (DLAB=1) DLM equ 2 ; Divisor
latch MSB (DLAB=1) ; Aliases for register names (different manufacturers) RBR
equ RHR ; Receive buffer register IIR equ ISR ; Interrupt identification
register SCR equ SPR ; Scratch register
;-------------------------------------------------------------------------------
; MCR register bits (active-low outputs directly connect to GPIO)
;-------------------------------------------------------------------------------
MCR_COPI equ 0 ; ~DTR: SPI COPI (bit 0 for easier shifting) MCR_LED equ 1 ;
~RTS: Status LED MCR_CLK equ 2 ; ~OUT1: SPI clock MCR_nSS equ 3 ; ~OUT2: SPI
chip select
;-------------------------------------------------------------------------------
; MSR register bits (active-low inputs from GPIO)
;-------------------------------------------------------------------------------
MSR_BTN1 equ 6 ; ~RI: Button input MSR_BTN1_PRESS equ 2 ; TERI: Button press
edge MSR_SQW equ 5 ; ~DSR: RTC square wave MSR_SQW_CHG equ 2 ; deltaDSR MSR_CIPO
equ 7 ; ~DCD: SPI CIPO (bit 7 for easier shifting)
;-------------------------------------------------------------------------------
; Convenience macros
;-------------------------------------------------------------------------------
; Wait for UART to be ready to transmit ; Argument 1: register containing UART
base address tx_wait macro .t\@!: btst.b #5,LSR(\1) beq .t\@@ endm ; Transmit a
character on the serial port ; Argument 1: effective address of value to
transmit ; Argument 2: register containing UART base address tx_char macro
tx_wait \2 move.b \1,THR(\2) endm ; LED control macros led_on macro bset.b
#MCR_LED,UART+MCR endm led_off macro bclr.b #MCR_LED,UART+MCR endm led_tgl macro
bchg.b #MCR_LED,UART+MCR endm
