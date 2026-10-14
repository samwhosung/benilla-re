//! Consequence tests (not a transcription comparison): what the exe's own code puts in the 256x256
//! composite when a source BLP is larger than the tile it fills. Each case runs the real load and blit
//! chain in the emulator: the section BLP loader `0x449840` (header parse, the device fit `0x448b60`,
//! the chain allocation `0x44af90`, the palettised decode `0x5a8430`) on a synthesized BLP, then the
//! base-skin helper `0x477070` or the overlay helper `0x4770f0` (descriptor fill `0x47b150`,
//! `0x44b430`, `0x44ae70`, trampoline `0x477460`, kernel `0x477c80` or `0x477f20`) into the scratch
//! target the compositor's initializer builds (`0x44af90(5, 256, 256)` into `[0xb422d8]`), with the
//! bounding-box table from its static initializer `0x475c50`. Only `SMemAlloc`, `SMemFree` and the
//! error reporter are stubbed.
//!
//! The observables are read back from emulator memory: the decoded source chain, the target chain,
//! and two sentinel words at the addresses the target's level-0 row-0 pixels spell when read as a
//! pointer.

use benilla_re_difftest::{Emu, Wow};

const LOAD_BLP: u64 = 0x449840;
const CHAIN_ALLOC: u64 = 0x44af90;
const BBOX_INIT: u64 = 0x475c50;
const BASE_BLIT: u64 = 0x477070;
const OVERLAY_BLIT: u64 = 0x4770f0;
const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_FREE: u64 = 0x646430;
const ERROR_REPORT: u64 = 0x64e850;
const GX_DEVICE: u64 = 0xc0ed38;
const SCRATCH_TARGET: u64 = 0xb422d8;
const BBOX: u64 = 0xb42450;

/// `SMemAlloc` arena, clear of the harness's own scratch cursor (which starts at `0x7200_0000`).
const ARENA: (u64, u64) = (0x7240_0000, 0x72f0_0000);
/// Where the target's row-0 pixels are made to point: mapped scratch past the arena, untouched by
/// any allocation, so a store through them lands somewhere observable instead of faulting.
const P9: u32 = 0x72f8_0000;
const P10: u32 = 0x72fc_0000;
const SENTINEL: u16 = 0x5a5a;

/// The 565 word a palette entry decodes to when it is an exact 565 point (`R8 = r5<<3`, `G8 =
/// g6<<2`, `B8 = b5<<3`): no rounding and no diffused error, so the decode is the identity on it.
fn pal_entry(v: u16) -> [u8; 4] {
    let (r5, g6, b5) = ((v >> 11) as u8, ((v >> 5) & 63) as u8, (v & 31) as u8);
    [b5 << 3, g6 << 2, r5 << 3, 0xff]
}

const WHITE: u8 = 255; // palette index of 0xFFFF
const BLACK: u8 = 254; // 0x0000
const HI9: u8 = 253; // high half of P9
const HI10: u8 = 252; // high half of P10

/// A palettised BLP2 (compression 1) with a full mip chain: `index(L,x,y)` and `alpha(L,x,y)`.
fn synth_blp(
    w: u32,
    h: u32,
    alpha_depth: u8,
    index: impl Fn(u32, u32, u32) -> u8,
    alpha: impl Fn(u32, u32, u32) -> u8,
) -> Vec<u8> {
    let levels = 32 - w.max(h).leading_zeros();
    let mut hdr = vec![0u8; 0x494];
    hdr[0..4].copy_from_slice(b"BLP2");
    hdr[4..8].copy_from_slice(&1u32.to_le_bytes());
    hdr[8] = 1; // palettised
    hdr[9] = alpha_depth;
    hdr[0xa] = 0;
    hdr[0xb] = 1; // has mips
    hdr[0xc..0x10].copy_from_slice(&w.to_le_bytes());
    hdr[0x10..0x14].copy_from_slice(&h.to_le_bytes());
    for i in 0..256u32 {
        let v: u16 = match i as u8 {
            WHITE => 0xffff,
            BLACK => 0x0000,
            HI9 => (P9 >> 16) as u16,
            HI10 => (P10 >> 16) as u16,
            // distinct, exact-565 colours for the pattern entries
            k => (u16::from(k) * 0x0101) ^ 0x2a4c,
        };
        let o = 0x94 + 4 * i as usize;
        hdr[o..o + 4].copy_from_slice(&pal_entry(v));
    }
    let mut data = Vec::new();
    for l in 0..levels {
        let (lw, lh) = ((w >> l).max(1), (h >> l).max(1));
        let off = 0x494 + data.len() as u32;
        for y in 0..lh {
            for x in 0..lw {
                data.push(index(l, x, y));
            }
        }
        if alpha_depth == 8 {
            for y in 0..lh {
                for x in 0..lw {
                    data.push(alpha(l, x, y));
                }
            }
        }
        let size = 0x494 + data.len() as u32 - off;
        hdr[0x14 + 4 * l as usize..0x18 + 4 * l as usize].copy_from_slice(&off.to_le_bytes());
        hdr[0x54 + 4 * l as usize..0x58 + 4 * l as usize].copy_from_slice(&size.to_le_bytes());
    }
    hdr.extend_from_slice(&data);
    hdr
}

