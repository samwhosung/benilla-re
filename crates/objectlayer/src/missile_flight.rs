//! The missile's flight math: `CMissile::Aim` (`0x61d230`), `CMissile::Advance` (`0x61e2a0`) and
//! the fishing-line vertices of `DrawFishingLine` (`0x61f780`) (`systems/objectlayer/missiles.md#
//! aiming-and-leading` and `#flight-each-frame`, `effects.md#the-fishing-line`).
//!
//! The x87 runs at 53 bits, so an operation is an `f64` operation, and a value is rounded to `f32`
//! only at the `fst dword` or `fstp dword` the code executes; a value kept on the stack stays
//! `f64`. A compare reads the live register. The vector helpers `0x4549a0` (store three floats),
//! `0x4549c0` (scale by `1/len`) and `0x4549f0` (the squared length, `(x*x + y*y) + z*z`) are
//! inlined, and the rotation `0x7be490` is `benilla_re_cmath::rotation::axis_angle_mat3`. The
//! `fsin` and `fsincos` use the host's math, as the emulator does.

use crate::missile::missile_scale_vec;
use crate::x87::ftol;
use benilla_re_cmath::rotation::axis_angle_mat3;

const ONE: f64 = 1.0; // 0x7ff9d8
const HALF: f64 = 0.5; // 0x7ffa24
/// `2^-22` (`0x8029d4`), the length floor of a normalize.
const EPS: f32 = f32::from_bits(0x3480_0000);
/// `4.0` (`0x80306c`), the discriminant's factor.
const FOUR: f64 = 4.0;
/// `9.99e-6` (`0x80c5c8`), the floor of `a * a` that guards the root divide.
const A_FLOOR: f32 = f32::from_bits(0x3727_c5ac);
/// `FLT_MAX` (`0x80c858`), the root when no positive root exists.
const FLT_MAX_SENTINEL: f32 = f32::from_bits(0x7f7f_ffff);
/// `0.75` (`0x8012cc`), the fraction of the target's height the aim point rises by.
const INTERCEPT_Z_FRAC: f64 = 0.75;
/// `2.0` (`0x801628`), the centre of the arc's random float.
const TWO: f64 = 2.0;
/// `pi` (`0x80c7d0`), the arc's sine frequency.
const ARC_PI: f32 = f32::from_bits(0x4049_0fdb);
/// `0.05236` (`0x80c850`), the arc's sine amplitude.
const ARC_AMP: f32 = f32::from_bits(0x3d56_7750);
/// `1/64` (`0x80a92c`), the line's parameter step.
const INV64: f32 = f32::from_bits(0x3c80_0000);
/// `pi` (`0x80c8e8`), the line's sag frequency.
const BEAM_PI: f32 = f32::from_bits(0x4049_0fdb);
/// `-0.5` (`0x80c9a8`), the line's sag amplitude.
const BEAM_AMP: f32 = f32::from_bits(0xbf00_0000);
/// `255.0` (`0x7ffe58`), the colour channel scale.
const C255: f64 = 255.0;

/// The outcome of one `CMissile::Advance` tick. The function returns 1 either way, so the
/// observable is what becomes of the position `+0x20`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AdvanceStep {
    /// `speed == 0`: it returns at once and the position stays (`0x61e2ac`).
    Stalled,
    /// `step^2 > |d|^2` with `flags & 2` clear: the arrival handler `0x61e1d0` runs and the
    /// position is not integrated (`0x61e3fd`).
    Arrived,
    /// The new position `P + normalize(dest - P) * (dt * speed)` (`0x61e422`).
    Moved([f32; 3]),
}

