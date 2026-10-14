//! The depth buckets, each from its entry to just after its `fistp`: `BucketByDepth` (`0x6816f0`)
//! to `0x681754`, `CChunkLiquid::BucketCentre` (`0x681970`) to `0x6819b4`, `BucketByDepthTail`
//! (`0x681a40`) to `0x681ab2` and `ComputeDepthBucket` (`0x6817c0`) to `0x68183e`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::depthbucket::{
    depth_bucket_v1, depth_bucket_v3, depth_bucket_v4, view_plane_bucket,
};
use common::next;

/// The view plane (`.bss`, zero at load): c0 to c3 at +0x0 to +0xc.
const COEF_C0_VA: u64 = 0x00c7_cfb8;

const COEF_C1_VA: u64 = 0x00c7_cfbc;

const COEF_C2_VA: u64 = 0x00c7_cfc0;

const COEF_C3_VA: u64 = 0x00c7_cfc4;

/// The camera plane (`.bss`, zero at load).
const CAM_C0_VA: u64 = 0x00c7_bcb0;

const CAM_C1_VA: u64 = 0x00c7_bcb4;

const CAM_C2_VA: u64 = 0x00c7_bcb8;

const CAM_C3_VA: u64 = 0x00c7_bcbc;

/// 0.03 and 0.5 as the exe holds them, checked in the test.
const F32_003: f32 = f32::from_bits(0x3cf5_c28f);

/// Writes the view plane.
fn write_coef(emu: &mut Emu, coef: [f32; 4]) {
    emu.write_u32(COEF_C0_VA, coef[0].to_bits());
    emu.write_u32(COEF_C1_VA, coef[1].to_bits());
    emu.write_u32(COEF_C2_VA, coef[2].to_bits());
    emu.write_u32(COEF_C3_VA, coef[3].to_bits());
}

/// Writes the camera plane.
fn write_cam(emu: &mut Emu, cam: [f32; 4]) {
    emu.write_u32(CAM_C0_VA, cam[0].to_bits());
    emu.write_u32(CAM_C1_VA, cam[1].to_bits());
    emu.write_u32(CAM_C2_VA, cam[2].to_bits());
    emu.write_u32(CAM_C3_VA, cam[3].to_bits());
}

/// Writes the point and radius to `this` at +0x5c to +0x68.
fn write_this_vec(emu: &mut Emu, this: u64, point: [f32; 3], radius: f32) {
    emu.write_u32(this + 0x5c, point[0].to_bits());
    emu.write_u32(this + 0x60, point[1].to_bits());
    emu.write_u32(this + 0x64, point[2].to_bits());
    emu.write_u32(this + 0x68, radius.to_bits());
}

/// Plane offsets that NaN, overflow or exceed `i32` once scaled by 0.03: the `fistp` stores the
/// integer indefinite `0x80000000`.
const NON_FINITE_OFFSETS: [f32; 6] = [
    f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    1.0e30,
    -1.0e30,
    1.0e11,
];

/// A plane offset for which `f32(0.03·dist)` is the integer `k`, so `k − 0.5` is a tie: the other
/// coefficients and the point are 0.
fn boundary_c3(k: f64) -> f32 {
    (k / (F32_003 as f64)) as f32
}

