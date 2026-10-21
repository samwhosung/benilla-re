//! Storm's string length `SStrLen` (`0x64a6f0`), append `SStrPack` (`0x64a760`) and find
//! `SStrStr` (`0x64b4f0`, and its second copy `0x64b4a0`)
//! (`systems/storm/strings.md`).

/// The bytes of a C string held in `bytes`: those before the first NUL, or all of them.
fn cstr(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())]
}

/// `SStrLen` (`0x64a6f0`) for a non-null string: the count of bytes before the first NUL. A null
/// string sets error 0x57 and gives 0.
pub fn str_len(s: &[u8]) -> u32 {
    cstr(s).len() as u32
}

/// `SStrPack(dest, src, size)` (`0x64a760`) for non-null strings, on the memory image `mem`,
/// which holds the address `base` at its first byte; `dest` is the offset of the destination
/// string in `mem`.
///
/// A `size` of 0 returns 0 and writes nothing. Otherwise `src` is appended to the string at
/// `dest`, byte by byte while the write address is below `dest + size − 1` (the sum and the
/// compare are unsigned 32-bit arithmetic on addresses); `size` 0x7fffffff has no limit. A NUL
/// follows the last byte written, which is the old end when the destination already reaches the
/// limit. The result is the new length of the destination string. A null `dest` or `src` sets
/// error 0x57 and gives 0.
pub fn str_pack(mem: &mut [u8], dest: usize, base: u32, src: &[u8], size: u32) -> u32 {
    if size == 0 {
        return 0;
    }
    let addr = |i: usize| base.wrapping_add(i as u32);
    let mut end = dest + cstr(&mem[dest..]).len();
    let src = cstr(src);
    if size == 0x7fff_ffff {
        for &b in src {
            mem[end] = b;
            end += 1;
        }
    } else {
        let limit = addr(dest).wrapping_add(size).wrapping_sub(1);
        for &b in src {
            if addr(end) >= limit {
                break;
            }
            mem[end] = b;
            end += 1;
        }
    }
    mem[end] = 0;
    (end - dest) as u32
}

/// `SStrStr(haystack, needle)` (`0x64b4f0`, and `0x64b4a0`) for non-null strings: the offset of
/// the first position of the haystack, before its NUL, where the needle's bytes all match
/// case-sensitively (`strncmp` over the needle's length), or `None`. An empty haystack gives
/// `None` even for an empty needle; an empty needle in a non-empty haystack matches at 0. A null
/// argument sets error 0x57 and gives null.
pub fn str_str(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let (h, n) = (cstr(haystack), cstr(needle));
    (0..h.len()).find(|&i| h[i..].starts_with(n))
}
