//! Differential tests of the arithmetic of the name-plate functions `CGNamePlateFrame::Bind`
//! (`0x7cb6d0`), `OnUpdate` (`0x7cb950`), `DisplayedLevel` (`0x7cbce0`) and `SetRaidIcon`
//! (`0x7cbe20`), each run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice is diffed as an **instruction slice** (`Emu::run`) — registers/memory/stack
//! synthesized, the binary's own bytes executed, the stored `fstp/fst [m32]` value or the `__ftol`
//! EAX read back. Every in-body FP-arith opcode (the arithmetic opcode set) lands in a diffed
//! slice; the `fild` integer-converts and `fxch`/loads/stores are not arith and ride along inside
//! the slices for free. The two `__ftol` callees (`0x40a2b0`) run NATIVELY inside the slice (pure
//! x87, truncate toward zero). Fuzz reads the LCG's HIGH bits. Each
//! test carries deterministic `==`-boundary probes: the `__ftol` half-step pivot of
//! 0x7cbce0 and the `min(b,100)` clamp edge; the `count` split parity of 0x7cbe20.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::nameplates::{
    percent_round, quarter_split, scale, scale1000_ftol, K0_BITS, K1_BITS, K2_BITS, K3_BITS,
    K4_BITS, K5_BITS,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

// The two coordinate globals 0x7cb6d0 multiplies.
const G_X: u64 = 0x87d9cc;
const G_Y: u64 = 0x87d9d0;

struct Frame {
    ebp: u64,
    esp: u64,
    esi: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    let esi = emu.alloc(0x600); // [esi+0x4f8] read by 0x7cbe20 — point at mapped scratch
    Frame { ebp, esp, esi }
}

// ===========================================================================================
// 0x7cb6d0 — 6× `fld [G]; fmul [k]; fstp [m32]`. The six triples are independent (no register
// carry between them). Four store to [ebp-4]/[ebp+8] with no interleaved memory derefs and are
// diffed at their OWN address. Blocks #2/#3 (`fstp [esp]`) interleave object-graph derefs
// (`mov eax,[esi+0x504]; mov eax,[ecx]; push ecx`) that READ_UNMAPPED in isolation, so each is
// diffed via a COPIED-BYTE ISLAND of just its three FP ops — the EXACT source `fld`+`fmul` bytes
// (byte-verified below) with the `fstp [esp]` redirected to a known cell. fmul ×6.
//
// (fld_start, end, global, kbits, dest) — dest selects [ebp-4] vs [ebp+8] for the in-place blocks.
// ===========================================================================================
#[derive(Clone, Copy)]
enum Dest {
    EbpM4,
    EbpP8,
}
const SCALE_INPLACE: [(u64, u64, u64, u32, Dest); 4] = [
    (0x7cb747, 0x7cb755 + 3, G_X, K0_BITS, Dest::EbpM4), // fstp [ebp-4]
    (0x7cb758, 0x7cb767 + 3, G_Y, K1_BITS, Dest::EbpP8), // fstp [ebp+8]
    (0x7cb7bf, 0x7cb7cd + 3, G_X, K4_BITS, Dest::EbpM4), // fstp [ebp-4]
    (0x7cb7d0, 0x7cb7df + 3, G_Y, K5_BITS, Dest::EbpP8), // fstp [ebp+8]
];

