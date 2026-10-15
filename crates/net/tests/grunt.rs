//! The logon link's commands (`systems/net/login.md`, `systems/net/logonlink.md`): each handler
//! runs whole on a plain data store holding a generated reply, beside the model on the same bytes.

mod common;

use benilla_re_difftest::Emu;
use benilla_re_net::{
    can_read, cmd_auth_logon_challenge, cmd_grunt_auth_verify, cmd_grunt_hello, cmd_realm_list,
    cmd_xfer_initiate, on_data, pending_expire, put_challenge_body, Arg, ChallengeRecord,
    DataStore, Fetch, HelloInfo, LinkCall, OnDataEnd, LINK_COMMANDS,
};
use common::{Lcg, Wow};

const CAN_READ: u64 = 0x5b90d0;
const CMD_AUTH_LOGON_CHALLENGE: u64 = 0x5baa20;
const CMD_REALM_LIST: u64 = 0x5ba8d0;
const CMD_XFER_INITIATE: u64 = 0x5baf70;
const PUT_CHALLENGE_BODY: u64 = 0x5bb460;
/// `ClientLink::SetState`, which stores its argument at `+0x80` under the link's lock; the stub
/// logs the state instead.
const SET_STATE: u64 = 0x5bb430;
/// The SRP6 client's proof (`__thiscall`, 9 arguments), stubbed to a chosen result.
const SRP_PROOF: u64 = 0x5d3650;
/// The log call (`__cdecl`).
const LOG: u64 = 0x6dd240;
/// How many stack arguments each slot of the logon object takes.
const SLOT_POPS: [(u32, u32); 4] = [(5, 1), (6, 3), (9, 4), (10, 3)];

struct Rig<'a> {
    e: Emu<'a>,
    link: u64,
    store: u64,
    buf: u64,
    slots: u64,
    capture: u64,
}

impl<'a> Rig<'a> {
    /// A link whose logon object (`+0x198`) has stubs in its slots, the SRP6 proof returning
    /// `srp`, and a plain store at `store` over the buffer `buf`.
    fn new(wow: &'a Wow, srp: u32) -> Rig<'a> {
        let mut e = Emu::new(wow);
        let link = e.alloc(0x400);
        let owner = e.alloc(0x10);
        let vt = e.alloc(0x80);
        let slots = e.alloc(0x200);
        let capture = e.alloc(0x10);
        let store = e.alloc(0x20);
        let buf = e.alloc(0x2000);
        e.write_u32(link + 0x198, owner as u32);
        e.write_u32(owner, vt as u32);
        for i in 0..32u64 {
            e.write_u32(vt + 4 * i, (slots + 0x10 * i) as u32);
        }
        for (slot, pops) in SLOT_POPS {
            e.stub(slots + 0x10 * u64::from(slot), 0, pops);
        }
        // Slot 13 records the store's cursor at the call and counts the calls.
        let mut island = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x40, 0x14, 0xa3];
        island.extend_from_slice(&(capture as u32).to_le_bytes());
        island.extend_from_slice(&[0xff, 0x05]);
        island.extend_from_slice(&(capture as u32 + 4).to_le_bytes());
        island.extend_from_slice(&[0xc2, 0x04, 0x00]);
        e.write(slots + 0x10 * 13, &island);
        e.stub(SET_STATE, 0, 1);
        e.stub(SRP_PROOF, srp, 9);
        e.stub(LOG, 0, 0);
        Rig {
            e,
            link,
            store,
            buf,
            slots,
            capture,
        }
    }

    /// Puts `bytes` in the plain store with the cursor at `read`.
    fn load(&mut self, bytes: &[u8], read: u32) {
        self.e.write(self.buf, bytes);
        let len = bytes.len() as u32;
        let fields = [0x7ff9e4, self.buf as u32, 0, len.max(1), len, read];
        for (i, v) in fields.into_iter().enumerate() {
            self.e.write_u32(self.store + 4 * i as u64, v);
        }
        self.e.write(self.capture, &[0u8; 8]);
        self.e.clear_calls();
    }

    fn read(&self) -> u32 {
        self.e.read_u32(self.store + 0x14)
    }

    /// An argument as the model gives it: a pointer into the buffer as its position.
    fn arg(&self, v: u32) -> Arg {
        let b = self.buf as u32;
        if (b..b + 0x2000).contains(&v) {
            Arg::Pos(v - b)
        } else {
            Arg::Val(v)
        }
    }

