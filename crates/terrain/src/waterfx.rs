//! The liquid tables of `InitLiquidRamps` (`0x68c4c0`, `systems/terrain/liquid.md#depth-ramps`), the
//! ripple set-up of `EmitRipple` (`0x68f5c0`) and the ripple update of `UpdateRipples`
//! (`0x68fae0`).

/// `[0x7ffe58]`: 255.0.
pub const SCALE_255: f32 = 255.0;
/// `[0x7ff9d8]`: 1.0.
pub const ONE: f32 = 1.0;
/// `[0x7ffd74]`: 0.0.
pub const ZERO: f32 = 0.0;
/// `[0x8029cc]`: 512.0, added so a float's bits hold the byte.
pub const PACK_BIAS_512: f32 = 512.0;
/// `[0x808de4]`: 64.0.
pub const SCALE_64: f32 = 64.0;
/// `[0x81023c]`: 0.125.
pub const GEOM_RATE_BASE: f32 = 0.125;
/// `[0x81028c]`: 1/9 (`0x3de38e39`).
pub const RIVER_SLOPE: f32 = f32::from_bits(0x3de3_8e39);
/// `[0x810380]`: 0.21428572 (`0x3e5b6db7`).
pub const RIVER_SCALE: f32 = f32::from_bits(0x3e5b_6db7);
/// `[0x810384]`: 4.6666665 (`0x40955555`), past which the river ramp is 1.0.
pub const RIVER_KNEE: f32 = f32::from_bits(0x4095_5555);
/// `[0x81037c]`: 1/63 (`0x3c820821`).
pub const GLARE_STEP: f32 = f32::from_bits(0x3c82_0821);
/// `[0x810358]`: 1.6 (`0x3fcccccd`).
pub const GLARE_COEFF: f32 = f32::from_bits(0x3fcc_cccd);
/// `[0x810390]`: 0.00097657740 (`0x3a800080`).
pub const DECAL_JITTER: f32 = f32::from_bits(0x3a80_0080);

/// `[0x810348]` up: the rates `0x68f5c0` picks by its last argument, 0.4, 0.4, −4.1666665, 0.24.
pub const RATE_TABLE: [f32; 4] = [
    f32::from_bits(0x3ecc_cccd),
    f32::from_bits(0x3ecc_cccd),
    f32::from_bits(0xc085_5555),
    f32::from_bits(0x3e75_c290),
];

/// `InitLiquidRamps` (`0x68c4c0`) to `0x68c535`: `1/(c·255)` with `c = [0xc81184]`, kept in the
/// register through the first loop.
pub fn ocean_ramp_recip(g: f32) -> f64 {
    f64::from(ONE) / (f64::from(g) * f64::from(SCALE_255))
}

/// `0x68c4c0` from `0x68c552`: ramp B at `0xc7fcd8`, `(c·i)·recip` kept below 1.0 (`fcom; test
/// ah,5; jnp` at `0x68c572`), else 1.0, a NaN taking 1.0.
pub fn ocean_ramp_entry(recip: f64, g: f32, idx: u32) -> f32 {
    let v = f64::from(g) * f64::from(idx) * recip;
    (if v < f64::from(ONE) {
        v
    } else {
        f64::from(ONE)
    }) as f32
}

/// `0x68c4c0` from `0x68c583`: ramp A at `0xc81768`, `i·(1/9)` times 0.21428572 unless above
/// 4.6666665 (`test ah,0x41; jne` at `0x68c594`), else 1.0.
pub fn river_ramp_entry(idx: u32) -> f32 {
    let x = f64::from(idx) * f64::from(RIVER_SLOPE);
    if x > f64::from(RIVER_KNEE) {
        ONE
    } else {
        (x * f64::from(RIVER_SCALE)) as f32
    }
}

/// `0x68c4c0` from `0x68c5c0`: the dword at `0xc7fbc0` up, `t = i·(1/63)`, `1.6·t⁸` with `t⁸` from
/// `t·t·t·t` squared on the stack, below 0.0 made 0.0 and at or above 1.0 (or NaN) made 1.0;
/// `f32(255·v + 512)`, whose bits `>> 14` are the top byte.
pub fn glare_entry(idx: u32) -> u32 {
    let x = f64::from(idx) * f64::from(GLARE_STEP);
    let x4 = x * x * x * x;
    let mut v = x4 * x4 * f64::from(GLARE_COEFF);
    if v < f64::from(ZERO) {
        v = f64::from(ZERO);
    } else if v >= f64::from(ONE) || v.is_nan() {
        v = f64::from(ONE);
    }
    let pack = (v * f64::from(SCALE_255) + f64::from(PACK_BIAS_512)) as f32;
    ((pack.to_bits() >> 0xe) & 0xff) << 0x18
}

/// `0x68c4c0` at `0x68c65c`, `0x68c69c` and `0x68c6cb`: `0.125·64.0` in the register for `__ftol`.
pub fn geom_rate() -> f64 {
    f64::from(GEOM_RATE_BASE) * f64::from(SCALE_64)
}

