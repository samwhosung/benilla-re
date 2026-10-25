//! Reimplements the arithmetic of the movie and animated-texture functions `0x78c360`, `0x78cf30`
//! and `0x78d1a0`, and the whole of `0x78c360`, the movie file reader. The x87 runs at 53-bit
//! precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests in
//! `tests/movieframe.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact; cited by VA).
// ===========================================================================================

/// `[0x801360]` (`.rdata`) = `0.0010000000474974513` as a **dword** `f32` (`0x3a83126f`), the
/// 0x78cf30 milliseconds→seconds time multiplier (`fmul dword [0x801360]`). Widened to f64.
pub const MS_TO_S_F32_BITS: u32 = 0x3a83_126f;
/// `[0x7ffa24]` (`.rdata`) = `0.5` as a **dword** `f32` (`0x3f000000`), the 0x78cf30 round-to-nearest
/// bias added before `__ftol` (`fadd dword [0x7ffa24]`). (Same `0.5` constant as `luabind_76e`.)
pub const HALF_F32_BITS: u32 = 0x3f00_0000;

// ===========================================================================================
// (1) 0x78c360 — one integer width/height ratio (fidiv ×1).
// ===========================================================================================
//
// On the chunk-parse path that records a stream's pixel aspect, the parser fills two table fields and
// the owned arithmetic at 0x78c5ae..0x78c5c9 is:
//   mov [ebp-0x38], [esi+ebx+0x18]   ; low dword of the qword numerator (a u32)
//   mov [ebp-0x34], eax              ; high dword = eax = 0 (the 0x64a4c0 return on this path)
//   fild qword [ebp-0x38]            ; numerator = (u64) low dword           (hi forced 0)
//   mov [ebp-0x40], [esi+ebx+0x14]   ; the i32 divisor
//   mov eax, [ebp-0x10]              ; the `this` store base
//   fidiv dword [ebp-0x40]           ; q = numerator / (i32 divisor)    OWNED arithmetic
//   fstp dword [eax+0x33c]           ; store q as f32 into [this+0x33c]
// The decomp reads `(float)*(uint*)(buf+0x20) / (float)*(int*)(buf+0x1c)` — uint num, int den.

/// `0x78c360`'s sole owned arithmetic (`fild qword [ebp-0x38]; fidiv dword [ebp-0x40]; fstp dword
/// [eax+0x33c]`): the **zero-extended u32** numerator divided by the **signed i32** divisor, stored as
/// f32 into `[this+0x33c]`. `num` is the `[esi+ebx+0x18]` u32 (the `fild`'d qword's low dword, high
/// dword 0); `den` is the `[esi+ebx+0x14]` i32 divisor. The division is the masked-IEEE f64 divide
/// (FPCW=0x027F masks `#Z`, so `den==0` yields ±inf / NaN exactly as the binary), then narrowed to
/// f32 by the `fstp dword`. Returns the f32 stored at `[this+0x33c]`.
pub fn aspect_ratio(num: u32, den: i32) -> f32 {
    // `fild qword` of a value with high-dword 0 = the u32 widened to a non-negative i64 = f64::from(u32).
    if num == 0 && den == 0 {
        // 0/0 raises x87 #IA (invalid); masked (FPCW=0x027F masks #IA), `fidiv` yields the x87
        // real-indefinite QNaN — the NEGATIVE indefinite `0xFFF8…` (f64), which `fstp dword` narrows
        // to the f32 `0xFFC00000`. Rust's `0.0/0.0` is the POSITIVE QNaN (`0x7FC00000`), so the
        // op-for-op match forces the x87 indefinite here. (Out of the function's real domain — the
        // divisor is a frame dimension — but transcribed bit-exact, never fudged.)
        return f32::from_bits(0xffc0_0000);
    }
    (f64::from(num) / f64::from(den)) as f32 // fidiv dword (i32 den); fstp dword
}

// ===========================================================================================
// (2) 0x78cf30 — two byte-identical time→frame-index quantize blocks (fmul ×4, fadd ×2).
// ===========================================================================================
//
// Both blocks (0x78cf9c.. and 0x78cfd8..) are byte-for-byte identical:
//   mov [ebp-0x8], eax ; mov [ebp-0x4], 0      ; the int time value, hi dword forced 0 (a u32)
//   fild qword [ebp-0x8]                        ; t = (u64) the time value           (hi 0)
//   fmul dword [esi+0x33c]                      ; · rate (the f32 field set by 0x78c360's fidiv)
//   fmul dword [0x801360]=0.001(f32)            ; · 0.001  (ms → s)
//   fadd dword [0x7ffa24]=0.5(f32)              ; + 0.5    (round-to-nearest bias)
//   call __ftol (0x40a2b0)                      ; n = trunc(... toward zero)
// The two blocks differ only in the `eax`/`edi` register bookkeeping around them; the FP is identical.

