//! The visibility walk pieces against `0x6834e0`, `0x683700`, `0x683040` and `0x6b3890`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::scenevis::{camplane_sort_key, near_audible};
use benilla_re_terrain::scenewalk::{
    arealow_far_project, camera_negate, near_audibility_diffs, portal_age_fst_live, portal_age_fstp,
};
use common::{assert_f32, next};

/// An ESP in the middle of the harness's stack, so every push of a slice lands in it.
const SLICE_ESP: u64 = 0x7008_0000;

/// The camera plane.
const CAMPLANE_C0: u64 = 0xc7bcb0;

/// The camera.
const CAM_POS: u64 = 0xc7cf20;

/// `[0x81017c]`, the squared distance `0x683700` compares with.
const THR_683700: u64 = 0x81017c;

/// `[0x8101b0]`, the far constant of `0x683040`.
const FAR_CONST_683040: u64 = 0x8101b0;

/// `[0xc62510]`, the frame time `0x6b3890` ages by.
const AGE_DT: u64 = 0xc62510;

/// `[0x7ffd74]`, the age `0x6b3890` expires at.
const AGE_THR: u64 = 0x7ffd74;

/// `test ah,5`: C0 and C2.
const TEST_AH_5: u8 = 0x5;

/// `test ah,0x41`: C0 and C3.
const TEST_AH_41: u8 = 0x41;

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any `f32`, from random bits: subnormals, infinities and NaN included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// The AH byte of EAX after `fnstsw ax` (AH = bits 8..15).
fn ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// A timer, the frame time and the threshold.
#[derive(Clone, Copy)]
struct AgeInputs {
    timer: f32,
    dt: f32,
    thr: f32,
}

/// `BeginMapObjectFrame` (`0x6b3890`) from a store-and-pop site to its `jp`: eax the node, its link
/// at `[ebx+link_off]`. The stored value, AH and PF.
fn run_age_fstp(
    e: &mut Emu,
    node: u64,
    parent: u64,
    link_off: u64,
    inp: AgeInputs,
    slice: (u64, u64),
) -> (f32, u8, u64) {
    e.write_u32(node + 0x8, inp.timer.to_bits());
    e.write_u32(parent + link_off, node as u32);
    e.write_u32(AGE_DT, inp.dt.to_bits());
    e.write_u32(AGE_THR, inp.thr.to_bits());
    e.set_reg(RegisterX86::EAX, node);
    e.set_reg(RegisterX86::EBX, parent);
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.run(slice.0, slice.1);
    let pf = (e.get_reg(RegisterX86::EFLAGS) >> 2) & 1;
    (e.read_f32(node + 0x8), ah(e) & TEST_AH_5, pf)
}

/// `0x6b3890` from a store-and-keep site to its `jp`, `ptr_reg` the object. The stored value, AH and
/// PF.
fn run_age_fst(
    e: &mut Emu,
    obj: u64,
    field_off: u64,
    inp: AgeInputs,
    slice: (u64, u64),
    ptr_reg: RegisterX86,
) -> (f32, u8, u64) {
    e.write_u32(obj + field_off, inp.timer.to_bits());
    e.write_u32(AGE_DT, inp.dt.to_bits());
    e.write_u32(AGE_THR, inp.thr.to_bits());
    e.set_reg(ptr_reg, obj);
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.run(slice.0, slice.1);
    let pf = (e.get_reg(RegisterX86::EFLAGS) >> 2) & 1;
    (e.read_f32(obj + field_off), ah(e) & TEST_AH_41, pf)
}

