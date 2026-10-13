//! A model's lights and lighting record (`systems/models/lights.md`), most functions called whole
//! with the record written and read back, the rest as slices. A stretch run in an island is the
//! exe's own bytes, checked, with a store appended.

#![allow(clippy::needless_range_loop)] // index loops follow the `+off + 4·k` layout

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::lights::{
    attach_light_grid_window, camera_facing_basis, driver_mag_gate, driver_vec_diff,
    fade_set_range, grid_scale_product, light_pack_triple, light_pack_triple3, m2_light_accumulate,
    m2_light_add_directional, m2_light_aggregate_dist, m2_light_color_pack,
    m2_light_coord_finalize, m2_light_set_direction, m2light_accum_vec3, set_dir_gate_ah,
    set_dir_len, sh_light_pack, CameraBasisIn, LightAccum, FLOOR_EPS, GRID_SCALE, NORM_EPS,
};
use common::{
    assert_f32, assert_f64, frand, fuzz_f32, fuzz_scales, island_bytes_match, next, read_ah,
    read_f64, Lcg, SLICE_ESP,
};

/// The scene's grid scale, written by `InitM2SceneHalfTenth` (`0x70dc40`, to `0x70dc52`).
const GRID_SCALE_VA: u64 = 0x00ce_ffec;

/// `0x71b6a0` whole, the direction read back from `+0x24`; and from `0x71b6bc` to the `jnp` at
/// `0x71b6ec`, `ah` after the compare of the length with 2.3841858e-7. A length of exactly that,
/// one step either side, zero, NaN and an infinite length probe the gate.
#[test]
fn m2_light_set_direction_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let light = e.alloc(0x40);
    let dir_in = e.alloc(0x10);
    let gate_frame = e.alloc(0x100);

    let run_full = |e: &mut Emu, dir: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(dir_in + 4 * k as u64, dir[k].to_bits());
            e.write_u32(light + 0x24 + 4 * k as u64, 0xdead_0000 + k as u32);
        }
        let _ = e.call(0x71b6a0, light as u32, 0, &[dir_in as u32]);
        let ours = m2_light_set_direction(dir);
        for k in 0..3 {
            assert_f32(
                e.read_f32(light + 0x24 + 4 * k as u64),
                ours[k],
                &format!("{ctx} dir[{k}]"),
            );
        }
    };
    let run_gate = |e: &mut Emu, dir: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(light + 0x24 + 4 * k as u64, dir[k].to_bits());
        }
        e.set_reg(RegisterX86::ECX, light + 0x24);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, gate_frame + 0x80);
        e.run(0x71b6bc, 0x71b6ec);
        let ah = read_ah(e) & 0x05;
        let ours = set_dir_gate_ah(set_dir_len(dir)) & 0x05;
        assert_eq!(ah, ours, "{ctx}: ah&5 binary={ah:#04x} ours={ours:#04x}");
    };

    let eps = NORM_EPS;
    for &(dir, ctx) in &[
        ([eps, 0.0, 0.0], "length == eps"),
        (
            [f32::from_bits(eps.to_bits() - 1), 0.0, 0.0],
            "just below eps",
        ),
        (
            [f32::from_bits(eps.to_bits() + 1), 0.0, 0.0],
            "just above eps",
        ),
        ([-eps, 0.0, 0.0], "length == eps, negative"),
        ([0.0, 0.0, 0.0], "zero"),
        ([1.0, 0.0, 0.0], "unit x"),
        ([0.0, 1.0, 0.0], "unit y"),
        ([0.0, 0.0, 1.0], "unit z"),
        ([3.0, 4.0, 0.0], "length 5"),
        ([-3.0, 0.0, 4.0], "length 5, signed"),
        ([1.0e-9, 0.0, 0.0], "tiny"),
        ([1.0e20, 1.0e20, 1.0e20], "infinite length"),
        ([f32::NAN, 0.0, 0.0], "NaN"),
        ([1.0, f32::NAN, 1.0], "NaN in y"),
        ([f32::INFINITY, 0.0, 0.0], "infinite component"),
    ] {
        run_full(&mut e, dir, ctx);
        run_gate(&mut e, dir, ctx);
    }

    let mut s = 0x6c61_6363u32;
    for i in 0..4000u32 {
        let dir = [
            fuzz_scales(&mut s),
            fuzz_scales(&mut s),
            fuzz_scales(&mut s),
        ];
        run_full(&mut e, dir, &format!("fuzz[{i}]"));
        run_gate(&mut e, dir, &format!("fuzz[{i}]"));
    }
    for i in 0..2000u32 {
        let dir = [frand(&mut s, eps * 0.5, eps * 2.0), 0.0, 0.0];
        run_full(&mut e, dir, &format!("near eps[{i}]"));
        run_gate(&mut e, dir, &format!("near eps[{i}]"));
    }
}

