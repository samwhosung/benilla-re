//! How `CSimpleEditBox` treats the colour and link escapes `|c...|r` and `|H...|h...|h`, by running
//! the exe: the contents of the per-byte token class array at `E+0x330` and the set of cursor byte
//! offsets the step law can reach, which a reimplementation depends on.
//! `systems/ui/editboxmarkup.md` has the entries.
//!
//! Nothing is transcribed. The tests synthesize an edit box `this`, run the exe's `0x77ba90`
//! (rebuild the class array) and `0x77bb30` (the step law), and assert the array and the lists of
//! reachable offsets. Only three fields are needed: `+0x32c` (the text buffer), `+0x330` (the class
//! array) and `+0x344` (the array's slot capacity, which `0x77ba90` uses to size its `rep stosd`).

use benilla_re_difftest::{Emu, RegisterX86, Wow};

/// A scratch stack top for the `Emu::run` slices (the harness only sets ESP for `Emu::call`).
const STACK_TOP: u64 = 0x7000_0000 + 0x0010_0000 - 0x2000; // inside the harness's mapped stack
const INSERT_TEXT: u64 = 0x77bee0;

const REBUILD_CLASS_ARRAY: u64 = 0x77ba90;
const ADVANCE_TOKENS: u64 = 0x77bb30;

const F_TEXT: u64 = 0x32c;
const F_CLASSARR: u64 = 0x330;
const F_LEN: u64 = 0x338;
const F_CAP: u64 = 0x344;
const F_CURSOR: u64 = 0x36c;

/// One decoded class-array slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Slot {
    len: u32,
    class: u32,
    in_link: bool,
}

fn decode(raw: u32) -> Slot {
    Slot {
        len: raw & 0xffff,
        class: (raw >> 16) & 0xff,
        in_link: raw & 0x8000_0000 != 0,
    }
}

/// A synthesized editbox with its class array already built by the binary's own `0x77ba90`.
struct Box_ {
    e: u64,
    slots: Vec<Slot>,
}

fn build(emu: &mut Emu, text: &[u8]) -> Box_ {
    let cap = (text.len() as u64 + 2 + 0x1f) & !0x1f; // the +32-step growth of 0x77bd90
    let e = emu.alloc(0x480);
    let buf = emu.alloc(cap + 0x20);
    let arr = emu.alloc(cap * 4 + 0x20);

    emu.write(buf, text);
    emu.write(buf + text.len() as u64, &[0]);
    emu.write_u32(e + F_TEXT, buf as u32);
    emu.write_u32(e + F_CLASSARR, arr as u32);
    emu.write_u32(e + F_CAP, cap as u32);
    emu.write_u32(e + F_LEN, text.len() as u32);

    emu.call(REBUILD_CLASS_ARRAY, e as u32, 0, &[]);

    let slots = (0..=text.len())
        .map(|i| decode(emu.read_u32(arr + 4 * i as u64)))
        .collect();
    Box_ { e, slots }
}

/// `0x77bb30(start, steps, atomic_links)` — returns the MAGNITUDE of the byte delta.
fn step(emu: &mut Emu, b: &Box_, start: u32, steps: i32, atomic: u32) -> u32 {
    emu.call(
        ADVANCE_TOKENS,
        b.e as u32,
        0,
        &[start, steps as u32, atomic],
    )
}

/// Walk forward from 0 one step at a time, collecting every cursor offset the law can rest on.
fn reachable_forward(emu: &mut Emu, b: &Box_, atomic: u32, end: u32) -> Vec<u32> {
    let mut out = vec![0u32];
    let mut pos = 0u32;
    for _ in 0..256 {
        let d = step(emu, b, pos, 1, atomic);
        if d == 0 {
            break;
        }
        pos += d;
        out.push(pos);
        if pos >= end {
            break;
        }
    }
    out
}

/// Walk backward from `from`, the mirror of the above.
fn reachable_backward(emu: &mut Emu, b: &Box_, atomic: u32, from: u32) -> Vec<u32> {
    let mut out = vec![from];
    let mut pos = from;
    for _ in 0..256 {
        let d = step(emu, b, pos, -1, atomic);
        if d == 0 {
            break;
        }
        pos -= d;
        out.push(pos);
        if pos == 0 {
            break;
        }
    }
    out.reverse();
    out
}

/// The buffer of a shift-clicked item link, byte-indexed:
///
/// `|cffa335ee` 0..9 · `|Hitem:11684:0:0:0|h` 10..29 · `[` 30 · `Ironfoe` 31..37 · `]` 38 ·
/// `|h` 39..40 · `|r` 41..42 · `dsfsdfsd` 43..50 · terminator slot 51.
const LINK: &[u8] = b"|cffa335ee|Hitem:11684:0:0:0|h[Ironfoe]|h|rdsfsdfsd";

