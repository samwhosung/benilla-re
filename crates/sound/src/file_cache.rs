//! The sound file cache's handles (`systems/sound/overview.md`, The file cache open).

/// The size of one file cache entry.
pub const FILE_CACHE_ENTRY: u32 = 0x1a4;

/// `0x7a8e50`: a handle's entry address, `entries + (handle >> 8) * 0x1a4` in 32 bits, and its
/// slot, `(handle & 0xff) - 1` in 32 bits (a slot byte of 0 gives `0xffffffff`).
pub fn file_cache_entry(entries: u32, handle: u32) -> (u32, u32) {
    let entry = (handle >> 8)
        .wrapping_mul(FILE_CACHE_ENTRY)
        .wrapping_add(entries);
    (entry, (handle & 0xff).wrapping_sub(1))
}
