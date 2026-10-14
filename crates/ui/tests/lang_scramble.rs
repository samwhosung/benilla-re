//! The chat language scramble, run as an oracle against the exe's own bytes. `0x49b560` turns a chat
//! line the listener's character cannot understand into fake-language gibberish, and `0x4982c0`
//! builds the `(language, length)` to words index it draws from. `systems/ui/chatlanguage.md` has the
//! entries.
//!
//! Nothing is transcribed. The tests run the exe's routines over the install's own
//! `LanguageWords.dbc` and pin the resulting `(input, language, output)` triples. They need the
//! install, so each skips without it, and no row names them.

use std::collections::{BTreeMap, BTreeSet};

use benilla_re_dbc::Dbc;
use benilla_re_difftest::{Emu, Wow};
use benilla_re_storm::Vfs;

mod fpkit;
use fpkit::data_fixture;

// ---- the routine and its collaborators -------------------------------------------------------
const GARBLE: u64 = 0x49b560; // (ecx=langId, edx=skill, src, dst, dstSize, keepAngle, keepPunct)
const INDEX_CLEAR: u64 = 0x497be0; // resets the (langId,len) -> words container
const INDEX_BUILD: u64 = 0x4982c0; // fills it from the LanguageWords store
const INDEX_BUILD_END: u64 = 0x498400; // the build loop's exit target; the tail past it is another job
const TABLE_RESIZE: u64 = 0x4a4530; // the container's own bucket-table resize
const HASH_CONTAINER: u32 = 0xb6e598; // the (langId, byteLen) -> words container

// ---- globals the routine reads ---------------------------------------------------------------
const LW_ROWS: u64 = 0xc0db54; // LanguageWords store: +0x0 rows, +0x4 count
const LW_COUNT: u64 = 0xc0db58;
const ACTIVE_PLAYER_CTX: u64 = 0xb41414; // ClntObjMgr's active-player context (GUID at +0xc0/+0xc4)

// ---- stubbed boundaries ----------------------------------------------------------------------
const OBJMGR_GET: u64 = 0x468460; // ObjectMgr::Get(guidLo, guidHi, __FILE__, __LINE__)
const SMEM_ALLOC: u64 = 0x6462e0; // SMemAlloc(size, file, line, flags)
const SMEM_REALLOC: u64 = 0x646320; // SMemReAlloc(ptr, size, file, line, flags)

const FAKE_PLAYER: u32 = 0x0060_0000; // any non-NULL; 0x49b560 only tests it against zero

/// One `LanguageWords.dbc` row as the client's `LanguageWordsRec::Read` (`0x57c900`) lays it out.
struct Row {
    id: u32,
    lang: u32,
    word: String,
}

