//! The control-code decoder and the line-break logic.
//!
//! Four integer and string functions with no floating point:
//!
//! - [`decode_quoted_code`] (`0x5c2810`): decodes one markup token at the cursor: `|c`AARRGGBB colour,
//!   `|H`...`|h` hyperlink, `|n`, `|r`, `|h`, `||`, and raw CR, LF and CRLF; returns a code class 0..6
//!   plus the consumed byte length, the parsed codepoint, and (for colour) the packed ARGB. Drives the
//!   UTF-8 next-char decoder (`0x41aab0`) and `strtoul` base 16 (`0x40daf2`), both transcribed below.
//! - [`line_break_class`] (`0x5c7780`): the kinsoku line-break-opportunity classifier over a codepoint
//!   pair.
//! - [`line_break_enumerate`] (`0x5c2430`): fills an array of per-line start offsets (it delegates the
//!   line measure to the layout kernel `0x5c7260`, taken here as the `lay_line` callback).
//! - [`strip_control_codes`] (`0x5c2590`): strips and unescapes markup, table-driven on `0x85f4d8`.
//!
//! These are integer, pointer and string logic with no floating-point operations in the disassembly;
//! the transcription is the exact byte and return-code behaviour.

// ── helpers transcribed from the CRT callees the decoder runs ──────────────────────────────────

/// `0x41aab0` — UTF-8 next-char decode. Reads the codepoint starting at `text[pos]` and returns
/// `(codepoint, bytes_consumed)`. Returns `0xffff_ffff` on a NUL lead byte or a truncated multibyte
/// (a continuation byte that is NUL — `bytes_consumed` counts only the bytes actually read), and
/// `0x8000_0000` when a lead/continuation byte violates the encoding. `bytes_consumed` is the number
/// of bytes read (1 lead + each non-NUL continuation), exactly as the binary increments its `*len`.
/// Reads past the slice end are treated as NUL (the binary reads a NUL-terminated C string).
fn utf8_next(text: &[u8], pos: usize) -> (u32, i32) {
    let b = |i: usize| -> u32 { u32::from(text.get(pos + i).copied().unwrap_or(0)) };
    let lead = b(0);
    if lead == 0 {
        return (0xffff_ffff, 0); // 41aad8: lead == 0 → -1, *len untouched (=0)
    }
    // 41aaea..41ab56: classify the lead byte → (continuation count, initial accumulator).
    let (n, mut acc): (i32, u32) = if lead & 0xfe == 0xfc {
        (5, lead & 0x01)
    } else if lead & 0xfc == 0xf8 {
        (4, lead & 0x03)
    } else if lead & 0xf8 == 0xf0 {
        (3, lead & 0x07)
    } else if lead & 0xf0 == 0xe0 {
        (2, lead & 0x0f)
    } else if lead & 0xe0 == 0xc0 {
        (1, lead & 0x1f)
    } else if lead & 0x80 == 0x80 {
        return (0x8000_0000, 1); // 41aba4: continuation byte as lead → invalid, *len=1
    } else {
        return (lead, 1); // 41aba9: ASCII (0x01..0x7f)
    };
    let mut len: i32 = 1;
    let mut count: i32 = 0;
    while count < n {
        let cb = b((count + 1) as usize);
        // 41ab69: NUL continuation → -1, *len NOT incremented for the NUL.
        if cb == 0 {
            return (0xffff_ffff, len);
        }
        len += 1; // 41ab75: *len++
                  // 41ab7f: a non-10xxxxxx continuation → invalid (*len already incremented).
        if cb & 0xc0 != 0x80 {
            return (0x8000_0000, len);
        }
        acc = (acc << 6) | (cb & 0x3f);
        count += 1;
    }
    (acc, len)
}

/// `0x40daf2`/`strtoul(buf, &end, 16)` restricted to the markup colour domain: `buf` is exactly two
/// non-NUL bytes (`hi`, `lo`) followed by NUL (the binary's 2-char hex scratch buffer). Returns
/// `Some((hi*16 + lo) as u8)` when both bytes are ASCII hex digits (`end` lands on the NUL — no parse
/// error), `None` when either is not (`*end != 0` — the decoder's parse-error path). The hex-digit test
/// and value match the CRT's ctype `_DIGIT`/alpha-`toupper`-`-0x37` path for `[0-9a-fA-F]`.
fn parse_hex_pair(hi: u8, lo: u8) -> Option<u8> {
    let v = |c: u8| -> Option<u32> {
        match c {
            b'0'..=b'9' => Some(u32::from(c - b'0')),
            b'a'..=b'f' => Some(u32::from(c - b'a') + 10),
            b'A'..=b'F' => Some(u32::from(c - b'A') + 10),
            _ => None,
        }
    };
    match (v(hi), v(lo)) {
        (Some(h), Some(l)) => Some((h * 16 + l) as u8),
        _ => None,
    }
}

