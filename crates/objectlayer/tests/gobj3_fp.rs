//! The transport kernels of `src/gobj3_fp.rs` against the exe. Each runs a slice of its function,
//! with the frame, registers and fields it reads set directly, reading the store it ends in or the
//! status byte `fnstsw ax` leaves:
//! - the handler constructor `0x5f5ab0`: the position sum, `[0x5f5bce, 0x5f5bf4)`;
//! - the transport tick `0x5f5f10`: the position sum `[0x5f5f47, 0x5f5f69)`, the displacement
//!   `[0x5f5f6e, 0x5f5f93)`, the squared length and gate `[0x5f5f98, 0x5f5fbb)`, the direction
//!   `[0x5f60f2, 0x5f612d)` and the speed `[0x5f61ac, 0x5f61c0)`;
//! - the ship mover `0x5f50a0`: the leg times `[0x5f51b7, 0x5f51ee)`, the fraction `[0x5f51ee,
//!   0x5f5230)` with `0x5f8dc0` replaced by a load of the easing, and the heading `[0x5f5251,
//!   0x5f528e)` and through its `fpatan` to the store `[0x5f5251, 0x5f52ae)`;
//! - `0x5f8cb0` runs inside the heading slices; its own test is in `objclass_fp.rs`.
//!
//! Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step either side,
//! to its random inputs.
//!
//! Host double (`docs/METHOD.md#the-harness`): `g3_transport_heading_angle` runs `fpatan` at
//! `0x5f528e`. The emulator computes each in host `double`, so where its result reaches a compared
//! value the host's math stands in for the x87's, and the test shows the code around it.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::gobj3_fp::*;
use benilla_re_objectlayer::objclass_fp::NORM2_EPS;
use common::*;

/// Three components from the specials and a rotation of them, so each slot sees each value.
fn triples(sp: &[f32]) -> Vec<[f32; 3]> {
    let mut v = Vec::new();
    for (i, &x) in sp.iter().enumerate() {
        for &y in sp {
            let z = sp[(i + 3) % sp.len()];
            v.push([x, y, z]);
            v.push([y, z, x]);
        }
    }
    v
}

#[test]
fn g3_transport_ctor_add() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let off = e.alloc(0x20);
    let esi = e.alloc(0x40);
    let mut s = 0x5f5a_b000u32;

    let case = |e: &mut Emu, base: [f32; 3], o: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x10, base[0].to_bits());
        e.write_u32(ebp - 0xc, base[1].to_bits());
        e.write_u32(ebp - 8, base[2].to_bits());
        for (k, v) in o.iter().enumerate() {
            e.write_u32(off + 4 * k as u64, v.to_bits());
        }
        e.set_reg(RegisterX86::EAX, off);
        e.set_reg(RegisterX86::ESI, esi);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5f5bce, 0x5f5bf4);
        let bin = [
            e.read_f32(ebp - 0x1c),
            e.read_f32(ebp - 0x18),
            e.read_f32(ebp - 0x14),
        ];
        let ours = transport_pos_add(base, o);
        for i in 0..3 {
            assert_f32(bin[i], ours[i], &format!("{ctx} [{i}]"));
        }
    };

    case(&mut e, [1.0, 2.0, 3.0], [10.0, 20.0, 30.0], "simple");
    case(&mut e, [0.0; 3], [0.0; 3], "zeros");
    case(&mut e, [-0.0; 3], [-0.0; 3], "negative zeros");
    let sp = specials(&[]);
    for b in triples(&sp) {
        for o in triples(&sp).iter().step_by(7) {
            case(&mut e, b, *o, "special");
        }
    }
    for i in 0..4000u32 {
        let base = if i % 3 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -5e3, 5e3),
                frand(&mut s, -5e3, 5e3),
                frand(&mut s, -5e3, 5e3),
            ]
        };
        let o = if i % 3 == 1 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e3, 1e3),
                frand(&mut s, -1e3, 1e3),
                frand(&mut s, -1e3, 1e3),
            ]
        };
        case(&mut e, base, o, &format!("fuzz{i}"));
    }
}

