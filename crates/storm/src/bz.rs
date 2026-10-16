//! The `.bz` loose file (`systems/storm/bz.md`): a 24-byte header, then a zlib stream.
//!
//! The header is the inflated size (`u32`), the magic `BZ00` and 16 bytes the client stores and never
//! reads. `SFile::OpenEx` (`0x6477c0`) keeps the size for `SFile::GetFileSize`; `0x6476e0` inflates the
//! stream through 4 KiB of input at a time; `SFile::SetFilePointer` (`0x648a60`) moves forward by
//! reading and backward by starting the stream again.
//!
//! The inflate is flate2's zlib-rs backend, which matches the client's zlib 1.2.2 on every stream
//! but one kind: a dynamic block whose code lengths give end-of-block no code fails here at its
//! header, while the client decodes it (`storm::bz_missing_eob_divergence`).

use flate2::{Decompress, FlushDecompress, Status};

/// The header's length; the zlib stream starts here.
pub const HEADER_LEN: usize = 24;
/// The magic at header offset 4.
pub const MAGIC: [u8; 4] = *b"BZ00";
/// The input buffer the client refills from the file.
pub const INPUT_LEN: usize = 0x1000;

/// The header fields the open keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BzHeader {
    /// Offset 0: the size `SFile::GetFileSize` answers.
    pub size: u32,
    /// Offsets 8 to 23, kept at `+0x44` of the file object and read by no live code.
    pub extra: [u8; 16],
}

/// Reads the header as the open does: `None` unless offset 4 holds `BZ00`.
pub fn parse_header(h: &[u8; HEADER_LEN]) -> Option<BzHeader> {
    if h[4..8] != MAGIC {
        return None;
    }
    Some(BzHeader {
        size: u32::from_le_bytes(h[0..4].try_into().unwrap()),
        extra: h[8..24].try_into().unwrap(),
    })
}

/// The file under a `.bz` handle: `read` is `fread`, `rewind` is `fseek(file, 24, SEEK_SET)`.
pub trait BzSource {
    fn read(&mut self, buf: &mut [u8]) -> usize;
    fn rewind(&mut self);
}

/// What a read answers: `ok` is the function's return; `read` the count it writes, `None` when it
/// leaves the count unwritten.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BzRead {
    pub ok: bool,
    pub read: Option<u32>,
}

/// An open `.bz` file: the source after its header, the input buffer and the inflate state.
pub struct BzFile<S: BzSource> {
    source: S,
    header: BzHeader,
    input: Box<[u8; INPUT_LEN]>,
    /// The unconsumed input, `input[next_in..next_in + avail_in]`.
    next_in: usize,
    avail_in: usize,
    z: Decompress,
    /// `+0x20`: the bytes delivered by successful reads since the stream's start.
    pos: u32,
}

impl<S: BzSource> BzFile<S> {
    /// The state the open leaves: no input, a fresh zlib stream, position 0.
    pub fn new(source: S, header: BzHeader) -> Self {
        BzFile {
            source,
            header,
            input: Box::new([0; INPUT_LEN]),
            next_in: 0,
            avail_in: 0,
            z: Decompress::new(true),
            pos: 0,
        }
    }

    /// `SFile::GetFileSize` of a `.bz` file: the header's size, the high word left unwritten.
    pub fn size(&self) -> u32 {
        self.header.size
    }

    pub fn position(&self) -> u32 {
        self.pos
    }

    /// `0x6476e0(file, out, out.len(), &read)`: inflates with `Z_SYNC_FLUSH` until the output is
    /// full, the stream ends or inflate fails, refilling the input whenever it is empty.
    pub fn read(&mut self, out: &mut [u8]) -> BzRead {
        let size = out.len();
        let mut produced = 0usize;
        loop {
            if self.avail_in == 0 {
                self.next_in = 0;
                let n = self.source.read(&mut self.input[..]);
                if n > 0 {
                    self.avail_in = n;
                }
            }
            let (in_before, out_before) = (self.z.total_in(), self.z.total_out());
            let input = &self.input[self.next_in..self.next_in + self.avail_in];
            let status = self
                .z
                .decompress(input, &mut out[produced..], FlushDecompress::Sync);
            let used = (self.z.total_in() - in_before) as usize;
            self.next_in += used;
            self.avail_in -= used;
            produced += (self.z.total_out() - out_before) as usize;
            if produced == size {
                self.pos = self.pos.wrapping_add(size as u32);
                return BzRead {
                    ok: true,
                    read: Some(size as u32),
                };
            }
            match status {
                Ok(Status::StreamEnd) => {
                    self.pos = self.pos.wrapping_add(produced as u32);
                    return BzRead {
                        ok: produced != 0,
                        read: Some(produced as u32),
                    };
                }
                Ok(Status::Ok) => continue,
                _ => {
                    return BzRead {
                        ok: false,
                        read: None,
                    }
                }
            }
        }
    }

    /// `SFile::SetFilePointer` of a `.bz` file with method 0 (begin), 1 (current) or other: returns
    /// the position, or 0 when a read on the way fails.
    pub fn seek(&mut self, distance: i32, method: u32) -> u32 {
        let skip = match method {
            0 => {
                if distance < self.pos as i32 {
                    self.z.reset(true);
                    self.avail_in = 0;
                    self.next_in = 0;
                    self.source.rewind();
                    self.pos = 0;
                    distance
                } else {
                    distance.wrapping_sub(self.pos as i32)
                }
            }
            1 => distance,
            _ => return self.pos,
        };
        let mut done = 0i32;
        let mut scratch = vec![0u8; INPUT_LEN];
        while skip > done {
            let chunk = ((skip - done) as u32).min(INPUT_LEN as u32) as usize;
            let r = self.read(&mut scratch[..chunk]);
            if !r.ok {
                return 0;
            }
            done += r.read.unwrap_or(0) as i32;
        }
        self.pos
    }
}
