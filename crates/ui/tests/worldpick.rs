//! Differential tests of the arithmetic inside the pick and chat-colour functions: the distance
//! and normalise in the pick tracer `TracePick` (`0x480df0`) and its driver `0x480a50`, the
//! milliseconds-to-minutes product of the play-time poll in `0x495590`, and the byte-to-unit colour channels of `LoadChatSettings` (`0x498a60`),
//! each run against `WoW.exe` in the emulator.
//!
//! Coverage of the arithmetic opcode set (load/store/compare/convert/`fild`/`fxch`/`fld st(k)`
//! excluded), counted in each `[addr, true_ret)`:
//!
//! * `0x480a50` [.,0x480c8c ret 0x14]: fsub×9 (6 coord deltas + 3 sqrt deltas), fmul×4 (3 squares +
//!   1 scale), faddp×2, fsqrt×1  = 16
//! * `0x480df0` [.,0x481048 ret 0x10]: block-0 fsub×3/fmul×4/faddp×2/fsqrt×1 (10); blocks A+B each
//!   fsub×3/fsqrt×1/fabs×1/fdivr×1/fmul×3 (9+9) = 28
//! * `0x495590` [.,0x4958dc ret]: fmul×1 (the (i64)·k product) = 1
//! * `0x498a60` [.,0x499a62 tail jmp]: fmul×12 (12 color-channel normalizes, all `fmul [0x8026c8]`)
//!
//! Diff strategy per op:
//! * **dist_scale** (0x480a50 SQRT block ≡ 0x480df0 block 0, byte-identical register-stack arith):
//!   own-address **slice** over 0x480df0 block 0 (`0x480e4a..0x480e78`), covering its 3 fsub deltas,
//!   3 fmul squares, 2 faddp, fsqrt, and the final `fmul [scale]` (10 ops). The 0x480a50 SQRT-block
//!   register ops (3 fmul/2 faddp/fsqrt + the dup/fstp) are **byte-verified == 0x480df0 block 0** and
//!   inherited (clone). 0x480a50's 3 sqrt-delta fsubs are the same `fld dword; fsub dword` operation
//!   the slice diffs.
//! * **coord_delta** (0x480a50's 6 `param_2/param_3 − centre` fsubs): own-address **slice** over the
//!   second delta block (`0x480b3e..0x480b60`, halt at the `call 0x4549a0`), reading the three f32
//!   stores. The first block's three fsub bytes are **byte-verified == the second's** (`d8 65 f0/ec/e8`).
//! * **normalize_block** (0x480df0 blocks A/B): a **copied-byte island** seeding st0 = dist² then the
//!   EXACT source `fsqrt; fld st(0); fabs; fcomp [0x8029d4]; fdivr; (fld; fmul st,st(1))×3` register
//!   ops — `d9 fa / d9 c0 / d9 e1 / d8 1d d4 29 80 00 / d8 c9` **byte-verified == BOTH** source blocks
//!   A and B; the `fdivr`'s opcode+reg-field (`0xd8`, reg 7 = fdivr) is verified == source `0x480f24`/
//!   `0x480fe9` (a memory `fdivr` whose disp differs but whose SWAP direction the reg-field fixes).
//! * **tick_scale** (0x495590, the milliseconds-to-minutes product): own-address **slice** `0x49569b..0x4956aa` (`fild qword; sub esp,8;
//!   fmul [0x804590]; fstp qword`, halt at the `call 0x73fdf5`), reading the pushed f64.
//! * **color_channel** (0x498a60 ×12): a **copied-byte island** `fild dword [in]; fmul dword
//!   [0x8026c8]; fstp qword [out]`, the `fmul` **byte-verified == ALL 12** source sites.
//!
//! Fuzz reads the LCG's HIGH bits. The mandatory `==`-boundary probes
//! sit on the channel-byte domain `0..=255`, the near-zero dist guard (`dist² == eps²`), and unit /
//! sign-of-zero coordinate values.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::worldpick::{
    color_channel, coord_delta, dist_abs, dist_scale, normalize_block, tick_scale, INV255_F32_BITS,
    K_804590_F32_BITS,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rngf(s: &mut u32, lo: f64, hi: f64) -> f64 {
    lo + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * (hi - lo)
}

