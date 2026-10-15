//! The tick `0x6ca190`, the packet handler `0x6ca5c0` and the handover services `0x6ca900` and
//! `0x6ca960` run whole, with the module's slots as machine code that logs its arguments, the
//! lock and the clock replaced, and the allocator a bump stub. The globals are the exe's own.

mod common;

use benilla_re_difftest::{Emu, Wow};
use common::Rng;

const TICK: u64 = 0x6ca190;
const HANDLER: u64 = 0x6ca5c0;
const STASH: u64 = 0x6ca900;
const TAKE: u64 = 0x6ca960;
const ENTER: u64 = 0x6579b0;
const LEAVE: u64 = 0x6579c0;
const CLOCK: u64 = 0x42c010;
const S_MEM_ALLOC: u64 = 0x6462e0;
const S_MEM_FREE: u64 = 0x646430;

const PENDING: u64 = 0xce8974;
const ACTIVE: u64 = 0xce8978;
const INTERFACE: u64 = 0xce897c;
const STASH_DATA: u64 = 0xce8980;
const STASH_SIZE: u64 = 0xce8984;
const LAST_TICK: u64 = 0xce8988;

struct Rig {
    emu: Emu<'static>,
    /// The module's tick and packet slots log here: a count, then the arguments.
    log: u64,
    iface: u64,
    clock: u64,
}

/// `mov ecx,[esp+4]; mov [log+4],ecx; ...; inc [log]; ret 4*n` for `n` arguments.
fn logger(log: u64, n: u32) -> Vec<u8> {
    let mut c = Vec::new();
    for i in 0..n {
        c.extend_from_slice(&[0x8b, 0x4c, 0x24, 4 + 4 * i as u8, 0x89, 0x0d]);
        c.extend_from_slice(&((log + 4 + 4 * u64::from(i)) as u32).to_le_bytes());
    }
    c.extend_from_slice(&[0xff, 0x05]);
    c.extend_from_slice(&(log as u32).to_le_bytes());
    c.extend_from_slice(&[0xc2, 4 * n as u8, 0]);
    c
}

fn rig(wow: &Wow) -> Rig {
    let mut emu = Emu::new(wow);
    // The lock is not under test; the clock reads a word the test sets.
    emu.stub(ENTER, 0, 0);
    emu.stub(LEAVE, 0, 0);
    let clock = emu.alloc(8);
    let mut code = vec![0xa1];
    code.extend_from_slice(&(clock as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(CLOCK, &code);
    let log = emu.alloc(0x40);
    // The module's object: a table whose slots 2 and 3 log.
    let slot2 = emu.alloc(0x40);
    emu.write(slot2, &logger(log, 3));
    let slot3 = emu.alloc(0x40);
    emu.write(slot3, &logger(log + 0x20, 1));
    let table = emu.alloc(0x10);
    emu.write_u32(table + 8, slot2 as u32);
    emu.write_u32(table + 0xc, slot3 as u32);
    let iface = emu.alloc(8);
    emu.write_u32(iface, table as u32);
    let arena = emu.alloc(0x10_0000);
    emu.stub_alloc_sized(S_MEM_ALLOC, arena, arena + 0x10_0000, 0, 4);
    emu.stub(S_MEM_FREE, 1, 4);
    Rig {
        emu,
        log,
        iface,
        clock,
    }
}

impl Rig {
    fn set_active(&mut self, active: bool) {
        self.emu.write_u32(PENDING, 0);
        self.emu.write_u32(ACTIVE, u32::from(active));
        self.emu
            .write_u32(INTERFACE, if active { self.iface as u32 } else { 0 });
        self.emu.write(self.log, &[0u8; 0x40]);
    }
}

#[test]
fn tick_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = rig(&wow);
    let mut rng = Rng(0x6ca1_9000);
    let mut cases = vec![
        (0u32, 0u32),
        (0, 1),
        (1000, 1016),
        (u32::MAX, 5),
        (5, u32::MAX),
        (0x8000_0000, 0x7fff_ffff),
        (7, 7),
    ];
    for _ in 0..200 {
        cases.push((rng.next(), rng.next()));
    }
    for (case, &(last, now)) in cases.iter().enumerate() {
        for active in [true, false] {
            r.set_active(active);
            r.emu.write_u32(LAST_TICK, last);
            r.emu.write_u32(r.clock, now);
            r.emu.call(TICK, 0, 0, &[]);
            assert_eq!(r.emu.read_u32(LAST_TICK), now, "case {case}: stamp");
            let calls = r.emu.read_u32(r.log + 0x20);
            if active {
                assert_eq!(calls, 1, "case {case}: one tick call");
                assert_eq!(
                    r.emu.read_u32(r.log + 0x24),
                    now.wrapping_sub(last),
                    "case {case}: milliseconds"
                );
            } else {
                assert_eq!(calls, 0, "case {case}: no module, no call");
            }
        }
    }
}

fn store(emu: &mut Emu, bytes: &[u8], cursor: u32) -> (u64, u64) {
    let buf = emu.alloc(bytes.len() as u64 + 16);
    emu.write(buf, bytes);
    let at = emu.alloc(0x20);
    emu.write(at, &[0u8; 0x20]);
    emu.write_u32(at + 4, buf as u32);
    emu.write_u32(at + 0xc, bytes.len() as u32);
    emu.write_u32(at + 0x10, bytes.len() as u32);
    emu.write_u32(at + 0x14, cursor);
    (at, buf)
}

#[test]
fn handler_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = rig(&wow);
    let mut rng = Rng(0x6ca5_c000);
    for case in 0..120 {
        let n = if case == 0 {
            0
        } else {
            rng.below(500) as usize
        };
        let bytes: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        let cursor = if n == 0 { 0 } else { rng.below(n as u32 + 1) };
        for active in [true, false] {
            r.set_active(active);
            let (st, buf) = store(&mut r.emu, &bytes, cursor);
            let eax = r.emu.call(HANDLER, 0, 0x2e6, &[0x1234, st as u32]);
            assert_eq!(eax, 1, "case {case}: handled");
            assert_eq!(
                r.emu.read_u32(st + 0x14),
                n as u32,
                "case {case}: the cursor is past the data"
            );
            if active {
                assert_eq!(r.emu.read_u32(r.log), 1, "case {case}: one packet call");
                assert_eq!(
                    r.emu.read_u32(r.log + 4),
                    (buf + u64::from(cursor)) as u32,
                    "case {case}: bytes"
                );
                assert_eq!(
                    r.emu.read_u32(r.log + 8),
                    n as u32 - cursor,
                    "case {case}: length"
                );
                let out = u64::from(r.emu.read_u32(r.log + 12));
                assert_eq!(r.emu.read_u32(out), 0, "case {case}: result slot");
            } else {
                assert_eq!(r.emu.read_u32(r.log), 0, "case {case}: no module");
            }
        }
    }
    // Any other opcode returns 0 and touches nothing.
    r.set_active(true);
    let (st, _) = store(&mut r.emu, &[1, 2, 3], 1);
    for opcode in [0u32, 0x2e5, 0x2e7, 0x1ee, 0xffff_ffff] {
        assert_eq!(r.emu.call(HANDLER, 0, opcode, &[0, st as u32]), 0);
        assert_eq!(r.emu.read_u32(st + 0x14), 1);
        assert_eq!(r.emu.read_u32(r.log), 0);
    }
}

