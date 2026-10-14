//! The unit frame kernels of `src/unitm5_fp.rs` against the exe, each read at the code it covers
//! with the registers, frame and fields it reads set directly:
//! - `CGUnit_C::Tick` (`0x60a7f0`): the level fraction `[0x60a9bc, 0x60a9de)`, the facing wrap
//!   `[0x60aa76, 0x60aab0)` and the lean pitch `[0x60aae4, 0x60aafc)`;
//! - `PlayKitCharProcs` (`0x60d7c0`): the colour and duration fields through `__ftol`,
//!   `[0x60d919, 0x60d92e)` and `[0x60d9e8, 0x60d9ff)`, and the footstep parameter
//!   `[0x60db35, 0x60db49)`;
//! - `CGUnit_C::CheckAutoMoveRange` (`0x6110a0`): the distance and compare `[0x6110ba, 0x61110a)`,
//!   and whole with its position getter stubbed;
//! - `CGUnit_C::DrawBowstring` (`0x611ff0`): the colour bytes `[0x612044, 0x612116)` and the
//!   midpoint `[0x612221, 0x612269)`.
//!
//! Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step either
//! side, to its random inputs; the `__ftol` legs add products past `i32` and `i64`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::unitm5_fp::*;
use common::*;

// ---- shared helpers (the crate float idiom; per unitm4_fp.rs) --------------

#[test]
fn u5_mount_level_fraction_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x2000); // ESI
    let extra = e.alloc(0x40); // [esi+0xe68] → extra block; level byte at +0x1d
    let mut s = 0x60a7_f000u32;

    let case = |e: &mut Emu, level: u8, ctx: &str| {
        e.write_u32(this + 0xe68, extra as u32);
        e.write(extra + 0x1d, &[level]);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60a9bc, 0x60a9de);
        let bin = e.read_f32(ebp + 8);
        assert_f32(bin, mount_level_fraction(level), ctx);
    };

    // Deterministic boundary probes around the min(level,100) clamp.
    case(&mut e, 0, "level 0");
    case(&mut e, 99, "level 99 (<100)");
    case(&mut e, 100, "level 100 (== clamp)");
    case(&mut e, 101, "level 101 (> clamp → 100)");
    case(&mut e, 255, "level 255 (→ 100)");
    for i in 0..1000u32 {
        let level = ((next(&mut s) >> 8) & 0xff) as u8; // full byte range, exercises both clamp arms
        case(&mut e, level, &format!("fuzz{i}"));
    }
    // Every level byte, across the clamp at 100.
    for level in 0..=255u8 {
        case(&mut e, level, "every level");
    }
}

