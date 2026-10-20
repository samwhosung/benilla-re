//! Storm's ordered printf (`systems/storm/printf.md`): `vsnoprintf` (`0x6697d0`) and the parser it
//! calls, `ParseFormatSpecifier` (`0x669ab0`), with the two spec rewrites
//! `RemoveOrderingFromFormatSpecifier` (`0x669c50`) and `FixUpLongLongFormatSpecifier`
//! (`0x669cc0`), and the wrapper `ISStrVPrintf` (`0x64a820`).
//!
//! Each conversion is handed to the C runtime's `_snprintf` (`0x740107`); here that is a callback,
//! so the binding of `%N$` to the arguments, the rewrites and the cursor arithmetic are what this
//! reproduces. Memory is one byte slice the callback and the formatter share: the buffer lies at an
//! offset inside it, so a write before or past the buffer lands where the client's would.

/// How an argument is read off the `va_list` and pushed to `_snprintf`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgKind {
    /// 0: a 4-byte integer, the default and the kind of an index no conversion names.
    Int32,
    /// 1: a pointer, `p`, `P`, `s` or `S`; 4 bytes.
    Pointer,
    /// 2: an 8-byte integer, after `ll` or `I64`.
    Int64,
    /// 3: a double, `e`, `E`, `f`, `F`, `g` or `G`.
    Double,
}

impl ArgKind {
    fn from_code(code: u32) -> Option<ArgKind> {
        match code {
            0 => Some(ArgKind::Int32),
            1 => Some(ArgKind::Pointer),
            2 => Some(ArgKind::Int64),
            3 => Some(ArgKind::Double),
            _ => None,
        }
    }
}

/// The class of a conversion byte, `0x669bd4` for bytes up to `x` (0x78): 0 for NUL and `%`, 1
/// for `C D U X c d i u x`, 2 for `E F G e f g`, 3 for `I`, 4 for `P S p s`, 5 for `l`, 6 for the
/// rest. A byte above 0x78, or one that sign-extends negative, is class 6 by the unsigned bound.
#[must_use]
pub fn conversion_class(byte: u8) -> u8 {
    match byte {
        0 | b'%' => 0,
        b'C' | b'D' | b'U' | b'X' | b'c' | b'd' | b'i' | b'u' | b'x' => 1,
        b'E' | b'F' | b'G' | b'e' | b'f' | b'g' => 2,
        b'I' => 3,
        b'P' | b'S' | b'p' | b's' => 4,
        b'l' => 5,
        _ => 6,
    }
}

/// Whether the C runtime's `_isdigit` (`0x73fc0f`) calls `byte` a digit as Storm passes it,
/// sign-extended: the table lookup has no bound, so 0xf7, 0xfb and 0xfd read words below the
/// table whose bit 2 is set (`systems/storm/printf.md#bytes-that-count-as-digits`). 0xed and 0xee
/// read a heap pointer whose bits are clear at load and are taken as no digit.
#[must_use]
pub fn is_storm_digit(byte: u8) -> bool {
    byte.is_ascii_digit() || matches!(byte, 0xf7 | 0xfb | 0xfd)
}

/// What `ParseFormatSpecifier` (`0x669ab0`) leaves: the cursor after the spec, the kind code (4
/// for a spec that takes no argument) and whether it takes one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parsed {
    pub end: usize,
    pub kind: u32,
    pub takes_argument: bool,
}