#[test]
fn editbox_markup_class_array() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    let b = build(&mut emu, LINK);

    assert_eq!(
        LINK.len(),
        51,
        "the note's byte indices assume this exact buffer"
    );
    assert_eq!(LINK[30], b'[');
    assert_eq!(LINK[38], b']');
    assert_eq!(LINK[43], b'd');

    let s = |len, class, in_link| Slot {
        len,
        class,
        in_link,
    };
    let zero = s(0, 0, false);

    // The token starts, read back out of the binary's own rebuild.
    assert_eq!(
        b.slots[0],
        s(10, 0, false),
        "|cffa335ee -> class 0, len 10, bit31 CLEAR"
    );
    assert_eq!(
        b.slots[10],
        s(20, 4, true),
        "|Hitem:…|h -> class 4, len 20, bit31 SET"
    );
    assert_eq!(
        b.slots[30],
        s(1, 6, true),
        "'[' -> class 6, inside the link"
    );
    for i in 31..=37 {
        assert_eq!(
            b.slots[i],
            s(1, 6, true),
            "'Ironfoe'[{}] inside the link",
            i - 31
        );
    }
    assert_eq!(
        b.slots[38],
        s(1, 6, true),
        "']' -> class 6, still inside the link"
    );
    assert_eq!(
        b.slots[39],
        s(2, 5, true),
        "the closing |h CARRIES bit31 (cleared after its store)"
    );
    assert_eq!(
        b.slots[41],
        s(2, 1, false),
        "|r -> class 1, bit31 already CLEAR"
    );
    for i in 43..=50 {
        assert_eq!(
            b.slots[i],
            s(1, 6, false),
            "trailing text is class 6, outside the link"
        );
    }

    // Every interior byte of a multi-byte token is left zero by the initial rep stosd.
    for i in [1, 5, 9, 11, 20, 29, 40, 42] {
        assert_eq!(
            b.slots[i], zero,
            "slot {i} is a token-interior byte and must stay 0"
        );
    }
    // And the terminator slot.
    assert_eq!(b.slots[51], zero);
}

#[test]
fn editbox_markup_reachable_cursor_indices() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    let b = build(&mut emu, LINK);
    let end = LINK.len() as u32;

    // (a) atomicLinks = 1 — every keyboard move and every deletion (0x77c6b0 / 0x77c280).
    // The whole colour + link + display text + close + reset run collapses to ONE step.
    let atomic = reachable_forward(&mut emu, &b, 1, end);
    assert_eq!(atomic, vec![0, 43, 44, 45, 46, 47, 48, 49, 50, 51]);

    // (b) atomicLinks = 0 — click, drag, UP/DOWN, scroll sizing, the IME span.
    // The link's VISIBLE text is landable; its escapes never are.
    let loose = reachable_forward(&mut emu, &b, 0, end);
    assert_eq!(
        loose,
        vec![0, 31, 32, 33, 34, 35, 36, 37, 38, 43, 44, 45, 46, 47, 48, 49, 50, 51]
    );

    // Index 30 — the '[' itself — is reachable in NEITHER mode: the class-0 and class-4 tokens
    // before it are skipped unconditionally, so a step from 0 always consumes the '[' as well.
    assert!(!atomic.contains(&30) && !loose.contains(&30));
    // 39/40/41/42 are never landable: the trailing |h and |r absorb FORWARD onto 43.
    for i in [39u32, 40, 41, 42] {
        assert!(!atomic.contains(&i) && !loose.contains(&i));
    }

    // The backward walk is exactly symmetric in both modes.
    assert_eq!(reachable_backward(&mut emu, &b, 1, end), atomic);
    assert_eq!(reachable_backward(&mut emu, &b, 0, end), loose);
}

#[test]
fn editbox_markup_step_consumes_whole_characters() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    // "a" + U+00E9 (2 B) + U+4E2D (3 B) + "b" — 0x41aab0 is an up-to-6-byte UTF-8 decoder.
    let text: &[u8] = b"a\xc3\xa9\xe4\xb8\xadb";
    let b = build(&mut emu, text);
    assert_eq!(b.slots[0].len, 1);
    assert_eq!(b.slots[1].len, 2, "U+00E9 is ONE token of 2 bytes");
    assert_eq!(b.slots[3].len, 3, "U+4E2D is ONE token of 3 bytes");
    assert_eq!(b.slots[6].len, 1);
    for i in [2usize, 4, 5] {
        assert_eq!(
            b.slots[i].len, 0,
            "continuation byte {i} has no token entry"
        );
    }
    // One step crosses a whole character, never a byte of one.
    assert_eq!(
        reachable_forward(&mut emu, &b, 1, text.len() as u32),
        vec![0, 1, 3, 6, 7]
    );
}

#[test]
fn editbox_markup_has_no_inline_texture_escape() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    // 5875's classifier has no `|T`/`|t` arm: the remap table at .text 0x5c2b10 sends both to the
    // literal arm. So "|TFoo|t" is a run of ordinary glyphs, every byte its own cursor stop.
    let text: &[u8] = b"|TFoo|t";
    let b = build(&mut emu, text);
    for (i, &ch) in text.iter().enumerate() {
        assert_eq!(
            b.slots[i],
            Slot {
                len: 1,
                class: 6,
                in_link: false
            },
            "byte {i} ({:?}) must classify as an ordinary literal",
            ch as char
        );
    }
    assert_eq!(
        reachable_forward(&mut emu, &b, 1, text.len() as u32),
        vec![0, 1, 2, 3, 4, 5, 6, 7],
        "no byte of a |T sequence is skipped or glued"
    );
}

