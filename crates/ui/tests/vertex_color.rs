//! The `Texture` colour and alpha scripts (`SetVertexColor`, `GetVertexColor`, `SetGradient`,
//! `SetGradientAlpha`, `SetAlpha`, `GetAlpha`) against the exe: each is run as the slice of
//! instructions that packs or unpacks the colour, with the Lua marshalling stubbed around it.
//!
//! A setter clamps each channel and quantizes it with 255.0 and 0.5; red, green and blue narrow
//! to `f32` at the clamp store and alpha stays an f64. A getter multiplies by the float at
//! `0x81c6b4`, pushing red as an f64 and the rest through an `f32` store. `SetAlpha` multiplies
//! by 255.0 (`0x806498`) and truncates.

#![allow(clippy::float_cmp)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::color::{
    channel_over_255, clamp01_alpha, clamp01_rgb, normalize_channel, pack_color, pack_color_pair,
    pack_gradient, quantize_channel, scaled_byte, unpack_vertex_color,
};
use fpkit::{next, read_f64, rng};

fn dbits(c: &benilla_re_difftest::StubCall) -> f64 {
    f64::from_bits((u64::from(c.args[1]) << 32) | u64::from(c.args[0]))
}
fn pushes(emu: &Emu) -> Vec<f64> {
    emu.calls()
        .iter()
        .filter(|c| c.addr == 0x6f3810)
        .map(dbits)
        .collect()
}

/// A synthesized stack frame: `(ebp, esp)` with room below `ebp` for locals and above for pushes.
fn frame(emu: &mut Emu) -> (u64, u64) {
    let base = emu.alloc(0x200);
    let ebp = base + 0x100;
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1000;
    (ebp, esp)
}
/// Patch `0x6f3620` (the lua arg-N fetcher, returns the arg as a LIVE f64 in st0) to
/// `fld qword [edx*8 + TABLE]; ret` so the per-arg-index f64 we wrote into `TABLE` lands LIVE in st0.
/// Returns the TABLE base; index it by the lua arg index (2..) `* 8`.
fn patch_arg_fetch(emu: &mut Emu) -> u64 {
    let table = emu.alloc(0x80); // indexed by edx (2..=10) * 8
    let mut code = vec![0xddu8, 0x04, 0xd5]; // fld qword [edx*8 + disp32]
    code.extend_from_slice(&(table as u32).to_le_bytes());
    code.push(0xc3); // ret
    emu.write(0x6f3620, &code);
    table
}

