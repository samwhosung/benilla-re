//! The tests and builders: `BuildFootprintBox` (`0x69a200`) run whole; `DrawMapEntity`
//! (`0x6c0cc0`) from `0x6c0cce` to its `jp` at `0x6c0cdf`; and `TraceWorldPlanes` (`0x6aa160`) from
//! its entry to its `jp` at `0x6aa1a1`. Where a test stops at a `jp`, the parity flag decides.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::geompred::{
    decal_query_box, facet_seg_gate_proceeds, vtable_pred_dispatch_second,
};
use common::{assert_f32, next};

/// A stack pointer inside the harness's stack, for slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any `f32`, from random bits: subnormals, infinities and NaN included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// AH after `fnstsw ax`.
fn ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// `0x69a200` (ecx = out, edx = the extent, the centre on the stack, `ret 4`) on edge cases and
/// random ones, any bits among them.
#[test]
fn decal_query_box_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out = e.alloc(0x20);
    let ext = e.alloc(0x10);
    let center = e.alloc(0x10);
    let mut s = 0x69a2_0000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, c: [f32; 3], xy: [f32; 2], ctx: &str| {
        e.write_u32(center, c[0].to_bits());
        e.write_u32(center + 4, c[1].to_bits());
        e.write_u32(center + 8, c[2].to_bits());
        e.write_u32(ext, xy[0].to_bits());
        e.write_u32(ext + 4, xy[1].to_bits());
        for i in 0..6u64 {
            e.write_u32(out + i * 4, 0xdead_beef);
        }
        // ecx = out, edx = the extent, the centre on the stack, `ret 4`
        e.call(0x69a200, out as u32, ext as u32, &[center as u32]);
        let bin: [f32; 6] = std::array::from_fn(|i| e.read_f32(out + i as u64 * 4));
        let ours = decal_query_box(c, xy);
        for k in 0..6 {
            assert_f32(bin[k], ours[k], &format!("{ctx} box[{k}]"));
        }
    };

    case(&mut e, [10.0, 20.0, 30.0], [3.0, 4.0], "3-4-5 (h=2.5)");
    case(&mut e, [0.0; 3], [0.0, 0.0], "zeros");
    case(&mut e, [-100.0, 5.0, -3.0], [1.5, -2.0], "mixed/neg-extent");
    case(&mut e, [1e20, 1e20, 1e20], [1e20, 1e20], "overflow-inf");
    for i in 0..4000u32 {
        let c = if i % 4 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
            ]
        };
        let xy = if i % 5 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [frand(&mut s, -100.0, 100.0), frand(&mut s, -100.0, 100.0)]
        };
        case(&mut e, c, xy, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!("decal_query_box: {checked} cases bit-exact vs WoW.exe @ 0x69a200");
}

/// The mask of `test ah,5`: C0 and C2.
const TEST_AH_5: u8 = 0x5;

/// `0x6c0cce` to the `jp` at `0x6c0cdf`: `fld [esi+0x78]; fsub [esi+0x68]; fcomp [0xc62964];
/// fnstsw ax; test ah,5`, with esi a record and the threshold written; the call is made when PF is
/// clear.

#[test]
fn vtable_pred_dispatch_second_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x100);
    let mut s = 0x6c0c_0000u32;
    let mut seen = std::collections::BTreeSet::new();

    let mut case = |e: &mut Emu, f78: f32, f68: f32, thr: f32, ctx: &str| {
        e.write_u32(this + 0x78, f78.to_bits());
        e.write_u32(this + 0x68, f68.to_bits());
        e.write_u32(0xc62964, thr.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6c0cce, 0x6c0cdf); // to the `jp`, after `test ah,5`
        let bin = ah(e) & TEST_AH_5;
        let pf = (e.get_reg(RegisterX86::EFLAGS) >> 2) & 1;
        let ours = vtable_pred_dispatch_second(f78, f68, thr);
        // `jp` jumps over the call when PF is set
        let dispatch = pf == 0;
        assert_eq!(
            dispatch, ours,
            "{ctx}: binary AH&5={bin:#04x} PF={pf} ours_dispatch={ours}"
        );
        seen.insert(bin);
    };

    // The difference exactly the threshold: C3 alone, an even count, so the call is skipped.
    case(&mut e, 3.0, 1.0, 2.0, "diff==thr (skip)");
    case(&mut e, 0.0, 0.0, 0.0, "0−0==0 (skip)");
    // below: C0 alone, odd, the call is made
    case(&mut e, 1.0, 0.0, 5.0, "diff<thr (dispatch)");
    // above: nothing set, even, skipped
    case(&mut e, 10.0, 0.0, 1.0, "diff>thr (skip)");
    // NaN: C0, C2 and C3, even, skipped
    case(&mut e, f32::NAN, 0.0, 1.0, "NaN field (skip)");
    case(&mut e, 0.0, 0.0, f32::NAN, "NaN thr (skip)");

    let mut checked = 0u32;
    for i in 0..4000u32 {
        let (f78, f68, thr) = if i % 5 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s))
        } else {
            // differences either side of the threshold
            (
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -50.0, 50.0),
            )
        };
        case(&mut e, f78, f68, thr, &format!("fuzz{i}"));
        checked += 1;
    }
    assert!(
        seen.contains(&0x00),
        "an even count (skip: above or equal) never seen"
    );
    assert!(seen.contains(&0x01), "C0 (below: the call) never seen");
    assert!(seen.contains(&0x05), "C0 and C2 (NaN: skip) never seen");
    eprintln!(
        "vtable_pred_dispatch_second: {checked} cases + probes bit-exact vs WoW.exe @ 0x6c0cc0 slice [0x6c0cce,0x6c0cdf)"
    );
}

