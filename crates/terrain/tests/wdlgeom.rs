//! The low-detail tile, the group lights and the camera height against `0x6944a0`, `0x695c00` and `0x6809c0`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::wdlgeom::{
    camera_probe_z_delta, detail_color_scale, detail_slope_bumped, detail_slope_recip,
    wdl_tile_geometry, SLOPE_EPS,
};
use common::{assert_f32, next};

/// A frame inside the harness stack (`0x70000000`, 0x100000 bytes).
const SLICE_ESP: u64 = 0x7008_0000;

/// A float in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// AH after `fnstsw ax`.
fn read_ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// Asserts bytes copied out of the client still equal the client's.
fn island_bytes_match(e: &Emu, island: u64, src: u64, len: usize) {
    let got = e.read(island, len);
    let want = e.read(src, len);
    assert_eq!(
        got, want,
        "island bytes at {island:#x} != WoW.exe[{src:#x}..] (len {len})"
    );
}

const PROBE_OUT: u64 = 0xc7bc60;

/// `CMap::LoadWdl` (`0x6944a0`) from `0x694648` to `0x69471b`, copied out after a `fld dword` of the
/// highest height (in the register there): esi the tile, the indices at `[ebp-0x34]` and `[ebp-0x3c]`,
/// the lowest height at `[ebp-0xc]`.
#[test]
fn wdl_tile_geometry_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    const SRC: u64 = 0x694648;
    const LEN: usize = 0x69471b - 0x694648;
    let body = e.read(SRC, LEN);
    // first instruction must be `fild qword [ebp-0x34]` (df 6d cc); last the radius store (d9 5e 24).
    assert_eq!(
        &body[0..3],
        &[0xdf, 0x6d, 0xcc],
        "0x694648 fild qword drifted"
    );
    assert_eq!(
        &body[LEN - 3..],
        &[0xd9, 0x5e, 0x24],
        "0x694718 fstp [esi+0x24] drifted"
    );

    let maxz_in = e.alloc(0x10);
    let isl = e.alloc(0x200);
    let this = e.alloc(0x100);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    // `fld dword [maxz]`, then the client's bytes
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(maxz_in as u32).to_le_bytes());
    let prefix = code.len();
    code.extend_from_slice(&body);
    e.write(isl, &code);
    island_bytes_match(&e, isl + prefix as u64, SRC, LEN);
    let end = isl + code.len() as u64;

    let run_case = |e: &mut Emu, ix: i32, iy: i32, min_z: f32, max_z: f32, ctx: &str| {
        e.write_u32(maxz_in, max_z.to_bits());
        // the row's qword at `[ebp-0x34]` is written before the stretch, the column's inside it from
        // edx and edi
        e.write_u32(ebp - 0x34, iy as u32);
        e.write_u32(ebp - 0x30, 0);
        e.write_u32(ebp - 0xc, min_z.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDX, ix as u32 as u64);
        e.set_reg(RegisterX86::EDI, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let ours = wdl_tile_geometry(ix, iy, min_z, max_z);
        assert_f32(e.read_f32(this), ours.min_y, &format!("{ctx} min_y[esi+0]"));
        assert_f32(
            e.read_f32(this + 0x4),
            ours.min_x,
            &format!("{ctx} min_x[esi+4]"),
        );
        assert_f32(
            e.read_f32(this + 0xc),
            ours.origin_y,
            &format!("{ctx} origin_y[esi+0xc]"),
        );
        assert_f32(
            e.read_f32(this + 0x10),
            ours.origin_x,
            &format!("{ctx} origin_x[esi+0x10]"),
        );
        assert_f32(
            e.read_f32(this + 0x14),
            ours.max_z,
            &format!("{ctx} max_z[esi+0x14]"),
        );
        assert_f32(
            e.read_f32(this + 0x18),
            ours.center[0],
            &format!("{ctx} center0[esi+0x18]"),
        );
        assert_f32(
            e.read_f32(this + 0x1c),
            ours.center[1],
            &format!("{ctx} center1[esi+0x1c]"),
        );
        assert_f32(
            e.read_f32(this + 0x20),
            ours.center[2],
            &format!("{ctx} center2[esi+0x20]"),
        );
        assert_f32(
            e.read_f32(this + 0x24),
            ours.radius,
            &format!("{ctx} radius[esi+0x24]"),
        );
        // and the copies at +0x28 and +0x2c
        assert_f32(
            e.read_f32(this + 0x28),
            ours.origin_y,
            &format!("{ctx} origin_y[esi+0x28]"),
        );
        assert_f32(
            e.read_f32(this + 0x2c),
            ours.origin_x,
            &format!("{ctx} origin_x[esi+0x2c]"),
        );
    };

    // the corner tiles, equal heights, and a 3-4-5 extent
    run_case(&mut e, 0, 0, 0.0, 0.0, "tile (0,0) zZ");
    run_case(&mut e, 0x3f0, 0x3f0, -100.0, 100.0, "tile far corner");
    run_case(&mut e, 0x200, 0x100, 50.0, 50.0, "tile mid equal-Z");
    run_case(
        &mut e,
        0x10,
        0x20,
        -32768.0,
        32767.0,
        "tile int16 Z extremes",
    );

    let mut s = 0x6944_a0a0u32;
    for i in 0..3000u32 {
        // indices in steps of 0x10, heights whole numbers
        let ix = ((next(&mut s) >> 8) % 0x40) as i32 * 0x10;
        let iy = ((next(&mut s) >> 8) % 0x40) as i32 * 0x10;
        let zlo = (((next(&mut s) >> 8) % 65535) as i32 - 32768) as f32;
        let zhi = (((next(&mut s) >> 8) % 65535) as i32 - 32768) as f32;
        let (min_z, max_z) = (zlo.min(zhi), zlo.max(zhi));
        run_case(&mut e, ix, iy, min_z, max_z, &format!("tile fuzz[{i}]"));
    }
}

