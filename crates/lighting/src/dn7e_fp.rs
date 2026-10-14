//! The sky drawing band `0x7e50a0..0x7e61b8`: the float arithmetic the glare, disc, cloud-strip and
//! star drawers run themselves, the rest of each being graphics-list calls.
//!
//! `0x7e50a0` and `0x7e5220` are the glare occlusion tests
//! (`systems/lighting/celestial.md#draw-occlusion`), `0x7e5670` and `0x7e57e0` the glare draw
//! (`#glare-draw`), `0x7e5b90` the disc draw (`#disc-draw`), `0x7e5f80` the cloud strip
//! (`systems/lighting/clouds.md#cloud-dome`) and `0x7e6120` the stars (`celestial.md#stars`).
//!
//! Operands are f32 widened to f64 (the harness runs the x87 at 53-bit precision); a value rounds
//! to f32 only where the client stores it to a dword slot, and a value never stored stays wide.

/// `[0x7ff9d8]` = 1.0: the normalize numerator (`fdivr` gives `1/len`) and the coverage clamp
/// bound.
pub const ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x7ffa24]` = 0.5: the `0x7e50a0` half-scale after the normalize.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x80c5c8]` = `0x3727c5ac` (about 1e-5): the `0x7e5670` limit on `|Vx*Vy|`, at or below which
/// the second axis falls back to (0, 1, 0).
pub const BASIS_EPS: f32 = f32::from_bits(0x3727_c5ac);
/// `[0x7ffe58]` = 255.0: the scale of the fixed-point colour-byte repack.
pub const C_255: f32 = f32::from_bits(0x437f_0000);
/// `[0x8029cc]` = 512.0: the bias of the fixed-point colour-byte repack.
pub const C_512: f32 = f32::from_bits(0x4400_0000);
/// `[0x8026c8]` = 1/255: the `0x7e6120` star counter scale.
pub const C_INV255: f32 = f32::from_bits(0x3b80_8081);
/// `[0x80fec4]` = `0x3ba3d70a` (0.005): subtracted from 1.0 for the glare depth range (`0x7e5220`
/// at `0x7e535f`, `0x7e57e0` at `0x7e59ed`).
pub const C_005: f32 = f32::from_bits(0x3ba3_d70a);

/// One camera-relative delta component `src - origin` of `0x7e57e0` (`fld src.z; fsub origin.z;
/// fstp [esp+8]`, and likewise y and x from `0x7e5943`), `0x7e5b90` (`0x7e5d0f`) and `0x7e5220`
/// (`0x7e54d1`): the f32 operands subtract wide and round at the store that feeds `0x4549a0`.
#[must_use]
pub fn vec3_sub_component(src: f32, origin: f32) -> f32 {
    (f64::from(src) - f64::from(origin)) as f32
}

/// The camera-relative delta vec3 `src - origin`, component-wise (see [`vec3_sub_component`]).
#[must_use]
pub fn vec3_sub(src: [f32; 3], origin: [f32; 3]) -> [f32; 3] {
    [
        vec3_sub_component(src[0], origin[0]),
        vec3_sub_component(src[1], origin[1]),
        vec3_sub_component(src[2], origin[2]),
    ]
}

/// `0x7e5670` BuildGlareBasis (`systems/lighting/celestial.md#glare-draw`), the whole function: the
/// basis `(V, U, W)` of an input direction. `V` is the direction normalized (`inv =
/// 1/sqrt((x²+y²)+z²)` kept wide and multiplied into each component, rounded at each store); `U` is
/// `(-Vy, Vx)` normalized by `0x456280` (z 0) when `|Vx*U.x|`, read with `fst` leaving `Vx` wide,
/// is above `[0x80c5c8]` (`fcomp; test ah,0x41; jne` falls back on at-or-below and on NaN) and `(0,
/// 1, 0)` otherwise; `W = V x U`.
#[must_use]
pub fn basis_from_normal(v_in: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let (vx, vy, vz) = (f64::from(v_in[0]), f64::from(v_in[1]), f64::from(v_in[2]));
    let len = (vx * vx + vy * vy + vz * vz).sqrt();
    let inv = f64::from(ONE) / len;
    let n_vx_live = inv * vx;
    let n_vx = n_vx_live as f32;
    let n_vy = (inv * vy) as f32;
    let n_vz = (inv * vz) as f32;
    let v = [n_vx, n_vy, n_vz];

    // `fst [ebx]` at 0x7e56e6 stores nVx but leaves it wide for the `fmul [edi]` that follows.
    let p = (n_vx_live * f64::from(-n_vy)).abs();
    let u = if p > f64::from(BASIS_EPS) {
        // 0x456280 normalizes (U.x, U.y) = (-nVy, nVx) in place.
        let ux = f64::from(-n_vy);
        let uy = f64::from(n_vx);
        let inv2 = f64::from(ONE) / (ux * ux + uy * uy).sqrt();
        [(inv2 * ux) as f32, (inv2 * uy) as f32, 0.0]
    } else {
        [0.0, ONE, 0.0]
    };

    let (ux, uy, uz) = (f64::from(u[0]), f64::from(u[1]), f64::from(u[2]));
    let (vxn, vyn, vzn) = (f64::from(v[0]), f64::from(v[1]), f64::from(v[2]));
    let w = [
        (vyn * uz - vzn * uy) as f32,
        (vzn * ux - vxn * uz) as f32,
        (uy * vxn - ux * vyn) as f32,
    ];
    (v, u, w)
}

