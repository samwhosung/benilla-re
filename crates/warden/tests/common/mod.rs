//! A small deterministic generator and the host the loader tests share.
#![allow(dead_code)]

use benilla_re_warden::image::Host;

pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
}

/// One thing the loader asks the process for, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Alloc(u32),
    LoadLibrary(Vec<u8>),
    ProcName(u32, Vec<u8>),
    ProcOrdinal(u32, u32),
}

/// The host the emulator stubs match: `LoadLibraryA` and `GetProcAddress` each return the next
/// value of one counter, from `FIRST` in steps of `STRIDE`; `VirtualAlloc` returns `base`.
pub struct CountingHost {
    pub base: u32,
    pub next: u32,
    pub events: Vec<Event>,
    pub library_fails: bool,
}

pub const FIRST: u32 = 0x5000_0000;
pub const STRIDE: u32 = 0x10;

impl CountingHost {
    pub fn new(base: u32, library_fails: bool) -> Self {
        CountingHost {
            base,
            next: FIRST,
            events: Vec::new(),
            library_fails,
        }
    }
    fn take(&mut self) -> u32 {
        let v = self.next;
        self.next += STRIDE;
        v
    }
}

impl Host for CountingHost {
    fn alloc(&mut self, size: u32) -> Option<u32> {
        self.events.push(Event::Alloc(size));
        Some(self.base)
    }
    fn load_library(&mut self, name: &[u8]) -> Option<u32> {
        self.events.push(Event::LoadLibrary(name.to_vec()));
        if self.library_fails {
            None
        } else {
            Some(self.take())
        }
    }
    fn proc_by_name(&mut self, library: u32, name: &[u8]) -> u32 {
        self.events.push(Event::ProcName(library, name.to_vec()));
        self.take()
    }
    fn proc_by_ordinal(&mut self, library: u32, ordinal: u32) -> u32 {
        self.events.push(Event::ProcOrdinal(library, ordinal));
        self.take()
    }
}

/// A module blob built from a flat image, and what the tests compare the loader's result with.
pub struct Built {
    pub blob: Vec<u8>,
    /// The image the loader makes before relocations, imports and the host's answers.
    pub flat: Vec<u8>,
    pub size: u32,
    pub libraries: u32,
    pub export_count: u32,
    pub export_base: u32,
}

const LIBS: [&str; 4] = ["KERNEL32.dll", "user32.dll", "ntdll.dll", "advapi32.dll"];
const PROCS: [&str; 6] = [
    "VirtualAlloc",
    "GetTickCount",
    "Sleep",
    "CreateFileA",
    "ReadFile",
    "ExitProcess",
];

fn put(flat: &mut [u8], at: u32, v: u32) {
    flat[at as usize..at as usize + 4].copy_from_slice(&v.to_le_bytes());
}

fn put_str(flat: &mut [u8], at: &mut u32, s: &str) -> u32 {
    let rva = *at;
    flat[rva as usize..rva as usize + s.len()].copy_from_slice(s.as_bytes());
    *at += s.len() as u32 + 1;
    *at = (*at + 3) & !3;
    rva
}

/// Builds a module: header, 1 to 4 sections, an export table, `libraries` imports of 1 to 4 slots
/// each (names and ordinals mixed), a relocation stream of short entries, and alternating copy and
/// skip runs that cover the image from the first section's address to its end.
pub fn build(rng: &mut Rng, libraries: u32) -> Built {
    let size = (0x2000 + rng.below(0x3000)) & !3;
    let start = 0x1000u32;
    let mut flat = vec![0u8; size as usize];
    let mut cur = start;

    // Exports.
    let export_count = 1 + rng.below(4);
    let export_base = rng.below(3);
    let export_rva = cur;
    for i in 0..export_count {
        put(&mut flat, cur + 4 * i, 0x1000 + 4 * rng.below(0x400));
    }
    cur += 4 * export_count;

    // Imports.
    let import_rva = cur;
    cur += 8 * libraries;
    for l in 0..libraries {
        let name = put_str(&mut flat, &mut cur, LIBS[l as usize % 4]);
        let slots = cur;
        let n = 1 + rng.below(4);
        cur += 4 * (n + 1);
        for s in 0..n {
            let v = if rng.below(3) == 0 {
                0x8000_0000 | (1 + rng.below(500))
            } else {
                put_str(&mut flat, &mut cur, PROCS[rng.below(6) as usize])
            };
            put(&mut flat, slots + 4 * s, v);
        }
        put(&mut flat, import_rva + 8 * l, name);
        put(&mut flat, import_rva + 8 * l + 4, slots);
    }

    // The relocation stream, then the code area, where the relocations land.
    let reloc_count = rng.below(40);
    let reloc_rva = cur;
    cur += 2 * reloc_count;
    let code_start = (cur + 0x40 + 15) & !15;
    let mut targets: Vec<u32> = (0..reloc_count)
        .map(|_| code_start + rng.below(size - code_start - 4))
        .collect();
    targets.sort_unstable();
    let mut prev = 0u32;
    let mut stream = Vec::new();
    for &t in &targets {
        // The image is under 0x7fff bytes, so every delta fits a two-byte entry.
        let delta = t - prev;
        assert!(delta <= 0x7fff);
        stream.extend_from_slice(&[(delta >> 8) as u8, delta as u8]);
        prev = t;
    }
    let entries = reloc_count;
    flat[reloc_rva as usize..reloc_rva as usize + stream.len()].copy_from_slice(&stream);
    let first_run = code_start.max(cur) - start;

    // The runs: random bytes where copied, zeros where skipped; the first copy run covers the
    // structures.
    let mut runs: Vec<(bool, u32)> = vec![(true, first_run)];
    let mut at = start + first_run;
    let mut copy = false;
    while at < size {
        let len = (1 + rng.below(300)).min(size - at);
        runs.push((copy, len));
        at += len;
        copy = !copy;
    }
    let mut data = Vec::new();
    let mut pos = start;
    for &(c, len) in &runs {
        data.extend_from_slice(&(len as u16).to_le_bytes());
        if c {
            for i in 0..len {
                // keep the structures; fill the rest
                let a = (pos + i) as usize;
                if pos + i >= cur {
                    flat[a] = rng.next() as u8;
                }
                data.push(flat[a]);
            }
        }
        pos += len;
    }

    let section_count = 1 + rng.below(4);
    let mut header = [0u32; 10];
    header[0] = size;
    header[1] = rng.next();
    header[2] = reloc_rva;
    header[3] = entries;
    header[4] = export_rva;
    header[5] = export_count;
    header[6] = export_base;
    header[7] = import_rva;
    header[8] = libraries;
    header[9] = section_count;
    let mut blob = Vec::new();
    for h in header {
        blob.extend_from_slice(&h.to_le_bytes());
    }
    let prots = [1u32, 2, 4, 8, 0x10, 0x20, 0x40, 0x80];
    for i in 0..section_count {
        let rva = if i == 0 { start } else { start + 0x100 * i };
        blob.extend_from_slice(&rva.to_le_bytes());
        blob.extend_from_slice(&(0x80 + rng.below(0x800)).to_le_bytes());
        blob.extend_from_slice(&prots[rng.below(8) as usize].to_le_bytes());
    }
    blob.extend_from_slice(&data);
    flat[..0x28].copy_from_slice(&blob[..0x28]);
    Built {
        blob,
        flat,
        size,
        libraries,
        export_count,
        export_base,
    }
}