    /// The calls out of the handler, in the model's terms.
    fn calls(&self) -> Vec<LinkCall> {
        let mut out = Vec::new();
        for c in self.e.calls() {
            let a = c.args;
            match c.addr {
                SET_STATE => out.push(LinkCall::State(a[0])),
                SRP_PROOF => {
                    assert_eq!((a[5], a[7]), (32, 32), "salt and B lengths");
                    let pos = |v: u32| v.wrapping_sub(self.buf as u32);
                    out.push(LinkCall::Srp {
                        n: pos(a[0]),
                        n_len: a[1],
                        g: pos(a[2]),
                        g_len: a[3],
                        salt: pos(a[4]),
                        b: pos(a[6]),
                    });
                }
                LOG => out.push(LinkCall::LogResult(a[3])),
                addr => {
                    let slot = ((addr - self.slots) / 0x10) as u32;
                    let pops = SLOT_POPS.iter().find(|p| p.0 == slot).expect("slot").1;
                    let mut args: Vec<Arg> =
                        a[..pops as usize].iter().map(|&v| self.arg(v)).collect();
                    if slot == 6 {
                        // The flag is pushed as the dword it was read into; its upper bytes are
                        // whatever the stack held.
                        args[0] = Arg::Val(a[0] & 0xff);
                    }
                    out.push(LinkCall::Owner(slot, args));
                }
            }
        }
        out
    }
}

fn plain(bytes: &[u8], read: u32) -> DataStore {
    let mut m = DataStore::new(Fetch::Heap);
    m.data = bytes.to_vec();
    m.alloc = (bytes.len() as u32).max(1);
    m.size = bytes.len() as u32;
    m.read = read;
    m
}

fn fill(g: &mut Lcg, n: usize) -> Vec<u8> {
    (0..n).map(|_| (g.next() >> 16) as u8).collect()
}

/// A cursor for a packet of `len` bytes: mostly 0, or a prefix the command follows, or past the
/// end.
fn prefix(g: &mut Lcg) -> Vec<u8> {
    match g.next() >> 29 {
        0 => {
            let n = (g.next() >> 20) as usize % 9;
            fill(g, n)
        }
        _ => Vec::new(),
    }
}

/// Cuts a well-formed reply short, or pads it, now and then.
fn cut(g: &mut Lcg, mut v: Vec<u8>) -> Vec<u8> {
    match g.next() >> 28 {
        0..=2 => {
            let n = (g.next() as usize >> 8) % (v.len() + 1);
            v.truncate(n);
        }
        3 => {
            let n = 1 + (g.next() >> 24) as usize % 8;
            let more = fill(g, n);
            v.extend(more);
        }
        _ => {}
    }
    v
}

/// `0x5b90d0` over cursors at, before and past the size, and counts around the bytes left.
#[test]
fn grunt_can_read() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow, 0);
    for size in [0u32, 1, 2, 7, 100, 0x7fff_ffff, 0xffff_fffe] {
        for read in [
            0,
            1,
            size.wrapping_sub(1),
            size,
            size.wrapping_add(1),
            0xffff_ffff,
        ] {
            for n in [
                0u32,
                1,
                2,
                3,
                0x21,
                size.wrapping_sub(read),
                size.wrapping_sub(read).wrapping_add(1),
                0xffff_ffff,
            ] {
                r.e.write_u32(r.store + 0x10, size);
                r.e.write_u32(r.store + 0x14, read);
                let got = r.e.call(CAN_READ, r.store as u32, n, &[]) & 0xff;
                let mut m = plain(&[], read);
                m.size = size;
                assert_eq!(
                    got,
                    u32::from(can_read(&m, n)),
                    "size {size:#x} read {read:#x} n {n:#x}"
                );
            }
        }
    }
}

fn challenge_reply(g: &mut Lcg) -> Vec<u8> {
    let mut v = vec![if g.next() >> 29 == 0 {
        1 + (g.next() >> 24) as u8 % 3
    } else {
        0
    }];
    let result = match g.next() >> 28 {
        0 => 1 + (g.next() >> 24) as u8 % 15,
        1 => 0x10 + (g.next() >> 24) as u8 % 0xf0,
        _ => 0,
    };
    v.push(result);
    if result == 0 {
        v.extend(fill(g, 32));
        let gl = [0usize, 1, 1, 1, 2, 8][(g.next() >> 16) as usize % 6];
        v.push(gl as u8);
        v.extend(fill(g, gl));
        let nl = [0usize, 1, 32, 32, 32, 40][(g.next() >> 16) as usize % 6];
        v.push(nl as u8);
        v.extend(fill(g, nl + 32 + 16));
        let flag = if g.next() >> 30 == 0 {
            0
        } else {
            (g.next() >> 24) as u8
        };
        v.push(flag);
        if flag != 0 {
            v.extend(fill(g, 20));
        }
    }
    cut(g, v)
}

