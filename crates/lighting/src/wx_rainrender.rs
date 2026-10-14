//! The rain patter geometry of `0x675ac0` Rain::DrawPatterFixed
//! (`systems/lighting/precipitation.md#rain-draw`): for the camera-facing quad of each patter, the
//! half-edge basis from the view matrix and the per-packet corners, centre and texture words. The
//! state setup, the camera fetch `0x672570`, the buffer calls and the tick-scale `0x674600` are
//! calls this arithmetic does not include.
//!
//! Values are f64 on the x87 stack and round to f32 at each `fstp dword`; the leaves `0x4549a0` (a
//! vector store) and `0x4549f0` (the squared length `(x² + y²) + z²`, left wide) run as their
//! arithmetic.

const C_ONE: f32 = f32::from_bits(0x3f80_0000);
const C_ZERO: f32 = f32::from_bits(0x0000_0000);
const C_HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x80e004]` = 1/12: the half width of a patter.
const C_1_12: f32 = f32::from_bits(0x3daa_aaab);
/// `[0x803568]` = 1/6: the height of a patter.
const C_1_6: f32 = f32::from_bits(0x3e2a_aaab);
const C_FOUR: f32 = f32::from_bits(0x4080_0000);
/// `[0x80ffd4]` = 3.99.
const C_399: f32 = f32::from_bits(0x407f_5c29);
const C_512: f32 = f32::from_bits(0x4400_0000);
/// `[0x8029b0]` = 0.25.
const C_QTR: f32 = f32::from_bits(0x3e80_0000);
/// `[0x802978]` = 0.125.
const C_EIGHTH: f32 = f32::from_bits(0x3e00_0000);
/// `[0x80ffd0]` = 0.04296875.
const C_TEXBIAS: f32 = f32::from_bits(0x3d30_0000);

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `0x675ac0` from `0x675bfc` to `0x675c9b`: the half-edge basis of a patter quad from the first
/// two rows `r0` and `r1` of the view matrix (read from `[ebp-0x11c]`, `[ebp-0x10c]`, `[ebp-0xfc]`
/// and `[ebp-0x118]`, `[ebp-0x108]`, `[ebp-0xf8]`). `right = r0 / 12` and `up = r1 / 6`, each
/// component rounded at its store, and `back` is the stored `right` negated.
#[must_use]
pub fn wx_quad_basis(r0: [f32; 3], r1: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let right = [
        (f(r0[0]) * f(C_1_12)) as f32,
        (f(r0[1]) * f(C_1_12)) as f32,
        (f(r0[2]) * f(C_1_12)) as f32,
    ];
    let up = [
        (f(r1[0]) * f(C_1_6)) as f32,
        (f(r1[1]) * f(C_1_6)) as f32,
        (f(r1[2]) * f(C_1_6)) as f32,
    ];
    let back = [-right[0], -right[1], -right[2]];
    (right, up, back)
}

/// The values one patter stores to the frame of `0x675ac0` between `0x675db3` and `0x6760c3`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadGeom {
    /// `[ebp-0x30]` to `[ebp-0x28]`: the patter position less the camera.
    pub delta: [f32; 3],
    /// `[ebp-0x134]` to `[ebp-0x12c]`: `delta` over its length.
    pub dir: [f32; 3],
    /// `[ebp-0x18]` to `[ebp-0x10]`: the centre of the quad.
    pub center: [f32; 3],
    /// `[ebp-0x13c]` and `[ebp-0x138]`: `up.y * c` and `up.z * c`; `up.x * c` is never stored.
    pub up_scaled: [f32; 2],
    /// `[ebp-0x1c]` and `[ebp-0x8]`: the two quantized words `bits(q) >> 14`.
    pub color: [u32; 2],
    /// `[ebp-0x70]` and the register left on st0: the low byte of each word times 0.25.
    pub tex: [f32; 2],
    /// `[ebp-0x7c]` to `[ebp-0x74]`: `back + center`.
    pub corner0: [f32; 3],
    /// `[ebp-0x5c]` to `[ebp-0x54]`: `up + center`.
    pub corner1: [f32; 3],
    /// `[ebp-0x68]` to `[ebp-0x60]`: `right + center`.
    pub corner2: [f32; 3],
    /// The texture words stored at `[ebp-0x98]`, `[ebp-0x94]`, `[ebp-0xa0]`, `[ebp-0x9c]`,
    /// `[ebp-0x8c]` and `[ebp-0x88]`.
    pub texword: [f32; 6],
}

/// `0x675ac0` from `0x675db3` to `0x6760c3`: the geometry of one patter at `packet_pos` for a
/// camera at `cam_pos`, with `cam_dist` (`[ebp-0x24]`) and `packet_far` (`[ebp-0xcc]`) and the
/// basis of [`wx_quad_basis`]. `delta = packet_pos - cam_pos` is stored; `dir = delta * (1 /
/// sqrt(sqlen))` with `inv` wide. `v = (-dir.y + -dir.x) * 0 + -dir.z` is compared with `fcomp 0.0;
/// test ah,0x41; jne`, which takes `V = 0` for `v <= 0` and NaN and otherwise `V = v` (wide). The
/// centre is `delta - up * c` with `c = V * 0.5`, where the x product stays wide and the y and z
/// ones are stored first, and z adds `V / 12`. The words are `bits(f32((1 - V) * 3.99 + 512)) >>
/// 14` and `bits(f32((cam_dist - packet_far) * 4 * 3.99 + 512)) >> 14`; their low bytes (`fild`)
/// times 0.25 give the texture bases, and the corners and texture words add the stored values.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn wx_quad_geom(
    packet_pos: [f32; 3],
    cam_pos: [f32; 3],
    cam_dist: f32,
    packet_far: f32,
    right: [f32; 3],
    up: [f32; 3],
    back: [f32; 3],
) -> QuadGeom {
    let delta = [
        (f(packet_pos[0]) - f(cam_pos[0])) as f32,
        (f(packet_pos[1]) - f(cam_pos[1])) as f32,
        (f(packet_pos[2]) - f(cam_pos[2])) as f32,
    ];
    let sqlen = f(delta[0]) * f(delta[0]) + f(delta[1]) * f(delta[1]) + f(delta[2]) * f(delta[2]);
    let inv = f(C_ONE) / sqlen.sqrt();
    let dir = [
        (f(delta[0]) * inv) as f32,
        (f(delta[1]) * inv) as f32,
        (f(delta[2]) * inv) as f32,
    ];
    let ndir = [-dir[0], -dir[1], -dir[2]];
    let v = ((f(ndir[1]) + f(ndir[0])) * f(C_ZERO) + f(ndir[2])) as f32;
    let vis: f32 = if v > 0.0 { v } else { C_ZERO };
    let vis64 = f(vis);
    let c = f(C_HALF) * vis64;
    let up_x_c = f(up[0]) * c;
    let up_y_c = f(up[1]) * c;
    let up_z_c = f(up[2]) * c;
    let up_scaled = [up_y_c as f32, up_z_c as f32];
    let center_x = (f(delta[0]) - up_x_c) as f32;
    let center_y = (f(delta[1]) - f(up_scaled[0])) as f32;
    let dz = f(delta[2]) - f(up_scaled[1]);
    let center_z = (dz + vis64 * f(C_1_12)) as f32;
    let center = [center_x, center_y, center_z];
    let q0 = ((f(C_ONE) - vis64) * f(C_399) + f(C_512)) as f32;
    let color0 = q0.to_bits() >> 14;
    let q1 = ((f(cam_dist) - f(packet_far)) * f(C_FOUR) * f(C_399) + f(C_512)) as f32;
    let color1 = q1.to_bits() >> 14;
    let th = (f64::from((color1 & 0xff) as i32) * f(C_QTR)) as f32;
    let tl = (f64::from((color0 & 0xff) as i32) * f(C_QTR)) as f32;
    let corner0 = [
        (f(back[0]) + f(center_x)) as f32,
        (f(back[1]) + f(center_y)) as f32,
        (f(back[2]) + f(center_z)) as f32,
    ];
    let corner1 = [
        (f(up[0]) + f(center_x)) as f32,
        (f(up[1]) + f(center_y)) as f32,
        (f(up[2]) + f(center_z)) as f32,
    ];
    let corner2 = [
        (f(right[0]) + f(center_x)) as f32,
        (f(right[1]) + f(center_y)) as f32,
        (f(right[2]) + f(center_z)) as f32,
    ];
    let qtl = (f(C_QTR) + f(tl)) as f32;
    let texword = [
        th,
        qtl,
        (f(th) + f(C_EIGHTH)) as f32,
        (f(tl) + f(C_TEXBIAS)) as f32,
        (f(th) + f(C_QTR)) as f32,
        qtl,
    ];
    QuadGeom {
        delta,
        dir,
        center,
        up_scaled,
        color: [color0, color1],
        tex: [th, tl],
        corner0,
        corner1,
        corner2,
        texword,
    }
}
