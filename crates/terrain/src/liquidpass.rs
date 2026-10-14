//! The liquid passes' arithmetic: the camera-relative offset of `DrawOceanPass` (`0x685010`) and
//! `DrawMagmaPass` (`0x6855a0`), and the frame indices of `GetLiquidFrame` (`0x68aac0`,
//! `systems/terrain/liquid.md#animated-textures`) and `CChunkLiquid::DrawMagma` (`0x68dca0`,
//! `systems/terrain/liquid.md#liquid-render-passes`).

use crate::x87::{fistp, ftol};

/// `[0x8022cc]`: 1000.0, seconds to milliseconds.
const MS_PER_SEC: f32 = 1000.0;
/// `[0x807330]`: 30.0, the frames of an animation.
const FRAME_COUNT: f32 = 30.0;
/// `[0x869ffc]`: 0.5, subtracted before `fistp`.
const HALF: f32 = 0.5;
/// `[0x7ffe5c]`: 0.025, the lava phase to a frame.
const PHASE_SCALE: f32 = 0.025;

/// `0x685010` from `0x685147` and `0x6855a0` from `0x68566a`, the same bytes: the vector at
/// `[node+0x1c]+0x6c` less the camera at `0xc7cf20`, each difference in the register and stored to
/// `[ebp-0xc]` up for the call to `0x58b250`.
pub fn chunk_liquid_cam_offset(field: [f32; 3], cam: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(field[0]) - f64::from(cam[0])) as f32,
        (f64::from(field[1]) - f64::from(cam[1])) as f32,
        (f64::from(field[2]) - f64::from(cam[2])) as f32,
    ]
}

/// `GetLiquidFrame` (`0x68aac0`) from `0x68aacc` to `0x68ab14`: the period `__ftol` of
/// `period_secs·1000` (`0x68aadb`), the clock `tick` modulo it (`div`), `fild` and `fidiv` the period
/// (`0x68aafc`), times 30 and stored, less 0.5 and `fistp` (`0x68ab11`). The stored product and the
/// frame.
pub fn liquid_anim_frame(period_secs: f32, tick: u32) -> (f32, i32) {
    let pm = ftol(f64::from(period_secs) * f64::from(MS_PER_SEC)) as u32;
    let r = if pm == 0 { 0 } else { tick % pm };
    let frac30 = (f64::from(r) / f64::from(pm) * f64::from(FRAME_COUNT)) as f32;
    let frame = fistp(f64::from(frac30) - f64::from(HALF));
    (frac30, frame)
}

/// `CChunkLiquid::DrawMagma` (`0x68dca0`) from `0x68dd26` to `0x68dd41`: `[0xc7b2a0]·0.025` stored to
/// `[ebp-0x4]`, less 0.5 and `fistp` to `[ebp-0x8]`. The stored product and the frame.
pub fn liquid_pass_anim_frame(phase: f32) -> (f32, i32) {
    let scaled = (f64::from(phase) * f64::from(PHASE_SCALE)) as f32;
    let frame = fistp(f64::from(scaled) - f64::from(HALF));
    (scaled, frame)
}
