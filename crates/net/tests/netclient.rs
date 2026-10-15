//! The world connection's client object (`systems/net/session.md`, `systems/net/dispatch.md`,
//! `systems/net/wire.md`), each function run whole beside the model.

mod common;

use benilla_re_difftest::Emu;
use benilla_re_net::{
    dispatch_opcode, net_deinitialize, net_initialize, net_send, queue_event, PingState,
};
use common::{Lcg, Wow};

const HANDLE_IDLE: u64 = 0x537ff0;
const PING: u64 = 0x537e10;
const HANDLE_PONG: u64 = 0x537d60;
const GET_NET_STATS: u64 = 0x537f20;
const SEND: u64 = 0x5379a0;
const DISPATCH: u64 = 0x537aa0;
const HANDLE_DATA: u64 = 0x537c50;
/// `OsGetAsyncTimeMs`, a `jmp` to the clock; the test points it at an island that returns the
/// dword at the rig's clock.
const CLOCK_FN: u64 = 0x42c010;
/// `WowConnection::Send` (`__thiscall`, 2 arguments); an island records the store it gets.
const CONN_SEND: u64 = 0x5b5630;
/// The global counts of packets dispatched, bytes received and bytes sent.
const DISPATCHED: u64 = 0xc0d46c;
const RECEIVED: u64 = 0xc0d468;
const SENT: u64 = 0xc0d45c;

struct Rig<'a> {
    e: Emu<'a>,
    nc: u64,
    conn: u64,
    clock: u64,
    /// What the `WowConnection::Send` island records: the store's buffer, size and cursor, a
    /// call count, and its second argument.
    sent: u64,
    slot16: u64,
}

