//! The file the logon link downloads a patch into (`systems/net/download.md`): the `.partial`
//! file with its 40-byte `DInf` trailer, resumed, fed, checked by MD5 and released.
//!
//! The object: `+0` the file, `+8` the trailer as kept (magic `DInf`, a zero dword, the expected
//! MD5, the bytes received, the total), `+0x30` the final path, `+0x134` whether the file is the
//! `.partial`, `+0x138` the state (2 complete, 3 failed), `+0x13c` the error, `+0x144` the bytes
//! fed since `DownloadFileInit`, `+0x148` the MiB a failed resize asked for.

use benilla_re_storm::md5;

/// The trailer's magic, `DInf`.
pub const TRAILER_MAGIC: u32 = 0x4449_6e66;
/// The trailer's size.
pub const TRAILER: u32 = 0x28;

/// The file object's calls: slot 1 read `(dst, lo, hi, n, 0)`, slot 2 write `(src, lo, hi, n)`,
/// slot 4 the size, slot 5 the size set `(lo, hi)`.
pub trait FileOps {
    fn read(&mut self, lo: u32, hi: u32, n: u32) -> Option<Vec<u8>>;
    fn write(&mut self, lo: u32, hi: u32, data: &[u8]) -> bool;
    fn size(&mut self) -> u64;
    fn set_size(&mut self, lo: u32, hi: u32) -> bool;
}

/// The download object's fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Download {
    pub magic: u32,
    pub zero: u32,
    pub digest: [u8; 16],
    pub received: u64,
    pub total: u64,
    pub partial: u8,
    pub state: u32,
    pub error: u32,
    pub fed: u32,
    pub mib: u32,
}

impl Download {
    fn trailer(&self) -> Vec<u8> {
        let mut t = self.magic.to_le_bytes().to_vec();
        t.extend(self.zero.to_le_bytes());
        t.extend(self.digest);
        t.extend(self.received.to_le_bytes());
        t.extend(self.total.to_le_bytes());
        t
    }
}

/// `ReleaseRefChain` (`0x5b04d0`): from an object, takes one reference from each object its
/// `+4` field chains to, reading the link before the release; an object whose count `+8`
/// reaches 0 gets slot 0 with 1, its deleting destructor. Returns the deleted objects in order.
pub fn release_ref_chain(counts: &mut [u32], next: &[Option<usize>], start: usize) -> Vec<usize> {
    let mut deleted = Vec::new();
    let mut at = Some(start);
    while let Some(k) = at {
        at = next[k];
        counts[k] = counts[k].wrapping_sub(1);
        if counts[k] == 0 {
            deleted.push(k);
        }
    }
    deleted
}

/// `DigestFileRange` (`0x5b0880`, `(file, start, end, digest)`): the MD5 of the file's bytes
/// read in chunks of up to 32 KiB, or 16 zero bytes when a read fails. Only the low dwords
/// count, and the range read is `start` to `end - start`, not to `end`: every caller passes a
/// start of 0. Each read passes a high dword of 0.
pub fn digest_file_range(f: &mut impl FileOps, start: u32, end: u32) -> [u8; 16] {
    let span = end.wrapping_sub(start);
    let mut pos = start;
    let mut data = Vec::new();
    while pos < span {
        let n = (span - pos).min(0x8000);
        match f.read(pos, 0, n) {
            Some(b) => data.extend(b),
            None => return [0; 16],
        }
        pos += n;
    }
    md5(&data)
}

/// `DownloadFileVerify` (`0x5b0920`): the MD5 of the first `total` bytes against the kept one.
fn verify(d: &Download, f: &mut impl FileOps) -> bool {
    digest_file_range(f, 0, d.total as u32) == d.digest
}

/// `DownloadFileIsComplete` (`0x5b0970`): the file's size is the total and its MD5 matches.
fn is_complete(d: &Download, f: &mut impl FileOps) -> bool {
    f.size() == d.total && verify(d, f)
}

/// `DownloadFileReadTrailer` (`0x5b09b0`): `(found, complete)`. A file one trailer longer than
/// the total must end in a trailer with the magic, a zero dword, the kept MD5 and the kept
/// total; it is then kept, and complete when it says every byte arrived and the MD5 of the data
/// matches (a mismatch fails). A file exactly the total long is found when it is complete.
pub fn read_trailer(d: &mut Download, f: &mut impl FileOps) -> (bool, bool) {
    let size = f.size();
    if size == d.total.wrapping_add(u64::from(TRAILER)) {
        let at = size.wrapping_sub(u64::from(TRAILER));
        let Some(t) = f.read(at as u32, (at >> 32) as u32, TRAILER) else {
            return (false, false);
        };
        let word = |o: usize| u32::from_le_bytes(t[o..o + 4].try_into().unwrap());
        let quad = |o: usize| u64::from_le_bytes(t[o..o + 8].try_into().unwrap());
        if word(0) != TRAILER_MAGIC
            || word(4) != 0
            || t[8..0x18] != d.digest
            || quad(0x20) != d.total
        {
            return (false, false);
        }
        let mut complete = false;
        if quad(0x18) == quad(0x20) {
            if !verify(d, f) {
                return (false, false);
            }
            complete = true;
        }
        d.magic = word(0);
        d.zero = word(4);
        d.received = quad(0x18);
        return (true, complete);
    }
    if size == d.total {
        let c = is_complete(d, f);
        return (c, c);
    }
    (false, false)
}

/// `DownloadFileProgress` (`0x5b0ce0`): the bytes received over the total, both converted from
/// unsigned 64 bits and divided at the x87's 53-bit precision, returned as a float; 0 for a
/// total of 0.
pub fn progress(d: &Download) -> f32 {
    if d.total == 0 {
        return 0.0;
    }
    (d.received as f64 / d.total as f64) as f32
}

