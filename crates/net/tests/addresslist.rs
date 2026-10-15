//! The address list's helpers and the connection's callback bracket and socket bitmap
//! (`systems/net/threads.md`), each run whole beside the model.

mod common;

use benilla_re_difftest::Emu;
use benilla_re_net::{
    align_up, check_character_name, mark_socket_open, random_address_index, CallbackBracket,
};
use common::{Lcg, Wow};

const ALIGN_UP: u64 = 0x5bd4d0;
const CONNECT_RANDOM: u64 = 0x5bc2b0;
const MARK_SOCKET_OPEN: u64 = 0x5b6330;
const BEGIN_CALLBACK: u64 = 0x5b7260;
const END_CALLBACK: u64 = 0x5b7290;
const CHECK_CHARACTER_NAME: u64 = 0x5abd50;
const VALIDATE_CHARACTER_NAME: u64 = 0x6c9ae0;
const LOCALE: u64 = 0xc0e080;

/// Points `at` at `code` in scratch with a five-byte `jmp`.
fn redirect(e: &mut Emu, at: u64, code: &[u8]) {
    let island = e.alloc(0x40);
    e.write(island, code);
    let rel = (island as u32).wrapping_sub(at as u32 + 5);
    let mut jmp = vec![0xe9];
    jmp.extend_from_slice(&rel.to_le_bytes());
    e.write(at, &jmp);
}

/// `AlignUpToMultiple` over values at 0, at and beside multiples, and near the wrap, with
/// multiples 1, 2, 3, powers of two, 0x40 and large ones.
#[test]
fn address_align_up() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut g = Lcg(0xa119_0001);
    let mut ms = vec![
        1u32,
        2,
        3,
        4,
        7,
        0x40,
        0x1000,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
    ];
    ms.extend(
        (0..20)
            .map(|_| g.next() >> (g.next() >> 27))
            .filter(|&m| m != 0),
    );
    for &m in &ms {
        let mut vs = vec![
            0u32,
            1,
            m - 1,
            m,
            m.wrapping_add(1),
            0xffff_ffff,
            0xffff_fffe,
            0x8000_0000,
        ];
        vs.extend((0..20).map(|_| g.next()));
        for &v in &vs {
            let got = e.call(ALIGN_UP, 0, 0, &[v, m]);
            assert_eq!(got, align_up(v, m), "value {v:#x} multiple {m:#x}");
        }
    }
}

/// `ConnectToRandomAddress` over clocks across the range and counts 1 to 300: the entry whose
/// address and port it dials, and, for a null list or a count of 0, the callback's failure slot.
#[test]
fn address_random_pick() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let clock = e.alloc(0x10);
    // time(): mov eax,[clock]; ret
    let mut code = vec![0xa1];
    code.extend_from_slice(&(clock as u32).to_le_bytes());
    code.push(0xc3);
    redirect(&mut e, 0x73f049, &code);
    // ntohs, through the import slot: movzx eax,word [esp+4]; xchg al,ah; ret 4
    let ntohs = e.alloc(0x10);
    e.write(
        ntohs,
        &[0x0f, 0xb7, 0x44, 0x24, 0x04, 0x86, 0xc4, 0xc2, 0x04, 0x00],
    );
    e.write_u32(0x7ff71c, ntohs as u32);
    e.stub(0x5bc250, 0, 2);
    let callback = e.alloc(0x10);
    let vt = e.alloc(0x20);
    let fail = e.alloc(0x10);
    e.write_u32(callback, vt as u32);
    e.write_u32(vt + 0xc, fail as u32);
    e.stub(fail, 0, 3);
    let list = e.alloc(16 * 300);
    let table = e.read(0x802700, 0x100);
    let mut g = Lcg(0x4a4d_0001);
    let entries: Vec<[u8; 16]> = (0..300)
        .map(|_| std::array::from_fn(|_| (g.next() >> 16) as u8))
        .collect();
    let flat: Vec<u8> = entries.iter().flatten().copied().collect();
    e.write(list, &flat);
    for case in 0..3000u32 {
        let time = match case % 6 {
            0 => case,
            1 => 0xffff_ffff - case,
            _ => g.next(),
        };
        let count = match case % 5 {
            0 => 1,
            1 => 300,
            2 => 0,
            _ => 1 + (g.next() >> 16) % 300,
        };
        let null = case % 17 == 0;
        e.write_u32(clock, time);
        e.clear_calls();
        let at = if null { 0 } else { list as u32 };
        e.call(CONNECT_RANDOM, 0, 0, &[at, count, callback as u32]);
        let calls: Vec<_> = e.calls().iter().map(|c| (c.addr, c.args)).collect();
        let ctx = format!("case {case} time {time:#x} count {count} null {null}");
        assert_eq!(calls.len(), 1, "{ctx}: one call");
        if null || count == 0 {
            assert_eq!(
                (calls[0].0, &calls[0].1[..3]),
                (fail, &[0u32, 0, 0][..]),
                "{ctx}: failure"
            );
            continue;
        }
        let i = random_address_index(time, count, &table) as usize;
        let ent = &entries[i];
        let ip = u32::from_le_bytes(ent[4..8].try_into().unwrap());
        let port = u32::from(u16::from_be_bytes([ent[2], ent[3]]));
        assert_eq!(calls[0].0, 0x5bc250, "{ctx}: dial");
        assert_eq!(
            (calls[0].1[0], calls[0].1[1] & 0xffff),
            (ip, port),
            "{ctx}: entry {i}"
        );
    }
}

