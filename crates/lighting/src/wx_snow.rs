//! Arithmetic of `0x6769d0` Rain::DrawPatterShader (`systems/lighting/precipitation.md#rain-draw`,
//! the texture scroll basis) and of `0x677ff0` Snow::Update (`precipitation.md#snow`: the packet
//! distance, the wind deflection and the exp2 blend that lowers the eye height, the frame quota and
//! the update interval). The rest of each is calls: render states, containers, the getters
//! `0x672570`, `0x5fefc0`, `0x5ff000`, `0x468550`, the exponential `0x454d40` and the leaves
//! `0x4549f0` (the squared length `(x² + y²) + z²`, left wide), `0x674600` and `0x6745d0`.
//!
//! Values are f64 on the x87 stack and round to f32 at each `fstp dword`; a value the exe leaves on
//! the stack for a later step stays wide.

/// `[0x80e004]` = 1/12.
const C_INV12: f32 = f32::from_bits(0x3daa_aaab);
/// `[0x803568]` = 1/6.
const C_INV6: f32 = f32::from_bits(0x3e2a_aaab);
/// `[0x7ff9d8]` = 1.0.
const C_ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x7ffd74]` = 0.0.
const C_ZERO: f32 = f32::from_bits(0x0000_0000);
/// `[0x801360]` = `0x3a83126f` (0.001): the wind gate.
const C_EPS_WIND: f32 = f32::from_bits(0x3a83_126f);
/// `[0x8029d4]` = `0x34800000` (2⁻²²): the gate on the direction's length.
const C_EPS_DEGEN: f32 = f32::from_bits(0x3480_0000);
/// `[0x810004]` = 2350.0.
const C_2350: f32 = f32::from_bits(0x4512_e000);
/// `[0x80fffc]` = `0xbdcccccd` (-0.1).
const C_NEG_TENTH: f32 = f32::from_bits(0xbdcc_cccd);
/// `[0x80fff8]` = `0x3fb8aa3b` (1/ln 2).
const C_INV_LN2: f32 = f32::from_bits(0x3fb8_aa3b);
/// `[0x80ffcc]` = `0x3c88893b` (1/60): the cap on the frame time.
const C_MIN_SEL: f32 = f32::from_bits(0x3c88_893b);
/// `[0x80ffc8]` = 6144.0.
const C_6144: f32 = f32::from_bits(0x45c0_0000);

/// The nine values `0x6769d0` stores from `0x676c37` to `0x676cb6`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnowScroll {
    /// `[ebp-0x1c]` to `[ebp-0x14]`: the first row times 1/12.
    pub m12: [f32; 3],
    /// `[ebp-0x68]` to `[ebp-0x60]`: the second row times 1/6.
    pub m6: [f32; 3],
    /// `[ebp-0xc]` to `[ebp-4]`: `m12` negated.
    pub neg12: [f32; 3],
}

/// `0x6769d0` from `0x676c37` to `0x676cb6`: the first row `a` of the view matrix times 1/12, the
/// second row `b` times 1/6, and the stored first product negated, each rounded at its store.
#[must_use]
pub fn snow_scroll_matrix(a: [f32; 3], b: [f32; 3]) -> SnowScroll {
    let f = f64::from;
    let m12 = [
        (f(a[0]) * f(C_INV12)) as f32,
        (f(a[1]) * f(C_INV12)) as f32,
        (f(a[2]) * f(C_INV12)) as f32,
    ];
    let m6 = [
        (f(b[0]) * f(C_INV6)) as f32,
        (f(b[1]) * f(C_INV6)) as f32,
        (f(b[2]) * f(C_INV6)) as f32,
    ];
    let neg12 = [-m12[0], -m12[1], -m12[2]];
    SnowScroll { m12, m6, neg12 }
}

/// The squared length of `0x4549f0`, `(x² + y²) + z²` wide.
#[inline]
fn sumsq3(x: f64, y: f64, z: f64) -> f64 {
    (x * x + y * y) + z * z
}

/// `0x677ff0` from `0x6780d8` to `0x678105`: the distance of a packet from the position `0x672570`
/// returns. The three differences are stored as f32 (`[ebp-0x34]` to `[ebp-0x2c]`), `0x4549f0` sums
/// their squares and the `fsqrt` is rounded at the caller's store.
#[must_use]
pub fn snow_record_distance(rec: [f32; 3], pos: [f32; 3]) -> f32 {
    let f = f64::from;
    let dx = (f(rec[0]) - f(pos[0])) as f32;
    let dy = (f(rec[1]) - f(pos[1])) as f32;
    let dz = (f(rec[2]) - f(pos[2])) as f32;
    sumsq3(f(dx), f(dy), f(dz)).sqrt() as f32
}

/// `0x677ff0` from `0x678270` to `0x678317`: the wind deflection of the eye height. `m` is the
/// manager's `+0x7c`; `fabs; fcomp 0.001; test ah,5; jnp` makes `sel = 0` only for `|m| < 0.001`
/// (NaN goes on) and otherwise `sel = dist / m` with `dist = sqrt((dz² + dy²) + dx²)` of `glob -
/// rec` (the camera position less the manager's `+0x68`). The value is `((e10 - sel) + 2350) / ec`
/// wide, and `fcom 0.0; test ah,5; jp` stores 0.0 for a value below 0, the value itself when it is
/// below 1, and 1.0 for 1 or more and for NaN (`fcom 1.0; test ah,5; jp`).
#[must_use]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn snow_wind_deflect(m: f32, e10: f32, ec: f32, glob: [f32; 3], rec: [f32; 3]) -> f32 {
    let f = f64::from;
    let sel: f64 = if !(m.abs() < C_EPS_WIND) {
        let dx = f(glob[0]) - f(rec[0]);
        let dy = f(glob[1]) - f(rec[1]);
        let dz = f(glob[2]) - f(rec[2]);
        let sumsq = (dz * dz + dy * dy) + dx * dx;
        sumsq.sqrt() / f(m)
    } else {
        f(C_ZERO)
    };
    let blended = (f(e10) - sel) + f(C_2350);
    let v = blended / f(ec);
    if v < 0.0 {
        0.0
    } else if v < f(C_ONE) {
        v as f32
    } else {
        1.0
    }
}

