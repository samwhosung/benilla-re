//! The place rungs of `CheckCasterStateForSpell` (`0x6094f0`) against the exe: the slice from
//! `0x609f59`, the load of `Attributes` after the reputation rung, to the pass exit `0x60a049`,
//! with `AttributesEx2` bit 16 clear so the taming rung that follows is not entered.
//!
//! The slice runs with the validator's own frame (record in `esi`, caster in `edi` and
//! `[ebp-4]`, the arguments at `ebx`). The outdoors thunk `0x612cc0` and the reporter `0x6e1a00`
//! are stubbed: the first answers a chosen `eax` and logs its `ecx`, the second logs its `ecx`,
//! `dl` and stack arguments. Every combination of the two attribute bits, of whether each effect
//! slot applies the mounted aura and of the answer runs, the answer's upper bytes set at random
//! (only `al` is read), then a fuzz over the record fields. `0x612cc0` and `0x6706f0` are not run
//! here (`terrain::entity_outdoors_difftest` runs the second whole).

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::castplace::{cast_place_rung, PlaceRung};
use common::next;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const SLICE_START: u64 = 0x609f59;
const OUTDOORS_THUNK: u64 = 0x612cc0;
const REPORTER: u64 = 0x6e1a00;
/// The return address the harness halts on (`Emu::call` uses the same value).
const RET_SENTINEL: u64 = 0x6f6f_6f6f;

/// The stack frame the validator's prologue builds: `ebx` the argument frame, `ebp` the local
/// frame whose `[ebp-4]` holds the caster.
const FRAME_EBX: u64 = 0x7008_2000;
const FRAME_EBP: u64 = 0x7008_1000;

const RECORD_SIZE: u64 = 0x300;

#[derive(Clone, Copy)]
struct Case {
    attributes: u32,
    attributes_ex2: u32,
    effects: [u32; 3],
    auras: [u32; 3],
    answer: u32,
    spell_id: u32,
}

/// A value that is `special` when `hit` and otherwise differs from it.
fn pick(s: &mut u32, hit: bool, special: u32) -> u32 {
    if hit {
        return special;
    }
    loop {
        let v = match (next(s) >> 8) % 4 {
            0 => 0,
            1 => special ^ 1,
            2 => special.wrapping_add(0x1_0000),
            _ => next(s),
        };
        if v != special {
            return v;
        }
    }
}

/// [`pick`] with the hit chosen at random, one time in four.
fn pick_rand(s: &mut u32, special: u32) -> u32 {
    let hit = next(s) & 0x300 == 0;
    pick(s, hit, special)
}

