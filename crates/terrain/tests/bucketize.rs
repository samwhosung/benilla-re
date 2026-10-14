//! The depth bucket key and the viewport gate, as slices: `CMapChunk::BucketPoint` (`0x6818b0`)
//! from its entry to `0x6818f3`, after the `fistp`; `WalkScene` (`0x682fa0`) from its entry to the
//! `test ah,5` at `0x682fb0`, and from `0x682fb5` to the `test` at `0x682fc3`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::bucketize::{depth_bucket_key, viewport_gate, FcompCc};
use common::next;

/// The four view-plane coefficients `0x6818b0` reads (`.bss`, zero at load).
const COEF_C0_VA: u64 = 0x00c7_cfb8;

const COEF_C1_VA: u64 = 0x00c7_cfbc;

const COEF_C2_VA: u64 = 0x00c7_cfc0;

const COEF_C3_VA: u64 = 0x00c7_cfc4;

/// 0.03 and 0.01 as the exe holds them, checked in the test.
const F32_003: f32 = f32::from_bits(0x3cf5_c28f);

const F32_001: f32 = f32::from_bits(0x3c23_d70a);

/// Plane offsets that NaN, overflow or exceed `i32` once scaled by 0.03.
const NON_FINITE_OFFSETS: [f32; 6] = [
    f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    1.0e30,
    -1.0e30,
    1.0e11,
];

/// Writes the four coefficients, points EDX at `vec_ptr`, makes a frame at `frame_top`, runs the
/// stretch and returns the `fistp` result at `[ebp-8]`.
fn run_bucket(emu: &mut Emu, vec_ptr: u64, frame_top: u64, coef: [f32; 4]) -> i32 {
    emu.write_u32(COEF_C0_VA, coef[0].to_bits());
    emu.write_u32(COEF_C1_VA, coef[1].to_bits());
    emu.write_u32(COEF_C2_VA, coef[2].to_bits());
    emu.write_u32(COEF_C3_VA, coef[3].to_bits());
    emu.set_reg(RegisterX86::EDX, vec_ptr);
    emu.set_reg(RegisterX86::ESP, frame_top);
    // the prologue: push ebp, sub esp,8, so [ebp-8] is frame_top − 12
    emu.run(0x6818b0, 0x6818f3);
    emu.read_u32(frame_top - 12) as i32
}

/// Writes the rectangle, four floats at `+0x0` to `+0xc`.
fn write_rect(emu: &mut Emu, rect_ptr: u64, rect: [f32; 4]) {
    for (i, &f) in rect.iter().enumerate() {
        emu.write_u32(rect_ptr + i as u64 * 4, f.to_bits());
    }
}

/// The x side: `push edi; mov edi,ecx; fld [edi+8]; fsub [edi]; fcomp; fnstsw` to the `test` at
/// `0x682fb0`, ECX the rectangle and ESP a stack of its own. Returns AH.
fn run_gate_dim0_ah(emu: &mut Emu, rect_ptr: u64, sp: u64, rect: [f32; 4]) -> u8 {
    write_rect(emu, rect_ptr, rect);
    emu.set_reg(RegisterX86::ECX, rect_ptr);
    emu.set_reg(RegisterX86::ESP, sp);
    emu.run(0x682fa0, 0x682fb0);
    (emu.get_reg(RegisterX86::EAX) >> 8) as u8
}

/// The y side: from `0x682fb5`, where the x side falls through, with EDI the rectangle, so y is run
/// for every case; `fld [edi+0xc]; fsub [edi+4]; fcomp; fnstsw` to the `test` at `0x682fc3`.
/// Returns AH.
fn run_gate_dim1_ah(emu: &mut Emu, rect_ptr: u64, rect: [f32; 4]) -> u8 {
    write_rect(emu, rect_ptr, rect);
    emu.set_reg(RegisterX86::EDI, rect_ptr);
    emu.run(0x682fb5, 0x682fc3);
    (emu.get_reg(RegisterX86::EAX) >> 8) as u8
}

