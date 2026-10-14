//! The index strips, each run whole: `CChunkLiquid::BuildStrip` (`0x68d9b0`) and
//! `FillLowDetailIndices` (`0x6bd9b0`) with the buffer lock and unlock (`0x58a080`, `0x58a0a0`)
//! stubbed to hand out the test's buffer, and `BuildLiquidCellStrip` (`0x6b61a0`).

use benilla_re_difftest::{Emu, RegisterX86, Wow};

#[test]
fn liquid_strip_indices() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let buf = emu.alloc(0x800);
    emu.stub(0x58a080, buf as u32, 0); // the index buffer lock hands out the test's buffer
    emu.stub(0x58a0a0, 0, 0); // the unlock does nothing
    let this = emu.alloc(0x40);
    let flags = emu.alloc(0x40);
    emu.write_u32(this + 0x10, flags as u32);
    let mut s: u32 = 0xfeed_f00d;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    let mut checked = 0u32;
    for case in 0..300u32 {
        let mut cells = [0u8; 64];
        for c in cells.iter_mut() {
            *c = (next() >> 13) as u8;
        }
        let ty = (case % 16) as u8;
        for (i, &c) in cells.iter().enumerate() {
            emu.write(flags + i as u64, &[c]);
        }
        let ret = emu.call(0x68d9b0, this as u32, 0, &[u32::from(ty)]);
        let ours = benilla_re_terrain::liquid_strip_indices(&cells, ty);
        assert_eq!(ret as usize, ours.len(), "liquid_strip count (ty={ty})");
        let got: Vec<u16> = (0..ours.len())
            .map(|i| emu.read_u32(buf + i as u64 * 2) as u16)
            .collect();
        assert_eq!(got, ours, "liquid_strip emission (ty={ty})");
        checked += 1;
    }
    eprintln!("liquid_strip_indices: {checked} grids bit-exact vs WoW.exe @ 0x68d9b0");
}

#[test]
fn hole_strip_indices() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let obj = emu.alloc(0x140);
    let cells = emu.alloc(0x100);
    let buf = emu.alloc(0x1000);
    let count = emu.alloc(8);
    let mut s: u32 = 0x0bad_beef;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    let mut checked = 0u32;
    for case in 0..200u32 {
        let rows = 1 + (next() % 8) as i32;
        let cols = 1 + (next() % 8) as i32;
        let stride = (9 + (next() % 9)) as u16;
        let base = (next() % 200) as i32;
        let include = case % 2 == 1;
        let n = (rows * cols) as usize;
        let mut cmap = vec![0u8; n];
        for c in cmap.iter_mut() {
            *c = (next() >> 9) as u8;
        }
        emu.write_u32(obj + 0x11c, cells as u32);
        emu.write_u32(obj + 0xf8, u32::from(stride));
        emu.write_u32(obj + 0x100, cols as u32);
        emu.write_u32(obj + 0x104, rows as u32);
        emu.write_u32(0xce269c, u32::from(include));
        for (i, &c) in cmap.iter().enumerate() {
            emu.write(cells + i as u64, &[c]);
        }
        emu.write_u32(count, 7); // a count already there: the function adds to it
        emu.call(
            0x6b61a0,
            0,
            0,
            &[obj as u32, buf as u32, base as u32, count as u32],
        );
        let ours = benilla_re_terrain::hole_strip_indices(&cmap, stride, cols, rows, base, include);
        assert_eq!(
            emu.read_u32(count),
            7 + ours.len() as u32,
            "hole_strip count (rows={rows} cols={cols})"
        );
        let got: Vec<u16> = (0..ours.len())
            .map(|i| emu.read_u32(buf + i as u64 * 2) as u16)
            .collect();
        assert_eq!(got, ours, "hole_strip emission");
        checked += 1;
    }
    eprintln!("hole_strip_indices: {checked} maps bit-exact vs WoW.exe @ 0x6b61a0");
}

#[test]
fn wdl_strip_indices() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let buf = emu.alloc(16 * 16 * 12 * 2 + 0x10);
    emu.stub(0x58a080, buf as u32, 0); // the lock hands out the test's buffer
    emu.stub(0x58a0a0, 0, 0); // unlock
    emu.stub(0x58a030, 0, 0); // release
    emu.call(0x6bd9b0, 0, 0, &[]);
    let ours = benilla_re_terrain::wdl_strip_indices();
    let got: Vec<u16> = (0..ours.len())
        .map(|i| emu.read_u32(buf + i as u64 * 2) as u16)
        .collect();
    assert_eq!(got, ours.to_vec(), "wdl pattern");
    eprintln!("wdl_strip_indices: fixed 16x16x12 pattern bit-exact vs WoW.exe @ 0x6bd9b0");
}
