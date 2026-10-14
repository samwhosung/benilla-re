//! The float kernels of `src/objf_fp.rs` against the exe, each read at the code the transcription
//! covers, with the stack frame, registers and fields it reads set directly:
//! - `0x620930`, `[0x62096e, 0x62099b)`: the squared magnitude stored at `[ebp-4]`;
//! - `0x6236e0`, `[0x62372a, 0x623738)`: the fall height against 2.0, the byte `test ah,0x41` reads;
//! - `0x61e7c0`, from `0x61e882` or `0x61e88a` to `0x61e8a5`: the product stored at `[esp]`;
//! - `0x62f880`, `[0x62f90a, 0x62f923)` and `0x62faa0`, `[0x62fb3c, 0x62fb52)`: the z biases;
//! - `0x628010`, `[0x6280ab, 0x6280d7)` and `0x62d8d0`, `[0x62d957, 0x62d983)`: the range
//!   compares, read as `test ah,0x41`;
//! - `0x61d790` whole, with `0x710cb0` stubbed to capture its float argument.
//!
//! Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step either
//! side, to its random inputs.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::objf_fp::{
    arrival_scale, combat_defer_visible, combat_range_visible, land_height_ah,
    proximity_smoothstep, sound_z_bias, svk_z_bias, vec3_mag2, LAND_HEIGHT_THRESHOLD, SOUND_Z_BIAS,
    SVK_Z_BIAS,
};
use common::*;

#[test]
fn diff_vec3_mag2() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let vec = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40; // [ebp-0x10..-4] valid below ebp
    let mut s = 0x9a61_u32;

    let case = |e: &mut Emu, x: f32, y: f32, z: f32, ctx: &str| {
        e.write_u32(vec, x.to_bits());
        e.write_u32(vec + 4, y.to_bits());
        e.write_u32(vec + 8, z.to_bits());
        e.set_reg(RegisterX86::EDI, vec);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20); // room for the slice's `sub esp,8`
        e.run(0x62096e, 0x62099b); // load locals .. fst [ebp-4]
        let bin = e.read_f32(ebp - 4);
        let ours = vec3_mag2(x, y, z);
        assert_f32(bin, ours, ctx);
    };

    case(&mut e, 0.0, 0.0, 0.0, "zero");
    case(&mut e, 1.0, 2.0, 3.0, "123"); // 4+9+1 = 14
    case(&mut e, -1.5, 2.5, -3.5, "neg");
    case(&mut e, 1e20, 1e20, 1e20, "overflow-inf");
    for &v in &specials(&[]) {
        case(&mut e, v, 1.0, 2.0, "special x");
        case(&mut e, 1.0, v, 2.0, "special y");
        case(&mut e, 1.0, 2.0, v, "special z");
    }
    for i in 0..400 {
        let pick = |s: &mut u32| {
            if i % 3 == 0 {
                frand_bits(s)
            } else {
                frand(s, -1e5, 1e5)
            }
        };
        let (x, y, z) = (pick(&mut s), pick(&mut s), pick(&mut s));
        case(&mut e, x, y, z, &format!("fuzz{i}"));
    }
}

const TEST_AH_41: u8 = 0x41;