// ── 0x5c2810 — decode_quoted_code ──────────────────────────────────────────────────────────────

/// One decoded markup token (the binary's return code + the values it writes through its out-pointers).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decoded {
    /// Return code: 0=colour, 1=restore, 2=newline, 3=escaped pipe, 4=link-open, 5=link-close,
    /// 6=literal/normal glyph (also every malformed/gate-suppressed case).
    pub class: u32,
    /// `*outConsumedLen` — bytes consumed by this token.
    pub consumed: i32,
    /// `*outCode` — the decoded codepoint (the first char's codepoint in every path; `0x7c` for any
    /// `|`-sequence, the glyph codepoint for a literal). Raw 32-bit (`0xffff_ffff`/`0x8000_0000` for
    /// the empty/invalid UTF-8 edges).
    pub out_code: u32,
    /// `*outColor` — packed `0xFF_RR_GG_BB` (forced full alpha); only meaningful when `class == 0`.
    pub color: u32,
}

/// `0x5c2810` (`0x5c2810`) — decode one markup token at the cursor `text[0..]` under `flags`.
///
/// `flags` gate-bits suppress sequences (clear = active): `0x100` suppresses `|c`/`|C` colour and `|r`/`|R`
/// restore; `0x200` suppresses `|n`/`|N`; `0x400` suppresses `|H` open and `|h` close; `0x800` suppresses
/// the whole `|` family (a `|` then becomes a literal). `text` must be NUL-terminated within the slice
/// (the `|`-scanning paths read forward to the terminator, exactly as the binary reads a C string).
pub fn decode_quoted_code(text: &[u8], flags: u32) -> Decoded {
    let gate_n = flags & 0x200; // |n/|N
    let gate_h = flags & 0x400; // |H / |h
    let gate_pipe = flags & 0x800; // whole | family
    let gate_c = flags & 0x100; // |c/|C and |r/|R

    let (cp0, first_len) = utf8_next(text, 0); // 5c284b: decode the first char
    let out_code = cp0; // *arg5 = cp0 (every path leaves/restores this; 0x7c for | paths)

    // 5c285f / 5c2868: NUL or invalid-empty lead → literal.
    if cp0 == 0 || cp0 == 0xffff_ffff {
        return lit(first_len, out_code);
    }
    // 5c286e: CR → line break; CRLF consumes 2 (the binary peeks the char after the CR).
    if cp0 == 0x0d {
        let (cp1, _) = utf8_next(text, first_len as usize);
        let consumed = if cp1 == 0x0a { 2 } else { 1 };
        return nl(consumed, out_code);
    }
    // 5c2896: LF → line break, consumes 1.
    if cp0 == 0x0a {
        return nl(1, out_code);
    }
    // 5c28af / 5c28b8: not a '|', or the '|' family is gated off → literal.
    if cp0 != 0x7c || gate_pipe != 0 {
        return lit(first_len, out_code);
    }

    // '|' opener: look at the byte after the pipe (5c28c2: esi += first_len; cl = *esi).
    let after = first_len as usize; // index of the char following '|'
    let nextc = text.get(after).copied().unwrap_or(0);
    if nextc == 0 {
        // 5c28c8: nothing after the '|' → *arg5=0x7c, consumed=first_len, literal.
        return lit(first_len, out_code);
    }
    match nextc {
        b'C' | b'c' if gate_c == 0 => decode_color(text, after, first_len, out_code),
        b'H' if gate_h == 0 => decode_link(text, first_len, out_code),
        b'N' | b'n' if gate_n == 0 => nl(2, out_code), // |n/|N → newline, consumes 2
        b'R' | b'r' if gate_c == 0 => Decoded {
            class: 1, // |r/|R → colour restore
            consumed: 2,
            out_code,
            color: 0,
        },
        b'h' if gate_h == 0 => Decoded {
            class: 5, // |h → link close
            consumed: 2,
            out_code,
            color: 0,
        },
        b'|' => Decoded {
            class: 3, // || → escaped literal pipe
            consumed: 2,
            out_code,
            color: 0,
        },
        // default / gate-suppressed: 5c2ae1 — *arg5=0x7c, consumed=first_len, literal.
        _ => lit(first_len, out_code),
    }
}