/// Reads `DBFilesClient\<name>` from the install through the client's archive mount.
fn open_dbc(vfs: &mut Vfs, name: &str) -> Dbc {
    let bytes = vfs
        .read(&format!("DBFilesClient\\{name}"))
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    Dbc::parse(bytes).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn read_language_words() -> Option<Vec<Row>> {
    let dir = data_fixture()?;
    let mut vfs = Vfs::mount_data_dir(dir).ok()?;
    let dbc = open_dbc(&mut vfs, "LanguageWords.dbc");
    // The client itself asserts these two (0x548a39 `cmp eax,3`, 0x548a6e `cmp eax,0xc`).
    assert_eq!(dbc.field_count(), 3, "LanguageWordsRec field count");
    assert_eq!(dbc.record_size(), 12, "LanguageWordsRec record size");
    Some(
        (0..dbc.record_count())
            .map(|r| Row {
                id: dbc.field_u32(r, 0),
                lang: dbc.field_u32(r, 1),
                word: dbc.string_field(r, 2).into_owned(),
            })
            .collect(),
    )
}

/// Map the real `LanguageWords` rows into emulator memory in the client's own record layout,
/// then let the binary's own `0x4982c0` build the `(langId, byteLen) -> word*` index from them.
fn build_index(emu: &mut Emu, rows: &[Row]) {
    // The string block: one NUL-terminated copy per row, pointers stored in the record.
    let mut blob: Vec<u8> = Vec::new();
    let mut at: Vec<usize> = Vec::with_capacity(rows.len());
    for r in rows {
        at.push(blob.len());
        blob.extend_from_slice(r.word.as_bytes());
        blob.push(0);
    }
    let strings = emu.alloc(blob.len() as u64 + 0x10);
    emu.write(strings, &blob);

    let recs = emu.alloc(rows.len() as u64 * 12 + 0x10);
    for (i, r) in rows.iter().enumerate() {
        let o = recs + i as u64 * 12;
        emu.write_u32(o, r.id);
        emu.write_u32(o + 4, r.lang);
        emu.write_u32(o + 8, (strings + at[i] as u64) as u32);
    }
    emu.write_u32(LW_ROWS, recs as u32);
    emu.write_u32(LW_COUNT, rows.len() as u32);

    // The container's growable node/slot storage crosses the Storm allocator boundary.
    // `SMemAlloc` is `ret 0x10` (4 dwords) and `SMemReAlloc` `ret 0x14` (5) — both stdcall.
    // The bump arena lives in the harness scratch map, reserved ahead of the per-call buffers.
    let arena = emu.alloc(0x40_0000);
    emu.stub_alloc_sized(SMEM_ALLOC, arena, arena + 0x40_0000, 0, 4);
    emu.stub_realloc_sized(SMEM_REALLOC, 5);

    emu.call(INDEX_CLEAR, 0, 0, &[]);
    // Size the bucket table to its final width UP FRONT, with the container's own resize
    // `0x4a4530`. The build would otherwise resize mid-flight, and a resize migrates every node
    // through a stack-local list whose intrusive links the bump-allocator stubs cannot reproduce
    // (nodes inserted before a resize are then unreachable). `0x1fff` is exactly where the client
    // itself ends up: `0x4a3560` grows whenever one bucket's chain passes 13 (`0x4a3634`), the
    // bucket index is `mask & langId` alone (`0x4a35ec`), so every length-node of one language
    // shares one chain and growing never shortens it — the table climbs to the hard cap
    // `0x4a35ee cmp eax,0x1fff`. It is output-neutral regardless: `0x49b560` accepts a node only
    // on `[node]==langId && [node+0x14]==len`, those keys are unique, and each node's word vector
    // is appended in DBC row order (`0x4983e9`) independently of the table. The positive control
    // is below — the built index reproduces the shipped file's per-language length histogram
    // exactly, which a lossy build does not.
    emu.call(TABLE_RESIZE, HASH_CONTAINER, 0, &[0x2000]);
    emu.run(INDEX_BUILD, INDEX_BUILD_END);
}

/// Run the binary's `0x49b560` over `text` and return the bytes it wrote into the destination.
fn garble(emu: &mut Emu, lang: u32, skill: u32, text: &str) -> String {
    let src = emu.alloc(text.len() as u64 + 0x10);
    emu.write(src, text.as_bytes());
    emu.write(src + text.len() as u64, &[0]);
    let dst = emu.alloc(0x800);
    emu.write(dst, &vec![0u8; 0x800]);
    emu.call(GARBLE, lang, skill, &[src as u32, dst as u32, 0x800, 0, 0]);
    let raw = emu.read(dst, 0x800);
    let n = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..n]).to_string()
}

struct Oracle<'a> {
    emu: Emu<'a>,
}

impl<'a> Oracle<'a> {
    fn new(wow: &'a Wow) -> Oracle<'a> {
        let mut emu = Emu::new(wow);
        let rows = read_language_words().expect("LanguageWords.dbc");
        // A non-NULL active player: `0x468550` reads the GUID out of `[0xb41414]+0xc0`, and
        // `0x468460` resolves it. Only non-NULL-ness matters to `0x49b560`.
        let ctx = emu.alloc(0x200);
        emu.write_u32(ACTIVE_PLAYER_CTX, ctx as u32);
        emu.write_u32(ctx + 0xc0, 0x1234_5678);
        emu.write_u32(ctx + 0xc4, 0x0000_0001);
        emu.stub(OBJMGR_GET, FAKE_PLAYER, 3);
        build_index(&mut emu, &rows);
        Oracle { emu }
    }
    /// Walk every bucket chain and return `(langId, wordLen) -> candidate count`.
    fn index_histogram(&mut self) -> BTreeMap<(u32, u32), u32> {
        let mask = self.emu.read_u32(0xb6e5bc);
        let base = self.emu.read_u32(0xb6e5b4) as u64;
        let mut h = BTreeMap::new();
        for b in 0..=u64::from(mask) {
            let slot = base + b * 12;
            let delta = u64::from(self.emu.read_u32(slot)); // node -> intrusive-link offset
            let mut node = u64::from(self.emu.read_u32(slot + 8)); // chain head; bit0 = sentinel
            while node & 1 == 0 && node != 0 {
                h.insert(
                    (self.emu.read_u32(node), self.emu.read_u32(node + 0x14)),
                    self.emu.read_u32(node + 0x20),
                );
                node = u64::from(self.emu.read_u32(node + delta + 4));
            }
        }
        h
    }
    fn g(&mut self, lang: u32, skill: u32, text: &str) -> String {
        garble(&mut self.emu, lang, skill, text)
    }
}

/// The built index must reproduce the shipped `LanguageWords.dbc`'s own (language, byte-length)
/// histogram exactly — the positive control that the emulated container is complete before any
/// vector is read out of it.
#[test]
fn lang_scramble_index_matches_dbc() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let rows = read_language_words().expect("LanguageWords.dbc");
    let mut want: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    for r in &rows {
        *want.entry((r.lang, r.word.len() as u32)).or_default() += 1;
    }
    let mut o = Oracle::new(&wow);
    assert_eq!(
        o.index_histogram(),
        want,
        "built index vs shipped LanguageWords.dbc"
    );
    // The file's own shape, so a silent DBC swap is visible here too.
    assert_eq!(rows.len(), 1481);
    assert_eq!(want.keys().map(|k| k.0).collect::<BTreeSet<_>>().len(), 13);
}

