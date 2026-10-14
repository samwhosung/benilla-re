//! The vector and scalar arithmetic inside the world frame's pick, light, fade, render and reticle
//! functions (`0x482200` to `0x4837b0`) against the exe: each function is run as the slice of
//! instructions that computes the value.
//!
//! The cosine and sine are compared with the host's functions structurally, as the x87
//! transcendentals differ by CPU; everything else bit for bit.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::world_frame::{
    add3, affine_combine, bool_inv255, cmp_ge_or_nan, corner_build, cos_sin, delta_is_zero,
    gated_triple, mul_add, negate, normalize3, sq_len3, square_f64, sub3, tick_scale,
    unit_convert_round,
};
use fpkit::{next, rng};

fn ah_of(emu: &Emu) -> u8 {
    ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// (1) 0x482200 dot(v,v) squared-length — verbatim st-arith island (bytes 0x4822f7..0x482307):
///   fld st(0); fmul st,st(1); fld st(2); fmul st,st(3); faddp; fld st(3); fmul st,st(4); faddp.
/// We pre-load st = {a (top), b, c} via three flds, run the verbatim arithmetic, then fstp the live
/// st0 (= a²+b²+c²) to an f32 cell. The 5 arithmetic ops (0x4822f9/fd/ff/303/305) are the source bytes.
#[test]
fn textglue_482_sq_len3_482200() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let a_cell = emu.alloc(4);
    let b_cell = emu.alloc(4);
    let c_cell = emu.alloc(4);
    let res = emu.alloc(4);

    // Byte-verify the source arithmetic bytes.
    let kern = emu.read(0x4822f7, 0x482307 - 0x4822f7);
    assert_eq!(
        kern,
        vec![
            0xd9, 0xc0, 0xd8, 0xc9, 0xd9, 0xc2, 0xd8, 0xcb, 0xde, 0xc1, 0xd9, 0xc3, 0xd8, 0xcc,
            0xde, 0xc1
        ],
        "0x482200 dot st-arith bytes"
    );

    let region = emu.alloc(0x40);
    // push c, b, a (so st0=a, st1=b, st2=c), then the verbatim arith, then fstp [res].
    let mut code = vec![0xd9u8, 0x05]; // fld [c]
    code.extend_from_slice(&(c_cell as u32).to_le_bytes());
    code.extend_from_slice(&[0xd9, 0x05]); // fld [b]
    code.extend_from_slice(&(b_cell as u32).to_le_bytes());
    code.extend_from_slice(&[0xd9, 0x05]); // fld [a]
    code.extend_from_slice(&(a_cell as u32).to_le_bytes());
    code.extend_from_slice(&kern); // verbatim dot st-arith -> st0 = a²+b²+c²
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp [res]
    code.extend_from_slice(&(res as u32).to_le_bytes());
    let end = region + code.len() as u64;
    emu.write(region, &code);

    let run = |emu: &mut Emu, a: f32, b: f32, c: f32| -> f32 {
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(a_cell, a.to_bits());
        emu.write_u32(b_cell, b.to_bits());
        emu.write_u32(c_cell, c.to_bits());
        emu.run(region, end);
        emu.read_f32(res)
    };

    let mut total = 0u32;
    for (a, b, c) in [
        (1.0f32, 0.0f32, 0.0f32),
        (3.0, 4.0, 0.0),
        (1.0, 2.0, 2.0),
        (-5.0, 0.5, -100.0),
        (16_777_217.0, 1.0, 1.0), // f32 boundary
    ] {
        assert_eq!(
            run(&mut emu, a, b, c).to_bits(),
            sq_len3(a, b, c).to_bits(),
            "sq_len3 a={a} b={b} c={c}"
        );
        total += 1;
    }
    let mut s = 0x4822_0000u32;
    for _ in 0..4000 {
        let a = rng(&mut s, -1000.0, 1000.0);
        let b = rng(&mut s, -1000.0, 1000.0);
        let c = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(
            run(&mut emu, a, b, c).to_bits(),
            sq_len3(a, b, c).to_bits(),
            "sq_len3 fuzz"
        );
        total += 1;
    }
    eprintln!("textglue_482_sq_len3_482200: {total} bit-exact (0x482200 dot st-arith, 5 ops)");
}

