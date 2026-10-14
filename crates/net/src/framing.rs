//! The big-endian size header that prefixes every world packet (`systems/net/wire.md#size-header`).
//!
//! A frame is `[header 2|3][opcode][body]`. The header holds `N`, the count of opcode and body
//! bytes, not counting the header itself.

/// The size header for `n`: two bytes `n >> 8` and `n & 0xff`, or, when `(n as i32) > 0x7fff`
/// (a signed compare), three bytes `((n >> 16) & 0x7f) | 0x80`, `n >> 8` and `n & 0xff`. This is
/// the selection and the byte writes of `WowConnection::AllocSendBuffer` (`0x5b5450`, `0x5b5495`
/// to `0x5b54f0`); `WowConnection::Send` (`0x5b5630`) builds the same bytes inline.
pub fn header_encode(n: u32) -> Vec<u8> {
    if (n as i32) > 0x7fff {
        vec![
            (((n >> 16) & 0x7f) | 0x80) as u8,
            ((n >> 8) & 0xff) as u8,
            (n & 0xff) as u8,
        ]
    } else {
        vec![((n >> 8) & 0xff) as u8, (n & 0xff) as u8]
    }
}

/// The two values `WowConnection::ReadFramedMessages` (`0x5b65f0`, `0x5b6730` to `0x5b6786`)
/// stores for the header at the start of its receive buffer: `[ebp-0x10]` and `[ebp-0xc]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeaderDecode {
    /// The whole frame's length, `N + hdr_len`, or `0xffff_ffff` (the client's `-1`) when the
    /// buffer holds fewer than `hdr_len` bytes.
    pub frame_len: u32,
    /// The header's length: 2 when the first byte's top bit is clear, 3 when it is set. It is set
    /// from the first byte even when the third byte has not arrived.
    pub hdr_len: u32,
}

impl HeaderDecode {
    /// `N`, the opcode and body length: `frame_len - hdr_len`, or `None` for an incomplete header.
    pub fn payload_len(self) -> Option<u32> {
        (self.frame_len != 0xffff_ffff).then(|| self.frame_len.wrapping_sub(self.hdr_len))
    }
}

/// The header decode of `WowConnection::ReadFramedMessages` (`0x5b6730` to `0x5b6786`) over the
/// readable prefix `buf` (its length is the fill count at `[esi+0x20]`). With fewer than 2 bytes
/// it gives the incomplete result. A first byte below `0x80` gives
/// `N = ((b0 & 0x7f) << 8) | b1` and `hdr_len = 2` (`0x5b6777`); otherwise `hdr_len = 3`, and with
/// fewer than 3 bytes the result is incomplete, else `N = ((b0 & 0x7f) << 16) | (b1 << 8) | b2`
/// (`0x5b675d`). `frame_len` is `N + hdr_len`.
pub fn header_decode(buf: &[u8]) -> HeaderDecode {
    let avail = buf.len() as u32;
    if avail < 2 {
        return HeaderDecode {
            frame_len: 0xffff_ffff,
            hdr_len: 2,
        };
    }
    let b0 = u32::from(buf[0]);
    if b0 & 0x80 == 0 {
        let n = ((b0 & 0x7f) << 8) | u32::from(buf[1]);
        HeaderDecode {
            frame_len: n.wrapping_add(2),
            hdr_len: 2,
        }
    } else if avail < 3 {
        HeaderDecode {
            frame_len: 0xffff_ffff,
            hdr_len: 3,
        }
    } else {
        let n = ((b0 & 0x7f) << 16) | (u32::from(buf[1]) << 8) | u32::from(buf[2]);
        HeaderDecode {
            frame_len: n.wrapping_add(3),
            hdr_len: 3,
        }
    }
}
