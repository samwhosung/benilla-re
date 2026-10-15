//! The record of the module cache: the 8-byte `{size, data}` buffer the cache holds for a module
//! blob, written by `0x6ca070` and read by `0x6ca0c0` (`systems/warden/cache.md`).

/// What `0x6ca070` puts in the store for a record of `data`: its length as a little-endian dword,
/// then the bytes one at a time. An empty record is the dword 0.
pub fn encode_record(data: &[u8]) -> Vec<u8> {
    let mut out = (data.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(data);
    out
}

/// What `0x6ca0c0` reads from a store that holds a whole record: the dword length, then that many
/// bytes. `None` when the store is shorter than the record claims; the client then reads past the
/// end (`systems/warden/cache.md`).
pub fn decode_record(store: &[u8]) -> Option<Vec<u8>> {
    let len = u32::from_le_bytes(store.get(..4)?.try_into().ok()?) as usize;
    store.get(4..4usize.checked_add(len)?).map(<[u8]>::to_vec)
}
