//! The deflection branch of `CMissile::ArriveMiss` (`0x61dd50`) in `src/unitbig3_fp.rs` against
//! the exe. Each test runs the slice of the function it covers, with the registers, frame and
//! fields it reads set directly, reading the stores it ends in or the status byte `fnstsw ax`
//! leaves:
//! - R1 `[0x61df13, 0x61df48)`, the gates `[0x61df13, 0x61df55)` and `[0x61df5b, 0x61df70)`, R3
//!   `[0x61df76, 0x61df9a)`, the random angle `[0x61df9a, 0x61dfe2)` with `CRandom::uint32` run
//!   for real, R5 `[0x61dfe9, 0x61e050)`, and R7 and R8 `[0x61e0a2, 0x61e158)`;
//! - the whole branch `[0x61df13, 0x61e158)` with the two axis-angle matrices `0x7be490` run for
//!   real.
//!
//! The first gate's eps edge is scanned at 64 steps either side. Each test adds NaN, the
//! infinities, both zeros and its thresholds, equal and a step either side, to its random inputs.
//!
//! Host double (`docs/METHOD.md#the-harness`): `b3_deflect_full` runs `fsincos` at `0x7be4f7`. The
//! emulator computes each in host `double`, so where its result reaches a compared value the host's
//! math stands in for the x87's, and the test shows the code around it.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::unitbig3_fp::*;
use common::*;

/// The CRandom global state pointer the deflect branch loads (`mov ecx, 0x882664`) before each
/// `call 0x4531e0`. Seeding `[0x882664]`/`[0x882668]` makes the RNG deterministic; the `.rdata`
/// table 0x802700 is mapped from the PE.
const RNG_STATE: u64 = 0x0088_2664;

// ---- shared helpers (the crate float idiom; per unith_fp.rs) ----------------

/// Seed the CRandom global state to a fixed pair so the two `0x4531e0` calls are deterministic.
fn seed_rng(e: &mut Emu, lo: u32, hi: u32) {
    e.write_u32(RNG_STATE, lo);
    e.write_u32(RNG_STATE + 4, hi);
}

#[test]
fn b3_deflect_length() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let ebxblk = e.alloc(0x40); // EBX; [ebx+0xc] = dir ptr
    let dir = e.alloc(0x10);
    e.write_u32(ebxblk + 0xc, dir as u32);
    let mut s = 0x61dd_5000u32;

    let case = |e: &mut Emu, d: [f32; 3], ctx: &str| {
        for (k, c) in d.iter().enumerate() {
            e.write_u32(dir + 4 * k as u64, c.to_bits());
        }
        e.set_reg(RegisterX86::EBX, ebxblk);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x61df13, 0x61df48);
        let (nx, ny, nz, len) = deflect_length(d);
        assert_f32(e.read_f32(ebp - 0xc), nx, &format!("{ctx} nx"));
        assert_f32(e.read_f32(ebp - 0x8), ny, &format!("{ctx} ny"));
        assert_f32(e.read_f32(ebp - 0x4), nz, &format!("{ctx} nz"));
        assert_f32(e.read_f32(ebp - 0x18), len, &format!("{ctx} len"));
    };

    case(&mut e, [3.0, 4.0, 0.0], "3-4-5 → len 5");
    case(&mut e, [1.0, 2.0, 2.0], "1-2-2 → len 3");
    case(&mut e, [0.0, 0.0, 0.0], "zero → len 0");
    case(&mut e, [-1.0, -1.0, -1.0], "neg → len sqrt3");
    for i in 0..4000u32 {
        let d = match i % 3 {
            0 => [
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
            ],
            1 => [
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
            ],
            _ => [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
        };
        case(&mut e, d, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut d = [1.0f32, 2.0, 3.0];
            d[k] = v;
            case(&mut e, d, "special");
        }
    }
}

