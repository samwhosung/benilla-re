//! Bit-exact tests of the integer and string control-code and line-break functions against
//! `WoW.exe`. They return ints or write through pointers, so they are driven through `Emu::call` (EAX
//! return) with synthesized input strings in scratch and the out-values read back with `read_u32`.
//! Each case compares the exe's own code with `benilla_re_font::markup`.
//!
//! Helper callees that run for real inside the emulator: `decode_quoted_code` runs the UTF-8
//! next-char decoder (`0x41aab0`) and `strtoul` base 16 (`0x40daf2`); `line_break_class` runs
//! `isspace` through `_isctype` (`0x74006f`, over the statically initialised CRT ctype table).
//! `line_break_enumerate`'s layout kernel (`0x5c7260`) is a separate FreeType-backed cluster, so its
//! test stays on the newline, offset, cap and trailing path where the kernel is never invoked (see
//! the module doc).

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_font::markup::{
    decode_quoted_code, line_break_class, line_break_enumerate, strip_control_codes, Decoded,
    StripOutcome,
};
use common::next;

const DECODE_QUOTED_CODE: u64 = 0x5c2810;
const LINE_BREAK_CLASS: u64 = 0x5c7780;
const LINE_BREAK_ENUMERATE: u64 = 0x5c2430;
const STRIP_CONTROL_CODES: u64 = 0x5c2590;
const ISSPACE: u64 = 0x74006f;

// ── decode_quoted_code ─────────────────────────────────────────────────────────────────────────

/// Diff one decode against the binary. `text` is written into `text_ptr` (zero-padded → NUL-terminated);
/// the three out-pointers are read back. Compares class, consumed length, out-code, and (code 0) colour.
#[allow(clippy::too_many_arguments)]
fn check_decode(
    e: &mut Emu,
    text_ptr: u64,
    consumed_ptr: u64,
    color_ptr: u64,
    outcode_ptr: u64,
    s: &[u8],
    flags: u32,
    ctx: &str,
) {
    assert!(s.len() < 60, "decode corpus string too long: {ctx}");
    e.write(text_ptr, &[0u8; 64]); // zero → NUL-terminated
    e.write(text_ptr, s);
    e.write_u32(color_ptr, 0xdead_beef); // sentinel: colour only written on code 0
    e.write_u32(consumed_ptr, 0xa5a5_a5a5);
    e.write_u32(outcode_ptr, 0xa5a5_a5a5);

    let class = e.call(
        DECODE_QUOTED_CODE,
        text_ptr as u32,
        consumed_ptr as u32,
        &[color_ptr as u32, flags, outcode_ptr as u32],
    );
    let ours: Decoded = decode_quoted_code(s, flags);

    assert_eq!(class, ours.class, "{ctx}: class");
    assert_eq!(
        e.read_u32(consumed_ptr),
        ours.consumed as u32,
        "{ctx}: consumed (bin={} ours={})",
        e.read_u32(consumed_ptr) as i32,
        ours.consumed
    );
    assert_eq!(e.read_u32(outcode_ptr), ours.out_code, "{ctx}: out_code");
    if class == 0 {
        assert_eq!(
            e.read_u32(color_ptr),
            ours.color,
            "{ctx}: color (bin={:#010x} ours={:#010x})",
            e.read_u32(color_ptr),
            ours.color
        );
    }
}

