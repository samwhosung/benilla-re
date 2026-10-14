//! The three 16-bpp texture blit kernels of the character component's skin and region paint:
//! plain copy, masked copy and blend.
//!
//! Each is a leaf that walks a sub-rectangle across every mip level of a paged source and
//! destination pair and copies 16-bit pixels, gated and blended in three escalating ways:
//!
//! - `blit_copy` (`0x477c80`): a plain `mov word` copy, no gate.
//! - `blit_masked` (`0x477db0`): a 2-bit-per-pixel mask gate; the pixel is copied only when its
//!   2-bit mask field is non-zero.
//! - `blit_blend` (`0x477f20`): the 2-bit field decodes to a blend mode `m`: 0 skips, 3 copies
//!   opaque, and 1 or 2 alpha-blend source over destination in RGB565 at a fixed weight (1/3 and
//!   2/3, or 2/3 and 1/3).
//!
//! All three share one skeleton:
//!
//! - Six cdecl stack arguments (`ret 0x18`). `arg0` and `arg1` are per-level base-pointer arrays
//!   (one dword per mip level) for the read (source) and write (destination) buffers: the base for
//!   level L is `[arg0 + 4*L]` (read) and `[arg1 + 4*L]` (write). `arg2` is the write coordinate
//!   struct (`x` at 0, `y` at 4); `arg3` is the read coordinate struct (the same layout); `arg4` is
//!   the rectangle (`w` at 0, `h` at 4); `arg5` is the source descriptor (`x0` at 0, so the read
//!   row stride is `2*x0`; the mip count at `+0xc`).
//! - The write row stride is the constant `0x200` bytes; the read row stride is `2*arg5.x0`.
//! - Per mip level the write pointer is `[arg1+4L] + wy*0x200 + 2*wx` and the read pointer is
//!   `[arg0+4L] + ry*rstride + 2*rx`; it copies `h` rows of `w` pixels of 2 bytes. After the level
//!   every quantity (`w`, `h`, `wx`, `wy`, `rx`, `ry` and both strides) is halved: `w` and `h` as
//!   `max(x>>1, 1)`, the coordinates by signed `sar`, the strides by unsigned `shr`. The level
//!   pointer-array cursors advance by 4.
//! - The masked and blend kernels keep a running pixel index `i` that resets to 0 at the start of
//!   each level and increments once per pixel across all rows of that level. The mask base for
//!   the level is `readBase + h*rstride` (the bytes just past the source rectangle), and the 2-bit
//!   field is `(mask[i>>2] & GATE[i&3]) >> SHIFT[i&3]`.
//!
//! The static tables, read from the image at the cited addresses:
//!
//! - `GATE` at `0x838ae4` is `[0x03, 0x0c, 0x30, 0xc0]`, the 2-bit field selector for pixel
//!   position `i&3`.
//! - `SHIFT` at `0x838ae8` is `[0, 2, 4, 6]` (dwords), which brings the selected field to bit 0.
//! - `WEIGHT` at `0x838af8` is `[0, 0x5555, 0xaaaa, 0x10000]`, the source weight by mode `m`.
//! - The destination weight is the dword at `0x838b04 - 4*m`, which is `WEIGHT[3-m]`: mode 1 is
//!   1/3 source and 2/3 destination, mode 2 is 2/3 source and 1/3 destination.
//!
//! Each kernel here works on one flat byte address space `mem` (the emulator's memory), taking
//! 32-bit byte addresses as the exe does. The test compares the whole of `mem` byte for byte after
//! running the exe's code and this transcription on the same inputs.

/// `GATE` at `0x838ae4`: the 2-bit field selector for pixel position `i & 3`.
pub const GATE: [u8; 4] = [0x03, 0x0c, 0x30, 0xc0];
/// `SHIFT` at `0x838ae8` (dword array, low byte used): the shift that brings field `i&3` down to bit 0.
pub const SHIFT: [u8; 4] = [0, 2, 4, 6];
/// The dword array at `0x838af8`, `[0, 0x5555, 0xaaaa, 0x10000]`: `WEIGHT[m]` is the source weight;
/// the destination weight is `[0x838b04 - 4*m]`, which is `WEIGHT[3-m]` (m in {1, 2}).
pub const WEIGHT: [u32; 4] = [0, 0x5555, 0xaaaa, 0x10000];