/// 0x79abd0 SetVertexColor — single colour pack (R=arg2,G=arg3,B=arg4 f32-clamp; A=arg5 live-f64).
///   slice 0x79ac86 (first arg fetch) .. 0x79adfe (call 0x6f34d0); ebx = assembled 0xAARRGGBB.
///   0x6f34d0 is also called at 0x79ad4a (alpha present) -> stub present(1).
/// 0x79bb40 SetTextColor — byte-identical pack; slice 0x79bbe9 .. 0x79bd53 (store ebx -> [ebp-0x14]).
#[test]
fn luabind_79a_set_vertex_color() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let table = patch_arg_fetch(&mut emu);
    emu.stub(0x6f34d0, 1, 0); // alpha-present -> present(1) so arg5 is supplied

    // (start, end, addr-label) for the two byte-identical pack rows. ebx holds the assembled u32.
    let run = |emu: &mut Emu, start: u64, end: u64, r: f64, g: f64, b: f64, a: f64| -> u32 {
        for (idx, v) in [(2u64, r), (3, g), (4, b), (5, a)] {
            emu.write(table + idx * 8, &v.to_bits().to_le_bytes());
        }
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EDI, 0); // assembly accumulator slots init from edi in the prologue
        emu.set_reg(RegisterX86::EBX, 0);
        emu.set_reg(RegisterX86::EDX, 2); // first arg index (the `mov edx,2` just before the slice)
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(start, end);
        emu.get_reg(RegisterX86::EBX) as u32
    };

    let rows: [(u64, u64, &str); 2] = [
        (0x79ac86, 0x79adfe, "SetVertexColor"),
        (0x79bbe9, 0x79bd60, "SetTextColor"),
    ];
    for &(start, end, label) in &rows {
        // deterministic: clamp edges (0.0/1.0), NaN->1.0, out-of-range, *.5 quantize ties, and the
        // f32-vs-f64 keep divergence (A is the un-narrowed f64 keep, R/G/B the f32 keep — the same
        // value quantizes to 128 in R/G/B but 129 in A).
        let div = f64::from_bits(0x3fe0_2020_2ad4_9d06); // 0.50392158856712466
        for (r, g, b, a) in [
            (0.0f64, 1.0, 0.5, 1.0),
            (-0.1, 1.1, 0.0039215, 0.9),
            (f64::NAN, 0.5, 1.0, 0.0),
            (1.0, 1.0, 1.0, 1.0),
            (0.5, 0.25, 0.125, 0.75),
            (0.5 / 255.0, 1.5 / 255.0, 2.5 / 255.0, 3.5 / 255.0),
            (-0.0, 1.0, 0.0, f64::NAN), // alpha NaN -> 1.0 (live path); -0.0 not < 0 -> kept
            (div, div, div, div),       // f32-keep (R/G/B) -> 128, f64-keep (A) -> 129
        ] {
            let bin = run(&mut emu, start, end, r, g, b, a);
            let mine = pack_color(r, g, b, a);
            assert_eq!(
                bin, mine,
                "{label} det r={r} g={g} b={b} a={a}: bin={bin:#010x} mine={mine:#010x}"
            );
        }
        let mut s = 0x79ab_d000u32 ^ (start as u32);
        let (mut lo, mut hi, mut mid) = (0u32, 0u32, 0u32);
        for _ in 0..5000 {
            let mk = |s: &mut u32| -> f64 {
                match (next(s) >> 8) % 3 {
                    0 => f64::from(rng(s, -0.5, 0.0)),
                    1 => f64::from(rng(s, 1.0, 1.5)),
                    _ => f64::from(rng(s, 0.0, 0.9999)),
                }
            };
            let (r, g, b, a) = (mk(&mut s), mk(&mut s), mk(&mut s), mk(&mut s));
            for v in [r, g, b, a] {
                if v < 0.0 {
                    lo += 1;
                } else if v >= 1.0 {
                    hi += 1;
                } else {
                    mid += 1;
                }
            }
            let bin = run(&mut emu, start, end, r, g, b, a);
            assert_eq!(
                bin,
                pack_color(r, g, b, a),
                "{label} fuzz r={r} g={g} b={b} a={a}"
            );
        }
        assert!(
            lo > 0 && hi > 0 && mid > 0,
            "{label}: need all clamp dispositions"
        );
        eprintln!("{label}: 5007 bit-exact (clamp_lo={lo} clamp_hi={hi} in_range={mid})");
    }
}

