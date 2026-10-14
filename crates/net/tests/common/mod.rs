//! Helpers shared by the net tests: the input sets, the `Dev` fixture around the emulator, and the
//! slices that drive the cipher and the size header inside the connection's functions.

#![allow(dead_code)]

pub use benilla_re_difftest::Wow;
use benilla_re_difftest::{Emu, RegisterX86};

/// The LCG used for the fuzz inputs.
pub struct Lcg(pub u32);

impl Lcg {
    pub fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    pub fn arr20(&mut self) -> [u8; 20] {
        std::array::from_fn(|_| (self.next() >> 16) as u8)
    }
}

/// Byte buffers for the hashes: the empty message, the standard vectors, every length from 0 to
/// 130 (across the padding boundary at 56 mod 64 and the block size) and a fuzz of other lengths.
pub fn synth_buffers() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"abc".to_vec(),
        b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".to_vec(),
        vec![b'a'; 1000],
    ];
    for len in 0..=130usize {
        v.push((0..len).map(|i| (i * 7 + 1) as u8).collect());
    }
    let mut s: u32 = 0x9e37_79b9;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..400 {
        let len = (next() >> 8) as usize % 300;
        v.push((0..len).map(|_| (next() >> 16) as u8).collect());
    }
    v
}

/// Shared secrets for the session key interleave: lengths 0, 1 and 2, odd lengths, leading zeros, an
/// all-zero buffer and a fuzz that also puts zeros at the start.
pub fn synth_secrets() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        vec![],
        vec![0x42],
        vec![0, 0, 0, 7, 8, 9],
        vec![0u8; 16],
        (0..32u8).map(|i| i + 1).collect(),
        (0..32u8).map(|i| i + 1).chain(std::iter::once(0)).collect(),
    ];
    let mut s: u32 = 0x1357_9bdf;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..400 {
        let len = (next() >> 8) as usize % 48;
        let lead = (next() >> 12) as usize % 3;
        let mut b: Vec<u8> = vec![0; lead.min(len)];
        while b.len() < len {
            b.push((next() >> 16) as u8);
        }
        v.push(b);
    }
    v
}

/// An emulator with scratch regions: a context, a message buffer, a digest output, a
/// `WowConnection` for the cipher state, a 40-byte key and a stack frame.
pub struct Dev<'a> {
    pub emu: Emu<'a>,
    pub ctx: u64,
    pub io: u64,
    pub out: u64,
    pub conn: u64,
    pub key: u64,
    pub frame: u64,
}

