//! Slices of `UpdateGlare` (`0x6cf490`), `BuildSceneFog` (`0x6cee30`) and `BlendAreaLights`
//! (`0x6d2d00`) against the functions of `dn_sky_fog`. Each of the three calls code that cannot be
//! laid out in a test (vtable getters, a record pool and heap sort, a band lookup, `pow` and
//! `fmod`), so each test runs the exe's own instructions between two addresses and compares what
//! they store, bit for bit.
//!
//! Two shapes. A slice that starts with a plain load runs in place. A slice that consumes a value a
//! call left on the x87 stack runs from a small island that loads the test's double, or doubles,
//! and jumps into the exe at the slice's first instruction, so the exe's own opcode bytes and
//! branches run. Every test includes NaN and the thresholds of its compares.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::dn_sky_fog::{
    cloud_layer_distance, cloud_layer_factor, cloud_phase_index, dn_blend_add, dn_blend_sub,
    dn_blue_premul, dn_case2_premul, dn_fog_color_lerp, dn_phase_alpha_delta, dn_phase_fade_arg,
    dn_repack_scale, sky_band_scale, sky_clamp_down, sky_clamp_up, sky_dir_normalize,
    sky_exposure_pre, sky_factor_mul, sky_light_dot, DnFogColorLerp, SkyDirNorm, SkyExposurePre,
};
use common::next;

/// Origin.x.
const G_B78: u64 = 0x00ce_9b78;
/// Origin.y.
const G_B7C: u64 = 0x00ce_9b7c;
/// Origin.z.
const G_B80: u64 = 0x00ce_9b80;
/// Light-dir.x (sky_light_dot).
const G_B84: u64 = 0x00ce_9b84;
/// Light-dir.y.
const G_B88: u64 = 0x00ce_9b88;
/// Light-dir.z.
const G_B8C: u64 = 0x00ce_9b8c;
/// Frame time (cloud_phase_index).
const G_B64: u64 = 0x00ce_9b64;
/// Dn max-floor (dn_blue_premul reads bd8 = max(c50,b94)).
const G_B94: u64 = 0x00ce_9b94;
/// Phase-fade numerator (dn_phase_fade_arg).
const G_B98: u64 = 0x00ce_9b98;
/// Blend step base (dn_blend_add/sub).
const G_B9C: u64 = 0x00ce_9b9c;
/// Dn blue scale (dn_blue_premul c54).
const G_C54: u64 = 0x00ce_9c54;
/// A0 / case-2 selected color (dn_fog_color_lerp / dn_case2_premul).
const G_BF4: u64 = 0x00ce_9bf4;
/// A1 / case-2 out.
const G_BF0: u64 = 0x00ce_9bf0;
/// B0.
const G_BD8: u64 = 0x00ce_9bd8;
/// B1 / dn_blue_premul out.
const G_BD4: u64 = 0x00ce_9bd4;
/// T / blend factor.
const G_BDC: u64 = 0x00ce_9bdc;
/// Raw-copy src.
const G_BD0: u64 = 0x00ce_9bd0;
/// Lerp out0.
const G_BE8: u64 = 0x00ce_9be8;
/// Lerp out1.
const G_BE4: u64 = 0x00ce_9be4;
/// Phase-fade divisor (dn_phase_fade_arg c9d8).
const G_99D8: u64 = 0x00ce_99d8;
/// Phase-fade subtrahend (dn_phase_fade_arg c9e8).
const G_99E8: u64 = 0x00ce_99e8;
/// Dn_phase_alpha_delta out.
const G_99D0: u64 = 0x00ce_99d0;

/// A fuzzed f32: random bits, and finite values of several scales and signs.
fn fuzz(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 6;
    let bits = next(s);
    match pick {
        0 => f32::from_bits(bits),
        1 => (f64::from(bits >> 4) / f64::from(1u32 << 20) - 128.0) as f32,
        2 => (f64::from(bits >> 8) / f64::from(1u32 << 23) * 2000.0 - 1000.0) as f32,
        3 => (f64::from(bits >> 8) / f64::from(1u32 << 23) * 2.0 - 1.0) as f32,
        4 => f32::from_bits(0x3f80_0000 | (bits >> 9)),
        _ => -f32::from_bits(0x3f00_0000 | (bits >> 9)),
    }
}