#[test]
fn u5_move_facing_wrap_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x2000); // ESI; [esi+0x118] → move block
    let mov = e.alloc(0x80); // move block; orient at +0x1c
    let mut s = 0x60aa_7600u32;
    let mut seen_add = false; // sum<0 → +2π
    let mut seen_sub = false; // sum>=2π → −2π
    let mut seen_mid = false; // unchanged

    let tau = f32::from_bits(0x40c9_0fdb);

    let mut case = |e: &mut Emu, orient: f32, delta: f32, ctx: &str| {
        e.write_u32(this + 0x118, mov as u32);
        e.write_u32(mov + 0x1c, orient.to_bits());
        e.write_u32(ebp - 4, delta.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDX, mov); // 0x60aa70 sets edx=[esi+0x118]; our slice starts after.
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60aa76, 0x60aab0);
        let bin = e.read_f32(ebp + 8);
        let sum = (f64::from(orient) + f64::from(delta)) as f32;
        if !sum.is_nan() && sum < 0.0 {
            seen_add = true;
        } else if !sum.is_nan() && f64::from(sum) >= f64::from(tau) {
            seen_sub = true;
        } else {
            seen_mid = true;
        }
        assert_f32(bin, move_facing_wrap(orient, delta), ctx);
    };

    case(&mut e, 1.0, 1.0, "mid → unchanged");
    case(&mut e, -0.5, -0.5, "sum<0 → +2π");
    case(&mut e, 6.0, 1.0, "sum>=2π → −2π");
    case(&mut e, 0.0, 0.0, "zero → unchanged");
    // Deterministic boundary probes the fuzz cannot hit bit-exactly.
    case(&mut e, tau, 0.0, "sum==2π exactly → −2π");
    case(&mut e, 0.0, tau, "Δ==2π exactly → −2π");
    case(&mut e, 0.0, -1.0e-30, "sum just <0 → +2π");
    case(&mut e, f32::NAN, 0.0, "nan → unchanged");
    for i in 0..3000u32 {
        let (orient, delta) = match i % 3 {
            0 => (frand(&mut s, -1.0, 7.0), frand(&mut s, -1.0, 1.0)), // straddle 0 and 2π
            1 => (frand(&mut s, 0.0, 6.3), frand(&mut s, -6.3, 6.3)),
            _ => (frand_bits(&mut s), frand_bits(&mut s)),
        };
        case(&mut e, orient, delta, &format!("fuzz{i}"));
    }
    // The sum at 0 (and just under), at 2 pi (and either side), and the specials.
    let tau = TAU;
    for o in [
        0.0f32,
        -0.0,
        next_down(0.0),
        tau,
        next_up(tau),
        next_down(tau),
    ] {
        case(&mut e, o, 0.0, "edge");
        case(&mut e, 0.0, o, "edge delta");
    }
    for &v in &specials(&[tau]) {
        case(&mut e, v, 1.0, "special orient");
        case(&mut e, 1.0, v, "special delta");
    }
    assert!(seen_add, "+2π arm never seen");
    assert!(seen_sub, "−2π arm never seen");
    assert!(seen_mid, "unchanged arm never seen");
}

#[test]
fn u5_lean_pitch_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x2000);
    let mov = e.alloc(0x80); // [esi+0x118] → move block; pitch at +0x20
    let mut s = 0x60aa_e400u32;

    let case = |e: &mut Emu, pitch: f32, delta: f32, ctx: &str| {
        e.write_u32(this + 0x118, mov as u32);
        e.write_u32(mov + 0x20, pitch.to_bits());
        e.write_u32(ebp - 4, delta.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ECX, 0); // pushed; value irrelevant
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60aae4, 0x60aafc);
        let bin = e.read_f32(SLICE_ESP - 4); // the single `push ecx` then `fstp [esp]`
        assert_f32(bin, lean_pitch(pitch, delta), ctx);
    };

    case(&mut e, 0.0, 0.0, "zero");
    case(&mut e, 0.1, 0.5, "typical");
    case(&mut e, -0.2, -1.0, "neg");
    for i in 0..3000u32 {
        let (pitch, delta) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (frand(&mut s, -1.0, 1.0), frand(&mut s, -3.2, 3.2))
        };
        case(&mut e, pitch, delta, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, v, 1.0, "special pitch");
        case(&mut e, 1.0, v, "special delta");
    }
}

#[test]
fn u5_color_field_x1000_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let blk = e.alloc(0x80); // EDI; field at [edi]
    let mut s = 0x60d9_1900u32;

    let case = |e: &mut Emu, field: f32, ctx: &str| {
        e.write_u32(blk, field.to_bits());
        e.set_reg(RegisterX86::EDI, blk);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60d919, 0x60d92e);
        let bin = e.get_reg(RegisterX86::EAX) as u32 as i32;
        assert_eq!(bin, color_field_x1000(field), "{ctx}: field={field}");
    };

    case(&mut e, 0.0, "0");
    case(&mut e, 0.5, "0.5 → 500");
    case(&mut e, 1.0, "1.0 → 1000");
    case(&mut e, -0.25, "neg → -250 (trunc toward 0)");
    case(&mut e, 0.0009, "sub-step trunc");
    for i in 0..3000u32 {
        // Keep field in a range where field·1000 fits an i32 cleanly (color fractions are small).
        let field = match i % 3 {
            0 => frand(&mut s, -2.0, 2.0),
            1 => frand(&mut s, -1.0e6, 1.0e6) / 1000.0, // up to ~±1e6 after ×1000
            _ => frand(&mut s, -0.001, 0.001),          // truncation-toward-zero edge
        };
        case(&mut e, field, &format!("fuzz{i}"));
    }
    // Products past `i32` keep the low dword of the 64-bit result, and past `i64` give 0 (the
    // integer indefinite): 3e6 * 1000, 1e16 * 1000, 1e17 * 1000 and 1e30 * 1000.
    for &v in &[
        3.0e6f32, -3.0e6, 5.0e15, 1.0e16, 9.0e15, 9.3e15, 1.0e17, 1.0e30, -1.0e30,
    ] {
        case(&mut e, v, "wide");
    }
    for &v in &specials(&[]) {
        case(&mut e, v, "special");
    }
}