/// 5c2ae3 / 5c2ae1: the literal/normal-glyph return (code 6).
#[inline]
fn lit(consumed: i32, out_code: u32) -> Decoded {
    Decoded {
        class: 6,
        consumed,
        out_code,
        color: 0,
    }
}

/// 5c2885 / 5c28a3: a newline return (code 2).
#[inline]
fn nl(consumed: i32, out_code: u32) -> Decoded {
    Decoded {
        class: 2,
        consumed,
        out_code,
        color: 0,
    }
}

/// `|c`/`|C` colour handler (5c2a36). `after` = index of the `C`/`c`; the 8 hex digits begin at
/// `after + 1`. Collects four bytes (two hex digits each); on a NUL before a full pair (truncation,
/// 5c291e) or a non-hex pair (`strtoul` parse-error, 5c2a95→5c299d) returns a literal (consumed =
/// `first_len`). On success: `*arg2=10`, packed colour `0xFF_RR_GG_BB` (alpha byte b0 discarded,
/// full alpha forced), code 0.
fn decode_color(text: &[u8], after: usize, first_len: i32, out_code: u32) -> Decoded {
    let mut bytes = [0u8; 4];
    let mut nbytes = 0usize;
    let mut i = 0usize; // offset into the hex run
    loop {
        let c0 = text.get(after + 1 + i).copied().unwrap_or(0);
        let c1 = text.get(after + 2 + i).copied().unwrap_or(0);
        if c0 == 0 || c1 == 0 {
            return lit(first_len, out_code); // 5c2a55/5c2a60 → 5c291e: truncated
        }
        i += 2;
        match parse_hex_pair(c0, c1) {
            None => return lit(first_len, out_code), // 5c2a95 → 5c299d: parse error
            Some(v) => {
                bytes[nbytes] = v;
                nbytes += 1;
                if nbytes >= 4 {
                    // 5c2ab2..5c2ace: result = 0xFF<<24 | b1<<16 | b2<<8 | b3.
                    let color = 0xff00_0000
                        | (u32::from(bytes[1]) << 16)
                        | (u32::from(bytes[2]) << 8)
                        | u32::from(bytes[3]);
                    return Decoded {
                        class: 0,
                        consumed: 10,
                        out_code,
                        color,
                    };
                }
            }
        }
    }
}

/// `|H`…`|h` hyperlink handler (5c28eb). `first_len` is the `|`'s byte length (=1); the `H` is at index
/// `first_len`. Scans (UTF-8 codepoint by codepoint) to the closing `|`, requires the following char to
/// be `h` with a non-NUL byte after it, rejects an empty link (`|H|h`, total length 4) and a doubled
/// close (`…|h|h`). Success → code 4 with `*arg2` = the full byte length of `|H…|h`; every malformed
/// case → literal (consumed = `first_len`).
fn decode_link(text: &[u8], first_len: i32, out_code: u32) -> Decoded {
    let byte = |i: usize| -> u32 { u32::from(text.get(i).copied().unwrap_or(0)) };
    let mut j = first_len as usize + 1; // 5c28f9/5c28fc: index of the char after 'H'
    if byte(j) == 0 {
        return lit(first_len, out_code); // 5c2900 → 5c291e: "|H" then NUL
    }
    // 5c2902 scan loop: advance to the closing '|'.
    loop {
        let (cp, len) = utf8_next(text, j);
        j += len as usize; // 5c290b: esi += *arg2
        if cp == 0x7c {
            break; // found the closing pipe; j now points just past it
        }
        if byte(j) == 0 {
            return lit(first_len, out_code); // 5c2918: NUL before a closing '|'
        }
    }
    // 5c2933: the char right after the closing '|'.
    if byte(j) == 0 {
        return lit(first_len, out_code); // 5c2937: '|' was last
    }
    let (cp2, len2) = utf8_next(text, j); // 5c2956: decode the char after the pipe
    let cl = byte(j + len2 as usize); // 5c295d: byte after that char
    j += len2 as usize; // 5c2960: esi += len2
    if cl == 0 {
        return lit(first_len, out_code); // 5c2964
    }
    if cp2 != 0x68 {
        return lit(first_len, out_code); // 5c2969: char after '|' is not 'h'
    }
    let consumed = j; // 5c2970: esi - base, base = text start
    if consumed == 4 {
        return lit(first_len, out_code); // 5c2977: empty link "|H|h"
    }
    if cl == 0x7c && byte(j + 1) == 0x68 {
        return lit(first_len, out_code); // 5c2992/5c2997 → 5c299d: a doubled "|h" follows
    }
    Decoded {
        class: 4,
        consumed: consumed as i32, // 5c29b8: *arg2 = full token length
        out_code,
        color: 0,
    }
}

