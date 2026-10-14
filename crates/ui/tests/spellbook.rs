//! Differential tests of the arithmetic of the book-frame functions `0x4b0940`, `0x4b14a0`,
//! `0x4b1600` and the slot reader `ReadBookSlotArgs` (`0x4b3ec0`), each run against `WoW.exe` in
//! the emulator.
//!
//! Each owned FP slice is diffed by running the binary's **own bytes** (`Emu::run`) over just the
//! FP compute — registers/memory/stack synthesized, the no-arg getter `0x41ad70` and the one-arg
//! scaler `0x41ae60` executing NATIVELY inside the slice (both are pure `fld`/`fmul` over the two
//! UI-scale globals `[0x832a4c]`/`[0x832a44]`, which the test writes), the stored `fst/fstp [m32]`
//! value read back. The `0x4b3ec0` `__ftol` (`0x40a2b0`) also runs natively (truncate toward zero).
//!
//! Coverage of the arithmetic opcode set, by row:
//!   * `0x4b0940` — fmul ×6 + fdivr ×6 = 12, each landing in one of the six idiom-block slices.
//!   * `0x4b14a0` — fsub ×2 + fabs ×2 = 4, covered by running the full per-axis `fld;fsub;fabs;
//!     fcomp;fnstsw;test;jcc` chain and asserting the binary's emitted-`jcc` outcome (threshold
//!     probe lands on `|d| == eps`).
//!   * `0x4b1600` — fmul ×1 + fdivr ×1 + fadd-st,st ×1 = 3, covered by the idiom+double slice;
//!     plus a threshold probe of the `fcomp [0.2]` and `fcomp [hi]` clamp compares.
//!   * `0x4b3ec0` — fsub-qword ×1 = 1, covered by a copied-byte island (`fld qword [cell]` +
//!     the source `fsub qword [1.0]` bytes + a relocated `call __ftol`), island bytes asserted
//!     == source.
//!
//! Fuzz reads the LCG's HIGH bits.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::spellbook::{
    axis_delta, axis_moved, hi_bound, idiom16, idiom4, index_from_f64, px_to_unit_f32, EPS_BITS,
    K16_BITS, ONE_F64_BITS,
};

// The two runtime UI-scale globals the idiom's helpers read.
const G_L: u64 = 0x832a4c; // 0x41ad70: `fld [0x832a4c]` → st0 = L
const G_M: u64 = 0x832a44; // 0x41ae60: `fld [0x832a44]; fmul [ebp+8]`

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

struct Frame {
    ebp: u64,
    esp: u64,
    esi: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x80;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    let esi = emu.alloc(0x600); // [esi+0x48..0x5c] written by 0x4b0940's stores
    Frame { ebp, esp, esi }
}

// ===========================================================================================
// 0x4b0940 — 6× the px→UI-unit idiom. Each block:
//   call 0x41ad70; fmul[1024]; push ecx; fdivr[k]; fstp[esp]; call 0x41ae60; <store>.
// Blocks 1/2/3 store to [esi+0x54]/[esi+0x48]/[esi+0x4c] (read directly). Blocks 4/5/6 store to
// [esp] (the call-arg) — read at the post-slice ESP. fmul ×6 + fdivr ×6 = 12 ops, all covered.
// ===========================================================================================
#[derive(Clone, Copy)]
enum Read {
    EsiOff(u64), // read [esi+off]
    Esp,         // read [esp] (post-slice ESP)
}
// (begin, until, store-read, k=4? , label)
const BLOCKS: [(u64, u64, Read, bool, u64); 6] = [
    (0x4b09d2, 0x4b09f8, Read::EsiOff(0x54), false, 0x4b09d2), // block 1 → [esi+0x54]
    (0x4b09f8, 0x4b0a15, Read::EsiOff(0x48), false, 0x4b09f8), // block 2 → [esi+0x48]
    (0x4b0a15, 0x4b0a32, Read::EsiOff(0x4c), false, 0x4b0a15), // block 3 → [esi+0x4c]
    (0x4b0a7d, 0x4b0a9b, Read::Esp, false, 0x4b0a7d),          // block 4 → [esp]
    (0x4b0aa2, 0x4b0ac0, Read::Esp, false, 0x4b0aa2),          // block 5 → [esp]
    (0x4b0ac7, 0x4b0ae5, Read::Esp, true, 0x4b0ac7),           // block 6 (k=4) → [esp]
];