/// `0x71bc70` whole: the sum at `+0x54` and the ambient terms at `+0x84`, `+0xa8` and `+0xcc`
/// seeded, called, read back.
#[test]
fn m2_light_accumulate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let acc = e.alloc(0x100);
    let dir_in = e.alloc(0x10);
    const SUM: u64 = 0x54;
    const SH: [u64; 3] = [0x84, 0xa8, 0xcc];

    let run_case = |e: &mut Emu, sum: [f32; 3], sh: [f32; 3], dir: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(acc + SUM + 4 * k as u64, sum[k].to_bits());
            e.write_u32(acc + SH[k], sh[k].to_bits());
            e.write_u32(dir_in + 4 * k as u64, dir[k].to_bits());
        }
        let _ = e.call(0x71bc70, acc as u32, 0, &[dir_in as u32]);
        let (our_sum, our_sh) = m2_light_accumulate(sum, sh, dir);
        for k in 0..3 {
            let got = e.read_f32(acc + SUM + 4 * k as u64);
            assert_f32(got, our_sum[k], &format!("{ctx} sum[{k}]"));
            assert_f32(
                e.read_f32(acc + SH[k]),
                our_sh[k],
                &format!("{ctx} sh[{k}]"),
            );
        }
    };

    run_case(&mut e, [0.0; 3], [0.0; 3], [1.0, 2.0, 3.0], "zero record");
    run_case(
        &mut e,
        [10.0, -5.0, 0.5],
        [100.0, 0.0, -7.0],
        [0.0; 3],
        "zero colour",
    );
    run_case(&mut e, [-1.0; 3], [-1.0; 3], [1.0; 3], "cancelling");
    run_case(
        &mut e,
        [1.0; 3],
        [1.0; 3],
        [f32::NAN, 1.0, 1.0],
        "NaN colour",
    );

    let mut s = 0x6163_756du32;
    for i in 0..6000u32 {
        let sum = [
            fuzz_scales(&mut s),
            fuzz_scales(&mut s),
            fuzz_scales(&mut s),
        ];
        let sh = [
            fuzz_scales(&mut s),
            fuzz_scales(&mut s),
            fuzz_scales(&mut s),
        ];
        let dir = [
            fuzz_scales(&mut s),
            fuzz_scales(&mut s),
            fuzz_scales(&mut s),
        ];
        run_case(&mut e, sum, sh, dir, &format!("fuzz[{i}]"));
    }
}

/// `0x71bce0` whole, with and without the matrix at the record's `+0x0`: a random record, colour
/// and direction, every field from `+0x18` to `+0xec` read back.
#[test]
fn m2_light_add_directional_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut rng = Lcg(0x6c69_6774);
    let accum = emu.alloc(0x100);
    let world = emu.alloc(0x100);
    let color = emu.alloc(0x10);
    let dir = emu.alloc(0x10);
    const OUTER: u64 = 0x18;
    const COLOR_SUM: u64 = 0x48;
    const SH_R: u64 = 0x84;
    const SH_G: u64 = 0xa8;
    const SH_B: u64 = 0xcc;

    let run_case = |emu: &mut Emu,
                    prior: LightAccum,
                    col: [f32; 3],
                    d: [f32; 3],
                    m3: Option<[[f32; 3]; 3]>,
                    ctx: &str| {
        emu.write_u32(accum, if m3.is_some() { world as u32 } else { 0 });
        for i in 0..12 {
            emu.write_u32(accum + OUTER + 4 * i as u64, prior.outer[i].to_bits());
        }
        for i in 0..3 {
            let v = prior.color_sum[i].to_bits();
            emu.write_u32(accum + COLOR_SUM + 4 * i as u64, v);
        }
        for i in 0..9 {
            emu.write_u32(accum + SH_R + 4 * i as u64, prior.sh_r[i].to_bits());
            emu.write_u32(accum + SH_G + 4 * i as u64, prior.sh_g[i].to_bits());
            emu.write_u32(accum + SH_B + 4 * i as u64, prior.sh_b[i].to_bits());
        }
        if let Some(m) = m3 {
            // Row r, column c at `+0x9c + c·0x10 + r·4`.
            for r in 0..3 {
                for c in 0..3 {
                    let off = 0x9c + (c as u64) * 0x10 + (r as u64) * 4;
                    emu.write_u32(world + off, m[r][c].to_bits());
                }
            }
        }
        for i in 0..3 {
            emu.write_u32(color + 4 * i as u64, col[i].to_bits());
            emu.write_u32(dir + 4 * i as u64, d[i].to_bits());
        }
        let _ = emu.call(0x71bce0, accum as u32, 0, &[color as u32, dir as u32]);
        let ours = m2_light_add_directional(prior, col, d, m3);
        for i in 0..12 {
            let got = emu.read_f32(accum + OUTER + 4 * i as u64);
            assert_f32(got, ours.outer[i], &format!("{ctx} outer[{i}]"));
        }
        for i in 0..3 {
            let got = emu.read_f32(accum + COLOR_SUM + 4 * i as u64);
            assert_f32(got, ours.color_sum[i], &format!("{ctx} color_sum[{i}]"));
        }
        for i in 0..9 {
            let off = 4 * i as u64;
            assert_f32(
                emu.read_f32(accum + SH_R + off),
                ours.sh_r[i],
                &format!("{ctx} sh_r[{i}]"),
            );
            assert_f32(
                emu.read_f32(accum + SH_G + off),
                ours.sh_g[i],
                &format!("{ctx} sh_g[{i}]"),
            );
            assert_f32(
                emu.read_f32(accum + SH_B + off),
                ours.sh_b[i],
                &format!("{ctx} sh_b[{i}]"),
            );
        }
    };

    run_case(
        &mut emu,
        LightAccum::zeroed(),
        [1.0; 3],
        [0.0, 0.0, 1.0],
        None,
        "unit z",
    );
    run_case(
        &mut emu,
        LightAccum::zeroed(),
        [1.0; 3],
        [f32::NAN, 0.0, 0.0],
        None,
        "NaN dir",
    );
    for use_matrix in [false, true] {
        for i in 0..250 {
            let mut prior = LightAccum::zeroed();
            prior.outer = std::array::from_fn(|_| rng.f32_mixed());
            prior.color_sum = std::array::from_fn(|_| rng.f32_mixed());
            for k in 0..9 {
                prior.sh_r[k] = rng.f32_mixed();
                prior.sh_g[k] = rng.f32_mixed();
                prior.sh_b[k] = rng.f32_mixed();
            }
            let col = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
            let d = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
            let m3 = use_matrix.then(|| {
                let mut m = [[0f32; 3]; 3];
                for row in m.iter_mut() {
                    for v in row.iter_mut() {
                        *v = rng.f32_mixed();
                    }
                }
                m
            });
            let ctx = format!("fuzz[{i}] matrix={use_matrix}");
            run_case(&mut emu, prior, col, d, m3, &ctx);
        }
    }
}