struct Frame {
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x200);
    let ebp = scratch + 0x100; // room for [ebp-0x100 .. ebp+0x40]
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1000;
    Frame { ebp, esp }
}

// ===========================================================================================
// (1) dist_scale — own-address slice over 0x480df0 block 0 (0x480e4a..0x480e78):
//   fld [esi];  fsub [edi];   fld [esi+4]; fsub [edi+4]; fld [esi+8]; fsub [edi+8]
//   fld st(0); fmul st,st(1); fld st(2); fmul st,st(3); faddp; fld st(3); fmul st,st(4); faddp
//   fsqrt; fstp st(3); fstp st(0); fstp st(0); fmul [ebp-0x4]; fstp [ebp-0x4]
// Covers 10 ops. The 0x480a50 SQRT register ops are byte-verified == this block (clone inheritance).
// ===========================================================================================
#[test]
fn glue_geom_480_480a50_480df0_dist_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // Byte-verify the 0x480a50 SQRT register-stack ops == 0x480df0 block-0's (clone inheritance for
    // 0x480a50's 3 fmul-square / 2 faddp / fsqrt / the dup+fstp register frame).
    let a50 = [
        0x480c62u64,
        0x480c64,
        0x480c66,
        0x480c68,
        0x480c6b,
        0x480c6d,
        0x480c6f,
        0x480c71,
        0x480c73,
        0x480c75,
        0x480c77,
        0x480c79,
    ];
    let df0 = [
        0x480e5au64,
        0x480e5c,
        0x480e5e,
        0x480e60,
        0x480e62,
        0x480e64,
        0x480e66,
        0x480e68,
        0x480e6a,
        0x480e6c,
        0x480e6e,
        0x480e70,
    ];
    for (i, (&a, &b)) in a50.iter().zip(df0.iter()).enumerate() {
        // each register-form op is 2 bytes (d9 c0, d8 c9, de c1, d9 fa, dd db, dd d8 …)
        assert_eq!(
            emu.read(a, 2),
            emu.read(b, 2),
            "0x480a50 SQRT op #{i} @ {a:#x} != 0x480df0 block-0 op @ {b:#x}"
        );
    }
    // Self-gate 0x480a50's three dist_scale delta fsubs + the final fmul: their ARITHMETIC is the
    // same already-diffed op (the `fld dword; fsub dword` delta — see `coord_delta` below; the
    // `fmul dword [scale]` — the 0x480e72 op this very slice runs), but they use ebp-relative
    // addressing distinct from 0x480df0's esi/edi-indirect, so confirm each is byte-for-byte the
    // same operation (opcode d8 + reg-field) rather than relying on prose.
    for src in [0x480c4cu64, 0x480c54, 0x480c5d] {
        let b = emu.read(src, 2);
        assert_eq!(b[0], 0xd8, "0x480a50 delta fsub @{src:#x} opcode");
        assert_eq!(b[1] & 0x38, 0x20, "0x480a50 delta fsub @{src:#x} reg /4");
    }
    let fm = emu.read(0x480c7b, 2);
    assert_eq!(fm[0], 0xd8, "0x480a50 fmul @0x480c7b opcode");
    assert_eq!(fm[1] & 0x38, 0x08, "0x480a50 fmul @0x480c7b reg /1");

    // The three coordinate points live at esi (p1) and edi (p2); scale at [ebp-0x4].
    let p1 = emu.alloc(0x10);
    let p2 = emu.alloc(0x10);
    let run = |emu: &mut Emu, h: [f32; 3], l: [f32; 3], scale: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, p1);
        emu.set_reg(RegisterX86::EDI, p2);
        for k in 0..3 {
            emu.write_u32(p1 + 4 * k as u64, h[k].to_bits());
            emu.write_u32(p2 + 4 * k as u64, l[k].to_bits());
        }
        emu.write_u32(f.ebp - 0x4, scale.to_bits());
        emu.run(0x480e4a, 0x480e78);
        emu.read_f32(f.ebp - 0x4)
    };

    let mut total = 0u32;
    let check = |emu: &mut Emu, total: &mut u32, h: [f32; 3], l: [f32; 3], s: f32| {
        let got = run(emu, h, l, s);
        let want = dist_scale(h[0], l[0], h[1], l[1], h[2], l[2], s);
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "dist_scale h={h:?} l={l:?} s={s}"
        );
        *total += 1;
    };

    // deterministic boundary probes: zero vector, unit axes, sign-of-zero, equal points.
    check(&mut emu, &mut total, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1.0);
    check(&mut emu, &mut total, [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1.0);
    check(&mut emu, &mut total, [3.0, 4.0, 0.0], [0.0, 0.0, 0.0], 1.0); // |v|=5
    check(&mut emu, &mut total, [3.0, 4.0, 0.0], [0.0, 0.0, 0.0], 2.0); // ·2 = 10
    check(
        &mut emu,
        &mut total,
        [-0.0, -0.0, -0.0],
        [0.0, 0.0, 0.0],
        1.0,
    );
    check(&mut emu, &mut total, [5.0, 5.0, 5.0], [5.0, 5.0, 5.0], 7.0); // zero delta
    check(&mut emu, &mut total, [1.0, 1.0, 1.0], [0.0, 0.0, 0.0], 0.5);

    let mut s = 0x480a_5000u32;
    for _ in 0..3000 {
        let h = [
            rngf(&mut s, -500.0, 500.0) as f32,
            rngf(&mut s, -500.0, 500.0) as f32,
            rngf(&mut s, -500.0, 500.0) as f32,
        ];
        let l = [
            rngf(&mut s, -500.0, 500.0) as f32,
            rngf(&mut s, -500.0, 500.0) as f32,
            rngf(&mut s, -500.0, 500.0) as f32,
        ];
        let sc = rngf(&mut s, -10.0, 10.0) as f32;
        check(&mut emu, &mut total, h, l, sc);
    }
    eprintln!(
        "glue_geom_480_480a50_480df0_dist_scale: {total} bit-exact (slice 0x480df0 block0; 0x480a50 SQRT inherited by byte-id)"
    );
}