/// The pattern entries: level-dependent so a level-matched read and a same-index read differ.
fn pattern(l: u32, x: u32, y: u32) -> u8 {
    ((x * 7 + y * 13 + l * 31) % 250) as u8
}

struct Env<'a> {
    emu: Emu<'a>,
    target: u32,
}

fn env(wow: &Wow, caps_max_texture: u32) -> Env<'_> {
    let mut emu = Emu::new(wow);
    emu.stub_alloc_sized(SMEM_ALLOC, ARENA.0, ARENA.1, 0, 4);
    emu.stub(SMEM_FREE, 1, 4);
    emu.stub(ERROR_REPORT, 0, 1);
    // gx device: caps = device + 0x23c (0x58a230); caps + 0x60 = the max texture size 0x448b60 fits to.
    let dev = emu.alloc(0x400);
    emu.write_u32(dev + 0x23c + 0x60, caps_max_texture);
    emu.write_u32(GX_DEVICE, dev as u32);
    emu.call(BBOX_INIT, 0, 0, &[]);
    // The compositor's own scratch target: 0x476020 calls 0x44af90(ecx=5, edx=0x100, push 0x100).
    let target = emu.call(CHAIN_ALLOC, 5, 0x100, &[0x100]);
    emu.write_u32(SCRATCH_TARGET, target);
    Env { emu, target }
}

impl Env<'_> {
    fn chain(&self, base: u32, n: u32) -> Vec<u32> {
        (0..n)
            .map(|i| self.emu.read_u32(u64::from(base) + 4 * u64::from(i)))
            .collect()
    }
    fn px(&self, level_base: u32, pitch_px: u32, x: u32, y: u32) -> u16 {
        let a = u64::from(level_base) + 2 * u64::from(y * pitch_px + x);
        let b = self.emu.read(a, 2);
        u16::from_le_bytes([b[0], b[1]])
    }
    fn word(&self, va: u32) -> u16 {
        let b = self.emu.read(u64::from(va), 2);
        u16::from_le_bytes([b[0], b[1]])
    }
    fn set_word(&mut self, va: u32, v: u16) {
        self.emu.write(u64::from(va), &v.to_le_bytes());
    }
    /// Run `0x449840` on `blp` as a section texture requested in gx format `fmt`; returns the
    /// gxImage (`+0x120` chain, `+0x144`/`+0x148` dims, `+0x114` dropped levels, `+0x11c` selector).
    fn load(&mut self, blp: &[u8], fmt: u32) -> u64 {
        let file = self.emu.alloc(blp.len() as u64);
        self.emu.write(file, blp);
        let req = self.emu.alloc(0x40);
        self.emu.write_u32(req + 4, file as u32);
        let inner = self.emu.alloc(0x160);
        self.emu.write_u32(inner + 0x138, req as u32);
        self.emu.write_u32(inner + 0x118, fmt);
        let ok = self.emu.call(LOAD_BLP, inner as u32, 0, &[]);
        assert_eq!(ok, 1, "0x449840 accepted the synthesized BLP");
        inner
    }
    /// A section wrapper over `inner` with its descriptor unfilled, in cell 0 of a fresh component.
    fn component(&mut self, inner: u64) -> (u64, u64) {
        let wrapper = self.emu.alloc(0xd0);
        self.emu.write_u32(wrapper + 0x18, inner as u32);
        let cc = self.emu.alloc(0x4e4);
        self.emu.write_u32(cc + 0x188, wrapper as u32);
        (cc, wrapper)
    }
    fn bbox(&self, g: u32) -> [u32; 4] {
        let b = BBOX + 0x10 * u64::from(g);
        [0, 4, 8, 12].map(|o| self.emu.read_u32(b + o))
    }
    /// Fill every level of the 9-level target with `SENTINEL`.
    fn clear_target(&mut self) {
        let t = self.chain(self.target, 9);
        for (l, &base) in t.iter().enumerate() {
            let n = (256usize >> l).pow(2);
            let bytes: Vec<u8> = (0..n).flat_map(|_| SENTINEL.to_le_bytes()).collect();
            self.emu.write(u64::from(base), &bytes);
        }
    }
}

