//! Five functions of the sky and light state with arithmetic of their own: `BuildSunGlare`
//! (`0x6d1e30`) and `BuildMoonGlare` (`0x6d1f30`) (`systems/lighting/celestial.md#body-objects`),
//! `UpdateCameraHeading` (`0x6d4530`, `daynight.md#recompute`), `BuildCloudNoiseTables`
//! (`0x6d0c90`, `clouds.md#cloud-noise`) and `ScaleLightPosition` (`0x6d5fb0`,
//! `systems/lighting/tables.md#light-positions-at-load`). The x87 runs at 53 bits and stores `f32`
//! at each `fstp dword`; `fcos` and `fpatan` are the host's, since the emulator's are, and tests
//! compare them structurally.

/// `fld dword [a]; fadd dword [b]; fstp dword`: the sum at 53 bits, stored as `f32`.
fn fadd_c(a: u32, b: u32) -> u32 {
    ((f64::from(f32::from_bits(a)) + f64::from(f32::from_bits(b))) as f32).to_bits()
}
/// `fld dword [a]; fsub dword [b]; fstp dword`: the difference at 53 bits, stored as `f32`.
fn fsub_c(a: u32, b: u32) -> u32 {
    ((f64::from(f32::from_bits(a)) - f64::from(f32::from_bits(b))) as f32).to_bits()
}

/// `[0x811510]`, 0.22916667.
const A_811510: u32 = 0x3e6a_aaab;
/// `[0x811518]`, 0.89583331.
const B_811518: u32 = 0x3f65_5555;
/// `[0x81151c]`, 0.91666669.
const F_81151C: u32 = 0x3f6a_aaab;
/// `[0x811528]`, 0.16666667.
const D_811528: u32 = 0x3e2a_aaab;
/// `[0x8115bc]`, 0.041666668.
const RD_8115BC: u32 = 0x3d2a_aaab;
/// `[0x80e004]`, 0.083333336.
const RD_80E004: u32 = 0x3daa_aaab;
/// `[0x81161c]`, 0.020833334.
const RD_81161C: u32 = 0x3caa_aaab;
/// `[0x811620]`, 0.082638889.
const RD_811620: u32 = 0x3da9_3e94;
/// `[0x811624]`, 0.03125.
const RD_811624: u32 = 0x3d00_0000;

/// `0x6d1e30()`: initialises the glare object at `0xce97a8` through `0x7e5000`, then stores
/// immediates and four sums of `.rdata` constants. The four are returned as `(address, bits)`:
/// `[0xce9818]` A + 0.041666668, `[0xce9820]` A + 0.083333336, `[0xce9828]` B − 0.083333336 and
/// `[0xce9830]` B − 0.020833334.
pub fn sun_glare_setup() -> [(u64, u32); 4] {
    [
        (0xce9818, fadd_c(A_811510, RD_8115BC)),
        (0xce9820, fadd_c(A_811510, RD_80E004)),
        (0xce9828, fsub_c(B_811518, RD_80E004)),
        (0xce9830, fsub_c(B_811518, RD_81161C)),
    ]
}

/// `0x6d1f30()`: the same for the glare object at `0xce96f8`: `[0xce9768]` D − 0.083333336,
/// `[0xce9770]` D − 0.03125, `[0xce9778]` F + 0.03125 and `[0xce9780]` F + 0.082638889.
pub fn moon_glare_setup() -> [(u64, u32); 4] {
    [
        (0xce9768, fsub_c(D_811528, RD_80E004)),
        (0xce9770, fsub_c(D_811528, RD_811624)),
        (0xce9778, fadd_c(F_81151C, RD_811624)),
        (0xce9780, fadd_c(F_81151C, RD_811620)),
    ]
}

/// `[0x811658]`, 0.0001 as an `f64`: the squared length below which the wind direction falls back
/// to the z component.
const MAG2_EPS: f64 = f64::from_bits(0x3f1a_36e2_eb1c_432d);
/// `[0x80295c]`, 2π.
const TWO_PI: u32 = 0x40c9_0fdb;

/// `0x6d4530()` up to its first call (`0x6d4596`): from `[0xce9b84]`, `[0xce9b88]` and
/// `[0xce9b8c]`, the heading stored at `[0xce9b90]`. With `m = gy² + gx²` at 53 bits, `fcomp` with
/// 0.0001 and `test ah,0x41; jne`: when `m > 0.0001` the angle is `atan2(gy, gx)`, and otherwise
/// (`m` at most 0.0001 or NaN) `atan2(gz, gx)`. The angle is stored as `f32`, and when below 0
/// (`test ah,5; jp` skips 0, positive and NaN) 2π is added at 53 bits and stored again.
pub fn wind_angle_update(gx: f32, gy: f32, gz: f32) -> f32 {
    let f = f64::from;
    let mag2 = f(gy) * f(gy) + f(gx) * f(gx);
    let angle = if mag2 > MAG2_EPS {
        f(gy).atan2(f(gx))
    } else {
        f(gz).atan2(f(gx))
    };
    let a = angle as f32;
    if a < 0.0 {
        (f(a) + f(f32::from_bits(TWO_PI))) as f32
    } else {
        a
    }
}

