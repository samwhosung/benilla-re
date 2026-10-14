//! `CGPlayer_C::GetSpellCastTime` (`0x5ee150`) against the exe, run whole on a synthesized player
//! (a vtable whose slot `0xa8` returns the level, a speed sub-object at `+0x110`, a descriptor at
//! `+0x8`) and `Spell.dbc` and `SpellCastTimes.dbc` records reached through the table pointers at
//! `0xc0d788` and `0xc0d878`. The cost is compared bit for bit.
//!
//! Every branch runs: the early-outs for a spell id below 0, past the table or null and for a
//! cast-time index below 0, past the table or null; the `/ 5` level term across its boundaries;
//! the floor taken and not taken; the speed gate over all 8 subsets of Attributes 0x2, 0x10 and
//! 0x20 and the speeds around `1 ± 2^-22`; and the 500 and the final clamp to 0. A second test
//! probes the speed and the times with NaN, infinities, zeros, products past `i32` and the
//! threshold exactly.
//!
//! The active-player overlay is outside: the stubbed `0x468550` returns 0 and the descriptor guid
//! is nonzero, so `0x6e6af0` never runs.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::cast_time::{player_cast_time, CastTimeRec, SpellRec};
use common::{next, next_down, next_up, specials};

const CAST_TIME_FN: u64 = 0x5ee150;
/// `ClntObjMgrGetActivePlayer`, stubbed to 0 so the active-player gate is not taken.
const ACTIVE_GUID_FN: u64 = 0x468550;
/// `ApplySpellModifierInt`, stubbed in case it is reached; it is not.
const OVERLAY_FN: u64 = 0x6e6af0;

const SPELL_ID: u32 = 100;
const MAX_SPELL: u32 = 200;
const CAST_IDX: i32 = 50;
const MAX_CAST: i32 = 100;

/// The synthesized object and record graph, allocated once and rewritten per case.
struct Fixture {
    this_va: u64,
    speed_obj: u64,
    level_slot: u64,
    spell_rec: u64,
    cast_rec: u64,
    spell_table: u64,
    cast_table: u64,
}

fn build_fixture(e: &mut Emu) -> Fixture {
    let vtable = e.alloc(0xb0);
    let level_slot = e.alloc(4);
    // The level getter `mov eax,[level_slot]; ret 4`, a thiscall that pops the spell id.
    let stub = e.alloc(8);
    let mut code = vec![0xa1u8];
    code.extend_from_slice(&(level_slot as u32).to_le_bytes());
    code.extend_from_slice(&[0xc2, 0x04, 0x00]);
    e.write(stub, &code);
    e.write_u32(vtable + 0xa8, stub as u32);

    let descriptor = e.alloc(8);
    e.write_u32(descriptor, 1); // a nonzero guid never equals the stubbed active guid, 0
    e.write_u32(descriptor + 4, 0);

    let speed_obj = e.alloc(0x230);
    let spell_rec = e.alloc(0x74);
    let cast_rec = e.alloc(0x10);

    let this_va = e.alloc(0x114);
    e.write_u32(this_va, vtable as u32);
    e.write_u32(this_va + 8, descriptor as u32);
    e.write_u32(this_va + 0x110, speed_obj as u32);

    let spell_table = e.alloc(u64::from(MAX_SPELL + 1) * 4);
    let cast_table = e.alloc((MAX_CAST as u64 + 1) * 4);
    e.write_u32(spell_table + u64::from(SPELL_ID) * 4, spell_rec as u32);
    e.write_u32(cast_table + CAST_IDX as u64 * 4, cast_rec as u32);

    e.write_u32(0xc0d788, spell_table as u32);
    e.write_u32(0xc0d78c, MAX_SPELL);
    e.write_u32(0xc0d878, cast_table as u32);
    e.write_u32(0xc0d87c, MAX_CAST as u32);

    Fixture {
        this_va,
        speed_obj,
        level_slot,
        spell_rec,
        cast_rec,
        spell_table,
        cast_table,
    }
}

/// `0x5ee150(this, spell_id)`.
fn run_bin(e: &mut Emu, this_va: u64, spell_id: u32) -> u32 {
    e.call(CAST_TIME_FN, this_va as u32, 0, &[spell_id])
}

/// One input to a resolving case.
#[derive(Clone, Copy)]
struct Case {
    level: u32,
    spell_level: i32,
    per_level: i32,
    base: i32,
    min: u32,
    flags: u8,
    speed: f32,
}