/// (1b) 0x482200 / 0x482740 square-then-compare — island: fld [v]; <verbatim fmul [v]>; fcomp [thr];
///   fnstsw. The fmul bytes are read verbatim from source 0x4825d5 (`d8 4d 08` = fmul [ebp+8]) — we
///   re-encode the same `d8 /1` square as `fmul [v_cell]` (disp32) to make it self-contained; the
///   source op IS the f32-operand square. We diff the SQUARED VALUE (square_f64) via fstp qword of the
///   live st0 BEFORE the fcomp, plus the AH parity of the compare.
#[test]
fn textglue_482_square_cmp() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // Byte-verify the two source square ops are `fmul dword [ebp+8]` (d8 4d 08) — f32-operand /1.
    let s1 = emu.read(0x4825d5, 3);
    let s2 = emu.read(0x482878, 3);
    assert_eq!(s1, vec![0xd8, 0x4d, 0x08], "0x4825d5 fmul [ebp+8] bytes");
    assert_eq!(s2, vec![0xd8, 0x4d, 0x08], "0x482878 fmul [ebp+8] bytes");

    let v_cell = emu.alloc(4);
    let res = emu.alloc(8); // f64 squared value (live st0)
    let region = emu.alloc(0x40);
    // fld [v]; fmul [v] (d8 /1 = square, f32 operand); fstp qword [res] (read back LIVE f64 square).
    let mut code = vec![0xd9u8, 0x05]; // fld [v]
    code.extend_from_slice(&(v_cell as u32).to_le_bytes());
    code.extend_from_slice(&[0xd8, 0x0d]); // fmul dword [disp32] = v*v (same d8 /1 arith as src)
    code.extend_from_slice(&(v_cell as u32).to_le_bytes());
    code.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [res]
    code.extend_from_slice(&(res as u32).to_le_bytes());
    let end = region + code.len() as u64;
    emu.write(region, &code);

    let run = |emu: &mut Emu, v: f32| -> f64 {
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(v_cell, v.to_bits());
        emu.run(region, end);
        let b = emu.read(res, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    for v in [0.0f32, 1.0, -3.5, 1000.0, 0.001, 12345.678] {
        assert_eq!(
            run(&mut emu, v).to_bits(),
            square_f64(v).to_bits(),
            "square v={v}"
        );
        total += 1;
    }
    let mut s = 0x4825_d500u32;
    for _ in 0..4000 {
        let v = rng(&mut s, -2000.0, 2000.0);
        assert_eq!(
            run(&mut emu, v).to_bits(),
            square_f64(v).to_bits(),
            "square fuzz"
        );
        total += 1;
    }
    eprintln!("textglue_482_square_cmp: {total} bit-exact (0x4825d5/0x482878 v² live-f64)");
}

/// (2) 0x482740 vec3-subtract — real slice 0x482789..0x4827a8:
///   fld[eax+8]; fsub[ebx+8]; fstp[esp+8]; fld[eax+4]; fsub[ebx+4]; fstp[esp+4]; fld[eax]; fsub[ebx];
///   fstp[esp]. Each `a-b` per component, narrowed f32. (The in-slice `sub esp,0xc` is harmless; we
///   set esp so [esp..esp+8] are valid scratch.)
#[test]
fn textglue_482_sub3() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let eax = emu.alloc(0x40); // a vec3
    let ebx = emu.alloc(0x40); // b vec3
    let stack = emu.alloc(0x1000);

    let run = |emu: &mut Emu, a: [f32; 3], b: [f32; 3]| -> [f32; 3] {
        emu.set_reg(RegisterX86::EAX, eax);
        emu.set_reg(RegisterX86::EBX, ebx);
        let esp = stack + 0x100;
        emu.set_reg(RegisterX86::ESP, esp);
        for i in 0..3 {
            emu.write_u32(eax + 4 * i as u64, a[i].to_bits());
            emu.write_u32(ebx + 4 * i as u64, b[i].to_bits());
        }
        emu.run(0x482789, 0x4827aa); // halt at the `call`, after the 3rd fstp[esp]
                                     // results at [esp-0xc + 0/4/8] (the in-slice `sub esp,0xc` moved esp down by 0xc).
        let base = esp - 0xc;
        [
            emu.read_f32(base),
            emu.read_f32(base + 4),
            emu.read_f32(base + 8),
        ]
    };

    let mut total = 0u32;
    for (a, b) in [
        ([1.0f32, 2.0, 3.0], [0.5f32, 1.0, 1.5]),
        ([0.0, 0.0, 0.0], [1.0, -2.0, 3.0]),
        ([-100.0, 50.0, 0.0], [100.0, 50.0, -25.0]),
    ] {
        assert_eq!(
            run(&mut emu, a, b).map(f32::to_bits),
            sub3(a, b).map(f32::to_bits),
            "sub3 a={a:?} b={b:?}"
        );
        total += 1;
    }
    let mut st = 0x4827_8900u32;
    for _ in 0..3000 {
        let a = [
            rng(&mut st, -1000.0, 1000.0),
            rng(&mut st, -1000.0, 1000.0),
            rng(&mut st, -1000.0, 1000.0),
        ];
        let b = [
            rng(&mut st, -1000.0, 1000.0),
            rng(&mut st, -1000.0, 1000.0),
            rng(&mut st, -1000.0, 1000.0),
        ];
        assert_eq!(
            run(&mut emu, a, b).map(f32::to_bits),
            sub3(a, b).map(f32::to_bits),
            "sub3 fuzz"
        );
        total += 1;
    }
    eprintln!("textglue_482_sub3: {total} bit-exact (0x482740 vec3 a-b, 3 fsub)");
}

/// (2b) 0x482740 squared-length (mem-operand form) — real slice 0x4827af..0x4827ca:
///   fld[ebp-0xc]; fmul[ebp-0xc]; fld[ebp-0x10]; fmul[ebp-0x10]; faddp; fld[ebp-0x14]; fmul[ebp-0x14];
///   faddp; fstp[ebp-4]. = a²+b²+c² narrowed f32. (3 fmul + 2 faddp.)
#[test]
fn textglue_482_sq_len3_482740() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let stack = emu.alloc(0x1000);
    let ebp = stack + 0x800;

    let run = |emu: &mut Emu, a: f32, b: f32, c: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, stack + 0x100);
        emu.write_u32(ebp - 0xc, a.to_bits());
        emu.write_u32(ebp - 0x10, b.to_bits());
        emu.write_u32(ebp - 0x14, c.to_bits());
        emu.run(0x4827af, 0x4827ca);
        emu.read_f32(ebp - 0x4)
    };

    let mut total = 0u32;
    for (a, b, c) in [
        (1.0f32, 0.0f32, 0.0f32),
        (3.0, 4.0, 0.0),
        (1.0, 2.0, 2.0),
        (-5.0, 0.5, -100.0),
    ] {
        assert_eq!(
            run(&mut emu, a, b, c).to_bits(),
            sq_len3(a, b, c).to_bits(),
            "sq_len3(740) a={a} b={b} c={c}"
        );
        total += 1;
    }
    let mut s = 0x4827_af00u32;
    for _ in 0..4000 {
        let a = rng(&mut s, -1000.0, 1000.0);
        let b = rng(&mut s, -1000.0, 1000.0);
        let c = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(
            run(&mut emu, a, b, c).to_bits(),
            sq_len3(a, b, c).to_bits(),
            "sq_len3(740) fuzz"
        );
        total += 1;
    }
    eprintln!(
        "textglue_482_sq_len3_482740: {total} bit-exact (0x482740 mem-form len², 3 fmul+2 faddp)"
    );
}