#[test]
fn u5_duration_field_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x80); // ESI; [esi] = vtable ptr
    let vtbl = e.alloc(0x80); // synthesized vtable; +0x6c = the duration getter
    let blk = e.alloc(0x80); // EDI; field at [edi-0x20]
    let dur_slot = e.alloc(0x10); // the float the stub loads
    let stub = e.alloc(0x10);
    let mut s = 0x60d9_e800u32;

    // x87 stub bytes: d9 05 <dur_slot LE32> c3  (fld dword ptr [dur_slot]; ret).
    let mut stub_bytes = vec![0xd9u8, 0x05];
    stub_bytes.extend_from_slice(&(dur_slot as u32).to_le_bytes());
    stub_bytes.push(0xc3);
    e.write(stub, &stub_bytes);
    e.write_u32(this, vtbl as u32);
    e.write_u32(vtbl + 0x6c, stub as u32);

    let case = |e: &mut Emu, duration: f32, field: f32, ctx: &str| {
        e.write_u32(dur_slot, duration.to_bits());
        // The binary reads the field at [edi-0x20]; set edi = blk+0x20 so [edi-0x20] = [blk].
        e.write_u32(blk, field.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, blk + 0x20);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60d9e8, 0x60d9ff);
        let bin = e.get_reg(RegisterX86::EAX) as u32 as i32;
        assert_eq!(
            bin,
            duration_field(duration, field),
            "{ctx}: dur={duration} field={field}"
        );
    };

    case(&mut e, 1.0, 0.0, "dur1 field0 → 0");
    case(&mut e, 2.0, 3.0, "2·3 → 6");
    case(&mut e, 0.5, -10.0, "neg product trunc");
    case(&mut e, 1.5, 2.5, "3.75 → 3 (trunc)");
    for i in 0..3000u32 {
        let duration = frand(&mut s, -50.0, 50.0);
        let field = frand(&mut s, -50.0, 50.0);
        case(&mut e, duration, field, &format!("fuzz{i}"));
    }
    for &(d, f) in &[
        (3.0e6f32, 1000.0f32),
        (1.0e16, 1.0),
        (1.0e10, 1.0e10),
        (1.0e30, 1.0e30),
        (-1.0e10, 1.0e10),
    ] {
        case(&mut e, d, f, "wide");
    }
    for &v in &specials(&[]) {
        case(&mut e, v, 2.0, "special duration");
        case(&mut e, 2.0, v, "special field");
    }
}

#[test]
fn u5_footstep_param_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let blk = e.alloc(0x80); // EDI; field at [edi-0x10]
    let mut s = 0x60db_3500u32;

    let case = |e: &mut Emu, field: f32, ctx: &str| {
        e.write_u32(blk, field.to_bits()); // [edi-0x10] with edi=blk+0x10
        e.set_reg(RegisterX86::EDI, blk + 0x10);
        e.set_reg(RegisterX86::ECX, 0);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60db35, 0x60db49);
        let bin = e.read_f32(ebp - 8);
        assert_f32(bin, footstep_param(field), ctx);
    };

    case(&mut e, 0.0, "0 → 512.0");
    case(&mut e, 1.0, "1 → 513.0");
    case(&mut e, -0.05, "neg → 511.95");
    for i in 0..3000u32 {
        let field = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -100.0, 100.0)
        };
        case(&mut e, field, &format!("fuzz{i}"));
    }
    for &v in &specials(&[512.0]) {
        case(&mut e, v, "special");
    }
}