/// The position integrate of `0x61e2a0`: `P += normalize(dest - P) * step`, `step = dt * speed`.
/// `flags` is the byte at `+0x38`; only bit 1 (suppress arrival) reaches this math.
///
/// - `0x61e2ac`: `fcomp 0.0; test ah,0x44; jnp` returns when `speed == 0`; NaN does not.
/// - `step` is `dt * speed` stored as `f32` (`0x61e3ae`); `d` is `dest - P` stored per component.
/// - `0x61e3d2`: `|d|^2 = (dy^2 + dz^2) + dx^2` against `step^2` by `fcompp; test ah,0x41; jne`:
///   arrival only when `step^2 > |d|^2` is ordered and `flags & 2` is clear.
/// - `0x61e437`: `len = sqrt((dx^2 + dy^2) + dz^2)`, stored as `f32` with the register kept. `fabs;
///   fcomp eps; test ah,5; jnp` skips the divide only for an ordered `len < eps` of the live
///   register; otherwise `0x4549c0` scales `d` by `1 / len_f32`.
/// - `0x61e45c`: each `unit * step` is stored, then added to `P` and stored.
pub fn missile_advance(
    cur: [f32; 3],
    dest: [f32; 3],
    speed: f32,
    dt: f32,
    flags: u8,
) -> AdvanceStep {
    if speed == 0.0 {
        return AdvanceStep::Stalled;
    }
    let step = (f64::from(dt) * f64::from(speed)) as f32;

    let dx = (f64::from(dest[0]) - f64::from(cur[0])) as f32;
    let dy = (f64::from(dest[1]) - f64::from(cur[1])) as f32;
    let dz = (f64::from(dest[2]) - f64::from(cur[2])) as f32;

    let dsq_arrival = ((f64::from(dy) * f64::from(dy)) + (f64::from(dz) * f64::from(dz)))
        + (f64::from(dx) * f64::from(dx));
    let step_sq = f64::from(step) * f64::from(step);
    if step_sq > dsq_arrival && (flags & 0x2) == 0 {
        return AdvanceStep::Arrived;
    }

    let lensq = ((f64::from(dx) * f64::from(dx)) + (f64::from(dy) * f64::from(dy)))
        + (f64::from(dz) * f64::from(dz));
    let len_live = lensq.sqrt();
    let len = len_live as f32;
    let (ux, uy, uz) = if len_live >= f64::from(EPS) || len_live.is_nan() {
        let factor = ONE / f64::from(len);
        (
            (f64::from(dx) * factor) as f32,
            (f64::from(dy) * factor) as f32,
            (f64::from(dz) * factor) as f32,
        )
    } else {
        (dx, dy, dz)
    };

    let mvx = (f64::from(ux) * f64::from(step)) as f32;
    let mvy = (f64::from(uy) * f64::from(step)) as f32;
    let mvz = (f64::from(uz) * f64::from(step)) as f32;
    AdvanceStep::Moved([
        (f64::from(cur[0]) + f64::from(mvx)) as f32,
        (f64::from(cur[1]) + f64::from(mvy)) as f32,
        (f64::from(cur[2]) + f64::from(mvz)) as f32,
    ])
}

/// The target point `0x61d2a7` builds: the position read, with `z = tgt90 * tgt_bc * 0.75 + raw_z`
/// stored as `f32`.
fn intercept_target(raw_pos: [f32; 3], tgt90: f32, tgt_bc: f32) -> [f32; 3] {
    let tz = (((f64::from(tgt90) * f64::from(tgt_bc)) * INTERCEPT_Z_FRAC) + f64::from(raw_pos[2]))
        as f32;
    [raw_pos[0], raw_pos[1], tz]
}