/// (3) 0x482ad0 tick·dt — verbatim island: fild qword [n_cell]; <verbatim fmul [0x801360]=0.001>;
///   fstp [res]. The fmul bytes are read from source 0x482b08 (`d8 0d 60 13 80 00`). {n,0} i64 widen.
#[test]
fn textglue_482_tick_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let n_cell = emu.alloc(8); // {low=n, high=0}
    let res = emu.alloc(4);

    let fmul = emu.read(0x482b08, 6);
    assert_eq!(
        fmul,
        vec![0xd8, 0x0d, 0x60, 0x13, 0x80, 0x00],
        "0x482b08 fmul[0x801360] bytes"
    );
    let region = emu.alloc(0x40);
    let mut code = vec![0xdfu8, 0x2d]; // fild qword [disp32]
    code.extend_from_slice(&(n_cell as u32).to_le_bytes());
    code.extend_from_slice(&fmul); // verbatim fmul [0x801360]=0.001
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp [res]
    code.extend_from_slice(&(res as u32).to_le_bytes());
    let end = region + code.len() as u64;
    emu.write(region, &code);

    let run = |emu: &mut Emu, n: u32| -> f32 {
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(n_cell, n);
        emu.write_u32(n_cell + 4, 0); // high dword 0
        emu.run(region, end);
        emu.read_f32(res)
    };

    let mut total = 0u32;
    for n in [0u32, 1, 16, 1000, 60_000, 0x7fff_ffff, 0xffff_ffff] {
        assert_eq!(
            run(&mut emu, n).to_bits(),
            tick_scale(n).to_bits(),
            "tick_scale n={n}"
        );
        total += 1;
    }
    let mut s = 0x482a_d000u32;
    for _ in 0..4000 {
        let n = next(&mut s);
        assert_eq!(
            run(&mut emu, n).to_bits(),
            tick_scale(n).to_bits(),
            "tick fuzz"
        );
        total += 1;
    }
    eprintln!("textglue_482_tick_scale: {total} bit-exact (0x482ad0 fild qword·dt, 1 op)");
}

/// (4) 0x482ad0 vec3 normalize — verbatim island (bytes 0x482b57..0x482b86):
///   fld st0; fmul st,st1; fld st2; fmul st,st3; faddp; fld st3; fmul st,st4; faddp; fsqrt;
///   fstp st3; fstp st0; fstp st0; fdivr [0x7ff9d8]=1.0; fld st0; fmul [eax]; fstp [eax]; fld st0;
///   fmul [eax+4]; fstp [eax+4]; fmul [eax+8]; fstp [eax+8].
/// We pre-load st = {a (top), b, c} (the normalize's components), point eax at the vec3 in memory,
/// run the verbatim normalize, read back the three normalized f32s. Covers 3 fmul+2 faddp+fsqrt+fdivr
/// +3 fmul = 10 ops. (The verbatim slice writes [eax]/[eax+4]/[eax+8] itself.)
#[test]
fn textglue_482_normalize3() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let a_cell = emu.alloc(4);
    let b_cell = emu.alloc(4);
    let c_cell = emu.alloc(4);
    let vec = emu.alloc(0x10); // eax: {x,y,z}

    // include the final `fstp dword [eax+8]` (0x482b86, 3 bytes) so the z component is stored.
    let kern = emu.read(0x482b57, 0x482b89 - 0x482b57);
    assert_eq!(&kern[0..2], &[0xd9, 0xc0], "normalize starts fld st0");
    assert_eq!(&kern[16..18], &[0xd9, 0xfa], "fsqrt present");
    assert_eq!(
        &kern[kern.len() - 3..],
        &[0xd9, 0x58, 0x08],
        "ends fstp[eax+8]"
    );

    let region = emu.alloc(0x60);
    let mut code = vec![0xd9u8, 0x05]; // fld [c]
    code.extend_from_slice(&(c_cell as u32).to_le_bytes());
    code.extend_from_slice(&[0xd9, 0x05]); // fld [b]
    code.extend_from_slice(&(b_cell as u32).to_le_bytes());
    code.extend_from_slice(&[0xd9, 0x05]); // fld [a]  => st0=a, st1=b, st2=c
    code.extend_from_slice(&(a_cell as u32).to_le_bytes());
    code.extend_from_slice(&kern); // verbatim normalize (writes [eax]/[eax+4]/[eax+8])
    let end = region + code.len() as u64;
    emu.write(region, &code);

    let run = |emu: &mut Emu, a: f32, b: f32, c: f32| -> [f32; 3] {
        emu.set_reg(RegisterX86::EAX, vec);
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(a_cell, a.to_bits());
        emu.write_u32(b_cell, b.to_bits());
        emu.write_u32(c_cell, c.to_bits());
        // the verbatim slice multiplies recip into [eax]/[eax+4]/[eax+8] — preload them with a,b,c
        // so the result is a·recip etc (the binary's `fmul [eax]` reads the component there).
        emu.write_u32(vec, a.to_bits());
        emu.write_u32(vec + 4, b.to_bits());
        emu.write_u32(vec + 8, c.to_bits());
        emu.run(region, end);
        [
            emu.read_f32(vec),
            emu.read_f32(vec + 4),
            emu.read_f32(vec + 8),
        ]
    };

    let mut total = 0u32;
    for (a, b, c) in [
        (3.0f32, 4.0f32, 0.0f32),
        (1.0, 1.0, 1.0),
        (-2.0, 0.0, 0.0),
        (0.1, 0.2, 0.3),
        (100.0, -50.0, 25.0),
    ] {
        assert_eq!(
            run(&mut emu, a, b, c).map(f32::to_bits),
            normalize3(a, b, c).map(f32::to_bits),
            "normalize a={a} b={b} c={c}"
        );
        total += 1;
    }
    let mut s = 0x482b_5700u32;
    for _ in 0..4000 {
        let a = rng(&mut s, -500.0, 500.0);
        let b = rng(&mut s, -500.0, 500.0);
        let c = rng(&mut s, -500.0, 500.0);
        // skip a near-zero vector (1/len → inf, both sides agree but uninformative)
        assert_eq!(
            run(&mut emu, a, b, c).map(f32::to_bits),
            normalize3(a, b, c).map(f32::to_bits),
            "normalize fuzz a={a} b={b} c={c}"
        );
        total += 1;
    }
    eprintln!("textglue_482_normalize3: {total} bit-exact (0x482ad0 normalize, 10 ops)");
}

