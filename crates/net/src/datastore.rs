//! The data store every packet is read from and written into (`CDataStore`, from the base
//! library; `systems/net/messages.md`).
//!
//! A store is a vtable at `+0x0`, a buffer pointer at `+0x4`, the position its first byte holds
//! (`base`, `+0x8`), the bytes the buffer holds (`alloc`, `+0xc`), the size written (`+0x10`) and
//! the read cursor (`+0x14`). The byte at position `p` is at `buffer - base + p`, and the buffer
//! covers the window `[base, base + alloc)`. A read or write outside the window first asks the
//! vtable to move it: slot 2 (`+0x8`) for a read, which can refuse, and slot 3 (`+0xc`) for a
//! write, whose answer is ignored. Every sum and compare is 32-bit unsigned with wrap.
//!
//! Here the bytes are kept by position, so a window move changes only `base` and `alloc`.

/// What the vtable does when an access leaves the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fetch {
    /// The plain store's vtable (`0x7ff9e4`): slot 2 is `0x417c20`, which refuses every read, and
    /// slot 3 is `0x417c30`, which sets `alloc` to `pos + n` rounded up to 256 and reallocates the
    /// buffer to that size. `base` stays.
    Heap,
    /// The network layer's store with a 1,024-byte buffer of its own at `+0x18` (`0x80a7a8`):
    /// slot 2 refuses as above, and slot 3, `0x5bc6c0`, leaves a window that already holds
    /// `pos + n` bytes, and otherwise sets `alloc` to exactly `pos + n`. Leaving the inline
    /// buffer it copies only the first `min(pos, alloc)` bytes into the new one.
    Inline,
    /// A store whose vtable moves the window to `[pos, pos + width)`, refusing a read of more than
    /// `width` bytes. The client has no such store; the tests use it to reach the window paths.
    Window(u32),
}

/// A data store's state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataStore {
    /// The bytes by position; positions past the end read as 0.
    pub data: Vec<u8>,
    pub base: u32,
    pub alloc: u32,
    pub size: u32,
    pub read: u32,
    /// Whether the inline buffer of a [`Fetch::Inline`] store is still the one in use.
    pub inline: bool,
    pub fetch: Fetch,
    /// The error code each null-pointer refusal passes to `SErrSetLastError` (`0x64e850`), 87.
    pub errors: Vec<u32>,
}

/// The bits of an `f32` after a load into and a store from the x87: a signalling NaN (exponent
/// all ones, quiet bit clear, a nonzero fraction) comes back with its quiet bit set.
pub fn quiet_f32(bits: u32) -> u32 {
    let nan = bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0;
    if nan {
        bits | 0x0040_0000
    } else {
        bits
    }
}

/// `ERROR_INVALID_PARAMETER`, which a null source or destination with a nonzero count sets.
pub const INVALID_PARAMETER: u32 = 0x57;

impl DataStore {
    pub fn new(fetch: Fetch) -> DataStore {
        DataStore {
            data: Vec::new(),
            base: 0,
            alloc: 0,
            size: 0,
            read: 0,
            inline: false,
            fetch,
            errors: Vec::new(),
        }
    }

    pub fn byte(&self, p: u32) -> u8 {
        self.data.get(p as usize).copied().unwrap_or(0)
    }

    fn set_byte(&mut self, p: u32, v: u8) {
        let i = p as usize;
        if self.data.len() <= i {
            self.data.resize(i + 1, 0);
        }
        self.data[i] = v;
    }

    /// Whether `[pos, pos + n)` lies in the window: `pos >= base` and `pos + n <= base + alloc`.
    fn in_window(&self, pos: u32, n: u32) -> bool {
        pos >= self.base && pos.wrapping_add(n) <= self.base.wrapping_add(self.alloc)
    }

    /// Slot 3, the write fetch: makes the window hold `[pos, pos + n)` for the store's kind.
    fn fetch_write(&mut self, pos: u32, n: u32) {
        match self.fetch {
            Fetch::Heap => {
                self.alloc = pos.wrapping_add(n).wrapping_add(0xff) & !0xff;
            }
            Fetch::Inline => {
                let need = pos.wrapping_add(n);
                if need <= self.alloc {
                    return;
                }
                if self.inline {
                    let kept = pos.min(self.alloc);
                    let end = self.data.len().min(need as usize);
                    for i in kept as usize..end {
                        self.data[i] = 0;
                    }
                    self.inline = false;
                }
                self.alloc = need;
            }
            Fetch::Window(w) => {
                self.base = pos;
                self.alloc = w;
            }
        }
    }

