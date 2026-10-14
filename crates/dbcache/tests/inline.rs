//! The inline-string record decoders against the exe, each whole: `GetCString` writes into the struct,
//! so the compare is byte for byte and `SStrDupA` is not called. Each test builds an
//! `SMSG_*_QUERY_RESPONSE` record buffer in wire (read) order, drives the exe's `T::Read` over a
//! `CDataStore`, reads the decoded struct and compares it with the transcription on the same buffer.
//! String lengths run from empty to near the field size (kept below it, so the copy is the ordinary
//! path) and the integers are fuzzed.

mod common;

use benilla_re_dbcache::datastore::DataStore;
use benilla_re_dbcache::decode_inline::{guild, name, page_text, quest};
use benilla_re_difftest::{Emu, Wow};
use common::{make_datastore, next, wire_cstring};

const PAGE_TEXT: u64 = 0x7cabd0; // __thiscall(ecx=record, [esp+4]=CDataStore) ret 4
const PAGE_TEXT_SZ: usize = 0x404;

const NAME: u64 = 0x6dc1f0;
const NAME_SZ: usize = 0x144;

const GUILD: u64 = 0x62f260;
const GUILD_SZ: usize = 0x2f8;

const QUEST: u64 = 0x6dbf60;
const QUEST_SZ: usize = 0x18e0; // record ~0x18dc (last objective text @+0x17dc, char[0x100]); rounded up.

/// PageTextCache_C — `GetCString(+0, 0x400)` page text, then `GetInt32(+0x400)` next id.
#[test]
fn dbcache_page_text() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x1234_0001u32;
    let lens = [0usize, 1, 5, 16, 0x100, 0x3fe]; // < 0x400
    for case in 0..200u32 {
        let tlen = lens[(case as usize) % lens.len()];
        let text = wire_cstring(&mut s, tlen);
        let next_id = next(&mut s);
        let mut buf = Vec::new();
        buf.extend_from_slice(&text);
        buf.extend_from_slice(&next_id.to_le_bytes());

        let mut e = Emu::new(&wow);
        let ds = make_datastore(&mut e, &buf);
        let rec = e.alloc(PAGE_TEXT_SZ as u64);
        e.write(rec, &[0u8; PAGE_TEXT_SZ]);
        e.call(PAGE_TEXT, rec as u32, 0, &[ds as u32]);
        let bin = e.read(rec, PAGE_TEXT_SZ);

        let mut ours = vec![0u8; PAGE_TEXT_SZ];
        page_text(&mut DataStore::new(&buf), &mut ours);
        assert_eq!(bin, ours, "page_text case={case} tlen={tlen}");
    }
}

/// NameCache (guid-keyed) — `GetGuid(+0x130)`, `GetCString(+0, 0x30)`, `GetCString(+0x30, 0x100)`,
/// `GetUInt32(+0x138/+0x13c/+0x140)`.
#[test]
fn dbcache_name() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x2345_0001u32;
    let name_lens = [0usize, 1, 5, 16, 0x2e]; // < 0x30
    let str2_lens = [0usize, 1, 8, 0x40, 0xfe]; // < 0x100
    for case in 0..240u32 {
        let nlen = name_lens[(case as usize) % name_lens.len()];
        let s2len = str2_lens[(case as usize) % str2_lens.len()];
        let guid_lo = next(&mut s);
        let guid_hi = next(&mut s);
        let nm = wire_cstring(&mut s, nlen);
        let s2 = wire_cstring(&mut s, s2len);
        let race = next(&mut s);
        let gender = next(&mut s);
        let class = next(&mut s);
        let mut buf = Vec::new();
        buf.extend_from_slice(&guid_lo.to_le_bytes());
        buf.extend_from_slice(&guid_hi.to_le_bytes());
        buf.extend_from_slice(&nm);
        buf.extend_from_slice(&s2);
        buf.extend_from_slice(&race.to_le_bytes());
        buf.extend_from_slice(&gender.to_le_bytes());
        buf.extend_from_slice(&class.to_le_bytes());

        let mut e = Emu::new(&wow);
        let ds = make_datastore(&mut e, &buf);
        let rec = e.alloc(NAME_SZ as u64);
        e.write(rec, &[0u8; NAME_SZ]);
        e.call(NAME, rec as u32, 0, &[ds as u32]);
        let bin = e.read(rec, NAME_SZ);

        let mut ours = vec![0u8; NAME_SZ];
        name(&mut DataStore::new(&buf), &mut ours);
        assert_eq!(bin, ours, "name case={case} nlen={nlen} s2len={s2len}");
    }
}