#[test]
fn g3_move_pos_add() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let off = e.alloc(0x20);
    let mut s = 0x5f5f_1000u32;

    // `sub esp,0xc` at 0x5f5f4d sits between z and y, so the sums end at `SLICE_ESP - 4` (z),
    // `- 8` (y) and `- 0xc` (x).
    let case = |e: &mut Emu, base: [f32; 3], o: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0xc, base[0].to_bits());
        e.write_u32(ebp - 8, base[1].to_bits());
        e.write_u32(ebp - 4, base[2].to_bits());
        for (k, v) in o.iter().enumerate() {
            e.write_u32(off + 4 * k as u64, v.to_bits());
        }
        e.set_reg(RegisterX86::EAX, off);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5f5f47, 0x5f5f69);
        let bin = [
            e.read_f32(SLICE_ESP - 0xc),
            e.read_f32(SLICE_ESP - 8),
            e.read_f32(SLICE_ESP - 4),
        ];
        let ours = transport_pos_add(base, o);
        for i in 0..3 {
            assert_f32(bin[i], ours[i], &format!("{ctx} [{i}]"));
        }
    };

    case(&mut e, [1.0, 2.0, 3.0], [10.0, 20.0, 30.0], "simple");
    let sp = specials(&[]);
    for b in triples(&sp) {
        for o in triples(&sp).iter().step_by(7) {
            case(&mut e, b, *o, "special");
        }
    }
    for i in 0..4000u32 {
        let base = if i % 3 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -5e3, 5e3),
                frand(&mut s, -5e3, 5e3),
                frand(&mut s, -5e3, 5e3),
            ]
        };
        let o = [
            frand(&mut s, -1e3, 1e3),
            frand(&mut s, -1e3, 1e3),
            frand(&mut s, -1e3, 1e3),
        ];
        case(&mut e, base, o, &format!("fuzz{i}"));
    }
}

#[test]
fn g3_move_delta_sub() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x60);
    let ebp = frame + 0x40;
    let edi = e.alloc(0x80);
    let mut s = 0x5f5f_2000u32;

    // The new position is at `[ebp-0x24]` and the current one at `[edi+0x2c]`; the three
    // differences end at `SLICE_ESP - 0xc`, `- 8` and `- 4`.
    let case = |e: &mut Emu, newpos: [f32; 3], cur: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x24, newpos[0].to_bits());
        e.write_u32(ebp - 0x20, newpos[1].to_bits());
        e.write_u32(ebp - 0x1c, newpos[2].to_bits());
        e.write_u32(edi + 0x2c, cur[0].to_bits());
        e.write_u32(edi + 0x30, cur[1].to_bits());
        e.write_u32(edi + 0x34, cur[2].to_bits());
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5f5f6e, 0x5f5f93);
        let bin = [
            e.read_f32(SLICE_ESP - 0xc),
            e.read_f32(SLICE_ESP - 8),
            e.read_f32(SLICE_ESP - 4),
        ];
        let ours = move_delta_sub(newpos, cur);
        for i in 0..3 {
            assert_f32(bin[i], ours[i], &format!("{ctx} [{i}]"));
        }
    };

    case(&mut e, [5.0, 6.0, 7.0], [1.0, 2.0, 3.0], "simple");
    case(&mut e, [0.0; 3], [0.0; 3], "zeros");
    case(&mut e, [-0.0; 3], [0.0; 3], "negative zero less zero");
    case(&mut e, [1.0; 3], [1.0; 3], "equal gives +0");
    let sp = specials(&[]);
    for b in triples(&sp) {
        for o in triples(&sp).iter().step_by(7) {
            case(&mut e, b, *o, "special");
        }
    }
    for i in 0..4000u32 {
        let newpos = if i % 3 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -5e3, 5e3),
                frand(&mut s, -5e3, 5e3),
                frand(&mut s, -5e3, 5e3),
            ]
        };
        let cur = [
            frand(&mut s, -5e3, 5e3),
            frand(&mut s, -5e3, 5e3),
            frand(&mut s, -5e3, 5e3),
        ];
        case(&mut e, newpos, cur, &format!("fuzz{i}"));
    }
}

