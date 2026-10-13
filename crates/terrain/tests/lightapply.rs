//! The map lighting against `0x6a7300`, `0x69e770` and `0x6b2b80`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::lightapply::{
    area_ambient_color, area_directional_color_scaled, color_step_magnitude,
    daynight_dir_blend_add, daynight_dir_diff_scaled, daynight_dir_normalize, detail_fade_scale,
    detail_rot_y, detail_translate, exterior_scaled_directional, fog_override_color,
    intensity_ramp, IntensityRamp,
};
use common::{assert_f32, next};

/// A frame inside the harness stack (`0x70000000`, 0x100000 bytes); each slice runs with esp 0x40
/// below it.
const SLICE_EBP: u64 = 0x7008_0000;

/// The object `0x6d48b0` returns.
const DAYNIGHT: u64 = 0xce9b60;

/// The pointer to the light block `0x6a7300` reads outdoors.
const BLEND_BLOCK_PTR: u64 = 0xc89ef0;

/// The frame time `0x69e770` reads.
const DT_GLOBAL: u64 = 0xc62510;

/// A float in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any float, NaN and the infinities included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// A byte from the high bits.
fn brand(s: &mut u32) -> u8 {
    ((next(s) >> 8) & 0xff) as u8
}

/// `ApplyMapLighting` (`0x6a7300`) outdoors, `0x6a7349` to `0x6a738d`, esi = this.
#[test]
fn apply_map_exterior_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let block = e.alloc(0x200);
    e.write_u32(BLEND_BLOCK_PTR, block as u32);
    let ebp = SLICE_EBP;
    let mut s = 0x6a73_4900u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, intensity: f32, comps: [f32; 3], label: &str| {
        e.write_u32(this + 0xa4, intensity.to_bits());
        e.write_u32(block + 0xb8, comps[0].to_bits());
        e.write_u32(block + 0xbc, comps[1].to_bits());
        e.write_u32(block + 0xc0, comps[2].to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x6a7349, 0x6a738d);
        // after `sub esp,0xc` the products are at `[esp]` up
        let bin = [
            e.read_f32(ebp - 0x4c), // +0xb8·i
            e.read_f32(ebp - 0x48), // +0xbc·i
            e.read_f32(ebp - 0x44), // +0xc0·i
        ];
        let ours = exterior_scaled_directional(intensity, comps);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{label} scaled[{k}]"));
        }
    };

    // 1.375 · 2.6624860763549805 is halfway between two floats, and the store takes the even one
    case(
        &mut e,
        1.375_f32,
        [f32::from_bits(0x402a_662c), 1.0, 2.0],
        "halfway",
    );
    case(&mut e, 1.0, [1.0, 2.0, 3.0], "identity ×1");
    case(&mut e, 0.0, [5.0, 6.0, 7.0], "×0");
    case(&mut e, -2.5, [4.0, -8.0, 0.5], "negative intensity");
    for i in 0..2000u32 {
        let intensity = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -4.0, 4.0)
        };
        let comps = if i % 7 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -10.0, 10.0),
                frand(&mut s, -10.0, 10.0),
                frand(&mut s, -10.0, 10.0),
            ]
        };
        case(&mut e, intensity, comps, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "apply_map_exterior: {checked} cases bit-exact vs WoW.exe @ 0x6a7300 slice [0x6a7349,0x6a738d)"
    );
}

/// `0x6a7300` from `0x6a73bd` to `0x6a743f`: the directional bytes times the intensity, at
/// `[ebp-0x1c]` up.
#[test]
fn apply_map_b1_dir_scaled_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let ebp = SLICE_EBP;
    let mut s = 0x6a73_bd00u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, bytes: [u8; 3], intensity: f32, label: &str| {
        e.write(this + 0xa0, &[bytes[2], bytes[1], bytes[0]]);
        e.write_u32(this + 0xa4, intensity.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x6a73bd, 0x6a743f);
        let bin = [
            e.read_f32(ebp - 0x1c), // +0xa2·i
            e.read_f32(ebp - 0x18), // +0xa1·i
            e.read_f32(ebp - 0x14), // +0xa0·i
        ];
        let ours = area_directional_color_scaled(bytes, intensity);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{label} out[{k}]"));
        }
    };

    case(&mut e, [0, 0, 0], 1.0, "zero bytes");
    case(&mut e, [255, 255, 255], 1.0, "max bytes ×1");
    // an intensity one step above 1.0
    case(
        &mut e,
        [128, 1, 254],
        f32::from_bits(0x3f80_0001),
        "near 1.0",
    );
    case(&mut e, [200, 50, 10], 0.0, "×0");
    case(&mut e, [10, 20, 30], -3.5, "negative intensity");
    for i in 0..2000u32 {
        let bytes = [brand(&mut s), brand(&mut s), brand(&mut s)];
        let intensity = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -8.0, 8.0)
        };
        case(&mut e, bytes, intensity, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "apply_map_b1_dir_scaled: {checked} cases bit-exact vs WoW.exe @ 0x6a7300 slice [0x6a73bd,0x6a743f)"
    );
}

