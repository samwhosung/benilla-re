//! The `ColorSelect` colour round trip: what `OnColorSelect` and `GetColorRGB` hand back after
//! `SetColorRGB(r, g, b)` is composed across four routines and no per-routine test gates it.
//!
//! The widget stores HSV floats (`+0x328` hue in degrees, `+0x32c` saturation, `+0x330` value),
//! never RGB. `SetColorRGB` (`0x78eae0`) quantizes each channel with `* 255 + 0.5` and `__ftol`,
//! packs, unpacks (`0x7bbf20`) and converts to HSV (`0x7bbc80`) for storage. Every read, the
//! `OnColorSelect` payload built in `0x78bae0` (`0x78bb36`, `0x78bb42`) and `GetColorRGB` (`0x78ed10`;
//! `0x78edda`, `0x78ede6`), converts HSV back to RGB (`0x7bbd60`) and packs with the floor quantizer
//! (`0x7bbec0`): two rounding rules around a lossy `f32` round trip.
//! `systems/ui/colorselect.md` has the entries.
//!
//! The first test runs the four routines in the exe's call order against the cmath
//! transcriptions. The second takes every one of the 256^3 byte triples, which `SetColorRGB`'s
//! quantizer maps every argument onto, and pins the loss, with a control class that must come back
//! clean. The third pins that the values a getter returns quantize back to the same byte, so the
//! `local r, g, b = f:GetColorRGB(); f:SetColorRGB(r, g, b)` idiom applies the loss again.

mod fpkit;

use benilla_re_cmath::colour::{color_pack_rgb, color_unpack_rgb, hsv_to_rgb, rgb_to_hsv};
use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::color::quantize_byte;
use fpkit::next;

const UNPACK: u64 = 0x7bbf20; // packed -> RGB f32[3]   (ecx = &packed, [ebp+8] = &out, ret 4)
const RGB2HSV: u64 = 0x7bbc80; // RGB -> HSV            (ecx = &rgb,    edx     = &out, ret 0)
const HSV2RGB: u64 = 0x7bbd60; // HSV -> RGB            (ecx = &hsv,    edx     = &out, ret 0)
const PACK: u64 = 0x7bbec0; // RGB -> packed BGRA       (ecx = &out,    [ebp+8] = &rgb, ret 4)

/// Pack a byte triple exactly as `SetColorRGB` does at `0x78ece9`: `0xAARRGGBB`, alpha forced 0xff
/// (the alpha `SetColorRGB` computes is never read by `0x7bbf20`, so its value cannot matter).
fn packed(r: u8, g: u8, b: u8) -> u32 {
    0xff00_0000 | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
}

/// The read-back byte triple, composed from the four **already-diffed** cmath primitives in the
/// order `SetColorRGB` + `0x78bae0` invoke them.
fn model(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let out = color_pack_rgb(&hsv_to_rgb(&rgb_to_hsv(&color_unpack_rgb(packed(r, g, b)))));
    (
        ((out >> 16) & 0xff) as u8,
        ((out >> 8) & 0xff) as u8,
        (out & 0xff) as u8,
    )
}

/// The same chain, run as the binary's own bytes under the emulator.
fn oracle(emu: &mut Emu, r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let scratch = emu.alloc(0x100);
    let p_in = scratch; // u32 packed input
    let v_rgb = scratch + 0x10; // f32[3]
    let v_hsv = scratch + 0x20; // f32[3]
    let v_rgb2 = scratch + 0x30; // f32[3]
    let p_out = scratch + 0x40; // u32 packed output

    emu.write_u32(p_in, packed(r, g, b));
    emu.call(UNPACK, p_in as u32, 0, &[v_rgb as u32]);
    emu.call(RGB2HSV, v_rgb as u32, v_hsv as u32, &[]);
    emu.call(HSV2RGB, v_hsv as u32, v_rgb2 as u32, &[]);
    emu.call(PACK, p_out as u32, 0, &[v_rgb2 as u32]);

    let out = emu.read_u32(p_out);
    (
        ((out >> 16) & 0xff) as u8,
        ((out >> 8) & 0xff) as u8,
        (out & 0xff) as u8,
    )
}

