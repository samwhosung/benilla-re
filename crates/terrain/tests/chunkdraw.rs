//! The draw passes' arithmetic, as slices: `DrawChunkPass` (`0x684510`) from `0x684690` to
//! `0x6846bb`; `DrawRiverPass` (`0x6851b0`) from `0x6854f0` to `0x685521`; `CWorld::Initialize`
//! (`0x66f6c0`) from `0x66f850` to `0x66f877`; `CMapChunk::Update` (`0x6afad0`) from `0x6afad6`
//! to `0x6afb28`, `0x6afb36` to `0x6afb48`, `0x6afbdd` to `0x6afbf7`, `0x6afc7c` to `0x6afca6` and
//! `0x6afd3d` to `0x6afd72`; and `DrawDriftCloud` (`0x68efe0`) from `0x68f181` to `0x68f1c9`,
//! `0x68f1eb` to `0x68f1f3` and `0x68f240` to `0x68f26c`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::chunkdraw::{
    accumulate_add, billboard_dot, cam_relative_delta, center_half, cone_negate, doodad_dist_sq,
    plane_signed_dist, quad_corner,
};
use common::{assert_f32, next};

/// A stack pointer inside the harness's stack, for slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

/// The camera's position, which `0x684510` and `0x6851b0` subtract.
const CAM_X: u64 = 0xc7cf20;

const CAM_Y: u64 = 0xc7cf24;

const CAM_Z: u64 = 0xc7cf28;

/// Where `0x66f6c0` stores `70·70`.
const DOODAD_DIST_SQ_OUT: u64 = 0xc62514;

/// `[0x867958]`, 70.0.
const DOODAD_DIST_GLOBAL: u64 = 0x867958;

/// `[0xc62510]`, the frame time.
const FRAME_DELTA: u64 = 0xc62510;

/// The view plane `0x6afad0` reads.
const PLANE_NA: u64 = 0xc7bcb0; // times [esi+0x5c]

const PLANE_NB: u64 = 0xc7bcb4; // times [esi+0x60]

const PLANE_NC: u64 = 0xc7bcb8; // times [esi+0x64]

const PLANE_D: u64 = 0xc7bcbc; // added

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// AH after `fnstsw ax`.
fn read_ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// AH of `fcomp [thresh]` masked by 0x41: C0 below, C3 equal. `test ah,0x41; jne` jumps when either
/// is set, at or below the threshold.
fn fcomp_ah_masked(st0: f64, thresh: f64) -> u8 {
    let c0 = (st0 < thresh) as u8;
    let c3 = (st0 == thresh) as u8;
    (c0 | (c3 << 6)) & 0x41
}

/// `0x684690` to `0x6846bb`, the `call 0x7bdc40` after the third store: x, y and z of ebx's `+0x6c`
/// less the camera, stored to `[ebp-0x50]`, `[ebp-0x4c]`, `[ebp-0x48]`.
#[test]
fn chunk_cam_relative_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let node = e.alloc(0x100); // ebx; the position at +0x6c
    let frame = e.alloc(0x300);
    let ebp = frame + 0x200;

    let run_case = |e: &mut Emu, pos: [f32; 3], cam: [f32; 3], ctx: &str| {
        e.write_u32(node + 0x6c, pos[0].to_bits());
        e.write_u32(node + 0x70, pos[1].to_bits());
        e.write_u32(node + 0x74, pos[2].to_bits());
        e.write_u32(CAM_X, cam[0].to_bits());
        e.write_u32(CAM_Y, cam[1].to_bits());
        e.write_u32(CAM_Z, cam[2].to_bits());
        e.set_reg(RegisterX86::EBX, node);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x684690, 0x6846bb);
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

    // the position on the camera, signed zeros, ±1, NaN and a subnormal
    run_case(&mut e, [5.0, 5.0, 5.0], [5.0, 5.0, 5.0], "684510 pos==cam");
    run_case(
        &mut e,
        [0.0, -0.0, 0.0],
        [0.0, 0.0, -0.0],
        "684510 signed zeros",
    );
    run_case(&mut e, [1.0, -1.0, 1.0], [-1.0, 1.0, -1.0], "684510 ±1");
    run_case(
        &mut e,
        [f32::from_bits(0x7fc0_0000), 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "684510 NaN pos",
    );
    run_case(
        &mut e,
        [f32::from_bits(0x0000_0001), 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "684510 denormal pos",
    );

    let mut s = 0x6846_9090u32;
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
        run_case(&mut e, pos, cam, &format!("684510 fuzz[{i}]"));
    }
}

