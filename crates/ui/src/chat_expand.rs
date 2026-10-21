//! `ExpandTargetTokens` (`0x5083a0`), the `%` expander `SendChatMessage` runs its text through,
//! with the name it substitutes (`systems/ui/chatsend.md`).

/// The bytes of a C string held in `bytes`: those before the first NUL, or all of them.
fn cstr(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())]
}

/// The name a token takes, by the lookups in the order `0x5083a0` makes them (`0x50844a` to
/// `0x5084bc`): `not_found` for a target GUID of 0; else `unit`, the name `GetUnitName` gives the
/// unit the GUID names when the object manager holds one (`0x468460`, mask 8); else `cache`, the
/// name cache's record for the GUID, with no query (`0x55f080`); else `not_found`, the text of
/// the global `TARGET_TOKEN_NOT_FOUND`.
pub fn target_name<'a>(
    guid: u64,
    unit: Option<&'a [u8]>,
    cache: Option<&'a [u8]>,
    not_found: &'a [u8],
) -> &'a [u8] {
    if guid == 0 {
        return not_found;
    }
    unit.or(cache).unwrap_or(not_found)
}

/// `SStrPack(dest, src, size)` (`0x64a760`) on the buffer `mem` for a `size` below 0x7fffffff
/// that `mem` holds: appends the C string `src` while the write offset is below `size − 1`,
/// then a NUL.
fn pack(mem: &mut [u8], src: &[u8], size: usize) {
    if size == 0 {
        return;
    }
    let mut end = cstr(mem).len();
    for &b in cstr(src) {
        if end >= size - 1 {
            break;
        }
        mem[end] = b;
        end += 1;
    }
    mem[end] = 0;
}

/// `0x5083a0(text, buf, size)` on the memory `mem`, which starts at `buf` and is larger than
/// `size`, for a non-null `text` and `size` from 1: copies `text` into the buffer and expands
/// each `%N`, `%T`, `%n` and `%t` to `name`. Returns 1 when no other `%` occurred, else 0.
///
/// Every append is bounded by `size`. The text before a `%` is not appended alone: the whole
/// rest of the text is appended and a NUL then stored at the end of the run, at
/// `buf + (the length so far + the run's length)` with no bound, so a run that reaches past
/// `size` stores that NUL outside the buffer.
pub fn expand_target_tokens(text: &[u8], mem: &mut [u8], size: usize, name: &[u8]) -> u32 {
    let text = cstr(text);
    mem[0] = 0;
    let mut stray = false;
    let mut span = 0;
    while let Some(run) = text[span..].iter().position(|&b| b == b'%') {
        if run != 0 {
            let length = cstr(mem).len();
            pack(mem, &text[span..], size);
            mem[length + run] = 0;
        }
        let mut next = span + run + 1;
        match text.get(next) {
            Some(b'N' | b'T' | b'n' | b't') => {
                pack(mem, name, size);
                next += 1;
            }
            _ => {
                stray = true;
                pack(mem, b"%", size);
            }
        }
        span = next;
    }
    pack(mem, &text[span..], size);
    u32::from(!stray)
}