#[test]
fn glue_4b0_4b0940_px_unit_idiom() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, begin: u64, until: u64, rd: Read, l: f32, m: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::ECX, 0); // `push ecx` reserves a stack slot — value unused
        emu.write_u32(G_L, l.to_bits());
        emu.write_u32(G_M, m.to_bits());
        emu.run(begin, until);
        match rd {
            Read::EsiOff(o) => emu.read_f32(f.esi + o),
            Read::Esp => {
                let esp = emu.get_reg(RegisterX86::ESP);
                emu.read_f32(esp)
            }
        }
    };

    // deterministic probes: identity scale (L=M=1), the .data defaults (L=1.0, M=0.8), tiny &
    // large scales (the divide makes the inner term huge/small), and a negative scale.
    let probes: [(f32, f32); 7] = [
        (1.0, 1.0),
        (1.0, 0.8), // the .data initialisers
        (0.5, 0.8),
        (2.0, 0.6),
        (1.0e-3, 1.5), // tiny L → huge 1/(L·1024)
        (1.0e3, 0.25), // large L → tiny quotient
        (-1.5, 0.8),   // negative scale (fdivr keeps the sign)
    ];

    let mut total = 0u32;
    for &(begin, until, rd, is_k4, lbl) in &BLOCKS {
        for &(l, m) in &probes {
            let want = if is_k4 { idiom4(l, m) } else { idiom16(l, m) };
            assert_eq!(
                run(&mut emu, begin, until, rd, l, m).to_bits(),
                want.to_bits(),
                "idiom @ {lbl:#x} l={l} m={m}"
            );
            total += 1;
        }
        let mut s = (lbl as u32).wrapping_mul(0x1001);
        for _ in 0..1500 {
            let l = rng(&mut s, 0.05, 4.0);
            let m = rng(&mut s, -2.0, 2.0);
            let want = if is_k4 { idiom4(l, m) } else { idiom16(l, m) };
            assert_eq!(
                run(&mut emu, begin, until, rd, l, m).to_bits(),
                want.to_bits(),
                "idiom @ {lbl:#x} fuzz l={l} m={m}"
            );
            total += 1;
        }
    }
    eprintln!("glue_4b0_4b0940_px_unit_idiom: {total} bit-exact (6 idiom blocks, fmul×6+fdivr×6)");
}

