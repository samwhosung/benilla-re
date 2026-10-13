//! The lightning bolts the lightning renderer draws (`systems/spell/visuals.md`): the bolt's points
//! (`0x7af6d0`), its update (`0x7af9b0`) and the strip it renders (`0x7afcb0`).
//!
//! The x87 runs at 53-bit precision, so an operation rounds to `f64` and an `fstp dword` store to
//! `f32`: an `f64` operation with a final `as f32`.

#![forbid(unsafe_code)]

use benilla_re_random::crandom_uint32;

/// 0.0 (`0x7ffd74`).
pub const ZERO: f32 = 0.0;
/// 1.0 (`0x7ff9d8`).
pub const ONE: f32 = 1.0;
/// 0.5 (`0x7ffa24`).
pub const HALF: f32 = 0.5;
/// 2.0 (`0x801628`).
const TWO: f32 = 2.0;
/// 0.001 (`0x801360`): the least length the strip's side is normalized above.
pub const NORM_EPS: f32 = 0.001;
/// 0.25 (`0x8029b0`) and 0.75 (`0x8012cc`): the weights of the update's blend.
pub const BLEND_REF_W: f64 = 0.25;
pub const BLEND_OLD_W: f64 = 0.75;
/// The global `CRandom` the bolt's points draw from.
pub const LIGHTNING_PRNG_VA: u32 = 0x00cf_57b0;

/// `__ftol` (`0x40a2b0`): towards zero, the low dword; NaN, an infinity or a value outside `i64`
/// gives 0.
fn ftol(v: f64) -> i32 {
    let t = v.trunc();
    if (-9_223_372_036_854_775_808.0_f64..9_223_372_036_854_775_808.0_f64).contains(&t) {
        t as i64 as i32
    } else {
        0
    }
}

/// A bolt's ends `+0x0`, `+0xc`, its spacing `+0x18` and its amplitude `+0x24`.
#[derive(Clone, Copy, Debug, Default)]
pub struct LightningBolt {
    pub p0: [f32; 3],
    pub p1: [f32; 3],
    pub spacing: f32,
    pub amplitude: f32,
}

/// `S` of a draw: `2 − x` for a negative word, else `x − 2`, with `x` the draw in `[1, 2)`.
fn signed_unit(raw: u32) -> f32 {
    let u = f32::from_bits((raw & 0x007f_ffff) | 0x3f80_0000);
    if (raw as i32) < 0 {
        2.0 - u
    } else {
        u - 2.0
    }
}

/// `0x7af6d0` (ecx = the bolt; the point array): `d = p1 − p0` stored as `f32`, `len = √((x² +
/// y²) + z²)` in the register, `n = ftol(len/spacing + 2.0)`, `1/n` (unsigned `fild qword`) and
/// `len·amplitude` stored as `f32`; `n + 1` points from `p0` to `p1`, each inner point `i` at
/// `f32(S·amp) + f32(f32(f32(d·i)·(1/n)) + p0)` with `S` from three draws of `state` (accumulator,
/// lanes) in x, y, z order.
pub fn lightning_subdivide(
    bolt: &LightningBolt,
    state: &mut (u32, u32),
    table: &[u8],
) -> Vec<[f32; 3]> {
    let seg = [0, 1, 2].map(|k| (f64::from(bolt.p1[k]) - f64::from(bolt.p0[k])) as f32);
    let [sx, sy, sz] = seg.map(f64::from);
    let len = ((sx * sx + sy * sy) + sz * sz).sqrt();
    let n = ftol(len / f64::from(bolt.spacing) + f64::from(TWO));
    let inv_n = (f64::from(ONE) / f64::from(n as u32)) as f32;
    let amp = (len * f64::from(bolt.amplitude)) as f32;
    let count = (n + 1).max(1) as usize;
    let mut pts = vec![[0.0f32; 3]; count];
    pts[0] = bolt.p0;
    pts[count - 1] = bolt.p1;
    for (i, pt) in pts.iter_mut().enumerate().take(count - 1).skip(1) {
        let pert = [0; 3].map(|_| {
            let (acc, lanes, eax) = crandom_uint32(state.0, state.1, table);
            *state = (acc, lanes);
            signed_unit(eax)
        });
        let fi = f64::from(i as u32);
        *pt = [0, 1, 2].map(|k| {
            let temp1 = (f64::from(seg[k]) * fi) as f32;
            let temp2 = (f64::from(temp1) * f64::from(inv_n)) as f32;
            let base = (f64::from(temp2) + f64::from(bolt.p0[k])) as f32;
            let scaled = (f64::from(pert[k]) * f64::from(amp)) as f32;
            (f64::from(scaled) + f64::from(base)) as f32
        });
    }
    pts
}

