//! Differential tests of the arithmetic of `0x503c10`, `0x5132f0`,
//! `CInputControl::AccumulateMotion` (`0x514400`) and `CInputControl::HandOffPitch` (`0x515330`),
//! each run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice is diffed as an **instruction slice** (`Emu::run`) over just the owned
//! arithmetic — registers/memory/stack synthesized, the binary's own bytes executed, the stored
//! `fstp [m32]` value read back via `read_f32`. Every in-body FP-arith opcode lands in a diffed slice; the loads/stores/`fabs`/`fchs` (sign-bit, exact) ride
//! along inside the slices. Slice→op coverage:
//! * 0x503c10  slice 0x503c73..0x503ca5 — fsub 0x503c76, fmul 0x503c7f, fsub 0x503c99, fmul 0x503c9c
//! * 0x5132f0  slice 0x5132f3..0x51330e — fmul 0x5132f8, fmul 0x513301, fmul 0x513308
//! * 0x514400  slice 0x51441c..0x514435 — fabs 0x51441f, fadd 0x514421, fabs 0x51442d, fadd 0x51442f
//! * 0x515330  slice 0x515364..0x515370 — fchs 0x51536b
//!
//! Fuzz reads the LCG's HIGH bits.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::input_motion::{
    abs_accumulate, negate, offset_scale2, vec3_scale, K033_BITS, K05_BITS,
};

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
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x80;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { ebp, esp }
}

// ===========================================================================================
// 0x503c10 — two independent `(a − b)·k → f32` offset-scale stores.
// Slice 0x503c73..0x503ca5 (fld;fsub;fmul;fstp [ebp-0x14]; ...; fstp [ebp-0x10]), halt at the
// trailing `call 0x7ac250`. Inputs at [ebp-0x38]/[ebp-0x44] (chain1) and [ebp-0x34]/[ebp-0x40]
// (chain2); outputs at [ebp-0x14]/[ebp-0x10]. The interleaved `mov [ebp-0x18],0` is non-FP.
// fsub 0x503c76, fmul 0x503c7f (·0.33); fsub 0x503c99, fmul 0x503c9c (·0.5).
// ===========================================================================================
#[test]
fn glue_503_503c10_offset_scale2() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // returns (out0=[ebp-0x14], out1=[ebp-0x10]).
    let run = |emu: &mut Emu, a0: f32, b0: f32, a1: f32, b1: f32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x38, a0.to_bits());
        emu.write_u32(f.ebp - 0x44, b0.to_bits());
        emu.write_u32(f.ebp - 0x34, a1.to_bits());
        emu.write_u32(f.ebp - 0x40, b1.to_bits());
        emu.run(0x503c73, 0x503ca5); // fld;fsub;fmul;fstp ×2 — halt before call 0x7ac250
        (emu.read_f32(f.ebp - 0x14), emu.read_f32(f.ebp - 0x10))
    };

    let mut total = 0u32;
    // deterministic: zeros, equal (a-b=0), opposite signs, large magnitudes, sub-normal-ish.
    let probes: [(f32, f32, f32, f32); 8] = [
        (0.0, 0.0, 0.0, 0.0),
        (1.0, 1.0, 1.0, 1.0),
        (100.0, 37.0, -5.5, 12.25),
        (-1.0e6, 1.0e6, 1.0e6, -1.0e6),
        (1024.0, 0.0, 0.0, 1024.0),
        (0.1, 0.2, 0.3, 0.4),
        (
            f32::from_bits(0x4b00_0001),
            0.0,
            0.0,
            f32::from_bits(0x4b00_0001),
        ),
        (-7.5, 3.25, 11.0, -2.0),
    ];
    for (a0, b0, a1, b1) in probes {
        let (o0, o1) = run(&mut emu, a0, b0, a1, b1);
        let (w0, w1) = offset_scale2(a0, b0, a1, b1);
        assert_eq!(o0.to_bits(), w0.to_bits(), "offset_scale2 out0 ({a0},{b0})");
        assert_eq!(o1.to_bits(), w1.to_bits(), "offset_scale2 out1 ({a1},{b1})");
        total += 2;
    }
    let mut s = 0x503c_1000u32;
    for _ in 0..4000 {
        let a0 = rng(&mut s, -10000.0, 10000.0);
        let b0 = rng(&mut s, -10000.0, 10000.0);
        let a1 = rng(&mut s, -10000.0, 10000.0);
        let b1 = rng(&mut s, -10000.0, 10000.0);
        let (o0, o1) = run(&mut emu, a0, b0, a1, b1);
        let (w0, w1) = offset_scale2(a0, b0, a1, b1);
        assert_eq!(
            o0.to_bits(),
            w0.to_bits(),
            "offset_scale2 out0 fuzz ({a0},{b0})"
        );
        assert_eq!(
            o1.to_bits(),
            w1.to_bits(),
            "offset_scale2 out1 fuzz ({a1},{b1})"
        );
        total += 2;
    }
    let _ = (K033_BITS, K05_BITS);
    eprintln!("glue_503_503c10_offset_scale2: {total} bit-exact ((a−b)·0.33, (a−b)·0.5)");
}