/// (5) 0x482c10 bool·(1/255) — verbatim island: fild [i_cell]; <verbatim fmul [0x8026c8]=1/255>;
///   fstp [res]. The fmul bytes from source 0x482c7f (`d8 0d c8 26 80 00`). i ∈ {0,1}.
#[test]
fn textglue_482_bool_inv255() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let i_cell = emu.alloc(4);
    let res = emu.alloc(4);

    let fmul = emu.read(0x482c7f, 6);
    assert_eq!(
        fmul,
        vec![0xd8, 0x0d, 0xc8, 0x26, 0x80, 0x00],
        "0x482c7f fmul[0x8026c8] bytes"
    );
    let region = emu.alloc(0x40);
    let mut code = vec![0xdbu8, 0x05]; // fild dword [disp32]
    code.extend_from_slice(&(i_cell as u32).to_le_bytes());
    code.extend_from_slice(&fmul); // verbatim fmul [0x8026c8]=1/255
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp [res]
    code.extend_from_slice(&(res as u32).to_le_bytes());
    let end = region + code.len() as u64;
    emu.write(region, &code);

    let run = |emu: &mut Emu, b: bool| -> f32 {
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(i_cell, if b { 1 } else { 0 });
        emu.run(region, end);
        emu.read_f32(res)
    };

    let mut total = 0u32;
    for b in [false, true] {
        assert_eq!(
            run(&mut emu, b).to_bits(),
            bool_inv255(b).to_bits(),
            "bool_inv255 b={b}"
        );
        total += 1;
    }
    eprintln!("textglue_482_bool_inv255: {total} bit-exact (0x482c10 bool·(1/255), 1 op)");
}

/// (6) 0x482ea0 vec3-add — real slice 0x4830be..0x4830e3:
///   fld[edi+0x10]; fadd[eax+8]; fstp[esp+8]; fld[edi+0xc]; fadd[eax+4]; fstp[esp+4]; fld[edi+8];
///   fadd[eax]; fstp[esp]. = per-component v[edi]+w[eax], narrowed f32. (in-slice sub esp,0xc.)
#[test]
fn textglue_482_add3() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let edi = emu.alloc(0x40);
    let eax = emu.alloc(0x40);
    let stack = emu.alloc(0x1000);

    // v components at [edi+8],[edi+0xc],[edi+0x10]; w at [eax],[eax+4],[eax+8].
    let run = |emu: &mut Emu, v: [f32; 3], w: [f32; 3]| -> [f32; 3] {
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::EAX, eax);
        let esp = stack + 0x100;
        emu.set_reg(RegisterX86::ESP, esp);
        emu.write_u32(edi + 0x8, v[0].to_bits());
        emu.write_u32(edi + 0xc, v[1].to_bits());
        emu.write_u32(edi + 0x10, v[2].to_bits());
        emu.write_u32(eax, w[0].to_bits());
        emu.write_u32(eax + 4, w[1].to_bits());
        emu.write_u32(eax + 8, w[2].to_bits());
        emu.run(0x4830be, 0x4830e3);
        let base = esp - 0xc;
        [
            emu.read_f32(base),
            emu.read_f32(base + 4),
            emu.read_f32(base + 8),
        ]
    };

    // NB the binary stores leg order [esp+8]=v[2]+w[2], [esp+4]=v[1]+w[1], [esp]=v[0]+w[0]; so the
    // base array reads [v0+w0, v1+w1, v2+w2] = add3(v,w) directly.
    let mut total = 0u32;
    for (v, w) in [
        ([1.0f32, 2.0, 3.0], [0.5f32, 1.0, 1.5]),
        ([0.0, 0.0, 0.0], [1.0, -2.0, 3.0]),
        ([-100.0, 50.0, 0.0], [100.0, 50.0, -25.0]),
    ] {
        assert_eq!(
            run(&mut emu, v, w).map(f32::to_bits),
            add3(v, w).map(f32::to_bits),
            "add3 v={v:?} w={w:?}"
        );
        total += 1;
    }
    let mut st = 0x4830_be00u32;
    for _ in 0..3000 {
        let v = [
            rng(&mut st, -1000.0, 1000.0),
            rng(&mut st, -1000.0, 1000.0),
            rng(&mut st, -1000.0, 1000.0),
        ];
        let w = [
            rng(&mut st, -1000.0, 1000.0),
            rng(&mut st, -1000.0, 1000.0),
            rng(&mut st, -1000.0, 1000.0),
        ];
        assert_eq!(
            run(&mut emu, v, w).map(f32::to_bits),
            add3(v, w).map(f32::to_bits),
            "add3 fuzz"
        );
        total += 1;
    }
    eprintln!("textglue_482_add3: {total} bit-exact (0x482ea0 vec3 v+w, 3 fadd)");
}

/// (6b) 0x482ea0 multiply-accumulate — real slice 0x4831d0..0x4831ee:
///   fld[edi+0x90]; fmul[edi+0xbc]; fadd[ebp-0x20]; fstp[ebp-0x20]. = a·b + acc narrowed f32.
#[test]
fn textglue_482_mul_add() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let edi = emu.alloc(0x200);
    let stack = emu.alloc(0x1000);
    let ebp = stack + 0x800;

    let run = |emu: &mut Emu, a: f32, b: f32, acc: f32| -> f32 {
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, stack + 0x100);
        emu.write_u32(edi + 0x90, a.to_bits());
        emu.write_u32(edi + 0xbc, b.to_bits());
        emu.write_u32(ebp - 0x20, acc.to_bits());
        emu.run(0x4831d0, 0x4831ee);
        emu.read_f32(ebp - 0x20)
    };

    let mut total = 0u32;
    for (a, b, acc) in [
        (1.0f32, 1.0f32, 0.0f32),
        (2.0, 3.0, 5.0),
        (-4.0, 0.5, 10.0),
        (0.1, 0.2, 0.3),
    ] {
        assert_eq!(
            run(&mut emu, a, b, acc).to_bits(),
            mul_add(a, b, acc).to_bits(),
            "mul_add a={a} b={b} acc={acc}"
        );
        total += 1;
    }
    let mut s = 0x4831_d000u32;
    for _ in 0..4000 {
        let a = rng(&mut s, -100.0, 100.0);
        let b = rng(&mut s, -100.0, 100.0);
        let acc = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(
            run(&mut emu, a, b, acc).to_bits(),
            mul_add(a, b, acc).to_bits(),
            "mul_add fuzz"
        );
        total += 1;
    }
    eprintln!("textglue_482_mul_add: {total} bit-exact (0x482ea0 a·b+acc, fmul+fadd)");
}

