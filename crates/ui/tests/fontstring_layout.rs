//! The arithmetic inside the font instance and multi-line text layout functions (`0x771ec0` to
//! `0x772d40`) against the exe: each function is run as the slice of instructions that computes the
//! value, including the leaf helpers `0x41ad70` and `0x41ae60`.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::fontstring_layout::{
    add_reload, div_scale, linecount_scale, recip_span_pair, rect_axis_settled, scale_mul,
    scale_offset, set_scaled_chain, settled_eps, settled_poseps,
};

/// `0x80c5c8`, the position-settle epsilon.
const POSEPS_BITS: u32 = 0x3727_c5ac;
/// `0x8029d4`, the change epsilon 2^-22.
const EPS_BITS: u32 = 0x3480_0000;
use fpkit::{next, rng};

fn ah_of(emu: &Emu) -> u8 {
    ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}
/// `test ah,5; jp/jnp` parity: odd set-bits in {C0,C2} ⟺ ordered `|d| < eps`.
fn parity_odd(ah: u8) -> bool {
    (ah & 0x5).count_ones() % 2 == 1
}

const POSEPS_F32: f32 = f32::from_bits(0x3727_c5ac);
const EPS_F32: f32 = f32::from_bits(0x3480_0000);

/// (1) scale-multiply — copied-byte island: fld [arg]; <verbatim fmul [esi+0x7c]>; fstp [res].
/// The fmul bytes are read from source 0x771ff4 (`d8 4e 7c`) via `e.read` — self-evidently faithful.
/// Covers all 28 `fmul [+0x7c]` ops across the family (one product, narrowed to f32 at the fstp).
#[test]
fn textglue_scale_mul() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x200); // esi (scale field at +0x7c)
    let arg_cell = emu.alloc(4);
    let res_cell = emu.alloc(4);

    let fmul_bytes = emu.read(0x771ff4, 3); // d8 4e 7c = fmul dword [esi+0x7c]
    assert_eq!(
        fmul_bytes,
        vec![0xd8, 0x4e, 0x7c],
        "fmul[esi+0x7c] source bytes"
    );
    // The same `fmul [esi+0x7c]` (`d8 4e 7c`) recurs in the sibling functions of the group, with the
    // site counts listed above. Its bytes are identical, so the slice run below stands for every
    // site; reading one site in each sibling checks that it is that operation.
    for site in [
        0x7721e0u64, // 0x772180
        0x772329,    // 0x7722a0
        0x772a0d,    // 0x7729b0
        0x772b29,    // 0x772ae0
        0x772b81,    // 0x772b60
        0x772bf2,    // 0x772bc0
        0x772c62,    // 0x772c30
    ] {
        assert_eq!(
            emu.read(site, 3),
            vec![0xd8, 0x4e, 0x7c],
            "fmul[esi+0x7c] family site {site:#x}"
        );
    }

    let region = emu.alloc(0x40);
    let mut c = vec![0xd9u8, 0x05]; // fld dword [arg_cell]
    c.extend_from_slice(&(arg_cell as u32).to_le_bytes());
    c.extend_from_slice(&fmul_bytes); // verbatim fmul [esi+0x7c]
    c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [disp32]
    c.extend_from_slice(&(res_cell as u32).to_le_bytes());
    let end = region + c.len() as u64;
    emu.write(region, &c);

    let run = |emu: &mut Emu, coord: f32, scale: f32| -> f32 {
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(arg_cell, coord.to_bits());
        emu.write_u32(this + 0x7c, scale.to_bits());
        emu.run(region, end);
        emu.read_f32(res_cell)
    };

    let mut total = 0u32;
    for (coord, scale) in [
        (1.0f32, 1.0f32),
        (10.0, 0.8),
        (-3.5, 2.0),
        (0.0, 100.0),
        (16_777_217.0, 1.0), // f32 rounding boundary
    ] {
        assert_eq!(
            run(&mut emu, coord, scale).to_bits(),
            scale_mul(coord, scale).to_bits(),
            "scale_mul coord={coord} scale={scale}"
        );
        total += 1;
    }
    let mut s = 0x7700_0001u32;
    for _ in 0..4000 {
        let coord = rng(&mut s, -2000.0, 2000.0);
        let scale = rng(&mut s, 0.01, 50.0);
        assert_eq!(
            run(&mut emu, coord, scale).to_bits(),
            scale_mul(coord, scale).to_bits(),
            "scale_mul fuzz coord={coord} scale={scale}"
        );
        total += 1;
    }
    eprintln!("textglue_scale_mul: {total} bit-exact (verbatim fmul[esi+0x7c], 28 family sites)");
}