/// `ParseFormatSpecifier` (`0x669ab0`), from the byte after a `%`. Digits ([`is_storm_digit`])
/// accumulate `n = 10n + c − 0x30` with `c` sign-extended (`0x669ae9`) and a `$` sets `index` to `n − 1`, or 0 for `n` of 0 (`0x669af2`), without
/// clearing `n`; any other byte clears it (`0x669b12`) and is classed. `ll` and `I64` set kind 2
/// and are skipped; the classes that end the spec are 0 (no argument), 1, 2 and 4. `fmt` reads as
/// zero past its end, where the client reads the bytes that follow the string.
pub fn parse_format_specifier(fmt: &[u8], start: usize, index: &mut i32) -> Parsed {
    let at = |p: usize| fmt.get(p).copied().unwrap_or(0);
    let mut p = start;
    let mut n: i32 = 0;
    let mut kind = 0u32;
    loop {
        let c = at(p);
        p += 1;
        if is_storm_digit(c) {
            n = n.wrapping_mul(10).wrapping_add(i32::from(c as i8) - 0x30);
            continue;
        }
        if c == b'$' {
            *index = if n != 0 { n.wrapping_sub(1) } else { 0 };
            continue;
        }
        n = 0;
        match conversion_class(c) {
            0 => {
                return Parsed {
                    end: p,
                    kind: 4,
                    takes_argument: false,
                }
            }
            1 => {
                return Parsed {
                    end: p,
                    kind,
                    takes_argument: true,
                }
            }
            2 => {
                return Parsed {
                    end: p,
                    kind: 3,
                    takes_argument: true,
                }
            }
            3 if at(p) == b'6' && at(p + 1) == b'4' => {
                p += 2;
                kind = 2;
            }
            4 => {
                return Parsed {
                    end: p,
                    kind: 1,
                    takes_argument: true,
                }
            }
            5 if at(p) == b'l' => {
                p += 1;
                kind = 2;
            }
            _ => {}
        }
    }
}

/// `RemoveOrderingFromFormatSpecifier` (`0x669c50`): the first `$` and the digits
/// ([`is_storm_digit`]) right before it are cut out of the NUL-terminated spec.
pub fn remove_ordering(spec: &mut Vec<u8>) {
    let Some(dollar) = spec.iter().position(|&b| b == b'$') else {
        return;
    };
    let mut first = dollar;
    while first > 0 && is_storm_digit(spec[first - 1]) {
        first -= 1;
    }
    spec.drain(first..=dollar);
}

/// `FixUpLongLongFormatSpecifier` (`0x669cc0`): the first `ll` becomes `I64`.
pub fn fix_long_long(spec: &mut Vec<u8>) {
    if let Some(at) = spec.windows(2).position(|w| w == b"ll") {
        spec.splice(at..at + 2, *b"I64");
    }
}

/// The C runtime's `_snprintf` as [`vsnoprintf`] calls it: the shared memory, the write position,
/// the room left, the rewritten spec and the dwords pushed after it; it returns `_snprintf`'s result.
pub type Snprintf<'a> = dyn FnMut(&mut [u8], i64, i32, &[u8], &[u32]) -> i32 + 'a;

/// One spec as the first pass records it: where it starts, its length counted from its `%`, and
/// the argument index it reads.
#[derive(Clone, Copy, Debug)]
struct Spec {
    start: usize,
    len: usize,
    index: i32,
}

