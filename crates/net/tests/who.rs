//! The `/who` query packet (`systems/net/who.md#who-query-tags`), `FriendList::SendWhoQuery`
//! run whole beside the model on generated filters, tags and DBC rows.

mod common;

use benilla_re_difftest::Emu;
use benilla_re_net::{send_who_query, sstr_str_i, sstr_to_int, WhoRow, WhoTables, WhoTags};
use common::{Lcg, Wow};
use std::sync::atomic::{AtomicU32, Ordering};

/// The units `GetStringTypeW` was asked about: sign-extended high bytes (`U+FF80` to `U+FFFE`)
/// and the ones it calls space.
static ASKED_HIGH: AtomicU32 = AtomicU32::new(0);
static ASKED_SPACE: AtomicU32 = AtomicU32::new(0);

const SEND_WHO_QUERY: u64 = 0x5aebb0;
const SSTR_STR_I: u64 = 0x64b540;
const SSTR_TO_INT: u64 = 0x64ac60;
const GET_TEXT: u64 = 0x703bf0;
const SEND: u64 = 0x5ab630;
const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_REALLOC: u64 = 0x646320;
const SMEM_FREE: u64 = 0x646430;
const SERR_SET_LAST_ERROR: u64 = 0x64e850;
const LOCALE: u64 = 0xc0e080;
/// The pointer `___crtGetStringTypeW` (`0x413ed3`) calls `GetStringTypeW` through.
const GET_STRING_TYPE_W: u64 = 0x8824a0;
/// The tag names `FrameScript_GetText` is asked for: name, guild, zone, race, class.
const TAG_KEYS: [u32; 5] = [0x85dce0, 0x85dcd0, 0x85dcc0, 0x85dcb0, 0x85dca0];
/// Each table's record array pointer (its count follows), record size and name offset:
/// `AreaTable`, `ChrRaces`, `ChrClasses`.
const TABLES: [(u64, u64, u64); 3] = [
    (0xc0e040, 0x64, 0x2c),
    (0xc0ded8, 0x74, 0x44),
    (0xc0deec, 0x44, 0x14),
];
const ARENA: u64 = 0x20_0000;

fn jmp(e: &mut Emu, from: u64, to: u64) {
    let rel = (to as u32).wrapping_sub(from as u32 + 5);
    let mut c = vec![0xe9];
    c.extend(rel.to_le_bytes());
    e.write(from, &c);
}

/// A string pool in the emulator, refilled each case.
struct Pool {
    base: u64,
    at: u64,
}

impl Pool {
    fn put(&mut self, e: &mut Emu, s: &[u8]) -> u32 {
        let a = self.at;
        e.write(a, s);
        e.write(a + s.len() as u64, &[0]);
        self.at += (s.len() as u64 + 4) & !3;
        a as u32
    }
}

struct Rig<'a> {
    e: Emu<'a>,
    arena: u64,
    /// The tag strings, 16 bytes each, that the text island returns.
    tags: u64,
    /// The packet the send island copied: its size, then its bytes.
    sent: u64,
    pool: Pool,
}