/// (2) scale-then-offset — real slice 0x772142..0x77216c of 0x772100 (two legs):
///   fld[ecx+0x7c]; fmul[ecx+0x110]; fadd[ecx+0x68]; fstp[ebp-0xc]   (leg 1)
///   fld[ecx+0x114]; fmul[ecx+0x7c]; fadd[ecx+0x64]; fstp[ebp-0x8]   (leg 2)
/// halts before the call at 0x77216c. 4 arithmetic ops. (the `mov [ebp-4],0`/`lea edx`/`mov ecx,eax`
/// in-slice are harmless.)  Both legs compute `pos·scale + off` — the kernel `scale_offset`.
#[test]
fn textglue_scale_offset() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x200); // ecx
    let stack = emu.alloc(0x1000);
    let ebp = stack + 0x800;

    // leg1: pos=[ecx+0x110], scale=[ecx+0x7c], off=[ecx+0x68] -> [ebp-0xc].
    // leg2: pos=[ecx+0x114], scale=[ecx+0x7c], off=[ecx+0x64] -> [ebp-0x8].
    let run =
        |emu: &mut Emu, pos1: f32, pos2: f32, scale: f32, off1: f32, off2: f32| -> (f32, f32) {
            emu.set_reg(RegisterX86::ECX, this);
            emu.set_reg(RegisterX86::EBP, ebp);
            emu.set_reg(RegisterX86::ESP, stack + 0x100);
            emu.write_u32(this + 0x110, pos1.to_bits());
            emu.write_u32(this + 0x114, pos2.to_bits());
            emu.write_u32(this + 0x7c, scale.to_bits());
            emu.write_u32(this + 0x68, off1.to_bits());
            emu.write_u32(this + 0x64, off2.to_bits());
            emu.run(0x772142, 0x77216c);
            (emu.read_f32(ebp - 0xc), emu.read_f32(ebp - 0x8))
        };

    let mut total = 0u32;
    for (p, scale, off) in [
        (1.0f32, 1.0f32, 0.0f32),
        (10.0, 0.8, 5.0),
        (-3.5, 2.0, -1.0),
        (0.0, 0.0, 7.0),
    ] {
        let (r1, r2) = run(&mut emu, p, p, scale, off, off);
        assert_eq!(r1.to_bits(), scale_offset(p, scale, off).to_bits(), "leg1");
        assert_eq!(r2.to_bits(), scale_offset(p, scale, off).to_bits(), "leg2");
        total += 2;
    }
    let mut s = 0x7721_0000u32;
    for _ in 0..3000 {
        let pos1 = rng(&mut s, -1000.0, 1000.0);
        let pos2 = rng(&mut s, -1000.0, 1000.0);
        let scale = rng(&mut s, 0.01, 30.0);
        let off1 = rng(&mut s, -500.0, 500.0);
        let off2 = rng(&mut s, -500.0, 500.0);
        let (r1, r2) = run(&mut emu, pos1, pos2, scale, off1, off2);
        assert_eq!(
            r1.to_bits(),
            scale_offset(pos1, scale, off1).to_bits(),
            "leg1 fuzz"
        );
        assert_eq!(
            r2.to_bits(),
            scale_offset(pos2, scale, off2).to_bits(),
            "leg2 fuzz"
        );
        total += 2;
    }
    eprintln!("textglue_scale_offset: {total} bit-exact (0x772100 two legs pos·scale+off)");
}

/// (3) divide-by-scale — copied-byte island: fld [arg]; <verbatim fdiv [esi+0x7c]>; fstp [res].
/// fdiv bytes read from source 0x7727ea (`d8 76 7c`). Covers 0x7727b0/0x772890/0x7729b0/0x772ae0.
#[test]
fn textglue_div_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x200); // esi
    let arg_cell = emu.alloc(4);
    let res_cell = emu.alloc(4);

    let fdiv_bytes = emu.read(0x7727ea, 3); // d8 76 7c = fdiv dword [esi+0x7c]
    assert_eq!(
        fdiv_bytes,
        vec![0xd8, 0x76, 0x7c],
        "fdiv[esi+0x7c] source bytes"
    );

    let region = emu.alloc(0x40);
    let mut c = vec![0xd9u8, 0x05]; // fld dword [arg_cell]
    c.extend_from_slice(&(arg_cell as u32).to_le_bytes());
    c.extend_from_slice(&fdiv_bytes); // verbatim fdiv [esi+0x7c]
    c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [disp32]
    c.extend_from_slice(&(res_cell as u32).to_le_bytes());
    let end = region + c.len() as u64;
    emu.write(region, &c);

    let run = |emu: &mut Emu, val: f32, scale: f32| -> f32 {
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(arg_cell, val.to_bits());
        emu.write_u32(this + 0x7c, scale.to_bits());
        emu.run(region, end);
        emu.read_f32(res_cell)
    };

    let mut total = 0u32;
    for (val, scale) in [
        (1.0f32, 1.0f32),
        (8.0, 0.8),
        (-100.0, 2.0),
        (0.0, 5.0),
        (1.0, 3.0), // 1/3 — exercises the divide's f32 rounding
    ] {
        assert_eq!(
            run(&mut emu, val, scale).to_bits(),
            div_scale(val, scale).to_bits(),
            "div_scale val={val} scale={scale}"
        );
        total += 1;
    }
    let mut s = 0x7727_b000u32;
    for _ in 0..4000 {
        let val = rng(&mut s, -2000.0, 2000.0);
        let scale = rng(&mut s, 0.05, 50.0);
        assert_eq!(
            run(&mut emu, val, scale).to_bits(),
            div_scale(val, scale).to_bits(),
            "div_scale fuzz val={val} scale={scale}"
        );
        total += 1;
    }
    eprintln!("textglue_div_scale: {total} bit-exact (verbatim fdiv[esi+0x7c], 4 family sites)");
}

