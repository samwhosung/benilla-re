//! The colour conversions and the arithmetic around them, as slices: `ConsoleSetShadowColor`
//! (`0x673290`) at `0x6732e2`; `GatherSlotLights` (`0x68df70`) from `0x68df92` to `0x68dfb9`, from
//! `0x68e0fd` to `0x68e130` and from `0x68e0b2` to `0x68e0c2`; `DrawInteriorGroupBatches` (`0x6b5190`)
//! from `0x6b531c` to `0x6b5335`; and `UpdateFootprints` (`0x69a3e0`) from `0x69a431` to
//! `0x69a48c`, from `0x69a53b` to `0x69a54a` and from `0x69a54f` to `0x69a583`. A stretch run in
//! an island is the exe's own bytes, checked, copied between a load and a store.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::colornorm::{
    color_normalize, color_scale255, corner_ray_negate, doodad_plane_value, footprint_alpha,
    footprint_fade_frac, footprint_gate_scales, footprint_jitter, light_mid_y, INV_255, INV_6000,
    SCALE_8106D8,
};
use common::{assert_f32, next};

/// A stack pointer inside the harness's stack, for slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// AH after `fnstsw ax`.
fn read_ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

const C7BCB0: u64 = 0xc7bcb0;

const C7BCB4: u64 = 0xc7bcb4;

const C7BCB8: u64 = 0xc7bcb8;

const C7BCBC: u64 = 0xc7bcbc;

const C62964: u64 = 0xc62964;

const CORNER_X: u64 = 0xc7cf20;

const CORNER_Y: u64 = 0xc7cf24;

const CORNER_Z: u64 = 0xc7cf28;

/// `0x6732e2`, in an island: the exe's `fmul dword [0x7ffe58]` between `fld dword [in]` and `fstp
/// dword [out]`, which rounds the register as `__ftol` would read it. The other three sites are the
/// same instruction on other slots.
#[test]
fn color_scale255_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // The exe's six bytes at 0x6732e2 must be `d8 0d 58 fe 7f 00`.
    const FMUL_VA: u64 = 0x6732e2;
    let fmul = e.read(FMUL_VA, 6);
    assert_eq!(
        fmul,
        vec![0xd8, 0x0d, 0x58, 0xfe, 0x7f, 0x00],
        "0x6732e2 fmul [0x7ffe58] bytes drifted"
    );

    let inp = e.alloc(0x10);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x40);
    // the island: fld dword [inp] (d9 05), the fmul, fstp dword [out] (d9 1d)
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(inp as u32).to_le_bytes());
    code.extend_from_slice(&fmul);
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let end = isl + code.len() as u64;
    e.write(isl, &code);

    let run_case = |e: &mut Emu, v: f32, ctx: &str| {
        e.write_u32(inp, v.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(isl, end);
        let bin = e.read_f32(out);
        // the register's value, rounded by the island's store
        let ours = color_scale255(v) as f32;
        assert_f32(bin, ours, ctx);
    };

    // The function admits only [0, 1]; the product is the same for any input, so 0, 1 and values
    // outside are run.
    run_case(&mut e, 0.0, "x255 v==0.0");
    run_case(&mut e, 1.0, "x255 v==1.0"); // → 255.0 exact
    run_case(&mut e, -0.0, "x255 v==-0.0");
    run_case(&mut e, 0.5, "x255 v==0.5"); // → 127.5
    run_case(&mut e, 2.0, "x255 v>1"); // → 510.0
    run_case(&mut e, -1.0, "x255 v<0"); // → -255.0
    run_case(
        &mut e,
        f32::from_bits(1.0f32.to_bits() - 1),
        "x255 just-below-1",
    );

    let mut s = 0x6732_e2e2u32;
    for i in 0..4000u32 {
        let v = match (next(&mut s) >> 8) % 3 {
            0 => frand(&mut s, 0.0, 1.0),
            1 => frand(&mut s, -2.0, 0.0),
            _ => frand(&mut s, 1.0, 3.0),
        };
        run_case(&mut e, v, &format!("x255 fuzz[{i}] v={v}"));
    }
}

