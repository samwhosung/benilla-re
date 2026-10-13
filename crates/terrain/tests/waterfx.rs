//! The liquid tables and the ripples against `0x68c4c0`, `0x68f5c0` and `0x68fae0`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::waterfx::{
    corner_ray_negate, decal_age_fade, decal_age_step, decal_color_pack, decal_jitter,
    decal_pos_integrate, decal_pos_minus, decal_pos_plus, geom_rate, glare_entry, ocean_ramp_entry,
    ocean_ramp_recip, ripple_params, ripple_surface, river_ramp_entry, DECAL_JITTER, GLARE_COEFF,
    GLARE_STEP, RATE_TABLE, RIVER_KNEE, RIVER_SCALE, RIVER_SLOPE,
};
use common::{assert_f32, next};

/// A frame inside the harness stack (`0x70000000`, 0x100000 bytes).
const SLICE_ESP: u64 = 0x7008_0000;

/// A float in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// The binary's AH after `fnstsw ax` — `(EAX >> 8) & 0xff`.
fn read_ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

const G_C81184: u64 = 0xc81184;

const WAVE_LUT: u64 = 0xc7fcd8;

const KNEE_LUT: u64 = 0xc81768;

const SPARKLE_LUT: u64 = 0xc7fbc0;

const G_C7B2A0: u64 = 0xc7b2a0;

const DECAL_DT: u64 = 0xc62510;

const CORNER_X: u64 = 0xc7cf20;

const CORNER_Y: u64 = 0xc7cf24;

const CORNER_Z: u64 = 0xc7cf28;

/// `InitLiquidRamps` (`0x68c4c0`) from `0x68c51b` to `0x68c5b9`, `c` at `[0xc81184]`: both ramps.
#[test]
fn depth_ramps_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, g: f32, ctx: &str| {
        e.write_u32(G_C81184, g.to_bits());
        e.set_reg(RegisterX86::EBX, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x68c51b, 0x68c5b9);
        let recip = ocean_ramp_recip(g);
        for idx in 0..256u32 {
            let bin_wave = e.read_f32(WAVE_LUT + 4 * u64::from(idx));
            assert_f32(
                bin_wave,
                ocean_ramp_entry(recip, g, idx),
                &format!("{ctx} wave[{idx}]"),
            );
            let bin_knee = e.read_f32(KNEE_LUT + 4 * u64::from(idx));
            assert_f32(
                bin_knee,
                river_ramp_entry(idx),
                &format!("{ctx} knee[{idx}]"),
            );
        }
    };

    run_case(&mut e, 0.0, "c 0");
    run_case(&mut e, f32::NAN, "c NaN");
    run_case(&mut e, 1.0, "radwave g=1.0");
    run_case(&mut e, 0.5, "radwave g=0.5");
    run_case(&mut e, 0.001, "radwave g=0.001 (recip large)");
    run_case(&mut e, 1000.0, "radwave g=1000 (recip tiny)");
    let mut s = 0x68c4_c0c0u32;
    for i in 0..400u32 {
        let g = frand(&mut s, 0.0001, 50.0);
        run_case(&mut e, g, &format!("radwave fuzz[{i}] g={g}"));
    }
}

/// `0x68c4c0` from `0x68c5c0` to `0x68c636`: the 64 dwords at `0xc7fbc0`.
#[test]
fn glare_table_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    e.set_reg(RegisterX86::EBX, 0);
    e.set_reg(RegisterX86::ECX, 0);
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.set_reg(RegisterX86::EBP, ebp);
    e.run(0x68c5c0, 0x68c636);
    for idx in 0..64u32 {
        let bin = e.read_u32(SPARKLE_LUT + 4 * u64::from(idx));
        assert_eq!(
            bin,
            glare_entry(idx),
            "sparkle[{idx}]: binary={bin:#010x} ours={:#010x}",
            glare_entry(idx)
        );
    }
}