// ===========================================================================================
// 0x4b14a0 — per-axis "did it move?" gate. The owned arith is `fld [new]; fsub [old]; fabs`,
// then `fcomp [eps]; fnstsw; test ah,5; jcc`. We run the FULL chain and assert the binary's
// emitted-jcc OUTCOME (the deterministic post-branch EIP), which carries fsub+fabs+the compare.
//
// Axis X (0x4b150f..): fld [ebp-8]=new; fsub [esi+0x324]=old; fabs; fcomp [eps]; jp 0x4b153f.
//   jp taken (→ 0x4b153f) ⟺ |new-old| >= eps ; fall-through → 0x4b1527.
// Axis Y (0x4b1527..): fld [ebp-4]=new; fsub [esi+0x328]=old; fabs; fcomp [eps]; jnp 0x4b157f.
//   jnp taken (→ 0x4b157f) ⟺ |new-old| < eps ; fall-through → 0x4b153f.
// fsub ×2 + fabs ×2 = 4 ops covered.
// ===========================================================================================
#[test]
fn glue_4b0_4b14a0_move_gate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let eps = f32::from_bits(EPS_BITS);

    // The owned arith is `fld [new]; fsub [old]; fabs; fcomp [eps]`. A `jp`/`jnp` branch outcome
    // can't be policed by `Emu::run` (it forces a single `until` and panics on any other halt), so
    // we run `fld;fsub;fabs;fcomp;fnstsw` to the `test ah,5` and read AX — that carries the owned
    // fsub+fabs+fcomp bit-exactly — then decode the EMITTED `jcc` in-test (the threshold probe).
    let ax_x = |emu: &mut Emu, new: f32, old: f32| -> u16 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::EAX, 0);
        emu.write_u32(f.ebp.wrapping_sub(8), new.to_bits());
        emu.write_u32(f.esi + 0x324, old.to_bits());
        emu.run(0x4b150f, 0x4b1522); // fld;fsub;fabs;fcomp;fnstsw — halt at `test ah,5`
        emu.get_reg(RegisterX86::EAX) as u16
    };
    let ax_y = |emu: &mut Emu, new: f32, old: f32| -> u16 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::EAX, 0);
        emu.write_u32(f.ebp.wrapping_sub(4), new.to_bits()); // [ebp-4]
        emu.write_u32(f.esi + 0x328, old.to_bits());
        emu.run(0x4b1527, 0x4b153a); // fld;fsub;fabs;fcomp;fnstsw — halt at `test ah,5`
        emu.get_reg(RegisterX86::EAX) as u16
    };

    // The binary's branch predicate, recomputed from the AX (ah&5) the binary produced — this is
    // the bit-exact carrier of the owned fsub+fabs+fcomp. `jp` taken ⟺ PF(ah&5)=1 ⟺ ah&5 ∈ {0,5}.
    let jp_taken = |ax: u16| -> bool {
        let r = ((ax >> 8) as u8) & 0x5;
        r.count_ones().is_multiple_of(2) // PF = even parity
    };

    let mut total = 0u32;
    // Both axes run the SAME `fld;fsub;fabs;fcomp [eps];fnstsw` chain (X on the X fields, Y on the
    // Y fields) — only the emitted `jcc` differs downstream (X is `jp`, Y is `jnp`, which is just
    // the complement of the same fcomp flags). We decode each slice's raw fcomp result with the
    // `jp` rule (`jp` taken ⟺ moved) and assert it equals the transcription's `axis_moved`. That
    // diffs the owned fsub+fabs+fcomp on both axes' actual operand sites, bit-exactly.
    let check = |emu: &mut Emu, total: &mut u32, new: f32, old: f32| {
        let want_delta = axis_delta(new, old);
        let want_moved = axis_moved(new, old);
        let ax = ax_x(emu, new, old);
        assert_eq!(
            jp_taken(ax),
            want_moved,
            "axis-X move predicate new={new} old={old} delta={want_delta}"
        );
        let ay = ax_y(emu, new, old);
        assert_eq!(
            jp_taken(ay),
            want_moved,
            "axis-Y move predicate new={new} old={old} delta={want_delta}"
        );
        *total += 2;
    };

    // Threshold probe: land EXACTLY on |new-old| == eps (the fcomp boundary). old=0, new=±eps,
    // and new just above/below eps (the next representable f32 steps).
    let eps_up = f32::from_bits(EPS_BITS + 1);
    let eps_dn = f32::from_bits(EPS_BITS - 1);
    let edge: [(f32, f32); 8] = [
        (eps, 0.0),       // |d| == eps  → moved (>=) true
        (eps_up, 0.0),    // |d| > eps   → true
        (eps_dn, 0.0),    // |d| < eps   → false
        (-eps, 0.0),      // |−eps| == eps → true
        (0.0, 0.0),       // |d| == 0    → false
        (1.0, 1.0),       // equal       → false
        (5.0, 5.0 + eps), // |d| == eps  → true
        (100.0, 100.0),   // large equal → false
    ];
    for &(n, o) in &edge {
        check(&mut emu, &mut total, n, o);
    }
    let mut s = 0x4b14_a000u32;
    for _ in 0..3000 {
        let old = rng(&mut s, -1000.0, 1000.0);
        // bias many cases to live near `old` so the eps boundary is exercised, not just gross moves.
        let jitter = rng(&mut s, -1.0e-5, 1.0e-5);
        let new = old + jitter;
        check(&mut emu, &mut total, new, old);
        let new2 = rng(&mut s, -1000.0, 1000.0); // gross moves too
        check(&mut emu, &mut total, new2, old);
    }
    eprintln!(
        "glue_4b0_4b14a0_move_gate: {total} bit-exact (fsub×2+fabs×2 via fcomp AX + jcc probe)"
    );
}

