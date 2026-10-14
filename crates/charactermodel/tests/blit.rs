//! Bit-exact tests of the three 16-bpp texture blit kernels of the character component, run whole
//! against `WoW.exe`. They are leaves (no calls out) operating on source and destination pixel
//! buffers, so the test compares the whole destination buffer pixel for pixel after running the exe's
//! code and the transcription on identical inputs.
//!
//! Memory model: one contiguous scratch region in the emulator is mirrored into a local `Vec<u8>`.
//! The kernels address a flat byte space by absolute 32-bit address; for the Rust run every address
//! (and every stored base pointer) is translated by `-base`, a uniform shift that preserves the
//! exe's arithmetic, the `[arg0+4L]` indirection included, since the stored pointers are translated
//! too. Only the destination pixel buffer, the one thing a kernel mutates, is compared, so the
//! translated pointer-array slots never enter the result.

use benilla_re_charactermodel::blit::{blit_blend, blit_copy, blit_masked};
use benilla_re_difftest::{Emu, Wow};

const COPY: u64 = 0x477c80;
const MASKED: u64 = 0x477db0;
const BLEND: u64 = 0x477f20;

/// A 32-bit LCG byte stream (deterministic), used to fill the src/dst/mask regions.
struct Lcg(u32);
impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = (self.next_u32() >> 24) as u8;
        }
    }
}

/// One blit case: a write rect of `w`×`h` pixels with `mip` mip levels, source rows of stride
/// `2*x0` bytes, write coords `(wx,wy)`, read coords `(rx,ry)`. `seed` drives the buffer fill;
/// `mask_pat` is the byte written into every mask byte (cycled), giving us specific 2-bit fields.
#[derive(Clone, Copy)]
struct Case {
    w: u32,
    h: u32,
    mip: u32,
    x0: u32,
    wx: u32,
    wy: u32,
    rx: u32,
    ry: u32,
    seed: u32,
    /// `None` → the read slab's own deterministic byte pattern doubles as the mask (mask fields land
    /// in src bytes — fully deterministic, naturally exercises all 2-bit values across cases). `Some(b)`
    /// → fill the entire read slab with the constant `b` (force a specific 2-bit field everywhere).
    mask_fill: Option<u8>,
}

/// Region layout (all sizes generous so the downscaled mip levels stay in bounds). Both the WRITE
/// (dst) and READ (src) buffers get their own slab; each mip level points at the SAME base (offset 0
/// of its slab) — the binary's per-level coord/stride downscale keeps the accesses disjoint enough,
/// and pointing every level at base 0 exercises the `[arg+4L]` indirection without a huge region.
struct Layout {
    base: u64,
    write_buf: u64, // dst pixels (WRITE stride 0x200)
    read_buf: u64,  // src pixels (READ stride 2*x0) + trailing mask
    write_arr: u64, // arg1: per-level WRITE base ptr array
    read_arr: u64,  // arg0: per-level READ base ptr array
    arg2: u64,      // WRITE coords {x,y}
    arg3: u64,      // READ coords {x,y}
    arg4: u64,      // rect {w,h}
    arg5: u64,      // {x0, _, _, mipcount}
    write_len: usize,
    read_len: usize,
}

const WRITE_SLAB: usize = 0x40000; // 256 KiB — covers 0x200-stride dst for the rects we sweep
const READ_SLAB: usize = 0x40000;

fn alloc_layout(emu: &mut Emu) -> Layout {
    let base = emu.alloc(0); // current scratch cursor = our origin
    let write_buf = emu.alloc(WRITE_SLAB as u64);
    let read_buf = emu.alloc(READ_SLAB as u64);
    let write_arr = emu.alloc(0x100);
    let read_arr = emu.alloc(0x100);
    let arg2 = emu.alloc(0x10);
    let arg3 = emu.alloc(0x10);
    let arg4 = emu.alloc(0x10);
    let arg5 = emu.alloc(0x10);
    let top = emu.alloc(0); // end cursor
    Layout {
        base,
        write_buf,
        read_buf,
        write_arr,
        read_arr,
        arg2,
        arg3,
        arg4,
        arg5,
        write_len: (read_buf - write_buf) as usize, // write slab extent
        read_len: (top - read_buf) as usize,        // everything from read_buf up
    }
}

