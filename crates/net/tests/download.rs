//! The patch download file (`systems/net/download.md`), each function run whole beside the model,
//! with the file objects, the disk, the lock and the delete and rename replaced by Rust closures
//! over the same fake files the model uses.

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use benilla_re_difftest::{Emu, RegisterX86};
use benilla_re_net::{
    digest_file_range, feed_download, open_download, progress, read_trailer, release_download,
    release_ref_chain, Disk, Download, DownloadCall, FileOps, TRAILER_MAGIC,
};
use common::{Lcg, Wow};

const RELEASE_REF_CHAIN: u64 = 0x5b04d0;
const RELEASE: u64 = 0x5b0450;
const OPEN: u64 = 0x5b05f0;
const DIGEST_RANGE: u64 = 0x5b0880;
const READ_TRAILER: u64 = 0x5b09b0;
const FEED: u64 = 0x5b0bc0;
const PROGRESS: u64 = 0x5b0ce0;
const FILE_EXISTS: u64 = 0x42a4e0;
const FILE_OPEN: u64 = 0x662910;
const DELETE: u64 = 0x5b04f0;
const COMMIT: u64 = 0x5b0570;

/// A file: its bytes, and whether reads, writes and a resize succeed. A read past the end fails.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct FakeFile {
    data: Vec<u8>,
    read_ok: bool,
    write_ok: bool,
    resize_ok: bool,
}

impl FileOps for FakeFile {
    fn read(&mut self, lo: u32, hi: u32, n: u32) -> Option<Vec<u8>> {
        let pos = u64::from(lo) | (u64::from(hi) << 32);
        let end = pos.checked_add(u64::from(n))?;
        (self.read_ok && end <= self.data.len() as u64)
            .then(|| self.data[pos as usize..end as usize].to_vec())
    }
    fn write(&mut self, lo: u32, hi: u32, data: &[u8]) -> bool {
        let pos = (u64::from(lo) | (u64::from(hi) << 32)) as usize;
        if !self.write_ok || pos > 0x10_0000 {
            return false;
        }
        if self.data.len() < pos + data.len() {
            self.data.resize(pos + data.len(), 0);
        }
        self.data[pos..pos + data.len()].copy_from_slice(data);
        true
    }
    fn size(&mut self) -> u64 {
        self.data.len() as u64
    }
    fn set_size(&mut self, lo: u32, hi: u32) -> bool {
        if !self.resize_ok || hi != 0 || lo > 0x10_0000 {
            return false;
        }
        self.data.resize(lo as usize, 0);
        true
    }
}

/// The two paths a download knows: the `.partial` (0) and the final file (1).
#[derive(Clone, Debug, Default)]
struct TestDisk {
    files: [Option<FakeFile>; 2],
    opens: [bool; 2],
    create_ok: bool,
    /// What a created `.partial` allows.
    fresh: FakeFile,
}

impl Disk for TestDisk {
    type File = FakeFile;
    fn exists(&self, partial: bool) -> bool {
        self.files[usize::from(!partial)].is_some()
    }
    fn open(&mut self, partial: bool, create: bool) -> Option<&mut FakeFile> {
        if create {
            if !self.create_ok {
                return None;
            }
            self.files[0] = Some(self.fresh.clone());
            return self.files[0].as_mut();
        }
        let i = usize::from(!partial);
        if self.opens[i] {
            self.files[i].as_mut()
        } else {
            None
        }
    }
}

/// The emulated side: two file objects, `+0` a vtable whose slots run on the shared disk, `+4`
/// an empty chain and `+8` a count; the download object; and the log of the closures.
struct Rig<'a> {
    e: Emu<'a>,
    obj: u64,
    files: [u64; 2],
    disk: Rc<RefCell<TestDisk>>,
    log: Rc<RefCell<Vec<DownloadCall>>>,
}

fn which(files: [u64; 2], this: u32) -> usize {
    usize::from(u64::from(this) == files[1])
}