/// `CMissile::Aim` (`0x61d230`) with `+0x38` bit 3 clear: the quadratic lead. It writes the aim
/// point at `+0x2c` and this returns it.
///
/// The target moves when bit 3 of `[[tgt+8]+8]` is set (`0x61d4cc`); otherwise the aim is the
/// target point `T`. The velocity is `0x61eec0` of the target's velocity object. With `s` the
/// missile speed and `d = T - P`:
/// - `s^2 < |V|^2` (`fcompp; test ah,5; jp`, so equal and NaN skip it) normalizes `V` as above and
///   scales it by `s`;
/// - `V' = V * 0.5`; `a = |V'|^2 - s^2` stored `f32`; `b = 2 * dot(V', d)` kept on the stack and
///   stored `f32`; `disc = b * b_f32 - |d|^2 * a * 4` kept and stored `f32`;
/// - `disc < 0` (`test ah,5; jnp`) or `a * a < 9.99e-6` writes `T`;
/// - the roots are `(-b_f32 - q) * h` and `(q - b_f32) * h` with `q = sqrt(disc_f32)` and `h = 0.5
///   / a`, each stored `f32`; the best is the smallest root with `0 < root < FLT_MAX`;
/// - `fcom FLT_MAX; test ah,0x44; jp` writes `T` only when `best` is still `FLT_MAX`; otherwise it
///   writes `T + V' * best`, where x and y store the product first and z does not.
#[allow(clippy::too_many_arguments)]
pub fn missile_target_intercept_quadratic(
    p: [f32; 3],
    speed: f32,
    raw_pos: [f32; 3],
    tgt90: f32,
    tgt_bc: f32,
    vel_dir: [f32; 3],
    vel_scalar: f32,
    moving: bool,
) -> [f32; 3] {
    let t = intercept_target(raw_pos, tgt90, tgt_bc);
    if !moving {
        return t;
    }

    let dx = (f64::from(t[0]) - f64::from(p[0])) as f32;
    let dy = (f64::from(t[1]) - f64::from(p[1])) as f32;
    let dz = (f64::from(t[2]) - f64::from(p[2])) as f32;

    let v = missile_scale_vec(vel_scalar, vel_dir);
    let (mut vx, mut vy, mut vz) = (v[0], v[1], v[2]);

    let s = f64::from(speed);
    let s2 = s * s;
    // `|V|^2` is summed `(Vz^2 + Vy^2) + Vx^2` here and `(Vx^2 + Vy^2) + Vz^2` for the length.
    let vlen2_clamp = ((f64::from(vz) * f64::from(vz)) + (f64::from(vy) * f64::from(vy)))
        + (f64::from(vx) * f64::from(vx));
    if s2 < vlen2_clamp {
        let lensq = ((f64::from(vx) * f64::from(vx)) + (f64::from(vy) * f64::from(vy)))
            + (f64::from(vz) * f64::from(vz));
        let len_live = lensq.sqrt();
        let len = len_live as f32;
        if len_live >= f64::from(EPS) || len_live.is_nan() {
            let factor = ONE / f64::from(len);
            vx = (f64::from(vx) * factor) as f32;
            vy = (f64::from(vy) * factor) as f32;
            vz = (f64::from(vz) * factor) as f32;
        }
        vx = (f64::from(vx) * s) as f32;
        vy = (f64::from(vy) * s) as f32;
        vz = (f64::from(vz) * s) as f32;
    }

    vx = (f64::from(vx) * HALF) as f32;
    vy = (f64::from(vy) * HALF) as f32;
    vz = (f64::from(vz) * HALF) as f32;

    let vplen2 = ((f64::from(vz) * f64::from(vz)) + (f64::from(vy) * f64::from(vy)))
        + (f64::from(vx) * f64::from(vx));
    let a = (vplen2 - s2) as f32;

    let dot = ((f64::from(vz) * f64::from(dz)) + (f64::from(vy) * f64::from(dy)))
        + (f64::from(dx) * f64::from(vx));
    let b_full = dot + dot;
    let b_f32 = b_full as f32;

    let b_sq = b_full * f64::from(b_f32);
    let dsq = ((f64::from(dz) * f64::from(dz)) + (f64::from(dy) * f64::from(dy)))
        + (f64::from(dx) * f64::from(dx));
    let disc_full = b_sq - ((dsq * f64::from(a)) * FOUR);
    let disc_f32 = disc_full as f32;

    if disc_full < 0.0 {
        return t;
    }
    if (f64::from(a) * f64::from(a)) < f64::from(A_FLOOR) {
        return t;
    }

    let q = f64::from(disc_f32).sqrt();
    let h = HALF / f64::from(a);
    let root1 = ((-f64::from(b_f32) - q) * h) as f32;
    let root2 = ((q - f64::from(b_f32)) * h) as f32;

    let mut best = FLT_MAX_SENTINEL;
    if root1 > 0.0 && root1 < FLT_MAX_SENTINEL {
        best = root1;
    }
    if root2 > 0.0 && root2 < best {
        best = root2;
    }

    if best == FLT_MAX_SENTINEL {
        return t;
    }

    let best = f64::from(best);
    [
        ((f64::from(vx) * best) as f32 as f64 + f64::from(t[0])) as f32,
        ((f64::from(vy) * best) as f32 as f64 + f64::from(t[1])) as f32,
        ((best * f64::from(vz)) + f64::from(t[2])) as f32,
    ]
}

