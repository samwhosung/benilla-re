//! The world map's projections (`systems/ui/worldmap.md#hover-and-click`,
//! `systems/ui/worldmap.md#unit-and-corpse-positions`): the cell of the continent's grid under the
//! cursor, and the world position of a point on a continent map and back.
//!
//! The kernels read the map state: the continent index `0x84506c`, the continent records `0xb6e668`
//! (stride `0x10024`), the row pointers `0xc0d5bc` and the sub-entry list `0xc0d5a0` (stride
//! `0x34`). Choosing the record from that state is not part of the kernels: each takes the fields
//! of the chosen record that it projects.
//!
//! The x87 runs at 53-bit precision, so a chain is `f64` and each `fst`/`fstp dword` is an
//! `as f32`. The constants are `f32` in `.rdata`.

/// The `f32` at `0x806544`, 2.9296876e-5.
const C544: f64 = f32::from_bits(0x37f5_c290) as f64;
/// The `f32` at `0x806554`, 2.994012e-5.
const C554: f64 = f32::from_bits(0x37fb_27eb) as f64;
/// The `f32` at `0x806558`, 4.4910183e-5.
const C558: f64 = f32::from_bits(0x383c_5df1) as f64;
/// The `f32` at `0x806524`, 1/41.75.
const C524: f64 = f32::from_bits(0x3cc4_3730) as f64;
/// The `f32` at `0x806528`, 1/62.625.
const C528: f64 = f32::from_bits(0x3c82_cf75) as f64;

/// The clamp to `[0, 1]` that `0x4a6ec0` applies (`0x4a6f3c` to `0x4a6fbc`): below 0 gives 0.0,
/// above 1 gives 1.0, and a NaN gives 1.0, as the `fcomp` ladder falls to the high clamp. The
/// stores are `mov dword imm` of the `f32` bits. `f32::clamp` would keep the NaN.
#[allow(clippy::manual_clamp)]
fn clamp01(x: f32) -> f32 {
    if x.is_nan() {
        1.0
    } else if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}

/// `0x4a6ec0` (`worldmap#hover-and-click`): the index into the continent's cell grid of the point
/// `(u, v)` on the map, or `None` where the function returns 0. The index is what the binary uses to
/// read the cell (`mov eax, [edi + 4 * idx + 0x14]`). `rec` is `[r10, r14, r18, r1c]`, the record's
/// floats at `+0x10`, `+0x14`, `+0x18` and `+0x1c`.
///
/// `u` and `v` are clamped by `clamp01`. `du = (r10 - r14)` is stored as `f32` (`fstp [ebp-4]`)
/// and `dv = (r18 - r1c)` as `f32` (`fstp [ebp-8]`); either one 0 gives `None`. Then
/// `f = (0.5 - ((1 - u) * du + r14) * C544)` is stored as `f32` (`fstp [ebp-4]`) and
/// `g = (0.5 - ((1 - v) * dv + r1c) * C544)` as `f32` (`fstp [ebp+8]`), with `du` and `dv`
/// reloaded as `f32`. An `f` or `g` outside `[0, 1]`, or NaN, gives `None`. Otherwise
/// `row = __ftol(g * -128.0)` and `col = __ftol(f * 128.0)`, each truncating, and the index is
/// `col - row * 128 + continent * 0x4009`.
pub fn zone_cell_index(u: f32, v: f32, continent: i32, rec: [f32; 4]) -> Option<i32> {
    let uc = clamp01(u);
    let vc = clamp01(v);
    let [r10, r14, r18, r1c] = rec;
    let du = (f64::from(r10) - f64::from(r14)) as f32;
    let dv = (f64::from(r18) - f64::from(r1c)) as f32;
    if du == 0.0 || dv == 0.0 {
        return None;
    }
    let f = (0.5 - ((1.0 - f64::from(uc)) * f64::from(du) + f64::from(r14)) * C544) as f32;
    let g = (0.5 - ((1.0 - f64::from(vc)) * f64::from(dv) + f64::from(r1c)) * C544) as f32;
    // The ladder at `0x4a7032` rejects a value outside [0, 1] and a NaN; `!contains` would treat
    // the NaN differently.
    #[allow(clippy::manual_range_contains)]
    let bad = f.is_nan() || f < 0.0 || f > 1.0 || g.is_nan() || g < 0.0 || g > 1.0;
    if bad {
        return None;
    }
    let row = benilla_re_cmath::x87::ftol(f64::from(g) * -128.0);
    let col = benilla_re_cmath::x87::ftol(f64::from(f) * 128.0);
    Some(col - row * 128 + continent.wrapping_mul(0x4009))
}