impl<'a> Rig<'a> {
    fn new(wow: &'a Wow) -> Rig<'a> {
        let mut e = Emu::new(wow);
        let obj = e.alloc(0x160);
        let files = [e.alloc(0x10), e.alloc(0x10)];
        let vt = e.alloc(0x20);
        let disk = Rc::new(RefCell::new(TestDisk::default()));
        let log = Rc::new(RefCell::new(Vec::new()));
        let slots: [u64; 6] = std::array::from_fn(|_| e.alloc(0x10));
        for (k, &s) in slots.iter().enumerate() {
            e.write_u32(vt + 4 * k as u64, s as u32);
        }
        for f in files {
            e.write_u32(f, vt as u32);
        }
        let l = log.clone();
        e.stub_fn(slots[0], 1, move |_| {
            l.borrow_mut().push(DownloadCall::DeleteFile);
            0
        });
        let d = disk.clone();
        e.stub_fn(slots[1], 5, move |c| {
            let i = which(files, c.reg(RegisterX86::ECX) as u32);
            let got = d.borrow_mut().files[i]
                .as_mut()
                .and_then(|f| f.read(c.arg(1), c.arg(2), c.arg(3)));
            match got {
                Some(b) => {
                    let dst = u64::from(c.arg(0));
                    c.write(dst, &b);
                    1
                }
                None => 0,
            }
        });
        let d = disk.clone();
        e.stub_fn(slots[2], 4, move |c| {
            let i = which(files, c.reg(RegisterX86::ECX) as u32);
            let src = c.read(u64::from(c.arg(0)), c.arg(3) as usize);
            let ok = d.borrow_mut().files[i]
                .as_mut()
                .is_some_and(|f| f.write(c.arg(1), c.arg(2), &src));
            u32::from(ok)
        });
        let d = disk.clone();
        e.stub_fn(slots[4], 1, move |c| {
            let i = which(files, c.reg(RegisterX86::ECX) as u32);
            let n = d.borrow_mut().files[i].as_mut().map_or(0, |f| f.size());
            let out = u64::from(c.arg(0));
            c.write(out, &n.to_le_bytes());
            0
        });
        let d = disk.clone();
        e.stub_fn(slots[5], 2, move |c| {
            let i = which(files, c.reg(RegisterX86::ECX) as u32);
            let ok = d.borrow_mut().files[i]
                .as_mut()
                .is_some_and(|f| f.set_size(c.arg(0), c.arg(1)));
            u32::from(ok)
        });
        let partial_path = |c: &benilla_re_difftest::StubIo, at: u32| -> bool {
            let s = c.read(u64::from(at), 0x110);
            let n = s.iter().position(|&b| b == 0).unwrap();
            s[..n].ends_with(b".partial")
        };
        let d = disk.clone();
        e.stub_fn(FILE_EXISTS, 0, move |c| {
            let partial = partial_path(c, c.reg(RegisterX86::ECX) as u32);
            u32::from(d.borrow().exists(partial))
        });
        let (d, l) = (disk.clone(), log.clone());
        e.stub_fn(FILE_OPEN, 0, move |c| {
            let spec = u64::from(c.arg(0));
            let partial = partial_path(c, c.read_u32(spec));
            let create = c.read_u32(spec + 0xc) == 2;
            assert_eq!(c.read_u32(spec + 4), 0xc000_0000, "read and write access");
            assert_eq!(c.read_u32(spec + 0xc), if create { 2 } else { 3 });
            l.borrow_mut().push(DownloadCall::Open { partial, create });
            if d.borrow_mut().open(partial, create).is_none() {
                return 0;
            }
            let f = files[usize::from(!partial)];
            c.write_u32(f + 4, 0);
            c.write_u32(f + 8, 1);
            f as u32
        });
        let l = log.clone();
        e.stub_fn(DELETE, 0, move |_| {
            l.borrow_mut().push(DownloadCall::Delete);
            0
        });
        let l = log.clone();
        e.stub_fn(COMMIT, 0, move |_| {
            l.borrow_mut().push(DownloadCall::Commit);
            0
        });
        e.stub(0x6579b0, 0, 0);
        e.stub(0x6579c0, 0, 0);
        Rig {
            e,
            obj,
            files,
            disk,
            log,
        }
    }

    fn put(&mut self, d: &Download, file: Option<(usize, u32)>) {
        let o = self.obj;
        let at = file.map_or(0, |(i, count)| {
            self.e.write_u32(self.files[i] + 4, 0);
            self.e.write_u32(self.files[i] + 8, count);
            self.files[i] as u32
        });
        self.e.write_u32(o, at);
        self.e.write_u32(o + 8, d.magic);
        self.e.write_u32(o + 0xc, d.zero);
        self.e.write(o + 0x10, &d.digest);
        self.e.write(o + 0x20, &d.received.to_le_bytes());
        self.e.write(o + 0x28, &d.total.to_le_bytes());
        self.e.write(o + 0x134, &[d.partial]);
        self.e.write_u32(o + 0x138, d.state);
        self.e.write_u32(o + 0x13c, d.error);
        self.e.write_u32(o + 0x144, d.fed);
        self.e.write_u32(o + 0x148, d.mib);
        self.log.borrow_mut().clear();
    }

    fn get(&self) -> (Download, Option<(usize, u32)>) {
        let o = self.obj;
        let r = |a: u64| self.e.read_u32(a);
        let q = |a: u64| u64::from_le_bytes(self.e.read(a, 8).try_into().unwrap());
        let at = r(o);
        let file = (at != 0).then(|| {
            let i = which(self.files, at);
            (i, r(self.files[i] + 8))
        });
        let d = Download {
            magic: r(o + 8),
            zero: r(o + 0xc),
            digest: self.e.read(o + 0x10, 16).try_into().unwrap(),
            received: q(o + 0x20),
            total: q(o + 0x28),
            partial: self.e.read(o + 0x134, 1)[0],
            state: r(o + 0x138),
            error: r(o + 0x13c),
            fed: r(o + 0x144),
            mib: r(o + 0x148),
        };
        (d, file)
    }
}

fn download(g: &mut Lcg) -> Download {
    Download {
        magic: TRAILER_MAGIC,
        zero: 0,
        digest: std::array::from_fn(|_| (g.next() >> 24) as u8),
        received: u64::from(g.next() >> 20),
        total: u64::from(g.next() >> 20),
        partial: (g.next() >> 31) as u8,
        state: g.next() >> 30,
        error: g.next() >> 30,
        fed: g.next() >> 12,
        mib: 0,
    }
}

fn bytes(g: &mut Lcg, n: usize) -> Vec<u8> {
    (0..n).map(|_| (g.next() >> 24) as u8).collect()
}

/// A file of `data` that reads and writes, the trailer after it when `trailer` is given.
fn file_of(data: &[u8], trailer: Option<Vec<u8>>, g: &mut Lcg) -> FakeFile {
    let mut d = data.to_vec();
    if let Some(t) = trailer {
        d.extend(t);
    }
    FakeFile {
        data: d,
        read_ok: g.next() >> 29 != 0,
        write_ok: g.next() >> 29 != 0,
        resize_ok: g.next() >> 29 != 0,
    }
}

/// A trailer for `d`: as kept, or with one field spoiled.
fn trailer(d: &Download, received: u64, g: &mut Lcg) -> Vec<u8> {
    let mut t = TRAILER_MAGIC.to_le_bytes().to_vec();
    t.extend(0u32.to_le_bytes());
    t.extend(d.digest);
    t.extend(received.to_le_bytes());
    t.extend(d.total.to_le_bytes());
    if g.next() >> 29 == 0 {
        let at = [0usize, 4, 8, 0x17, 0x20, 0x27][(g.next() >> 16) as usize % 6];
        t[at] ^= 1;
    }
    t
}

/// `ReleaseRefChain` whole on chains of 1 to 5 objects with counts of 1, 2 and others: the
/// objects deleted, in order, and every count.
#[test]
fn download_release_chain() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let log = Rc::new(RefCell::new(Vec::new()));
    let objs: Vec<u64> = (0..5).map(|_| e.alloc(0x10)).collect();
    for &o in &objs {
        let vt = e.alloc(0x10);
        let slot = e.alloc(0x10);
        e.write_u32(o, vt as u32);
        e.write_u32(vt, slot as u32);
        let l = log.clone();
        e.stub_fn(slot, 1, move |c| {
            assert_eq!(c.arg(0), 1, "the deleting flag");
            l.borrow_mut().push(c.reg(RegisterX86::ECX) as u32);
            0
        });
    }
    let mut g = Lcg(0xc4a1_0001);
    for case in 0..2000 {
        let n = 1 + (g.next() >> 16) as usize % 5;
        let mut counts: Vec<u32> = (0..n)
            .map(|_| [1u32, 1, 2, 3, 0][(g.next() >> 16) as usize % 5])
            .collect();
        let next: Vec<Option<usize>> = (0..n).map(|k| (k + 1 < n).then_some(k + 1)).collect();
        for k in 0..n {
            e.write_u32(objs[k] + 4, next[k].map_or(0, |j| objs[j] as u32));
            e.write_u32(objs[k] + 8, counts[k]);
        }
        log.borrow_mut().clear();
        e.call(RELEASE_REF_CHAIN, objs[0] as u32, 0, &[]);
        let deleted = release_ref_chain(&mut counts, &next, 0);
        let want: Vec<u32> = deleted.iter().map(|&k| objs[k] as u32).collect();
        assert_eq!(*log.borrow(), want, "case {case}: deleted");
        for k in 0..n {
            assert_eq!(e.read_u32(objs[k] + 8), counts[k], "case {case}: count {k}");
        }
    }
}

