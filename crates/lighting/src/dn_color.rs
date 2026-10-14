//! The colour stage of the light build: `ScaleColourValue` (`0x6d1620`,
//! `systems/lighting/daynight.md#underwater-tint`) and `BuildLightColours` (`0x6d37e0`,
//! `daynight.md#body-colours`). Both take a packed colour through `RgbToHsv` (`0x7bbc80`), change
//! the saturation or value, and return it through `HsvToRgb` (`0x7bbd60`), which are
//! `benilla_re_cmath::colour`. The 8-bit channel lerp `0x6d51e0` and the ambient darken are in
//! `dn_sky`. The x87 runs at 53 bits and stores `f32` at each `fstp dword`.

use crate::dn_sky::{dn_ambient_darken_channel, dn_channel_lerp8};
use benilla_re_cmath::colour::{hsv_to_rgb, rgb_to_hsv};

/// `[0x8026c8]`, 1/255 (`0x3b808081`): a byte to a channel in [0, 1].
const INV_255: f32 = f32::from_bits(0x3b80_8081);
/// `[0x7ffe58]`, 255: a channel in [0, 1] to a byte.
const SCALE_255: f32 = f32::from_bits(0x437f_0000);
/// `[0x8029cc]`, 512: the bias that puts a byte in the high bits of an `f32`.
const REPACK_BIAS: f32 = f32::from_bits(0x4400_0000);

/// One channel back to a byte: `channel·255 + 512` at 53 bits, stored as `f32`, then `bits >> 14`,
/// low byte (`fmul`, `fadd`, `fstp dword`, `shr 0xe`).
fn repack_byte(rgb_component: f32) -> u32 {
    let v = (f64::from(rgb_component) * f64::from(SCALE_255) + f64::from(REPACK_BIAS)) as f32;
    (v.to_bits() >> 14) & 0xff
}

/// The three low bytes of a packed colour as channels: byte 2, byte 1 and byte 0 each times 1/255,
/// stored as `f32`.
fn unpack_rgb01(packed: u32) -> [f32; 3] {
    let b = |shift: u32| (f64::from((packed >> shift) & 0xff) * f64::from(INV_255)) as f32;
    [b(16), b(8), b(0)]
}

/// `0x6d1620` (`ecx` = out, the colour and a `f32` multiplier `m` on the stack; `ret 8`): unpacks
/// the colour, converts it to hue, saturation and value, stores `value·m` (53 bits, `f32`),
/// converts back, and packs `0xff000000` with the repacked channels 0, 1 and 2 as bytes 2, 1 and 0.
pub fn dn_color_to_bgra(packed: u32, value_mul: f32) -> u32 {
    let rgb = unpack_rgb01(packed);
    let mut hsv = rgb_to_hsv(&rgb);
    hsv[2] = (f64::from(hsv[2]) * f64::from(value_mul)) as f32;
    let out = hsv_to_rgb(&hsv);
    0xff00_0000 | (repack_byte(out[0]) << 16) | (repack_byte(out[1]) << 8) | repack_byte(out[2])
}

/// What `BuildLightColours` writes. Its inputs are `[0xce9c0c]` and `[0xce9c10]` (the two base
/// colours) and the byte `[0xce9c5a]`, and the fields of the light state it copies before they are
/// rewritten (`FogPrior`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FogDerive {
    /// `[0xce9cd4]`: the first base colour.
    pub cd4: u32,
    /// `[0xce9cd8]`: the second base colour, the input of the hue round trip.
    pub cd8: u32,
    /// `[0xce9cdc]`: the first colour lerped toward the second by 0x80.
    pub cdc: u32,
    /// `[0xce9ce0]`: the second colour lerped toward the first by 0x80, then `fog_swar`.
    pub ce0: u32,
    /// `[0xce9ce4]`, `[0xce9ce8]`, `[0xce9cec]`: the round-tripped channels as `f32` bits.
    pub ce4: u32,
    pub ce8: u32,
    pub cec: u32,
    /// `[0xce9cf0]`: 1.0.
    pub cf0: u32,
    /// `[0xce9cf4]`: the second colour darkened, with the alpha byte.
    pub cf4: u32,
    /// `[0xce9c9c]` to `[0xce9cc4]`: eleven dwords in address order.
    pub interleave: [u32; 11],
}