// ===========================================================================================
// 0x4b1600 — px→UI-unit idiom (k=16), DOUBLED, then clamped to [0.2, hi]. Owned arith:
//   call 0x41ad70; fmul[1024]; push ecx; fdivr[16]; fstp[esp]; call 0x41ae60; fadd st,st;
//   fstp [ebp+0x10].
// Slice 0x4b172c..0x4b1757 (through the fstp[ebp+0x10], halt at the `call [edx+0x1c]` which is
// orchestration). Read [ebp+0x10] = hi-bound f32. fmul×1 + fdivr×1 + fadd-st,st×1 = 3 ops.
// (The downstream `fcomp [0.2]`/`fcomp [hi]` clamp compares are orchestration; a threshold probe
// of them is below for completeness, decoding the emitted `test ah,0x41; jne` / `test ah,5; jp`.)
// ===========================================================================================
#[test]
fn glue_4b0_4b1600_hi_bound() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // The slice interleaves an object-graph read between `call 0x41ae60` and the `fadd/fstp`:
    //   mov eax,[ebx+0x31c]; mov edx,[eax+0x24]   (does not touch the FP value, but must not fault).
    // Point ebx at a scratch object whose +0x31c slot holds a mapped pointer, so the two reads land
    // in mapped scratch. We halt at the `call [edx+0x1c]` (0x4b1757), so edx's target is never used.
    let obj = emu.alloc(0x400); // ebx (the CSimpleFrame); +0x31c read
    let inner = emu.alloc(0x40); // [ebx+0x31c] target; +0x24 read
    emu.write_u32(obj + 0x31c, inner as u32);

    let run = |emu: &mut Emu, l: f32, m: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.set_reg(RegisterX86::EBX, obj);
        emu.write_u32(G_L, l.to_bits());
        emu.write_u32(G_M, m.to_bits());
        // call ad70; fmul; push; fdivr; fstp[esp]; call ae60; fadd st,st; fstp [ebp+0x10].
        emu.run(0x4b172c, 0x4b1757); // halt at `call [edx+0x1c]` (after the fstp [ebp+0x10])
        emu.read_f32(f.ebp + 0x10)
    };

    let probes: [(f32, f32); 7] = [
        (1.0, 1.0),
        (1.0, 0.8),
        (0.5, 0.8),
        (2.0, 0.6),
        (1.0e-3, 1.5),
        (1.0e3, 0.25),
        (-1.5, 0.8),
    ];
    let mut total = 0u32;
    for &(l, m) in &probes {
        assert_eq!(
            run(&mut emu, l, m).to_bits(),
            hi_bound(l, m).to_bits(),
            "hi_bound l={l} m={m}"
        );
        total += 1;
    }
    let mut s = 0x4b16_0000u32;
    for _ in 0..4000 {
        let l = rng(&mut s, 0.05, 4.0);
        let m = rng(&mut s, -2.0, 2.0);
        assert_eq!(
            run(&mut emu, l, m).to_bits(),
            hi_bound(l, m).to_bits(),
            "hi_bound fuzz l={l} m={m}"
        );
        total += 1;
    }
    eprintln!("glue_4b0_4b1600_hi_bound: {total} bit-exact (fmul[1024]+fdivr[16]+fadd st,st)");
}

