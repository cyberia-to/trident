// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Pure ARM64 instruction encoders.
//!
//! Every function returns a u32 little-endian machine word (or Vec<u32>).
//! No state, no allocations. Used by both the nox JIT backend (arm64.rs)
//! and the LIR register-machine backend (arm64_lir.rs).
//!
//! Register numbers follow ARM64 convention: 0-30 are X0-X30, 31 is XZR/SP.

// ── Arithmetic ────────────────────────────────────────────────────

/// ADD Xd, Xn, Xm (64-bit, no flags)
#[inline] pub fn add(rd: u8, rn: u8, rm: u8) -> u32 {
    0x8B000000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// ADDS Xd, Xn, Xm — sets NZCV flags
#[inline] pub fn adds(rd: u8, rn: u8, rm: u8) -> u32 {
    0xAB000000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// SUB Xd, Xn, Xm (64-bit, no flags)
#[inline] pub fn sub(rd: u8, rn: u8, rm: u8) -> u32 {
    0xCB000000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// SUBS Xd, Xn, Xm — sets NZCV flags
#[inline] pub fn subs(rd: u8, rn: u8, rm: u8) -> u32 {
    0xEB000000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// MUL Xd, Xn, Xm = MADD Xd, Xn, Xm, XZR (low 64 bits of 64×64)
#[inline] pub fn mul(rd: u8, rn: u8, rm: u8) -> u32 {
    0x9B007C00 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// UMULH Xd, Xn, Xm — upper 64 bits of unsigned 64×64 product
#[inline] pub fn umulh(rd: u8, rn: u8, rm: u8) -> u32 {
    0x9BC07C00 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// UDIV Xd, Xn, Xm — unsigned divide
#[inline] pub fn udiv(rd: u8, rn: u8, rm: u8) -> u32 {
    0x9AC00800 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// SDIV Xd, Xn, Xm — signed divide
#[inline] pub fn sdiv(rd: u8, rn: u8, rm: u8) -> u32 {
    0x9AC00C00 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// MSUB Xd, Xn, Xm, Xa — Xd = Xa - Xn*Xm (remainder: Xa - quotient*divisor)
#[inline] pub fn msub(rd: u8, rn: u8, rm: u8, ra: u8) -> u32 {
    0x9B008000 | ((rm as u32) << 16) | ((ra as u32) << 10) | ((rn as u32) << 5) | (rd as u32)
}

/// CMP Xn, Xm = SUBS XZR, Xn, Xm
#[inline] pub fn cmp(rn: u8, rm: u8) -> u32 {
    subs(31, rn, rm)
}

/// NEG Xd, Xm = SUB Xd, XZR, Xm
#[inline] pub fn neg(rd: u8, rm: u8) -> u32 {
    sub(rd, 31, rm)
}

/// ADD Xd, Xn, #imm12 (no shift; imm12 must be 0..4095)
#[inline] pub fn add_imm(rd: u8, rn: u8, imm12: u16) -> u32 {
    debug_assert!(imm12 < 4096, "add_imm: imm12 out of range");
    0x91000000 | ((imm12 as u32) << 10) | ((rn as u32) << 5) | (rd as u32)
}

/// SUB Xd, Xn, #imm12 (no shift; imm12 must be 0..4095)
#[inline] pub fn sub_imm(rd: u8, rn: u8, imm12: u16) -> u32 {
    debug_assert!(imm12 < 4096, "sub_imm: imm12 out of range");
    0xD1000000 | ((imm12 as u32) << 10) | ((rn as u32) << 5) | (rd as u32)
}

// ── Bitwise ───────────────────────────────────────────────────────

/// AND Xd, Xn, Xm
#[inline] pub fn and(rd: u8, rn: u8, rm: u8) -> u32 {
    0x8A000000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// ORR Xd, Xn, Xm
#[inline] pub fn orr(rd: u8, rn: u8, rm: u8) -> u32 {
    0xAA000000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// EOR Xd, Xn, Xm