/// A fuzzed f64 (the synthesized foreign-call return for the island slices): full-domain bits with
/// a bias toward small finite magnitudes.
fn fuzz_f64(s: &mut u32) -> f64 {
    let hi = next(s);
    let lo = next(s);
    match (next(s) >> 8) % 4 {
        0 => f64::from_bits((u64::from(hi) << 32) | u64::from(lo)),
        1 => f64::from(hi) / f64::from(1u32 << 24) * 4.0 - 2.0,
        2 => f64::from(hi as i32) / 1.0e6,
        _ => f64::from(fuzz(s)),
    }
}

/// f32 bit-exact assert with NaN-pair equality.
fn assert_field(binary: f32, ours: f32, tag: &str, field: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{tag}.{field}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// Write a scratch "load-then-jump" island: push `inputs` onto the x87 stack as qwords (`fld qword
/// [slot]`, first pushed = deepest = highest `st(n)`), then `jmp target`. Returns the island entry
/// VA. Each input qword is written to its own scratch slot. The caller then `run(island, end)` so
/// the REAL binary bytes at `target` (with their in-place branches) execute over the synthesized
/// x87 stack.
fn build_island(emu: &mut Emu, inputs: &[f64], target: u64) -> u64 {
    let mut code: Vec<u8> = Vec::new();
    for &v in inputs {
        let slot = emu.alloc(8);
        emu.write(slot, &v.to_bits().to_le_bytes());
        code.push(0xdd);
        code.push(0x05);
        code.extend_from_slice(&(slot as u32).to_le_bytes());
    }
    let island = emu.alloc((code.len() + 5) as u64);
    let jmp_addr = island + code.len() as u64;
    let rel = (target as i64 - (jmp_addr as i64 + 5)) as i32;
    code.push(0xe9);
    code.extend_from_slice(&rel.to_le_bytes());
    emu.write(island, &code);
    island
}

/// `0x6cf4a0` to `0x6cf4f1`.
#[test]
fn sky_dir_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x80);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x6e6f_726d_u32;

    let run = |emu: &mut Emu, pos: [f32; 3], origin: [f32; 3], tag: &str| {
        emu.write_u32(obj, 0);
        emu.write_u32(obj + 0xc, pos[0].to_bits());
        emu.write_u32(obj + 0x10, pos[1].to_bits());
        emu.write_u32(obj + 0x14, pos[2].to_bits());
        emu.write_u32(G_B78, origin[0].to_bits());
        emu.write_u32(G_B7C, origin[1].to_bits());
        emu.write_u32(G_B80, origin[2].to_bits());
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x6cf4a0, 0x6cf4f1);
        let binary = SkyDirNorm {
            nx: emu.read_f32(frame - 0x10),
            ny: emu.read_f32(frame - 0xc),
            nz: emu.read_f32(frame - 8),
            inv: emu.read_f32(frame - 4),
        };
        let ours = sky_dir_normalize(pos, origin);
        assert_field(binary.nx, ours.nx, tag, "nx");
        assert_field(binary.ny, ours.ny, tag, "ny");
        assert_field(binary.nz, ours.nz, tag, "nz");
        assert_field(binary.inv, ours.inv, tag, "inv");
    };

    run(&mut emu, [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], "unit-x");
    run(&mut emu, [3.0, 4.0, 0.0], [0.0, 0.0, 0.0], "3-4-0");
    run(&mut emu, [5.0, 5.0, 5.0], [5.0, 5.0, 5.0], "zero-len");
    run(
        &mut emu,
        [0.1, 0.2, f32::from_bits(0x3eaa_aaab)],
        [0.0, 0.0, 0.0],
        "dz-subf32",
    );
    run(&mut emu, [f32::NAN, 1.0, 2.0], [0.0, 0.0, 0.0], "nan-x");
    run(&mut emu, [-0.0, -0.0, -0.0], [0.0, 0.0, 0.0], "neg-zero");
    run(&mut emu, [1.0e30, 1.0e30, 1.0e30], [0.0, 0.0, 0.0], "huge");
    for _ in 0..400 {
        let pos = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let origin = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        run(&mut emu, pos, origin, "fuzz");
    }
}

