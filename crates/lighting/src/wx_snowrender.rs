//! Arithmetic of `0x678960` Snow::DrawFixed (`systems/lighting/precipitation.md#snow`), the draw of
//! the started snow packets on the processor, without shaders: the corner basis from the view
//! matrix, the fade window and age of a flake, its position, the alpha word and the three corner
//! positions. The state setup, the buffer calls, the eye getter `0x672570`, the ramp `0x67ad60`,
//! `0x67ad90` and the vector store `0x4549a0` are calls this arithmetic does not include.
//!
//! Values are f64 on the x87 stack and round to f32 at each `fstp dword`; a value left on the stack
//! stays wide until its own store.

/// `[0x7ff9d8]` = 1.0: the numerator of each normalize.
const C_ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x80e004]` = 1/12: the size of a basis vector.
const C_INV12: f32 = f32::from_bits(0x3daa_aaab);
/// `[0x8029b0]` = 0.25: added to the upper edge of the fade window.
const C_QUARTER: f32 = f32::from_bits(0x3e80_0000);
/// `[0x7ffe58]` = 255.0.
const C_255: f32 = f32::from_bits(0x437f_0000);
/// `[0x8029cc]` = 512.0.
const C_512: f32 = f32::from_bits(0x4400_0000);

/// The three corner offsets of a flake's triangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnowBasis {
    /// `[ebp-0x54]` to `[ebp-0x4c]`: `normalize(U) / 12`.
    pub a: [f32; 3],
    /// `[ebp-0x48]` to `[ebp-0x40]`: `normalize(R - U) / 12`.
    pub b: [f32; 3],
    /// `[ebp-0x3c]` to `[ebp-0x34]`: `normalize(-R - U) / 12`.
    pub c: [f32; 3],
}

/// `0x678960` from `0x678adf` to `0x678cbe`: the corner offsets from the view matrix columns `R`
/// (`[ebp-0xc]`) and `U` (`[ebp-0x1c]`). `D1 = R - U` and `D3 = -R - U` are stored as f32. Each of
/// `U`, `D1` and `D3` is divided by its length, `1 / sqrt(len²)` wide, with `len²` summed in the
/// loads' order, `(x² + z²) + y²` for `U` and `(x² + y²) + z²` for `D1` and `D3`; the products are
/// stored as f32 except `U.x * inv`, which stays on the stack into its multiply by 1/12.
#[must_use]
pub fn snow_basis(r: [f32; 3], u: [f32; 3]) -> SnowBasis {
    let f = f64::from;
    let d1 = [
        (f(r[0]) - f(u[0])) as f32,
        (f(r[1]) - f(u[1])) as f32,
        (f(r[2]) - f(u[2])) as f32,
    ];
    let d3 = [
        (f(-r[0]) - f(u[0])) as f32,
        (f(-r[1]) - f(u[1])) as f32,
        (f(-r[2]) - f(u[2])) as f32,
    ];
    let len_u2 = (f(u[0]) * f(u[0]) + f(u[2]) * f(u[2])) + f(u[1]) * f(u[1]);
    let inv_u = f(C_ONE) / len_u2.sqrt();
    let a0_live = f(u[0]) * inv_u;
    let a1_f32 = (f(u[1]) * inv_u) as f32;
    let a2_f32 = (f(u[2]) * inv_u) as f32;
    let len_d12 = (f(d1[0]) * f(d1[0]) + f(d1[1]) * f(d1[1])) + f(d1[2]) * f(d1[2]);
    let inv_d1 = f(C_ONE) / len_d12.sqrt();
    let b_norm = [
        (f(d1[0]) * inv_d1) as f32,
        (f(d1[1]) * inv_d1) as f32,
        (f(d1[2]) * inv_d1) as f32,
    ];
    let len_d32 = (f(d3[0]) * f(d3[0]) + f(d3[1]) * f(d3[1])) + f(d3[2]) * f(d3[2]);
    let inv_d3 = f(C_ONE) / len_d32.sqrt();
    let c_norm = [
        (f(d3[0]) * inv_d3) as f32,
        (f(d3[1]) * inv_d3) as f32,
        (f(d3[2]) * inv_d3) as f32,
    ];
    let a = [
        (a0_live * f(C_INV12)) as f32,
        (f(a1_f32) * f(C_INV12)) as f32,
        (f(a2_f32) * f(C_INV12)) as f32,
    ];
    let b = [
        (f(b_norm[0]) * f(C_INV12)) as f32,
        (f(b_norm[1]) * f(C_INV12)) as f32,
        (f(b_norm[2]) * f(C_INV12)) as f32,
    ];
    let c = [
        (f(c_norm[0]) * f(C_INV12)) as f32,
        (f(c_norm[1]) * f(C_INV12)) as f32,
        (f(c_norm[2]) * f(C_INV12)) as f32,
    ];
    SnowBasis { a, b, c }
}

/// `0x678960` from `0x678de9` to `0x678e2c`: whether a flake is drawn, and its age. `t` is the
/// flake's time (`[ebp-0x30]`), `f1` and `f2` the packet's edges. `fcomp; test ah,5; jnp` skips the
/// flake for `t < f1` and for `t > f2 + 0.25`, the sum compared wide; NaN draws it. The age is
/// `min(t, f2) - f1` (`fcomp; jp` takes `t` only when `t < f2`), rounded at the store.
#[must_use]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn snow_flake_gate(t: f32, f1: f32, f2: f32) -> (bool, f32) {
    let f = f64::from;
    let emit_lo = !(t < f1);
    let emit_hi = !(f(f2) + f(C_QUARTER) < f(t));
    let emit = emit_lo && emit_hi;
    let m = if !(t < f2) { f2 } else { t };
    let span_top = (f(m) - f(f1)) as f32;
    (emit, span_top)
}

/// `0x678960` from `0x678e4f` to `0x678e8e`: the position of a flake, `span_top * vel + origin` per
/// axis. The x and y products stay wide into the add; the z product is stored as f32
/// (`[ebp-0x104]`) first.
#[must_use]
pub fn snow_flake_center(span_top: f32, vel: [f32; 3], origin: [f32; 3]) -> [f32; 3] {
    let f = f64::from;
    let p0 = f(span_top) * f(vel[0]);
    let p1 = f(span_top) * f(vel[1]);
    let p2 = (f(span_top) * f(vel[2])) as f32;
    [
        (p0 + f(origin[0])) as f32,
        (p1 + f(origin[1])) as f32,
        (f(p2) + f(origin[2])) as f32,
    ]
}

/// `0x678960` from `0x678ecd` to `0x678ee1`: the alpha word, `alpha * 255 + 512` wide and stored as
/// f32, whose bits `>> 14` form the colour's alpha byte.
#[must_use]
pub fn snow_alpha_pack(alpha_clamped: f32) -> f32 {
    let f = f64::from;
    (f(alpha_clamped) * f(C_255) + f(C_512)) as f32
}

/// `0x678960` from `0x678ee4` to `0x678fcf`: the three corner positions `(center - eye) + offset`
/// for the offsets `a`, `b`, `c` of [`snow_basis`]. The x and y differences stay wide across the
/// three corners; the z difference is stored as f32 and reloaded for each.
#[must_use]
pub fn snow_tri_vertices(center: [f32; 3], eye: [f32; 3], basis: SnowBasis) -> [[f32; 3]; 3] {
    let f = f64::from;
    let dx = f(center[0]) - f(eye[0]);
    let dy = f(center[1]) - f(eye[1]);
    let dz = (f(center[2]) - f(eye[2])) as f32;
    let corner = |off: [f32; 3]| {
        [
            (dx + f(off[0])) as f32,
            (dy + f(off[1])) as f32,
            (f(dz) + f(off[2])) as f32,
        ]
    };
    [corner(basis.a), corner(basis.b), corner(basis.c)]
}