#[inline]
fn rd16(mem: &[u8], addr: u32) -> u16 {
    let a = addr as usize;
    u16::from_le_bytes([mem[a], mem[a + 1]])
}
#[inline]
fn wr16(mem: &mut [u8], addr: u32, v: u16) {
    let a = addr as usize;
    let b = v.to_le_bytes();
    mem[a] = b[0];
    mem[a + 1] = b[1];
}
#[inline]
fn rd32(mem: &[u8], addr: u32) -> u32 {
    let a = addr as usize;
    u32::from_le_bytes([mem[a], mem[a + 1], mem[a + 2], mem[a + 3]])
}

/// The level-local geometry an iteration works on; halved (per the binary) at the end of each level.
#[derive(Clone, Copy)]
struct Geom {
    w: u32,
    h: u32,
    wstride: u32, // write row stride (starts at 0x200)
    rstride: u32, // read row stride (starts at 2*arg5.x0)
    wx: i32,
    wy: i32,
    rx: i32,
    ry: i32,
}

impl Geom {
    /// The per-level downscale: `w` and `h` clamp to at least 1, the coordinates shift arithmetic and
    /// the strides shift logical (the `shr` and `sar` mix at `0x477d38`, `0x477eac` and `0x4780e6`).
    fn halve(&mut self) {
        self.w = (self.w >> 1).max(1);
        self.h = (self.h >> 1).max(1);
        self.wx >>= 1; // sar
        self.wy >>= 1;
        self.rx >>= 1;
        self.ry >>= 1;
        self.wstride >>= 1; // shr (unsigned)
        self.rstride >>= 1;
    }
}

/// Reads the six cdecl args and loads the initial geometry from `mem`. `arg5.x0` (`[arg5]`) gives the
/// read stride `2*x0`; the mip count is `[arg5+0xc]`. Returns `(read_arr, write_arr, mip, geom)`.
fn setup(mem: &[u8], args: [u32; 6]) -> (u32, u32, u32, Geom) {
    let [read_arr, write_arr, arg2, arg3, arg4, arg5] = args;
    let geom = Geom {
        w: rd32(mem, arg4),                       // arg4[0]
        h: rd32(mem, arg4 + 4),                   // arg4[4]
        wstride: 0x200,                           // the write stride constant
        rstride: rd32(mem, arg5).wrapping_mul(2), // 2*arg5.x0  (`shl edi`)
        wx: rd32(mem, arg2) as i32,
        wy: rd32(mem, arg2 + 4) as i32,
        rx: rd32(mem, arg3) as i32,
        ry: rd32(mem, arg3 + 4) as i32,
    };
    let mip = rd32(mem, arg5 + 0xc);
    (read_arr, write_arr, mip, geom)
}

/// Per-level base pointers: write base `[arg1+4L]`, read base `[arg0+4L]`, and the resulting write and
/// read rectangle origin pointers, as the `lea` and `imul` chain computes them.
#[inline]
fn level_ptrs(mem: &[u8], read_arr: u32, write_arr: u32, level: u32, g: &Geom) -> (u32, u32, u32) {
    let read_base = rd32(mem, read_arr + 4 * level);
    let write_base = rd32(mem, write_arr + 4 * level);
    // write ptr = write_base + wy*0x200 + 2*wx ; read ptr = read_base + ry*rstride + 2*rx
    let wptr = write_base
        .wrapping_add((g.wy as u32).wrapping_mul(g.wstride))
        .wrapping_add((g.wx as u32).wrapping_mul(2));
    let rptr = read_base
        .wrapping_add((g.ry as u32).wrapping_mul(g.rstride))
        .wrapping_add((g.rx as u32).wrapping_mul(2));
    (read_base, wptr, rptr)
}

/// `0x477c80`: plain 16-bpp sub-rectangle copy across all mip levels (no gate).
pub fn blit_copy(mem: &mut [u8], args: [u32; 6]) {
    let (read_arr, write_arr, mip, mut g) = setup(mem, args);
    for level in 0..mip {
        let (_rbase, mut wptr, mut rptr) = level_ptrs(mem, read_arr, write_arr, level, &g);
        for _row in 0..g.h {
            let (mut w, mut r) = (wptr, rptr);
            for _ in 0..g.w {
                let px = rd16(mem, r);
                wr16(mem, w, px);
                w = w.wrapping_add(2);
                r = r.wrapping_add(2);
            }
            wptr = wptr.wrapping_add(g.wstride);
            rptr = rptr.wrapping_add(g.rstride);
        }
        g.halve();
    }
}