/// The target is the 9-level chain `0x5a4af0` lays out: a 9-entry pointer table, then level 0 —
/// so `table[9]`/`table[10]` ARE level 0's row-0 pixels (0,1) and (2,3).
fn assert_target_layout(e: &Env) {
    let t = e.chain(e.target, 9);
    assert_eq!(
        t[0],
        e.target + 36,
        "level 0 starts right after the 9-entry table"
    );
    for l in 1..9u32 {
        let prev = 256u32 >> (l - 1);
        assert_eq!(
            t[l as usize] - t[l as usize - 1],
            prev * prev * 2,
            "level {l} offset"
        );
    }
}

#[test]
fn cm_oversized_base_skin_1024_is_a_crop_and_writes_past_the_table() {
    let Some(wow) = Wow::load() else {
        return;
    };
    // A realistic device: max texture size 2048 (every GPU since 2002 reports >= 2048).
    let mut e = env(&wow, 2048);
    assert_target_layout(&e);
    let blp = synth_blp(
        1024,
        1024,
        0,
        |l, x, y| match (l, x, y) {
            (0, 0, 0) | (0, 2, 0) => BLACK,
            (0, 1, 0) => HI9,
            (0, 3, 0) => HI10,
            _ => pattern(l, x, y),
        },
        |_, _, _| 0,
    );
    let inner = e.load(&blp, 5); // the base skin's section wrapper is kind 5
    let rd = |o: u64| e.emu.read_u32(inner + o);
    assert_eq!((rd(0x144), rd(0x148)), (1024, 1024), "not reduced to fit");
    assert_eq!(rd(0x114), 0, "0x448b60 dropped no level at caps 2048");
    assert_eq!(rd(0x11c), 0, "alphaDepth 0 -> REPLACE selector");
    let src = e.chain(rd(0x120), 11);
    // Preconditions: the magic row-0 pixels decoded exactly.
    assert_eq!(
        [0, 1, 2, 3].map(|x| e.px(src[0], 1024, x, 0)),
        [0, (P9 >> 16) as u16, 0, (P10 >> 16) as u16]
    );

    e.clear_target();
    e.set_word(P9, SENTINEL);
    e.set_word(P10, SENTINEL);
    let (cc, wrapper) = e.component(inner);
    e.emu.call(BASE_BLIT, cc as u32, 0, &[0]); // group 0: bbox {0,0,128,64}
    assert_eq!(
        e.emu.read_u32(wrapper + 0x28),
        11,
        "mip count from the source's own dims"
    );
    assert!(e.emu.calls().iter().all(|c| c.addr != ERROR_REPORT));

    let [bx, by, bw, bh] = e.bbox(0);
    assert_eq!([bx, by, bw, bh], [0, 0, 128, 64]);
    let dst = e.chain(e.target, 9);
    for l in 0..9u32 {
        let (w, h) = ((bw >> l).max(1), (bh >> l).max(1));
        let (x0, y0) = (bx >> l, by >> l);
        for y in 0..h {
            for x in 0..w {
                let got = e.px(dst[l as usize], 256 >> l, x0 + x, y0 + y);
                let want = e.px(src[l as usize], 1024 >> l, x0 + x, y0 + y);
                assert_eq!(got, want, "target L{l} ({x},{y}) = source L{l} same coords");
            }
        }
    }
    // The level a 256² target would need (source L2) is NOT what lands at target L0.
    let differs =
        (0..64u32).any(|y| (0..128u32).any(|x| e.px(dst[0], 256, x, y) != e.px(src[2], 256, x, y)));
    assert!(differs, "target L0 is not the level-matched (L2) image");
    // Levels 9 and 10 store through table[9]/table[10] = the pixel pairs written at level 0.
    assert_ne!(e.px(src[9], 2, 0, 0), SENTINEL);
    assert_ne!(e.px(src[10], 1, 0, 0), SENTINEL);
    assert_eq!(
        e.word(P9),
        e.px(src[9], 2, 0, 0),
        "L9 store at the address pixels (0,1) spell"
    );
    assert_eq!(
        e.word(P10),
        e.px(src[10], 1, 0, 0),
        "L10 store at pixels (2,3)"
    );
}

