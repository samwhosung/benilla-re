//! The per-drop quad geometry of `0x676160` Rain::DrawDropsFixed
//! (`systems/lighting/precipitation.md#rain-draw`), the draw without shaders: for each drop inside
//! its fade window, the vector from the camera, the two ends of the streak, a normal basis, the
//! corner positions and the texture transform. The state setup, the camera fetch `0x672570`, the
//! tilt `0x674a70`, the speed gate `0x674bf0`, `0x674600` and the buffer calls are calls this
//! arithmetic does not include; the vector store `0x4549a0` runs as a plain store.
//!
//! Values are f64 on the x87 stack and round to f32 at each store; a value left on the stack stays
//! wide.

/// `[0x7ff9d8]` = 1.0: the numerator of each normalize.
const C_ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x80ff78]` = `0x3d4ccccd` (0.05): the half width of a streak.
const C_005: f32 = f32::from_bits(0x3d4c_cccd);
/// `[0x80ff74]` = 2.0: the length of the projected normal.
const C_2: f32 = f32::from_bits(0x4000_0000);

/// A stored f32 widened for the x87.
#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// A wide value rounded at a `fstp dword`.
#[inline]
fn nar(x: f64) -> f32 {
    x as f32
}

/// The values one drop stores to its frame and vertex buffer between `0x676478` and `0x676932`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropQuad {
    /// `delta = dropPos − cam` → `[ebp-0x58/-0x54/-0x50]`.
    pub delta: [f32; 3],
    /// `span = min(t, f3edge) − f2edge` (the LIVE-st0 corner scale; observed narrowed for the
    /// diff).
    pub span: f32,
    /// `span2 = min(t, tmp) − f2edge` → `[ebp-0x4]` (`tmp = −(wind/drop0) + f3edge`).
    pub span2: f32,
    /// `vertexA = span·dropV + delta` → `[ebp-0x48/-0x44/-0x40]`.
    pub vertex_a: [f32; 3],
    /// `vertexB = span2·dropV + delta` → `[ebp-0x114/-0x110/-0x10c]`.
    pub vertex_b: [f32; 3],
    /// `nA = normalize(−vertexA)` (in place at `[ebp-0x24/-0x20/-0x1c]`).
    pub n_a: [f32; 3],
    /// `nF = normalize(−dropV)` (in place at `[ebp-0x14/-0x10/-0xc]`).
    pub n_f: [f32; 3],
    /// `cr = normalize(nF × nA)` (in place at `[ebp-0x30/-0x2c/-0x28]`).
    pub cr: [f32; 3],
    /// corner-vertex 0 = `(-0.05)·cr + vertexA` → `[ebp-0x108/-0x104/-0x100]` (vbuf `[esi]`).
    pub corner0: [f32; 3],
    /// corner-vertex 1 = `(0.05)·cr + vertexA` → `[ebp-0xcc/-0xc8/-0xc4]` (vbuf `[esi+0x14]`).
    pub corner1: [f32; 3],
    /// `proj = 2.0·nF` → `[ebp-0xfc/-0xf8/-0xf4]` (vbuf `[esi+0x28]`).
    pub proj: [f32; 3],
    /// the per-corner matrix·proj texcoord/color transform → `[ebp-0x144/-0x140/-0x13c]`.
    pub matvec: [f32; 3],
    /// the final three corner-component adds stored back to the third vbuf vertex: `[esi] = 0.5 +
    /// matvec.x`, `[esi-0x10] = vertexB.y + matvec.y`, `[esi-0xc] = vertexB.z + matvec.z`.
    pub finalv: [f32; 3],
}

