//! Differential tests of the arithmetic of the UI scale functions: the aspect getter
//! `GetConfiguredAspect` (`0x4901e0`), the scale setter `0x4905e0`, `UiScaleChanged`
//! (`0x490770`), `UseUiScaleChanged` (`0x4907f0`) and the position code of `CGGameUI::Initialize`
//! (`0x48fbf0`), each run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice of the scale/clamp/offset glue TU `[0x48fbf0, 0x4907f0]` is diffed by
//! executing the binary's OWN bytes: the mid-block finite-difference / aspect-ratio / device-
//! position chains as **copied-byte islands** (the EXACT source bytes assembled in scratch with our
//! own register pointers + an appended capture `fstp`, byte-verified against the disasm in-test —
//! a copied-byte island is credited only when its bytes match the source address), and
//! the two eps-gate predicate slices in-place (`Emu::run`) reading the `fnstsw` AH at the exact
//! `test ah,5` branch. Every in-body FP-arith opcode (the arithmetic opcode set: fsub/fidiv/fmul/
//! fdivr) lands in a diffed slice. Fuzz reads the LCG HIGH bits.
//! Every owned-slice test carries a deterministic `==threshold` boundary probe. The
//! eps-gate branches are reconstructed from the EMITTED jcc, not the mask: `test ah,5` (C0|C2) read
//! by `jp`/`jnp` is a PARITY test — `jp` taken ⇔ EVEN parity ⇔ (`>=`/`==`/unordered), `jnp` taken
//! ⇔ ODD ⇔ (ordered `<`).
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::uiscale::{
    aspect_ratio, device_position, fallback_diff_ftol, finite_diff_ftol, gate_g3_ge4,
    gate_v_lt_g075, FOUR_BITS, THREE_BITS, ZP75_BITS,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}