impl<'a> Rig<'a> {
    fn new(wow: &'a Wow) -> Rig<'a> {
        let mut e = Emu::new(wow);
        let arena = e.alloc(ARENA);
        e.stub_alloc_sized(SMEM_ALLOC, arena + 0x1_0000, arena + ARENA, 0, 4);
        e.stub_realloc_sized(SMEM_REALLOC, 5);
        e.stub(SMEM_FREE, 1, 4);
        e.stub(SERR_SET_LAST_ERROR, 0, 1);
        // `iswctype` past `0xff` asks the system: page 0 holds `fs:0` for its exception frame,
        // and `GetStringTypeW(CT_CTYPE1, &unit, 1, &type)` answers with [`wide`].
        e.map(0, 0x1000);
        let api = e.alloc(0x10);
        e.stub_fn(api, 4, |c| {
            assert_eq!(
                (c.arg(0), c.arg(2)),
                (1, 1),
                "GetStringTypeW(CT_CTYPE1, _, 1, _)"
            );
            let unit = u16::from_le_bytes(c.read(u64::from(c.arg(1)), 2).try_into().unwrap());
            if (0xff80..0xffff).contains(&unit) {
                ASKED_HIGH.fetch_add(1, Ordering::Relaxed);
            }
            if wide(unit) & 8 != 0 {
                ASKED_SPACE.fetch_add(1, Ordering::Relaxed);
            }
            c.write(u64::from(c.arg(3)), &wide(unit).to_le_bytes());
            1
        });
        e.write_u32(GET_STRING_TYPE_W, api as u32);
        let tags = e.alloc(0x100);
        // `FrameScript_GetText(ecx = key)`: `cmp ecx,key; jne next; mov eax,text; ret 4`.
        let mut c = Vec::new();
        for (k, key) in TAG_KEYS.iter().enumerate() {
            c.extend([0x81, 0xf9]);
            c.extend(key.to_le_bytes());
            c.extend([0x75, 0x08, 0xb8]);
            c.extend((tags as u32 + 0x20 * k as u32).to_le_bytes());
            c.extend([0xc2, 0x04, 0x00]);
        }
        c.extend([0x31, 0xc0, 0xc2, 0x04, 0x00]);
        let island = e.alloc(0x100);
        e.write(island, &c);
        jmp(&mut e, GET_TEXT, island);
        // `ClientServices::Send(ecx = store)`: copies the store's bytes out.
        let sent = e.alloc(0x1000);
        let mut c = vec![0x56, 0x57, 0x8b, 0x71, 0x04, 0x8b, 0x41, 0x10, 0xa3];
        c.extend((sent as u32).to_le_bytes());
        c.push(0xbf);
        c.extend((sent as u32 + 4).to_le_bytes());
        c.extend([0x89, 0xc1, 0xf3, 0xa4, 0x5f, 0x5e, 0xc3]);
        let island = e.alloc(0x40);
        e.write(island, &c);
        jmp(&mut e, SEND, island);
        let base = e.alloc(0x4_0000);
        Rig {
            e,
            arena,
            tags,
            sent,
            pool: Pool { base, at: base },
        }
    }
}

/// `GetStringTypeW`'s `CT_CTYPE1` answer the stub and the model share: `C1_SPACE` for the
/// Ogham space mark, the en quad to the hair space and the ideographic space, and no digit.
fn wide(unit: u16) -> u16 {
    match unit {
        0x1680 | 0x2000..=0x200a | 0x3000 => 8,
        _ => 0,
    }
}

/// UTF-8 pieces: letters of two and three bytes, white space of three (`U+3000`, `U+2003`), an
/// overlong space (`C0 A0`), a four-byte code point whose low unit is a space (`U+10020`), the
/// half-width `U+FF91`, a stray continuation, leads a space or a letter breaks, and the bytes
/// `_isdigit` reads before its table.
const WIDE: &[&[u8]] = &[
    b"\xc3\xa9",
    b"\xc3\x89",
    b"\xe5\x90\x8d",
    b"\xe3\x80\x80",
    b"\xe2\x80\x83",
    b"\xc0\xa0",
    b"\xf0\x90\x80\xa0",
    b"\xef\xbe\x91",
    b"\x80",
    b"\xc3",
    b"\xe3\x81",
    b"\xfc\x80\x80\x80\x80\x80",
    b"\xfd",
    b"\xfb",
    b"\xf7",
    b"\xff",
    b"\xfe",
];

/// A word of the alphabet the row names and the filters share, so that searches hit, with now
/// and then a UTF-8 piece.
fn word(g: &mut Lcg, max: usize) -> Vec<u8> {
    const PICK: &[u8] = b"abAB";
    let n = (g.next() >> 16) as usize % (max + 1);
    let mut w = Vec::new();
    for _ in 0..n {
        if g.next() >> 28 == 0 {
            w.extend(WIDE[(g.next() >> 16) as usize % WIDE.len()]);
        } else {
            w.push(PICK[(g.next() >> 16) as usize % PICK.len()]);
        }
    }
    w
}

fn pick<'b>(g: &mut Lcg, from: &[&'b [u8]]) -> &'b [u8] {
    from[(g.next() >> 16) as usize % from.len()]
}