/// GuildStats_C — `GetUInt32(+0)`, `GetCString(+4, 0x60)` name, 10× `GetCString(+0x64+i*0x40, 0x40)` ranks,
/// `GetInt32` ×5 @ +0x2e4..+0x2f4.
#[test]
fn dbcache_guild() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x3456_0001u32;
    let name_lens = [0usize, 1, 8, 16, 0x5e]; // < 0x60
    let rank_lens = [0usize, 1, 5, 0x3e]; // < 0x40
    for case in 0..150u32 {
        let id = next(&mut s);
        let nlen = name_lens[(case as usize) % name_lens.len()];
        let gname = wire_cstring(&mut s, nlen);
        let mut buf = Vec::new();
        buf.extend_from_slice(&id.to_le_bytes());
        buf.extend_from_slice(&gname);
        for i in 0..10usize {
            let rlen = rank_lens[(case as usize + i) % rank_lens.len()];
            buf.extend_from_slice(&wire_cstring(&mut s, rlen));
        }
        for _ in 0..5 {
            buf.extend_from_slice(&next(&mut s).to_le_bytes());
        }

        let mut e = Emu::new(&wow);
        let ds = make_datastore(&mut e, &buf);
        let rec = e.alloc(GUILD_SZ as u64);
        e.write(rec, &[0u8; GUILD_SZ]);
        e.call(GUILD, rec as u32, 0, &[ds as u32]);
        let bin = e.read(rec, GUILD_SZ);

        let mut ours = vec![0u8; GUILD_SZ];
        guild(&mut DataStore::new(&buf), &mut ours);
        assert_eq!(bin, ours, "guild case={case} nlen={nlen}");
    }
}

/// QuestCache, the long sequence (`0x6dbf60`): 39 dwords (15 ints, 4 by 2 and 6 by 2 interleaved loop ints,
/// the int at 0x8c, 2 floats, the int at 0x98), 4 big inline strings (Title, Objectives, Details, EndText), 16
/// loop ints, 4 inline objective texts. Wire order == read order, so the buffer feeds reads sequentially.
#[test]
fn dbcache_quest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x4567_0001u32;
    let big_lens = [0usize, 1, 0x10, 0x40, 0x100]; // < each field (Details/+0x129c 0x200; Objectives/EndText 0x800)
    let obj_lens = [0usize, 1, 5, 0x40, 0x80]; // < 0x100
    for case in 0..150u32 {
        let mut buf = Vec::new();
        for _ in 0..39 {
            buf.extend_from_slice(&next(&mut s).to_le_bytes());
        }
        for j in 0..4usize {
            let l = big_lens[(case as usize + j) % big_lens.len()];
            buf.extend_from_slice(&wire_cstring(&mut s, l));
        }
        for _ in 0..16 {
            buf.extend_from_slice(&next(&mut s).to_le_bytes());
        }
        for j in 0..4usize {
            let l = obj_lens[(case as usize + j) % obj_lens.len()];
            buf.extend_from_slice(&wire_cstring(&mut s, l));
        }

        let mut e = Emu::new(&wow);
        let ds = make_datastore(&mut e, &buf);
        let rec = e.alloc(QUEST_SZ as u64);
        e.write(rec, &vec![0u8; QUEST_SZ]);
        e.call(QUEST, rec as u32, 0, &[ds as u32]);
        let bin = e.read(rec, QUEST_SZ);

        let mut ours = vec![0u8; QUEST_SZ];
        quest(&mut DataStore::new(&buf), &mut ours);
        assert_eq!(bin, ours, "quest case={case}");
    }
}
