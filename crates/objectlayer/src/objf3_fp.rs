//! Float kernels of the combat log and the object base: `LogPowerDrain` (`0x627930`),
//! `LogAuraChange` (`0x62b480`) and the render position smoothing of `0x614cd0`
//! (`systems/objectlayer/combatlogmessages.md#periodic-lines` and `#auras`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`. A value
//! left on the stack is not rounded, an `fst` leaves its register live, and a value spilled to a
//! `dword` and reloaded is rounded at the spill. `fild` reads a signed integer and `fistp dword`
//! rounds to nearest-even, storing `0x80000000` for NaN or a value outside `i32`. The pow `0x73f940`,
//! the position virtuals and the model push are other code; their results are inputs.

use crate::x87::fistp;

/// 0.5 (`0x8628f4`): the bias of the displayed amount, `2p -+ 0.5`.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);
/// 0.0 (`0x7ffd74`).
pub const ZERO: f32 = 0.0;
/// 0.3572123646736145 (`0x80c6a8`, `0x3eb6e48a`): the height from which `0x614cd0` smooths.
pub const SMOOTH_Z_THRESHOLD: f32 = f32::from_bits(0x3eb6_e48a);

/// The displayed amount of `LogPowerDrain` (`0x627930`), `0x627a8f` to `0x627ace`: with `count`
/// `[ebp+0x10]` (a signed integer, `fild`) and `damage` `[ebp+0x14]`, `p = count * damage` is
/// stored as `f32` by `fst [ebp-0x10]` and left live. `fcomp 0.0; test ah,1; jne` reads the live
/// product: C0 is `p < 0` or NaN and adds 0.5, otherwise 0.5 is subtracted. Both reload the stored
/// `p`, double it (`fadd st,st`), and `fistp [ebp+0x14]` rounds to nearest-even into a dword. Then
/// `sar [ebp+0x14],1` shifts it right by one, signed. It returns that dword.
pub fn powerleech_display_amount(count: i32, damage: f32) -> i32 {
    let p_live = f64::from(count) * f64::from(damage);
    let p = p_live as f32;
    let pf = f64::from(p);
    let two_p = pf + pf;
    let biased = if p_live < 0.0 || p_live.is_nan() {
        two_p + f64::from(HALF)
    } else {
        two_p - f64::from(HALF)
    };
    fistp(biased) >> 1
}

/// The squared distance of `LogAuraChange` (`0x62b480`, `0x62b5ad` to `0x62b5e0`): the differences
/// `pos_a - pos_b` stay live and the squares sum as `(dx^2 + dz^2) + dy^2` (`fld st(2)`, `fmul`,
/// `fld st(1)`, `fmul`, `faddp`, `fld st(2)`, `fmul`, `faddp`). `pos_a` is `[eax]` and `pos_b` is
/// `[esi]`.
pub fn combat_log_dist2(pos_a: [f32; 3], pos_b: [f32; 3]) -> f64 {
    let dx = f64::from(pos_a[0]) - f64::from(pos_b[0]);
    let dy = f64::from(pos_a[1]) - f64::from(pos_b[1]);
    let dz = f64::from(pos_a[2]) - f64::from(pos_b[2]);
    let dx2 = dx * dx;
    let dz2 = dz * dz;
    (dx2 + dz2) + dy * dy
}

/// The range gate of `LogAuraChange`: `fld range; fmul range` against the distance by `fcompp; test
/// ah,0x41; jne 0x62b7c3`, which leaves for `range^2 < dist^2`, equal and NaN. It returns whether
/// the line is shown, `dist^2 < range^2` ordered. `range` is the display range `0x626810` returns,
/// stored at `[ebp-0x14]`.
pub fn combat_log_in_range(pos_a: [f32; 3], pos_b: [f32; 3], range: f32) -> bool {
    let dist2 = combat_log_dist2(pos_a, pos_b);
    let range2 = f64::from(range) * f64::from(range);
    !dist2.is_nan() && !range2.is_nan() && dist2 < range2
}

/// The gate of `0x614cd0` at `0x614ce5`: with the position from `[eax+0x20]` at `[ebp-0xc]`, `fcomp
/// 0.357...; test ah,1; jne 0x614d71` skips the smoothing for `cur_z < threshold` and NaN. It
/// returns whether the smoothing runs, `cur_z >= threshold` ordered.
pub fn render_smooth_active(cur_z: f32) -> bool {
    let z = f64::from(cur_z);
    !z.is_nan() && z >= f64::from(SMOOTH_Z_THRESHOLD)
}

/// The deltas of `0x614cd0`, `0x614cf5` onward: `target[i] - cur[i]`, each `fld; fsub; fstp dword`.
/// `target` is `this[+0x104]` onward and `cur` the position at `[ebp-0xc]`.
pub fn render_smooth_delta(target: [f32; 3], cur: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(target[0]) - f64::from(cur[0])) as f32,
        (f64::from(target[1]) - f64::from(cur[1])) as f32,
        (f64::from(target[2]) - f64::from(cur[2])) as f32,
    ]
}

/// The lerp of `0x614cd0`: `new[i] = cur[i] + delta[i] * t`, with `t` the live `pow` result at 53
/// bits. x keeps `delta.x * t` on the stack and adds `cur.x` to it; y and z store the product as
/// `f32`, reload it and then add. The product is commutative, so only the spill differs.
pub fn render_smooth_lerp(delta: [f32; 3], cur: [f32; 3], t: f64) -> [f32; 3] {
    let nx = (f64::from(delta[0]) * t + f64::from(cur[0])) as f32;
    let py = (f64::from(delta[1]) * t) as f32;
    let ny = (f64::from(py) + f64::from(cur[1])) as f32;
    let pz = (f64::from(delta[2]) * t) as f32;
    let nz = (f64::from(pz) + f64::from(cur[2])) as f32;
    [nx, ny, nz]
}

/// The amplitude product of `0x614cd0`, `0x614d8b` to `0x614db1`: `amp * (b * c)`, with `amp` =
/// `this[+0x9c]` when `this[+0xdc]` is set, else `this[+0x98]`, `b` = `this[+0x94]` and `c` =
/// `this[+0x90]`; `fld amp; fld b; fmul c; fmulp` stays live and is stored as `f32` into three
/// slots.
pub fn render_amp_product(amp: f32, b: f32, c: f32) -> f32 {
    (f64::from(amp) * (f64::from(b) * f64::from(c))) as f32
}