#[inline] pub fn eor(rd: u8, rn: u8, rm: u8) -> u32 {
    0xCA000000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// MVN Xd, Xm = ORN Xd, XZR, Xm (bitwise NOT)
#[inline] pub fn mvn(rd: u8, rm: u8) -> u32 {
    0xAA2003E0 | ((rm as u32) << 16) | (rd as u32)
}

/// CLZ Xd, Xn — count leading zeros
#[inline] pub fn clz(rd: u8, rn: u8) -> u32 {
    0xDAC01000 | ((rn as u32) << 5) | (rd as u32)
}

// ── Shifts ────────────────────────────────────────────────────────

/// LSL Xd, Xn, #shift — logical shift left by immediate (shift: 0..63)
#[inline] pub fn lsl_imm(rd: u8, rn: u8, shift: u8) -> u32 {
    debug_assert!(shift < 64, "lsl_imm: shift out of range");
    // UBFM Xd, Xn, #(-shift mod 64), #(63-shift)
    let immr = (64 - shift as u32) & 63;
    let imms = 63 - shift as u32;
    0xD3400000 | (immr << 16) | (imms << 10) | ((rn as u32) << 5) | (rd as u32)
}

/// LSL Xd, Xn, Xm = LSLV Xd, Xn, Xm (shift by register)
#[inline] pub fn lsl_reg(rd: u8, rn: u8, rm: u8) -> u32 {
    0x9AC02000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// LSR Xd, Xn, Xm = LSRV Xd, Xn, Xm (logical shift right by register)
#[inline] pub fn lsr_reg(rd: u8, rn: u8, rm: u8) -> u32 {
    0x9AC02400 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// ASR Xd, Xn, Xm = ASRV Xd, Xn, Xm (arithmetic shift right by register)
#[inline] pub fn asr_reg(rd: u8, rn: u8, rm: u8) -> u32 {
    0x9AC02800 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

/// LSR Xd, Xn, #shift — logical shift right by immediate
#[inline] pub fn lsr_imm(rd: u8, rn: u8, shift: u8) -> u32 {
    debug_assert!(shift < 64, "lsr_imm: shift out of range");
    // UBFM Xd, Xn, #shift, #63
    0xD3400000 | ((shift as u32) << 16) | (63 << 10) | ((rn as u32) << 5) | (rd as u32)
}

// ── Move ──────────────────────────────────────────────────────────

/// MOV Xd, Xm = ORR Xd, XZR, Xm
#[inline] pub fn mov_reg(rd: u8, rm: u8) -> u32 {
    0xAA0003E0 | ((rm as u32) << 16) | (rd as u32)
}

/// MOVZ Xd, #imm16 (zero-extend; no shift)
#[inline] pub fn movz(rd: u8, imm16: u16) -> u32 {
    0xD2800000 | ((imm16 as u32) << 5) | (rd as u32)
}

/// MOVK Xd, #imm16, lsl #shift (shift: 0, 16, 32, 48)
#[inline] pub fn movk(rd: u8, imm16: u16, shift: u8) -> u32 {
    debug_assert!(shift == 0 || shift == 16 || shift == 32 || shift == 48);
    let hw = (shift / 16) as u32;
    0xF2800000 | (hw << 21) | ((imm16 as u32) << 5) | (rd as u32)
}

/// Encode a 64-bit immediate value into a MOVZ + up to 3 MOVK instructions.
pub fn mov_imm64(rd: u8, val: u64) -> Vec<u32> {
    let mut v = Vec::with_capacity(4);
    v.push(movz(rd, val as u16));
    if val > 0xFFFF {
        v.push(movk(rd, (val >> 16) as u16, 16));
    }
    if val > 0xFFFF_FFFF {
        v.push(movk(rd, (val >> 32) as u16, 32));
    }
    if val > 0xFFFF_FFFF_FFFF {
        v.push(movk(rd, (val >> 48) as u16, 48));
    }
    v
}