/// `0x4a72b0` (it calls `0x4a7320` to find the record from `ecx`): a world position on the world
/// view, the axes swapped. `r18`, `r1c` and `zoom` (`record+0x20`) are the record's fields, and
/// `arg0` and `arg1` (`[ebp+8]`, `[ebp+0xc]`) the world coordinates. Each output is one `f64` chain
/// stored as `f32`:
///
/// - `out[0] = (r18 * (1/62.625) + 0.5) - arg1 * C554 * zoom`, through `edx` (`0x4a72e3` to
///   `0x4a7303`);
/// - `out[1] = 1.0 - ((r1c * (1/41.75) + 0.5) - arg0 * C558 * zoom)`, through `[ebp+0x10]`
///   (`0x4a72c6` to `0x4a730e`).
pub fn project_world_position_view(
    r18: f32,
    r1c: f32,
    zoom: f32,
    arg0: f32,
    arg1: f32,
) -> [f32; 2] {
    let z = f64::from(zoom);
    let a = f64::from(r1c) * C524 + 0.5;
    let b = f64::from(arg0) * C558 * z;
    let c = f64::from(r18) * C528 + 0.5;
    let d = f64::from(arg1) * C554 * z;
    let out0 = (c - d) as f32;
    let out1 = (1.0 - (a - b)) as f32;
    [out0, out1]
}

/// The map record's fields `ProjectWorldPosition` (`0x4a7360`) projects with, by the mode that the
/// continent index selects.
pub enum WorldProjection {
    /// A continent of -1 (the world view) where the helper found a sub-entry: the forward map
    /// (`0x4a7393`), `out[0] = (r18 * (1/62.625) + 0.5) - in1 * C554 * zoom` and `out[1] =
    /// (r1c * (1/41.75) + 0.5) - in0 * C558 * zoom`, one chain each, stored as `f32`.
    World { r18: f32, r1c: f32, zoom: f32 },
    /// Any other continent: the zone record's rectangle, `out = 1 - (in - lo) / (hi - lo)`, a zero
    /// numerator or divisor giving 0, then a joint range test that zeroes both outputs when either
    /// is outside `[0, 1]`.
    Zone {
        r10: f32,
        r14: f32,
        r18: f32,
        r1c: f32,
    },
}

/// `ProjectWorldPosition` (`0x4a7360`, `worldmap#unit-and-corpse-positions`): the normalized `(u, v)`
/// of a world position, written through the out pointers at `[ebp+8]` and `[ebp+0xc]`. `in0` and
/// `in1` are `[ecx]` and `[ecx+4]`, the world coordinates; `edx` is the map id that chose the record.
///
/// The world mode is in [`WorldProjection::World`]. The zone mode (`0x4a743c`) builds the u from
/// `denom = (r10 - r14) as f32` (`fstp [ebp-4]`) and `num = (in1 - r14) as f32` (`fstp [ebp+8]`),
/// with `u = 0.0` if `num == 0` or `denom == 0` and `u = (1.0 - num / denom) as f32` otherwise, and
/// the v the same way from `(r18 - r1c)` and `(in0 - r1c)`. Then (`0x4a74d6`) if `u < 0`, `u > 1`,
/// `v < 0` or `v > 1`, both outputs become 0.0; a NaN passes unchanged, as `NaN < 0` and `NaN > 1`
/// are false.
pub fn project_world_position(map: WorldProjection, in0: f32, in1: f32) -> [f32; 2] {
    match map {
        WorldProjection::World { r18, r1c, zoom } => {
            let z = f64::from(zoom);
            let o0 = ((f64::from(r18) * C528 + 0.5) - f64::from(in1) * C554 * z) as f32;
            let o1 = ((f64::from(r1c) * C524 + 0.5) - f64::from(in0) * C558 * z) as f32;
            [o0, o1]
        }
        WorldProjection::Zone { r10, r14, r18, r1c } => {
            let inv_lerp = |hi: f32, lo: f32, inp: f32| -> f32 {
                let denom = (f64::from(hi) - f64::from(lo)) as f32;
                let num = (f64::from(inp) - f64::from(lo)) as f32;
                if num == 0.0 || denom == 0.0 {
                    0.0
                } else {
                    (1.0 - f64::from(num) / f64::from(denom)) as f32
                }
            };
            let result_u = inv_lerp(r10, r14, in1);
            let result_v = inv_lerp(r18, r1c, in0);
            #[allow(clippy::manual_range_contains)]
            let out_of_range = result_u < 0.0 || result_u > 1.0 || result_v < 0.0 || result_v > 1.0;
            if out_of_range {
                [0.0, 0.0]
            } else {
                [result_u, result_v]
            }
        }
    }
}
