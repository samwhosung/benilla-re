//! `ColourCloudBand` (`0x6cfb00`, `systems/lighting/clouds.md#cloud-colours`) and `BuildDiscQuad`
//! (`0x6d1710`, `systems/lighting/celestial.md#disc-draw`). The x87 runs at 53 bits and stores
//! `f32` at each `fstp dword`. `ColourCloudBand` also calls `ProjectCloudLight` (`0x6cf9c0`),
//! `ProjectToCloudTexture` (`0x6cf870`), `EvalSkyTrack` (`0x6cf6c0`) and the inverse square root
//! seed `0x456330` (`benilla_re_cmath::rsqrt`); the test runs them in the emulator, and `dn_sky_uv`
//! and `dn_bands` hold the transcriptions.

use benilla_re_cmath::rsqrt::rsqrt_estimate;

/// `[0x8026c8]`, 1/255 (`0x3b808081`): a byte to a channel.
pub const INV_255: f32 = f32::from_bits(0x3b80_8081);
/// `[0x7ffe58]`, 255.
pub const F255: f32 = 255.0;
/// `[0x8029cc]`, 512: the bias that puts a byte in the high bits of the `f32`.
pub const PACK_BIAS: f32 = 512.0;
/// `[0x7ff9d8]`, 1.
pub const ONE: f32 = 1.0;
/// `[0x8015b8]`, 1 as an `f64`: the channel ceiling.
pub const ONE_F64: f64 = 1.0;
/// `[0x8012cc]`, 0.75.
pub const C_075: f32 = 0.75;
/// `[0x811608]`, 192.
pub const C_192: f32 = 192.0;
/// `[0x808de4]`, 64.
pub const C_64: f32 = 64.0;

/// The magic number in `ecx` before each call to `0x456330`.
pub const FISR_MAGIC: u32 = 0x5f39_97bb;

/// `0x456330` with the magic `0x5f3997bb` (`systems/lighting/clouds.md#cloud-colours`), as
/// `ColourCloudBand` calls it: an inverse square root seed with no refinement.
fn fast_inv_sqrt_seed(x: f32) -> f32 {
    rsqrt_estimate(FISR_MAGIC, x)
}

/// One channel to a byte (bytes `0x6cfeb3` to `0x6cff44`): above 1 it is 1 (`fcomp` with the `f64`
/// 1, `test ah,0x41; jne` keeps a value at most 1 and NaN), then `ch·255 + 512` at 53 bits stored
/// as `f32` and shifted right 14, low byte.
fn pack_channel(ch: f64) -> u8 {
    let clamped = if ch > ONE_F64 { 1.0 } else { ch };
    let biased = ((clamped * f64::from(F255) + f64::from(PACK_BIAS)) as f32).to_bits();
    (biased >> 14) as u8
}

/// The fields of the cloud object `0x6cfb00` reads: `+0x14` the row count, `+0x18` the first row
/// (also the start of the byte table: `+0x18 << +0x20`), `+0x1c` the vertices per row and `+0x20`
/// the shift. The buffers are `+0x38` (colour words), `+0x44` (one byte per vertex) and `+0x68` (a
/// pair of floats per vertex).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkyDomeObj {
    pub f14: u32,
    pub f18: u32,
    pub f1c: u32,
    pub f20: u32,
}

/// What `0x6cfb00` computes before its loops: the three colour triples as floats, the z bias and
/// the intensity.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkyDomeFrame {
    /// Palette bytes 4, 5 and 6 over 255, for output bytes 0, 1 and 2.
    pub slope: [f32; 3],
    /// Palette bytes 0, 1 and 2 over 255.
    pub sun: [f32; 3],
    /// Palette bytes 8, 9 and 10 over 255.
    pub gbase: [f32; 3],
    /// `bcc·192 + 64`.
    pub z_bias: f32,
    /// `lookup·(1 − bcc·0.75)`.
    pub intensity: f32,
}

/// The setup values from the palette bytes at `[0xce9c30]`, `bcc` (`[0xce9bcc]`) and the factor
/// `EvalSkyTrack` returns in `st0` and leaves wide (`0x6cfd3f`): the triples are `INV_255·byte`
/// stored as `f32`; `z_bias = bcc·192 + 64`; `intensity = lookup·(1 − bcc·0.75)` with the lookup
/// not narrowed before the multiply.
pub fn sky_dome_frame(palette: &[u8; 11], bcc: f32, lookup_factor: f64) -> SkyDomeFrame {
    let f = f64::from;
    let pal = |i: usize| (f(INV_255) * f64::from(palette[i])) as f32;
    SkyDomeFrame {
        slope: [pal(4), pal(5), pal(6)],
        sun: [pal(0), pal(1), pal(2)],
        gbase: [pal(8), pal(9), pal(10)],
        z_bias: (f(bcc) * f(C_192) + f(C_64)) as f32,
        intensity: (lookup_factor * (f(ONE) - f(bcc) * f(C_075))) as f32,
    }
}