#[test]
fn b3_deflect_gates() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let ebxblk = e.alloc(0x40);
    let dir = e.alloc(0x10);
    e.write_u32(ebxblk + 0xc, dir as u32);
    let mut s = 0x61df_4800u32;
    let mut g1_abort = false;
    let mut g1_ok = false;
    let mut g2_abort = false;
    let mut g2_ok = false;

    // GATE 1 — run R1's length compute then gate1's compare (so len is the real f32 sqrt).
    let mut gate1 = |e: &mut Emu, d: [f32; 3], ctx: &str| {
        for (k, c) in d.iter().enumerate() {
            e.write_u32(dir + 4 * k as u64, c.to_bits());
        }
        e.set_reg(RegisterX86::EBX, ebxblk);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x61df13, 0x61df52); // R1 + fabs + fcomp eps (ends just before `test ah,5`)
        e.run(0x61df52, 0x61df55); // execute `test ah,0x5`
        let m = ah(e) & 0x5;
        // jnp abort: jnp TAKEN on ODD parity (C0 alone) ⟺ |len|<eps. abort ⟺ ODD.
        let bits = (m & 0x01).count_ones() + (m & 0x04).count_ones();
        let bin_abort = bits % 2 == 1;
        if bin_abort {
            g1_abort = true;
        } else {
            g1_ok = true;
        }
        let (nx, ny, nz, len) = deflect_length(d);
        assert_eq!(
            bin_abort,
            deflect_len_degenerate([nx, ny, nz]),
            "{ctx} gate1: abort={bin_abort} ah&5={m:#04x} len={len}"
        );
    };

    gate1(&mut e, [3.0, 4.0, 0.0], "len5 → ok");
    gate1(&mut e, [0.0, 0.0, 0.0], "len0 → abort");
    gate1(&mut e, [1e-8, 0.0, 0.0], "tiny → abort");
    // sqrt straddles eps=2^-22 (c = eps/√3 ≈ 1.3765e-7) — the off-boundary fuzz below cannot hit it.
    let eps = f32::from_bits(0x3480_0000); // 2^-22
    let c0 = (f64::from(eps) / 3.0f64.sqrt()) as f32;
    for k in -64i32..=64 {
        let c = f32::from_bits((c0.to_bits() as i32 + k) as u32);
        gate1(&mut e, [c, c, c], &format!("eps-iso k={k}"));
        gate1(&mut e, [-c, c, -c], &format!("eps-iso signs k={k}"));
    }
    for i in 0..2500u32 {
        let scale = match i % 3 {
            0 => 1e-7,
            1 => 1.0,
            _ => 100.0,
        };
        let d = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        gate1(&mut e, d, &format!("g1-fuzz{i}"));
    }
    assert!(g1_abort, "gate1 abort (|len|<eps) never seen");
    assert!(g1_ok, "gate1 proceed never seen");

    // GATE 2 — set [ebp-0x18]=len and [ebp-0x4]=nz directly; run [0x61df5b, 0x61df6d) (compute) +
    // the `test ah,5`. abort ⟺ |len−|nz|| < eps (ODD parity).
    let mut gate2 = |e: &mut Emu, len: f32, nz: f32, ctx: &str| {
        e.write_u32(ebp - 0x18, len.to_bits());
        e.write_u32(ebp - 0x4, nz.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x61df5b, 0x61df6d); // fld nz;fabs;fsubr len;fabs;fcomp eps
        e.run(0x61df6d, 0x61df70); // test ah,0x5
        let m = ah(e) & 0x5;
        let bits = (m & 0x01).count_ones() + (m & 0x04).count_ones();
        let bin_abort = bits % 2 == 1;
        if bin_abort {
            g2_abort = true;
        } else {
            g2_ok = true;
        }
        assert_eq!(
            bin_abort,
            deflect_axis_degenerate(len, nz),
            "{ctx} gate2: abort={bin_abort} ah&5={m:#04x} len={len} nz={nz}"
        );
    };

    gate2(&mut e, 5.0, 0.0, "len5 nz0 → ok");
    gate2(&mut e, 5.0, 5.0, "len==|nz| → abort");
    gate2(&mut e, 5.0, -5.0, "len==|nz| neg → abort");
    gate2(&mut e, 5.0, 4.9999, "near-z → straddle");
    for i in 0..2500u32 {
        let len = frand(&mut s, 0.1, 50.0);
        let nz = match i % 3 {
            0 => len + frand(&mut s, -1e-5, 1e-5), // straddle len==|nz|
            1 => -(len + frand(&mut s, -1e-5, 1e-5)),
            _ => frand(&mut s, -50.0, 50.0),
        };
        gate2(&mut e, len, nz, &format!("g2-fuzz{i}"));
    }
    assert!(g2_abort, "gate2 abort (near-z) never seen");
    assert!(g2_ok, "gate2 proceed never seen");
}

