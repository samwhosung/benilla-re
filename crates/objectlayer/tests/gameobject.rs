//! The transport's path point and speed easing against the exe, each run whole: `0x5f6280` on
//! random legs, quaternions, time bases and ring cursors, comparing the three output floats and
//! the cursor it advances; and `0x5f8dc0` over the four `(has_prev, has_next)` cases, comparing the
//! return narrowed to `f32` and the animation tag.
//!
//! The path test fuzzes 2 to 8 legs with ascending times, a cursor anywhere in the ring, a time
//! that puts the fraction anywhere in `[0, 1)`, and arbitrary quaternions including unnormalized
//! and zero ones; it also runs legs whose times reach past `2^31`, where the denominator of the
//! fraction is signed. The sound side effect is suppressed with `this+0x10 = 0`.
//!
//! The easing test drives `dist` across each case so every leaf fires, and checks that the full
//! set of tags of each case was seen. A second test probes the compares at their thresholds
//! (equal and a step either side) and with NaN, infinities and zeros.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::gameobject::{
    gameobject_path_eval, transport_speed_easing, PathSegment,
};
use common::{assert_f32, frand, next, specials};

const PATH_EVAL: u64 = 0x5f6280;
const EASING: u64 = 0x5f8dc0;

#[allow(clippy::too_many_arguments)]
fn check_path_eval(
    e: &mut Emu,
    bufs: (u64, u64, u64, u64, u64), // fields, game object, legs, handler, output
    quat: [f32; 4],
    time_base: i32,
    progress: i32,
    segs: &[PathSegment],
    cursor: u32,
    ctx: &str,
) {
    let (fields, go, seg, this, out) = bufs;
    let count = segs.len() as u32;

    // The game object's fields: the quaternion at +0x10 to +0x1c.
    for (i, c) in quat.iter().enumerate() {
        e.write_u32(fields + 0x10 + 4 * i as u64, c.to_bits());
    }
    // The game object: +0x110 is the fields pointer and +0x278 the time base.
    e.write_u32(go + 0x110, fields as u32);
    e.write_u32(go + 0x278, time_base as u32);
    // The legs, at stride 0x1c: +8 time, +0xc to +0x14 position, +0x18 sound id.
    for (i, s) in segs.iter().enumerate() {
        let b = seg + i as u64 * 0x1c;
        e.write_u32(b + 0x8, s.time);
        e.write_u32(b + 0xc, s.pos[0].to_bits());
        e.write_u32(b + 0x10, s.pos[1].to_bits());
        e.write_u32(b + 0x14, s.pos[2].to_bits());
        e.write_u32(b + 0x18, s.anim_id as u32);
    }
    // The handler: +0x04 game object, +0x10 sound flag (0 suppresses it), +0x20 legs, +0x24 count,
    // +0x28 ring cursor, +0x58 last sound id.
    e.write_u32(this + 0x4, go as u32);
    e.write_u32(this + 0x10, 0);
    e.write_u32(this + 0x20, seg as u32);
    e.write_u32(this + 0x24, count);
    e.write_u32(this + 0x28, cursor);
    e.write_u32(this + 0x58, 0);
    e.write(out, &[0u8; 0xc]);

    e.call(PATH_EVAL, this as u32, 0, &[out as u32, progress as u32]);
    let bin = [e.read_f32(out), e.read_f32(out + 4), e.read_f32(out + 8)];
    let bin_cursor = e.read_u32(this + 0x28);

    let mut cur = cursor;
    let ours = gameobject_path_eval(quat, time_base, progress, segs, &mut cur);
    for i in 0..3 {
        assert_f32(bin[i], ours[i], &format!("{ctx}: out[{i}]"));
    }
    assert_eq!(bin_cursor, cur, "{ctx}: ring cursor");
}

