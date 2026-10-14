//! Differential tests of the progress ratio of `LoadTocFile` (`0x6edb90`), each run against
//! `WoW.exe` in the emulator.
//!
//! Coverage of the arithmetic opcode set (load/store/compare/convert/`fild` excluded), counted in
//! `[0x6edb90, 0x6ede00)` (TRUE `ret 0x4` at `0x6ede00`, followed by `nop` padding):
//!
//! * `0x6edb90` glue_geom_6ed [.,0x6ede00 ret 0x4]: `fidiv dword [den]` ×1  (@0x6edd80)
//!
//! Diff strategy: an own-address **slice** `Emu::run(0x6edd6f, 0x6edd91)` over the exact source
//! sequence `inc eax; mov [ebp-0x24]; mov [ebp-0x20]; fild qword; mov [ebp-0x1c]; mov [ebp-0x18];
//! push ecx; fidiv dword [ebp-0x1c]; mov ecx,[g]; mov [g],eax; fstp dword [esp]`, halting at the
//! `call edx` (a basic-block boundary; the call is never executed). The slice's reads come from
//! registers we control (`eax` = pre-increment count, `ecx` = denominator, `ebx` = 0) and from the
//! `[ebp-...]` stack cells the slice itself fills; it reads `[0xceea38]` into ecx (harmless) and
//! writes `[0xcee9fc]` (harmless), both in mapped `.data`. The `push ecx` reserves the float-arg
//! slot, into which `fstp dword [esp]` narrows the PC_53 `f64` ratio; we read that `f32` back.
//!
//! The numerator the source `fild`s is `count + 1` with the high dword forced to 0, so it is a
//! non-negative `u32` value; the denominator is the **signed** i32 of `fidiv dword`. The `==`
//! boundary probe sits on the exact-representable pivots (count+1 == den -> ratio exactly 1.0, and
//! the count == 0 first-tick). Fuzz reads the LCG's HIGH bits.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::toc_loader::progress_ratio;

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

// ===========================================================================================
// Row 0x6edb90 progress-ratio — slice 0x6edd6f..0x6edd91:
//   inc eax; mov [ebp-0x24],eax; mov [ebp-0x20],ebx; fild qword [ebp-0x24];
//   mov [ebp-0x1c],ecx; mov [ebp-0x18],ebx; push ecx; fidiv dword [ebp-0x1c];
//   mov ecx,[0xceea38]; mov [0xcee9fc],eax; fstp dword [esp]   (halt at the `call edx`).
// fidiv @0x6edd80 (×1). Feed eax = count (inc -> count+1 = numerator), ecx = den, ebx = 0.
// The `push ecx` drops esp by 4 before the fstp, so the f32 lands at [esp_initial - 4].
// ===========================================================================================
#[test]
fn glue_geom_6edb90_progress_ratio() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x80; // room for [ebp-0x80 .. ebp]
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;

    // `count` = pre-increment running count [0xcee9fc]; numerator the source fild's = count + 1
    // (its high dword forced to 0 -> the u32 `count+1`). `den` = total [0xceea3c] (signed i32).
    let run = |emu: &mut Emu, count: u32, den: i32| -> f32 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::EAX, u64::from(count)); // `inc eax` -> count+1 = numerator
        emu.set_reg(RegisterX86::ECX, u64::from(den as u32)); // -> [ebp-0x1c], the fidiv divisor
        emu.set_reg(RegisterX86::EBX, 0); // numerator hi-dword + [ebp-0x18]
        emu.run(0x6edd6f, 0x6edd91);
        emu.read_f32(esp - 4) // fstp dword [esp] after the `push ecx`
    };

    let mut total = 0u32;

    // deterministic boundary probes: the runtime domain is 1 <= count+1 <= den (the call is gated
    // by count < den), so the ratio sweeps (0, 1]. Pivots: first tick, last tick (== 1.0),
    // mid-points, and exact-representable denominators.
    let probes: &[(u32, i32)] = &[
        (0, 1),               // num=1, den=1 -> 1.0
        (0, 2),               // num=1, den=2 -> 0.5
        (1, 2),               // num=2, den=2 -> 1.0
        (0, 100),             // num=1, den=100
        (49, 100),            // num=50, den=100 -> 0.5
        (99, 100),            // num=100, den=100 -> 1.0
        (0, 3),               // num=1, den=3 -> 0.333...
        (1, 3),               // num=2, den=3
        (2, 3),               // num=3, den=3 -> 1.0
        (0, 1000),            // first tick of a big TOC
        (999, 1000),          // last tick -> 1.0
        (0, 16777216),        // den = 2^24 (f32 mantissa edge)
        (16777215, 16777216), // num=2^24, den=2^24 -> 1.0 (num exactly representable in f32)
        (16777216, 16777217), // num=2^24+1 (NOT exactly f32-representable; f64 chain then narrow)
    ];
    for &(count, den) in probes {
        assert_eq!(
            run(&mut emu, count, den).to_bits(),
            progress_ratio(count.wrapping_add(1), den).to_bits(),
            "progress_ratio count={count} den={den}"
        );
        total += 1;
    }

    // dense sweep: small TOC sizes, every tick.
    for den in 1i32..=64 {
        for count in 0u32..(den as u32) {
            assert_eq!(
                run(&mut emu, count, den).to_bits(),
                progress_ratio(count + 1, den).to_bits(),
                "progress_ratio (sweep) count={count} den={den}"
            );
            total += 1;
        }
    }

    // fuzz: HIGH LCG bits, broad numerator + denominator domain (including the domain-agnostic
    // arithmetic outside the runtime [1,den] window — den negative / huge num — the op is the same).
    let mut s = 0x6edb_9000u32;
    for _ in 0..6000 {
        let count = next(&mut s) >> 8; // u32 numerator base (24 high bits)
        let den = (next(&mut s) >> 8) as i32; // i32 denominator (24 high bits, may be 0)
                                              // skip den==0 (the runtime never divides by zero; an x87 fdiv by 0 yields inf/nan which
                                              // .to_bits() still compares exactly, but keep the diff on the meaningful domain).
        if den == 0 {
            continue;
        }
        assert_eq!(
            run(&mut emu, count, den).to_bits(),
            progress_ratio(count.wrapping_add(1), den).to_bits(),
            "progress_ratio (fuzz) count={count} den={den}"
        );
        total += 1;
    }

    // explicit den == 0 case: x87 fidiv by 0 -> +inf (num>0); confirm bit-exact incl. the special.
    {
        let count = 5u32;
        assert_eq!(
            run(&mut emu, count, 0).to_bits(),
            progress_ratio(count + 1, 0).to_bits(),
            "progress_ratio div-by-zero count={count}"
        );
        total += 1;
    }

    eprintln!(
        "glue_geom_6edb90_progress_ratio: {total} bit-exact (slice: fild qword; fidiv dword [den]; fstp dword)"
    );
}