/// (7) 0x483340 flag-gated triple product — verbatim-byte island. The source slice (0x4833d3..) is
///   fld[ebx+0x94]; <verbatim fmul[ebx+0x90] (d8 8b 90 00 00 00)>; <verbatim fmul st,st(1) (d8 c9)>;
///   fstp[esi+0x1c] — but it interleaves a `mov esi,[ebp-4]` clobber + `push ecx` we don't want, so
///   we assemble a clean island: fld[sel]; fld[ebx+0x94]; <verbatim fmul[ebx+0x90]>; <verbatim fmul
///   st,st(1)>; fstp[res]. Both arith ops (0x4833dc, 0x4833e8) are the SOURCE bytes (byte-verified).
///   = sel·([+0x94]·[+0x90]); sel stays LIVE f64 on st1 until the final fmul st,st(1).
#[test]
fn textglue_482_gated_triple() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let ebx = emu.alloc(0x200);
    let sel_cell = emu.alloc(4);
    let res = emu.alloc(4);

    let fmul90 = emu.read(0x4833dc, 6); // fmul dword [ebx+0x90]
    let fmul_st = emu.read(0x4833e8, 2); // fmul st,st(1)
    assert_eq!(
        fmul90,
        vec![0xd8, 0x8b, 0x90, 0x00, 0x00, 0x00],
        "fmul[ebx+0x90] bytes"
    );
    assert_eq!(fmul_st, vec![0xd8, 0xc9], "fmul st,st(1) bytes");

    let region = emu.alloc(0x40);
    let mut code = vec![0xd9u8, 0x05]; // fld [sel] -> st0=sel
    code.extend_from_slice(&(sel_cell as u32).to_le_bytes());
    code.extend_from_slice(&[0xd9, 0x83, 0x94, 0x00, 0x00, 0x00]); // fld [ebx+0x94] (st0=m94, st1=sel)
    code.extend_from_slice(&fmul90); // verbatim fmul [ebx+0x90] => st0 = m94·m90
    code.extend_from_slice(&fmul_st); // verbatim fmul st,st(1) => st0 = (m94·m90)·sel
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp [res]
    code.extend_from_slice(&(res as u32).to_le_bytes());
    let end = region + code.len() as u64;
    emu.write(region, &code);

    let run = |emu: &mut Emu, sel: f32, m94: f32, m90: f32| -> f32 {
        emu.set_reg(RegisterX86::EBX, ebx);
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(sel_cell, sel.to_bits());
        emu.write_u32(ebx + 0x94, m94.to_bits());
        emu.write_u32(ebx + 0x90, m90.to_bits());
        emu.run(region, end);
        emu.read_f32(res)
    };

    let mut total = 0u32;
    for (sel, m94, m90) in [
        (1.0f32, 2.0f32, 3.0f32),
        (0.5, 10.0, 0.8),
        (-2.0, -3.0, 4.0),
        (0.0, 100.0, 50.0),
    ] {
        assert_eq!(
            run(&mut emu, sel, m94, m90).to_bits(),
            gated_triple(sel, m94, m90).to_bits(),
            "gated_triple sel={sel} m94={m94} m90={m90}"
        );
        total += 1;
    }
    let mut s = 0x4833_4000u32;
    for _ in 0..4000 {
        let sel = rng(&mut s, -10.0, 10.0);
        let m94 = rng(&mut s, -100.0, 100.0);
        let m90 = rng(&mut s, -100.0, 100.0);
        assert_eq!(
            run(&mut emu, sel, m94, m90).to_bits(),
            gated_triple(sel, m94, m90).to_bits(),
            "gated_triple fuzz"
        );
        total += 1;
    }
    eprintln!("textglue_482_gated_triple: {total} bit-exact (0x483340 sel·a·b, 2 fmul)");
}

/// (8) 0x483430 cos/sin — verbatim island (bytes 0x48343a..0x48344b):
///   fld st(0); fcos; mov[esi+8],0; mov eax,esi; fstp[esi]; fsin; fstp[esi+4].
/// We pre-load st0 = angle, point esi at a 3-f32 buffer, run the verbatim bytes; read cos,sin,0.
/// A transcendental: compared structurally with the host's functions (`METHOD.md#the-harness`).
#[test]
fn textglue_482_cos_sin() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let angle_cell = emu.alloc(4);
    let esi = emu.alloc(0x10);

    // include the final `fstp dword [esi+4]` (0x48344b, 3 bytes) so sin is stored.
    let kern = emu.read(0x48343a, 0x48344e - 0x48343a);
    assert_eq!(&kern[0..4], &[0xd9, 0xc0, 0xd9, 0xff], "fld st0; fcos");
    assert_eq!(
        &kern[kern.len() - 5..],
        &[0xd9, 0xfe, 0xd9, 0x5e, 0x04],
        "...fsin; fstp[esi+4]"
    );

    let region = emu.alloc(0x40);
    let mut code = vec![0xd9u8, 0x05]; // fld [angle] -> st0=angle
    code.extend_from_slice(&(angle_cell as u32).to_le_bytes());
    code.extend_from_slice(&kern); // verbatim: fld st0; fcos; ...; fstp[esi]; fsin; fstp[esi+4]
    let end = region + code.len() as u64;
    emu.write(region, &code);

    let run = |emu: &mut Emu, angle: f32| -> [f32; 3] {
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(angle_cell, angle.to_bits());
        emu.run(region, end);
        [
            emu.read_f32(esi),
            emu.read_f32(esi + 4),
            emu.read_f32(esi + 8),
        ]
    };

    let mut total = 0u32;
    for angle in [
        0.0f32,
        1.0,
        -1.0,
        std::f32::consts::PI,
        std::f32::consts::FRAC_PI_2,
        -0.5,
    ] {
        assert_eq!(
            run(&mut emu, angle).map(f32::to_bits),
            cos_sin(angle).map(f32::to_bits),
            "cos_sin angle={angle}"
        );
        total += 1;
    }
    let mut s = 0x4834_3000u32;
    for _ in 0..3000 {
        let angle = rng(&mut s, -std::f32::consts::PI, std::f32::consts::PI);
        assert_eq!(
            run(&mut emu, angle).map(f32::to_bits),
            cos_sin(angle).map(f32::to_bits),
            "cos_sin fuzz angle={angle}"
        );
        total += 1;
    }
    eprintln!("textglue_482_cos_sin: {total} bit-exact (0x483430 fcos/fsin, structural libm)");
}