#[test]
fn decode_quoted_code_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let text = e.alloc(64);
    let consumed = e.alloc(4);
    let color = e.alloc(4);
    let outcode = e.alloc(4);

    // Every branch the note accounts for, all four gate bits exercised.
    let cases: &[(&[u8], u32, &str)] = &[
        // literal glyphs (code 6): ASCII + every UTF-8 width + the invalid/empty edges.
        (b"A", 0, "ascii"),
        (b"\xc3\xa9", 0, "2byte U+00E9"),
        (b"\xe2\x82\xac", 0, "3byte U+20AC"),
        (b"\xf0\x9f\x98\x80", 0, "4byte U+1F600"),
        (b"\xf8\x88\x80\x80\x80", 0, "5byte lead"),
        (b"\xfc\x84\x80\x80\x80\x80", 0, "6byte lead"),
        (b"\x80", 0, "stray continuation (invalid lead)"),
        (b"\xc3", 0, "truncated 2byte"),
        (b"\xe2\x82", 0, "truncated 3byte"),
        (b"\xc3\x41", 0, "bad continuation"),
        (b"\xc0\x80", 0, "overlong NUL -> cp0"),
        (b"", 0, "empty"),
        // newlines (code 2): raw CR / LF / CRLF / CR+other.
        (b"\r", 0, "CR"),
        (b"\r\n", 0, "CRLF"),
        (b"\rZ", 0, "CR then non-LF"),
        (b"\n", 0, "LF"),
        (b"\n\n", 0, "LF run (first only)"),
        // colour (code 0): valid (upper/lower, both C/c), truncated, parse-error.
        (b"|cFF8040C0", 0, "color upper"),
        (b"|caabbccdd", 0, "color lower"),
        (b"|CFF00FF00", 0, "color C-upper"),
        (b"|c00000000", 0, "color zero"),
        (b"|cFFFFFFFF", 0, "color white"),
        (b"|cFF80", 0, "color truncated even"),
        (b"|cFFF", 0, "color truncated odd"),
        (b"|c", 0, "color empty"),
        (b"|cFFGG0000", 0, "color parse-error first pair"),
        (b"|cFF80GZ00", 0, "color parse-error mid"),
        (b"|cFF8040C", 0, "color one-short"),
        // hyperlink (code 4): valid open (needs a byte after |h) + every malformed shape.
        (b"|Habc|hX", 0, "link open ok"),
        (b"|Hk:1|h[", 0, "link open colon"),
        (b"|H\xc3\xa9|hY", 0, "link multibyte body"),
        (b"|Habc|h", 0, "link no byte after h"),
        (b"|Habc|XY", 0, "link not-h after pipe"),
        (b"|H|hX", 0, "link empty"),
        (b"|Habc|h|hX", 0, "link doubled close"),
        (b"|Habc", 0, "link no closing pipe"),
        (b"|H", 0, "link bare H"),
        (b"|Ha|", 0, "link pipe at end"),
        // simple sequences.
        (b"|n", 0, "|n newline"),
        (b"|N", 0, "|N newline"),
        (b"|r", 0, "|r restore"),
        (b"|R", 0, "|R restore"),
        (b"|h", 0, "|h link-close"),
        (b"||", 0, "|| escaped pipe"),
        (b"|x", 0, "|x unknown"),
        (b"|", 0, "bare pipe"),
        (b"|Z", 0, "|Z unknown"),
        // gate suppression (each bit turns the matching sequence into a literal).
        (b"|cFF00FF00", 0x100, "color gated 0x100"),
        (b"|r", 0x100, "restore gated 0x100"),
        (b"|R", 0x100, "restore-upper gated 0x100"),
        (b"|n", 0x200, "newline gated 0x200"),
        (b"|N", 0x200, "newline-upper gated 0x200"),
        (b"|Habc|hX", 0x400, "link-open gated 0x400"),
        (b"|h", 0x400, "link-close gated 0x400"),
        (b"||", 0x800, "escpipe gated 0x800 (whole family)"),
        (b"|cFF00FF00", 0x800, "color gated by 0x800"),
        (b"|n", 0x800, "newline-seq gated by 0x800"),
        (b"\r", 0x800, "raw CR unaffected by 0x800"),
        (b"\n", 0x200, "raw LF unaffected by 0x200"),
        (b"|cFF00FF00", 0x200, "color unaffected by 0x200"),
        (b"|n", 0x100, "|n unaffected by 0x100"),
        (b"|cFF00FF00", 0x700, "color gated, all-but-pipe"),
    ];
    for (s, flags, ctx) in cases {
        check_decode(&mut e, text, consumed, color, outcode, s, *flags, ctx);
    }

    // Fuzz: random short strings over a markup-shaped alphabet that excludes the chars where the
    // 2-char hex scratch buffer would hit strtoul's whitespace/sign/"0x"-prefix paths (space, tab,
    // '+', '-', 'x', 'X') — so the colour parser's hex-domain model provably matches `strtoul`.
    const ALPHA: &[u8] = b"0123456789abcdefABCDEF|HhnNrRcCgzYZ\n\r";
    const HIGH: &[u8] = &[0xc3, 0xa9, 0xe2, 0x82, 0xac, 0xf0, 0x9f, 0x80];
    let mut s: u32 = 0x5c28_1001;
    for i in 0..6000u32 {
        let len = (next(&mut s) >> 28) as usize % 14; // 0..13
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            let r = next(&mut s);
            let byte = if r & 0x7 == 0 {
                HIGH[(r >> 8) as usize % HIGH.len()]
            } else {
                ALPHA[(r >> 8) as usize % ALPHA.len()]
            };
            buf.push(byte);
        }
        let flags = (next(&mut s) >> 20) & 0xf00; // random subset of the four gate bits
        check_decode(
            &mut e,
            text,
            consumed,
            color,
            outcode,
            &buf,
            flags,
            &format!("fuzz[{i}]"),
        );
    }
}