/// `0x6a7300`: the day light's direction blended in, with `0x6d48b0` and `0x4549a0` stubbed and
/// the globals written to `[ebp-0x10]` up.
#[test]
fn apply_map_b2_daynight_blend_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let ebp = SLICE_EBP;
    e.stub(0x6d48b0, DAYNIGHT as u32, 0);
    e.stub(0x4549a0, 0, 3);
    let mut s = 0x6a74_5400u32;
    let mut checked = 0u32;

    // `0x6a7454` to the call at `0x6a74a3`
    let case_a = |e: &mut Emu, dn: [f32; 3], global: [f32; 3], a3: u8, label: &str| {
        e.write_u32(DAYNIGHT + 0x168, dn[0].to_bits());
        e.write_u32(DAYNIGHT + 0x16c, dn[1].to_bits());
        e.write_u32(DAYNIGHT + 0x170, dn[2].to_bits());
        e.write(this + 0xa3, &[a3]);
        e.write_u32(ebp - 0x10, global[0].to_bits());
        e.write_u32(ebp - 0xc, global[1].to_bits());
        e.write_u32(ebp - 0x8, global[2].to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.clear_calls();
        e.run(0x6a7454, 0x6a74a3);
        // the arguments of the call to `0x4549a0`, `[esp]` up after `sub esp,0xc`
        let bin = [
            e.read_f32(ebp - 0x4c),
            e.read_f32(ebp - 0x48),
            e.read_f32(ebp - 0x44),
        ];
        let ours = daynight_dir_diff_scaled(dn, global, a3);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{label} B2a[{k}]"));
        }
    };
    case_a(&mut e, [1.0, 2.0, 3.0], [0.5, 1.0, 1.5], 255, "max alpha");
    case_a(&mut e, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0, "zero");
    case_a(
        &mut e,
        [f32::from_bits(0x3f80_0001), -4.0, 8.0],
        [1.0, -2.0, 3.0],
        128,
        "near 1.0",
    );
    for i in 0..1500u32 {
        let dn = [
            frand(&mut s, -20.0, 20.0),
            frand(&mut s, -20.0, 20.0),
            frand(&mut s, -20.0, 20.0),
        ];
        let global = [
            frand(&mut s, -20.0, 20.0),
            frand(&mut s, -20.0, 20.0),
            frand(&mut s, -20.0, 20.0),
        ];
        case_a(&mut e, dn, global, brand(&mut s), &format!("fuzz{i}"));
        checked += 1;
    }

    // `0x6a74a8` to the call at `0x6a74cb`
    let case_b = |e: &mut Emu, diff_scaled: [f32; 3], global: [f32; 3], label: &str| {
        e.write_u32(ebp - 0x34, diff_scaled[0].to_bits());
        e.write_u32(ebp - 0x30, diff_scaled[1].to_bits());
        e.write_u32(ebp - 0x2c, diff_scaled[2].to_bits());
        e.write_u32(ebp - 0x10, global[0].to_bits());
        e.write_u32(ebp - 0xc, global[1].to_bits());
        e.write_u32(ebp - 0x8, global[2].to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.clear_calls();
        e.run(0x6a74a8, 0x6a74cb);
        let bin = [
            e.read_f32(ebp - 0x4c),
            e.read_f32(ebp - 0x48),
            e.read_f32(ebp - 0x44),
        ];
        let ours = daynight_dir_blend_add(diff_scaled, global);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{label} B2b[{k}]"));
        }
    };
    case_b(&mut e, [1.0, 2.0, 3.0], [0.5, 1.5, 2.5], "simple");
    case_b(
        &mut e,
        [f32::from_bits(0x3f80_0001), -1.0, 0.0],
        [f32::from_bits(0xb400_0000), 1.0, 0.0],
        "near cancelling",
    );
    for i in 0..1500u32 {
        let ds = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        let g = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        case_b(&mut e, ds, g, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "apply_map_b2_daynight_blend: {checked} cases + probes bit-exact vs WoW.exe @ 0x6a7300 \
         slices [0x6a7454,0x6a74a3) + [0x6a74a8,0x6a74cb)"
    );
}