    /// Slot 2, the read fetch: whether the window could be moved to hold `[pos, pos + n)`.
    fn fetch_read(&mut self, pos: u32, n: u32) -> bool {
        match self.fetch {
            Fetch::Heap | Fetch::Inline => false,
            Fetch::Window(w) => {
                if n > w {
                    return false;
                }
                self.base = pos;
                self.alloc = w;
                true
            }
        }
    }

    /// `Put` of `bytes.len()` bytes at the size (`0x418070` 1, `0x4180d0` and `0x418130` 2,
    /// `0x418190`, `0x4181f0`, `0x4182b0` and `0x4183d0` 4, `0x418370` 8): moves the window when
    /// `[size, size + n)` is outside it, writes, and adds `n` to the size.
    pub fn put(&mut self, bytes: &[u8]) {
        let n = bytes.len() as u32;
        let pos = self.size;
        if !self.in_window(pos, n) {
            self.fetch_write(pos, n);
        }
        for (i, &b) in bytes.iter().enumerate() {
            self.set_byte(pos.wrapping_add(i as u32), b);
        }
        self.size = self.size.wrapping_add(n);
    }

    /// `PutFloat` (`0x4183d0`): [`DataStore::put`] of the float's bits after an x87 `fld` and
    /// `fstp`, which keep every value but set the quiet bit (`0x00400000`) of a signalling NaN.
    pub fn put_f32(&mut self, bits: u32) {
        self.put(&quiet_f32(bits).to_le_bytes());
    }

    /// `Set` at an explicit position (`0x417cd0` 1, `0x417d70` 2, `0x417dc0` and `0x417e10` 4):
    /// as [`DataStore::put`], at `pos`, with the size unchanged.
    pub fn set_at(&mut self, pos: u32, bytes: &[u8]) {
        let n = bytes.len() as u32;
        if !self.in_window(pos, n) {
            self.fetch_write(pos, n);
        }
        for (i, &b) in bytes.iter().enumerate() {
            self.set_byte(pos.wrapping_add(i as u32), b);
        }
    }

    /// `PutArray` (`0x418570`, and `0x418c10`, which forwards to it). `src` is `None` for a null
    /// pointer, which with a nonzero `n` sets error 87 and writes nothing. Otherwise it moves the
    /// window for all `n` bytes at once, then copies in chunks of `min(remaining, alloc)`, at
    /// least 1, moving the window again for any chunk outside it.
    pub fn put_array(&mut self, src: Option<&[u8]>, n: u32) {
        let Some(src) = src else {
            if n != 0 {
                self.errors.push(INVALID_PARAMETER);
            }
            return;
        };
        if !self.in_window(self.size, n) {
            self.fetch_write(self.size, n);
        }
        let mut rem = n;
        let mut at = 0u32;
        while rem != 0 {
            let mut chunk = if rem < self.alloc { rem } else { self.alloc };
            if chunk <= 1 {
                chunk = 1;
            }
            if !self.in_window(self.size, chunk) {
                self.fetch_write(self.size, chunk);
            }
            for i in 0..chunk {
                let b = src[(at + i) as usize];
                self.set_byte(self.size.wrapping_add(i), b);
            }
            at += chunk;
            self.size = self.size.wrapping_add(chunk);
            rem = rem.wrapping_sub(chunk);
        }
    }

    /// `PutCString` (`0x418430`): a null string sets error 87; otherwise [`DataStore::put_array`]
    /// of the string and its NUL.
    pub fn put_cstring(&mut self, s: Option<&[u8]>) {
        match s {
            None => self.errors.push(INVALID_PARAMETER),
            Some(s) => {
                let mut z = s.to_vec();
                z.push(0);
                self.put_array(Some(&z), z.len() as u32);
            }
        }
    }

    /// The bound and window test of the fixed-width reads; `false` latches the cursor at
    /// `size + 1`.
    fn gate_read(&mut self, n: u32) -> bool {
        let r = self.read;
        if r.wrapping_add(n) > self.size {
            self.read = self.size.wrapping_add(1);
            return false;
        }
        if !self.in_window(r, n) && !self.fetch_read(r, n) {
            self.read = self.size.wrapping_add(1);
            return false;
        }
        true
    }

