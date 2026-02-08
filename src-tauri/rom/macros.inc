; vim:noet:sw=8:ts=8:sts=8:ai:syn=asm68k
;
; Flux32 Convenience Macros
; Common patterns and helper macros for M68K assembly

;-------------------------------------------------------------------------------
; Breakpoint (68000 lacks BKPT instruction)
;-------------------------------------------------------------------------------
brk              macro
                 trap       #15
                 endm

;-------------------------------------------------------------------------------
; Flag manipulation
;-------------------------------------------------------------------------------
sev              macro                                                          ; Set overflow flag
                 or         #%00000010,ccr
                 endm

clv              macro                                                          ; Clear overflow flag
                 and        #%11111101,ccr
                 endm

;-------------------------------------------------------------------------------
; Stack operations
;-------------------------------------------------------------------------------

; Save multiple registers to stack
                 pushm      macro
                 if         NARG>1
                 fail       multiple args passed to pushm--did you use a comma instead of a slash?
                 endif
                 movem.l    \1,-(sp)
                 endm

; Save single register to stack
push             macro
                 if         NARG>1
                 fail       multiple args passed to push--use pushm for multiple registers
                 endif
                 move.l     \1,-(sp)
                 endm

; Restore multiple registers from stack
                 popm       macro
                 if         NARG>1
                 fail       multiple args passed to popm--did you use a comma instead of a slash?
                 endif
                 movem.l    (sp)+,\1
                 endm

; Restore single register from stack
pop              macro
                 if         NARG>1
                 fail       multiple args passed to pop--use popm for multiple registers
                 endif
                 move.l     (sp)+,\1
                 endm

;-------------------------------------------------------------------------------
; Subroutine calls with link register (ARM-style optimization)
;-------------------------------------------------------------------------------

; Branch to subroutine and link, storing return address in A6
; Useful for calling leaf functions without stack overhead
                 bl         macro
                 lea        (.ret\@,pc),a6
                 bra        \1
.ret\@:
                 endm

; Return from subroutine to address in A6
                 rl         macro
                 jmp        (a6)
                 endm

;-------------------------------------------------------------------------------
; Delay loop
;-------------------------------------------------------------------------------

; Spin for a given number of iterations (0-65535)
; Clobbers D0
spin             macro
                 move.l     #\1,d0
.spin\@:         dbra       d0,.spin\@
                 endm

;-------------------------------------------------------------------------------
; String declarations
;-------------------------------------------------------------------------------

; Declare null-terminated string
                 asciz      macro
                 rept       \#
                 dc.b       \+
                 endr
                 dc.b       0
                 even
                 endm

; Declare null-terminated string inline, place its address in A0, and skip over it
                 litstr     macro
                 lea.l      (.str\@,pc),a0
                 bra        .next\@
.str\@:
                 rept       \#
                 dc.b       \+
                 endr
                 dc.b       0
                 even
.next\@:
                 endm

;-------------------------------------------------------------------------------
; Status register bit offsets
;-------------------------------------------------------------------------------
SR_C             equ        0                                                   ; Carry flag
SR_V             equ        1                                                   ; Overflow flag
SR_Z             equ        2                                                   ; Zero flag
SR_N             equ        3                                                   ; Negative flag
SR_X             equ        4                                                   ; Extend flag
SR_I             equ        8                                                   ; Interrupt mask (3 bits)
SR_S             equ        13                                                  ; Supervisor mode
SR_T             equ        15                                                  ; Trace flag

CCR_C            equ        SR_C
CCR_V            equ        SR_V
CCR_Z            equ        SR_Z
CCR_N            equ        SR_N
CCR_X            equ        SR_X

;-------------------------------------------------------------------------------
; Debug helper: fill registers with identifiable patterns
;-------------------------------------------------------------------------------
LOUD_NOISES      macro
                 move.l     #$d0d0dd00,d0
                 move.l     #$d1d1dd11,d1
                 move.l     #$d2d2dd22,d2
                 move.l     #$d3d3dd33,d3
                 move.l     #$d4d4dd44,d4
                 move.l     #$d5d5dd55,d5
                 move.l     #$d6d6dd66,d6
                 move.l     #$d7d7dd77,d7
                 move.l     #$a0a0aa00,a0
                 move.l     #$a1a1aa11,a1
                 move.l     #$a2a2aa22,a2
                 move.l     #$a3a3aa33,a3
                 move.l     #$a4a4aa44,a4
                 move.l     #$a5a5aa55,a5
                 move.l     #$a6a6aa66,a6
                 endm
