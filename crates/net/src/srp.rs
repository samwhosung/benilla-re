//! The session key of the SRP6 logon (`systems/net/login.md#session-key-interleave`).

use crate::sha1;

/// `Srp6InterleaveSessionKey` (`0x5d3360`, `__fastcall` with the 40-byte output in `ecx`, `S` in
/// `edx` and the length on the stack): the 40-byte session key `K` from the shared secret `S`.
/// Leading zero bytes are dropped, then one more if the count left is odd. Of the rest, the
/// even-indexed bytes are hashed with SHA-1 to `G` and the odd-indexed to `H`; `K[2i] = G[i]` and
/// `K[2i + 1] = H[i]`. Whatever the length of `S`, each digest is 20 bytes.
pub fn srp6_interleave(s: &[u8]) -> [u8; 40] {
    let mut t = s;
    while !t.is_empty() && t[0] == 0 {
        t = &t[1..];
    }
    if t.len() & 1 == 1 {
        t = &t[1..];
    }
    let half = t.len() / 2;

    let even: Vec<u8> = (0..half).map(|i| t[i * 2]).collect();
    let odd: Vec<u8> = (0..half).map(|i| t[i * 2 + 1]).collect();
    let g = sha1(&even);
    let h = sha1(&odd);

    let mut out = [0u8; 40];
    for i in 0..20 {
        out[i * 2] = g[i];
        out[i * 2 + 1] = h[i];
    }
    out
}