// ===========================================================================================
// 0x5132f0 — in-place scale of a 3-element f32 vector by a scalar arg [ebp+8].
// Slice 0x5132f3..0x51330e (mov eax,ecx; 3× fld;fmul;fstp), halt before `pop ebp`. ecx = vec ptr;
// `mov eax,ecx` (0x5132f6) sets the base inside the slice. fmul 0x5132f8/0x513301/0x513308.
// ===========================================================================================
#[test]
fn glue_503_5132f0_vec3_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let vec = emu.alloc(0x10); // the 3-element f32 vector (ecx)

    // returns the three scaled elements [vec+0]/[vec+4]/[vec+8].
    let run = |emu: &mut Emu, arg: f32, v: [f32; 3]| -> [f32; 3] {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ECX, vec);
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.write_u32(vec, v[0].to_bits());
        emu.write_u32(vec + 0x4, v[1].to_bits());
        emu.write_u32(vec + 0x8, v[2].to_bits());
        emu.run(0x5132f3, 0x51330e); // 3× fld [ebp+8]; fmul [eax+N]; fstp [eax+N]
        [
            emu.read_f32(vec),
            emu.read_f32(vec + 0x4),
            emu.read_f32(vec + 0x8),
        ]
    };

    let mut total = 0u32;
    let probes: [(f32, [f32; 3]); 6] = [
        (0.0, [1.0, 2.0, 3.0]),
        (1.0, [1.0, 2.0, 3.0]),
        (2.5, [-4.0, 0.0, 17.5]),
        (-1.0, [1.0e6, -1.0e6, 0.001]),
        (0.33, [3.0, 6.0, 9.0]),
        (1234.5, [f32::from_bits(0x4b00_0001), -7.0, 0.5]),
    ];
    for (arg, v) in probes {
        let got = run(&mut emu, arg, v);
        let want = vec3_scale(arg, v);
        for i in 0..3 {
            assert_eq!(
                got[i].to_bits(),
                want[i].to_bits(),
                "vec3_scale[{i}] arg={arg}"
            );
        }
        total += 3;
    }
    let mut s = 0x5132_f000u32;
    for _ in 0..4000 {
        let arg = rng(&mut s, -5000.0, 5000.0);
        let v = [
            rng(&mut s, -5000.0, 5000.0),
            rng(&mut s, -5000.0, 5000.0),
            rng(&mut s, -5000.0, 5000.0),
        ];
        let got = run(&mut emu, arg, v);
        let want = vec3_scale(arg, v);
        for i in 0..3 {
            assert_eq!(
                got[i].to_bits(),
                want[i].to_bits(),
                "vec3_scale[{i}] fuzz arg={arg}"
            );
        }
        total += 3;
    }
    eprintln!("glue_503_5132f0_vec3_scale: {total} bit-exact (3× arg·v_i in place)");
}