/// One case with both records present, compared bit for bit.
fn check_resolved(e: &mut Emu, fx: &Fixture, c: Case, ctx: &str) {
    // Restore the table wiring the early-out cases perturb.
    e.write_u32(
        fx.spell_table + u64::from(SPELL_ID) * 4,
        fx.spell_rec as u32,
    );
    e.write_u32(fx.cast_table + CAST_IDX as u64 * 4, fx.cast_rec as u32);

    e.write_u32(fx.level_slot, c.level);
    e.write_u32(fx.speed_obj + 0x22c, c.speed.to_bits());
    e.write_u32(fx.spell_rec + 0x18, u32::from(c.flags));
    e.write_u32(fx.spell_rec + 0x48, CAST_IDX as u32);
    e.write_u32(fx.spell_rec + 0x70, c.spell_level as u32);
    e.write_u32(fx.cast_rec + 0x4, c.base as u32);
    e.write_u32(fx.cast_rec + 0x8, c.per_level as u32);
    e.write_u32(fx.cast_rec + 0xc, c.min);

    let bin = run_bin(e, fx.this_va, SPELL_ID);

    let spell = SpellRec {
        flags: c.flags,
        cast_time_idx: CAST_IDX,
        spell_level: c.spell_level,
    };
    let cast = CastTimeRec {
        base: c.base,
        per_level: c.per_level,
        min: c.min,
    };
    let ours = player_cast_time(Some(&spell), Some(&cast), c.level, c.speed);
    assert_eq!(bin, ours, "{ctx}: time (bin {bin:#x} != ours {ours:#x})");
}

#[test]
fn player_cast_time_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    e.stub(ACTIVE_GUID_FN, 0, 0);
    e.stub(OVERLAY_FN, 0, 1);
    let fx = build_fixture(&mut e);

    // The early-outs return 0 without the level call: a spell id past the table, below 0, and a
    // null spell record.
    e.write_u32(fx.spell_rec + 0x48, CAST_IDX as u32);
    for (spell_id, ctx) in [(250u32, "spell>max"), (0x8000_0000u32, "spell<0")] {
        assert_eq!(run_bin(&mut e, fx.this_va, spell_id), 0, "{ctx}: bin");
        assert_eq!(player_cast_time(None, None, 0, 1.0), 0, "{ctx}: ours");
    }
    e.write_u32(fx.spell_table + u64::from(SPELL_ID) * 4, 0);
    assert_eq!(run_bin(&mut e, fx.this_va, SPELL_ID), 0, "spell-null: bin");
    assert_eq!(player_cast_time(None, None, 0, 1.0), 0, "spell-null: ours");
    e.write_u32(
        fx.spell_table + u64::from(SPELL_ID) * 4,
        fx.spell_rec as u32,
    );
    // A cast-time index past the table, below 0, and a null record, with the spell resolving.
    for (idx, ctx) in [(150i32, "cast>max"), (-1i32, "cast<0")] {
        e.write_u32(fx.spell_rec + 0x48, idx as u32);
        let spell = SpellRec {
            flags: 0,
            cast_time_idx: idx,
            spell_level: 0,
        };
        assert_eq!(run_bin(&mut e, fx.this_va, SPELL_ID), 0, "{ctx}: bin");
        assert_eq!(
            player_cast_time(Some(&spell), None, 0, 1.0),
            0,
            "{ctx}: ours"
        );
    }
    e.write_u32(fx.spell_rec + 0x48, CAST_IDX as u32);
    e.write_u32(fx.cast_table + CAST_IDX as u64 * 4, 0);
    assert_eq!(run_bin(&mut e, fx.this_va, SPELL_ID), 0, "cast-null: bin");
    let spell = SpellRec {
        flags: 0,
        cast_time_idx: CAST_IDX,
        spell_level: 0,
    };
    assert_eq!(
        player_cast_time(Some(&spell), None, 0, 1.0),
        0,
        "cast-null: ours"
    );
    e.write_u32(fx.cast_table + CAST_IDX as u64 * 4, fx.cast_rec as u32);

    // The kernel: all flag subsets, the speeds around the dead band, and the level, floor and
    // sign probes.
    let one = 1.0f32.to_bits();
    let speeds: [(f32, &str); 6] = [
        (1.0, "speed=1"),
        (1.5, "speed=1.5"),
        (0.5, "speed=0.5"),
        (f32::from_bits(one + 1), "speed=1+1ulp"),
        (f32::from_bits(one + 2), "speed=1+eps"), // 2^-22: equal to the threshold
        (f32::from_bits(one - 1), "speed=1-1ulp"),
    ];
    // (level, spell_level, per_level, base, min)
    let scales: [(u32, i32, i32, i32, u32, &str); 7] = [
        (1000, 50, 7, 10, 0, "pos"),             // (200 - 50) * 7 + 10 = 1060
        (4, 0, 3, 25, 0, "level<5"),             // 0 * 3 + 25: the floor of /5
        (6, 0, 11, 0, 0, "level=6"),             // 1 * 11
        (1000, 50, 7, 10, 5000, "min-clamp"),    // 1060 raised to 5000
        (10, 100, 5, 0, 0, "neg-to-0min"),       // -490 raised to 0
        (10, 100, 5, 0, 0xffff_fff0, "neg-min"), // -490 against a minimum of -16: -16
        (3000, 0, 1, 0, 0, "big"),               // 600
    ];
    for flag_bits in 0u8..8 {
        let flags = (if flag_bits & 1 != 0 { 0x2 } else { 0 })
            | (if flag_bits & 2 != 0 { 0x10 } else { 0 })
            | (if flag_bits & 4 != 0 { 0x20 } else { 0 });
        for (speed, sname) in speeds {
            for (level, spell_level, per_level, base, min, name) in scales {
                let c = Case {
                    level,
                    spell_level,
                    per_level,
                    base,
                    min,
                    flags,
                    speed,
                };
                check_resolved(&mut e, &fx, c, &format!("flags={flags:#x} {sname} {name}"));
            }
        }
    }

    let mut seed = 0x59e1_1c05u32;
    for i in 0..3000 {
        let c = Case {
            level: next(&mut seed) % 20_000,
            spell_level: (next(&mut seed) % 400) as i32,
            per_level: ((next(&mut seed) % 100) as i32) - 50,
            base: ((next(&mut seed) % 7000) as i32) - 2000,
            min: next(&mut seed) % 3000,
            flags: (next(&mut seed) & 0x32) as u8,
            // A speed in [0, 3) straddles the dead band and exercises ties to even.
            speed: (next(&mut seed) >> 8) as f32 / (1u32 << 24) as f32 * 3.0,
        };
        check_resolved(&mut e, &fx, c, &format!("fuzz {i}"));
    }
}