/// `0x6854f0` to `0x685521`, a `call`: from `mov eax,[ebx+0x1c]`, the position at `eax+0x6c` less the
/// camera, z then y then x, stored x to `[ebp-0xc]`, y to `[ebp-8]`, z to `[ebp-4]`.
#[test]
fn liquid_cam_relative_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let node = e.alloc(0x100); // ebx
    let pos = e.alloc(0x100); // [ebx+0x1c] points here; the position at +0x6c
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, p: [f32; 3], cam: [f32; 3], ctx: &str| {
        e.write_u32(pos + 0x6c, p[0].to_bits());
        e.write_u32(pos + 0x70, p[1].to_bits());
        e.write_u32(pos + 0x74, p[2].to_bits());
        e.write_u32(node + 0x1c, pos as u32);
        e.write_u32(CAM_X, cam[0].to_bits());
        e.write_u32(CAM_Y, cam[1].to_bits());
        e.write_u32(CAM_Z, cam[2].to_bits());
        e.set_reg(RegisterX86::EBX, node);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6854f0, 0x685521); // from `mov eax,[ebx+0x1c]`
        let bin = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
            e.read_f32(ebp - 0x4),
        ];
        for k in 0..3 {
            let ours = cam_relative_delta(p[k], cam[k]);
            assert_f32(bin[k], ours, &format!("{ctx} delta[{k}]"));
        }
    };

    // edge cases
    run_case(&mut e, [5.0, 5.0, 5.0], [5.0, 5.0, 5.0], "6851b0 pos==cam");
    run_case(
        &mut e,
        [0.0, -0.0, 0.0],
        [0.0, 0.0, -0.0],
        "6851b0 signed zeros",
    );
    run_case(&mut e, [1.0, -1.0, 1.0], [-1.0, 1.0, -1.0], "6851b0 ±1");
    run_case(
        &mut e,
        [f32::from_bits(0x7fc0_0000), 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "6851b0 NaN pos",
    );
    run_case(
        &mut e,
        [f32::from_bits(0x0000_0001), 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "6851b0 denormal pos",
    );

    let mut s = 0x6854_f3f3u32;
    for i in 0..3000u32 {
        let p = [
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
        ];
        let cam = [
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
        ];
        run_case(&mut e, p, cam, &format!("6851b0 fuzz[{i}]"));
    }
}

/// `0x66f850` to `0x66f877`: `fld [0x867958]; fmul [0x867958]; fstp [0xc62514]`, with the 70.0 the
/// image holds.
#[test]
fn doodad_dist_sq_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // the image's 70.0 matches DOODAD_DIST
    let dist = e.read_f32(DOODAD_DIST_GLOBAL);
    assert_eq!(
        dist.to_bits(),
        0x428c_0000,
        "[0x867958] expected 70.0 (0x428c0000), got {:#010x}",
        dist.to_bits()
    );
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
    e.run(0x66f850, 0x66f877);
    let bin = e.read_f32(DOODAD_DIST_SQ_OUT);
    assert_f32(bin, doodad_dist_sq(), "66f6c0 doodad dist²");
}