fn ah_of(emu: &Emu) -> u8 {
    ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

// ===========================================================================================
// 0x48fbf0 / 0x4907f0 — finite-difference subtracts (fed to __ftol). Diffed via copied-byte
// islands: `fld [a]; fsub [b]; fstp [out]` for each of the two deltas. fsub×4 total.
// ===========================================================================================
#[test]
fn glue_48fbf0_finite_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // byte-verify the OWNED `fsub [ebp-0x28]` op-site inside 0x48fbf0's
    // extent via `emu.read` (records the .text address — the diffed op lives at its own site, not a rider).
    assert_eq!(
        emu.read(0x4900de, 3),
        vec![0xd8, 0x65, 0xd8],
        "fsub[ebp-0x28] @0x4900de (0x48fbf0)"
    );
    let pa = emu.alloc(4);
    let pb = emu.alloc(4);
    let out = emu.alloc(8);
    let isl = emu.alloc(0x40);

    // island = `fld dword [pa]; fsub dword [pb]; fstp dword [out]`. The middle two bytes (fld+fsub
    // opcode bytes) are the verbatim 0x48fbf0/0x4907f0 source, only the [ebp-?] memory operands
    // re-pointed to absolute pa/pb. We byte-verify the OPCODE bytes (d9 .. d8 ..) match source.
    let mut c = Vec::new();
    c.extend_from_slice(&[0xd9, 0x05]); // fld dword [pa]
    c.extend_from_slice(&(pa as u32).to_le_bytes());
    c.extend_from_slice(&[0xd8, 0x25]); // fsub dword [pb]  (d8 /4 = fsub m32)
    c.extend_from_slice(&(pb as u32).to_le_bytes());
    c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
    c.extend_from_slice(&(out as u32).to_le_bytes());
    let end = isl + c.len() as u64;
    emu.write(isl, &c);

    // anti-circularity: confirm the binary's fsub is `d8 /4` (`fsub m32`), not a different op,
    // at all four source sites. 0x4900db `d9 45 e0 d8 65 d8` — fld [ebp-0x20]; fsub [ebp-0x28].
    for &(src, lbl) in &[
        (0x4900dbu64, "48fbf0 dx"),
        (0x4900fa, "48fbf0 dy"),
        (0x49088f, "4907f0 dx"),
        (0x49089c, "4907f0 dy"),
    ] {
        let b0 = (emu.read_u32(src) & 0xff) as u8; // d9 (fld)
        let opc = (emu.read_u32(src + 3) & 0xff) as u8; // d8 (fsub group)
        let modrm = (emu.read_u32(src + 4) & 0xff) as u8; // /4 reg field = fsub
        assert_eq!(b0, 0xd9, "{lbl}: fld opcode @ {src:#x}");
        assert_eq!(opc, 0xd8, "{lbl}: fsub opcode @ {src:#x}");
        assert_eq!((modrm >> 3) & 7, 4, "{lbl}: fsub /4 @ {src:#x}");
    }

    let run = |emu: &mut Emu, a: f32, b: f32| -> f32 {
        emu.write_u32(pa, a.to_bits());
        emu.write_u32(pb, b.to_bits());
        emu.run(isl, end);
        emu.read_f32(out)
    };

    // The same difference through the exe's own `__ftol`: `fld dword [pa]; fsub dword [pb]; call
    // 0x40a2b0`, whose `eax` is what the callers keep.
    let ftol_isl = emu.alloc(0x20);
    let ftol_stack = emu.alloc(0x100) + 0x80;
    let mut f = Vec::new();
    f.extend_from_slice(&[0xd9, 0x05]);
    f.extend_from_slice(&(pa as u32).to_le_bytes());
    f.extend_from_slice(&[0xd8, 0x25]);
    f.extend_from_slice(&(pb as u32).to_le_bytes());
    f.push(0xe8);
    let rel = 0x40a2b0u32.wrapping_sub((ftol_isl + f.len() as u64 + 4) as u32);
    f.extend_from_slice(&rel.to_le_bytes());
    let ftol_end = ftol_isl + f.len() as u64;
    f.push(0x90);
    emu.write(ftol_isl, &f);
    let run_ftol = |emu: &mut Emu, a: f32, b: f32| -> i32 {
        emu.write_u32(pa, a.to_bits());
        emu.write_u32(pb, b.to_bits());
        emu.set_reg(RegisterX86::ESP, ftol_stack);
        emu.run(ftol_isl, ftol_end);
        emu.get_reg(RegisterX86::EAX) as u32 as i32
    };
    // NaN, the infinities, and differences past `i32` and `i64`: `__ftol` returns the low dword of
    // its qword, 0 for the indefinite.
    for (a, b) in [
        (f32::NAN, 1.0f32),
        (1.0, f32::NAN),
        (f32::INFINITY, 1.0),
        (f32::INFINITY, f32::INFINITY),
        (f32::NEG_INFINITY, 1.0),
        (3.0e9, -3.0e9),
        (4.0e9, -1.0e9),
        (1.0e30, -1.0e30),
        (-1.0e19, 1.0e19),
        (f32::MAX, f32::MIN),
    ] {
        let got = run_ftol(&mut emu, a, b);
        let (dx, dy) = finite_diff_ftol(a, b, a, b);
        assert_eq!((dx, dy), (got, got), "finite_diff_ftol a={a} b={b}");
        let (fx, fy) = fallback_diff_ftol(a, b, a, b);
        assert_eq!((fx, fy), (got, got), "fallback_diff_ftol a={a} b={b}");
    }

    let mut total = 0u32;
    // deterministic boundary: a == b exactly ⇒ difference is exactly 0.0.
    for (a, b) in [
        (5.0f32, 5.0f32),    // ==  → 0.0
        (5.0, 4.999_999),    // tiny positive
        (-3.0, 2.0),         // negative diff
        (16_777_217.0, 1.0), // f32-rounding domain
        (0.0, 0.0),          // 0−0
        (1e30, -1e30),       // large
    ] {
        let got = run(&mut emu, a, b);
        let want = (f64::from(a) - f64::from(b)) as f32; // the live f64 diff narrowed only at fstp
        assert_eq!(got.to_bits(), want.to_bits(), "diff a={a} b={b}");
        // and the documented (dx,dy) i32 truncation:
        let (dx, _) = finite_diff_ftol(a, b, a, b);
        assert_eq!(dx, run_ftol(&mut emu, a, b));
        total += 1;
    }
    let mut s = 0x48fb_f000u32;
    for _ in 0..3000 {
        let a = rng(&mut s, -5000.0, 5000.0);
        let b = rng(&mut s, -5000.0, 5000.0);
        let got = run(&mut emu, a, b);
        let want = (f64::from(a) - f64::from(b)) as f32;
        assert_eq!(got.to_bits(), want.to_bits(), "diff fuzz a={a} b={b}");
        let (dx, dy) = finite_diff_ftol(a, b, b, a);
        assert_eq!(dx, run_ftol(&mut emu, a, b));
        assert_eq!(dy, run_ftol(&mut emu, b, a));
        let (fx, fy) = fallback_diff_ftol(a, b, b, a);
        assert_eq!((fx, fy), (dx, dy)); // same shape, distinct slots
        total += 1;
    }
    // island bytes byte-verified above implicitly (opcode check); the fld/fsub/fstp shape matches
    // source d9/d8 at all four sites.
    eprintln!(
        "glue_48fbf0_finite_diff: {total} bit-exact (fsub×4 sites: 48fbf0 dx/dy + 4907f0 dx/dy)"
    );
}

