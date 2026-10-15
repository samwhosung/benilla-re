//! The `/who` query packet `FriendList::SendWhoQuery` (`0x5aebb0`) builds from a filter
//! (`systems/net/who.md#who-query-tags`, `#who-query-packet`).
//!
//! The function keeps its buffers in one stack frame, and their order matters: four 128-byte term
//! slots, the guild (48 bytes in a 96-byte gap), the 128-byte copy of the filter, then the name
//! (48 bytes). A token is written with no bound, and the copy's last byte, when the filter fills
//! it, is followed by the name: the model keeps the same frame.

use crate::strnicmp;

/// `CMSG_WHO`.
pub const CMSG_WHO: u32 = 0x62;

const TERMS: usize = 0;
const GUILD: usize = 0x200;
const COPY: usize = 0x260;
const NAME: usize = 0x2e0;
const FRAME: usize = 0x310;

/// The five localized tags (`WHO_TAG_NAME`, `WHO_TAG_GUILD`, `WHO_TAG_ZONE`, `WHO_TAG_RACE`,
/// `WHO_TAG_CLASS`) as `FrameScript_GetText` returns them.
pub struct WhoTags<'a> {
    pub name: &'a [u8],
    pub guild: &'a [u8],
    pub zone: &'a [u8],
    pub race: &'a [u8],
    pub class: &'a [u8],
}

/// A DBC row the query searches: its id at `+0`, its parent (an area's `+8`, 0 for the others)
/// and its localized name.
pub struct WhoRow<'a> {
    pub id: u32,
    pub parent: u32,
    pub name: &'a [u8],
}

/// The rows of `AreaTable` (`0xc0e040`), `ChrRaces` (`0xc0ded8`) and `ChrClasses`
/// (`0xc0deec`), in record order.
pub struct WhoTables<'a> {
    pub areas: &'a [WhoRow<'a>],
    pub races: &'a [WhoRow<'a>],
    pub classes: &'a [WhoRow<'a>],
}

/// The C string at `at`.
fn cstr(f: &[u8], at: usize) -> &[u8] {
    let n = f[at..].iter().position(|&c| c == 0).unwrap_or(f.len() - at);
    &f[at..at + n]
}

/// `SStrCopy` (`0x64a5a0`) with a bound: at most `cap - 1` bytes and a NUL.
fn copy(f: &mut [u8], dst: usize, src: &[u8], cap: usize) {
    let n = src
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(src.len())
        .min(cap - 1);
    f[dst..dst + n].copy_from_slice(&src[..n]);
    f[dst + n] = 0;
}

/// `sgetu8` (`0x41aab0`): the code point at `at` and the bytes read. A NUL lead, or a NUL where a
/// continuation belongs, gives `0xffffffff`; a stray continuation lead, or a continuation that is
/// not `10xxxxxx`, gives `0x80000000`. The count includes every non-NUL byte read, the bad one
/// too. Leads up to `0xfc` take up to five continuations, with no check for overlong forms.
fn utf8_next(f: &[u8], at: usize) -> (u32, usize) {
    let b = |i: usize| u32::from(f.get(at + i).copied().unwrap_or(0));
    let lead = b(0);
    if lead == 0 {
        return (0xffff_ffff, 0);
    }
    let (n, mut acc) = match lead {
        0xfc..=0xfd => (5, lead & 1),
        0xf8..=0xfb => (4, lead & 3),
        0xf0..=0xf7 => (3, lead & 7),
        0xe0..=0xef => (2, lead & 0xf),
        0xc0..=0xdf => (1, lead & 0x1f),
        0x80..=0xff => return (0x8000_0000, 1),
        _ => return (lead, 1),
    };
    for k in 1..=n {
        let c = b(k);
        if c == 0 {
            return (0xffff_ffff, k);
        }
        if c & 0xc0 != 0x80 {
            return (0x8000_0000, k + 1);
        }
        acc = acc << 6 | (c & 0x3f);
    }
    (acc, n + 1)
}

/// The C ctype table at `0x831726` (`_pctype` and `_pwctype`, which `_setlocale`, unreached,
/// would replace): its `_SPACE` bit is set for `0x09` to `0x0d` and the space, its `_DIGIT` bit
/// for `0` to `9`, and no bit at all for `0x80` to `0xff`.
const CT_SPACE: u16 = 8;
const CT_DIGIT: u16 = 4;

fn c_ctype(w: u16) -> u16 {
    match w {
        0x09..=0x0d | 0x20 => CT_SPACE,
        0x30..=0x39 => CT_DIGIT,
        _ => 0,
    }
}

