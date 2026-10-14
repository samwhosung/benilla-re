//! The `CDataStore` cursor the movement codec runs on: the reads `0x418eb0`, `0x418fb0`,
//! `0x419130` (dwords and floats), `0x4190b0` (a guid) and `0x419ac0` (a skip), and the appends
//! `0x4181f0`, `0x4182b0`, `0x4183d0` and `0x418370`.
//!
//! A read of `n` bytes first tests `size < pos + n` in 32 bits with wrap, so a huge `n` that
//! wraps the sum small passes the test and moves the cursor with the same wrap. A failed test
//! latches the cursor at `size + 1`, leaves the destination unwritten, and fails every later read
//! the same way. A skip zeroes its out-pointer before the test.

/// The read-side cursor over an inbound message buffer.
pub struct DataStore<'a> {
    pub buf: &'a [u8],
    /// `this[5]`, the read cursor. `size + 1` is the latched error state.
    pub pos: u32,
    /// `this[4]`, the total byte size of the message.
    pub size: u32,
}

impl<'a> DataStore<'a> {
    pub fn new(buf: &'a [u8]) -> DataStore<'a> {
        DataStore {
            buf,
            pos: 0,
            size: u32::try_from(buf.len()).expect("message fits u32"),
        }
    }

    /// The bounds test every read shares: `size < pos + n` (wrapping) latches `pos = size + 1`.
    fn gate(&mut self, n: u32) -> bool {
        if self.size < self.pos.wrapping_add(n) {
            self.pos = self.size + 1;
            return false;
        }
        true
    }

    /// Reads a dword (`0x418eb0`, `0x418fb0`, `0x419130`: one body per element type). `None` on the
    /// error latch, with the destination unwritten.
    pub fn get_u32(&mut self) -> Option<u32> {
        if !self.gate(4) {
            return None;
        }
        let p = self.pos as usize;
        let v = u32::from_le_bytes(self.buf[p..p + 4].try_into().unwrap());
        self.pos += 4;
        Some(v)
    }

    /// Reads a float by its bits (`0x419130`).
    pub fn get_f32(&mut self) -> Option<f32> {
        self.get_u32().map(f32::from_bits)
    }

    /// Reads a guid as two little-endian dwords (`0x4190b0`).
    pub fn get_u64(&mut self) -> Option<u64> {
        if !self.gate(8) {
            return None;
        }
        let p = self.pos as usize;
        let v = u64::from_le_bytes(self.buf[p..p + 8].try_into().unwrap());
        self.pos += 8;
        Some(v)
    }

    /// Skips `n` bytes and yields the skipped window (`0x419ac0` returns the pre-skip data
    /// pointer). The cursor moves with the wrapping add; the window is clamped to the buffer.
    pub fn skip(&mut self, n: u32) -> Option<&'a [u8]> {
        if !self.gate(n) {
            return None;
        }
        let p = self.pos as usize;
        let end = p.saturating_add(n as usize).min(self.buf.len());
        self.pos = self.pos.wrapping_add(n);
        Some(&self.buf[p.min(self.buf.len())..end])
    }
}

/// The write-side cursor. The tests give it ample capacity, so the grow virtual (`vtbl+0xc`) never
/// runs and every append lands in the chunk.
#[derive(Default)]
pub struct DataStoreW {
    pub buf: Vec<u8>,
}

impl DataStoreW {
    pub fn put_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub fn put_f32(&mut self, v: f32) {
        self.put_u32(v.to_bits());
    }
    pub fn put_u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
}
