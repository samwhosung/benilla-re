//! The layer texture arithmetic against `0x6bef30`, `0x6bf690` and `0x6813d0`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::texgen::{
    cam_relative_delta, cam_relative_negate, detail_texgen_scale, lod_gate_subtract, neg_fvar1,
    texgen_apply, texgen_scale_recip,
};
use common::{assert_f32, next};

/// A frame inside the harness stack (`0x70000000`, 0x100000 bytes).
const SLICE_ESP: u64 = 0x7008_0000;

/// The camera.
const CAM_X: u64 = 0xc7cf20;

const CAM_Y: u64 = 0xc7cf24;

const CAM_Z: u64 = 0xc7cf28;

/// The stage divisors, `[0x86c318 + 4·stage]`.
const TEXGEN_DIVISOR_TABLE: u64 = 0x86c318;

/// A float in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// `CMapChunk::DrawLayers` (`0x6bef30`) from `0x6bf035` and `CMapChunk::DrawLayersShader` (`0x6bf690`)
/// from `0x6bf6aa`, esi the layer: the scale at `[ebp-0x28]` and `[ebp-0x20]`.
#[test]
fn texgen_scale_recip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let layer = e.alloc(0x1000);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let sites: [(u64, u64, i64); 2] = [(0x6bf035, 0x6bf0b8, -0x28), (0x6bf6aa, 0x6bf748, -0x20)];

    let run_case = |e: &mut Emu, texscale: f32, ctx: &str| {
        for &(begin, end, slot) in &sites {
            e.write_u32(layer + 0x84c, texscale.to_bits());
            e.set_reg(RegisterX86::ECX, layer);
            e.set_reg(RegisterX86::ESI, layer);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(begin, end);
            let bin = e.read_f32((ebp as i64 + slot) as u64);
            let ours = texgen_scale_recip(texscale);
            assert_f32(bin, ours, &format!("{ctx} @{begin:#x}"));
        }
    };

    run_case(&mut e, 1.0, "recip texscale=1");
    run_case(&mut e, -1.0, "recip texscale=-1");
    run_case(&mut e, 0.0, "recip texscale=0 (-Inf)");
    run_case(&mut e, 2.0, "recip texscale=2 (-0.5)");
    run_case(&mut e, 8.0, "recip texscale=8 (-0.125)");
    run_case(
        &mut e,
        f32::from_bits(0x0000_0001),
        "recip texscale subnormal",
    );

    let mut s = 0x6bf0_3535u32;
    for i in 0..3000u32 {
        let texscale = frand(&mut s, -2048.0, 2048.0);
        run_case(&mut e, texscale, &format!("recip fuzz[{i}] ts={texscale}"));
    }
}

/// `0x6bef30` at `0x6bef4c`: its six bytes copied out with `fstp dword [out]` appended.
#[test]
fn lod_gate_subtract_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    const SRC: u64 = 0x6bef4c;
    const LEN: usize = 0x6bef52 - 0x6bef4c;
    let mut code = e.read(SRC, LEN);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    assert_eq!(
        e.read(isl, LEN),
        e.read(SRC, LEN),
        "island bytes at {isl:#x} != WoW.exe[{SRC:#x}..] (len {LEN})"
    );
    let end = isl + code.len() as u64;

    let layer = e.alloc(0x1000);

    let run_case = |e: &mut Emu, a: f32, b: f32, ctx: &str| {
        e.write_u32(layer + 0x78, a.to_bits());
        e.write_u32(layer + 0x68, b.to_bits());
        e.set_reg(RegisterX86::ESI, layer);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(isl, end);
        let bin = e.read_f32(out);
        let ours = lod_gate_subtract(a, b) as f32;
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 5.0, 5.0, "gate a==b");
    run_case(&mut e, 0.0, -0.0, "gate signed zeros");
    run_case(&mut e, 1.0, -1.0, "gate ±1");
    run_case(&mut e, f32::from_bits(0x7fc0_0000), 0.0, "gate NaN a");
    run_case(&mut e, f32::from_bits(0x0000_0001), 0.0, "gate denormal a");

    let mut s = 0x6bef_4c4cu32;
    for i in 0..3000u32 {
        let a = frand(&mut s, -10000.0, 10000.0);
        let b = frand(&mut s, -10000.0, 10000.0);
        run_case(&mut e, a, b, &format!("gate fuzz[{i}]"));
    }
}

