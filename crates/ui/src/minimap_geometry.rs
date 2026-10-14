//! The minimap's arithmetic (`systems/ui/minimap.md`, `systems/ui/minimapblips.md`): the icon
//! lattice, the blip projection and quad, the scale about an anchor, and the ping position.
//!
//! The x87 runs at 53-bit precision, so a chain is `f64` and each `fst`/`fstp dword` is an
//! `as f32`.

/// The two fixed tables `LoadMinimapTextures` fills (in `.bss`, `0xbc7978` and `0xbc7898`). Each
/// row is 8 `f32`; the first table is always 64 rows, the second always 5.
#[derive(Clone, Debug, PartialEq)]
pub struct IconLattice {
    /// `0xbc7978`: 64 rows set by the divisor `n` (`[0xbc7644]`).
    pub table1: [[f32; 8]; 64],
    /// `0xbc7898`: 5 rows that do not depend on `n`.
    pub table2: [[f32; 8]; 5],
}

/// The two table-filling loops of `LoadMinimapTextures` (`0x4ed810`, `minimap#minimap-textures`);
/// the texture loads and the mesh setup (`0x4ef7a0`, `0x58a980`, `0x449d90`, `0x419e30`) are not
/// part of them. `n` is the divisor read from `[0xbc7644]`, used as the integer divisor and, through
/// `fild`, as the float one; it must be at least 1, as 0 faults the integer divide.
///
/// Loop 1 (`0x4ed8d4` to `0x4ed933`, stride `0x20`): for `i` in 0..64, `r = i / n` and `c = i % n`
/// (integers), and, each stored as `f32`, `c/n`, `(r+1)/n`, `(c+1)/n` and `r/n` (`fild` then `fdiv`
/// by `n`). The row is `[c/n, (r+1)/n, (c+1)/n, (r+1)/n, c/n, r/n, (c+1)/n, r/n]`.
///
/// Loop 2 (`0x4ed940` to `0x4ed9b1`, stride `0x20`): for `j` in 0..5, `k4 = j & 3` and `k2 = j >> 2`,
/// with the multiplier 0.25 (`0x8029b0`): `a = k4 * 0.25`, `b = (k2+1) * 0.25`, `cc = (k4+1) * 0.25`
/// and `d = k2 * 0.25`, each stored as `f32`. The row is `[a, b, cc, b, a, d, cc, d]`.
pub fn minimap_icon_lattice(n: u32) -> IconLattice {
    let nf = f64::from(n);
    let mut table1 = [[0.0f32; 8]; 64];
    for (i, row) in table1.iter_mut().enumerate() {
        let i = i as u32;
        let r = i / n;
        let c = i % n;
        let v_c_n = (f64::from(c) / nf) as f32;
        let v_r1_n = (f64::from(r + 1) / nf) as f32;
        let v_c1_n = (f64::from(c + 1) / nf) as f32;
        let v_r_n = (f64::from(r) / nf) as f32;
        *row = [v_c_n, v_r1_n, v_c1_n, v_r1_n, v_c_n, v_r_n, v_c1_n, v_r_n];
    }
    let mut table2 = [[0.0f32; 8]; 5];
    for (j, row) in table2.iter_mut().enumerate() {
        let j = j as u32;
        let k4 = j & 3;
        let k2 = j >> 2;
        let a = (f64::from(k4) * 0.25) as f32;
        let b = (f64::from(k2 + 1) * 0.25) as f32;
        let cc = (f64::from(k4 + 1) * 0.25) as f32;
        let d = (f64::from(k2) * 0.25) as f32;
        *row = [a, b, cc, b, a, d, cc, d];
    }
    IconLattice { table1, table2 }
}