// ===========================================================================================
// (2) coord_delta — own-address slice over 0x480a50's second delta block (0x480b3e..0x480b60):
//   fld [eax+8]; fsub [ebp-0x10]; fstp [esp+8]
//   fld [eax+4]; fsub [ebp-0x14]; fstp [esp+4]
//   fld [eax];   fsub [ebp-0x18]; fstp [esp]      (halt at the call 0x4549a0)
// Covers the 3 fsubs of block 2; block 1's three fsub bytes are byte-verified == block 2's.
// ===========================================================================================
#[test]
fn glue_geom_480_480a50_coord_delta() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // block 1 fsub bytes == block 2 fsub bytes (same coord_delta op at all 6 sites).
    for (b1, b2) in [
        (0x480b05u64, 0x480b44u64),
        (0x480b18, 0x480b51),
        (0x480b21, 0x480b5a),
    ] {
        assert_eq!(
            emu.read(b1, 3),
            emu.read(b2, 3),
            "0x480a50 coord_delta fsub @ {b1:#x} != @ {b2:#x}"
        );
    }

    let coords = emu.alloc(0x10); // eax -> [eax]/[eax+4]/[eax+8]
    let run = |emu: &mut Emu, a: [f32; 3], c: [f32; 3]| -> [f32; 3] {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EAX, coords);
        for (k, &av) in a.iter().enumerate() {
            emu.write_u32(coords + 4 * k as u64, av.to_bits());
        }
        // source legs: [esp]=[eax]−[ebp-0x18], [esp+4]=[eax+4]−[ebp-0x14], [esp+8]=[eax+8]−[ebp-0x10].
        // pair centre c[k] with coord a[k] (= [eax+4k]) so got[k] == coord_delta(a[k], c[k]).
        emu.write_u32(f.ebp - 0x18, c[0].to_bits()); // [eax]   (k=0) leg
        emu.write_u32(f.ebp - 0x14, c[1].to_bits()); // [eax+4] (k=1) leg
        emu.write_u32(f.ebp - 0x10, c[2].to_bits()); // [eax+8] (k=2) leg
        emu.run(0x480b3e, 0x480b60);
        // after `sub esp,0xc` the three stores land at [esp-0xc .. esp-0x4].
        [
            emu.read_f32(f.esp - 0xc),     // fstp [esp]   = [eax]   − c[0]
            emu.read_f32(f.esp - 0xc + 4), // fstp [esp+4] = [eax+4] − c[1]
            emu.read_f32(f.esp - 0xc + 8), // fstp [esp+8] = [eax+8] − c[2]
        ]
    };

    let mut total = 0u32;
    let check = |emu: &mut Emu, total: &mut u32, a: [f32; 3], c: [f32; 3]| {
        let got = run(emu, a, c);
        let want = [
            coord_delta(a[0], c[0]),
            coord_delta(a[1], c[1]),
            coord_delta(a[2], c[2]),
        ];
        for k in 0..3 {
            assert_eq!(
                got[k].to_bits(),
                want[k].to_bits(),
                "coord_delta leg {k} a={a:?} c={c:?}"
            );
        }
        *total += 3;
    };

    check(&mut emu, &mut total, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    check(&mut emu, &mut total, [1.0, -1.0, 0.5], [0.5, 0.5, 0.5]);
    check(&mut emu, &mut total, [-0.0, 0.0, -0.0], [0.0, -0.0, 0.0]);
    check(&mut emu, &mut total, [1e9, -1e9, 1e-9], [1e9, 1e9, -1e-9]);

    let mut s = 0x480a_5001u32;
    for _ in 0..3000 {
        let a = [
            rngf(&mut s, -1000.0, 1000.0) as f32,
            rngf(&mut s, -1000.0, 1000.0) as f32,
            rngf(&mut s, -1000.0, 1000.0) as f32,
        ];
        let c = [
            rngf(&mut s, -1000.0, 1000.0) as f32,
            rngf(&mut s, -1000.0, 1000.0) as f32,
            rngf(&mut s, -1000.0, 1000.0) as f32,
        ];
        check(&mut emu, &mut total, a, c);
    }
    eprintln!("glue_geom_480_480a50_coord_delta: {total} bit-exact (slice block2; block1 fsubs inherited by byte-id)");
}