#[test]
fn cm_oversized_base_skin_fits_only_under_a_256_device_cap() {
    let Some(wow) = Wow::load() else {
        return;
    };
    // The one reduction on the path: 0x448b60 halves until both sides fit max(caps, 256).
    let mut e = env(&wow, 256);
    // File level 2 carries the row-0 pixels that spell P9, so a tenth level WOULD store there.
    let blp = synth_blp(
        1024,
        1024,
        0,
        |l, x, y| match (l, x, y) {
            (2, 0, 0) => BLACK,
            (2, 1, 0) => HI9,
            _ => pattern(l, x, y),
        },
        |_, _, _| 0,
    );
    let inner = e.load(&blp, 5);
    let rd = |o: u64| e.emu.read_u32(inner + o);
    assert_eq!((rd(0x144), rd(0x148), rd(0x114)), (256, 256, 2));
    let src = e.chain(rd(0x120), 9);
    e.clear_target();
    e.set_word(P9, SENTINEL);
    let (cc, wrapper) = e.component(inner);
    e.emu.call(BASE_BLIT, cc as u32, 0, &[0]);
    assert_eq!(e.emu.read_u32(wrapper + 0x28), 9);
    let dst = e.chain(e.target, 9);
    assert_eq!(
        [e.px(dst[0], 256, 0, 0), e.px(dst[0], 256, 1, 0)],
        [0, (P9 >> 16) as u16]
    );
    // Chain level 0 is the file's level 2; the target gets the whole image at 256².
    for y in 0..64u32 {
        for x in 0..128u32 {
            assert_eq!(e.px(dst[0], 256, x, y), e.px(src[0], 256, x, y));
            if y == 0 && x < 2 {
                continue;
            }
            let want = pattern(2, x, y);
            let got_idx =
                (0..250u8).find(|&k| e.px(src[0], 256, x, y) == ((u16::from(k) * 0x0101) ^ 0x2a4c));
            assert_eq!(got_idx, Some(want), "chain L0 ({x},{y}) is file level 2");
        }
    }
    assert_eq!(
        e.word(P9),
        SENTINEL,
        "a 9-level source stays inside the table"
    );
}

