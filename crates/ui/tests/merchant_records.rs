//! The vendor window's record wait against the exe, both functions run whole
//! (`systems/ui/merchant.md#vendor-row-records-and-stock`): `0x4faed0` with the item cache's
//! `GetRecord` (`0x55ba30`) stubbed to answer a record or a miss, and `0x4fb0f0` with
//! `FrameScript_SignalEvent` (`0x703e50`), its tail call, stubbed to log the event.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::merchant_records::{ask_row_record, count_record_answer, MERCHANT_UPDATE};
use std::cell::RefCell;
use std::rc::Rc;

const ASK: u64 = 0x4faed0;
const ANSWER: u64 = 0x4fb0f0;
const GET_RECORD: u64 = 0x55ba30;
const SIGNAL_EVENT: u64 = 0x703e50;
const ITEM_CACHE: u32 = 0xc0e2a0;
const CALLBACK: u32 = 0x4fada0;
const VENDOR_GUID: u64 = 0xbddfa0;
const PENDING: u64 = 0xbddfb0;

fn set_guid(e: &mut Emu, guid: u64) {
    e.write_u32(VENDOR_GUID, guid as u32);
    e.write_u32(VENDOR_GUID + 4, (guid >> 32) as u32);
}

#[test]
fn merchant_row_record_ask_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // What GetRecord saw: this, entry, the GUID behind the owner pointer, callback, data, dedupe.
    let seen: Rc<RefCell<Vec<[u32; 7]>>> = Rc::default();
    let answer: Rc<RefCell<u32>> = Rc::default();
    {
        let seen = seen.clone();
        let answer = answer.clone();
        e.stub_fn(GET_RECORD, 5, move |io| {
            let owner = u64::from(io.arg(1));
            seen.borrow_mut().push([
                io.reg(RegisterX86::ECX) as u32,
                io.arg(0),
                io.read_u32(owner),
                io.read_u32(owner + 4),
                io.arg(2),
                io.arg(3),
                io.arg(4),
            ]);
            *answer.borrow()
        });
    }
    let guids = [0u64, 0xf130_0000_0000_1234, 0x0000_0001_ffff_ffff];
    let entries = [0u32, 1, 0x4a2b, 0xffff_ffff];
    let records = [0u32, 0x0123_4560];
    let pendings = [0u32, 1, 7, 0xffff_ffff];
    for &guid in &guids {
        for &entry in &entries {
            for &record in &records {
                for &pending in &pendings {
                    set_guid(&mut e, guid);
                    e.write_u32(PENDING, pending);
                    *answer.borrow_mut() = record;
                    seen.borrow_mut().clear();
                    let got = e.call(ASK, entry, 0, &[]);
                    let (asks, want_pending, want) = ask_row_record(entry, record, pending);
                    let what = format!(
                        "guid {guid:#x} entry {entry:#x} record {record:#x} pending {pending}"
                    );
                    assert_eq!(got, want, "{what}: return");
                    assert_eq!(e.read_u32(PENDING), want_pending, "{what}: count");
                    let seen = seen.borrow();
                    if asks {
                        assert_eq!(
                            seen.as_slice(),
                            &[[
                                ITEM_CACHE,
                                entry,
                                guid as u32,
                                (guid >> 32) as u32,
                                CALLBACK,
                                0,
                                0
                            ]],
                            "{what}: the ask"
                        );
                    } else {
                        assert!(seen.is_empty(), "{what}: no ask");
                    }
                }
            }
        }
    }
}

#[test]
fn merchant_record_answer_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let events: Rc<RefCell<Vec<u32>>> = Rc::default();
    {
        let events = events.clone();
        e.stub_fn(SIGNAL_EVENT, 0, move |io| {
            events.borrow_mut().push(io.reg(RegisterX86::ECX) as u32);
            0
        });
    }
    let guids = [
        0u64,
        1,
        0x1_0000_0000,
        0xf130_0000_0000_1234,
        0xffff_ffff_ffff_ffff,
    ];
    let pendings = [0u32, 1, 2, 3, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff];
    for &guid in &guids {
        for &pending in &pendings {
            set_guid(&mut e, guid);
            e.write_u32(PENDING, pending);
            events.borrow_mut().clear();
            e.call(ANSWER, 0, 0, &[]);
            let (want_pending, signals) = count_record_answer(pending, guid);
            let what = format!("guid {guid:#x} pending {pending:#x}");
            assert_eq!(e.read_u32(PENDING), want_pending, "{what}: count");
            let want: &[u32] = if signals { &[MERCHANT_UPDATE] } else { &[] };
            assert_eq!(events.borrow().as_slice(), want, "{what}: signal");
        }
    }
}