/// (4) set-scaled chain — 0x772930 / 0x772a60 (clone). The fmul[1024];fdivr[1.0] producing the f32
/// arg is a copied-byte island (verbatim bytes from 0x77295c/0x772963), st0 pre-loaded with SCALE;
/// the ×0.8 tail is the real leaf 0x41ae60 via `call_f64`. The composition = `set_scaled_chain`.
#[test]
fn textglue_set_scaled_chain() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scale_cell = emu.alloc(4);
    let arg_cell = emu.alloc(4);

    // island = fld [scale_cell] (=SCALE on st0); <verbatim fmul [0x7ffd68]=1024>; <verbatim
    // fdivr [0x7ff9d8]=1.0>; fstp [arg_cell] (the f32 arg leaf 0x41ae60 receives).
    let fmul1024 = emu.read(0x77295c, 6); // d8 0d 68 fd 7f 00
    let fdivr1 = emu.read(0x772963, 6); // d8 3d d8 f9 7f 00
    assert_eq!(
        fmul1024,
        vec![0xd8, 0x0d, 0x68, 0xfd, 0x7f, 0x00],
        "fmul[1024] bytes"
    );
    assert_eq!(
        fdivr1,
        vec![0xd8, 0x3d, 0xd8, 0xf9, 0x7f, 0x00],
        "fdivr[1.0] bytes"
    );

    let region = emu.alloc(0x40);
    let mut c = vec![0xd9u8, 0x05]; // fld [scale_cell]
    c.extend_from_slice(&(scale_cell as u32).to_le_bytes());
    c.extend_from_slice(&fmul1024); // fmul [0x7ffd68]
    c.extend_from_slice(&fdivr1); // fdivr [0x7ff9d8]
    c.extend_from_slice(&[0xd9, 0x1d]); // fstp [arg_cell]
    c.extend_from_slice(&(arg_cell as u32).to_le_bytes());
    let end = region + c.len() as u64;
    emu.write(region, &c);

    // SCALE = [0x832a4c] = 1.0 in the mapped image — but feed the real value read from the binary.
    let scale_val = f32::from_bits(emu.read_u32(0x832a4c));
    emu.write_u32(scale_cell, scale_val.to_bits());
    emu.set_reg(RegisterX86::ESP, 0x0020_0000);
    emu.run(region, end);
    let arg = emu.read_f32(arg_cell); // 1.0/(SCALE·1024) as f32

    // ×0.8 tail: the real leaf 0x41ae60 (fld [0x832a44]=0.8; fmul [ebp+8]; ret 4), returns f64 in st0.
    let chain = emu.call_f64(0x41ae60, 0, 0, &[arg.to_bits()]);

    assert_eq!(
        chain.to_bits(),
        set_scaled_chain().to_bits(),
        "set_scaled_chain (SCALE={scale_val})"
    );
    eprintln!("textglue_set_scaled_chain: 1 bit-exact (island fmul[1024]+fdivr[1.0] + leaf 0x41ae60 ×0.8)");
}

/// (5a) POSEPS st-sub change-detect — copied-byte island: fld[b]; fld[a]; <verbatim fsub st,st(1);
/// fabs; fcomp[POSEPS]; fnstsw> (bytes from 0x771f88, 0xc long). fsub st,st(1) = st0-st1 = a-b.
/// Covers 0x771ec0 (0x771f88/0x771fd0) and 0x7724a0 (0x7725fb/0x772632) — the same kernel.
#[test]
fn textglue_settled_poseps() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let a_cell = emu.alloc(4);
    let b_cell = emu.alloc(4);

    let kernel = emu.read(0x771f88, 0xc); // fsub st,st(1); fabs; fcomp[0x80c5c8]; fnstsw
    assert_eq!(
        kernel,
        vec![0xd8, 0xe1, 0xd9, 0xe1, 0xd8, 0x1d, 0xc8, 0xc5, 0x80, 0x00, 0xdf, 0xe0],
        "POSEPS kernel bytes"
    );
    let region = emu.alloc(0x40);
    let mut c = vec![0xd9u8, 0x05]; // fld [b_cell]  -> st0=b
    c.extend_from_slice(&(b_cell as u32).to_le_bytes());
    c.extend_from_slice(&[0xd9, 0x05]); // fld [a_cell] -> st0=a, st1=b
    c.extend_from_slice(&(a_cell as u32).to_le_bytes());
    c.extend_from_slice(&kernel); // fsub st,st(1) = a-b; fabs; fcomp[POSEPS]; fnstsw
    let end = region + c.len() as u64;
    emu.write(region, &c);

    let run = |emu: &mut Emu, a: f32, b: f32| -> u8 {
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(a_cell, a.to_bits());
        emu.write_u32(b_cell, b.to_bits());
        emu.run(region, end);
        ah_of(emu)
    };

    let mut total = 0u32;
    let (mut settled_n, mut changed_n) = (0u32, 0u32);
    let _ = POSEPS_BITS;
    for (a, b, _l) in [
        (0.0f32, POSEPS_F32, "==eps"),
        (0.0, POSEPS_F32 * 0.5, "below"),
        (0.0, POSEPS_F32 * 2.0, "above"),
        (5.0, 5.0, "equal"),
        (1.0, f32::NAN, "nan"),
    ] {
        let bin = parity_odd(run(&mut emu, a, b));
        assert_eq!(bin, settled_poseps(a, b), "poseps a={a} b={b}");
        if bin {
            settled_n += 1;
        } else {
            changed_n += 1;
        }
        total += 1;
    }
    let mut s = 0x771f_8800u32;
    for _ in 0..3000 {
        let a = rng(&mut s, -10.0, 10.0);
        let b = a + rng(&mut s, -3.0e-5, 3.0e-5);
        let bin = parity_odd(run(&mut emu, a, b));
        assert_eq!(bin, settled_poseps(a, b), "poseps fuzz a={a} b={b}");
        if bin {
            settled_n += 1;
        } else {
            changed_n += 1;
        }
        total += 1;
    }
    assert!(settled_n > 0 && changed_n > 0, "need both outcomes");
    eprintln!("textglue_settled_poseps: {total} bit-exact (settled={settled_n} changed={changed_n}; verbatim POSEPS kernel)");
}

