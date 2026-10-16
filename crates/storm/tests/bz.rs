//! The `.bz` loose file against the exe: the open's header slice of `SFile::OpenEx`, the read
//! `0x6476e0`, `SFile::SetFilePointer` (`0x648a60`) and `SFile::GetFileSize` (`0x6487f0`) for a
//! kind-1 file object, with the exe's own zlib inflate running and the C runtime's `fread` and
//! `fseek` stubbed over an in-memory file. Against `benilla_re_storm::BzFile`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86};
use benilla_re_storm::{parse_header, BzFile, BzHeader, BzSource, HEADER_LEN};
use common::Wow;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

const BZ_READ: u64 = 0x6476e0;
const SET_FILE_POINTER: u64 = 0x648a60;
const GET_FILE_SIZE: u64 = 0x6487f0;
/// `"1.1.3"`, the version string the open passes to `inflateInit_`.
const ZLIB_VERSION_ARG: u32 = 0x865be0;
const FREAD: u64 = 0x740c97;
const FSEEK: u64 = 0x740dae;
/// The C runtime's `malloc`, which zlib's allocator `0x7348f0` calls, and zlib's `zcfree`.
const MALLOC: u64 = 0x40bc91;
const ZCFREE: u64 = 0x734910;
const S_MEM_ALLOC: u64 = 0x6462e0;
const CRIT_ENTER: u64 = 0x6579b0;
const CRIT_LEAVE: u64 = 0x6579c0;
/// The `FILE *` the stubs answer for.
const FILE_HANDLE: u32 = 0xf11e;
const UNWRITTEN: u32 = 0xdead_beef;

/// The file under the handle: its bytes and the C runtime's cursor.
struct Disk {
    bytes: Vec<u8>,
    cursor: usize,
    /// The `fseek` calls, one per restart of the stream.
    seeks: usize,
}

#[derive(Clone)]
struct Shared(Rc<RefCell<Disk>>);

impl BzSource for Shared {
    fn read(&mut self, buf: &mut [u8]) -> usize {
        let mut d = self.0.borrow_mut();
        let n = buf.len().min(d.bytes.len().saturating_sub(d.cursor));
        let at = d.cursor;
        buf[..n].copy_from_slice(&d.bytes[at..at + n]);
        d.cursor += n;
        n
    }
    fn rewind(&mut self) {
        let mut d = self.0.borrow_mut();
        d.cursor = HEADER_LEN;
        d.seeks += 1;
    }
}

/// The reimplementation's own copy of the file, positioned after the header as the open leaves it.
fn own_disk(bytes: &[u8]) -> Shared {
    Shared(Rc::new(RefCell::new(Disk {
        bytes: bytes.to_vec(),
        cursor: HEADER_LEN,
        seeks: 0,
    })))
}

fn zlib(data: &[u8], level: u32) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

fn bz_file(size: u32, stream: &[u8]) -> Vec<u8> {
    let mut f = size.to_le_bytes().to_vec();
    f.extend_from_slice(b"BZ00");
    f.extend((0..16u8).map(|i| i.wrapping_mul(37) ^ 0x5a));
    f.extend_from_slice(stream);
    f
}

fn rng(seed: u32) -> impl FnMut() -> u32 {
    let mut s = seed;
    move || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s >> 8
    }
}

/// Inflated contents: empty, one byte, text-like, incompressible and long runs.
fn payloads() -> Vec<Vec<u8>> {
    let mut next = rng(0x1234_5678);
    let text: Vec<u8> = (0..70_000)
        .map(|i| b"the quick brown fox jumps over "[i % 31])
        .collect();
    let noise: Vec<u8> = (0..20_000).map(|_| next() as u8).collect();
    let runs: Vec<u8> = (0..300_000).map(|i| (i / 5000) as u8).collect();
    vec![Vec::new(), vec![0x42], text, noise, runs]
}

struct Rig<'a> {
    emu: Emu<'a>,
    disk: Rc<RefCell<Disk>>,
    file: u64,
    out: u64,
    count: u64,
}