/// `ClientLink::CmdAuthLogonChallenge` whole, on replies that fail, succeed, carry the security
/// fields or not, and stop at every length, with the SRP6 proof succeeding and failing: the
/// result, the cursor, `B` at `+0x1dc` and every call out, in order.
#[test]
fn grunt_logon_challenge_reply() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut g = Lcg(0x6c09_0001);
    for srp in [0u32, 1] {
        let mut r = Rig::new(&wow, srp);
        for case in 0..3000 {
            let mut bytes = prefix(&mut g);
            let at = bytes.len() as u32;
            bytes.extend(challenge_reply(&mut g));
            r.load(&bytes, at);
            r.e.write(r.link + 0x1dc, &[0xee; 32]);
            let got = r.e.call(
                CMD_AUTH_LOGON_CHALLENGE,
                r.link as u32,
                0,
                &[r.store as u32],
            );
            let mut m = plain(&bytes, at);
            let mut b = [0xee; 32];
            let (want, calls) = cmd_auth_logon_challenge(&mut m, srp, &mut b);
            let ctx = format!("srp {srp} case {case} {bytes:02x?}");
            assert_eq!(got, want, "{ctx}: result");
            assert_eq!(r.read(), m.read, "{ctx}: cursor");
            assert_eq!(r.e.read(r.link + 0x1dc, 32), b.to_vec(), "{ctx}: B");
            assert_eq!(r.calls(), calls, "{ctx}: calls");
        }
    }
}

fn cstring(g: &mut Lcg) -> Vec<u8> {
    let n = match g.next() >> 29 {
        0 => 0xff + (g.next() >> 28) as usize % 4,
        1 => 0,
        _ => (g.next() >> 24) as usize % 24,
    };
    let mut s: Vec<u8> = fill(g, n).into_iter().map(|b| b | 1).collect();
    s.push(0);
    s
}

fn realm_list(g: &mut Lcg) -> Vec<u8> {
    let mut body = fill(g, 4);
    let count = [0u8, 1, 2, 5][(g.next() >> 16) as usize % 4];
    body.push(count);
    for _ in 0..count {
        body.extend(fill(g, 5));
        body.extend(cstring(g));
        body.extend(cstring(g));
        body.extend(fill(g, 7));
    }
    body.extend(fill(g, 2));
    let mut size = body.len() as u16;
    if g.next() >> 29 == 0 {
        size = size.wrapping_add([1u16, 0xffff, 2][(g.next() >> 16) as usize % 3]);
    }
    let mut v = size.to_le_bytes().to_vec();
    v.extend(body);
    cut(g, v)
}

/// `ClientLink::CmdRealmList` whole, on lists of 0 to 5 records with names up to past the 256-byte
/// capacity, a size field that matches or is off by one or two, and every truncation: the
/// result, the cursor, and the cursor slot 13 sees.
#[test]
fn grunt_realm_list() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut g = Lcg(0x4ea1_0001);
    let mut r = Rig::new(&wow, 0);
    for case in 0..3000 {
        let mut bytes = prefix(&mut g);
        let at = bytes.len() as u32;
        bytes.extend(realm_list(&mut g));
        r.load(&bytes, at);
        let got =
            r.e.call(CMD_REALM_LIST, r.link as u32, 0, &[r.store as u32]);
        let mut m = plain(&bytes, at);
        let (want, seen) = cmd_realm_list(&mut m);
        let ctx = format!("case {case} {bytes:02x?}");
        assert_eq!(got, want, "{ctx}: result");
        assert_eq!(r.read(), m.read, "{ctx}: cursor");
        let calls = r.e.read_u32(r.capture + 4);
        assert_eq!(calls, u32::from(seen.is_some()), "{ctx}: slot 13 calls");
        if let Some(at) = seen {
            assert_eq!(r.e.read_u32(r.capture), at, "{ctx}: cursor at slot 13");
        }
    }
}