#[test]
fn cm_oversized_overlay_512x256_crops_and_reads_coverage_from_colour_rows() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = env(&wow, 2048);
    // TorsoUpper-shaped overlay: 512x256 where the tile is 128x64 (group 3). Colour rows at and
    // below the tile height of each level are white; the TRUE alpha is 0 everywhere except the
    // 1x1 level 9 — a correct reader would draw nothing at levels 0..8.
    let tile_h = |l: u32| (64u32 >> l).max(1);
    let blp = synth_blp(
        512,
        256,
        8,
        |l, x, y| {
            if y >= tile_h(l) {
                WHITE
            } else {
                pattern(l, x, y)
            }
        },
        |l, _, _| if l == 9 { 255 } else { 0 },
    );
    let inner = e.load(&blp, 9); // overlays are kind 9
    let rd = |o: u64| e.emu.read_u32(inner + o);
    assert_eq!(
        (rd(0x144), rd(0x148), rd(0x114), rd(0x11c)),
        (512, 256, 0, 8)
    );
    let src = e.chain(rd(0x120), 10);
    // Preconditions: level 0's true coverage plane (after its 512x256 colour plane) is all 0, and
    // the colour rows the kernel will read as coverage (row 64 on) decoded to 0xFFFF.
    let cov0 = e.emu.read(u64::from(src[0]) + 512 * 256 * 2, 512 * 256 / 4);
    assert!(cov0.iter().all(|&b| b == 0), "true alpha is 0 at level 0");
    assert!((0..512u32).all(|x| e.px(src[0], 512, x, 64) == 0xffff));
    assert_ne!(e.px(src[9], 1, 0, 0), SENTINEL);
    e.clear_target();
    // Make the target's row-0 pixels (0,1) spell P9, as a prior group's blit would leave them.
    let dst = e.chain(e.target, 9);
    e.set_word(dst[0], 0);
    e.set_word(dst[0] + 2, (P9 >> 16) as u16);
    e.set_word(P9, SENTINEL);
    let (cc, wrapper) = e.component(inner);
    e.emu.call(OVERLAY_BLIT, cc as u32, 0, &[3, wrapper as u32]);
    assert_eq!(e.emu.read_u32(wrapper + 0x28), 10);
    let [bx, by, bw, bh] = e.bbox(3);
    assert_eq!([bx, by, bw, bh], [128, 0, 128, 64]);
    for l in 0..9u32 {
        let (w, h) = ((bw >> l).max(1), (bh >> l).max(1));
        let (x0, y0) = (bx >> l, by >> l);
        // The coverage base the kernel uses: srcMip + h * srcPitch, inside the colour plane
        // (white rows -> coverage 3 -> copy) until the colour plane is shorter than that.
        let colour_bytes = (512u32 >> l).max(1) * (256u32 >> l).max(1) * 2;
        let cov_off = h * (1024u32 >> l);
        let copied = cov_off < colour_bytes;
        for y in 0..h {
            for x in 0..w {
                let got = e.px(dst[l as usize], 256 >> l, x0 + x, y0 + y);
                let want = if copied {
                    e.px(src[l as usize], 512 >> l, x, y)
                } else {
                    SENTINEL
                };
                assert_eq!(got, want, "L{l} ({x},{y}) copied={copied}");
            }
        }
        if l == 0 {
            assert!(
                copied,
                "L0 draws the top-left 128x64 of a source whose alpha is 0"
            );
        }
    }
    // Level 9 (1x1, true alpha 255 -> coverage 3) stores through table[9].
    assert_eq!(
        e.word(P9),
        e.px(src[9], 1, 0, 0),
        "L9 store at the address pixels (0,1) spell"
    );
}

#[test]
fn cm_oversized_control_stock_128x64_overlay_reads_its_own_alpha() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = env(&wow, 2048);
    // The exact-size control: 128x64, alpha 0 everywhere -> nothing drawn, nothing past the table.
    let blp = synth_blp(128, 64, 8, pattern, |_, _, _| 0);
    let inner = e.load(&blp, 9);
    e.clear_target();
    let dst = e.chain(e.target, 9);
    e.set_word(dst[0], 0);
    e.set_word(dst[0] + 2, (P9 >> 16) as u16);
    e.set_word(P9, SENTINEL);
    let (cc, wrapper) = e.component(inner);
    e.emu.call(OVERLAY_BLIT, cc as u32, 0, &[3, wrapper as u32]);
    assert_eq!(e.emu.read_u32(wrapper + 0x28), 8);
    for l in 0..8u32 {
        for y in 0..(64u32 >> l).max(1) {
            for x in 0..(128u32 >> l).max(1) {
                let got = e.px(dst[l as usize], 256 >> l, (128 >> l) + x, y);
                assert_eq!(got, SENTINEL, "L{l} ({x},{y}) untouched under alpha 0");
            }
        }
    }
    assert_eq!(e.word(P9), SENTINEL);
}
