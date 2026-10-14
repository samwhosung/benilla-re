//! The span each edge resolver passes to the combiners (`systems/ui/anchors.md#edge-resolution-from-size`)
//! run from the exe: `CLayoutFrame::GetCenterX` (`0x767260`), `GetCenterY` (`0x767360`), `GetLeft`
//! (`0x7673d0`), `GetTop` (`0x7674d0`), `GetRight` (`0x767540`) and `GetBottom` (`0x7675b0`).
//!
//! The span is `size * scale`: `fmul dword [esi+0x58]` (`d8 4e 58`) on the width or height that
//! the getter leaves in `st0`, with `fchs` (`d9 e0`) for left and bottom, then `fstp dword [esp]`
//! stores the argument as `f32`. The store goes to the slot a `push ecx` reserved one instruction
//! before it, and `mov ecx, esi` between them is x87-inert.
//!
//! Each row runs as an island: an `fld dword [size_cell]` that stands in for the getter, followed
//! by the source bytes copied verbatim from the `fmul` to the `fstp`. The test first asserts that
//! the bytes at every row's address are the island's bytes, so the island is the exe's code and
//! not a copy of the Rust. The four no-negate rows share one byte shape and the two negate rows
//! another. Deterministic probes (size 0, a product that overflows to infinity, the sign of zero,
//! NaN) accompany the random draws.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::anchors::{span, span_neg};

// ---- LCG draws (the high bits only) ----
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

// Verbatim source span bytes (`fmul [esi+0x58]; push ecx; mov ecx,esi; [fchs;] fstp [esp]`).
const SPAN_NONEG: &[u8] = &[0xd8, 0x4e, 0x58, 0x51, 0x8b, 0xce, 0xd9, 0x1c, 0x24];
const SPAN_NEG: &[u8] = &[
    0xd8, 0x4e, 0x58, 0x51, 0x8b, 0xce, 0xd9, 0xe0, 0xd9, 0x1c, 0x24,
];

// The six rows: (source_addr, label).
const ROWS_NONEG: [(u64, &str); 4] = [
    (0x76729c, "Xcenter 0x767260"),
    (0x76739c, "Ycenter 0x767360"),
    (0x76750c, "TOP 0x7674d0"),
    (0x76757c, "RIGHT 0x767540"),
];
const ROWS_NEG: [(u64, &str); 2] = [(0x76740c, "LEFT 0x7673d0"), (0x7675ec, "BOTTOM 0x7675b0")];

/// Build a span island: `fld dword [size_cell]` + the verbatim source span bytes (ending in
/// `fstp dword [esp]`, which stores into the slot the island's `push ecx` reserved). Returns
/// `(island_va, island_end)`.
fn build_island(emu: &mut Emu, size_cell: u64, verbatim: &[u8]) -> (u64, u64) {
    let region = emu.alloc(0x40);
    let mut code = vec![0xd9u8, 0x05]; // fld dword [size_cell]
    code.extend_from_slice(&(size_cell as u32).to_le_bytes());
    code.extend_from_slice(verbatim); // VERBATIM source: fmul; push ecx; mov ecx,esi; [fchs;] fstp [esp]
    let end = region + code.len() as u64;
    emu.write(region, &code);
    (region, end)
}

#[test]
fn layout_767_span() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x100); // the frame struct (esi); +0x58 = layoutScale
    let size_cell = emu.alloc(4);
    let stack = emu.alloc(0x1000);
    let esp0 = stack + 0xf00; // a valid scratch ESP; `push ecx` writes esp0-4, `fstp [esp]` overwrites it

    // byte-verify the source span ops at EVERY row address BEFORE building the islands.
    for &(addr, l) in &ROWS_NONEG {
        assert_eq!(
            emu.read(addr, SPAN_NONEG.len()),
            SPAN_NONEG,
            "{l}: source span bytes at {addr:#x} differ from no-negate island verbatim"
        );
    }
    for &(addr, l) in &ROWS_NEG {
        assert_eq!(
            emu.read(addr, SPAN_NEG.len()),
            SPAN_NEG,
            "{l}: source span bytes at {addr:#x} differ from negate island verbatim"
        );
    }

    let (isl_p, isl_p_end) = build_island(&mut emu, size_cell, SPAN_NONEG);
    let (isl_n, isl_n_end) = build_island(&mut emu, size_cell, SPAN_NEG);

    let run = |emu: &mut Emu, start: u64, end: u64, size: f32, scale: f32| -> f32 {
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::ECX, 0); // pushed then overwritten by `mov ecx,esi` — value irrelevant
        emu.set_reg(RegisterX86::ESP, esp0);
        emu.write_u32(size_cell, size.to_bits());
        emu.write_u32(this + 0x58, scale.to_bits());
        emu.run(start, end);
        emu.read_f32(esp0 - 4) // the slot `push ecx` reserved, then `fstp [esp]` overwrote
    };

    let mut total = 0u32;

    // deterministic boundary probes — the values fuzz never lands on exactly.
    let probes: [(f32, f32); 9] = [
        (0.0, 1.0),                // 0·scale = +0
        (-0.0, 1.0),               // -0·scale: span = -0, span_neg = +0
        (1.0, 0.0),                // size·0 = +0
        (1.0, -0.0),               // size·-0: span = -0, span_neg = +0
        (1.0, 1.0),                // identity
        (f32::MAX, 2.0),           // overflow → +inf at the f32 store
        (f32::MAX, -2.0),          // overflow → -inf
        (f32::NAN, 1.0),           // NaN propagation (fchs flips NaN sign bit)
        (123.456_79, 0.012_345_6), // a non-trivial exact-bit case
    ];
    for &(size, scale) in &probes {
        for &(_addr, l) in &ROWS_NONEG {
            let bin = run(&mut emu, isl_p, isl_p_end, size, scale);
            assert_eq!(
                bin.to_bits(),
                span(size, scale).to_bits(),
                "{l} span size={size} scale={scale}"
            );
            total += 1;
        }
        for &(_addr, l) in &ROWS_NEG {
            let bin = run(&mut emu, isl_n, isl_n_end, size, scale);
            assert_eq!(
                bin.to_bits(),
                span_neg(size, scale).to_bits(),
                "{l} span_neg size={size} scale={scale}"
            );
            total += 1;
        }
    }

    // fuzz: scales in the layout range (0..~6, the frame/camera scale band) × sizes across the
    // pixel-extent range, both signs.
    let mut s = 0x7672_6000u32;
    for _ in 0..4000 {
        let size = rng(&mut s, -4096.0, 4096.0);
        let scale = rng(&mut s, 0.0, 6.0);
        let bin_p = run(&mut emu, isl_p, isl_p_end, size, scale);
        assert_eq!(
            bin_p.to_bits(),
            span(size, scale).to_bits(),
            "noneg fuzz size={size} scale={scale}"
        );
        let bin_n = run(&mut emu, isl_n, isl_n_end, size, scale);
        assert_eq!(
            bin_n.to_bits(),
            span_neg(size, scale).to_bits(),
            "neg fuzz size={size} scale={scale}"
        );
        total += 2;
    }

    eprintln!(
        "layout_767_span: {total} bit-exact (6 rows: 4 no-negate fmul, 2 negate fmul+fchs; \
         islands byte-verified vs source)"
    );
}