/// `ClientLink::CmdXferInitiate` whole, on names of 0 to 255 bytes and every truncation: the
/// result, the cursor, and slot 9's name, size and digest.
#[test]
fn grunt_xfer_initiate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut g = Lcg(0x0f11_e001);
    let mut r = Rig::new(&wow, 0);
    for case in 0..3000 {
        let mut bytes = prefix(&mut g);
        let at = bytes.len() as u32;
        let n = match g.next() >> 29 {
            0 => 255,
            1 => 0,
            _ => (g.next() >> 24) as usize % 40,
        };
        let mut v = vec![n as u8];
        v.extend(fill(&mut g, n + 24));
        bytes.extend(cut(&mut g, v));
        r.load(&bytes, at);
        let got =
            r.e.call(CMD_XFER_INITIATE, r.link as u32, 0, &[r.store as u32]);
        let mut m = plain(&bytes, at);
        let (want, offer) = cmd_xfer_initiate(&mut m);
        let ctx = format!("case {case} {bytes:02x?}");
        assert_eq!(got, want, "{ctx}: result");
        assert_eq!(r.read(), m.read, "{ctx}: cursor");
        let calls: Vec<_> = r.e.calls().to_vec();
        match offer {
            None => assert!(calls.is_empty(), "{ctx}: no call"),
            Some(o) => {
                assert_eq!(calls.len(), 1, "{ctx}: one call");
                let a = calls[0].args;
                assert_eq!(calls[0].addr, r.slots + 0x10 * 9, "{ctx}: slot 9");
                let name = r.e.read(u64::from(a[0]), o.name.len());
                assert_eq!(name, o.name, "{ctx}: name");
                assert_eq!((a[1], a[2]), o.size, "{ctx}: size");
                assert_eq!(a[3], r.buf as u32 + o.digest, "{ctx}: digest");
            }
        }
    }
}

/// `ClientLink::PutChallengeBody` whole, appending to stores that already hold 0 to 19 bytes,
/// with accounts of 0 to 63 characters, the most the senders copy to `+0x19c`, and of 64 to 300
/// written there directly: every byte of the store and its size.
#[test]
fn grunt_challenge_body() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut g = Lcg(0xc4a1_1e00);
    let mut r = Rig::new(&wow, 0);
    let rec = r.e.alloc(0x40);
    for case in 0..2000 {
        let pn = (g.next() >> 16) as usize % 20;
        let pre = fill(&mut g, pn);
        let n = match g.next() >> 29 {
            0 => 64 + (g.next() >> 16) as usize % 237,
            1 => 0,
            2 => 63,
            _ => (g.next() >> 24) as usize % 64,
        };
        let account: Vec<u8> = fill(&mut g, n).into_iter().map(|b| b | 1).collect();
        let record = ChallengeRecord {
            game: g.next(),
            platform: g.next(),
            system: g.next(),
            version: [
                g.next() as u8,
                (g.next() >> 8) as u8,
                (g.next() >> 16) as u8,
            ],
            build: g.next() as u16,
            locale: g.next(),
            timezone: g.next(),
        };
        let address = g.next();
        // The record as the client lays it out.
        let mut raw = vec![0u8; 0x24];
        raw[8..12].copy_from_slice(&record.game.to_le_bytes());
        raw[0xc..0x10].copy_from_slice(&record.platform.to_le_bytes());
        raw[0x10..0x14].copy_from_slice(&record.system.to_le_bytes());
        raw[0x14..0x17].copy_from_slice(&record.version);
        raw[0x18..0x1a].copy_from_slice(&record.build.to_le_bytes());
        raw[0x1c..0x20].copy_from_slice(&record.locale.to_le_bytes());
        raw[0x20..0x24].copy_from_slice(&record.timezone.to_le_bytes());
        r.e.write(rec, &raw);
        r.e.write_u32(r.link + 0x7c, address);
        let mut z = account.clone();
        z.push(0);
        r.e.write(r.link + 0x19c, &z);
        // A plain store whose buffer already holds the prefix, with room for the body.
        r.load(&pre, 0);
        r.e.write_u32(r.store + 0xc, 0x1000);
        r.e.call(
            PUT_CHALLENGE_BODY,
            r.link as u32,
            0,
            &[r.store as u32, rec as u32],
        );
        let mut m = plain(&pre, 0);
        m.alloc = 0x1000;
        put_challenge_body(&mut m, &record, address, &account);
        let ctx = format!("case {case} account {n}");
        assert_eq!(r.e.read_u32(r.store + 0x10), m.size, "{ctx}: size");
        let want: Vec<u8> = (0..m.size).map(|p| m.byte(p)).collect();
        assert_eq!(r.e.read(r.buf, m.size as usize), want, "{ctx}: bytes");
    }
}

