//! `CanInspect` (`0x48a1b0`) run whole against the exe (`systems/ui/distancegates.md#caninspect`).
//!
//! `CanInspect` runs with the Lua stack calls, the token resolver `0x515940`, the cooperation test
//! `0x606ba0`, the message display `0x496720` and the GUID lookup `0x464870` stubbed. The active
//! player's GUID getter `0x468550`, the typed lookup `0x468460`, the answer function `0x4944a0` and
//! the inline distance test are the exe's own. The test logs, in order, the usage error, each
//! message and the value pushed, and compares them with `distance_gates::can_inspect` over every
//! combination of: an argument that is or is not a string or number, an empty or nonempty token, no
//! object or an object that is or is not a player and is or is not the player's own and that the
//! player does or does not cooperate with, no active player in two ways, and positions on each side
//! of the squared distance 100 (the cell `0xb4d918`, which the static initializer `0x48cc80` fills
//! and the test writes), exactly on it, NaN and infinite.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::distance_gates::{can_inspect, InspectEffect, InspectTarget, InspectWorld};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const CAN_INSPECT: u64 = 0x48a1b0;
const LUA_ISSTRING: u64 = 0x6f3510;
const LUA_TOSTRING: u64 = 0x6f3690;
const LUA_ERROR: u64 = 0x6f4940;
const LUA_PUSHNUMBER: u64 = 0x6f3810;
const LUA_PUSHNIL: u64 = 0x6f37f0;
const GET_UNIT_FROM_TOKEN: u64 = 0x515940;
const CAN_COOPERATE: u64 = 0x606ba0;
const DISPLAY_ERROR: u64 = 0x496720;
const GET_OBJECT_PTR: u64 = 0x464870;
const MGR_PTR: u64 = 0xb41414;
const THRESHOLD: u64 = 0xb4d918;
const USAGE_FORMAT: u32 = 0x84270c;

/// What the stubs saw, in order.
#[derive(Clone, Debug, PartialEq)]
enum Seen {
    Usage(u32),
    Message(u32),
    PushNumber(f64),
    PushNil,
}

fn effects(seen: &[Seen]) -> Vec<InspectEffect> {
    seen.iter()
        .map(|s| match s {
            Seen::Usage(f) => {
                assert_eq!(*f, USAGE_FORMAT, "the usage text");
                InspectEffect::Usage
            }
            Seen::Message(m) => InspectEffect::Message(*m),
            Seen::PushNumber(v) => {
                assert_eq!(*v, 1.0, "the number pushed");
                InspectEffect::PushOne
            }
            Seen::PushNil => InspectEffect::PushNil,
        })
        .collect()
}

