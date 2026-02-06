; vim:noet:sw=8:ts=8:sts=8:ai:syn=asm68k
;
; Hello World Example for Flux32
; Demonstrates basic UART output using syscalls

        include    "../app.inc"

start:  sys        WaitBtn                     ; Wait for button press
        led_tgl                                ; Toggle LED
        lea.l      str,a0                      ; Load string address
        sys        OutStr                      ; Print string
        bra        start                       ; Loop forever

str:    dc.b       "Hello from Flux32!\n",0