impl<'a> Rig<'a> {
    fn new(wow: &'a Wow) -> Rig<'a> {
        let mut e = Emu::new(wow);
        let nc = e.alloc(0x1b00);
        let conn = e.alloc(0x140);
        let vt = e.alloc(0x80);
        let slot16 = e.alloc(0x10);
        let clock = e.alloc(0x10);
        let sent = e.alloc(0x20);
        let arena = e.alloc(0x100000);
        e.write_u32(nc, vt as u32);
        e.write_u32(vt + 0x40, slot16 as u32);
        e.write_u32(nc + 0x1a58, conn as u32);
        e.stub(slot16, 0, 1);
        // mov eax,[clock]; ret
        let island = e.alloc(0x10);
        let mut code = vec![0xa1];
        code.extend_from_slice(&(clock as u32).to_le_bytes());
        code.push(0xc3);
        e.write(island, &code);
        let rel = (island as u32).wrapping_sub(CLOCK_FN as u32 + 5);
        let mut jmp = vec![0xe9];
        jmp.extend_from_slice(&rel.to_le_bytes());
        e.write(CLOCK_FN, &jmp);
        // The send island: copies the store's +4, +0x10 and +0x14, counts, keeps the argument.
        let s = sent as u32;
        let mut code = vec![0x8b, 0x44, 0x24, 0x04];
        for (off, slot) in [(0x04u8, 0u32), (0x10, 4), (0x14, 8)] {
            code.extend_from_slice(&[0x8b, 0x50, off, 0x89, 0x15]);
            code.extend_from_slice(&(s + slot).to_le_bytes());
        }
        code.extend_from_slice(&[0xff, 0x05]);
        code.extend_from_slice(&(s + 12).to_le_bytes());
        code.extend_from_slice(&[0x8b, 0x54, 0x24, 0x08, 0x89, 0x15]);
        code.extend_from_slice(&(s + 16).to_le_bytes());
        code.extend_from_slice(&[0xc2, 0x08, 0x00]);
        let send_island = e.alloc(0x40);
        e.write(send_island, &code);
        let rel = (send_island as u32).wrapping_sub(CONN_SEND as u32 + 5);
        let mut jmp = vec![0xe9];
        jmp.extend_from_slice(&rel.to_le_bytes());
        e.write(CONN_SEND, &jmp);
        e.stub(0x6579b0, 0, 0);
        e.stub(0x6579c0, 0, 0);
        e.stub(0x63cd50, 0, 0);
        e.stub(0x538020, 0, 0);
        e.stub(0x538030, 0, 0);
        e.stub_alloc_sized(0x6462e0, arena, arena + 0x100000, 0, 4);
        e.stub_realloc_sized(0x646320, 5);
        e.stub(0x646430, 1, 4);
        Rig {
            e,
            nc,
            conn,
            clock,
            sent,
            slot16,
        }
    }

    fn put(&mut self, st: &PingState) {
        let n = self.nc;
        self.e.write_u32(n + 0x70, st.state);
        for (off, v) in [
            (0x1a64, st.last_ping),
            (0x1a68, st.seq),
            (0x1aac, st.read_idx),
            (0x1ab0, st.write_idx),
            (0x1ab4, st.sent),
            (0x1ab8, st.received),
            (0x1abc, st.connected_at),
        ] {
            self.e.write_u32(n + off, v);
        }
        for (i, &v) in st.ring.iter().enumerate() {
            self.e.write_u32(n + 0x1a6c + 4 * i as u64, v);
        }
        self.e.write(self.sent, &[0u8; 0x14]);
        self.e.clear_calls();
    }

    fn get(&self) -> PingState {
        let f = |o: u64| self.e.read_u32(self.nc + o);
        PingState {
            state: f(0x70),
            last_ping: f(0x1a64),
            seq: f(0x1a68),
            ring: std::array::from_fn(|i| f(0x1a6c + 4 * i as u64)),
            read_idx: f(0x1aac),
            write_idx: f(0x1ab0),
            sent: f(0x1ab4),
            received: f(0x1ab8),
            connected_at: f(0x1abc),
        }
    }
}

fn state(g: &mut Lcg) -> PingState {
    PingState {
        state: [6, 6, 6, 2, 0, 7][(g.next() >> 16) as usize % 6],
        last_ping: g.next(),
        seq: if g.next() >> 29 == 0 {
            0xffff_ffff
        } else {
            g.next() >> 20
        },
        ring: std::array::from_fn(|_| {
            if g.next() >> 30 == 0 {
                g.next()
            } else {
                g.next() >> 20
            }
        }),
        read_idx: (g.next() >> 16) % 16,
        write_idx: (g.next() >> 16) % 16,
        sent: g.next(),
        received: g.next(),
        connected_at: g.next(),
    }
}

/// `NetClient::HandleIdle` at `now − last_ping` of 0, 29999, 30000, 30001, the sign boundary and
/// the wrap, and fuzzed: whether it reaches `NetClient::Ping`, stubbed here.
#[test]
fn netclient_idle() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    r.e.stub(PING, 0, 0);
    let mut g = Lcg(0x1d1e_0001);
    let mut deltas = vec![
        0u32,
        1,
        29999,
        30000,
        30001,
        0x7fff_ffff,
        0x8000_752f,
        0x8000_7530,
        0xffff_ffff,
    ];
    deltas.extend((0..200).map(|_| g.next()));
    for &d in &deltas {
        for last in [0u32, 1, 0x8000_0000, 0xffff_ff00, g.next()] {
            let st = PingState {
                last_ping: last,
                ..PingState::default()
            };
            r.put(&st);
            let now = last.wrapping_add(d);
            r.e.write_u32(r.clock, now);
            r.e.call(HANDLE_IDLE, r.nc as u32, 0, &[]);
            let pinged = r.e.calls().iter().any(|c| c.addr == PING);
            assert_eq!(pinged, st.idle_due(now), "last {last:#x} delta {d:#x}");
        }
    }
}

