//! The arithmetic of `CGUnit_C::OnMonsterMove` (`0x6018f0`,
//! `systems/objectlayer/unitmovement.md#monster-moves`), which reads a monster move from a packet
//! and starts its path.
//!
//! Each function below is one stretch of its x87 code: the near test against the start
//! (`0x601982`), the facing's cosine and sine (`0x601ab7`), the delta from them (`0x601aca`, and
//! again at `0x601b7e`), the coincident-point tests (`0x601c1d`, `0x601d41`), the reflection of a
//! point (`0x601e83`), the path length (`0x601ef4`), the speed (`0x601f3d`) and the duration
//! (`0x601f9f`). The squared distances sum `(dx² + dz²) + dy²` in the register. The emulator
//! evaluates `fsincos` and `fsqrt` with the host, as `f64` does here.

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x80c5bc]`: 9.0, the squared radius of the near test.
pub const NEAR_R2: f32 = f32::from_bits(0x4110_0000);

/// `[0x80c450]`: (1/36)², the squared radius within which two path points are one.
pub const PT_EPS2: f32 = f32::from_bits(0x3a4a_4588);

/// `[0x803568]`: 1/6, the floor the path length is compared with.
pub const ARC_FLOOR: f32 = f32::from_bits(0x3e2a_aaab);

/// `[0x80306c]`: 4.0, the speed scale.
pub const SPEED_SCALE_DEFAULT: f32 = f32::from_bits(0x4080_0000);

/// `[0x801624]`: 10.0, the speed scale when flag `0x200` (bit 9) is set.
pub const SPEED_SCALE_ALT: f32 = f32::from_bits(0x4120_0000);

/// `[0x801360]`: 0.001, milliseconds to seconds.
pub const MS_TO_S: f32 = f32::from_bits(0x3a83_126f);

/// `[0x8026bc]`: 2^-20, the floor the chosen speed is compared with.
pub const SEL_EPS: f32 = f32::from_bits(0x3580_0000);

/// `[0x8022cc]`: 1000.0, seconds to milliseconds.
pub const S_TO_MS: f32 = f32::from_bits(0x447a_0000);

/// `(dx² + dz²) + dy²`, unrounded.
#[inline]
fn sum_of_squares(dx: f64, dy: f64, dz: f64) -> f64 {
    (dx * dx + dz * dz) + dy * dy
}

/// `0x601982`: `|start − P|²` against 9.0. Returns it and whether the code goes on to the forced
/// move: below 9.0, or NaN.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn case1_near_gate(base: [f32; 3], pkt: [f32; 3]) -> (f64, bool) {
    let dx = f(base[0]) - f(pkt[0]);
    let dy = f(base[1]) - f(pkt[1]);
    let dz = f(base[2]) - f(pkt[2]);
    let sos = sum_of_squares(dx, dy, dz);

    let proceed = !(sos >= f(NEAR_R2));
    (sos, proceed)
}

/// `0x601ab7`: `fsincos` of the facing, the cosine and the sine each stored as `f32`.
pub fn direction_sincos(angle: f32) -> (f32, f32) {
    let a = f(angle);
    (a.cos() as f32, a.sin() as f32)
}

/// `0x601aca` and `0x601b7e`: `P − (cos, sin, 0)`, each stored as `f32`.
pub fn case_delta_sub(pkt: [f32; 3], cos: f32, sin: f32) -> [f32; 3] {
    [
        (f(pkt[0]) - f(cos)) as f32,
        (f(pkt[1]) - f(sin)) as f32,
        (f(pkt[2]) - 0.0_f64) as f32,
    ]
}

/// `0x601c1d`: `|A − P|²` against eps². Returns it and whether the point is stored: at least eps².
pub fn case2_coincident_gate(a: [f32; 3], pkt: [f32; 3]) -> (f64, bool) {
    let dx = f(a[0]) - f(pkt[0]);
    let dy = f(a[1]) - f(pkt[1]);
    let dz = f(a[2]) - f(pkt[2]);
    let sos = sum_of_squares(dx, dy, dz);

    let store = sos >= f(PT_EPS2);
    (sos, store)
}

/// `0x601d41`: `|A − start|²` against eps² (`test ah,0x41`). Returns it and whether the point is
/// stored: above eps².
pub fn case34_coincident_gate(a: [f32; 3], base: [f32; 3]) -> (f64, bool) {
    let dx = f(a[0]) - f(base[0]);
    let dy = f(a[1]) - f(base[1]);
    let dz = f(a[2]) - f(base[2]);
    let sos = sum_of_squares(dx, dy, dz);

    let store = sos > f(PT_EPS2);
    (sos, store)
}

/// `0x601e83`: `p − (q − p)`, with the z difference stored as `f32` first.
pub fn reflect_through(p: [f32; 3], q: [f32; 3]) -> [f32; 3] {
    let dx = f(q[0]) - f(p[0]);
    let dy = f(q[1]) - f(p[1]);
    let dz_f32 = (f(q[2]) - f(p[2])) as f32;
    [
        (f(p[0]) - dx) as f32,
        (f(p[1]) - dy) as f32,
        (f(p[2]) - f(dz_f32)) as f32,
    ]
}

/// `0x601ef4`: the sum over consecutive points of `√((da² + dc²) + db²)`, kept in the register.
pub fn arc_length(pts: &[[f32; 3]]) -> f64 {
    let mut acc = 0.0_f64;
    for w in pts.windows(2) {
        let lo = w[0];
        let hi = w[1];
        let da = f(hi[0]) - f(lo[0]);
        let db = f(hi[1]) - f(lo[1]);
        let dc = f(hi[2]) - f(lo[2]);

        let sos = (da * da + dc * dc) + db * db;
        acc += sos.sqrt();
    }
    acc
}

/// `0x601f3d`: the speed times 4 (or 10), and with a nonzero duration (the packet's `[ebp-0xc]`, in
/// milliseconds), the path speed `length/(move_ms·0.001)` stored as `f32` when it is the smaller.
pub fn select_speed(acc: f64, speed: f32, alt_scale: bool, move_ms: u32) -> f64 {
    let scale = if alt_scale {
        f(SPEED_SCALE_ALT)
    } else {
        f(SPEED_SCALE_DEFAULT)
    };
    let speed_scaled = f(speed) * scale;
    if move_ms == 0 {
        return speed_scaled;
    }
    let arc_speed = acc / (f64::from(move_ms) * f(MS_TO_S));
    let arc_speed_f32 = arc_speed as f32;

    if arc_speed < speed_scaled {
        f(arc_speed_f32)
    } else {
        speed_scaled
    }
}

/// `0x601f9f`: `(length/speed)·1000`, stored as `f32`.
pub fn duration_ms(acc: f64, selected: f64) -> f32 {
    ((acc / selected) * f(S_TO_MS)) as f32
}

/// The speed choice and the duration together.
pub fn arc_speed_duration(acc: f64, speed: f32, alt_scale: bool, move_ms: u32) -> f32 {
    let selected = select_speed(acc, speed, alt_scale, move_ms);
    duration_ms(acc, selected)
}