/// `DownloadFileWriteTrailer` (`0x5b0b70`): the kept trailer at the total's low dword; a failed
/// write sets state 3 and error 1.
fn write_trailer(d: &mut Download, f: &mut impl FileOps) {
    if !f.write(d.total as u32, 0, &d.trailer()) {
        d.state = 3;
        d.error = 1;
    }
}

/// `DownloadFileFeed` (`0x5b0bc0`, `(data, size)`): writes the data at the received count's low
/// dword and adds it to the count and to `+0x144`. Past the total it sets state 3 and error 1;
/// otherwise, once `+0x144` reaches 1 MiB, which nothing resets, or at the total, it rewrites the
/// trailer. At the total the MD5 decides state 2 or 3. Without a file, or when the write fails,
/// it returns 0 with state 3; else 1.
pub fn feed_download(d: &mut Download, f: Option<&mut impl FileOps>, data: &[u8]) -> u32 {
    let Some(f) = f else {
        d.state = 3;
        return 0;
    };
    if !f.write(d.received as u32, 0, data) {
        d.state = 3;
        return 0;
    }
    let n = data.len() as u32;
    d.received = d.received.wrapping_add(u64::from(n));
    d.fed = d.fed.wrapping_add(n);
    if d.received > d.total {
        d.state = 3;
        d.error = 1;
    } else if d.fed >= 0x10_0000 || d.received == d.total {
        write_trailer(d, f);
    }
    if d.received == d.total {
        d.state = if digest_file_range(f, 0, d.total as u32) == d.digest {
            2
        } else {
            3
        };
    }
    1
}

/// What [`release_download`] and [`open_download`] do with the file objects and the paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DownloadCall {
    /// `0x662910` opens a path: the `.partial` one or the final one, and whether it creates.
    Open {
        partial: bool,
        create: bool,
    },
    /// The file object's deleting destructor, its count having reached 0.
    DeleteFile,
    /// `DownloadFileDelete` (`0x5b04f0`) or `DownloadFileCommit` (`0x5b0570`).
    Delete,
    Commit,
}

/// `DownloadFileRelease` (`0x5b0450`, `(delete, commit)`): drops the file object's reference
/// (`count` is its `+8`; at 0 it is deleted; its chain `+4` is empty here) and clears `+0`, then
/// deletes the file or, with `commit`, renames the `.partial` to the final path.
pub fn release_download(
    file: &mut Option<u32>,
    delete: bool,
    commit: bool,
    calls: &mut Vec<DownloadCall>,
) {
    if let Some(count) = file.take() {
        if count.wrapping_sub(1) == 0 {
            calls.push(DownloadCall::DeleteFile);
        }
    }
    if delete {
        calls.push(DownloadCall::Delete);
    } else if commit {
        calls.push(DownloadCall::Commit);
    }
}

/// The files `DownloadFileOpen` can find: the `.partial` and the final one, each existing or
/// not and opening or not, and whether a created `.partial` opens and takes its size.
pub trait Disk {
    type File: FileOps;
    fn exists(&self, partial: bool) -> bool;
    fn open(&mut self, partial: bool, create: bool) -> Option<&mut Self::File>;
}

/// `DownloadFileOpen` (`0x5b05f0`, `(path, digest, total, &offset)`): releases the file it held,
/// keeps the total and the MD5, and sets the offset to 0. An existing `.partial` whose trailer
/// reads resumes: offset the received count, return 1, or, complete, state 2 and return 0. Else
/// an existing final file that is complete gives state 2, the offset `+0x20` holds, and 0.
/// Else it creates the `.partial` and sizes it to the total's low dword plus a trailer, returning
/// 1; a failed create sets state 3, error 1, and a failed size state 3, error 2 and the MiB
/// wanted, each returning 0. Returns the result's low byte and the offset.
pub fn open_download(
    d: &mut Download,
    file: &mut Option<u32>,
    disk: &mut impl Disk,
    digest: [u8; 16],
    total: u64,
    calls: &mut Vec<DownloadCall>,
) -> (u8, u64) {
    release_download(file, false, false, calls);
    d.total = total;
    d.digest = digest;
    let mut offset = 0;
    if disk.exists(true) {
        calls.push(DownloadCall::Open {
            partial: true,
            create: false,
        });
        if let Some(f) = disk.open(true, false) {
            let (found, complete) = read_trailer(d, f);
            if found {
                d.partial = 1;
                *file = Some(1);
                offset = d.received;
                if complete {
                    d.state = 2;
                    return (0, offset);
                }
                return (1, offset);
            }
            calls.push(DownloadCall::DeleteFile);
        }
    }
    if disk.exists(false) {
        calls.push(DownloadCall::Open {
            partial: false,
            create: false,
        });
        if let Some(f) = disk.open(false, false) {
            let done = is_complete(d, f);
            calls.push(DownloadCall::DeleteFile);
            if done {
                d.state = 2;
                return (0, d.received);
            }
        }
    }
    calls.push(DownloadCall::Open {
        partial: true,
        create: true,
    });
    d.partial = 1;
    let Some(f) = disk.open(true, true) else {
        d.state = 3;
        d.error = 1;
        return (0, offset);
    };
    *file = Some(1);
    if !f.set_size((total as u32).wrapping_add(TRAILER), 0) {
        d.state = 3;
        d.error = 2;
        d.mib = ((total.wrapping_add(u64::from(TRAILER)) >> 20) as u32).wrapping_add(1);
        return (0, offset);
    }
    (1, offset)
}
