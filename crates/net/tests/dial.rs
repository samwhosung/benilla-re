//! The dial addresses (`systems/net/dial.md`): `_atol`, the resolver thread and its digit-led
//! piece, `NetClient::Connect` and `WowConnection::ConnectToHost`, each run whole beside the
//! models of `dial.rs`, and the size-header writes of `WowConnection::Send`.
//!
//! `inet_addr`, `gethostbyname`, `htons`, the event calls and the allocator are the platform's or
//! Storm's and are replaced where the exe imports or calls them. The stand-in for `inet_addr`
//! reads a dotted quad of four decimal parts and gives `0xffffffff` for any other text, the
//! platform's failure value; the model and the exe both call it and both record the text they
//! pass, so the texts are compared as well as the results.

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use benilla_re_difftest::{Emu, RegisterX86};
use benilla_re_net::{
    atol, digit_piece, header_encode, host_address, resolve_list, world_dial, Hostent,
};
use common::{Lcg, Wow};

const ATOL: u64 = 0x414411;
const ATOI: u64 = 0x41449c;
const DIGIT_PIECE: u64 = 0x43f3d0;
const RESOLVE_THREAD: u64 = 0x43b890;
const NETCLIENT_CONNECT: u64 = 0x537820;
const CONNECT_TO_HOST: u64 = 0x5b6e40;

/// The import slots the code calls through (WSOCK32 ordinals 10, 52 and 9; KERNEL32's
/// `WaitForSingleObject` and `SetEvent`).
const IMP_INET_ADDR: u64 = 0x7ff718;
const IMP_GETHOSTBYNAME: u64 = 0x7ff6d4;
const IMP_HTONS: u64 = 0x7ff6e8;
const IMP_WAIT: u64 = 0x7ff1c0;
const IMP_SET_EVENT: u64 = 0x7ff1c4;

/// Handles the resolver test hands the thread.
const LOCK: u32 = 0x11;
const WAKE: u32 = 0x22;
const STARTED: u32 = 0x33;

/// The stand-in for `inet_addr`: four decimal parts of at most three digits, each up to 255, as
/// the bytes of the address in memory order; any other text gives `0xffffffff`.
fn inet_addr_standin(text: &[u8]) -> u32 {
    let parts: Vec<&[u8]> = text.split(|&b| b == b'.').collect();
    if parts.len() != 4 {
        return 0xffff_ffff;
    }
    let mut bytes = [0u8; 4];
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() || p.len() > 3 || !p.iter().all(u8::is_ascii_digit) {
            return 0xffff_ffff;
        }
        let v: u32 = p.iter().fold(0, |a, &d| a * 10 + u32::from(d - b'0'));
        if v > 255 {
            return 0xffff_ffff;
        }
        bytes[i] = v as u8;
    }
    u32::from_le_bytes(bytes)
}

/// Texts recorded by a stand-in, in call order.
type Seen = Rc<RefCell<Vec<Vec<u8>>>>;

/// Points the `inet_addr` import at a stand-in that records its text.
fn install_inet_addr(e: &mut Emu, seen: &Seen) {
    let island = e.alloc(0x10);
    let seen = seen.clone();
    e.stub_fn(island, 1, move |io| {
        let text = io.read_cstr(u64::from(io.arg(0)));
        let a = inet_addr_standin(&text);
        seen.borrow_mut().push(text);
        a
    });
    e.write_u32(IMP_INET_ADDR, island as u32);
}

/// Points the `htons` import at `movzx eax,word [esp+4]; xchg al,ah; ret 4`.
fn install_htons(e: &mut Emu) {
    let island = e.alloc(0x10);
    e.write(
        island,
        &[0x0f, 0xb7, 0x44, 0x24, 0x04, 0x86, 0xc4, 0xc2, 0x04, 0x00],
    );
    e.write_u32(IMP_HTONS, island as u32);
}

/// The names `gethostbyname` knows.
type Hosts = Rc<RefCell<Vec<(Vec<u8>, Hostent)>>>;

fn lookup(hosts: &Hosts, name: &[u8]) -> Option<Hostent> {
    hosts
        .borrow()
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, h)| h.clone())
}