/// `MarkSocketOpen` for sockets 0 to 1023, negative ones and the sign boundary: the one bitmap
/// word it sets a bit in.
#[test]
fn connection_mark_socket() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(0x6579b0, 0, 0);
    e.stub(0x6579c0, 0, 0);
    let bitmap = e.alloc(0x100);
    e.write_u32(0xc2a378, bitmap as u32);
    let mut g = Lcg(0x50c4_0001);
    let mut fds: Vec<i32> = (0..1024).collect();
    fds.extend([-1, -2, i32::MIN, -0x7fff_fff0]);
    for &fd in &fds {
        let words: Vec<u32> = (0..32).map(|_| g.next()).collect();
        let flat: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        e.write(bitmap, &flat);
        e.call(MARK_SOCKET_OPEN, fd as u32, 0, &[]);
        let mut want = words.clone();
        if let Some((i, w)) = mark_socket_open(fd, words[(fd.max(0) as usize >> 5) % 32]) {
            want[i as usize] = w;
        }
        let got: Vec<u32> = (0..32).map(|i| e.read_u32(bitmap + 4 * i)).collect();
        assert_eq!(got, want, "socket {fd}");
    }
}

/// `WowConnection::BeginCallback` and `EndCallback` from depths 0, 1 and the wrap, with a held
/// value and without: the depth, the thread, the hold and `+0x14`.
#[test]
fn connection_callback_bracket() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(0x6579b0, 0, 0);
    e.stub(0x6579c0, 0, 0);
    const THREAD: u32 = 0x0000_0d1c;
    e.stub(0x64bbd0, THREAD, 0);
    let conn = e.alloc(0x140);
    let mut g = Lcg(0xb4ac_0001);
    for case in 0..400u32 {
        let mut b = CallbackBracket {
            depth: [0u32, 1, 2, 0xffff_ffff][(case % 4) as usize],
            thread: g.next(),
            deferred: if case % 3 == 0 { 0 } else { g.next() },
            field14: g.next(),
        };
        let put = |e: &mut Emu, b: &CallbackBracket| {
            e.write_u32(conn + 0xb4, b.depth);
            e.write_u32(conn + 0xb8, b.thread);
            e.write_u32(conn + 0xbc, b.deferred);
            e.write_u32(conn + 0x14, b.field14);
        };
        let get = |e: &Emu| CallbackBracket {
            depth: e.read_u32(conn + 0xb4),
            thread: e.read_u32(conn + 0xb8),
            deferred: e.read_u32(conn + 0xbc),
            field14: e.read_u32(conn + 0x14),
        };
        put(&mut e, &b);
        let begin = case % 2 == 0;
        e.call(
            if begin { BEGIN_CALLBACK } else { END_CALLBACK },
            conn as u32,
            0,
            &[],
        );
        if begin {
            b.begin(THREAD);
        } else {
            b.end();
        }
        assert_eq!(get(&e), b, "case {case}");
    }
}

/// `CheckCharacterName` whole, the validator replaced by code that logs its `ecx` and `edx` and
/// returns a chosen result: every result from 0 to 13, the top values and the locales 0 to 7, the
/// locale passed in `ecx` and the name in `edx`.
#[test]
fn connection_check_name() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let log = e.alloc(0x10);
    let l = log as u32;
    let mut c = vec![0xa1];
    c.extend(l.to_le_bytes());
    c.extend([0x89, 0x0d]);
    c.extend((l + 4).to_le_bytes());
    c.extend([0x89, 0x15]);
    c.extend((l + 8).to_le_bytes());
    c.push(0xc3);
    redirect(&mut e, VALIDATE_CHARACTER_NAME, &c);
    let mut g = Lcg(0xc4a2_0001);
    let results = (0u32..=13).chain([14, 0x7fff_ffba, 0x7fff_ffff, 0xffff_ffba, 0xffff_ffff]);
    for result in results {
        for locale in 0..8u32 {
            let name = g.next();
            e.write_u32(log, result);
            e.write_u32(LOCALE, locale);
            let got = e.call(CHECK_CHARACTER_NAME, name, 0, &[]);
            assert_eq!(got, check_character_name(result), "{result:#x}");
            assert_eq!(
                (e.read_u32(log + 4), e.read_u32(log + 8)),
                (locale, name),
                "the arguments"
            );
        }
    }
}