/// `0x68c4c0` at `0x68c65c`: the bytes copied out with `fstp dword [out]` appended.
#[test]
fn geom_rate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let fld = e.read(0x68c65c, 6);
    let fmul = e.read(0x68c664, 6);
    assert_eq!(
        fld,
        vec![0xd9, 0x05, 0x3c, 0x02, 0x81, 0x00],
        "0x68c65c fld [0x81023c] drifted"
    );
    assert_eq!(
        fmul,
        vec![0xd8, 0x0d, 0xe4, 0x8d, 0x80, 0x00],
        "0x68c664 fmul [0x808de4] drifted"
    );
    for site in [0x68c6a4u64, 0x68c6d3] {
        assert_eq!(
            e.read(site, 6),
            fmul,
            "0x68c4c0 fmul [0x808de4] site {site:#x} drifted"
        );
    }

    let out = e.alloc(0x10);
    let isl = e.alloc(0x40);
    let mut code = fld.clone();
    code.extend_from_slice(&fmul);
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let end = isl + code.len() as u64;
    e.write(isl, &code);
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
    e.run(isl, end);
    let bin = e.read_f32(out);
    assert_f32(bin, geom_rate() as f32, "geom_rate 0.125*64");
}

/// `EmitRipple` (`0x68f5c0`) from `0x68f5c6` to `0x68f6a7`, edi the slot: the four fields, the stored sum
/// and the surface point.
#[test]
fn ripple_params_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out = e.alloc(0x80);
    let pt = e.alloc(0x20);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    #[allow(clippy::too_many_arguments)]
    let run_case = |e: &mut Emu,
                    p4: f32,
                    p5: f32,
                    p6: f32,
                    p7: f32,
                    g_off: f32,
                    pt3: [f32; 3],
                    p8: u32,
                    ctx: &str| {
        e.write_u32(ebp + 0x8, pt as u32);
        e.write_u32(ebp + 0xc, 0);
        e.write_u32(ebp + 0x10, p4.to_bits());
        e.write_u32(ebp + 0x14, p5.to_bits());
        e.write_u32(ebp + 0x18, p6.to_bits());
        e.write_u32(ebp + 0x1c, p7.to_bits());
        e.write_u32(ebp + 0x20, p8);
        for (k, c) in pt3.iter().enumerate() {
            e.write_u32(pt + 4 * k as u64, c.to_bits());
        }
        e.write_u32(G_C7B2A0, g_off.to_bits());
        e.set_reg(RegisterX86::ECX, out as u32 as u64);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x68f5c6, 0x68f6a7);

        let idx = (p8 != 0) as usize;
        let ours = ripple_params(p4, p5, p6, p7, g_off, idx);
        assert_f32(e.read_f32(out + 0x14), ours.out5, &format!("{ctx} out5"));
        assert_f32(e.read_f32(out + 0x20), ours.out8, &format!("{ctx} out8"));
        assert_f32(e.read_f32(out + 0x24), ours.out9, &format!("{ctx} out9"));
        assert_f32(e.read_f32(out + 0x28), ours.out10, &format!("{ctx} out10"));
        assert_f32(
            e.read_f32(ebp - 0x10),
            ours.local_14,
            &format!("{ctx} local_14"),
        );
        let local_14_live = f64::from(p6) * f64::from(p7) + f64::from(p4);
        let surf = ripple_surface(local_14_live, pt3);
        assert_f32(e.read_f32(ebp - 0x40), surf[0], &format!("{ctx} surf0"));
        assert_f32(e.read_f32(ebp - 0x3c), surf[1], &format!("{ctx} surf1"));
        assert_f32(e.read_f32(ebp - 0x38), surf[2], &format!("{ctx} surf2"));
    };

    run_case(
        &mut e,
        0.0,
        1.0,
        1.0,
        1.0,
        2.0,
        [1.0, 2.0, 3.0],
        0,
        "ripple p8=0 unit",
    );
    run_case(
        &mut e,
        1.0,
        2.0,
        0.5,
        0.25,
        -1.0,
        [0.0, 0.0, 0.0],
        1,
        "ripple p8=1",
    );
    let mut s = 0x68f5_c0c0u32;
    for i in 0..3000u32 {
        let p4 = frand(&mut s, -100.0, 100.0);
        let p5 = frand(&mut s, -100.0, 100.0);
        let p6 = frand(&mut s, 0.1, 100.0);
        let p7 = frand(&mut s, -10.0, 10.0);
        let g_off = frand(&mut s, -50.0, 50.0);
        let pt3 = [
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, -100.0, 100.0),
        ];
        let p8 = next(&mut s) >> 8 & 1;
        run_case(
            &mut e,
            p4,
            p5,
            p6,
            p7,
            g_off,
            pt3,
            p8,
            &format!("ripple fuzz[{i}]"),
        );
    }
}