    /// `Get` of `n` bytes (`0x418cb0` 1, `0x418d30` and `0x418db0` 2, `0x418e30`, `0x418eb0`,
    /// `0x418fb0` and `0x419130` 4, `0x4190b0` 8): `None`, with the destination unwritten and the
    /// cursor at `size + 1`, when `read + n > size` or the window cannot be moved.
    pub fn get(&mut self, n: u32) -> Option<Vec<u8>> {
        if !self.gate_read(n) {
            return None;
        }
        let r = self.read;
        let v = (0..n).map(|i| self.byte(r.wrapping_add(i))).collect();
        self.read = r.wrapping_add(n);
        Some(v)
    }

    /// `GetDataInSitu` (`0x419ac0`): the position of `n` bytes at the cursor, which the client
    /// returns as a pointer into the buffer, or `None` (a null pointer) as [`DataStore::get`]
    /// fails. Nothing is copied, so `n` may be any count, including one whose sum with the cursor
    /// wraps.
    pub fn get_in_situ(&mut self, n: u32) -> Option<u32> {
        if !self.gate_read(n) {
            return None;
        }
        let r = self.read;
        self.read = r.wrapping_add(n);
        Some(r)
    }

    /// `GetData` (`0x419400`, and `0x419aa0`, which forwards to it) into `dst`, whose length is
    /// the count; `null` is a null destination. A null destination with a nonzero count sets
    /// error 87. A cursor past the size reads nothing. Otherwise it copies chunks of
    /// `min(size - read, remaining, alloc)`, at least 1, until the count is met; a chunk past the
    /// size, or outside the window when the window cannot move, latches the cursor at `size + 1`
    /// and leaves what was copied.
    pub fn get_data(&mut self, dst: &mut [u8], null: bool) {
        let n = dst.len() as u32;
        if null && n != 0 {
            self.errors.push(INVALID_PARAMETER);
            return;
        }
        if self.read > self.size || n == 0 {
            return;
        }
        let mut rem = n;
        let mut at = 0u32;
        loop {
            let s = self.size;
            let r = self.read;
            let mut chunk = s.wrapping_sub(r);
            if chunk >= rem {
                chunk = rem;
            }
            if chunk >= self.alloc {
                chunk = self.alloc;
            }
            if chunk <= 1 {
                chunk = 1;
            }
            if r.wrapping_add(chunk) > s {
                self.read = s.wrapping_add(1);
                return;
            }
            if !self.in_window(r, chunk) && !self.fetch_read(r, chunk) {
                self.read = self.size.wrapping_add(1);
                return;
            }
            for i in 0..chunk {
                dst[(at + i) as usize] = self.byte(r.wrapping_add(i));
            }
            at += chunk;
            self.read = r.wrapping_add(chunk);
            rem -= chunk;
            if rem == 0 {
                return;
            }
        }
    }

    /// `GetCString` (`0x4191b0`) into `dst`, whose length is the capacity; `null` is a null
    /// destination, which with a nonzero capacity sets error 87. A zero capacity, or a cursor
    /// past the size, reads nothing. It copies bytes up to and including a NUL, a window at a
    /// time; when the capacity fills before a NUL it latches the cursor at `size + 1`, and a
    /// cursor past the size at the end writes 0 at `dst[0]`.
    pub fn get_cstring(&mut self, dst: &mut [u8], null: bool) {
        let cap = dst.len() as u32;
        if null {
            if cap != 0 {
                self.errors.push(INVALID_PARAMETER);
            }
            return;
        }
        if cap == 0 {
            return;
        }
        if self.read <= self.size {
            let mut copied = 0u32;
            loop {
                let r = self.read;
                if r.wrapping_add(1) > self.size {
                    self.read = self.size.wrapping_add(1);
                    break;
                }
                if !self.in_window(r, 1) && !self.fetch_read(r, 1) {
                    self.read = self.size.wrapping_add(1);
                    break;
                }
                let mut end = self.alloc.wrapping_add(self.base);
                if end >= self.size {
                    end = self.size;
                }
                let mut m = end.wrapping_sub(self.read);
                let room = cap.wrapping_sub(copied);
                if m >= room {
                    m = room;
                }
                let mut i = 0u32;
                while m != 0 {
                    let c = self.byte(self.read.wrapping_add(i));
                    dst[copied as usize] = c;
                    copied += 1;
                    i += 1;
                    if c == 0 {
                        break;
                    }
                    m -= 1;
                }
                self.read = self.read.wrapping_add(i);
                if m != 0 {
                    break;
                }
                if copied >= cap {
                    self.read = self.size.wrapping_add(1);
                    break;
                }
            }
        }
        if self.read > self.size {
            dst[0] = 0;
        }
    }

    /// `0x401d10`: 1 when the cursor equals the size, else 0.
    pub fn is_read(&self) -> u32 {
        u32::from(self.read == self.size)
    }
}