impl<'a> Dev<'a> {
    pub fn new(wow: &'a Wow) -> Dev<'a> {
        let mut emu = Emu::new(wow);
        let ctx = emu.alloc(0x80);
        let io = emu.alloc(0x800);
        let out = emu.alloc(0x40);
        let conn = emu.alloc(0x140);
        let key = emu.alloc(0x40);
        let frame = emu.alloc(0x40);
        Dev {
            emu,
            ctx,
            io,
            out,
            conn,
            key,
            frame,
        }
    }

    /// Runs the cipher loop of `WowConnection::Send`, `0x5b5842` to `0x5b5896`, over a data store
    /// whose buffer pointer is at `+0x08` (`esi`) and the connection's state (`ebx`). It sets the
    /// send index `j` (`+0x121`), the feedback `prev` (`+0x122`), the key length 40 (`+0x127`) and
    /// the key pointer (`+0x128`), and returns the ciphertext and the advanced `(j, prev)`. The loop
    /// counter is `[ebp+0xc]` and its bound `edi`. An empty buffer returns unchanged, since the
    /// exe tests for it before the slice (`0x5b5840`).
    pub fn cipher_send_slice(
        &mut self,
        key: &[u8; 40],
        j: u8,
        prev: u8,
        plain: &[u8],
    ) -> (Vec<u8>, u8, u8) {
        if plain.is_empty() {
            return (Vec::new(), j, prev);
        }
        self.emu.write(self.key, key);
        self.emu.write(self.io, plain);
        self.emu.write(self.conn + 0x121, &[j]);
        self.emu.write(self.conn + 0x122, &[prev]);
        self.emu.write(self.conn + 0x127, &[40]);
        self.emu.write_u32(self.conn + 0x128, self.key as u32);
        let cds = self.out;
        self.emu.write_u32(cds + 0x08, self.io as u32);
        self.emu.write_u32(self.frame + 0x0c, 0);
        self.emu.set_reg(RegisterX86::EBX, self.conn);
        self.emu.set_reg(RegisterX86::ESI, cds);
        self.emu.set_reg(RegisterX86::EBP, self.frame);
        self.emu.set_reg(RegisterX86::ECX, 0);
        self.emu.set_reg(RegisterX86::EDI, plain.len() as u64);
        self.emu.run(0x5b5842, 0x5b5896);
        let ct = self.read_bytes(self.io, plain.len());
        let j2 = self.read_bytes(self.conn + 0x121, 1)[0];
        let prev2 = self.read_bytes(self.conn + 0x122, 1)[0];
        (ct, j2, prev2)
    }

    /// Runs the header form selection and byte writes of `WowConnection::AllocSendBuffer`,
    /// `0x5b5495` to `0x5b54f0`, for the length `n` (in `eax`) and returns the header bytes it
    /// writes at the buffer's data pointer (`ebx + 8`). The whole function does not run because
    /// it ends by storing `n + 0x23` over the header and copies the payload. `esi` differs from
    /// `edi`, which skips the no-payload adjustment, and the header length is stored at
    /// `[ebp+0x10]`.
    pub fn header_encode_slice(&mut self, n: u32) -> Vec<u8> {
        let buf = self.conn;
        let hdr = buf + 0x20;
        self.emu.write_u32(buf + 0x08, hdr as u32);
        self.emu.write_u32(self.frame + 0x10, 0xdead_beef);
        self.emu.write(hdr, &[0xaa; 4]);
        self.emu.set_reg(RegisterX86::EAX, u64::from(n));
        self.emu.set_reg(RegisterX86::EBX, buf);
        self.emu.set_reg(RegisterX86::ESI, 1);
        self.emu.set_reg(RegisterX86::EDI, 0);
        self.emu.set_reg(RegisterX86::ECX, 0);
        self.emu.set_reg(RegisterX86::EBP, self.frame);
        self.emu.run(0x5b5495, 0x5b54f0);
        let hdr_len = self.emu.read_u32(self.frame + 0x10) as usize;
        assert!(
            hdr_len == 2 || hdr_len == 3,
            "n={n:#x}: bad header length {hdr_len}"
        );
        let data_ptr = u64::from(self.emu.read_u32(buf + 0x08));
        self.read_bytes(data_ptr, hdr_len)
    }

    /// Runs the header decode of `WowConnection::ReadFramedMessages`, `0x5b6730` to `0x5b6789`,
    /// with the receive buffer pointer at `+0x1c` and the fill count at `+0x20` of the connection
    /// (`esi`), and returns the two values it stores at `[ebp-0x10]` and `[ebp-0xc]`:
    /// `(frame_len, hdr_len)`.
    pub fn header_decode_slice(&mut self, bytes: &[u8], avail: u32) -> (u32, u32) {
        let bp = self.frame + 0x20;
        self.emu.write_u32(bp - 0x10, 0x1234_5678);
        self.emu.write_u32(bp - 0xc, 0x1234_5678);
        if !bytes.is_empty() {
            self.emu.write(self.io, bytes);
        }
        self.emu.write_u32(self.conn + 0x1c, self.io as u32);
        self.emu.write_u32(self.conn + 0x20, avail);
        self.emu.set_reg(RegisterX86::ESI, self.conn);
        self.emu.set_reg(RegisterX86::EBP, bp);
        self.emu.run(0x5b6730, 0x5b6789);
        let frame_len = self.emu.read_u32(bp - 0x10);
        let hdr_len = self.emu.read_u32(bp - 0xc);
        (frame_len, hdr_len)
    }

    /// Reads `n` bytes of emulator memory.
    pub fn read_bytes(&self, va: u64, n: usize) -> Vec<u8> {
        self.emu.read(va, n)
    }

    /// Runs the span computation and cipher loop of `WowConnection::SendRaw`, `0x5b5df1` to
    /// `0x5b5e79`; the socket send after it is not run. It sets the armed byte (`+0x120`), the send
    /// index and feedback (`+0x121`, `+0x122`), the span (`+0x123`), the key length and pointer,
    /// and `edi` to `frame_len`, which selects the 2- or 3-byte header span; the loop reloads the
    /// buffer pointer from `[ebp+8]`. Only `buf[..span]` is touched, so a short buffer with a large
    /// `frame_len` reaches the 3-byte case. It returns the first `plain.len()` bytes and the
    /// advanced `(j, prev)`.
    pub fn encipher_send_frame_slice(
        &mut self,
        key: &[u8; 40],
        j: u8,
        prev: u8,
        span_extra: u8,
        plain: &[u8],
        frame_len: u32,
    ) -> (Vec<u8>, u8, u8) {
        self.emu.write(self.key, key);
        self.emu.write(self.io, plain);
        self.emu.write(self.conn + 0x120, &[1]);
        self.emu.write(self.conn + 0x121, &[j]);
        self.emu.write(self.conn + 0x122, &[prev]);
        self.emu.write(self.conn + 0x123, &[span_extra]);
        self.emu.write(self.conn + 0x127, &[40]);
        self.emu.write_u32(self.conn + 0x128, self.key as u32);
        self.emu.write_u32(self.frame + 0x08, self.io as u32);
        self.emu.set_reg(RegisterX86::ESI, self.conn);
        self.emu.set_reg(RegisterX86::EBX, self.io);
        self.emu.set_reg(RegisterX86::EDI, u64::from(frame_len));
        self.emu.set_reg(RegisterX86::EBP, self.frame);
        self.emu.run(0x5b5df1, 0x5b5e79);
        let ct = self.read_bytes(self.io, plain.len());
        let j2 = self.read_bytes(self.conn + 0x121, 1)[0];
        let prev2 = self.read_bytes(self.conn + 0x122, 1)[0];
        (ct, j2, prev2)
    }

    /// Runs the cipher loop of `WowConnection::ReadFramedMessages`, `0x5b6848` to `0x5b68a8`, over
    /// the receive buffer at `+0x1c` (consumed offset `+0x20` of 0), with the receive index and
    /// feedback at `+0x124` and `+0x125`. The span (`ebx`, `[ebp-8]`) is large and the bound
    /// (`[ebp-0x14]`) is the length, so exactly `cipher.len()` bytes are deciphered. It returns the
    /// plaintext and the advanced `(j, prev)`.
    pub fn cipher_recv_slice(
        &mut self,
        key: &[u8; 40],
        j: u8,
        prev: u8,
        cipher: &[u8],
    ) -> (Vec<u8>, u8, u8) {
        if cipher.is_empty() {
            return (Vec::new(), j, prev);
        }
        self.emu.write(self.key, key);
        self.emu.write(self.io, cipher);
        self.emu.write(self.conn + 0x124, &[j]);
        self.emu.write(self.conn + 0x125, &[prev]);
        self.emu.write(self.conn + 0x127, &[40]);
        self.emu.write_u32(self.conn + 0x128, self.key as u32);
        self.emu.write_u32(self.conn + 0x1c, self.io as u32);
        self.emu.write_u32(self.conn + 0x20, 0);
        let ebp = self.frame + 0x20;
        let span = 0x10000u32;
        let bound = cipher.len() as u32;
        self.emu.write_u32(ebp - 0x14, bound);
        self.emu.write_u32(ebp - 0x08, span);
        self.emu.set_reg(RegisterX86::ESI, self.conn);
        self.emu.set_reg(RegisterX86::EDI, 0);
        self.emu.set_reg(RegisterX86::EBX, u64::from(span));
        self.emu.set_reg(RegisterX86::EBP, ebp);
        self.emu.run(0x5b6848, 0x5b68a8);
        let pt = self.read_bytes(self.io, cipher.len());
        let j2 = self.read_bytes(self.conn + 0x124, 1)[0];
        let prev2 = self.read_bytes(self.conn + 0x125, 1)[0];
        (pt, j2, prev2)
    }
}