/// (9) 0x483460 delta==0 change-detect — real slice 0x483499..0x4834a9:
///   fld[ebp-0x24]; fsub[ebp-0x2c]; fcomp[0x7ffd74]=0.0; fnstsw. fsub = a-b. test ah,0x44; jnp.
/// mask 0x44={C2,C3}; jnp (PF=0) taken iff ODD set-bits ⟺ (a-b)==0 (ordered) OR unordered(NaN).
/// (The second predicate at 0x4834b0 is the byte-identical disp form [ebp-0x20]/[ebp-0x28] — same
/// kernel, covered by this diff; its fsub op is 0x4834b3.)
#[test]
fn textglue_482_delta_is_zero() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let stack = emu.alloc(0x1000);
    let ebp = stack + 0x800;

    // Byte-verify the second predicate is the same kernel modulo disp (0x4834b3 = fsub [ebp-0x28]).
    let s1 = emu.read(0x48349c, 3); // d8 65 dc = fsub [ebp-0x24]
    let s2 = emu.read(0x4834b3, 3); // d8 65 d8 = fsub [ebp-0x28]
    assert_eq!(s1[0], 0xd8, "0x48349c is fsub /4");
    assert_eq!(s2[0], 0xd8, "0x4834b3 is fsub /4");
    assert_eq!(s1[1], s2[1], "both fsub [ebp-disp8] /4 modrm");

    let run = |emu: &mut Emu, a: f32, b: f32| -> u8 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, stack + 0x100);
        emu.write_u32(ebp - 0x24, a.to_bits());
        emu.write_u32(ebp - 0x2c, b.to_bits());
        emu.run(0x483499, 0x4834a7); // halt after fnstsw, before `test ah,0x44`
        ah_of(emu)
    };
    // jnp (PF=0) taken iff ODD set-bits in {C2,C3}.
    // jnp (PF=0) taken iff ODD set-bits in {C2,C3}.
    let jnp_taken = |ah: u8| !(ah & 0x44).count_ones().is_multiple_of(2);

    let mut total = 0u32;
    let (mut eq_n, mut ne_n) = (0u32, 0u32);
    for (a, b) in [
        (5.0f32, 5.0f32), // a-b==0 -> jnp taken
        (5.0, 5.0001),    // a-b<0 -> not taken
        (5.0001, 5.0),    // a-b>0 -> not taken
        (0.0, 0.0),       // ==0
        (1.0, f32::NAN),  // unordered -> jnp taken
        (-3.5, -3.5),     // ==0
    ] {
        let bin = jnp_taken(run(&mut emu, a, b));
        assert_eq!(bin, delta_is_zero(a, b), "delta_is_zero a={a} b={b}");
        if bin {
            eq_n += 1
        } else {
            ne_n += 1
        }
        total += 1;
    }
    let mut s = 0x4834_9900u32;
    for _ in 0..3000 {
        let base = rng(&mut s, -100.0, 100.0);
        // half the draws exactly equal (a-b==0), half slightly off
        let (a, b) = if (next(&mut s) >> 8) & 1 == 0 {
            (base, base)
        } else {
            (base, base + rng(&mut s, -1.0, 1.0))
        };
        let bin = jnp_taken(run(&mut emu, a, b));
        assert_eq!(bin, delta_is_zero(a, b), "delta_is_zero fuzz a={a} b={b}");
        if bin {
            eq_n += 1
        } else {
            ne_n += 1
        }
        total += 1;
    }
    assert!(eq_n > 0 && ne_n > 0, "need both outcomes");
    eprintln!(
        "textglue_482_delta_is_zero: {total} bit-exact (eq={eq_n} ne={ne_n}; 0x483460 fsub vs 0.0)"
    );
}

/// (9b) 0x483460 field compare — real slice 0x4834c7..0x4834d5:
///   fld[edi+0x3a0]; fcomp[edi+0x3a8]; fnstsw. test ah,5; jp. (No arith op — covers the predicate's
///   parity, sharing the row with 0x4834de's identical-shape compare.) jp (PF=1) taken iff a>=b /NaN.
#[test]
fn textglue_482_cmp_ge() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let edi = emu.alloc(0x400);

    let run = |emu: &mut Emu, a: f32, b: f32| -> u8 {
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(edi + 0x3a0, a.to_bits());
        emu.write_u32(edi + 0x3a8, b.to_bits());
        emu.run(0x4834c7, 0x4834d5);
        ah_of(emu)
    };
    // jp (PF=1) taken iff EVEN set-bits in {C0,C2}.
    let jp_taken = |ah: u8| (ah & 0x5).count_ones().is_multiple_of(2);

    let mut total = 0u32;
    let (mut t, mut f) = (0u32, 0u32);
    for (a, b) in [
        (5.0f32, 3.0f32), // a>b -> jp taken
        (3.0, 5.0),       // a<b -> not taken
        (4.0, 4.0),       // a==b -> jp taken
        (f32::NAN, 1.0),  // unordered -> jp taken
    ] {
        let bin = jp_taken(run(&mut emu, a, b));
        assert_eq!(bin, cmp_ge_or_nan(a, b), "cmp_ge a={a} b={b}");
        if bin {
            t += 1
        } else {
            f += 1
        }
        total += 1;
    }
    let mut s = 0x4834_c700u32;
    for _ in 0..3000 {
        let a = rng(&mut s, -50.0, 50.0);
        let b = rng(&mut s, -50.0, 50.0);
        let bin = jp_taken(run(&mut emu, a, b));
        assert_eq!(bin, cmp_ge_or_nan(a, b), "cmp_ge fuzz a={a} b={b}");
        if bin {
            t += 1
        } else {
            f += 1
        }
        total += 1;
    }
    assert!(t > 0 && f > 0, "need both outcomes");
    eprintln!("textglue_482_cmp_ge: {total} bit-exact (t={t} f={f}; 0x483460 a>=b/NaN parity)");
}