/// `UpdateRipples` (`0x68fae0`) from `0x68fb8e` to `0x68fb9d`, ebx the ripple: `[ebx+0x10]`.
#[test]
fn decal_pos_integrate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let node = e.alloc(0x80);
    let run_case = |e: &mut Emu, g: f32, vel: f32, pos: f32, ctx: &str| {
        e.write_u32(DECAL_DT, g.to_bits());
        e.write_u32(node + 0x14, vel.to_bits());
        e.write_u32(node + 0x10, pos.to_bits());
        e.set_reg(RegisterX86::EBX, node);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(0x68fb8e, 0x68fb9d);
        assert_f32(
            e.read_f32(node + 0x10),
            decal_pos_integrate(g, vel, pos),
            ctx,
        );
    };
    run_case(&mut e, 0.0, 5.0, 3.0, "posint dt=0");
    run_case(&mut e, 1.0, 2.0, 1.0, "posint dt=1");
    let mut s = 0x68fb_8e8eu32;
    for i in 0..2000u32 {
        let g = frand(&mut s, -2.0, 2.0);
        let vel = frand(&mut s, -1000.0, 1000.0);
        let pos = frand(&mut s, -1000.0, 1000.0);
        run_case(&mut e, g, vel, pos, &format!("posint fuzz[{i}]"));
    }
}

/// `0x68fae0` from `0x68fb9d` to `0x68fbdd`, ebx the ripple: `[ebx+0x18]`.
#[test]
fn decal_age_fade_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let node = e.alloc(0x80);
    let run_case = |e: &mut Emu, g: f32, cur: f32, lim: f32, rate: f32, ctx: &str| {
        e.write_u32(DECAL_DT, g.to_bits());
        e.write_u32(node + 0x18, cur.to_bits());
        e.write_u32(node + 0x1c, lim.to_bits());
        e.write_u32(node + 0x20, rate.to_bits());
        e.set_reg(RegisterX86::EDI, 0);
        e.set_reg(RegisterX86::EBX, node);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(0x68fb9d, 0x68fbdd);
        let (cur2, _g2) = decal_age_fade(g, cur, lim, rate);
        assert_f32(e.read_f32(node + 0x18), cur2, &format!("{ctx} node[0x18]"));
    };

    run_case(
        &mut e,
        1.0,
        0.0,
        1.0,
        1.0,
        "fade cur'==lim (1·1+0==1, no clamp)",
    );
    run_case(&mut e, 1.0, 0.0, 0.5, 1.0, "fade cur'>lim (clamp)");
    run_case(&mut e, 0.5, 0.25, 10.0, 1.0, "fade cur'<lim");
    run_case(
        &mut e,
        0.0,
        2.0,
        1.0,
        0.5,
        "fade dt=0 (cur'==cur>lim → clamp)",
    );
    let mut s = 0x68fb_b3b3u32;
    for i in 0..2500u32 {
        let g = frand(&mut s, 0.0, 3.0);
        let cur = frand(&mut s, -5.0, 5.0);
        let lim = frand(&mut s, -5.0, 5.0);
        let mut rate = frand(&mut s, -3.0, 3.0);
        if rate == 0.0 {
            rate = 1.0;
        }
        run_case(&mut e, g, cur, lim, rate, &format!("fade fuzz[{i}]"));
    }
}