/// `0x68df92` to `0x68dfb9`: `f32((hi − lo)·0.5 + lo)` of `[edi+8]` and `[edi+4]`, stored to
/// `[ebp-8]`.
#[test]
fn light_mid_y_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let edi = e.alloc(0x20);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    let run_case = |e: &mut Emu, hi: f32, lo: f32, ctx: &str| {
        e.write_u32(edi + 4, lo.to_bits());
        e.write_u32(edi + 8, hi.to_bits());
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // fld [edi+8]; fsub [edi+4]; fmul [0.5]; fadd [edi+4]; fstp [ebp-8] at 0x68dfb6
        e.run(0x68df92, 0x68dfb9);
        let bin = e.read_f32(ebp - 0x8);
        assert_f32(bin, light_mid_y(hi, lo), ctx);
    };

    run_case(&mut e, 0.0, 0.0, "mid zero");
    run_case(&mut e, 1.0, 0.0, "mid 1-0"); // → 0.5
    run_case(&mut e, 2.0, 2.0, "mid equal"); // → 2.0
    run_case(&mut e, -1.0, 1.0, "mid -1,1"); // → 0.0
    let mut s = 0x68df_9292u32;
    for i in 0..3000u32 {
        let hi = frand(&mut s, -1000.0, 1000.0);
        let lo = frand(&mut s, -1000.0, 1000.0);
        run_case(&mut e, hi, lo, &format!("mid fuzz[{i}]"));
    }
}

/// `0x68e0fd` to `0x68e128`, in an island of the exe's bytes with a store after it: the value left
/// for the compare, from `[ebp-0x1c]` = z60, `[ebp-0x18]` = mid, `[ebp-0x20]` = z, `[ebp-0x14]` =
/// bias and the plane's four globals.
#[test]
fn doodad_plane_value_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    const ISL_VA: u64 = 0x68e0fd;
    const ISL_LEN: usize = 0x68e128 - 0x68e0fd;
    let body = e.read(ISL_VA, ISL_LEN);
    // first `fld dword [ebp-0x1c]` (d9 45 e4), last `fsub [ebp-0x14]` (d8 65 ec)
    assert_eq!(
        &body[0..3],
        &[0xd9, 0x45, 0xe4],
        "0x68e0fd fld[ebp-0x1c] drifted"
    );
    assert_eq!(
        &body[ISL_LEN - 3..],
        &[0xd8, 0x65, 0xec],
        "0x68e125 fsub[ebp-0x14] drifted"
    );

    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;
    let mut code = body.clone();
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let end = isl + code.len() as u64;
    e.write(isl, &code);

    #[allow(clippy::too_many_arguments)]
    let run_case = |e: &mut Emu,
                    z: f32,
                    z60: f32,
                    mid: f32,
                    b0: f32,
                    b4: f32,
                    b8: f32,
                    bc: f32,
                    bias: f32,
                    ctx: &str| {
        e.write_u32(ebp - 0x1c, z60.to_bits());
        e.write_u32(ebp - 0x18, mid.to_bits());
        e.write_u32(ebp - 0x20, z.to_bits());
        e.write_u32(ebp - 0x14, bias.to_bits());
        e.write_u32(C7BCB0, b0.to_bits());
        e.write_u32(C7BCB4, b4.to_bits());
        e.write_u32(C7BCB8, b8.to_bits());
        e.write_u32(C7BCBC, bc.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        // the register's value, rounded by the island's store
        let bin = e.read_f32(out);
        let ours = doodad_plane_value(z, z60, mid, b0, b4, b8, bc, bias) as f32;
        assert_f32(bin, ours, ctx);
    };

    // the bias is the function's immediate, 23.570166 (0x41bc8fb3)
    let bias = f32::from_bits(0x41bc_8fb3);
    run_case(
        &mut e,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        "plane all-zero",
    );
    run_case(
        &mut e,
        1.0,
        1.0,
        1.0,
        1.0,
        1.0,
        1.0,
        1.0,
        bias,
        "plane unit",
    );
    let mut s = 0x68e0_fdfdu32;
    for i in 0..3000u32 {
        let z = frand(&mut s, -500.0, 500.0);
        let z60 = frand(&mut s, -500.0, 500.0);
        let mid = frand(&mut s, -500.0, 500.0);
        let b0 = frand(&mut s, -2.0, 2.0);
        let b4 = frand(&mut s, -2.0, 2.0);
        let b8 = frand(&mut s, -2.0, 2.0);
        let bc = frand(&mut s, -50.0, 50.0);
        run_case(
            &mut e,
            z,
            z60,
            mid,
            b0,
            b4,
            b8,
            bc,
            bias,
            &format!("plane fuzz[{i}]"),
        );
    }
}