#[test]
fn depth_bucket_v1_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    assert_eq!(emu.read_f32(0x810174).to_bits(), F32_003.to_bits());
    assert_eq!(emu.read_f32(0x86861c), 0.5);

    let this = emu.alloc(0x100);
    let frame_top = emu.alloc(0x400) + 0x300;

    // the prologue pushes ebp, so [ebp-8], where the key goes, is frame_top − 12
    let run_v1 = |emu: &mut Emu, point: [f32; 3], radius: f32, coef: [f32; 4]| -> i32 {
        write_coef(emu, coef);
        write_this_vec(emu, this, point, radius);
        emu.set_reg(RegisterX86::ECX, this);
        emu.set_reg(RegisterX86::ESP, frame_top);
        emu.run(0x6816f0, 0x681754);
        emu.read_u32(frame_top - 12) as i32
    };

    let mut checked = 0u32;
    let (mut below, mut inrange, mut above) = (0u32, 0u32, 0u32);

    // Ties: with the plane (0, 0, 0, c3) and radius 0, `f32(0.03·c3)` is an integer K of each
    // parity; `fistp` takes the even neighbour where truncation or rounding half away would not.
    for &k in &[2.0f64, 3.0, 4.0, 5.0, 6.0, 7.0, 24.0, 25.0, 30.0, 31.0] {
        let coef = [0.0f32, 0.0, 0.0, boundary_c3(k)];
        let point = [1.5f32, -2.0, 3.0];
        let bkey = run_v1(&mut emu, point, 0.0, coef);
        let okey = depth_bucket_v1(point, 0.0, coef);
        assert_eq!(bkey, okey, "v1 BOUNDARY K={k}: binary={bkey} ours={okey}");
        checked += 1;
    }
    // a tie with a radius too: c3 − radius = k/0.03
    for &k in &[3.0f64, 4.0, 10.0, 11.0] {
        let radius = 7.25f32;
        let coef = [0.0f32, 0.0, 0.0, (k / (F32_003 as f64)) as f32 + radius];
        let point = [0.0f32, 0.0, 0.0];
        let bkey = run_v1(&mut emu, point, radius, coef);
        let okey = depth_bucket_v1(point, radius, coef);
        assert_eq!(
            bkey, okey,
            "v1 BOUNDARY+radius K={k}: binary={bkey} ours={okey}"
        );
        checked += 1;
    }

    for &c3 in &NON_FINITE_OFFSETS {
        let coef = [0.0f32, 0.0, 0.0, c3];
        let point = [1.5f32, -2.0, 3.0];
        let bkey = run_v1(&mut emu, point, 0.0, coef);
        let okey = depth_bucket_v1(point, 0.0, coef);
        assert_eq!(bkey, okey, "v1 c3={c3}: binary={bkey} ours={okey}");
    }
    assert_eq!(
        depth_bucket_v1([0.0; 3], 0.0, [0.0, 0.0, 0.0, f32::NAN]),
        i32::MIN
    );

    let mut s: u32 = 0x1357_9bdf;
    for _ in 0..3000u32 {
        let coef = [
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            ((next(&mut s) >> 8) % 2000) as f32 - 500.0,
        ];
        let radius = ((next(&mut s) >> 8) % 1000) as f32 * 0.1; // 0 to 100
        if !coef.iter().all(|c| c.is_finite()) {
            continue;
        }
        let point = [
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
        ];
        let bkey = run_v1(&mut emu, point, radius, coef);
        let okey = depth_bucket_v1(point, radius, coef);
        assert_eq!(
            bkey, okey,
            "v1: binary={bkey} ours={okey} coef={coef:?} point={point:?} r={radius}"
        );
        if okey < 0 {
            below += 1;
        } else if okey < 0x20 {
            inrange += 1;
        } else {
            above += 1;
        }
        checked += 1;
    }
    assert!(
        below > 0 && inrange > 0 && above > 0,
        "v1 key must span <0 / [0,0x20) / >=0x20: below={below} inrange={inrange} above={above}"
    );
    eprintln!(
        "depth_bucket_v1: {checked} cases bit-exact vs WoW.exe @ 0x6816f0 \
         (below={below} inrange={inrange} above={above})"
    );
}