/// `0x6cfb00()` (`ecx` = the cloud object): writes one colour word per vertex at `+0x38`, from the
/// per-frame values, the sun position `sun_dir` (`[ebp-0x6c]`, `[ebp-0x68]`, the unscaled output of
/// `0x6cf870` on the hit of `0x6cf9c0`, which the test takes from the emulator), the pairs at
/// `+0x68` and the bytes at `+0x44`. For row `r` and vertex `v` with `t = table[row·cols + v]`:
/// - `t == 0` leaves the first vertex of a row as it is, and copies the previous word with alpha 0;
/// - otherwise `n = ((255 − t) >> 1) + 0x40` as a byte, `p = n/255`, the base `slope·p + gbase` per
///   channel stored as `f32`; `V = (sun_x − v, sun_y − (first_row + r), z_bias)` and `S` the pair;
///   the cosine is `(V·S + Vz)·(invS·invV)`, with `|V|² = (Vz² + Vy²) + Vx²` and `|S|² = (Sx² +
///   Sy²) + 1` through `fast_inv_sqrt_seed`;
/// - when the cosine is above 0 (`test ah,0x41; jne` skips 0, a negative and NaN) each channel
///   gains `sun·(cosine·intensity)`, channels 0 and 1 stored as `f32` and channel 2 kept wide;
/// - each channel is packed by `pack_channel`, written as bytes 2, 1 and 0 with `t` as byte 3.
#[allow(clippy::too_many_arguments)]
pub fn sky_dome_vtx_color_build(
    obj: &SkyDomeObj,
    frame: &SkyDomeFrame,
    sun_dir: [f32; 2],
    src: &[[f32; 2]],
    table: &[u8],
    prev_color: &[[u8; 4]],
) -> Vec<[u8; 4]> {
    let f = f64::from;
    let rows = obj.f14 as usize;
    let cols = obj.f1c as usize;
    let mut out: Vec<[u8; 4]> = prev_color.to_vec();
    out.resize(rows * cols, [0u8; 4]);

    for row in 0..rows {
        let row_base = (obj.f18 as i64 + row as i64) as f32;
        for vtx in 0..cols {
            let g = row * cols + vtx;
            let t = table[g];
            if t == 0 {
                if vtx != 0 {
                    let prev = out[g - 1];
                    out[g] = [prev[0], prev[1], prev[2], 0];
                }
                continue;
            }
            let n = (((255u8.wrapping_sub(t)) >> 1).wrapping_add(0x40)) as u32;
            let p = f(INV_255) * f64::from(n);

            let mut ch = [
                (f(frame.slope[0]) * p + f(frame.gbase[0])) as f32,
                (f(frame.slope[1]) * p + f(frame.gbase[1])) as f32,
                (f(frame.slope[2]) * p + f(frame.gbase[2])) as f32,
            ];

            let vx = (f(sun_dir[0]) - f64::from(vtx as u32)) as f32;
            let vy = (f(sun_dir[1]) - f(row_base)) as f32;
            let vz = frame.z_bias;
            let s = src[g];

            let len_v_sq = ((f(vz) * f(vz) + f(vy) * f(vy)) + f(vx) * f(vx)) as f32;
            let len_s_sq = ((f(s[0]) * f(s[0]) + f(s[1]) * f(s[1])) + f(ONE)) as f32;
            let inv_v = fast_inv_sqrt_seed(len_v_sq);
            let inv_s = fast_inv_sqrt_seed(len_s_sq);

            let dot = f(vx) * f(s[0]) + f(vy) * f(s[1]) + f(vz);
            let cos_t = dot * (f(inv_v) * f(inv_s));

            if cos_t > 0.0 {
                let m = cos_t * f(frame.intensity);
                ch[0] = (f(frame.sun[0]) * m + f(ch[0])) as f32;
                ch[1] = (f(frame.sun[1]) * m + f(ch[1])) as f32;
                let ch2_wide = f(frame.sun[2]) * m + f(ch[2]);
                out[g] = [
                    pack_channel(f(ch[0])),
                    pack_channel(f(ch[1])),
                    pack_channel(ch2_wide),
                    t,
                ];
            } else {
                out[g] = [
                    pack_channel(f(ch[0])),
                    pack_channel(f(ch[1])),
                    pack_channel(f(ch[2])),
                    t,
                ];
            }
        }
    }
    out
}

/// `0xce8be8`, the corner table of six rows of three floats, which a first-use block fills with
/// immediates.
pub const TMPL1_BASE: u64 = 0xce8be8;
/// `0xce8bb8`, a second table of six rows that the same block fills.
pub const TMPL2_BASE: u64 = 0xce8bb8;
/// `0xce8c30`, the flag byte of the first-use blocks: bit 0x1 guards the first table, bit 0x2 the
/// second.
pub const INIT_FLAG: u64 = 0xce8c30;

/// The loop of `BuildDiscQuad` (`0x6d1710`, bytes `0x6d18c0` to `0x6d190f`): for each of the six
/// rows of the corner table, `scale·x`, `scale·y` and `scale·z` at 53 bits, stored as `f32`, with
/// `scale` the float at `+0x14` of the object. The rest of the function stores immediates into the
/// tables the first time it runs and sets its outputs.
pub fn sky_dome_geom_template(scale: f32, tmpl1: &[[f32; 3]; 6]) -> [[f32; 3]; 6] {
    let f = f64::from;
    let mut out = [[0.0f32; 3]; 6];
    for (i, row) in tmpl1.iter().enumerate() {
        out[i][0] = (f(scale) * f(row[0])) as f32;
        out[i][1] = (f(scale) * f(row[1])) as f32;
        out[i][2] = (f(scale) * f(row[2])) as f32;
    }
    out
}
