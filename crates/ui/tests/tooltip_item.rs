//! The arithmetic inside `CGTooltip::BuildItem` (`0x52b650`) against the exe: ten x87 operations in
//! the per-bar loop `0x52c250` to `0x52c5a0`, each run as the slice that computes it.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::tooltip::{
    accumulate, bias_round_neg, bias_round_pos, final_divide, half_quantize, scale_001,
};
use fpkit::{next, rngf};

struct Frame {
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x200);
    let ebp = scratch + 0x100; // room for [ebp-0x100 .. ebp]
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00; // headroom both ways (the slices `sub esp,8`, `call __ftol` push)
    Frame { ebp, esp }
}

/// Ops 0x52c263 fsub [0.99999] + 0x52c27f fadd [0.99999] — the asymmetric ±bias round pair.
/// Slice 0x52c250..0x52c28a:
///   mov edi,[ebp-0x14]; fld [edi-0x14]; fcom [0.0]; fnstsw; test ah,0x41; je skip; fsub [0.99999];
///   call __ftol (-> esi via the mov esi,eax @0x52c276); fld [edi]; ...; jne skip; fadd [0.99999];
///   call __ftol (-> eax). Halt at 0x52c28a (`test esi,esi`): esi = neg-round(v0), eax = pos-round(v1).
/// Feed [ebp-0x14] -> a buffer with v0 at [-0x14], v1 at [0]. __ftol runs natively.
#[test]
fn textglue_52b_bias_round_pair() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let buf = emu.alloc(0x40);
    let ptr = buf + 0x20; // so [ptr-0x14] and [ptr] are both in-bounds

    let run = |emu: &mut Emu, v0: f32, v1: f32| -> (i32, i32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x14, ptr as u32); // [ebp-0x14] = ptr
        emu.write_u32(ptr - 0x14, v0.to_bits()); // [edi-0x14] = v0
        emu.write_u32(ptr, v1.to_bits()); // [edi] = v1
        emu.run(0x52c250, 0x52c28a);
        (
            emu.get_reg(RegisterX86::ESI) as i32, // the neg-bias round of v0
            emu.get_reg(RegisterX86::EAX) as i32, // the pos-bias round of v1
        )
    };

    let check = |emu: &mut Emu, total: &mut u32, v0: f32, v1: f32| {
        let (g20, g14) = run(emu, v0, v1);
        assert_eq!(g20, bias_round_neg(v0), "bias_round_neg v0={v0}");
        assert_eq!(g14, bias_round_pos(v1), "bias_round_pos v1={v1}");
        *total += 2;
    };

    let mut total = 0u32;
    // ==threshold boundary probes: the fcom [0.0] pivot (the AH branch flips at exactly 0.0/-0.0),
    // and the __ftol integer-truncate pivots around the ±0.99999 bias.
    let probes = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.99999,
        -0.99999,
        1.000_01,
        -1.000_01,
        1e-7,
        -1e-7,
        2.0,
        -2.0,
        2.999_99,
        -2.999_99,
        100.5,
        -100.5,
        0.000_009_5, // < 1-0.99999, biased positive stays < 1 -> trunc 0 vs unbiased trunc 0
        // NaN, the infinities and values past `i32` and `i64`: `__ftol` returns the low dword of
        // its qword, 0 for the indefinite.
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        3.0e9,
        -3.0e9,
        1.0e10,
        1.0e30,
        -1.0e30,
    ];
    for &v0 in &probes {
        for &v1 in &probes {
            check(&mut emu, &mut total, v0, v1);
        }
    }
    let mut s = 0x52c2_6300u32;
    for _ in 0..4000 {
        let v0 = rngf(&mut s, -2000.0, 2000.0) as f32;
        let v1 = rngf(&mut s, -2000.0, 2000.0) as f32;
        check(&mut emu, &mut total, v0, v1);
    }
    // tight sweep across zero where the two halves diverge most.
    for k in -50i32..=50 {
        let v = (k as f32) * 0.1;
        check(&mut emu, &mut total, v, v);
    }
    eprintln!(
        "textglue_52b_bias_round_pair: {total} bit-exact (fsub/fadd [0.99999] asym ±bias + native __ftol)"
    );
}