const ON_DATA: u64 = 0x5bb0e0;
const INIT_LINK_COMMANDS: u64 = 0x5b87b0;
const LINK_TABLE: u64 = 0x85e278;
const REQUEST_DISCONNECT: u64 = 0x5bb290;
const HEX_DUMP: u64 = 0x6dd260;

/// A command handler that adds `[k]` to the cursor and returns `[r]`, or, past the size, puts the
/// cursor at `size + 1` and returns 1, as the link's handlers do; `ret 4`.
fn handler_island(k: u32, r: u32) -> Vec<u8> {
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x48, 0x14, 0x03, 0x0d];
    c.extend_from_slice(&k.to_le_bytes());
    c.extend_from_slice(&[
        0x8b, 0x50, 0x10, 0x3b, 0xca, 0x76, 0x0c, 0x42, 0x89, 0x50, 0x14,
    ]);
    c.extend_from_slice(&[0xb8, 1, 0, 0, 0, 0xc2, 0x04, 0x00, 0x89, 0x48, 0x14, 0xa1]);
    c.extend_from_slice(&r.to_le_bytes());
    c.extend_from_slice(&[0xc2, 0x04, 0x00]);
    c
}

/// `ClientLink::OnData` whole on the link's plain store (`+0x17c`), its command table filled by
/// `0x5b87b0` and each handler replaced by one that consumes a chosen count and returns a chosen
/// result: chunks of 0 to 40 bytes, and of 96 to 351 past the 128-byte dump, appended to 0 to 15 kept
/// bytes, with unknown command bytes,
/// handlers that wait, fail, consume or read on. The store's fields and bytes, the log line, the
/// hex dump and the disconnect are compared.
#[test]
fn grunt_on_data() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut g = Lcg(0x0da7_a001);
    let mut r = Rig::new(&wow, 0);
    let arena = r.e.alloc(0x100000);
    r.e.stub_alloc_sized(0x6462e0, arena + 0x1000, arena + 0x100000, 0, 4);
    r.e.stub_realloc_sized(0x646320, 5);
    r.e.stub(REQUEST_DISCONNECT, 0, 0);
    r.e.stub(HEX_DUMP, 0, 2);
    r.e.call(INIT_LINK_COMMANDS, 0, 0, &[]);
    let script = r.e.alloc(0x40);
    for i in 0..7u64 {
        let island = r.e.alloc(0x40);
        r.e.write(
            island,
            &handler_island(
                script as u32 + 8 * i as u32,
                script as u32 + 8 * i as u32 + 4,
            ),
        );
        r.e.write_u32(LINK_TABLE + 16 * i + 8, island as u32);
        assert_eq!(
            r.e.read(LINK_TABLE + 16 * i, 1)[0],
            LINK_COMMANDS[i as usize],
            "table byte {i}"
        );
    }
    let store = r.link + 0x17c;
    for case in 0..4000 {
        let mut plan = [(0u32, 0u32); 7];
        for p in plan.iter_mut() {
            let k = (g.next() >> 16) % 12;
            let res = [0u32, 1, 2, 2, 2, 3][(g.next() >> 16) as usize % 6];
            *p = (k, res);
        }
        for (i, &(k, res)) in plan.iter().enumerate() {
            r.e.write_u32(script + 8 * i as u64, k);
            r.e.write_u32(script + 8 * i as u64 + 4, res);
        }
        let kept = (g.next() >> 16) as usize % 16;
        let n = if g.next() >> 29 == 0 {
            0x60 + (g.next() >> 16) as usize % 0x100
        } else {
            (g.next() >> 16) as usize % 41
        };
        let cmd = |g: &mut Lcg| {
            if g.next() >> 29 == 0 {
                (g.next() >> 24) as u8
            } else {
                LINK_COMMANDS[(g.next() >> 16) as usize % 7]
            }
        };
        let old: Vec<u8> = (0..kept).map(|_| cmd(&mut g)).collect();
        let data: Vec<u8> = (0..n).map(|_| cmd(&mut g)).collect();
        // The store as the last call left it: the kept bytes in a 256-byte buffer.
        r.e.write(arena, &[0u8; 0x1000]);
        r.e.reset_bump(arena + 0x1000);
        r.e.write(arena, &old);
        let fields = [0x7ff9e4, arena as u32, 0, 0x100, kept as u32, 0xffff_ffff];
        for (i, v) in fields.into_iter().enumerate() {
            r.e.write_u32(store + 4 * i as u64, v);
        }
        r.e.write(r.buf, &data);
        r.e.clear_calls();
        r.e.call(ON_DATA, r.link as u32, 0, &[0, 0, r.buf as u32, n as u32]);

        let mut m = DataStore::new(Fetch::Heap);
        m.data = old.clone();
        m.alloc = 0x100;
        m.size = kept as u32;
        m.read = 0xffff_ffff;
        let end = on_data(&mut m, &data, |i, s| {
            let (k, res) = plan[i];
            let to = s.read.wrapping_add(k);
            if to > s.size {
                s.read = s.size.wrapping_add(1);
                1
            } else {
                s.read = to;
                res
            }
        });
        let ctx = format!("case {case} kept {old:02x?} data {data:02x?} plan {plan:?}");
        let f = |o: u64| r.e.read_u32(store + o);
        assert_eq!(
            (f(8), f(12), f(16), f(20)),
            (m.base, m.alloc, m.size, m.read),
            "{ctx}: fields"
        );
        let buffer = u64::from(f(4));
        if m.size != 0 {
            let want: Vec<u8> = (0..m.size).map(|p| m.byte(p)).collect();
            assert_eq!(r.e.read(buffer, m.size as usize), want, "{ctx}: bytes");
        }
        let calls = r.e.calls();
        let logged: Vec<u32> = calls
            .iter()
            .filter(|c| c.addr == LOG)
            .map(|c| c.args[3])
            .collect();
        let dumped: Vec<(u32, u32)> = calls
            .iter()
            .filter(|c| c.addr == HEX_DUMP)
            .map(|c| (c.args[0].wrapping_sub(buffer as u32), c.args[1]))
            .collect();
        let closed = calls
            .iter()
            .filter(|c| c.addr == REQUEST_DISCONNECT)
            .count();
        match end {
            OnDataEnd::Keep => {
                assert_eq!(
                    (logged.len(), dumped.len(), closed),
                    (0, 0, 0),
                    "{ctx}: keep"
                );
            }
            OnDataEnd::Bad { command, dump } => {
                assert_eq!(logged, vec![u32::from(command)], "{ctx}: log");
                assert_eq!(dumped, dump.into_iter().collect::<Vec<_>>(), "{ctx}: dump");
                assert_eq!(closed, 1, "{ctx}: disconnect");
            }
        }
    }
}

