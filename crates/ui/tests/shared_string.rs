//! `0x41e3a0`, which points a shared path string at a new text, run whole in the emulator against
//! `assign_shared_string`.
//!
//! `SStrLen` (`0x64a6f0`) runs as the exe has it. The pool getter `0x41ddd0` and the intern
//! `0x41e030` are stubs that return the case's values, the intern checking that it is asked on the
//! pool for the text; `SErrSetLastError` (`0x64e850`) is a stub that logs its code. Each record is a
//! scratch block whose vtable's slot 2 is a stub that logs the record it releases and the counts it
//! sees then, so the order of the count and the release shows. The null record `0x883e78` gets the
//! same vtable. The cases cross old records (none, another, the null record, the one interned), the
//! texts (null, empty, one byte, 0x3ff, 0x400 and 0x500 bytes), a pool or none, and an interned
//! record or none.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::shared_string::{assign_shared_string, Assign, NULL_RECORD};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const ASSIGN: u64 = 0x41e3a0;
const POOL_GETTER: u64 = 0x41ddd0;
const INTERN: u64 = 0x41e030;
const SERR: u64 = 0x64e850;

/// Each release: the record released and the three records' counts then.
type Releases = Rc<RefCell<Vec<(u32, [u32; 3])>>>;

#[test]
fn assign_shared_string_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let release_stub = emu.alloc(0x10);
    let vtable = emu.alloc(0x10);
    emu.write_u32(vtable + 8, release_stub as u32);
    let old_rec = emu.alloc(0x20) as u32;
    let new_rec = emu.alloc(0x20) as u32;
    let handle = emu.alloc(0x10);
    let text_buf = emu.alloc(0x600);
    let pool_obj = 0x5150_0000u32;
    let records = [NULL_RECORD, old_rec, new_rec];

    let pool = Rc::new(Cell::new(0u32));
    let interned = Rc::new(Cell::new(0u32));
    let releases: Releases = Rc::default();
    let interns: Rc<RefCell<Vec<(u32, u32)>>> = Rc::default();
    let pool_asks = Rc::new(Cell::new(0u32));
    {
        let (pool, pool_asks) = (pool.clone(), pool_asks.clone());
        emu.stub_fn(POOL_GETTER, 0, move |_| {
            pool_asks.set(pool_asks.get() + 1);
            pool.get()
        });
    }
    {
        let (interned, interns) = (interned.clone(), interns.clone());
        emu.stub_fn(INTERN, 1, move |io| {
            interns
                .borrow_mut()
                .push((io.reg(RegisterX86::ECX) as u32, io.arg(0)));
            interned.get()
        });
    }
    {
        let releases = releases.clone();
        emu.stub_fn(release_stub, 0, move |io| {
            let counts = records.map(|r| io.read_u32(u64::from(r) + 4));
            releases
                .borrow_mut()
                .push((io.reg(RegisterX86::ECX) as u32, counts));
            0
        });
    }
    emu.stub(SERR, 0, 1);

    let texts: Vec<Option<Vec<u8>>> = vec![
        None,
        Some(Vec::new()),
        Some(b"a".to_vec()),
        Some(b"Fonts\\FRIZQT__.TTF".to_vec()),
        Some(vec![b'a'; 0x3ff]),
        Some(vec![b'a'; 0x400]),
        Some(vec![b'a'; 0x500]),
    ];
    let mut cases = 0;
    for old in [0, old_rec, NULL_RECORD, new_rec] {
        for text in &texts {
            for p in [0, pool_obj] {
                for i in [0, new_rec] {
                    for (k, &r) in records.iter().enumerate() {
                        emu.write_u32(u64::from(r), vtable as u32);
                        emu.write_u32(u64::from(r) + 4, 10 + k as u32);
                    }
                    emu.write_u32(handle + 8, old);
                    let ptr = match text {
                        Some(t) => {
                            let mut z = t.clone();
                            z.push(0);
                            emu.write(text_buf, &z);
                            text_buf as u32
                        }
                        None => 0,
                    };
                    pool.set(p);
                    interned.set(i);
                    pool_asks.set(0);
                    releases.borrow_mut().clear();
                    interns.borrow_mut().clear();
                    emu.clear_calls();
                    emu.call(ASSIGN, handle as u32, 0, &[ptr]);

                    let after = records.map(|r| emu.read_u32(u64::from(r) + 4));
                    let raised: Vec<usize> =
                        (0..3).filter(|&k| after[k] != 10 + k as u32).collect();
                    assert!(raised.len() <= 1, "one count changes: {after:?}");
                    let add_ref = raised.first().map_or(0, |&k| {
                        assert_eq!(after[k], 11 + k as u32, "a count goes up by one");
                        records[k]
                    });
                    let rel = releases.borrow();
                    assert!(rel.len() <= 1, "one release");
                    if let (Some(&(_, seen)), Some(&k)) = (rel.first(), raised.first()) {
                        assert_eq!(seen[k], after[k], "the count rises before the release");
                    }
                    let ints = interns.borrow();
                    assert!(ints.len() <= 1);
                    if let Some(&(ecx, arg)) = ints.first() {
                        assert_eq!((ecx, arg), (p, ptr), "the intern is asked on the pool");
                    }
                    let errors: Vec<u32> = emu
                        .calls()
                        .iter()
                        .filter(|c| c.addr == SERR)
                        .map(|c| c.args[0])
                        .collect();
                    assert!(errors.len() <= 1);
                    let got = Assign {
                        asked_pool: pool_asks.get() == 1,
                        asked_intern: !ints.is_empty(),
                        add_ref,
                        release: rel.first().map_or(0, |&(ecx, _)| ecx),
                        error: errors.first().copied().unwrap_or(0),
                        slot: emu.read_u32(handle + 8),
                    };
                    let want = assign_shared_string(old, text.as_deref(), p, i);
                    assert_eq!(
                        got,
                        want,
                        "old {old:#x}, text {:?}, pool {p:#x}, interned {i:#x}",
                        text.as_ref().map(Vec::len)
                    );
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 4 * 7 * 2 * 2);
}