/// `0x68e0fd` to `0x68e130`: the value, then `fcomp [0xc62964]; fnstsw ax`; AH against
/// `test ah,5; jp`, with the value equal to the threshold.
#[test]
fn doodad_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    // `fcomp` sets C0 below, C3 equal, all three unordered; `test ah,5; jp 0x68e199` jumps on an even
    // count of C0 and C2, so a value at or above the threshold skips.
    let gate_taken = |val: f64, thr: f32| -> bool { val >= f64::from(thr) };

    #[allow(clippy::too_many_arguments)]
    let run_case = |e: &mut Emu,
                    z: f32,
                    z60: f32,
                    mid: f32,
                    b0: f32,
                    b4: f32,
                    b8: f32,
                    bc: f32,
                    bias: f32,
                    thr: f32,
                    ctx: &str| {
        e.write_u32(ebp - 0x1c, z60.to_bits());
        e.write_u32(ebp - 0x18, mid.to_bits());
        e.write_u32(ebp - 0x20, z.to_bits());
        e.write_u32(ebp - 0x14, bias.to_bits());
        e.write_u32(C7BCB0, b0.to_bits());
        e.write_u32(C7BCB4, b4.to_bits());
        e.write_u32(C7BCB8, b8.to_bits());
        e.write_u32(C7BCBC, bc.to_bits());
        e.write_u32(C62964, thr.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // to the `test ah,5` at 0x68e130
        e.run(0x68e0fd, 0x68e130);
        let ah = read_ah(e);
        // the jump is taken on an even count
        let bin_taken = (ah & 0x5).count_ones().is_multiple_of(2);
        let val = doodad_plane_value(z, z60, mid, b0, b4, b8, bc, bias);
        assert_eq!(
            bin_taken,
            gate_taken(val, thr),
            "{ctx}: binary jp-taken={bin_taken} ours={} (val={val} thr={thr} ah={ah:#04x})",
            gate_taken(val, thr)
        );
    };

    let bias = f32::from_bits(0x41bc_8fb3);
    // On the threshold: a zero plane and bias give 0, and the threshold is 0.
    run_case(
        &mut e,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        "gate val==thr (0==0)",
    );
    // one clearly above, one clearly below
    run_case(
        &mut e,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        100.0,
        0.0,
        1.0,
        "gate above (100>=1)",
    );
    run_case(
        &mut e,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        -100.0,
        0.0,
        1.0,
        "gate below (-100<1)",
    );
    let mut s = 0x68e1_2828u32;
    for i in 0..3000u32 {
        let z = frand(&mut s, -50.0, 50.0);
        let z60 = frand(&mut s, -50.0, 50.0);
        let mid = frand(&mut s, -50.0, 50.0);
        let b0 = frand(&mut s, -2.0, 2.0);
        let b4 = frand(&mut s, -2.0, 2.0);
        let b8 = frand(&mut s, -2.0, 2.0);
        let bc = frand(&mut s, -50.0, 50.0);
        let thr = frand(&mut s, -100.0, 100.0);
        run_case(
            &mut e,
            z,
            z60,
            mid,
            b0,
            b4,
            b8,
            bc,
            bias,
            thr,
            &format!("gate fuzz[{i}]"),
        );
    }
}

