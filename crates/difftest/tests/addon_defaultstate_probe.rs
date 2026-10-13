//! The `UIADDON` constructor (`0x520550`) sets `[rec+0x2b]`, the `.toc` `DefaultState` byte, to 1
//! (`0x5205b9 mov [esi+0x2b],al` with `al = 1` from `0x5205b1`), while its neighbours `+0x18`,
//! `+0x28`, `+0x29`, `+0x2a`, `+0x2c` and `+0x2d` are written from `bl = 0`. So an addon without a
//! `## DefaultState:` line is enabled.
//!
//! The record is filled with `0xFF` before the run, so the result tells the readings apart: 1 is
//! a store of 1, 0 a store of 0, and `0xFF` no store. On a zeroed record the first two would look
//! alike, and the allocation does zero it (`0x52055c or eax,0x8`, then `rep stos` after
//! `0x6462fc test byte [ebp+0x14],0x8`).
//!
//! The run is the constructor's initialiser, `0x52057f` to `0x520606`, with `esi` = the record
//! and `ebx = 0` (`0x520575 xor ebx,ebx`): everything between the allocation's test and the list
//! link, including the sub-object constructors it calls (`0x520670`, `0x520680` twice, `0x520690`,
//! `0x5206a0`). Nothing is stubbed.

use benilla_re_difftest::{Emu, RegisterX86, Wow};

/// `UIADDON` (allocation tag `.?AUUIADDON@@` at `0x853afc`), its size from `0x520567 add ecx,0x1d4`.
const REC_SIZE: u64 = 0x1d4;

/// The initialiser: from the first store after the allocation test (`0x520579 je 0x52061b`) to the
/// list link, which reads the caller's list header and is not run.
const INIT_BEGIN: u64 = 0x52057f;
const INIT_END: u64 = 0x520606;

/// The byte under test, and its neighbours written the same way from `bl`.
const F_DEFAULT_STATE: u64 = 0x2b;
const F_ZEROED_NEIGHBOURS: [u64; 6] = [0x18, 0x28, 0x29, 0x2a, 0x2c, 0x2d];

/// `0x5205b6 mov [esi+0x24],eax` stores the same `eax = 1`.
const F_SHARES_THE_ONE: u64 = 0x24;

const POISON: u8 = 0xFF;

/// The run's entry state: `esi` = the record, `ebx` = 0, and a stack for the sub-object
/// constructors it calls (`Emu::run` leaves `esp` to the test).
fn enter_slice(emu: &mut Emu, rec: u64) {
    let stack = emu.alloc(0x400);
    emu.set_reg(RegisterX86::ESP, stack + 0x300);
    emu.set_reg(RegisterX86::ESI, rec);
    emu.set_reg(RegisterX86::EBX, 0);
}

/// Runs the initialiser over a record filled with `0xFF` and returns its bytes.
fn construct_over_poison(emu: &mut Emu) -> Vec<u8> {
    let rec = emu.alloc(REC_SIZE + 0x10);
    emu.write(rec, &vec![POISON; (REC_SIZE + 0x10) as usize]);
    enter_slice(emu, rec);
    emu.run(INIT_BEGIN, INIT_END);
    emu.read(rec, REC_SIZE as usize)
}

/// The constructor sets `+0x2b = 1`: an addon without `## DefaultState:` is enabled.
#[test]
fn ctor_seeds_defaultstate_enabled() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rec = construct_over_poison(&mut emu);

    assert_eq!(
        rec[F_DEFAULT_STATE as usize], 1,
        "[UIADDON+0x2b] after the initialiser: expected 1, stored at 0x5205b9 from al=1; 0 would be \
         a store of 0, 0xFF no store"
    );
}

/// The neighbouring bytes are zeroed, so `+0x2b` is the one byte set to 1 among them.
#[test]
fn only_defaultstate_is_seeded_true_in_that_run() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rec = construct_over_poison(&mut emu);

    for off in F_ZEROED_NEIGHBOURS {
        assert_eq!(
            rec[off as usize], 0,
            "[UIADDON+{off:#x}] should be 0 (written from bl); it is {:#04x}",
            rec[off as usize]
        );
    }
    assert_eq!(rec[F_DEFAULT_STATE as usize], 1);
    assert_eq!(
        u32::from_le_bytes(
            rec[F_SHARES_THE_ONE as usize..F_SHARES_THE_ONE as usize + 4]
                .try_into()
                .unwrap()
        ),
        1,
        "[UIADDON+0x24] carries the same eax=1 as +0x2b (0x5205b6 / 0x5205b9)"
    );
}

/// The control: before the run the record reads `0xFF` at the offsets the other tests check, so the
/// run is what writes them.
#[test]
fn the_poison_is_what_the_slice_overwrites() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let rec = emu.alloc(REC_SIZE + 0x10);
    emu.write(rec, &vec![POISON; (REC_SIZE + 0x10) as usize]);
    let before = emu.read(rec, REC_SIZE as usize);
    assert_eq!(before[F_DEFAULT_STATE as usize], POISON);
    assert_eq!(before[F_ZEROED_NEIGHBOURS[0] as usize], POISON);

    enter_slice(&mut emu, rec);
    emu.run(INIT_BEGIN, INIT_END);
    let after = emu.read(rec, REC_SIZE as usize);

    assert_ne!(
        after[F_DEFAULT_STATE as usize], POISON,
        "the slice wrote it"
    );
    assert_eq!(after[F_DEFAULT_STATE as usize], 1);
    // A byte the initialiser does not write keeps its `0xFF`: it sets members, not the whole record.
    assert_eq!(
        after[0x1d0], POISON,
        "the ctor's init list does not blanket-clear the record"
    );
}
