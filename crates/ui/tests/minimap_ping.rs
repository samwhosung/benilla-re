//! `Minimap::GetPingPosition` (`0x4eefd0`, `systems/ui/minimap.md#reading-the-ping`) run from the exe.
//!
//! The player lookup and the zoom radius are stubbed, and `lua_pushnumber` (`0x6f3810`) is captured;
//! the test seeds the stored ping and compares the two pushed numbers, as `f64` bits, with
//! `ping_position`. The first is left in `st0` and pushed unrounded; the second is stored as `f32`
//! first. Every draw comes off the high bits of an LCG.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::minimap_geometry::ping_position as ping_calc;

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

fn dbits(c: &benilla_re_difftest::StubCall) -> f64 {
    f64::from_bits((u64::from(c.args[1]) << 32) | u64::from(c.args[0]))
}

fn pushes(emu: &Emu) -> Vec<f64> {
    emu.calls()
        .iter()
        .filter(|c| c.addr == 0x6f3810)
        .map(dbits)
        .collect()
}

// ===========================================================================================
// ping_position (0x4eefd0) — world→minimap (whole-call; radius fn patched, glue stubbed)
// ===========================================================================================
#[test]
fn ping_position() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // patch 0x6da9b0 (zoom radius) -> `fld dword [radius_cell]; ret`.
    let radius_cell = emu.alloc(4);
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(radius_cell as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(0x6da9b0, &code);
    // object lookup glue -> a fake object whose vtable+0x14 getter returns our position buffer.
    let obj = emu.alloc(0x10);
    let vtable = emu.alloc(0x28);
    let pos_buf = emu.alloc(12);
    let getpos = emu.alloc(0x10); // `mov eax, pos_buf; ret 4`
    let mut gp = vec![0xb8u8];
    gp.extend_from_slice(&(pos_buf as u32).to_le_bytes());
    gp.push(0xc2);
    gp.extend_from_slice(&4u16.to_le_bytes());
    emu.write(getpos, &gp);
    emu.write_u32(vtable + 0x14, getpos as u32);
    emu.write_u32(obj, vtable as u32);
    emu.stub(0x468550, 0, 1); // GUID-by-type (push 0x90)
    emu.stub(0x468460, obj as u32, 2); // obj-by-guid (push hi,lo)
    emu.stub(0x6f3810, 0, 2); // lua_pushnumber capture

    let mut s = 0x4eef_d000u32;
    let mut total = 0u32;
    for i in 0..4000 {
        let cx = rng(&mut s, -2000.0, 2000.0);
        let cy = rng(&mut s, -2000.0, 2000.0);
        let px = rng(&mut s, -2000.0, 2000.0);
        let py = rng(&mut s, -2000.0, 2000.0);
        let radius = if i == 0 { 1.0 } else { rng(&mut s, 5.0, 400.0) };
        emu.write_u32(0xbc787c, cx.to_bits());
        emu.write_u32(0xbc7880, cy.to_bits());
        emu.write_u32(pos_buf, px.to_bits());
        emu.write_u32(pos_buf + 4, py.to_bits());
        emu.write_u32(pos_buf + 8, 0);
        emu.write_u32(radius_cell, radius.to_bits());
        emu.clear_calls();
        emu.call(0x4eefd0, 0x1234, 0, &[]);
        let p = pushes(&emu);
        assert_eq!(p.len(), 2, "expected 2 pushnumber, got {}", p.len());
        let (r0, r1) = ping_calc(cx, cy, px, py, radius);
        assert_eq!(
            p[0].to_bits(),
            r0.to_bits(),
            "ping r0 cx={cx} cy={cy} px={px} py={py} r={radius}"
        );
        assert_eq!(
            p[1].to_bits(),
            r1.to_bits(),
            "ping r1 cx={cx} cy={cy} px={px} py={py} r={radius}"
        );
        total += 1;
    }
    eprintln!("ping_position: {total} bit-exact");
}