// ── 0x5c7780 — line_break_class ────────────────────────────────────────────────────────────────

/// C-locale `isspace` (`0x74006f` = `_isctype(c, _SPACE)`). For `c & 0xffff < 0x100` the binary
/// indexes the statically-initialised CRT ctype table — whose `_SPACE` set is exactly `{0x09,0x0a,0x0b,
/// 0x0c,0x0d,0x20}`. For `c >= 0x100` it takes a runtime-locale path (`GetStringTypeA`); modelled here
/// as **false** (the value the kinsoku table's CJK branches assume; the test's `cjk` group covers them). The function masks `c` to 16 bits before the table lookup.
fn isspace_c(c: u32) -> bool {
    matches!(c & 0xffff, 0x09..=0x0d | 0x20)
}

/// `0x5c7780` (`0x5c7780`) — kinsoku line-break-opportunity classifier. Returns 1 if a line break is
/// permitted between `cur` and `next`, else 0. `next == 0xffff_ffff` is the "no next char" sentinel.
///
/// Order (exactly the binary's): a few ASCII quick-yeses, the whitespace rules, the "no-break-after"
/// opener set tested on `cur`, the "no-break-before" closer set tested on `next`, then the CJK/Hangul/Kana
/// script ranges tested on `next`. The opener/closer sets are the decoded jump-table case sets
/// (cross-checked against `0x5c7a2c`…`0x5c7ca8`).
pub fn line_break_class(cur: u32, next: u32) -> u32 {
    if cur == 0 {
        return 0; // 5c7785
    }
    if cur == 0x2d || cur == 0x3a || cur == 0x2f {
        return 1; // 5c778e/5c779b/5c77a8: '-', ':', '/'
    }
    if next == 0xffff_ffff {
        return 1; // 5c77b5
    }
    if isspace_c(cur) {
        return 0; // 5c77c3
    }
    if isspace_c(next) {
        return 1; // 5c77d4
    }
    if CUR_OPENERS.contains(&cur) {
        return 0; // no break after an opener
    }
    if NEXT_CLOSERS.contains(&next) {
        return 0; // no break before a closer
    }
    if cjk_breakable(next) {
        return 1; // CJK/Hangul/Kana script ranges
    }
    0 // 5c785f default
}

/// "No-break-after" openers tested on `cur` (5c77e8 if-tree + jump tables `0x5c7a2c`/`0x5c7acc`).
const CUR_OPENERS: [u32; 25] = [
    0x24, 0x28, 0x5b, 0x5c, 0x7b, 0x2018, 0x201c, 0x2035, 0x3008, 0x300a, 0x300c, 0x300e, 0x3010,
    0x3014, 0x301d, 0xfe59, 0xfe5b, 0xfe5d, 0xff04, 0xff08, 0xff3b, 0xff5b, 0xffe1, 0xffe5, 0xffe6,
];

/// "No-break-before" closers tested on `next` (5c7814 if-tree + jump tables `0x5c7a2c`, `0x5c7bb4`,
/// `0x5c7bd8`, `0x5c7bf0`, `0x5c7ca8`).
const NEXT_CLOSERS: [u32; 57] = [
    // T2 [0x21..0xb7]
    0x21, 0x25, 0x29, 0x2c, 0x2e, 0x3a, 0x3b, 0x3f, 0x5d, 0x7d, 0xb0, 0xb7, //
    0x2013, 0x2014, // [0x2013,0x2014]
    // T3 [0x2019..0x2033]
    0x2019, 0x201d, 0x2022, 0x2026, 0x2027, 0x2032, 0x2033, //
    0x2103, //
    // T4 [0x3001..0x300d]
    0x3001, 0x3002, 0x3009, 0x300b, 0x300d, //
    0x300f, 0x3011, 0x3015, 0x301e, 0x30fc, 0xfe30, //
    0xfe50, 0xfe51, 0xfe52, // [0xfe50,0xfe52]
    // T5 [0xfe54..0xff01]
    0xfe54, 0xfe55, 0xfe56, 0xfe57, 0xfe5a, 0xfe5c, 0xfe5e, 0xff01, //
    0xff05, //
    // T6 [0xff09..0xffe0]
    0xff09, 0xff0c, 0xff0e, 0xff1a, 0xff1b, 0xff1f, 0xff3d, 0xff5d, 0xff70, 0xff9e, 0xff9f, 0xffe0,
];