/// `_iswctype` (`0x7458e4`) on the UTF-16 unit `w`: 0 for `0xffff`, the C table below `0x100`,
/// and otherwise `GetStringTypeW`'s `CT_CTYPE1` bits (`0x413ed3`), which `wide` gives.
fn iswctype(w: u16, mask: u16, wide: &dyn Fn(u16) -> u16) -> bool {
    let bits = match w {
        0xffff => 0,
        0..=0xff => c_ctype(w),
        _ => wide(w),
    };
    bits & mask != 0
}

/// `iswspace` (`0x74006f`) of a decoded code point: only its low 16 bits are tested, so
/// `0x80000000` is the NUL's class and a code point past `0xffff` is its low unit's.
fn is_space(c: u32, wide: &dyn Fn(u16) -> u16) -> bool {
    iswctype(c as u16, CT_SPACE, wide)
}

/// `iswdigit` (`0x740050`) of a byte sign-extended to 16 bits (`movsx ax`), so a byte of `0x80`
/// or more asks `wide` about `U+FF80` to `U+FFFE`, and `0xff` is no digit.
fn is_wdigit(c: u8, wide: &dyn Fn(u16) -> u16) -> bool {
    iswctype(c as i8 as i16 as u16, CT_DIGIT, wide)
}

/// `_isdigit` (`0x73fc0f`) of a sign-extended byte with `__mb_cur_max` 1: the table entry at the
/// byte, which for a byte of `0x80` or more lies before the table. Of those 128 bytes, the ones
/// with `_DIGIT` set are the low bytes of the pointers `_pwctype` (`0x831720`) and `_pctype`
/// (`0x83171c`), `0x26`, and the decimal point (`0x831714`), `.`: bytes `0xfd`, `0xfb` and
/// `0xf7`.
fn is_digit(c: u8) -> bool {
    match c {
        0..=0x7f => c_ctype(u16::from(c)) & CT_DIGIT != 0,
        _ => matches!(c, 0xfd | 0xfb | 0xf7),
    }
}

/// `SStrToInt` (`0x64ac60`): an optional `-`, then decimal digits in wrapping 32-bit arithmetic;
/// 0 when no digit follows.
pub fn sstr_to_int(s: &[u8]) -> u32 {
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let neg = at(0) == b'-';
    let mut i = usize::from(neg);
    let mut v = 0u32;
    while at(i).is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add(u32::from(at(i) - b'0'));
        i += 1;
    }
    if neg {
        v.wrapping_neg()
    } else {
        v
    }
}

/// `0x64b540`: whether `needle` occurs in `hay` without case, at a position before the end of
/// `hay`; an empty `hay` holds nothing, and an empty needle is found in any other.
pub fn sstr_str_i(hay: &[u8], needle: &[u8]) -> bool {
    (0..hay.len()).any(|i| strnicmp(&hay[i..], needle, needle.len() as u32) == 0)
}

/// The query as it is sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhoQuery {
    pub min: u32,
    pub max: u32,
    pub name: Vec<u8>,
    pub guild: Vec<u8>,
    pub races: u32,
    pub classes: u32,
    pub zones: Vec<u32>,
    pub terms: Vec<Vec<u8>>,
}

impl WhoQuery {
    /// The packet: the opcode, the minimum and maximum level, the name and guild as C strings,
    /// the race and class masks, the zone count and ids, the term count and the terms, every
    /// number a u32.
    pub fn packet(&self) -> Vec<u8> {
        let mut p = Vec::new();
        let u = |p: &mut Vec<u8>, v: u32| p.extend(v.to_le_bytes());
        let z = |p: &mut Vec<u8>, s: &[u8]| {
            p.extend(s);
            p.push(0);
        };
        u(&mut p, CMSG_WHO);
        u(&mut p, self.min);
        u(&mut p, self.max);
        z(&mut p, &self.name);
        z(&mut p, &self.guild);
        u(&mut p, self.races);
        u(&mut p, self.classes);
        u(&mut p, self.zones.len() as u32);
        for &id in &self.zones {
            u(&mut p, id);
        }
        u(&mut p, self.terms.len() as u32);
        for t in &self.terms {
            z(&mut p, t);
        }
        p
    }
}

