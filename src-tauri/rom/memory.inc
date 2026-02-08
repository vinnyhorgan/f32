; vim:noet:sw=8:ts=8:sts=8:ai:syn=asm68k
;
; Flux32 Memory Map
; Defines memory regions, sizes, and system variables.

ROMSIZE          equ        $010000                                             ; 64KB ROM
RAMSIZE          equ        $100000                                             ; 1MB RAM
RAMEND           equ        RAM+RAMSIZE
INITIAL_SP       equ        RAMEND

;-------------------------------------------------------------------------------
; Base addresses (matching SBC hardware decode)
;-------------------------------------------------------------------------------
ROM              equ        $000000                                             ; ROM at $000000-$0FFFFF (64KB repeated 16x)
RAM              equ        $E00000                                             ; RAM at $E00000-$EFFFFF (mirrored from $C00000)
CFCARD           equ        $900000                                             ; CompactFlash at $900000-$9FFFFF
UART             equ        $A00000                                             ; UART 16550 at $A00000-$AFFFFF

;-------------------------------------------------------------------------------
; Memory reserved for the system (first 256 bytes of RAM)
;-------------------------------------------------------------------------------
SYSMEMSTART      equ        RAM
SYSMEMSIZE       equ        256
                 rsset      SYSMEMSTART
OUTCH_VEC        rs.l       1                                                   ; Output character vector
INCH_VEC         rs.l       1                                                   ; Input character vector
HEXDIGITS        rs.l       1                                                   ; Pointer to hex digit table
SEPARATORS       rs.l       1                                                   ; Date/time/thousands separators
DATE_SEP         equ        SEPARATORS
TIME_SEP         equ        SEPARATORS+1
THOUSANDS_SEP    equ        SEPARATORS+2

; Mounted filesystem parameters
PARTSIZE         rs.l       1                                                   ; Size of partition in sectors (0 = not mounted)
BPBSECTOR        rs.l       1                                                   ; Sector number of BIOS Parameter Block
; These are in reverse order of BPB for byte-swapping
FATSIZE          rs.w       1                                                   ; Size of FAT in sectors
MAXRDIRENTS      rs.w       1                                                   ; Maximum root directory entries
FATCOPIES        rs.w       1                                                   ; Number of FATs (padded to 2 bytes)
RSVDSECTORS      rs.w       1                                                   ; Number of reserved sectors
CLUSTERSIZE      rs.w       1                                                   ; Sectors per cluster
RDIRSECTOR       rs.l       1                                                   ; Sector number of root directory
FATSECTOR        rs.l       1                                                   ; Sector number of FAT #1
DATASTART        rs.l       1                                                   ; Sector number of data region start
VOLNAME          rs.b       12                                                  ; Volume label (11 bytes + padding)
FSVARSTART       equ        PARTSIZE
FSVARLEN         equ        VOLNAME-FSVARSTART+12

INPUTBUFLEN      equ        64
INPUTBUF         rs.b       INPUTBUFLEN+2                                       ; Command input buffer

;-------------------------------------------------------------------------------
; Application load address
;-------------------------------------------------------------------------------
APPMEMSTART      equ        SYSMEMSTART+SYSMEMSIZE                              ; $E00100

