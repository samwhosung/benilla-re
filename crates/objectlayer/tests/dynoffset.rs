//! `FieldCopyOffset` (`0x466830`) against the exe, run whole: every field index of each block id
//! from 0 to past its table's last value, so the table scan reaches its default, the player block
//! with the active-player flag both ways, and a check that the dead first stack argument changes
//! nothing.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::dynoffset::field_dynamic_offset;

const DYNOFF: u64 = 0x466830;

/// One call of `0x466830`: `ecx` is the block, `edx` the field index, the stack the two arguments.
fn bin(e: &mut Emu, block: u32, f: u32, arg3: u32, arg4: u32) -> u32 {
    e.call(DYNOFF, block, f, &[arg3, arg4])
}

#[test]
fn field_dynamic_offset_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // Each block's sweep runs past its table's last value into the default, up to block 4's
    // 0x14a0 for the widest table, `PLAYER_B` (largest value 1094).
    let ceilings: [(u32, u32); 9] = [
        (0, 0x40),   // the default table, largest value 5
        (1, 0xd0),   // item, largest value 41
        (2, 0x200),  // container, largest value 73, and the item fall-through
        (3, 0x300),  // unit, largest value 182
        (4, 0x14a0), // player, both tables, and the unit fall-through
        (5, 0x80),   // block 5, largest value 20
        (6, 0x40),   // block 6, the constant 10 and no table
        (7, 0xb0),   // corpse, largest value 32
        (8, 0x40),   // past 7, the default
    ];

    for (block, ceil) in ceilings {
        // Only the player branch reads the active-player flag, so only it sweeps both values.
        let selfs: &[u32] = if block == 4 { &[0, 1] } else { &[0] };
        for &arg4 in selfs {
            for f in 0..=ceil {
                let got = bin(&mut e, block, f, 0, arg4);
                let ours = field_dynamic_offset(block, f, arg4 != 0);
                assert_eq!(
                    got, ours,
                    "block {block} field {f:#x} self {arg4}: bin {got:#x} ours {ours:#x}"
                );
            }
        }
    }

    // The first stack argument only reaches the getters' unused `edx`: the result does not
    // depend on it.
    for &(block, f) in &[(4u32, 0x300u32), (4, 0x800), (2, 0x100), (1, 0x40)] {
        let ours = field_dynamic_offset(block, f, true);
        for &arg3 in &[0u32, 0xdead_beef, 7, 0xffff_ffff] {
            assert_eq!(
                bin(&mut e, block, f, arg3, 1),
                ours,
                "arg3 {arg3:#x} must not affect block {block} field {f:#x}"
            );
        }
    }
}