/// `Grunt::Pending::Expire` whole on lists of 0 to 6 nodes linked as `Grunt::Pending::Add` links
/// them, with times of 0, expired, at the timeout and fresh, clocks that wrap, and `force` 0 and
/// 1: the list left behind, its back links, the node handed to slot 0 and the node freed.
#[test]
fn grunt_pending_expire() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x40);
    let nodes = e.alloc(0x200);
    let vt = e.alloc(0x10);
    let expired = e.alloc(0x10);
    let now_at = e.alloc(0x10);
    e.stub(expired, 0, 1);
    e.write_u32(vt, expired as u32);
    e.stub(0x6579b0, 0, 0);
    e.stub(0x6579c0, 0, 0);
    e.stub(0x646430, 1, 4);
    let mut c = vec![0xa1];
    c.extend((now_at as u32).to_le_bytes());
    c.push(0xc3);
    let clock = e.alloc(0x10);
    e.write(clock, &c);
    let mut j = vec![0xe9];
    j.extend((clock as u32).wrapping_sub(0x42c010 + 5).to_le_bytes());
    e.write(0x42c010, &j);
    let term = obj + 0x28;
    let mut g = Lcg(0x9e1d_0001);
    for case in 0..4000 {
        let n = (g.next() >> 16) as usize % 7;
        let now = if g.next() >> 30 == 0 {
            g.next() >> 28
        } else {
            g.next()
        };
        let timeout = [0u32, 1, 1000, 30000, 0xffff_ffff][(g.next() >> 16) as usize % 5];
        let times: Vec<u32> = (0..n)
            .map(|_| match g.next() >> 29 {
                0 | 1 => 0,
                2 => now.wrapping_sub(timeout),
                3 => now.wrapping_sub(timeout).wrapping_add(1),
                4 => now,
                _ => g.next(),
            })
            .collect();
        let force = g.next() >> 30 == 0;
        e.write_u32(now_at, now);
        e.write_u32(obj, vt as u32);
        e.write_u32(obj + 4, timeout);
        e.write_u32(obj + 0x24, 0);
        let at = |i: usize| nodes + 0x20 * i as u64;
        let mut prev = term;
        for (i, &t) in times.iter().enumerate() {
            e.write(at(i), &[0xa5; 0x20]);
            e.write_u32(at(i), prev as u32);
            e.write_u32(prev + 4, at(i) as u32);
            e.write_u32(at(i) + 0x10, t);
            prev = at(i);
        }
        e.write_u32(prev + 4, term as u32 | 1);
        e.write_u32(term, prev as u32);
        e.clear_calls();
        e.call(0x5b8650, obj as u32, 0, &[u32::from(force)]);
        let want = pending_expire(&times, force, now, timeout);
        let ctx =
            format!("case {case} times {times:x?} force {force} now {now:#x} timeout {timeout:#x}");
        let left: Vec<u64> = (0..n).filter(|&i| Some(i) != want).map(at).collect();
        let mut walked = Vec::new();
        let mut link = term;
        loop {
            let next = e.read_u32(link + 4);
            let back = if walked.is_empty() { term } else { link };
            if next & 1 == 1 {
                assert_eq!(u64::from(next & !1), term, "{ctx}: the end");
                assert_eq!(u64::from(e.read_u32(term)), back, "{ctx}: the last link");
                break;
            }
            assert_eq!(
                u64::from(e.read_u32(u64::from(next))),
                back,
                "{ctx}: a back link"
            );
            walked.push(u64::from(next));
            link = u64::from(next);
            assert!(walked.len() <= n, "{ctx}: a loop");
        }
        assert_eq!(walked, left, "{ctx}: the list");
        let calls: Vec<(u64, u32)> = e
            .calls()
            .iter()
            .filter(|c| c.addr == expired || c.addr == 0x646430)
            .map(|c| (c.addr, c.args[0]))
            .collect();
        let want_calls = match want {
            Some(i) => vec![(expired, at(i) as u32), (0x646430, at(i) as u32)],
            None => vec![],
        };
        assert_eq!(calls, want_calls, "{ctx}: the calls");
    }
}