const OUT_LEN: usize = 0x20000;

impl<'a> Rig<'a> {
    fn new(wow: &'a Wow, bytes: Vec<u8>) -> Rig<'a> {
        let mut emu = Emu::new(wow);
        let disk = Rc::new(RefCell::new(Disk {
            bytes,
            cursor: 0,
            seeks: 0,
        }));
        let arena = emu.alloc(0x80_0000);
        emu.stub_alloc_sized(MALLOC, arena, arena + 0x80_0000, 0, 0);
        emu.stub_alloc_sized(S_MEM_ALLOC, arena, arena + 0x80_0000, 0, 4);
        emu.stub(ZCFREE, 0, 0);
        emu.stub(CRIT_ENTER, 0, 0);
        emu.stub(CRIT_LEAVE, 0, 0);
        let d = disk.clone();
        emu.stub_fn(FREAD, 0, move |io| {
            let (buf, size, count, f) = (io.arg(0), io.arg(1), io.arg(2), io.arg(3));
            assert_eq!(f, FILE_HANDLE);
            let want = (size * count) as usize;
            let mut d = d.borrow_mut();
            let n = want.min(d.bytes.len().saturating_sub(d.cursor));
            let chunk = d.bytes[d.cursor..d.cursor + n].to_vec();
            d.cursor += n;
            io.write(u64::from(buf), &chunk);
            (n / size as usize) as u32
        });
        let d = disk.clone();
        emu.stub_fn(FSEEK, 0, move |io| {
            assert_eq!(io.arg(0), FILE_HANDLE);
            assert_eq!(io.arg(2), 0, "only SEEK_SET");
            let mut d = d.borrow_mut();
            d.cursor = io.arg(1) as usize;
            d.seeks += 1;
            0
        });
        let file = emu.alloc(0x60);
        let out = emu.alloc(OUT_LEN as u64);
        let count = emu.alloc(4);
        Rig {
            emu,
            disk,
            file,
            out,
            count,
        }
    }

    /// The object `SFile::OpenEx` leaves for a kind-1 file, built by the exe's own open slice from
    /// the header the stub hands it. Returns the header the slice accepted, or `None` when it took
    /// the wrong-magic branch.
    fn open(&mut self) -> Option<BzHeader> {
        let slot = self.emu.alloc(4);
        self.emu.write_u32(slot, self.file as u32);
        self.emu.write_u32(self.file, 1);
        self.emu.write_u32(self.file + 4, FILE_HANDLE);
        let esp = 0x7008_0000u64;
        self.emu.set_reg(RegisterX86::ESP, esp);
        self.emu.set_reg(RegisterX86::EBP, esp + 0x200);
        self.emu.set_reg(RegisterX86::ESI, slot);
        self.emu.set_reg(RegisterX86::EBX, 0);
        self.disk.borrow_mut().cursor = 0;
        let bytes = self.disk.borrow().bytes.clone();
        let header: [u8; HEADER_LEN] = bytes[..HEADER_LEN].try_into().unwrap();
        let want = parse_header(&header);
        // 0x647940 reads the header; 0x647966 is the wrong-magic branch, 0x647a22 the end of the
        // success branch after inflateInit_.
        let until = if want.is_some() { 0x647a22 } else { 0x647966 };
        self.emu.run(0x647940, until);
        if let Some(h) = want {
            assert_eq!(self.emu.read_u32(self.file + 0x14), h.size, "+0x14 size");
            assert_eq!(
                self.emu.read(self.file + 0x44, 16),
                h.extra.to_vec(),
                "+0x44 extra bytes"
            );
            assert_eq!(self.emu.read_u32(self.file + 0x54), 1, "+0x54");
            assert_eq!(self.emu.read_u32(self.file + 0x20), 0, "+0x20 position");
            assert_eq!(self.disk.borrow().cursor, HEADER_LEN, "fread of the header");
        }
        want
    }

    fn read(&mut self, size: usize) -> (u32, Option<u32>, Vec<u8>) {
        self.emu.write_u32(self.count, UNWRITTEN);
        let eax = self.emu.call(
            BZ_READ,
            self.file as u32,
            self.out as u32,
            &[size as u32, self.count as u32],
        );
        let c = self.emu.read_u32(self.count);
        let read = (c != UNWRITTEN).then_some(c);
        let bytes = self.emu.read(self.out, read.unwrap_or(0) as usize);
        (eax, read, bytes)
    }

    fn seek(&mut self, distance: i32, method: u32) -> u32 {
        self.emu.call(
            SET_FILE_POINTER,
            0,
            0,
            &[self.file as u32, distance as u32, 0, method],
        )
    }

    fn size(&mut self) -> (u32, u32) {
        self.emu.write_u32(self.count, UNWRITTEN);
        let eax = self
            .emu
            .call(GET_FILE_SIZE, 0, 0, &[self.file as u32, self.count as u32]);
        (eax, self.emu.read_u32(self.count))
    }

    fn position(&self) -> u32 {
        self.emu.read_u32(self.file + 0x20)
    }
}

fn check_read<S: BzSource>(rig: &mut Rig, bz: &mut BzFile<S>, size: usize, what: &str) {
    let (eax, read, bytes) = rig.read(size);
    let mut mine = vec![0u8; size];
    let r = bz.read(&mut mine);
    assert_eq!((eax != 0, read), (r.ok, r.read), "{what}: read {size}");
    if let Some(n) = read {
        assert_eq!(
            bytes,
            mine[..n as usize].to_vec(),
            "{what}: bytes of read {size}"
        );
    }
    assert_eq!(
        rig.position(),
        bz.position(),
        "{what}: position after read {size}"
    );
}

/// Whole streams read in chunks of every regime: 1 byte, the 4 KiB input size and around it, the
/// whole payload, and reads past the end, at zlib levels 0 (stored), 1, 6 and 9.
#[test]
fn bz_read_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    for (pi, data) in payloads().iter().enumerate() {
        for level in [0, 1, 6, 9] {
            let file = bz_file(data.len() as u32, &zlib(data, level));
            for plan in 0..4u32 {
                let mut rig = Rig::new(&wow, file.clone());
                let h = rig.open().expect("magic");
                let (size, high) = rig.size();
                assert_eq!((size, high), (h.size, UNWRITTEN), "GetFileSize");
                let mut bz = BzFile::new(own_disk(&file), h);
                let mut next = rng(pi as u32 * 100 + level * 10 + plan);
                let what = format!("payload {pi} level {level} plan {plan}");
                for step in 0..60 {
                    let size = match plan {
                        0 => 1 + next() as usize % 0x3000,
                        1 => [1, 0xfff, 0x1000, 0x1001, 0x2000][step % 5],
                        2 => OUT_LEN,
                        _ => 1 + next() as usize % 64,
                    };
                    check_read(&mut rig, &mut bz, size, &what);
                }
            }
        }
    }
}