/// The `fmul dword [mem]` of `UpdateGlare` on a value left on the stack, through an island.
#[test]
fn sky_factor_mul_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x100);
    let mut s = 0x6661_6374_u32;

    let sites: [(u64, u64, u64, u64, &str); 5] = [
        (0x6cf4f4, 0x6cf4fe, 0x34, 0x34, "0x6cf4f4"),
        (0x6cf501, 0x6cf507, 0x34, 0x34, "0x6cf501"),
        (0x6cf57b, 0x6cf581, 0x34, 0x34, "0x6cf57b"),
        (0x6cf595, 0x6cf59b, 0x34, 0x34, "0x6cf595"),
        (0x6cf6a7, 0x6cf6b0, 0x30, 0xa4, "0x6cf6a7"),
    ];

    let run = |emu: &mut Emu, st0: f64, mem: f32| {
        for &(target, halt, mem_off, out_off, sitetag) in &sites {
            emu.write_u32(obj + mem_off, mem.to_bits());
            if out_off != mem_off {
                emu.write_u32(obj + out_off, 0xdead_beef);
            }
            emu.set_reg(RegisterX86::ESI, obj);
            let island = build_island(emu, &[st0], target);
            emu.run(island, halt);
            let binary = emu.read_f32(obj + out_off);
            let ours = sky_factor_mul(st0, mem);
            assert_field(binary, ours, sitetag, "f");
        }
    };

    run(&mut emu, 1.0, 1.0);
    run(&mut emu, 0.5, 2.0);
    run(&mut emu, 0.0, 5.0);
    run(&mut emu, -3.0, 7.0);
    run(&mut emu, f64::INFINITY, 0.0);
    run(&mut emu, f64::NAN, 1.0);
    run(&mut emu, 0.1, f32::from_bits(0x3f8c_cccd));
    for _ in 0..200 {
        run(&mut emu, fuzz_f64(&mut s), fuzz(&mut s));
    }
}

/// `0x6cf557` to `0x6cf56a`, through an island for the stack value.
#[test]
fn sky_band_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x100);
    let mut s = 0x6261_6e64_u32;

    let run = |emu: &mut Emu, m: f32, sv: f64, tag: &str| {
        emu.set_reg(RegisterX86::ESI, obj);
        emu.write_u32(obj + 0x34, 0xdead_beef);
        let island = build_island(emu, &[sv, f64::from(m)], 0x6cf557);
        emu.run(island, 0x6cf56c);
        let binary = emu.read_f32(obj + 0x34);
        let ours = sky_band_scale(m, sv);
        assert_field(binary, ours, tag, "s");
    };

    run(&mut emu, 0.0, 1.0, "m0");
    run(&mut emu, 1.0, 1.0, "m1");
    run(&mut emu, 0.25, 4.0, "quarter");
    run(&mut emu, f32::NAN, 1.0, "nan-m");
    run(&mut emu, 0.5, f64::INFINITY, "inf-s");
    for _ in 0..300 {
        run(&mut emu, fuzz(&mut s), fuzz_f64(&mut s), "fuzz");
    }
}