#[test]
fn b3_deflect_normalize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x61df_7600u32;

    let case = |e: &mut Emu, n: [f32; 3], len: f32, ctx: &str| {
        e.write_u32(ebp - 0xc, n[0].to_bits());
        e.write_u32(ebp - 0x8, n[1].to_bits());
        e.write_u32(ebp - 0x4, n[2].to_bits());
        e.write_u32(ebp - 0x18, len.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x61df76, 0x61df9a);
        let nn = deflect_normalize(n, len);
        assert_f32(e.read_f32(ebp - 0xc), nn[0], &format!("{ctx} nx'"));
        assert_f32(e.read_f32(ebp - 0x8), nn[1], &format!("{ctx} ny'"));
        assert_f32(e.read_f32(ebp - 0x4), nn[2], &format!("{ctx} nz'"));
    };

    case(&mut e, [3.0, 4.0, 0.0], 5.0, "3-4-0 / 5");
    case(&mut e, [1.0, 2.0, 2.0], 3.0, "1-2-2 / 3");
    case(&mut e, [1.0, 1.0, 1.0], 1.0, "/1 identity");
    for i in 0..4000u32 {
        let n = [
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, -100.0, 100.0),
        ];
        let len = match i % 3 {
            0 => (f64::from(n[0]) * f64::from(n[0])
                + f64::from(n[1]) * f64::from(n[1])
                + f64::from(n[2]) * f64::from(n[2]))
            .sqrt() as f32,
            1 => frand(&mut s, 0.01, 200.0),
            _ => frand_bits(&mut s).abs().max(1e-6),
        };
        case(&mut e, n, len, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, [v, 2.0, 3.0], 3.0, "special x");
        case(&mut e, [1.0, v, 3.0], 3.0, "special y");
        case(&mut e, [1.0, 2.0, v], 3.0, "special z");
        case(&mut e, [1.0, 2.0, 3.0], v, "special len");
    }
}

#[test]
fn b3_deflect_random_angle() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x61df_9a00u32;
    let mut seen_neg = false;
    let mut seen_pos = false;

    let mut case = |e: &mut Emu, lo: u32, hi: u32, ctx: &str| {
        // 1) Capture the exact rand_u32 the binary draws from this seed (real 0x4531e0 bytes).
        seed_rng(e, lo, hi);
        let rand_u32 = e.call(0x4531e0, RNG_STATE as u32, 0, &[]);
        // 2) Re-seed (the call advanced the state) and run the angle slice, which itself draws.
        seed_rng(e, lo, hi);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        // [0x61df9a, 0x61dfe2): call 0x4531e0 ; build r ; ±base ; fld π;fmul 0.5;fmul; fstp [esp].
        e.run(0x61df9a, 0x61dfe2);
        let bin_angle = e.read_f32(SLICE_ESP - 8); // [esp] after push 0x1 + push ecx = the angle
        let sign = (rand_u32 as i32) < 0;
        if sign {
            seen_neg = true;
        } else {
            seen_pos = true;
        }
        assert_f32(
            bin_angle,
            deflect_random_angle(rand_u32),
            &format!("{ctx}: rand={rand_u32:#010x} sign_neg={sign}"),
        );
    };

    case(&mut e, 0x1234_5678, 0x9abc_def0, "seed A");
    case(&mut e, 0xdead_beef, 0x0bad_f00d, "seed B");
    case(&mut e, 1, 2, "seed small");
    case(&mut e, 0xffff_ffff, 0xffff_ffff, "seed all-ones");
    for i in 0..3000u32 {
        let lo = next(&mut s);
        let hi = next(&mut s);
        case(&mut e, lo, hi, &format!("fuzz{i}"));
    }
    assert!(seen_neg, "angle sign-neg (rand<0 → 2-r) never seen");
    assert!(seen_pos, "angle sign-pos (rand>=0 → r-2) never seen");
}