/// Damaged streams: cut at several points, a byte flipped in the body, and the adler-32 trailer
/// changed, so inflate ends in Z_BUF_ERROR or Z_DATA_ERROR.
#[test]
fn bz_damaged_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    // Sixteen symbols at random: a long Huffman-coded stream, so damage lands mid-stream.
    let mut next = rng(0x5eed);
    let data: &Vec<u8> = &(0..70_000)
        .map(|_| b"abcdefghijklmnop"[next() as usize % 16])
        .collect();
    let stream = zlib(data, 6);
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for cut in [
        0,
        1,
        2,
        10,
        stream.len() / 2,
        stream.len() - 4,
        stream.len() - 1,
    ] {
        cases.push(stream[..cut].to_vec());
    }
    for k in 1..48 {
        let mut flipped = stream.clone();
        flipped[2 + (stream.len() - 6) * k / 48] ^= 0x55;
        cases.push(flipped);
    }
    let mut adler = stream.clone();
    let n = adler.len();
    adler[n - 1] ^= 1;
    cases.push(adler);
    for (ci, s) in cases.iter().enumerate() {
        // A read of the whole length fills the output before the trailer's check fails, and a seek
        // to the end stops at the last 4 KiB step that succeeded.
        for size in [data.len(), data.len() + 1] {
            let mut rig = Rig::new(&wow, bz_file(data.len() as u32, s));
            let h = rig.open().expect("magic");
            let mut bz = BzFile::new(own_disk(&bz_file(data.len() as u32, s)), h);
            check_read(&mut rig, &mut bz, size, &format!("damaged case {ci} whole"));
        }
        let mut rig = Rig::new(&wow, bz_file(data.len() as u32, s));
        let h = rig.open().expect("magic");
        let mut bz = BzFile::new(own_disk(&bz_file(data.len() as u32, s)), h);
        let what = format!("damaged case {ci} seek");
        assert_eq!(
            rig.seek(data.len() as i32, 0),
            bz.seek(data.len() as i32, 0),
            "{what}"
        );
        assert_eq!(rig.position(), bz.position(), "{what}: position");
    }
    let mut early = 0;
    for (ci, s) in cases.iter().enumerate() {
        for size in [0x800usize, 0x5000] {
            let mut rig = Rig::new(&wow, bz_file(data.len() as u32, s));
            let h = rig.open().expect("magic");
            let mut bz = BzFile::new(own_disk(&bz_file(data.len() as u32, s)), h);
            for _ in 0..80 {
                check_read(&mut rig, &mut bz, size, &format!("damaged case {ci}"));
            }
            if (bz.position() as usize) < data.len() - 0x5000 {
                early += 1;
            }
        }
    }
    // Some damage stops the stream well before its end, not only at the trailer's check.
    assert!(early > 0, "no damaged case failed mid-stream");
}