#[test]
fn handover_buffer() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = rig(&wow);
    let mut rng = Rng(0x6ca9_0000);
    r.emu.write_u32(STASH_DATA, 0);
    r.emu.write_u32(STASH_SIZE, 0);
    let dest = r.emu.alloc(0x400);
    let len_at = r.emu.alloc(8);
    // Nothing held: the take returns 0 and writes nothing.
    r.emu.write_u32(len_at, 9);
    assert_eq!(
        r.emu.call(TAKE, 0, 0, &[dest as u32, len_at as u32]) & 0xff,
        0
    );
    assert_eq!(r.emu.read_u32(len_at), 9);

    for case in 0..60 {
        let n = 1 + rng.below(600) as usize;
        let data: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        let src = r.emu.alloc(n as u64 + 16);
        r.emu.write(src, &data);
        r.emu.clear_calls();
        r.emu.call(STASH, 0, 0, &[src as u32, n as u32]);
        assert_eq!(r.emu.read_u32(STASH_SIZE), n as u32, "case {case}");
        let held = u64::from(r.emu.read_u32(STASH_DATA));
        assert_ne!(held, u64::from(src as u32));
        assert_eq!(r.emu.read(held, n), data, "case {case}");

        // A request at most as long, exactly as long, and longer than the buffer.
        let ask = [n as u32 / 2, n as u32, n as u32 + 50][case % 3];
        r.emu.write(dest, &[0xee; 0x400]);
        r.emu.write_u32(len_at, ask);
        let ok = r.emu.call(TAKE, 0, 0, &[dest as u32, len_at as u32]) & 0xff;
        assert_eq!(ok, 1, "case {case}");
        let got = ask.min(n as u32);
        assert_eq!(r.emu.read_u32(len_at), got, "case {case}: count");
        assert_eq!(
            r.emu.read(dest, got as usize),
            data[..got as usize],
            "case {case}"
        );
        assert_eq!(
            r.emu.read(dest + u64::from(got), 1)[0],
            0xee,
            "case {case}: no overrun"
        );
        assert_eq!(
            (r.emu.read_u32(STASH_DATA), r.emu.read_u32(STASH_SIZE)),
            (0, 0),
            "case {case}: the whole buffer is freed"
        );
    }
    // A second stash replaces the first.
    let a = r.emu.alloc(8);
    r.emu.write(a, &[1; 8]);
    r.emu.call(STASH, 0, 0, &[a as u32, 8]);
    let first = r.emu.read_u32(STASH_DATA);
    r.emu.clear_calls();
    r.emu.call(STASH, 0, 0, &[a as u32, 4]);
    assert_eq!(r.emu.read_u32(STASH_SIZE), 4);
    assert!(r
        .emu
        .calls()
        .iter()
        .any(|c| c.addr == S_MEM_FREE && c.args[0] == first));
}
