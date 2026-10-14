//! `UnitAttackPower` (`0x518f80`) and `UnitRangedAttackPower` (`0x5190c0`)
//! (`systems/ui/unitstats.md#attack-power`) run from the exe.
//!
//! The unit lookup and the Lua glue are stubbed, and `lua_pushnumber` (`0x6f3810`) is captured; the
//! test seeds the unit's multiplier, base and modifiers, and compares the three pushed numbers with
//! `unit_attack_power`. Ties in the rounding are probed on purpose, as `fistp` rounds half to even.
//! Every draw comes off the high bits of an LCG.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::unit_stats::unit_attack_power as ap_calc;

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

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

// ===========================================================================================
// UnitAttackPower — melee 0x518f80 / ranged 0x5190c0 (whole-call, glue stubbed)
// ===========================================================================================
struct ApRig {
    typeinfo: u64,
    block: u64,
}

fn ap_setup(emu: &mut Emu) -> ApRig {
    let unit = emu.alloc(0x140);
    let typeinfo = emu.alloc(0x10);
    let block = emu.alloc(0x300);
    emu.write_u32(unit + 8, typeinfo as u32);
    emu.write_u32(unit + 0x110, block as u32);
    emu.stub(0x6f3510, 1, 0); // CScriptObject type-guard ok
    emu.stub(0x6f3690, 0, 0); // arg fetch
    emu.stub(0x515940, unit as u32, 0); // resolve -> our unit
    emu.stub(0x6f3810, 0, 2); // lua_pushnumber(L, double): capture, ret 8
    ApRig { typeinfo, block }
}

/// `off` = the melee/ranged block field offsets (base i32, posMod i16, negMod i16, mult f32).
fn ap_check(
    emu: &mut Emu,
    rig: &ApRig,
    fn_va: u64,
    off: (u64, u64, u64, u64),
    mult: f32,
    base: i32,
    pos: i16,
    neg: i16,
    is_unit: bool,
) {
    emu.write_u32(rig.block + off.0, base as u32);
    emu.write(rig.block + off.1, &(pos as u16).to_le_bytes());
    emu.write(rig.block + off.2, &(neg as u16).to_le_bytes());
    emu.write_u32(rig.block + off.3, mult.to_bits());
    emu.write_u32(rig.typeinfo + 8, if is_unit { 0x8 } else { 0 });
    emu.clear_calls();
    emu.call(fn_va, 0x1234, 0, &[]);
    let p = pushes(emu);
    assert_eq!(p.len(), 3, "expected 3 pushnumber, got {}", p.len());
    let mine = ap_calc(mult, base, pos, neg, is_unit);
    for i in 0..3 {
        assert_eq!(
            p[i].to_bits(),
            f64::from(mine[i]).to_bits(),
            "AP fn={fn_va:#x} idx={i} mult={mult} base={base} pos={pos} neg={neg} unit={is_unit}: bin={} mine={}",
            p[i],
            mine[i]
        );
    }
}

const MELEE_OFF: (u64, u64, u64, u64) = (0x27c, 0x280, 0x282, 0x284);

const RANGED_OFF: (u64, u64, u64, u64) = (0x288, 0x28c, 0x28e, 0x290);

fn ap_fuzz(fn_va: u64, off: (u64, u64, u64, u64), seed: u32, lbl: &str) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rig = ap_setup(&mut emu);
    // guard-fail path: pushes 0,0,0.
    ap_check(&mut emu, &rig, fn_va, off, 1.5, 100, 20, -10, false);
    // round-half-to-even boundary: (mult+1)*value lands on *.5 ⇒ ties-to-even.
    ap_check(&mut emu, &rig, fn_va, off, 0.0, 5, 3, -3, true); // (1)*v
    ap_check(&mut emu, &rig, fn_va, off, 0.5, 5, 7, -1, true); // 1.5*5=7.5 -> 8 (even)
    ap_check(&mut emu, &rig, fn_va, off, 0.5, 3, 9, -7, true); // 1.5*3=4.5 -> 4 (even)
                                                               // NaN, the infinities, and products past `i32` and `i64`: the `fistp` stores `0x80000000`.
    for mult in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.0e9,
        -1.0e9,
        1.0e30,
        -1.0e30,
    ] {
        ap_check(&mut emu, &rig, fn_va, off, mult, 5000, 3, -3, true);
        ap_check(&mut emu, &rig, fn_va, off, mult, i32::MAX, 20, -10, true);
        ap_check(&mut emu, &rig, fn_va, off, mult, 0, 0, 0, true);
    }
    let mut s = seed;
    let mut unit_n = 0u32;
    for _ in 0..2000 {
        let is_unit = (next(&mut s) >> 8) & 1 == 1;
        let mult = rng(&mut s, -1.0, 6.0);
        let base = (next(&mut s) >> 8) as i32 % 20000 - 5000;
        let pos = ((next(&mut s) >> 8) % 4000) as i16 - 2000;
        let neg = ((next(&mut s) >> 8) % 4000) as i16 - 2000;
        ap_check(&mut emu, &rig, fn_va, off, mult, base, pos, neg, is_unit);
        if is_unit {
            unit_n += 1;
        }
    }
    eprintln!("{lbl}: 2004 cases bit-exact (is_unit hits={unit_n})");
    assert!(unit_n > 0 && unit_n < 2000, "need both guard outcomes");
}

#[test]
fn unit_attack_power() {
    ap_fuzz(0x518f80, MELEE_OFF, 0x0518_f800, "unit_attack_power");
}

#[test]
fn unit_ranged_attack_power() {
    ap_fuzz(
        0x5190c0,
        RANGED_OFF,
        0x0519_0c00,
        "unit_ranged_attack_power",
    );
}