/// `0x68fae0` from `0x68fb9d` to the `test` at `0x68fbc3`, in a fresh emulator each case: the stored
/// sum and AH.
#[test]
fn decal_age_fade_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let run_case = |g: f32, cur: f32, lim: f32, rate: f32, ctx: &str| {
        let mut e = Emu::new(&wow);
        let node = e.alloc(0x80);
        e.write_u32(DECAL_DT, g.to_bits());
        e.write_u32(node + 0x18, cur.to_bits());
        e.write_u32(node + 0x1c, lim.to_bits());
        e.write_u32(node + 0x20, rate.to_bits());
        e.set_reg(RegisterX86::EBX, node);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(0x68fb9d, 0x68fbc3);
        let ah = read_ah(&e);
        let bin_no_clamp = (ah & 0x41) != 0;
        let cur2_live = f64::from(g) * f64::from(rate) + f64::from(cur);
        let our_clamp = cur2_live > f64::from(lim);
        assert_eq!(
            bin_no_clamp, !our_clamp,
            "{ctx}: gate AH={ah:#04x} binary-no-clamp={bin_no_clamp} ours-clamp={our_clamp} \
             (cur'={cur2_live} lim={lim})"
        );
    };
    run_case(
        1.0,
        0.0,
        1.0,
        1.0,
        "gate cur'==lim (no clamp, the boundary)",
    );
    run_case(1.0, 0.0, 0.5, 1.0, "gate cur'>lim (clamp)");
    run_case(0.5, 0.25, 10.0, 1.0, "gate cur'<lim (no clamp)");
    let mut s = 0x68fb_c3c3u32;
    for i in 0..400u32 {
        let g = frand(&mut s, 0.0, 3.0);
        let cur = frand(&mut s, -5.0, 5.0);
        let lim = frand(&mut s, -5.0, 5.0);
        let mut rate = frand(&mut s, -3.0, 3.0);
        if rate == 0.0 {
            rate = 1.0;
        }
        run_case(g, cur, lim, rate, &format!("gate fuzz[{i}]"));
    }
}

/// `0x68fae0` from `0x68fb9d` to `0x68fbfa` with a rate of 0.0: `[ebx+0x18]`.
#[test]
fn decal_age_step_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let node = e.alloc(0x80);
    let run_case = |e: &mut Emu, g: f32, step: f32, cur: f32, ctx: &str| {
        e.write_u32(DECAL_DT, g.to_bits());
        e.write_u32(node + 0x20, 0u32);
        e.write_u32(node + 0x24, step.to_bits());
        e.write_u32(node + 0x18, cur.to_bits());
        e.set_reg(RegisterX86::EBX, node);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(0x68fb9d, 0x68fbf6);
        assert_f32(
            e.read_f32(node + 0x18),
            decal_age_step(f64::from(g), step, cur),
            ctx,
        );
    };
    run_case(&mut e, 0.0, 5.0, 3.0, "agestep dt=0");
    run_case(&mut e, 1.0, 2.0, 1.0, "agestep dt=1");
    let mut s = 0x68fb_eded_u32;
    for i in 0..2000u32 {
        let g = frand(&mut s, -2.0, 2.0);
        let step = frand(&mut s, -1000.0, 1000.0);
        let cur = frand(&mut s, -1000.0, 1000.0);
        run_case(&mut e, g, step, cur, &format!("agestep fuzz[{i}]"));
    }
}

