//! The regions of `CGUnit_C::UpdateAutoMove` (`0x6106b0`) in `src/unitbig2_fp.rs` against the exe.
//! Each runs the slice of the function it covers, with the registers, frame and fields it reads
//! set directly, reading the store it ends in or the status byte `fnstsw ax` leaves:
//! - R1 `[0x61075b, 0x6107db)`, R2 `[0x61080f, 0x610823)`, R3 `[0x610872, 0x61087f)`,
//!   R4 `[0x61088a, 0x610897)`, R5 `[0x610915, 0x610929)`, R6 `[0x6109ae, 0x6109b7)`,
//!   R7 `[0x6109ba, 0x6109cb)`, R8 `[0x610a48, 0x610a56)`, R9 `[0x610aa2, 0x610aee)`;
//! - R10, entered through a scratch island that loads `scaled` and jumps to `0x610b07`;
//! - R11 `[0x610bd7, 0x610c16)` for the value and `[0x610bd7, 0x610c1f)` for the gate;
//! - R12 `[0x610c85, 0x610c9a)`, through the exe's `fcos`, and R13 `[0x610cb0, 0x610d28)`.
//!
//! Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step either
//! side, to its random inputs.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::unitbig2_fp::*;
use common::*;

// ---- shared helpers (the crate float idiom; per unitm3_fp.rs) --------------

/// Parity of the masked AH bits (the x86 `jp`/`jnp` PF predicate: PF=1 ⟺ even number of set bits).
/// Returns `true` for ODD parity (so `jp` would not be taken).
fn odd_parity(m: u8) -> bool {
    m.count_ones() % 2 == 1
}

/// Install an x87 float-returning vtable getter: a `fld dword[slot]; ret` island the synthesized
/// vtable slot points at (the `unitm5_fp` vtbl-float-stub idiom). Writing the desired st0 float into
/// `slot` before a `run` makes the real `call [reg+off]` land our chosen orient. Returns
/// `(island_va, slot_va)`.
fn install_x87_getter(e: &mut Emu) -> (u64, u64) {
    let slot = e.alloc(0x10);
    let island = e.alloc(0x10);
    // d9 05 <slot LE32> c3  =  fld dword ptr [slot] ; ret
    let mut bytes = vec![0xd9u8, 0x05];
    bytes.extend_from_slice(&(slot as u32).to_le_bytes());
    bytes.push(0xc3);
    e.write(island, &bytes);
    (island, slot)
}

/// The movement-block timer `0x7c4c90` — a `__thiscall`(ecx) returning a float on st0. Patch it to
/// `fld dword[slot]; ret 4` and return the slot the test writes the timer into. (`.text` is
/// `Prot::ALL`; the call site pushes one arg then `call`, so the stub cleans 4 bytes.)
const TIMER_FN: u64 = 0x7c4c90;
fn install_timer_stub(e: &mut Emu) -> u64 {
    let slot = e.alloc(0x10);
    // d9 05 <slot LE32> c2 04 00  =  fld dword ptr [slot] ; ret 4
    let mut bytes = vec![0xd9u8, 0x05];
    bytes.extend_from_slice(&(slot as u32).to_le_bytes());
    bytes.extend_from_slice(&[0xc2, 0x04, 0x00]);
    e.write(TIMER_FN, &bytes);
    slot
}