/// `CreateGroupLights` (`0x695c00`) from `0x695c93` to the call at `0x695cea`: `[edi+0xf0]`, and AH
/// against `detail_slope_bumped` (`jp` skips the raise when PF is set).
#[test]
fn detail_slope_recip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x80);
    let out = e.alloc(0x200);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, a: f32, b: f32, ctx: &str| {
        e.write_u32(this + 0x28, a.to_bits());
        e.write_u32(this + 0x2c, b.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, out);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x695c93, 0x695cea);
        let ah = read_ah(e);
        let bin = e.read_f32(out + 0xf0);
        let ours = detail_slope_recip(a, b);
        assert_f32(bin, ours, ctx);
        let bin_skip_bump = (ah & 0x5).count_ones().is_multiple_of(2);
        assert_eq!(
            !bin_skip_bump,
            detail_slope_bumped(a, b),
            "{ctx}: binary bumps={} value-model |d|<eps={} (ah={ah:#04x})",
            !bin_skip_bump,
            detail_slope_bumped(a, b)
        );
    };

    let eps = f32::from_bits(SLOPE_EPS.to_bits());
    run_case(&mut e, 1.0, 1.0, "slope A==B (|d|=0 < eps → bump)");
    run_case(&mut e, 1.0 + eps, 1.0, "slope |d|==eps (boundary)");
    run_case(
        &mut e,
        1.0 + f32::from_bits(eps.to_bits() - 1),
        1.0,
        "slope |d| just-below-eps (bump)",
    );
    run_case(
        &mut e,
        1.0 + f32::from_bits(eps.to_bits() + 1),
        1.0,
        "slope |d| just-above-eps",
    );
    run_case(&mut e, 5.0, 2.0, "slope |d|=3 (no bump)");
    run_case(&mut e, 2.0, 5.0, "slope |d|=3 negative (no bump)");

    let mut s = 0x695c_9393u32;
    for i in 0..3000u32 {
        let (a, b) = if (next(&mut s) >> 8).is_multiple_of(4) {
            let base = frand(&mut s, -50.0, 50.0);
            (base + frand(&mut s, -0.003, 0.003), base)
        } else {
            (frand(&mut s, -500.0, 500.0), frand(&mut s, -500.0, 500.0))
        };
        run_case(&mut e, a, b, &format!("slope fuzz[{i}]"));
    }
}

