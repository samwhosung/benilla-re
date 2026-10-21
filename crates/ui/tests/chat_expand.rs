//! `ExpandTargetTokens` (`0x5083a0`), the `%` expander of `SendChatMessage`, run whole in the
//! emulator against `expand_target_tokens` and `target_name`.
//!
//! It runs with its own callees `SStrChr` (`0x64a3c0`), `SStrLen` (`0x64a6f0`) and `SStrPack`
//! (`0x64a760`). The four calls that depend on the game are stubbed and logged with their
//! registers and arguments: the typed object lookup `0x468460`, `GetUnitName` `0x609210`, the name
//! cache lookup `0x55f080` and the script text lookup `0x703bf0`; `SErrSetLastError` (`0x64e850`)
//! is stubbed too and must not be called. The buffer is a 0x800-byte region filled with a sentinel,
//! compared whole after each call, so the stray NUL the expander stores past `size` shows. The
//! inputs are every text of five symbols to length six against five sizes and the five ways a
//! target resolves, then a fuzz of long texts, and runs before a `%t` that put the stray store at
//! every offset from 256 to 395.

use benilla_re_difftest::{Emu, RegisterX86, StubIo, Wow};
use benilla_re_ui::chat_expand::{expand_target_tokens, target_name};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const EXPAND: u64 = 0x5083a0;
const OBJECT_PTR: u64 = 0x468460;
const UNIT_NAME: u64 = 0x609210;
const NAME_CACHE_GET: u64 = 0x55f080;
const GET_TEXT: u64 = 0x703bf0;
const SERR: u64 = 0x64e850;
/// `s_targetGuid`, `s_tokenCursor`, the names cache object and the `TARGET_TOKEN_NOT_FOUND` key.
const TARGET_GUID: u64 = 0xb4e2d8;
const TOKEN_CURSOR: u64 = 0xbe0b80;
const NAME_CACHE: u32 = 0xc0e228;
const NOT_FOUND_KEY: u32 = 0x84f3a4;

const SENTINEL: u8 = 0xaa;
const REGION: usize = 0x800;
const CURSOR_SENTINEL: u32 = 0x5555_5555;
/// A stand-in for the unit object the object manager returns; `GetUnitName` is stubbed, so it
/// is never read.
const UNIT_OBJECT: u32 = 0x7123_4560;

/// One stubbed call: the callee, its `ecx` and `edx`, and its stack arguments in order.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Call {
    callee: u64,
    ecx: u32,
    edx: u32,
    args: Vec<u32>,
}

/// How the target resolves: the five regimes of the lookup order.
#[derive(Clone, Copy, Debug)]
enum Situation {
    NoTarget,
    HeldUnit,
    CachedLowGuid,
    CachedHighGuidOnly,
    Missing,
}

const SITUATIONS: [Situation; 5] = [
    Situation::NoTarget,
    Situation::HeldUnit,
    Situation::CachedLowGuid,
    Situation::CachedHighGuidOnly,
    Situation::Missing,
];

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s >> 8
}

/// Every string of `alphabet` up to `max` bytes, shortest first.
fn strings(alphabet: &[u8], max: usize) -> Vec<Vec<u8>> {
    let mut all = vec![Vec::new()];
    let mut from = 0;
    for _ in 0..max {
        let to = all.len();
        for i in from..to {
            for &c in alphabet {
                let mut v = all[i].clone();
                v.push(c);
                all.push(v);
            }
        }
        from = to;
    }
    all
}

/// The emulator with the stubs, the cells they return, and the log they fill.
struct Rig<'a> {
    emu: Emu<'a>,
    text: u64,
    buf: u64,
    unit_name: u64,
    cache_record: u64,
    not_found: u64,
    log: Rc<RefCell<Vec<Call>>>,
    unit_held: Rc<Cell<bool>>,
    cache_hit: Rc<Cell<bool>>,
}