/// A token: a tag with text, a level range, a quoted phrase or a plain term.
fn token(g: &mut Lcg, tags: &[Vec<u8>; 5]) -> Vec<u8> {
    let mut t = Vec::new();
    match g.next() >> 28 {
        0..=4 => {
            let tag = &tags[(g.next() >> 16) as usize % 5];
            t.extend(if g.next() >> 31 == 0 {
                tag.to_ascii_uppercase()
            } else {
                tag.clone()
            });
            t.extend(word(g, 4));
        }
        5..=8 => t.extend(pick(
            g,
            &[
                b"5",
                b"10-20",
                b"-30",
                b"7-",
                b"5x",
                b"-",
                b"-x",
                b"60-",
                b"0",
                b"12-3-4",
                b"4294967297",
                b"99a-5",
                b"1--2",
                b"-0",
                b"-\xfd",
                b"-\xfb7",
                b"-\xf7",
                b"-\xfe",
                b"3-\xfd",
                b"\xff5",
                b"\xef\xbe\x915",
                b"9\xc3\xa9-2",
            ],
        )),
        9..=10 => {
            t.push(b'"');
            if g.next() >> 31 == 0 {
                t.extend(&tags[(g.next() >> 16) as usize % 5]);
            }
            t.extend(word(g, 3));
            t.push(b' ');
            t.extend(word(g, 3));
            if g.next() >> 30 != 0 {
                t.push(b'"');
            }
        }
        _ => t.extend(word(g, 6)),
    }
    t
}

fn filter(g: &mut Lcg, tags: &[Vec<u8>; 5]) -> Vec<u8> {
    let mut f = Vec::new();
    if g.next() >> 30 == 0 {
        f.extend(pick(g, &[b" ", b"\t", b"  "]));
    }
    let n = (g.next() >> 16) as usize % 8;
    for _ in 0..n {
        f.extend(token(g, tags));
        f.extend(pick(
            g,
            &[
                b" ",
                b"\t",
                b"  ",
                b"\x0b",
                b" \r\n",
                b"\xe3\x80\x80",
                b"\xc0\xa0",
                b"\xf0\x90\x80\xa0",
                b"\xc2\xa0",
            ],
        ));
    }
    if g.next() >> 29 == 0 {
        // Long filters reach the copy's end and the name that follows it.
        while f.len() < 140 {
            f.extend(token(g, tags));
            f.push(b' ');
        }
        let cut = 120 + (g.next() >> 16) as usize % 20;
        f.truncate(cut);
    }
    f
}

/// `FriendList::SendWhoQuery` whole on filters of tags, level ranges, quoted phrases and terms
/// separated by any white space, ASCII or wide, up to and past the 127 bytes the copy keeps,
/// with UTF-8 in words, tags and names (valid, overlong, cut short, stray, and the high bytes the
/// digit tests read), the tags themselves varied (empty, long, any case) and race, class and area
/// rows whose names, ids, parents and locales vary: the packet it sends, byte for byte.
#[test]
fn who_query_packet() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x3e00_0001);
    for case in 0..4000 {
        r.e.reset_bump(r.arena + 0x1_0000);
        r.pool.at = r.pool.base;
        let mut tags: [Vec<u8>; 5] = [
            b"n-".to_vec(),
            b"g-".to_vec(),
            b"z-".to_vec(),
            b"r-".to_vec(),
            b"c-".to_vec(),
        ];
        if g.next() >> 29 == 0 {
            let k = (g.next() >> 16) as usize % 5;
            tags[k] = pick(
                &mut g,
                &[b"", b"n", b"name:", b"G-", b"\xe5\x90\x8d-", b"\xc3\xa9-"],
            )
            .to_vec();
        }
        for (k, t) in tags.iter().enumerate() {
            let at = r.tags + 0x20 * k as u64;
            r.e.write(at, t);
            r.e.write(at + t.len() as u64, &[0]);
        }
        let locale = (g.next() >> 16) % 8;
        r.e.write_u32(LOCALE, locale);
        let mut names: Vec<Vec<(u32, u32, Vec<u8>)>> = Vec::new();
        for (t, &(table, size, off)) in TABLES.iter().enumerate() {
            let n = [0usize, 1, 5, 14][(g.next() >> 16) as usize % 4];
            let rows: Vec<(u32, u32, Vec<u8>)> = (0..n)
                .map(|_| {
                    let id = if g.next() >> 29 == 0 {
                        g.next() >> 24
                    } else {
                        (g.next() >> 16) % 20
                    };
                    let parent = if t == 0 && g.next() >> 30 == 0 {
                        1 + (g.next() >> 28)
                    } else {
                        0
                    };
                    (id, parent, word(&mut g, 4))
                })
                .collect();
            let base = r.pool.at;
            r.pool.at += size * n as u64;
            for (i, (id, parent, name)) in rows.iter().enumerate() {
                let row = base + size * i as u64;
                r.e.write(row, &vec![0u8; size as usize]);
                r.e.write_u32(row, *id);
                if t == 0 {
                    r.e.write_u32(row + 8, *parent);
                }
                for l in 0..8 {
                    let p = if l == locale {
                        r.pool.put(&mut r.e, name)
                    } else {
                        r.pool.put(&mut r.e, b"zz")
                    };
                    r.e.write_u32(row + off + 4 * u64::from(l), p);
                }
            }
            r.e.write_u32(table, base as u32);
            r.e.write_u32(table + 4, n as u32);
            names.push(rows);
        }
        let f = filter(&mut g, &tags);
        let at = r.pool.put(&mut r.e, &f);
        r.e.write_u32(r.sent, 0xffff_ffff);
        r.e.call(SEND_WHO_QUERY, 0, 0, &[at]);
        let n = r.e.read_u32(r.sent) as usize;
        let got = r.e.read(r.sent + 4, n);
        let rows = |t: usize| -> Vec<WhoRow> {
            names[t]
                .iter()
                .map(|(id, parent, name)| WhoRow {
                    id: *id,
                    parent: *parent,
                    name,
                })
                .collect()
        };
        let (areas, races, classes) = (rows(0), rows(1), rows(2));
        let tables = WhoTables {
            areas: &areas,
            races: &races,
            classes: &classes,
        };
        let t = WhoTags {
            name: &tags[0],
            guild: &tags[1],
            zone: &tags[2],
            race: &tags[3],
            class: &tags[4],
        };
        let want = send_who_query(&f, &t, &tables, &wide);
        assert_eq!(
            got,
            want.packet(),
            "case {case} filter {:?} tags {tags:?} {want:?}",
            String::from_utf8_lossy(&f)
        );
    }
    // The system's answers were asked for, for high bytes and for wide white space.
    assert!(
        ASKED_HIGH.load(Ordering::Relaxed) > 0,
        "no high byte reached iswdigit"
    );
    assert!(
        ASKED_SPACE.load(Ordering::Relaxed) > 0,
        "no wide space reached iswspace"
    );
}