/// Ops 0x52c2fc fmul [0.5] + 0x52c3c5 fmul [0.5] — the two (s20+s14)*0.5 half-quantizes.
/// Slice A 0x52c2ed..0x52c305:  add edi,esi; mov [ebp-0x28],edi; fild [ebp-0x28]; mov ecx;
///   fmul dword [0.5]; fstp dword [ebp-0x28].  Slice B 0x52c3b6..0x52c3ce (identical FP).
/// Feed edi=s14, esi=s20; read f32 at [ebp-0x28].
#[test]
fn textglue_52b_half_quantize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, start: u64, end: u64, s20: i32, s14: i32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, s20 as u32 as u64); // esi = the first rounded value
        emu.set_reg(RegisterX86::EDI, s14 as u32 as u64); // edi = the second rounded value
        emu.run(start, end);
        emu.read_f32(f.ebp - 0x28)
    };

    let blocks = [(0x52c2edu64, 0x52c305u64), (0x52c3b6, 0x52c3ce)];
    let check = |emu: &mut Emu, total: &mut u32, s20: i32, s14: i32| {
        let want = half_quantize(s20, s14);
        for &(start, end) in &blocks {
            assert_eq!(
                run(emu, start, end, s20, s14).to_bits(),
                want.to_bits(),
                "half_quantize @ {start:#x} s20={s20} s14={s14}"
            );
            *total += 1;
        }
    };

    let mut total = 0u32;
    // ==threshold: odd sums (·0.5 lands on an exact .5 fraction — the f32 store is exact for small
    // magnitudes) and the i32-wrap boundary of `add edi,esi`.
    let probes = [
        (0i32, 0i32),
        (1, 0),
        (0, 1),
        (1, 1),
        (2, 1),
        (3, 0),
        (-1, 0),
        (-1, -1),
        (-3, 0),
        (5, -2),
        (1000, 1001),
        (i32::MAX, 1),  // wrap to i32::MIN
        (i32::MIN, -1), // wrap to i32::MAX
        (i32::MAX, i32::MAX),
        (16_777_217, 0), // > 2^24: f32 round of the ·0.5 product
        (-16_777_217, 0),
    ];
    for &(a, b) in &probes {
        check(&mut emu, &mut total, a, b);
    }
    let mut s = 0x52c2_fc00u32;
    for _ in 0..4000 {
        let a = next(&mut s) as i32;
        let b = next(&mut s) as i32;
        check(&mut emu, &mut total, a, b);
    }
    eprintln!("textglue_52b_half_quantize: {total} bit-exact (2x fild;fmul [0.5];fstp dword)");
}

/// Op 0x52c4ce fmul [0.001] — the int at +0x110 times 0.001, pushed f64.
/// Slice 0x52c4bf..0x52c4dd:  fild [esi+0x110]; sub esp,8; lea edx; fmul dword [0.001];
///   lea eax; fstp qword [esp].  Feed [esi+0x110]=x; read f64 at [esp-8].
#[test]
fn textglue_52b_scale_001() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let obj = emu.alloc(0x200); // so [esi+0x110] is in-bounds

    let run = |emu: &mut Emu, x: i32| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, obj); // esi = obj base ([esi+0x110] = x)
        emu.write_u32(obj + 0x110, x as u32);
        emu.run(0x52c4bf, 0x52c4dd);
        let bytes = emu.read(f.esp - 0x8, 8); // fstp qword [esp] after the slice's `sub esp,8`
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };

    let mut total = 0u32;
    // ==threshold: x at the f32-exact / f32-rounding boundaries of the · 0.001f product.
    let probes = [
        0i32,
        1,
        -1,
        2,
        10,
        100,
        1000,
        -1000,
        1_000_000,
        -1_000_000,
        16_777_216,
        16_777_217,
        i32::MAX,
        i32::MIN,
        12345,
        -98765,
    ];
    for &x in &probes {
        assert_eq!(
            run(&mut emu, x).to_bits(),
            scale_001(x).to_bits(),
            "scale_001 x={x}"
        );
        total += 1;
    }
    let mut s = 0x52c4_ce00u32;
    for _ in 0..4000 {
        let x = next(&mut s) as i32;
        assert_eq!(
            run(&mut emu, x).to_bits(),
            scale_001(x).to_bits(),
            "scale_001 fuzz x={x}"
        );
        total += 1;
    }
    eprintln!("textglue_52b_scale_001: {total} bit-exact (fild;fmul dword [0.001];fstp qword)");
}