// ── Compare and select ────────────────────────────────────────────

/// CSET Xd, cond — set Xd to 1 if cond true, else 0.
/// Condition codes: 0=EQ, 1=NE, 2=CS/HS, 3=CC/LO, 4=MI, 5=PL,
///                  6=VS, 7=VC, 8=HI, 9=LS, 10=GE, 11=LT, 12=GT, 13=LE
#[inline] pub fn cset(rd: u8, cond: u8) -> u32 {
    // CSINC Xd, XZR, XZR, inv(cond)
    let inv = cond ^ 1;
    0x9A9F07E0 | ((inv as u32) << 12) | (rd as u32)
}

// ── Memory ────────────────────────────────────────────────────────

/// LDR Xd, [Xn, #(imm12 * 8)] — 64-bit load, unsigned scaled offset
#[inline] pub fn ldr_imm(rd: u8, rn: u8, imm12: u16) -> u32 {
    debug_assert!(imm12 < 4096, "ldr_imm: imm12 out of range");
    0xF9400000 | ((imm12 as u32) << 10) | ((rn as u32) << 5) | (rd as u32)
}

/// STR Xd, [Xn, #(imm12 * 8)] — 64-bit store, unsigned scaled offset
#[inline] pub fn str_imm(rs: u8, rn: u8, imm12: u16) -> u32 {
    debug_assert!(imm12 < 4096, "str_imm: imm12 out of range");
    0xF9000000 | ((imm12 as u32) << 10) | ((rn as u32) << 5) | (rs as u32)
}

/// LDUR Xd, [Xn, #simm9] — 64-bit load, unscaled signed offset (-256..255)
#[inline] pub fn ldur(rd: u8, rn: u8, simm9: i16) -> u32 {
    debug_assert!((-256..=255).contains(&(simm9 as i32)), "ldur: offset out of range");
    let imm9 = (simm9 as u32) & 0x1FF;
    0xF8400000 | (imm9 << 12) | ((rn as u32) << 5) | (rd as u32)
}

/// STUR Xd, [Xn, #simm9] — 64-bit store, unscaled signed offset
#[inline] pub fn stur(rs: u8, rn: u8, simm9: i16) -> u32 {
    debug_assert!((-256..=255).contains(&(simm9 as i32)), "stur: offset out of range");
    let imm9 = (simm9 as u32) & 0x1FF;
    0xF8000000 | (imm9 << 12) | ((rn as u32) << 5) | (rs as u32)
}

/// STP Xn, Xm, [Xd, #(imm7 * 8)]! — pre-index pair store (e.g. prologue)
#[inline] pub fn stp_pre(rn: u8, rm: u8, rd: u8, simm7: i8) -> u32 {
    let imm7 = (simm7 as u32) & 0x7F;
    0xA9800000 | (imm7 << 15) | ((rm as u32) << 10) | ((rd as u32) << 5) | (rn as u32)
}

/// LDP Xn, Xm, [Xd], #(imm7 * 8) — post-index pair load (e.g. epilogue)
#[inline] pub fn ldp_post(rn: u8, rm: u8, rd: u8, simm7: i8) -> u32 {
    let imm7 = (simm7 as u32) & 0x7F;
    0xA8C00000 | (imm7 << 15) | ((rm as u32) << 10) | ((rd as u32) << 5) | (rn as u32)
}

/// STP Xn, Xm, [Xd, #(imm7 * 8)] — signed-offset pair store (no writeback)
#[inline] pub fn stp_off(rn: u8, rm: u8, rd: u8, simm7: i8) -> u32 {
    let imm7 = (simm7 as u32) & 0x7F;
    0xA9000000 | (imm7 << 15) | ((rm as u32) << 10) | ((rd as u32) << 5) | (rn as u32)
}

