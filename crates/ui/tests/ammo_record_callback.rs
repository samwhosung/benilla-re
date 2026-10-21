//! The record callback `0x4c84f0` of the ammo legs of `GetInventoryItemTexture` and
//! `GetInventoryItemQuality` (`systems/ui/inventoryverbs.md#ammo-record-callback`), run whole
//! against the exe with `FrameScript_SignalEvent2` (`0x703f50`) stubbed.
//!
//! The item cache calls it with the key in `ecx`, a pointer to the owner id in `edx`, the data and
//! the status dword on the stack, and the function reads one byte of the status. The test passes
//! status dwords whose low byte is zero and nonzero and compares the event id, the format and the
//! token the stub sees with `inventory_verbs::ammo_record_signal`.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::inventory_verbs::ammo_record_signal;
use std::cell::RefCell;
use std::rc::Rc;

const AMMO_CALLBACK: u64 = 0x4c84f0;
const SIGNAL_EVENT2: u64 = 0x703f50;

#[test]
fn ammo_record_callback_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let seen: Rc<RefCell<Vec<(u32, String, String)>>> = Rc::default();
    let s = seen.clone();
    e.stub_fn(SIGNAL_EVENT2, 0, move |io| {
        let text = |p: u32| String::from_utf8(io.read_cstr(u64::from(p))).expect("ascii");
        s.borrow_mut()
            .push((io.arg(0), text(io.arg(1)), text(io.arg(2))));
        0
    });
    // ecx = the key, edx = the owner pointer, then the data and the status dword.
    let mut fired = 0;
    for status in [
        0u32,
        1,
        2,
        0x7f,
        0x80,
        0xff,
        0x100,
        0x1_0000,
        0xffff_ff00,
        0xffff_ff01,
        0xffff_ffff,
    ] {
        seen.borrow_mut().clear();
        e.call(AMMO_CALLBACK, 0x1234, 0x5678, &[0xdead_beef, status]);
        let got = seen.borrow().clone();
        let want: Vec<_> = ammo_record_signal(status)
            .map(|(id, f, a)| (id, f.to_string(), a.to_string()))
            .into_iter()
            .collect();
        assert_eq!(got, want, "status {status:#x}");
        fired += got.len();
    }
    // The key and the owner pointer are not read: other values give the same signal.
    seen.borrow_mut().clear();
    e.call(AMMO_CALLBACK, 0, 0, &[0, 1]);
    assert_eq!(seen.borrow().len(), 1);
    assert!(fired > 0 && fired < 11);
}