fn run_clamp(emu: &mut Emu, x: f32, scale: f32, base: f32, fsubr: bool) -> f32 {
    let obj = emu.alloc(0x100);
    let xframe = emu.alloc(0x40) + 0x20;
    let out = emu.alloc(8);
    emu.write_u32(xframe + 8, x.to_bits());
    emu.write_u32(obj + 0x28, scale.to_bits());
    emu.write_u32(obj + 0x2c, scale.to_bits());
    emu.write_u32(obj + 0x30, base.to_bits());
    emu.set_reg(RegisterX86::ESI, obj);
    emu.set_reg(RegisterX86::EBP, xframe);
    let mut code: Vec<u8> = vec![0xd9, 0x45, 0x08];
    if fsubr {
        code.extend_from_slice(&[0xd8, 0x4e, 0x2c]);
        code.extend_from_slice(&[0xd8, 0x6e, 0x30]);
    } else {
        code.extend_from_slice(&[0xd8, 0x4e, 0x28]);
        code.extend_from_slice(&[0xd8, 0x46, 0x30]);
    }
    code.push(0xd9);
    code.push(0x1d);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let island = emu.alloc(code.len() as u64);
    emu.write(island, &code);
    emu.run(island, island + code.len() as u64);
    emu.read_f32(out)
}

/// `0x6cf5a8` to `0x6cf5b1` and `0x6cf5ce` to `0x6cf5d7`.
#[test]
fn sky_clamp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x636c_6d70_u32;

    let cases: [(f32, f32, f32, &str); 6] = [
        (0.0, 1.0, 0.5, "x0"),
        (1.0, 0.5, 0.25, "x1"),
        (-1.0, 2.0, 3.0, "neg"),
        (f32::NAN, 1.0, 0.0, "nan-x"),
        (2.0, f32::INFINITY, 0.0, "inf-scale"),
        (0.333_333_34, 0.1, 0.7, "subf32"),
    ];
    for &(x, scale, base, tag) in &cases {
        let bu = run_clamp(&mut emu, x, scale, base, false);
        assert_field(bu, sky_clamp_up(x, scale, base), tag, "up");
        let bd = run_clamp(&mut emu, x, scale, base, true);
        assert_field(bd, sky_clamp_down(x, scale, base), tag, "down");
    }
    for _ in 0..300 {
        let (x, scale, base) = (fuzz(&mut s), fuzz(&mut s), fuzz(&mut s));
        let bu = run_clamp(&mut emu, x, scale, base, false);
        assert_field(bu, sky_clamp_up(x, scale, base), "fuzz", "up");
        let bd = run_clamp(&mut emu, x, scale, base, true);
        assert_field(bd, sky_clamp_down(x, scale, base), "fuzz", "down");
    }
}

/// `0x6cf5ed` to `0x6cf60c`.
#[test]
fn sky_light_dot_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x6c64_6f74_u32;

    let run = |emu: &mut Emu, n: [f32; 3], light: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x10, n[0].to_bits());
        emu.write_u32(frame - 0xc, n[1].to_bits());
        emu.write_u32(frame - 8, n[2].to_bits());
        emu.write_u32(G_B84, light[0].to_bits());
        emu.write_u32(G_B88, light[1].to_bits());
        emu.write_u32(G_B8C, light[2].to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x6cf5ed, 0x6cf60f);
        let binary = emu.read_f32(frame + 8);
        let ours = sky_light_dot(n, light);
        assert_field(binary, ours, tag, "dot");
    };

    run(&mut emu, [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], "para");
    run(&mut emu, [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], "anti");
    run(&mut emu, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "zero");
    run(&mut emu, [f32::NAN, 1.0, 0.0], [1.0, 1.0, 1.0], "nan");
    for _ in 0..400 {
        let n = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let light = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        run(&mut emu, n, light, "fuzz");
    }
}

