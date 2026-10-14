//! The sort key, the audibility gates and the segment hit against `0x683340`, `0x6838f0` and `0x69bff0`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::scenevis::{camplane_sort_key, near_audible, segment_lerp};
use common::{assert_f32, next};

/// A frame inside the harness stack (`0x70000000`, 0x100000 bytes).
const SLICE_ESP: u64 = 0x7008_0000;

/// `test ah,5`: C0 and C2.
const TEST_AH_5: u8 = 0x5;

/// The camera plane.
const CAMPLANE_C0: u64 = 0xc7bcb0;

/// The camera.
const CAM_POS: u64 = 0xc7cf20;

/// `0x683340`'s squared distance.
const THR_683340: u64 = 0x810178;

/// `0x6838f0`'s squared distance.
const THR_6838F0: u64 = 0x81017c;

/// A float in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any float, NaN and the infinities included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// AH after `fnstsw ax`.
fn ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// `CullEntityList` (`0x683340`) from `0x683388` and `CullGroupDoodadDefList` (`0x6838f0`) from
/// `0x683972`, each to the call after: the key at `+0x78`.
#[test]
fn camplane_sort_key_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x100);
    let mut s = 0x6833_4001u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, c: [f32; 4], v: [f32; 3], radius: f32, ctx: &str| {
        for (i, &ci) in c.iter().enumerate() {
            e.write_u32(CAMPLANE_C0 + i as u64 * 4, ci.to_bits());
        }
        e.write_u32(rec + 0x5c, v[0].to_bits());
        e.write_u32(rec + 0x60, v[1].to_bits());
        e.write_u32(rec + 0x64, v[2].to_bits());
        e.write_u32(rec + 0x68, radius.to_bits());
        e.write_u32(rec + 0x78, 0xdead_beef);

        let ours = camplane_sort_key(c, v, radius);

        // `0x683340`: ebx the record
        e.set_reg(RegisterX86::EBX, rec);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x683388, 0x6833bb);
        assert_f32(e.read_f32(rec + 0x78), ours, &format!("{ctx} [0x683340]"));

        e.write_u32(rec + 0x78, 0xdead_beef);
        // `0x6838f0`: esi the record
        e.set_reg(RegisterX86::ESI, rec);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x683972, 0x6839a5);
        assert_f32(e.read_f32(rec + 0x78), ours, &format!("{ctx} [0x6838f0]"));
    };

    case(
        &mut e,
        [1.0, 0.0, 0.0, 0.0],
        [3.0, 4.0, 5.0],
        2.0,
        "plane x, key=3-2=1",
    );
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
        "camplane_sort_key: {checked} cases bit-exact vs WoW.exe @ 0x683340[0x683388,0x6833bb) + 0x6838f0[0x683972,0x6839a5)"
    );
}

/// `0x683340` from `0x683470` to the `jp` at `0x6834ab`, esi the centre: 1 is passed when PF is
/// clear.
#[test]
fn near_audibility_jp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let vec = e.alloc(0x10);
    let mut s = 0x6834_7001u32;
    let mut seen = std::collections::BTreeSet::new();

    let mut case = |e: &mut Emu, v: [f32; 3], cam: [f32; 3], thr: f32, ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(vec + k * 4, v[k as usize].to_bits());
            e.write_u32(CAM_POS + k * 4, cam[k as usize].to_bits());
        }
        e.write_u32(THR_683340, thr.to_bits());
        e.set_reg(RegisterX86::ESI, vec);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x683470, 0x6834ab);
        let bin = ah(e) & TEST_AH_5;
        let pf = (e.get_reg(RegisterX86::EFLAGS) >> 2) & 1;
        let ours = near_audible(v, cam, thr);
        // `jp` jumps to passing 0 when PF is set
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
        "equal",
    );
    case(&mut e, [0.0; 3], [0.0; 3], 0.0, "0 = 0");
    // below: 1
    case(&mut e, [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], 100.0, "below");
    // above: 0
    case(&mut e, [10.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1.0, "d²>thr");
    // NaN: 0
    case(&mut e, [f32::NAN, 0.0, 0.0], [0.0; 3], 1.0, "NaN coord");

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
        "near_audibility_jp: {checked} cases + probes bit-exact vs WoW.exe @ 0x683340 from 0x683470 to the jp"
    );
}