/// 0x79ae30 SetGradient — TWO 3-channel colours (R/G f32-clamp, B live-f64), alpha forced 0xff.
///   slice 0x79aef1 (first clamp, sets edi=0) .. 0x79b006 (mov [ebp-0x14],edi). colour-2 edi->esi
///   is assembled by 0x79b116; run the whole 0x79aef1 .. 0x79b119 (call 0x77f910) and read [ebp-0x14]
///   (colour 0) + [ebp-0x10] (colour 1).
#[test]
fn luabind_79a_set_gradient() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let table = patch_arg_fetch(&mut emu);

    // run the whole two-colour clamp+quantize body; colour0 stored [ebp-0x14], colour1 [ebp-0x10].
    let run = |emu: &mut Emu, c0: [f64; 3], c1: [f64; 3]| -> (u32, u32) {
        for (idx, v) in [
            (3u64, c0[0]),
            (4, c0[1]),
            (5, c0[2]),
            (6, c1[0]),
            (7, c1[1]),
            (8, c1[2]),
        ] {
            emu.write(table + idx * 8, &v.to_bits().to_le_bytes());
        }
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79aef1, 0x79b119); // halt at `call 0x77f910`; both colours assembled + stored
        (emu.read_u32(ebp - 0x14), emu.read_u32(ebp - 0x10))
    };

    // The f32-vs-f64 clamp-keep divergence: 0x3fe020202ad49d06 quantizes to 128 via the f32 keep
    // (R/G) but 129 via the un-narrowed f64 keep (the 3rd/B channel). Placing it in EVERY channel
    // probes the asymmetry — if B used the f32 clamp (or R/G the f64), this case diverges from the
    // binary. (The strict-vs-inclusive clamp boundary the fuzz never lands on.)
    let div = f64::from_bits(0x3fe0_2020_2ad4_9d06); // 0.50392158856712466
    for (c0, c1, _l) in [
        ([0.0f64, 1.0, 0.5], [0.5, 0.25, 0.125], "edges/fracs"),
        (
            [-0.1, 1.1, 0.0039215],
            [1.0, 1.0, 1.0],
            "out-of-range / all 1",
        ),
        ([f64::NAN, 0.5, 1.0], [0.0, f64::NAN, 0.999], "nan"),
        (
            [0.5 / 255.0, 1.5 / 255.0, 2.5 / 255.0],
            [3.5 / 255.0, 0.5, 0.5],
            "half-ties",
        ),
        (
            [div, div, div],
            [div, div, div],
            "f32-vs-f64 keep divergence (B is f64, R/G f32)",
        ),
    ] {
        let (b0, b1) = run(&mut emu, c0, c1);
        let (m0, m1) = pack_gradient(c0[0], c0[1], c0[2], c1[0], c1[1], c1[2]);
        assert_eq!(b0, m0, "gradient c0 {c0:?}: bin={b0:#010x} mine={m0:#010x}");
        assert_eq!(b1, m1, "gradient c1 {c1:?}: bin={b1:#010x} mine={m1:#010x}");
    }

    let mut s = 0x79ae_3000u32;
    for _ in 0..4000 {
        let mk = |s: &mut u32| -> f64 {
            match (next(s) >> 8) % 3 {
                0 => f64::from(rng(s, -0.5, 0.0)),
                1 => f64::from(rng(s, 1.0, 1.5)),
                _ => f64::from(rng(s, 0.0, 0.9999)),
            }
        };
        let c0 = [mk(&mut s), mk(&mut s), mk(&mut s)];
        let c1 = [mk(&mut s), mk(&mut s), mk(&mut s)];
        let (b0, b1) = run(&mut emu, c0, c1);
        let (m0, m1) = pack_gradient(c0[0], c0[1], c0[2], c1[0], c1[1], c1[2]);
        assert_eq!(b0, m0, "gradient c0 fuzz {c0:?}");
        assert_eq!(b1, m1, "gradient c1 fuzz {c1:?}");
    }
    eprintln!(
        "luabind_79a_set_gradient: 4004 bit-exact (R/G f32-clamp, B live-f64, A forced 0xff)"
    );
}

/// 0x79b180 — TWO full BGRA colours (R/G/B f32-clamp, A live-f64). colour0 args 3,4,5+alpha6,
///   colour1 args 7,8,9+alpha10. slice 0x79b240 (first clamp) .. 0x79b53c (call 0x77f910);
///   colour0 -> [ebp-0x18], colour1 -> [ebp-0x14]. Two alpha-present probes 0x6f34d0 -> present.
#[test]
fn luabind_79a_pack_color_pair() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let table = patch_arg_fetch(&mut emu);
    emu.stub(0x6f34d0, 1, 0); // both alphas present

    let run = |emu: &mut Emu, c0: [f64; 4], c1: [f64; 4]| -> (u32, u32) {
        for (idx, v) in [
            (3u64, c0[0]),
            (4, c0[1]),
            (5, c0[2]),
            (6, c0[3]),
            (7, c1[0]),
            (8, c1[1]),
            (9, c1[2]),
            (10, c1[3]),
        ] {
            emu.write(table + idx * 8, &v.to_bits().to_le_bytes());
        }
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79b240, 0x79b53c); // halt at `call 0x77f910`
        (emu.read_u32(ebp - 0x18), emu.read_u32(ebp - 0x14))
    };

    // f32-vs-f64 keep divergence (here A is the f64 keep, R/G/B the f32 keep): the same value
    // quantizes to 128 in R/G/B (f32) but 129 in A (f64) — probes the alpha asymmetry exactly.
    let div = f64::from_bits(0x3fe0_2020_2ad4_9d06);
    for (c0, c1, _l) in [
        ([0.0f64, 1.0, 0.5, 1.0], [0.5, 0.25, 0.125, 0.75], "edges"),
        (
            [-0.1, 1.1, 0.5, 0.9],
            [1.0, 1.0, 1.0, 1.0],
            "out-of-range / all 1",
        ),
        (
            [f64::NAN, 0.5, 1.0, f64::NAN],
            [0.0, 1.0, 0.5, 0.0],
            "nan rgb + nan alpha",
        ),
        (
            [0.5 / 255.0, 1.5 / 255.0, 2.5 / 255.0, 3.5 / 255.0],
            [0.5, 0.5, 0.5, 0.5],
            "half-ties",
        ),
        (
            [div, div, div, div],
            [div, div, div, div],
            "f32-vs-f64 keep divergence (A is f64)",
        ),
    ] {
        let (b0, b1) = run(&mut emu, c0, c1);
        let (m0, m1) = pack_color_pair(c0[0], c0[1], c0[2], c0[3], c1[0], c1[1], c1[2], c1[3]);
        assert_eq!(b0, m0, "pair c0 {c0:?}: bin={b0:#010x} mine={m0:#010x}");
        assert_eq!(b1, m1, "pair c1 {c1:?}: bin={b1:#010x} mine={m1:#010x}");
    }

    let mut s = 0x79b1_8000u32;
    for _ in 0..4000 {
        let mk = |s: &mut u32| -> f64 {
            match (next(s) >> 8) % 3 {
                0 => f64::from(rng(s, -0.5, 0.0)),
                1 => f64::from(rng(s, 1.0, 1.5)),
                _ => f64::from(rng(s, 0.0, 0.9999)),
            }
        };
        let c0 = [mk(&mut s), mk(&mut s), mk(&mut s), mk(&mut s)];
        let c1 = [mk(&mut s), mk(&mut s), mk(&mut s), mk(&mut s)];
        let (b0, b1) = run(&mut emu, c0, c1);
        let (m0, m1) = pack_color_pair(c0[0], c0[1], c0[2], c0[3], c1[0], c1[1], c1[2], c1[3]);
        assert_eq!(b0, m0, "pair c0 fuzz {c0:?}");
        assert_eq!(b1, m1, "pair c1 fuzz {c1:?}");
    }
    eprintln!(
        "luabind_79a_pack_color_pair: 4004 bit-exact (two BGRA, R/G/B f32-clamp, A live-f64)"
    );
}