#[test]
fn depth_bucket_v3_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    assert_eq!(emu.read_f32(0x810174).to_bits(), F32_003.to_bits());

    let vec_ptr = emu.alloc(0x10);
    let sp = emu.alloc(0x400) + 0x300;

    // `__stdcall` with the point's pointer at [ebp+8]: at ESP a return address, at ESP + 4 the
    // pointer; after `push ebp; push ecx` the key is at [ebp-4] = sp − 8
    let run_v3 = |emu: &mut Emu, point: [f32; 3], coef: [f32; 4]| -> i32 {
        write_coef(emu, coef);
        for (i, &f) in point.iter().enumerate() {
            emu.write_u32(vec_ptr + i as u64 * 4, f.to_bits());
        }
        emu.set_reg(RegisterX86::ESP, sp);
        emu.write_u32(sp, 0); // a return address, never used
        emu.write_u32(sp + 4, vec_ptr as u32); // [ebp+8]
        emu.run(0x681970, 0x6819b4);
        emu.read_u32(sp - 8) as i32
    };

    let mut checked = 0u32;
    let (mut below, mut inrange, mut above) = (0u32, 0u32, 0u32);

    // ties, both parities
    for &k in &[2.0f64, 3.0, 4.0, 5.0, 6.0, 7.0, 24.0, 25.0, 30.0, 31.0] {
        let coef = [0.0f32, 0.0, 0.0, boundary_c3(k)];
        let point = [2.5f32, 4.0, -1.0];
        let bkey = run_v3(&mut emu, point, coef);
        let okey = depth_bucket_v3(point, coef);
        assert_eq!(bkey, okey, "v3 BOUNDARY K={k}: binary={bkey} ours={okey}");
        checked += 1;
    }

    for &c3 in &NON_FINITE_OFFSETS {
        let coef = [0.0f32, 0.0, 0.0, c3];
        let point = [2.5f32, 4.0, -1.0];
        let bkey = run_v3(&mut emu, point, coef);
        let okey = depth_bucket_v3(point, coef);
        assert_eq!(bkey, okey, "v3 c3={c3}: binary={bkey} ours={okey}");
    }

    let mut s: u32 = 0x2468_ace0;
    for _ in 0..3000u32 {
        let coef = [
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            ((next(&mut s) >> 8) % 2000) as f32 - 500.0,
        ];
        if !coef.iter().all(|c| c.is_finite()) {
            continue;
        }
        let point = [
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
        ];
        let bkey = run_v3(&mut emu, point, coef);
        let okey = depth_bucket_v3(point, coef);
        assert_eq!(
            bkey, okey,
            "v3: binary={bkey} ours={okey} coef={coef:?} point={point:?}"
        );
        if okey < 0 {
            below += 1;
        } else if okey < 0x20 {
            inrange += 1;
        } else {
            above += 1;
        }
        checked += 1;
    }
    assert!(
        below > 0 && inrange > 0 && above > 0,
        "v3 key must span <0 / [0,0x20) / >=0x20: below={below} inrange={inrange} above={above}"
    );
    eprintln!(
        "depth_bucket_v3: {checked} cases bit-exact vs WoW.exe @ 0x681970 \
         (below={below} inrange={inrange} above={above})"
    );
}