/// (5b) EPS mem-fsubr change-detect — real slice 0x77225a..0x77226d of 0x772240:
///   fld [ecx+0xf4]; fsubr [ebp+8]; fabs; fcomp [0x8029d4=2⁻²²]; fnstsw. halts before `test ah,5`.
/// fsubr = [ebp+8] - [ecx+0xf4] = arg - field.
#[test]
fn textglue_settled_eps() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x200); // ecx (field at +0xf4)
    let stack = emu.alloc(0x1000);
    let ebp = stack + 0x800;

    let run = |emu: &mut Emu, arg: f32, field: f32| -> u8 {
        emu.set_reg(RegisterX86::ECX, this);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, stack + 0x100);
        emu.write_u32(this + 0xf4, field.to_bits());
        emu.write_u32(ebp + 0x8, arg.to_bits());
        emu.run(0x77225a, 0x77226d);
        ah_of(emu)
    };

    let mut total = 0u32;
    let (mut settled_n, mut changed_n) = (0u32, 0u32);
    let _ = EPS_BITS;
    for (arg, field, _l) in [
        (EPS_F32, 0.0f32, "==eps"),
        (EPS_F32 * 0.5, 0.0, "below"),
        (EPS_F32 * 2.0, 0.0, "above"),
        (7.0, 7.0, "equal"),
        (f32::NAN, 0.0, "nan"),
    ] {
        let bin = parity_odd(run(&mut emu, arg, field));
        assert_eq!(bin, settled_eps(arg, field), "eps arg={arg} field={field}");
        if bin {
            settled_n += 1;
        } else {
            changed_n += 1;
        }
        total += 1;
    }
    let mut s = 0x7722_4000u32;
    for _ in 0..3000 {
        let field = rng(&mut s, -10.0, 10.0);
        let arg = field + rng(&mut s, -1.0e-6, 1.0e-6);
        let bin = parity_odd(run(&mut emu, arg, field));
        assert_eq!(bin, settled_eps(arg, field), "eps fuzz");
        if bin {
            settled_n += 1;
        } else {
            changed_n += 1;
        }
        total += 1;
    }
    assert!(settled_n > 0 && changed_n > 0, "need both outcomes");
    eprintln!("textglue_settled_eps: {total} bit-exact (settled={settled_n} changed={changed_n}; 0x772240 fsubr[2⁻²²])");
}

/// (6) recip-span-pair — real slice 0x77253a..0x77255c of 0x7724a0:
///   fld[0x7ff9d8]=1.0; fdiv[esi+0x7c]; (recip on st1)
///   fld[esi+0x70]; fsub[esi+0x68]; fmul st,st(1); fstp[ebp-0x8]    (leg 0)
///   fld[esi+0x6c]; fsub[esi+0x64]; fmul st,st(1); fstp[ebp-0x4]    (leg 1)
/// 5 arithmetic. The recip is LIVE f64 on st1 across both legs (NOT a reloaded f32).
#[test]
fn textglue_recip_span_pair() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x200); // esi
    let stack = emu.alloc(0x1000);
    let ebp = stack + 0x800;

    let run = |emu: &mut Emu, scale: f32, hi0: f32, lo0: f32, hi1: f32, lo1: f32| -> (f32, f32) {
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, stack + 0x100);
        emu.write_u32(this + 0x7c, scale.to_bits());
        emu.write_u32(this + 0x70, hi0.to_bits());
        emu.write_u32(this + 0x68, lo0.to_bits());
        emu.write_u32(this + 0x6c, hi1.to_bits());
        emu.write_u32(this + 0x64, lo1.to_bits());
        emu.run(0x77253a, 0x77255c);
        (emu.read_f32(ebp - 0x8), emu.read_f32(ebp - 0x4))
    };

    let mut total = 0u32;
    for (scale, hi0, lo0, hi1, lo1) in [
        (1.0f32, 10.0f32, 2.0f32, 8.0f32, 1.0f32),
        (0.8, 100.0, 0.0, 50.0, 25.0),
        (3.0, -5.0, 5.0, 0.0, 0.0), // negative span
        (0.5, 1.0, 1.0, 2.0, 2.0),  // zero spans
    ] {
        let (r0, r1) = run(&mut emu, scale, hi0, lo0, hi1, lo1);
        let exp = recip_span_pair(scale, hi0, lo0, hi1, lo1);
        assert_eq!(r0.to_bits(), exp[0].to_bits(), "recip leg0");
        assert_eq!(r1.to_bits(), exp[1].to_bits(), "recip leg1");
        total += 2;
    }
    let mut s = 0x7724_a000u32;
    for _ in 0..3000 {
        let scale = rng(&mut s, 0.05, 30.0);
        let hi0 = rng(&mut s, -500.0, 500.0);
        let lo0 = rng(&mut s, -500.0, 500.0);
        let hi1 = rng(&mut s, -500.0, 500.0);
        let lo1 = rng(&mut s, -500.0, 500.0);
        let (r0, r1) = run(&mut emu, scale, hi0, lo0, hi1, lo1);
        let exp = recip_span_pair(scale, hi0, lo0, hi1, lo1);
        assert_eq!(r0.to_bits(), exp[0].to_bits(), "recip leg0 fuzz");
        assert_eq!(r1.to_bits(), exp[1].to_bits(), "recip leg1 fuzz");
        total += 2;
    }
    eprintln!(
        "textglue_recip_span_pair: {total} bit-exact (0x7724a0 (1/scale)·span, live-st1 recip)"
    );
}

