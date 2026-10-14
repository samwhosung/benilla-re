//! The float arithmetic of `CMissile::ArriveMiss` (`0x61dd50`), 64 operations, all in the
//! deflection branch `[0x61df13, 0x61e158)` (`systems/objectlayer/missiles.md#misses-resists-and-
//! reflections`). With `arg1 == 0`, bit `0x40` of the missile flags `[esi+0x38]` set and `0x8`
//! clear, a miss deflects the missile: it negates and normalizes the incoming travel direction,
//! builds two random axis-angle rotations (`CRandom::uint32` `0x4531e0` and `0x7be490`, whose
//! results are inputs here), turns the direction by them, halves and damps the speed, and moves the
//! missile's position. The other branches of the function do no float arithmetic. The regions are:
//!
//! | region | addresses           | computes                                         | function |
//! |--------|---------------------|--------------------------------------------------|----------|
//! | R1     | 0x61df13-0x61df48   | the negated direction and its length             | [`deflect_length`] |
//! | R2     | 0x61df48-0x61df76   | the two degenerate gates, read as `test ah,5`    | [`deflect_len_degenerate`], [`deflect_axis_degenerate`] |
//! | R3     | 0x61df76-0x61df9a   | the normalize                                    | [`deflect_normalize`] |
//! | R4, R6 | 0x61df9a, 0x61e050  | a random angle                                   | [`deflect_random_angle`] |
//! | R5     | 0x61dfe9-0x61e050   | the second axis from the first matrix            | [`deflect_axis_from_mat`] |
//! | R7, R8 | 0x61e0a2-0x61e158   | the turned direction, the speed and the position | [`deflect_transform_and_move`] |
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`. A value
//! left on the stack is not rounded and an `fst` leaves its register live: R2's first gate reads the
//! live length, and R8's position update keeps `pos.y`'s step live while it rounds `pos.x` and
//! `pos.z`.

/// `_DAT_008029d4` (`0x34800000`): 2^-22, the length below which the deflection stops, for the
/// direction and for the distance of `|nz|` from it.
pub const EPS_2EM22: f32 = f32::from_bits(0x3480_0000);
/// `_DAT_007ff9d8` (`0x3f800000`): 1.0, the dividend of `1.0 / len`.
pub const ONE: f32 = 1.0;
/// `_DAT_00801628` (`0x40000000`): 2.0, in the random angle `±(2.0 - r)` or `±(r - 2.0)` for the
/// float `r` in `[1, 2)`.
pub const TWO: f32 = 2.0;
/// `_DAT_0080c7d0` (`0x40490fdb`): π, in the random angle `base·(π·0.5)`.
pub const PI: f32 = f32::from_bits(0x4049_0fdb);
/// `_DAT_007ffa24` (`0x3f000000`): 0.5, the half of the random angle and the halving of the speed.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `_DAT_007ffd74` (`0x00000000`): 0.0, the third component of the second axis, `M·0.0`.
pub const ZERO: f32 = 0.0;
/// `_DAT_0080c85c` (`0x3f333334`): 0.7, the damping of the halved speed.
pub const SPEED_DAMP_0P7: f32 = f32::from_bits(0x3f33_3334);

/// R1 (`0x61df13` to `0x61df45`): the direction at `[ebx+0xc]` is negated and each component stored
/// to a dword by `fchs; fstp`. The length is `sqrt((ny² + nx²) + nz²)`, and `fst dword [ebp-0x18]`
/// rounds it to `f32` while the register stays live. Returns `(nx, ny, nz, len)`.
pub fn deflect_length(dir: [f32; 3]) -> (f32, f32, f32, f32) {
    let nx = (-f64::from(dir[0])) as f32;
    let ny = (-f64::from(dir[1])) as f32;
    let nz = (-f64::from(dir[2])) as f32;
    let len2 = (f64::from(ny) * f64::from(ny) + f64::from(nx) * f64::from(nx))
        + f64::from(nz) * f64::from(nz);
    let len = len2.sqrt() as f32;
    (nx, ny, nz, len)
}

/// R2 gate 1 (`0x61df48` to `0x61df55`): `fabs; fcomp 2^-22; test ah,5; jnp 0x61e1b9` aborts only
/// for an odd count, `|len| < 2^-22`. The `fabs` reads the register `fst` left, the 53-bit length
/// and not the stored `f32`, so this takes the negated components and recomputes it. NaN goes on.
pub fn deflect_len_degenerate(n: [f32; 3]) -> bool {
    let len2 = (f64::from(n[1]) * f64::from(n[1]) + f64::from(n[0]) * f64::from(n[0]))
        + f64::from(n[2]) * f64::from(n[2]);
    let a = len2.sqrt().abs();
    !a.is_nan() && a < f64::from(EPS_2EM22)
}

/// R2 gate 2 (`0x61df5b` to `0x61df70`): `fld nz; fabs; fsubr [ebp-0x18]` is `len - |nz|` on the
/// stored `f32` length, `fabs; fcomp 2^-22; test ah,5; jnp` aborts for a difference below 2^-22, a
/// direction along the z axis. NaN goes on.
pub fn deflect_axis_degenerate(len: f32, nz: f32) -> bool {
    let d = (f64::from(len) - f64::from(nz).abs()).abs();
    !d.is_nan() && d < f64::from(EPS_2EM22)
}

/// R3 (`0x61df76` to `0x61df97`): `inv = 1.0 / len` stays on the stack and each component is
/// `fmul`'d and stored back as a dword.
pub fn deflect_normalize(n: [f32; 3], len: f32) -> [f32; 3] {
    let inv = f64::from(ONE) / f64::from(len);
    [
        (f64::from(n[0]) * inv) as f32,
        (f64::from(n[1]) * inv) as f32,
        (f64::from(n[2]) * inv) as f32,
    ]
}

/// R4 and R6 (`0x61df9a` to `0x61dfdd`, `0x61e050` to `0x61e096`): `CRandom::uint32` gives `r =
/// bits((rand & 0x7fffff) | 0x3f800000)` in `[1, 2)`. A negative `rand` (`test eax,eax; jge`) takes
/// `base = 2.0 - r` and the others `r - 2.0`; the angle is `base·(π·0.5)` with both factors left on
/// the stack, stored as a dword for `0x7be490`.
pub fn deflect_random_angle(rand_u32: u32) -> f32 {
    let r = f32::from_bits((rand_u32 & 0x007f_ffff) | 0x3f80_0000);
    let base = if (rand_u32 as i32) < 0 {
        f64::from(TWO) - f64::from(r)
    } else {
        f64::from(r) - f64::from(TWO)
    };
    let half_pi = f64::from(PI) * f64::from(HALF);
    (base * half_pi) as f32
}