/// `0x71bf60` whole: the three sums at `+0x6c` seeded, called, read back.
#[test]
fn m2light_accum_vec3_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x100);
    let vec = e.alloc(0x10);

    let run_case = |e: &mut Emu, cur: [f32; 3], v: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(obj + 0x6c + 4 * k as u64, cur[k].to_bits());
            e.write_u32(vec + 4 * k as u64, v[k].to_bits());
        }
        e.call(0x71bf60, obj as u32, 0, &[vec as u32]);
        let ours = m2light_accum_vec3(cur, v);
        for k in 0..3 {
            let got = e.read_f32(obj + 0x6c + 4 * k as u64);
            assert_f32(got, ours[k], &format!("{ctx} [{k}]"));
        }
    };

    run_case(&mut e, [0.0, -0.0, 0.0], [-0.0, 0.0, -0.0], "zeros");
    run_case(&mut e, [1.0, 2.0, 3.0], [-1.0, -2.0, -3.0], "cancel");
    let inf = f32::INFINITY;
    run_case(&mut e, [inf, 1.0, -inf], [1.0, inf, inf], "infinities");
    run_case(&mut e, [f32::NAN, 1.0, 2.0], [1.0, 2.0, 3.0], "NaN");

    let mut s = 0x71bf_6060u32;
    for i in 0..3000u32 {
        let cur = [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)];
        let v = [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)];
        run_case(&mut e, cur, v, &format!("fuzz[{i}]"));
    }
}

/// `0x71bf90` from `0x71bfb0` to `0x71bff5` in an island with `fstp qword` appended: the light at
/// `ebx` (`+0xc`), the record at `ecx` (`+0x4`); the squared distance read as the `f64` it is kept
/// as.
#[test]
fn m2_light_aggregate_dist_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SRC: u64 = 0x71bfb0;
    const LEN: usize = 0x71bff5 - 0x71bfb0;
    let mut code = e.read(SRC, LEN);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x100);
    code.extend_from_slice(&[0xdd, 0x1d]);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl, SRC, LEN);
    let end = isl + code.len() as u64;
    let light = e.alloc(0x40);
    let probe = e.alloc(0x40);
    let frame = e.alloc(0x100);

    let run_case = |e: &mut Emu, lpos: [f32; 3], pc: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(light + 0xc + 4 * k as u64, lpos[k].to_bits());
            e.write_u32(probe + 4 + 4 * k as u64, pc[k].to_bits());
        }
        e.set_reg(RegisterX86::EBX, light);
        e.set_reg(RegisterX86::ECX, probe);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, frame + 0x80);
        e.run(isl, end);
        let ours = m2_light_aggregate_dist(lpos, pc);
        assert_f64(read_f64(e, out), ours, ctx);
    };

    run_case(&mut e, [5.0; 3], [5.0; 3], "zero distance");
    run_case(&mut e, [1.0, 0.0, 0.0], [0.0; 3], "dx 1");
    run_case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], "25");
    run_case(&mut e, [0.0; 3], [3.0, 4.0, 0.0], "25, negative");
    run_case(&mut e, [1.0e20, 0.0, 0.0], [0.0; 3], "huge");
    run_case(&mut e, [f32::NAN, 0.0, 0.0], [0.0; 3], "NaN");

    let mut s = 0x6167_6772u32;
    for i in 0..6000u32 {
        let lpos = [
            fuzz_scales(&mut s),
            fuzz_scales(&mut s),
            fuzz_scales(&mut s),
        ];
        let pc = [
            fuzz_scales(&mut s),
            fuzz_scales(&mut s),
            fuzz_scales(&mut s),
        ];
        run_case(&mut e, lpos, pc, &format!("fuzz[{i}]"));
    }
}