/// `CullGroupDoodads` (`0x6834e0`) from `0x68354a` and `CullEntityList` (`0x683700`) from `0x68377b`,
/// each to the call after: the key at `+0x78`.
#[test]
fn scenewalk_camplane_sort_key_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x100);
    let mut s = 0x6834_e001u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, c: [f32; 4], v: [f32; 3], radius: f32, ctx: &str| {
        for (i, &ci) in c.iter().enumerate() {
            e.write_u32(CAMPLANE_C0 + i as u64 * 4, ci.to_bits());
        }
        e.write_u32(rec + 0x5c, v[0].to_bits());
        e.write_u32(rec + 0x60, v[1].to_bits());
        e.write_u32(rec + 0x64, v[2].to_bits());
        e.write_u32(rec + 0x68, radius.to_bits());
        let ours = camplane_sort_key(c, v, radius);

        // `0x6834e0`: esi the record
        e.write_u32(rec + 0x78, 0xdead_beef);
        e.set_reg(RegisterX86::ESI, rec);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x68354a, 0x68357d);
        assert_f32(e.read_f32(rec + 0x78), ours, &format!("{ctx} [0x6834e0]"));

        // `0x683700`: ebx the record
        e.write_u32(rec + 0x78, 0xdead_beef);
        e.set_reg(RegisterX86::EBX, rec);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x68377b, 0x6837ae);
        assert_f32(e.read_f32(rec + 0x78), ours, &format!("{ctx} [0x683700]"));
    };

    // a key of exactly 0
    case(&mut e, [1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 1.0, "key 0");
    case(&mut e, [0.0; 4], [0.0; 3], 0.0, "zeros");
    case(
        &mut e,
        [0.5, -0.25, 1.5, -10.0],
        [12.0, -8.0, 3.0],
        7.5,
        "mixed",
    );
    case(
        &mut e,
        [1e20, 1e20, 1e20, 1e20],
        [1e20, 1e20, 1e20],
        1e20,
        "overflow-inf",
    );

    for i in 0..4000u32 {
        let c = if i % 4 == 0 {
            [
                frand_bits(&mut s),
                frand_bits(&mut s),
                frand_bits(&mut s),
                frand_bits(&mut s),
            ]
        } else {
            [
                frand(&mut s, -2.0, 2.0),
                frand(&mut s, -2.0, 2.0),
                frand(&mut s, -2.0, 2.0),
                frand(&mut s, -1e4, 1e4),
            ]
        };
        let v = if i % 5 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
            ]
        };
        let radius = if i % 7 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 1e3)
        };
        case(&mut e, c, v, radius, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "scenewalk_camplane_sort_key: {checked} cases bit-exact vs WoW.exe @ 0x6834e0[0x68354a,0x68357d) + 0x683700[0x68377b,0x6837ae)"
    );
}

/// `0x6834e0` from `0x683681` to the call at `0x6836ac`, ebx the centre: the differences at `[esp]`
/// up after `sub esp,0xc`.
#[test]
fn scenewalk_near_audibility_diffs_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let vec = e.alloc(0x10);
    let mut s = 0x6836_8101u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, v: [f32; 3], cam: [f32; 3], ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(vec + k * 4, v[k as usize].to_bits());
            e.write_u32(CAM_POS + k * 4, cam[k as usize].to_bits());
        }
        e.set_reg(RegisterX86::EBX, vec);
        // ebp only for a `lea`
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x683681, 0x6836ac);
        let esp_after = SLICE_ESP - 0xc;
        let bin = [
            e.read_f32(esp_after),
            e.read_f32(esp_after + 4),
            e.read_f32(esp_after + 8),
        ];
        let ours = near_audibility_diffs(v, cam);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} diff[{k}]"));
        }
    };

    // equal points
    case(
        &mut e,
        [10.0, 20.0, 30.0],
        [10.0, 20.0, 30.0],
        "at the camera",
    );
    case(&mut e, [0.0; 3], [0.0; 3], "zeros");
    case(&mut e, [1.0, -2.0, 3.5], [0.5, 0.5, 0.5], "mixed");

    for i in 0..4000u32 {
        let (v, cam) = if i % 5 == 0 {
            (
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
            )
        } else {
            let cam = [
                frand(&mut s, -500.0, 500.0),
                frand(&mut s, -500.0, 500.0),
                frand(&mut s, -500.0, 500.0),
            ];
            let v = [
                cam[0] + frand(&mut s, -100.0, 100.0),
                cam[1] + frand(&mut s, -100.0, 100.0),
                cam[2] + frand(&mut s, -100.0, 100.0),
            ];
            (v, cam)
        };
        case(&mut e, v, cam, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "scenewalk_near_audibility_diffs: {checked} cases bit-exact vs WoW.exe @ 0x6834e0 slice [0x683681,0x6836ac)"
    );
}