/// `0x6bef30` at `0x6bf0b8` and `0x6bf690` at `0x6bf748`, to the call after: `[esp]`.
#[test]
fn neg_fvar1_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let sites: [(u64, u64, i64); 2] = [(0x6bf0b8, 0x6bf0c0, -0x28), (0x6bf748, 0x6bf750, -0x20)];

    let run_case = |e: &mut Emu, fvar1: f32, ctx: &str| {
        for &(begin, end, slot) in &sites {
            e.write_u32((ebp as i64 + slot) as u64, fvar1.to_bits());
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(begin, end);
            let bin = e.read_f32(SLICE_ESP);
            let ours = neg_fvar1(fvar1);
            assert_f32(bin, ours, &format!("{ctx} @{begin:#x}"));
        }
    };

    run_case(&mut e, 0.0, "neg-fvar1 0");
    run_case(&mut e, -0.0, "neg-fvar1 -0");
    run_case(&mut e, 1.0, "neg-fvar1 1");
    run_case(&mut e, -1.0, "neg-fvar1 -1");
    run_case(&mut e, f32::from_bits(0x7fc0_0000), "neg-fvar1 NaN");
    run_case(&mut e, f32::from_bits(0x0000_0001), "neg-fvar1 denormal");

    let mut s = 0x6bf0_b8b8u32;
    for i in 0..3000u32 {
        let fvar1 = frand(&mut s, -4096.0, 4096.0);
        run_case(&mut e, fvar1, &format!("neg-fvar1 fuzz[{i}]"));
    }
}

/// `0x6bef30` at `0x6bf18d` and `0x6bf690` at `0x6bf82e`, to the call after: `[esp]` after one push.
#[test]
fn detail_texgen_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let sites: [(u64, u64, i64); 2] = [(0x6bf18d, 0x6bf1b1, -0x28), (0x6bf82e, 0x6bf84c, -0x20)];

    let run_case = |e: &mut Emu, fvar1: f32, ctx: &str| {
        for &(begin, end, slot) in &sites {
            e.write_u32((ebp as i64 + slot) as u64, fvar1.to_bits());
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(begin, end);
            let bin = e.read_f32(SLICE_ESP - 4);
            let ours = detail_texgen_scale(fvar1);
            assert_f32(bin, ours, &format!("{ctx} @{begin:#x}"));
        }
    };

    run_case(&mut e, 0.0, "detail fvar1=0");
    run_case(&mut e, -0.0, "detail fvar1=-0");
    run_case(&mut e, 1.0, "detail fvar1=1 (-0.125)");
    run_case(&mut e, -1.0, "detail fvar1=-1 (0.125)");
    run_case(&mut e, 8.0, "detail fvar1=8 (-1.0)");
    run_case(
        &mut e,
        f32::from_bits(0x0000_0001),
        "detail fvar1 subnormal",
    );

    let mut s = 0x6bf1_8d8du32;
    for i in 0..3000u32 {
        let fvar1 = frand(&mut s, -4096.0, 4096.0);
        run_case(&mut e, fvar1, &format!("detail fuzz[{i}] fv={fvar1}"));
    }
}