/// `NetClient::Ping` in and out of state 6, at every write index, with the sequence number at
/// its wrap: the clock stamp, the number, and the packet `NetClient::Send` passes on.
#[test]
fn netclient_ping() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x9149_0001);
    for case in 0..2000 {
        let mut st = state(&mut g);
        st.write_idx = case % 16;
        r.put(&st);
        r.e.write(r.conn + 0x120, &[1]);
        let now = g.next();
        r.e.write_u32(r.clock, now);
        r.e.call(PING, r.nc as u32, 0, &[]);
        let packet = st.ping(now);
        let ctx = format!("case {case} {st:?}");
        let got = r.get();
        assert_eq!(
            (got.last_ping, got.seq),
            (st.last_ping, st.seq),
            "{ctx}: stamp, number"
        );
        let calls = r.e.read_u32(r.sent + 12);
        if st.state == 6 {
            assert_eq!(calls, 1, "{ctx}: sent");
            let buf = u64::from(r.e.read_u32(r.sent));
            let size = r.e.read_u32(r.sent + 4);
            assert_eq!(r.e.read_u32(r.sent + 8), 0, "{ctx}: cursor");
            assert_eq!(r.e.read(buf, size as usize), packet, "{ctx}: packet");
        } else {
            assert_eq!(calls, 0, "{ctx}: dropped");
        }
    }
}

/// `NetClient::HandlePong` with the current sequence number and others, at every pair of read
/// and write indices: the ring, both indices, and the log of a stale pong.
#[test]
fn netclient_pong() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let store = r.e.alloc(0x20);
    let buf = r.e.alloc(0x10);
    let mut g = Lcg(0x9049_0001);
    for case in 0..2560u32 {
        let mut st = state(&mut g);
        st.read_idx = case % 16;
        st.write_idx = (case / 16) % 16;
        let value = if case % 5 == 4 {
            st.seq ^ (1 << (g.next() >> 27))
        } else {
            st.seq
        };
        r.put(&st);
        r.e.write(buf, &value.to_le_bytes());
        for (i, v) in [0x7ff9e4u32, buf as u32, 0, 4, 4, 0]
            .into_iter()
            .enumerate()
        {
            r.e.write_u32(store + 4 * i as u64, v);
        }
        let now = g.next();
        r.e.write_u32(r.clock, now);
        r.e.call(HANDLE_PONG, r.nc as u32, 0, &[store as u32]);
        let matched = st.pong(value, now);
        let ctx = format!("case {case}");
        assert_eq!(r.get(), st, "{ctx}: state");
        let logged = r.e.calls().iter().any(|c| c.addr == 0x63cd50);
        assert_eq!(logged, !matched, "{ctx}: log");
    }
}

/// `NetClient::GetNetStats` with elapsed times of 0, 1, the wrap and fuzzed, byte counts at 0 and
/// the extremes, and every pair of ring indices: both rates by their bits (any NaN for a NaN)
/// and the mean, whose sum wraps.
#[test]
fn netclient_stats() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let out = r.e.alloc(0x10);
    let mut g = Lcg(0x57a7_0001);
    for case in 0..3000u32 {
        let mut st = state(&mut g);
        st.read_idx = case % 16;
        st.write_idx = (case / 16) % 16;
        if case % 7 == 0 {
            st.received = [0, 1, 0xffff_ffff][(case / 7 % 3) as usize];
        }
        let elapsed = match case % 11 {
            0 => 0,
            1 => 1,
            2 => 0xffff_ffff,
            3 => 0x8000_0000,
            _ => g.next() >> (g.next() >> 27),
        };
        r.put(&st);
        let now = st.connected_at.wrapping_add(elapsed);
        r.e.write_u32(r.clock, now);
        r.e.write(out, &[0xee; 12]);
        r.e.call(
            GET_NET_STATS,
            r.nc as u32,
            0,
            &[out as u32, out as u32 + 4, out as u32 + 8],
        );
        let (recv, sent, mean) = st.net_stats(now);
        let ctx = format!("case {case} elapsed {elapsed:#x}");
        for (i, want) in [recv, sent].into_iter().enumerate() {
            let got = r.e.read_f32(out + 4 * i as u64);
            assert!(
                got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan()),
                "{ctx}: rate {i}: binary {:#010x}, ours {:#010x}",
                got.to_bits(),
                want.to_bits()
            );
        }
        assert_eq!(r.e.read_u32(out + 8), mean, "{ctx}: mean");
    }
}