/// The light state `BuildLightColours` copies into the block at `0xce9c9c` before the round trip
/// rewrites it.
#[derive(Debug, Clone, Copy, Default)]
pub struct FogPrior {
    /// `[0xce9cc8]`, `[0xce9ccc]` and `[0xce9cd0]`: the sun direction.
    pub sun: [u32; 3],
    /// `[0xce9ce4]`, `[0xce9ce8]`, `[0xce9cec]` and `[0xce9cf0]`.
    pub prior_fog: [u32; 4],
}

/// The byte fix-up on the lerped colour (`0x6d3824` to `0x6d3846`): `c = x − 0xefeff0`, `u = (x ^ c
/// ^ 0xfffefeff) & 0x1010100`, `(u − (u >> 8)) | (c − u)`, all wrapping.
fn fog_swar(x: u32) -> u32 {
    let c = x.wrapping_sub(0x00ef_eff0);
    let u = (x ^ c ^ 0xfffe_feff) & 0x0101_0100;
    u.wrapping_sub(u >> 8) | c.wrapping_sub(u)
}

/// `0x6d37e0()`: sets `[0xce9cd4]` and `[0xce9cdc]` to `c0c` and `[0xce9cd8]` and `[0xce9ce0]` to
/// `c10`; lerps `[0xce9ce0]` toward `c0c` by 0x80 and fixes it with `fog_swar`; lerps `[0xce9cdc]`
/// toward `c10` by 0x80; copies the block at `0xce9c9c`; darkens `c10` (each channel `(c + 3)·0x55
/// >> 8`) under the alpha byte; and takes `c10` through hue, saturation and value with the
/// saturation times 0.33 (`[0x808300]`) and the value times 1.25 (`[0x80c5d0]`).
pub fn dn_fog_derive(c0c: u32, c10: u32, fog_alpha: u8, prior: &FogPrior) -> FogDerive {
    let cd4 = c0c;
    let cd8 = c10;

    let ce0 = fog_swar(dn_channel_lerp8(c10, c0c, 0x80));

    let cdc = dn_channel_lerp8(c0c, c10, 0x80);

    let interleave = [
        prior.sun[0],
        prior.sun[1],
        prior.sun[2],
        cd4,
        cd8,
        cdc,
        ce0,
        prior.prior_fog[0],
        prior.prior_fog[1],
        prior.prior_fog[2],
        prior.prior_fog[3],
    ];

    let d = |c: u8| dn_ambient_darken_channel(c) as u32;
    let cf4 = ((fog_alpha as u32) << 24)
        | (d(((c10 >> 16) & 0xff) as u8) << 16)
        | (d(((c10 >> 8) & 0xff) as u8) << 8)
        | d((c10 & 0xff) as u8);

    let rgb = unpack_rgb01(cd8);
    let mut hsv = rgb_to_hsv(&rgb);
    const S_MUL: f32 = f32::from_bits(0x3ea8_f5c3);
    const V_MUL: f32 = f32::from_bits(0x3fa0_0000);
    hsv[1] = (f64::from(hsv[1]) * f64::from(S_MUL)) as f32;
    hsv[2] = (f64::from(hsv[2]) * f64::from(V_MUL)) as f32;
    let out = hsv_to_rgb(&hsv);

    FogDerive {
        cd4,
        cd8,
        cdc,
        ce0,
        ce4: out[0].to_bits(),
        ce8: out[1].to_bits(),
        cec: out[2].to_bits(),
        cf0: 0x3f80_0000,
        cf4,
        interleave,
    }
}