/// Logs a stubbed call: `ecx`, `edx` when the callee reads it, and `n` stack arguments.
fn logged(log: &RefCell<Vec<Call>>, io: &StubIo, callee: u64, n: u64, edx_read: bool) {
    log.borrow_mut().push(Call {
        callee,
        ecx: io.reg(RegisterX86::ECX) as u32,
        edx: if edx_read {
            io.reg(RegisterX86::EDX) as u32
        } else {
            0
        },
        args: (0..n).map(|i| io.arg(i)).collect(),
    });
}

impl<'a> Rig<'a> {
    fn new(wow: &Wow) -> Rig<'a> {
        let mut emu = Emu::new(wow);
        let text = emu.alloc(0x800);
        let buf = emu.alloc(REGION as u64);
        let unit_name = emu.alloc(0x400);
        let cache_record = emu.alloc(0x400);
        let not_found = emu.alloc(0x400);
        let log = Rc::new(RefCell::new(Vec::new()));
        let unit_held = Rc::new(Cell::new(false));
        let cache_hit = Rc::new(Cell::new(false));
        let (l, held) = (log.clone(), unit_held.clone());
        emu.stub_fn(OBJECT_PTR, 3, move |io| {
            logged(&l, io, OBJECT_PTR, 3, true);
            if held.get() {
                UNIT_OBJECT
            } else {
                0
            }
        });
        let l = log.clone();
        emu.stub_fn(UNIT_NAME, 1, move |io| {
            logged(&l, io, UNIT_NAME, 1, false);
            unit_name as u32
        });
        let (l, hit) = (log.clone(), cache_hit.clone());
        emu.stub_fn(NAME_CACHE_GET, 6, move |io| {
            logged(&l, io, NAME_CACHE_GET, 6, false);
            if hit.get() {
                cache_record as u32
            } else {
                0
            }
        });
        let l = log.clone();
        emu.stub_fn(GET_TEXT, 1, move |io| {
            logged(&l, io, GET_TEXT, 1, true);
            not_found as u32
        });
        emu.stub(SERR, 0, 1);
        Rig {
            emu,
            text,
            buf,
            unit_name,
            cache_record,
            not_found,
            log,
            unit_held,
            cache_hit,
        }
    }

    /// One call, compared with the model: the return value, the whole region, the cursor global
    /// and the sequence of lookups.
    fn check(
        &mut self,
        text: &[u8],
        size: u32,
        situation: Situation,
        names: &[&[u8]; 3],
    ) -> Vec<u8> {
        let (unit, cache, missing) = (names[0], names[1], names[2]);
        let guid: u64 = match situation {
            Situation::NoTarget => 0,
            Situation::CachedHighGuidOnly => 0x0000_0123_0000_0000,
            _ => 0x0000_0000_0123_4567 | 0x4000_0000_0000_0000,
        };
        let held = matches!(situation, Situation::HeldUnit);
        let hit = matches!(
            situation,
            Situation::CachedLowGuid | Situation::CachedHighGuidOnly
        );
        for (cell, bytes) in [
            (self.unit_name, unit),
            (self.cache_record, cache),
            (self.not_found, missing),
        ] {
            let mut z = bytes.to_vec();
            z.push(0);
            self.emu.write(cell, &z);
        }
        self.unit_held.set(held);
        self.cache_hit.set(hit);
        self.emu.write(TARGET_GUID, &guid.to_le_bytes());
        let mut z = text.to_vec();
        z.push(0);
        self.emu.write(self.text, &z);
        self.emu.write(self.buf, &[SENTINEL; REGION]);
        self.emu.write_u32(TOKEN_CURSOR, CURSOR_SENTINEL);
        self.log.borrow_mut().clear();
        self.emu.clear_calls();
        let got = self
            .emu
            .call(EXPAND, self.text as u32, self.buf as u32, &[size]);

        let name = target_name(guid, held.then_some(unit), hit.then_some(cache), missing);
        let mut mem = vec![SENTINEL; REGION];
        let want = expand_target_tokens(text, &mut mem, size as usize, name);
        let what = format!(
            "{:?} size {size} {situation:?}",
            String::from_utf8_lossy(text)
        );
        assert_eq!(got, want, "result: {what}");
        let out = self.emu.read(self.buf, REGION);
        if let Some(i) = (0..REGION).find(|&i| out[i] != mem[i]) {
            panic!(
                "memory: {what}: first difference at offset {i}: exe {:#x}, model {:#x}",
                out[i], mem[i]
            );
        }
        assert_eq!(self.emu.read_u32(TOKEN_CURSOR), 0, "cursor global: {what}");
        assert!(
            self.emu.calls().iter().all(|c| c.addr != SERR),
            "no error: {what}"
        );

        // The lookups, once per token and in this order.
        let mut tokens = 0;
        let mut i = 0;
        while i < text.len() {
            if text[i] == b'%' && matches!(text.get(i + 1), Some(b'N' | b'T' | b'n' | b't')) {
                tokens += 1;
                i += 2;
            } else {
                i += 1;
            }
        }
        let (lo, hi) = (guid as u32, (guid >> 32) as u32);
        let mut one = Vec::new();
        let not_found = Call {
            callee: GET_TEXT,
            ecx: NOT_FOUND_KEY,
            edx: 0xffff_ffff,
            args: vec![0],
        };
        if guid == 0 {
            one.push(not_found);
        } else {
            one.push(Call {
                callee: OBJECT_PTR,
                ecx: 8,
                edx: 0x84f2dc,
                args: vec![lo, hi, 0x324],
            });
            if held {
                one.push(Call {
                    callee: UNIT_NAME,
                    ecx: UNIT_OBJECT,
                    edx: 0,
                    args: vec![0],
                });
            } else {
                one.push(Call {
                    callee: NAME_CACHE_GET,
                    ecx: NAME_CACHE,
                    edx: 0,
                    args: vec![lo, hi, 0, 0, 0, 0],
                });
                if !hit {
                    one.push(not_found);
                }
            }
        }
        let mut log = self.log.borrow().clone();
        for call in &mut log {
            // The owner pointer is the expander's local; its cell holds 0 and 0.
            if call.callee == NAME_CACHE_GET {
                let owner = u64::from(call.args[2]);
                assert_eq!(self.emu.read(owner, 8), [0; 8], "owner cell: {what}");
                call.args[2] = 0;
            }
        }
        let expect: Vec<Call> = (0..tokens).flat_map(|_| one.clone()).collect();
        assert_eq!(log, expect, "lookups: {what}");
        out
    }
}