/// `0x683700` from `0x683881` to the `jnp` at `0x6838bc`, esi the centre: 1 is passed when PF is
/// clear.
#[test]
fn scenewalk_near_audibility_jnp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let vec = e.alloc(0x10);
    let mut s = 0x6838_8101u32;
    let mut seen = std::collections::BTreeSet::new();

    let mut case = |e: &mut Emu, v: [f32; 3], cam: [f32; 3], thr: f32, ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(vec + k * 4, v[k as usize].to_bits());
            e.write_u32(CAM_POS + k * 4, cam[k as usize].to_bits());
        }
        e.write_u32(THR_683700, thr.to_bits());
        e.set_reg(RegisterX86::ESI, vec);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x683881, 0x6838bc);
        let bin = ah(e) & TEST_AH_5;
        let pf = (e.get_reg(RegisterX86::EFLAGS) >> 2) & 1;
        let ours = near_audible(v, cam, thr);
        // `jnp` jumps to passing 1 when PF is clear
        assert_eq!(
            pf == 0,
            ours,
            "{ctx}: binary AH&5={bin:#04x} PF={pf} ours={ours}"
        );
        seen.insert(bin);
    };

    // equal: 0
    case(
        &mut e,
        [10.5, 20.0, 30.0],
        [10.0, 20.0, 30.0],
        0.25,
        "d² at the threshold",
    );
    case(&mut e, [0.0; 3], [0.0; 3], 0.0, "zero at zero");
    case(
        &mut e,
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        100.0,
        "d²<thr (jnp not)",
    );
    case(
        &mut e,
        [10.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        1.0,
        "d²>thr (jnp)",
    );
    case(
        &mut e,
        [f32::NAN, 0.0, 0.0],
        [0.0; 3],
        1.0,
        "NaN coord (unordered → jnp)",
    );

    let mut checked = 0u32;
    for i in 0..4000u32 {
        let (v, cam, thr) = if i % 5 == 0 {
            (
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                frand_bits(&mut s),
            )
        } else {
            let cam = [
                frand(&mut s, -500.0, 500.0),
                frand(&mut s, -500.0, 500.0),
                frand(&mut s, -500.0, 500.0),
            ];
            let span = match i % 3 {
                0 => 30.0_f32,
                1 => 5.0,
                _ => 100.0,
            };
            let v = [
                cam[0] + frand(&mut s, -span, span),
                cam[1] + frand(&mut s, -span, span),
                cam[2] + frand(&mut s, -span, span),
            ];
            (v, cam, frand(&mut s, 0.0, 3000.0))
        };
        case(&mut e, v, cam, thr, &format!("fuzz{i}"));
        checked += 1;
    }
    assert!(seen.contains(&0x00), "no distance at or above");
    assert!(seen.contains(&0x01), "no distance below");
    assert!(seen.contains(&0x05), "no NaN");
    eprintln!(
        "scenewalk_near_audibility_jnp: {checked} cases + probes bit-exact vs WoW.exe @ 0x683700 from 0x683881 to the jnp"
    );
}

/// `CullLowDetailTiles` (`0x683040`) from `0x6830c4` to `0x6830d9`: `[ebp-0x4]` less `[0x8101b0]`.
#[test]
fn scenewalk_arealow_far_project_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x6830_c401u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, field_3c: f32, far_const: f32, ctx: &str| {
        e.write_u32(ebp - 0x4, field_3c.to_bits());
        e.write_u32(FAR_CONST_683040, far_const.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6830c4, 0x6830d9);
        let bin = e.read_f32(ebp - 0x4);
        let ours = arealow_far_project(field_3c, far_const);
        assert_f32(bin, ours, ctx);
    };

    // equal: 0
    case(&mut e, 1234.5, 1234.5, "field at the constant");
    case(&mut e, 0.0, 0.0, "zeros");
    case(&mut e, 1000.0, 250.0, "typical far distance");
    case(&mut e, -50.0, 100.0, "negative result");

    for i in 0..4000u32 {
        let (f, fc) = if i % 5 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (frand(&mut s, -1e5, 1e5), frand(&mut s, -1e3, 1e3))
        };
        case(&mut e, f, fc, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "scenewalk_arealow_far_project: {checked} cases bit-exact vs WoW.exe @ 0x683040 slice [0x6830c4,0x6830d9)"
    );
}

/// `0x683040` from `0x6831fc` to the call at `0x683224`: the camera negated to `[ebp-0x2c]` up.
#[test]
fn scenewalk_camera_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x6831_fc01u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, cam: [f32; 3], ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(CAM_POS + k * 4, cam[k as usize].to_bits());
        }
        e.write_u32(ebp - 0x2c, 0xdead_beef);
        e.write_u32(ebp - 0x28, 0xdead_beef);
        e.write_u32(ebp - 0x24, 0xdead_beef);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6831fc, 0x683224);
        let bin = [
            e.read_f32(ebp - 0x2c),
            e.read_f32(ebp - 0x28),
            e.read_f32(ebp - 0x24),
        ];
        let ours = camera_negate(cam);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} neg[{k}]"));
        }
    };

    // zero negates to −0.0
    case(&mut e, [0.0, 0.0, 0.0], "camera at 0 gives -0.0");
    case(&mut e, [100.0, -200.0, 300.0], "typical camera position");
    case(&mut e, [-1.5, 2.5, -3.5], "mixed signs");

    for i in 0..4000u32 {
        let cam = if i % 5 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -5000.0, 5000.0),
                frand(&mut s, -5000.0, 5000.0),
                frand(&mut s, -5000.0, 5000.0),
            ]
        };
        case(&mut e, cam, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "scenewalk_camera_negate: {checked} cases bit-exact vs WoW.exe @ 0x683040 slice [0x6831fc,0x683224)"
    );
}