#[allow(clippy::too_many_arguments)]
/// `0x6cf625` to `0x6cf68e`.
#[test]
fn sky_exposure_pre_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x100);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x6578_706f_u32;

    #[allow(clippy::too_many_arguments)]
    let run = |emu: &mut Emu,
               d: f32,
               e98: f32,
               e94: f32,
               e90: f32,
               e20: f32,
               ea0: f32,
               e9c: f32,
               e30: f32,
               byte: u8,
               tag: &str| {
        emu.write_u32(frame + 8, d.to_bits());
        emu.write_u32(obj + 0x98, e98.to_bits());
        emu.write_u32(obj + 0x94, e94.to_bits());
        emu.write_u32(obj + 0x90, e90.to_bits());
        emu.write_u32(obj + 0x20, e20.to_bits());
        emu.write_u32(obj + 0xa0, ea0.to_bits());
        emu.write_u32(obj + 0x9c, e9c.to_bits());
        emu.write_u32(obj + 0x30, e30.to_bits());
        emu.write(obj + 0x1b, &[byte]);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x6cf625, 0x6cf68e);
        let binary = SkyExposurePre {
            e24: emu.read_f32(obj + 0x24),
            packed: emu.read_f32(frame - 4),
        };
        let ours = sky_exposure_pre(d, e98, e94, e90, e20, ea0, e9c, e30, byte);
        assert_field(binary.e24, ours.e24, tag, "e24");
        assert_field(binary.packed, ours.packed, tag, "packed");
    };

    run(&mut emu, 0.5, 0.5, 0.2, 0.1, 1.0, 0.8, 0.3, 2.0, 0, "t=0");
    run(&mut emu, 1.0, 0.0, 0.2, 0.1, 1.0, 0.8, 0.3, 2.0, 255, "t=1");
    run(&mut emu, 0.5, 0.0, 0.2, 0.1, 1.0, 0.8, 0.3, 2.0, 128, "mid");
    run(
        &mut emu, 0.5, 1.0, 0.2, 0.1, 1.0, 0.8, 0.3, 2.0, 64, "e98=1",
    );
    run(
        &mut emu,
        f32::NAN,
        0.0,
        0.2,
        0.1,
        1.0,
        0.8,
        0.3,
        2.0,
        0,
        "nan-d",
    );
    for _ in 0..400 {
        run(
            &mut emu,
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            (next(&mut s) >> 8) as u8,
            "fuzz",
        );
    }
}

/// `0x6cee61` to `0x6cee89`.
#[test]
fn dn_blue_premul_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x626c_7565_u32;

    let run = |emu: &mut Emu, c54: f32, bd8: f32, tag: &str| {
        emu.write_u32(G_C54, c54.to_bits());
        emu.write_u32(G_BD8, bd8.to_bits());
        emu.write_u32(0x00ce_9c28, 0x1234_5678);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x6cee61, 0x6cee89);
        let binary = emu.read_f32(G_BD4);
        let ours = dn_blue_premul(c54, bd8);
        assert_field(binary, ours, tag, "bd4");
    };

    run(&mut emu, 1.0, 1.0, "one");
    run(&mut emu, 0.5, 2.0, "half");
    run(&mut emu, f32::NAN, 1.0, "nan");
    run(&mut emu, f32::INFINITY, 0.0, "inf-zero");
    for _ in 0..300 {
        run(&mut emu, fuzz(&mut s), fuzz(&mut s), "fuzz");
    }
}

/// `0x6cef50`, through an island.
#[test]
fn dn_case2_premul_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rec = emu.alloc(0x20);
    let mut s = 0x6361_7332_u32;

    let run = |emu: &mut Emu, b94: f32, rec4: f32, tag: &str| {
        emu.write_u32(G_B94, b94.to_bits());
        emu.write_u32(rec + 4, rec4.to_bits());
        emu.write_u32(rec + 8, 0);
        emu.set_reg(RegisterX86::EDX, rec);
        emu.set_reg(RegisterX86::ECX, 2);
        emu.run(0x6cef43, 0x6cef68);
        let binary = emu.read_f32(G_BF0);
        let ours = dn_case2_premul(b94, rec4);
        assert_field(binary, ours, tag, "bf0");
    };

    run(&mut emu, 1.0, 1.0, "one");
    run(&mut emu, 0.25, 4.0, "quarter");
    run(&mut emu, f32::NAN, 1.0, "nan");
    for _ in 0..300 {
        run(&mut emu, fuzz(&mut s), fuzz(&mut s), "fuzz");
    }
}