/// `0x70dc40` from its start to `0x70dc52` writes the grid scale, checked against
/// [`GRID_SCALE`]; then `0x70a0f0`'s four grid coordinates, from `0x70a122` to `0x70a13a`,
/// `0x70a144` to `0x70a15e`, `0x70a168` to `0x70a183` and `0x70a18d` to `0x70a1aa`, each read as
/// the `f64` stored for `floor`. The sphere is at `edi`: centre `+0x4`, `+0x8`, radius `+0x10`.
#[test]
fn attach_light_grid_window_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.run(0x70dc40, 0x70dc52);
    let scale = e.read_f32(GRID_SCALE_VA);
    assert_eq!(scale.to_bits(), GRID_SCALE.to_bits(), "grid scale");

    let bbox = e.alloc(0x40);
    let frame = e.alloc(0x100);
    let run_case = |e: &mut Emu, cy: f32, cz: f32, ext: f32, ctx: &str| {
        e.write_u32(bbox + 0x4, cy.to_bits());
        e.write_u32(bbox + 0x8, cz.to_bits());
        e.write_u32(bbox + 0x10, ext.to_bits());
        e.set_reg(RegisterX86::EDI, bbox);
        e.set_reg(RegisterX86::EBP, frame + 0x80);
        // The first stretch makes the `f64` slot with `sub esp,8`; the others reuse it.
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x70a122, 0x70a13a);
        let y_min = read_f64(e, SLICE_ESP - 8);
        let mut rest = [0f64; 3];
        for (k, (from, to)) in [
            (0x70a144, 0x70a15e),
            (0x70a168, 0x70a183),
            (0x70a18d, 0x70a1aa),
        ]
        .into_iter()
        .enumerate()
        {
            e.set_reg(RegisterX86::ESP, SLICE_ESP - 8);
            e.run(from, to);
            rest[k] = read_f64(e, SLICE_ESP - 8);
        }
        let ours = attach_light_grid_window(cy, cz, ext, scale);
        assert_f64(y_min, ours.y_min, &format!("{ctx} y_min"));
        assert_f64(rest[0], ours.y_max, &format!("{ctx} y_max"));
        assert_f64(rest[1], ours.z_min, &format!("{ctx} z_min"));
        assert_f64(rest[2], ours.z_max, &format!("{ctx} z_max"));
    };

    run_case(&mut e, 0.0, 0.0, 0.0, "zero");
    run_case(&mut e, 10.0, 20.0, 0.0, "zero radius");
    run_case(&mut e, 100.0, -100.0, 5.0, "spread");
    let half_cell = (0.5 / f64::from(scale)) as f32;
    run_case(&mut e, half_cell, half_cell, 0.0, "a cell edge");
    run_case(&mut e, -640.0, 640.0, 33.333, "a tile");
    run_case(&mut e, f32::NAN, 0.0, 1.0, "NaN");

    let mut rng = Lcg(0x70a0_f000);
    for i in 0..4000u32 {
        let (cy, cz, ext) = (rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed().abs());
        run_case(&mut e, cy, cz, ext, &format!("fuzz[{i}]"));
    }
}

/// `0x71ccf0` stores 0.05 at `0xcf04d0`; then `0x71bb60` from `0x71bbb3` to `0x71bbc2` and from
/// `0x71bbcc` to `0x71bbdd`, with other scales written there: the coordinate at `esi+0xc` or
/// `esi+0x10`, the `f64` pushed for `floor` read from `[esp]`.
#[test]
fn grid_scale_product_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SCALE: u64 = 0x00cf_04d0;
    e.run(0x71ccf0, 0x71cd02);
    assert_eq!(
        e.read_u32(SCALE),
        GRID_SCALE.to_bits(),
        "the scale 0x71ccf0 stores"
    );
    let esi = e.alloc(0x40);
    let frame = e.alloc(0x100);

    let run_site = |e: &mut Emu, second: bool, scale: f32, coord: f32, ctx: &str| {
        e.write_u32(SCALE, scale.to_bits());
        e.write_u32(esi + if second { 0x10 } else { 0xc }, coord.to_bits());
        e.set_reg(RegisterX86::ESI, esi);
        e.set_reg(RegisterX86::EBP, frame + 0x80);
        e.set_reg(RegisterX86::EAX, 0);
        if second {
            e.set_reg(RegisterX86::ESP, SLICE_ESP - 8);
            e.run(0x71bbcc, 0x71bbdd);
        } else {
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.run(0x71bbb3, 0x71bbc2);
        }
        let bin = read_f64(e, e.get_reg(RegisterX86::ESP));
        assert_f64(
            bin,
            grid_scale_product(scale, coord),
            &format!("{ctx} second={second}"),
        );
    };

    for &(sc, co, name) in &[
        (1.0f32, 0.0f32, "1·0"),
        (1.0, 63.0, "1·63"),
        (0.5, 32.0, "0.5·32"),
        (-1.0, 10.0, "negative scale"),
        (0.015625, 100.0, "1/64·100"),
        (1.0e6, 1.0e-6, "huge·tiny"),
        (1.123_45, 9.876_54, "inexact"),
        (0.05, f32::NAN, "NaN"),
    ] {
        run_site(&mut e, false, sc, co, name);
        run_site(&mut e, true, sc, co, name);
    }
    let mut s = 0x71bb_6000u32;
    for i in 0..5000u32 {
        let (scale, coord) = (fuzz_scales(&mut s), fuzz_scales(&mut s));
        run_site(&mut e, false, scale, coord, &format!("fuzz[{i}]"));
        run_site(&mut e, true, scale, coord, &format!("fuzz[{i}]"));
    }
}