/// `ServerLink::CmdGruntAuthVerify` whole on stores from empty to past a whole command, length
/// bytes from 0 to `0x1f`, `0x20` and above, and cursors at the start and inside: the result and
/// the cursor.
#[test]
fn grunt_auth_verify() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow, 0);
    let mut g = Lcg(0xa0e1_0001);
    for case in 0..6000 {
        let len = [0u8, 1, 0x10, 0x1f, 0x20, 0x21, 0xff][(g.next() >> 16) as usize % 7];
        let lead = (g.next() >> 16) as usize % 3;
        let mut bytes = fill(&mut g, lead);
        bytes.push(len);
        bytes.extend(fill(&mut g, usize::from(len) + 0x24));
        let keep = match g.next() >> 30 {
            0 => (g.next() >> 16) as usize % (bytes.len() + 1),
            1 => bytes.len() - 1,
            _ => bytes.len() + (g.next() >> 30) as usize,
        };
        bytes.resize(keep, 0x5a);
        r.load(&bytes, lead.min(bytes.len()) as u32);
        let got = r.e.call(0x5b9630, r.link as u32, 0, &[r.store as u32]);
        let mut m = plain(&bytes, lead.min(bytes.len()) as u32);
        let want = cmd_grunt_auth_verify(&mut m);
        assert_eq!(
            (got, r.read()),
            (want, m.read),
            "case {case} len {len:#x} {} bytes",
            bytes.len()
        );
    }
}