/// Points the `gethostbyname` import at a stand-in that records the name and builds the
/// hostent in `buf` (0x400 bytes), as the platform's call does in a per-thread buffer: type at
/// `+8`, length at `+0xa`, the list at `+0xc`, each address four bytes.
fn install_gethostbyname(e: &mut Emu, hosts: &Hosts, seen: &Seen) {
    let island = e.alloc(0x10);
    let buf = e.alloc(0x400);
    let (hosts, seen) = (hosts.clone(), seen.clone());
    e.stub_fn(island, 1, move |io| {
        let name = io.read_cstr(u64::from(io.arg(0)));
        let found = lookup(&hosts, &name);
        seen.borrow_mut().push(name);
        let Some(h) = found else {
            return 0;
        };
        let (list, addrs) = (buf + 0x20, buf + 0x200);
        io.write(buf, &[0u8; 0x20]);
        io.write(buf + 8, &h.addr_type.to_le_bytes());
        io.write(buf + 0xa, &h.length.to_le_bytes());
        io.write_u32(buf + 0xc, list as u32);
        for (i, a) in h.addrs.iter().enumerate() {
            io.write_u32(list + 4 * i as u64, (addrs + 4 * i as u64) as u32);
            io.write_u32(addrs + 4 * i as u64, *a);
        }
        io.write_u32(list + 4 * h.addrs.len() as u64, 0);
        buf as u32
    });
    e.write_u32(IMP_GETHOSTBYNAME, island as u32);
}

/// Every byte alone, with a digit after it, with a sign and a digit, and with white space,
/// a sign and digits; strings drawn from an alphabet of white space, signs, digits, letters and
/// high bytes; digit runs across the 32-bit wrap.
#[test]
fn crt_atol() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let at = e.alloc(0x400);
    let check = |e: &mut Emu, s: &[u8], f: u64| {
        let mut m = s.to_vec();
        m.push(0);
        m.extend_from_slice(b"789 -1");
        e.write(at, &m);
        let got = e.call(f, 0, 0, &[at as u32]) as i32;
        assert_eq!(got, atol(s), "{f:#x} on {s:02x?}");
    };
    for b in 1..=255u8 {
        for tail in [&b""[..], b"7", b"-7", b"+7", b" 7", b"7x", b"12345"] {
            let mut s = vec![b];
            s.extend_from_slice(tail);
            check(&mut e, &s, ATOL);
            let mut t = b" \t".to_vec();
            t.push(b);
            t.extend_from_slice(tail);
            check(&mut e, &t, ATOL);
        }
    }
    check(&mut e, b"", ATOL);
    for s in [
        &b"2147483647"[..],
        b"2147483648",
        b"-2147483648",
        b"-2147483649",
        b"4294967295",
        b"4294967296",
        b"4294967297",
        b"-4294967296",
        b"99999999999999999999",
        b"-99999999999999999999",
        b"+0",
        b"-0",
        b"+",
        b"-",
        b"--5",
        b"+-5",
        b"-+5",
        b"- 5",
        b"\t\n\x0b\x0c\r 42",
        b"\x1f42",
        b"\xa042",
        b"0x10",
        b"1e5",
        b"9876543210123",
    ] {
        check(&mut e, s, ATOL);
        check(&mut e, s, ATOI);
    }
    let alphabet: &[u8] = b" \t\n\x0b\x0c\r+-0123456789abcxX.\x1f\x21\x80\xa0\xff";
    let mut g = Lcg(0xa701_0001);
    for _ in 0..6000 {
        let len = (g.next() >> 24) % 18;
        let s: Vec<u8> = (0..len)
            .map(|_| alphabet[(g.next() >> 16) as usize % alphabet.len()])
            .collect();
        check(&mut e, &s, ATOL);
        check(&mut e, &s, ATOI);
    }
}

