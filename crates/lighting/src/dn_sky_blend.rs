//! The two blends of Light records: `BlendLightRecord` (`0x6d67c0`,
//! `systems/lighting/daynight.md#record-blend`) and `OverblendAreaLight` (`0x6d30e0`,
//! `daynight.md#area-overblend`). A record is the 0x90 bytes of 0x24 dwords the band gather
//! `0x6d64d0` fills: packed colours and `f32`s. The x87 runs at 53 bits, `fstp dword` stores `f32`,
//! and `fistp` rounds to nearest, ties to even.

use crate::x87::fistp;

/// `[0x7ffe58]`, 255.
const C_255: f32 = f32::from_bits(0x437f_0000);
/// `[0x8029cc]`, 512.
const C_512: f32 = f32::from_bits(0x4400_0000);
/// `[0x7ff9d8]`, 1.0.
const C_ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x86f2c8]`, 0.5, subtracted before `fistp`.
const C_HALF: f64 = 0.5;

/// A Light record: 0x24 dwords.
pub type GatherRecord = [u32; 0x24];

/// The bits of `x` stored as `f32`.
#[inline]
fn f32_bits(x: f64) -> u32 {
    (x as f32).to_bits()
}

/// The clamp of `t` at `0x6d67c8` to `0x6d67fb`: below 0 it is 0 (`fcomp` with 0, `test ah,5; jp`
/// falls through only for a negative), and anything not below 1, NaN included, is 1 (`fcomp` with
/// 1, `jnp` taken only for a value below it).
fn clamp_unit_f32(t: f32) -> f32 {
    if t < 0.0 {
        0.0
    } else if t < 1.0 {
        t
    } else {
        1.0
    }
}

/// `0x6d6c20` and `0x690ae0`, the colour blend of one packed colour toward another by `a`: per
/// channel `dst + ((src − dst)·a) >> 8`, with the signed product shifted logically and the byte add
/// wrapping; alpha is kept from `dst`. A weight of 0 leaves `dst` (`0x6d6c20` returns at once;
/// `0x6d67c0` does not call `0x690ae0` for it), and 0xff replaces the 24 colour bits.
pub(crate) fn color_blend_leaf(dst: u32, src: u32, a: u32) -> u32 {
    if a == 0 {
        return dst;
    }
    if a == 0xff {
        return (dst & 0xff00_0000) | (src & 0x00ff_ffff);
    }
    let chan = |sh: u32| -> u32 {
        let d = (dst >> sh) & 0xff;
        let s = (src >> sh) & 0xff;
        let prod = s.wrapping_sub(d).wrapping_mul(a);
        (prod >> 8).wrapping_add(d) & 0xff
    };
    (dst & 0xff00_0000) | (chan(16) << 16) | (chan(8) << 8) | chan(0)
}

/// `0x6d67c0` (`ecx` = dst, `edx` = src, the weight `t` on the stack; `ret 4`): blends `src` into
/// `dst`. `t` is clamped to [0, 1] (`clamp_unit_f32`); the byte `a8` is `bits(f32(t·255 + 512)) >>
/// 14`, low byte. The eighteen colours at `+0x00` to `+0x40` and `+0x4c` blend by `a8` through
/// `color_blend_leaf`; the floats at `+0x44`, `+0x48`, `+0x58`, `+0x64`, `+0x68`, `+0x6c`, `+0x70`
/// and `+0x74` become `(src − dst)·t + dst` at 53 bits, stored as `f32`. `+0x50`, `+0x54`, `+0x5c`
/// and `+0x60` are kept.
pub fn sky_weighted_blend(dst: &mut GatherRecord, src: &GatherRecord, t: f32) {
    let t = clamp_unit_f32(t);
    let v = (f64::from(t) * f64::from(C_255) + f64::from(C_512)) as f32;
    let a8 = (v.to_bits() >> 14) & 0xff;

    const COLOR_OFF: [usize; 18] = [
        0x00, 0x04, 0x08, 0x0c, 0x10, 0x14, 0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38,
        0x3c, 0x40, 0x4c,
    ];
    for &off in &COLOR_OFF {
        let i = off / 4;
        dst[i] = color_blend_leaf(dst[i], src[i], a8);
    }

    const LERP_OFF: [usize; 8] = [0x44, 0x48, 0x58, 0x64, 0x68, 0x6c, 0x70, 0x74];
    for &off in &LERP_OFF {
        let i = off / 4;
        let df = f64::from(f32::from_bits(dst[i]));
        let sf = f64::from(f32::from_bits(src[i]));
        dst[i] = f32_bits((sf - df) * f64::from(t) + df);
    }
}

/// `0x6d15f0(a, b, t)` (`LerpChannel`), the result left wide: `fcomp` of `b` with `a`, `test ah,1;
/// jne`: when `b < a` or either is NaN it is `a − (a − b)·t`, otherwise `(b − a)·t + a`. `t` is not
/// clamped.
#[inline]
fn lerp_scalar(a: f64, b: f64, t: f64) -> f64 {
    if b >= a {
        (b - a) * t + a
    } else {
        a - (a - b) * t
    }
}