/// The binary's own `0x7bbf20`→`0x7bbc80`→`0x7bbd60`→`0x7bbec0` chain vs the
/// composed transcriptions, over the fuzz plus every structural class the law turns on: greys (the
/// `S == 0` / hue `-1.0` sentinel leg), pure primaries, two-channel ties at the maximum (the exact
/// 60°-multiple hue sector boundaries), the saturated extremes, and the named drift witnesses.
#[test]
fn colorselect_hsv_roundtrip_chain() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let mut cases: Vec<(u8, u8, u8)> = vec![
        // named drift witnesses (each must reproduce under the binary, not just the model)
        (0, 255, 255),
        (0, 1, 90),
        (2, 0, 236),
        (146, 0, 146),
        (0, 8, 132),
        // greys — the S == 0 leg that stores hue = -1.0f (0x7bbccd)
        (0, 0, 0),
        (1, 1, 1),
        (128, 128, 128),
        (254, 254, 254),
        (255, 255, 255),
        // primaries + secondaries (ties at the max => exact 60° hue multiples)
        (255, 0, 0),
        (0, 255, 0),
        (0, 0, 255),
        (255, 255, 0),
        (255, 0, 255),
        (1, 0, 0),
        (0, 1, 0),
        (0, 0, 1),
        // a control the law says must be clean
        (128, 64, 32),
    ];
    let mut s = 0x1234_5678_u32;
    for _ in 0..600 {
        cases.push((
            (next(&mut s) >> 8) as u8,
            (next(&mut s) >> 8) as u8,
            (next(&mut s) >> 8) as u8,
        ));
    }

    for (r, g, b) in cases {
        assert_eq!(
            oracle(&mut emu, r, g, b),
            model(r, g, b),
            "chain mismatch at ({r},{g},{b})"
        );
    }
}

/// The loss, exhaustive over the whole reachable domain (all 256³ byte triples).
/// Pins the law a downstream client reads out of the note: the read-back is **not** the identity;
/// every deviation is exactly `-1` on exactly one channel; it is never `+1`, never larger than 1,
/// and never on the channel that is the minimum of the three. The control class (greys) must come
/// back clean, so a green run cannot be vacuous.
#[test]
fn colorselect_roundtrip_is_not_identity_exhaustive() {
    let mut triples_wrong = 0_u64;
    let mut chan_wrong = 0_u64;
    let mut greys_wrong = 0_u64;
    let mut wrong_is_max = 0_u64;
    let mut wrong_is_max_untied = 0_u64;
    let mut wrong_is_min = 0_u64;

    for r in 0..=255_u8 {
        for g in 0..=255_u8 {
            for b in 0..=255_u8 {
                let (r2, g2, b2) = model(r, g, b);
                if (r2, g2, b2) == (r, g, b) {
                    continue;
                }
                triples_wrong += 1;
                if r == g && g == b {
                    greys_wrong += 1;
                }
                let hi = r.max(g).max(b);
                let lo = r.min(g).min(b);
                for (inp, out) in [(r, r2), (g, g2), (b, b2)] {
                    if inp == out {
                        continue;
                    }
                    chan_wrong += 1;
                    assert_eq!(
                        i32::from(out) - i32::from(inp),
                        -1,
                        "deviation is not exactly -1 at ({r},{g},{b})"
                    );
                    if inp == hi {
                        wrong_is_max += 1;
                        // a max-channel loss requires a tie at the maximum (an exact 60° hue)
                        if (u8::from(r == hi) + u8::from(g == hi) + u8::from(b == hi)) == 1 {
                            wrong_is_max_untied += 1;
                        }
                    }
                    if inp == lo {
                        wrong_is_min += 1;
                    }
                }
            }
        }
    }

    // The control: S == 0 takes the `(v,v,v)` leg of 0x7bbd60 — an exact copy, never lossy.
    assert_eq!(greys_wrong, 0, "a grey lost a count — control failed");
    // Non-vacuity: the law claims the round trip is lossy, so it must actually lose.
    assert!(triples_wrong > 0, "control: expected a lossy round trip");
    // The shape of the loss.
    assert_eq!(wrong_is_min, 0, "the minimum channel is never lost");
    assert_eq!(
        wrong_is_max_untied, 0,
        "an untied maximum channel is never lost"
    );

    // The measured population, pinned so a drift in any of the four primitives is caught here too.
    assert_eq!(triples_wrong, 1_636_226, "wrong-triple count");
    assert_eq!(
        chan_wrong, 1_636_226,
        "wrong-channel count (one per triple)"
    );
    assert_eq!(wrong_is_max, 2_530, "max-channel (tied) loss count");
}

/// The feedback is exact, which makes the loss repeat. The values a client
/// gets back are `byte·(1/255 f64)`; feeding those straight back into `SetColorRGB`'s round-half-up
/// quantizer must reproduce the same byte for all 256, so the FrameXML idiom
/// `local r,g,b = f:GetColorRGB(); … f:SetColorRGB(r,g,b)` re-applies the lossy map instead of
/// settling.
#[test]
fn colorselect_getcolorrgb_feeds_back_exactly() {
    let inv255 = f64::from_bits(0x3f70_1010_1010_1010); // 0x804578, the f64 1/255
    for b in 0..=255_u32 {
        assert_eq!(
            quantize_byte(f64::from(b) * inv255),
            b as u8,
            "GetColorRGB→SetColorRGB is not exact at {b}"
        );
    }
    // Repeated set/get walks a channel down, one count per cycle.
    let (mut r, mut g, mut b) = (0_u8, 8_u8, 132_u8);
    let mut steps = 0;
    for _ in 0..16 {
        let n = model(r, g, b);
        if n == (r, g, b) {
            break;
        }
        (r, g, b) = n;
        steps += 1;
    }
    assert_eq!((r, g, b), (0, 0, 132), "repeated loss fixed point");
    assert_eq!(steps, 8, "repeated loss step count");
}