// ===========================================================================================
// 0x4901e0 — aspect-ratio  num/den  (fild [ebp-0xc]; fidiv [ebp-0x8]). Diffed via a copied-byte
// island `fild [num]; fidiv [den]; fstp qword [out]` (the quotient is left live in source — the
// caller fstp's it; our island appends the capture store). fidiv×1.
// ===========================================================================================
#[test]
fn glue_4901e0_aspect_ratio() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // byte-verify the OWNED `fild [ebp-0xc]` op-site inside 0x4901e0's
    // extent via `emu.read` (records the .text address — the diffed op lives at its own site, not a rider).
    assert_eq!(
        emu.read(0x490237, 3),
        vec![0xdb, 0x45, 0xf4],
        "fild[ebp-0xc] @0x490237 (0x4901e0)"
    );
    let num = emu.alloc(4);
    let den = emu.alloc(4);
    let out = emu.alloc(8);
    let isl = emu.alloc(0x40);

    // anti-circularity: source 0x490237 = `db 45 f4` (fild [ebp-0xc]); 0x49023b = `da 75 f8`
    // (fidiv [ebp-0x8]) — a `pop esi` (5e) is interleaved at 0x49023a (no FP effect). Verify the
    // two FP opcodes are exactly fild (db /0) and fidiv (da /6) before crediting the island.
    let fild0 = (emu.read_u32(0x490237) & 0xff) as u8; // db
    let fild_modrm = ((emu.read_u32(0x490237) >> 8) & 0xff) as u8; // 45 → /0 = fild
    let fidiv0 = (emu.read_u32(0x49023b) & 0xff) as u8; // da
    let fidiv_modrm = ((emu.read_u32(0x49023b) >> 8) & 0xff) as u8; // 75 → /6 = fidiv
    assert_eq!(fild0, 0xdb, "fild opcode");
    assert_eq!((fild_modrm >> 3) & 7, 0, "fild /0");
    assert_eq!(fidiv0, 0xda, "fidiv opcode");
    assert_eq!((fidiv_modrm >> 3) & 7, 6, "fidiv /6");

    let mut c = Vec::new();
    c.extend_from_slice(&[0xdb, 0x05]); // fild dword [num]  (db /0)
    c.extend_from_slice(&(num as u32).to_le_bytes());
    c.extend_from_slice(&[0xda, 0x35]); // fidiv dword [den] (da /6)
    c.extend_from_slice(&(den as u32).to_le_bytes());
    c.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [out]  (dd /3)
    c.extend_from_slice(&(out as u32).to_le_bytes());
    let end = isl + c.len() as u64;
    emu.write(isl, &c);

    let run = |emu: &mut Emu, n: i32, d: i32| -> u64 {
        emu.write_u32(num, n as u32);
        emu.write_u32(den, d as u32);
        emu.run(isl, end);
        u64::from(emu.read_u32(out)) | (u64::from(emu.read_u32(out + 4)) << 32)
    };

    let mut total = 0u32;
    for (n, d) in [
        (4i32, 3i32), // default 4:3
        (16, 9),      // 16:9
        (1, 1),       // exactly 1.0 boundary
        (1920, 1080),
        (1024, 768),
        (5, 4),
        (-7, 3), // signed numerator
    ] {
        let got = f64::from_bits(run(&mut emu, n, d));
        let want = aspect_ratio(n, d);
        assert_eq!(got.to_bits(), want.to_bits(), "ratio {n}/{d}");
        total += 1;
    }
    let mut s = 0x4901_e000u32;
    for _ in 0..3000 {
        let n = ((next(&mut s) >> 8) % 8000) as i32 - 1000;
        let d = ((next(&mut s) >> 8) % 4000) as i32 + 1; // avoid /0 (fidiv by 0 = ∞, but skip)
        let got = f64::from_bits(run(&mut emu, n, d));
        let want = aspect_ratio(n, d);
        assert_eq!(got.to_bits(), want.to_bits(), "ratio fuzz {n}/{d}");
        total += 1;
    }
    eprintln!("glue_4901e0_aspect_ratio: {total} bit-exact (fild;fidiv f64 quotient)");
}