#[test]
fn diff_land_height() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x1a4d_u32;
    let thr = LAND_HEIGHT_THRESHOLD; // = the .rdata 0x801628 value (2.0)

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, world_z: f32, ground_z: f32, ctx: &str| {
        e.write_u32(ebp - 4, world_z.to_bits());
        e.write_u32(ebp - 0x1c, ground_z.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x62372a, 0x623738); // fld;fsub;fcomp;fnstsw
        let bin = ah(e) & TEST_AH_41;
        let ours = land_height_ah(world_z, ground_z, thr) & TEST_AH_41;
        assert_eq!(
            bin, ours,
            "{ctx}: binary AH&41={bin:#04x} ours={ours:#04x} (wz={world_z} gz={ground_z})"
        );
        seen.insert(bin);
    };

    case(&mut e, 5.0, 0.0, "high"); // delta 5 > 2 -> C=000 -> 0 (play)
    case(&mut e, 1.0, 0.0, "low"); // delta 1 < 2 -> C0 -> 0x01 (skip)
    case(&mut e, 2.0, 0.0, "exact"); // delta 2 == 2 -> C3 -> 0x40 (skip)
    case(&mut e, f32::NAN, 0.0, "nan"); // unordered -> 0x41
                                        // delta equal to 2.0 and a step either side; both inputs at the specials.
    for d in [2.0f32, next_up(2.0), next_down(2.0)] {
        case(&mut e, d, 0.0, "threshold step");
        case(&mut e, 10.0 + d, 10.0, "threshold step offset");
    }
    for &v in &specials(&[2.0]) {
        case(&mut e, v, 0.0, "special world");
        case(&mut e, 0.0, v, "special ground");
    }
    for i in 0..400 {
        let (wz, gz) = if i % 2 == 0 {
            (frand(&mut s, -10.0, 10.0), frand(&mut s, -10.0, 10.0))
        } else {
            (frand_bits(&mut s), frand_bits(&mut s))
        };
        case(&mut e, wz, gz, &format!("fuzz{i}"));
    }
    assert!(seen.contains(&0x00), "delta>2 (play) never seen");
    assert!(seen.contains(&0x01), "delta<2 (C0) never seen");
    assert!(seen.contains(&0x40), "delta==2 (C3) never seen");
}

#[test]
fn diff_arrival_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let fields = e.alloc(0x100);
    let frame = e.alloc(0x40);
    let esp0 = frame + 0x20; // a known stack top; the slice nets one `push` (-4)
    let mut s = 0x7e7c_u32;

    let case = |e: &mut Emu, b: f32, c: f32, p9c: f32, p98: f32, gate: bool, ctx: &str| {
        e.write_u32(fields + 0x90, c.to_bits());
        e.write_u32(fields + 0x94, b.to_bits());
        e.write_u32(fields + 0x98, p98.to_bits());
        e.write_u32(fields + 0x9c, p9c.to_bits());
        e.set_reg(RegisterX86::EDI, fields);
        e.set_reg(RegisterX86::ESP, esp0);
        let begin = if gate { 0x61e882 } else { 0x61e88a };
        e.run(begin, 0x61e8a5); // fld pick .. fstp [esp]
        let prod_va = e.get_reg(RegisterX86::ESP); // after the net `push ecx`
        let bin = e.read_f32(prod_va);
        let ours = arrival_scale(b, c, p9c, p98, gate);
        assert_f32(bin, ours, ctx);
    };

    case(&mut e, 2.0, 3.0, 5.0, 7.0, true, "gate-9c"); // 2*3*5 = 30
    case(&mut e, 2.0, 3.0, 5.0, 7.0, false, "gate-98"); // 2*3*7 = 42
    case(&mut e, 0.1, 0.2, 0.3, 0.4, true, "round");
    case(&mut e, -1.5, 4.0, -2.0, 8.0, false, "neg");
    for &v in &specials(&[]) {
        for gate in [true, false] {
            case(&mut e, v, 2.0, 3.0, 4.0, gate, "special b");
            case(&mut e, 2.0, v, 3.0, 4.0, gate, "special c");
            case(&mut e, 2.0, 3.0, v, v, gate, "special pick");
        }
    }
    for i in 0..400 {
        let gate = (next(&mut s) >> 8) & 1 == 0;
        let f = |s: &mut u32| {
            if i % 4 == 0 {
                frand_bits(s)
            } else {
                frand(s, -1e3, 1e3)
            }
        };
        let (b, c, p9c, p98) = (f(&mut s), f(&mut s), f(&mut s), f(&mut s));
        case(&mut e, b, c, p9c, p98, gate, &format!("fuzz{i}"));
    }
}