/// The header bytes `WowConnection::Send` writes in front of a message that has room before its
/// data, `0x5b56da` to `0x5b572e`, for the lengths of `header_encode`'s test.
#[test]
fn header_send_inplace() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let node = e.alloc(0x40);
    let data = e.alloc(0x40) + 0x10;
    for n in header_lengths() {
        e.write(data - 8, &[0xaa; 8]);
        e.write_u32(frame + 0x20 - 8, n);
        e.write_u32(frame + 0x20 - 0xc, data as u32);
        e.set_reg(RegisterX86::EAX, u64::from(n));
        e.set_reg(RegisterX86::ESI, node);
        e.set_reg(RegisterX86::EDI, 3);
        e.set_reg(RegisterX86::EBP, frame + 0x20);
        e.run(0x5b56da, 0x5b572e);
        let len = e.get_reg(RegisterX86::ECX);
        let want = header_encode(n);
        assert_eq!(len as usize, want.len(), "n {n:#x}: header length");
        assert_eq!(
            u64::from(e.read_u32(node + 8)),
            data - len,
            "n {n:#x}: data pointer"
        );
        assert_eq!(e.read(data - len, len as usize), want, "n {n:#x}: bytes");
        assert_eq!(
            e.read(data - 8, 8 - len as usize),
            vec![0xaa; 8 - len as usize]
        );
    }
}

/// The same header writes in the copy `Send` makes for a message without room, `0x5b577a` to
/// `0x5b57d0`, with and without a source buffer: the header's place is the node's data pointer.
#[test]
fn header_send_node() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let node = e.alloc(0x80);
    for n in header_lengths() {
        for src in [0u64, 0x1000] {
            e.write(node, &[0xaa; 0x30]);
            let base = node + 0x20;
            if src != 0 {
                e.write_u32(node + 8, base as u32);
            }
            e.set_reg(RegisterX86::EAX, node);
            e.set_reg(RegisterX86::ECX, u64::from(n));
            e.set_reg(RegisterX86::EDX, base);
            e.set_reg(RegisterX86::ESI, src);
            e.set_reg(RegisterX86::EDI, 3);
            e.set_reg(RegisterX86::EBP, frame + 0x20);
            e.run(0x5b577a, 0x5b57d0);
            let want = header_encode(n);
            let len = e.read_u32(frame + 0x20 - 0x10) as usize;
            assert_eq!(len, want.len(), "n {n:#x} src {src:#x}: header length");
            let at = u64::from(e.read_u32(node + 8));
            let place = if src != 0 { base } else { base - len as u64 };
            assert_eq!(at, place, "n {n:#x} src {src:#x}: data pointer");
            assert_eq!(e.read(at, len), want, "n {n:#x} src {src:#x}: bytes");
        }
    }
}

/// The lengths either side of each boundary of the header form, the 3-byte form's first byte
/// from 0 to 0x7f, 0x800000 and above, and the signed edge.
fn header_lengths() -> Vec<u32> {
    let mut v: Vec<u32> = vec![
        0,
        1,
        2,
        0x7f,
        0x80,
        0x100,
        0x7ffe,
        0x7fff,
        0x8000,
        0x8001,
        0xffff,
        0x1_0000,
        0x12_3456,
        0x7f_fffe,
        0x7f_ffff,
        0x80_0000,
        0x80_0001,
        0xff_ffff,
        0x100_0000,
        0x1ab_cdef,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
    ];
    for k in 0..0x100u32 {
        v.push((k << 16) | 0x1234);
    }
    let mut g = Lcg(0x4eaf_0001);
    for _ in 0..300 {
        v.push(g.next());
    }
    v
}