/// LDP Xn, Xm, [Xd, #(imm7 * 8)] — signed-offset pair load
#[inline] pub fn ldp_off(rn: u8, rm: u8, rd: u8, simm7: i8) -> u32 {
    let imm7 = (simm7 as u32) & 0x7F;
    0xA9400000 | (imm7 << 15) | ((rm as u32) << 10) | ((rd as u32) << 5) | (rn as u32)
}

// ── PC-relative address ───────────────────────────────────────────

/// ADRP Xd, #imm21 — load page-aligned PC-relative address.
/// imm21 is the signed page offset in 4096-byte pages (imm21 << 12 is added to PC).
#[inline] pub fn adrp(rd: u8, imm21: i32) -> u32 {
    let immlo = (imm21 as u32) & 0x3;
    let immhi = ((imm21 as u32) >> 2) & 0x7FFFF;
    0x90000000 | (immlo << 29) | (immhi << 5) | (rd as u32)
}

/// ADR Xd, #imm21 — load PC-relative address (no page alignment).
#[inline] pub fn adr(rd: u8, imm21: i32) -> u32 {
    let immlo = (imm21 as u32) & 0x3;
    let immhi = ((imm21 as u32) >> 2) & 0x7FFFF;
    0x10000000 | (immlo << 29) | (immhi << 5) | (rd as u32)
}

// ── Branches ──────────────────────────────────────────────────────

/// B #imm26 — unconditional branch; imm26 is in instructions (not bytes).
#[inline] pub fn b_imm(imm26: i32) -> u32 {
    0x14000000 | ((imm26 as u32) & 0x03FF_FFFF)
}

/// BL #imm26 — branch with link (call); imm26 in instructions.
#[inline] pub fn bl_imm(imm26: i32) -> u32 {
    0x94000000 | ((imm26 as u32) & 0x03FF_FFFF)
}

/// BL placeholder — emits BL with zero offset; patched by the linker.
#[inline] pub fn bl_placeholder() -> u32 { 0x94000000 }

/// B.cond label — conditional branch; imm19 in instructions.
/// cond: 0=EQ, 1=NE, 2=CS, 3=CC, 4=MI, 5=PL, 6=VS, 7=VC,
///       8=HI, 9=LS, 10=GE, 11=LT, 12=GT, 13=LE
#[inline] pub fn b_cond(cond: u8, imm19: i32) -> u32 {
    0x54000000 | (((imm19 as u32) & 0x7FFFF) << 5) | (cond as u32)
}

/// CBNZ Xn, #imm19 — branch if Xn != 0; imm19 in instructions.
#[inline] pub fn cbnz(rn: u8, imm19: i32) -> u32 {
    0xB5000000 | (((imm19 as u32) & 0x7FFFF) << 5) | (rn as u32)
}

/// CBZ Xn, #imm19 — branch if Xn == 0.
#[inline] pub fn cbz(rn: u8, imm19: i32) -> u32 {
    0xB4000000 | (((imm19 as u32) & 0x7FFFF) << 5) | (rn as u32)
}

/// BR Xn — branch to register.
#[inline] pub fn br(rn: u8) -> u32 {
    0xD61F0000 | ((rn as u32) << 5)
}

/// BLR Xn — branch with link to register.
#[inline] pub fn blr(rn: u8) -> u32 {
    0xD63F0000 | ((rn as u32) << 5)
}

// ── Misc ──────────────────────────────────────────────────────────

/// RET — return via X30 (link register).
#[inline] pub fn ret() -> u32 { 0xD65F03C0 }

/// NOP
#[inline] pub fn nop() -> u32 { 0xD503201F }

/// SVC #imm16 — supervisor call (syscall on macOS/Linux).
#[inline] pub fn svc(imm16: u16) -> u32 {
    0xD4000001 | ((imm16 as u32) << 5)
}

/// BRK #imm16 — software breakpoint.
#[inline] pub fn brk(imm16: u16) -> u32 {
    0xD4200000 | ((imm16 as u32) << 5)
}
