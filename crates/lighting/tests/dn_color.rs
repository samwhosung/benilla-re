//! `ScaleColourValue` (`0x6d1620`) and `BuildLightColours` (`0x6d37e0`) whole, against
//! `dn_color_to_bgra` and `dn_fog_derive`. The functions call `RgbToHsv` (`0x7bbc80`) and
//! `HsvToRgb` (`0x7bbd60`) and the channel lerp `0x6d51e0`, which run in the emulator too.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::dn_color::{dn_color_to_bgra, dn_fog_derive, FogPrior};
use common::next;

/// `0x6d1620` with grays (zero saturation), colours with each channel dominant and random ones, and
/// multipliers of 0, 1, below and above 1, and (apart) NaN, the infinities and a negative.
#[test]
fn dn_color_to_bgra_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let out_p = emu.alloc(4);
    let mut s = 0x636f_6c6f_u32;
    for case in 0..800 {
        let packed = match case % 6 {
            0 => 0xff00_0000,
            1 => 0xffff_ffff,
            2 => 0xff80_8080,
            3 => 0xffff_0000 | (next(&mut s) & 0xff),
            4 => 0xff00_ff00 | (next(&mut s) & 0xff),
            _ => next(&mut s),
        };
        let value_mul = match case % 5 {
            0 => 0.0f32,
            1 => 1.0,
            2 => 0.5,
            3 => 1.5,
            _ => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 23)) as f32,
        };
        let binary = {
            emu.call(0x6d1620, out_p as u32, 0, &[packed, value_mul.to_bits()]);
            emu.read_u32(out_p)
        };
        let ours = dn_color_to_bgra(packed, value_mul);
        assert_eq!(
            binary, ours,
            "to_bgra (case {case} packed={packed:#010x} m={value_mul})"
        );
    }
    for packed in [
        0xff10_2030,
        0xff80_8080,
        0xffff_0000,
        0xff00_0000,
        0xff01_0203,
    ] {
        for value_mul in [
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            -1.0,
            -0.0,
            1e30,
            f32::MIN_POSITIVE,
        ] {
            emu.call(0x6d1620, out_p as u32, 0, &[packed, value_mul.to_bits()]);
            assert_eq!(
                emu.read_u32(out_p),
                dn_color_to_bgra(packed, value_mul),
                "to_bgra packed={packed:#010x} m={value_mul}"
            );
        }
    }
}

/// The globals `0x6d37e0` reads and writes.
const G_C0C: u64 = 0xce9c0c;
const G_C10: u64 = 0xce9c10;
const G_FOG_ALPHA: u64 = 0xce9c5a;
const G_SUN_X: u64 = 0xce9cc8;
const G_SUN_Y: u64 = 0xce9ccc;
const G_SUN_Z: u64 = 0xce9cd0;
const G_CD4: u64 = 0xce9cd4;
const G_CD8: u64 = 0xce9cd8;
const G_CDC: u64 = 0xce9cdc;
const G_CE0: u64 = 0xce9ce0;
const G_CE4: u64 = 0xce9ce4;
const G_CE8: u64 = 0xce9ce8;
const G_CEC: u64 = 0xce9cec;
const G_CF0: u64 = 0xce9cf0;
const G_CF4: u64 = 0xce9cf4;
/// Eleven dwords, up to `0xce9cc4`.
const G_INTERLEAVE: u64 = 0xce9c9c;

/// Writes one byte of a global without changing its neighbours.
fn write_byte(emu: &mut Emu, va: u64, b: u8) {
    let word = va & !0x3;
    let off = (va & 0x3) as u32 * 8;
    let cur = emu.read_u32(word);
    emu.write_u32(word, (cur & !(0xffu32 << off)) | ((b as u32) << off));
}

/// `0x6d37e0` over grays and saturated colours as the base colours, random alpha, sun and prior
/// light state, and every field it writes read back.
#[test]
fn dn_fog_derive_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x666f_6764_u32;
    for case in 0..600 {
        let c0c = match case % 5 {
            0 => 0xff80_8080,
            1 => 0xffff_ffff,
            2 => 0xff00_0000,
            _ => next(&mut s),
        };
        let c10 = match (case / 5) % 5 {
            0 => 0xff40_4040,
            1 => 0xffc0_2010,
            2 => 0xff00_ff80,
            _ => next(&mut s),
        };
        let fog_alpha = (next(&mut s) >> 24) as u8;
        let sun = [next(&mut s), next(&mut s), next(&mut s)];
        let prior_fog = [next(&mut s), next(&mut s), next(&mut s), next(&mut s)];

        emu.write_u32(G_C0C, c0c);
        emu.write_u32(G_C10, c10);
        write_byte(&mut emu, G_FOG_ALPHA, fog_alpha);
        emu.write_u32(G_SUN_X, sun[0]);
        emu.write_u32(G_SUN_Y, sun[1]);
        emu.write_u32(G_SUN_Z, sun[2]);
        emu.write_u32(G_CE4, prior_fog[0]);
        emu.write_u32(G_CE8, prior_fog[1]);
        emu.write_u32(G_CEC, prior_fog[2]);
        emu.write_u32(G_CF0, prior_fog[3]);

        emu.call(0x6d37e0, 0, 0, &[]);

        let prior = FogPrior { sun, prior_fog };
        let ours = dn_fog_derive(c0c, c10, fog_alpha, &prior);

        let chk = |va: u64, o: u32, name: &str| {
            let b = emu.read_u32(va);
            assert_eq!(
                b, o,
                "fog.{name} (case {case} c0c={c0c:#010x} c10={c10:#010x})"
            );
        };
        chk(G_CD4, ours.cd4, "cd4");
        chk(G_CD8, ours.cd8, "cd8");
        chk(G_CDC, ours.cdc, "cdc");
        chk(G_CE0, ours.ce0, "ce0");
        chk(G_CE4, ours.ce4, "ce4");
        chk(G_CE8, ours.ce8, "ce8");
        chk(G_CEC, ours.cec, "cec");
        chk(G_CF0, ours.cf0, "cf0");
        chk(G_CF4, ours.cf4, "cf4");
        for (k, &iv) in ours.interleave.iter().enumerate() {
            chk(G_INTERLEAVE + 4 * k as u64, iv, &format!("interleave[{k}]"));
        }
    }
}