#[test]
fn b2_arrival_test_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x6106_b001u32;
    let mut seen_stop = false;
    let mut seen_go = false;

    let mut case = |e: &mut Emu, sp: [f32; 3], pr: [f32; 3], tg: [f32; 3], ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(ebp - 0x38 + 4 * k, sp[k as usize].to_bits()); // self.x/y/z
            e.write_u32(ebp - 0x50 + 4 * k, pr[k as usize].to_bits()); // pred.x/y/z
            e.write_u32(ebp - 0x2c + 4 * k, tg[k as usize].to_bits()); // tgt.x/y/z
        }
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x61075b, 0x6107db);
        let m = ah(e) & 0x5;
        // jp 0x6107f3 (continue): jp taken (EVEN parity of C0|C2) → continue; not taken (ODD, C0
        // alone) → STOP. So stop ⟺ ODD parity ⟺ d2²<d1² ordered.
        let bin_stop = odd_parity(m);
        if bin_stop {
            seen_stop = true;
        } else {
            seen_go = true;
        }
        let (_d1, _d2, our_stop) = arrival_test(sp, pr, tg);
        assert_eq!(
            bin_stop, our_stop,
            "{ctx}: bin_stop={bin_stop} ah&5={m:#04x}"
        );
    };

    case(
        &mut e,
        [0.0; 3],
        [1.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        "pred closer → continue",
    );
    case(
        &mut e,
        [0.0; 3],
        [11.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        "pred past (1 yd) → continue (d2>d1? 1<100 → stop)",
    );
    // Deterministic boundary: d1² == d2² exactly (pred mirrors self about the target on x).
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [20.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        "d1²==d2²==100 → continue (==)",
    );
    case(
        &mut e,
        [5.0, 0.0, 0.0],
        [5.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        "self==pred → d1²==d2² → continue",
    );
    for i in 0..5000u32 {
        let tg = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        let sp = [
            tg[0] + frand(&mut s, -20.0, 20.0),
            tg[1] + frand(&mut s, -20.0, 20.0),
            tg[2] + frand(&mut s, -20.0, 20.0),
        ];
        let pr = match i % 3 {
            0 => [
                sp[0] + frand(&mut s, -2.0, 2.0),
                sp[1] + frand(&mut s, -2.0, 2.0),
                sp[2] + frand(&mut s, -2.0, 2.0),
            ],
            1 => [
                tg[0] + frand(&mut s, -20.0, 20.0),
                tg[1] + frand(&mut s, -20.0, 20.0),
                tg[2] + frand(&mut s, -20.0, 20.0),
            ],
            _ => [
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
            ],
        };
        case(&mut e, sp, pr, tg, &format!("fuzz{i}"));
    }
    // The predicted position on the unit (`d2 == d1`) and a step either side, then the specials.
    for x in [0.0f32, next_up(0.0), next_down(0.0)] {
        case(
            &mut e,
            [1.0, 2.0, 3.0],
            [1.0 + x, 2.0, 3.0],
            [4.0, 6.0, 8.0],
            "equal",
        );
    }
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut a = [1.0f32, 2.0, 3.0];
            a[k] = v;
            case(&mut e, a, [1.5, 2.5, 3.5], [4.0, 6.0, 8.0], "special self");
            case(&mut e, [1.0, 2.0, 3.0], a, [4.0, 6.0, 8.0], "special pred");
            case(
                &mut e,
                [1.0, 2.0, 3.0],
                [1.5, 2.5, 3.5],
                a,
                "special target",
            );
        }
    }
    assert!(seen_stop, "stop (d2²<d1²) arm never seen");
    assert!(seen_go, "continue arm never seen");
}

#[test]
fn b2_delta_time_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x6106_b002u32;

    let case = |e: &mut Emu, delta: i64, ctx: &str| {
        let d = delta as u64;
        e.write_u32(ebp - 0x1c, (d & 0xffff_ffff) as u32);
        e.write_u32(ebp - 0x18, (d >> 32) as u32);
        e.set_reg(RegisterX86::EDI, 0); // the [0xc4da68] store value (irrelevant to the FP)
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x61080f, 0x610823);
        let bin = e.read_f32(ebp - 0x10);
        assert_f32(bin, delta_time(delta), ctx);
    };

    case(&mut e, 0, "delta 0 → 0");
    case(&mut e, 16, "16 ms → 0.016");
    case(&mut e, -16, "−16 ms → 0.016 (fabs)");
    case(&mut e, 1000, "1000 ms → 1.0");
    case(&mut e, 33, "33 ms");
    for i in 0..3000u32 {
        // The binary builds `delta` from a 32-bit subtraction with a 0 high dword (so the i64 is the
        // sign-extended 32-bit difference). Fuzz over the 32-bit range as a signed delta.
        let delta = i64::from((next(&mut s) >> 8) as i32 - 0x80_0000);
        case(&mut e, delta, &format!("fuzz{i}"));
    }
    for d in [
        0i64,
        1,
        -1,
        i64::MAX,
        i64::MIN,
        1 << 40,
        -(1 << 40),
        0xffff_ffff,
    ] {
        case(&mut e, d, "extremes");
    }
}