#[test]
fn diff_sound_z_bias() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x050d_u32;
    let bias = SOUND_Z_BIAS; // .rdata 0x801628 = 2.0

    let case = |e: &mut Emu, z: f32, ctx: &str| {
        e.write_u32(ebp - 0xc, z.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20); // the slice pushes 3 dwords
        e.run(0x62f90a, 0x62f923); // fld;fadd;...;fstp [ebp-0xc]
        let bin = e.read_f32(ebp - 0xc);
        let ours = sound_z_bias(z, bias);
        assert_f32(bin, ours, ctx);
    };

    case(&mut e, 0.0, "zero"); // 0 + 2 = 2
    case(&mut e, -2.0, "negbias"); // -2 + 2 = 0
    case(&mut e, 100.5, "pos");
    for &v in &specials(&[2.0]) {
        case(&mut e, v, "special");
    }
    for i in 0..400 {
        let z = if i % 2 == 0 {
            frand(&mut s, -1e6, 1e6)
        } else {
            frand_bits(&mut s)
        };
        case(&mut e, z, &format!("fuzz{i}"));
    }
}

#[test]
fn diff_svk_z_bias() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x054b_u32;
    let bias = SVK_Z_BIAS; // .rdata 0x7ff9d8 = 1.0

    let case = |e: &mut Emu, z: f32, ctx: &str| {
        e.write_u32(ebp - 4, z.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20); // the slice pushes 2 dwords
        e.run(0x62fb3c, 0x62fb52); // fld;fadd;...;fstp [ebp-4]
        let bin = e.read_f32(ebp - 4);
        let ours = svk_z_bias(z, bias);
        assert_f32(bin, ours, ctx);
    };

    case(&mut e, 0.0, "zero"); // 0 + 1 = 1
    case(&mut e, -1.0, "negbias"); // -1 + 1 = 0
    case(&mut e, 50.25, "pos");
    for &v in &specials(&[1.0]) {
        case(&mut e, v, "special");
    }
    for i in 0..400 {
        let z = if i % 2 == 0 {
            frand(&mut s, -1e6, 1e6)
        } else {
            frand_bits(&mut s)
        };
        case(&mut e, z, &format!("fuzz{i}"));
    }
}

#[test]
fn diff_combat_range_visible() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let base = e.alloc(0x20);
    let sub = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x0006_2801_u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, b: [f32; 3], su: [f32; 3], range: f32, ctx: &str| {
        for k in 0..3 {
            e.write_u32(base + 4 * k as u64, b[k].to_bits());
            e.write_u32(sub + 4 * k as u64, su[k].to_bits());
        }
        e.write_u32(ebp + 0xc, range.to_bits());
        e.set_reg(RegisterX86::EAX, base);
        e.set_reg(RegisterX86::ESI, sub);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6280ab, 0x6280d7); // dist² compute + range² + fcompp + fnstsw
        let bin_ah = ah(e) & TEST_AH_41;
        let bin_visible = bin_ah == 0; // jne fail when masked != 0 -> visible iff masked == 0
        let ours = combat_range_visible(b, su, range);
        assert_eq!(
            bin_visible, ours,
            "{ctx}: binary AH&41={bin_ah:#04x} visible={bin_visible} ours={ours}"
        );
        seen.insert(bin_ah);
    };

    case(&mut e, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 5.0, "atpoint"); // 0 < 25 -> visible (AH 0)
    case(&mut e, [10.0, 0.0, 0.0], [0.0, 0.0, 0.0], 5.0, "far"); // 100 > 25 -> C0 -> 0x01
    case(&mut e, [3.0, 4.0, 0.0], [0.0, 0.0, 0.0], 5.0, "edge"); // 25 == 25 -> C3 -> 0x40
    case(&mut e, [1.0, 2.0, 2.0], [0.0, 0.0, 0.0], 3.0, "exact9"); // 9 == 9 -> C3
                                                                   // The squared range equal to the squared distance and a step either side, then NaN, infinities
                                                                   // and zeros in the range and in each coordinate.
    for r in [3.0f32, next_up(3.0), next_down(3.0)] {
        case(&mut e, [1.0, 2.0, 2.0], [0.0; 3], r, "range step");
    }
    for &v in &specials(&[3.0]) {
        case(&mut e, [1.0, 2.0, 2.0], [0.0; 3], v, "special range");
        for k in 0..3 {
            let mut b = [1.0f32, 2.0, 2.0];
            b[k] = v;
            case(&mut e, b, [0.0; 3], 3.0, "special base");
            let mut su = [0.0f32; 3];
            su[k] = v;
            case(&mut e, [1.0, 2.0, 2.0], su, 3.0, "special subtracted");
        }
    }
    for i in 0..600 {
        let b = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        let su = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        let range = match i % 3 {
            0 => frand(&mut s, 0.0, 100.0),
            1 => frand(&mut s, 0.0, 5.0),
            _ => f32::from_bits(next(&mut s)).abs(),
        };
        case(&mut e, b, su, range, &format!("fuzz{i}"));
    }
    assert!(seen.contains(&0x00), "visible (range²>dist²) never seen");
    assert!(seen.contains(&0x01), "out-of-range (C0) never seen");
}

