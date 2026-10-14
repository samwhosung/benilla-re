//! The keyed SHA-1 hashes of the logon (`systems/net/login.md#logon-proof`). Each is built on
//! [`sha1`]; what the exe owns is the layout of the bytes it feeds in.

use crate::sha1;

/// The HMAC-SHA-1 (RFC 2104) of `ChecksumExecutables` (`0x5b1170`, `__fastcall` with the key in
/// `ecx`, its length in `edx` and the 20-byte output pointer on the stack). The exe fills `ipad`
/// with `0x36` and `opad` with `0x5c` (64 bytes each), XORs the key into their first bytes, and
/// writes `SHA1(opad_key ‖ SHA1(ipad_key ‖ message))`.
///
/// The exe does not shorten a key longer than the 64-byte block: it XORs it into the pads, so such
/// a key would overrun them. This function asserts `key.len() <= 64` instead of diverging. In the
/// exe the message is not an argument: the function reads five files whole and hashes them in turn
/// ([`checksum_executables`]).
pub fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; 20] {
    assert!(
        key.len() <= 64,
        "hmac_sha1: the exe does not shorten keys over 64 bytes"
    );

    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for (i, &k) in key.iter().enumerate() {
        ipad[i] ^= k;
        opad[i] ^= k;
    }

    let mut inner = Vec::with_capacity(64 + message.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(message);
    let inner_digest = sha1(&inner);

    let mut outer = Vec::with_capacity(64 + 20);
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_digest);
    sha1(&outer)
}

/// `HashPinWithSalts` (`0x5b31e0`, `__fastcall` with the PIN salt in `ecx`, the client salt in
/// `edx`, the PIN text and the 20-byte output pointer on the stack): `SHA1(k2 ‖ SHA1(k1 ‖ data))`
/// with `k1` the PIN salt, `k2` the client salt (both 16 bytes) and `data` the PIN text. The exe
/// feeds `data` through `Sha1UpdateString` (`0x5d1b30`), which hashes up to the first NUL byte;
/// this function hashes the slice whole, so a slice with a NUL differs from the exe.
pub fn checksum_proof(k1: &[u8; 16], k2: &[u8; 16], data: &[u8]) -> [u8; 20] {
    let mut inner = Vec::with_capacity(16 + data.len());
    inner.extend_from_slice(k1);
    inner.extend_from_slice(data);
    let inner_digest = sha1(&inner);

    let mut outer = Vec::with_capacity(16 + 20);
    outer.extend_from_slice(k2);
    outer.extend_from_slice(&inner_digest);
    sha1(&outer)
}

/// The five files `ChecksumExecutables` hashes, in the order of the pointer table at `0x85def0`
/// (strings at `0x85df34`, `0x85df28`, `0x85df1c`, `0x85df10`, `0x85df04`). The loop from `0x5b11e0`
/// steps the table by 4 up to `0x14`.
pub const CHECKSUM_EXECUTABLES: [&str; 5] = [
    "WoW.exe",
    "fmod.dll",
    "ijl15.dll",
    "dbghelp.dll",
    "unicows.dll",
];

/// `ChecksumExecutables` (`0x5b1170`): the 20-byte integrity hash of the logon proof, the HMAC-SHA-1
/// ([`hmac_sha1`]) with `salt` as the key of the whole contents of the five files of
/// [`CHECKSUM_EXECUTABLES`] in `install_dir`, concatenated in that order. The exe opens and reads
/// each file afresh (one `GetFileSize`, one `ReadFile`), not the running image.
///
/// A file that is missing (`0x5b11eb`), cannot be opened (`0x5b1212`), is empty (`0x5b121f`) or is
/// read short or fails (`0x5b1246`, `0x5b124b`) adds no bytes, and the loop goes on. With all five
/// missing the result is the HMAC of the empty message, not twenty zero bytes. The 20 zero bytes
/// the client sends on a reconnect come from another path: the checksum-salt slot `0x5b1c70`
/// passes them to `0x5bbad0` at `0x5b1c98` (`systems/net/login.md#logon-proof`).
///
/// With the 16-byte salt the vmangos realm daemon sends (`src/realmd/AuthSocket.cpp` line 65) and
/// the files of a 1.12.1.5875 Windows install, the result is
/// `95EDB27C7823B363CBDDAB56A392E7CB73FCCA20`, the value vmangos stores for that build
/// (`sql/migrations/20221117065844_logon.sql`).
pub fn checksum_executables(salt: &[u8; 16], install_dir: &std::path::Path) -> [u8; 20] {
    let mut message = Vec::new();
    for name in CHECKSUM_EXECUTABLES {
        if let Ok(bytes) = std::fs::read(install_dir.join(name)) {
            message.extend_from_slice(&bytes);
        }
    }
    hmac_sha1(salt, &message)
}