/// `0x6cefcb` to `0x6cefdd` and `0x6cf011` to `0x6cf023`.
#[test]
fn dn_blend_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x626c_6e64_u32;

    let run_add = |emu: &mut Emu, c9c: f32, bdc: f32, tag: &str| {
        emu.write_u32(G_B9C, c9c.to_bits());
        emu.write_u32(G_BDC, bdc.to_bits());
        let out = emu.alloc(8);
        let mut code: Vec<u8> = vec![0xd9, 0x05];
        code.extend_from_slice(&0x0081_15b0u32.to_le_bytes());
        code.extend_from_slice(&[0xd8, 0x0d]);
        code.extend_from_slice(&0x00ce_9b9cu32.to_le_bytes());
        code.extend_from_slice(&[0xd8, 0x05]);
        code.extend_from_slice(&0x00ce_9bdcu32.to_le_bytes());
        code.push(0xd9);
        code.push(0x1d);
        code.extend_from_slice(&(out as u32).to_le_bytes());
        let island = emu.alloc(code.len() as u64);
        emu.write(island, &code);
        emu.run(island, island + code.len() as u64);
        let binary = emu.read_f32(out);
        assert_field(binary, dn_blend_add(c9c, bdc), tag, "add");
    };

    let run_sub = |emu: &mut Emu, c9c: f32, bdc: f32, tag: &str| {
        emu.write_u32(G_B9C, c9c.to_bits());
        emu.write_u32(G_BDC, bdc.to_bits());
        emu.run(0x6cf011, 0x6cf029);
        let binary = emu.read_f32(G_BDC);
        assert_field(binary, dn_blend_sub(c9c, bdc), tag, "sub");
    };

    for &(c9c, bdc, tag) in &[
        (0.0, 0.5_f32, "z"),
        (1.0, 0.0, "one"),
        (f32::NAN, 0.5, "nan"),
        (0.333_333_34, 0.1, "subf32"),
    ] {
        run_add(&mut emu, c9c, bdc, tag);
        run_sub(&mut emu, c9c, bdc, tag);
    }
    for _ in 0..300 {
        let (c9c, bdc) = (fuzz(&mut s), fuzz(&mut s));
        run_add(&mut emu, c9c, bdc, "fuzz");
        run_sub(&mut emu, c9c, bdc, "fuzz");
    }
}

/// `0x6cf054` to `0x6cf099`.
#[test]
fn dn_fog_color_lerp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x6c65_7270_u32;

    let run = |emu: &mut Emu, a0: f32, b0: f32, a1: f32, b1: f32, t: f32, tag: &str| {
        emu.write_u32(G_BF4, a0.to_bits());
        emu.write_u32(G_BD8, b0.to_bits());
        emu.write_u32(G_BF0, a1.to_bits());
        emu.write_u32(G_BD4, b1.to_bits());
        emu.write_u32(G_BDC, t.to_bits());
        emu.write_u32(G_BD0, 0x1234_5678);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x6cf054, 0x6cf099);
        let binary = DnFogColorLerp {
            be8: emu.read_f32(G_BE8),
            be4: emu.read_f32(G_BE4),
        };
        let ours = dn_fog_color_lerp(a0, b0, a1, b1, t);
        assert_field(binary.be8, ours.be8, tag, "be8");
        assert_field(binary.be4, ours.be4, tag, "be4");
    };

    run(&mut emu, 1.0, 0.0, 0.5, 0.25, 0.0, "t=0");
    run(&mut emu, 1.0, 0.0, 0.5, 0.25, 1.0, "t=1");
    run(&mut emu, 1.0, 0.0, 0.5, 0.25, 0.5, "t=.5");
    run(&mut emu, 0.7, 0.7, 0.3, 0.3, 0.33, "a==b");
    run(&mut emu, f32::NAN, 0.0, 0.5, 0.25, 0.5, "nan-a0");
    for _ in 0..400 {
        run(
            &mut emu,
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            "fuzz",
        );
    }
}