/// CJK/Hangul-Jamo/Kana/compat/halfwidth+fullwidth script ranges tested on `next` (5c7974).
fn cjk_breakable(next: u32) -> bool {
    matches!(next,
        0x1100..=0x11ff | 0x3000..=0xd7af | 0xf900..=0xfaff | 0xff00..=0xff9f | 0xffa0..=0xffdc)
}

// ── 0x5c2590 — strip_control_codes ─────────────────────────────────────────────────────────────

/// The `0x85f4d8` per-class table (7 classes × 3 dwords): `(keep/skip mask, replacement char,
/// emit-leading-'|' flag)`. Index by token code 0..6. Codes 0/1/4 (colour/restore/link-open) consult
/// only the mask (kept verbatim when the mask bit is clear); codes 2/3/5 (newline/escpipe/link-close)
/// emit the canonical replacement (with optional `|` prefix) when the mask bit is clear; code 6 is the
/// literal run (always copied, mask 0). Bytes read at file offset `0x45f4d8`.
const STRIP_CLASS: [(u32, u8, bool); 7] = [
    (0x100, b'C', true),   // 0 colour
    (0x100, b'R', true),   // 1 restore
    (0x200, b'\n', false), // 2 newline   → "\n"
    (0x800, b'|', false),  // 3 esc-pipe   → "|"
    (0x400, b'H', true),   // 4 link-open
    (0x400, b'h', true),   // 5 link-close → "|h"
    (0x000, b'-', true),   // 6 literal (table present but unused: code 6 copies the raw run)
];

/// `0x5c2590` (`0x5c2590`) — strip/unescape markup from `src[0..src_len]` into `dst`, NUL-terminating.
/// `mask` selects which classes to remove (the `flags` bits `0x100/0x200/0x400/0x800`). Decodes with
/// gate-flags = 0 (every sequence active). Returns the number of bytes written (the index of the NUL).
/// `dst` must hold at least `dst_cap` bytes (the binary's bound). The degenerate guard (empty `src` or
/// `src_len == 0`) delegates to the CRT `SStrPrintf(dst, dst_cap, "%s", src)` and is reported by
/// [`StripOutcome::Degenerate`]; the test covers the table-driven main path.
pub fn strip_control_codes(
    src: &[u8],
    src_len: i32,
    mask: u32,
    dst: &mut [u8],
    dst_cap: i32,
) -> StripOutcome {
    let byte = |i: usize| -> u8 { src.get(i).copied().unwrap_or(0) };
    if !dst.is_empty() {
        dst[0] = 0; // 5c259f: *dst = 0
    }
    // 5c259d..5c25b4: degenerate guards.
    if src.is_empty() || byte(0) == 0 || src_len == 0 {
        return StripOutcome::Degenerate;
    }
    // overflow guard (5c2621/5c265c/5c2680): out >= dst_cap - clen, UNSIGNED.
    let overflow = |out: usize, clen: i32| -> bool {
        (out as u32) >= (dst_cap as u32).wrapping_sub(clen as u32)
    };
    let mut out = 0usize; // esi
    let mut cur = 0usize; // edi - src
    let mut remaining = src_len; // [ebp+0xc]
    loop {
        if byte(cur) == 0 || remaining == 0 {
            break; // 5c26a4 / 5c25cb
        }
        let token_start = cur; // [ebp-8]
        let dec = decode_quoted_code(&src[cur..], 0); // 5c25e3: flags = 0
        let code = dec.class;
        let clen = dec.consumed;
        match code {
            0 | 1 | 4 => {
                // 5c25fb: keep verbatim unless this class's mask bit is set.
                if STRIP_CLASS[code as usize].0 & mask != 0 {
                    cur += clen as usize; // 5c260a: SKIP
                    remaining -= clen;
                } else {
                    if overflow(out, clen) {
                        break; // 5c2623 → 5c267b → 5c26ad: truncate, done
                    }
                    if clen > 0 {
                        for k in 0..clen as usize {
                            dst[out] = byte(cur + k);
                            out += 1;
                        }
                    }
                    cur += clen as usize;
                    remaining -= clen;
                }
            }
            2 | 3 | 5 => {
                // 5c263c: advance first, then emit the canonical replacement unless masked.
                remaining -= clen;
                cur += clen as usize;
                let (cmask, repl, prefix) = STRIP_CLASS[code as usize];
                if cmask & mask == 0 {
                    if overflow(out, clen) {
                        break; // 5c265e → 5c267b → 5c26ad
                    }
                    if prefix {
                        dst[out] = b'|';
                        out += 1;
                    }
                    dst[out] = repl;
                    out += 1;
                }
            }
            _ => {
                // 5c267b: code 6 literal — copy the raw token run.
                if overflow(out, clen) {
                    break;
                }
                if clen > 0 {
                    for k in 0..clen as usize {
                        dst[out] = byte(token_start + k);
                        out += 1;
                    }
                }
                cur += clen as usize;
                remaining -= clen;
            }
        }
    }
    dst[out] = 0; // 5c26ad
    StripOutcome::Stripped(out)
}