/// `DigestFileRange` whole on files of 0 to 70,000 bytes, across the 32 KiB chunk, with a start
/// of 0 and others, ends past the file and reads that fail: the 16 bytes written.
#[test]
fn download_digest_range() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let out = r.e.alloc(0x10);
    let mut g = Lcg(0xd16e_0001);
    for case in 0..300 {
        let n = [0usize, 1, 64, 0x7fff, 0x8000, 0x8001, 70_000][(g.next() >> 16) as usize % 7];
        let data = bytes(&mut g, n);
        let mut f = file_of(&data, None, &mut g);
        let start = if g.next() >> 30 == 0 {
            (g.next() >> 16) % 0x9000
        } else {
            0
        };
        let end = match g.next() >> 30 {
            0 => n as u32 + (g.next() >> 28),
            _ => (g.next() >> 8) % (n as u32 + 1),
        };
        r.disk.borrow_mut().files[0] = Some(f.clone());
        r.e.write(out, &[0xa5; 16]);
        r.e.call(
            DIGEST_RANGE,
            r.obj as u32,
            0,
            &[
                r.files[0] as u32,
                start,
                g.next(),
                end,
                g.next(),
                out as u32,
            ],
        );
        let want = digest_file_range(&mut f, start, end);
        assert_eq!(
            r.e.read(out, 16),
            want,
            "case {case} n {n} start {start:#x} end {end:#x}"
        );
    }
}