/// `0x477db0`: the same blit gated by a 2-bit-per-pixel mask: pixel `i` is copied only when
/// `mask[i>>2] & GATE[i&3]` is non-zero. `i` runs continuously across all rows of a level.
pub fn blit_masked(mem: &mut [u8], args: [u32; 6]) {
    let (read_arr, write_arr, mip, mut g) = setup(mem, args);
    for level in 0..mip {
        let (read_base, mut wptr, mut rptr) = level_ptrs(mem, read_arr, write_arr, level, &g);
        let mask_base = read_base.wrapping_add(g.h.wrapping_mul(g.rstride));
        let mut i: u32 = 0;
        for _row in 0..g.h {
            let (mut w, mut r) = (wptr, rptr);
            for _ in 0..g.w {
                let mask_byte = mem[mask_base.wrapping_add(i >> 2) as usize];
                if mask_byte & GATE[(i & 3) as usize] != 0 {
                    wr16(mem, w, rd16(mem, r));
                }
                w = w.wrapping_add(2);
                r = r.wrapping_add(2);
                i += 1;
            }
            wptr = wptr.wrapping_add(g.wstride);
            rptr = rptr.wrapping_add(g.rstride);
        }
        g.halve();
    }
}

/// The RGB565 fixed-weight blend for mode `m` in {1, 2}: per channel
/// `(src_ch * WEIGHT[m] + dst_ch * WEIGHT[3-m]) >> 0x10`, masked again and OR'd.
#[inline]
fn blend565(src: u16, dst: u16, m: u32) -> u16 {
    let sw = WEIGHT[m as usize]; // source weight
    let dw = WEIGHT[(3 - m) as usize]; // destination weight, [0x838b04 - 4*m]
    let src = u32::from(src);
    let dst = u32::from(dst);
    let g = (((src & 0x7e0) * sw).wrapping_add((dst & 0x7e0) * dw) >> 0x10) & 0x7e0;
    let b = (((src & 0x1f) * sw).wrapping_add((dst & 0x1f) * dw) >> 0x10) & 0x1f;
    let r = (((src & 0xf800) * sw).wrapping_add((dst & 0xf800) * dw) >> 0x10) & 0xf800;
    (g | b | r) as u16
}

/// `0x477f20`: the same mask decode feeding a per-pixel blend-mode switch. The 2-bit field is `m =
/// (mask[i>>2] & GATE[i&3]) >> SHIFT[i&3]`: `0` skips, `3` copies opaque, and `1` or `2` blend the
/// source pixel over the existing destination pixel in RGB565 at a fixed weight.
pub fn blit_blend(mem: &mut [u8], args: [u32; 6]) {
    let (read_arr, write_arr, mip, mut g) = setup(mem, args);
    for level in 0..mip {
        let (read_base, mut wptr, mut rptr) = level_ptrs(mem, read_arr, write_arr, level, &g);
        let mask_base = read_base.wrapping_add(g.h.wrapping_mul(g.rstride));
        let mut i: u32 = 0;
        for _row in 0..g.h {
            let (mut w, mut r) = (wptr, rptr);
            for _ in 0..g.w {
                let mask_byte = u32::from(mem[mask_base.wrapping_add(i >> 2) as usize]);
                let pos = (i & 3) as usize;
                let m = (mask_byte & u32::from(GATE[pos])) >> SHIFT[pos];
                match m {
                    0 => {}                          // skip
                    3 => wr16(mem, w, rd16(mem, r)), // opaque copy
                    1 | 2 => {
                        let blended = blend565(rd16(mem, r), rd16(mem, w), m);
                        wr16(mem, w, blended);
                    }
                    _ => {} // m can only be 0..=3 (a 2-bit field)
                }
                w = w.wrapping_add(2);
                r = r.wrapping_add(2);
                i += 1;
            }
            wptr = wptr.wrapping_add(g.wstride);
            rptr = rptr.wrapping_add(g.rstride);
        }
        g.halve();
    }
}