// ===========================================================================================
// 0x514400 — accumulate `|src.x|`,`|src.y|` into a running (x,y) field pair.
// Slice 0x51441c..0x514435 (fld [edi+0x24]; fabs; fadd [esi+8]; fstp [esi+8]; fld [edi+0x28];
// mov [esi+0x10],eax; fabs; fadd [esi+0xc]; fstp [esi+0xc]), halt before `call 0x4818f0`.
// edi = src ([ebp+8]), esi = this (ecx). The interleaved `mov [esi+0x10],eax` is non-FP (eax is
// the prior callee's result — set to a sentinel; it stores to a mapped field). fabs 0x51441f,
// fadd 0x514421, fabs 0x51442d, fadd 0x51442f.
// ===========================================================================================
#[test]
fn glue_503_514400_abs_accumulate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let src = emu.alloc(0x40); // edi = [ebp+8]; reads [edi+0x24],[edi+0x28]
    let this = emu.alloc(0x40); // esi = ecx; reads/writes [esi+0x8],[esi+0xc]; writes [esi+0x10]

    // returns (acc_x'=[esi+0x8], acc_y'=[esi+0xc]).
    let run = |emu: &mut Emu, sx: f32, sy: f32, ax: f32, ay: f32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EDI, src);
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::EAX, 0xdead_beef); // [esi+0x10] sink, non-FP
        emu.write_u32(src + 0x24, sx.to_bits());
        emu.write_u32(src + 0x28, sy.to_bits());
        emu.write_u32(this + 0x8, ax.to_bits());
        emu.write_u32(this + 0xc, ay.to_bits());
        emu.run(0x51441c, 0x514435); // through both |s|+acc stores, halt before call 0x4818f0
        (emu.read_f32(this + 0x8), emu.read_f32(this + 0xc))
    };

    let mut total = 0u32;
    let probes: [(f32, f32, f32, f32); 8] = [
        (0.0, 0.0, 0.0, 0.0),
        (1.0, -1.0, 0.0, 0.0),
        (-3.5, 7.25, 100.0, 200.0),
        (1.0e6, -1.0e6, 1.0e6, 1.0e6),
        (-0.001, 0.001, -5.0, 5.0),
        (f32::from_bits(0x4b00_0001), -2.0, 1.0, -1.0),
        (-123.456, 0.0, -0.0, 64.0),
        (12.5, -12.5, 12.5, -12.5),
    ];
    for (sx, sy, ax, ay) in probes {
        let (gx, gy) = run(&mut emu, sx, sy, ax, ay);
        let (wx, wy) = abs_accumulate(sx, sy, ax, ay);
        assert_eq!(gx.to_bits(), wx.to_bits(), "abs_accumulate x ({sx},{ax})");
        assert_eq!(gy.to_bits(), wy.to_bits(), "abs_accumulate y ({sy},{ay})");
        total += 2;
    }
    let mut s = 0x5144_0000u32;
    for _ in 0..4000 {
        let sx = rng(&mut s, -10000.0, 10000.0);
        let sy = rng(&mut s, -10000.0, 10000.0);
        let ax = rng(&mut s, -10000.0, 10000.0);
        let ay = rng(&mut s, -10000.0, 10000.0);
        let (gx, gy) = run(&mut emu, sx, sy, ax, ay);
        let (wx, wy) = abs_accumulate(sx, sy, ax, ay);
        assert_eq!(
            gx.to_bits(),
            wx.to_bits(),
            "abs_accumulate x fuzz ({sx},{ax})"
        );
        assert_eq!(
            gy.to_bits(),
            wy.to_bits(),
            "abs_accumulate y fuzz ({sy},{ay})"
        );
        total += 2;
    }
    eprintln!("glue_503_514400_abs_accumulate: {total} bit-exact (|src.x|+acc, |src.y|+acc)");
}

// ===========================================================================================
// 0x515330 — negate one arg f32 and emit it as a call argument.
// Slice 0x515364..0x515370 (fld [ebp+0xc]; mov edx,[ebp+8]; push ecx; fchs; fstp [esp]), halt
// before `push edx`. The fstp stores `−arg` into the slot just pushed by `push ecx`; read it at
// the post-push esp (esp - 4). fchs 0x51536b.
// ===========================================================================================
#[test]
fn glue_503_515330_negate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // returns −arg, read from [esp] after the slice's `push ecx` (esp decremented by 4).
    let run = |emu: &mut Emu, arg: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ECX, 0); // pushed then overwritten by fstp
        emu.write_u32(f.ebp + 0xc, arg.to_bits()); // the arg negated
        emu.write_u32(f.ebp + 0x8, 0); // [ebp+8] read into edx (non-FP)
        emu.run(0x515364, 0x515370); // fld [ebp+0xc]; push ecx; fchs; fstp [esp]
        emu.read_f32(f.esp - 0x4) // the slot `push ecx` created, now holding −arg
    };

    let mut total = 0u32;
    for arg in [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        1234.5,
        -1234.5,
        1.0e6,
        -1.0e6,
        f32::from_bits(0x4b00_0001),
        f32::from_bits(0xcb00_0001),
    ] {
        assert_eq!(
            run(&mut emu, arg).to_bits(),
            negate(arg).to_bits(),
            "negate arg={arg}"
        );
        total += 1;
    }
    let mut s = 0x5153_3000u32;
    for _ in 0..4000 {
        let arg = rng(&mut s, -100000.0, 100000.0);
        assert_eq!(
            run(&mut emu, arg).to_bits(),
            negate(arg).to_bits(),
            "negate fuzz arg={arg}"
        );
        total += 1;
    }
    eprintln!("glue_503_515330_negate: {total} bit-exact (−arg via fchs)");
}