/// `DownloadFileReadTrailer` whole on files exactly the total long and one trailer longer, with
/// trailers whole or spoiled in each field, saying all the data arrived or not, data whose MD5
/// matches or not, other sizes and failed reads: the result, the completion flag and the fields.
#[test]
fn download_read_trailer() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let flag = r.e.alloc(0x10);
    let mut g = Lcg(0x7a11_0001);
    for case in 0..3000 {
        let n = (g.next() >> 16) as usize % 300;
        let data = bytes(&mut g, n);
        let mut d = download(&mut g);
        d.total = n as u64;
        if g.next() >> 30 != 0 {
            d.digest = benilla_re_storm::md5(&data);
        }
        let received = if g.next() >> 30 == 0 {
            u64::from(g.next() >> 24)
        } else {
            d.total
        };
        let t = trailer(&d, received, &mut g);
        let mut f = match g.next() >> 29 {
            0 => file_of(&data[..n / 2], None, &mut g),
            1 | 2 => file_of(&data, None, &mut g),
            _ => file_of(&data, Some(t), &mut g),
        };
        r.disk.borrow_mut().files[0] = Some(f.clone());
        r.put(&d, Some((0, 1)));
        r.e.write(flag, &[0xa5]);
        let ret = r.e.call(READ_TRAILER, r.obj as u32, 0, &[flag as u32]) & 0xff;
        let (found, complete) = read_trailer(&mut d, &mut f);
        let ctx = format!("case {case} n {n}");
        assert_eq!(
            (ret, r.e.read(flag, 1)[0]),
            (u32::from(found), u8::from(complete)),
            "{ctx}"
        );
        assert_eq!(r.get().0, d, "{ctx}: fields");
    }
}

