//! Storm's MD5 and CRC-32.

mod common;

use common::*;

/// Runs `MD5Init` (`0x661da0`), the update (`0x661dd0`) and `MD5Final` (`0x661e90`) whole, with the
/// transform (`0x661f00`) inside them, over each buffer, and compares the digest with `md5`.
#[test]
fn md5() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let bufs = synth_buffers();
    for buf in &bufs {
        d.emu.write(d.io, buf);
        d.emu.call(0x661da0, d.ctx as u32, 0, &[]); // Init(ctx)
        d.emu
            .call(0x661dd0, d.ctx as u32, d.io as u32, &[buf.len() as u32]); // Update(ctx, data, len)
        d.emu.call(0x661e90, d.out as u32, d.ctx as u32, &[]); // Final(out, ctx)
        let got = d.read_bytes(d.out, 16);
        assert_eq!(
            got,
            benilla_re_storm::md5(buf).to_vec(),
            "md5 mismatch on {}-byte input",
            buf.len()
        );
    }
}

/// Runs the CRC-32 routine (`0x662860`) whole with the flags 1, 2 and 4 (seed, fold, invert) over each
/// buffer, and compares the value it writes through the pointer argument with `crc32`.
#[test]
fn crc32() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let bufs = synth_buffers();
    for buf in &bufs {
        d.emu.write(d.io, buf);
        d.emu.write_u32(d.out, 0); // crc_inout slot
        d.emu
            .call(0x662860, d.io as u32, buf.len() as u32, &[d.out as u32, 7]);
        let got = d.emu.read_u32(d.out);
        assert_eq!(
            got,
            benilla_re_storm::crc32(buf),
            "crc32 mismatch on {}-byte input",
            buf.len()
        );
    }
}