/// The arc's angle from the generator's return `rv` (`0x61d3d5` to `0x61d423`): `f` is the float in
/// `[1, 2)` of `rv`'s low 23 bits, `base` is `2 - f` for a negative `rv` (signed) and `f - 2`
/// otherwise, and the angle is `sin(base * (pi * 0.5)) * amp` stored as `f32`.
pub fn curved_arc_angle(rv: u32) -> f32 {
    let f = f32::from_bits((rv & 0x007f_ffff) | 0x3f80_0000);
    let base = if (rv as i32) < 0 {
        TWO - f64::from(f)
    } else {
        f64::from(f) - TWO
    };
    let arg = base * (f64::from(ARC_PI) * HALF);
    (arg.sin() * f64::from(ARC_AMP)) as f32
}

/// `CMissile::Aim` (`0x61d230`) with `+0x38` bit 3 set: the random arc. It returns the point it
/// writes at `+0x2c`, or `None` at the two early returns (`len < eps`, and `|nz - 1| < eps` for the
/// normalized z). `rv` is the `0x4531e0` return.
///
/// - `d = T - P`; `len = sqrt((dx^2 + dy^2) + dz^2)` stored `f32` with the register kept, and
///   `fabs; fcomp eps; test ah,5; jnp` returns on an ordered `len < eps` of the live register.
/// - `r = 1 / len_f32`; `nx` and `ny` are stored, `nz = r * dz` stays on the stack.
/// - `axis = n * (speed * 0.5 + len_f32)`, stored per component.
/// - The axis of the turn is `(-(dx*dz), -(dy*dz), dx^2 - (-dy)*dy)` from the unnormalized `d`;
///   `theta` is [`curved_arc_angle`]; the matrix is `0x7be490` and the axis is rotated by its
///   columns; `P` is added back.
pub fn missile_target_intercept_curved(
    p: [f32; 3],
    speed: f32,
    raw_pos: [f32; 3],
    tgt90: f32,
    tgt_bc: f32,
    rv: u32,
) -> Option<[f32; 3]> {
    let t = intercept_target(raw_pos, tgt90, tgt_bc);
    let dx = (f64::from(t[0]) - f64::from(p[0])) as f32;
    let dy = (f64::from(t[1]) - f64::from(p[1])) as f32;
    let dz = (f64::from(t[2]) - f64::from(p[2])) as f32;

    let lensq = ((f64::from(dx) * f64::from(dx)) + (f64::from(dy) * f64::from(dy)))
        + (f64::from(dz) * f64::from(dz));
    let len_live = lensq.sqrt();
    let len = len_live as f32;
    if len_live < f64::from(EPS) {
        return None;
    }

    let r = ONE / f64::from(len);
    let nx = (f64::from(dx) * r) as f32;
    let ny = (f64::from(dy) * r) as f32;
    let nz = r * f64::from(dz);

    if (nz - ONE).abs() < f64::from(EPS) {
        return None;
    }

    let scl = (f64::from(speed) * HALF) + f64::from(len);
    let axis = [
        (f64::from(nx) * scl) as f32,
        (f64::from(ny) * scl) as f32,
        (scl * nz) as f32,
    ];

    let perp = [
        (-(f64::from(dx) * f64::from(dz))) as f32,
        (-(f64::from(dy) * f64::from(dz))) as f32,
        ((f64::from(dx) * f64::from(dx)) - (-f64::from(dy) * f64::from(dy))) as f32,
    ];

    let theta = curved_arc_angle(rv);
    let m = axis_angle_mat3(&perp, theta, false);

    let rotx = ((f64::from(axis[0]) * f64::from(m[0]))
        + (f64::from(axis[1]) * f64::from(m[3]))
        + (f64::from(axis[2]) * f64::from(m[6]))) as f32;
    let roty = ((f64::from(axis[0]) * f64::from(m[1]))
        + (f64::from(axis[1]) * f64::from(m[4]))
        + (f64::from(axis[2]) * f64::from(m[7]))) as f32;
    let rotz = ((f64::from(axis[0]) * f64::from(m[2]))
        + (f64::from(axis[1]) * f64::from(m[5]))
        + (f64::from(axis[2]) * f64::from(m[8]))) as f32;

    Some([
        (f64::from(rotx) + f64::from(p[0])) as f32,
        (f64::from(roty) + f64::from(p[1])) as f32,
        (f64::from(rotz) + f64::from(p[2])) as f32,
    ])
}