/// The text of the pieces the dial tests cut and parse: dotted quads with and without ports,
/// texts `inet_addr` cannot read, digit-led names, names with a colon, and white space or high
/// bytes first.
fn piece_pool() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = [
        &b"1.2.3.4"[..],
        b"10.0.0.1:3724",
        b"127.0.0.1:8085",
        b"255.255.255.255",
        b"0.0.0.0",
        b"1.2.3",
        b"1abc",
        b"9",
        b"9.9.9.9:",
        b"9.9.9.9:abc",
        b"9.9.9.9: 77",
        b"9.9.9.9:+77",
        b"9.9.9.9:-1",
        b"1.2.3.4:99999",
        b"1.2.3.4:65536",
        b"1.2.3.4:4294967297",
        b"1.2.3.4:5:6",
        b"1.2.3.4 :5",
        b":5",
        b"0:80",
        b"3com.test",
        b"7.test:80",
        b"logon.test",
        b"alias.test",
        b"nowhere.test",
        b"six.test",
        b"badlen.test",
        b"empty.test",
        b"name:3724",
        b" lead",
        b"\x80high",
        b"\xffname",
        b"/path",
        b".dot",
        b"us.logon.worldofwarcraft.com:3724",
    ]
    .iter()
    .map(|p| p.to_vec())
    .collect();
    // A prefix before the colon of 30, 31, 32 and 40 characters, and long pieces.
    for n in [29usize, 30, 31, 32, 33, 40] {
        let mut p = vec![b'1'; n];
        p.extend_from_slice(b":65");
        v.push(p);
    }
    for n in [1022usize, 1023, 1024, 1500] {
        v.push(vec![b'5'; n]);
        let mut p = b"host".to_vec();
        p.resize(n, b'z');
        v.push(p);
        let mut p = b"4.3.2.1:".to_vec();
        p.resize(n, b'8');
        v.push(p);
    }
    v
}

/// The host table of the dial tests.
fn host_table() -> Vec<(Vec<u8>, Hostent)> {
    let h = |t, l, a: &[u32]| Hostent {
        addr_type: t,
        length: l,
        addrs: a.to_vec(),
    };
    vec![
        (
            b"logon.test".to_vec(),
            h(2, 4, &[0x0403_0201, 0x0807_0605, 0x0100_007f]),
        ),
        (b"alias.test".to_vec(), h(2, 4, &[0xdead_beef])),
        (b"six.test".to_vec(), h(23, 16, &[1, 2])),
        (b"badlen.test".to_vec(), h(2, 8, &[3])),
        (b"empty.test".to_vec(), h(2, 4, &[])),
        (b"3com.test".to_vec(), h(2, 4, &[0x1111_1111])),
        (b"host".to_vec(), h(2, 4, &[0x0a0b_0c0d])),
    ]
}

/// The digit-led piece, `0x43f3d0`, whole: the text before the colon and the port after it, the
/// 31-byte cap, the request's port when there is no colon, the unchecked `inet_addr` result.
#[test]
fn dial_digit_piece() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let seen: Seen = Rc::default();
    install_inet_addr(&mut e, &seen);
    install_htons(&mut e);
    let text = e.alloc(0x800);
    let out = e.alloc(0x20);
    let mut g = Lcg(0xd161_0001);
    for p in piece_pool() {
        if !p[0].is_ascii_digit() {
            continue;
        }
        for port in [0u32, 3724, 0xffff, 0x1_0000, 0xdead_0e8c, g.next()] {
            let mut m = p.clone();
            m.push(0);
            e.write(text, &m);
            e.write(out, &[0xaa; 0x20]);
            seen.borrow_mut().clear();
            e.call(DIGIT_PIECE, text as u32, port, &[out as u32]);
            let got = e.read(out, 16);
            let exe_seen = std::mem::take(&mut *seen.borrow_mut());
            let want = digit_piece(&p, port, |t| {
                seen.borrow_mut().push(t.to_vec());
                inet_addr_standin(t)
            });
            assert_eq!(got, want, "piece {p:02x?} port {port:#x}");
            assert_eq!(
                exe_seen,
                *seen.borrow(),
                "piece {p:02x?}: text to inet_addr"
            );
            assert_eq!(
                e.read(out + 16, 16),
                vec![0xaa; 16],
                "piece {p:02x?}: extent"
            );
            seen.borrow_mut().clear();
        }
    }
}