/// `0x68fae0` from `0x68fd00` to `0x68fd0f`: `[esp]`.
#[test]
fn decal_jitter_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let run_case = |e: &mut Emu, v: f32, ctx: &str| {
        e.write_u32(ebp - 0x4, v.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x68fd00, 0x68fd0f);
        assert_f32(e.read_f32(SLICE_ESP - 4), decal_jitter(v), ctx);
    };
    run_case(&mut e, 0.0, "jitter 0");
    run_case(&mut e, 1.0, "jitter 1");
    run_case(&mut e, -1024.0, "jitter -1024");
    let mut s = 0x68fd_0000u32;
    for i in 0..2000u32 {
        let v = frand(&mut s, -5000.0, 5000.0);
        run_case(&mut e, v, &format!("jitter fuzz[{i}]"));
    }
}

/// `0x68fae0` from `0x68fd14` to `0x68fd51`: `[ebp-0x24]` up.
#[test]
fn decal_corner_ray_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // the slice writes down to `[ebp-0x190]`
    let frame = e.alloc(0x400);
    let ebp = frame + 0x200;
    let run_case = |e: &mut Emu, g: [f32; 3], ctx: &str| {
        e.write_u32(CORNER_X, g[0].to_bits());
        e.write_u32(CORNER_Y, g[1].to_bits());
        e.write_u32(CORNER_Z, g[2].to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x68fd14, 0x68fd54);
        let bin = [
            e.read_f32(ebp - 0x24),
            e.read_f32(ebp - 0x20),
            e.read_f32(ebp - 0x1c),
        ];
        for k in 0..3 {
            assert_f32(bin[k], corner_ray_negate(g[k]), &format!("{ctx} neg[{k}]"));
        }
    };
    run_case(&mut e, [0.0, 0.0, 0.0], "corner zero");
    run_case(&mut e, [1.0, -1.0, 1.0], "corner ±1");
    let mut s = 0x68fd_1414u32;
    for i in 0..2000u32 {
        let g = [
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
        ];
        run_case(&mut e, g, &format!("corner fuzz[{i}]"));
    }
}

/// `0x68fae0` from `0x69009a` to the call at `0x6900cb`, ebx the ripple: `[esp]` up.
#[test]
fn decal_pos_plus_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let node = e.alloc(0x80);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let run_case = |e: &mut Emu, n0: f32, n4: f32, n8: f32, n10: f32, ctx: &str| {
        e.write_u32(node, n0.to_bits());
        e.write_u32(node + 4, n4.to_bits());
        e.write_u32(node + 8, n8.to_bits());
        e.write_u32(node + 0x10, n10.to_bits());
        e.set_reg(RegisterX86::EBX, node);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x69009a, 0x6900cb);
        let base = SLICE_ESP - 0xc;
        let bin = [e.read_f32(base + 8), e.read_f32(base + 4), e.read_f32(base)];
        let ours = decal_pos_plus(n8, n10, n4, n0);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} plus[{k}]"));
        }
    };
    run_case(&mut e, 1.0, 2.0, 3.0, 4.0, "plus exact");
    run_case(&mut e, 0.0, 0.0, 0.0, 0.0, "plus zero");
    let mut s = 0x6900_9a9au32;
    for i in 0..2000u32 {
        let n0 = frand(&mut s, -500.0, 500.0);
        let n4 = frand(&mut s, -500.0, 500.0);
        let n8 = frand(&mut s, -500.0, 500.0);
        let n10 = frand(&mut s, -500.0, 500.0);
        run_case(&mut e, n0, n4, n8, n10, &format!("plus fuzz[{i}]"));
    }
}