/// `NetClient::Send` in and out of state 6, with 0 and some bytes left to read, on an armed and
/// an unarmed connection: the connection's send, the counts, and the cipher fields it arms.
#[test]
fn netclient_send() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let store = r.e.alloc(0x20);
    let mut g = Lcg(0x5e4d_0001);
    for case in 0..800u32 {
        let st = state(&mut g);
        let armed = case % 2 == 1;
        let size = [0u32, 1, 6, 0x400][(case / 2 % 4) as usize];
        let read = if size == 0 {
            0
        } else {
            (g.next() >> 8) % (size + 1)
        };
        r.put(&st);
        r.e.write(r.conn + 0x120, &[0xa5; 0x10]);
        r.e.write(r.conn + 0x120, &[u8::from(armed)]);
        for (i, v) in [0x7ff9e4u32, 0x1000, 0, size, size, read]
            .into_iter()
            .enumerate()
        {
            r.e.write_u32(store + 4 * i as u64, v);
        }
        let global = g.next();
        r.e.write_u32(SENT, global);
        r.e.call(SEND, r.nc as u32, 0, &[store as u32]);
        let fx = net_send(st.state, size - read, armed);
        let ctx = format!("case {case} state {} pending {}", st.state, size - read);
        let n = if fx.sent { size - read } else { 0 };
        assert_eq!(r.e.read_u32(r.sent + 12), u32::from(fx.sent), "{ctx}: send");
        if fx.sent {
            assert_eq!(r.e.read_u32(r.sent + 16), 0, "{ctx}: second argument");
        }
        assert_eq!(
            r.e.read_u32(SENT),
            global.wrapping_add(n),
            "{ctx}: global count"
        );
        assert_eq!(r.get().sent, st.sent.wrapping_add(n), "{ctx}: sent count");
        let cipher = r.e.read(r.conn + 0x120, 8);
        let key = r.e.read_u32(r.conn + 0x128);
        if fx.arm {
            assert_eq!(cipher, vec![1, 0, 0, 4, 0, 0, 2, 0x28], "{ctx}: armed");
            assert_eq!(key, r.nc as u32 + 0x48, "{ctx}: key");
        } else {
            let mut want = vec![0xa5; 8];
            want[0] = u8::from(armed);
            assert_eq!(cipher, want, "{ctx}: untouched");
        }
    }
}