#[test]
fn g3_move_dist2_gate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x5f5f_3000u32;
    let mut seen_ge = false;
    let mut seen_lt = false;

    let mut case = |e: &mut Emu, delta: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x18, delta[0].to_bits());
        e.write_u32(ebp - 0x14, delta[1].to_bits());
        e.write_u32(ebp - 0x10, delta[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5f5f98, 0x5f5fbb);
        let bin_dist2 = e.read_f32(ebp + 8);
        let bin_ge = parity_even(ah(e) & 0x05);
        if bin_ge {
            seen_ge = true;
        } else {
            seen_lt = true;
        }
        let (our_dist2, our_ge) = move_dist2_gate(delta);
        assert_f32(bin_dist2, our_dist2, &format!("{ctx} dist2"));
        assert_eq!(
            bin_ge,
            our_ge,
            "{ctx}: bin_ge={bin_ge} ah&5={:#04x} dist2={bin_dist2}",
            ah(e) & 0x05
        );
    };

    let eps = EPS_2EM20;
    let root = 2.0f32.powi(-10);
    case(&mut e, [3.0, 4.0, 12.0], "169 well above 2^-20");
    case(&mut e, [0.0; 3], "0 below");
    case(&mut e, [root, 0.0, 0.0], "equal to 2^-20");
    case(&mut e, [next_down(root), 0.0, 0.0], "a step below 2^-20");
    case(&mut e, [next_up(root), 0.0, 0.0], "a step above 2^-20");
    assert_eq!(root * root, eps);
    // The squared length stored as a dword rounds up to exactly 2^-20 while the sum on the stack is
    // below it: the gate reads the stack.
    case(
        &mut e,
        [
            f32::from_bits(0xba4e_3c5b),
            f32::from_bits(0xb85d_ff07),
            f32::from_bits(0x3a17_07e2),
        ],
        "stack below 2^-20, dword at 2^-20",
    );
    let sp = specials(&[root]);
    for &x in &sp {
        for &y in &[0.0, -0.0, f32::NAN, f32::INFINITY, 1.0, root] {
            for &z in &[0.0, f32::NAN, root] {
                case(&mut e, [x, y, z], "special");
                case(&mut e, [z, x, y], "special, rotated");
            }
        }
    }
    for i in 0..5000u32 {
        let delta = match i % 4 {
            0 => {
                let t = 1e-3 * frand(&mut s, 0.0, 2.0);
                [t, frand(&mut s, -1e-3, 1e-3), frand(&mut s, -1e-3, 1e-3)]
            }
            1 => [
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
            ],
            2 => [
                frand(&mut s, -1e3, 1e3),
                frand(&mut s, -1e3, 1e3),
                frand(&mut s, -1e3, 1e3),
            ],
            _ => [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
        };
        case(&mut e, delta, &format!("fuzz{i}"));
    }
    assert!(seen_ge, "the movement arm never seen");
    assert!(seen_lt, "the standing arm never seen");
}

#[test]
fn g3_move_normalize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let edi = e.alloc(0x80);
    let mut s = 0x5f60_f000u32;

    let case = |e: &mut Emu, delta: [f32; 3], dist2: f32, ctx: &str| {
        e.write_u32(ebp - 0x18, delta[0].to_bits());
        e.write_u32(ebp - 0x14, delta[1].to_bits());
        e.write_u32(ebp - 0x10, delta[2].to_bits());
        e.write_u32(ebp + 8, dist2.to_bits());
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5f60f2, 0x5f612d);
        let bin = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 8),
            e.read_f32(ebp - 4),
        ];
        let ours = move_normalize(delta, dist2);
        for i in 0..3 {
            assert_f32(bin[i], ours[i], &format!("{ctx} [{i}]"));
        }
    };

    let sq = |d: [f32; 3]| move_dist2_gate(d).0;
    let d0 = [3.0, 4.0, 0.0];
    case(&mut e, d0, sq(d0), "3-4-0, length 5");
    case(&mut e, [0.0; 3], 0.0, "zero length, 1/0");
    let sp = specials(&[]);
    for &d2 in &sp {
        for d in [
            [3.0, 4.0, 0.0],
            [-1.0, 0.0, 2.0],
            [0.0; 3],
            [f32::NAN, 1.0, 1.0],
        ] {
            case(&mut e, d, d2, "special squared length");
        }
    }
    for b in triples(&sp).iter().step_by(5) {
        case(&mut e, *b, sq(*b), "special delta");
    }
    for i in 0..5000u32 {
        let delta = if i % 4 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            let scale = match i % 4 {
                1 => 1.0,
                2 => 1000.0,
                _ => 0.01,
            };
            [
                frand(&mut s, -scale, scale),
                frand(&mut s, -scale, scale),
                frand(&mut s, -scale, scale),
            ]
        };
        let dist2 = if i % 3 == 0 {
            frand(&mut s, 0.0, 1e6)
        } else {
            sq(delta)
        };
        case(&mut e, delta, dist2, &format!("fuzz{i}"));
    }
}