/// `0x7af9c7..0x7af9e8`: the age `fmod(dt + age, period)` (`0x73f90a`) when the period `+0x28`
/// is not equal to 0 (`test ah,0x44; jnp`, so NaN takes the `fmod`), else 0.
pub fn lightning_age(elapsed: f32, age_prev: f32, period: f32) -> f32 {
    if period == ZERO {
        return 0.0;
    }
    ((f64::from(elapsed) + f64::from(age_prev)) % f64::from(period)) as f32
}

/// `0x7afa17..0x7afa26`: `1/(2·n − 2)` from a `fild qword`, stored as `f32`.
pub fn lightning_step_fraction(segs: i32) -> f32 {
    let denom = i64::from(segs).wrapping_mul(2).wrapping_sub(2);
    (f64::from(ONE) / (denom as f64)) as f32
}

/// `0x7afb1d..0x7afb2f`: `i·step`, stored as `f32`.
pub fn lightning_param_pos(i: i64, step: f32) -> f32 {
    (i as f64 * f64::from(step)) as f32
}

/// One vertex of the update's blend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlendVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// `0x7afc0d..0x7afc81`: `0.75·vertex + 0.25·reference` (`0xcf585c`): x both products in the
/// register, y the vertex product stored as `f32`, z both stored as `f32`.
pub fn lightning_blend(vert: [f32; 3], reference: [f32; 3]) -> BlendVertex {
    let a = reference.map(|c| f64::from(c) * BLEND_REF_W);
    let b = vert.map(|c| f64::from(c) * BLEND_OLD_W);
    let r = |v: f64| f64::from(v as f32);
    BlendVertex {
        x: (b[0] + a[0]) as f32,
        y: (r(b[1]) + a[1]) as f32,
        z: (r(b[2]) + r(a[2])) as f32,
    }
}

/// A point through the working matrix `0xcf5800` (`0x7bca80`).
pub type Point = [f32; 3];

/// `0x7b0191..0x7b01dd`: the side `(−(cur.y − prev.y), cur.x − prev.x, 0)`, each stored as `f32`,
/// scaled by `1/len` in the register when `len = √((x² + y²) + z²)` compares above 0.001 (`test
/// ah,0x41; jne`, so 0.001 and NaN stay).
pub fn normalize_perp(prev: Point, cur: Point) -> [f32; 3] {
    let dx = (f64::from(cur[0]) - f64::from(prev[0])) as f32;
    let dy = (f64::from(cur[1]) - f64::from(prev[1])) as f32;
    let p = [(-f64::from(dy)) as f32, dx, ZERO];
    let [px, py, pz] = p.map(f64::from);
    let len = ((px * px + py * py) + pz * pz).sqrt();
    if f64::from(NORM_EPS) < len {
        let recip = f64::from(ONE) / len;
        return p.map(|c| (f64::from(c) * recip) as f32);
    }
    p
}

/// `0x7b01e2..0x7b04b3`: with `o = side·width` (`+0x1c`) stored as `f32`, the four vertices
/// `(o + prev)·0.5`, `(prev − o)·0.5`, `(o + cur)·0.5` and `(cur − o)·0.5`, each sum stored as
/// `f32` before the half; the first two are added to the previous record, the last two stored.
pub fn strip_offsets(
    prev: Point,
    cur: Point,
    perp: [f32; 3],
    width: f32,
) -> ([f32; 3], [f32; 3], [f32; 3], [f32; 3]) {
    let o = perp.map(|c| (f64::from(c) * f64::from(width)) as f32);
    let half = f64::from(HALF);
    let plus = |p: Point| {
        [0, 1, 2].map(|k| (f64::from((f64::from(o[k]) + f64::from(p[k])) as f32) * half) as f32)
    };
    let minus = |p: Point| {
        [0, 1, 2].map(|k| (f64::from((f64::from(p[k]) - f64::from(o[k])) as f32) * half) as f32)
    };
    (plus(prev), minus(prev), plus(cur), minus(cur))
}

/// `0x7b0544..0x7b0579`: `−(+0x60 / d)` with `d` the period `+0x28`, or 1.0 when it equals 0
/// (`test ah,0x44; jnp`).
pub fn bolt_scalar(denom: f32, numer: f32) -> f32 {
    let d = if denom == 0.0 { 1.0 } else { f64::from(denom) };
    (-(f64::from(numer) / d)) as f32
}