/// `0x695c00` from `0x695d1f` to `0x695d87`: esi the light, `ebx+0x3c` the output.
#[test]
fn detail_color_scale_g1_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x80);
    let outbuf = e.alloc(0x80);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, r: u8, g: u8, b: u8, scale: f32, ctx: &str| {
        e.write(this + 0x4, &[r, g, b]);
        e.write_u32(this + 0x14, scale.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBX, outbuf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x695d1f, 0x695d87);
        let bin = [
            e.read_f32(outbuf + 0x3c),
            e.read_f32(outbuf + 0x40),
            e.read_f32(outbuf + 0x44),
        ];
        let ours = detail_color_scale(r, g, b, scale);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} out[{k}]"));
        }
    };

    // black, white, grey, and scales outside [0, 1]
    run_case(&mut e, 0, 0, 0, 1.0, "g1 black");
    run_case(&mut e, 255, 255, 255, 1.0, "g1 white");
    run_case(&mut e, 128, 64, 32, 0.5, "g1 mid scale=0.5");
    run_case(&mut e, 200, 100, 50, 3.7, "g1 scale>1");
    let mut s = 0x695d_1f1fu32;
    for i in 0..2500u32 {
        let r = ((next(&mut s) >> 8) & 0xff) as u8;
        let g = ((next(&mut s) >> 8) & 0xff) as u8;
        let b = ((next(&mut s) >> 8) & 0xff) as u8;
        let scale = frand(&mut s, -10.0, 10.0);
        run_case(&mut e, r, g, b, scale, &format!("g1 fuzz[{i}]"));
    }
}

/// `0x695c00` from `0x695d8e` to `0x695df6`, the scale at `[ebp-0xc]`: `ebx+0x48`.
#[test]
fn detail_color_scale_g2_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x80);
    let outbuf = e.alloc(0x80);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, r: u8, g: u8, b: u8, scale: f32, ctx: &str| {
        e.write(this + 0x4, &[r, g, b]);
        e.write_u32(this + 0x14, scale.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBX, outbuf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x695d8e, 0x695df6);
        let bin = [
            e.read_f32(outbuf + 0x48),
            e.read_f32(outbuf + 0x4c),
            e.read_f32(outbuf + 0x50),
        ];
        let ours = detail_color_scale(r, g, b, scale);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} out[{k}]"));
        }
    };

    run_case(&mut e, 0, 0, 0, 1.0, "g2 black");
    run_case(&mut e, 255, 255, 255, 1.0, "g2 white");
    run_case(&mut e, 128, 64, 32, 0.5, "g2 mid scale=0.5");
    run_case(&mut e, 200, 100, 50, 3.7, "g2 scale>1");
    let mut s = 0x695d_8e8eu32;
    for i in 0..2500u32 {
        let r = ((next(&mut s) >> 8) & 0xff) as u8;
        let g = ((next(&mut s) >> 8) & 0xff) as u8;
        let b = ((next(&mut s) >> 8) & 0xff) as u8;
        let scale = frand(&mut s, -10.0, 10.0);
        run_case(&mut e, r, g, b, scale, &format!("g2 fuzz[{i}]"));
    }
}

/// `UpdateCameraEnvironment` (`0x6809c0`) from `0x680a81` to `0x680a8d`: `[0xc7bc60]`.
#[test]
fn camera_probe_z_delta_a_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, a: f32, b: f32, ctx: &str| {
        e.write_u32(ebp - 0x8, a.to_bits());
        e.write_u32(ebp - 0x24, b.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x680a81, 0x680a8d);
        assert_f32(e.read_f32(PROBE_OUT), camera_probe_z_delta(a, b), ctx);
    };

    run_case(&mut e, 0.0, 0.0, "probeA zero");
    run_case(&mut e, 1.0, 0.0, "probeA 1-0");
    run_case(&mut e, -5.0, 3.0, "probeA -5,3");
    let mut s = 0x680a_8181u32;
    for i in 0..2000u32 {
        let a = frand(&mut s, -5000.0, 5000.0);
        let b = frand(&mut s, -5000.0, 5000.0);
        run_case(&mut e, a, b, &format!("probeA fuzz[{i}]"));
    }
}

/// `0x6809c0` from `0x680aaa` to `0x680ab6`: `[0xc7bc60]`.
#[test]
fn camera_probe_z_delta_b_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, a: f32, b: f32, ctx: &str| {
        e.write_u32(ebp - 0x8, a.to_bits());
        e.write_u32(ebp - 0xc, b.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x680aaa, 0x680ab6);
        assert_f32(e.read_f32(PROBE_OUT), camera_probe_z_delta(a, b), ctx);
    };

    run_case(&mut e, 0.0, 0.0, "probeB zero");
    run_case(&mut e, 1.0, 0.0, "probeB 1-0");
    run_case(&mut e, -5.0, 3.0, "probeB -5,3");
    let mut s = 0x680a_aaaau32;
    for i in 0..2000u32 {
        let a = frand(&mut s, -5000.0, 5000.0);
        let b = frand(&mut s, -5000.0, 5000.0);
        run_case(&mut e, a, b, &format!("probeB fuzz[{i}]"));
    }
}
