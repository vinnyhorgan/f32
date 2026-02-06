; vim:noet:sw=8:ts=8:sts=8:ai:syn=asm68k
;
; FizzBuzz for Flux32
; Classic programming challenge demonstrating DIVU and formatted output

        include    "../app.inc"

start:  sys        WaitBtn

        moveq      #1,d3           ; Counter
.loop:
        moveq      #0,d2           ; Flag
        ; Compute modulo 3
        move.l     d3,d0
        divu.w     #3,d0
        swap       d0              ; Get remainder
        tst.w      d0
        bne        .1              ; Don't print "Fizz" if remainder nonzero
        litstr     "Fizz"
        sys        OutStr
        addq.w     #1,d2           ; Set flag
.1:     ; Compute modulo 5
        move.l     d3,d0
        divu.w     #5,d0
        swap       d0              ; Get remainder
        tst.w      d0
        bne        .2              ; Don't print "Buzz" if remainder nonzero
        litstr     "Buzz"
        sys        OutStr
        addq       #1,d2           ; Set flag
.2:     ; If flag not set, print value as decimal
        tst.w      d2
        bne        .3
        move.w     d3,-(sp)
        litstr     FMT_U16,0
        sys        OutFmt
        addq       #2,sp
.3:     moveq      #$0a,d0         ; Print newline
        sys        OutChar
        addq       #1,d3
        led_tgl
        move.l     #$40000,d0
        bsr        delay
        bra        .loop


; Delay by number of loop iterations in D0 (32-bit)
delay:  subq.l     #1,d0
        bne        delay
        rts
