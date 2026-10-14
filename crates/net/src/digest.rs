//! SHA-1, which the logon and session code use for the SRP6 proofs, the session digests and the
//! keyed hashes.

/// SHA-1 (FIPS 180-1) of `data`: big-endian message words, MD-style padding and a 64-bit
/// big-endian bit length, with the round constants `0x5a827999`, `0x6ed9eba1`, `0x8f1bbcdc` and
/// `0xca62c1d6`.
///
/// The exe holds two copies of the algorithm, in two context layouts, with the same digest: the
/// one that starts at `SHA1_Init` (`0x5d1db0`) with `SHA1_Update` (`0x5d1de0`), `SHA1_Final`
/// (`0x5d32b0`) and `SHA1_Transform` (`0x5d1ec0`), and the one that starts at `Sha1Init`
/// (`0x5d1a40`) with `Sha1Update` (`0x5d1a80`), `Sha1Final` (`0x5d1b60`) and `Sha1Transform`
/// (`0x5d1420`). The first overwrites the caller's data with its message schedule
/// (`systems/net/login.md#sha-1-overwrites-its-input`); this function does not.
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [
        0x6745_2301,
        0xefcd_ab89,
        0x98ba_dcfe,
        0x1032_5476,
        0xc3d2_e1f0,
    ];

    // A 0x80 byte, zeros up to 56 mod 64, then the length in bits, big-endian.
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity(data.len() + 72);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for block in padded.as_chunks::<64>().0 {
        sha1_block(&mut h, block);
    }

    let mut out = [0u8; 20];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// One compression of a 64-byte block: 16 big-endian words extended to 80 by the rotate-left-1
/// schedule, then four rounds of 20 with the functions choose, parity, majority and parity.
fn sha1_block(h: &mut [u32; 5], block: &[u8]) {
    let mut w = [0u32; 80];
    for (i, word) in w.iter_mut().take(16).enumerate() {
        *word = u32::from_be_bytes([
            block[i * 4],
            block[i * 4 + 1],
            block[i * 4 + 2],
            block[i * 4 + 3],
        ]);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }

    let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
    for (i, &wi) in w.iter().enumerate() {
        let (f, k) = match i {
            0..=19 => ((b & c) | ((!b) & d), 0x5a82_7999u32),
            20..=39 => (b ^ c ^ d, 0x6ed9_eba1),
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
            _ => (b ^ c ^ d, 0xca62_c1d6),
        };
        let tmp = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(wi);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = tmp;
    }

    h[0] = h[0].wrapping_add(a);
    h[1] = h[1].wrapping_add(b);
    h[2] = h[2].wrapping_add(c);
    h[3] = h[3].wrapping_add(d);
    h[4] = h[4].wrapping_add(e);
}