/// R5 (`0x61dfe9` to `0x61e04d`): the axis of the second rotation is the first matrix `m1` (written
/// at `[ebp-0x50]`) applied to `(ny', -nx', 0)`. The three stores are `(m0·V0 + m3·V1) + m6·0.0` to
/// `[ebp-0x2c]`, `(m1·V0 + m4·V1) + m7·0.0` to `[ebp-0x28]` and `(m2·V0 + m5·V1) + m8·0.0` to
/// `[ebp-0x24]`; the products by 0.0 keep a non-finite matrix visible. Returns the three in the
/// order `[[ebp-0x24], [ebp-0x28], [ebp-0x2c]]`.
pub fn deflect_axis_from_mat(m1: [f32; 9], nx_n: f32, ny_n: f32) -> [f32; 3] {
    let v0 = f64::from(ny_n);
    let v1 = -f64::from(nx_n);
    let zero = f64::from(ZERO);
    let r0 = (f64::from(m1[2]) * v0 + f64::from(m1[5]) * v1) + f64::from(m1[8]) * zero;
    let r1 = (f64::from(m1[1]) * v0 + f64::from(m1[4]) * v1) + f64::from(m1[7]) * zero;
    let r2 = (f64::from(m1[0]) * v0 + f64::from(m1[3]) * v1) + f64::from(m1[6]) * zero;
    [r0 as f32, r1 as f32, r2 as f32]
}

/// R7 and R8 (`0x61e0a2` to `0x61e158`): the direction is rotated by the second matrix `m2` (at
/// `[ebp-0x84]`); the sums are `(m5·ny + m2·nx) + m8·nz`, `(m4·ny + m1·nx) + m7·nz` and `(m3·ny +
/// m6·nz) + m0·nx`. The last is stored as a dword and the first two stay on the stack. The speed is
/// halved and stored (`fst [esi+0x1c]`), then times 0.7 into `sp`, stored to a dword while the
/// register stays live. The steps are `sp_live·out_x` (rounded to `f32`), `out_y·sp_f32` (left
/// live) and `out_z·sp_f32` (rounded to `f32`), each added to `[esi+0x20..]` and stored. Returns
/// `(new_speed, pos)`.
pub fn deflect_transform_and_move(
    m2: [f32; 9],
    dir: [f32; 3],
    speed: f32,
    origin: [f32; 3],
) -> (f32, [f32; 3]) {
    let nxp = f64::from(dir[0]);
    let nyp = f64::from(dir[1]);
    let nzp = f64::from(dir[2]);
    let out0 = f64::from(m2[2]) * nxp + f64::from(m2[5]) * nyp + f64::from(m2[8]) * nzp;
    let out1 = f64::from(m2[1]) * nxp + f64::from(m2[4]) * nyp + f64::from(m2[7]) * nzp;
    let out2 = f64::from(m2[3]) * nyp + f64::from(m2[6]) * nzp + f64::from(m2[0]) * nxp;
    let out2_f32 = out2 as f32;
    let half_speed = f64::from(speed) * f64::from(HALF);
    let new_speed = half_speed as f32;
    let sp_live = half_speed * f64::from(SPEED_DAMP_0P7);
    let sp_f32 = sp_live as f32;
    let step_x = (sp_live * f64::from(out2_f32)) as f32;
    let step_z = (out0 * f64::from(sp_f32)) as f32;
    let pos_x = (f64::from(step_x) + f64::from(origin[0])) as f32;
    let pos_y = ((out1 * f64::from(sp_f32)) + f64::from(origin[1])) as f32;
    let pos_z = (f64::from(step_z) + f64::from(origin[2])) as f32;
    (new_speed, [pos_x, pos_y, pos_z])
}