/// What `0x68f5c0` computes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RippleParams {
    /// `p6·p7 + p4`, stored.
    pub local_14: f32,
    /// `+0x14`: `(p6·p7 + p4 − p4)/p6`, the sum unrounded.
    pub out5: f32,
    /// `+0x20`: `p5/(p6·rate)`.
    pub out8: f32,
    /// `+0x24`: `−(p5/((1 − rate)·p6))`.
    pub out9: f32,
    /// `+0x28`: `[0xc7b2a0] + p6`.
    pub out10: f32,
}

/// `EmitRipple` (`0x68f5c0`) from `0x68f5c6`, the arguments `p4` to `p7` at `[ebp+0x10]` to
/// `[ebp+0x1c]`, `rate = RATE_TABLE[idx]` with `idx` 1 when the last argument is set.
pub fn ripple_params(p4: f32, p5: f32, p6: f32, p7: f32, g_off: f32, idx: usize) -> RippleParams {
    let rate = f64::from(RATE_TABLE[idx]);
    let local_14_live = f64::from(p6) * f64::from(p7) + f64::from(p4);
    RippleParams {
        local_14: local_14_live as f32,
        out5: ((local_14_live - f64::from(p4)) / f64::from(p6)) as f32,
        out8: (f64::from(p5) / (f64::from(p6) * rate)) as f32,
        out9: (-(f64::from(p5) / ((f64::from(ONE) - rate) * f64::from(p6)))) as f32,
        out10: (f64::from(g_off) + f64::from(p6)) as f32,
    }
}

/// `0x68f5c0` from `0x68f666`: the point less the unrounded `p6·p7 + p4` in x and y, and less 1.0
/// in z, each stored.
pub fn ripple_surface(local_14_live: f64, pt: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(pt[0]) - local_14_live) as f32,
        (f64::from(pt[1]) - local_14_live) as f32,
        (f64::from(pt[2]) - f64::from(ONE)) as f32,
    ]
}

/// `UpdateRipples` (`0x68fae0`) at `0x68fb8e`: `dt·[ebx+0x14] + [ebx+0x10]`, `dt = [0xc62510]`.
pub fn decal_pos_integrate(g_dt: f32, vel: f32, pos: f32) -> f32 {
    (f64::from(g_dt) * f64::from(vel) + f64::from(pos)) as f32
}

/// `0x68fae0` from `0x68fbb3`, the rate `[ebx+0x20]` not 0.0: `dt·rate + [ebx+0x18]` stored with
/// `fst`; past the limit `[ebx+0x1c]` (`test ah,0x41; jne`) the limit is stored, the rate cleared,
/// and the time left is `dt − (sum − limit)/rate`. The stored value and the time carried.
pub fn decal_age_fade(g_dt: f32, cur: f32, lim: f32, rate: f32) -> (f32, f64) {
    let g = f64::from(g_dt);
    let cur2 = g * f64::from(rate) + f64::from(cur);
    if cur2 > f64::from(lim) {
        (lim, g - (cur2 - f64::from(lim)) / f64::from(rate))
    } else {
        (cur2 as f32, g)
    }
}

/// `0x68fae0` at `0x68fbed`, the rate 0.0: `time·[ebx+0x24] + [ebx+0x18]`.
pub fn decal_age_step(rate2: f64, step: f32, cur: f32) -> f32 {
    (rate2 * f64::from(step) + f64::from(cur)) as f32
}

/// `0x68fae0` at `0x68fd00`: `[ebp-0x4]·0.00097657740` to `[esp]`.
pub fn decal_jitter(v: f32) -> f32 {
    (f64::from(v) * f64::from(DECAL_JITTER)) as f32
}

/// `0x68fae0` at `0x68fd14`, `0x68fd29` and `0x68fd3d`: a camera component negated.
pub fn corner_ray_negate(g: f32) -> f32 {
    (-f64::from(g)) as f32
}

/// `0x68fae0` from `0x6900a9`: `[n8 + 1, n10 + n4, n10 + n0]`, `n10` the stored copy of `[ebx+0x10]`.
pub fn decal_pos_plus(n8: f32, n10: f32, n4: f32, n0: f32) -> [f32; 3] {
    [
        (f64::from(n8) + f64::from(ONE)) as f32,
        (f64::from(n10) + f64::from(n4)) as f32,
        (f64::from(n10) + f64::from(n0)) as f32,
    ]
}

/// `0x68fae0` from `0x6900d0`: `[n8 − 1, n4 − n10, n0 − n10]`.
pub fn decal_pos_minus(n8: f32, n10: f32, n4: f32, n0: f32) -> [f32; 3] {
    [
        (f64::from(n8) - f64::from(ONE)) as f32,
        (f64::from(n4) - f64::from(n10)) as f32,
        (f64::from(n0) - f64::from(n10)) as f32,
    ]
}

/// `0x68fae0` from `0x690150`: `f32([ebx+0x18]·255 + 512)`, its bits `>> 14` as the top byte over
/// `0xffffff`.
pub fn decal_color_pack(alpha: f32) -> u32 {
    let pack = (f64::from(alpha) * f64::from(SCALE_255) + f64::from(PACK_BIAS_512)) as f32;
    (((pack.to_bits() >> 0xe) & 0xff) << 0x18) | 0x00ff_ffff
}