#[test]
fn glue_7cb6d0_scale6() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run_inplace =
        |emu: &mut Emu, start: u64, end: u64, gaddr: u64, dest: Dest, g: f32| -> f32 {
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.write_u32(gaddr, g.to_bits());
            emu.run(start, end);
            match dest {
                Dest::EbpM4 => emu.read_f32(f.ebp - 0x4),
                Dest::EbpP8 => emu.read_f32(f.ebp + 0x8),
            }
        };

    // Copied-byte islands for blocks #2 (0x7cb782 fld;0x7cb78e fmul) and #3 (0x7cb7a6 fld;0x7cb7ac
    // fmul). The `fld`/`fmul` bytes are copied VERBATIM from the source addresses; a `fstp [cell]`
    // (D9 1D + disp32) is appended in place of the source's `fstp [esp]` (same store width).
    let cell2 = emu.alloc(4);
    let cell3 = emu.alloc(4);
    // block #2: fld [0x87d9cc] (D9 05 cc d9 87 00); fmul [0x81defc] (D8 0D fc de 81 00).
    let isl2 = emu.alloc(0x20);
    let mut c2 = vec![0xd9u8, 0x05, 0xcc, 0xd9, 0x87, 0x00];
    c2.extend_from_slice(&[0xd8, 0x0d, 0xfc, 0xde, 0x81, 0x00]);
    c2.extend_from_slice(&[0xd9, 0x1d]);
    c2.extend_from_slice(&(cell2 as u32).to_le_bytes());
    let isl2_end = isl2 + c2.len() as u64;
    emu.write(isl2, &c2);
    // block #3: fld [0x87d9d0] (D9 05 d0 d9 87 00); fmul [0x81def8] (D8 0D f8 de 81 00).
    let isl3 = emu.alloc(0x20);
    let mut c3 = vec![0xd9u8, 0x05, 0xd0, 0xd9, 0x87, 0x00];
    c3.extend_from_slice(&[0xd8, 0x0d, 0xf8, 0xde, 0x81, 0x00]);
    c3.extend_from_slice(&[0xd9, 0x1d]);
    c3.extend_from_slice(&(cell3 as u32).to_le_bytes());
    let isl3_end = isl3 + c3.len() as u64;
    emu.write(isl3, &c3);
    // byte-verify each island FP op against its REAL source address (the source interleaves an
    // integer `mov eax,[esi+0x504]` between fld and fmul — the island packs the two FP ops
    // contiguously, so each is verified against its own source span, not a contiguous window).
    // islands credited only when the bytes match source.
    assert_eq!(
        &emu.read(isl2, 6),
        &emu.read(0x7cb782, 6),
        "isl2 fld != src"
    ); // fld
    assert_eq!(
        &emu.read(isl2 + 6, 6),
        &emu.read(0x7cb78e, 6),
        "isl2 fmul != src"
    ); // fmul
    assert_eq!(
        &emu.read(isl3, 6),
        &emu.read(0x7cb7a6, 6),
        "isl3 fld != src"
    ); // fld
    assert_eq!(
        &emu.read(isl3 + 6, 6),
        &emu.read(0x7cb7ac, 6),
        "isl3 fmul != src"
    ); // fmul

    let run_island = |emu: &mut Emu, isl: u64, end: u64, gaddr: u64, cell: u64, g: f32| -> f32 {
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(gaddr, g.to_bits());
        emu.run(isl, end);
        emu.read_f32(cell)
    };

    let probes = [
        0.0f32,
        1.0,
        1024.0,
        -7.5,
        1.0e6,
        f32::from_bits(0x4b00_0001),
    ];
    let mut total = 0u32;
    for &(start, end, gaddr, kbits, dest) in &SCALE_INPLACE {
        for g in probes {
            assert_eq!(
                run_inplace(&mut emu, start, end, gaddr, dest, g).to_bits(),
                scale(g, kbits).to_bits(),
                "scale @ {start:#x} g={g}"
            );
            total += 1;
        }
        let mut s = (start as u32).wrapping_mul(0x1000);
        for _ in 0..1500 {
            let g = rng(&mut s, -10000.0, 10000.0);
            assert_eq!(
                run_inplace(&mut emu, start, end, gaddr, dest, g).to_bits(),
                scale(g, kbits).to_bits(),
                "scale @ {start:#x} fuzz g={g}"
            );
            total += 1;
        }
    }
    // islands (blocks #2 K2 @G_X, #3 K3 @G_Y).
    for &(isl, end, gaddr, cell, kbits, lbl) in &[
        (isl2, isl2_end, G_X, cell2, K2_BITS, 0x7cb782u64),
        (isl3, isl3_end, G_Y, cell3, K3_BITS, 0x7cb7a6u64),
    ] {
        for g in probes {
            assert_eq!(
                run_island(&mut emu, isl, end, gaddr, cell, g).to_bits(),
                scale(g, kbits).to_bits(),
                "scale island @ {lbl:#x} g={g}"
            );
            total += 1;
        }
        let mut s = (lbl as u32).wrapping_mul(0x2000);
        for _ in 0..1500 {
            let g = rng(&mut s, -10000.0, 10000.0);
            assert_eq!(
                run_island(&mut emu, isl, end, gaddr, cell, g).to_bits(),
                scale(g, kbits).to_bits(),
                "scale island @ {lbl:#x} fuzz g={g}"
            );
            total += 1;
        }
    }
    eprintln!("glue_7cb6d0_scale6: {total} bit-exact (4 in-place + 2 copied-byte-island fld;fmul[k];fstp)");
}