#[test]
fn b2_heading_add_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let (island, slot) = install_x87_getter(&mut e);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let this = e.alloc(0x40); // ESI; [esi] = vtbl
    let vtbl = e.alloc(0x40);
    e.write_u32(this, vtbl as u32);
    e.write_u32(vtbl + 0x18, island as u32);
    let mut s = 0x6106_b003u32;

    let case = |e: &mut Emu, orient: f32, arg: f32, ctx: &str| {
        e.write_u32(slot, orient.to_bits());
        e.write_u32(ebp + 0xc, arg.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x610872, 0x61087f);
        let bin = e.read_f32(ebp - 0xc);
        assert_f32(bin, heading_add(orient, arg), ctx);
    };

    case(&mut e, 1.0, 2.0, "1+2");
    case(&mut e, -1.5, 0.25, "neg");
    case(&mut e, 0.0, 0.0, "zero");
    for i in 0..3000u32 {
        let (orient, arg) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (frand(&mut s, -6.3, 6.3), frand(&mut s, -6.3, 6.3))
        };
        case(&mut e, orient, arg, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, v, 1.5, "special first");
        case(&mut e, 1.5, v, "special second");
    }
}

#[test]
fn b2_turn_step_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x6106_b004u32;

    let case = |e: &mut Emu, turn_rate: f32, dt: f32, ctx: &str| {
        e.write_u32(0xc4d93c, turn_rate.to_bits());
        e.write_u32(ebp - 0x10, dt.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x61088a, 0x610897);
        let bin = e.read_f32(ebp - 0x18);
        assert_f32(bin, turn_step(turn_rate, dt), ctx);
    };

    case(&mut e, 3.0, 0.016, "3·0.016");
    case(&mut e, 0.0, 1.0, "0");
    case(&mut e, -2.5, 0.033, "neg rate");
    for i in 0..3000u32 {
        let (turn_rate, dt) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (frand(&mut s, -10.0, 10.0), frand(&mut s, 0.0, 0.2))
        };
        case(&mut e, turn_rate, dt, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, v, 1.5, "special first");
        case(&mut e, 1.5, v, "special second");
    }
}

#[test]
fn b2_rng_jitter_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x6106_b005u32;
    let mut seen_lo = false;
    let mut seen_hi = false;

    let mut case = |e: &mut Emu, rand_u32: u32, arg: f32, ctx: &str| {
        let r = f32::from_bits((rand_u32 & 0x007f_ffff) | 0x3f80_0000);
        e.write_u32(ebp - 0x18, r.to_bits());
        e.write_u32(ebp + 8, arg.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x610915, 0x610929);
        let m = ah(e) & 0x5;
        // jp 0x610935 (skip): jp taken (EVEN) → skip; not taken (ODD, C0 alone) → clear flag.
        let bin_clear = odd_parity(m);
        if bin_clear {
            seen_lo = true;
        } else {
            seen_hi = true;
        }
        assert_eq!(
            bin_clear,
            rng_jitter_gate(rand_u32, arg),
            "{ctx}: bin_clear={bin_clear} ah&5={m:#04x} r={r} arg={arg}"
        );
    };

    case(&mut e, 0, 1.0, "r=1.0 → (r−1)=0 < 0.07 → clear");
    case(&mut e, 0x7fffff, 1.0, "r≈2 → (r−1)≈1·1 > 0.07 → skip");
    // Deterministic `==threshold` boundary: the f64 product `(r−1)·arg` EXACTLY equals the f64
    // J5_THRESH (0.07_f32). r=1.5 (rand mantissa 0x400000 → r−1 = 0.5 exact); arg = 2·0.07_f32 =
    // 0.14000000059604645 (= bits 0x3e0f5c29, exact since ·2 is an exponent shift); 0.5·that = 0.07_f32
    // exactly. The binary's strict `<` does not clear at `==` (jp taken, even parity), so the gate
    // returns false here — a `<=` mistranscription would invert it.
    case(
        &mut e,
        0x0040_0000,
        f32::from_bits(0x3e0f_5c29),
        "product == 0.07 exactly → skip (==, not <)",
    );
    for i in 0..4000u32 {
        let rand_u32 = next(&mut s);
        let arg = match i % 3 {
            0 => frand(&mut s, 0.0, 1.0), // straddle the 0.07 crossing as (r−1)∈[0,1)·arg
            1 => frand(&mut s, -5.0, 5.0),
            _ => frand_bits(&mut s),
        };
        case(&mut e, rand_u32, arg, &format!("fuzz{i}"));
    }
    // `(r - 1) * arg == 0.07` exactly: r - 1 = 0.5 (0x400000) and arg = the f32 0.14, whose half
    // is the f32 0.07; and a step either side.
    let arg = f32::from_bits(0x3e0f_5c29);
    for a in [arg, next_up(arg), next_down(arg)] {
        case(&mut e, 0x40_0000, a, "threshold");
    }
    for r in [
        0u32,
        1,
        0x7f_ffff,
        0x80_0000,
        0xffff_ffff,
        0x40_0000,
        0x3f_ffff,
    ] {
        for &v in &specials(&[0.07, 0.14]) {
            case(&mut e, r, v, "special arg");
        }
    }
    assert!(seen_lo, "clear (product<0.07) arm never seen");
    assert!(seen_hi, "skip (product>=0.07) arm never seen");
}