/// `(languageId, languageSkill, input, the exact output bytes `0x49b560` writes)`.
/// The chat display path always calls with `keepAngle = keepPunct = 0` (`0x49aa7c`), which is what
/// these vectors use.
const GOLDEN: &[(u32, u32, &str, &str)] = &[
    (1, 0, "hello", "kazum"),
    (1, 0, "h", "o"),
    (1, 0, "ab", "ha"),
    (1, 0, "antidisestablishment", "khaz'rogg'ahn"),
    (
        1,
        0,
        "pneumonoultramicroscopicsilicovolcanoconiosis",
        "khaz'rogg'ahn",
    ),
    (
        1,
        0,
        "the cat sat on the mat the cat",
        "mog ruk ogg gi mog gul mog ruk",
    ),
    (1, 0, "hello hello", "kazum kazum"),
    (1, 0, "hello, world.", "kazum magan "),
    (1, 0, "don't", "re'ka"),
    (1, 0, "abc123", "moguna"),
    (1, 0, "12345", "regas"),
    (1, 0, "hello    world", "kazum magan"),
    (1, 0, "  hello  ", " kazum "),
    (1, 0, "hello\tworld", "kazum magan"),
    (1, 0, "Hello", "Kazum"),
    (1, 0, "HELLO", "KAZUM"),
    (1, 0, "hEllO", "kAzuM"),
    (1, 0, "Hello There Friend", "Kazum No'ku Raznos"),
    (1, 0, "", ""),
    (1, 0, "!!!", " "),
    (1, 0, "a b c", "g g o"),
    (7, 0, "hello", "majis"),
    (2, 0, "hello", "talah"),
    (33, 0, "hello", "majis"),
    (14, 0, "hello", "atuad"),
    (1, 1, "hello", "kazum"),
    (1, 75, "hello", "kazum"),
    (1, 150, "hello", "hello"),
    (1, 299, "hello", "hello"),
    (1, 300, "hello", "hello"),
    (0, 0, "hello", "hello"),
    (1, 0, "Kek lol Rofl", "Ogg kek Ogar"),
    (1, 0, "Thrall sends his regards", "No'gor zugas kil gul'rok"),
    (7, 0, "For the Alliance!", "Nud ras Landowar "),
    (
        1,
        150,
        "the quick brown fox jumps over the lazy dog",
        "the quick nogah fox re'ka nogu the maka kil",
    ),
];

/// The golden vectors: `(language, skill, input) -> the exact bytes `0x49b560` writes`.
/// Produced by running the binary's own code; this is the oracle a re-implementation diffs against.
#[test]
fn lang_scramble_golden_vectors() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut o = Oracle::new(&wow);
    for (lang, skill, input, want) in GOLDEN {
        let got = o.g(*lang, *skill, input);
        assert_eq!(&got, want, "lang={lang} skill={skill} input={input:?}");
    }
}