#[test]
fn depth_bucket_v4_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    assert_eq!(emu.read_f32(0x810174).to_bits(), F32_003.to_bits());

    let this = emu.alloc(0x100);
    let frame_top = emu.alloc(0x400) + 0x300;

    // the prologue `push ebp; mov ebp,esp; sub esp,8`: the distance to [this+0x78], the key to
    // [ebp-8] = frame_top − 12
    let run_v4 = |emu: &mut Emu,
                  point: [f32; 3],
                  radius: f32,
                  cam: [f32; 4],
                  coef: [f32; 4]|
     -> (f32, i32) {
        write_cam(emu, cam);
        write_coef(emu, coef);
        write_this_vec(emu, this, point, radius);
        emu.write_u32(this + 0x78, 0xdead_beef); // so a missing store shows
        emu.set_reg(RegisterX86::ECX, this);
        emu.set_reg(RegisterX86::ESP, frame_top);
        emu.run(0x681a40, 0x681ab2);
        let sd = emu.read_f32(this + 0x78);
        let key = emu.read_u32(frame_top - 12) as i32;
        (sd, key)
    };

    let mut checked = 0u32;
    let (mut below, mut inrange, mut above) = (0u32, 0u32, 0u32);
    let (mut sd_pos, mut sd_neg) = (0u32, 0u32);

    // ties: the plane (0, 0, 0, c3) and radius 0; the camera plane set apart, so its store runs
    // on real values
    for &k in &[2.0f64, 3.0, 4.0, 5.0, 6.0, 7.0, 24.0, 25.0, 30.0, 31.0] {
        let coef = [0.0f32, 0.0, 0.0, boundary_c3(k)];
        let cam = [0.5f32, -0.25, 0.125, 3.0];
        let point = [1.5f32, -2.0, 3.0];
        let (bsd, bkey) = run_v4(&mut emu, point, 0.0, cam, coef);
        let (osd, okey) = depth_bucket_v4(point, 0.0, cam, coef);
        assert_eq!(
            bkey, okey,
            "v4 BOUNDARY K={k}: key binary={bkey} ours={okey}"
        );
        assert_eq!(
            bsd.to_bits(),
            osd.to_bits(),
            "v4 BOUNDARY K={k}: signed_dist binary={:#010x} ours={:#010x}",
            bsd.to_bits(),
            osd.to_bits()
        );
        checked += 1;
    }

    for &c3 in &NON_FINITE_OFFSETS {
        let coef = [0.0f32, 0.0, 0.0, c3];
        let cam = [0.5f32, -0.25, 0.125, 3.0];
        let point = [1.5f32, -2.0, 3.0];
        let (_, bkey) = run_v4(&mut emu, point, 0.0, cam, coef);
        let (_, okey) = depth_bucket_v4(point, 0.0, cam, coef);
        assert_eq!(bkey, okey, "v4 c3={c3}: key binary={bkey} ours={okey}");
    }

    let mut s: u32 = 0x0f1e_2d3c;
    for _ in 0..3000u32 {
        let coef = [
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            ((next(&mut s) >> 8) % 2000) as f32 - 500.0,
        ];
        let cam = [
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            ((next(&mut s) >> 8) % 2000) as f32 - 1000.0,
        ];
        let radius = ((next(&mut s) >> 8) % 1000) as f32 * 0.1;
        if !coef.iter().all(|c| c.is_finite()) || !cam.iter().all(|c| c.is_finite()) {
            continue;
        }
        let point = [
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
        ];
        let (bsd, bkey) = run_v4(&mut emu, point, radius, cam, coef);
        let (osd, okey) = depth_bucket_v4(point, radius, cam, coef);
        assert_eq!(
            bkey, okey,
            "v4 key: binary={bkey} ours={okey} coef={coef:?} cam={cam:?} point={point:?} r={radius}"
        );
        assert_eq!(
            bsd.to_bits(),
            osd.to_bits(),
            "v4 signed_dist: binary={:#010x} ours={:#010x} cam={cam:?} point={point:?} r={radius}",
            bsd.to_bits(),
            osd.to_bits()
        );
        if okey < 0 {
            below += 1;
        } else if okey < 0x20 {
            inrange += 1;
        } else {
            above += 1;
        }
        if osd > 0.0 {
            sd_pos += 1;
        } else if osd < 0.0 {
            sd_neg += 1;
        }
        checked += 1;
    }
    assert!(
        below > 0 && inrange > 0 && above > 0,
        "v4 key must span <0 / [0,0x20) / >=0x20: below={below} inrange={inrange} above={above}"
    );
    assert!(
        sd_pos > 0 && sd_neg > 0,
        "v4 signed_dist must span both signs: pos={sd_pos} neg={sd_neg}"
    );
    eprintln!(
        "depth_bucket_v4: {checked} cases bit-exact vs WoW.exe @ 0x681a40 \
         (key below={below} inrange={inrange} above={above}; signed_dist pos={sd_pos} neg={sd_neg})"
    );
}