/// Ops 0x52c528 fadd [eax-0x14] + 0x52c52b fmul [0.5] + 0x52c531 fadd [ebp-0x24] — accumulate.
/// Slice 0x52c51c..0x52c537:  mov eax,[ebp-0x14]; fld [eax]; mov [ebp-0x10],0; fadd [eax-0x14];
///   fmul dword [0.5]; fadd dword [ebp-0x24]; fstp dword [ebp-0x24].
/// Feed [ebp-0x14]=ptr (a at [ptr], b at [ptr-0x14]); acc at [ebp-0x24]; read f32 at [ebp-0x24].
#[test]
fn textglue_52b_accumulate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let buf = emu.alloc(0x40);
    let ptr = buf + 0x20;

    let run = |emu: &mut Emu, a: f32, b: f32, acc: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x14, ptr as u32); // [ebp-0x14] = ptr
        emu.write_u32(ptr, a.to_bits()); // [eax]      = a
        emu.write_u32(ptr - 0x14, b.to_bits()); // [eax-0x14] = b
        emu.write_u32(f.ebp - 0x24, acc.to_bits()); // [ebp-0x24] = acc
        emu.run(0x52c51c, 0x52c537);
        emu.read_f32(f.ebp - 0x24)
    };

    let check = |emu: &mut Emu, total: &mut u32, a: f32, b: f32, acc: f32| {
        assert_eq!(
            run(emu, a, b, acc).to_bits(),
            accumulate(a, b, acc).to_bits(),
            "accumulate a={a} b={b} acc={acc}"
        );
        *total += 1;
    };

    let mut total = 0u32;
    // ==threshold: acc==0.0 (the additive identity), a+b==0 cancellation, and the f32-store boundary.
    let probes = [
        (0.0f32, 0.0f32, 0.0f32),
        (1.0, 1.0, 0.0),
        (1.0, -1.0, 5.0),
        (2.5, 1.5, -2.0),
        (1e-7, 1e-7, 1.0),
        (1e7, 1e7, 1e7),
        (0.1, 0.2, 0.3),
        (-3.0, 3.0, 0.0),
        (16_777_217.0, 1.0, 0.0),
    ];
    for &(a, b, acc) in &probes {
        check(&mut emu, &mut total, a, b, acc);
    }
    let mut s = 0x52c5_2800u32;
    for _ in 0..4000 {
        let a = rngf(&mut s, -1e4, 1e4) as f32;
        let b = rngf(&mut s, -1e4, 1e4) as f32;
        let acc = rngf(&mut s, -1e4, 1e4) as f32;
        check(&mut emu, &mut total, a, b, acc);
    }
    eprintln!(
        "textglue_52b_accumulate: {total} bit-exact (fld;fadd [b];fmul [0.5];fadd [acc];fstp dword)"
    );
}

/// Ops 0x52c595 fmul [0.001] + 0x52c59b fdivr [ebp-0x24] — final divide acc / ((int)x*0.001).
/// Slice 0x52c57d..0x52c5a1:  mov edx,[ebp-0x4]; fild [edx+0x110]; sub esp,8; lea eax; lea ecx;
///   fmul dword [0.001]; fdivr dword [ebp-0x24]; fstp qword [esp].
/// fdivr [ebp-0x24] = [ebp-0x24] / st0 (reverse). Feed [edx+0x110]=x, [ebp-0x24]=acc; read f64 [esp-8].
#[test]
fn textglue_52b_final_divide() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let obj = emu.alloc(0x200);

    let run = |emu: &mut Emu, x: i32, acc: f32| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x4, obj as u32); // [ebp-0x4] = obj base ([edx+0x110]=x)
        emu.write_u32(obj + 0x110, x as u32);
        emu.write_u32(f.ebp - 0x24, acc.to_bits()); // [ebp-0x24] = acc (the dividend)
        emu.run(0x52c57d, 0x52c5a1);
        let bytes = emu.read(f.esp - 0x8, 8); // fstp qword [esp] after `sub esp,8`
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };

    let check = |emu: &mut Emu, total: &mut u32, x: i32, acc: f32| {
        assert_eq!(
            run(emu, x, acc).to_bits(),
            final_divide(x, acc).to_bits(),
            "final_divide x={x} acc={acc}"
        );
        *total += 1;
    };

    let mut total = 0u32;
    // ==threshold: acc==0.0 (numerator zero -> ±0.0 by divisor sign), x==0 (divisor 0 -> ±inf/NaN),
    // and the reverse-divide swap (dividend = acc, divisor = x·0.001) must hold at these pivots.
    let probes = [
        (1i32, 0.0f32),
        (1, 1.0),
        (1, -1.0),
        (1000, 500.0),
        (-1000, 500.0),
        (0, 1.0),  // divisor 0 -> +inf
        (0, -1.0), // divisor 0 -> -inf
        (0, 0.0),  // 0/0 -> NaN (bit-exact NaN must match)
        (2, 3.0),
        (i32::MAX, 1.0),
        (i32::MIN, 1.0),
        (12345, 67.89),
    ];
    for &(x, acc) in &probes {
        check(&mut emu, &mut total, x, acc);
    }
    let mut s = 0x52c5_9500u32;
    for _ in 0..4000 {
        let x = next(&mut s) as i32;
        let acc = rngf(&mut s, -1e5, 1e5) as f32;
        check(&mut emu, &mut total, x, acc);
    }
    eprintln!(
        "textglue_52b_final_divide: {total} bit-exact (fild;fmul [0.001];fdivr [acc] reverse-div;fstp qword)"
    );
}