/// `DownloadFileProgress` whole on counts and totals of 0, small, at 2^53 and above, and with
/// the top bit set: the float it returns.
#[test]
fn download_progress() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x9e0c_0001);
    let pick = |g: &mut Lcg| -> u64 {
        match g.next() >> 29 {
            0 => 0,
            1 => u64::from(g.next() >> 20),
            2 => (1 << 53) + u64::from(g.next()),
            3 => u64::MAX - u64::from(g.next()),
            4 => 1 << 63,
            _ => (u64::from(g.next()) << 32) | u64::from(g.next()),
        }
    };
    for case in 0..20000 {
        let mut d = download(&mut g);
        d.received = pick(&mut g);
        d.total = pick(&mut g);
        r.put(&d, None);
        let got = r.e.call_f32(PROGRESS, r.obj as u32, 0, &[]);
        let want = progress(&d);
        assert!(
            got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan()),
            "case {case} {:#x}/{:#x}: {got} {want}",
            d.received,
            d.total
        );
    }
}

/// `DownloadFileFeed` whole: no file, writes that fail, chunks that end short of the total, at it
/// and past it, the MiB counter below and past 1 MiB, data whose MD5 matches or not, trailer
/// writes that fail: the result, every field and the file's bytes.
#[test]
fn download_feed() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let buf = r.e.alloc(0x400);
    let mut g = Lcg(0xfeed_0001);
    for case in 0..3000 {
        let total = 1 + (g.next() >> 16) as usize % 600;
        let full = bytes(&mut g, total);
        let mut d = download(&mut g);
        d.total = total as u64;
        d.received = u64::from((g.next() >> 16) % (total as u32 + 1));
        if g.next() >> 30 != 0 {
            d.digest = benilla_re_storm::md5(&full);
        }
        d.fed = [0u32, 0xf_ff00, 0x10_0000, 0xffff_fff0][(g.next() >> 16) as usize % 4];
        let have = d.received as usize;
        let n = match g.next() >> 30 {
            0 => total - have,
            1 => total - have + 1 + (g.next() >> 28) as usize,
            _ => (g.next() >> 16) as usize % (total - have + 1),
        };
        let chunk: Vec<u8> = (have..have + n)
            .map(|i| full.get(i).copied().unwrap_or(0x5a))
            .collect();
        let mut f = file_of(&full[..have], None, &mut g);
        let held = g.next() >> 29 != 0;
        r.disk.borrow_mut().files[0] = Some(f.clone());
        r.put(&d, held.then_some((0, 1)));
        r.e.write(buf, &chunk);
        let ret = r.e.call(FEED, r.obj as u32, 0, &[buf as u32, n as u32]);
        let want = feed_download(&mut d, held.then_some(&mut f), &chunk);
        let ctx = format!("case {case} total {total} received {have} n {n}");
        assert_eq!(ret, want, "{ctx}: result");
        assert_eq!(r.get().0, d, "{ctx}: fields");
        assert_eq!(
            r.disk.borrow().files[0].as_ref().unwrap().data,
            f.data,
            "{ctx}: file"
        );
    }
}

/// `DownloadFileRelease` whole: a file object held with a count of 1 or more, or none, and every
/// pair of the delete and commit flags: the calls, the count and `+0`.
#[test]
fn download_release() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x4e1e_0001);
    for case in 0..200 {
        let d = download(&mut g);
        let count = [0u32, 1, 1, 2][(g.next() >> 16) as usize % 4];
        let (delete, commit) = (g.next() >> 31 == 0, g.next() >> 31 == 0);
        r.put(&d, (count != 0).then_some((0, count)));
        r.e.call(
            RELEASE,
            r.obj as u32,
            0,
            &[u32::from(delete), u32::from(commit)],
        );
        let mut file = (count != 0).then_some(count);
        let mut calls = Vec::new();
        release_download(&mut file, delete, commit, &mut calls);
        let ctx = format!("case {case} count {count} delete {delete} commit {commit}");
        assert_eq!(*r.log.borrow(), calls, "{ctx}: calls");
        assert_eq!(r.get().1, None, "{ctx}: +0 cleared");
        if count != 0 {
            assert_eq!(r.e.read_u32(r.files[0] + 8), count - 1, "{ctx}: count");
        }
    }
}