/// The result of [`strip_control_codes`]: the main path's written byte count, or the degenerate guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StripOutcome {
    /// Main path: `dst` holds the stripped string; payload = bytes written before the NUL.
    Stripped(usize),
    /// `src` empty/`src_len == 0`: the binary copies `src` into `dst` via `SStrPrintf("%s")`.
    Degenerate,
}

// ── 0x5c2430 — line_break_enumerate ────────────────────────────────────────────────────────────

/// `0x5c2430` (`0x5c2430`) — fill `out` with the byte offset (from `text` start) of each line break
/// point in `text[0..max_bytes]`, returning the count.
///
/// Each iteration records the current offset (capped by `out.len().min(cap)`), then decodes one token:
/// a newline (code 2) advances the cursor by the token's consumed length; any other token delegates to
/// the line-layout kernel `0x5c7260` — supplied here as `lay_line(cursor) -> next_cursor` — and the
/// cursor jumps to its result (a `next == cursor` no-progress result ends the walk). A trailing LF byte
/// at `text[max_bytes-1]` appends one extra offset (`= max_bytes`). `size`/`box_width` are validated
/// (both `!= 0.0`, else 0 is returned) and otherwise only feed the kernel; when `flags & 4` the binary
/// substitutes the font default size (`default_size`, = `0x5c2540`).
///
/// **Test scope:** the kernel (`lay_line`) is a separate, FreeType-backed cluster not in this module,
/// so the test exercises only the newline, offset, cap and trailing-LF path (where `lay_line` is never
/// invoked); the kernel-driven wrap path is not covered.
#[allow(clippy::too_many_arguments)]
pub fn line_break_enumerate(
    text: &[u8],
    max_bytes: i32,
    size: f32,
    box_width: f32,
    default_size: f32,
    flags: u32,
    out: &mut [i32],
    cap: u32,
    mut lay_line: impl FnMut(usize) -> usize,
) -> u32 {
    // 5c2439..5c247a: guards.
    if text.is_empty() || text[0] == 0 || max_bytes == 0 || size == 0.0 || box_width == 0.0 {
        return 0;
    }
    let mut size = size;
    if flags & 4 != 0 {
        size = default_size; // 5c248a: size = 0x5c2540()
    }
    let _ = size; // consumed only by the kernel (lay_line) in real use

    let end = max_bytes as usize; // text + max_bytes
    let mut cursor = 0usize; // esi - text (line_start is fixed at 0, never updated)
    let mut count = 0u32; // edi
    loop {
        if cursor >= end {
            break; // 5c24a4
        }
        if count < cap {
            out[count as usize] = cursor as i32; // 5c24ae: out[count] = cursor - line_start(0)
        }
        count += 1; // 5c24c7
        let dec = decode_quoted_code(&text[cursor..], flags); // 5c24c8
        if dec.class == 2 {
            cursor += dec.consumed as usize; // 5c24d2: newline advance
        } else {
            let next = lay_line(cursor); // 5c24f9: 0x5c7260 → outNext
            if cursor == next {
                break; // 5c2503: no progress
            }
            cursor = next;
        }
        if cursor == 0 {
            break; // 5c2507: pointer-non-null loop guard (never trips for a real cursor)
        }
    }
    // 5c250e: a trailing LF appends one extra offset (= max_bytes).
    if text.get(end.wrapping_sub(1)).copied().unwrap_or(0) == b'\n' {
        if count < cap {
            out[count as usize] = max_bytes;
        }
        count += 1;
    }
    count
}