#[test]
fn b2_heading_add_field_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let blk = e.alloc(0x40); // EAX; field at +0x20
    let mut s = 0x6106_b006u32;

    let case = |e: &mut Emu, field: f32, arg: f32, ctx: &str| {
        e.write_u32(blk + 0x20, field.to_bits());
        e.write_u32(ebp + 0xc, arg.to_bits());
        e.set_reg(RegisterX86::EAX, blk);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        // Halt at 0x6109b7 (`lea ecx,[ebp-0x3]`), AFTER the `fstp [ebp+8]` at 0x6109b4.
        e.run(0x6109ae, 0x6109b7);
        let bin = e.read_f32(ebp + 8);
        assert_f32(bin, heading_add_field(field, arg), ctx);
    };

    case(&mut e, 1.0, 2.0, "1+2");
    case(&mut e, -0.5, 3.0, "neg");
    case(&mut e, 0.0, 0.0, "zero");
    for i in 0..3000u32 {
        let (field, arg) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (frand(&mut s, -6.3, 6.3), frand(&mut s, -6.3, 6.3))
        };
        case(&mut e, field, arg, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, v, 1.5, "special first");
        case(&mut e, 1.5, v, "special second");
    }
}

#[test]
fn b2_turn_step_pi_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let slot = e.alloc(0x10);
    // Patch 0x6109c5 (`fld qword [0x803090]`, dd 05 90 30 80 00, 6 bytes) → `fstp dword [slot]`
    // (d9 1d <slot LE32>, 6 bytes). The slice then ends with dt·π stored at [slot].
    let mut patch = vec![0xd9u8, 0x1d];
    patch.extend_from_slice(&(slot as u32).to_le_bytes());
    e.write(0x6109c5, &patch);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x6106_b007u32;

    let case = |e: &mut Emu, dt: f32, ctx: &str| {
        e.write_u32(ebp - 0x10, dt.to_bits());
        e.set_reg(RegisterX86::EDI, 0); // the push edi @0x6109bd
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6109ba, 0x6109cb);
        let bin = e.read_f32(slot);
        assert_f32(bin, turn_step_pi(dt), ctx);
    };

    case(&mut e, 0.016, "0.016·π");
    case(&mut e, 0.0, "0");
    case(&mut e, -1.0, "−π");
    case(&mut e, 1.0, "π");
    for i in 0..3000u32 {
        let dt = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -2.0, 2.0)
        };
        case(&mut e, dt, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, v, "special");
    }
}

#[test]
fn b2_facing_error_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let (island, slot) = install_x87_getter(&mut e);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let this = e.alloc(0x40); // ESI; [esi] = vtbl
    let vtbl = e.alloc(0x40);
    e.write_u32(this, vtbl as u32);
    e.write_u32(vtbl + 0x18, island as u32);
    let mut s = 0x6106_b008u32;

    let case = |e: &mut Emu, orient: f32, facing: f32, ctx: &str| {
        e.write_u32(slot, orient.to_bits());
        e.write_u32(ebp - 0x8, facing.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x610a48, 0x610a56);
        // `push ecx` (0x610a52) moved esp to SLICE_ESP-4; `fstp [esp]` wrote there.
        let bin = e.read_f32(SLICE_ESP - 4);
        assert_f32(bin, facing_error(orient, facing), ctx);
    };

    case(&mut e, 1.0, 3.0, "3−1");
    case(&mut e, 3.0, 1.0, "1−3 (neg)");
    case(&mut e, 0.0, 0.0, "zero");
    for i in 0..3000u32 {
        let (orient, facing) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (frand(&mut s, -6.3, 6.3), frand(&mut s, -6.3, 6.3))
        };
        case(&mut e, orient, facing, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, v, 1.5, "special orient");
        case(&mut e, 1.5, v, "special facing");
    }
}