/// `0x71c110` whole: start `+0x184`, end `+0x188` and `1/(end − start)` `+0x18c` read back. Equal
/// ends divide by zero.
#[test]
fn fade_set_range_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x300);
    let vec3 = e.alloc(0x10);

    let run_case = |e: &mut Emu, start: f32, end: f32, ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(vec3 + 4 * k, 0x1111_1111 * (k as u32 + 1));
            e.write_u32(this + 0x184 + 4 * k, 0xdead_beef);
        }
        let args = [vec3 as u32, start.to_bits(), end.to_bits()];
        let _ = e.call(0x71c110, this as u32, 0, &args);
        let ours = fade_set_range(start, end);
        assert_f32(
            e.read_f32(this + 0x184),
            ours.start,
            &format!("{ctx} start"),
        );
        assert_f32(e.read_f32(this + 0x188), ours.end, &format!("{ctx} end"));
        assert_f32(
            e.read_f32(this + 0x18c),
            ours.inv_range,
            &format!("{ctx} inv"),
        );
    };

    run_case(&mut e, 5.0, 5.0, "equal");
    run_case(&mut e, 0.0, 1.0, "span 1");
    run_case(&mut e, 1.0, 0.0, "span −1");
    run_case(&mut e, 0.0, 2.0, "span 2");
    run_case(&mut e, 0.0, 4.0, "span 4");
    run_case(&mut e, -1.0, -1.0, "equal negatives");
    run_case(&mut e, 1.0e30, 1.0e-30, "huge and tiny");
    run_case(&mut e, f32::from_bits(1), 0.0, "subnormal start");
    run_case(&mut e, 0.0, f32::NAN, "NaN end");

    let mut s = 0x71c1_1010u32;
    for i in 0..4000u32 {
        let start = fuzz_f32(&mut s);
        let end = if next(&mut s) & 0x7 == 0 {
            start
        } else {
            fuzz_f32(&mut s)
        };
        run_case(&mut e, start, end, &format!("fuzz[{i}]"));
    }
}

/// `0x71c4e0` from `0x71c4ee` to `0x71c70e`: the terms from `+0x84` to `+0xec` written, the 27
/// constants from `+0xf0` read back. The weights at `0x8120a8` to `0x8120c4` are checked against
/// the bits the model uses.
#[test]
fn sh_light_pack_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for (va, bits) in [
        (0x8120a8, 0x3e0b_d8a1),
        (0x8120ac, 0x3e72_3882),
        (0x8120b0, 0xbe8b_d8a1),
        (0x8120b4, 0x3e8b_d8a1),
        (0x8120b8, 0x3da1_7b01),
        (0x8120bc, 0x3e90_6ebb),
        (0x8120c0, 0x3ea6_c6bd),
        (0x8120c4, 0xbea6_c6bd),
    ] {
        assert_eq!(e.read_u32(va), bits, "weight at {va:#x}");
    }
    let strct = e.alloc(0x200);
    let in_offs: Vec<u32> = (0x84u32..=0xec).step_by(4).collect();

    let run_case = |e: &mut Emu, vals: &[(u32, f32)], ctx: &str| {
        for &(off, v) in vals {
            e.write_u32(strct + u64::from(off), v.to_bits());
        }
        e.write_u32(strct + 0x14, 0);
        e.set_reg(RegisterX86::ECX, strct);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(0x71c4ee, 0x71c70e);
        let read_in = |off: u32| vals.iter().find(|&&(o, _)| o == off).unwrap().1;
        let ours = sh_light_pack(&read_in);
        for (k, our) in ours.iter().enumerate() {
            let off = 0xf0 + 4 * k as u64;
            assert_f32(e.read_f32(strct + off), *our, &format!("{ctx}: +{off:#x}"));
        }
    };

    let fill = |v: f32| -> Vec<(u32, f32)> { in_offs.iter().map(|&o| (o, v)).collect() };
    run_case(&mut e, &fill(1.0), "ones");
    run_case(&mut e, &fill(0.0), "zeros");
    run_case(&mut e, &fill(f32::NAN), "NaN");
    let mut s = 0x71c4_eeeeu32;
    for i in 0..3000u32 {
        let vals: Vec<(u32, f32)> = in_offs
            .iter()
            .map(|&o| (o, frand(&mut s, -100.0, 100.0)))
            .collect();
        run_case(&mut e, &vals, &format!("fuzz[{i}]"));
    }
}

/// `0x71c2f0` whole on a record with flag 2 clear: the direction, matrix, colour sum and ambient
/// sum written, the direction `+0x78`, colour `+0x60` and ambient `+0x54` read back. A squared
/// length of exactly 9.9999997e-6, one step above it, zero and NaN probe the floor gate.
#[test]
fn camera_facing_basis_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x100);
    let mut rng = Lcg(0x6362_6173);

    let run_case = |emu: &mut Emu, inp: CameraBasisIn, ctx: &str| {
        emu.write_u32(this + 0x14, 0);
        for i in 0..3 {
            emu.write_u32(this + 0x3c + 4 * i as u64, inp.src_dir[i].to_bits());
            emu.write_u32(this + 0x48 + 4 * i as u64, inp.vec[i].to_bits());
            emu.write_u32(this + 0x54 + 4 * i as u64, inp.accum_prev[i].to_bits());
        }
        for i in 0..9 {
            emu.write_u32(this + 0x18 + 4 * i as u64, inp.m[i].to_bits());
        }
        for off in [0x60u64, 0x64, 0x68, 0x78, 0x7c, 0x80] {
            emu.write_u32(this + off, 0xdead_beef);
        }
        let _ = emu.call(0x71c2f0, this as u32, 0, &[]);
        let ours = camera_facing_basis(inp);
        for k in 0..3 {
            let o = 4 * k as u64;
            assert_f32(
                emu.read_f32(this + 0x78 + o),
                ours.dir[k],
                &format!("{ctx} dir[{k}]"),
            );
            assert_f32(
                emu.read_f32(this + 0x60 + o),
                ours.basis[k],
                &format!("{ctx} basis[{k}]"),
            );
            assert_f32(
                emu.read_f32(this + 0x54 + o),
                ours.accum[k],
                &format!("{ctx} accum[{k}]"),
            );
        }
    };
    let gen_in = |rng: &mut Lcg, src_dir: [f32; 3]| CameraBasisIn {
        src_dir,
        m: std::array::from_fn(|_| rng.f32_mixed()),
        vec: [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()],
        accum_prev: [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()],
    };

    // `(x² + y²) + z²` is exactly the floor for these.
    let at_floor = [
        f32::from_bits(0x3b4f_2000),
        f32::from_bits(0x38cc_0000),
        f32::from_bits(0x3838_0000),
    ];
    let [x, y, z] = at_floor.map(f64::from);
    assert_eq!((x * x + y * y) + z * z, f64::from(FLOOR_EPS));
    let above = [f32::from_bits(0x3b4f_2001), at_floor[1], at_floor[2]];
    let nan = f32::NAN;
    for (dir, ctx) in [
        ([0.0, 0.0, 0.0], "zero"),
        (at_floor, "at the floor"),
        (above, "above the floor"),
        ([1.0, 0.0, 0.0], "unit x"),
        ([1.0, 2.0, 3.0], "generic"),
        ([1e-4, 0.0, 0.0], "below the floor"),
        ([nan, 0.0, 0.0], "NaN"),
        ([1.0, nan, 1.0], "NaN in y"),
        ([f32::INFINITY, 0.0, 0.0], "infinite"),
    ] {
        let inp = gen_in(&mut rng, dir);
        run_case(&mut emu, inp, ctx);
    }
    for i in 0..4000u32 {
        let src_dir = if (rng.next() >> 8).is_multiple_of(4) {
            [0, 1, 2].map(|_| rng.f32_mixed() * 1.0e-4)
        } else {
            [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()]
        };
        let inp = gen_in(&mut rng, src_dir);
        run_case(&mut emu, inp, &format!("fuzz[{i}]"));
    }
}