#[test]
fn view_plane_bucket_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    assert_eq!(emu.read_f32(0x810174).to_bits(), F32_003.to_bits());
    // [0xc7b748] is zero at load, so `this` never matches it
    assert_eq!(emu.read_u32(0x00c7_b748), 0);

    let this = emu.alloc(0x100);
    let frame_top = emu.alloc(0x400) + 0x300;

    // the head's `cmp ecx,[0xc7b748]; je` falls through; the distance to [this+0x78], the key to
    // [ebp-8]
    let run_vp = |emu: &mut Emu,
                  point: [f32; 3],
                  radius: f32,
                  cam: [f32; 4],
                  coef: [f32; 4]|
     -> (f32, i32) {
        write_cam(emu, cam);
        write_coef(emu, coef);
        write_this_vec(emu, this, point, radius);
        emu.write_u32(this + 0x78, 0xdead_beef);
        emu.set_reg(RegisterX86::ECX, this);
        emu.set_reg(RegisterX86::ESP, frame_top);
        emu.run(0x6817c0, 0x68183e);
        let sd = emu.read_f32(this + 0x78);
        let key = emu.read_u32(frame_top - 12) as i32;
        (sd, key)
    };

    let mut checked = 0u32;
    let (mut below, mut inrange, mut above) = (0u32, 0u32, 0u32);
    let (mut sd_pos, mut sd_neg) = (0u32, 0u32);

    // ties
    for &k in &[2.0f64, 3.0, 4.0, 5.0, 6.0, 7.0, 24.0, 25.0, 30.0, 31.0] {
        let coef = [0.0f32, 0.0, 0.0, boundary_c3(k)];
        let cam = [0.5f32, -0.25, 0.125, 3.0];
        let point = [1.5f32, -2.0, 3.0];
        let (bsd, bkey) = run_vp(&mut emu, point, 0.0, cam, coef);
        let (osd, okey) = view_plane_bucket(point, 0.0, cam, coef);
        assert_eq!(
            bkey, okey,
            "vp BOUNDARY K={k}: key binary={bkey} ours={okey}"
        );
        assert_eq!(
            bsd.to_bits(),
            osd.to_bits(),
            "vp BOUNDARY K={k}: signed_dist binary={:#010x} ours={:#010x}",
            bsd.to_bits(),
            osd.to_bits()
        );
        checked += 1;
    }

    for &c3 in &NON_FINITE_OFFSETS {
        let coef = [0.0f32, 0.0, 0.0, c3];
        let cam = [0.5f32, -0.25, 0.125, 3.0];
        let point = [1.5f32, -2.0, 3.0];
        let (_, bkey) = run_vp(&mut emu, point, 0.0, cam, coef);
        let (_, okey) = view_plane_bucket(point, 0.0, cam, coef);
        assert_eq!(bkey, okey, "vp c3={c3}: key binary={bkey} ours={okey}");
    }

    let mut s: u32 = 0x7c0d_e5a1;
    for _ in 0..3000u32 {
        let coef = [
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            ((next(&mut s) >> 8) % 2000) as f32 - 500.0,
        ];
        let cam = [
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            ((next(&mut s) >> 8) % 2000) as f32 - 1000.0,
        ];
        let radius = ((next(&mut s) >> 8) % 1000) as f32 * 0.1;
        if !coef.iter().all(|c| c.is_finite()) || !cam.iter().all(|c| c.is_finite()) {
            continue;
        }
        let point = [
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
        ];
        let (bsd, bkey) = run_vp(&mut emu, point, radius, cam, coef);
        let (osd, okey) = view_plane_bucket(point, radius, cam, coef);
        assert_eq!(
            bkey, okey,
            "vp key: binary={bkey} ours={okey} coef={coef:?} cam={cam:?} point={point:?} r={radius}"
        );
        assert_eq!(
            bsd.to_bits(),
            osd.to_bits(),
            "vp signed_dist: binary={:#010x} ours={:#010x} cam={cam:?} point={point:?} r={radius}",
            bsd.to_bits(),
            osd.to_bits()
        );
        if okey < 0 {
            below += 1;
        } else if okey < 0x20 {
            inrange += 1;
        } else {
            above += 1;
        }
        if osd > 0.0 {
            sd_pos += 1;
        } else if osd < 0.0 {
            sd_neg += 1;
        }
        checked += 1;
    }
    assert!(
        below > 0 && inrange > 0 && above > 0,
        "vp key must span <0 / [0,0x20) / >=0x20: below={below} inrange={inrange} above={above}"
    );
    assert!(
        sd_pos > 0 && sd_neg > 0,
        "vp signed_dist must span both signs: pos={sd_pos} neg={sd_neg}"
    );
    eprintln!(
        "view_plane_bucket: {checked} cases bit-exact vs WoW.exe @ 0x6817c0 \
         (key below={below} inrange={inrange} above={above}; signed_dist pos={sd_pos} neg={sd_neg})"
    );
}