/// `0x68e0b2` to `0x68e0c2`: `fild [ebp-4]; fmul [0x8026c8]; fstp [esp+8]`. The sites at
/// `0x68e0cb` and `0x68e0d8` are the same instruction.
#[test]
fn doodad_color_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // the instruction at 0x68e0b8 is `d8 0d c8 26 80 00`
    assert_eq!(
        e.read(0x68e0b8, 6),
        vec![0xd8, 0x0d, 0xc8, 0x26, 0x80, 0x00],
        "0x68e0b8 fmul [0x8026c8] drifted"
    );
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    let run_case = |e: &mut Emu, byte: u8, ctx: &str| {
        e.write_u32(ebp - 0x4, u32::from(byte)); // the integer `fild dword [ebp-4]` reads
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // fild [ebp-4]; mov [ebp-4],eax; fmul [0x8026c8] at 0x68e0b8; fstp [esp+8] at 0x68e0be
        e.run(0x68e0b2, 0x68e0c2);
        let bin = e.read_f32(SLICE_ESP + 0x8);
        assert_f32(bin, color_normalize(byte), ctx);
    };

    for b in [0u8, 1, 127, 128, 254, 255] {
        run_case(&mut e, b, &format!("dn-color byte={b}"));
    }
    let mut s = 0x68e0_b2b2u32;
    for i in 0..1500u32 {
        let b = ((next(&mut s) >> 8) & 0xff) as u8;
        run_case(&mut e, b, &format!("dn-color fuzz[{i}] b={b}"));
    }
}

/// `0x6b531c` to `0x6b5335`: `fild [ebp-8]; fmul [0x8026c8]; fstp [ebp-0x24]`. All twelve sites,
/// from `0x6b5326` to `0x6b5756`, are checked to be the same `d8 0d c8 26 80 00`.
#[test]
fn liquid_color_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // the twelve sites hold the same instruction
    const SITES: [u64; 12] = [
        0x6b5326, 0x6b5342, 0x6b5354, 0x6b5381, 0x6b539b, 0x6b53ad, 0x6b56d5, 0x6b56ee, 0x6b5700,
        0x6b572a, 0x6b5744, 0x6b5756,
    ];
    let expect = vec![0xd8u8, 0x0d, 0xc8, 0x26, 0x80, 0x00];
    for site in SITES {
        assert_eq!(
            e.read(site, 6),
            expect,
            "0x6b5190 fmul [0x8026c8] site {site:#x} drifted"
        );
    }

    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;
    let run_case = |e: &mut Emu, byte: u8, ctx: &str| {
        e.write_u32(ebp - 0x8, u32::from(byte)); // fild dword [ebp-8]
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // the `movzx eax, byte [eax+0x17c]` at 0x6b531f needs eax mapped
        e.run(0x6b531c, 0x6b5335);
        let bin = e.read_f32(ebp - 0x24);
        assert_f32(bin, color_normalize(byte), ctx);
    };

    // eax for that `movzx`
    let scratch = e.alloc(0x200);
    e.set_reg(RegisterX86::EAX, scratch);

    for b in [0u8, 1, 127, 128, 254, 255] {
        e.set_reg(RegisterX86::EAX, scratch);
        run_case(&mut e, b, &format!("liquid-color byte={b}"));
    }
    let mut s = 0x6b53_1c1cu32;
    for i in 0..1500u32 {
        let b = ((next(&mut s) >> 8) & 0xff) as u8;
        e.set_reg(RegisterX86::EAX, scratch);
        run_case(&mut e, b, &format!("liquid-color fuzz[{i}] b={b}"));
    }
}