/// `0x677ff0` from `0x67833d` to `0x6783a8`: the lead direction `v` scaled by `m` (`[ebp-4]`).
/// `0x4549f0` and `fsqrt` give `len`; `fabs; fcomp 2⁻²²; test ah,5; jnp` keeps `v` as it is only
/// for `|len| < 2⁻²²`, and otherwise `inv = 1 / len` multiplies it (x and y stored as f32, z left
/// wide). Each is then multiplied by `m` and stored; the z product comes from the wide value.
#[must_use]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn snow_normalize_scale(v: [f32; 3], m: f32) -> [f32; 3] {
    let f = f64::from;
    let sumsq = sumsq3(f(v[0]), f(v[1]), f(v[2]));
    let len = sumsq.sqrt();
    let normalize = !(len.abs() < f(C_EPS_DEGEN));
    let (norm, live2): ([f32; 3], f64) = if normalize {
        let inv = f(C_ONE) / len;
        let n0 = (f(v[0]) * inv) as f32;
        let n1 = (f(v[1]) * inv) as f32;
        let live = f(v[2]) * inv;
        ([n0, n1, live as f32], live)
    } else {
        (v, f(v[2]))
    };
    [
        (f(norm[0]) * f(m)) as f32,
        (f(norm[1]) * f(m)) as f32,
        (live2 * f(m)) as f32,
    ]
}

/// `0x677ff0` from `0x678405` to `0x678417`: the argument of the exponential `0x454d40`, `(w *
/// -0.1) * (1/ln 2)` for the scaled z `w` (`[ebp-0x14]`), stored as a double with no rounding to
/// f32.
#[must_use]
pub fn snow_exp2_arg(w: f32) -> f64 {
    let f = f64::from;
    (f(w) * f(C_NEG_TENTH)) * f(C_INV_LN2)
}

/// `0x677ff0` from `0x67841f` to `0x678433`: `(e - lo) / (hi - lo)` wide, with `e` the exponential
/// the call leaves on st0 and `lo` and `hi` the globals `[0xc7b550]` and `[0xc7b56c]`. A clamp
/// follows at `0x678433`: `fcom 0.0; test ah,5; jp` replaces a value below 0 by 0, and `fcom 1.0;
/// jnp` keeps a value below 1 and replaces 1 or more, and NaN, by 1.
#[must_use]
pub fn snow_exp2_blend(e: f64, lo: f32, hi: f32) -> f64 {
    let f = f64::from;
    (e - f(lo)) / (f(hi) - f(lo))
}

/// `0x677ff0` from `0x67845f` to `0x67846a`: the eye height `e20 - (e18 - ec) * t`, `t` the clamped
/// blend left wide on st0, rounded at the `fstp [ebp-0x20]`.
#[must_use]
pub fn snow_field_lerp(t: f64, e18: f32, ec: f32, e20: f32) -> f32 {
    let f = f64::from;
    let span = (f(e18) - f(ec)) * t;
    (f(e20) - span) as f32
}

/// `0x677ff0` from `0x67846f` to `0x678496`: the frame quota `min(g, 1/60) * ea0`, with `g` the
/// frame time `[0xc62510]`: `fcomp; test ah,5; jp` takes `g` only below 1/60 (NaN takes the cap),
/// and the product is rounded at the `fst [ebp-0x10]`.
#[must_use]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn snow_min_select_scale(g: f32, ea0: f32) -> f32 {
    let f = f64::from;
    let sel = if g < C_MIN_SEL { g } else { C_MIN_SEL };
    (f(sel) * f(ea0)) as f32
}

/// `0x677ff0` from `0x67854e` to `0x678568`: the update interval `6144 / ea0`, rounded at the `fstp
/// dword [esp]` for `0x6745d0`.
#[must_use]
pub fn snow_final_recip(ea0: f32) -> f32 {
    let f = f64::from;
    (f(C_6144) / f(ea0)) as f32
}