/// `vsnoprintf` (`0x6697d0`), `cdecl(dest, size, fmt, va)`: formats `fmt` into `mem[dest..]`
/// through `snprintf` and returns the client's result, the write position less `dest`.
///
/// The first pass records each spec and, for a spec that takes an argument, stores its kind at its
/// index (the last spec to name an index decides it) and moves the next index past it, so a plain
/// spec after `%N$` reads argument `N + 1`. The second pass reads arguments `0` to the highest
/// index named, in order, by kind: an index no spec names counts as a 4-byte integer, and a double
/// passes through `fld`/`fstp`, which quiets a signalling NaN. The third pass copies literal bytes
/// while the position is below `dest + size − 1` and hands each spec, with its `N$` cut and `ll`
/// made `I64` for kind 2, to `snprintf(mem, position, room, spec, pushed)`, where `room` is the
/// bytes left including the last one and `pushed` the one or two dwords of its argument; the
/// position then moves by the return, `-1` included. It ends with a NUL at the position.
///
/// `args` is the `va_list` as dwords. The tables are the client's 256 entries; past them the
/// client overruns its frame, which this does not model, so the indices must stay below 256.
pub fn vsnoprintf(
    mem: &mut [u8],
    dest: usize,
    size: u32,
    fmt: &[u8],
    args: &[u32],
    snprintf: &mut Snprintf<'_>,
) -> i32 {
    let at = |p: usize| fmt.get(p).copied().unwrap_or(0);
    // Pass 1 (`0x669850` to `0x66989d`).
    let mut specs: Vec<Spec> = Vec::new();
    let mut kinds = [0u32; 256];
    let mut next: i32 = 0;
    let mut count: i32 = 0;
    let mut cur = 0usize;
    loop {
        let c = at(cur);
        cur += 1;
        if c == b'%' {
            let start = cur - 1;
            let parsed = parse_format_specifier(fmt, cur, &mut next);
            cur = parsed.end;
            specs.push(Spec {
                start,
                len: cur - start,
                index: next,
            });
            if parsed.takes_argument {
                kinds[next as usize] = parsed.kind;
                next += 1;
                if count < next {
                    count = next;
                }
            }
        } else if c == 0 {
            break;
        }
    }
    // Pass 2 (`0x6698a3` to `0x669919`): one value per index below `count`.
    let mut values = [[0u32; 2]; 256];
    let mut va = 0usize;
    let arg = |i: usize| args.get(i).copied().unwrap_or(0);
    for (i, value) in values.iter_mut().enumerate().take(count.max(0) as usize) {
        match kinds[i] {
            0 | 1 => {
                value[0] = arg(va);
                va += 1;
            }
            2 => {
                *value = [arg(va), arg(va + 1)];
                va += 2;
            }
            3 => {
                let bits = u64::from(arg(va)) | (u64::from(arg(va + 1)) << 32);
                let quiet = if f64::from_bits(bits).is_nan() {
                    bits | (1 << 51)
                } else {
                    bits
                };
                *value = [quiet as u32, (quiet >> 32) as u32];
                va += 2;
            }
            _ => *value = [0, 0],
        }
    }
    // Pass 3 (`0x66991e` to `0x669a69`); positions are offsets from `mem`'s start.
    let end = dest as i64 + i64::from(size) - 1;
    let mut pos = dest as i64;
    let mut f = 0usize;
    let mut spec_at = 0usize;
    if pos < end {
        loop {
            let c = at(f);
            f += 1;
            if c == 0 {
                break;
            }
            if c != b'%' {
                mem[pos as usize] = c;
                pos += 1;
            } else {
                let spec = specs[spec_at];
                spec_at += 1;
                let mut text: Vec<u8> = (spec.start..spec.start + spec.len).map(at).collect();
                // The copy is NUL-terminated at its length, so text from an earlier NUL on is cut.
                if let Some(nul) = text.iter().position(|&b| b == 0) {
                    text.truncate(nul);
                }
                remove_ordering(&mut text);
                let index = spec.index as usize;
                let kind = kinds[index];
                if kind == 2 {
                    fix_long_long(&mut text);
                }
                let room = (end - pos + 1) as i32;
                let value = values[index];
                let written = match ArgKind::from_code(kind) {
                    Some(ArgKind::Int32 | ArgKind::Pointer) => {
                        snprintf(mem, pos, room, &text, &value[..1])
                    }
                    Some(ArgKind::Int64 | ArgKind::Double) => {
                        snprintf(mem, pos, room, &text, &value)
                    }
                    None => 0,
                };
                pos += i64::from(written);
                f = f + spec.len - 1;
            }
            if pos >= end {
                break;
            }
        }
    }
    mem[pos as usize] = 0;
    (pos - dest as i64) as i32
}

/// `ISStrVPrintf` (`0x64a820`) with the ordered formatter selected (`[0xc53f10]` nonzero): a size
/// of 0 returns 0 and writes nothing; `0x7fffffff` runs `vsoprintf` (`0x669d10`), which is
/// [`vsnoprintf`] with a size of 2^20, and returns its result as it is; any other size runs
/// [`vsnoprintf`] and, when its result is not below the size, writes a NUL at `size − 1` and
/// returns `size − 1`.
pub fn isstrvprintf(
    mem: &mut [u8],
    dest: usize,
    size: u32,
    fmt: &[u8],
    args: &[u32],
    snprintf: &mut Snprintf<'_>,
) -> u32 {
    if size == 0 {
        return 0;
    }
    if size == 0x7fff_ffff {
        return vsnoprintf(mem, dest, 0x10_0000, fmt, args, snprintf) as u32;
    }
    let written = vsnoprintf(mem, dest, size, fmt, args, snprintf) as u32;
    if written < size {
        written
    } else {
        mem[dest + size as usize - 1] = 0;
        size - 1
    }
}