// ===========================================================================================
// 0x7cb950 — `fld [ebp+8]; fmul [1000.0]; call __ftol`. __ftol native (truncate). fmul ×1.
// ===========================================================================================
#[test]
fn glue_7cb950_scale1000() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, arg: f32| -> i32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.run(0x7cb962, 0x7cb970); // fld;fmul;call __ftol — halt after the call returns
        emu.get_reg(RegisterX86::EAX) as i32
    };

    let mut total = 0u32;
    // deterministic probes incl. the truncate-toward-zero boundary (x·1000 just below/at an int).
    for arg in [
        0.0f32,
        1.0,
        1.5,
        -1.5,
        0.0009999, // ·1000 = 0.9999 → 0
        0.001,     // ·1000 = 1.0 → 1
        -0.001,
        12.3456,
        1.9999999, // just under 2000
        // NaN, the infinities and products past `i32` and `i64`: `__ftol` returns the low dword
        // of its qword, 0 for the indefinite.
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        4.0e6,
        -4.0e6,
        1.0e30,
        -1.0e30,
    ] {
        assert_eq!(
            run(&mut emu, arg),
            scale1000_ftol(arg),
            "scale1000 arg={arg}"
        );
        total += 1;
    }
    let mut s = 0x7cb9_5000u32;
    for _ in 0..4000 {
        let arg = rng(&mut s, -5000.0, 5000.0);
        assert_eq!(
            run(&mut emu, arg),
            scale1000_ftol(arg),
            "scale1000 fuzz arg={arg}"
        );
        total += 1;
    }
    eprintln!("glue_7cb950_scale1000: {total} bit-exact (·1000 then __ftol truncate)");
}

// ===========================================================================================
// 0x7cbce0 — `fild [ebp-4]=n; fmul [0.05]; fadd [0.5]; call __ftol`. fmul ×1 + fadd ×1.
// The clamp `min(b,100)` is integer (in the body's prologue); the diffed slice covers the FP
// arith on the already-clamped n (we feed n directly, and also exercise the clamp via
// `percent_round` for the full-fn boundary).
// ===========================================================================================
#[test]
fn glue_7cbce0_percent_round() {
    // The byte is clamped to 100, so the `__ftol` input is at most 5.5 and no NaN or out-of-range
    // value reaches it.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // diff the FP slice directly: set [ebp-4]=n, run fild;fmul[0.05];fadd[0.5];__ftol, read EAX.
    let run = |emu: &mut Emu, n: u32| -> i32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x4, n);
        emu.run(0x7cbd27, 0x7cbd3b); // fild;fmul;fadd;call __ftol — halt after the call returns
        emu.get_reg(RegisterX86::EAX) as i32
    };

    let mut total = 0u32;
    // every clamped n in 0..=100 (the full domain after min(b,100)), incl. the round-half pivots.
    for n in 0..=100u32 {
        // the binary's percent_round(b) clamps b→min(b,100); for n<=100 that is n. Diff n·0.05+0.5.
        let want = {
            // mirror percent_round but with n pre-clamped (it is <=100 here).
            let v = (f64::from(n) * f64::from(f32::from_bits(0x3d4c_cccd)))
                + f64::from(f32::from_bits(0x3f00_0000));
            v.trunc() as i32
        };
        assert_eq!(run(&mut emu, n), want, "percent_round_fp n={n}");
        // and the full-fn transcription (b<=100 ⇒ same path).
        assert_eq!(
            percent_round(n as u8),
            want,
            "percent_round transcription n={n}"
        );
        total += 1;
    }
    // clamp boundary: b in 100..=255 all collapse to n=100 inside the binary; the transcription's
    // `b.min(100)` must match. (The diffed slice feeds n directly, so the clamp itself is checked
    // against the binary's `cmp al,0x64` purely in the transcription path here — n=100 is the cap.)
    let want100 = run(&mut emu, 100);
    for b in [100u8, 101, 150, 200, 255] {
        assert_eq!(percent_round(b), want100, "percent_round clamp b={b}");
        total += 1;
    }
    eprintln!(
        "glue_7cbce0_percent_round: {total} bit-exact (n·0.05+0.5 then __ftol; clamp min(b,100))"
    );
}