#[test]
fn b3_deflect_axis_from_mat() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x61df_e900u32;

    let case = |e: &mut Emu, m1: [f32; 9], nxp: f32, nyp: f32, ctx: &str| {
        // M1[k] at [ebp-0x50 + 4k].
        for k in 0..9u64 {
            e.write_u32(ebp - 0x50 + 4 * k, m1[k as usize].to_bits());
        }
        e.write_u32(ebp - 0xc, nxp.to_bits()); // nx'
        e.write_u32(ebp - 0x8, nyp.to_bits()); // ny'
        e.write_u32(ebp - 0x24, 0); // V2 slot (the ·0.0 term)
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x61dfe9, 0x61e050);
        let ax = deflect_axis_from_mat(m1, nxp, nyp);
        // The three rows are pushed R0,R1,R2 (R2 on top) and popped by 3× fstp in slot order
        // [ebp-0x2c],[ebp-0x28],[ebp-0x24] — so the store REVERSES: [ebp-0x24]=R0, [ebp-0x28]=R1,
        // [ebp-0x2c]=R2. (The 2nd 0x7be490 then reads the axis as (R2,R1,R0) from [ebp-0x2c]↑.)
        assert_f32(e.read_f32(ebp - 0x24), ax[0], &format!("{ctx} R0"));
        assert_f32(e.read_f32(ebp - 0x28), ax[1], &format!("{ctx} R1"));
        assert_f32(e.read_f32(ebp - 0x2c), ax[2], &format!("{ctx} R2"));
    };

    case(
        &mut e,
        [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        0.6,
        0.8,
        "identity M1",
    );
    case(
        &mut e,
        [0.5, -0.5, 0.2, 0.1, 0.9, -0.3, 0.4, 0.2, 0.7],
        0.6,
        0.8,
        "general M1",
    );
    for i in 0..4000u32 {
        let mut m1 = [0.0f32; 9];
        for v in m1.iter_mut() {
            *v = frand(&mut s, -1.5, 1.5);
        }
        let nxp = frand(&mut s, -1.0, 1.0);
        let nyp = frand(&mut s, -1.0, 1.0);
        case(&mut e, m1, nxp, nyp, &format!("fuzz{i}"));
    }
    let ident = [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    for &v in &specials(&[]) {
        for k in 0..9 {
            let mut m = ident;
            m[k] = v;
            case(&mut e, m, 0.5, 0.25, "special matrix");
        }
        case(&mut e, ident, v, 0.25, "special nx");
        case(&mut e, ident, 0.5, v, "special ny");
    }
}

#[test]
fn b3_deflect_transform_and_move() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0xc0; // high enough that [ebp-0x84]=frame+0x3c is mapped (M2 spans -0x84..-0x64)
    let this = e.alloc(0x80); // ESI; speed@+0x1c, origin@+0x20/24/28, pos@+0x2c/30/34
    let mut s = 0x61e0_a200u32;

    let case =
        |e: &mut Emu, m2: [f32; 9], dir: [f32; 3], speed: f32, origin: [f32; 3], ctx: &str| {
            for k in 0..9u64 {
                e.write_u32(ebp - 0x84 + 4 * k, m2[k as usize].to_bits());
            }
            e.write_u32(ebp - 0xc, dir[0].to_bits());
            e.write_u32(ebp - 0x8, dir[1].to_bits());
            e.write_u32(ebp - 0x4, dir[2].to_bits());
            e.write_u32(this + 0x1c, speed.to_bits());
            e.write_u32(this + 0x20, origin[0].to_bits());
            e.write_u32(this + 0x24, origin[1].to_bits());
            e.write_u32(this + 0x28, origin[2].to_bits());
            e.set_reg(RegisterX86::ESI, this);
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.run(0x61e0a2, 0x61e158);
            let (ns, pos) = deflect_transform_and_move(m2, dir, speed, origin);
            assert_f32(e.read_f32(this + 0x1c), ns, &format!("{ctx} new_speed"));
            assert_f32(e.read_f32(this + 0x2c), pos[0], &format!("{ctx} pos.x"));
            assert_f32(e.read_f32(this + 0x30), pos[1], &format!("{ctx} pos.y"));
            assert_f32(e.read_f32(this + 0x34), pos[2], &format!("{ctx} pos.z"));
        };

    case(
        &mut e,
        [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        [0.6, 0.8, 0.0],
        10.0,
        [100.0, 200.0, 50.0],
        "identity M2",
    );
    case(
        &mut e,
        [0.5, -0.5, 0.2, 0.1, 0.9, -0.3, 0.4, 0.2, 0.7],
        [0.36, 0.48, 0.8],
        25.0,
        [-100.0, 33.0, -7.0],
        "general M2",
    );
    // pos.y (live step) diverge from pos.x/pos.z (f32-spilled step) under an all-f32 mistranscription.
    case(
        &mut e,
        [
            0.333_333, 0.666_667, 0.123_457, 0.987_654, 0.111_111, 0.555_555, 0.246_802, 0.864_209,
            0.135_792,
        ],
        [0.577_35, 0.577_35, 0.577_35],
        1.234_567_8e6,
        [8_388_609.0, 8_388_611.0, 16_777_217.0],
        "live-vs-spilled witness",
    );
    // The sum of the x row is `(m3·ny + m6·nz) + m0·nx`. Here `m0·nx = 1 + 2^-11 + 2^-24` is an
    // exact tie for `f32`, and `m3·ny` and `m6·nz` are each half an ulp of `f64` at 1.0: added to
    // it one at a time they vanish and the tie rounds down, added to each other first they round it up.
    let big = 1.0 + 2.0f32.powi(-12);
    let tiny = 2.0f32.powi(-27);
    let mut m = [0.0f32; 9];
    m[0] = big;
    m[3] = tiny;
    m[6] = tiny;
    case(
        &mut e,
        m,
        [big, 2.0f32.powi(-26), 2.0f32.powi(-26)],
        2.857_143,
        [0.0; 3],
        "x row sum order",
    );
    for i in 0..4000u32 {
        let mut m2 = [0.0f32; 9];
        for v in m2.iter_mut() {
            *v = frand(&mut s, -1.0, 1.0);
        }
        let dir = [
            frand(&mut s, -1.0, 1.0),
            frand(&mut s, -1.0, 1.0),
            frand(&mut s, -1.0, 1.0),
        ];
        let speed = frand(&mut s, 0.0, 5000.0);
        let origin = match i % 2 {
            0 => [
                frand(&mut s, -500.0, 500.0),
                frand(&mut s, -500.0, 500.0),
                frand(&mut s, -500.0, 500.0),
            ],
            _ => [
                frand(&mut s, -2e7, 2e7),
                frand(&mut s, -2e7, 2e7),
                frand(&mut s, -2e7, 2e7),
            ],
        };
        case(&mut e, m2, dir, speed, origin, &format!("fuzz{i}"));
    }
    let ident = [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    for &v in &specials(&[]) {
        for k in 0..9 {
            let mut m = ident;
            m[k] = v;
            case(
                &mut e,
                m,
                [0.5, 0.25, 0.125],
                10.0,
                [1.0, 2.0, 3.0],
                "special matrix",
            );
        }
        for k in 0..3 {
            let mut d = [0.5f32, 0.25, 0.125];
            d[k] = v;
            case(&mut e, ident, d, 10.0, [1.0, 2.0, 3.0], "special dir");
            let mut o = [1.0f32, 2.0, 3.0];
            o[k] = v;
            case(&mut e, ident, [0.5, 0.25, 0.125], 10.0, o, "special origin");
        }
        case(
            &mut e,
            ident,
            [0.5, 0.25, 0.125],
            v,
            [1.0, 2.0, 3.0],
            "special speed",
        );
    }
}

#[test]
fn b3_deflect_full() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let this = e.alloc(0x80); // ESI
    let ebxblk = e.alloc(0x40); // EBX; [ebx+0xc] = dir ptr
    let dir = e.alloc(0x10);
    e.write_u32(ebxblk + 0xc, dir as u32);
    let axis_buf = e.alloc(0x40);
    let mat_buf = e.alloc(0x40);
    let mut s = 0x61dd_5050u32;
    let mut count = 0u32;

    // Reproduce 0x7be490(out=ecx, axis=edx, angle=[esp+4], normflag=[esp+8]) byte-for-byte.
    let run_mat = |e: &mut Emu, axis: [f32; 3], angle: f32, normflag: u32| -> [f32; 9] {
        for k in 0..3u64 {
            e.write_u32(axis_buf + 4 * k, axis[k as usize].to_bits());
        }
        e.call(
            0x7be490,
            mat_buf as u32,
            axis_buf as u32,
            &[angle.to_bits(), normflag],
        );
        let mut m = [0.0f32; 9];
        for (k, slot) in m.iter_mut().enumerate() {
            *slot = e.read_f32(mat_buf + 4 * k as u64);
        }
        m
    };

    let mut case =
        |e: &mut Emu, d: [f32; 3], speed: f32, origin: [f32; 3], lo: u32, hi: u32, ctx: &str| {
            // ---- our transcription, composing the regions exactly as the binary sequences them ----
            let (nx, ny, nz, len) = deflect_length(d);
            // R2 gates must not abort for a valid input (assert the binary agrees below).
            let nn = deflect_normalize([nx, ny, nz], len);
            // R4 angle #1 — draw rand #1 from the seed (real RNG bytes).
            seed_rng(e, lo, hi);
            let rand1 = e.call(0x4531e0, RNG_STATE as u32, 0, &[]);
            let angle1 = deflect_random_angle(rand1);
            // 0x7be490 #1: axis = the normalized dir (nx',ny',nz'), normflag=1 (push 0x1 at 0x61dfce).
            let m1 = run_mat(e, [nn[0], nn[1], nn[2]], angle1, 1);
            // R5 axis from M1 — returns [R0,R1,R2]; the binary stores them REVERSED into the contiguous
            // slots [ebp-0x2c]=R2,[ebp-0x28]=R1,[ebp-0x24]=R0, and the 2nd 0x7be490 reads its axis vec
            // from [ebp-0x2c]↑ — so the axis the matrix builder sees in memory order is (R2, R1, R0).
            let axis = deflect_axis_from_mat(m1, nn[0], nn[1]);
            let axis2 = [axis[2], axis[1], axis[0]];
            // R6 angle #2 — draw rand #2 (state advanced once by the first call's real draw).
            let rand2 = e.call(0x4531e0, RNG_STATE as u32, 0, &[]);
            let angle2 = deflect_random_angle(rand2);
            // 0x7be490 #2: axis = axis2(memory order R2,R1,R0), normflag=0 (push 0x0 at 0x61e084).
            let m2 = run_mat(e, axis2, angle2, 0);
            // R7+R8 transform + move.
            let (ours_speed, ours_pos) =
                deflect_transform_and_move(m2, [nn[0], nn[1], nn[2]], speed, origin);

            // ---- the binary: run the WHOLE deflect branch with the same seed ----
            seed_rng(e, lo, hi);
            for (k, c) in d.iter().enumerate() {
                e.write_u32(dir + 4 * k as u64, c.to_bits());
            }
            e.write_u32(this + 0x1c, speed.to_bits());
            e.write_u32(this + 0x20, origin[0].to_bits());
            e.write_u32(this + 0x24, origin[1].to_bits());
            e.write_u32(this + 0x28, origin[2].to_bits());
            e.set_reg(RegisterX86::ESI, this);
            e.set_reg(RegisterX86::EBX, ebxblk);
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.run(0x61df13, 0x61e158);

            assert_f32(e.read_f32(this + 0x1c), ours_speed, &format!("{ctx} speed"));
            assert_f32(
                e.read_f32(this + 0x2c),
                ours_pos[0],
                &format!("{ctx} pos.x"),
            );
            assert_f32(
                e.read_f32(this + 0x30),
                ours_pos[1],
                &format!("{ctx} pos.y"),
            );
            assert_f32(
                e.read_f32(this + 0x34),
                ours_pos[2],
                &format!("{ctx} pos.z"),
            );
            count += 1;
        };

    case(
        &mut e,
        [3.0, 4.0, 1.0],
        30.0,
        [100.0, 200.0, 50.0],
        0x1234_5678,
        0x9abc_def0,
        "A",
    );
    case(
        &mut e,
        [10.0, -5.0, 2.0],
        12.5,
        [-50.0, 0.0, 7.0],
        0xdead_beef,
        0x0bad_f00d,
        "B",
    );
    case(
        &mut e,
        [1.0, 1.0, 0.5],
        100.0,
        [0.0, 0.0, 0.0],
        0x0f0f_0f0f,
        0xf0f0_f0f0,
        "C",
    );
    for i in 0..1500u32 {
        // A valid, non-degenerate, non-z-aligned direction (|len|>>eps and |len−|nz||>>eps).
        let d = [
            frand(&mut s, 1.0, 50.0) * if i & 1 == 0 { 1.0 } else { -1.0 },
            frand(&mut s, 1.0, 50.0) * if i & 2 == 0 { 1.0 } else { -1.0 },
            frand(&mut s, -20.0, 20.0),
        ];
        let speed = frand(&mut s, 0.1, 5000.0);
        let origin = [
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
        ];
        let lo = next(&mut s);
        let hi = next(&mut s);
        case(&mut e, d, speed, origin, lo, hi, &format!("fuzz{i}"));
    }
    assert!(count > 1000, "whole-branch cases never ran");
}

/// The two degenerate gates of the deflection, `|len| < eps` and `| len - |nz| | < eps` with
/// `eps = 2^-22`, with NaN, infinities, zeros and the threshold, equal and a step either side. The
/// first gate runs the length slice and reads `test ah,5`; the second sets `len` and `nz` directly.
#[test]
fn b3_deflect_gates_special_values() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let ebxblk = e.alloc(0x40);
    let dir = e.alloc(0x10);
    e.write_u32(ebxblk + 0xc, dir as u32);
    let odd = |m: u8| ((m & 0x01).count_ones() + (m & 0x04).count_ones()) % 2 == 1;
    let eps = EPS_2EM22;

    let gate1 = |e: &mut Emu, d: [f32; 3], ctx: &str| {
        for (k, c) in d.iter().enumerate() {
            e.write_u32(dir + 4 * k as u64, c.to_bits());
        }
        e.set_reg(RegisterX86::EBX, ebxblk);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x61df13, 0x61df52);
        e.run(0x61df52, 0x61df55);
        let bin_abort = odd(ah(e) & 0x5);
        let (nx, ny, nz, _len) = deflect_length(d);
        assert_eq!(
            bin_abort,
            deflect_len_degenerate([nx, ny, nz]),
            "{ctx} gate1 {d:?}"
        );
    };
    for l in [eps, next_up(eps), next_down(eps)] {
        gate1(&mut e, [l, 0.0, 0.0], "eps along x");
        gate1(&mut e, [0.0, -l, 0.0], "eps along y");
    }
    for &v in &specials(&[eps]) {
        for k in 0..3 {
            let mut d = [0.0f32; 3];
            d[k] = v;
            gate1(&mut e, d, "special");
        }
    }

    let gate2 = |e: &mut Emu, len: f32, nz: f32, ctx: &str| {
        e.write_u32(ebp - 0x18, len.to_bits());
        e.write_u32(ebp - 0x4, nz.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x61df5b, 0x61df6d);
        e.run(0x61df6d, 0x61df70);
        let bin_abort = odd(ah(e) & 0x5);
        assert_eq!(
            bin_abort,
            deflect_axis_degenerate(len, nz),
            "{ctx} gate2 len={len} nz={nz}"
        );
    };
    // `len - |nz| == eps` exactly (len = 4 + 2^-22 against nz = 4) and a step either side.
    for l in [
        4.0f32 + eps,
        next_up(4.0 + eps),
        next_down(4.0 + eps),
        4.0 - eps,
    ] {
        gate2(&mut e, l, 4.0, "eps edge");
        gate2(&mut e, l, -4.0, "eps edge neg");
    }
    for &v in &specials(&[eps, 4.0]) {
        gate2(&mut e, v, 4.0, "special len");
        gate2(&mut e, 4.0, v, "special nz");
    }
}