#[test]
fn diff_combat_defer_visible() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let base = e.alloc(0x20);
    let sub = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x0006_2d8d_u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case =
        |e: &mut Emu, span: (u64, u64), b: [f32; 3], su: [f32; 3], range: f32, ctx: &str| {
            for k in 0..3 {
                e.write_u32(base + 4 * k as u64, b[k].to_bits());
                e.write_u32(sub + 4 * k as u64, su[k].to_bits());
            }
            e.write_u32(ebp - 4, range.to_bits());
            e.set_reg(RegisterX86::EAX, base);
            e.set_reg(RegisterX86::EDI, sub);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(span.0, span.1);
            let bin_ah = ah(e) & TEST_AH_41;
            let bin_defer = bin_ah == 0; // jne out when masked != 0 -> defer iff masked == 0
            let ours = combat_defer_visible(b, su, range);
            assert_eq!(
                bin_defer, ours,
                "{ctx}: binary AH&41={bin_ah:#04x} defer={bin_defer} ours={ours}"
            );
            seen.insert(bin_ah);
        };

    // `LogTradeSkillInRange` and `LogFeedPetInRange` are the same code, 0xd0 apart.
    for span in [(0x62d957, 0x62d983), (0x62d887, 0x62d8b3)] {
        case(
            &mut e,
            span,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            5.0,
            "atpoint",
        ); // 0 < 25 -> defer (AH 0)
        case(&mut e, span, [10.0, 0.0, 0.0], [0.0, 0.0, 0.0], 5.0, "far"); // 100 > 25 -> C0
        case(&mut e, span, [3.0, 4.0, 0.0], [0.0, 0.0, 0.0], 5.0, "edge"); // 25 == 25 -> C3
        for r in [5.0f32, next_up(5.0), next_down(5.0)] {
            case(&mut e, span, [3.0, 4.0, 0.0], [0.0; 3], r, "range step");
        }
        for &v in &specials(&[5.0]) {
            case(&mut e, span, [3.0, 4.0, 0.0], [0.0; 3], v, "special range");
            for k in 0..3 {
                let mut b = [3.0f32, 4.0, 0.0];
                b[k] = v;
                case(&mut e, span, b, [0.0; 3], 5.0, "special base");
                let mut su = [0.0f32; 3];
                su[k] = v;
                case(&mut e, span, [3.0, 4.0, 0.0], su, 5.0, "special subtracted");
            }
        }
        for i in 0..600 {
            let b = [
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
            ];
            let su = [
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
            ];
            let range = match i % 3 {
                0 => frand(&mut s, 0.0, 100.0),
                1 => frand(&mut s, 0.0, 5.0),
                _ => f32::from_bits(next(&mut s)).abs(),
            };
            case(&mut e, span, b, su, range, &format!("fuzz{i}"));
        }
    }
    assert!(seen.contains(&0x00), "defer (range²>dist²) never seen");
    assert!(seen.contains(&0x01), "no-defer (C0) never seen");
}