/// The mapping carries no per-call state: a second, independently constructed emulator, with the
/// index rebuilt from scratch, reproduces every vector byte for byte, and repeating a call on one
/// emulator reproduces it too.
#[test]
fn lang_scramble_is_deterministic_across_sessions() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut a = Oracle::new(&wow);
    let first: Vec<String> = GOLDEN.iter().map(|(l, s, i, _)| a.g(*l, *s, i)).collect();
    let again: Vec<String> = GOLDEN.iter().map(|(l, s, i, _)| a.g(*l, *s, i)).collect();
    assert_eq!(first, again, "same emulator, repeated call");
    let mut b = Oracle::new(&wow);
    let fresh: Vec<String> = GOLDEN.iter().map(|(l, s, i, _)| b.g(*l, *s, i)).collect();
    assert_eq!(first, fresh, "fresh emulator");
}

/// Print the table for the record (`cargo test … -- --nocapture --ignored`).
#[test]
#[ignore]
fn lang_scramble_print_table() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut o = Oracle::new(&wow);
    for (lang, skill, input, _) in GOLDEN {
        println!(
            "| {lang} | {skill} | `{input}` | `{}` |",
            o.g(*lang, *skill, input)
        );
    }
}

/// The `languageId -> spell` join measured over the SHIPPED content, gated so it cannot drift.
///
/// `0x4b25b0` tests ONLY effect slot 0 (`[SpellRec+0xf4]` = field 61) and keys on
/// `EffectMiscValue_1` (`+0x1a8` = field 106); `0x5ec720` then needs a `SkillLineAbility` row for that
/// spell (the index `0x6ddb50` builds from `SkillLineAbility.dbc`, keyed by its field 2). This pins
/// what that chain covers.
#[test]
fn lang_scramble_language_spell_coverage() {
    let Some(dir) = data_fixture() else {
        return;
    };
    let Ok(mut v) = Vfs::mount_data_dir(dir) else {
        return;
    };
    let sp = open_dbc(&mut v, "Spell.dbc");
    let sla = open_dbc(&mut v, "SkillLineAbility.dbc");
    let lang = open_dbc(&mut v, "Languages.dbc");
    let (sp_rows, sla_rows, lang_rows) =
        (sp.record_count(), sla.record_count(), lang.record_count());
    let f = |d: &Dbc, r: u32, c: u32| d.field_u32(r, c);

    // Every SPELL_EFFECT_LANGUAGE declaration, scanned across ALL THREE effect slots — the client
    // reads only slot 0, and the point of scanning all three is to show it loses nothing.
    let mut slot0: Vec<(u32, u32)> = Vec::new();
    let mut other_slots = 0usize;
    for r in 0..sp_rows {
        if f(&sp, r, 61) == 39 {
            slot0.push((f(&sp, r, 0), f(&sp, r, 106)));
        }
        if f(&sp, r, 62) == 39 || f(&sp, r, 63) == 39 {
            other_slots += 1;
        }
    }
    assert_eq!(
        other_slots, 0,
        "no shipped spell declares LANGUAGE outside effect slot 0"
    );
    assert_eq!(
        slot0,
        vec![
            (668, 7),
            (669, 1),
            (670, 3),
            (671, 2),
            (672, 6),
            (813, 7),
            (814, 7),
            (815, 7),
            (816, 7),
            (817, 7),
            (7340, 13),
            (7341, 14),
            (17737, 33),
            (25674, 11),
        ],
        "the 14 language-declaring spells and the language each declares"
    );

    // Six spells declare language 7, so the entry for language 7 at [0xb700ac] is the last one added.
    assert_eq!(slot0.iter().filter(|(_, m)| *m == 7).count(), 6);

    // The second hop: a language is only ever understandable if its spell has a SkillLineAbility row.
    let with_sla: BTreeSet<u32> = slot0
        .iter()
        .filter(|(id, _)| (0..sla_rows).any(|r| f(&sla, r, 2) == *id))
        .map(|(_, m)| *m)
        .collect();
    assert_eq!(
        with_sla,
        BTreeSet::from([1, 2, 3, 6, 7, 13, 14, 33]),
        "the eight player languages are the whole reachable set"
    );
    // Draconic (11) enters [0xb700ac] via 25674 and dead-ends: no SkillLineAbility row exists.
    assert!(!(0..sla_rows).any(|r| f(&sla, r, 2) == 25674));

    let all: BTreeSet<u32> = (0..lang_rows).map(|r| f(&lang, r, 0)).collect();
    assert_eq!(
        all.difference(&with_sla).copied().collect::<BTreeSet<_>>(),
        BTreeSet::from([8, 9, 10, 11, 12]),
        "Demonic, Titan, Thalassian, Draconic and Kalimag garble for everyone, always"
    );
}
