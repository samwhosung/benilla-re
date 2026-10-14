//! Bit-exact diff for `playername::swing::swing_step_count` (`0x6c6560`'s cadence slice
//! `[0x6c65c6, 0x6c6606)`) vs `WoW.exe`.
//!
//! Slice trace (re-derived from objdump bytes):
//! ```text
//!   6c65c6: fild  [ebp+8]           ; elapsed (i32)
//!   6c65cf: movzx edi, byte [esi+0x60b] ; segcount (u8)
//!   6c65d9: fmul  [0x811278]=1/300 f32
//!   6c65df: fimul [ebp+0xc]=segcount i32
//!   6c65ea: call  0x40a2b0          ; __ftol -> eax (truncate toward zero, runs native in slice)
//!   6c65f4: al = max(al, 1)
//!   6c6601: idiv  ecx               ; eax = segcount / clamp  (signed, native idiv)
//!   6c6606: (halt) eax = steps
//! ```
//! Entry state: `esi` = swing `this` (scratch object holding `[+0x60b]`=segcount, `[+0x614]`=last);
//! `[ebp+8]` = `elapsed`; `edi` = `now_ms` (only used by the `mov [esi+0x614], edi` store side
//! effect inside the slice — does not feed `eax`). We observe `eax` at the `jg` (`0x6c6606`).

#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_playername::swing::{string_measure_layout, swing_step_count};

// shared LCG fuzz (high bits only)
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

#[test]
fn pn_swing_step_count() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // swing `this` object: only [+0x60b] (segcount byte) and [+0x614] (last-eval, written) matter.
    let this = emu.alloc(0x640);
    let scratch = emu.alloc(0x40);
    let ebp = scratch + 0x20; // the slice reads [ebp+8]=elapsed and spills segcount to [ebp+0xc]
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;

    // Run the slice for (elapsed, segcount): write the inputs, run [0x6c65c6, 0x6c6606), read eax.
    let run = |emu: &mut Emu, elapsed: i32, segcount: u8| -> i32 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::EDI, 0); // now_ms — only the [esi+0x614] store reads it; irrelevant
        emu.write_u32(ebp + 0x8, elapsed as u32); // [ebp+8] = elapsed (i32) consumed by fild
        emu.write(this + 0x60b, &[segcount]); // swing[+0x60b] = segcount (u8)
        emu.write_u32(this + 0x614, 0); // swing[+0x614] = last-eval (overwritten in slice)
        emu.run(0x6c65c6, 0x6c6606);
        emu.get_reg(RegisterX86::EAX) as u32 as i32
    };

    let mut total = 0u32;

    // Deterministic boundary probes.
    // The __ftol truncation pivot: elapsed·(1/300)·segcount landing exactly on an integer N, and
    // N±1ms either side; the al==1 clamp edge (product in [0,1) -> al=0 -> clamp 1); the al-byte
    // wrap (product >= 256 -> al wraps, e.g. 256 -> al=0 -> clamp 1); the steps<=1 boundary.
    let probes: &[(i32, u8)] = &[
        (0, 0),     // elapsed 0, segcount 0  -> product 0, idiv 0/1 = 0
        (0, 1),     // 0/1 = 0
        (0, 8),     // product 0 -> clamp 1 -> 8/1 = 8
        (0, 255),   // 255/1
        (300, 1),   // 300·(1/300)·1 = ~1.0 (f32 rounding near 1) — clamp/pivot edge
        (300, 8),   // 300·(1/300)·8 = ~8.0 -> 8/8 = 1 -> steps<=1 boundary
        (300, 16),  // ~16 -> 16/16 = 1
        (299, 8),   // just under -> ftol 7 -> 8/7 = 1
        (301, 8),   // just over -> ftol 8 -> 8/8 = 1
        (150, 8),   // 8·150/300 = 4.0 -> 8/4 = 2 -> steps>1
        (75, 8),    // 8·75/300 = 2.0 -> 8/2 = 4
        (37, 8),    // 8·37/300 ~= 0.986 -> ftol 0 -> clamp 1 -> 8/1 = 8
        (38, 8),    // ~1.013 -> ftol 1 -> 8/1 = 8
        (9600, 8),  // 8·9600/300 = 256.0 -> al wraps to 0 -> clamp 1 -> 8/1 = 8
        (9637, 8),  // ~257 -> al wraps to 1 -> 8/1 = 8
        (-300, 8), // negative elapsed -> product -8.0 -> ftol -8, al=0xf8, clamp 0xf8=248 -> 255? no, segcount 8 -> 8/248=0
        (-1, 255), // negative small
        (-9600, 8), // -256 -> al = 0 -> clamp 1
        (i32::MAX, 255),
        (i32::MIN, 255),
        (1, 255),
        (300, 255), // ~255 -> 255/255 = 1
        (301, 255), // ftol 255 -> 255/255 = 1 ; check rounding
        (12345, 100),
    ];
    for &(elapsed, segcount) in probes {
        let got = run(&mut emu, elapsed, segcount);
        let want = swing_step_count(elapsed, segcount);
        assert_eq!(
            got, want,
            "swing_step_count probe elapsed={elapsed} segcount={segcount}: emu={got} rust={want}"
        );
        total += 1;
    }

    // Fuzz: HIGH LCG bits for elapsed (wide signed range incl. negatives), segcount 0..=255.
    let mut s = 0x6c65_6000u32;
    for _ in 0..20000 {
        // elapsed: full i32 range via the high bits of two LCG draws.
        let elapsed = next(&mut s) as i32;
        let segcount = (next(&mut s) >> 24) as u8;
        let got = run(&mut emu, elapsed, segcount);
        let want = swing_step_count(elapsed, segcount);
        assert_eq!(
            got, want,
            "swing_step_count fuzz elapsed={elapsed} segcount={segcount}: emu={got} rust={want}"
        );
        total += 1;
    }

    // A second fuzz band concentrated near the realistic regime (small elapsed, all segcounts),
    // so the truncation/clamp boundaries are densely exercised.
    let mut s2 = 0x0c6c_5601u32;
    for _ in 0..20000 {
        let elapsed = (next(&mut s2) >> 16) as i32; // 0..=65535 ms
        let segcount = (next(&mut s2) >> 24) as u8;
        let got = run(&mut emu, elapsed, segcount);
        let want = swing_step_count(elapsed, segcount);
        assert_eq!(
            got, want,
            "swing_step_count fuzz2 elapsed={elapsed} segcount={segcount}: emu={got} rust={want}"
        );
        total += 1;
    }

    eprintln!("pn_swing_step_count: {total} bit-exact (fild;fmul[1/300];fimul;__ftol;clamp;idiv)");
}