#[test]
fn g3_move_leg_speed() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let edi = e.alloc(0x80);
    let mut s = 0x5f61_a000u32;

    // The count is a `dword` with a zero high dword.
    let case = |e: &mut Emu, len: f32, count: u32, ctx: &str| {
        e.write_u32(ebp - 8, count);
        e.write_u32(ebp - 4, 0);
        e.write_u32(ebp + 8, len.to_bits());
        e.set_reg(RegisterX86::ESI, 0);
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5f61ac, 0x5f61c0);
        let bin = e.read_f32(edi + 0x38);
        assert_f32(bin, move_leg_speed(len, i64::from(count)), ctx);
    };

    case(&mut e, 100.0, 1000, "100 over 1 second");
    case(&mut e, 50.0, 1, "50 over a millisecond");
    case(&mut e, 0.0, 0, "0 over 0");
    for &len in &specials(&[1.0]) {
        for count in [0, 1, 1000, 0x7fff_ffff, 0x8000_0000, u32::MAX] {
            case(&mut e, len, count, "special length");
        }
    }
    for i in 0..5000u32 {
        let len = match i % 3 {
            0 => frand(&mut s, 0.0, 1e3),
            1 => frand(&mut s, 0.0, 1e-2),
            _ => frand_bits(&mut s).abs(),
        };
        let count = match i % 4 {
            0 => (next(&mut s) >> 8) % 5000,
            1 => (next(&mut s) >> 8) % 100 + 1,
            2 => next(&mut s),
            _ => 0,
        };
        case(&mut e, len, count, &format!("fuzz{i}"));
    }
}

#[test]
fn g3_transport_ms_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x5f50_a100u32;

    // The two conversions store the arguments of the next call at `esp` and `esp+4`, below the
    // `push`es; with `ecx = edx = 0` and `[ebp-0x10] = 0` the first is `[ebp-0xc]` and the second
    // `[ebp-0x18]`.
    let case = |e: &mut Emu, a: u32, b: u32, ctx: &str| {
        e.write_u32(ebp - 0xc, a);
        e.write_u32(ebp - 8, 0);
        e.write_u32(ebp - 0x10, 0);
        e.write_u32(ebp - 0x18, b);
        e.set_reg(RegisterX86::ECX, 0);
        e.set_reg(RegisterX86::EDX, 0);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5f51b7, 0x5f51ee);
        let bin1 = e.read_f32(SLICE_ESP - 0x10);
        let bin2 = e.read_f32(SLICE_ESP - 0x14);
        assert_f32(
            bin1,
            transport_ms_scale(i64::from(a)),
            &format!("{ctx} conv1"),
        );
        assert_f32(
            bin2,
            transport_ms_scale(i64::from(b)),
            &format!("{ctx} conv2"),
        );
    };

    case(&mut e, 1000, 500, "1000 gives 1.0, 500 gives 0.5");
    case(&mut e, 0, 0, "zeros");
    let edges = [
        0,
        1,
        999,
        1000,
        (1 << 24) - 1,
        1 << 24,
        (1 << 24) + 1,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_fffe,
        u32::MAX,
    ];
    for &a in &edges {
        for &b in &edges {
            case(&mut e, a, b, "edges");
        }
    }
    for i in 0..5000u32 {
        let a = match i % 3 {
            0 => (next(&mut s) >> 8) % 100_000,
            1 => next(&mut s),
            _ => (next(&mut s) >> 8) % 10,
        };
        let b = match i % 3 {
            0 => (next(&mut s) >> 8) % 100_000,
            1 => next(&mut s),
            _ => (next(&mut s) >> 8) % 10,
        };
        case(&mut e, a, b, &format!("fuzz{i}"));
    }
}