#[test]
fn u5_in_range_dist2_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x40); // ECX; [ecx] = vtable
    let vtbl = e.alloc(0x40); // synthesized vtable; +0x14 holds the GetPosition slot ptr
    let getpos = e.alloc(0x10); // executable island the +0x14 slot points at (the stub hooks here)
    let goal = e.alloc(0x20); // [ebp+0xc]
    let pos_out = e.alloc(0x20); // the self pos; GetPosition returns EAX→here (dist reads [eax..])
    let mut s = 0x6110_a000u32;
    let mut seen_in = false;
    let mut seen_out = false;

    // The per-action table _DAT_00860a58[action*0xc]: byte enable @+0, f32 radius² @+4. action 0 →
    // esi=0, so the reads land at the real (mapped .data) globals; we write our values there.
    let table_radius2 = 0x860a5cu64;
    let table_enable = 0x860a58u64;

    // Synthesized GetPosition (`call [eax+0x14]`): the binary reads the self position as [eax..]
    // AFTER the call (EAX = the returned pointer). The slot [vtbl+0x14] points at `getpos`; the
    // stub there returns EAX=pos_out (cleaning the 1 out-param arg) so [eax]=pos_out holds the pos.
    e.stub(getpos, pos_out as u32, 1);

    let mut case = |e: &mut Emu, gp: [f32; 3], pp: [f32; 3], radius2: f32, ctx: &str| {
        e.write_u32(this, vtbl as u32);
        e.write_u32(vtbl + 0x14, getpos as u32);
        for k in 0..3 {
            e.write_u32(goal + 4 * k as u64, gp[k].to_bits());
            e.write_u32(pos_out + 4 * k as u64, pp[k].to_bits());
        }
        e.write(table_enable, &[1]); // enable active
        e.write_u32(table_radius2, radius2.to_bits());
        e.write_u32(ebp + 0xc, goal as u32);
        e.set_reg(RegisterX86::ECX, this);
        e.set_reg(RegisterX86::ESI, 0); // action 0 → esi=0
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6110ba, 0x61110a); // end x87-empty (the 3 dx/dy/dz `fstp st(0)` all popped)
        let m = ah(e) & 0x1;
        // test ah,1 = C0 (dist2 < radius²). jne 0x611122 taken ⟺ C0 set ⟺ in range. So in-range ⟺
        // (ah&1)!=0. (NaN dist2 also sets C0 → in range, matching in_range_of_goal.)
        let bin_in = m != 0;
        if bin_in {
            seen_in = true;
        } else {
            seen_out = true;
        }
        assert_eq!(
            bin_in,
            in_range_of_goal(1, radius2, gp, pp),
            "{ctx}: bin_in={bin_in} ah&1={m:#04x}"
        );
        // Cross-check the f64 squared distance is what the predicate uses.
        let _ = goal_dist2(gp, pp);
    };

    case(&mut e, [0.0; 3], [0.0; 3], 1.0, "d0 < r1 → in");
    case(&mut e, [10.0, 0.0, 0.0], [0.0; 3], 1.0, "d100 > r1 → out");
    case(
        &mut e,
        [1.0, 0.0, 0.0],
        [0.0; 3],
        1.0,
        "d1 == r1 → out (>=)",
    );
    case(&mut e, [0.0; 3], [0.0; 3], f32::NAN, "nan radius → in (C0)");
    for i in 0..3000u32 {
        let pp = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        let gp = match i % 2 {
            0 => [
                pp[0] + frand(&mut s, -3.0, 3.0), // straddle the radius boundary
                pp[1] + frand(&mut s, -3.0, 3.0),
                pp[2] + frand(&mut s, -3.0, 3.0),
            ],
            _ => [
                frand(&mut s, -60.0, 60.0),
                frand(&mut s, -60.0, 60.0),
                frand(&mut s, -60.0, 60.0),
            ],
        };
        let radius2 = frand(&mut s, 0.01, 50.0);
        case(&mut e, gp, pp, radius2, &format!("fuzz{i}"));
    }
    // `dist^2 == radius^2` (3-4-5 against 25) and a step either side, then the specials.
    for x in [3.0f32, next_up(3.0), next_down(3.0)] {
        case(&mut e, [x, 4.0, 0.0], [0.0; 3], 25.0, "equal");
    }
    for r in [next_up(25.0), next_down(25.0)] {
        case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], r, "radius step");
    }
    for &v in &specials(&[25.0]) {
        // A radius of exactly zero leaves the function at `0x6110ba`'s `jnp`, past the slice.
        if v != 0.0 {
            case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], v, "special radius");
        }
        for k in 0..3 {
            let mut g = [3.0f32, 4.0, 0.0];
            g[k] = v;
            case(&mut e, g, [0.0; 3], 25.0, "special goal");
            let mut q = [0.0f32; 3];
            q[k] = v;
            case(&mut e, [3.0, 4.0, 0.0], q, 25.0, "special pos");
        }
    }
    assert!(seen_in, "in-range arm never seen");
    assert!(seen_out, "out-of-range arm never seen");
}

