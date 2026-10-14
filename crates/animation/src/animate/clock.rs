//! The per-bone clocks (`0x714530` to `0x71481a`) and the blend weight (`0x714820` to `0x71492d`)
//! of `CM2Model::Animate`: the clamp and loop legs, the inherited clocks, the end of the blend
//! slot's fade, and the smoothstep weight (`systems/animation/clock.md`).

use super::{BoneState, SeqRecord, NONE32};
use benilla_re_cmath::x87::ftol;

/// One slot's time, primary or blend, from its sequence record, cursors, speed and offset
/// (`systems/animation/clock.md#clock-legs`, `#loop-and-clamp-legs`). `dur` is the scene clock
/// `[[this+0x2c]+0xc]`. Returns the time.
fn clock(
    seqs: &[SeqRecord],
    dur: u32,
    track_idx: u32,
    lo: u32,
    hi: u32,
    rate: f32,
    bias: i32,
) -> u32 {
    let rec = seqs[track_idx as usize];
    // `fild` the window as a signed integer, times the speed, `__ftol`, plus the offset as an
    // integer add.
    let scaled = |window: u32| -> i32 {
        ftol(f64::from(window as i32) * f64::from(rate)).wrapping_add(bias)
    };
    // Every comparison is signed, mostly a subtraction and a test of the sign of the wrapped
    // difference (`0x714597 jg`, `0x7145e7 jle`, `0x7145d9 jle`, `0x7145f9 jle`).
    let wrap = |base: u32| -> u32 {
        if (rec.end as i32) <= (rec.start as i32) {
            return rec.start;
        }
        let s = scaled(base.wrapping_sub(lo));
        (s as u32) % rec.end.wrapping_sub(rec.start) + rec.start
    };
    if rec.flags & 1 != 0 {
        if (hi.wrapping_sub(dur) as i32) > 0 {
            // `hi - dur > 0`: the loop leg, with base `lo` when `lo - dur > 0`, else `dur`.
            let base = if (lo.wrapping_sub(dur) as i32) > 0 {
                lo
            } else {
                dur
            };
            wrap(base)
        } else {
            // The clamp leg: the scaled window held within `[start, end]`.
            let s = scaled(hi.wrapping_sub(lo));
            if s < 0 {
                return rec.start;
            }
            if s <= rec.end.wrapping_sub(rec.start) as i32 {
                return rec.start.wrapping_add(s as u32);
            }
            rec.end
        }
    } else {
        wrap(dur)
    }
}