/// `0x4ead90` (a `MinimapFrame.cpp` function): twice the distance of each of four values from an
/// anchor, `[2 * (p0 - B), 2 * (p1 - A), 2 * (p2 - B), 2 * (p3 - A)]`. The anchor pair is read from
/// the table at `0xbc7810` (stride `0x18`) by index: `A` at `0xbc7810 + 0x18 * index` and `B` at
/// `0xbc7814 + 0x18 * index`.
///
/// The binary stores the difference as `f32` before it doubles it for components 0 and 3, and keeps
/// it at register precision for 1 and 2 (the `anchor_b` copy at `[ebp-8]` that component 2 reads
/// equals `anchor_b`, which is an `f32`). The output cannot tell the two apart: a difference of two
/// `f32` is a multiple of 2^-149, so it is exact in the subnormal range, and a doubling commutes
/// with round-to-nearest-even in the normal range, so `round(2 * round(d)) == round(2 * d)` for
/// every finite pair. The code keeps the stores as the binary has them.
pub fn scale_about_anchor(anchor_a: f32, anchor_b: f32, params: [f32; 4]) -> [f32; 4] {
    let a = f64::from(anchor_a);
    let b = f64::from(anchor_b);
    let [p0, p1, p2, p3] = params.map(f64::from);
    let d0 = (p0 - b) as f32 as f64;
    let d3 = (p3 - a) as f32 as f64;
    let d1 = p1 - a;
    let d2 = p2 - b;
    [
        (2.0 * d0) as f32,
        (2.0 * d1) as f32,
        (2.0 * d2) as f32,
        (2.0 * d3) as f32,
    ]
}

/// `ProjectBlip` (`0x4eaa30`, `minimapblips#the-north-up-disc`): the blip's offset from the disc
/// centre, `out.x = k - (k/R) * dB` and `out.y = k + (k/R) * dA` for the half width `k`, in two
/// `f32` stores. `a8`, `ac` are the player's coordinates and `a18`, `a1c` the entity's, `depth` is
/// the disc radius `R`, and `size` and `scale` give `k = size * scale * 0.5`.
///
/// The x87 stack decides where it rounds: `dx = a18 - a8` and `dy = a1c - ac` at register
/// precision; `f = size * scale * 0.5` stays at register precision in `st0` and is spilled as an
/// `f32` copy `f_f32`, which the two final sums reload; `fdx = (f * dx) as f32` (`fstp [ebp+8]`);
/// `fdy = f * dy` stays at register precision; `inv = 1 / depth`; `(fdx * inv) as f32` is stored
/// and `fdy * inv` is not. `out[0] = (f_f32 - fdy * inv) as f32` and
/// `out[1] = (f_f32 + (fdx * inv) as f32) as f32`. `ecx` is the out pointer and the function
/// returns with `ret 0x20`.
pub fn project_blip(
    a8: f32,
    ac: f32,
    depth: f32,
    a18: f32,
    a1c: f32,
    size: f32,
    scale: f32,
) -> [f32; 2] {
    let dx = f64::from(a18) - f64::from(a8);
    let dy = f64::from(a1c) - f64::from(ac);
    let f_full = f64::from(size) * f64::from(scale) * 0.5;
    let f_f32 = f64::from(f_full as f32);
    let fdx = f64::from((f_full * dx) as f32);
    let fdy = f_full * dy;
    let inv = 1.0 / f64::from(depth);
    let fdx_inv = f64::from((fdx * inv) as f32);
    let fdy_inv = fdy * inv;
    [(f_f32 - fdy_inv) as f32, (f_f32 + fdx_inv) as f32]
}