// ===========================================================================================
// 0x4b3ec0 — Lua arg → (f64)getter − 1.0 → __ftol. The owned `fsub qword [0x8015b8]` reads a
// getter value off st0; the getter (0x6f3620) is object-graph dependent, so we synthesize st0 via
// a COPIED-BYTE ISLAND: `fld qword [cell]` (DD 05 + disp32) + the EXACT source `fsub qword [1.0]`
// bytes (DC 25 b8 15 80 00, byte-verified) + a `call __ftol` (E8 + recomputed rel32). __ftol runs
// natively (truncate toward zero); EAX read back. fsub-qword ×1 covered.
// ===========================================================================================
#[test]
fn glue_4b0_4b3ec0_index_ftol() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let cell = emu.alloc(8); // the f64 getter value loaded by `fld qword [cell]`
    let isl = emu.alloc(0x20);
    let ftol = 0x40a2b0u64;

    // island: fld qword [cell] (DD 05 + disp32) ; fsub qword [0x8015b8] (DC 25 b8 15 80 00) ;
    //         call __ftol (E8 + rel32) ; (halt here, after the call returns)
    let mut isl_code: Vec<u8> = vec![0xdd, 0x05];
    isl_code.extend_from_slice(&(cell as u32).to_le_bytes()); // fld qword [cell]
    let fsub_off = isl_code.len() as u64; // island offset of the fsub
    isl_code.extend_from_slice(&[0xdc, 0x25, 0xb8, 0x15, 0x80, 0x00]); // fsub qword [0x8015b8]
    let call_site = isl + isl_code.len() as u64; // VA of the `call` opcode
    let rel32 = (ftol as i64 - (call_site as i64 + 5)) as i32;
    isl_code.push(0xe8);
    isl_code.extend_from_slice(&rel32.to_le_bytes()); // call __ftol
    let halt = isl + isl_code.len() as u64; // EIP after the call returns
    emu.write(isl, &isl_code);

    // an island is credited only when its FP bytes match the SOURCE bytes.
    assert_eq!(
        &emu.read(isl + fsub_off, 6),
        &emu.read(0x4b3efd, 6),
        "island fsub bytes != source @0x4b3efd"
    );

    // a generous scratch stack for __ftol's own frame (`add esp,-0xc; fnstcw [ebp-2]; fistp [ebp-c]`)
    // plus the `call`'s return-address push.
    let stack = emu.alloc(0x1000);
    let run = |emu: &mut Emu, v: f64| -> i32 {
        let esp = stack + 0xf00;
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::EBP, esp);
        emu.write(cell, &v.to_bits().to_le_bytes());
        emu.run(isl, halt);
        emu.get_reg(RegisterX86::EAX) as i32
    };

    let mut total = 0u32;
    // deterministic incl. truncate-toward-zero boundaries around integers after the −1.0 bias.
    for v in [
        0.0f64,
        1.0,
        2.0,
        2.9999,
        3.0,
        3.0001,
        1.5,
        -0.5,
        0.9999999,
        1.0000001,
        1024.0,
        1025.0,
        -10.0,
        100.5,
        // NaN, the infinities and values past `i32` and `i64`: `__ftol` returns the low dword of
        // its qword, 0 for the indefinite.
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        3.0e9,
        -3.0e9,
        4_294_967_297.0,
        1.0e19,
        1.0e300,
    ] {
        assert_eq!(run(&mut emu, v), index_from_f64(v), "index v={v}");
        total += 1;
    }
    let mut s = 0x4b3e_c000u32;
    for _ in 0..4000 {
        // spread over a wide f64 range, biased so values land near integers (the trunc boundary).
        let base = (next(&mut s) >> 8) as f64 / 4096.0 - 256.0;
        let v = base; // already has a fractional part
        assert_eq!(run(&mut emu, v), index_from_f64(v), "index fuzz v={v}");
        total += 1;
    }
    eprintln!("glue_4b0_4b3ec0_index_ftol: {total} bit-exact (fsub qword [1.0] island + __ftol)");
}

// reference the idiom helper + the two constants the per-block transcriptions are derived from, so a
// future edit that drops one is a loud compile error, not a silent coverage gap.
#[allow(dead_code)]
fn _ref() {
    let _ = (
        px_to_unit_f32 as fn(f32, f32, u32) -> f32,
        K16_BITS,
        ONE_F64_BITS,
    );
}
