//! `BuildCursorPicture` (`0x523840`, `systems/ui/cursor.md#dragging-picture`) run from the exe.
//!
//! The function reads a 64 by 64 texture through `0x44ae00` and `0x44b3b0`, filters it to 32 by 32
//! at `0xbe1c28`, frees the texture (`0x41aed0`) and calls `0x523790`. The test stubs all four, runs
//! the filter over a synthesized source, and compares the 1024 output dwords exactly with
//! `build_cursor_picture`. The arithmetic is integer, so every dword must match.
#![allow(clippy::unreadable_literal)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::cursor_picture::build_cursor_picture as downsample;

const FUN_523840: u64 = 0x523840;

const FUN_44AE00: u64 = 0x44ae00; // the texture's pixels: returns a pointer to the buffer pointer

const FUN_44B3B0: u64 = 0x44b3b0; // the texture's size: writes width and height (ret 0x8)

const FUN_41AED0: u64 = 0x41aed0; // frees the texture

const FUN_523790: u64 = 0x523790; // called last

const DAT_DIRTY: u64 = 0x00be2c60; // the texture handle; nonzero runs the filter

const DAT_PRESENT: u64 = 0x00be2c48; // the picture state; zero frees the texture and returns

const DAT_DEST: u64 = 0x00be1c28; // 32×32 output mip base

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

#[test]
fn geo_523840() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // 64×64 ARGB source buffer + the lock indirection: FUN_0044ae00 returns a ptr whose [0] is the
    // source buffer VA (the binary does `local_8 = *piVar2`).
    let src = emu.alloc(64 * 64 * 4);
    let holder = emu.alloc(4);
    emu.write_u32(holder, src as u32);
    emu.stub(FUN_44AE00, holder as u32, 0);

    // dims: write w=0x40,h=0x40 through the out-pointer (arg0); the binary requires both == 0x40.
    let dims = emu.alloc(8);
    emu.write_u32(dims, 0x40);
    emu.write_u32(dims + 4, 0x40);
    emu.stub_write_through(FUN_44B3B0, 0, dims, 8, 1, 2);
    emu.stub(FUN_41AED0, 0, 0);
    emu.stub(FUN_523790, 0, 0);

    let run = |emu: &mut Emu, pixels: &[u32; 64 * 64]| -> Vec<u32> {
        for (i, p) in pixels.iter().enumerate() {
            emu.write_u32(src + 4 * i as u64, *p);
        }
        // poison the dest so a non-write is visible (never a silent match).
        for i in 0..(32 * 32u64) {
            emu.write_u32(DAT_DEST + 4 * i, 0xa5a5_a5a5);
        }
        emu.write_u32(DAT_DIRTY, 1);
        emu.write_u32(DAT_PRESENT, 1);
        emu.clear_calls();
        emu.call(FUN_523840, 0, 0, &[]);
        (0..(32 * 32u64))
            .map(|i| emu.read_u32(DAT_DEST + 4 * i))
            .collect()
    };

    // Build the case grid: a handful of structured tiles (solid, gradients, alpha probes, rounding-
    // bias straddles) then fuzz.
    let mut tiles: Vec<[u32; 64 * 64]> = Vec::new();
    // solid mid-grey, solid white, solid black, solid red/green/blue.
    for &v in &[
        0xff808080u32,
        0xffffffff,
        0xff000000,
        0xffff0000,
        0xff00ff00,
        0xff0000ff,
        0x00000000,
        0x80402010,
    ] {
        tiles.push([v; 64 * 64]);
    }
    // per-channel gradients (exercise the >>2 round biases at every residue).
    let mut grad = [0u32; 64 * 64];
    for (i, px) in grad.iter_mut().enumerate() {
        let r = (i & 0xff) as u32;
        let g = ((i * 3) & 0xff) as u32;
        let b = ((i * 7) & 0xff) as u32;
        let a = ((i * 5) & 0xff) as u32;
        *px = (a << 24) | (r << 16) | (g << 8) | b;
    }
    tiles.push(grad);
    // a tile whose quads sum to just-below / just-above each rounding step (values 1,2,3 per channel
    // so the +bias and >>2 truncation matter).
    let mut steps = [0u32; 64 * 64];
    for (i, px) in steps.iter_mut().enumerate() {
        let v = (i % 5) as u32; // 0..4 per channel -> straddles the /4 rounding
        *px = (0xffu32 << 24) | (v << 16) | (v << 8) | v;
    }
    tiles.push(steps);

    let mut s = 0x0052_3840u32;
    for _ in 0..24 {
        let mut t = [0u32; 64 * 64];
        for px in t.iter_mut() {
            *px = next(&mut s);
        }
        tiles.push(t);
    }

    let mut checked = 0u32;
    for (ti, tile) in tiles.iter().enumerate() {
        let bin = run(&mut emu, tile);
        let mine = downsample(tile);
        for (i, (&b, &m)) in bin.iter().zip(mine.iter()).enumerate() {
            assert_eq!(
                b, m,
                "geo_523840 tile {ti} texel {i} (row {},col {}): binary {b:#010x} vs rust {m:#010x}",
                i / 32,
                i % 32
            );
        }
        checked += 1;
    }
    eprintln!("geo_523840: {checked} tiles × 1024 texels bit-exact");
    assert!(checked > 0);
}
