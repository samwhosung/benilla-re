//! `TextureDecodeTga` (`0x4491f0`) against the exe (`src/textures.rs`), as two slices: the opaque
//! flag `0x449237` to `0x449244` over every descriptor byte, and the pixel copy `0x4492c0` to
//! `0x449300` over random pixels and counts 1 to 64 (the loop runs at least once; a zero count
//! skips it at `0x4492b8`, outside the slice).

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_gx::textures::{tga_copy_pixels, tga_opaque};

mod common;
use common::next;

/// `0x449237` to `0x449244`: the descriptor at `[ebp-0x37]`, the flag stored through `eax`.
#[test]
fn tga_opaque_flag_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x80);
    let out = emu.alloc(0x10);
    let ebp = frame + 0x40;
    for d in 0u8..=255 {
        emu.write(ebp - 0x37, &[d]);
        emu.write_u32(out, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::EAX, out);
        emu.run(0x449237, 0x449244);
        assert_eq!(emu.read_u32(out), tga_opaque(d), "descriptor {d:#04x}");
    }
}

/// `0x4492c0` to `0x449300`: `edi` the decoded pixels, `eax` level 0 of the chain, `esi` the count.
#[test]
fn tga_pixel_copy_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let src = emu.alloc(0x200);
    let dst = emu.alloc(0x200);
    let frame = emu.alloc(0x40);
    let mut s = 0x7ea5_0001u32;
    for count in 1..=64usize {
        let bytes: Vec<u8> = (0..4 * count).map(|_| (next(&mut s) >> 24) as u8).collect();
        emu.write(src, &bytes);
        emu.write(dst, &[0xcc; 0x110]);
        emu.set_reg(RegisterX86::EDI, src);
        emu.set_reg(RegisterX86::EAX, dst);
        emu.set_reg(RegisterX86::ESI, count as u64);
        emu.set_reg(RegisterX86::EBP, frame + 0x20);
        emu.run(0x4492c0, 0x449300);
        assert_eq!(
            emu.read(dst, 4 * count),
            tga_copy_pixels(&bytes, count),
            "count {count}"
        );
        assert_eq!(
            emu.read(dst + 4 * count as u64, 4),
            vec![0xcc; 4],
            "past the end, count {count}"
        );
    }
}

/// `0x4493a0`'s copy of a mip file's pixels, `0x4494b6` to `0x4494f6`: `esi` the decoded pixels,
/// `edi` the chain level, `eax` the count.
#[test]
fn tga_mip_copy_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let src = emu.alloc(0x200);
    let dst = emu.alloc(0x200);
    let mut s = 0x3c0b_aa17u32;
    for count in 1..=64usize {
        let bytes: Vec<u8> = (0..4 * count).map(|_| (next(&mut s) >> 24) as u8).collect();
        emu.write(src, &bytes);
        emu.write(dst, &[0xcc; 0x110]);
        emu.set_reg(RegisterX86::ESI, src);
        emu.set_reg(RegisterX86::EDI, dst);
        emu.set_reg(RegisterX86::EAX, count as u64);
        emu.run(0x4494b6, 0x4494f6);
        assert_eq!(
            emu.read(dst, 4 * count),
            tga_copy_pixels(&bytes, count),
            "count {count}"
        );
        assert_eq!(
            emu.read(dst + 4 * count as u64, 4),
            vec![0xcc; 4],
            "past the end, count {count}"
        );
    }
}