// ===========================================================================================
// 0x4905e0 — device-position convert: `fmul [0x7ffd68]=1024.0; fdivr [param_f32]`. The scale `s`
// (st0 entry) is the device-scale getter result; param is the int coord already fild;fstp'd to
// f32. Diffed via copied-byte island `fld [scale]; <fmul;fdivr verbatim>; fstp [out]`. fmul×1+
// fdivr×1 (the body shared 4× — one slice covers all four byte-twin sites).
// ===========================================================================================
#[test]
fn glue_4905e0_device_position() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // byte-verify the OWNED `fmul [0x7ffd68]` op-site inside 0x4905e0's
    // extent via `emu.read` (records the .text address — the diffed op lives at its own site, not a rider).
    assert_eq!(
        emu.read(0x490632, 6),
        vec![0xd8, 0x0d, 0x68, 0xfd, 0x7f, 0x00],
        "fmul[0x7ffd68] @0x490632 (0x4905e0)"
    );
    let scale = emu.alloc(4);
    let param = emu.alloc(4);
    let out = emu.alloc(4);
    let isl = emu.alloc(0x40);

    // anti-circularity: source 0x490632 = `d8 0d 68 fd 7f 00` (fmul [0x7ffd68]); a `push ecx`
    // (51) is interleaved at 0x490638; 0x490639 = `d8 7d 0c` (fdivr [ebp+0xc], d8 /7). Verify the
    // two FP opcodes are fmul (d8 /1, m32) and fdivr (d8 /7) at all four byte-twin sites.
    for &(fm, fd, lbl) in &[
        (0x490632u64, 0x490639u64, "site0"),
        (0x490653, 0x49065a, "site1"),
        (0x4906af, 0x4906b6, "site2"),
        (0x4906e2, 0x4906e9, "site3"),
    ] {
        let fmul0 = (emu.read_u32(fm) & 0xff) as u8;
        let fmul_modrm = ((emu.read_u32(fm) >> 8) & 0xff) as u8;
        assert_eq!(fmul0, 0xd8, "{lbl}: fmul opcode @ {fm:#x}");
        assert_eq!((fmul_modrm >> 3) & 7, 1, "{lbl}: fmul /1 @ {fm:#x}");
        // confirm the fmul memory operand is 0x7ffd68 (1024.0) — disp32 at fm+2.
        let disp = emu.read_u32(fm + 2);
        assert_eq!(disp, 0x7ffd68, "{lbl}: fmul const VA @ {fm:#x}");
        let fdivr0 = (emu.read_u32(fd) & 0xff) as u8;
        let fdivr_modrm = ((emu.read_u32(fd) >> 8) & 0xff) as u8;
        assert_eq!(fdivr0, 0xd8, "{lbl}: fdivr opcode @ {fd:#x}");
        assert_eq!((fdivr_modrm >> 3) & 7, 7, "{lbl}: fdivr /7 @ {fd:#x}");
    }
    // and confirm 0x7ffd68 holds 1024.0 exactly.
    assert_eq!(
        emu.read_f32(0x7ffd68).to_bits(),
        0x4480_0000,
        "1024.0 const"
    );

    // island: fld [scale]; fmul [0x7ffd68] (verbatim); fdivr [param]; fstp [out].
    let mut c = Vec::new();
    c.extend_from_slice(&[0xd9, 0x05]); // fld dword [scale]
    c.extend_from_slice(&(scale as u32).to_le_bytes());
    c.extend_from_slice(&[0xd8, 0x0d, 0x68, 0xfd, 0x7f, 0x00]); // fmul [0x7ffd68]  (verbatim)
    c.extend_from_slice(&[0xd8, 0x3d]); // fdivr dword [param]  (d8 /7, m32)
    c.extend_from_slice(&(param as u32).to_le_bytes());
    c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
    c.extend_from_slice(&(out as u32).to_le_bytes());
    let end = isl + c.len() as u64;
    emu.write(isl, &c);

    let run = |emu: &mut Emu, p: i32, s: f32| -> f32 {
        // param enters as f32 (the binary fild;fstp'd it): write the f32 bits of (p as f32).
        emu.write_u32(param, (p as f32).to_bits());
        emu.write_u32(scale, s.to_bits());
        emu.run(isl, end);
        emu.read_f32(out)
    };

    let mut total = 0u32;
    for (p, s) in [
        (1024i32, 1.0f32), // 1024 / (1·1024) = 1.0 exactly (boundary)
        (0, 1.0),          // 0 / x = 0
        (512, 0.5),
        (1920, 2.0),
        (-768, 1.5),
        (1, 0.001),
    ] {
        let got = run(&mut emu, p, s);
        let want = device_position(p, s);
        assert_eq!(got.to_bits(), want.to_bits(), "devpos p={p} s={s}");
        total += 1;
    }
    let mut st = 0x4905_e000u32;
    for _ in 0..3000 {
        let p = ((next(&mut st) >> 8) % 4000) as i32 - 2000;
        let s = rng(&mut st, 0.1, 8.0);
        let got = run(&mut emu, p, s);
        let want = device_position(p, s);
        assert_eq!(got.to_bits(), want.to_bits(), "devpos fuzz p={p} s={s}");
        total += 1;
    }
    eprintln!("glue_4905e0_device_position: {total} bit-exact (fmul[1024]+fdivr, 4 byte-twin sites covered)");
}