/// The resolver thread, `0x43b890`, whole, over a request in its list: lists of the pieces above
/// joined by `;`, with a trailing `;`, an empty piece in the middle, and ports of every size. The
/// entries it appends, whether it marks the request done, the texts it gives `inet_addr` and
/// `gethostbyname`, and its use of the lock and the wake event.
#[test]
fn dial_resolve_thread() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let inet_seen: Seen = Rc::default();
    let name_seen: Seen = Rc::default();
    let hosts: Hosts = Rc::new(RefCell::new(host_table()));
    install_inet_addr(&mut e, &inet_seen);
    install_gethostbyname(&mut e, &hosts, &name_seen);
    install_htons(&mut e);
    let waits: Rc<RefCell<Vec<u32>>> = Rc::default();
    let events: Rc<RefCell<Vec<u32>>> = Rc::default();
    let (w, v) = (waits.clone(), events.clone());
    let island = e.alloc(0x10);
    e.stub_fn(island, 2, move |io| {
        w.borrow_mut().push(io.arg(0));
        0
    });
    e.write_u32(IMP_WAIT, island as u32);
    let island = e.alloc(0x10);
    e.stub_fn(island, 1, move |io| {
        v.borrow_mut().push(io.arg(0));
        1
    });
    e.write_u32(IMP_SET_EVENT, island as u32);
    let arena = e.alloc(0x100000);
    e.stub_alloc_sized(0x6462e0, arena, arena + 0x100000, 0, 4);
    e.stub_realloc_sized(0x646320, 5);
    e.stub(0x646430, 1, 4);

    let obj = e.alloc(0x700);
    let req = e.alloc(0x40);
    let param = e.alloc(0x10);
    let text = e.alloc(0x4000);
    let pool = piece_pool();
    let mut g = Lcg(0x7e50_0001);
    for case in 0..1500u32 {
        // The list: pieces drawn from the pool, joined by ';'.
        let n = match case % 7 {
            0 => 0,
            1 => 1,
            _ => 1 + (g.next() >> 20) as usize % 6,
        };
        let mut list: Vec<u8> = Vec::new();
        for i in 0..n {
            if i > 0 {
                list.push(b';');
            }
            match g.next() >> 29 {
                0 if n > 1 => {}
                _ => list.extend_from_slice(&pool[(g.next() >> 8) as usize % pool.len()]),
            }
        }
        if case % 5 == 0 {
            list.push(b';');
        }
        let port = match case % 4 {
            0 => 3724,
            1 => 0,
            2 => 0xffff,
            _ => (g.next() >> 16) as u16,
        };
        // The request and the thread's parameter block.
        e.write(obj + 0x600, &[0u8; 0x100]);
        e.write_u32(obj + 0x618, LOCK);
        e.write_u32(obj + 0x544, WAKE);
        e.write_u32(obj + 0x628, req as u32);
        e.write(req, &[0u8; 0x40]);
        e.write_u32(req + 4, 1);
        e.write_u32(req + 0xc, text as u32);
        e.write_u32(req + 0x10, 7);
        let mut m = list.clone();
        m.push(0);
        e.write(text, &m);
        e.write_u32(param, obj as u32);
        e.write_u32(param + 4, 7);
        e.write_u32(param + 8, STARTED);
        e.write_u32(param + 0xc, 0xabcd_0000 | u32::from(port));
        e.reset_bump(arena);
        for s in [&inet_seen, &name_seen] {
            s.borrow_mut().clear();
        }
        waits.borrow_mut().clear();
        events.borrow_mut().clear();
        e.call(RESOLVE_THREAD, 0, 0, &[param as u32]);

        let count = e.read_u32(req + 0x28) as usize;
        let data = u64::from(e.read_u32(req + 0x2c));
        let got: Vec<[u8; 16]> = (0..count)
            .map(|i| e.read(data + 16 * i as u64, 16).try_into().unwrap())
            .collect();
        let done = e.read_u32(req + 0x20) != 0;
        let exe_seen = (inet_seen.borrow().clone(), name_seen.borrow().clone());
        let (mi, mn) = (Seen::default(), Seen::default());
        let want = resolve_list(
            &list,
            port,
            |t| {
                mi.borrow_mut().push(t.to_vec());
                inet_addr_standin(t)
            },
            |t| {
                mn.borrow_mut().push(t.to_vec());
                lookup(&hosts, t)
            },
        );
        let ctx = format!(
            "case {case}: list {:?} port {port:#x}",
            String::from_utf8_lossy(&list)
        );
        assert_eq!(got, want.entries, "{ctx}: entries");
        assert_eq!(done, want.done, "{ctx}: done");
        assert_eq!(
            exe_seen,
            (mi.borrow().clone(), mn.borrow().clone()),
            "{ctx}: texts"
        );
        let ev = events.borrow();
        assert_eq!(ev[0], STARTED, "{ctx}: start signal");
        assert_eq!(
            waits.borrow().iter().filter(|&&h| h == LOCK).count(),
            ev.iter().filter(|&&h| h == LOCK).count(),
            "{ctx}: every wait on the lock is released"
        );
        assert_eq!(waits.borrow().iter().filter(|&&h| h != LOCK).count(), 0);
        if done {
            assert_eq!(ev[ev.len() - 2..], [LOCK, WAKE], "{ctx}: wake after done");
        } else {
            assert!(!ev.contains(&WAKE), "{ctx}: no wake when abandoned");
        }
    }
}