/// `0x6a7300` from `0x6a74d0` to `0x6a751a`: the direction at `[ebp-0x1c]` up normalized to
/// `[ebp-0x10]` up.
#[test]
fn apply_map_b3_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp = SLICE_EBP;
    let mut s = 0x6a74_d000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, v: [f32; 3], label: &str| {
        e.write_u32(ebp - 0x1c, v[0].to_bits());
        e.write_u32(ebp - 0x18, v[1].to_bits());
        e.write_u32(ebp - 0x14, v[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x6a74d0, 0x6a751a);
        let bin = [
            e.read_f32(ebp - 0x10),
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
        ];
        let ours = daynight_dir_normalize(v);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{label} n[{k}]"));
        }
    };

    case(&mut e, [1.0, 0.0, 0.0], "unit-x");
    case(&mut e, [3.0, 4.0, 0.0], "3-4-5");
    case(&mut e, [1.0, 1.0, 1.0], "diagonal");
    // a length of 2, whose reciprocal is exact
    case(&mut e, [2.0, 0.0, 0.0], "length 2");
    for i in 0..2500u32 {
        let v = if i % 6 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -30.0, 30.0),
                frand(&mut s, -30.0, 30.0),
                frand(&mut s, -30.0, 30.0),
            ]
        };
        case(&mut e, v, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "apply_map_b3_normalize: {checked} cases bit-exact vs WoW.exe @ 0x6a7300 slice [0x6a74d0,0x6a751a)"
    );
}

/// `0x6a7300`: the fog and ambient colours.
#[test]
fn apply_map_b4_b5_colors_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let fogsrc = e.alloc(0x200);
    let ebp = SLICE_EBP;
    let mut s = 0x6a75_a600u32;
    let mut checked = 0u32;

    // `0x6a754b` to the call at `0x6a75a1`, ecx = the fog's source
    let case_b4 = |e: &mut Emu, bytes: [u8; 3], label: &str| {
        e.write(fogsrc + 0x80, &[bytes[2], bytes[1], bytes[0]]);
        e.write_u32(fogsrc + 0x84, 0);
        e.write_u32(fogsrc + 0x88, 0);
        e.set_reg(RegisterX86::ECX, fogsrc);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x6a754b, 0x6a75a1);
        let bin = [
            e.read_f32(ebp - 0x34),
            e.read_f32(ebp - 0x30),
            e.read_f32(ebp - 0x2c),
        ];
        let ours = fog_override_color(bytes);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{label} fog[{k}]"));
        }
    };
    case_b4(&mut e, [0, 0, 0], "fog zero");
    case_b4(&mut e, [255, 255, 255], "fog max");
    case_b4(&mut e, [7, 200, 64], "fog spread");

    // `0x6a75a6` to the call at `0x6a75ee`, esi = this
    let case_b5 = |e: &mut Emu, bytes: [u8; 3], label: &str| {
        e.write(this + 0x9c, &[bytes[2], bytes[1], bytes[0]]);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x6a75a6, 0x6a75ee);
        let bin = [
            e.read_f32(ebp - 0x34),
            e.read_f32(ebp - 0x30),
            e.read_f32(ebp - 0x2c),
        ];
        let ours = area_ambient_color(bytes);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{label} amb[{k}]"));
        }
    };
    case_b5(&mut e, [0, 0, 0], "amb zero");
    case_b5(&mut e, [255, 255, 255], "amb max");
    case_b5(&mut e, [1, 128, 254], "amb spread");

    for i in 0..2000u32 {
        let bytes = [brand(&mut s), brand(&mut s), brand(&mut s)];
        case_b4(&mut e, bytes, &format!("b4 fuzz{i}"));
        let bytes = [brand(&mut s), brand(&mut s), brand(&mut s)];
        case_b5(&mut e, bytes, &format!("b5 fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "apply_map_b4_b5_colors: {checked} cases bit-exact vs WoW.exe @ 0x6a7300 \
         slices [0x6a754b,0x6a75a1) + [0x6a75a6,0x6a75ee)"
    );
}

/// `AnimateMapEntityLight` (`0x69e770`): the colour step from `[0xc62510]`.
#[test]
fn color_step_magnitude_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp = SLICE_EBP;
    let mut s = 0x69e7_7600u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, dt: f32, label: &str| {
        e.write_u32(DT_GLOBAL, dt.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x69e776, 0x69e79a); // through `__ftol` and the clamp
        let bin = e.get_reg(RegisterX86::EAX) as u32 as i32;
        let ours = color_step_magnitude(dt);
        assert_eq!(ours, bin, "{label}: dt={dt} ours={ours} binary={bin}");
    };

    // a product of 1.0, and one just below it that truncates to 0
    case(&mut e, 1.0 / 510.0, "product 1.0");
    case(&mut e, 0.999_f32 / 510.0, "just below 1.0");
    case(&mut e, 0.0, "dt 0");
    case(&mut e, 0.016_f32, "dt 0.016");
    case(&mut e, 0.1_f32, "dt 0.1");
    case(&mut e, -0.05_f32, "negative dt");
    for i in 0..3000u32 {
        let dt = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -1.0, 2.0)
        };
        case(&mut e, dt, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "color_step_magnitude: {checked} cases + probes bit-exact vs WoW.exe @ 0x69e770 from 0x69e776 to 0x69e79a"
    );
}