// ===========================================================================================
// (3) normalize_block — copied-byte island reproducing 0x480df0 blocks A/B's owned FP exactly:
//   fld qword [dist2]                      (seed st0 = dist²)
//   <d9 fa fsqrt> <d9 c0 fld st(0)> <d9 e1 fabs> <d8 1d d4 29 80 00 fcomp [0x8029d4]>  (== A and B)
//   fdivr dword [scalar]                   (d8 3d <disp32>; reg-field 7 verified == source fdivr)
//   fld dword [cy]; <d8 c9 fmul st,st(1)>; fstp dword [o0]
//   fld dword [cx]; <d8 c9 fmul st,st(1)>; fstp dword [o1]
//   fld dword [cz]; <d8 c9 fmul st,st(1)>; fstp dword [o2]
// Covers fsqrt, fabs, fdivr (SWAP), fmul×3 — the owned ops of BOTH normalize blocks.
// ===========================================================================================
#[test]
fn glue_geom_480_480df0_normalize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let dist2 = emu.alloc(8);
    let scalar = emu.alloc(4);
    let cy = emu.alloc(4);
    let cx = emu.alloc(4);
    let cz = emu.alloc(4);
    let o0 = emu.alloc(4);
    let o1 = emu.alloc(4);
    let o2 = emu.alloc(4);
    let isl = emu.alloc(0x80);

    // assemble the island
    let mut code: Vec<u8> = Vec::new();
    // fld qword [dist2] = DD 05 <disp32>
    code.extend_from_slice(&[0xdd, 0x05]);
    code.extend_from_slice(&(dist2 as u32).to_le_bytes());
    // <source fsqrt; fld st(0); fabs; fcomp [0x8029d4]> verbatim (byte-verified below)
    let fsqrt_off = code.len();
    code.extend_from_slice(&[0xd9, 0xfa]); // fsqrt
    let flds0_off = code.len();
    code.extend_from_slice(&[0xd9, 0xc0]); // fld st(0)
    let fabs_off = code.len();
    code.extend_from_slice(&[0xd9, 0xe1]); // fabs
    let fcomp_off = code.len();
    code.extend_from_slice(&[0xd8, 0x1d, 0xd4, 0x29, 0x80, 0x00]); // fcomp [0x8029d4]
                                                                   // fdivr dword [scalar] = D8 3D <disp32>  (mod 00, reg 7 = fdivr, rm 5 = disp32)
    let fdivr_off = code.len();
    code.extend_from_slice(&[0xd8, 0x3d]);
    code.extend_from_slice(&(scalar as u32).to_le_bytes());
    // (fld dword [c]; fmul st,st(1); fstp dword [o]) × 3
    let mut fmul_offs = [0usize; 3];
    for (j, (c, o)) in [(cy, o0), (cx, o1), (cz, o2)].into_iter().enumerate() {
        code.extend_from_slice(&[0xd9, 0x05]); // fld dword [c]
        code.extend_from_slice(&(c as u32).to_le_bytes());
        fmul_offs[j] = code.len();
        code.extend_from_slice(&[0xd8, 0xc9]); // fmul st,st(1)
        code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [o]
        code.extend_from_slice(&(o as u32).to_le_bytes());
    }
    code.extend_from_slice(&[0xdd, 0xd8]); // fstp st(0) — discard the ratio (matches source tail)
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);

    // byte-verify the verbatim register ops == BOTH source blocks A and B.
    for (off, n, (sa, sb)) in [
        (fsqrt_off, 2usize, (0x480effu64, 0x480fc1u64)),
        (flds0_off, 2, (0x480f13, 0x480fd8)),
        (fabs_off, 2, (0x480f15, 0x480fda)),
        (fcomp_off, 6, (0x480f17, 0x480fdc)),
        (fmul_offs[0], 2, (0x480f30, 0x480ff5)),
        (fmul_offs[1], 2, (0x480f39, 0x480ffe)),
        (fmul_offs[2], 2, (0x480f42, 0x481007)),
    ] {
        let isl_bytes = emu.read(isl + off as u64, n);
        assert_eq!(
            isl_bytes,
            emu.read(sa, n),
            "normalize island op != block A @ {sa:#x}"
        );
        assert_eq!(
            isl_bytes,
            emu.read(sb, n),
            "normalize island op != block B @ {sb:#x}"
        );
    }
    // Self-gate block-A's three delta fsubs (0x480ecb/ee4/eed) — the `fld dword; fsub dword` delta op
    // diffed by `coord_delta` (the island feeds the post-delta dist² directly); confirm each is that
    // same fsub-dword op (opcode d8 + reg-field /4) rather than relying on prose.
    for src in [0x480ecbu64, 0x480ee4, 0x480eed] {
        let b = emu.read(src, 2);
        assert_eq!(b[0], 0xd8, "block-A delta fsub @{src:#x} opcode");
        assert_eq!(b[1] & 0x38, 0x20, "block-A delta fsub @{src:#x} reg /4");
    }
    // verify the island fdivr's opcode + reg-field (SWAP direction) == both source fdivrs.
    let isl_fdivr = emu.read(isl + fdivr_off as u64, 2);
    for src in [0x480f24u64, 0x480fe9] {
        let s = emu.read(src, 2);
        assert_eq!(isl_fdivr[0], 0xd8, "island fdivr opcode");
        assert_eq!(s[0], 0xd8, "source fdivr opcode @ {src:#x}");
        assert_eq!(
            (isl_fdivr[1] >> 3) & 7,
            (s[1] >> 3) & 7,
            "island fdivr reg-field != source @ {src:#x} (SWAP direction)"
        );
        assert_eq!(
            (s[1] >> 3) & 7,
            7,
            "source @ {src:#x} reg-field is fdivr (7)"
        );
    }

    let run = |emu: &mut Emu, d2: f64, sc: f32, c: [f32; 3]| -> [f32; 3] {
        emu.write(dist2, &d2.to_bits().to_le_bytes());
        emu.write_u32(scalar, sc.to_bits());
        emu.write_u32(cy, c[0].to_bits());
        emu.write_u32(cx, c[1].to_bits());
        emu.write_u32(cz, c[2].to_bits());
        emu.run(isl, isl_end);
        [emu.read_f32(o0), emu.read_f32(o1), emu.read_f32(o2)]
    };

    let mut total = 0u32;
    let check = |emu: &mut Emu, total: &mut u32, d2: f64, sc: f32, c: [f32; 3]| {
        let got = run(emu, d2, sc, c);
        let want = normalize_block(d2, sc, c[0], c[1], c[2]);
        assert_eq!(
            (got[0].to_bits(), got[1].to_bits(), got[2].to_bits()),
            (want.0.to_bits(), want.1.to_bits(), want.2.to_bits()),
            "normalize_block d2={d2} sc={sc} c={c:?}"
        );
        *total += 1;
        // also exercise dist_abs at this distance (the owned fabs of the guard).
        let dist = d2.sqrt();
        assert_eq!(
            dist_abs(dist).to_bits(),
            dist.abs().to_bits(),
            "dist_abs dist={dist}"
        );
        *total += 1;
    };

    // deterministic probes: unit dist, the near-zero eps boundary, sign-of-zero components.
    let eps = f64::from(f32::from_bits(0x3480_0000)); // 0x8029d4
    check(&mut emu, &mut total, 1.0, 1.0, [1.0, 2.0, 3.0]);
    check(&mut emu, &mut total, 25.0, 5.0, [3.0, 4.0, 0.0]); // dist=5
    check(&mut emu, &mut total, eps * eps, 1.0, [1.0, 0.0, -0.0]); // |dist|==eps boundary
    check(
        &mut emu,
        &mut total,
        (eps * eps) * 0.25,
        1.0,
        [1.0, 1.0, 1.0],
    ); // dist=eps/2 (< eps)
    check(&mut emu, &mut total, 4.0, -2.0, [-1.0, 0.0, 2.0]); // negative scalar
    check(&mut emu, &mut total, 1.0e8, 1.0, [-0.0, 0.0, -0.0]);

    let mut s = 0x480d_f000u32;
    for _ in 0..3000 {
        let d2 = rngf(&mut s, 1e-6, 1e6);
        let sc = rngf(&mut s, -50.0, 50.0) as f32;
        let c = [
            rngf(&mut s, -100.0, 100.0) as f32,
            rngf(&mut s, -100.0, 100.0) as f32,
            rngf(&mut s, -100.0, 100.0) as f32,
        ];
        check(&mut emu, &mut total, d2, sc, c);
    }
    eprintln!(
        "glue_geom_480_480df0_normalize: {total} bit-exact (island: fsqrt;fabs;fdivr(SWAP);fmul×3 == blocks A&B)"
    );
}

