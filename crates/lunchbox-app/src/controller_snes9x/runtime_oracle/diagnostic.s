; Original Lunchbox SNES diagnostic. The ROM reads the console's automatic
; joypad registers rather than frontend memory or generated configuration.
;
; SRAM layout:
;   $700000..1  "LB" signature
;   $700002     boot count (proves fresh-process SRAM reload)
;   $700003     $5a after SNES B was observed (state mutation marker)
;   $700004..5  running NMI count
;   $700008..b  current P1/P2 hardware words
;   $700010..3  cumulative P1/P2 hardware words
;   $700020..3  P1/P2 bits observed as isolated one-button states
;   $700024..7  previous P1/P2 hardware words
;   $700030..3  P1/P2 ordered-log entry counts
;   $700040..57 P1 ordered one-button hardware words (maximum 12)
;   $700060..77 P2 ordered one-button hardware words (maximum 12)
.p816
.a8
.i16

.segment "CODE"

reset:
    sei
    clc
    xce
    rep #$30
    ldx #$1fff
    txs
    sep #$20
    lda #$00
    sta $4200

    lda $700000
    cmp #'L'
    bne initialize
    lda $700001
    cmp #'B'
    bne initialize
    lda $700002
    inc a
    sta $700002
    bra start

initialize:
    ldx #$007f
    lda #$00
clear_sram:
    sta $700000,x
    dex
    bpl clear_sram
    lda #'L'
    sta $700000
    lda #'B'
    sta $700001
    lda #$01
    sta $700002

start:
    lda #$81
    sta $4200
    cli
loop:
    wai
    bra loop

nmi:
    php
    pha
    phx
    phy
    sep #$20
.a8
    lda $4210
    rep #$20
.a16
    lda $4218
    sta $700008
    ora $700010
    sta $700010
    lda $700008
    beq p1_not_single
    dec a
    and $700008
    bne p1_not_single
    lda $700008
    ora $700020
    sta $700020
    lda $700024
    bne p1_not_single
    lda $700030
    cmp #$000c
    bcs p1_not_single
    asl a
    tax
    lda $700008
    sta $700040,x
    lda $700030
    inc a
    sta $700030
p1_not_single:
    lda $700008
    sta $700024
    lda $421a
    sta $70000a
    ora $700012
    sta $700012
    lda $70000a
    beq p2_not_single
    dec a
    and $70000a
    bne p2_not_single
    lda $70000a
    ora $700022
    sta $700022
    lda $700026
    bne p2_not_single
    lda $700032
    cmp #$000c
    bcs p2_not_single
    asl a
    tax
    lda $70000a
    sta $700060,x
    lda $700032
    inc a
    sta $700032
p2_not_single:
    lda $70000a
    sta $700026
    lda $700004
    inc a
    sta $700004
    lda $4218
    bit #$8000
    beq no_mutation
    sep #$20
.a8
    lda #$5a
    sta $700003
no_mutation:
    sep #$20
.a8
    ply
    plx
    pla
    plp
    rti

irq:
    rti

.segment "HEADER"
    .byte "LUNCHBOX SNES9X TEST "
    .byte $20
    .byte $02
    .byte $05
    .byte $05
    .byte $01
    .byte $00
    .byte $00
    .word $75ee
    .word $8a11

.segment "VECTORS"
    .word $0000
    .word $0000
    .word $0000
    .word $0000
    .word $0000
    .word nmi
    .word $0000
    .word irq
    .word $0000
    .word $0000
    .word $0000
    .word $0000
    .word $0000
    .word nmi
    .word reset
    .word irq