/// `0x69a431` to `0x69a440`, in an island of the exe's bytes with a store after it: `fild qword
/// [ebp-0x2c]; fmul [0x8106dc]; fsubr [0x7ff9d8]`, the age a signed 64-bit integer at `[ebp-0x2c]`.
#[test]
fn footprint_fade_frac_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    const ISL_VA: u64 = 0x69a431;
    const ISL_LEN: usize = 0x69a440 - 0x69a431;
    let body = e.read(ISL_VA, ISL_LEN);
    // first `fild qword [ebp-0x2c]` (df 6d d4)
    assert_eq!(
        &body[0..3],
        &[0xdf, 0x6d, 0xd4],
        "0x69a431 fild qword drifted"
    );

    let out = e.alloc(0x10);
    let isl = e.alloc(0x40);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;
    let mut code = body.clone();
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let end = isl + code.len() as u64;
    e.write(isl, &code);

    let run_case = |e: &mut Emu, age: i64, ctx: &str| {
        e.write_u32(ebp - 0x2c, age as u32); // low dword
        e.write_u32(ebp - 0x28, (age >> 32) as u32); // high dword
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let bin = e.read_f32(out);
        // the register's value, rounded by the island's store
        let ours = footprint_fade_frac(age) as f32;
        assert_f32(bin, ours, ctx);
    };

    // ages 0, 6000 (the fraction exactly 0) and beyond
    run_case(&mut e, 0, "frac age=0 (→1.0)");
    run_case(&mut e, 6000, "frac age=6000 (→0.0)");
    run_case(&mut e, 3000, "frac age=3000 (→0.5)");
    run_case(&mut e, 12000, "frac age=12000 (→-1.0)");
    run_case(&mut e, -100, "frac age<0 (→>1)");
    let mut s = 0x69a4_3131u32;
    for i in 0..3000u32 {
        let age = i64::from((next(&mut s) >> 8) % 12000);
        run_case(&mut e, age, &format!("frac fuzz[{i}] age={age}"));
    }
}

/// `0x69a431` to `0x69a448`: the fraction, then `fcom [0x7ffd74]; fnstsw ax`, against
/// `test ah,5; jp`: ages 0 (above), 6000 (exactly 0, which scales) and 12000 (below, expires).
#[test]
fn footprint_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    let run_case = |e: &mut Emu, age: i64, ctx: &str| {
        e.write_u32(ebp - 0x2c, age as u32);
        e.write_u32(ebp - 0x28, (age >> 32) as u32);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // to after the `fnstsw ax` at 0x69a446
        e.run(0x69a431, 0x69a448);
        let ah = read_ah(e);
        let bin_scales = (ah & 0x5).count_ones().is_multiple_of(2); // `test ah,5; jp` jumps to the scaling
        let ours = footprint_gate_scales(ah);
        assert_eq!(
            bin_scales, ours,
            "{ctx}: binary scales={bin_scales} ours={ours} (ah={ah:#04x})"
        );
        // and against the value: at or above 0 scales
        let frac = footprint_fade_frac(age);
        let pred = frac >= 0.0;
        assert_eq!(
            bin_scales, pred,
            "{ctx}: binary scales={bin_scales} value-model frac>=0={pred} (frac={frac})"
        );
    };

    run_case(&mut e, 0, "gate age=0 (frac=1>0 → scale)");
    run_case(&mut e, 6000, "gate age=6000 (frac==0 → scale, >= boundary)");
    run_case(&mut e, 6001, "gate age=6001 (frac<0 → expire)");
    run_case(&mut e, 12000, "gate age=12000 (frac<0 → expire)");
    let mut s = 0x69a4_4040u32;
    for i in 0..3000u32 {
        let age = i64::from((next(&mut s) >> 8) % 9000);
        run_case(&mut e, age, &format!("gate fuzz[{i}] age={age}"));
    }
}