// ===========================================================================================
// (4) tick_scale — the play-time poll's minutes product, own-address slice over 0x495590 (0x49569b..0x4956aa):
//   fild qword [ebp-0x8]; sub esp,8; fmul dword [0x804590]; fstp qword [esp]   (halt at the call).
// fmul @0x4956a1 (×1). Feed the i64 at [ebp-0x8]; the slice's `sub esp,8` runs before the fstp, so
// the f64 lands at [esp_initial - 8].
// ===========================================================================================
#[test]
fn glue_geom_480_495590_tick_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // verify the multiplier constant at 0x804590 matches the transcription.
    assert_eq!(
        u32::from_le_bytes(emu.read(0x804590, 4).try_into().unwrap()),
        K_804590_F32_BITS,
        "0x804590 multiplier constant"
    );

    let run = |emu: &mut Emu, n: i64| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write(f.ebp - 0x8, &n.to_le_bytes()); // qword [ebp-0x8]:[ebp-0x4]
        emu.run(0x49569b, 0x4956aa);
        let b = emu.read(f.esp - 0x8, 8); // fstp qword [esp] after the `sub esp,8`
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    let check = |emu: &mut Emu, total: &mut u32, n: i64| {
        assert_eq!(
            run(emu, n).to_bits(),
            tick_scale(n).to_bits(),
            "tick_scale n={n}"
        );
        *total += 1;
    };

    // deterministic: zero, negatives (signed fild), and tick-magnitude boundaries.
    for &n in &[
        0i64,
        1,
        -1,
        60_000,
        -60_000,
        300_000,
        -300_000,
        15_000,
        1_000_000,
        -1_000_000,
        i64::from(i32::MAX),
        i64::from(i32::MIN),
        0x0001_0000_0000, // beyond 32-bit (qword fild)
        -0x0001_0000_0000,
    ] {
        check(&mut emu, &mut total, n);
    }
    let mut s = 0x4955_9000u32;
    for _ in 0..3000 {
        // a signed 48-bit-ish tick delta (well within i53, where fild qword is exact and == n as f64)
        let hi = (next(&mut s) >> 8) as i64;
        let lo = next(&mut s) as i64;
        let n = ((hi << 16) ^ lo) - 0x4000_0000_0000;
        check(&mut emu, &mut total, n);
    }
    eprintln!("glue_geom_480_495590_tick_scale: {total} bit-exact (slice 0x49569b: fild qword; fmul [0x804590]; fstp qword)");
}