/// `0x6818b0` to `0x6818f3`, before the clamp: one straight run of x87 code. Depths below zero,
/// within `[0, 0x20)` and above, and keys on a tie.
#[test]
fn depth_bucket_key_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // the constants match the exe's
    assert_eq!(emu.read_f32(0x810174).to_bits(), F32_003.to_bits());
    assert_eq!(emu.read_f32(0x86861c), 0.5);

    let vec_ptr = emu.alloc(0x10);
    // a frame clear of the arena's later allocations
    let frame_top = emu.alloc(0x400) + 0x300;

    let mut s: u32 = 0x1234_5678;
    let mut checked = 0u32;
    let (mut below, mut inrange, mut above) = (0u32, 0u32, 0u32);

    // Ties: `dist = K/0.03` with `c = (0, 0, 0, dist)` makes `f32(0.03·dist)` the integer K, so `K −
    // 0.5` is halfway; `fistp` takes the even neighbour, K − 1 for odd K (rounding half away would
    // give K) and K for even K (truncation would give K − 1).
    for &k in &[
        2.0f64, 3.0, 4.0, 5.0, 6.0, 7.0, 10.0, 11.0, 24.0, 25.0, 30.0, 31.0,
    ] {
        let dist = k / (F32_003 as f64);
        let c3 = dist as f32;
        let coef = [0.0f32, 0.0, 0.0, c3];
        // the point does not matter with c0 = c1 = c2 = 0
        for (i, &f) in [1.5f32, -2.0, 3.0].iter().enumerate() {
            emu.write_u32(vec_ptr + i as u64 * 4, f.to_bits());
        }
        let bkey = run_bucket(&mut emu, vec_ptr, frame_top, coef);
        let okey = depth_bucket_key([1.5, -2.0, 3.0], coef);
        assert_eq!(
            bkey, okey,
            "bucket-key BOUNDARY probe K={k}: binary={bkey} ours={okey} (c3={c3})"
        );
        checked += 1;
    }

    // NaN, the infinities and offsets far past `i32` reach the `fistp`, which stores the integer
    // indefinite `0x80000000` where Rust's `as` would saturate or give 0.
    for &c3 in &NON_FINITE_OFFSETS {
        let coef = [0.0f32, 0.0, 0.0, c3];
        let point = [1.5f32, -2.0, 3.0];
        for (i, &f) in point.iter().enumerate() {
            emu.write_u32(vec_ptr + i as u64 * 4, f.to_bits());
        }
        let bkey = run_bucket(&mut emu, vec_ptr, frame_top, coef);
        let okey = depth_bucket_key(point, coef);
        assert_eq!(bkey, okey, "bucket-key c3={c3}: binary={bkey} ours={okey}");
        checked += 1;
    }
    assert_eq!(
        depth_bucket_key([0.0; 3], [0.0, 0.0, 0.0, f32::NAN]),
        i32::MIN
    );
    assert_eq!(
        depth_bucket_key([0.0; 3], [0.0, 0.0, 0.0, 1.0e30]),
        i32::MIN
    );

    for _ in 0..3000u32 {
        // coefficients near 1, points over the map, so the depth sweeps the buckets
        let coef = [
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            f32::from_bits(next(&mut s)) % 1.0,
            ((next(&mut s) >> 8) % 2000) as f32 - 500.0,
        ];
        // skip non-finite coefficients
        if !coef.iter().all(|c| c.is_finite()) {
            continue;
        }
        let point = [
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
            ((next(&mut s) >> 8) % 4000) as f32 - 2000.0,
        ];
        for (i, &f) in point.iter().enumerate() {
            emu.write_u32(vec_ptr + i as u64 * 4, f.to_bits());
        }
        let bkey = run_bucket(&mut emu, vec_ptr, frame_top, coef);
        let okey = depth_bucket_key(point, coef);
        assert_eq!(
            bkey, okey,
            "bucket-key: binary={bkey} ours={okey} coef={coef:?} point={point:?}"
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
        "bucket key must span <0, [0,0x20), >=0x20: below={below} inrange={inrange} above={above}"
    );
    eprintln!(
        "depth_bucket_key: {checked} cases bit-exact vs WoW.exe @ 0x6818b0 \
         (below={below} inrange={inrange} above={above})"
    );
}