/// `DownloadFileOpen` whole on every arrangement of the disk: a `.partial` with a trailer whole,
/// spoiled or complete, a final file complete or not, files that do not open, a create that fails
/// and a resize that fails: the result, the offset, the fields, the path kept, the calls, the
/// file held and what the disk holds after.
#[test]
fn download_open() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let path = r.e.alloc(0x200);
    let digest_at = r.e.alloc(0x10);
    let offset_at = r.e.alloc(0x10);
    let mut g = Lcg(0x09e1_0001);
    for case in 0..4000 {
        let n = (g.next() >> 16) as usize % 200;
        let data = bytes(&mut g, n);
        let mut d = download(&mut g);
        let digest = if g.next() >> 30 != 0 {
            benilla_re_storm::md5(&data)
        } else {
            std::array::from_fn(|_| (g.next() >> 24) as u8)
        };
        let total = if g.next() >> 30 == 0 {
            u64::from(g.next()) << 12
        } else {
            n as u64
        };
        let mut probe = d.clone();
        probe.total = total;
        probe.digest = digest;
        let received = if g.next() >> 30 == 0 {
            u64::from(g.next() >> 24)
        } else {
            total
        };
        let partial = match g.next() >> 30 {
            0 => None,
            1 => Some(file_of(&data[..n / 2], None, &mut g)),
            _ => Some(file_of(
                &data,
                Some(trailer(&probe, received, &mut g)),
                &mut g,
            )),
        };
        let fin = (g.next() >> 30 != 0).then(|| file_of(&data, None, &mut g));
        let mut disk = TestDisk {
            files: [partial, fin],
            opens: [g.next() >> 29 != 0, g.next() >> 29 != 0],
            create_ok: g.next() >> 29 != 0,
            fresh: FakeFile {
                data: Vec::new(),
                read_ok: true,
                write_ok: true,
                resize_ok: g.next() >> 29 != 0,
            },
        };
        let held = (g.next() >> 30 == 0).then_some(1u32);
        // Names whose `.partial` form fits the 0x104-byte buffers.
        let name: Vec<u8> = (0..[1usize, 3, 40, 0xfb][(g.next() >> 16) as usize % 4])
            .map(|_| b'a' + ((g.next() >> 16) % 26) as u8)
            .collect();
        *r.disk.borrow_mut() = disk.clone();
        r.put(&d, held.map(|c| (1, c)));
        r.e.write(path, &name);
        r.e.write(path + name.len() as u64, &[0]);
        r.e.write(digest_at, &digest);
        r.e.write(offset_at, &[0xa5; 8]);
        let ret = r.e.call(
            OPEN,
            r.obj as u32,
            0,
            &[
                path as u32,
                digest_at as u32,
                total as u32,
                (total >> 32) as u32,
                offset_at as u32,
            ],
        ) & 0xff;
        let mut file = held;
        let mut calls = Vec::new();
        let (want, offset) = open_download(&mut d, &mut file, &mut disk, digest, total, &mut calls);
        let ctx = format!("case {case} n {n} total {total:#x} name {}", name.len());
        let (got_d, got_file) = r.get();
        assert_eq!(*r.log.borrow(), calls, "{ctx}: calls");
        assert_eq!(
            (ret, r.e.read(offset_at, 8)),
            (u32::from(want), offset.to_le_bytes().to_vec()),
            "{ctx}: result"
        );
        assert_eq!(got_d, d, "{ctx}: fields");
        assert_eq!(got_file.map(|(_, c)| c), file, "{ctx}: the file held");
        let kept = r.e.read(r.obj + 0x30, 0x104);
        let kept: Vec<u8> = kept.into_iter().take_while(|&b| b != 0).collect();
        assert_eq!(kept, name, "{ctx}: the path");
        let after = r.disk.borrow();
        assert_eq!(after.files[0], disk.files[0], "{ctx}: the .partial");
    }
}