/// (7) linecount-scale — verbatim slice 0x771f5c..0x771f64 of 0x771ec0:
///   fild qword [ebp-0xc]; fmul st,st(1); fstp [ebp+0xc]; fstp st(0).
/// st1 = the live coord (we pre-load via an island fld), then the verbatim slice runs.
/// fimul-equivalent: int(n) widened then × coord. We run the REAL slice with ebp set + st1 loaded.
#[test]
fn textglue_linecount_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let stack = emu.alloc(0x1000);
    let ebp = stack + 0x800;
    let coord_cell = emu.alloc(4);

    // island prologue: fld [coord_cell] -> st0=coord (becomes st1 after the fild), then jump into
    // the verbatim 0x771f5c slice. We assemble: fld[coord]; <verbatim 0x771f5c..0x771f64>.
    let slice = emu.read(0x771f5c, 0x771f66 - 0x771f5c); // fild qword[ebp-0xc]; fmul st,st(1); fstp[ebp+0xc]; fstp st(0)
    assert_eq!(
        &slice[0..5],
        &[0xdf, 0x6d, 0xf4, 0xd8, 0xc9],
        "fild qword[ebp-0xc]; fmul st,st(1) bytes"
    );
    let region = emu.alloc(0x40);
    let mut c = vec![0xd9u8, 0x05]; // fld [coord_cell] -> st0 (=> st1 after fild)
    c.extend_from_slice(&(coord_cell as u32).to_le_bytes());
    c.extend_from_slice(&slice); // verbatim: fild; fmul st,st(1); fstp[ebp+0xc]; fstp st(0)
    let end = region + c.len() as u64;
    emu.write(region, &c);

    let run = |emu: &mut Emu, n: u32, coord: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, stack + 0x100);
        emu.write_u32(coord_cell, coord.to_bits());
        // [ebp-0xc] = n (low dword), [ebp-0x8] = 0 (high dword) — the {n,0} i64 widen.
        emu.write_u32(ebp - 0xc, n);
        emu.write_u32(ebp - 0x8, 0);
        emu.run(region, end);
        emu.read_f32(ebp + 0xc)
    };

    let mut total = 0u32;
    for (n, coord) in [
        (0u32, 5.0f32),
        (1, 12.5),
        (3, -7.0),
        (40, 0.8),
        (0x0000_7fff, 1.0),
    ] {
        assert_eq!(
            run(&mut emu, n, coord).to_bits(),
            linecount_scale(n, coord).to_bits(),
            "linecount n={n} coord={coord}"
        );
        total += 1;
    }
    let mut s = 0x771e_c000u32;
    for _ in 0..3000 {
        let n = (next(&mut s) >> 16) & 0xffff; // counts stay modest, high-dword 0
        let coord = rng(&mut s, -100.0, 100.0);
        assert_eq!(
            run(&mut emu, n, coord).to_bits(),
            linecount_scale(n, coord).to_bits(),
            "linecount fuzz n={n} coord={coord}"
        );
        total += 1;
    }
    eprintln!("textglue_linecount_scale: {total} bit-exact (0x771ec0 fild qword + fmul st,st(1))");
}

/// (8) rect two-diff change-detect — real slice 0x772d62..0x772d7d of 0x772d40 (Y axis):
///   fld[ebx+0xc]; lea edi,[esi+0x40]; fsub[ebx+4]; fld[edi+0xc]; fsub[edi+4]; fsubp st(1),st;
///   fabs; fcomp[POSEPS]; fnstsw. halts before `test ah,5`. edi := esi+0x40 (set by the in-slice lea).
///   d1=[ebx+0xc]-[ebx+4], d2=[edi+0xc]-[edi+4], |d1-d2| vs POSEPS.
#[test]
fn textglue_rect_axis_settled() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x200); // edi := esi+0x40
    let ebx = emu.alloc(0x40);

    let run = |emu: &mut Emu, b1: f32, a1: f32, b2: f32, a2: f32| -> u8 {
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EBX, ebx);
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(ebx + 0xc, b1.to_bits());
        emu.write_u32(ebx + 0x4, a1.to_bits());
        emu.write_u32(esi + 0x40 + 0xc, b2.to_bits()); // [edi+0xc]
        emu.write_u32(esi + 0x40 + 0x4, a2.to_bits()); // [edi+4]
        emu.run(0x772d62, 0x772d7d);
        ah_of(emu)
    };

    let mut total = 0u32;
    let (mut settled_n, mut changed_n) = (0u32, 0u32);
    // d1-d2 boundary cases (the predicate is |d1-d2| vs POSEPS).
    for (b1, a1, b2, a2, _l) in [
        (10.0f32, 5.0f32, 7.0f32, 2.0f32, "d1==d2 (|diff|=0)"), // d1=5 d2=5 -> settled
        (POSEPS_F32, 0.0, 0.0, 0.0, "|diff|==eps"),
        (POSEPS_F32 * 2.0, 0.0, 0.0, 0.0, "above"),
        (POSEPS_F32 * 0.5, 0.0, 0.0, 0.0, "below"),
        (1.0, 0.0, f32::NAN, 0.0, "nan"),
    ] {
        let bin = parity_odd(run(&mut emu, b1, a1, b2, a2));
        assert_eq!(
            bin,
            rect_axis_settled(b1, a1, b2, a2),
            "rect b1={b1} a1={a1} b2={b2} a2={a2}"
        );
        if bin {
            settled_n += 1;
        } else {
            changed_n += 1;
        }
        total += 1;
    }
    let mut s = 0x772d_4000u32;
    for _ in 0..3000 {
        let base = rng(&mut s, -200.0, 200.0);
        let span = rng(&mut s, -50.0, 50.0);
        let b1 = base + span;
        let a1 = base;
        // d2 close to d1 so both settled/changed outcomes appear.
        let b2 = rng(&mut s, -200.0, 200.0);
        let a2 = b2 - (span + rng(&mut s, -2.0e-5, 2.0e-5));
        let bin = parity_odd(run(&mut emu, b1, a1, b2, a2));
        assert_eq!(bin, rect_axis_settled(b1, a1, b2, a2), "rect fuzz");
        if bin {
            settled_n += 1;
        } else {
            changed_n += 1;
        }
        total += 1;
    }
    assert!(settled_n > 0 && changed_n > 0, "need both outcomes");
    eprintln!("textglue_rect_axis_settled: {total} bit-exact (settled={settled_n} changed={changed_n}; 0x772d40 two-diff fsubp)");
}