/// `0x6838f0` from `0x683a48` to the `jnp` at `0x683a83`, edi the centre: 1 is passed when PF is
/// clear.
#[test]
fn near_audibility_jnp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let vec = e.alloc(0x10);
    let mut s = 0x683a_4801u32;
    let mut seen = std::collections::BTreeSet::new();

    let mut case = |e: &mut Emu, v: [f32; 3], cam: [f32; 3], thr: f32, ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(vec + k * 4, v[k as usize].to_bits());
            e.write_u32(CAM_POS + k * 4, cam[k as usize].to_bits());
        }
        e.write_u32(THR_6838F0, thr.to_bits());
        e.set_reg(RegisterX86::EDI, vec);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x683a48, 0x683a83);
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
        "equal",
    );
    case(&mut e, [0.0; 3], [0.0; 3], 0.0, "0 = 0");
    // below: 1
    case(&mut e, [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], 100.0, "below");
    // above: 0
    case(
        &mut e,
        [10.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        1.0,
        "d²>thr (jnp)",
    );
    // NaN: 0
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
        "near_audibility_jnp: {checked} cases + probes bit-exact vs WoW.exe @ 0x6838f0 from 0x683a48 to the jnp"
    );
}

/// `TraceWorldSegmentBody` (`0x69bff0`) from `0x69c06c` to `0x69c0bf`: esi = A, edi = B, ecx the
/// output, `[ebp+0xc]` a pointer to t.
#[test]
fn segment_lerp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pa = e.alloc(0x10);
    let pb = e.alloc(0x10);
    let out = e.alloc(0x10);
    let tslot = e.alloc(0x10);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x69bf_f001u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], t: f32, ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(pa + k * 4, a[k as usize].to_bits());
            e.write_u32(pb + k * 4, b[k as usize].to_bits());
            e.write_u32(out + k * 4, 0xdead_beef);
        }
        e.write_u32(tslot, t.to_bits());
        e.write_u32(ebp + 0xc, tslot as u32);
        e.set_reg(RegisterX86::ESI, pa);
        e.set_reg(RegisterX86::EDI, pb);
        e.set_reg(RegisterX86::ECX, out);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x69c06c, 0x69c0bf);
        let bin: [f32; 3] = std::array::from_fn(|i| e.read_f32(out + i as u64 * 4));
        let ours = segment_lerp(a, b, t);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} out[{k}]"));
        }
    };

    // t of 0, 1 and 0.5
    case(
        &mut e,
        [1.0, 2.0, 3.0],
        [10.0, 20.0, 30.0],
        0.0,
        "t=0 (out=A)",
    );
    case(&mut e, [1.0, 2.0, 3.0], [10.0, 20.0, 30.0], 1.0, "t=1");
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [2.0, 4.0, 8.0],
        0.5,
        "t=0.5 midpoint",
    );
    case(&mut e, [0.0; 3], [0.0; 3], 0.0, "all zero");
    // components that round differently
    case(
        &mut e,
        [1.000_000_1, 1.000_000_1, 1.000_000_1],
        [1.300_000_5, 1.300_000_5, 1.300_000_5],
        0.333_333_34,
        "rounding-divergence (live-y vs spilled-x/z)",
    );

    for i in 0..4000u32 {
        let (a, b, t) = if i % 5 == 0 {
            (
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                frand_bits(&mut s),
            )
        } else {
            (
                [
                    frand(&mut s, -1e4, 1e4),
                    frand(&mut s, -1e4, 1e4),
                    frand(&mut s, -1e4, 1e4),
                ],
                [
                    frand(&mut s, -1e4, 1e4),
                    frand(&mut s, -1e4, 1e4),
                    frand(&mut s, -1e4, 1e4),
                ],
                frand(&mut s, -0.5, 1.5),
            )
        };
        case(&mut e, a, b, t, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "segment_lerp: {checked} cases + probes bit-exact vs WoW.exe @ 0x69bff0 slice [0x69c06c,0x69c0bf)"
    );
}
