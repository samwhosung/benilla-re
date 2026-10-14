//! A model of the `CDataStore` read cursor that the decoders call: `GetInt32` (`0x418e30`), `GetUInt8`
//! (`0x418cb0`) and `GetCString` (`0x4191b0`) read little-endian values and leave the cursor at the
//! consumed-byte position. The real functions belong to the network layer; this models their
//! semantics so a decoder's wire to struct mapping can be compared with the exe on its own.

/// A read cursor over a record buffer (the SMSG fields after the key), as `CDataStore` reads its own buffer.
pub struct DataStore<'a> {
    buf: &'a [u8],
    pub cursor: usize,
}

impl<'a> DataStore<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, cursor: 0 }
    }

    /// `GetUInt8` (0x418cb0): 1 byte, advance 1.
    pub fn u8(&mut self) -> u8 {
        let v = self.buf[self.cursor];
        self.cursor += 1;
        v
    }

    /// `GetInt16`/`GetUInt16` (0x418d30/0x418db0): 2 LE bytes, advance 2.
    pub fn u16(&mut self) -> u16 {
        let v = u16::from_le_bytes([self.buf[self.cursor], self.buf[self.cursor + 1]]);
        self.cursor += 2;
        v
    }

    /// `GetInt32`, `GetUInt32`, `GetUInt32_2` and `GetFloat` (`0x418e30`, `0x418eb0`, `0x418fb0`, `0x419130`):
    /// 4 little-endian bytes, advance 4; all are the same 4-byte copy, and a float is read by its bits.
    pub fn u32(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.buf[self.cursor..self.cursor + 4].try_into().unwrap());
        self.cursor += 4;
        v
    }

    /// `GetGuid` (0x4190b0): 8 bytes, advance 8.
    pub fn guid(&mut self) -> [u8; 8] {
        let v = self.buf[self.cursor..self.cursor + 8].try_into().unwrap();
        self.cursor += 8;
        v
    }

    /// `GetCString(dst, max)` (`0x4191b0`): copies the NUL-terminated string at the cursor into
    /// `out[off..off + max]`, writing at most `max` bytes including the NUL, and advances the cursor by
    /// the source length including the NUL. `out` is the zeroed record struct; bytes past the copied
    /// string stay as they were.
    pub fn cstring(&mut self, out: &mut [u8], off: usize, max: usize) {
        let mut i = 0usize;
        loop {
            let b = self.buf[self.cursor + i];
            if i < max {
                out[off + i] = b;
            }
            i += 1;
            if b == 0 {
                break;
            }
        }
        self.cursor += i;
    }

    /// `GetCString` into a scratch local: the `char *` field decoders read into a stack buffer, then
    /// `SStrDupA` it. This advances the cursor past the NUL-terminated string, by its length including
    /// the NUL, and writes no struct field.
    pub fn skip_cstring(&mut self, _max: usize) {
        let mut i = 0usize;
        loop {
            let b = self.buf[self.cursor + i];
            i += 1;
            if b == 0 {
                break;
            }
        }
        self.cursor += i;
    }
}

/// Models the heap pointer `SStrDupA` returns to the `char *` field decoders. A decoder reads a field
/// into a local with `GetCString`, copies it with `SStrDupA` and stores the returned pointer. The
/// pointer is a heap address, so the tests stub `SStrDupA` (`0x64a620`) to a bump allocator and this
/// follows the same sequence, which makes the stored pointers comparable. What is compared is the read
/// order, the field placement and when `SStrDupA` is called, not the pointer values.
pub struct Dup {
    pub base: u32,
    pub stride: u32,
    pub n: u32,
}

impl Dup {
    pub fn new(base: u32, stride: u32) -> Self {
        Self { base, stride, n: 0 }
    }
    /// The next `SStrDupA` return, as the tests' `stub_alloc(0x64a620, base, stride, 3)` gives it.
    pub fn alloc(&mut self) -> u32 {
        let p = self.base.wrapping_add(self.n.wrapping_mul(self.stride));
        self.n += 1;
        p
    }
}