#[test]
fn g3_transport_leg_fraction() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // `0x5f8dc0` becomes `fld dword [slot]; ret 0x14`, so its call leaves the easing in `st0`.
    let slot = e.alloc(0x10);
    let mut island = vec![0xd9u8, 0x05];
    island.extend_from_slice(&(slot as u32).to_le_bytes());
    island.extend_from_slice(&[0xc2, 0x14, 0x00]);
    e.write(0x5f8dc0, &island);

    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let edi = e.alloc(0x80);
    let legbase = e.alloc(0x40);
    e.write_u32(edi + 0x30, legbase as u32);
    let mut s = 0x5f50_a200u32;
    let mut seen_lo = false;
    let mut seen_hi = false;
    let mut seen_mid = false;

    let mut case = |e: &mut Emu, easing: f32, addend: f32, divisor: f32, ctx: &str| {
        e.write_u32(slot, easing.to_bits());
        e.write_u32(ebp - 4, addend.to_bits());
        e.write_u32(legbase + 8, divisor.to_bits());
        e.set_reg(RegisterX86::ESI, 0);
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5f51ee, 0x5f5230);
        let bin = e.read_f32(ebp - 0x10);
        let ours = transport_leg_fraction(easing, addend, divisor);
        if bin.to_bits() == 0 {
            seen_lo = true;
        } else if bin.to_bits() == 0x3f80_0000 {
            seen_hi = true;
        } else {
            seen_mid = true;
        }
        assert_f32(
            bin,
            ours,
            &format!("{ctx}: easing={easing} addend={addend} divisor={divisor}"),
        );
    };

    case(&mut e, 0.5, 0.0, 1.0, "mid");
    case(&mut e, -1.0, 0.0, 1.0, "below 0");
    case(&mut e, 5.0, 0.0, 1.0, "above 1");
    case(&mut e, 0.0, 1.0, 2.0, "mid by the addend");
    case(&mut e, 1.0, 0.0, 1.0, "exactly 1");
    case(&mut e, next_down(1.0), 0.0, 1.0, "a step below 1");
    case(&mut e, next_up(1.0), 0.0, 1.0, "a step above 1");
    case(&mut e, -0.0, -0.0, 1.0, "negative zero");
    case(&mut e, 0.0, 0.0, 0.0, "0 over 0");
    case(&mut e, f32::NAN, 0.0, 1.0, "nan easing");
    let sp = specials(&[1.0]);
    for &easing in &sp {
        for &addend in &[0.0, -0.0, 1.0, -1.0, f32::NAN, f32::INFINITY] {
            for &divisor in &sp {
                case(&mut e, easing, addend, divisor, "special");
            }
        }
    }
    for i in 0..5000u32 {
        let easing = match i % 4 {
            0 => frand(&mut s, -0.5, 1.5),
            1 => frand(&mut s, 0.0, 1.0),
            2 => frand(&mut s, -2.0, 3.0),
            _ => frand_bits(&mut s),
        };
        let addend = match i % 3 {
            0 => 0.0,
            1 => frand(&mut s, -1.0, 1.0),
            _ => frand(&mut s, -0.2, 0.2),
        };
        let divisor = match i % 4 {
            0 => 1.0,
            1 => frand(&mut s, 0.1, 4.0),
            2 => frand(&mut s, -2.0, 2.0),
            _ => frand(&mut s, 0.5, 1.5),
        };
        case(&mut e, easing, addend, divisor, &format!("fuzz{i}"));
    }
    assert!(seen_lo, "the lower clamp never seen");
    assert!(seen_hi, "the upper clamp never seen");
    assert!(seen_mid, "the unclamped fraction never seen");
}