/// `NetClient::DispatchMessage` on packets of two or more bytes, opcodes across the tables, with
/// and without a handler: the slot 16 call, the handler's context, opcode, time and store, its
/// return value, the dispatch count, and the reset of a store with no handler.
#[test]
fn netclient_dispatch() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let store = r.e.alloc(0x20);
    let buf = r.e.alloc(0x20);
    let seen = r.e.alloc(0x20);
    // A handler that records ecx, edx and its two stack arguments and returns the dword at
    // seen+8: mov [seen],ecx; mov [seen+4],edx; mov eax,[esp+4]; mov [seen+12],eax;
    // mov eax,[esp+8]; mov [seen+16],eax; mov eax,[seen+8]; ret 8
    let handler = r.e.alloc(0x40);
    let s = seen as u32;
    let mut code = vec![0x89, 0x0d];
    code.extend_from_slice(&s.to_le_bytes());
    code.extend_from_slice(&[0x89, 0x15]);
    code.extend_from_slice(&(s + 4).to_le_bytes());
    code.extend_from_slice(&[0x8b, 0x44, 0x24, 0x04, 0xa3]);
    code.extend_from_slice(&(s + 12).to_le_bytes());
    code.extend_from_slice(&[0x8b, 0x44, 0x24, 0x08, 0xa3]);
    code.extend_from_slice(&(s + 16).to_le_bytes());
    code.push(0xa1);
    code.extend_from_slice(&(s + 8).to_le_bytes());
    code.extend_from_slice(&[0xc2, 0x08, 0x00]);
    r.e.write(handler, &code);
    let mut g = Lcg(0xd15a_0001);
    for case in 0..3000u32 {
        let op = match case % 5 {
            0 => 0,
            1 => 0x33b,
            _ => (g.next() >> 16) % 0x33c,
        };
        let has = g.next() >> 30 != 0;
        let ctxv = g.next();
        let ret = g.next();
        let time = g.next();
        let tail = (g.next() >> 28) as usize;
        let mut packet = (op as u16).to_le_bytes().to_vec();
        packet.extend((0..tail).map(|_| (g.next() >> 16) as u8));
        r.e.write(buf, &packet);
        let len = packet.len() as u32;
        for (i, v) in [0x7ff9e4u32, buf as u32, 0, len, len, 0]
            .into_iter()
            .enumerate()
        {
            r.e.write_u32(store + 4 * i as u64, v);
        }
        r.e.write(r.nc + 0x74, &vec![0u8; 0x33c * 4]);
        r.e.write_u32(
            r.nc + 0x74 + 4 * u64::from(op),
            if has { handler as u32 } else { 0 },
        );
        r.e.write_u32(r.nc + 0xd64 + 4 * u64::from(op), ctxv);
        r.e.write(seen, &[0u8; 0x14]);
        r.e.write_u32(seen + 8, ret);
        let count = g.next();
        r.e.write_u32(DISPATCHED, count);
        r.e.clear_calls();
        let got = r.e.call(DISPATCH, r.nc as u32, 0, &[time, store as u32]);
        let want_op = u32::from(dispatch_opcode(&packet));
        let ctx = format!("case {case} op {op:#x} handler {has}");
        assert_eq!(
            r.e.read_u32(DISPATCHED),
            count.wrapping_add(1),
            "{ctx}: count"
        );
        let hook: Vec<u32> =
            r.e.calls()
                .iter()
                .filter(|c| c.addr == r.slot16)
                .map(|c| c.args[0])
                .collect();
        assert_eq!(hook, vec![want_op], "{ctx}: slot 16");
        assert_eq!(
            r.e.read_u32(store + 0x14),
            if has { 2 } else { 0xffff_ffff },
            "{ctx}: cursor"
        );
        if has {
            assert_eq!(got, ret, "{ctx}: result");
            let regs = (r.e.read_u32(seen), r.e.read_u32(seen + 4));
            assert_eq!(regs, (ctxv, want_op), "{ctx}: ecx, edx");
            let args = (r.e.read_u32(seen + 12), r.e.read_u32(seen + 16));
            assert_eq!(args, (time, store as u32), "{ctx}: time, store");
        } else {
            assert_eq!(r.e.read_u32(store + 0x10), 0, "{ctx}: reset size");
        }
    }
}