/// `0x71c160` from `0x71c1d7` to `0x71c2d9`, with both calls of `TransformPoint` (`0x7bca80`)
/// run: the record at `edi`, the matrix at `[edi]+0x9c`, the plane at `+0x1a0` and `+0x1ac` read
/// back. A direction whose length is exactly 2.3841858e-7 after the rotation, zero, NaN and the
/// identity probe the gate.
#[test]
fn m2_light_coord_finalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x200);
    let bone = e.alloc(0x100);
    let frame = e.alloc(0x100);

    let run_case = |e: &mut Emu, m: &[f32; 16], dir: [f32; 3], d: f32, ctx: &str| {
        for (j, &f) in m.iter().enumerate() {
            e.write_u32(bone + 0x9c + 4 * j as u64, f.to_bits());
        }
        e.write_u32(obj, bone as u32);
        for k in 0..3 {
            e.write_u32(obj + 0x1a0 + 4 * k as u64, dir[k].to_bits());
        }
        e.write_u32(obj + 0x1ac, d.to_bits());
        e.set_reg(RegisterX86::EDI, obj);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, frame + 0x80);
        e.run(0x71c1d7, 0x71c2d9);
        let ours = m2_light_coord_finalize(m, dir, d);
        for k in 0..3 {
            let got = e.read_f32(obj + 0x1a0 + 4 * k as u64);
            assert_f32(got, ours.dir[k], &format!("{ctx} dir[{k}]"));
        }
        assert_f32(e.read_f32(obj + 0x1ac), ours.plane_d, &format!("{ctx} d"));
    };

    let mut identity = [0f32; 16];
    for k in [0, 5, 10, 15] {
        identity[k] = 1.0;
    }
    for &s in &[2.0f32, 1.0, 5.0, -3.0, 0.5] {
        run_case(
            &mut e,
            &identity,
            [1.0, 0.0, 0.0],
            s,
            &format!("identity, d={s}"),
        );
    }
    let mut moved = identity;
    moved[12..15].copy_from_slice(&[3.0, -2.0, 7.0]);
    run_case(&mut e, &moved, [0.0, 1.0, 0.0], 4.0, "translated");
    run_case(&mut e, &identity, [0.0; 3], 1.0, "zero");
    run_case(&mut e, &identity, [0.0, 0.0, 1.0], 2.0, "unit z");
    run_case(&mut e, &identity, [3.0, 4.0, 0.0], 1.0, "length 5");
    run_case(&mut e, &identity, [1.0e-9, 0.0, 0.0], 1.0, "tiny");
    run_case(
        &mut e,
        &identity,
        [NORM_EPS, 0.0, 0.0],
        1.0,
        "length == eps",
    );
    let below = f32::from_bits(NORM_EPS.to_bits() - 1);
    run_case(&mut e, &identity, [below, 0.0, 0.0], 1.0, "just below eps");
    run_case(&mut e, &identity, [f32::NAN, 0.0, 0.0], 1.0, "NaN");
    run_case(&mut e, &identity, [1.0, 0.0, 0.0], f32::NAN, "NaN d");

    let mut st = 0x6c62_6e65u32;
    for i in 0..4000u32 {
        let m: [f32; 16] = std::array::from_fn(|_| fuzz_scales(&mut st));
        let dir = [
            fuzz_scales(&mut st),
            fuzz_scales(&mut st),
            fuzz_scales(&mut st),
        ];
        let d = fuzz_scales(&mut st);
        run_case(&mut e, &m, dir, d, &format!("fuzz[{i}]"));
    }
}