#[test]
fn can_inspect_whole_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let seen: Rc<RefCell<Vec<Seen>>> = Rc::default();

    // The inputs the stubs answer from, set per case.
    let arg_ok = Rc::new(Cell::new(true));
    let token = Rc::new(Cell::new(0u32));
    let target_obj = Rc::new(Cell::new(0u32));
    let cooperates = Rc::new(Cell::new(false));
    let player_obj = Rc::new(Cell::new(0u32));
    let active_guid = 0x0000_0004_0000_0abcu64;

    let c = arg_ok.clone();
    e.stub_fn(LUA_ISSTRING, 0, move |_| u32::from(c.get()));
    let t = token.clone();
    e.stub_fn(LUA_TOSTRING, 0, move |_| t.get());
    let s = seen.clone();
    e.stub_fn(LUA_ERROR, 0, move |io| {
        s.borrow_mut().push(Seen::Usage(io.arg(1)));
        0
    });
    let s = seen.clone();
    e.stub_fn(DISPLAY_ERROR, 0, move |io| {
        s.borrow_mut().push(Seen::Message(io.arg(0)));
        0
    });
    let s = seen.clone();
    e.stub_fn(LUA_PUSHNUMBER, 2, move |io| {
        let bits = (u64::from(io.arg(1)) << 32) | u64::from(io.arg(0));
        s.borrow_mut().push(Seen::PushNumber(f64::from_bits(bits)));
        0
    });
    let s = seen.clone();
    e.stub_fn(LUA_PUSHNIL, 0, move |_| {
        s.borrow_mut().push(Seen::PushNil);
        0
    });
    let o = target_obj.clone();
    e.stub_fn(GET_UNIT_FROM_TOKEN, 0, move |_| o.get());
    let coop = cooperates.clone();
    e.stub_fn(CAN_COOPERATE, 1, move |_| u32::from(coop.get()));
    // GetObjectPtr: the player's GUID names the player's object, when there is one.
    let p = player_obj.clone();
    e.stub_fn(GET_OBJECT_PTR, 2, move |io| {
        let guid = (u64::from(io.arg(1)) << 32) | u64::from(io.arg(0));
        if guid == active_guid {
            p.get()
        } else {
            0
        }
    });
    // The position getter, vtable slot +0x14 of every object: the vector at the object's +0x20.
    let getter = e.alloc(0x10);
    e.stub_fn(getter, 1, |io| io.reg(RegisterX86::ECX) as u32 + 0x20);
    let vtable = e.alloc(0x40);
    e.write_u32(vtable + 0x14, getter as u32);

    let empty = e.alloc(0x10) as u32;
    let named = e.alloc(0x10) as u32;
    e.write(u64::from(named), b"player\0");
    let mgr = e.alloc(0xe0);
    e.write(mgr + 0xc0, &active_guid.to_le_bytes());

    // Two objects: the player's and another unit's.
    let make = |e: &mut Emu| -> (u32, u32) {
        let obj = e.alloc(0x120) as u32;
        let desc = e.alloc(0x40) as u32;
        e.write_u32(u64::from(obj), vtable as u32);
        e.write_u32(u64::from(obj) + 8, desc);
        (obj, desc)
    };
    let (player_va, player_desc) = make(&mut e);
    let (other_va, other_desc) = make(&mut e);
    e.write_u32(THRESHOLD, 100.0f32.to_bits());

    let player_positions: [[f32; 3]; 2] = [[0.0, 0.0, 0.0], [1.5, -2.25, 300.0]];
    // Target offsets from the player: in, on and past 100, NaN and infinity.
    let offsets: [[f32; 3]; 12] = [
        [0.0, 0.0, 0.0],
        [3.0, 4.0, 0.0],
        [10.0, 0.0, 0.0],
        [6.0, 8.0, 0.0],
        [0.0, 0.0, -10.0],
        [10.000001, 0.0, 0.0],
        [6.0, 8.0, 0.001],
        [0.0, -10.0, 0.01],
        [100.0, 100.0, 100.0],
        [f32::NAN, 0.0, 0.0],
        [0.0, f32::INFINITY, 0.0],
        [0.0, 0.0, f32::NEG_INFINITY],
    ];
    // (the object exists, has the player bit, is the player's own, the player cooperates):
    // none, then another unit or the player's own object with each answer.
    let kinds: [(bool, bool, bool, bool); 7] = [
        (false, false, false, false),
        (true, true, false, true),
        (true, true, false, false),
        (true, false, false, true),
        (true, false, false, false),
        (true, true, true, false),
        (true, false, true, true),
    ];
    let objects = [(player_va, player_desc), (other_va, other_desc)];
    let mut cases = 0u32;
    let mut tally = [0u32; 6];
    let mut run = |e: &mut Emu, w: InspectWorld, state: u32, pos: [f32; 3], pp: [f32; 3]| {
        let (exists, is_player, is_self, coop) = match w.target {
            None => (false, false, false, false),
            Some(t) => (true, t.is_player, t.is_self, t.cooperates),
        };
        arg_ok.set(w.arg_ok);
        token.set(if w.token_empty { empty } else { named });
        cooperates.set(coop);
        e.write_u32(MGR_PTR, if state == 0 { 0 } else { mgr as u32 });
        player_obj.set(if state == 2 { player_va } else { 0 });
        for (i, &(va, desc)) in objects.iter().enumerate() {
            let at = if i == 0 { pp } else { pos };
            // The player's object has the player bit unless it is the target and lacks it.
            let ty = if (i == 0 && !(is_self && !is_player)) || (i == 1 && is_player) {
                0x19
            } else {
                0x9
            };
            e.write_u32(u64::from(desc) + 8, ty);
            for (k, v) in at.iter().enumerate() {
                e.write_u32(u64::from(va) + 0x20 + 4 * k as u64, v.to_bits());
            }
        }
        target_obj.set(match (exists, is_self) {
            (false, _) => 0,
            (true, true) => player_va,
            (true, false) => other_va,
        });

        seen.borrow_mut().clear();
        let ret = e.call(CAN_INSPECT, 0x1000, 1, &[]);
        let got = seen.borrow().clone();
        let want = can_inspect(&w);
        assert_eq!(effects(&got), want, "world {w:?}");
        assert_eq!(ret, u32::from(w.arg_ok), "eax for {w:?}");
        cases += 1;
        for x in &want {
            match x {
                InspectEffect::Usage => tally[0] += 1,
                InspectEffect::Message(295) => tally[1] += 1,
                InspectEffect::Message(184) => tally[2] += 1,
                InspectEffect::Message(297) => tally[3] += 1,
                InspectEffect::Message(125) => tally[4] += 1,
                InspectEffect::PushOne => tally[5] += 1,
                _ => {}
            }
        }
    };

    // An argument that is neither a string nor a number raises the usage error and nothing else.
    run(
        &mut e,
        InspectWorld {
            arg_ok: false,
            token_empty: false,
            target: None,
            player: None,
        },
        0,
        [0.0; 3],
        [0.0; 3],
    );
    for token_empty in [true, false] {
        for &(exists, is_player, is_self, cooperates) in &kinds {
            // 0: no manager. 1: a manager whose GUID the table does not hold. 2: the player.
            for state in 0..3u32 {
                if is_self && state != 2 {
                    continue; // the player's own object needs the player's object
                }
                for &pp in &player_positions {
                    for off in offsets {
                        let pos = if is_self {
                            pp
                        } else {
                            [pp[0] + off[0], pp[1] + off[1], pp[2] + off[2]]
                        };
                        let w = InspectWorld {
                            arg_ok: true,
                            token_empty,
                            target: exists.then_some(InspectTarget {
                                is_player,
                                is_self,
                                cooperates,
                                pos,
                            }),
                            player: (state == 2).then_some(pp),
                        };
                        run(&mut e, w, state, pos, pp);
                    }
                }
            }
        }
    }
    eprintln!("can_inspect: {cases} cases; usage/295/184/297/125/one = {tally:?}");
    assert!(tally.iter().all(|&n| n > 0), "every outcome must occur");
}