/// `NetClient::HandleData` in and out of state 6 with lengths 0, 1 and more: the received count
/// (the length plus 2), the store it hands `NetClient::DispatchMessage` (stubbed), borrowing the
/// data with a window of `0xffffffff`, the free of a buffer a handler left the store owning, and
/// its result 1.
#[test]
fn netclient_handle_data() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let seen = r.e.alloc(0x20);
    // mov eax,[esp+8]; copy its six dwords to seen; ret 8
    let mut code = vec![0x8b, 0x44, 0x24, 0x08];
    for i in 0..6u8 {
        code.extend_from_slice(&[0x8b, 0x50, 4 * i, 0x89, 0x15]);
        code.extend_from_slice(&(seen as u32 + 4 * u32::from(i)).to_le_bytes());
    }
    code.extend_from_slice(&[0x8b, 0x54, 0x24, 0x04, 0x89, 0x15]);
    code.extend_from_slice(&(seen as u32 + 24).to_le_bytes());
    // mov edx,[seen+28]; mov [eax+0xc],edx: a handler that leaves the store owning a buffer.
    code.extend_from_slice(&[0x8b, 0x15]);
    code.extend_from_slice(&(seen as u32 + 28).to_le_bytes());
    code.extend_from_slice(&[0x89, 0x50, 0x0c]);
    code.extend_from_slice(&[0xc2, 0x08, 0x00]);
    let island = r.e.alloc(0x80);
    r.e.write(island, &code);
    let rel = (island as u32).wrapping_sub(DISPATCH as u32 + 5);
    let mut jmp = vec![0xe9];
    jmp.extend_from_slice(&rel.to_le_bytes());
    r.e.write(DISPATCH, &jmp);
    let mut g = Lcg(0xda7a_0001);
    for case in 0..600u32 {
        let st = state(&mut g);
        r.put(&st);
        let len = [0u32, 1, 2, 0x10, 0xffff, 0xffff_fffe][(case % 6) as usize];
        let data = g.next() | 1;
        let time = g.next();
        let global = g.next();
        r.e.write_u32(RECEIVED, global);
        r.e.write(seen, &[0u8; 0x20]);
        let owns = case % 4 == 3;
        r.e.write_u32(seen + 28, if owns { 0x100 } else { 0xffff_ffff });
        r.e.clear_calls();
        let got = r.e.call(HANDLE_DATA, r.nc as u32, 0, &[time, data, len]);
        let freed: Vec<u32> =
            r.e.calls()
                .iter()
                .filter(|c| c.addr == 0x646430)
                .map(|c| c.args[0])
                .collect();
        let want_freed = if st.state == 6 && owns {
            vec![data]
        } else {
            vec![]
        };
        let ctx = format!("case {case} state {} len {len:#x}", st.state);
        assert_eq!(got, 1, "{ctx}: result");
        assert_eq!(
            freed, want_freed,
            "{ctx}: a buffer the store came to own is freed"
        );
        assert_eq!(
            r.e.read_u32(RECEIVED),
            global.wrapping_add(len).wrapping_add(2),
            "{ctx}: count"
        );
        let fields: Vec<u32> = (0..7).map(|i| r.e.read_u32(seen + 4 * i)).collect();
        if st.state == 6 {
            assert_eq!(
                fields,
                vec![0x7ff9e4, data, 0, 0xffff_ffff, len, 0, time],
                "{ctx}: store"
            );
        } else {
            assert_eq!(fields, vec![0; 7], "{ctx}: no dispatch");
        }
    }
}

const INITIALIZE: u64 = 0x537600;
const DEINITIALIZE: u64 = 0x5376f0;
const ADD_EVENT: u64 = 0x538790;
/// The count of initialized clients.
const CLIENTS: u64 = 0xc0d4b4;

/// Writes `code` at a fresh scratch address and points `at` there with a five-byte `jmp`.
fn redirect(e: &mut Emu, at: u64, code: &[u8]) -> u64 {
    let island = e.alloc(0x40);
    e.write(island, code);
    let rel = (island as u32).wrapping_sub(at as u32 + 5);
    let mut jmp = vec![0xe9];
    jmp.extend_from_slice(&rel.to_le_bytes());
    e.write(at, &jmp);
    island
}