// ── line_break_class ───────────────────────────────────────────────────────────────────────────

#[test]
fn line_break_class_ascii_difftest() {
    // The whole c<0x100 domain runs natively: `isspace` reads the statically-initialised CRT ctype
    // table, so every ASCII/Latin-1 branch (quick-yes '-'/':'/'/', whitespace, the low opener set
    // {0x24,0x28,0x5b,0x5c,0x7b}, the low closer set {0x21..0xb7}, the -1 sentinel) is exhaustively
    // diffed without any stub.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for cur in 0u32..0x100 {
        for next in 0u32..0x100 {
            let bin = e.call(LINE_BREAK_CLASS, cur, next, &[]);
            assert_eq!(
                bin,
                line_break_class(cur, next),
                "lbc cur={cur:#x} next={next:#x}"
            );
        }
        // the "no next char" sentinel
        let bin = e.call(LINE_BREAK_CLASS, cur, 0xffff_ffff, &[]);
        assert_eq!(
            bin,
            line_break_class(cur, 0xffff_ffff),
            "lbc cur={cur:#x} next=-1"
        );
    }
}

#[test]
fn line_break_class_cjk_difftest() {
    // The c>=0x100 branches (the high opener/closer tables + the CJK/Hangul/Kana script ranges) reach
    // `isspace` on a >=0x100 codepoint, whose CRT path is the runtime-locale `GetStringTypeA` (an
    // IAT/SEH call that cannot run in isolation). We stub `isspace`→0 — the value the kinsoku table is
    // written to assume for non-ASCII (matching the transcription's `isspace_c` model for c>=0x100) —
    // and drive only non-whitespace inputs, so the stub and the model agree by construction. This
    // validates the high jump-table case sets + the range boundaries; the isspace(c>=0x100) behaviour
    // itself is noted below.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(ISSPACE, 0, 0); // cdecl: caller cleans, return 0

    // High openers (cur >= 0x100): each must yield 0 (no break after). next is an in-range ideograph
    // (would be 1 if cur were NOT recognised as an opener — so a missed opener is caught).
    let high_openers: &[u32] = &[
        0x2018, 0x201c, 0x2035, 0x3008, 0x300a, 0x300c, 0x300e, 0x3010, 0x3014, 0x301d, 0xfe59,
        0xfe5b, 0xfe5d, 0xff04, 0xff08, 0xff3b, 0xff5b, 0xffe1, 0xffe5, 0xffe6,
    ];
    for &cur in high_openers {
        let bin = e.call(LINE_BREAK_CLASS, cur, 0x4e00, &[]);
        assert_eq!(
            bin,
            line_break_class(cur, 0x4e00),
            "lbc opener cur={cur:#x}"
        );
    }

    // High closers (next >= 0x100): cur is a non-opener ideograph; each closer must yield 0.
    let high_closers: &[u32] = &[
        0x2013, 0x2014, 0x2019, 0x201d, 0x2022, 0x2026, 0x2027, 0x2032, 0x2033, 0x2103, 0x3001,
        0x3002, 0x3009, 0x300b, 0x300d, 0x300f, 0x3011, 0x3015, 0x301e, 0x30fc, 0xfe30, 0xfe50,
        0xfe51, 0xfe52, 0xfe54, 0xfe55, 0xfe56, 0xfe57, 0xfe5a, 0xfe5c, 0xfe5e, 0xff01, 0xff05,
        0xff09, 0xff0c, 0xff0e, 0xff1a, 0xff1b, 0xff1f, 0xff3d, 0xff5d, 0xff70, 0xff9e, 0xff9f,
        0xffe0,
    ];
    for &nxt in high_closers {
        let bin = e.call(LINE_BREAK_CLASS, 0x4e51, nxt, &[]);
        assert_eq!(
            bin,
            line_break_class(0x4e51, nxt),
            "lbc closer next={nxt:#x}"
        );
    }

    // CJK/Hangul/Kana range boundaries (next), cur = a non-opener ideograph: just outside → not
    // breakable here, the start/mid/end → breakable (unless the value is also a closer).
    for &(lo, hi) in &[
        (0x1100u32, 0x11ffu32),
        (0x3000, 0xd7af),
        (0xf900, 0xfaff),
        (0xff00, 0xff9f),
        (0xffa0, 0xffdc),
    ] {
        for nxt in [lo.wrapping_sub(1), lo, (lo + hi) / 2, hi, hi + 1] {
            let bin = e.call(LINE_BREAK_CLASS, 0x4e51, nxt, &[]);
            assert_eq!(
                bin,
                line_break_class(0x4e51, nxt),
                "lbc range cur=0x4e51 next={nxt:#x}"
            );
        }
    }

    // A broad high-codepoint sweep (cur and next both high, non-whitespace) for anything missed.
    let mut s: u32 = 0x5c77_8001;
    for i in 0..4000u32 {
        let cur = 0x100 + (next(&mut s) >> 8) % 0xff00;
        let nxt = 0x100 + (next(&mut s) >> 8) % 0xff00;
        let bin = e.call(LINE_BREAK_CLASS, cur, nxt, &[]);
        assert_eq!(
            bin,
            line_break_class(cur, nxt),
            "lbc sweep[{i}] cur={cur:#x} next={nxt:#x}"
        );
    }
}

