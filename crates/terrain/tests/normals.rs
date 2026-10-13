//! The chunk normals and the area origin against `0x6aff10` and `0x6c2720`.

#![allow(clippy::needless_range_loop)]

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_terrain::normals::{adt_area_origin, mcnr_normals};
use common::assert_f32;

struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
}

/// `CMapChunk::LoadNormals` (`0x6aff10`, ecx = the chunk, the bytes at `[chunk+0xf18]`): the 435
/// floats at `chunk+0x170`, over every signed byte and random buffers.
#[test]
fn mcnr_normals_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let chunk = emu.alloc(0x1000);
    let src = emu.alloc(0x200);
    emu.write_u32(chunk + 0xf18, src as u32);

    let mut rng = Lcg(0x6d63_6e72);
    let mut checked = 0u32;
    // every signed byte, the extremes, then random buffers
    for case in 0..200u32 {
        let mut s = [0i8; 435];
        if case == 0 {
            for (k, b) in s.iter_mut().enumerate() {
                *b = (k as i32 - 128) as i8;
            }
        } else if case == 1 {
            s = [0i8; 435];
        } else if case == 2 {
            s = [-128i8; 435];
        } else if case == 3 {
            s = [127i8; 435];
        } else {
            for b in s.iter_mut() {
                *b = (rng.next() >> 16) as i8;
            }
        }
        let bytes: Vec<u8> = s.iter().map(|&b| b as u8).collect();
        emu.write(src, &bytes);
        for i in 0..435u64 {
            emu.write_u32(chunk + 0x170 + i * 4, 0xdead_beef);
        }
        emu.call(0x6aff10, chunk as u32, 0, &[]);
        let ours = mcnr_normals(&s);
        for i in 0..435usize {
            let binary = emu.read_f32(chunk + 0x170 + i as u64 * 4);
            assert_f32(
                binary,
                ours[i],
                &format!("mcnr_normals case={case} comp[{i}]"),
            );
        }
        checked += 1;
    }
    eprintln!("mcnr_normals: {checked} buffers × 435 components bit-exact vs WoW.exe @ 0x6aff10");
}

/// `LoadArea` (`0x6c2720`, ecx = the column, edx = the row) called whole with its calls stubbed and
/// the list globals set so the insert takes its plain path: the six fields of the area, over the
/// 64×64 grid and beyond it as far as the read of `0xc89f50` stays mapped.
#[test]
fn adt_area_origin_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let area = emu.alloc(0x100);
    let node = emu.alloc(0x40);
    let listhead = emu.alloc(0x40);

    // the name format (the caller pops), the area and the node it takes, and the read (`ret 4`)
    emu.stub(0x64a7f0, 0, 0);
    emu.stub(0x6a05d0, area as u32, 0);
    emu.stub(0x6a0450, node as u32, 0);
    emu.stub(0x6c1e70, 0, 1);

    // `[0xc96080]` 0 and `[node]` 0 skip the unlink; the insert goes to the head at `[0xc96084]`
    emu.write_u32(0xc96080, 0);
    emu.write_u32(node, 0);
    emu.write_u32(0xc96084, listhead as u32);
    emu.write_u32(listhead + 4, 0);

    let mut rng = Lcg(0x6164_7421);
    // corners and centres of the grid, and a few past it
    let structured: &[(u32, u32)] = &[
        (0, 0),
        (1, 0),
        (0, 1),
        (32, 32),
        (31, 33),
        (63, 63),
        (63, 0),
        (0, 63),
        (16, 48),
        (200, 1),
        (1, 200),
        (255, 255),
    ];
    let mut checked = 0u32;
    for (tx, ty) in structured
        .iter()
        .copied()
        .chain((0..300).map(|_| (rng.next() % 64, rng.next() % 64)))
    {
        for off in [0x7cu64, 0x80, 0x88, 0x8c, 0x90, 0x94] {
            emu.write_u32(area + off, 0xdead_beef);
        }
        // the insert changes the list globals
        emu.write_u32(0xc96080, 0);
        emu.write_u32(node, 0);
        emu.write_u32(0xc96084, listhead as u32);
        emu.write_u32(listhead + 4, 0);

        emu.call(0x6c2720, tx, ty, &[]);

        let ours = adt_area_origin(tx, ty);
        assert_f32(
            emu.read_f32(area + 0x7c),
            ours.origin0,
            &format!("adt_area_origin origin0(+0x7c) tx={tx} ty={ty}"),
        );
        assert_f32(
            emu.read_f32(area + 0x80),
            ours.origin1,
            &format!("adt_area_origin origin1(+0x80) tx={tx} ty={ty}"),
        );
        assert_eq!(
            emu.read_u32(area + 0x88),
            ours.tile_x,
            "adt_area_origin tile_x(+0x88) tx={tx} ty={ty}"
        );
        assert_eq!(
            emu.read_u32(area + 0x8c),
            ours.tile_y,
            "adt_area_origin tile_y(+0x8c) tx={tx} ty={ty}"
        );
        assert_eq!(
            emu.read_u32(area + 0x90),
            ours.chunk_base_x,
            "adt_area_origin chunk_base_x(+0x90) tx={tx} ty={ty}"
        );
        assert_eq!(
            emu.read_u32(area + 0x94),
            ours.chunk_base_y,
            "adt_area_origin chunk_base_y(+0x94) tx={tx} ty={ty}"
        );
        checked += 1;
    }
    eprintln!("adt_area_origin: {checked} tile pairs bit-exact vs WoW.exe @ 0x6c2720");
}