/// `ServerLink::CmdGruntHello` whole, its realm source, address formatter, log and send replaced:
/// stores from empty to past the u32, the modes 0, 1 and others, a realm source that answers or
/// not, records of every field width with names of 0 to 34 bytes and an address of 0, which takes
/// the link's own: the result, the value at `+0x74`, the reply byte for byte and the state.
#[test]
fn grunt_hello() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow, 0);
    let link = r.e.alloc(0x200);
    let source = r.e.alloc(0x20);
    let vt = r.e.alloc(0x20);
    let info = r.e.alloc(0x40);
    let answer = r.e.alloc(0x10);
    let sent = r.e.alloc(0x800);
    r.e.write_u32(link + 0x108, source as u32);
    r.e.write_u32(source, vt as u32);
    // Slot 4: copies the record at `info` to the argument and returns the byte at `answer`.
    let mut c = vec![0x56, 0x57, 0x8b, 0x7c, 0x24, 0x0c, 0xbe];
    c.extend((info as u32).to_le_bytes());
    c.extend([0xb9, 0x40, 0, 0, 0, 0xf3, 0xa4, 0x5f, 0x5e, 0xa0]);
    c.extend((answer as u32).to_le_bytes());
    c.extend([0xc2, 0x04, 0x00]);
    let slot4 = r.e.alloc(0x40);
    r.e.write(slot4, &c);
    r.e.write_u32(vt + 0x10, slot4 as u32);
    // `ServerLink::Send(store)`: copies the store's bytes to `sent + 4`, the count to `sent`.
    let mut c = vec![
        0x56, 0x57, 0x8b, 0x4c, 0x24, 0x0c, 0x8b, 0x71, 0x04, 0x8b, 0x41, 0x10, 0xa3,
    ];
    c.extend((sent as u32).to_le_bytes());
    c.push(0xbf);
    c.extend((sent as u32 + 4).to_le_bytes());
    c.extend([0x89, 0xc1, 0xf3, 0xa4, 0x5f, 0x5e, 0xc2, 0x04, 0x00]);
    let send = r.e.alloc(0x40);
    r.e.write(send, &c);
    let mut j = vec![0xe9];
    j.extend((send as u32).wrapping_sub(0x5b9b00 + 5).to_le_bytes());
    r.e.write(0x5b9b00, &j);
    let text = r.e.alloc(0x10);
    r.e.stub(0x5b90a0, text as u32, 2);
    r.e.stub(0x646430, 1, 4);
    let mut g = Lcg(0x4e11_0001);
    for case in 0..4000 {
        let n = [0usize, 1, 3, 4, 7][(g.next() >> 16) as usize % 5];
        let bytes = fill(&mut g, n);
        let mode = [0u32, 0, 0, 1, 2, 0xffff_ffff][(g.next() >> 16) as usize % 6];
        let answers = g.next() >> 30 != 0;
        let name: Vec<u8> = (0..(g.next() >> 16) % 35)
            .map(|_| 1 + (g.next() >> 24) as u8 % 0xff)
            .collect();
        let i = HelloInfo {
            id: g.next(),
            flags: (g.next() >> 24) as u8,
            addr: if g.next() >> 30 == 0 { 0 } else { g.next() },
            port: (g.next() >> 16) as u16,
            b12: (g.next() >> 24) as u8,
            b13: (g.next() >> 24) as u8,
            b14: (g.next() >> 24) as u8,
            w16: (g.next() >> 16) as u16,
            d18: g.next(),
            b1c: (g.next() >> 24) as u8,
            name,
        };
        let mut rec = vec![0xa5u8; 0x40];
        rec[0..4].copy_from_slice(&i.id.to_le_bytes());
        rec[4] = i.flags;
        rec[0xc..0x10].copy_from_slice(&i.addr.to_le_bytes());
        rec[0x10..0x12].copy_from_slice(&i.port.to_le_bytes());
        rec[0x12] = i.b12;
        rec[0x13] = i.b13;
        rec[0x14] = i.b14;
        rec[0x16..0x18].copy_from_slice(&i.w16.to_le_bytes());
        rec[0x18..0x1c].copy_from_slice(&i.d18.to_le_bytes());
        rec[0x1c] = i.b1c;
        rec[0x1d..0x1d + i.name.len()].copy_from_slice(&i.name);
        rec[0x1d + i.name.len()] = 0;
        r.e.write(info, &rec);
        r.e.write(answer, &[u8::from(answers)]);
        let (own, tail) = (g.next(), g.next());
        r.e.write_u32(link + 0x74, 0xdead_beef);
        r.e.write_u32(link + 0x78, tail);
        r.e.write_u32(link + 0x7c, own);
        r.e.write_u32(link + 0x90, 7);
        r.e.write_u32(link + 0x104, mode);
        r.e.write_u32(sent, 0xffff_ffff);
        r.load(&bytes, 0);
        let got = r.e.call(0x5b8c40, link as u32, 0, &[r.store as u32]);
        let mut m = plain(&bytes, 0);
        let want = cmd_grunt_hello(&mut m, mode, answers.then_some(&i), own, tail);
        let ctx = format!("case {case} {n} bytes mode {mode:#x} answers {answers}");
        assert_eq!(got, want.result, "{ctx}: result");
        let value = r.e.read_u32(link + 0x74);
        assert_eq!(value, want.value.unwrap_or(0xdead_beef), "{ctx}: +0x74");
        let count = r.e.read_u32(sent);
        let reply = (count != 0xffff_ffff).then(|| r.e.read(sent + 4, count as usize));
        assert_eq!(reply, want.reply, "{ctx}: reply");
        let state = if want.reply.is_some() { 3 } else { 7 };
        assert_eq!(r.e.read_u32(link + 0x90), state, "{ctx}: state");
    }
}