/// `FriendList::SendWhoQuery` on a filter of any bytes, read as UTF-8 by `sgetu8`: a code point is
/// copied to the token as every byte the decoder read, so a bad sequence takes the byte that broke
/// it, and the scan ends at a NUL or a sequence a NUL cuts short. White space is `iswspace` of
/// the code point, where `wide` answers for units of `0x100` and more as `GetStringTypeW` does.
/// The filter is copied up to 127 bytes
/// and a space is appended; a filter of 127 or more puts that space in the copy's last byte and
/// its NUL in the name's first, so a name a tag sets later runs on from the copy and is parsed
/// too. Tokens end at white space outside double quotes, which toggle and are dropped; a token
/// still open at the end is dropped. Each token is tested, without case and as a prefix, for
/// the name, guild, zone, race and class tags in that order, then for a level range; any other
/// token is a term, and the fourth term ends the scan. A tag's text is the token after the tag.
pub fn send_who_query(
    filter: &[u8],
    tags: &WhoTags,
    tables: &WhoTables,
    wide: &dyn Fn(u16) -> u16,
) -> WhoQuery {
    let mut f = [0u8; FRAME];
    copy(&mut f, COPY, filter, 0x80);
    let len = cstr(&f, COPY).len();
    f[COPY + len] = b' ';
    f[COPY + len + 1] = 0;
    let mut q = WhoQuery {
        min: 0,
        max: 100,
        name: Vec::new(),
        guild: Vec::new(),
        races: !0,
        classes: !0,
        zones: Vec::new(),
        terms: Vec::new(),
    };
    let mut terms = 0usize;
    let mut tlen = 0usize;
    let mut quote = false;
    let mut pos = COPY;
    let skip = |f: &[u8], pos: &mut usize| loop {
        let (c, n) = utf8_next(f, *pos);
        if !is_space(c, wide) {
            break;
        }
        *pos += n;
    };
    skip(&f, &mut pos);
    loop {
        let (c, n) = utf8_next(&f, pos);
        if c == 0xffff_ffff {
            break;
        }
        if c == u32::from(b'"') {
            quote = !quote;
            pos += n;
            continue;
        }
        if quote || !is_space(c, wide) {
            let at = TERMS + 0x80 * terms + tlen;
            f.copy_within(pos..pos + n, at);
            tlen += n;
            pos += n;
            continue;
        }
        let slot = TERMS + 0x80 * terms;
        f[slot + tlen] = 0;
        tlen = 0;
        if !tag(&mut f, slot, &mut q, tags, tables, wide) {
            terms += 1;
        }
        if terms >= 4 {
            break;
        }
        skip(&f, &mut pos);
    }
    q.name = cstr(&f, NAME).to_vec();
    q.guild = cstr(&f, GUILD).to_vec();
    q.terms = (0..terms)
        .map(|k| cstr(&f, TERMS + 0x80 * k).to_vec())
        .collect();
    q
}

/// Tests the token at `slot` for a tag or a level range and applies it; `false` for a term.
fn tag(
    f: &mut [u8],
    slot: usize,
    q: &mut WhoQuery,
    tags: &WhoTags,
    tables: &WhoTables,
    wide: &dyn Fn(u16) -> u16,
) -> bool {
    let tok = cstr(f, slot).to_vec();
    let is = |t: &[u8]| strnicmp(&tok, t, t.len() as u32) == 0;
    let text = |t: &[u8]| tok[t.len().min(tok.len())..].to_vec();
    if is(tags.name) {
        let s = text(tags.name);
        copy(f, NAME, &s, 0x30);
        return true;
    }
    if is(tags.guild) {
        let s = text(tags.guild);
        copy(f, GUILD, &s, 0x30);
        return true;
    }
    if is(tags.zone) {
        let s = text(tags.zone);
        if q.zones.len() < 10 {
            for row in tables.areas {
                if row.parent == 0 && q.zones.len() < 10 && sstr_str_i(row.name, &s) {
                    q.zones.push(row.id);
                }
            }
        }
        if q.zones.is_empty() {
            q.zones.push(0);
        }
        return true;
    }
    for (t, rows, mask) in [
        (tags.race, tables.races, &mut q.races),
        (tags.class, tables.classes, &mut q.classes),
    ] {
        if is(t) {
            let s = text(t);
            if *mask == !0 {
                *mask = 0;
            }
            for row in rows {
                if sstr_str_i(row.name, &s) {
                    *mask |= 1 << (row.id & 31);
                }
            }
            return true;
        }
    }
    let at = |i: usize| tok.get(i).copied().unwrap_or(0);
    let mut range = false;
    let mut p = 1;
    if is_wdigit(at(0), wide) {
        let v = sstr_to_int(&tok);
        q.min = v;
        q.max = v;
        range = true;
        while is_wdigit(at(p), wide) {
            p += 1;
        }
        if at(p) == b'-' {
            p += 1;
            q.max = 100;
        }
    } else if at(0) != b'-' {
        return false;
    }
    if is_digit(at(p)) {
        q.max = sstr_to_int(&tok[p..]);
        return true;
    }
    range
}