/// 0x79aa50 GetVertexColor — unpack 0xAARRGGBB -> (R f64, G/B/A f32) pushed (R,G,B,A).
///   color in [ebp-4]; slice 0x79ab08 .. 0x79abbd (4 pushes via stubbed 0x6f3810). 0x81c6b4 = INV255.
#[test]
fn luabind_79a_get_vertex_color() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    emu.stub(0x6f3810, 0, 2);
    let run = |emu: &mut Emu, color: u32| -> Vec<f64> {
        emu.write_u32(ebp - 0x4, color);
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79ab08, 0x79abbd);
        pushes(emu)
    };
    let mut det: Vec<u32> = vec![
        0x0000_0000,
        0xffff_ffff,
        0xff00_0000,
        0x00ff_0000,
        0x0000_ff00,
        0x0000_00ff,
        0x8040_2010,
        0x0103_07ff,
    ];
    let mut s = 0x79aa_5000u32;
    for _ in 0..3000 {
        det.push(next(&mut s));
    }
    for color in det {
        let p = run(&mut emu, color);
        assert_eq!(p.len(), 4, "79aa50: expected 4 pushes, got {}", p.len());
        let m = unpack_vertex_color(color);
        for (i, (g, e)) in p.iter().zip(m.iter()).enumerate() {
            assert_eq!(
                g.to_bits(),
                e.to_bits(),
                "79aa50 color={color:#010x} idx={i}"
            );
        }
    }
    eprintln!("luabind_79a_get_vertex_color: 3008 bit-exact (R f64, G/B/A f32)");
}

/// Exercise the per-channel unpack normalize directly across the full byte domain (covers fild;fmul).
#[test]
fn luabind_79a_normalize_full_domain() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    emu.stub(0x6f3810, 0, 2);
    // every byte appears in the R (high-of-RGB) channel for color = 0x00BB0000 -> push idx 0 (f64).
    let run = |emu: &mut Emu, byte: u8| -> f64 {
        emu.write_u32(ebp - 0x4, u32::from(byte) << 16);
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79ab08, 0x79abbd);
        pushes(emu)[0] // R channel = (color>>16)&0xff, pushed as the un-narrowed f64
    };
    for b in 0u8..=255 {
        let bin = run(&mut emu, b);
        assert_eq!(
            bin.to_bits(),
            normalize_channel(b).to_bits(),
            "normalize byte={b} bin={bin} mine={}",
            normalize_channel(b)
        );
    }
    eprintln!("luabind_79a_normalize_full_domain: 256 bit-exact");
}