/// The colour word of the fishing line (`0x61f82e` to `0x61f936`), the same for all 65 vertices.
/// `chan` is the descriptor's `(+0x54, +0x58, +0x5c)`, red, green and blue. Each channel is `0` for
/// `c <= 0` (`fcomp 0.0; test ah,0x41; jp`), `255` for `c >= 1`, and otherwise `c * 255 + 0.5`, all
/// kept on the stack, then truncated by `__ftol` to a byte; NaN falls to the product and gives 0.
/// The word is `0xff << 24 | R << 16 | G << 8 | B`, and with `swap` (the `[+0x1c] == 1` flag of
/// `0x58a230`) the red and blue swap.
pub fn beam_color_word(chan: [f32; 3], swap: bool) -> u32 {
    fn quant(c: f32) -> u32 {
        let v: f64 = if c <= 0.0 {
            0.0
        } else if c >= 1.0 {
            C255
        } else {
            f64::from(c) * C255 + HALF
        };
        (ftol(v) as u32) & 0xff
    }
    let r = quant(chan[0]);
    let g = quant(chan[1]);
    let b = quant(chan[2]);
    if swap {
        0xff00_0000 | (b << 16) | (g << 8) | r
    } else {
        0xff00_0000 | (r << 16) | (g << 8) | b
    }
}

/// One line vertex at a stride of 16: the position at `+0` and the colour at `+0xc`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamVertex {
    pub pos: [f32; 3],
    pub color: u32,
}

/// The 65 vertices of the fishing line from `p0` to `p1` (`0x61f7ea` to `0x61f9ab`). The end's z is
/// `p1.z + scale * tgt_bc * 0.5` stored as `f32`, and `delta = p1 - p0` is stored per component.
/// For vertex `i`, `t = i / 64`:
/// - `x = d.x * t + p0.x` and `y = d.y * t + p0.y`, with the product kept;
/// - `z = (float(d.z * t) + p0.z)` stored, then `sin(t * pi) * -0.5` added and stored again.
pub fn effect_beam_trail(
    p0: [f32; 3],
    p1_raw: [f32; 3],
    scale: f32,
    tgt_bc: f32,
    chan: [f32; 3],
    swap: bool,
) -> Vec<BeamVertex> {
    let p1z = (((f64::from(scale) * f64::from(tgt_bc)) * HALF) + f64::from(p1_raw[2])) as f32;
    let dx = (f64::from(p1_raw[0]) - f64::from(p0[0])) as f32;
    let dy = (f64::from(p1_raw[1]) - f64::from(p0[1])) as f32;
    let dz = (f64::from(p1z) - f64::from(p0[2])) as f32;

    let color = beam_color_word(chan, swap);

    let mut out = Vec::with_capacity(65);
    for i in 0..65i64 {
        let t = i as f64 * f64::from(INV64);
        let vx = (f64::from(dx) * t + f64::from(p0[0])) as f32;
        let vy = (f64::from(dy) * t + f64::from(p0[1])) as f32;
        let dzt = (f64::from(dz) * t) as f32;
        let z0 = (f64::from(dzt) + f64::from(p0[2])) as f32;
        let perturb = (t * f64::from(BEAM_PI)).sin() * f64::from(BEAM_AMP);
        let vz = (perturb + f64::from(z0)) as f32;
        out.push(BeamVertex {
            pos: [vx, vy, vz],
            color,
        });
    }
    out
}
