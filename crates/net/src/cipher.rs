//! The world connection's stream cipher: additive feedback on the ciphertext byte, with a
//! 40-byte key (`systems/net/wire.md#header-cipher`). Only the size header and the opcode are
//! enciphered; the body travels in clear.

/// Enciphers `buf` in place from the state `(j, prev)` and returns the advanced state. Per byte:
/// `c = (key[j] ^ plain) + prev` modulo 256, `prev = c`, `j = (j + 1) mod 40`. This is the loop of
/// `WowConnection::Send` (`0x5b5842` to `0x5b5896`).
pub fn cipher_send(key: &[u8; 40], mut j: u8, mut prev: u8, buf: &mut [u8]) -> (u8, u8) {
    for b in buf.iter_mut() {
        let c = (key[j as usize] ^ *b).wrapping_add(prev);
        *b = c;
        prev = c;
        j = (j + 1) % 40;
    }
    (j, prev)
}

/// Deciphers `buf` in place and returns the advanced state: `plain = key[j] ^ (c - prev)` modulo
/// 256, `prev = c`, `j = (j + 1) mod 40`. This is the loop of `WowConnection::ReadFramedMessages`
/// (`0x5b6848` to `0x5b68a8`). Both directions feed `prev` with the ciphertext byte and advance
/// `j` alike, so deciphering what [`cipher_send`] wrote from the same state returns the plaintext
/// and ends in the same state.
pub fn cipher_recv(key: &[u8; 40], mut j: u8, mut prev: u8, buf: &mut [u8]) -> (u8, u8) {
    for b in buf.iter_mut() {
        let c = *b;
        *b = key[j as usize] ^ c.wrapping_sub(prev);
        prev = c;
        j = (j + 1) % 40;
    }
    (j, prev)
}

/// The enciphered span of `WowConnection::SendRaw` (`0x5b5dd0`, `0x5b5df1` to `0x5b5e79`): the
/// first `min(span_extra + hdr_len, len)` bytes of `buf`, run through [`cipher_send`], where
/// `hdr_len` is 3 when `len - 2 > 0x7fff` (a signed compare) and 2 otherwise. `span_extra` is the
/// connection's send span at `+0x123`, the opcode width. Returns the advanced state.
pub fn encipher_send_frame(
    key: &[u8; 40],
    j: u8,
    prev: u8,
    span_extra: u8,
    buf: &mut [u8],
) -> (u8, u8) {
    let len = buf.len() as i64;
    let hdr_len: i64 = if len - 2 > 0x7fff { 3 } else { 2 };
    let span = (i64::from(span_extra) + hdr_len).clamp(0, len) as usize;
    cipher_send(key, j, prev, &mut buf[..span])
}
