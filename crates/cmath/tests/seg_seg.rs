//! `0x6dc900` run whole through `call_f32`, whose `fstp dword` is the caller's store, against
//! `seg_seg_dist_sq`: random segments, one in nine parallel, which reach the interior, and two
//! built cases for the clamped exits.

use benilla_re_cmath::seg_seg::seg_seg_dist_sq;
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 8.0 - 4.0
}
fn wv3(emu: &mut Emu, va: u64, v: &[f32; 3]) {
    for (i, c) in v.iter().enumerate() {
        emu.write_u32(va + (i as u64) * 4, c.to_bits());
    }
}
fn af(b: f32, o: f32, what: &str) {
    assert_eq!(
        b.to_bits(),
        o.to_bits(),
        "{what}: binary={:#010x} ours={:#010x}",
        b.to_bits(),
        o.to_bits()
    );
}

/// Writes `p1`, `p2`, `d1`, `d2` and runs `0x6dc900` on them.
fn run(
    emu: &mut Emu,
    seg_va: u64,
    q1: &[f32; 3],
    q2: &[f32; 3],
    e1: &[f32; 3],
    e2: &[f32; 3],
) -> f32 {
    wv3(emu, seg_va, q1);
    wv3(emu, seg_va + 0xc, q2);
    wv3(emu, seg_va + 0x18, e1);
    wv3(emu, seg_va + 0x24, e2);
    emu.call_f32(
        0x6dc900,
        seg_va as u32,
        (seg_va + 0xc) as u32,
        &[(seg_va + 0x18) as u32, (seg_va + 0x24) as u32],
    )
}

#[test]
fn seg_seg_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let seg_va = emu.alloc(0x40);
    let mut s = 0x5ec2_2026u32;

    for case in 0..400u32 {
        let (q1, q2) = (
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
        );
        let (e1, e2) = (
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
        );
        let (e1, e2) = if case % 9 == 0 {
            (e1, [e1[0] * 2.0, e1[1] * 2.0, e1[2] * 2.0])
        } else {
            (e1, e2)
        };
        let bd = run(&mut emu, seg_va, &q1, &q2, &e1, &e2);
        let od = seg_seg_dist_sq(&q1, &q2, &e1, &e2);
        af(bd, od, &format!("case {case}: segseg dist2"));
    }
    eprintln!("seg_seg: 400 cases bit-exact");
}

/// The clamped exits random segments miss. A: `(0,0,0) + s(1,0,0)` against `(5,1,1) + t(0,1,0)`
/// clamps s to 0 with `f ≥ 0`, the `|r·r|` exit (`0x6dca6c`), 27. C: against `(−3,1,1) + t(0,1,0)`
/// it clamps s to 1 through `0x6dcc99` to `|(2c + r·r) + a|`, 6.
#[test]
fn seg_seg_clamp_probe() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let seg_va = emu.alloc(0x40);
    for (q2, label) in [([5.0f32, 1.0, 1.0], "A"), ([-3.0, 1.0, 1.0], "C")] {
        let (q1, e1, e2) = ([0.0f32; 3], [1.0f32, 0.0, 0.0], [0.0f32, 1.0, 0.0]);
        let bd = run(&mut emu, seg_va, &q1, &q2, &e1, &e2);
        let od = seg_seg_dist_sq(&q1, &q2, &e1, &e2);
        af(bd, od, &format!("seg_seg_clamp {label}: dist2"));
    }
}