#[test]
fn b2_speed_from_timer_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let timer_slot = install_timer_stub(&mut e);
    let out = e.alloc(0x10);
    // Patch 0x610ae8 (`cmp eax,0xa; je 0x610af2`, 83 f8 0a 74 05 = 5 bytes) → `fstp dword [out]`
    // (d9 1d <out LE32> = 6 bytes; overwrites into 0x610aee `cmp eax,5`, harmless — we halt at 0x610aee).
    let mut patch = vec![0xd9u8, 0x1d];
    patch.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(0x610ae8, &patch);
    let mut s = 0x6106_b009u32;

    let case = |e: &mut Emu, timer: f32, radius: f32, ctx: &str| {
        e.write_u32(timer_slot, timer.to_bits());
        e.write_u32(0xc4d910, radius.to_bits());
        // ecx = [esi+0x118] for the timer call's thiscall; any mapped ptr works (stub ignores ecx).
        let mov = e.alloc(0x40);
        e.write_u32(mov, 0);
        let this = e.alloc(0x200);
        e.write_u32(this + 0x118, mov as u32);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x610aa2, 0x610aee);
        let bin = e.read_f32(out);
        assert_f32(bin, speed_from_timer(timer, radius), ctx);
    };

    case(&mut e, 14.0, 2.0, "14/7=2 → 2·2=4");
    case(&mut e, 3.5, 5.0, "3.5/7=0.5 <1 → 1·5=5");
    case(&mut e, 7.0, 3.0, "7/7=1 == 1 → 1·3=3");
    case(&mut e, 0.0, 4.0, "0/7=0 <1 → 1·4=4");
    for i in 0..3000u32 {
        let radius = frand(&mut s, 0.5, 50.0);
        let timer = match i % 3 {
            0 => frand(&mut s, 6.0, 8.0), // straddle 7.0 (the q==1 boundary)
            1 => frand(&mut s, 0.0, 20.0),
            _ => frand(&mut s, -5.0, 5.0),
        };
        case(&mut e, timer, radius, &format!("fuzz{i}"));
    }
    // `timer / 7 == 1` and a step either side, then the specials.
    for t in [7.0f32, next_up(7.0), next_down(7.0)] {
        case(&mut e, t, 3.0, "unity edge");
    }
    for &v in &specials(&[7.0]) {
        case(&mut e, v, 3.0, "special timer");
        case(&mut e, 14.0, v, "special radius");
    }
}

