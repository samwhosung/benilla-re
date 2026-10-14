//! The two SHA-1 implementations of the exe (`systems/net/login.md#sha-1-overwrites-its-input`).

mod common;

use common::*;

/// Runs `SHA1_Init` (`0x5d1db0`), `SHA1_Update` (`0x5d1de0`) and `SHA1_Final` (`0x5d32b0`) whole,
/// with `SHA1_Transform` (`0x5d1ec0`) inside them, over each buffer, and compares the digest with `sha1`.
#[test]
fn sha1_family_b() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let bufs = synth_buffers();
    for buf in &bufs {
        d.emu.write(d.io, buf);
        d.emu.call(0x5d1db0, d.ctx as u32, 0, &[]); // init
        d.emu
            .call(0x5d1de0, d.ctx as u32, d.io as u32, &[buf.len() as u32]); // update
        d.emu.call(0x5d32b0, d.out as u32, d.ctx as u32, &[]); // final
        let got = d.read_bytes(d.out, 20);
        assert_eq!(
            got,
            benilla_re_net::sha1(buf).to_vec(),
            "sha1_family_b mismatch on {}-byte input",
            buf.len()
        );
    }
}

/// Runs `Sha1Init` (`0x5d1a40`), `Sha1Update` (`0x5d1a80`) and `Sha1Final` (`0x5d1b60`) whole, with
/// `Sha1Transform` (`0x5d1420`) inside them, over each buffer, and compares the digest with `sha1`.
#[test]
fn sha1_family_a() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let bufs = synth_buffers();
    for buf in &bufs {
        d.emu.write(d.io, buf);
        d.emu.call(0x5d1a40, d.ctx as u32, 0, &[]); // init
        d.emu
            .call(0x5d1a80, d.ctx as u32, 0, &[d.io as u32, buf.len() as u32]); // update
        d.emu.call(0x5d1b60, d.ctx as u32, 0, &[d.out as u32]); // final
        let got = d.read_bytes(d.out, 20);
        assert_eq!(
            got,
            benilla_re_net::sha1(buf).to_vec(),
            "sha1_family_a mismatch on {}-byte input",
            buf.len()
        );
    }
}