/// Seeks from the start, from the position and from the end, forward, backward (which starts the
/// stream again), to zero, past the end and by a negative distance, each followed by a read.
#[test]
fn bz_seek_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    for (pi, data) in payloads().iter().enumerate() {
        let file = bz_file(data.len() as u32, &zlib(data, 6));
        let mut rig = Rig::new(&wow, file);
        let h = rig.open().expect("magic");
        let mine = own_disk(&bz_file(data.len() as u32, &zlib(data, 6)));
        let seeks = mine.clone();
        let bz_seeks = move |_: &BzFile<Shared>| seeks.0.borrow().seeks;
        let mut bz = BzFile::new(mine, h);
        let len = data.len() as i32;
        let mut next = rng(0xbeef + pi as u32);
        let mut moves: Vec<(i32, u32)> = vec![
            (0, 0),
            (100, 0),
            (50, 0),
            (0, 1),
            (4096, 1),
            (-10, 1),
            (5, 2),
            (len, 0),
            (len + 1, 0),
            (0, 0),
            (len - 1, 0),
            (3, 0),
            (0x1001, 1),
        ];
        for _ in 0..40 {
            let d = if len > 0 {
                (next() % (len as u32 + 2)) as i32
            } else {
                next() as i32 % 3
            };
            moves.push((d, next() % 3));
        }
        for (i, &(d, m)) in moves.iter().enumerate() {
            let what = format!("payload {pi} move {i} ({d}, {m})");
            let got = rig.seek(d, m);
            let want = bz.seek(d, m);
            assert_eq!(got, want, "{what}: SetFilePointer");
            assert_eq!(rig.position(), bz.position(), "{what}: position");
            assert_eq!(rig.disk.borrow().seeks, bz_seeks(&bz), "{what}: restarts");
            check_read(&mut rig, &mut bz, 1 + next() as usize % 0x1800, &what);
        }
    }
}