/// `0x7e50a0` TraceGlareOcclusion, the normalize and half-scale from `0x7e50af` to `0x7e5120`
/// (`systems/lighting/celestial.md#draw-occlusion`): `out = normalize(src - origin) * 0.5`. The
/// delta components stay wide; `Dz` is also stored with `fst` and the z square is `Dz_wide *
/// Dz_f32`; the sum is `(Dz² + Dy²) + Dx²`; `1/len` is stored as f32; x and y reload their f32
/// normalized values for the half-scale while z keeps the wide `Dz_f32 * inv`.
#[must_use]
pub fn dn7e50a0_normalize_half(src: [f32; 3], origin: [f32; 3]) -> [f32; 3] {
    let dx = f64::from(src[0]) - f64::from(origin[0]);
    let dy = f64::from(src[1]) - f64::from(origin[1]);
    let dz = f64::from(src[2]) - f64::from(origin[2]);
    let dz_f32 = dz as f32;

    let len_sq = dz * f64::from(dz_f32) + dy * dy + dx * dx;
    let inv = (f64::from(ONE) / len_sq.sqrt()) as f32;
    let inv64 = f64::from(inv);

    let n_dx = (dx * inv64) as f32;
    let n_dy = (dy * inv64) as f32;
    let n_dz_live = f64::from(dz_f32) * inv64;

    let half = f64::from(HALF);
    [
        (f64::from(n_dx) * half) as f32,
        (f64::from(n_dy) * half) as f32,
        (n_dz_live * half) as f32,
    ]
}

/// `0x7e5220` QueryGlareOcclusion at `0x7e535f` (`systems/lighting/celestial.md#draw-occlusion`):
/// the glare depth range's lower bound, `1.0 - [0x80fec4]`, rounded at the store.
#[must_use]
pub fn dn7e5220_one_minus_c() -> f32 {
    (f64::from(ONE) - f64::from(C_005)) as f32
}

/// `0x7e5220` at `0x7e55b1`: the screen box area `|bx - ax| * |by - ay|`, each subtraction wide,
/// the product rounded at `fstp [ebp-0x2c]`.
#[must_use]
pub fn dn7e5220_bbox_area(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = (f64::from(bx) - f64::from(ax)).abs();
    let dy = (f64::from(by) - f64::from(ay)).abs();
    (dx * dy) as f32
}

/// `0x7e5220` from `0x7e5627`: the coverage of a visible-quad count over the box area. `fcomp 1.0;
/// test ah,0x41; jne` returns 0 for an area at or below 1 and for NaN; otherwise the ratio `count /
/// area` (`fild qword` of the u32 count, `fdiv`) is clamped by `fld 1.0; fcomp ratio; test ah,5;
/// jp` to at most 1. The result stays wide in st0.
#[must_use]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn dn7e5220_coverage(count: u32, area: f32) -> f64 {
    if !(area > ONE) {
        return 0.0;
    }
    let ratio = f64::from(count) / f64::from(area);
    if f64::from(ONE) < ratio {
        f64::from(ONE)
    } else {
        ratio
    }
}

/// `0x7e5f80` DrawClouds at `0x7e602d` (`systems/lighting/clouds.md#cloud-dome`): the strip's alpha
/// byte `bits(f32(g * 255 + 512)) >> 14` (low 8 bits) from the blend scale `g` at `[0xce99d0]`.
#[must_use]
pub fn strip_color_repack(g_scale: f32) -> u8 {
    let v = (f64::from(g_scale) * f64::from(C_255) + f64::from(C_512)) as f32;
    ((v.to_bits() >> 14) & 0xff) as u8
}

/// `0x7e6120` DrawStars at `0x7e6153` (`systems/lighting/celestial.md#stars`): the star alpha byte
/// as a fraction, `fild byte; fmul [0x8026c8]` (1/255), rounded at the push to `0x710cb0`.
#[must_use]
pub fn star_byte_norm(byte: u8) -> f32 {
    (f64::from(byte) * f64::from(C_INV255)) as f32
}