#[test]
fn b2_range_gate_square_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let scaled_slot = e.alloc(0x10);
    // the real squaring/compare at 0x610b07.
    let island = e.alloc(0x20);
    {
        let mut b = vec![0xd9u8, 0x05]; // fld dword [scaled_slot]
        b.extend_from_slice(&(scaled_slot as u32).to_le_bytes());
        b.push(0xe9); // jmp rel32
        let from = island + 6 + 5; // EIP after the jmp
        let rel = (0x610b07i64 - from as i64) as i32;
        b.extend_from_slice(&rel.to_le_bytes());
        e.write(island, &b);
    }
    let mut s = 0x6106_b010u32;
    let mut seen_commit = false;
    let mut seen_no = false;

    let mut case = |e: &mut Emu, scaled: f32, goal_r2: f32, ctx: &str| {
        e.write_u32(scaled_slot, scaled.to_bits());
        e.write_u32(ebp - 0x14, goal_r2.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        // Run the island (loads scaled, jmps to 0x610b07), halting at the `test ah,0x41` (0x610b14).
        e.run(island, 0x610b14);
        let m = ah(e) & 0x41;
        // jp 0x610b29 (not-arrived): jp taken (EVEN parity) → no commit; not taken (ODD) → commit.
        let bin_commit = odd_parity(m);
        if bin_commit {
            seen_commit = true;
        } else {
            seen_no = true;
        }
        let (_sq, our_commit) = range_gate_square(scaled, goal_r2);
        assert_eq!(
            bin_commit, our_commit,
            "{ctx}: bin_commit={bin_commit} ah&0x41={m:#04x} scaled={scaled} g={goal_r2}"
        );
    };

    case(&mut e, 5.0, 100.0, "25 < 100 → no commit");
    case(&mut e, 11.0, 100.0, "121 > 100 → commit");
    case(&mut e, 10.0, 100.0, "100 == 100 → commit (<=)");
    for i in 0..4000u32 {
        let scaled = frand(&mut s, 0.0, 20.0);
        let goal_r2 = match i % 3 {
            0 => f64::from(scaled * scaled) as f32 + frand(&mut s, -5.0, 5.0), // straddle scaled²
            1 => frand(&mut s, 0.0, 400.0),
            _ => frand_bits(&mut s).abs(),
        };
        case(&mut e, scaled, goal_r2, &format!("fuzz{i}"));
    }
    // `goal_r2 == scaled^2` (9 against 3) and a step either side, then the specials.
    for g in [9.0f32, next_up(9.0), next_down(9.0)] {
        case(&mut e, 3.0, g, "equal");
    }
    for &v in &specials(&[3.0, 9.0]) {
        case(&mut e, v, 9.0, "special scaled");
        case(&mut e, 3.0, v, "special goal");
    }
    assert!(seen_commit, "commit (scaled²>=goal_r²) arm never seen");
    assert!(seen_no, "no-commit arm never seen");
}