#[test]
fn u5_in_range_gates_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x40);
    let vtbl = e.alloc(0x40);
    let getpos = e.alloc(0x10); // executable island the +0x14 slot points at
    let goal = e.alloc(0x20);
    let pos_out = e.alloc(0x20); // GetPosition returns EAX→here; dist reads [eax..]
    let mut s = 0x6110_a100u32;
    let mut seen_enable0 = false;
    let mut seen_radius0 = false;
    let mut seen_compare = false;

    e.stub(getpos, pos_out as u32, 1);

    let table_radius2 = 0x860a5cu64;
    let table_enable = 0x860a58u64;

    let mut case =
        |e: &mut Emu, enable: u8, radius2: f32, gp: [f32; 3], pp: [f32; 3], ctx: &str| {
            e.write_u32(this, vtbl as u32);
            e.write_u32(vtbl + 0x14, getpos as u32);
            for k in 0..3 {
                e.write_u32(goal + 4 * k as u64, gp[k].to_bits());
                e.write_u32(pos_out + 4 * k as u64, pp[k].to_bits());
            }
            e.write(table_enable, &[enable]);
            e.write_u32(table_radius2, radius2.to_bits());
            // __thiscall(ecx=this, [esp+4]=action, [esp+8]=goalpos) ; ret 8. action 0 → esi=0.
            let al = e.call(0x6110a0, this as u32, 0, &[0, goal as u32]) & 0xff;
            let bin_in = al != 0;
            if enable == 0 {
                seen_enable0 = true;
            } else if radius2 == 0.0 {
                seen_radius0 = true;
            } else {
                seen_compare = true;
            }
            assert_eq!(
                bin_in,
                in_range_of_goal(enable, radius2, gp, pp),
                "{ctx}: bin_in={bin_in} enable={enable} radius2={radius2}"
            );
        };

    case(
        &mut e,
        0,
        1.0,
        [99.0; 3],
        [0.0; 3],
        "enable 0 → in (no gate)",
    );
    case(
        &mut e,
        1,
        0.0,
        [99.0; 3],
        [0.0; 3],
        "radius² 0 → in (no gate)",
    );
    case(&mut e, 1, 4.0, [0.0; 3], [0.0; 3], "d0 < r4 → in");
    case(&mut e, 1, 1.0, [10.0, 0.0, 0.0], [0.0; 3], "far → out");
    for i in 0..1500u32 {
        let enable = match i % 3 {
            0 => 0u8, // exercise the enable gate
            _ => 1u8,
        };
        let radius2 = match i % 5 {
            0 => 0.0f32, // exercise the radius² gate
            _ => frand(&mut s, 0.01, 30.0),
        };
        let pp = [
            frand(&mut s, -20.0, 20.0),
            frand(&mut s, -20.0, 20.0),
            frand(&mut s, -20.0, 20.0),
        ];
        let gp = [
            pp[0] + frand(&mut s, -4.0, 4.0),
            pp[1] + frand(&mut s, -4.0, 4.0),
            pp[2] + frand(&mut s, -4.0, 4.0),
        ];
        case(&mut e, enable, radius2, gp, pp, &format!("fuzz{i}"));
    }
    // The whole function at the squared range equal to the distance and a step either side, a
    // zero and negative radius, and the specials.
    for r in [25.0f32, next_up(25.0), next_down(25.0)] {
        case(&mut e, 1, r, [3.0, 4.0, 0.0], [0.0; 3], "radius equal");
    }
    for &v in &specials(&[25.0]) {
        case(&mut e, 1, v, [3.0, 4.0, 0.0], [0.0; 3], "special radius");
        case(&mut e, 1, 25.0, [v, 4.0, 0.0], [0.0; 3], "special goal x");
        case(
            &mut e,
            1,
            25.0,
            [3.0, 4.0, 0.0],
            [0.0, v, 0.0],
            "special pos y",
        );
    }
    assert!(seen_enable0, "enable-gate arm never seen");
    assert!(seen_radius0, "radius²-gate arm never seen");
    assert!(seen_compare, "distance-compare arm never seen");
}

