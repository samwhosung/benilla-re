//! The SHA-1 steps of the SRP6 logon and the logon proof that stand alone
//! (`systems/net/login.md#srp6-client`, `systems/net/login.md#logon-proof`).

use crate::sha1;

/// The two digests `Srp6SetCredentials` (`0x5d3590`) stores in the SRP object.
pub struct CredentialDigests {
    /// `+0x5c`: `SHA1(USER)`.
    pub username: [u8; 20],
    /// `+0x70`: `SHA1(USER ‖ ':' ‖ PASS)`, the identity hash `p` of the SRP6 formulas.
    pub credential: [u8; 20],
}

/// `Srp6SetCredentials` (`0x5d3590`, `__thiscall` with the SRP object in `ecx`, the account and the
/// password as NUL-terminated strings on the stack): the digests of [`CredentialDigests`]. The
/// caller upper-cases the account (`0x64b460`); the separator is the single byte `:` (the one-byte
/// update at `0x5d360f`). The exe also starts, without feeding, the hash at `+0x84` that
/// `Srp6VerifyServerProof` (`0x5d3ad0`) finishes later.
pub fn srp6_credential_hash(user: &[u8], pass: &[u8]) -> CredentialDigests {
    let mut identity = Vec::with_capacity(user.len() + 1 + pass.len());
    identity.extend_from_slice(user);
    identity.push(b':');
    identity.extend_from_slice(pass);
    CredentialDigests {
        username: sha1(user),
        credential: sha1(&identity),
    }
}

/// `HashCdKeyProof` (`0x5bba80`, `__stdcall(record, chunk, chunk_len, key, key_len, out)`):
/// `SHA1(chunk ‖ key ‖ tail)`, `tail` being the 20 bytes at `record + 0xc`. `ClientLink::SendProof`
/// (`0x5bbad0`) calls it for each CD-key record.
pub fn srp6_file_hmac(chunk: &[u8], key: &[u8], tail: &[u8; 20]) -> [u8; 20] {
    let mut msg = Vec::with_capacity(chunk.len() + key.len() + 20);
    msg.extend_from_slice(chunk);
    msg.extend_from_slice(key);
    msg.extend_from_slice(tail);
    sha1(&msg)
}