/// Populate the emulator memory for a case and return `(args, write_off, write_cmp_len)` — the 6
/// stack args and the dst-region offset/length to compare. Also returns the relative args + a local
/// mirror `Vec<u8>` of the whole region for the Rust run.
#[allow(clippy::type_complexity)]
fn setup_case(emu: &mut Emu, lo: &Layout, c: &Case) -> ([u32; 6], Vec<u8>, usize, usize) {
    // Fill the dst (write) and src (read) slabs with distinct deterministic patterns. The read slab
    // serves BOTH as source pixels and (in its trailing bytes, where the kernel's mask_base lands) as
    // the mask — every byte the binary reads is deterministic and identical in our local mirror.
    let mut wfill = vec![0u8; WRITE_SLAB];
    let mut rfill = vec![0u8; READ_SLAB];
    Lcg(c.seed ^ 0x0D57_u32).fill(&mut wfill);
    Lcg(c.seed).fill(&mut rfill);
    if let Some(b) = c.mask_fill {
        // Force a specific 2-bit field everywhere: the whole read slab becomes the constant `b`, so
        // every mask byte = `b` (and the src pixels are uniform too — still a valid exact diff).
        rfill.iter_mut().for_each(|x| *x = b);
    }

    emu.write(lo.write_buf, &wfill);
    emu.write(lo.read_buf, &rfill);

    // Per-level base ptr arrays: every level → base 0 of its slab (absolute emu addresses).
    for l in 0..c.mip {
        emu.write_u32(lo.write_arr + 4 * u64::from(l), lo.write_buf as u32);
        emu.write_u32(lo.read_arr + 4 * u64::from(l), lo.read_buf as u32);
    }
    // coords / rect / desc
    emu.write_u32(lo.arg2, c.wx);
    emu.write_u32(lo.arg2 + 4, c.wy);
    emu.write_u32(lo.arg3, c.rx);
    emu.write_u32(lo.arg3 + 4, c.ry);
    emu.write_u32(lo.arg4, c.w);
    emu.write_u32(lo.arg4 + 4, c.h);
    emu.write_u32(lo.arg5, c.x0);
    emu.write_u32(lo.arg5 + 0xc, c.mip);

    // Build the local mirror of the whole region [base, top) for the Rust run.
    let total = lo.read_buf as usize + lo.read_len - lo.base as usize;
    let mut mem = emu.read(lo.base, total);
    // Translate the stored base pointers to base-relative in the Rust mirror (the binary stores
    // absolute pointers; the Rust kernel runs in a `-base` shifted space).
    let off = |va: u64| (va - lo.base) as usize;
    for l in 0..c.mip {
        let wa = off(lo.write_arr + 4 * u64::from(l));
        let ra = off(lo.read_arr + 4 * u64::from(l));
        let wrel = (lo.write_buf - lo.base) as u32;
        let rrel = (lo.read_buf - lo.base) as u32;
        mem[wa..wa + 4].copy_from_slice(&wrel.to_le_bytes());
        mem[ra..ra + 4].copy_from_slice(&rrel.to_le_bytes());
    }

    // The base-relative args for the Rust kernel.
    let args_rel = [
        (lo.read_arr - lo.base) as u32,
        (lo.write_arr - lo.base) as u32,
        (lo.arg2 - lo.base) as u32,
        (lo.arg3 - lo.base) as u32,
        (lo.arg4 - lo.base) as u32,
        (lo.arg5 - lo.base) as u32,
    ];
    let write_off = (lo.write_buf - lo.base) as usize;
    (args_rel, mem, write_off, lo.write_len.min(WRITE_SLAB))
}