/// `0x6afad6` to `0x6afb06`: the depth stored to `[esi+0x78]`. The stretch's `mov esi,ecx` at
/// `0x6afade` sets ESI from ECX; the plane is `0xc7bcb0` to `0xc7bcbc`, the chunk's values `+0x5c`
/// to `+0x68`.
#[test]
fn plane_signed_dist_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let node = e.alloc(0x200); // ecx, then esi

    let run_case =
        |e: &mut Emu, n: [f32; 3], d: f32, v: [f32; 3], ee: f32, ctx: &str| -> (f32, f32) {
            e.write_u32(PLANE_NA, n[0].to_bits());
            e.write_u32(PLANE_NB, n[1].to_bits());
            e.write_u32(PLANE_NC, n[2].to_bits());
            e.write_u32(PLANE_D, d.to_bits());
            e.write_u32(node + 0x5c, v[0].to_bits());
            e.write_u32(node + 0x60, v[1].to_bits());
            e.write_u32(node + 0x64, v[2].to_bits());
            e.write_u32(node + 0x68, ee.to_bits());
            e.set_reg(RegisterX86::ECX, node);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
            e.run(0x6afad6, 0x6afb06);
            let bin = e.read_f32(node + 0x78);
            let ours = plane_signed_dist(n[0], n[1], n[2], d, v[0], v[1], v[2], ee);
            assert_f32(bin, ours, ctx);
            (bin, ours)
        };

    // results on simple values, signed zeros and NaN
    run_case(
        &mut e,
        [1.0, 0.0, 0.0],
        0.0,
        [3.0, 0.0, 0.0],
        0.0,
        "plane n=x v=3 (→3)",
    );
    run_case(
        &mut e,
        [1.0, 1.0, 1.0],
        0.5,
        [2.0, 3.0, 4.0],
        1.0,
        "plane ones",
    );
    run_case(
        &mut e,
        [0.0, 0.0, 0.0],
        0.0,
        [0.0, 0.0, 0.0],
        0.0,
        "plane all-zero (→0)",
    );
    run_case(
        &mut e,
        [f32::from_bits(0x7fc0_0000), 0.0, 0.0],
        0.0,
        [1.0, 0.0, 0.0],
        0.0,
        "plane NaN normal",
    );

    let mut s = 0x6afa_d6d6u32;
    for i in 0..4000u32 {
        let n = [
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
        ];
        let d = frand(&mut s, -500.0, 500.0);
        let v = [
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
        ];
        let ee = frand(&mut s, -1000.0, 1000.0);
        run_case(&mut e, n, d, v, ee, &format!("plane fuzz[{i}]"));
    }
}

/// `0x6afad6` to the `jne` at `0x6afb28`: after the store, `fld [esi+0x78]; fcomp [0x867960]; test
/// ah,0x41; jne`, which jumps at or below the threshold. AH masked by 0x41 is compared with
/// [`fcomp_ah_masked`], on the threshold exactly and on random depths; `[0xc7b2a4]` has bit 2 set so
/// the compare runs.
#[test]
fn plane_lod_gate_ah_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let node = e.alloc(0x200);
    const LOD_THRESH: u64 = 0x867960; // the compare's operand
    const FLAGS: u64 = 0xc7b2a4; // `test byte ptr [0xc7b2a4],0x4` must be nonzero to reach the compare

    // The plane is chosen so the stored depth is exactly the probe's `dist`.
    let run_case = |e: &mut Emu, dist: f32, thresh: f32, ctx: &str| {
        // n = (1, 0, 0), v_a = dist, the rest 0
        e.write_u32(PLANE_NA, 1.0f32.to_bits());
        e.write_u32(PLANE_NB, 0.0f32.to_bits());
        e.write_u32(PLANE_NC, 0.0f32.to_bits());
        e.write_u32(PLANE_D, 0.0f32.to_bits());
        e.write_u32(node + 0x5c, dist.to_bits());
        e.write_u32(node + 0x60, 0.0f32.to_bits());
        e.write_u32(node + 0x64, 0.0f32.to_bits());
        e.write_u32(node + 0x68, 0.0f32.to_bits());
        e.write_u32(LOD_THRESH, thresh.to_bits());
        e.write_u32(FLAGS, 0x4); // bit 2, so the `je` after the test is not taken
        e.set_reg(RegisterX86::ECX, node);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(0x6afad6, 0x6afb28); // to the `jne`
        let ah = read_ah(e) & 0x41;
        let ours = fcomp_ah_masked(f64::from(dist), f64::from(thresh));
        assert_eq!(
            ah, ours,
            "{ctx}: LOD AH binary={ah:#04x} ours={ours:#04x} (dist={dist} thresh={thresh})"
        );
    };

    // the depth on the threshold exactly
    run_case(&mut e, 70.0, 70.0, "lod distance at the threshold");
    run_case(&mut e, 70.0, 70.0001, "lod dist<thresh");
    run_case(&mut e, 70.0001, 70.0, "lod dist>thresh");
    run_case(&mut e, -0.0, 0.0, "lod -0==+0");
    run_case(&mut e, 0.0, -0.0, "lod +0==-0");

    let mut s = 0x6afb_1d1du32;
    for i in 0..3000u32 {
        let thresh = frand(&mut s, -100.0, 100.0);
        // half the depths close either side of the threshold
        let dist = if (next(&mut s) >> 8).is_multiple_of(2) {
            thresh + frand(&mut s, -1.0, 1.0)
        } else {
            frand(&mut s, -100.0, 100.0)
        };
        run_case(&mut e, dist, thresh, &format!("lod fuzz[{i}]"));
    }
}

