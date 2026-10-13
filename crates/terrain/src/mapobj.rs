//! `BuildFootprintMatrix` (`0x69a2a0`), `CMapLight::UpdateBounds` (`0x6c0bb0`) and the size scale of
//! `AddSplash` (`0x68bda0`).

use benilla_re_cmath::transform::scale_matrix_rows;

/// Which quotient `0x69a2a0` scales by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AspectBranch {
    /// `a < b`: row 1 times `b/a` (`0x69a352`).
    ScaleY,
    /// `a > b`: row 0 times `a/b` (`0x69a372`).
    ScaleX,
    /// Equal or NaN: no scale.
    None,
}

/// What `0x69a2a0` builds before its rotation.
#[derive(Clone, Debug)]
pub struct LiquidTexMatrix {
    /// The matrix after the mirror and the aspect scale.
    pub matrix: [f32; 16],
    /// Which aspect scale applied.
    pub branch: AspectBranch,
    /// The angle negated (`fchs` at `0x69a397`), passed to `RotateMatrix` (`0x7bdd60`).
    pub neg_angle: f32,
}

/// `BuildFootprintMatrix` (`0x69a2a0`, ecx = the matrix, edx = the mirror flag, then the angle and
/// the pair `[a, b]`): the identity, rows scaled by `(−1, 1, 1)` when mirrored (`ScaleMatrixRows`,
/// `0x7bdca0`); then `fcomp` of a with b, `test ah,5; jp`: below scales row 1 by `f32(b/a)`, else a
/// second compare of b with a scales row 0 by `f32(a/b)` when b is below; equal and NaN scale
/// nothing. The rotation by the negated angle that follows is not modelled.
pub fn liquid_tex_matrix(mirror: bool, angle: f32, aspect: [f32; 2]) -> LiquidTexMatrix {
    let mut mat: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ];
    if mirror {
        scale_matrix_rows(&mut mat, [-1.0, 1.0, 1.0]);
    }
    let a = f64::from(aspect[0]);
    let b = f64::from(aspect[1]);
    let branch = if a < b {
        scale_matrix_rows(&mut mat, [1.0, (b / a) as f32, 1.0]);
        AspectBranch::ScaleY
    } else if b < a {
        scale_matrix_rows(&mut mat, [(a / b) as f32, 1.0, 1.0]);
        AspectBranch::ScaleX
    } else {
        AspectBranch::None
    };
    LiquidTexMatrix {
        matrix: mat,
        branch,
        neg_angle: -angle,
    }
}

/// What `0x6c0bb0` writes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightBounds {
    /// `+0xec`, the radius.
    pub radius: f32,
    /// `+0x24`, the position copied.
    pub center: [f32; 3],
    /// `+0x44`, the position less the radius.
    pub aabb_min: [f32; 3],
    /// `+0x50`, the position plus the radius.
    pub aabb_max: [f32; 3],
    /// `+0x5c`, the position copied.
    pub sphere_center: [f32; 3],
    /// `+0x68`, the radius.
    pub sphere_radius: f32,
}

/// `CMapLight::UpdateBounds` (`0x6c0bb0`): the radius `[+0xec]`, or 5.0 (`[0x80a1e8]`) when the
/// falloff `[+0xf0]` equals 0.0 (`test ah,0x44; jp` at `0x6c0bcd`; NaN keeps it), stored with `fst`
/// and kept in the register; the box is the position `[+0x88]` less and plus it, each stored as
/// `f32`.
pub fn light_bounds_refresh(radius_in: f32, falloff: f32, pos: [f32; 3]) -> LightBounds {
    let r_f32: f32 = if falloff == 0.0 { 5.0 } else { radius_in };
    let r = f64::from(r_f32);
    let p = pos.map(f64::from);
    LightBounds {
        radius: r_f32,
        center: pos,
        aabb_min: p.map(|c| (c - r) as f32),
        aabb_max: p.map(|c| (c + r) as f32),
        sphere_center: pos,
        sphere_radius: r_f32,
    }
}

/// The size scale of `0x68bda0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FalloffScale {
    /// `[ebp+0x1c]`: `f32(param·6.0)`, or 1.0.
    pub scale: f32,
    /// Whether the product was taken.
    pub scaled: bool,
}

/// `AddSplash` (`0x68bda0`) from `0x68be62` to `0x68be85`: `[ebp+0x1c]` is 1.0 unless `param` is
/// below 1/6 (`[0x803568]`, `0x3e2aaaab`; `test ah,5; jp`), when it is `f32(param·6.0)`
/// (`[0x802990]`, `0x68be7c`); equal and NaN keep 1.0.
pub fn falloff_scale(param: f32) -> FalloffScale {
    const ONE_SIXTH: f32 = f32::from_bits(0x3e2a_aaab);
    const SCALE6: f32 = 6.0;
    if param < ONE_SIXTH {
        FalloffScale {
            scale: (f64::from(param) * f64::from(SCALE6)) as f32,
            scaled: true,
        }
    } else {
        FalloffScale {
            scale: 1.0,
            scaled: false,
        }
    }
}