/// `0x68fae0` from `0x6900d0` to the call at `0x6900f5`, the copies of `[ebx+0x10]` at `[ebp-0x24]`
/// and `[ebp-0x20]`: `[esp]` up.
#[test]
fn decal_pos_minus_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let node = e.alloc(0x80);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let run_case = |e: &mut Emu, n0: f32, n4: f32, n8: f32, n10: f32, ctx: &str| {
        e.write_u32(node, n0.to_bits());
        e.write_u32(node + 4, n4.to_bits());
        e.write_u32(node + 8, n8.to_bits());
        e.write_u32(node + 0x10, n10.to_bits());
        // the copies of `[ebx+0x10]` the stretch before makes
        e.write_u32(ebp - 0x20, n10.to_bits());
        e.write_u32(ebp - 0x24, n10.to_bits());
        e.set_reg(RegisterX86::EBX, node);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6900d0, 0x6900f5);
        let base = SLICE_ESP - 0xc;
        let bin = [e.read_f32(base + 8), e.read_f32(base + 4), e.read_f32(base)];
        let ours = decal_pos_minus(n8, n10, n4, n0);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} minus[{k}]"));
        }
    };
    run_case(&mut e, 1.0, 2.0, 3.0, 4.0, "minus exact");
    run_case(&mut e, 0.0, 0.0, 0.0, 0.0, "minus zero");
    let mut s = 0x6900_d0d0u32;
    for i in 0..2000u32 {
        let n0 = frand(&mut s, -500.0, 500.0);
        let n4 = frand(&mut s, -500.0, 500.0);
        let n8 = frand(&mut s, -500.0, 500.0);
        let n10 = frand(&mut s, -500.0, 500.0);
        run_case(&mut e, n0, n4, n8, n10, &format!("minus fuzz[{i}]"));
    }
}

/// `0x68fae0` from `0x690150` to `0x690162`: `[ebp-0xc]`, and the packed colour from it.
#[test]
fn decal_color_pack_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let node = e.alloc(0x80);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let run_case = |e: &mut Emu, alpha: f32, ctx: &str| {
        e.write_u32(node + 0x18, alpha.to_bits());
        e.set_reg(RegisterX86::EBX, node);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x690150, 0x690162);
        let bin_pack = e.read_f32(ebp - 0xc);
        let bits = bin_pack.to_bits();
        let bin_packed = (((bits >> 0xe) & 0xff) << 0x18) | 0x00ff_ffff;
        assert_eq!(
            bin_packed,
            decal_color_pack(alpha),
            "{ctx}: binary={bin_packed:#010x} ours={:#010x} (pack_f32={bin_pack})",
            decal_color_pack(alpha)
        );
    };
    // 0.0, 1.0 and the byte edges
    run_case(&mut e, 0.0, "pack a=0");
    run_case(&mut e, 1.0, "pack a=1");
    run_case(&mut e, 0.5, "pack a=0.5");
    let mut s = 0x6901_5050u32;
    for i in 0..3000u32 {
        let alpha = frand(&mut s, -1.0, 2.0);
        run_case(&mut e, alpha, &format!("pack fuzz[{i}] a={alpha}"));
    }
}

/// The constants equal the client's.
#[test]
fn liquid_constants_match_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let e = Emu::new(&wow);
    assert_eq!(e.read_u32(0x81028c), RIVER_SLOPE.to_bits(), "0x81028c 1/9");
    assert_eq!(e.read_u32(0x810380), RIVER_SCALE.to_bits(), "0x810380 3/14");
    assert_eq!(e.read_u32(0x810384), RIVER_KNEE.to_bits(), "0x810384 knee");
    assert_eq!(e.read_u32(0x81037c), GLARE_STEP.to_bits(), "0x81037c 1/63");
    assert_eq!(e.read_u32(0x810358), GLARE_COEFF.to_bits(), "0x810358 1.6");
    assert_eq!(
        e.read_u32(0x810390),
        DECAL_JITTER.to_bits(),
        "0x810390 jitter"
    );
    for (k, rate) in RATE_TABLE.iter().enumerate() {
        assert_eq!(
            e.read_u32(0x810348 + 4 * k as u64),
            rate.to_bits(),
            "0x810348 rate[{k}]"
        );
    }
}