/// 0x79b580 SetAlpha — (i32)(lua_f64 * 255.0) truncate, store byte. NO clamp, NO +0.5.
///   The lua value is the LIVE f64 from 0x6f3620 -> patch it to `fld qword [cell]; ret`. slice
///   0x79b633 (call 0x6f3620) .. 0x79b643 (lea edx,[ebp-4] after `mov [ebp-1],al`); read [ebp-1].
#[test]
fn luabind_79a_set_alpha() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    // patch 0x6f3620 -> `fld qword [cell]; ret`; write the chosen lua f64 into the cell.
    let cell = emu.alloc(8);
    let mut code = vec![0xddu8, 0x05]; // fld qword [disp32]
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(0x6f3620, &code);

    let run = |emu: &mut Emu, lua: f64| -> u8 {
        emu.write(cell, &lua.to_bits().to_le_bytes());
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79b633, 0x79b643); // call 0x6f3620; fmul 255.0; __ftol -> al; halt before storing
        (emu.get_reg(RegisterX86::EAX) & 0xff) as u8 // al = the byte the binary stores to [ebp-1]
    };

    // deterministic: 0, 1, *.5 / *.999 ties, out-of-[0,1] (NO clamp -> wraps), negatives, NaN.
    for lua in [
        0.0f64,
        1.0,
        0.5,
        0.999_999,
        1.0 / 255.0,
        254.0 / 255.0,
        2.0,    // 2*255 = 510 -> low byte 0xfe
        -1.0,   // -255 -> __ftol of negative; low byte
        -0.003, // small negative
        100.5,
    ] {
        let bin = run(&mut emu, lua);
        assert_eq!(
            bin,
            scaled_byte(lua),
            "SetAlpha det lua={lua} bin={bin} mine={}",
            scaled_byte(lua)
        );
    }
    let mut s = 0x79b5_8000u32;
    for _ in 0..5000 {
        let lua = f64::from(rng(&mut s, -2.0, 4.0));
        let bin = run(&mut emu, lua);
        assert_eq!(bin, scaled_byte(lua), "SetAlpha fuzz lua={lua}");
    }
    eprintln!("luabind_79a_set_alpha: 5010 bit-exact ((i32)(lua*255) truncate, no clamp/bias)");
}

/// 0x79b690 GetAlpha — byte * (1/255 f64) pushed as f64. byte at [ebp-1]; slice 0x79b745 ..
///   0x79b762 (read the f64 push-arg at [esp]). The byte is read from [ebp-1] by `movzx`.
#[test]
fn luabind_79a_get_alpha() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let run = |emu: &mut Emu, byte: u8| -> f64 {
        emu.write(ebp - 0x1, &[byte]);
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79b745, 0x79b75d); // movzx; fild; fmul 1/255; fstp qword [esp]
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };
    for b in 0u8..=255 {
        let bin = run(&mut emu, b);
        assert_eq!(
            bin.to_bits(),
            channel_over_255(b).to_bits(),
            "GetAlpha byte={b} bin={bin} mine={}",
            channel_over_255(b)
        );
    }
    eprintln!("luabind_79a_get_alpha: 256 bit-exact (byte * 1/255 f64)");
}

/// Pure-Rust boundary checks for the clamp/quantize kernels (where strict-vs-inclusive would flip).
#[test]
fn luabind_79a_clamp_quantize_boundaries() {
    assert_eq!(clamp01_rgb(0.0), 0.0);
    assert_eq!(clamp01_rgb(-0.0), -0.0); // -0.0 is not < 0.0 -> kept (then quantizes to 0)
    assert_eq!(clamp01_rgb(1.0), 1.0);
    assert_eq!(
        clamp01_rgb(f64::from(f32::from_bits(0x3f7f_ffff))),
        f32::from_bits(0x3f7f_ffff)
    ); // 1-ulp kept
    assert_eq!(clamp01_rgb(f64::NAN).to_bits(), 1.0f32.to_bits());
    assert_eq!(clamp01_alpha(0.0), 0.0);
    assert_eq!(clamp01_alpha(1.0), 1.0);
    assert_eq!(clamp01_alpha(f64::NAN).to_bits(), 1.0f64.to_bits());
    assert_eq!(quantize_channel(0.0), 0);
    assert_eq!(quantize_channel(1.0), 255); // 255.0 + 0.5 = 255.5 -> trunc 255
    assert_eq!(quantize_channel(0.5), 128); // 127.5 + 0.5 = 128.0 -> trunc 128
}