/// (8b) rect SECOND-axis two-diff change-detect — real slice 0x772d82..0x772d98 of 0x772d40:
///   fld[ebx+0x8]; fsub[ebx](d8 23); fld[edi+0x8]; fsub[edi](d8 27); fsubp st(1),st; fabs;
///   fcomp[POSEPS]; fnstsw. halts before `test ah,5` at 0x772d98. This is the X axis: its fsubs are
///   `d8 23`/`d8 27` (no-disp [ebx]/[edi]) — DISTINCT bytes from axis-1's `d8 63 04`/`d8 67 04`, so
///   NOT a byte-clone of (8); it needs its own diffed range. edi must be pre-set to esi+0x40 (the
///   `lea edi,[esi+0x40]` lives at 0x772d65, before this slice). Same `rect_axis_settled` kernel:
///   d1=[ebx+8]-[ebx], d2=[edi+8]-[edi], |d1-d2| vs POSEPS.
#[test]
fn textglue_rect_axis2_settled() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x200);
    let edi = esi + 0x40; // the binary's `lea edi,[esi+0x40]` (precedes this slice — set it here)
    let ebx = emu.alloc(0x40);

    let run = |emu: &mut Emu, b1: f32, a1: f32, b2: f32, a2: f32| -> u8 {
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::EBX, ebx);
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(ebx + 0x8, b1.to_bits()); // [ebx+8]
        emu.write_u32(ebx, a1.to_bits()); // [ebx]
        emu.write_u32(edi + 0x8, b2.to_bits()); // [edi+8]
        emu.write_u32(edi, a2.to_bits()); // [edi]
        emu.run(0x772d82, 0x772d98);
        ah_of(emu)
    };

    let mut total = 0u32;
    let (mut settled_n, mut changed_n) = (0u32, 0u32);
    for (b1, a1, b2, a2, _l) in [
        (10.0f32, 5.0f32, 7.0f32, 2.0f32, "d1==d2 (|diff|=0)"),
        (POSEPS_F32, 0.0, 0.0, 0.0, "|diff|==eps"),
        (POSEPS_F32 * 2.0, 0.0, 0.0, 0.0, "above"),
        (POSEPS_F32 * 0.5, 0.0, 0.0, 0.0, "below"),
        (1.0, 0.0, f32::NAN, 0.0, "nan"),
    ] {
        let bin = parity_odd(run(&mut emu, b1, a1, b2, a2));
        assert_eq!(
            bin,
            rect_axis_settled(b1, a1, b2, a2),
            "rect-axis2 b1={b1} a1={a1} b2={b2} a2={a2}"
        );
        if bin {
            settled_n += 1;
        } else {
            changed_n += 1;
        }
        total += 1;
    }
    let mut s = 0x772d_8200u32;
    for _ in 0..3000 {
        let base = rng(&mut s, -200.0, 200.0);
        let span = rng(&mut s, -50.0, 50.0);
        let b1 = base + span;
        let a1 = base;
        let b2 = rng(&mut s, -200.0, 200.0);
        let a2 = b2 - (span + rng(&mut s, -2.0e-5, 2.0e-5));
        let bin = parity_odd(run(&mut emu, b1, a1, b2, a2));
        assert_eq!(bin, rect_axis_settled(b1, a1, b2, a2), "rect-axis2 fuzz");
        if bin {
            settled_n += 1;
        } else {
            changed_n += 1;
        }
        total += 1;
    }
    assert!(settled_n > 0 && changed_n > 0, "need both outcomes");
    eprintln!("textglue_rect_axis2_settled: {total} bit-exact (settled={settled_n} changed={changed_n}; 0x772d40 X-axis fsub[ebx]/[edi] d8 23/27)");
}