// 0x6c81a0 string_measure_layout FP tail [0x6c828f, 0x6c82b4):
//   fld [ebp-0xc]=w; fmul [0x7ffa24]=0.5; fstp [ebp-0x18]; mov [esi+0xc],edx
//   fld [ebp-0x10]=h; fmul [0x7ffa24]=0.5; fstp [ebp-0x14]; mov [esi+0x10],eax
// Entry state: esi=this, [ebp-0xc]=w, [ebp-0x10]=h (gx-measure outputs). The slice has scratch
// frame writes; the rest of [ebp-*] is uninitialized (irrelevant). Observe this[+0xc]/this[+0x10].
#[test]
fn pn_string_measure_layout() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40); // result struct: this[+0xc]=w·0.5, this[+0x10]=h·0.5
    let scratch = emu.alloc(0x60);
    let ebp = scratch + 0x30; // slice reads [ebp-0xc],[ebp-0x10]; writes [ebp-0x18],[ebp-0x14]
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;

    let run = |emu: &mut Emu, w: f32, h: f32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::ESI, this);
        emu.write_u32(ebp - 0xc, w.to_bits()); // [ebp-0xc] = w (measured run width, f32)
        emu.write_u32(ebp - 0x10, h.to_bits()); // [ebp-0x10] = h (measured run height, f32)
        emu.run(0x6c828f, 0x6c82b4); // fld;fmul[0.5];fstp ×2 + the two stores to this
        (emu.read_f32(this + 0xc), emu.read_f32(this + 0x10))
    };

    let mut total = 0u32;
    // Deterministic probes: 0/-0, exact halves, sub-normals, denormal-producing, signs, large.
    let probes: &[(f32, f32)] = &[
        (0.0, 0.0),
        (-0.0, -0.0),
        (1.0, 1.0),
        (-1.0, 2.0),
        (3.0, 5.0), // odd -> 1.5 / 2.5 (exact halves)
        (0.333_333_34, 0.666_666_7),
        (1e-7, -1e-7),
        (f32::from_bits(1), f32::from_bits(2)), // smallest denormals -> halve to denormal/0
        (f32::MIN_POSITIVE, f32::MIN_POSITIVE),
        (1e30, -1e30),
        (16384.0, 9000.0), // px-scale glyph runs
        (100.5, 32.25),
    ];
    for &(w, h) in probes {
        let (gw, gh) = run(&mut emu, w, h);
        let (rw, rh) = string_measure_layout(w, h);
        assert_eq!(
            gw.to_bits(),
            rw.to_bits(),
            "string_measure_layout w={w}: emu={gw} rust={rw}"
        );
        assert_eq!(
            gh.to_bits(),
            rh.to_bits(),
            "string_measure_layout h={h}: emu={gh} rust={rh}"
        );
        total += 1;
    }
    // Fuzz HIGH LCG bits across realistic px widths/heights and arbitrary f32 bit patterns.
    let mut s = 0x6c81_a000u32;
    for _ in 0..10000 {
        let w = f32::from_bits(next(&mut s));
        let h = f32::from_bits(next(&mut s));
        if !w.is_finite() || !h.is_finite() {
            continue; // NaN/Inf: the gx measure never yields these; the ·0.5 is the same anyway
        }
        let (gw, gh) = run(&mut emu, w, h);
        let (rw, rh) = string_measure_layout(w, h);
        assert_eq!(gw.to_bits(), rw.to_bits(), "fuzz w={w}");
        assert_eq!(gh.to_bits(), rh.to_bits(), "fuzz h={h}");
        total += 1;
    }
    // A band of realistic positive px magnitudes (where the layout actually runs).
    let mut s2 = 0x00a1_816cu32;
    for _ in 0..10000 {
        let w = (next(&mut s2) >> 16) as f32 * 0.5; // 0..~32767 px
        let h = (next(&mut s2) >> 18) as f32 * 0.25; // 0..~4095 px
        let (gw, gh) = run(&mut emu, w, h);
        let (rw, rh) = string_measure_layout(w, h);
        assert_eq!(gw.to_bits(), rw.to_bits(), "fuzz2 w={w}");
        assert_eq!(gh.to_bits(), rh.to_bits(), "fuzz2 h={h}");
        total += 1;
    }
    eprintln!("pn_string_measure_layout: {total} bit-exact (fld;fmul[0.5];fstp ×2)");
}