// ===========================================================================================
// 0x7cbe20 — split count → 2 quarter pairs. Slice 0x7cbe4d..0x7cbe9b: set EAX=count, read the
// four f32 stores [ebp-0xc]/[ebp-0x10]/[ebp-0x4]/[ebp-0x8]. fmul ×2 + fadd ×2.
// ===========================================================================================
#[test]
fn glue_7cbe20_quarter_split() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // returns (a=[ebp-0xc], b=[ebp-0x10], c_out=[ebp-0x4], d=[ebp-0x8]).
    let run = |emu: &mut Emu, count: u32| -> (f32, f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, f.esi); // [esi+0x4f8] read at 0x7cbe69
        emu.set_reg(RegisterX86::EAX, u64::from(count));
        emu.run(0x7cbe4d, 0x7cbe9b); // through the 4 stores, halt before call 0x770410
        (
            emu.read_f32(f.ebp - 0xc),
            emu.read_f32(f.ebp - 0x10),
            emu.read_f32(f.ebp - 0x4),
            emu.read_f32(f.ebp - 0x8),
        )
    };

    let mut total = 0u32;
    // deterministic: all four (count&3) residues, small counts, large, and the path-entry min (8).
    for count in [
        8u32, 9, 10, 11, 12, 13, 16, 31, 32, 100, 255, 256, 1023, 65535, 1_000_000,
    ] {
        let (a, b, c, d) = run(&mut emu, count);
        let (wa, wb, wc, wd) = quarter_split(count);
        assert_eq!(a.to_bits(), wa.to_bits(), "qsplit a count={count}");
        assert_eq!(b.to_bits(), wb.to_bits(), "qsplit b count={count}");
        assert_eq!(c.to_bits(), wc.to_bits(), "qsplit c count={count}");
        assert_eq!(d.to_bits(), wd.to_bits(), "qsplit d count={count}");
        total += 4;
    }
    let mut s = 0x7cbe_2000u32;
    for _ in 0..4000 {
        // count >= 8 (the slice's path); high bits give a wide spread.
        let count = 8 + ((next(&mut s) >> 8) % 4_000_000);
        let (a, b, c, d) = run(&mut emu, count);
        let (wa, wb, wc, wd) = quarter_split(count);
        assert_eq!(a.to_bits(), wa.to_bits(), "qsplit a fuzz count={count}");
        assert_eq!(b.to_bits(), wb.to_bits(), "qsplit b fuzz count={count}");
        assert_eq!(c.to_bits(), wc.to_bits(), "qsplit c fuzz count={count}");
        assert_eq!(d.to_bits(), wd.to_bits(), "qsplit d fuzz count={count}");
        total += 4;
    }
    eprintln!("glue_7cbe20_quarter_split: {total} bit-exact (fmul[0.25]×2 + fadd[0.25]×2)");
}