/// `0x6cf099` to `0x6cf0ab`.
#[test]
fn dn_repack_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x7270_6b73_u32;

    let run = |emu: &mut Emu, bdc: f32, tag: &str| {
        emu.write_u32(G_BDC, bdc.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x6cf099, 0x6cf0ae);
        let binary = emu.read_f32(frame - 0x14);
        assert_field(binary, dn_repack_scale(bdc), tag, "rp");
    };

    run(&mut emu, 0.0, "z");
    run(&mut emu, 1.0, "one");
    run(&mut emu, 0.5, "half");
    run(&mut emu, f32::NAN, "nan");
    for _ in 0..300 {
        run(&mut emu, fuzz(&mut s), "fuzz");
    }
}

/// `0x6cf11a` to `0x6cf138`, the double passed to `exp2`.
#[test]
fn dn_phase_fade_arg_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let espbase = emu.alloc(0x40) + 0x20;
    let mut s = 0x70686172_u32;

    let run = |emu: &mut Emu, b98: f32, c9e8: f32, c9d8: f32, tag: &str| {
        emu.write_u32(G_B98, b98.to_bits());
        emu.write_u32(G_99E8, c9e8.to_bits());
        emu.write_u32(G_99D8, c9d8.to_bits());
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x6cf11a, 0x6cf138);
        let lo = emu.read_u32(espbase - 8);
        let hi = emu.read_u32(espbase - 4);
        let binary = f64::from_bits((u64::from(hi) << 32) | u64::from(lo));
        let ours = dn_phase_fade_arg(b98, c9e8, c9d8);
        if binary.is_nan() && ours.is_nan() {
        } else {
            assert_eq!(
                binary.to_bits(),
                ours.to_bits(),
                "{tag}: binary={:#018x} ours={:#018x}",
                binary.to_bits(),
                ours.to_bits()
            );
        }
    };

    run(&mut emu, 1.0, 0.0, 2.0, "basic");
    run(&mut emu, 0.5, 0.5, 1.0, "zero-num");
    run(&mut emu, 1.0, 0.0, 0.0, "div0");
    run(&mut emu, f32::NAN, 0.0, 1.0, "nan");
    for _ in 0..300 {
        run(&mut emu, fuzz(&mut s), fuzz(&mut s), fuzz(&mut s), "fuzz");
    }
}

/// `0x6cf13d` to `0x6cf17e`, through an island for the `exp2` result; the reciprocal is probed
/// below 0, at 0, between 0 and 1, at 1, above 1 and NaN.
#[test]
fn dn_phase_alpha_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x70616c66_u32;

    let run = |emu: &mut Emu, w: f64, tag: &str| {
        let island = build_island(emu, &[w], 0x6cf13d);
        emu.run(island, 0x6cf182);
        let binary = emu.read_f32(G_99D0);
        assert_field(binary, dn_phase_alpha_delta(w), tag, "ad");
    };

    run(&mut emu, f64::INFINITY, "w-inf");
    run(&mut emu, 1.0, "w1");
    run(&mut emu, 2.0, "w2");
    run(&mut emu, 0.5, "w.5");
    run(&mut emu, -1.0, "wneg");
    run(&mut emu, f64::NAN, "wnan");
    run(&mut emu, 0.0, "w0");
    run(&mut emu, -0.0, "wnegzero");
    for _ in 0..400 {
        run(&mut emu, fuzz_f64(&mut s), "fuzz");
    }
}