#[test]
fn gameobject_path_eval_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // Buffers reused across the loop, for up to 8 legs.
    let fields = e.alloc(0x20);
    let go = e.alloc(0x280);
    let seg = e.alloc(8 * 0x1c);
    let this = e.alloc(0x60);
    let out = e.alloc(0x10);
    let bufs = (fields, go, seg, this, out);

    let mut s = 0x6a7e_0280u32;
    for it in 0..4000u32 {
        // 2 to 8 legs with strictly ascending times from 0, each step 1 to 400; the last is the period.
        let count = 2 + (next(&mut s) >> 8) % 7;
        let mut segs = Vec::with_capacity(count as usize);
        let mut t = 0u32;
        for _ in 0..count {
            segs.push(PathSegment {
                time: t,
                pos: [
                    frand(&mut s, -500.0, 500.0),
                    frand(&mut s, -500.0, 500.0),
                    frand(&mut s, -500.0, 500.0),
                ],
                anim_id: (next(&mut s) >> 8) as i32 & 0x7,
            });
            t = t.wrapping_add(1 + (next(&mut s) >> 8) % 400);
        }
        // Unnormalized and signed quaternions exercise the matrix.
        let quat = if next(&mut s) >> 31 == 1 {
            // About unit length
            [
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
            ]
        } else {
            [
                frand(&mut s, -2.0, 2.0),
                frand(&mut s, -2.0, 2.0),
                frand(&mut s, -2.0, 2.0),
                frand(&mut s, -2.0, 2.0),
            ]
        };
        let time_base = next(&mut s) as i32;
        let progress = next(&mut s) as i32;
        let cursor = (next(&mut s) >> 8) % count;
        check_path_eval(
            &mut e,
            bufs,
            quat,
            time_base,
            progress,
            &segs,
            cursor,
            &format!("path it {it}"),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_easing(
    e: &mut Emu,
    seg: u64,
    tag_va: u64,
    accel: f32,
    decel: f32,
    dist: f32,
    total: f32,
    has_prev: bool,
    has_next: bool,
    ctx: &str,
) -> u32 {
    e.write_u32(seg + 0x18, accel.to_bits());
    e.write_u32(seg + 0x20, decel.to_bits());
    e.write_u32(tag_va, 0xffff_ffff);
    let bin = e.call_f32(
        EASING,
        seg as u32,
        0,
        &[
            dist.to_bits(),
            total.to_bits(),
            tag_va as u32,
            u32::from(has_prev),
            u32::from(has_next),
        ],
    );
    let bin_tag = e.read_u32(tag_va);

    let (our_val, our_tag) = transport_speed_easing(accel, decel, dist, total, has_prev, has_next);
    assert_f32(bin, our_val as f32, &format!("{ctx}: return"));
    assert_eq!(bin_tag, our_tag, "{ctx}: anim tag");
    bin_tag
}

#[test]
fn transport_speed_easing_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let seg = e.alloc(0x24);
    let tag_va = e.alloc(0x10);

    let combos = [(false, false), (false, true), (true, false), (true, true)];
    let mut s = 0xea51_08d0u32;

    for &(has_prev, has_next) in &combos {
        let mut seen = 0u32; // bitset of tags observed (bit n = tag 0xa0+n)

        // Targeted probes: a/d = 1, total = 100, so the cruise knee sits at dist ~ 1..99 — each
        // leaf reachable by a chosen dist (small / mid / large).
        for &dist in &[0.5f32, 50.0, 150.0] {
            let tag = check_easing(
                &mut e,
                seg,
                tag_va,
                1.0,
                1.0,
                dist,
                100.0,
                has_prev,
                has_next,
                &format!("probe p={has_prev} n={has_next} d={dist}"),
            );
            seen |= 1 << (tag - 0xa0);
        }

        // Broad fuzz over positive accel/decel/total and dist spanning the leg (and a bit past).
        for it in 0..3000u32 {
            let accel = frand(&mut s, 0.25, 60.0);
            let decel = frand(&mut s, 0.25, 60.0);
            let total = frand(&mut s, 1.0, 1000.0);
            let dist = frand(&mut s, 0.0, total * 1.3);
            let tag = check_easing(
                &mut e,
                seg,
                tag_va,
                accel,
                decel,
                dist,
                total,
                has_prev,
                has_next,
                &format!("fuzz p={has_prev} n={has_next} it={it}"),
            );
            seen |= 1 << (tag - 0xa0);
        }

        // The achievable tag set per quadrant (the spec's leaf accounting) must all have fired.
        let expect: u32 = match (has_prev, has_next) {
            (false, false) => 1 << 3,                       // {0xa3}
            (false, true) => (1 << 3) | (1 << 4),           // {0xa3, 0xa4}
            (true, false) => (1 << 2) | (1 << 3),           // {0xa2, 0xa3}
            (true, true) => (1 << 2) | (1 << 3) | (1 << 4), // {0xa2, 0xa3, 0xa4}
        };
        assert_eq!(
            seen & expect,
            expect,
            "p={has_prev} n={has_next}: not all leaf tags fired (seen {seen:#x}, expect {expect:#x})"
        );
    }
}

/// Legs with times past `2^31`: the unsigned numerator and the signed `fidiv` denominator of the
/// fraction, and the unsigned `div` of the period.
#[test]
fn gameobject_path_eval_large_times_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let fields = e.alloc(0x20);
    let go = e.alloc(0x280);
    let seg = e.alloc(8 * 0x1c);
    let this = e.alloc(0x60);
    let out = e.alloc(0x10);
    let bufs = (fields, go, seg, this, out);
    let mut s = 0x6a7e_0281u32;
    for it in 0..1500u32 {
        // A period in [2^31, 2^32) and sorted random cut points, so a leg can pass 2^31.
        let period = 0x8000_0000 + next(&mut s) / 2;
        let mut times = vec![0u32];
        for _ in 0..(next(&mut s) >> 8) % 3 {
            times.push(1 + next(&mut s) % (period - 1));
        }
        times.sort_unstable();
        times.dedup();
        times.push(period);
        let count = times.len() as u32;
        let segs: Vec<PathSegment> = times
            .iter()
            .enumerate()
            .map(|(i, &time)| PathSegment {
                time,
                pos: [
                    frand(&mut s, -500.0, 500.0),
                    frand(&mut s, -500.0, 500.0),
                    frand(&mut s, -500.0, 500.0),
                ],
                anim_id: i as i32,
            })
            .collect();
        let quat = [
            frand(&mut s, -1.0, 1.0),
            frand(&mut s, -1.0, 1.0),
            frand(&mut s, -1.0, 1.0),
            frand(&mut s, -1.0, 1.0),
        ];
        let cursor = (next(&mut s) >> 8) % count;
        check_path_eval(
            &mut e,
            bufs,
            quat,
            next(&mut s) as i32,
            next(&mut s) as i32,
            &segs,
            cursor,
            &format!("large it {it}"),
        );
    }
}