/// (8c) rect position transform — real slice 0x772d9d..0x772dc4 of 0x772d40 (two scale+offset legs):
///   fld[esi+0xec]; fmul[esi+0x58](d8 4e 58); fadd[esi+0x44](d8 46 44); fstp[ebp-0xc]   (leg 1)
///   fld[esi+0xf0]; fmul[esi+0x58]; fadd[edi](d8 07); fstp[ebp-0x8]                      (leg 2)
///   halts before `call 0x44d600` at 0x772dc4. The in-slice `lea edx,[ebp-0xc]` (0x772da3) and
///   `mov [ebp-0x4],0` (0x772da9) are harmless. Both legs are `pos·scale+off` = `scale_offset`
///   (scale=[esi+0x58]); leg-2's add is `fadd [edi]` = `[esi+0x40]` (edi:=esi+0x40, pre-set here).
#[test]
fn textglue_rect_pos_transform() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x200);
    let edi = esi + 0x40; // leg-2's `fadd [edi]` operand lives at [esi+0x40]
    let stack = emu.alloc(0x1000);
    let ebp = stack + 0x800;

    // leg1: pos=[esi+0xec], scale=[esi+0x58], off=[esi+0x44] -> [ebp-0xc].
    // leg2: pos=[esi+0xf0], scale=[esi+0x58], off=[edi]=[esi+0x40] -> [ebp-0x8].
    let run =
        |emu: &mut Emu, pos1: f32, pos2: f32, scale: f32, off1: f32, off2: f32| -> (f32, f32) {
            emu.set_reg(RegisterX86::ESI, esi);
            emu.set_reg(RegisterX86::EDI, edi);
            emu.set_reg(RegisterX86::EBP, ebp);
            emu.set_reg(RegisterX86::ESP, stack + 0x100);
            emu.write_u32(esi + 0xec, pos1.to_bits());
            emu.write_u32(esi + 0xf0, pos2.to_bits());
            emu.write_u32(esi + 0x58, scale.to_bits());
            emu.write_u32(esi + 0x44, off1.to_bits());
            emu.write_u32(esi + 0x40, off2.to_bits()); // [edi]
            emu.run(0x772d9d, 0x772dc4);
            (emu.read_f32(ebp - 0xc), emu.read_f32(ebp - 0x8))
        };

    let mut total = 0u32;
    for (p, scale, off) in [
        (1.0f32, 1.0f32, 0.0f32),
        (10.0, 0.8, 5.0),
        (-3.5, 2.0, -1.0),
        (0.0, 0.0, 7.0),
    ] {
        let (r1, r2) = run(&mut emu, p, p, scale, off, off);
        assert_eq!(
            r1.to_bits(),
            scale_offset(p, scale, off).to_bits(),
            "pos leg1"
        );
        assert_eq!(
            r2.to_bits(),
            scale_offset(p, scale, off).to_bits(),
            "pos leg2"
        );
        total += 2;
    }
    let mut s = 0x772d_9d00u32;
    for _ in 0..3000 {
        let pos1 = rng(&mut s, -1000.0, 1000.0);
        let pos2 = rng(&mut s, -1000.0, 1000.0);
        let scale = rng(&mut s, 0.01, 30.0);
        let off1 = rng(&mut s, -500.0, 500.0);
        let off2 = rng(&mut s, -500.0, 500.0);
        let (r1, r2) = run(&mut emu, pos1, pos2, scale, off1, off2);
        assert_eq!(
            r1.to_bits(),
            scale_offset(pos1, scale, off1).to_bits(),
            "pos leg1 fuzz"
        );
        assert_eq!(
            r2.to_bits(),
            scale_offset(pos2, scale, off2).to_bits(),
            "pos leg2 fuzz"
        );
        total += 2;
    }
    eprintln!("textglue_rect_pos_transform: {total} bit-exact (0x772d40 two pos·scale+off legs, scale=[esi+0x58])");
}

