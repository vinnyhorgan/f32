; vim:noet:sw=8:ts=8:sts=8:ai:syn=asm68k
;
; CompactFlash Card Register Definitions
; IDE/ATA in True IDE mode for Flux32 SBC

;-------------------------------------------------------------------------------
; Task file register offsets (odd addresses for 8-bit mode)
;-------------------------------------------------------------------------------
CF_DATA          equ        0                                                   ; Data register (16-bit read/write)
CF_ERROR         equ        3                                                   ; Error register (8-bit read)
CF_FEATURE       equ        3                                                   ; Feature register (8-bit write)
CF_COUNT         equ        5                                                   ; Sector count register
CF_SECTOR        equ        7                                                   ; Sector number register
CF_LBA0          equ        7                                                   ; LBA bits 7-0
CF_CYLL          equ        9                                                   ; Cylinder low register
CF_LBA1          equ        9                                                   ; LBA bits 15-8
CF_CYLH          equ        11                                                  ; Cylinder high register
CF_LBA2          equ        11                                                  ; LBA bits 23-16
CF_DRIVEHEAD     equ        13                                                  ; Drive select/head register
CF_LBA3          equ        13                                                  ; LBA bits 27-24
CF_STATUS        equ        15                                                  ; Status register (8-bit read)
CF_COMMAND       equ        15                                                  ; Command register (8-bit write)

; Word-aligned offsets for word access
CF_ERROR_W       equ        CF_ERROR-1
CF_FEATURE_W     equ        CF_FEATURE-1
CF_COUNT_W       equ        CF_COUNT-1
CF_SECTOR_W      equ        CF_SECTOR-1
CF_LBA0_W        equ        CF_LBA0-1
CF_CYLL_W        equ        CF_CYLL-1
CF_LBA1_W        equ        CF_LBA1-1
CF_CYLH_W        equ        CF_CYLH-1
CF_LBA2_W        equ        CF_LBA2-1
CF_DRIVEHEAD_W   equ        CF_DRIVEHEAD-1
CF_LBA3_W        equ        CF_LBA3-1
CF_STATUS_W      equ        CF_STATUS-1
CF_COMMAND_W     equ        CF_COMMAND-1

;-------------------------------------------------------------------------------
; ATA commands
;-------------------------------------------------------------------------------
CFCMD_IDENTIFY   equ        $EC
CFCMD_RDSECTOR   equ        $20

;-------------------------------------------------------------------------------
; Error codes
;-------------------------------------------------------------------------------
; Bit 5 clear = card-reported error
; Bit 5 set = filesystem error
FSERR_TIMEOUT    equ        32                                                  ; No card detected
FSERR_WRONGTYPE  equ        33                                                  ; Not FAT16
FSERR_BPS        equ        34                                                  ; Bytes per sector not 512
FSERR_NMOUNTED   equ        35                                                  ; Filesystem not mounted
FSERR_NOTFOUND   equ        36                                                  ; File not found
FSERR_INVCLSTR   equ        37                                                  ; Invalid cluster number
FSERR_BADSECTOR  equ        38                                                  ; Bad sector
FSERR_INVNAME    equ        39                                                  ; Invalid filename
FSERR_ISDIR      equ        40                                                  ; Entry is a directory

;-------------------------------------------------------------------------------
; Constants
;-------------------------------------------------------------------------------
SECTORSIZE       equ        512                                                 ; Sector size (hardcoded)
DIRBUFSIZE       equ        SECTORSIZE+8                                        ; Directory listing buffer size
DIRENTLEN        equ        32                                                  ; FAT16 directory entry size

; FAT16 directory entry field offsets
FNAME            equ        0
FNAMELEN         equ        11
FATTRS           equ        11
FCLUSTER         equ        26
FSIZE            equ        28