/// `SStrStrI` (`0x64b540`) on haystacks and needles of the shared alphabet, empty ones included,
/// and its null arguments; `SStrToInt` (`0x64ac60`) on signs, digits, overflow, trailing text
/// and a null string.
#[test]
fn who_string_helpers() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x5751_0001);
    for case in 0..20000 {
        r.pool.at = r.pool.base;
        let hay = word(&mut g, 6);
        let needle = word(&mut g, 3);
        let h = r.pool.put(&mut r.e, &hay);
        let n = r.pool.put(&mut r.e, &needle);
        let (hp, np) = match g.next() >> 28 {
            0 => (0, n),
            1 => (h, 0),
            _ => (h, n),
        };
        r.e.clear_calls();
        let got = r.e.call(SSTR_STR_I, hp, np, &[]);
        let want = if hp == 0 || np == 0 {
            assert_eq!(r.e.calls().len(), 1, "case {case}: error 0x57");
            None
        } else {
            (0..hay.len())
                .find(|&i| benilla_re_net::strnicmp(&hay[i..], &needle, needle.len() as u32) == 0)
        };
        assert_eq!(
            got != 0,
            sstr_str_i(&hay, &needle) && hp != 0 && np != 0,
            "case {case} {hay:?} {needle:?}"
        );
        assert_eq!(
            got,
            want.map_or(0, |i| h + i as u32),
            "case {case} {hay:?} {needle:?}"
        );
        let mut s: Vec<u8> = match g.next() >> 29 {
            0 => b"-".to_vec(),
            1 => b"+".to_vec(),
            _ => Vec::new(),
        };
        let digits = (g.next() >> 16) as usize % 13;
        for _ in 0..digits {
            s.push(b'0' + ((g.next() >> 16) % 10) as u8);
        }
        s.extend(pick(&mut g, &[b"", b"x", b"-5", b" 1", b"\xb5"]));
        let p = r.pool.put(&mut r.e, &s);
        assert_eq!(
            r.e.call(SSTR_TO_INT, 0, 0, &[p]),
            sstr_to_int(&s),
            "SStrToInt {s:?}"
        );
    }
    // A null string sets error 0x57 (`0x64ac6a`) and returns 0.
    r.e.clear_calls();
    assert_eq!(r.e.call(SSTR_TO_INT, 0, 0, &[0]), 0, "SStrToInt null");
    let c = r.e.calls();
    assert_eq!(c.len(), 1, "SStrToInt null: one call");
    assert_eq!((c[0].addr, c[0].args[0]), (SERR_SET_LAST_ERROR, 0x57));
}