/// (2b) 0x7724a0 scale-then-offset legs — real slice 0x772515..0x77253a of 0x7724a0:
///   fld[esi+0x110]; fmul[esi+0x7c]; fadd[esi+0x68](d8 46 68); fstp[ebp-0x20]   (leg 1)
///   fld[esi+0x114]; fmul[esi+0x7c]; fadd[esi+0x64](d8 46 64); fstp[ebp-0x1c]   (leg 2)
///   halts at 0x77253a (start of the recip-span slice). The in-slice `mov [ebp-0x18],0` (0x77251b)
///   is harmless. These two `fadd`s are `d8 46 68`/`d8 46 64` (esi-based) — DISTINCT bytes from
///   0x772100's covered `d8 41 68`/`d8 41 64` (ecx-based), so NOT a byte-clone; they need their own
///   diffed range. Both legs compute `pos·scale+off` = `scale_offset` (scale=[esi+0x7c]).
#[test]
fn textglue_scale_offset_7724a0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x200);
    let stack = emu.alloc(0x1000);
    let ebp = stack + 0x800;

    // leg1: pos=[esi+0x110], scale=[esi+0x7c], off=[esi+0x68] -> [ebp-0x20].
    // leg2: pos=[esi+0x114], scale=[esi+0x7c], off=[esi+0x64] -> [ebp-0x1c].
    let run =
        |emu: &mut Emu, pos1: f32, pos2: f32, scale: f32, off1: f32, off2: f32| -> (f32, f32) {
            emu.set_reg(RegisterX86::ESI, esi);
            emu.set_reg(RegisterX86::EBP, ebp);
            emu.set_reg(RegisterX86::ESP, stack + 0x100);
            emu.write_u32(esi + 0x110, pos1.to_bits());
            emu.write_u32(esi + 0x114, pos2.to_bits());
            emu.write_u32(esi + 0x7c, scale.to_bits());
            emu.write_u32(esi + 0x68, off1.to_bits());
            emu.write_u32(esi + 0x64, off2.to_bits());
            emu.run(0x772515, 0x77253a);
            (emu.read_f32(ebp - 0x20), emu.read_f32(ebp - 0x1c))
        };

    let mut total = 0u32;
    for (p, scale, off) in [
        (1.0f32, 1.0f32, 0.0f32),
        (10.0, 0.8, 5.0),
        (-3.5, 2.0, -1.0),
        (0.0, 0.0, 7.0),
    ] {
        let (r1, r2) = run(&mut emu, p, p, scale, off, off);
        assert_eq!(
            r1.to_bits(),
            scale_offset(p, scale, off).to_bits(),
            "7724a0 leg1"
        );
        assert_eq!(
            r2.to_bits(),
            scale_offset(p, scale, off).to_bits(),
            "7724a0 leg2"
        );
        total += 2;
    }
    let mut s = 0x7724_a051u32;
    for _ in 0..3000 {
        let pos1 = rng(&mut s, -1000.0, 1000.0);
        let pos2 = rng(&mut s, -1000.0, 1000.0);
        let scale = rng(&mut s, 0.01, 30.0);
        let off1 = rng(&mut s, -500.0, 500.0);
        let off2 = rng(&mut s, -500.0, 500.0);
        let (r1, r2) = run(&mut emu, pos1, pos2, scale, off1, off2);
        assert_eq!(
            r1.to_bits(),
            scale_offset(pos1, scale, off1).to_bits(),
            "7724a0 leg1 fuzz"
        );
        assert_eq!(
            r2.to_bits(),
            scale_offset(pos2, scale, off2).to_bits(),
            "7724a0 leg2 fuzz"
        );
        total += 2;
    }
    eprintln!("textglue_scale_offset_7724a0: {total} bit-exact (0x7724a0 two pos·scale+off legs, fadd[esi+0x68]/[esi+0x64])");
}

/// (7b) base-add — 0x771ec0's `fadd [ebp-4]` (`d8 45 fc`): the delegated text-extent (0x766750's
/// LIVE f64 in st0) + the f32 reload of [ebp-4]. The sum is kept LIVE f64 (feeds linecount's `fmul
/// st,st(1)`), NOT narrowed at this op. Island: fld qword[live]=true-f64 st0; <fadd dword [mem32]>
/// (same `d8 /0` arithmetic as the source `d8 45 fc`, disp32 addressing); fstp qword[res] reads back
/// the LIVE f64 (no f32 narrowing). We byte-verify the source op is the f32-operand fadd.
#[test]
fn textglue_add_reload() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // Byte-verify the source op: 0x771f45 = `d8 45 fc` = fadd dword [ebp-4] (f32 operand, /0=add).
    let src = emu.read(0x771f45, 3);
    assert_eq!(src, vec![0xd8, 0x45, 0xfc], "fadd [ebp-4] source bytes");

    let live_cell = emu.alloc(8); // f64 live st0 input
    let mem_cell = emu.alloc(4); // f32 reload operand
    let res_cell = emu.alloc(8); // f64 result (live, un-narrowed)

    let region = emu.alloc(0x40);
    let mut c = vec![0xddu8, 0x05]; // fld qword [live_cell] -> st0 = true f64
    c.extend_from_slice(&(live_cell as u32).to_le_bytes());
    c.extend_from_slice(&[0xd8, 0x05]); // fadd dword [mem_cell]  (d8 /0, f32 operand — same as d8 45 fc)
    c.extend_from_slice(&(mem_cell as u32).to_le_bytes());
    c.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [res_cell]  (read back the LIVE f64)
    c.extend_from_slice(&(res_cell as u32).to_le_bytes());
    let end = region + c.len() as u64;
    emu.write(region, &c);

    let run = |emu: &mut Emu, live: f64, reload: f32| -> f64 {
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write(live_cell, &live.to_bits().to_le_bytes());
        emu.write_u32(mem_cell, reload.to_bits());
        emu.run(region, end);
        let b = emu.read(res_cell, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    for (live, reload) in [
        (0.0f64, 0.0f32),
        (10.5, 2.25),
        (-3.0, 7.0),
        // a live f64 with low mantissa bits a reloaded-f32 add would round differently if narrowed:
        (1.000_000_000_1_f64, 1.0f32),
        (123_456.789, -0.001),
    ] {
        assert_eq!(
            run(&mut emu, live, reload).to_bits(),
            add_reload(live, reload).to_bits(),
            "add_reload live={live} reload={reload}"
        );
        total += 1;
    }
    let mut s = 0x771f_4500u32;
    for _ in 0..3000 {
        let live =
            f64::from(rng(&mut s, -1000.0, 1000.0)) + f64::from(rng(&mut s, -1.0, 1.0)) * 1.0e-9; // sub-f32 mantissa tail
        let reload = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(
            run(&mut emu, live, reload).to_bits(),
            add_reload(live, reload).to_bits(),
            "add_reload fuzz live={live} reload={reload}"
        );
        total += 1;
    }
    eprintln!(
        "textglue_add_reload: {total} bit-exact (0x771ec0 fadd [ebp-4], live-f64 + f32 reload)"
    );
}