// ===========================================================================================
// (5) color_channel — copied-byte island `fild dword [in]; fmul dword [0x8026c8]; fstp qword [out]`.
// The `fmul dword [0x8026c8]` (D8 0D C8 26 80 00) is byte-verified == ALL 12 source sites in 0x498a60.
// Covers all 12 fmul ops. (The fild/fstp ride along; the input domain is the channel byte 0..=255.)
// ===========================================================================================
#[test]
fn glue_geom_480_498a60_color_channel() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // verify the multiplier constant at 0x8026c8 matches the transcription.
    assert_eq!(
        u32::from_le_bytes(emu.read(0x8026c8, 4).try_into().unwrap()),
        INV255_F32_BITS,
        "0x8026c8 1/255 f32 constant"
    );

    let in_cell = emu.alloc(4);
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    let mut code: Vec<u8> = Vec::new();
    // fild dword [in_cell] = DB 05 <disp32>
    code.extend_from_slice(&[0xdb, 0x05]);
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    // copied source fmul dword [0x8026c8] = D8 0D C8 26 80 00
    let fmul_off = code.len();
    code.extend_from_slice(&[0xd8, 0x0d, 0xc8, 0x26, 0x80, 0x00]);
    // fstp qword [out_cell] = DD 1D <disp32>
    code.extend_from_slice(&[0xdd, 0x1d]);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);

    // byte-verify the island's fmul == EVERY source site (else the island proves nothing).
    let isl_fmul = emu.read(isl + fmul_off as u64, 6);
    for src in [
        0x4996eau64,
        0x4996fa,
        0x499707,
        0x499762,
        0x499772,
        0x49977f,
        0x499962,
        0x499972,
        0x49997f,
        0x4999d2,
        0x4999e2,
        0x4999ef,
    ] {
        assert_eq!(
            isl_fmul,
            emu.read(src, 6),
            "color_channel island fmul != source @ {src:#x}"
        );
    }

    let run = |emu: &mut Emu, byte: u8| -> f64 {
        emu.write_u32(in_cell, u32::from(byte)); // movzx-style zero-extended channel byte
        emu.run(isl, isl_end);
        let b = emu.read(out_cell, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    for byte in 0u32..=255 {
        let got = run(&mut emu, byte as u8);
        let want = color_channel(byte as u8);
        assert_eq!(got.to_bits(), want.to_bits(), "color_channel byte={byte}");
        total += 1;
    }
    eprintln!("glue_geom_480_498a60_color_channel: {total} bit-exact (island: fild dword; fmul [1/255 f32]; fstp qword == all 12 sites)");
}