/// `0x6d2d69` to `0x6d2da0`.
#[test]
fn cloud_layer_distance_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let pos_p = emu.alloc(0x20);
    let scratch = emu.alloc(0x20);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x6469_7374_u32;

    let run = |emu: &mut Emu, origin: [f32; 3], pos: [f32; 3], tag: &str| {
        emu.write_u32(G_B78, origin[0].to_bits());
        emu.write_u32(G_B7C, origin[1].to_bits());
        emu.write_u32(G_B80, origin[2].to_bits());
        emu.write_u32(pos_p, pos[0].to_bits());
        emu.write_u32(pos_p + 4, pos[1].to_bits());
        emu.write_u32(pos_p + 8, pos[2].to_bits());
        emu.write_u32(scratch, 0);
        emu.write_u32(frame - 4, 0xdead_beef);
        emu.set_reg(RegisterX86::EAX, pos_p);
        emu.set_reg(RegisterX86::EBX, scratch);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x6d2d69, 0x6d2da0);
        let binary = emu.read_f32(frame - 4);
        let ours = cloud_layer_distance(origin, pos);
        assert_field(binary, ours, tag, "len");
    };

    run(&mut emu, [0.0, 0.0, 0.0], [3.0, 4.0, 0.0], "5");
    run(&mut emu, [1.0, 2.0, 3.0], [1.0, 2.0, 3.0], "coincident");
    run(&mut emu, [0.0, 0.0, 0.0], [f32::NAN, 1.0, 2.0], "nan");
    run(
        &mut emu,
        [1.0e30, 1.0e30, 1.0e30],
        [-1.0e30, 0.0, 0.0],
        "huge",
    );
    for _ in 0..400 {
        let origin = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let pos = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        run(&mut emu, origin, pos, "fuzz");
    }
}

/// `0x6d2f40` and `0x6d2f98`: `fistp` of the product minus 0.5, with ties, exact odd integers, the
/// range edge and NaN.
#[test]
fn cloud_phase_index_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x70696478_u32;

    let sites: [(u64, u64, u64, &str); 2] = [
        (0x6d2f40, 0x6d2f61, 0x4c, "site1"),
        (0x6d2f98, 0x6d2fb3, 0x44, "site2"),
    ];
    let run = |emu: &mut Emu, time: f32, tag: &str| {
        let ours = cloud_phase_index(time);
        for &(begin, halt, out_off, sitetag) in &sites {
            emu.write_u32(G_B64, time.to_bits());
            emu.write_u32(frame - out_off, 0xdead_beef);
            emu.set_reg(RegisterX86::EBP, frame);
            emu.run(begin, halt);
            let binary = emu.read_u32(frame - out_off) as i32;
            assert_eq!(
                binary, ours,
                "{tag}.{sitetag}: binary={binary} ours={ours} (time={time})"
            );
        }
    };

    run(&mut emu, 0.0, "zero");
    run(&mut emu, 0.5 / 2880.0, "half-step");
    run(&mut emu, 1.0, "one");
    run(&mut emu, 1.0e9, "huge");
    run(&mut emu, -1.0e9, "huge-neg");
    run(&mut emu, f32::NAN, "nan");
    run(&mut emu, f32::INFINITY, "inf");
    for k in 0..8u32 {
        let t = (f64::from(k) + 0.5 + 0.5) / 2880.0;
        run(&mut emu, t as f32, "tie");
    }
    for _ in 0..400 {
        let t = if (next(&mut s) >> 8).is_multiple_of(4) {
            fuzz(&mut s)
        } else {
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 23) * 2.0 - 1.0) as f32
        };
        run(&mut emu, t, "fuzz");
    }
}

/// `0x6d300e` to `0x6d3025`.
#[test]
fn cloud_layer_factor_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rec = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x6c66_6163_u32;

    let run = |emu: &mut Emu, c3c: f32, r14: f32, r18: f32, tag: &str| {
        emu.write_u32(frame - 0x3c, c3c.to_bits());
        emu.write_u32(rec + 0x14, r14.to_bits());
        emu.write_u32(rec + 0x18, r18.to_bits());
        emu.set_reg(RegisterX86::ESI, rec);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x6d300e, 0x6d3025);
        let binary = emu.read_f32(frame - 8);
        let ours = cloud_layer_factor(c3c, r14, r18);
        assert_field(binary, ours, tag, "f");
    };

    run(&mut emu, 0.0, 0.0, 1.0, "f0");
    run(&mut emu, 1.0, 0.0, 1.0, "f1");
    run(&mut emu, 0.5, 0.0, 1.0, "fhalf");
    run(&mut emu, 0.5, 1.0, 1.0, "div0");
    run(&mut emu, f32::NAN, 0.0, 1.0, "nan");
    for _ in 0..300 {
        run(&mut emu, fuzz(&mut s), fuzz(&mut s), fuzz(&mut s), "fuzz");
    }
}