#[test]
fn diff_proximity_smoothstep() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x80);
    // dword so the deref is valid. 0x710cb0 is stubbed, so the loaded ecx is otherwise unused.
    let sub = e.alloc(0x10);
    e.write_u32(this, sub as u32);
    e.stub(0x710cb0, 0, 0); // sound-volume set: no-op, capture the pushed f32 arg
    let mut s = 0x1d79_u32;

    let mut none = 0u32;
    let mut smooth = 0u32;
    let mut zero = 0u32;
    let mut case =
        |e: &mut Emu, cur: [f32; 3], origin: [f32; 3], speed: f32, flag8: bool, ctx: &str| {
            e.write_u32(this + 0x1c, speed.to_bits());
            for k in 0..3 {
                e.write_u32(this + 0x20 + 4 * k as u64, origin[k].to_bits());
                e.write_u32(this + 0x2c + 4 * k as u64, cur[k].to_bits());
            }
            e.write_u32(this + 0x38, if flag8 { 0x08 } else { 0x00 });
            e.clear_calls();
            e.call(0x61d790, this as u32, 0, &[]);
            let calls = e.calls();
            let bin = if calls.is_empty() {
                None
            } else {
                Some(f32::from_bits(calls[0].args[0]))
            };
            let ours = proximity_smoothstep(cur, origin, speed, flag8);
            match (bin, ours) {
                (None, None) => none += 1,
                (Some(b), Some(o)) => {
                    assert_f32(b, o, ctx);
                    // classify the reached arm for coverage.
                    if o == 0.0 {
                        zero += 1;
                    } else {
                        smooth += 1;
                    }
                }
                (b, o) => panic!("{ctx}: branch mismatch binary={b:?} ours={o:?}"),
            }
        };

    // origin..cur close + positive speed -> dist² < scaledSpeed² -> smoothstep.
    case(
        &mut e,
        [0.1, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        10.0,
        false,
        "smooth-noflag",
    );
    case(
        &mut e,
        [0.0, 0.2, 0.0],
        [0.0, 0.0, 0.0],
        10.0,
        true,
        "smooth-flag",
    );
    // far apart -> dist² >= scaledSpeed² -> None (degenerate exit).
    case(
        &mut e,
        [100.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        1.0,
        false,
        "none-far",
    );
    // close + NEGATIVE speed -> scaledSpeed<0 -> t<0 -> 0.0 clamp.
    case(
        &mut e,
        [0.1, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        -10.0,
        false,
        "neg-speed-zero",
    );
    // `dist^2 == scaled^2` exactly (3-4-5 with a scaled speed of 5): the gate leaves; and a zero
    // distance (t = 0), a step either side of the equality, and NaN, infinities and zeros.
    for x in [3.0f32, next_up(3.0), next_down(3.0)] {
        case(&mut e, [x, 4.0, 0.0], [0.0; 3], 10.0, true, "gate equality");
    }
    case(&mut e, [0.0; 3], [0.0; 3], 10.0, true, "zero distance");
    case(&mut e, [0.0; 3], [0.0; 3], 0.0, true, "zero both");
    for &v in &specials(&[10.0]) {
        for flag in [true, false] {
            case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], v, flag, "special speed");
            for k in 0..3 {
                let mut c = [0.1f32, 0.2, 0.3];
                c[k] = v;
                case(&mut e, c, [0.0; 3], 10.0, flag, "special cur");
                let mut o = [0.0f32; 3];
                o[k] = v;
                case(&mut e, [0.1, 0.2, 0.3], o, 10.0, flag, "special origin");
            }
        }
    }
    for i in 0..500 {
        let flag8 = (next(&mut s) >> 8) & 1 == 0;
        // bias toward close points so the gate passes often; mix in far + full-bit.
        let scale = match i % 3 {
            0 => 0.5,
            1 => 5.0,
            _ => 50.0,
        };
        let cur = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        let origin = [0.0, 0.0, 0.0];
        let speed = match i % 5 {
            0 => frand(&mut s, -20.0, -0.1), // negative -> t<0 clamp
            4 => frand_bits(&mut s),
            _ => frand(&mut s, 0.1, 20.0),
        };
        case(&mut e, cur, origin, speed, flag8, &format!("fuzz{i}"));
    }
    assert!(none > 0, "degenerate (None) branch never exercised");
    assert!(smooth > 0, "smoothstep branch never exercised");
    assert!(zero > 0, "t<0 clamp (0.0) branch never exercised");
}