/// The pair a heading slice leaves at `[ebp-0x20]`, from the deltas at `[ebp-0xb0]`.
fn heading_pair(e: &mut Emu, ebp: u64, a: f32, b: f32, end: u64) -> [f32; 2] {
    e.write_u32(ebp - 0xb0, a.to_bits());
    e.write_u32(ebp - 0xac, b.to_bits());
    e.set_reg(RegisterX86::EBP, ebp);
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.run(0x5f5251, end);
    [e.read_f32(ebp - 0x20), e.read_f32(ebp - 0x1c)]
}

#[test]
fn g3_transport_heading() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0xc0;
    let mut s = 0x5f50_a300u32;
    let mut seen_norm = false;
    let mut seen_degen = false;

    // The slice ends at the `fpatan`; the pair is what the angle is taken from.
    let mut case = |e: &mut Emu, a: f32, b: f32, ctx: &str| {
        let bin = heading_pair(e, ebp, a, b, 0x5f528e);
        let (ours, _) = transport_heading(a, b);
        if bin[0].to_bits() == (-a).to_bits() && bin[1].to_bits() == (-b).to_bits() {
            seen_degen = true;
        } else {
            seen_norm = true;
        }
        assert_f32(bin[0], ours[0], &format!("{ctx} x_in (a={a} b={b})"));
        assert_f32(bin[1], ours[1], &format!("{ctx} y_in (a={a} b={b})"));
    };

    case(&mut e, 1.0, 0.0, "-1, 0");
    case(&mut e, 0.0, 1.0, "0, -1");
    case(&mut e, 1.0, 1.0, "diagonal");
    case(&mut e, 0.0, 0.0, "zero, below 2^-22");
    case(&mut e, -3.0, 4.0, "3, -4");
    let edge = NORM2_EPS;
    case(&mut e, edge, 0.0, "length exactly 2^-22");
    case(&mut e, next_down(edge), 0.0, "length a step below 2^-22");
    case(&mut e, next_up(edge), 0.0, "length a step above 2^-22");
    let sp = specials(&[edge]);
    for &a in &sp {
        for &b in &sp {
            case(&mut e, a, b, "special");
        }
    }
    for i in 0..5000u32 {
        let (a, b) = match i % 4 {
            0 => (frand(&mut s, -100.0, 100.0), frand(&mut s, -100.0, 100.0)),
            1 => (frand(&mut s, -1.0, 1.0), frand(&mut s, -1.0, 1.0)),
            2 => (frand(&mut s, -1e-7, 1e-7), frand(&mut s, -1e-7, 1e-7)),
            _ => (frand_bits(&mut s), frand_bits(&mut s)),
        };
        case(&mut e, a, b, &format!("fuzz{i}"));
    }
    assert!(seen_norm, "the normalised arm never seen");
    assert!(seen_degen, "the unchanged arm never seen");
}

#[test]
fn g3_transport_heading_angle() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0xc0;
    let edi = e.alloc(0x100);
    let mut s = 0x5f50_a301u32;

    // Through `fpatan` and the `fstp dword [edi+0x50]`; the copies to `[edi+0x44]` read the frame.
    let case = |e: &mut Emu, a: f32, b: f32| {
        e.set_reg(RegisterX86::EDI, edi);
        heading_pair(e, ebp, a, b, 0x5f52ae);
        let bin = e.read_f32(edi + 0x50);
        let (_, ours) = transport_heading(a, b);
        assert_f32(bin, ours, &format!("angle of a={a} b={b}"));
    };
    let sp = specials(&[NORM2_EPS]);
    for &a in &sp {
        for &b in &sp {
            case(&mut e, a, b);
        }
    }
    for i in 0..3000u32 {
        let (a, b) = match i % 3 {
            0 => (frand(&mut s, -100.0, 100.0), frand(&mut s, -100.0, 100.0)),
            1 => (frand(&mut s, -1e-7, 1e-7), frand(&mut s, -1e-7, 1e-7)),
            _ => (frand_bits(&mut s), frand_bits(&mut s)),
        };
        case(&mut e, a, b);
    }
}