#[test]
fn scenewalk_portal_age_fstp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let node = e.alloc(0x40);
    let parent = e.alloc(0x40);
    let mut s = 0x6b39_8a01u32;
    let mut seen = std::collections::BTreeSet::new();

    // the two sites, their links at +0x4 and +0x8
    let mut case = |e: &mut Emu, timer: f32, dt: f32, thr: f32, ctx: &str| {
        let inp = AgeInputs { timer, dt, thr };
        let ours = portal_age_fstp(timer, dt, thr);
        let (bin1, ah1, pf1) = run_age_fstp(e, node, parent, 0x4, inp, (0x6b398a, 0x6b39a7));
        assert_f32(bin1, ours.0, &format!("{ctx} #1 stored"));
        // `jp` skips the expiry when PF is set
        assert_eq!(pf1 == 0, ours.1, "{ctx} #1: AH&5={ah1:#04x} PF={pf1}");
        let (bin2, ah2, pf2) = run_age_fstp(e, node, parent, 0x8, inp, (0x6b39be, 0x6b39db));
        assert_f32(bin2, ours.0, &format!("{ctx} #2 stored"));
        // `jp` skips the expiry when PF is set
        assert_eq!(pf2 == 0, ours.1, "{ctx} #2: AH&5={ah2:#04x} PF={pf2}");
        seen.insert(ah1);
    };

    // equal does not expire
    case(&mut e, 2.0, 0.5, 1.5, "equal");
    case(&mut e, 1.0, 1.0, 0.0, "0 = 0");
    case(&mut e, 5.0, 1.0, 1.0, "result 4 > thr 1 ");
    case(&mut e, 1.0, 0.9, 5.0, "result 0.1 < thr 5");
    case(&mut e, f32::NAN, 0.0, 1.0, "NaN timer");

    let mut checked = 0u32;
    for i in 0..4000u32 {
        let (timer, dt, thr) = if i % 5 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (
                frand(&mut s, -2.0, 50.0),
                frand(&mut s, 0.0, 2.0),
                frand(&mut s, -1.0, 30.0),
            )
        };
        case(&mut e, timer, dt, thr, &format!("fuzz{i}"));
        checked += 1;
    }
    assert!(seen.contains(&0x00), "no result at or above");
    assert!(seen.contains(&0x01), "no result below");
    eprintln!(
        "scenewalk_portal_age_fstp: {checked} cases + probes bit-exact vs WoW.exe @ 0x6b3890 sites 0x6b398a and 0x6b39be"
    );
}

#[test]
fn scenewalk_portal_age_fst_live_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x200);
    let mut s = 0x6b39_eb01u32;
    let mut seen = std::collections::BTreeSet::new();

    // the two sites, `+0x15c` off ebx and `+0x1cc` off ecx
    let mut case = |e: &mut Emu, timer: f32, dt: f32, thr: f32, ctx: &str| {
        let inp = AgeInputs { timer, dt, thr };
        let ours = portal_age_fst_live(timer, dt, thr);
        let (bin3, ah3, pf3) =
            run_age_fst(e, obj, 0x15c, inp, (0x6b39eb, 0x6b3a08), RegisterX86::EBX);
        assert_f32(bin3, ours.0, &format!("{ctx} #3 stored"));
        // `jp` skips the expiry when PF is set
        assert_eq!(pf3 == 0, ours.1, "{ctx} #3: AH&0x41={ah3:#04x} PF={pf3}");
        let (bin4, ah4, pf4) =
            run_age_fst(e, obj, 0x1cc, inp, (0x6b3a72, 0x6b3a8f), RegisterX86::ECX);
        assert_f32(bin4, ours.0, &format!("{ctx} #4 stored"));
        // `jp` skips the expiry when PF is set
        assert_eq!(pf4 == 0, ours.1, "{ctx} #4: AH&0x41={ah4:#04x} PF={pf4}");
        seen.insert(ah3);
    };

    // equal expires
    case(&mut e, 2.0, 0.5, 1.5, "equal");
    case(&mut e, 1.0, 1.0, 0.0, "0 = 0");
    case(&mut e, 5.0, 1.0, 1.0, "result 4 > thr 1 ");
    case(&mut e, 1.0, 0.9, 5.0, "result < thr");
    case(&mut e, f32::NAN, 0.0, 1.0, "NaN timer");

    let mut checked = 0u32;
    for i in 0..4000u32 {
        let (timer, dt, thr) = if i % 5 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (
                frand(&mut s, -2.0, 50.0),
                frand(&mut s, 0.0, 2.0),
                frand(&mut s, -1.0, 30.0),
            )
        };
        case(&mut e, timer, dt, thr, &format!("fuzz{i}"));
        checked += 1;
    }
    assert!(seen.contains(&0x00), "no result above");
    assert!(seen.contains(&0x01), "no result below");
    assert!(seen.contains(&0x40), "no result equal");
    eprintln!(
        "scenewalk_portal_age_fst_live: {checked} cases + probes bit-exact vs WoW.exe @ 0x6b3890 sites 0x6b39eb and 0x6b3a72"
    );
}
