//! `CGPlayer_C::SetAutoLoot` (`0x5df460`) against the exe, run whole
//! (`systems/objectlayer/interaction.md#the-auto-loot-byte`). The sweep it calls is stubbed and
//! logged; the byte it stores is read back.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::auto_loot_byte::{set_auto_loot, SWEEP};
use std::cell::RefCell;
use std::rc::Rc;

const SET_AUTO_LOOT: u64 = 0x5df460;
const BYTE: u64 = 0x1d30;
const TARGET: u64 = 0x1d28;

#[test]
fn set_auto_loot_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let sweeps: Rc<RefCell<Vec<u32>>> = Rc::default();
    let s2 = sweeps.clone();
    e.stub_fn(u64::from(SWEEP), 0, move |io| {
        s2.borrow_mut().push(io.reg(RegisterX86::ECX) as u32);
        0
    });
    let player = e.alloc(0x2000);
    let mut cases = 0u32;
    let mut swept = 0u32;
    // The argument is read as a byte: 0x100 has a zero low byte.
    for arg in [0u32, 1, 2, 0x80, 0xff, 0x100, 0x101, 0xffff_ff00] {
        for target in [0u64, 1, 1 << 32, 0xf130_0000_0000_5eed, u64::MAX] {
            for before in [0u8, 1, 0x7f] {
                e.write(player + TARGET, &target.to_le_bytes());
                e.write(player + BYTE, &[before]);
                sweeps.borrow_mut().clear();
                e.call(SET_AUTO_LOOT, player as u32, 0, &[arg]);
                let (byte, sweep) = set_auto_loot(arg, target);
                assert_eq!(
                    e.read(player + BYTE, 1)[0],
                    byte,
                    "arg {arg:#x} target {target:#x}: byte"
                );
                let want: Vec<u32> = if sweep { vec![0] } else { vec![] };
                assert_eq!(
                    *sweeps.borrow(),
                    want,
                    "arg {arg:#x} target {target:#x}: sweep"
                );
                // The target is untouched.
                assert_eq!(e.read(player + TARGET, 8), target.to_le_bytes());
                cases += 1;
                swept += u32::from(sweep);
            }
        }
    }
    eprintln!("set_auto_loot: {cases} cases, {swept} sweep");
    assert!(swept > 0 && swept < cases);
}