#[test]
fn b2_anim_time_speed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let this = e.alloc(0x1000); // ESI; anim_time at +0xa34
    let scaled_out = e.alloc(0x10);
    let mut s = 0x6106_b011u32;
    let mut seen_a = false;
    let mut seen_b = false;

    // Value check: patch the squaring branch entry to ALSO store `scaled` before squaring. The op at
    // 0x610c12 (`fld st(0); fmul st,st(1)`) squares the live `scaled` (st0). We verify `scaled` (the
    // f32 product X·4.5) by a SEPARATE value-slice halting at 0x610c10 with `scaled` on st0 — read it
    // via a patched store at 0x610c10 (`cmp eax,3; jne`, 83 f8 03 75 14 = 5 bytes) → `fstp [out]`
    // (6 bytes; overwrites into the je, harmless — we halt at 0x610c16).
    let mut value_emu = Emu::new(&wow);
    {
        let mut patch = vec![0xd9u8, 0x1d];
        patch.extend_from_slice(&(scaled_out as u32).to_le_bytes());
        value_emu.write(0x610c10, &patch);
    }

    let mut case = |e: &mut Emu, ve: &mut Emu, anim_time: f32, goal_r2: f32, ctx: &str| {
        let (our_scaled, _our_sq) = anim_time_speed(anim_time);

        // --- value slice: read the f32 product `scaled` = X·4.5 ---
        ve.write_u32(this + 0xa34, anim_time.to_bits());
        ve.write_u32(0xc4d888, 3); // move-type 3 (so the `cmp eax,3` path is taken; we store before it)
        ve.set_reg(RegisterX86::ESI, this);
        ve.set_reg(RegisterX86::EBP, ebp);
        ve.set_reg(RegisterX86::ESP, SLICE_ESP);
        ve.run(0x610bd7, 0x610c16);
        let bin_scaled = ve.read_f32(scaled_out);
        assert_f32(bin_scaled, our_scaled, &format!("{ctx} scaled"));

        // --- AH slice: the squared in-range gate ---
        e.write_u32(this + 0xa34, anim_time.to_bits());
        e.write_u32(ebp - 0x14, goal_r2.to_bits());
        e.write_u32(0xc4d888, 3);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x610bd7, 0x610c1f);
        let m = ah(e) & 0x1;
        // fcompp (st0=goal_r², st1=scaled²) ; test ah,1 ; jne 0x610c30. C0 = goal_r²<scaled². jne
        // taken ⟺ C0 set ⟺ goal_r²<scaled² ordered → 0x610c30 (out of range). not taken ⟺ C0 clear
        // ⟺ goal_r²>=scaled² → fall through to 0x60e790 (in range). So in-range ⟺ goal_r² >= scaled²
        // ordered (i.e. scaled² <= goal_r²). NaN → C0 set → jne → out.
        let bin_in = (m & 0x1) == 0;
        if bin_in {
            seen_a = true;
        } else {
            seen_b = true;
        }
        // Our scaled² ; in-range ⟺ !(goal_r² < scaled²) [ordered; NaN → C0 set → out].
        let (_sf, sq) = anim_time_speed(anim_time);
        let g = f64::from(goal_r2);
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        let our_in = !sq.is_nan() && !g.is_nan() && !(g < sq);
        assert_eq!(
            bin_in, our_in,
            "{ctx}: bin_in={bin_in} ah&1={m:#04x} t={anim_time} g={goal_r2} sq={sq}"
        );
    };

    case(
        &mut e,
        &mut value_emu,
        14.0,
        100.0,
        "t14/7=2 → X2·4.5=9, 81<100 in",
    );
    case(
        &mut e,
        &mut value_emu,
        3.5,
        1.0,
        "t<7 → X1·4.5=4.5, 20.25>1 out",
    );
    case(
        &mut e,
        &mut value_emu,
        7.0,
        20.25,
        "t==7 → X1·4.5=4.5, 20.25==20.25 in",
    );
    case(
        &mut e,
        &mut value_emu,
        0.0,
        30.0,
        "t0 → X1·4.5=4.5, 20.25<30 in",
    );
    for i in 0..3000u32 {
        let anim_time = match i % 3 {
            0 => frand(&mut s, 6.0, 8.0), // straddle 7.0
            1 => frand(&mut s, 0.0, 40.0),
            _ => frand(&mut s, -3.0, 3.0),
        };
        let (_sf, sq) = anim_time_speed(anim_time);
        let goal_r2 = match i % 2 {
            0 => sq as f32 + frand(&mut s, -10.0, 10.0), // straddle scaled²
            _ => frand(&mut s, 0.0, 200.0),
        };
        case(
            &mut e,
            &mut value_emu,
            anim_time,
            goal_r2,
            &format!("fuzz{i}"),
        );
    }
    // `anim_time / 7 == 1` and a step either side, `scaled^2 == goal_r2` (4.5^2 against 20.25),
    // and the specials.
    for t in [7.0f32, next_up(7.0), next_down(7.0)] {
        case(&mut e, &mut value_emu, t, 20.25, "unity edge");
    }
    for g in [20.25f32, next_up(20.25), next_down(20.25)] {
        case(&mut e, &mut value_emu, 7.0, g, "gate equal");
    }
    for &v in &specials(&[7.0, 20.25]) {
        case(&mut e, &mut value_emu, v, 20.25, "special time");
        case(&mut e, &mut value_emu, 14.0, v, "special goal");
    }
    assert!(seen_a, "in-range arm never seen");
    assert!(seen_b, "out-of-range arm never seen");
}

#[test]
fn b2_arrival_cos_threshold_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::EAX, 0); // al (the lazy-init flag byte being OR'd to 1)
    e.set_reg(RegisterX86::ECX, 1);
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.run(0x610c85, 0x610c9a);
    let bin = e.read_f32(0xc4d9d0);
    assert_f32(bin, arrival_cos_threshold(), "cos(π/180)");
    // Sanity: it is the cosine of 1°, very close to 1.0.
    assert!(
        (bin - 0.9998477).abs() < 1e-5,
        "cos(1°) ≈ 0.9998477, got {bin}"
    );
}