/// `0x71b7c0` from its start to the call of `0x589d80` at `0x71bb16`, with `+0x8` zero: nine
/// channels in, the three packed words read from `[ebp-0x3c]`, `[ebp-0x34]` and `[ebp-0x38]`.
/// Exactly 0 and 1 and NaN probe the ladder.
#[test]
fn light_clamp_pack_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x80);
    let entry_esp: u64 = SLICE_ESP;
    let ebp: u64 = entry_esp - 4;
    // The channels' offsets: the first triple, the second, the third.
    const OFFS: [u64; 9] = [0x30, 0x34, 0x38, 0x48, 0x4c, 0x50, 0x3c, 0x40, 0x44];

    let run_case = |e: &mut Emu, chans: [f32; 9], aux: [u32; 4], ctx: &str| {
        for (off, c) in OFFS.iter().zip(chans) {
            e.write_u32(this + off, c.to_bits());
        }
        e.write_u32(this + 0x8, 0);
        for (off, v) in [
            (0x60, 0),
            (0x24, 1),
            (0x28, 2),
            (0x2c, 3),
            (0x54, 0),
            (0x58, 1),
            (0x5c, 2),
        ] {
            e.write_u32(this + off, aux[v]);
        }
        e.set_reg(RegisterX86::ECX, this);
        e.set_reg(RegisterX86::ESP, entry_esp);
        e.run(0x71b7c0, 0x71bb16);
        let d1 = e.read_u32(ebp - 0x3c);
        let d2 = e.read_u32(ebp - 0x34);
        let d3 = e.read_u32(ebp - 0x38);
        let o1 = light_pack_triple(chans[2], chans[1], chans[0]);
        let o2 = light_pack_triple(chans[5], chans[4], chans[3]);
        let o3 = light_pack_triple3(chans[8], chans[7], chans[6]);
        assert_eq!(d1, o1, "{ctx} first binary={d1:#010x} ours={o1:#010x}");
        assert_eq!(d2, o2, "{ctx} second binary={d2:#010x} ours={o2:#010x}");
        assert_eq!(d3, o3, "{ctx} third binary={d3:#010x} ours={o3:#010x}");
    };

    for (v, ctx) in [
        (0.0, "zero"),
        (-0.0, "−0"),
        (1.0, "one"),
        (0.5, "half"),
        (-1.0, "negative"),
        (2.0, "two"),
        (f32::from_bits(0x3f7f_ffff), "just below one"),
        (f32::NAN, "NaN"),
    ] {
        run_case(&mut e, [v; 9], [0; 4], ctx);
    }
    let sweep = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
    run_case(
        &mut e,
        sweep,
        [0xdead_beef, 0x1234_5678, 0xfeed_face, 0x0bad_f00d],
        "sweep",
    );

    let mut rng = Lcg(0x71b7_c000);
    for i in 0..3000u32 {
        let mut chans = [0f32; 9];
        for c in chans.iter_mut() {
            let m = |rng: &mut Lcg| f64::from(rng.next() >> 8) / f64::from(1u32 << 24);
            *c = match rng.next() >> 30 {
                0 => m(&mut rng) as f32,
                1 => rng.f32_mixed(),
                2 => (m(&mut rng) as f32) * 1.5,
                _ => -(m(&mut rng) as f32),
            };
        }
        let aux = [rng.next(), rng.next(), rng.next(), rng.next()];
        run_case(&mut e, chans, aux, &format!("fuzz[{i}]"));
    }
}

/// `0x71c730` from `0x71c820` to `0x71c843`: the light at `esi` (`+0xc`), the model's position at
/// `eax`; read from `[ebp-0x24]`.
#[test]
fn driver_vec_diff_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let vert = e.alloc(0x40);
    let sub = e.alloc(0x20);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, pos: [f32; 3], s: [f32; 3], ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(vert + 0xc + 4 * k, pos[k as usize].to_bits());
            e.write_u32(sub + 4 * k, s[k as usize].to_bits());
        }
        e.write_u32(ebp - 0x4, 0);
        e.set_reg(RegisterX86::ESI, vert);
        e.set_reg(RegisterX86::EAX, sub);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x71c820, 0x71c843);
        let ours = driver_vec_diff(pos, s);
        for k in 0..3 {
            let got = e.read_f32(ebp - 0x24 + 4 * k as u64);
            assert_f32(got, ours[k], &format!("{ctx} [{k}]"));
        }
    };

    run_case(&mut e, [1.0, 2.0, 3.0], [0.5, -1.0, 4.0], "generic");
    run_case(&mut e, [0.0; 3], [0.0; 3], "zero");
    run_case(&mut e, [1.0e30, 1.0, 1.0], [1.0e30, 1.0, 1.0], "cancel");
    let mut rng = Lcg(0x7664_6966);
    for i in 0..3000u32 {
        let pos = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        let s = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        run_case(&mut e, pos, s, &format!("fuzz[{i}]"));
    }
}