/// The bone's clock block: the pause slide added to the cursors, the primary and blend times with
/// their inherited fallbacks, and the end of the blend slot when its fade is over.
#[allow(clippy::too_many_arguments)]
pub(super) fn bone_clocks(
    blocks: &mut [BoneState],
    seqs: &[SeqRecord],
    bone_count: u32,
    parent_word: u16,
    bone_idx: usize,
    dur: u32,
    pause_stamp: u32,
    wrap_delta: u32,
) {
    // The primary slot.
    if blocks[bone_idx].track_idx == NONE32 {
        // No record: the parent's clock, else bone 0's; bone 0 keeps its own.
        let p = u32::from(parent_word);
        if p < bone_count {
            let (t, g, s) = {
                let pb = &blocks[p as usize];
                (pb.time, pb.gtime, pb.stamp)
            };
            let b = &mut blocks[bone_idx];
            b.time = t;
            b.gtime = g;
            b.stamp = s;
        } else if bone_idx > 0 {
            let (t, g, s) = (blocks[0].time, blocks[0].gtime, blocks[0].stamp);
            let b = &mut blocks[bone_idx];
            b.time = t;
            b.gtime = g;
            b.stamp = s;
        }
    } else {
        let b = &mut blocks[bone_idx];
        if pause_stamp != 0 {
            b.cur_lo = b.cur_lo.wrapping_add(wrap_delta);
            b.cur_hi = b.cur_hi.wrapping_add(wrap_delta);
        }
        let t = clock(seqs, dur, b.track_idx, b.cur_lo, b.cur_hi, b.rate, b.bias);
        b.gtime = b.track_idx;
        b.time = t;
        b.stamp = bone_idx as u32;
    }

    // The blend slot.
    if blocks[bone_idx].blend_track_idx == NONE32 {
        let p = u32::from(parent_word);
        if p < bone_count {
            let (t, g) = {
                let pb = &blocks[p as usize];
                (pb.blend_time, pb.blend_gtime)
            };
            let b = &mut blocks[bone_idx];
            b.blend_time = t;
            b.blend_gtime = g;
        } else if bone_idx > 0 {
            let (t, g) = (blocks[0].blend_time, blocks[0].blend_gtime);
            let b = &mut blocks[bone_idx];
            b.blend_time = t;
            b.blend_gtime = g;
        } else {
            // Bone 0 with no blend record: the blend clock is the primary clock.
            let b = &mut blocks[bone_idx];
            b.blend_time = b.time;
            b.blend_gtime = b.gtime;
        }
    } else {
        let b = &mut blocks[bone_idx];
        if pause_stamp != 0 {
            b.blend_lo = b.blend_lo.wrapping_add(wrap_delta);
            b.blend_hi = b.blend_hi.wrapping_add(wrap_delta);
        }
        let t = clock(
            seqs,
            dur,
            b.blend_track_idx,
            b.blend_lo,
            b.blend_hi,
            b.blend_rate,
            b.blend_bias,
        );
        b.blend_gtime = b.blend_track_idx;
        b.blend_time = t;
        // `0x7147b1`: the blend record is cleared once the fade end has passed.
        if (dur.wrapping_sub(b.trans_end) as i32) >= 0 {
            b.blend_track_idx = NONE32;
        }
    }
}

/// The blend weight (`systems/animation/clock.md#weight-selection`): inherited from the parent,
/// else bone 0, when the bone has neither record; 0 when the fade is over or the two slots hold the
/// same time; otherwise the smoothstep `(3 - 2t) · t · t · amp`, narrowed once at the `[esi+0x10c]`
/// store.
pub(super) fn bone_lambda(
    blocks: &mut [BoneState],
    bone_count: u32,
    parent_word: u16,
    bone_idx: usize,
    scene_clock: u32,
) {
    let (ti, bti) = (blocks[bone_idx].track_idx, blocks[bone_idx].blend_track_idx);
    if ti == NONE32 && bti == NONE32 {
        let p = u32::from(parent_word);
        if p < bone_count {
            blocks[bone_idx].lambda = blocks[p as usize].lambda;
        } else if bone_idx > 0 {
            blocks[bone_idx].lambda = blocks[0].lambda;
        } else {
            blocks[bone_idx].lambda = 0.0;
        }
        return;
    }
    let b = &mut blocks[bone_idx];
    let elapsed = b.trans_end.wrapping_sub(scene_clock) as i32;
    if elapsed <= 0 || (b.time == b.blend_time && b.gtime == b.blend_gtime) {
        b.lambda = 0.0;
        return;
    }
    // `t = fild(elapsed) · inv_duration`, and `fst [ebp+0x14]` stores it as a float. The `t < 0`
    // test compares the 53-bit register; the comparison with 1 and every smoothstep factor read the
    // stored float (`0x7148b3` to `0x71491b`). A stored `t` of exactly 1 takes the smoothstep,
    // whose value there is `amp`.
    let t64 = f64::from(elapsed) * f64::from(b.inv_duration);
    let t32 = t64 as f32;
    let amp = f64::from(b.blend_amp);
    b.lambda = if t64 < 0.0 {
        (0.0 * amp) as f32
    } else if f64::from(t32) <= 1.0 || t32.is_nan() {
        let t = f64::from(t32);
        ((3.0 - (t + t)) * t * t * amp) as f32
    } else {
        amp as f32
    };
}