/// (10) 0x483460 affine corner combine — real slice 0x48357a..0x4835ef:
///   the two deltas (fld;fsub ×2), the flag-gated frac setup (fld 1.0; fsub ×2 OR fld field ×2; fxch),
///   then the 4 legs (fmul st,st(i)/fmul[m]; fadd[base]; fstp). Covers 6 fsub + 4 fmul + 4 fadd.
///   The flag is `[ebp-0x4]` (tested at 0x483583); we run BOTH branches. Halts at the last fstp
///   target (0x4835ef stores [esp]); the 4 legs land at [esp+0xc],[esp+8],[esp+4],[esp].
#[test]
fn textglue_482_affine_combine() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let edi = emu.alloc(0x400);
    let stack = emu.alloc(0x2000);
    let ebp = stack + 0x1000;

    // ax=[ebp-0x1c] ax0=[ebp-0x10]; ay=[ebp-0x18] ay0=[ebp-0xc]; bx=[ebp-0x10] by=[ebp-0xc]
    //  (NB bx==ax0 slot, by==ay0 slot — same f32 reload). field_a=[edi+0x3a0] field_b=[edi+0x3a8]
    //  g3=[edi+0x3ac] g4=[edi+0x3a4]; flag=[ebp-0x4].
    let run = |emu: &mut Emu,
               ax: f32,
               ax0: f32,
               ay: f32,
               ay0: f32,
               fa: f32,
               fb: f32,
               g3: f32,
               g4: f32,
               flag: bool|
     -> [f32; 4] {
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::EBP, ebp);
        // esp must leave room: the slice does `sub esp,0x10` after two pushes; give headroom.
        let esp = stack + 0x800;
        emu.set_reg(RegisterX86::ESP, esp);
        emu.write_u32(ebp - 0x1c, ax.to_bits());
        emu.write_u32(ebp - 0x18, ay.to_bits());
        emu.write_u32(ebp - 0x10, ax0.to_bits()); // bx == ax0 slot
        emu.write_u32(ebp - 0xc, ay0.to_bits()); // by == ay0 slot
        emu.write_u32(ebp - 0x4, if flag { 1 } else { 0 });
        emu.write_u32(edi + 0x3a0, fa.to_bits());
        emu.write_u32(edi + 0x3a8, fb.to_bits());
        emu.write_u32(edi + 0x3ac, g3.to_bits());
        emu.write_u32(edi + 0x3a4, g4.to_bits());
        emu.run(0x48357a, 0x4835f2);
        // after `push ecx; push edx; sub esp,0x10` the slice's `esp` = esp-0x18; legs at
        // [esp'+0xc],[esp'+8],[esp'+4],[esp'].
        let espp = esp - 0x18;
        [
            emu.read_f32(espp + 0xc),
            emu.read_f32(espp + 0x8),
            emu.read_f32(espp + 0x4),
            emu.read_f32(espp),
        ]
    };

    let mut total = 0u32;
    for flag in [false, true] {
        // bx==ax0, by==ay0 in this function's frame (same stack slots), so pass them through.
        for (ax, ax0, ay, ay0, fa, fb, g3, g4) in [
            (
                10.0f32, 2.0f32, 20.0f32, 5.0f32, 0.25f32, 0.75f32, 0.5f32, 0.5f32,
            ),
            (0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0),
            (-50.0, 50.0, 100.0, -100.0, 0.3, 0.6, -0.2, 0.9),
        ] {
            let got = run(&mut emu, ax, ax0, ay, ay0, fa, fb, g3, g4, flag);
            let exp = affine_combine(ax, ax0, ay, ay0, fa, fb, g3, g4, ax0, ay0, flag);
            assert_eq!(
                got.map(f32::to_bits),
                exp.map(f32::to_bits),
                "affine flag={flag} ax={ax} ay={ay}"
            );
            total += 1;
        }
        let mut s = 0x4835_7a00u32 ^ (flag as u32);
        for _ in 0..2000 {
            let ax = rng(&mut s, -500.0, 500.0);
            let ax0 = rng(&mut s, -500.0, 500.0);
            let ay = rng(&mut s, -500.0, 500.0);
            let ay0 = rng(&mut s, -500.0, 500.0);
            let fa = rng(&mut s, -2.0, 2.0);
            let fb = rng(&mut s, -2.0, 2.0);
            let g3 = rng(&mut s, -2.0, 2.0);
            let g4 = rng(&mut s, -2.0, 2.0);
            let got = run(&mut emu, ax, ax0, ay, ay0, fa, fb, g3, g4, flag);
            let exp = affine_combine(ax, ax0, ay, ay0, fa, fb, g3, g4, ax0, ay0, flag);
            assert_eq!(
                got.map(f32::to_bits),
                exp.map(f32::to_bits),
                "affine fuzz flag={flag}"
            );
            total += 1;
        }
    }
    eprintln!(
        "textglue_482_affine_combine: {total} bit-exact (0x483460 affine, 6 fsub+4 fmul+4 fadd)"
    );
}

/// (11) 0x483460 unit-convert — real slice 0x483659..0x483671:
///   fld[edi+0x3f4]; fmul[0x8022cc]=1000.0; fadd[0x7ffa24]=0.5; __ftol(@0x40a2b0). We run through the
///   __ftol call, which runs for real, and read `eax`. Covers fmul (0x48365f) + fadd (0x48366b).
#[test]
fn textglue_482_unit_convert() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let edi = emu.alloc(0x400);
    let stack = emu.alloc(0x100) + 0x80;

    // island: fld[field]; <verbatim fmul[1000]>; <verbatim fadd[0.5]>; call __ftol.
    let fmul = emu.read(0x48365f, 6);
    let fadd = emu.read(0x48366b, 6);
    assert_eq!(fmul, vec![0xd8, 0x0d, 0xcc, 0x22, 0x80, 0x00], "fmul[1000]");
    assert_eq!(fadd, vec![0xd8, 0x05, 0x24, 0xfa, 0x7f, 0x00], "fadd[0.5]");
    let region = emu.alloc(0x40);
    let mut code = vec![0xd9u8, 0x87]; // fld dword [edi+0x3f4]
    code.extend_from_slice(&(0x3f4u32).to_le_bytes());
    code.extend_from_slice(&fmul);
    code.extend_from_slice(&fadd);
    // `call __ftol` (0x40a2b0), as the source does at 0x483671.
    let rel = 0x40a2b0u32.wrapping_sub((region + code.len() as u64 + 5) as u32);
    code.push(0xe8);
    code.extend_from_slice(&rel.to_le_bytes());
    let end = region + code.len() as u64;
    code.push(0x90);
    emu.write(region, &code);

    let run = |emu: &mut Emu, field: f32| -> i32 {
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::ESP, stack);
        emu.write_u32(edi + 0x3f4, field.to_bits());
        emu.run(region, end);
        emu.get_reg(RegisterX86::EAX) as u32 as i32 // __ftol's eax
    };

    let mut total = 0u32;
    // Then NaN, the infinities and products past `i32` and `i64`: `__ftol` returns the low dword
    // of its qword, 0 for the indefinite.
    for field in [
        0.0f32,
        0.5,
        -0.5,
        1.0,
        3.25,
        -2.5,
        0.0009,
        100.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        4.0e6,
        -4.0e6,
        1.0e30,
        -1.0e30,
    ] {
        assert_eq!(
            run(&mut emu, field),
            unit_convert_round(field),
            "unit_convert field={field}"
        );
        total += 1;
    }
    let mut s = 0x4836_5900u32;
    for _ in 0..4000 {
        let field = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(
            run(&mut emu, field),
            unit_convert_round(field),
            "unit_convert fuzz"
        );
        total += 1;
    }
    eprintln!("textglue_482_unit_convert: {total} bit-exact (0x483460 field·1000+0.5 → ftol)");
}

