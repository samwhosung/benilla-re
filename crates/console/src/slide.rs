//! The console's drop-down slide (the idle handler `0x63bd50`).
//!
//! Each idle tick moves the text-area top `[0x864548]` toward its target: `1.0 - lineMetric` open,
//! `1.0` closed, clamped to `[0, 1]`. The step is `textTop += dir * dt * 5.0`, clamped to the target.
//! The deferred command run (`0x63ce00`) and the viewport commit (`0x4429c0`) are calls this
//! function makes and the transcription leaves out; it returns the new `textTop`.

const ONE: f32 = f32::from_bits(0x3f80_0000); // [0x7ff9d8] = 1.0
const NEG_ONE: f32 = f32::from_bits(0xbf80_0000); // [0x8030a0] = -1.0 (closing direction)
const RATE: f32 = f32::from_bits(0x40a0_0000); // [0x80a1e8] = 5.0 (slide rate)

/// `OnIdle` slide step (0x63bd50). `open`/`snap` are the `[0xc4eac8]`/`[0xc4ec38]` flags (≠0),
/// `line_metric`=`[0xc4ec10]`, `text_top`=`[0x864548]` (the current animated top), `dt`=`*param_1`.
/// Returns the new `text_top`; equals `text_top` unchanged when already at the (clamped) target.
pub fn slide_animate(open: u32, line_metric: f32, snap: u32, text_top: f32, dt: f32) -> f32 {
    // target = clamp(open ? 1.0 - lineMetric : 1.0, 0.0, 1.0), with the f32 store at each step.
    let mut target: f32 = if open == 0 {
        ONE
    } else {
        (f64::from(ONE) - f64::from(line_metric)) as f32
    };
    // fld 1.0; fcomp target; clamp to 1.0 when 1.0 < target  (0x63bd6f..0x63bd88)
    if f64::from(ONE) < f64::from(target) {
        target = ONE;
    }
    // fld target(×2); fcomp 0.0; clamp to 0.0 when target <= 0.0  (0x63bd91..0x63bdb2)
    if f64::from(target) <= 0.0 {
        target = 0.0;
    }
    // fld textTop; fcomp target; no change when textTop == target  (0x63bdd6, jnp 0x63be83)
    if f64::from(text_top) == f64::from(target) {
        return text_top;
    }
    if snap != 0 {
        return target; // 0x63bdf3
    }
    // direction: textTop <= target ? +1.0 : -1.0  (0x63bdfd..0x63be1b; test ah,0x41 = C0|C3)
    let dir = if f64::from(text_top) <= f64::from(target) {
        ONE
    } else {
        NEG_ONE
    };
    // newTop = dir · dt · 5.0 + textTop  (f64 chain, the m32 loads widen; 0x63be1b..0x63be2a)
    let new_top = f64::from(dir) * f64::from(dt) * f64::from(RATE) + f64::from(text_top);
    // clamp toward target by slide direction (the open-flag-keyed branch, 0x63be30..0x63be5d):
    if open == 0 {
        // closing: store newTop when target >= newTop, else clamp to target (test ah,5 = C0)
        if f64::from(target) >= new_top {
            new_top as f32
        } else {
            target
        }
    } else {
        // opening: store newTop when target <= newTop, else clamp to target (test ah,0x41 = C0|C3)
        if f64::from(target) <= new_top {
            new_top as f32
        } else {
            target
        }
    }
}