/// The open's header slice takes the magic at offset 4 only: `BZ00` there passes, anything else
/// (the magic at offset 0, a lower-case magic, one byte off) takes the failure branch.
#[test]
fn bz_open_header_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let stream = zlib(b"abc", 6);
    let good = bz_file(3, &stream);
    let mut at0 = good.clone();
    at0[0..4].copy_from_slice(b"BZ00");
    at0[4..8].copy_from_slice(&3u32.to_le_bytes());
    let mut lower = good.clone();
    lower[4..8].copy_from_slice(b"bz00");
    let mut off = good.clone();
    off[7] = b'1';
    for (f, ok) in [(good, true), (at0, false), (lower, false), (off, false)] {
        let mut rig = Rig::new(&wow, f);
        assert_eq!(rig.open().is_some(), ok);
        if ok {
            // The slice's inflateInit_ left a stream that inflates.
            let (eax, read, bytes) = rig.read(3);
            assert_eq!((eax, read, bytes), (1, Some(3), b"abc".to_vec()));
        }
    }
    // The client's version argument is "1.1.3"; inflateInit2_ checks only its first character.
    let emu = Emu::new(&wow);
    assert_eq!(
        emu.read(u64::from(ZLIB_VERSION_ARG), 6),
        b"1.1.3\0".to_vec()
    );
}

/// Deflate's bit order: values least significant bit first.
struct Bits {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Bits {
    fn put(&mut self, value: u32, count: u32) {
        for i in 0..count {
            self.acc |= ((value >> i) & 1) << self.n;
            self.n += 1;
            if self.n == 8 {
                self.out.push(self.acc as u8);
                self.acc = 0;
                self.n = 0;
            }
        }
    }
}

/// A zlib stream of one final dynamic block whose literal/length code gives only `a` and `b` a
/// code (one bit each) and end-of-block none, then `symbols` one-bit codes alternating `a`, `b`.
fn no_eob_stream(symbols: usize) -> Vec<u8> {
    let mut b = Bits {
        out: vec![0x78, 0x01],
        acc: 0,
        n: 0,
    };
    b.put(1, 1); // BFINAL
    b.put(2, 2); // BTYPE dynamic
    b.put(0, 5); // HLIT: 257 codes
    b.put(0, 5); // HDIST: 1 code
    b.put(14, 4); // HCLEN: 18 code-length code lengths
                  // In the order 16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1: symbols 18 and 1
                  // have one bit each, so 1 is code 0 and 18 is code 1.
    for len in [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1] {
        b.put(len, 3);
    }
    let zeros = |b: &mut Bits, n: u32| {
        b.put(1, 1); // symbol 18
        b.put(n - 11, 7);
    };
    zeros(&mut b, 97); // literals 0 to 96
    b.put(0, 1); // 97 'a': length 1
    b.put(0, 1); // 98 'b': length 1
    zeros(&mut b, 138); // 99 to 236
    zeros(&mut b, 20); // 237 to 256: end-of-block gets no code
    b.put(0, 1); // distance code 0: length 1
    for k in 0..symbols {
        b.put((k % 2) as u32, 1); // 'a' is code 0, 'b' code 1
    }
    b.put(0, 7);
    b.out
}

/// The known divergence of the reimplementation's zlib-rs backend: the client's zlib 1.2.2 has no
/// end-of-block test after the code lengths (`0x731994`, then `inflate_table` at `0x7319c9`), so it
/// decodes the block above, while zlib-rs fails it at its header. The test holds both answers.
#[test]
fn bz_missing_eob_divergence() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let file = bz_file(4000, &no_eob_stream(4000));
    let mut rig = Rig::new(&wow, file.clone());
    let h = rig.open().expect("magic");
    let want: Vec<u8> = (0..1000)
        .map(|k| if k % 2 == 0 { b'a' } else { b'b' })
        .collect();
    assert_eq!(
        rig.read(1000),
        (1, Some(1000), want),
        "the client decodes on"
    );
    let mut bz = BzFile::new(own_disk(&file), h);
    let mut out = vec![0u8; 1000];
    let r = bz.read(&mut out);
    assert_eq!((r.ok, r.read), (false, None), "zlib-rs fails the header");
}