/// Run one case through both the binary (`fn_va`) and the matching Rust kernel; assert the whole
/// dst (write) buffer matches byte-for-byte.
fn run_case(
    emu: &mut Emu,
    lo: &Layout,
    fn_va: u64,
    rust: fn(&mut [u8], [u32; 6]),
    c: &Case,
    tag: &str,
) {
    let (args_rel, mut mem, write_off, cmp_len) = setup_case(emu, lo, c);

    // Rust run on the local mirror.
    rust(&mut mem, args_rel);
    let rust_dst = mem[write_off..write_off + cmp_len].to_vec();

    // Binary run: cdecl, 6 stack args (read_arr, write_arr, arg2, arg3, arg4, arg5) = (arg0..arg5).
    let stack = [
        lo.read_arr as u32,
        lo.write_arr as u32,
        lo.arg2 as u32,
        lo.arg3 as u32,
        lo.arg4 as u32,
        lo.arg5 as u32,
    ];
    emu.call(fn_va, 0, 0, &stack);
    let bin_dst = emu.read(lo.write_buf, cmp_len);

    if bin_dst != rust_dst {
        // Find first differing 16-bit pixel for a useful message.
        let mut first = None;
        for i in (0..cmp_len).step_by(2) {
            if bin_dst[i] != rust_dst[i] || bin_dst[i + 1] != rust_dst[i + 1] {
                first = Some(i / 2);
                break;
            }
        }
        panic!(
            "{tag}: dst mismatch w={} h={} mip={} x0={} w({},{}) r({},{}) — first diff at pixel {:?}: \
             bin={:02x?} rust={:02x?}",
            c.w,
            c.h,
            c.mip,
            c.x0,
            c.wx,
            c.wy,
            c.rx,
            c.ry,
            first,
            first.map(|p| [bin_dst[p * 2], bin_dst[p * 2 + 1]]),
            first.map(|p| [rust_dst[p * 2], rust_dst[p * 2 + 1]]),
        );
    }
}

/// A representative sweep of rect sizes / mip counts / coords used by every kernel.
fn cases() -> Vec<Case> {
    let mut v = Vec::new();
    let mut seed = 0x1234_5678u32;
    for &(w, h) in &[
        (1u32, 1u32),
        (8, 1),
        (1, 8),
        (8, 8),
        (16, 12),
        (32, 24),
        (7, 13),
        (64, 4),
        (4, 64),
        (100, 80),
    ] {
        for &mip in &[1u32, 2, 4] {
            for &(wx, wy, rx, ry) in &[(0u32, 0u32, 0u32, 0u32), (2, 1, 1, 3), (5, 7, 3, 2)] {
                // x0 must give a read stride wide enough for the rect: 2*x0 >= 2*(rx+w).
                let x0 = 0x100; // 256-pixel src rows (512-byte stride) — ample for our rects
                seed = seed.wrapping_add(0x9e37_79b9);
                v.push(Case {
                    w,
                    h,
                    mip,
                    x0,
                    wx,
                    wy,
                    rx,
                    ry,
                    seed,
                    mask_fill: None,
                });
            }
        }
    }
    // Explicit mask fills to pin every 2-bit field for the gate/blend mode switch: 0x00 (all skip),
    // 0xFF (all field=3 → opaque), 0x55 (all field=1 → blend m=1), 0xAA (all field=2 → blend m=2),
    // and 0xE4 = 0b11_10_01_00 (fields 0,1,2,3 by pixel position → exercises the whole switch).
    for &mf in &[0x00u8, 0xff, 0x55, 0xaa, 0xe4] {
        for &(w, h) in &[(8u32, 8u32), (16, 12), (7, 13), (100, 80)] {
            for &mip in &[1u32, 4] {
                seed = seed.wrapping_add(0x9e37_79b9);
                v.push(Case {
                    w,
                    h,
                    mip,
                    x0: 0x100,
                    wx: 1,
                    wy: 2,
                    rx: 3,
                    ry: 1,
                    seed,
                    mask_fill: Some(mf),
                });
            }
        }
    }
    v
}

#[test]
fn cm_fanout_f_blit_copy() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let lo = alloc_layout(&mut emu);
    let cs = cases();
    for c in &cs {
        run_case(&mut emu, &lo, COPY, blit_copy, c, "blit_copy");
    }
    eprintln!(
        "cm_fanout_f_blit_copy: {} cases bit-exact (0x477c80)",
        cs.len()
    );
}

#[test]
fn cm_fanout_f_blit_masked() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let lo = alloc_layout(&mut emu);
    // Sweep the base cases plus a few explicit mask patterns (forcing all-on / all-off / alternating
    // 2-bit fields) by varying mask_seed deterministically via the case set.
    let cs = cases();
    for c in &cs {
        run_case(&mut emu, &lo, MASKED, blit_masked, c, "blit_masked");
    }
    eprintln!(
        "cm_fanout_f_blit_masked: {} cases bit-exact (0x477db0)",
        cs.len()
    );
}

#[test]
fn cm_fanout_f_blit_blend() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let lo = alloc_layout(&mut emu);
    let cs = cases();
    for c in &cs {
        run_case(&mut emu, &lo, BLEND, blit_blend, c, "blit_blend");
    }
    eprintln!(
        "cm_fanout_f_blit_blend: {} cases bit-exact (0x477f20)",
        cs.len()
    );
}