/// Two of `0x6afad0`'s `delta + slot` sums, each `fld [0xc62510]; fadd [slot]; fst [slot]`: the
/// chunk's `+0xac` from `0x6afb36` to the `fcomp` at `0x6afb48`, and a liquid slot's `+0x30` from
/// `0x6afbdd` to the `je` at `0x6afbf7`, with the slot's `+0x400`, which `mov eax,[edx+0x400]; test
/// eax,eax` reads, at 0.
#[test]
fn timer_accumulate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let buf = e.alloc(0x500); // edx for the liquid slot, esi for the chunk

    // the chunk's +0xac, at 0x6afb3c
    let run_node = |e: &mut Emu, delta: f32, slot: f32, ctx: &str| {
        e.write_u32(FRAME_DELTA, delta.to_bits());
        e.write_u32(buf + 0xac, slot.to_bits());
        e.set_reg(RegisterX86::ESI, buf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(0x6afb36, 0x6afb48); // halts at the `fcomp`
        let bin = e.read_f32(buf + 0xac);
        assert_f32(bin, accumulate_add(delta, slot), &format!("{ctx} (node)"));
    };

    // the liquid slot's +0x30, at 0x6afbeb
    let run_case = |e: &mut Emu, delta: f32, slot: f32, ctx: &str| {
        e.write_u32(FRAME_DELTA, delta.to_bits());
        e.write_u32(buf + 0x30, slot.to_bits());
        e.write_u32(buf + 0x400, 0); // 0, so the `je` is taken where the stretch stops
        e.set_reg(RegisterX86::EDX, buf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(0x6afbdd, 0x6afbf7);
        let bin = e.read_f32(buf + 0x30);
        assert_f32(bin, accumulate_add(delta, slot), ctx);
        run_node(e, delta, slot, ctx);
    };

    // zeros, signed zeros, exact sums and NaN
    run_case(&mut e, 0.0, 0.0, "timer 0+0");
    run_case(&mut e, 0.0, -0.0, "timer 0+(-0)");
    run_case(&mut e, 1.0, -1.0, "timer 1+(-1)=0");
    run_case(&mut e, 0.5, 0.25, "timer exact");
    run_case(&mut e, f32::from_bits(0x7fc0_0000), 1.0, "timer NaN delta");

    let mut s = 0x6afb_dddd_u32;
    for i in 0..3000u32 {
        let delta = frand(&mut s, -1.0, 1.0);
        let slot = frand(&mut s, -10000.0, 10000.0);
        run_case(&mut e, delta, slot, &format!("timer fuzz[{i}]"));
    }
}

/// `0x6afc7c` to `0x6afca6`: three sums, `[esi+0x844] + [esi+0x74]`, `[esi+0x840] + [esi+0x70]` and
/// `[esi+0x83c] + [esi+0x6c]`, stored in reverse to `[ebp-8]`, `[ebp-0xc]`, `[ebp-0x10]`; EAX is 0,
/// the index into the table at `+0x83c`.
#[test]
fn center_accumulate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let node = e.alloc(0x1000); // esi: the table at +0x83c, the position at +0x6c
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, tbl: [f32; 3], pos: [f32; 3], ctx: &str| {
        e.write_u32(node + 0x844, tbl[0].to_bits());
        e.write_u32(node + 0x840, tbl[1].to_bits());
        e.write_u32(node + 0x83c, tbl[2].to_bits());
        e.write_u32(node + 0x74, pos[2].to_bits());
        e.write_u32(node + 0x70, pos[1].to_bits());
        e.write_u32(node + 0x6c, pos[0].to_bits());
        e.set_reg(RegisterX86::ESI, node);
        e.set_reg(RegisterX86::EAX, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6afc7c, 0x6afca6);
        // [ebp-0x10] the third sum, [ebp-0xc] the second, [ebp-8] the first
        assert_f32(
            e.read_f32(ebp - 0x10),
            accumulate_add(tbl[2], pos[0]),
            &format!("{ctx} sum3"),
        );
        assert_f32(
            e.read_f32(ebp - 0xc),
            accumulate_add(tbl[1], pos[1]),
            &format!("{ctx} sum2"),
        );
        assert_f32(
            e.read_f32(ebp - 0x8),
            accumulate_add(tbl[0], pos[2]),
            &format!("{ctx} sum1"),
        );
    };

    // zeros, signed zeros, exact sums and NaN
    run_case(&mut e, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "center-acc zeros");
    run_case(
        &mut e,
        [1.0, -1.0, 0.5],
        [-1.0, 1.0, 0.25],
        "center-acc exact",
    );
    run_case(
        &mut e,
        [f32::from_bits(0x7fc0_0000), 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "center-acc NaN",
    );

    let mut s = 0x6afc_7c7cu32;
    for i in 0..3000u32 {
        let tbl = [
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
        ];
        let pos = [
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
            frand(&mut s, -34000.0, 34000.0),
        ];
        run_case(&mut e, tbl, pos, &format!("center-acc fuzz[{i}]"));
    }
}

/// `0x6afd3d` to `0x6afd72`: `SX = [ebp-0x28] + [ebp-0x34]`, `SY = [ebp-0x30] + [ebp-0x24]`, `SZ =
/// [ebp-0x2c] + [ebp-0x20]`, SZ stored to `[ebp-0x14]`; after `fxch st(1)` SX·0.5 goes to
/// `[ebp-0x10]`, SY·0.5 to `[ebp-0xc]`, and SZ reloaded ·0.5 to `[ebp-8]`.
#[test]
fn center_half_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case =
        |e: &mut Emu, xa: f32, xb: f32, ya: f32, yb: f32, za: f32, zb: f32, ctx: &str| {
            e.write_u32(ebp - 0x28, xa.to_bits());
            e.write_u32(ebp - 0x34, xb.to_bits());
            e.write_u32(ebp - 0x30, ya.to_bits());
            e.write_u32(ebp - 0x24, yb.to_bits());
            e.write_u32(ebp - 0x2c, za.to_bits());
            e.write_u32(ebp - 0x20, zb.to_bits());
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(0x6afd3d, 0x6afd72);
            assert_f32(
                e.read_f32(ebp - 0x10),
                center_half(xa, xb),
                &format!("{ctx} SX*0.5"),
            );
            assert_f32(
                e.read_f32(ebp - 0xc),
                center_half(ya, yb),
                &format!("{ctx} SY*0.5"),
            );
            assert_f32(
                e.read_f32(ebp - 0x8),
                center_half(za, zb),
                &format!("{ctx} SZ*0.5"),
            );
        };

    // zeros, sums that round, signed zeros and NaN
    run_case(&mut e, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, "center-half zeros");
    run_case(&mut e, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, "center-half exact");
    run_case(
        &mut e,
        1.0,
        f32::from_bits(0x3f80_0001),
        -1.0,
        0.0,
        0.5,
        0.25,
        "center-half odd-mantissa",
    );
    run_case(
        &mut e,
        f32::from_bits(0x7fc0_0000),
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        "center-half NaN",
    );

    let mut s = 0x6afd_3d3du32;
    for i in 0..3000u32 {
        let v: [f32; 6] = std::array::from_fn(|_| frand(&mut s, -34000.0, 34000.0));
        run_case(
            &mut e,
            v[0],
            v[1],
            v[2],
            v[3],
            v[4],
            v[5],
            &format!("center-half fuzz[{i}]"),
        );
    }
}

