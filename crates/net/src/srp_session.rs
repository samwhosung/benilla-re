//! The SRP6 client session of the logon (`systems/net/login.md#srp6-client`).
//!
//! The multiplier is 3 (SRP-6, not SRP-6a). With `H` for SHA-1 and numbers little-endian:
//!
//! - `x = H(salt ‖ p)`, with `p = H(USER ‖ ':' ‖ PASS)`
//! - `v = g^x mod N`
//! - `a = a_rand + bitlen(N)`, with `a_rand` 32 bytes from the random pool
//! - `A = g^a mod N`
//! - `u = H(A ‖ B)`
//! - `S = (B - 3v)^(a + u·x) mod N`, with the base built as `(B + 3(N - v)) mod N`
//! - `K = Interleave(S)`
//! - `M1 = H((H(N) ⊕ H(g)) ‖ H(USER) ‖ salt ‖ A ‖ B ‖ K)`
//!
//! Storm's BigNumber reads and writes its bytes little-endian: byte `i` is bits `8(i mod 4)` of
//! word `i/4` (`0x665910`). Every byte buffer given to it (`N`, `g`, `B`, the ephemeral and the
//! digests of `x` and `u`) is read that way, and `A` and the bytes of `S` are written as 32 bytes
//! little-endian, zero-padded at the top.

use crate::{sha1, srp6_interleave};
use num_bigint::BigUint;

/// What `Srp6ClientComputeProof` (`0x5d3650`) stores in the SRP object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SrpClientSession {
    /// `A`, at `+0x00`: 32 bytes little-endian.
    pub a_pub: [u8; 32],
    /// `K`, at `+0x20`: the 40-byte session key.
    pub k: [u8; 40],
    /// `M1`, at `+0x48`: the 20-byte proof sent to the server.
    pub m1: [u8; 20],
}

/// `Srp6ClientComputeProof` (`0x5d3650`): `n` and `g` as the server sent them, `salt` and `b` (`B`)
/// from the challenge, `p` and `h_i` the digests `Srp6SetCredentials` stored at `+0x70` and `+0x5c`,
/// and `a_rand` the 32 bytes drawn from the pool. The checks on `N`, `g`, the salt and `B` that
/// make the exe return -1 or -2 are not reproduced: the inputs are valid.
pub fn srp6_client_session(
    n: &[u8],
    g: &[u8],
    salt: &[u8; 32],
    b: &[u8; 32],
    p: &[u8; 20],
    h_i: &[u8; 20],
    a_rand: &[u8; 32],
) -> SrpClientSession {
    let n_bn = BigUint::from_bytes_le(n);
    let g_bn = BigUint::from_bytes_le(g);

    let mut xbuf = Vec::with_capacity(salt.len() + p.len());
    xbuf.extend_from_slice(salt);
    xbuf.extend_from_slice(p);
    let x = BigUint::from_bytes_le(&sha1(&xbuf));

    let v = g_bn.modpow(&x, &n_bn);

    // `0x5d38c2` adds the bit length of N to the random number.
    let a = BigUint::from_bytes_le(a_rand) + BigUint::from(n_bn.bits());

    let a_pub = to_fixed_le_32(&g_bn.modpow(&a, &n_bn));

    let mut ubuf = Vec::with_capacity(32 + 32);
    ubuf.extend_from_slice(&a_pub);
    ubuf.extend_from_slice(b);
    let u = BigUint::from_bytes_le(&sha1(&ubuf));

    let b_bn = BigUint::from_bytes_le(b);
    let base = (&b_bn + 3u32 * (&n_bn - &v)) % &n_bn;
    let exp = &a + &u * &x;
    let s_bytes = to_fixed_le_32(&base.modpow(&exp, &n_bn));

    let k = srp6_interleave(&s_bytes);

    let hn = sha1(n);
    let hg = sha1(g);
    let mut xored = [0u8; 20];
    for i in 0..20 {
        xored[i] = hn[i] ^ hg[i];
    }
    let mut m1buf = Vec::with_capacity(20 + 20 + 32 + 32 + 32 + 40);
    m1buf.extend_from_slice(&xored);
    m1buf.extend_from_slice(h_i);
    m1buf.extend_from_slice(salt);
    m1buf.extend_from_slice(&a_pub);
    m1buf.extend_from_slice(b);
    m1buf.extend_from_slice(&k);
    let m1 = sha1(&m1buf);

    SrpClientSession { a_pub, k, m1 }
}

/// The number as 32 bytes little-endian, zero-padded at the top: the exe's `getBytes(_, 0x20)`.
/// `A` and `S` are below `N`, which is under 2^256.
fn to_fixed_le_32(v: &BigUint) -> [u8; 32] {
    let raw = v.to_bytes_le();
    let mut out = [0u8; 32];
    let n = raw.len().min(32);
    out[..n].copy_from_slice(&raw[..n]);
    out
}
