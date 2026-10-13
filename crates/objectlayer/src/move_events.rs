//! Three stretches of x87 code in the object layer's movement handlers: the packed path offset of
//! `UnpackSplineOffset` (`0x602130`, `systems/objectlayer/unitmovement.md#monster-moves`), two
//! values from `+0xa5c` in `0x6030c0`, and the 0.1-yard test of `ProcessLocalMoveEvent`
//! (`0x60e0a0`).

/// `[0x8029b0]`: 0.25, the step of a packed offset field.
pub const DELTA_SCALE: f32 = f32::from_bits(0x3e80_0000);

/// `[0x8012cc]`: 0.75, the scale of the first `0x6030c0` value.
pub const PITCH_RATE_SCALE: f32 = f32::from_bits(0x3f40_0000);

/// `[0x7ff9d0]`: 1/36, subtracted from it.
pub const PITCH_BIAS: f32 = f32::from_bits(0x3ce3_8e39);

/// `[0x80c5c0]`: 0.4, the scale of the second.
pub const PITCH_THRESH_SCALE: f32 = f32::from_bits(0x3ecc_cccd);

/// `[0x80c408]`: 0.1, the smallest move along x or y that `0x60e0a0` reports.
pub const MOVE_DEADBAND: f32 = f32::from_bits(0x3dcc_cccd);

/// Sign-extends a `bits`-wide field, as `shl` and `sar` do.
#[inline]
fn sext(field: u32, bits: u32) -> i32 {
    let shift = 32 - bits;
    ((field << shift) as i32) >> shift
}

/// `0x602130`: the three signed fields of the packed word: bits 0 to 10, 11 to 21 and 22 to 31.
pub fn unpack_delta_fields(packed: u32) -> [i32; 3] {
    let fx = sext(packed & 0x7ff, 11);
    let fy = sext((packed >> 11) & 0x7ff, 11);
    let fz = sext(packed >> 22, 10);
    [fx, fy, fz]
}

/// `0x602148`: `base − field·0.25` per component; the x and y products are stored as `f32` first,
/// the z product stays in the register (`fsubr`). Each result is stored as `f32`.
pub fn decode_packed_delta(packed: u32, base: [f32; 3]) -> [f32; 3] {
    let [fx, fy, fz] = unpack_delta_fields(packed);
    let scale = f64::from(DELTA_SCALE);

    let sx_f32 = (f64::from(fx) * scale) as f32;
    let sy_f32 = (f64::from(fy) * scale) as f32;

    let sz_live = f64::from(fz) * scale;
    [
        (f64::from(base[0]) - f64::from(sx_f32)) as f32,
        (f64::from(base[1]) - f64::from(sy_f32)) as f32,
        (f64::from(base[2]) - sz_live) as f32,
    ]
}

/// `0x6030e0`: `[+0xa5c]·0.75 − 1/36`, stored as `f32`.
pub fn pitch_rate_value(pitch_rate: f32) -> f32 {
    (f64::from(pitch_rate) * f64::from(PITCH_RATE_SCALE) - f64::from(PITCH_BIAS)) as f32
}

/// `0x60314a`: `[+0xa5c]·0.4`, kept in the register.
pub fn pitch_threshold(pitch_rate: f32) -> f64 {
    f64::from(pitch_rate) * f64::from(PITCH_THRESH_SCALE)
}

/// `0x603159`: whether the argument is above `[+0xa5c]·0.4`; equal and NaN are not (`test
/// ah,0x41`).
pub fn pitch_arg_exceeds(arg: f32, pitch_rate: f32) -> bool {
    let a = f64::from(arg);
    let t = pitch_threshold(pitch_rate);
    a > t
}

/// `0x60e1f0`: whether `|new − current| ≥ 0.1`, or is NaN (`test ah,0x41`). The x test and the y
/// test are the same code.
pub fn move_axis_changed(new: f32, cur: f32) -> bool {
    let delta = (f64::from(new) - f64::from(cur)).abs();

    delta.is_nan() || delta >= f64::from(MOVE_DEADBAND)
}