/// `0x6aa160` to the `jp` at `0x6aa1a1`, from the prologue (ecx = the segment, start at +0 and end at
/// +0xc): the length and `fcomp [0x8029d0]; fnstsw ax; test ah,5`; it goes on when PF is set.
#[test]
fn facet_seg_gate_proceeds_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let seg = e.alloc(0x20);
    let mut s = 0x6aa1_0000u32;
    let mut seen = std::collections::BTreeSet::new();

    let mut case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(seg + k * 4, a[k as usize].to_bits());
            e.write_u32(seg + 0xc + k * 4, b[k as usize].to_bits());
        }
        e.set_reg(RegisterX86::ECX, seg);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6aa160, 0x6aa1a1); // to the `jp`, after `test ah,5`
        let bin = ah(e) & TEST_AH_5;
        let pf = (e.get_reg(RegisterX86::EFLAGS) >> 2) & 1;
        let ours = facet_seg_gate_proceeds(a, b);
        // `jp` goes on to the traces when PF is set
        let bin_proceed = pf == 1;
        assert_eq!(
            bin_proceed, ours,
            "{ctx}: binary AH&5={bin:#04x} PF={pf} ours={ours}"
        );
        seen.insert(bin);
    };

    // A segment of length exactly 0.01 along x: C3 alone, an even count, so it goes on.
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.01_f32, 0.0, 0.0],
        "|seg|==0.01 (go on)",
    );
    // just under 0.01: C0 alone, odd, it returns 0
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.005_f32, 0.0, 0.0],
        "|seg|=0.005 (return 0)",
    );
    // well above: nothing set, even, it goes on
    case(&mut e, [0.0, 0.0, 0.0], [3.0, 4.0, 0.0], "|seg|=5 (go on)");
    // length 0: C0, it returns 0
    case(
        &mut e,
        [1.0, 2.0, 3.0],
        [1.0, 2.0, 3.0],
        "|seg|=0 (return 0)",
    );
    // NaN: C0, C2 and C3, even, it goes on
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [f32::NAN, 0.0, 0.0],
        "NaN coord (go on)",
    );

    let mut checked = 0u32;
    for i in 0..4000u32 {
        let (a, b) = if i % 5 == 0 {
            (
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
            )
        } else {
            let a = [
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
            ];
            // the end near the start, so the length falls either side of 0.01
            let span = match i % 3 {
                0 => 0.02_f32,
                1 => 0.005,
                _ => 5.0,
            };
            let b = [
                a[0] + frand(&mut s, -span, span),
                a[1] + frand(&mut s, -span, span),
                a[2] + frand(&mut s, -span, span),
            ];
            (a, b)
        };
        case(&mut e, a, b, &format!("fuzz{i}"));
        checked += 1;
    }
    assert!(
        seen.contains(&0x00),
        "an even count (go on: above or equal) never seen"
    );
    assert!(seen.contains(&0x01), "C0 (below: return 0) never seen");
    assert!(seen.contains(&0x05), "C0 and C2 (NaN: go on) never seen");
    eprintln!(
        "facet_seg_gate_proceeds: {checked} cases + probes bit-exact vs WoW.exe @ 0x6aa160 slice [0x6aa160,0x6aa1a1)"
    );
}