/// `NetClient::Initialize` for the first client and later ones, the platform starting or not:
/// the result, the count, the allocations of the queue and the connection, the objects built in
/// them, both tables zeroed, the state and the start time.
#[test]
fn netclient_initialize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let platform = r.e.alloc(0x10);
    // 0x5b7080 returns [platform]; the queue's and the connection's constructors return `this`.
    let mut code = vec![0xa1];
    code.extend_from_slice(&(platform as u32).to_le_bytes());
    code.extend_from_slice(&[0xc2, 0x08, 0x00]);
    redirect(&mut r.e, 0x5b7080, &code);
    let queue_ctor = r.e.alloc(0x10);
    let conn_ctor = r.e.alloc(0x10);
    r.e.write(queue_ctor, &[0x8b, 0xc1, 0xc2, 0x04, 0x00]);
    r.e.write(conn_ctor, &[0x8b, 0xc1, 0xc2, 0x08, 0x00]);
    let jmp = |e: &mut Emu, from: u64, to: u64| {
        let rel = (to as u32).wrapping_sub(from as u32 + 5);
        let mut j = vec![0xe9];
        j.extend_from_slice(&rel.to_le_bytes());
        e.write(from, &j);
    };
    jmp(&mut r.e, 0x538630, queue_ctor);
    jmp(&mut r.e, 0x5b4f10, conn_ctor);
    r.e.stub(0x41db60, 0x1234_5678, 0);
    let mut g = Lcg(0x1417_0001);
    for case in 0..64u32 {
        let count = [0u32, 0, 1, 7][(case % 4) as usize];
        let ok = case % 8 < 4;
        r.e.write_u32(CLIENTS, count);
        r.e.write_u32(platform, u32::from(ok) * (g.next() | 1));
        r.e.write(r.nc + 0x74, &vec![0xa5u8; 0x33c * 4]);
        r.e.write(r.nc + 0xd64, &vec![0x5au8; 0x33c * 4]);
        r.e.write_u32(r.nc + 0x70, 0xdead);
        let now = g.next();
        r.e.write_u32(r.clock, now);
        r.e.clear_calls();
        let got = r.e.call(INITIALIZE, r.nc as u32, 0, &[]);
        let fx = net_initialize(count, ok);
        let ctx = format!("case {case} count {count} ok {ok}");
        assert_eq!(got, fx.result, "{ctx}: result");
        assert_eq!(r.e.read_u32(CLIENTS), fx.count, "{ctx}: count");
        let allocs: Vec<(u32, u32)> =
            r.e.calls()
                .iter()
                .filter(|c| c.addr == 0x6462e0)
                .map(|c| (c.args[0], c.args[2]))
                .collect();
        if fx.result == 0 {
            assert!(allocs.is_empty(), "{ctx}: nothing allocated");
            assert_eq!(r.e.read_u32(r.nc + 0x70), 0xdead, "{ctx}: state kept");
            continue;
        }
        if fx.starts_platform {
            assert_eq!(r.e.read_u32(0xc0d454), 0x1234_5678, "{ctx}: context");
        }
        assert_eq!(
            allocs,
            vec![(0x28, 0x84), (0x130, 0x88)],
            "{ctx}: allocations"
        );
        let queue = r.e.read_u32(r.nc + 0x1a54);
        let conn = r.e.read_u32(r.nc + 0x1a58);
        assert!(queue != 0 && conn != 0 && queue != conn, "{ctx}: objects");
        assert!(
            r.e.read(r.nc + 0x74, 0x33c * 8).iter().all(|&b| b == 0),
            "{ctx}: tables"
        );
        assert_eq!(r.e.read_u32(r.nc + 0x70), 2, "{ctx}: state");
        assert_eq!(r.e.read_u32(0xc0d474), now, "{ctx}: start time");
    }
}

