//! The decoder `0x6ca360`: a module blob and a 16-byte key in, the decompressed module out
//! (`systems/warden/modules.md`). The blob's RC4 layer, its SHA-1 and RSA signature and its zlib
//! stream are the exe's library routines; this module calls the standard ones.

use flate2::read::ZlibDecoder;
use num_bigint::BigUint;
use sha1::{Digest, Sha1};
use std::io::Read;

/// The signature's tail: four tag bytes and the 256-byte signature.
pub const TAIL: usize = 0x104;
/// The tag that opens the tail, `NGIS` as the dword `0x5349474e` reads.
pub const TAIL_TAG: [u8; 4] = *b"NGIS";
/// Below this length the client returns null at once (`0x6ca36e`).
pub const MIN_LEN: usize = 0x108;

/// RC4, the key schedule and stream of the exe's Storm routines `0x66ec20` and `0x66ecc0`.
pub fn rc4(key: &[u8], data: &mut [u8]) {
    let mut s: Vec<u8> = (0..=255).collect();
    let mut j = 0u8;
    for i in 0..256 {
        j = j.wrapping_add(s[i]).wrapping_add(key[i % key.len()]);
        s.swap(i, j as usize);
    }
    let (mut i, mut j) = (0u8, 0u8);
    for b in data.iter_mut() {
        i = i.wrapping_add(1);
        j = j.wrapping_add(s[i as usize]);
        s.swap(i as usize, j as usize);
        *b ^= s[s[i as usize].wrapping_add(s[j as usize]) as usize];
    }
}

/// The signature check the client makes through Storm's `SSignature` (`0x66cdd0`, `0x66ce40`,
/// `0x66cf10`): the tag, then RSA with the public key `(modulus, exponent)`, both little-endian,
/// must give the SHA-1 of `body ‖ name`, then `0xbb` filling 235 bytes, then `0x0b`.
pub fn signature_ok(decrypted: &[u8], name: &[u8], modulus: &[u8], exponent: u32) -> bool {
    let Some(split) = decrypted.len().checked_sub(TAIL) else {
        return false;
    };
    let (body, tail) = decrypted.split_at(split);
    if tail[..4] != TAIL_TAG {
        return false;
    }
    let n = BigUint::from_bytes_le(modulus);
    let sig = BigUint::from_bytes_le(&tail[4..]);
    let m = sig.modpow(&BigUint::from(exponent), &n);
    let mut got = m.to_bytes_le();
    got.resize(0x100, 0);
    let mut hasher = Sha1::new();
    hasher.update(body);
    hasher.update(name);
    let mut want = vec![0xbbu8; 0x100];
    want[..20].copy_from_slice(&hasher.finalize());
    want[0xff] = 0x0b;
    got == want
}

/// Where the decoder stops.
#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    TooShort,
    BadSignature,
    BadStream,
}

/// `0x6ca360` up to the loader: decrypts a copy of `blob` with `key`, checks the signature over
/// the whole decrypted blob less its last `0x104` bytes plus the upper-cased name, then
/// inflates the zlib stream that starts after the first dword into a buffer of that dword's
/// size. `name` is the decoded name before upper-casing. Returns the decompressed bytes.
pub fn decode(
    blob: &[u8],
    key: &[u8; 16],
    name: &[u8],
    modulus: &[u8],
    exponent: u32,
) -> Result<Vec<u8>, DecodeError> {
    if blob.len() < MIN_LEN {
        return Err(DecodeError::TooShort);
    }
    let mut dec = blob.to_vec();
    rc4(key, &mut dec);
    let upper = name.to_ascii_uppercase();
    if !signature_ok(&dec, &upper, modulus, exponent) {
        return Err(DecodeError::BadSignature);
    }
    let want = u32::from_le_bytes(dec[..4].try_into().unwrap()) as usize;
    let mut out = Vec::new();
    ZlibDecoder::new(&dec[4..])
        .take(want as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|_| DecodeError::BadStream)?;
    if out.len() > want {
        return Err(DecodeError::BadStream);
    }
    Ok(out)
}