/// `0x4eb910` (a `MinimapFrame.cpp` function): the quad of a blip, its corner positions, its
/// scale-about-anchor basis and its corner UVs. A `__thiscall` with seven stack arguments
/// (`ret 0x1c`, the one at `+0x10` unused) that reads no field of `this` and writes 28 `f32` at
/// `this+0x0` to `this+0x6c`: the vertex positions (`+0x0` to `+0x2c`), the basis (`+0x30` to
/// `+0x4c`) and the UVs (`+0x50` to `+0x6c`). `0x4ef370` copies four floats and relays the corner
/// deltas, and `0x4ead90` ([`scale_about_anchor`]) is called for the basis.
///
/// `p` is the source vector `*arg0`, `c0` and `c1` the corner pair `*arg4`, `half` is `arg3`, and
/// `k1` and `k2` are the arguments at `+0x1c` and `+0x20`, whose product is the scale; `anchor_a`
/// and `anchor_b` are the anchor pair for `0x4ead90`.
///
/// ```text
/// two_half = (half + half) as f32          ; fadd st, st ; fstp [m32]
/// inv      = 1.0 / two_half                ; register precision, over the stored f32
/// d0 = (p0 - c0) as f32   d1 = (p1 - c1) as f32   d2 = (p2 - c0) as f32   d3 = (p3 - c1) as f32
/// ```
///
/// The UVs are the eight `(d_i * inv) as f32` in the order `d1, d2, d3, d2, d1, d0, d3, d0`. The
/// basis is the result `[E0, E1, E2, E3]` of `0x4ead90` in the order `E1, E2, E3, E2, E1, E0, E3,
/// E0`. The positions are four `(x, y, 0)` vertices, with `K = k1 * k2` at register precision and
/// `fk = K as f32`:
///
/// ```text
/// H1    = (fk * f32(d1*inv)) as f32           x of vertices 0 and 2
/// H3    = (fk * f32(d3*inv)) as f32           x of vertices 1 and 3
/// fk_h2 = (fk - fk * f32(d2*inv)) as f32      y of vertices 0 and 1 (the product at register precision)
/// fk_g0 = (fk - (d0*inv) * K) as f32          y of vertices 2 and 3 (both factors at register precision)
/// ```
///
/// Both `d0 * inv` as `f32` (in the UVs) and `d0 * inv` at register precision (in `fk_g0`) are
/// used. Returns the 28 values indexed by the offset from `this` divided by 4.
#[allow(clippy::too_many_arguments)]
pub fn blip_quad(
    p: [f32; 4],
    c0: f32,
    c1: f32,
    half: f32,
    k1: f32,
    k2: f32,
    anchor_a: f32,
    anchor_b: f32,
) -> [f32; 28] {
    let two_half = (f64::from(half) + f64::from(half)) as f32;
    let inv = 1.0 / f64::from(two_half);

    let d0 = (f64::from(p[0]) - f64::from(c0)) as f32;
    let d1 = (f64::from(p[1]) - f64::from(c1)) as f32;
    let d2 = (f64::from(p[2]) - f64::from(c0)) as f32;
    let d3 = (f64::from(p[3]) - f64::from(c1)) as f32;

    let d0inv = (f64::from(d0) * inv) as f32;
    let d1inv = (f64::from(d1) * inv) as f32;
    let d2inv = (f64::from(d2) * inv) as f32;
    let d3inv = (f64::from(d3) * inv) as f32;
    let d0inv_full = f64::from(d0) * inv;

    let e = scale_about_anchor(anchor_a, anchor_b, p);

    let k_full = f64::from(k1) * f64::from(k2);
    let fk = k_full as f32;

    let h1 = (f64::from(fk) * f64::from(d1inv)) as f32;
    let h3 = (f64::from(fk) * f64::from(d3inv)) as f32;
    let h2_full = f64::from(fk) * f64::from(d2inv);
    let fk_h2 = (f64::from(fk) - h2_full) as f32;
    let g0_full = d0inv_full * k_full;
    let fk_g0 = (f64::from(fk) - g0_full) as f32;

    [
        // positions +0x0..+0x2c
        h1, fk_h2, 0.0, h3, fk_h2, 0.0, h1, fk_g0, 0.0, h3, fk_g0, 0.0,
        // basis +0x30..+0x4c
        e[1], e[2], e[3], e[2], e[1], e[0], e[3], e[0], // UVs +0x50..+0x6c
        d1inv, d2inv, d3inv, d2inv, d1inv, d0inv, d3inv, d0inv,
    ]
}

/// `Minimap::GetPingPosition` (`0x4eefd0`, `minimap#reading-the-ping`): the two numbers it pushes
/// with `lua_pushnumber` (`0x6f3810`), from the stored ping `(center_x, center_y)` (`0xbc787c`,
/// `0xbc7880`), the player's position and the zoom radius (`0x6da9b0`).
///
/// `tmp_x = (center_x - pos_x) as f32` (`0x4ef035`) and `tmp_y = (center_y - pos_y) as f32`
/// (`0x4ef041`); `scale = 1 / (radius + radius)` (`fadd st, st; fdivr 1.0`). The first value,
/// `-(scale * tmp_y)`, is left in `st0` and pushed as a double with no `f32` store; the second,
/// `tmp_x * scale`, is stored as `f32` (`fstp [ebp-8]`, `0x4ef05b`) and reloaded. Returns
/// `(first, second)`.
pub fn ping_position(
    center_x: f32,
    center_y: f32,
    pos_x: f32,
    pos_y: f32,
    radius: f32,
) -> (f64, f64) {
    let tmp_x = center_x - pos_x;
    let tmp_y = center_y - pos_y;
    let scale = 1.0_f64 / (2.0 * f64::from(radius));
    let result0 = -(scale * f64::from(tmp_y));
    let result1_f32 = (f64::from(tmp_x) * scale) as f32;
    (result0, f64::from(result1_f32))
}