/// `NetClient::Deinitialize` in state 0 and in others, for the last client and with others left:
/// the calls that tear the connection and the queue down, both tables, the count, the platform
/// stop and the tail call.
#[test]
fn netclient_deinitialize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    for f in [0x537910u64, 0x5b5200, 0x538660, 0x5b7140, 0x5377a0] {
        r.e.stub(f, 0, 0);
    }
    r.e.stub(0x5b71e0, 0, 2);
    r.e.stub(0x429d50, 0, 0);
    let mut g = Lcg(0xde14_0001);
    for case in 0..64u32 {
        let state = [0u32, 6, 2, 1][(case % 4) as usize];
        let count = [1u32, 2, 5, 1][(case / 4 % 4) as usize];
        let queue = if case % 3 == 0 { 0 } else { g.next() | 0x10 };
        r.e.write_u32(CLIENTS, count);
        r.e.write_u32(r.nc + 0x70, state);
        r.e.write_u32(r.nc + 0x1a54, queue);
        r.e.write(r.nc + 0x74, &vec![0xa5u8; 0x33c * 8]);
        r.e.clear_calls();
        r.e.call(DEINITIALIZE, r.nc as u32, 0, &[]);
        let fx = net_deinitialize(state, count);
        let ctx = format!("case {case} state {state} count {count}");
        let calls: Vec<u64> = r.e.calls().iter().map(|c| c.addr).collect();
        assert_eq!(r.e.read_u32(CLIENTS), fx.count, "{ctx}: count");
        if !fx.runs {
            assert!(calls.is_empty(), "{ctx}: nothing");
            assert_eq!(r.e.read_u32(r.nc + 0x70), 0, "{ctx}: state");
            continue;
        }
        let mut want = vec![0x537910, 0x5b71e0, 0x5b5200];
        if queue != 0 {
            want.extend([0x538660, 0x646430]);
        }
        if fx.stops_platform {
            want.extend([0x429d50, 0x5b7140]);
        }
        if fx.tail {
            want.push(0x5377a0);
        }
        assert_eq!(calls, want, "{ctx}: calls");
        assert!(
            r.e.read(r.nc + 0x74, 0x33c * 8).iter().all(|&b| b == 0),
            "{ctx}: tables"
        );
        assert_eq!(
            (
                r.e.read_u32(r.nc + 0x70),
                r.e.read_u32(r.nc + 0x1a54),
                r.e.read_u32(r.nc + 0x1a58)
            ),
            (0, 0, 0),
            "{ctx}: state, queue, connection"
        );
    }
}

/// `NETEVENTQUEUE::AddEvent` with the five event types and lengths 0 to 300: the node's links,
/// type, time, length and copied data, the queue's last link, and the client's reference count.
#[test]
fn netclient_add_event() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let queue = r.e.alloc(0x40);
    let tail = r.e.alloc(0x10);
    let data = r.e.alloc(0x200);
    let mut g = Lcg(0xe7e7_0001);
    for case in 0..500u32 {
        let kind = 0x12 + case % 5;
        let len = match case % 7 {
            0 => 0,
            1 => 1,
            2 => 300,
            _ => (g.next() >> 16) % 64,
        };
        let bytes: Vec<u8> = (0..len).map(|_| (g.next() >> 16) as u8).collect();
        r.e.write(data, &bytes);
        let tail_next = g.next();
        r.e.write_u32(queue + 0x20, tail as u32);
        r.e.write_u32(tail + 4, tail_next);
        let refs = g.next();
        r.e.write_u32(r.nc + 0x1a5c, refs);
        let now = g.next();
        r.e.write_u32(r.clock, now);
        r.e.clear_calls();
        r.e.call(
            ADD_EVENT,
            queue as u32,
            0,
            &[kind, g.next(), r.nc as u32, data as u32, len],
        );
        let want = queue_event(tail as u32, tail_next, kind, len, now);
        let ctx = format!("case {case} type {kind:#x} len {len}");
        let node = r.e.read_u32(queue + 0x20);
        let allocs: Vec<(u32, u32)> =
            r.e.calls()
                .iter()
                .filter(|c| c.addr == 0x6462e0)
                .map(|c| (c.args[0], c.args[3]))
                .collect();
        let mut want_allocs = vec![(0x18, 8)];
        if len != 0 {
            want_allocs.push((len, 0));
        }
        assert_eq!(allocs, want_allocs, "{ctx}: allocations");
        assert_eq!(r.e.read_u32(tail + 4), node, "{ctx}: tail link");
        let f = |o: u64| r.e.read_u32(u64::from(node) + o);
        assert_eq!((f(0), f(4)), want.links, "{ctx}: links");
        assert_eq!(
            (f(8), f(0xc), f(0x14)),
            (want.kind, want.time, want.len),
            "{ctx}: fields"
        );
        if len == 0 {
            assert_eq!(f(0x10), 0, "{ctx}: no data");
        } else {
            assert_eq!(
                r.e.read(u64::from(f(0x10)), len as usize),
                bytes,
                "{ctx}: data"
            );
        }
        assert_eq!(
            r.e.read_u32(r.nc + 0x1a5c),
            refs.wrapping_add(1),
            "{ctx}: reference"
        );
    }
}