/// One 0x78cf30 time→frame-index quantize block (`fild qword [ebp-0x8]; fmul dword [esi+0x33c];
/// fmul dword [0x801360]=0.001; fadd dword [0x7ffa24]=0.5; call __ftol`): the **zero-extended u32**
/// time value `t` (the `fild`'d qword, high dword 0) `· rate · 0.001 + 0.5`, then `__ftol` truncates
/// toward zero. `t` is the `[ebp-0x8]` time value (a u32); `rate` is the `[esi+0x33c]` f32 field
/// (set by 0x78c360 / `aspect_ratio`). Returns the truncated `i32` (the `__ftol` result). Both the
/// 0x78cf9c and 0x78cfd8 blocks compute exactly this (identical bytes).
pub fn time_to_index(t: u32, rate: f32) -> i32 {
    // `fild qword` (hi dword 0) = u32 widened exactly to f64.
    let p = f64::from(t) * f64::from(rate); // fmul dword [esi+0x33c]   (rate widens f32->f64)
    let p = p * k(MS_TO_S_F32_BITS); // fmul dword [0x801360]=0.001
    let p = p + k(HALF_F32_BITS); // fadd dword [0x7ffa24]=0.5
    benilla_re_cmath::x87::ftol(p) // __ftol = truncate toward zero
}

// ===========================================================================================
// (3) 0x78d1a0 — two glyph-quad coord scales (fmul ×2). the gx getter is not transcribed.
// ===========================================================================================
//
// The gx scalar getter `0x41ad70` (call; fstp dword [ebp-0x4]) yields the scale `[ebp-0x4]` (an f32);
// that BOUNDARY row is NOT transcribed here. The two owned products (0x78d478.. / 0x78d4a6..) are:
//   fld dword [ebp-0x4]                         ; scale (the gx-getter f32)
//   fmul dword [eax+0xc]  (then +0x8)           ; · glyph-quad field   OWNED arithmetic
//   fst  dword [ebp-0x48] (then -0x30)          ; store product as f32 (no pop, st0 live)
//   fstp dword [ebp-0x3c] (then -0x24)          ; store SAME product as f32 (pop)
// No arithmetic consumes the live st0 between the `fst`/`fstp`, so both slots get the identical f32.

/// One 0x78d1a0 glyph-quad coord scale (`fld dword [ebp-0x4]; fmul dword [eax+off]; fst/fstp dword`):
/// the gx-getter scale `· field`, narrowed to f32. `scale` is the `[ebp-0x4]` f32 (the gx getter
/// `0x41ad70`'s result — a BOUNDARY value handed in, NOT computed here); `field` is the glyph-quad
/// geometry-table f32 (`[eax+0xc]` for the first product, `[eax+0x8]` for the second). The PC_53 f64
/// product is stored to two f32 slots, both narrowing the SAME value. Returns the f32 stored.
pub fn quad_scale(scale: f32, field: f32) -> f32 {
    (f64::from(scale) * f64::from(field)) as f32 // fmul dword [eax+off]; fst/fstp dword
}

// ===========================================================================================
// (4) 0x78c360 — the whole reader: RIFF walk, `hdrl` stream headers, `idx1` copy.
// ===========================================================================================

/// What `0x78c360` leaves in the frame object when it returns 1. `None` is a field the reader
/// does not write (`+0x33c`, `+0x340`, `+0x344`, `+0x348`); `video` is the buffer at `+0x39c`
/// (`+0x3a4` bytes: each video record as a dword size and its data) and `audio` the one at
/// `+0x3a8` (`+0x3ac` bytes).
#[derive(Debug, PartialEq)]
pub struct MovieAvi {
    pub frame_rate: Option<f32>,
    pub frame_count: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub video: Vec<u8>,
    pub audio: Vec<u8>,
}

/// `SStrCmpI(a, tag, 4) == 0` on four-byte tags: equal ignoring ASCII case.
fn tag_is(a: &[u8], tag: &[u8; 4]) -> bool {
    a.len() >= 4 && a[..4].eq_ignore_ascii_case(tag)
}

fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("four bytes"))
}

/// An open file with a position, as `SFile::Read` and `SFile::SetFilePointer` see it.
struct Cursor<'a> {
    data: &'a [u8],
    pos: u64,
}