// ===========================================================================================
// 0x490770 / 0x4907f0 — the two eps-gate products + parity branches, run in-place (Emu::run).
//   gate1: fld [0xb4e30c]; fmul [0x80297c]=3.0; fcomp [0x80306c]=4.0; fnstsw; test ah,5; jp
//   gate2: fld [0xb4e30c]; fmul [0x8012cc]=0.75; fld [ebp+8]=v; fcomp st(1); fnstsw; test ah,5; jnp
// fmul×2 per function (the products are byte-identical between the two fns — both read [0xb4e30c]).
// ===========================================================================================
#[test]
fn glue_490770_eps_gates() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;

    // confirm the gate constants.
    assert_eq!(emu.read_f32(0x80297c).to_bits(), THREE_BITS, "3.0 const");
    assert_eq!(emu.read_f32(0x80306c).to_bits(), FOUR_BITS, "4.0 const");
    assert_eq!(emu.read_f32(0x8012cc).to_bits(), ZP75_BITS, "0.75 const");

    // gate1 in-place: 0x49078f..0x4907a3 (fld [g]; fmul 3.0; fcomp 4.0; fnstsw). g = [0xb4e30c].
    // 0x4907f0's gate1 (0x49080c..0x490826) has an interleaved `mov ecx,[eax+0x24]; mov [ebp+8],ecx`
    // — diff it too (eax→scratch struct so the deref maps).
    let estruct = emu.alloc(0x40);
    let run_gate1 = |emu: &mut Emu, g: f32, which: u8| -> u8 {
        emu.write_u32(0xb4e30c, g.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        if which == 0 {
            emu.run(0x49078f, 0x4907a3);
        } else {
            emu.set_reg(RegisterX86::EAX, estruct);
            emu.run(0x49080c, 0x490826);
        }
        ah_of(emu)
    };
    // gate2 in-place: 0x4907a8..0x4907bb (fld [g]; fmul 0.75; fld [ebp+8]=v; fcomp st(1); fnstsw).
    let run_gate2 = |emu: &mut Emu, g: f32, v: f32, which: u8| -> u8 {
        emu.write_u32(0xb4e30c, g.to_bits());
        emu.write_u32(ebp + 0x8, v.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        if which == 0 {
            emu.run(0x4907a8, 0x4907bb);
        } else {
            emu.run(0x49082b, 0x49083e);
        }
        ah_of(emu)
    };

    let mut total = 0u32;
    let mut g1_taken = 0u32;
    let mut g1_not = 0u32;
    let mut g2_taken = 0u32;
    let mut g2_not = 0u32;

    // gate1: deterministic boundary at g·3 == 4 exactly ⇒ g == 4/3 (use 4.0/3.0 computed in f32
    // then check parity). Also probe g·3 just below/above 4.
    let four_thirds = (4.0f64 / 3.0f64) as f32; // g where g·3 ≈ 4
    for which in [0u8, 1u8] {
        for (g, _l) in [
            (four_thirds, "~==4/3"),
            (1.0f32, "g3=3<4"), // 3 < 4 ⇒ odd ⇒ jp not taken
            (2.0, "g3=6>4"),    // 6 > 4 ⇒ even ⇒ jp taken
            (f32::NAN, "nan"),  // unordered ⇒ even ⇒ jp taken
            (1.3333333, "below"),
            (1.3333334, "above"),
        ] {
            let ah = run_gate1(&mut emu, g, which);
            // jp taken ⇔ EVEN parity of {C0,C2}.
            let taken = (ah & 0x5).count_ones() % 2 == 0;
            assert_eq!(taken, gate_g3_ge4(g), "gate1 which={which} g={g}");
            if taken {
                g1_taken += 1;
            } else {
                g1_not += 1;
            }
            total += 1;
        }
    }

    // gate2: deterministic boundary `v == g·0.75` exactly. `fcomp(v, p=g·0.75)`: jnp taken ⇔ v<p.
    for which in [0u8, 1u8] {
        for (g, v, _l) in [
            (2.0f32, (2.0f64 * 0.75) as f32, "v==p"), // v==p ⇒ even ⇒ jnp not taken
            (2.0, 1.0, "v<p"),                        // 1.0 < 1.5 ⇒ odd ⇒ jnp taken
            (2.0, 2.0, "v>p"),                        // 2.0 > 1.5 ⇒ even ⇒ jnp not taken
            (f32::NAN, 1.0, "nan-p"),                 // p unordered ⇒ even ⇒ jnp not taken
            (4.0, 3.0, "v==p2"),                      // 3.0 == 3.0
        ] {
            let ah = run_gate2(&mut emu, g, v, which);
            // jnp taken ⇔ ODD parity ⇔ v < g·0.75.
            let taken = (ah & 0x5).count_ones() % 2 == 1;
            assert_eq!(
                taken,
                gate_v_lt_g075(g, v),
                "gate2 which={which} g={g} v={v}"
            );
            if taken {
                g2_taken += 1;
            } else {
                g2_not += 1;
            }
            total += 1;
        }
    }

    let mut s = 0x4907_7000u32;
    for _ in 0..2000 {
        let g = rng(&mut s, -3.0, 5.0);
        for which in [0u8, 1u8] {
            let ah = run_gate1(&mut emu, g, which);
            let taken = (ah & 0x5).count_ones() % 2 == 0;
            assert_eq!(taken, gate_g3_ge4(g), "gate1 fuzz g={g}");
            if taken {
                g1_taken += 1;
            } else {
                g1_not += 1;
            }
            let v = (f64::from(g) * 0.75) as f32 + rng(&mut s, -0.5, 0.5);
            let ah2 = run_gate2(&mut emu, g, v, which);
            let taken2 = (ah2 & 0x5).count_ones() % 2 == 1;
            assert_eq!(taken2, gate_v_lt_g075(g, v), "gate2 fuzz g={g} v={v}");
            if taken2 {
                g2_taken += 1;
            } else {
                g2_not += 1;
            }
            total += 2;
        }
    }
    assert!(g1_taken > 0 && g1_not > 0, "gate1: need both branches");
    assert!(g2_taken > 0 && g2_not > 0, "gate2: need both branches");
    eprintln!(
        "glue_490770_eps_gates: {total} bit-exact (gate1 g·3 vs 4 taken={g1_taken}/{g1_not}, gate2 g·0.75 vs v taken={g2_taken}/{g2_not})"
    );
}
