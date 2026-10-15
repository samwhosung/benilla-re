//! The address list's helpers (`AddressList.cpp`; `systems/net/threads.md`).

use benilla_re_random::{crandom_uint32, srand_seed_state};

/// `AlignUpToMultiple` (`0x5bd4d0`, `(value, multiple)`): `value` rounded up to a multiple of
/// `multiple` with `div`, the add wrapping. A `multiple` of 0 faults on the division.
pub fn align_up(value: u32, multiple: u32) -> u32 {
    let r = value % multiple;
    if r == 0 {
        value
    } else {
        value.wrapping_add(multiple - r)
    }
}

/// The entry `ConnectToRandomAddress` (`0x5bc2b0`, `(addresses, count, callback)`) dials among
/// `count` 16-byte entries: a random state seeded with the clock's seconds
/// (`NTempest::CRndSeed::SetSeed`), advanced once (`NTempest::CRandom::uint32`, inlined), modulo
/// the count. `table` is the random table at `0x802700`.
pub fn random_address_index(time: u32, count: u32, table: &[u8]) -> u32 {
    let (acc, lanes) = srand_seed_state(time);
    let (acc, _, _) = crandom_uint32(acc, lanes, table);
    acc % count
}

/// `MarkSocketOpen` (`0x5b6330`, the socket in `ecx`): sets the socket's bit, word `fd >> 5`, bit
/// `fd & 31`, in the bitmap `[0xc2a378]` points at; a negative socket does nothing. Returns the
/// word index and the new word.
pub fn mark_socket_open(fd: i32, word: u32) -> Option<(u32, u32)> {
    if fd < 0 {
        return None;
    }
    let fd = fd as u32;
    Some((fd >> 5, word | (1 << (fd & 31))))
}