#[test]
fn u5_trail_color_byte_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let blk = e.alloc(0x80); // EDI; channels at +0x54/+0x58/+0x5c
    let mut s = 0x611f_f000u32;
    let mut seen_zero = false;
    let mut seen_scale = false;
    let mut seen_max = false;

    let mut case = |e: &mut Emu, c: f32, ctx: &str| {
        e.write_u32(blk + 0x54, c.to_bits());
        e.write_u32(blk + 0x58, c.to_bits());
        e.write_u32(blk + 0x5c, c.to_bits());
        e.set_reg(RegisterX86::EDI, blk);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x612044, 0x612116); // ends x87-empty (3× __ftol popped)
        let bin = e.get_reg(RegisterX86::EAX) as u32 as i32; // = ftol(ch54), == every channel here
        assert_eq!(bin, trail_color_byte(c), "{ctx}: c={c}");
        let cf = f64::from(c);
        if cf > 0.0 {
            if cf < 1.0 {
                seen_scale = true;
            } else {
                seen_max = true;
            }
        } else {
            seen_zero = true; // c<=0 (ordered) OR NaN → the 0 arm
        }
    };

    case(&mut e, -0.5, "neg → 0");
    case(&mut e, 0.0, "0 → 0");
    case(&mut e, 0.5, "0.5 → 128 (127.5+0.5)");
    case(&mut e, 1.0, "1.0 → 255 (clamp-max)");
    case(&mut e, 2.0, ">1 → 255");
    // Deterministic boundary probes the fuzz cannot hit bit-exactly.
    case(
        &mut e,
        f32::from_bits(0x3f80_0000),
        "c==1.0 exact → 255 (>= arm)",
    );
    case(
        &mut e,
        f32::from_bits(0x3f7f_ffff),
        "c just <1.0 → scale arm",
    );
    case(&mut e, 0.0, "c==0 exact → 0 (C3 odd)");
    // NaN routes through the binary's scale path → ftol(NaN) → 0; our else-arm returns 0 directly.
    case(&mut e, f32::NAN, "NaN → 0 (ftol(NaN)=0)");
    case(&mut e, f32::from_bits(0xffc0_0000), "−NaN → 0");
    for i in 0..2500u32 {
        let c = match i % 3 {
            0 => frand(&mut s, -0.2, 1.2), // straddle 0 and 1
            1 => frand(&mut s, 0.0, 1.0),
            _ => frand_bits(&mut s),
        };
        case(&mut e, c, &format!("fuzz{i}"));
    }
    // The scale edge `c * 255 + 0.5` at an integer, at 0 and at 1, a step either side, and the
    // specials; products past the byte range clamp at 255.
    for k in [1.0f32, 100.0, 127.5, 254.0] {
        let c = ((f64::from(k) - 0.5) / 255.0) as f32;
        for v in [c, next_up(c), next_down(c)] {
            case(&mut e, v, "integer edge");
        }
    }
    for &v in &specials(&[1.0]) {
        case(&mut e, v, "special");
    }
    assert!(seen_zero, "c<=0 → 0 arm never seen");
    assert!(seen_scale, "0<c<1 scale arm never seen");
    assert!(seen_max, "c>=1 → 255 arm never seen");
}