/// The speed and the time with the values a compare or a conversion distinguishes: NaN and the
/// infinities, both zeros, the dead-band threshold `1 ± 2^-22` exactly and a step either side, and
/// times whose product with the speed leaves `i32`, which `fistp` stores as `0x80000000`.
#[test]
fn player_cast_time_special_values_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    e.stub(ACTIVE_GUID_FN, 0, 0);
    e.stub(OVERLAY_FN, 0, 1);
    let fx = build_fixture(&mut e);

    let eps = f32::from_bits(0x3480_0000);
    let mut speeds = specials(&[]);
    for t in [1.0 + eps, 1.0 - eps] {
        speeds.extend([t, next_up(t), next_down(t)]);
    }
    speeds.extend([2.5, 0.5, 4.0e9, -4.0e9, 1.0e10, 2147483648.0, -2147483648.0]);

    // (level, spell_level, per_level, base, min): small, near i32::MAX and a negative total.
    let times: [(u32, i32, i32, i32, u32); 5] = [
        (1000, 50, 7, 10, 0),
        (0, 0, 0, 1, 0),
        (5, 0, 0, i32::MAX, 0),
        (5, 0, 0, 0x4000_0000, 0),
        (5, 100, 5, 0, 0),
    ];
    for flags in [0u8, 0x2, 0x10, 0x20] {
        for &speed in &speeds {
            for &(level, spell_level, per_level, base, min) in &times {
                let c = Case {
                    level,
                    spell_level,
                    per_level,
                    base,
                    min,
                    flags,
                    speed,
                };
                check_resolved(
                    &mut e,
                    &fx,
                    c,
                    &format!(
                        "special flags={flags:#x} speed={speed:e} ({:#010x}) time base={base:#x}",
                        speed.to_bits()
                    ),
                );
            }
        }
    }
}