// ── strip_control_codes ────────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn check_strip(
    e: &mut Emu,
    src_ptr: u64,
    dst_ptr: u64,
    dst_room: usize,
    src: &[u8],
    mask: u32,
    dst_cap: i32,
    ctx: &str,
) {
    assert!(
        src.len() < 120 && (dst_cap as usize) <= dst_room,
        "strip case too big: {ctx}"
    );
    e.write(src_ptr, &[0u8; 128]);
    e.write(src_ptr, src);
    e.write(src_ptr, src); // (no-op clarity)
    e.write(src_ptr.wrapping_add(src.len() as u64), &[0u8]); // NUL
    let sentinel = vec![0xABu8; dst_room];
    e.write(dst_ptr, &sentinel);

    let ret = e.call(
        STRIP_CONTROL_CODES,
        src_ptr as u32,
        src.len() as u32,
        &[mask, dst_ptr as u32, dst_cap as u32],
    );

    let mut mine = vec![0xABu8; dst_room];
    let outcome = strip_control_codes(src, src.len() as i32, mask, &mut mine, dst_cap);
    match outcome {
        StripOutcome::Degenerate => { /* delegates to CRT SStrPrintf — not diffed (see module doc) */
        }
        StripOutcome::Stripped(n) => {
            assert_eq!(ret, dst_ptr as u32, "{ctx}: return = dst");
            let bin = e.read(dst_ptr, dst_room);
            assert_eq!(bin, mine, "{ctx}: dst bytes (n={n})");
        }
    }
}