/// `0x682fa0`, each side `hi − lo` against 0.01: above passes, below fails, equal passes and NaN
/// passes. The x side runs all four, the y side above, equal and NaN; C0, C2 and C3 are compared
/// with AH, and the decision both ways.
#[test]
fn viewport_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    assert_eq!(emu.read_f32(0x8029d0).to_bits(), F32_001.to_bits());
    let rect_ptr = emu.alloc(0x40);
    // a stack for the `push edi` and `pop edi`
    let sp = emu.alloc(0x400) + 0x300;

    // C0, C2 and C3 from AH, at bits 0, 2 and 6
    let cc_of = |ah: u8| FcompCc {
        c0: ah & 0x01 != 0,
        c2: ah & 0x04 != 0,
        c3: ah & 0x40 != 0,
    };

    // fixed cases first, then random ones
    let f32nan = f32::from_bits(0x7fc0_0000);
    let mut cases: Vec<[f32; 4]> = vec![
        // both sides exactly 0.01 wide (lo = 0)
        [0.0, 0.0, F32_001, F32_001],
        // x on the edge, y just under: fails at y
        [0.0, 0.0, F32_001, 0.005],
        // x just under: fails at x
        [0.0, 0.0, 0.005, 100.0],
        // both wide
        [10.0, 20.0, 100.0, 200.0],
        // x just above, from a non-zero lo
        [1.0, 1.0, 1.0 + F32_001 + 0.001, 1.0 + F32_001 + 0.001],
        // x NaN: passes
        [0.0, 0.0, f32nan, 100.0],
        // y NaN: passes
        [0.0, 0.0, 100.0, f32nan],
    ];
    let mut s: u32 = 0x0bad_f00d;
    for _ in 0..3000 {
        // lo and hi drawn apart
        let lo_x = ((next(&mut s) >> 8) % 2000) as f32 * 0.01 - 10.0;
        let lo_y = ((next(&mut s) >> 8) % 2000) as f32 * 0.01 - 10.0;
        // hi = lo + a width around 0.01, so both outcomes are frequent
        let wx = ((next(&mut s) >> 8) % 400) as f32 * 0.0001; // 0 .. 0.04
        let wy = ((next(&mut s) >> 8) % 400) as f32 * 0.0001;
        cases.push([lo_x, lo_y, lo_x + wx, lo_y + wy]);
    }

    let mut checked = 0u32;
    let (mut d0_pass, mut d0_fail, mut d1_pass, mut d1_fail) = (0u32, 0u32, 0u32, 0u32);
    let (mut boundary_pass, mut nan_pass) = (0u32, 0u32);
    for rect in cases {
        let (od0, od1) = viewport_gate(rect);

        // the x side, AH for every case
        let bah0 = run_gate_dim0_ah(&mut emu, rect_ptr, sp, rect);
        let bcc0 = cc_of(bah0);
        assert_eq!(
            bcc0, od0.cc,
            "gate dim0 cc: rect={rect:?} binary_ah={bah0:#04x}"
        );
        // the jump the AH gives, against the reimplementation's
        let b_jnp0 = !(bah0 & 0x5).count_ones().is_multiple_of(2);
        assert_eq!(b_jnp0, od0.jnp_taken, "gate dim0 jnp: rect={rect:?}");
        if od0.passed {
            d0_pass += 1;
        } else {
            d0_fail += 1;
        }

        // the y side, for every case
        let bah1 = run_gate_dim1_ah(&mut emu, rect_ptr, rect);
        let bcc1 = cc_of(bah1);
        assert_eq!(
            bcc1, od1.cc,
            "gate dim1 cc: rect={rect:?} binary_ah={bah1:#04x}"
        );
        let b_jnp1 = !(bah1 & 0x5).count_ones().is_multiple_of(2);
        assert_eq!(b_jnp1, od1.jnp_taken, "gate dim1 jnp: rect={rect:?}");
        if od1.passed {
            d1_pass += 1;
        } else {
            d1_fail += 1;
        }

        // count the equal and NaN cases that pass, on both sides
        for cc in [od0.cc, od1.cc] {
            if cc.c3 && !cc.c2 {
                boundary_pass += 1;
            }
            if cc.c0 && cc.c2 && cc.c3 {
                nan_pass += 1;
            }
        }
        checked += 1;
    }
    assert!(
        d0_pass > 0 && d0_fail > 0 && d1_pass > 0 && d1_fail > 0,
        "gate branches: d0_pass={d0_pass} d0_fail={d0_fail} d1_pass={d1_pass} d1_fail={d1_fail}"
    );
    assert!(
        boundary_pass > 0,
        "the width==0.01 boundary (C3) must fire (the `<=` edge)"
    );
    assert!(nan_pass > 0, "the unordered-NaN gate-pass path must fire");
    eprintln!(
        "viewport_gate: {checked} rects bit-exact vs WoW.exe @ 0x682fa0 \
         (d0 pass={d0_pass}/fail={d0_fail}, d1 pass={d1_pass}/fail={d1_fail}, \
         boundary={boundary_pass} nan={nan_pass})"
    );
}
