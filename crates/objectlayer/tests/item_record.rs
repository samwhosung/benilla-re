//! `CGItem_C::ApplyRecord` (`0x5d8440`) against the exe, run whole
//! (`systems/objectlayer/items.md#the-item-record`, `#item-record-owner-signal`).
//!
//! The test compares what the function calls and the bytes it stores: every callee but the active
//! player's GUID getter (`0x468550`, run for real over a manager block the test writes) is
//! stubbed, logs its address and arguments, and returns a value the test chooses (the record, the
//! player object, the display id). The log is compared with `item_record::apply_record_calls` for
//! the world latch `0xb4b424` both ways, an owner that is the active player, differs in each
//! dword or is 0, an active GUID of 0 (no manager), a record that is missing or has each side of
//! `cmp [rec+0x60],1; jle`, and a player object that is missing or present. The five bytes the
//! getters return and the item's flags are read back. The display row is read only as absent
//! (`0x5da0b0` answering −1): the sound row at `0x5d84b6` to `0x5d84f3` is not run.

use benilla_re_difftest::{Emu, RegisterX86, StubIo, Wow};
use benilla_re_objectlayer::item_record::{
    apply_record_calls, Call, World, APPLY_EXPIRATIONS, BAG_REFRESH, CLEAR_LOCKS, DISPLAY_ID,
    GETTERS, GET_RECORD, OBJECT_PTR, REFRESH, SIGNAL_EVENT, SIGNAL_UNIT_EVENT, UNLOCK_ITEM,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const APPLY_RECORD: u64 = 0x5d8440;
/// The world latch byte, and the object manager pointer `0x468550` reads.
const LATCH: u64 = 0xb4b424;
const MGR_PTR: u64 = 0xb41414;

type Log = Rc<RefCell<Vec<Call>>>;

fn arg(io: &StubIo, i: u64) -> u32 {
    io.arg(i)
}

fn ecx(io: &StubIo) -> u32 {
    io.reg(RegisterX86::ECX) as u32
}

fn edx(io: &StubIo) -> u32 {
    io.reg(RegisterX86::EDX) as u32
}

/// Stubs `addr`, popping `pop` stack dwords, to log `args(io)` and return `ret()`.
fn logged(
    e: &mut Emu,
    log: &Log,
    addr: u32,
    pop: u32,
    args: fn(&StubIo) -> [u32; 4],
    ret: impl Fn() -> u32 + 'static,
) {
    let log = log.clone();
    e.stub_fn(u64::from(addr), pop, move |io| {
        log.borrow_mut().push(Call {
            addr,
            args: args(io),
        });
        ret()
    });
}

#[test]
fn apply_record_calls_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let log: Log = Rc::default();
    let record_cell = Rc::new(Cell::new(0u32));
    let player_cell = Rc::new(Cell::new(0u32));

    // The five getters answer distinct bytes, which the function stores at +0x318 to +0x31c.
    let getter_bytes = [0x11u32, 0x22, 0x33, 0x44, 0x55];
    for (&a, &b) in GETTERS.iter().zip(&getter_bytes) {
        logged(&mut e, &log, a, 0, |io| [ecx(io), 0, 0, 0], move || b);
    }
    // A display id of −1 is below the table, so the sound row is 0.
    logged(
        &mut e,
        &log,
        DISPLAY_ID,
        0,
        |io| [ecx(io), 0, 0, 0],
        || u32::MAX,
    );
    let rc = record_cell.clone();
    logged(
        &mut e,
        &log,
        GET_RECORD,
        5,
        |io| [ecx(io), arg(io, 0), 0, 0],
        move || rc.get(),
    );
    let pc = player_cell.clone();
    logged(
        &mut e,
        &log,
        OBJECT_PTR,
        3,
        |io| [ecx(io), arg(io, 0), arg(io, 1), arg(io, 2)],
        move || pc.get(),
    );
    logged(
        &mut e,
        &log,
        CLEAR_LOCKS,
        2,
        |io| [ecx(io), arg(io, 0), arg(io, 1), 0],
        || 0,
    );
    logged(&mut e, &log, SIGNAL_EVENT, 0, |io| [ecx(io), 0, 0, 0], || 0);
    logged(
        &mut e,
        &log,
        UNLOCK_ITEM,
        2,
        |io| [arg(io, 0), arg(io, 1), 0, 0],
        || 0,
    );
    logged(
        &mut e,
        &log,
        APPLY_EXPIRATIONS,
        1,
        |io| [ecx(io), arg(io, 0), 0, 0],
        || 0,
    );
    logged(&mut e, &log, REFRESH[0], 0, |io| [ecx(io), 0, 0, 0], || 0);
    logged(&mut e, &log, REFRESH[1], 0, |_| [0, 0, 0, 0], || 0);
    logged(&mut e, &log, REFRESH[2], 0, |io| [ecx(io), 0, 0, 0], || 0);
    logged(&mut e, &log, REFRESH[3], 0, |io| [ecx(io), 0, 0, 0], || 0);
    logged(
        &mut e,
        &log,
        BAG_REFRESH,
        2,
        |io| [arg(io, 0), arg(io, 1), 0, 0],
        || 0,
    );
    // SignalUnitEvent(&guid, event): the GUID is read from the pointer in ecx.
    logged(
        &mut e,
        &log,
        SIGNAL_UNIT_EVENT,
        0,
        |io| {
            let p = u64::from(ecx(io));
            [io.read_u32(p), io.read_u32(p + 4), edx(io), 0]
        },
        || 0,
    );

    let item = e.alloc(0x400) as u32;
    let desc = e.alloc(0x100) as u32;
    let record = e.alloc(0x100) as u32;
    let player = e.alloc(0x10) as u32;
    let mgr = e.alloc(0xe0);

    let item_guid = 0x0000_0007_0000_0101u64;
    let actives = [0x0000_0004_0000_0abcu64, 0];
    let mut cases = 0u32;
    let mut signals = 0u32;
    let mut active_legs = 0u32;
    for latch in [false, true] {
        for &active in &actives {
            let owners = [
                active,
                active ^ 1,
                active ^ (1 << 32),
                0xdead_beef_0000_0001,
                0,
            ];
            for owner in owners {
                for stack in [
                    None,
                    Some(0),
                    Some(1),
                    Some(2),
                    Some(i32::MAX),
                    Some(-1),
                    Some(i32::MIN),
                ] {
                    for player_obj in [None, Some(player)] {
                        let w = World {
                            latch,
                            item_guid,
                            owner,
                            active,
                            entry: 0x1234,
                            record_stack: stack,
                            player: player_obj,
                            item,
                        };
                        // The item's memory.
                        e.write_u32(u64::from(item) + 8, desc);
                        e.write_u32(u64::from(item) + 0x114, desc + 0x18);
                        e.write_u32(u64::from(item) + 0x314, 0x8000_0001);
                        e.write_u32(u64::from(item) + 0x340, 0xdead_beef);
                        e.write(u64::from(desc), &item_guid.to_le_bytes());
                        e.write_u32(u64::from(desc) + 0xc, w.entry);
                        e.write(u64::from(desc) + 0x18, &owner.to_le_bytes());
                        // The manager, the latch, the record and the player object.
                        if active == 0 {
                            e.write_u32(MGR_PTR, 0);
                        } else {
                            e.write(mgr + 0xc0, &active.to_le_bytes());
                            e.write_u32(MGR_PTR, mgr as u32);
                        }
                        e.write(LATCH, &[u8::from(latch)]);
                        if let Some(s) = stack {
                            e.write_u32(u64::from(record) + 0x60, s as u32);
                            record_cell.set(record);
                        } else {
                            record_cell.set(0);
                        }
                        player_cell.set(player_obj.unwrap_or(0));

                        log.borrow_mut().clear();
                        e.call(APPLY_RECORD, item, 0, &[]);
                        let want = apply_record_calls(&w);
                        let got = log.borrow().clone();
                        assert_eq!(
                            got, want,
                            "latch {latch} active {active:#x} owner {owner:#x} record {stack:?} \
                             player {player_obj:?}"
                        );
                        // Bit 1 is set and the rest of the flags kept; the five bytes are the
                        // getters' answers; the sound row is 0.
                        assert_eq!(e.read_u32(u64::from(item) + 0x314), 0x8000_0003);
                        let bytes = e.read(u64::from(item) + 0x318, 5);
                        assert_eq!(bytes, [0x11, 0x22, 0x33, 0x44, 0x55], "cached bytes");
                        assert_eq!(e.read_u32(u64::from(item) + 0x340), 0, "sound row");
                        cases += 1;
                        signals += u32::from(got.iter().any(|c| c.addr == SIGNAL_UNIT_EVENT));
                        active_legs += u32::from(got.iter().any(|c| c.addr == OBJECT_PTR));
                    }
                }
            }
        }
    }
    eprintln!(
        "apply_record: {cases} cases, {signals} signal the owner, {active_legs} run the active leg"
    );
    assert!(signals > 0 && signals < cases && active_legs > 0 && active_legs < signals);
}
