//! `CGMinimapFrame::PlacePlayerModel` (`0x4ee100`) against `WoW.exe` in the emulator, whole
//! (`systems/ui/minimap.md#the-player-arrow-each-frame`). The minimap and its player arrow are
//! built in scratch memory; the arrow's layout vtable at `+0x24` points slots 7 and 8 at two
//! stubs that leave a chosen `f64` in `st0`, as the width and height getters of a Model frame
//! do. The three dwords at the arrow's `+0x390` are compared bit for bit, and a minimap with no
//! arrow writes nothing.
use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::minimap_arrow::arrow_position;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*s).rotate_right(8)
}

#[test]
fn minimap_arrow_centre() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let minimap = emu.alloc(0x344);
    let arrow = emu.alloc(0x3d8);
    let vtable = emu.alloc(0x40);
    let width_stub = emu.alloc(0x10);
    let height_stub = emu.alloc(0x10);
    let width_cell = emu.alloc(8);
    let height_cell = emu.alloc(8);
    emu.write(width_stub, &[0xc3]);
    emu.write(height_stub, &[0xc3]);
    emu.stub_f64(width_stub, width_cell, 0);
    emu.stub_f64(height_stub, height_cell, 0);
    emu.write_u32(vtable + 0x1c, width_stub as u32);
    emu.write_u32(vtable + 0x20, height_stub as u32);
    emu.write_u32(arrow + 0x24, vtable as u32);

    let mut cases: Vec<(f64, f64)> = vec![
        (0.0, 0.0),
        (-0.0, 0.0),
        (1.0, 2.0),
        (0.140625, 0.140625),
        (f64::from(f32::from_bits(0x4b00_0001)), 3.0),
        (f64::from(f32::MIN_POSITIVE), f64::from(f32::from_bits(1))),
        (f64::from(f32::MAX), -f64::from(f32::MAX)),
        (f64::INFINITY, f64::NEG_INFINITY),
        (1.0 + f64::EPSILON, 1.0 / 3.0),
        (2.0 * f64::from(f32::MAX), 1e-40),
    ];
    let mut s = 0x4ee1_0000u32;
    for _ in 0..5_000 {
        let a = f64::from(next(&mut s)) / 65_536.0 - 30_000.0;
        let b = f64::from(next(&mut s)) / 1e6;
        cases.push((a, b));
    }
    let mut total = 0u32;
    for &(w, h) in &cases {
        emu.write(width_cell, &w.to_bits().to_le_bytes());
        emu.write(height_cell, &h.to_bits().to_le_bytes());
        emu.write(arrow + 0x390, &[0x55; 12]);
        emu.write_u32(minimap + 0x338, arrow as u32);
        emu.call(0x4e_e100, minimap as u32, 0, &[]);
        let want = arrow_position(w, h);
        for (k, v) in want.iter().enumerate() {
            let got = emu.read_u32(arrow + 0x390 + 4 * k as u64);
            assert_eq!(got, v.to_bits(), "w={w} h={h} component {k}");
        }
        total += 1;
    }
    // No arrow: nothing is written.
    emu.write_u32(minimap + 0x338, 0);
    emu.write(arrow + 0x390, &[0x55; 12]);
    emu.call(0x4e_e100, minimap as u32, 0, &[]);
    assert_eq!(emu.read(arrow + 0x390, 12), vec![0x55; 12], "no arrow");
    eprintln!(
        "minimap_arrow_centre: {} bit-exact (0x4ee100 whole)",
        total + 1
    );
}