/// `0x69a474` to `0x69a48c`, in an island of the exe's bytes: `fmul [0x7ffe58]; fst [ebp-0x10];
/// fstp [ebp-0x18]; fld [ebp-0x18]; fsub [0x86a704]; fistp [ebp-0x14]`, after a `fld dword [in]`
/// that stands for the fraction in the register. Both sides get the same `f32` fraction; the
/// integer is read from `[ebp-0x14]` and clamped to 127 as the function does.
#[test]
fn footprint_alpha_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    const ISL_VA: u64 = 0x69a474;
    const ISL_LEN: usize = 0x69a48c - 0x69a474;
    let body = e.read(ISL_VA, ISL_LEN);
    // first `fmul dword [0x7ffe58]` (d8 0d 58 fe 7f 00)
    assert_eq!(
        &body[0..6],
        &[0xd8, 0x0d, 0x58, 0xfe, 0x7f, 0x00],
        "0x69a474 fmul [0x7ffe58] drifted"
    );

    let inp = e.alloc(0x10);
    let isl = e.alloc(0x40);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;
    // the island: fld dword [inp], then the exe's bytes, whose `fistp [ebp-0x14]` stores the result
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(inp as u32).to_le_bytes());
    code.extend_from_slice(&body);
    let end = isl + code.len() as u64;
    e.write(isl, &code);

    let run_case = |e: &mut Emu, frac: f32, ctx: &str| {
        e.write_u32(inp, frac.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let bin_pre = e.read_u32(ebp - 0x14) as i32;
        let bin_alpha = bin_pre.min(0x7f); // `cmp ecx,0x7f; jge`

        let ours = footprint_alpha(f64::from(frac));
        assert_eq!(
            bin_alpha, ours,
            "{ctx}: binary alpha={bin_alpha} (pre={bin_pre}) ours={ours} (frac={frac})"
        );
    };

    // 1.0 gives 254, clamped to 127; 0.0 gives 0; 0.5 gives 127 exactly
    run_case(&mut e, 1.0, "alpha frac=1 (→ clamp 127)");
    run_case(&mut e, 0.0, "alpha frac=0 (→ 0)");
    run_case(&mut e, 0.5, "alpha frac=0.5");
    run_case(
        &mut e,
        f32::from_bits(0x3f000001),
        "alpha frac just-above-0.5",
    );
    run_case(&mut e, 0.002, "alpha frac tiny"); // 0.51 − 0.5 rounds to 0
    let mut s = 0x69a4_7474u32;
    for i in 0..4000u32 {
        let frac = frand(&mut s, 0.0, 1.0);
        run_case(&mut e, frac, &format!("alpha fuzz[{i}] frac={frac}"));
    }
}

/// `0x69a53b` to `0x69a54a`: `fld [ecx+0x24]; push ecx; fmul [0x8106d8]; fstp [esp]`.
#[test]
fn footprint_jitter_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ecx = e.alloc(0x40);
    let run_case = |e: &mut Emu, v: f32, ctx: &str| {
        e.write_u32(ecx + 0x24, v.to_bits());
        e.set_reg(RegisterX86::ECX, ecx);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(0x69a53b, 0x69a54a);
        let bin = e.read_f32(SLICE_ESP - 4); // after the `push ecx`
        assert_f32(bin, footprint_jitter(v), ctx);
    };
    run_case(&mut e, 0.0, "jitter 0");
    run_case(&mut e, 1.0, "jitter 1");
    run_case(&mut e, -1000.0, "jitter -1000");
    let mut s = 0x69a5_3b3bu32;
    for i in 0..2000u32 {
        let v = frand(&mut s, -5000.0, 5000.0);
        run_case(&mut e, v, &format!("jitter fuzz[{i}]"));
    }
}

/// `0x69a54f` to `0x69a583`: the three globals negated, to `[ebp-0x30]`, `[ebp-0x2c]`, `[ebp-0x28]`.
#[test]
fn footprint_corner_ray_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;
    let run_case = |e: &mut Emu, g: [f32; 3], ctx: &str| {
        e.write_u32(CORNER_X, g[0].to_bits());
        e.write_u32(CORNER_Y, g[1].to_bits());
        e.write_u32(CORNER_Z, g[2].to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x69a54f, 0x69a583);
        let bin = [
            e.read_f32(ebp - 0x30),
            e.read_f32(ebp - 0x2c),
            e.read_f32(ebp - 0x28),
        ];
        for k in 0..3 {
            assert_f32(bin[k], corner_ray_negate(g[k]), &format!("{ctx} neg[{k}]"));
        }
    };
    run_case(&mut e, [0.0, 0.0, 0.0], "corner zero");
    run_case(&mut e, [1.0, -1.0, 1.0], "corner ±1");
    let mut s = 0x69a5_4f4fu32;
    for i in 0..2000u32 {
        let g = [
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
        ];
        run_case(&mut e, g, &format!("corner fuzz[{i}]"));
    }
}

/// The constants equal the exe's stored bits.
#[test]
fn constants_match_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let e = Emu::new(&wow);
    assert_eq!(e.read_u32(0x8026c8), INV_255.to_bits(), "0x8026c8 1/255");
    assert_eq!(e.read_u32(0x8106dc), INV_6000.to_bits(), "0x8106dc 1/6000");
    assert_eq!(
        e.read_u32(0x8106d8),
        SCALE_8106D8.to_bits(),
        "0x8106d8 jitter scalar"
    );
}