/// `0x69e770` from `0x69e880` to `0x69e906`, esi = this: the intensity stored to `[esi+0xa4]`.
#[test]
fn intensity_ramp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let ebp = SLICE_EBP;
    let mut s = 0x69e8_8000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu,
                i: f32,
                t: f32,
                dt: f32,
                label: &str,
                track: &mut (bool, bool, bool, bool)| {
        e.write_u32(this + 0xa4, i.to_bits());
        e.write_u32(this + 0xf8, t.to_bits());
        e.write_u32(DT_GLOBAL, dt.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x69e880, 0x69e906);
        let bin = e.read_f32(this + 0xa4);
        let (disp, ours) = intensity_ramp(i, t, dt);
        assert_f32(bin, ours, &format!("{label} [disp={disp:?}]"));
        match disp {
            IntensityRamp::Up => track.0 = true,
            IntensityRamp::Down => track.1 = true,
            IntensityRamp::Hold => track.2 = true,
        }
        // a clamp: the target stored, from elsewhere
        if ours.to_bits() == t.to_bits() && i.to_bits() != t.to_bits() {
            track.3 = true;
        }
    };
    let mut track = (false, false, false, false);

    // at the target already
    case(&mut e, 0.5, 0.5, 0.016, "at target (hold)", &mut track);
    case(
        &mut e,
        0.0,
        1.0,
        10.0,
        "ramp up overshoot → clamp t",
        &mut track,
    );
    case(&mut e, 0.0, 1.0, 0.001, "ramp up undershoot", &mut track);
    case(
        &mut e,
        1.0,
        0.0,
        10.0,
        "ramp down overshoot → clamp t",
        &mut track,
    );
    case(&mut e, 1.0, 0.0, 0.001, "ramp down undershoot", &mut track);
    // a rise that ends near the target
    case(&mut e, 0.0, 1.0, 0.3, "rise near target", &mut track);

    for i in 0..4000u32 {
        let (iv, tv, dtv) = if i % 7 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (
                frand(&mut s, -5.0, 5.0),
                frand(&mut s, -5.0, 5.0),
                frand(&mut s, -1.0, 2.0),
            )
        };
        case(&mut e, iv, tv, dtv, &format!("fuzz{i}"), &mut track);
        checked += 1;
    }
    let (seen_up, seen_down, seen_hold, seen_clamp) = track;
    assert!(seen_up, "UP branch never exercised");
    assert!(seen_down, "DOWN branch never exercised");
    assert!(seen_hold, "HOLD branch never exercised");
    assert!(seen_clamp, "clamp-to-target never exercised");
    eprintln!(
        "intensity_ramp: {checked} cases + probes bit-exact vs WoW.exe @ 0x69e770 slice [0x69e880,0x69e906) \
         (up/down/hold/clamp all hit)"
    );
}

/// `DrawDetailDoodadBatch` (`0x6b2b80`): the three values it passes to the matrix calls.
#[test]
fn detail_doodad_texgen_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp = SLICE_EBP;

    // the scale, stored to `[esp]` at `0x6b2caa` after one push
    e.set_reg(RegisterX86::EBP, ebp);
    e.set_reg(RegisterX86::ESP, ebp - 0x60);
    e.run(0x6b2c24, 0x6b2cad);
    let bin_scale = e.read_f32(ebp - 0x64);
    let our_scale = detail_fade_scale();
    assert_f32(bin_scale, our_scale, "detail scale 1/(70·0.25)");

    // the angle, stored to `[esp]` at `0x6b2cc8` after three pushes
    e.set_reg(RegisterX86::EBP, ebp);
    e.set_reg(RegisterX86::ESP, ebp - 0x60);
    e.run(0x6b2cb2, 0x6b2ccb);
    let bin_rot = e.read_f32(ebp - 0x6c);
    let our_rot = detail_rot_y();
    assert_f32(bin_rot, our_rot, "detail rotY π/2");

    // the offset, stored to `[ebp-0x4]`
    e.set_reg(RegisterX86::EBP, ebp);
    e.set_reg(RegisterX86::ESP, ebp - 0x60);
    e.run(0x6b2ce5, 0x6b2cfb);
    let bin_tr = e.read_f32(ebp - 0x4);
    let our_tr = detail_translate();
    assert_f32(bin_tr, our_tr, "detail translate -52.5");

    eprintln!(
        "detail_doodad_texgen: scale/rotY/translate bit-exact vs WoW.exe @ 0x6b2b80 \
         (1/(70·0.25)={bin_scale}, π/2={bin_rot}, -52.5={bin_tr})"
    );
}