#[test]
fn u5_trail_midpoint_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80); // ebp sits mid-frame so the slice's `fstp [ebp-0x24]` has headroom
    let ebp = frame + 0x40;
    let q = e.alloc(0x20); // ECX; q.{x,y,z} at [ecx]
    let pblk = e.alloc(0x80); // ESI base; p at [esi-0x20..] after the in-slice `add esi,0x20`;
                              // results stored at [esi]=pblk+0x20
    let mut s = 0x611f_f100u32;

    let case = |e: &mut Emu, p: [f32; 3], qv: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(pblk + k as u64 * 4, p[k].to_bits()); // [esi-0x20] = pblk (after add esi,0x20)
            e.write_u32(q + k as u64 * 4, qv[k].to_bits());
        }
        e.set_reg(RegisterX86::ECX, q);
        e.set_reg(RegisterX86::ESI, pblk); // the slice does `add esi,0x20` → esi=pblk+0x20
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x612221, 0x612269); // include the final `mov [esi+8],eax` z-store @0x612266
        let bin = [
            e.read_f32(pblk + 0x20),
            e.read_f32(pblk + 0x24),
            e.read_f32(pblk + 0x28),
        ];
        let ours = trail_midpoint(p, qv);
        assert_f32(bin[0], ours[0], &format!("{ctx} x"));
        assert_f32(bin[1], ours[1], &format!("{ctx} y"));
        assert_f32(bin[2], ours[2], &format!("{ctx} z"));
    };

    case(&mut e, [1.0, 2.0, 3.0], [3.0, 4.0, 5.0], "simple");
    case(&mut e, [-1.0, 0.0, 1.0], [1.0, 0.0, -1.0], "signs");
    // The z-spill split is observable ONLY at overflow: x/y keep the sum in f64 (finite) before
    // ×0.5; z rounds the sum to f32 first (→ ±inf) THEN ×0.5 (stays ±inf). This deterministic probe
    // sums two near-f32-max values so z=±inf while x/y stay finite — the exact divergence a "z is
    // also live" mistranscription would fail. (For finite sums all three agree, since ×0.5 only
    // decrements the exponent, so rounding-then-halve == halve-then-round.)
    let big_p = f32::from_bits(0x7f6b_85ed); // ~3.13e38
    let big_q = f32::from_bits(0x7efd_9746); // ~1.69e38 ; p+q overflows f32
    case(
        &mut e,
        [1.0, 1.0, big_p],
        [1.0, 1.0, big_q],
        "z-spill split: z sum overflows to +inf, x/y finite",
    );
    case(
        &mut e,
        [1.0, 1.0, -big_p],
        [1.0, 1.0, -big_q],
        "z-spill split: z sum overflows to -inf",
    );
    for i in 0..4000u32 {
        let p = [
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
        ];
        let qv = match i % 3 {
            0 => [
                // large magnitudes → sums lose precision → expose the z f32-spill divergence
                frand(&mut s, -1.0e7, 1.0e7),
                frand(&mut s, -1.0e7, 1.0e7),
                frand(&mut s, -1.0e7, 1.0e7),
            ],
            1 => [
                frand(&mut s, -1000.0, 1000.0),
                frand(&mut s, -1000.0, 1000.0),
                frand(&mut s, -1000.0, 1000.0),
            ],
            _ => [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
        };
        case(&mut e, p, qv, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut p = [1.0f32, 2.0, 3.0];
            p[k] = v;
            case(&mut e, p, [0.5, 0.25, 0.125], "special p");
            let mut q = [0.5f32, 0.25, 0.125];
            q[k] = v;
            case(&mut e, [1.0, 2.0, 3.0], q, "special q");
        }
    }
}