#[test]
fn b2_normalize_zdot_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let cos1 = arrival_cos_threshold();
    let mut s = 0x6106_b013u32;
    let mut seen_stop = false;
    let mut seen_go = false;

    let mut case = |e: &mut Emu, sp: [f32; 3], tg: [f32; 3], ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(ebp - 0x38 + 4 * k, sp[k as usize].to_bits()); // self.x/y/z
            e.write_u32(ebp - 0x2c + 4 * k, tg[k as usize].to_bits()); // tgt.x/y/z
        }
        e.write_u32(0xc4d9d0, cos1.to_bits()); // the cached cos(1°) threshold
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);

        let (_zdot, mag2, proceed, our_stop) = normalize_zdot(sp, tg, cos1);
        if !proceed {
            // Degenerate (mag²<=0 / NaN): the binary bails at 0x610ce0 (jne 0x610d40) BEFORE the slice
            // end at 0x610d28 — running it would halt early (EIP never reaches 0x610d28). We assert our
            // transcription agrees the gate bails and skip the (unreachable) AH read.
            assert!(!our_stop, "{ctx}: degenerate mag²={mag2} must not stop");
            return;
        }
        e.run(0x610cb0, 0x610d28);
        let m = ah(e) & 0x41;
        // fcomp cos1 (|dz·inv|) ; test ah,0x41 ; jne 0x610d46 (return, no stop). C0=|zdot|<cos1,
        // C3=eq. jne taken ⟺ (C0|C3)!=0 ⟺ |zdot|<cos1 OR == OR NaN → no stop. not taken ⟺ |zdot|>cos1
        // ordered → STOP. So stop ⟺ (ah&0x41)==0.
        let bin_stop = m == 0;
        if bin_stop {
            seen_stop = true;
        } else {
            seen_go = true;
        }
        assert_eq!(
            bin_stop, our_stop,
            "{ctx}: bin_stop={bin_stop} ah&0x41={m:#04x} mag²={mag2}"
        );
    };

    // Nearly vertical (|dz| dominates) → |dz/|d|| > cos(1°) → STOP.
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.001, 0.0, 100.0],
        "near-vertical up → stop",
    );
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.0, 0.001, -100.0],
        "near-vertical down → stop",
    );
    // Mostly horizontal → small |dz/|d|| → no stop.
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [100.0, 0.0, 1.0],
        "horizontal → continue",
    );
    // Exactly along z (dx=dy=0) → |dz/|d||=1 > cos(1°) → stop.
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 50.0],
        "exact +z → stop (1>cos1)",
    );
    // (not f32×f32). A large NON-f32-representable dz with the horizontal swept across the cos(1°)
    // crossing flips the stop boolean between the live and f32 forms — the grader's proven witness +
    // a dense sweep (the off-boundary fuzz lands 0 flip cases).
    case(
        &mut e,
        [-45.14, 23.10, -15.10],
        [-23.61, 23.10, 1218.62],
        "live-dz flip witness",
    );
    {
        let sp = [0.0f32, 0.0, -15.10];
        let dz = 1233.72f32; // a large, non-f32-representable tgt.z−self.z difference
        let crossing = dz * (std::f32::consts::PI / 180.0).tan(); // horizontal at exactly 1° off +z
        for k in -200i32..=200 {
            let h = f32::from_bits((crossing.to_bits() as i32 + k) as u32);
            case(
                &mut e,
                sp,
                [sp[0] + h, sp[1], sp[2] + dz],
                &format!("live-dz cos1 sweep k={k}"),
            );
        }
    }
    for i in 0..5000u32 {
        let sp = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        let tg = match i % 3 {
            0 => {
                // tune dz vs horizontal to straddle the cos(1°) crossing.
                let h = frand(&mut s, 0.1, 30.0);
                let dz = h * frand(&mut s, 5.0, 200.0); // mostly-vertical band
                [sp[0] + h, sp[1], sp[2] + dz]
            }
            1 => [
                sp[0] + frand(&mut s, -30.0, 30.0),
                sp[1] + frand(&mut s, -30.0, 30.0),
                sp[2] + frand(&mut s, -30.0, 30.0),
            ],
            _ => [
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
            ],
        };
        // Skip self==tgt (mag²==0 → degenerate; handled by the proceed check, but avoid noise).
        if sp == tg {
            continue;
        }
        case(&mut e, sp, tg, &format!("fuzz{i}"));
    }
    // Zero and axis vectors, and the specials in each coordinate.
    case(&mut e, [1.0, 2.0, 3.0], [1.0, 2.0, 3.0], "coincident");
    case(&mut e, [0.0; 3], [0.0, 0.0, 5.0], "straight up");
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut a = [1.0f32, 2.0, 3.0];
            a[k] = v;
            case(&mut e, a, [4.0, 6.0, 8.0], "special self");
            case(&mut e, [1.0, 2.0, 3.0], a, "special target");
        }
    }
    assert!(seen_stop, "near-vertical STOP arm never seen");
    assert!(seen_go, "continue arm never seen");
}
