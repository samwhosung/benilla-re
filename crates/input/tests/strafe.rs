//! `0x514e80` against [`emit_strafe_axis`]: every combination of the six bits it reads and the
//! latch, under random other bits, with the stand state zero and not, for a player and not.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_input::{emit_strafe_axis, StrafeCall};

const EMIT_STRAFE_AXIS: u64 = 0x514e80;
const STRAFE_START: u64 = 0x60dcb0;
const STRAFE_STOP: u64 = 0x60dce0;
const STAND_UP: u64 = 0x60be30;
const CLEAR_AFK: u64 = 0x5eb830;

/// The whole function with its four callees and the unit's stand-state slot stubbed: the test
/// compares the word it leaves and the calls it makes, with their arguments, in order.
#[test]
fn emit_strafe_axis_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(STRAFE_START, 0, 2);
    e.stub(STRAFE_STOP, 0, 1);
    e.stub(STAND_UP, 0, 1);
    e.stub(CLEAR_AFK, 0, 1);
    let control = e.alloc(0x1c);
    let unit = e.alloc(0x20);
    let descriptors = e.alloc(0x20);
    let vtables = e.alloc(0x200);
    let slots = [e.alloc(0x10), e.alloc(0x10)];
    let stand = [0u32, 7];
    for (k, &slot) in slots.iter().enumerate() {
        e.stub(slot, stand[k], 0);
        e.write_u32(vtables + 0x100 * k as u64 + 0xa4, slot as u32);
    }
    e.write_u32(unit + 8, descriptors as u32);
    let mut s: u32 = 0x5e4a_0001;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    const READ: [u32; 7] = [0x1, 0x40, 0x80, 0x100, 0x200, 0x20000, 0x2];
    for case in 0..(1u32 << 7) * 16 {
        let mut word = next() & !READ.iter().fold(0, |a, b| a | b);
        if case % 4 == 0 {
            word = 0;
        }
        for (i, bit) in READ.iter().enumerate() {
            if (case >> 4) & (1 << i) != 0 {
                word |= bit;
            }
        }
        let k = (case & 1) as usize;
        let player = case & 2 != 0;
        let time = next();
        e.write_u32(control + 4, word);
        e.write_u32(unit, (vtables + 0x100 * k as u64) as u32);
        let type_word = (next() & !0x10) | if player { 0x10 } else { 0 };
        e.write_u32(descriptors + 8, type_word);
        e.clear_calls();
        e.call(EMIT_STRAFE_AXIS, control as u32, 0, &[time, unit as u32]);
        let got: Vec<StrafeCall> = e
            .calls()
            .iter()
            .map(|c| match c.addr {
                STRAFE_STOP => StrafeCall::Stop { time: c.args[0] },
                STAND_UP => {
                    assert_eq!(c.args[0], 0, "stand state 0");
                    StrafeCall::StandUp
                }
                CLEAR_AFK => {
                    assert_eq!(c.args[0], 0, "not forced");
                    StrafeCall::ClearAfk
                }
                STRAFE_START => StrafeCall::Start {
                    time: c.args[0],
                    left: match c.args[1] {
                        1 => true,
                        0 => false,
                        v => panic!("strafe direction {v}"),
                    },
                },
                a if a == slots[k] => StrafeCall::ReadStandState,
                a => panic!("unexpected call {a:#x}"),
            })
            .collect();
        let (want_word, want) = emit_strafe_axis(word, time, stand[k], player);
        assert_eq!(got, want, "case {case} word {word:#x}");
        assert_eq!(e.read_u32(control + 4), want_word, "word {case} {word:#x}");
    }
}