#[test]
fn cast_place_rung_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let record = e.alloc(RECORD_SIZE);
    let caster = e.alloc(0x100);
    let answer = Rc::new(Cell::new(0u32));
    let asked = Rc::new(RefCell::new(Vec::<u32>::new()));
    let reports = Rc::new(RefCell::new(Vec::<(u32, u32, [u32; 3])>::new()));
    {
        let (answer, asked) = (answer.clone(), asked.clone());
        e.stub_fn(OUTDOORS_THUNK, 0, move |io| {
            asked.borrow_mut().push(io.reg(RegisterX86::ECX) as u32);
            answer.get()
        });
    }
    {
        let reports = reports.clone();
        e.stub_fn(REPORTER, 3, move |io| {
            reports.borrow_mut().push((
                io.reg(RegisterX86::ECX) as u32,
                io.reg(RegisterX86::EDX) as u32 & 0xff,
                [io.arg(0), io.arg(1), io.arg(2)],
            ));
            0
        });
    }

    let run = |e: &mut Emu, c: Case, ctx: &str| {
        e.write(record, &vec![0u8; RECORD_SIZE as usize]);
        e.write_u32(record, c.spell_id);
        e.write_u32(record + 0x18, c.attributes);
        e.write_u32(record + 0x20, c.attributes_ex2);
        for i in 0..3 {
            e.write_u32(record + 0xf4 + 4 * i as u64, c.effects[i]);
            e.write_u32(record + 0x16c + 4 * i as u64, c.auras[i]);
        }
        e.write_u32(FRAME_EBP, 0);
        e.write_u32(FRAME_EBP - 4, caster as u32);
        e.write_u32(FRAME_EBX, 0);
        e.write_u32(FRAME_EBX + 4, RET_SENTINEL as u32);
        e.write_u32(FRAME_EBX + 8, record as u32);
        e.write_u32(FRAME_EBX + 12, 0);
        e.set_reg(RegisterX86::EBX, FRAME_EBX);
        e.set_reg(RegisterX86::EBP, FRAME_EBP);
        e.set_reg(RegisterX86::ESP, FRAME_EBP - 0x40);
        e.set_reg(RegisterX86::ESI, record);
        e.set_reg(RegisterX86::EDI, caster);
        e.set_reg(RegisterX86::EAX, 0xdead_beef);
        e.set_reg(RegisterX86::ECX, 0xfeed_f00d);
        e.set_reg(RegisterX86::EDX, 0x1234_5678);
        answer.set(c.answer);
        asked.borrow_mut().clear();
        reports.borrow_mut().clear();
        e.run(SLICE_START, RET_SENTINEL);

        let ours = cast_place_rung(c.attributes, c.effects, c.auras, c.answer & 0xff != 0);
        let eax = e.get_reg(RegisterX86::EAX) as u32;
        let asked = asked.borrow();
        let reports = reports.borrow();
        let bin = PlaceRung {
            asked: !asked.is_empty(),
            refusal: reports.first().map(|r| r.1 as u8),
        };
        assert_eq!(bin, ours, "{ctx}: attributes {:#x}", c.attributes);
        assert!(
            asked.len() <= 1 && reports.len() <= 1,
            "{ctx}: one call each"
        );
        assert!(
            asked.iter().all(|&ecx| ecx == caster as u32),
            "{ctx}: the thunk takes the caster in ecx"
        );
        if let Some(&(ecx, _, args)) = reports.first() {
            assert_eq!(ecx, c.spell_id, "{ctx}: the report names the spell");
            assert_eq!(args, [0xffff_ffff, 0xffff_ffff, 0], "{ctx}: report args");
        }
        assert_eq!(
            eax & 0xff,
            u32::from(ours.refusal.is_none()),
            "{ctx}: the slice returns 1 on a pass, 0 on a refusal"
        );
    };

    let mut s = 0x609f_5900u32;
    let mut cases = 0u32;

    // The four attribute-bit pairs, each slot applying the mounted aura or not through each of the
    // two compares, both answers, with random bytes around the tested ones.
    for pair in 0u32..4 {
        for slots in 0u32..64 {
            for outdoors in [0u32, 1] {
                for rep in 0..4 {
                    let mut attributes = next(&mut s) & !0xc000;
                    if pair & 1 != 0 {
                        attributes |= 0x4000;
                    }
                    if pair & 2 != 0 {
                        attributes |= 0x8000;
                    }
                    let mut effects = [0u32; 3];
                    let mut auras = [0u32; 3];
                    for i in 0..3 {
                        effects[i] = pick(&mut s, (slots >> (2 * i)) & 1 != 0, 6);
                        auras[i] = pick(&mut s, (slots >> (2 * i + 1)) & 1 != 0, 0x4e);
                    }
                    // The answer: al is the answer, the upper bytes any.
                    let junk = next(&mut s) & !0xff;
                    run(
                        &mut e,
                        Case {
                            attributes,
                            attributes_ex2: next(&mut s) & !0x10000,
                            effects,
                            auras,
                            answer: junk | (outdoors * (1 + rep * 0x3f % 0xff)),
                            spell_id: next(&mut s),
                        },
                        "sweep",
                    );
                    cases += 1;
                }
            }
        }
    }

    // Every single attribute bit with the answer both ways: only bits 14 and 15 are read.
    for bit in 0..32 {
        for outdoors in [0u32, 1] {
            run(
                &mut e,
                Case {
                    attributes: 1 << bit,
                    attributes_ex2: 0,
                    effects: [6, 6, 6],
                    auras: [0x4e, 0x4e, 0x4e],
                    answer: outdoors,
                    spell_id: 7,
                },
                "bits",
            );
            run(
                &mut e,
                Case {
                    attributes: !(1u32 << bit),
                    attributes_ex2: 0,
                    effects: [6, 0, 6],
                    auras: [0, 0x4e, 0],
                    answer: outdoors,
                    spell_id: 7,
                },
                "bits",
            );
            cases += 2;
        }
    }

    // Fuzz.
    for _ in 0..3000 {
        run(
            &mut e,
            Case {
                attributes: next(&mut s),
                attributes_ex2: next(&mut s) & !0x10000,
                effects: [
                    pick_rand(&mut s, 6),
                    pick_rand(&mut s, 6),
                    pick_rand(&mut s, 6),
                ],
                auras: [
                    pick_rand(&mut s, 0x4e),
                    pick_rand(&mut s, 0x4e),
                    pick_rand(&mut s, 0x4e),
                ],
                answer: next(&mut s),
                spell_id: next(&mut s),
            },
            "fuzz",
        );
        cases += 1;
    }
    eprintln!("cast_place_rung: {cases} cases bit-exact vs WoW.exe, slice {SLICE_START:#x}");
}