/// `0x676160` from `0x676478` to `0x676932`: the quad of one drop. `delta = drop_pos - cam` is
/// stored. With `tmp = -(wind / drop_v.z) + f3edge` stored, the drop is skipped (`emit` false)
/// when `t < f2edge` (`fcomp; test ah,5; jnp`) or `t > tmp` (`test ah,0x41; je`); NaN draws it. The
/// span is `min(t, f3edge) - f2edge` wide (`fcomp; test ah,5; jp` takes `t` only when `t < f3edge`)
/// and `span2` is the same with `tmp`, stored. Vertex A is `span * drop_v + delta` with each
/// product stored, vertex B the same with `span2`. The normals are `normalize(-A)` and
/// `normalize(-drop_v)` (each component stored after the multiply by the wide `1 / sqrt((x² + z²) +
/// y²)`, so a zero vector gives NaN), and `cr = normalize(nF x nA)`. The corners are `cr * -0.05 +
/// A` and `cr * 0.05 + A`, `proj = 2 * nF`, and each row of `mat` dotted with `proj` as `(z + y) +
/// x` is added to B.
#[allow(clippy::neg_cmp_op_on_partial_ord, clippy::too_many_arguments)]
pub fn rain_drop_quad(
    drop_pos: [f32; 3],
    drop_v: [f32; 3],
    f2edge: f32,
    f3edge: f32,
    cam: [f32; 3],
    wind: f32,
    t: f32,
    mat: [[f32; 3]; 3],
) -> (bool, DropQuad) {
    let delta = [
        nar(f(drop_pos[0]) - f(cam[0])),
        nar(f(drop_pos[1]) - f(cam[1])),
        nar(f(drop_pos[2]) - f(cam[2])),
    ];

    let tmp = nar(-(f(wind) / f(drop_v[2])) + f(f3edge));
    let emit_lo = !(t < f2edge);
    let emit_hi = !(t > tmp);
    let emit = emit_lo && emit_hi;
    let m1 = if !(t < f3edge) { f3edge } else { t };
    let span_live = f(m1) - f(f2edge);
    let span = nar(span_live);
    let m2 = if !(t < tmp) { tmp } else { t };
    let span2 = nar(f(m2) - f(f2edge));

    if !emit {
        return (
            false,
            DropQuad {
                delta,
                span,
                span2,
                vertex_a: [0.0; 3],
                vertex_b: [0.0; 3],
                n_a: [0.0; 3],
                n_f: [0.0; 3],
                cr: [0.0; 3],
                corner0: [0.0; 3],
                corner1: [0.0; 3],
                proj: [0.0; 3],
                matvec: [0.0; 3],
                finalv: [0.0; 3],
            },
        );
    }

    let sv = [
        nar(span_live * f(drop_v[0])),
        nar(span_live * f(drop_v[1])),
        nar(span_live * f(drop_v[2])),
    ];
    let vertex_a = [
        nar(f(sv[0]) + f(delta[0])),
        nar(f(sv[1]) + f(delta[1])),
        nar(f(sv[2]) + f(delta[2])),
    ];

    let sv2 = [
        nar(f(span2) * f(drop_v[0])),
        nar(f(span2) * f(drop_v[1])),
        nar(f(span2) * f(drop_v[2])),
    ];
    let vertex_b = [
        nar(f(sv2[0]) + f(delta[0])),
        nar(f(sv2[1]) + f(delta[1])),
        nar(f(sv2[2]) + f(delta[2])),
    ];

    let neg_a = [
        nar(-f(vertex_a[0])),
        nar(-f(vertex_a[1])),
        nar(-f(vertex_a[2])),
    ];
    let n_a = normalize_inplace(neg_a);

    let neg_f = [nar(-f(drop_v[0])), nar(-f(drop_v[1])), nar(-f(drop_v[2]))];
    let n_f = normalize_inplace(neg_f);

    let cross = [
        nar(f(n_f[2]) * f(n_a[1]) - f(n_f[1]) * f(n_a[2])),
        nar(f(n_a[2]) * f(n_f[0]) - f(n_f[2]) * f(n_a[0])),
        nar(f(n_f[1]) * f(n_a[0]) - f(n_a[1]) * f(n_f[0])),
    ];
    let cr = normalize_inplace(cross);

    let neg005 = f(-C_005);
    let sc0 = [
        nar(f(cr[0]) * neg005),
        nar(neg005 * f(cr[1])),
        nar(neg005 * f(cr[2])),
    ];
    let corner0 = [
        nar(f(sc0[0]) + f(vertex_a[0])),
        nar(f(sc0[1]) + f(vertex_a[1])),
        nar(f(sc0[2]) + f(vertex_a[2])),
    ];
    let pos005 = f(C_005);
    let sc1 = [
        nar(f(cr[0]) * pos005),
        nar(pos005 * f(cr[1])),
        nar(pos005 * f(cr[2])),
    ];
    let corner1 = [
        nar(f(sc1[0]) + f(vertex_a[0])),
        nar(f(sc1[1]) + f(vertex_a[1])),
        nar(f(sc1[2]) + f(vertex_a[2])),
    ];
    let two = f(C_2);
    let proj = [
        nar(f(n_f[0]) * two),
        nar(two * f(n_f[1])),
        nar(two * f(n_f[2])),
    ];

    let matvec = [
        matvec_row(mat[2], proj),
        matvec_row(mat[1], proj),
        matvec_row(mat[0], proj),
    ];

    let finalv = [
        nar(f(vertex_b[0]) + f(matvec[0])),
        nar(f(vertex_b[1]) + f(matvec[1])),
        nar(f(vertex_b[2]) + f(matvec[2])),
    ];

    (
        true,
        DropQuad {
            delta,
            span,
            span2,
            vertex_a,
            vertex_b,
            n_a,
            n_f,
            cr,
            corner0,
            corner1,
            proj,
            matvec,
            finalv,
        },
    )
}

/// Normalize in place as the exe does: `len² = (x² + z²) + y²`, `inv = 1 / sqrt(len²)` wide, each
/// component times `inv` and stored.
#[inline]
fn normalize_inplace(v: [f32; 3]) -> [f32; 3] {
    let len2 = (f(v[0]) * f(v[0]) + f(v[2]) * f(v[2])) + f(v[1]) * f(v[1]);
    let inv = f(C_ONE) / len2.sqrt();
    [nar(f(v[0]) * inv), nar(f(v[1]) * inv), nar(f(v[2]) * inv)]
}

/// One row of the texture transform dotted with `proj`, `(row.z * proj.z + row.y * proj.y) + row.x
/// * proj.x`, rounded at the vertex store.
#[inline]
fn matvec_row(mat_row: [f32; 3], proj: [f32; 3]) -> f32 {
    let acc = f(mat_row[2]) * f(proj[2]) + f(mat_row[1]) * f(proj[1]);
    nar(acc + f(mat_row[0]) * f(proj[0]))
}
