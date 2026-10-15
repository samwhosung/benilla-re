//! The client's side of Warden (`systems/warden/overview.md`): the decoder a module blob goes
//! through (`0x6ca360`), the module image loader (`0x7a79d0`) with its unloader and entry lookup,
//! and the record the module cache stores. The RC4, SHA-1, RSA and zlib steps are the exe's
//! library routines; this crate calls the standard ones.

pub mod builtin;
pub mod cache;
pub mod image;
pub mod module;

/// The table `0x6ca3f0` reads, from its second byte: `0x811321`.
pub const NAME_TABLE_VA: u32 = 0x0081_1321;

/// The name the signature covers, as `0x6ca3e7` to `0x6ca401` build it (`systems/warden/modules.md`):
/// `M`, then each table byte times `0xb9` modulo 256 until a product is zero. The returned bytes
/// have no terminating zero and are as built, before the upper-casing at `0x6ca407`.
pub fn signature_name(table: &[u8]) -> Vec<u8> {
    let mut out = vec![b'M'];
    for &b in table {
        let p = b.wrapping_mul(0xb9);
        if p == 0 {
            break;
        }
        out.push(p);
    }
    out
}