/// `0x77bee0`'s head decides accept-vs-refuse at `0x77bf0d`: it either falls through to the body
/// at `0x77bf13`, or jumps clean over it to the epilogue at `0x77c1b8`. So halting at `0x77bf13`
/// IS acceptance — the refuse path can never reach that address. (Halting at `0x77c1b8` does not
/// discriminate: the accept path ends there too.) The test below carries its own positive and
/// negative control for this instrument.
fn insert_accepted(wow: &Wow, text: &[u8], cursor: u32) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut emu = Emu::new(wow);
        let b = build(&mut emu, text);
        emu.write_u32(b.e + F_CURSOR, cursor);
        emu.set_reg(RegisterX86::ECX, b.e);
        emu.set_reg(RegisterX86::ESP, STACK_TOP);
        emu.set_reg(RegisterX86::EBP, STACK_TOP);
        emu.run(INSERT_TEXT, 0x77bf13);
    }))
    .is_ok()
}

#[test]
fn editbox_markup_insert_refusal_range() {
    let Some(wow) = Wow::load() else { return };
    // Controls for the discriminator itself : a known-accept and a known-refuse.
    assert!(
        insert_accepted(&wow, LINK, 0),
        "control: cursor 0 must accept"
    );
    assert!(
        !insert_accepted(&wow, LINK, 33),
        "control: mid-name must refuse"
    );

    let refused: Vec<u32> = (0..=LINK.len() as u32)
        .filter(|&c| !insert_accepted(&wow, LINK, c))
        .collect();
    // Exactly the link's interior: the first byte of the display text through the closing |h.
    assert_eq!(refused, (30..=39).collect::<Vec<u32>>());

    // The link's own boundaries ACCEPT: 10 is the |H token's own first byte (bit31 set, but the
    // token before it is the |c, bit31 clear), and 41 is the |r just past the close.
    assert!(insert_accepted(&wow, LINK, 10));
    assert!(insert_accepted(&wow, LINK, 41));
    // 40 is a token-INTERIOR byte of the closing |h; its slot is 0, so bit31 reads clear and the
    // predicate accepts. Unreachable as a cursor value (nothing can place one mid-token), but the
    // predicate itself does not reject it — a re-implementation must not read the guard as
    // "rejects every byte of the link".
    assert!(insert_accepted(&wow, LINK, 40));
}

/// The step law does NOT terminate on every buffer. The inner skip loop re-enters at `0x77bb60`
/// **without** the outer zero check at `0x77bb56`, and the terminator slot decodes as class 0 with
/// length 0 — so a walk that is still skipping when it reaches the end spins forever, consuming no
/// bytes. Reachable in the real client: `0x77c750` (RIGHT) guards only on `cursor < E+0x338`, so
/// `editBox:SetText("|cffffffff")` + one Right keypress reaches `0x77bb30(0, +1, 1)` and hangs.
/// This is a **client defect**, recorded so a faithful transcription does not inherit the freeze.
#[test]
fn editbox_markup_step_law_hangs_on_a_trailing_skip_class() {
    let Some(wow) = Wow::load() else { return };
    let walks = |text: &'static [u8], atomic: u32| {
        std::panic::catch_unwind(|| {
            let mut emu = Emu::new(&wow);
            let b = build(&mut emu, text);
            let mut pos = 0u32;
            for _ in 0..64 {
                let d = step(&mut emu, &b, pos, 1, atomic);
                if d == 0 {
                    break;
                }
                pos += d;
                if pos >= text.len() as u32 {
                    break;
                }
            }
        })
        .is_ok()
    };

    // A trailing class-0 |c is the skip class in BOTH modes -> hangs in both.
    assert!(!walks(b"|cffffffff", 0));
    assert!(!walks(b"|cffffffff", 1));
    assert!(!walks(b"x|cffffffff", 1));
    // A link left UNCLOSED is a skip class only when atomicLinks is set (bit 31 keeps the walk
    // skipping past the display text and off the end) -> hangs on the keyboard path only.
    assert!(!walks(b"|Hitem:1:0:0:0|h[A]", 1));
    assert!(walks(b"|Hitem:1:0:0:0|h[A]", 0));

    // The absorb loop at 0x77bb98 DOES exit on the terminator (class 0 is neither 1 nor 5), so a
    // trailing |r or |h is safe, and so is a well-formed closed link.
    assert!(walks(b"|cffffffff|r", 1));
    assert!(walks(b"x|r", 1));
    assert!(walks(b"x|h", 1));
    assert!(walks(b"|Hitem:1:0:0:0|h[A]|h|r", 1));
    // And a link with EMPTY display text is demoted to literals by 0x5c2810, so it never arms the
    // atomic skip at all.
    assert!(walks(b"|Hitem:1:0:0:0|h", 1));
    assert!(walks(b"|Hx|h|h", 1));
}