/// `0x5083a0` whole.
#[test]
fn expand_target_tokens_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let long: Vec<u8> = (0..300).map(|i| b'A' + (i % 26) as u8).collect();
    let pools: [[&[u8]; 3]; 4] = [
        [b"Unit", b"Cached", b"<missing>"],
        [b"", b"", b""],
        [b"Thrall", b"Jaina", b""],
        [&long, &long, &long],
    ];

    // Every text of five symbols to length six, against the sizes and the target regimes.
    let grid = strings(b"a%tNx", 6);
    let sizes = [0u32, 1, 2, 3, 0x100];
    let mut calls = 0;
    let mut seed = 0x5083_a001u32;
    for text in &grid {
        for size in sizes {
            let situation = SITUATIONS[next(&mut seed) as usize % SITUATIONS.len()];
            let names = &pools[next(&mut seed) as usize % pools.len()];
            rig.check(text, size, situation, names);
            calls += 1;
        }
    }

    // A fuzz of longer texts over the same symbols, the other token letters and high bytes.
    let alphabet = b"abc  %%%tTnNxX\x80\xff";
    for _ in 0..4000 {
        let len = next(&mut seed) as usize % 0x300;
        let text: Vec<u8> = (0..len)
            .map(|_| alphabet[next(&mut seed) as usize % alphabet.len()])
            .collect();
        let size = [0x100, 0x100, 0x100, 0x20, 0x80, 7][next(&mut seed) as usize % 6];
        let situation = SITUATIONS[next(&mut seed) as usize % SITUATIONS.len()];
        let names = &pools[next(&mut seed) as usize % pools.len()];
        rig.check(&text, size, situation, names);
        calls += 1;
    }

    // The stray NUL: a run of k letters before `%t`, behind 0, 1, 7, 33 and 66 tokens `x%t`
    // that fill the buffer to 0, 5, 35, 165 and 255 bytes (held unit) or to 0, 10, 70, 255 and
    // 255 (missing target), so the store lands at every offset from 256 to 395.
    for tokens in [0usize, 1, 7, 33, 66] {
        for k in 1..=140 {
            let mut text = b"x%t".repeat(tokens);
            text.extend(std::iter::repeat_n(b'q', k));
            text.extend_from_slice(b"%t tail");
            for situation in [Situation::HeldUnit, Situation::Missing] {
                rig.check(&text, 0x100, situation, &pools[0]);
                calls += 1;
            }
        }
    }
    // The same at an empty buffer with runs of 240 to 300, around the first outside offset.
    for k in 240..=300 {
        let mut text = vec![b'q'; k];
        text.extend_from_slice(b"%t tail");
        rig.check(&text, 0x100, Situation::HeldUnit, &pools[0]);
        calls += 1;
    }

    // What the entry states, read off the exe's output.
    let names: [&[u8]; 3] = [b"Bob", b"Cached", b"<none>"];
    let expand = |rig: &mut Rig, text: &[u8], situation| {
        let out = rig.check(text, 0x100, situation, &names);
        out[..out.iter().position(|&b| b == 0).unwrap()].to_vec()
    };
    let held = Situation::HeldUnit;
    assert_eq!(expand(&mut rig, b"hi %t and %N", held), b"hi Bob and Bob");
    assert_eq!(expand(&mut rig, b"%T%n%t", held), b"BobBobBob");
    assert_eq!(expand(&mut rig, b"%%t", held), b"%Bob");
    assert_eq!(expand(&mut rig, b"%x %z 50%", held), b"%x %z 50%");
    assert_eq!(expand(&mut rig, b"%%", held), b"%%");
    assert_eq!(expand(&mut rig, b"%", held), b"%");
    assert_eq!(expand(&mut rig, b"%t", Situation::NoTarget), b"<none>");
    assert_eq!(expand(&mut rig, b"%t", Situation::CachedLowGuid), b"Cached");
    assert_eq!(expand(&mut rig, b"%t", Situation::Missing), b"<none>");
    assert_eq!(expand(&mut rig, b"abc", held), b"abc");
    assert_eq!(expand(&mut rig, b"", held), b"");

    // The result is 1 only when every `%` is followed by a token letter.
    for (text, result) in [
        (&b""[..], 1),
        (b"plain", 1),
        (b"%t", 1),
        (b"%t %n", 1),
        (b"%", 0),
        (b"%x", 0),
        (b"%%t", 0),
        (b"100%", 0),
    ] {
        rig.emu.write(rig.text, &[text, b"\0"].concat());
        rig.emu.write(rig.buf, &[SENTINEL; REGION]);
        rig.emu.write(TARGET_GUID, &[0; 8]);
        let got = rig
            .emu
            .call(EXPAND, rig.text as u32, rig.buf as u32, &[0x100]);
        assert_eq!(got, result, "{:?}", String::from_utf8_lossy(text));
    }

    // A null text gives 0 and touches nothing: not the buffer, not the cursor, no lookup.
    rig.emu.write(rig.buf, &[SENTINEL; REGION]);
    rig.emu.write_u32(TOKEN_CURSOR, CURSOR_SENTINEL);
    rig.log.borrow_mut().clear();
    rig.emu.clear_calls();
    assert_eq!(rig.emu.call(EXPAND, 0, rig.buf as u32, &[0x100]), 0);
    assert_eq!(rig.emu.read(rig.buf, REGION), [SENTINEL; REGION]);
    assert_eq!(rig.emu.read_u32(TOKEN_CURSOR), CURSOR_SENTINEL);
    assert!(rig.log.borrow().is_empty() && rig.emu.calls().is_empty());
    eprintln!("expand_target_tokens_whole: {calls} calls bit-exact");
}