#[test]
fn strip_control_codes_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let src = e.alloc(128);
    const ROOM: usize = 96;
    let dst = e.alloc(ROOM as u64);

    // Each strippable class kept (mask clear) vs stripped (mask set), for a generous dst_cap. Codes
    // 0/1/4 keep raw bytes; codes 2/3/5 emit the canonical replacement ("\n", "|", "|h").
    let big = ROOM as i32;
    let cases: &[(&[u8], u32, i32, &str)] = &[
        (b"plain text", 0, big, "plain"),
        (b"ab|cFF00FF00cd", 0x100, big, "strip color"),
        (b"ab|cFF00FF00cd", 0, big, "keep color raw"),
        (b"a|rb", 0x100, big, "strip restore"),
        (b"a|rb", 0, big, "keep restore raw"),
        (b"a|nb", 0x200, big, "strip newline"),
        (b"a|nb", 0, big, "newline -> \\n"),
        (b"a||b", 0x800, big, "strip escpipe"),
        (b"a||b", 0, big, "escpipe -> |"),
        (b"a|hb", 0x400, big, "strip linkclose"),
        (b"a|hb", 0, big, "linkclose -> |h"),
        (b"|Hk|hTEXT|hZ", 0x400, big, "strip link open+close"),
        (b"|Hk|hTEXT|hZ", 0, big, "keep link raw"),
        (b"a\r\nb", 0, big, "CRLF -> \\n"),
        (b"a\r\nb", 0x200, big, "strip CRLF"),
        (b"a\rb", 0, big, "CR -> \\n"),
        (b"a\nb", 0, big, "LF -> \\n"),
        (b"x|cFFFFFFFF|n|r|h||y", 0xf00, big, "strip all"),
        (b"x|cFFFFFFFF|n|r|h||y", 0, big, "keep all"),
        (b"|cZZ", 0, big, "malformed color kept as literal pipe"),
        (b"\xe2\x82\xacmid\xc3\xa9", 0, big, "multibyte literals"),
        // truncation against dst_cap (the unsigned out >= cap-clen guard, codes 6 / B / A).
        (b"abcdef", 0x100, 4, "truncate literals cap4"),
        (b"ab|ncd", 0x100, 3, "truncate before replacement cap3"),
        (b"ab|cFFFFFFFFcd", 0, 6, "truncate keep-raw color cap6"),
        (b"\nabc", 0x100, 2, "truncate at first emit cap2"),
        (b"abc", 4, "abc".len() as i32 + 1, "exact-fit cap"),
    ];
    for (s, mask, cap, ctx) in cases {
        check_strip(&mut e, src, dst, ROOM, s, *mask, *cap, ctx);
    }

    // Degenerate guard is a Rust-side check (delegates to CRT SStrPrintf in the binary).
    let mut tmp = [0xABu8; 8];
    assert_eq!(
        strip_control_codes(b"", 0, 0, &mut tmp, 8),
        StripOutcome::Degenerate
    );
    assert_eq!(
        strip_control_codes(b"abc", 0, 0, &mut tmp, 8),
        StripOutcome::Degenerate,
        "src_len 0"
    );

    // Fuzz over markup-shaped content with random masks and a range of caps.
    const ALPHA: &[u8] = b"abc |cFF00ff00|n|r|h||\r\n|Hk|hT|h";
    let mut st: u32 = 0x5c25_9001;
    for i in 0..3000u32 {
        let len = (next(&mut st) >> 27) as usize % 40;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(ALPHA[(next(&mut st) >> 8) as usize % ALPHA.len()]);
        }
        let mask = (next(&mut st) >> 20) & 0xf00;
        let cap = 8 + (next(&mut st) >> 24) as i32 % (ROOM as i32 - 8);
        check_strip(
            &mut e,
            src,
            dst,
            ROOM,
            &buf,
            mask,
            cap,
            &format!("fuzz[{i}]"),
        );
    }
}