/// A recorded dial: the connection, the host text, the port dword and the timeout.
type Dial = (u32, Vec<u8>, u32, u32);

/// `NetClient::Connect`, `0x537820`, whole, with the cipher switch and the dial recorded: in
/// state 2 the host and the port dword the dial gets; in any other state the fatal error and no
/// dial.
#[test]
fn dial_world_connect() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let nc = e.alloc(0x1c00);
    let text = e.alloc(0x800);
    let dials: Rc<RefCell<Vec<Dial>>> = Rc::default();
    let d = dials.clone();
    e.stub_fn(CONNECT_TO_HOST, 3, move |io| {
        let host = io.read_cstr(u64::from(io.arg(0)));
        d.borrow_mut()
            .push((io.reg(RegisterX86::ECX) as u32, host, io.arg(1), io.arg(2)));
        1
    });
    let disarms: Rc<RefCell<Vec<(u32, u32)>>> = Rc::default();
    let d = disarms.clone();
    e.stub_fn(0x5b73c0, 1, move |io| {
        d.borrow_mut()
            .push((io.reg(RegisterX86::ECX) as u32, io.arg(0)));
        0
    });
    e.stub(0x64d0e0, 0, 0);
    let mut addresses: Vec<Vec<u8>> = [
        &b"host"[..],
        b"host:123",
        b"host:",
        b":",
        b":80",
        b"h:p:q",
        b"host: 77",
        b"host:+5",
        b"host:-1",
        b"host:99999",
        b"host:65536",
        b"host:4294967296",
        b"host:4294967297",
        b"host:12abc",
        b"1.2.3.4:8085",
        b"1.2.3.4",
        b"",
        b"a b:\t\n9",
    ]
    .iter()
    .map(|a| a.to_vec())
    .collect();
    for (len, colon) in [
        (1500usize, 1100usize),
        (1500, 1022),
        (1500, 1023),
        (1500, 1024),
        (1023, 1022),
    ] {
        let mut a = vec![b'x'; len];
        a[colon] = b':';
        a.extend_from_slice(b"55");
        addresses.push(a);
    }
    for address in addresses {
        for state in 0..9u32 {
            let mut m = address.clone();
            m.push(0);
            e.write(text, &m);
            e.write_u32(nc + 0x70, state);
            e.write_u32(nc + 0x1a58, 0x00c0_ffee);
            dials.borrow_mut().clear();
            disarms.borrow_mut().clear();
            e.clear_calls();
            let ret = e.call(NETCLIENT_CONNECT, nc as u32, 0, &[text as u32]);
            let ctx = format!(
                "address {:?} state {state}",
                String::from_utf8_lossy(&address)
            );
            if state != 2 {
                assert!(dials.borrow().is_empty(), "{ctx}: no dial");
                assert!(disarms.borrow().is_empty(), "{ctx}: cipher kept");
                assert_eq!(e.read_u32(nc + 0x70), state, "{ctx}: state kept");
                let fatal: Vec<_> = e.calls().iter().filter(|c| c.addr == 0x64d0e0).collect();
                assert_eq!(fatal.len(), 1, "{ctx}: fatal error");
                assert_eq!(fatal[0].args[1], state, "{ctx}: fatal error argument");
                continue;
            }
            let want = world_dial(&address);
            assert_eq!(ret, 1, "{ctx}: result");
            assert_eq!(e.read_u32(nc + 0x70), 5, "{ctx}: state");
            assert_eq!(
                *disarms.borrow(),
                vec![(0x00c0_ffee, 0)],
                "{ctx}: cipher off"
            );
            assert_eq!(
                *dials.borrow(),
                vec![(0x00c0_ffee, want.host, want.port, 0xffff_ffff)],
                "{ctx}: dial"
            );
            assert!(
                e.calls().iter().all(|c| c.addr != 0x64d0e0),
                "{ctx}: no error"
            );
        }
    }
}