/// `0x6bef30` from `0x6bf0c5` and `0x6bf1d9`, and `0x6bf690` from `0x6bf755` and `0x6bf87b`, esi the
/// chunk: the three negated differences.
#[test]
fn cam_relative_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let layer = e.alloc(0x1000);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let sites: [(u64, u64, [i64; 3]); 4] = [
        (0x6bf0c5, 0x6bf128, [-0x14, -0x10, -0xc]),
        (0x6bf1d9, 0x6bf25d, [-0x14, -0x10, -0xc]),
        (0x6bf755, 0x6bf7df, [-0xc, -0x8, -0x4]),
        (0x6bf87b, 0x6bf8d5, [-0xc, -0x8, -0x4]),
    ];

    let run_case = |e: &mut Emu, coord: [f32; 3], cam: [f32; 3], ctx: &str| {
        for &(begin, end, slots) in &sites {
            e.write_u32(layer + 0x6c, coord[0].to_bits());
            e.write_u32(layer + 0x70, coord[1].to_bits());
            e.write_u32(layer + 0x74, coord[2].to_bits());
            e.write_u32(CAM_X, cam[0].to_bits());
            e.write_u32(CAM_Y, cam[1].to_bits());
            e.write_u32(CAM_Z, cam[2].to_bits());
            e.set_reg(RegisterX86::ESI, layer);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(begin, end);
            for k in 0..3 {
                let bin = e.read_f32((ebp as i64 + slots[k]) as u64);
                let ours = cam_relative_negate(coord[k], cam[k]);
                assert_f32(bin, ours, &format!("{ctx} @{begin:#x} comp[{k}]"));
            }
        }
    };

    run_case(&mut e, [5.0, 5.0, 5.0], [5.0, 5.0, 5.0], "neg coord==cam");
    run_case(
        &mut e,
        [0.0, -0.0, 0.0],
        [0.0, 0.0, -0.0],
        "neg signed zeros",
    );
    run_case(&mut e, [1.0, -1.0, 1.0], [-1.0, 1.0, -1.0], "neg ±1");
    run_case(
        &mut e,
        [f32::from_bits(0x7fc0_0000), 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "neg NaN coord",
    );
    run_case(
        &mut e,
        [f32::from_bits(0x0000_0001), 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "neg denormal coord",
    );

    let mut s = 0x6bf0_c5c5u32;
    for i in 0..3000u32 {
        let coord = [
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
        ];
        let cam = [
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
        ];
        run_case(&mut e, coord, cam, &format!("neg fuzz[{i}]"));
    }
}

/// `0x6bef30` from `0x6bf3cc` and `0x6bf690` from `0x6bf974`: eax the stage's vector, ecx its divisor's
/// index; the three products.
#[test]
fn texgen_apply_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let tgbuf = e.alloc(0x20);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let sites: [(u64, u64, i64, [i64; 3]); 2] = [
        (0x6bf3cc, 0x6bf424, -0x28, [-0x20, -0x1c, -0x18]),
        (0x6bf974, 0x6bf9cc, -0x20, [-0x18, -0x14, -0x10]),
    ];

    let divisors: [f32; 8] =
        std::array::from_fn(|k| e.read_f32(TEXGEN_DIVISOR_TABLE + 4 * k as u64));

    let run_case = |e: &mut Emu, fvar1: f32, k: usize, tg: [f32; 3], ctx: &str| {
        for &(begin, end, fslot, outs) in &sites {
            for (c, v) in tg.iter().enumerate() {
                e.write_u32(tgbuf + 4 * c as u64, v.to_bits());
            }
            e.write_u32((ebp as i64 + fslot) as u64, fvar1.to_bits());
            e.set_reg(RegisterX86::EAX, tgbuf);
            e.set_reg(RegisterX86::ECX, (k as u64) << 3);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(begin, end);
            let ours = texgen_apply(fvar1, divisors[k], tg);
            for c in 0..3 {
                let bin = e.read_f32((ebp as i64 + outs[c]) as u64);
                assert_f32(bin, ours[c], &format!("{ctx} @{begin:#x} out[{c}]"));
            }
        }
    };

    for k in 0..8usize {
        run_case(
            &mut e,
            1.0,
            k,
            [1.0, 0.0, 0.0],
            &format!("apply k={k} fv=1 unit-x"),
        );
        run_case(
            &mut e,
            -1.0,
            k,
            [1.0, 1.0, 1.0],
            &format!("apply k={k} fv=-1 ones"),
        );
        run_case(
            &mut e,
            2.0,
            k,
            [0.0, 0.0, 0.0],
            &format!("apply k={k} fv=2 zero-tg"),
        );
    }

    let mut s = 0x6bf3_cccc_u32;
    for i in 0..3000u32 {
        let fvar1 = frand(&mut s, -1024.0, 1024.0);
        let k = ((next(&mut s) >> 8) % 8) as usize;
        let tg = [
            frand(&mut s, -4.0, 4.0),
            frand(&mut s, -4.0, 4.0),
            frand(&mut s, -4.0, 4.0),
        ];
        run_case(&mut e, fvar1, k, tg, &format!("apply fuzz[{i}] k={k}"));
    }
}

/// `SetupWorldStages` (`0x6813d0`) from `0x68151b` to the call at `0x681546`, ebx the position: the
/// differences at `[ebp-0x50]` up.
#[test]
fn cam_relative_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let doodad = e.alloc(0x100);
    let frame = e.alloc(0x300);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, pos: [f32; 3], cam: [f32; 3], ctx: &str| {
        e.write_u32(doodad + 0x6c, pos[0].to_bits());
        e.write_u32(doodad + 0x70, pos[1].to_bits());
        e.write_u32(doodad + 0x74, pos[2].to_bits());
        e.write_u32(CAM_X, cam[0].to_bits());
        e.write_u32(CAM_Y, cam[1].to_bits());
        e.write_u32(CAM_Z, cam[2].to_bits());
        e.set_reg(RegisterX86::EBX, doodad);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x68151b, 0x681546);
        let bin = [
            e.read_f32(ebp - 0x50),
            e.read_f32(ebp - 0x4c),
            e.read_f32(ebp - 0x48),
        ];
        for k in 0..3 {
            let ours = cam_relative_delta(pos[k], cam[k]);
            assert_f32(bin[k], ours, &format!("{ctx} delta[{k}]"));
        }
    };

    run_case(&mut e, [5.0, 5.0, 5.0], [5.0, 5.0, 5.0], "delta pos==cam");
    run_case(
        &mut e,
        [0.0, -0.0, 0.0],
        [0.0, 0.0, -0.0],
        "delta signed zeros",
    );
    run_case(&mut e, [1.0, -1.0, 1.0], [-1.0, 1.0, -1.0], "delta ±1");
    run_case(
        &mut e,
        [f32::from_bits(0x7fc0_0000), 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "delta NaN pos",
    );
    run_case(
        &mut e,
        [f32::from_bits(0x0000_0001), 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "delta denormal pos",
    );

    let mut s = 0x6815_1b1bu32;
    for i in 0..3000u32 {
        let pos = [
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
        ];
        let cam = [
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
        ];
        run_case(&mut e, pos, cam, &format!("delta fuzz[{i}]"));
    }
}