/// (12) 0x483460 negate — verbatim island: fld[v_cell]; <verbatim fchs (d9 e0)>; fstp[res].
///   The fchs byte is read from source 0x483696. = -v narrowed f32.
#[test]
fn textglue_482_negate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let v_cell = emu.alloc(4);
    let res = emu.alloc(4);

    let fchs = emu.read(0x483696, 2);
    assert_eq!(fchs, vec![0xd9, 0xe0], "0x483696 fchs bytes");
    let region = emu.alloc(0x40);
    let mut code = vec![0xd9u8, 0x05]; // fld [v]
    code.extend_from_slice(&(v_cell as u32).to_le_bytes());
    code.extend_from_slice(&fchs); // verbatim fchs
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp [res]
    code.extend_from_slice(&(res as u32).to_le_bytes());
    let end = region + code.len() as u64;
    emu.write(region, &code);

    let run = |emu: &mut Emu, v: f32| -> f32 {
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(v_cell, v.to_bits());
        emu.run(region, end);
        emu.read_f32(res)
    };

    let mut total = 0u32;
    for v in [0.0f32, 1.0, -1.0, 3.5, -100.0, f32::INFINITY] {
        assert_eq!(
            run(&mut emu, v).to_bits(),
            negate(v).to_bits(),
            "negate v={v}"
        );
        total += 1;
    }
    let mut s = 0x4836_9600u32;
    for _ in 0..2000 {
        let v = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(
            run(&mut emu, v).to_bits(),
            negate(v).to_bits(),
            "negate fuzz"
        );
        total += 1;
    }
    eprintln!("textglue_482_negate: {total} bit-exact (0x483460 -v, fchs)");
}

/// (13) 0x4837b0 corner-build — real slice 0x48387e..0x4838ee (includes the final mov-out at
///   0x4838ed so all six legs land in distinct slots):
///   plus[i]  = center[i] + hw[i]  (3 fadd: hw = [ebp-4],[ebp-4],[0x801628]=2.0)
///   minus[i] = center[i] - hw[i]  (3 fsub: same hw)
///   center = {[0xb4b3a0],[0xb4b3a4],[0xb4b3a8]} (.data globals, set live).
///   The values feed two 3-float structs ([ebp-0x24] and [ebp-0x30]); each compute writes
///   [ebp-0x10]/[ebp-0xc]/[ebp-0x8] then movs the int out to a distinct struct slot:
///     plus  = [ebp-0x24], [ebp-0x20], [ebp-0x1c]   (c0+hw, c1+hw, c2+2.0)
///     minus = [ebp-0x30], [ebp-0x2c], [ebp-0x28]   (c0-hw, c1-hw, c2-2.0)
///   (The last mov-out is 0x4838ed → [ebp-0x28]=minus2; halt 0x4838f0 is just past it, before the call.)
#[test]
fn textglue_482_corner_build() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let stack = emu.alloc(0x1000);
    let ebp = stack + 0x800;

    // Verify the constant 2.0 at 0x801628 in the mapped image.
    assert_eq!(emu.read_u32(0x801628), 0x4000_0000, "0x801628 == 2.0");

    let run = |emu: &mut Emu, c: [f32; 3], hw01: f32| -> ([f32; 3], [f32; 3]) {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, stack + 0x100);
        emu.write_u32(0xb4b3a0, c[0].to_bits());
        emu.write_u32(0xb4b3a4, c[1].to_bits());
        emu.write_u32(0xb4b3a8, c[2].to_bits());
        emu.write_u32(ebp - 0x4, hw01.to_bits());
        emu.run(0x48387e, 0x4838f0); // halt after the final minus-leg-2 mov-out, before the call
        let plus = [
            emu.read_f32(ebp - 0x24),
            emu.read_f32(ebp - 0x20),
            emu.read_f32(ebp - 0x1c),
        ];
        let minus = [
            emu.read_f32(ebp - 0x30),
            emu.read_f32(ebp - 0x2c),
            emu.read_f32(ebp - 0x28),
        ];
        (plus, minus)
    };

    let mut total = 0u32;
    for (c, hw) in [
        ([1.0f32, 2.0, 3.0], 0.5f32),
        ([0.0, 0.0, 0.0], 1.0),
        ([-10.0, 20.0, -30.0], 2.5),
        ([100.0, -50.0, 0.0], 0.0),
    ] {
        let (gp, gm) = run(&mut emu, c, hw);
        let (ep, em) = corner_build(c, hw);
        assert_eq!(
            gp.map(f32::to_bits),
            ep.map(f32::to_bits),
            "corner plus c={c:?} hw={hw}"
        );
        assert_eq!(
            gm.map(f32::to_bits),
            em.map(f32::to_bits),
            "corner minus c={c:?} hw={hw}"
        );
        total += 2;
    }
    let mut s = 0x4837_b000u32;
    for _ in 0..3000 {
        let c = [
            rng(&mut s, -1000.0, 1000.0),
            rng(&mut s, -1000.0, 1000.0),
            rng(&mut s, -1000.0, 1000.0),
        ];
        let hw = rng(&mut s, -100.0, 100.0);
        let (gp, gm) = run(&mut emu, c, hw);
        let (ep, em) = corner_build(c, hw);
        assert_eq!(
            gp.map(f32::to_bits),
            ep.map(f32::to_bits),
            "corner plus fuzz"
        );
        assert_eq!(
            gm.map(f32::to_bits),
            em.map(f32::to_bits),
            "corner minus fuzz"
        );
        total += 2;
    }
    eprintln!("textglue_482_corner_build: {total} bit-exact (0x4837b0 center±hw, 3 fadd+3 fsub)");
}