/// `WowConnection::ConnectToHost`, `0x5b6e40`, whole: the address and the 16-bit port it stores
/// before connecting, for dotted addresses, `inet_addr`'s two failure values, names found with any
/// hostent type and length, and names not found.
#[test]
fn dial_connect_host() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let inet_seen: Seen = Rc::default();
    let name_seen: Seen = Rc::default();
    // Names that read as addresses, which the lookup serves when `inet_addr` gives 0 or -1.
    let quad = |t: &[u8], a| {
        (
            t.to_vec(),
            Hostent {
                addr_type: 2,
                length: 4,
                addrs: vec![a],
            },
        )
    };
    let mut table: Vec<_> = host_table()
        .into_iter()
        .filter(|(_, h)| !h.addrs.is_empty())
        .collect();
    table.push(quad(b"0.0.0.0", 0x0100_007f));
    table.push(quad(b"255.255.255.255", 0x0200_007f));
    let hosts: Hosts = Rc::new(RefCell::new(table));
    install_inet_addr(&mut e, &inet_seen);
    install_gethostbyname(&mut e, &hosts, &name_seen);
    e.stub(0x5b6c80, 0, 0);
    let conn = e.alloc(0x130);
    let text = e.alloc(0x800);
    let hosts_to_try: Vec<&[u8]> = vec![
        b"1.2.3.4",
        b"0.0.0.0",
        b"255.255.255.255",
        b"256.1.1.1",
        b"logon.test",
        b"alias.test",
        b"six.test",
        b"badlen.test",
        b"3com.test",
        b"host",
        b"nowhere.test",
        b"",
        b"1.2.3",
        b"0",
    ];
    for host in hosts_to_try {
        for port in [0u32, 1, 3724, 9090, 0xffff, 0x1_0000, 0x1_2345, 0xffff_ffff] {
            for timeout in [0xffff_ffffu32, 5000] {
                let mut m = host.to_vec();
                m.push(0);
                e.write(text, &m);
                e.write(conn + 0x60, &[0xa5; 0x10]);
                inet_seen.borrow_mut().clear();
                name_seen.borrow_mut().clear();
                e.clear_calls();
                let ret = e.call(
                    CONNECT_TO_HOST,
                    conn as u32,
                    0,
                    &[text as u32, port, timeout],
                );
                let (mi, mn) = (Seen::default(), Seen::default());
                let addr = host_address(
                    host,
                    |t| {
                        mi.borrow_mut().push(t.to_vec());
                        inet_addr_standin(t)
                    },
                    |t| {
                        mn.borrow_mut().push(t.to_vec());
                        lookup(&hosts, t).map(|h| h.addrs[0])
                    },
                );
                let ctx = format!("host {:?} port {port:#x}", String::from_utf8_lossy(host));
                assert_eq!(ret & 0xff, 1, "{ctx}: result");
                assert_eq!(e.read_u32(conn + 0x64), addr, "{ctx}: address");
                assert_eq!(
                    e.read(conn + 0x68, 2),
                    (port as u16).to_le_bytes(),
                    "{ctx}: port"
                );
                assert_eq!(e.read(conn + 0x6a, 6), vec![0xa5; 6], "{ctx}: neighbours");
                assert_eq!(e.read(conn + 0x60, 4), vec![0xa5; 4], "{ctx}: neighbours");
                assert_eq!(*inet_seen.borrow(), *mi.borrow(), "{ctx}: inet_addr text");
                assert_eq!(*name_seen.borrow(), *mn.borrow(), "{ctx}: lookup text");
                let connects = e.calls().iter().filter(|c| c.addr == 0x5b6c80).count();
                assert_eq!(connects, 1, "{ctx}: one connect");
            }
        }
    }
}