/// One colour of `0x6d30e0`: per byte `k`, `lerp_scalar(dst, src, w)` stored as `f32`, minus 0.5,
/// `fistp`. The packer `0x4a2c10(0xff, ch2, ch1, ch0)` gives `0xff << 24 | ch2 << 16 | ch1 << 8 |
/// ch0` from the low byte of each, so a channel `fistp` cannot hold (`0x80000000`) is 0.
#[inline]
fn overblend_color(dst: u32, src: u32, w: f64) -> u32 {
    let chan = |k: u32| -> u32 {
        let a = f64::from((dst >> (8 * k)) & 0xff);
        let b = f64::from((src >> (8 * k)) & 0xff);
        let t = lerp_scalar(a, b, w);
        let narrowed = f64::from(t as f32);
        let biased = narrowed - C_HALF;
        fistp(biased) as u32
    };
    let ch0 = chan(0);
    let ch1 = chan(1);
    let ch2 = chan(2);
    (0xffu32 << 24) | ((ch2 & 0xff) << 16) | ((ch1 & 0xff) << 8) | (ch0 & 0xff)
}

/// One float of `0x6d30e0`: `fld src; fld dst; fld st1; fcomp st1; test ah,1; jne`. When `src <
/// dst` or either is NaN it is `dst − (dst − src)·w`, otherwise `(src − dst)·w + dst`; stored as
/// `f32`.
#[inline]
fn overblend_f32_bits(dst: f32, src: f32, w: f64) -> u32 {
    let d = f64::from(dst);
    let s = f64::from(src);
    let r = if s >= d {
        (s - d) * w + d
    } else {
        d - (d - s) * w
    };
    (r as f32).to_bits()
}

/// `0x6d30e0` (`ecx` = dst, `edx` = src, the weight `w` on the stack; `ret 4`): blends `dst` toward
/// `src` by `w`, with no `w` clamp and no fast path at 0 or 0xff. In order, over the record:
/// - the colours at `+0x04`, `+0x00`, `+0x4c`, `+0x08` to `+0x1c`, `+0x20` to `+0x30` and `+0x34`
///   to `+0x40` (eighteen), by `overblend_color`;
/// - the floats at `+0x44`, `+0x48`, `+0x50`, `+0x58`, `+0x64`, `+0x68`, `+0x6c` and `+0x70`, by
///   `overblend_f32_bits`, and `+0x74` by `lerp_scalar` stored as `f32`;
/// - when `src[+0x78]` is not 0 and `w > 0` (`fcomp` with 0, `test ah,0x41; jne` skips 0, a
///   negative and NaN), the cloud accumulator of two `{id, weight}` pairs at `+0x78` and `+0x80`:
///   the first pair whose id equals `src`'s adds `w` (stored as `f32`, and set to 1.0 when the wide
///   sum exceeds 1.0); otherwise the first empty pair takes `{id, w}`; with both taken by other ids
///   nothing is stored;
/// - `+0x88` copied from `src`, and `+0x8c` set to the bits of `w`.
///
/// `+0x54`, `+0x5c` and `+0x60` are untouched.
pub fn cloud_record_overblend(dst: &mut GatherRecord, src: &GatherRecord, w: f32) {
    let wf = f64::from(w);
    let off = |b: usize| b / 4;

    const COLOR_OFF: [usize; 18] = [
        0x04, 0x00, 0x4c, 0x08, 0x0c, 0x10, 0x14, 0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34,
        0x38, 0x3c, 0x40,
    ];
    for &b in &COLOR_OFF {
        let i = off(b);
        dst[i] = overblend_color(dst[i], src[i], wf);
    }

    const LERP_OFF: [usize; 8] = [0x44, 0x48, 0x50, 0x58, 0x64, 0x68, 0x6c, 0x70];
    for &b in &LERP_OFF {
        let i = off(b);
        dst[i] = overblend_f32_bits(f32::from_bits(dst[i]), f32::from_bits(src[i]), wf);
    }

    {
        let i = off(0x74);
        let a = f64::from(f32::from_bits(dst[i]));
        let b = f64::from(f32::from_bits(src[i]));
        dst[i] = (lerp_scalar(a, b, wf) as f32).to_bits();
    }

    let src_id = src[off(0x78)];
    if src_id != 0 && wf > 0.0 {
        let mut c = 0usize;
        loop {
            let id_slot = off(0x78) + c * 2;
            let w_slot = id_slot + 1;
            let cur = dst[id_slot];
            if cur == src_id {
                let acc = f64::from(f32::from_bits(dst[w_slot])) + wf;
                let acc_bits = (acc as f32).to_bits();
                if acc > 1.0 {
                    dst[w_slot] = C_ONE.to_bits();
                } else {
                    dst[w_slot] = acc_bits;
                }
                break;
            }
            if cur == 0 {
                dst[id_slot] = src_id;
                dst[w_slot] = w.to_bits();
                break;
            }
            c += 1;
            if c >= 2 {
                break;
            }
        }
    }

    dst[off(0x88)] = src[off(0x88)];
    dst[off(0x8c)] = w.to_bits();
}