impl Cursor<'_> {
    /// Reads `n` bytes. A short read keeps what exists, advances past it and fails.
    fn read(&mut self, n: usize) -> (Vec<u8>, bool) {
        let start = usize::try_from(self.pos)
            .unwrap_or(usize::MAX)
            .min(self.data.len());
        let end = start.saturating_add(n).min(self.data.len());
        self.pos += (end - start) as u64;
        (self.data[start..end].to_vec(), end - start == n)
    }
}

/// `0x78c360(name)` over the bytes of the file it opens: `None` when it returns 0 (the open
/// fails, a short first read, or a `RIFF` or `AVI ` tag that does not match, ignoring case), else
/// what it stores. Runs the chunk walk (`0x78c422`), the `hdrl` walk (`0x78c555`), the two passes
/// over `idx1` (`0x78c650`, `0x78c6d0`) and stops a walk at the first read that fails. Defined
/// for a `hdrl` and an `idx1` that are whole and a multiple of 16 bytes, as a real file has.
pub fn read_movie_avi(file: &[u8]) -> Option<MovieAvi> {
    let mut f = Cursor { data: file, pos: 0 };
    let (head, ok) = f.read(12);
    if !ok || !tag_is(&head[0..4], b"RIFF") || !tag_is(&head[8..12], b"AVI ") {
        return None;
    }
    let mut hdrl = Vec::new();
    let mut idx1 = Vec::new();
    let mut movi = 0u64; // `[ebp-0x1c]`: the position after the `movi` type, else 0
    loop {
        let (h, ok) = f.read(8);
        if !ok {
            break;
        }
        let size = le32(&h, 4) as usize;
        if tag_is(&h, b"LIST") {
            let (ty, ok) = f.read(4);
            if !ok {
                break;
            }
            let body = size.wrapping_sub(4) & 0xffff_ffff;
            if tag_is(&ty, b"hdrl") {
                let (b, ok) = f.read(body);
                hdrl = b;
                if !ok {
                    break;
                }
            } else {
                if tag_is(&ty, b"movi") {
                    movi = f.pos;
                }
                f.pos += body as u64;
            }
        } else if tag_is(&h, b"idx1") {
            let (b, ok) = f.read(size);
            idx1 = b;
            if !ok {
                break;
            }
        } else {
            f.pos += size as u64;
        }
    }

    let mut out = MovieAvi {
        frame_rate: None,
        frame_count: None,
        width: None,
        height: None,
        video: Vec::new(),
        audio: Vec::new(),
    };
    let (mut counter, mut video_at, mut audio_at) = (0i32, -1i32, -1i32);
    let mut at = 0usize;
    while at < hdrl.len() {
        if tag_is(&hdrl[at..], b"LIST") {
            at += 12;
            continue;
        }
        let size = le32(&hdrl, at + 4) as usize;
        let is_header = tag_is(&hdrl[at..], b"strh");
        let is_format = tag_is(&hdrl[at..], b"strf");
        at += 8;
        if is_header {
            if tag_is(&hdrl[at..], b"vids") {
                // 0x78c5a0..0x78c5cd: dwRate / dwScale (the qword's high dword is 0), dwLength
                out.frame_rate = Some(aspect_ratio(
                    le32(&hdrl, at + 0x18),
                    le32(&hdrl, at + 0x14) as i32,
                ));
                out.frame_count = Some(le32(&hdrl, at + 0x20));
                video_at = counter;
            }
            if tag_is(&hdrl[at..], b"auds") {
                audio_at = counter;
            }
            counter += 1;
        } else if is_format && video_at >= 0 && audio_at < 0 {
            out.width = Some(le32(&hdrl, at + 4));
            out.height = Some(le32(&hdrl, at + 8));
        }
        at += size;
    }

    // The third byte of a chunk id: `d` (0x64) is video, `w` (0x77) audio.
    for e in idx1.as_chunks::<16>().0 {
        let (offset, size) = (le32(e, 8), le32(e, 12));
        if e[2] == 0x64 || e[2] == 0x77 {
            // 0x78c6eb, 0x78c728: seek to offset + P + 4 from the start, P = `movi`
            f.pos = u64::from(offset.wrapping_add(movi as u32).wrapping_add(4));
            let (mut data, _) = f.read(size as usize);
            data.resize(size as usize, 0); // the buffer's bytes past a short read stay as allocated
            if e[2] == 0x64 {
                out.video.extend_from_slice(&size.to_le_bytes());
                out.video.extend_from_slice(&data);
            } else {
                out.audio.extend_from_slice(&data);
            }
        }
    }
    Some(out)
}