// ── line_break_enumerate ───────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn check_enum(
    e: &mut Emu,
    font_ptr: u64,
    text_ptr: u64,
    out_ptr: u64,
    out_room: u32,
    text: &[u8],
    flags: u32,
    cap: u32,
    ctx: &str,
) {
    assert!(
        text.len() < 120 && cap <= out_room,
        "enum case too big: {ctx}"
    );
    e.write(text_ptr, &[0u8; 128]);
    e.write(text_ptr, text);
    let sentinel: Vec<u8> = (0..out_room)
        .flat_map(|_| 0x7eee_eeeeu32.to_le_bytes())
        .collect();
    e.write(out_ptr, &sentinel);

    let size = 0.5f32;
    let boxw = 0.5f32;
    let max_bytes = text.len() as i32;
    let ret = e.call(
        LINE_BREAK_ENUMERATE,
        font_ptr as u32,
        text_ptr as u32,
        &[
            max_bytes as u32,
            size.to_bits(),
            boxw.to_bits(),
            out_ptr as u32,
            cap,
            0,
            0,
            flags,
        ],
    );

    let mut mine = vec![0x7eee_eeeei32; out_room as usize];
    let mycount = line_break_enumerate(
        text,
        max_bytes,
        size,
        boxw,
        0.0,
        flags,
        &mut mine,
        cap,
        |_| panic!("{ctx}: layout kernel must not be reached on newline-only text"),
    );
    assert_eq!(ret, mycount, "{ctx}: count");
    let written = mycount.min(cap) as usize;
    for (i, &m) in mine.iter().enumerate().take(written) {
        assert_eq!(
            e.read_u32(out_ptr + 4 * i as u64),
            m as u32,
            "{ctx}: out[{i}]"
        );
    }
}

#[test]
fn line_break_enumerate_difftest() {
    // Newline-only text: every token decodes to a line break (code 2), so the layout kernel `0x5c7260`
    // is never invoked and the offset-recording, cap, newline-advance and trailing-LF logic runs
    // in isolation. (The kernel-driven wrap path is out of this module — see the module doc.)
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let font = e.alloc(0x250); // dummy CGxFont — never dereferenced for the newline path
    let text = e.alloc(128);
    const ROOM: u32 = 24;
    let out = e.alloc(ROOM as u64 * 4);

    // flags=0: |n decodes as a newline (code 2); raw CR/LF always do.
    let cases: &[(&[u8], u32, u32, &str)] = &[
        (b"\n", 0, 8, "single LF"),
        (b"\n\n", 0, 8, "LF LF (+trailing)"),
        (b"\n\n\n", 0, 8, "LF LF LF"),
        (b"\r\n", 0, 8, "CRLF"),
        (b"\r\n\r\n", 0, 8, "CRLF CRLF"),
        (b"\r", 0, 8, "bare CR (no trailing)"),
        (b"\r\r", 0, 8, "CR CR"),
        (b"|n|n", 0, 8, "|n |n"),
        (b"|n\n", 0, 8, "|n then LF (+trailing)"),
        (b"\n|n\r\n", 0, 8, "mixed newlines"),
        (b"\n\n\n\n", 0, 2, "cap=2 truncates recording"),
        (b"\n\n\n\n", 0, 1, "cap=1"),
        (b"\n\n\n\n", 0, 0, "cap=0 (count only)"),
        (b"\r\n\n\r\r\n\n", 0, 16, "long mixed"),
    ];
    for (s, flags, cap, ctx) in cases {
        check_enum(&mut e, font, text, out, ROOM, s, *flags, *cap, ctx);
    }

    // flags=0x800 gates the whole '|' family but leaves raw CR/LF as newlines (still kernel-free).
    check_enum(
        &mut e,
        font,
        text,
        out,
        ROOM,
        b"\r\n\n",
        0x800,
        8,
        "flags=0x800 CRLF/LF",
    );

    // Guards: max_bytes==0, size==0.0 → return 0 (no decode, no kernel).
    let ret0 = e.call(
        LINE_BREAK_ENUMERATE,
        font as u32,
        text as u32,
        &[
            0,
            0.5f32.to_bits(),
            0.5f32.to_bits(),
            out as u32,
            8,
            0,
            0,
            0,
        ],
    );
    assert_eq!(ret0, 0, "guard max_bytes=0");
    e.write(text, b"\n\n\0");
    let ret_sz = e.call(
        LINE_BREAK_ENUMERATE,
        font as u32,
        text as u32,
        &[
            2,
            0.0f32.to_bits(),
            0.5f32.to_bits(),
            out as u32,
            8,
            0,
            0,
            0,
        ],
    );
    assert_eq!(ret_sz, 0, "guard size=0.0");
    assert_eq!(
        line_break_enumerate(b"\n\n", 2, 0.0, 0.5, 0.0, 0, &mut [0i32; 8], 8, |_| 0),
        0,
        "guard size=0.0 (ours)"
    );
}