/// `0x71c730` from `0x71c902` to `0x71c912` in an island with `mov [out],eax` appended: `|x|` at
/// `edi+0x18c` compared with 2.3841858e-7; the branch `jnp` would take derived from `ah`.
/// Exactly that value, one step either side, zero and NaN probe it.
#[test]
fn driver_mag_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SRC: u64 = 0x71c902;
    const LEN: usize = 0x71c912 - 0x71c902;
    let mut code = e.read(SRC, LEN);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    code.push(0xa3);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl, SRC, LEN);
    let end = isl + code.len() as u64;
    let this = e.alloc(0x200);

    let run_case = |e: &mut Emu, x: f32, ctx: &str| {
        e.write_u32(this + 0x18c, x.to_bits());
        e.set_reg(RegisterX86::EDI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(isl, end);
        let ah = (e.read_u32(out) >> 8) & 0xff;
        // `test ah,5; jnp` skips the colour on odd parity, so it packs on even.
        let packs = (ah & 0x05).count_ones().is_multiple_of(2);
        let ours = driver_mag_gate(x);
        assert_eq!(
            packs, ours,
            "{ctx}: binary={packs} ours={ours} (ah={ah:#04x})"
        );
    };

    let eps = NORM_EPS;
    for (x, ctx) in [
        (eps, "== eps"),
        (f32::from_bits(eps.to_bits() - 1), "just below"),
        (f32::from_bits(eps.to_bits() + 1), "just above"),
        (-eps, "−eps"),
        (0.0, "0"),
        (-0.0, "−0"),
        (f32::NAN, "NaN"),
        (100.0, "big"),
        (-100.0, "−big"),
        (1.0e-10, "tiny"),
    ] {
        run_case(&mut e, x, ctx);
    }
    let mut rng = Lcg(0x6761_7465);
    for i in 0..3000u32 {
        let x = match rng.next() >> 30 {
            0 => eps * (rng.f32_mixed() * 1.0e-3 + 1.0),
            1 => rng.f32_mixed() * 1.0e-7,
            _ => rng.f32_mixed(),
        };
        run_case(&mut e, x, &format!("fuzz[{i}]"));
    }
}

/// `0x71c730` from `0x71c91b` to `0x71ca2a`, past the gate: the channels at `edi+0x198`, `+0x194`,
/// `+0x190`; the packed word stored through `[ebp+8]`, which the stretch points at its own push.
#[test]
fn driver_clamp_pack_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let frame = e.alloc(0x100);
    let pack_slot = SLICE_ESP - 4;

    let run_case = |e: &mut Emu, c198: f32, c194: f32, c190: f32, ctx: &str| {
        e.write_u32(this + 0x198, c198.to_bits());
        e.write_u32(this + 0x194, c194.to_bits());
        e.write_u32(this + 0x190, c190.to_bits());
        e.write_u32(pack_slot, 0xdead_beef);
        e.set_reg(RegisterX86::EDI, this);
        e.set_reg(RegisterX86::EBP, frame + 0x80);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x71c91b, 0x71ca2a);
        let bin = e.read_u32(pack_slot);
        let ours = light_pack_triple3(c198, c194, c190);
        assert_eq!(bin, ours, "{ctx}: binary={bin:#010x} ours={ours:#010x}");
    };

    run_case(&mut e, 0.0, 0.0, 0.0, "zero");
    run_case(&mut e, 1.0, 1.0, 1.0, "one");
    run_case(&mut e, 0.5, 0.5, 0.5, "half");
    run_case(&mut e, -1.0, -1.0, -1.0, "negative");
    run_case(&mut e, 2.0, 2.0, 2.0, "two");
    run_case(&mut e, 0.1, 0.5, 0.9, "distinct");
    run_case(&mut e, f32::NAN, 0.5, 1.0, "NaN");
    let mut rng = Lcg(0x7063_6b30);
    for i in 0..3000u32 {
        let (a, b, c) = (rng.clamp_chan(), rng.clamp_chan(), rng.clamp_chan());
        run_case(&mut e, a, b, c, &format!("fuzz[{i}]"));
    }
}

/// `0x71ca80` whole: the colour and seed in, the packed word read through `ecx`. Channels all
/// below 9.9999997e-6, exactly it, and NaN in the seed or a channel probe the gates.
#[test]
fn m2_light_color_pack_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut rng = Lcg(0x6c63_6c72);
    let out = emu.alloc(0x10);
    let max_scratch = emu.alloc(0x10);
    let color = emu.alloc(0x10);

    let run_case = |emu: &mut Emu, col: [f32; 3], seed: f32, ctx: &str| {
        for i in 0..3 {
            emu.write_u32(color + 4 * i as u64, col[i].to_bits());
        }
        emu.write_u32(out, 0xdead_beef);
        emu.write_u32(max_scratch, 0xdead_beef);
        let args = [color as u32, seed.to_bits()];
        let _ = emu.call(0x71ca80, out as u32, max_scratch as u32, &args);
        let ours = m2_light_color_pack(col, seed);
        let bin = emu.read_u32(out);
        assert_eq!(bin, ours, "{ctx} binary={bin:#010x} ours={ours:#010x}");
    };

    let fl = FLOOR_EPS;
    let below = f32::from_bits(fl.to_bits() - 1);
    let nan = f32::NAN;
    run_case(&mut emu, [fl, 0.0, 0.0], 0.0, "max == floor");
    run_case(&mut emu, [below, 0.0, 0.0], below, "max just below floor");
    run_case(&mut emu, [0.5, 0.25, 1.0], nan, "NaN seed");
    run_case(&mut emu, [nan, 0.25, 1.0], 0.5, "NaN channel");
    run_case(&mut emu, [nan, nan, nan], nan, "all NaN");
    run_case(&mut emu, [0.5, 0.5, 0.5], 0.5, "ties");
    for kind in 0..3u32 {
        for i in 0..250 {
            let tiny = |rng: &mut Lcg| rng.f32_mixed().abs() * 1.0e-9;
            let (col, seed) = if kind == 1 {
                (
                    [tiny(&mut rng), tiny(&mut rng), tiny(&mut rng)],
                    tiny(&mut rng),
                )
            } else {
                let col = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
                (col, rng.f32_mixed())
            };
            run_case(&mut emu, col, seed, &format!("kind {kind} fuzz[{i}]"));
        }
    }
}