/// The easing compares at their thresholds and with NaN, infinities and zeros in each input. The
/// base legs make the thresholds exact: `accel / decel` is 1, 2 or 0.5 and the totals are small
/// integers, so `dist` equal to the knee, to `total - knee` and a step either side runs each
/// `<=` and `<` exactly.
#[test]
fn transport_speed_easing_special_values_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let seg = e.alloc(0x24);
    let tag_va = e.alloc(0x10);

    let legs: [(f32, f32, f32); 6] = [
        (1.0, 1.0, 100.0),
        (2.0, 1.0, 100.0),
        (1.0, 2.0, 100.0),
        (3.0, 1.0, 4.0),
        (1.0, 1.0, 2.0),
        (4.0, 2.0, 1.0),
    ];
    for &(has_prev, has_next) in &[(false, false), (false, true), (true, false), (true, true)] {
        for &(accel, decel, total) in &legs {
            let knee = accel / decel;
            let mut dists =
                specials(&[total, knee, 0.5 * total, total - knee, total - 0.5 * total]);
            dists.extend([
                knee,
                total - knee,
                total - 2.0 * knee,
                0.5 * total,
                total + knee,
            ]);
            for dist in dists {
                check_easing(
                    &mut e,
                    seg,
                    tag_va,
                    accel,
                    decel,
                    dist,
                    total,
                    has_prev,
                    has_next,
                    &format!("dist p={has_prev} n={has_next} a={accel} d={decel} t={total} dist={dist:e}"),
                );
            }
            for v in specials(&[]) {
                for dist in [0.5f32, 50.0, 150.0] {
                    for (ia, id, it) in [(v, decel, total), (accel, v, total), (accel, decel, v)] {
                        check_easing(
                            &mut e,
                            seg,
                            tag_va,
                            ia,
                            id,
                            dist,
                            it,
                            has_prev,
                            has_next,
                            &format!("special p={has_prev} n={has_next} a={ia:e} d={id:e} t={it:e} dist={dist}"),
                        );
                    }
                }
            }
        }
    }
}