/// `0x68f181` to `0x68f1c9`: the three dots, `(m_hi·[esi+4] + m_mid·[esi]) + m_lo·[esi-4]`, the
/// matrix in frame slots; the first two `fstp`, the third `fst [ebp-8]`, kept for the compare after
/// it. ESI points 0x10 into the buffer so `[esi-4]` is mapped.
#[test]
fn billboard_dot_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let buf = e.alloc(0x100);
    let esi = buf + 0x10; // so [esi-4] is mapped
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    // (begin, end, the matrix slots, the output slot)
    let sites: [(u64, u64, [i64; 3], i64); 3] = [
        (0x68f181, 0x68f199, [-0x5c, -0x6c, -0x7c], -0x10), // fstp [ebp-0x10]
        (0x68f199, 0x68f1b1, [-0x58, -0x68, -0x78], -0xc),  // fstp [ebp-0xc]
        (0x68f1b1, 0x68f1c9, [-0x54, -0x64, -0x74], -0x8),  // fst [ebp-8]
    ];

    let run_case = |e: &mut Emu, m: [[f32; 3]; 3], c: [f32; 3], ctx: &str| {
        // the direction: [esi-4] = c_m1, [esi] = c0, [esi+4] = c1
        e.write_u32(esi - 0x4, c[2].to_bits());
        e.write_u32(esi, c[1].to_bits());
        e.write_u32(esi + 0x4, c[0].to_bits());
        for (d, &(begin, end, slots, out)) in sites.iter().enumerate() {
            e.write_u32((ebp as i64 + slots[0]) as u64, m[d][0].to_bits());
            e.write_u32((ebp as i64 + slots[1]) as u64, m[d][1].to_bits());
            e.write_u32((ebp as i64 + slots[2]) as u64, m[d][2].to_bits());
            e.set_reg(RegisterX86::ESI, esi);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(begin, end);
            let bin = e.read_f32((ebp as i64 + out) as u64);
            let ours = billboard_dot(m[d][0], m[d][1], m[d][2], c[0], c[1], c[2]);
            assert_f32(bin, ours, &format!("{ctx} dot{}", d + 1));
        }
    };

    // rows near the identity, zeros, signs, NaN and subnormals
    run_case(
        &mut e,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        [3.0, 4.0, 5.0],
        "dot identity",
    );
    run_case(
        &mut e,
        [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        [1.0, 2.0, 3.0],
        "dot zero-matrix",
    );
    run_case(
        &mut e,
        [[1.0, -1.0, 1.0], [-1.0, 1.0, -1.0], [1.0, 1.0, 1.0]],
        [-0.0, 0.0, -0.0],
        "dot signed",
    );
    run_case(
        &mut e,
        [
            [f32::from_bits(0x7fc0_0000), 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
        [1.0, 1.0, 1.0],
        "dot NaN m_hi",
    );

    let mut s = 0x68f1_8181u32;
    for i in 0..3000u32 {
        let m: [[f32; 3]; 3] =
            std::array::from_fn(|_| std::array::from_fn(|_| frand(&mut s, -2.0, 2.0)));
        let c = [
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
        ];
        run_case(&mut e, m, c, &format!("dot fuzz[{i}]"));
    }
}

/// `0x68f1eb` to `0x68f1f3`: `fld [ebp-8]; fchs; fstp [ebp-0x18]`.
#[test]
fn cone_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, fvar6: f32, ctx: &str| {
        e.write_u32(ebp - 0x8, fvar6.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x68f1eb, 0x68f1f3);
        let bin = e.read_f32(ebp - 0x18);
        assert_f32(bin, cone_negate(fvar6), ctx);
    };

    // 0, −0, ±1, NaN and a subnormal
    run_case(&mut e, 0.0, "cone-neg 0");
    run_case(&mut e, -0.0, "cone-neg -0");
    run_case(&mut e, 1.0, "cone-neg 1");
    run_case(&mut e, -1.0, "cone-neg -1");
    run_case(&mut e, f32::from_bits(0x7fc0_0000), "cone-neg NaN");
    run_case(&mut e, f32::from_bits(0x0000_0001), "cone-neg denormal");

    let mut s = 0x68f1_ebebu32;
    for i in 0..3000u32 {
        let fvar6 = frand(&mut s, -4096.0, 4096.0);
        run_case(&mut e, fvar6, &format!("cone-neg fuzz[{i}]"));
    }
}

/// `0x68f240` to `0x68f26c`: `fld [ecx+0xc800f4]; fmul [esi+8]; fadd [ebp-0x10]; fstp [ebp-0x30]` for
/// x and the same with `0xc800ec`, `[ebp-0xc]` and `[ebp-0x2c]` for y; ECX is 0, so the texture
/// coordinates are at `0xc800f4` and `0xc800ec`.
#[test]
fn quad_corner_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let buf = e.alloc(0x100);
    let esi = buf + 0x10;
    let vtx = e.alloc(0x40); // ebx: the `mov [ebx],edi` at 0x68f264 writes the vertex here
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    const TEX_U: u64 = 0xc800f4;
    const TEX_V: u64 = 0xc800ec;

    // (begin, end, the coordinate, the dot slot, the output slot)
    let sites: [(u64, u64, u64, i64, i64); 2] = [
        (0x68f240, 0x68f258, TEX_U, -0x10, -0x30), // x
        (0x68f258, 0x68f26c, TEX_V, -0xc, -0x2c),  // y
    ];

    let run_case = |e: &mut Emu, tex: f32, size: f32, dot: f32, ctx: &str| {
        for (i, &(begin, end, tex_addr, dot_slot, out_slot)) in sites.iter().enumerate() {
            e.write_u32(tex_addr, tex.to_bits());
            e.write_u32(esi + 0x8, size.to_bits());
            e.write_u32((ebp as i64 + dot_slot) as u64, dot.to_bits());
            e.set_reg(RegisterX86::ECX, 0);
            e.set_reg(RegisterX86::ESI, esi);
            e.set_reg(RegisterX86::EBX, vtx);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(begin, end);
            let bin = e.read_f32((ebp as i64 + out_slot) as u64);
            assert_f32(
                bin,
                quad_corner(tex, size, dot),
                &format!("{ctx} comp[{i}]"),
            );
        }
    };

    // a zero coordinate or size, exact products, signs and NaN
    run_case(&mut e, 0.0, 5.0, 3.0, "quad tex=0");
    run_case(&mut e, 2.0, 0.0, -1.0, "quad size=0");
    run_case(&mut e, 1.0, 1.0, 0.0, "quad 1*1+0");
    run_case(&mut e, -1.0, 2.0, 0.5, "quad signed");
    run_case(
        &mut e,
        f32::from_bits(0x7fc0_0000),
        1.0,
        1.0,
        "quad NaN tex",
    );

    let mut s = 0x68f2_4040u32;
    for i in 0..3000u32 {
        let tex = frand(&mut s, -4.0, 4.0);
        let size = frand(&mut s, -50.0, 50.0);
        let dot = frand(&mut s, -2000.0, 2000.0);
        run_case(&mut e, tex, size, dot, &format!("quad fuzz[{i}]"));
    }
}