/// `[0x811610]`, 3.0518509e-5 (`0x38000100`): the scale from a random value to [-1, 1].
const NOISE_SCALE: u32 = 0x3800_0100;
/// `[0x7ff9d8]`, 1.
const ONE: u32 = 0x3f80_0000;
/// `[0x811508]`, π.
const PI: u32 = 0x4049_0fdb;
/// `[0x80a8ac]`, 1/256.
const INV_256: u32 = 0x3b80_0000;
/// `[0x7ffa24]`, 0.5.
const HALF: u32 = 0x3f00_0000;

/// An entry of the noise table `0x6d0c90` fills at `0xce92d8` (bytes `0x6d0ca0` to `0x6d0cc5`):
/// with `k` the value `rand` (`0x7400e5`) returns, taken as a signed integer, `1 − 2·(k ·
/// 3.0518509e-5)` at 53 bits stored as `f32`.
pub fn sky_noise_value(k: u32) -> f32 {
    let p = f64::from(k as i32) * f64::from(f32::from_bits(NOISE_SCALE));
    let q = p + p;
    (f64::from(f32::from_bits(ONE)) - q) as f32
}

/// An entry of the cosine table `0x6d0c90` fills at `0xce8dd4` (bytes `0x6d0cd0` to `0x6d0cfd`):
/// for the counter `v` (0 to 255), `(1 − cos(v·π·(1/256)))·0.5` with the two multiplies in that
/// order, stored as `f32`. The store index is `v + 1`, so the loop fills `table[1..=256]` and
/// leaves `table[0]`.
pub fn sky_bump_value(v: u32) -> f32 {
    let a = f64::from(v) * f64::from(f32::from_bits(PI));
    let b = a * f64::from(f32::from_bits(INV_256));
    let c = b.cos();
    let d = f64::from(f32::from_bits(ONE)) - c;
    (d * f64::from(f32::from_bits(HALF))) as f32
}

/// `[0x7ff9d0]`, 1/36.
const SCALE_1_36: u32 = 0x3ce3_8e39;
/// The offset `0x6d5fb0` uses when `0x66fc30` (which returns `[0xc9e384] != 0`) returns 0:
/// 17066.667.
const OFFSET_BIG: u32 = 0x4685_5555;

/// The five floats of a light position record at `+0x8` to `+0x18`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VecRemap {
    pub f8: f32,
    pub fc: f32,
    pub f10: f32,
    pub f14: f32,
    pub f18: f32,
}

/// `0x6d5fb0(gate)` (`ecx` = the record, `dl` = `gate`): the record is left alone when `f8`, `fc`
/// and `f10` are all exactly 0 (`fcomp` with 0, `test ah,0x44; jp` or `jnp`: −0 is zero, NaN is
/// not) or when `gate` is false. Otherwise every component is multiplied by 1/36 (53 bits, `fstp
/// dword`), and with `x`, `y`, `z` the scaled `f8`, `fc`, `f10` and `off` 0 when `0x66fc30` returns
/// non-zero and 17066.667 otherwise: `f8 = −(z − off)`, `fc = −(x − off)` (`fsub` then `fchs`,
/// which differs from `off − v` only in the sign of a zero) and `f10 = y`, copied as bits.
pub fn vec_scale_remap(rec: VecRemap, gate: bool, c66_nonzero: bool) -> VecRemap {
    let all_zero = rec.f8 == 0.0 && rec.fc == 0.0 && rec.f10 == 0.0;
    if all_zero || !gate {
        return rec;
    }
    let f = f64::from;
    let c = f(f32::from_bits(SCALE_1_36));
    let x = (f(rec.f8) * c) as f32;
    let y = (f(rec.fc) * c) as f32;
    let z = (f(rec.f10) * c) as f32;
    let f14 = (f(rec.f14) * c) as f32;
    let f18 = (f(rec.f18) * c) as f32;

    let off = if c66_nonzero {
        0.0_f32
    } else {
        f32::from_bits(OFFSET_BIG)
    };
    VecRemap {
        f8: (-(f(z) - f(off))) as f32,
        fc: (-(f(x) - f(off))) as f32,
        f10: y,
        f14,
        f18,
    }
}
