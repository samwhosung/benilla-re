//! The two table loops of `LoadMinimapTextures` (`0x4ed810`, `systems/ui/minimap.md#minimap-textures`)
//! run from the exe.
//!
//! The function loads three textures and fills two `.bss` tables, one of 64 rows set by the
//! divisor `[0xbc7644]` and one of 5 rows. The test stubs the texture and mesh calls and compares
//! both tables, row by row and bit for bit, with `minimap_icon_lattice` for a range of divisors.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::minimap_geometry::minimap_icon_lattice as quadgrid;

fn assert_bits(mine: f32, bin: f32, ctx: &str) {
    if mine.is_nan() && bin.is_nan() {
        return;
    }
    assert_eq!(
        mine.to_bits(),
        bin.to_bits(),
        "{ctx}: rust {:#010x} ({mine}) vs binary {:#010x} ({bin})",
        mine.to_bits(),
        bin.to_bits()
    );
}

// ---- geo_4ed810: grid divisor global + table addresses + boundary stubs ----
const G_GRID_N: u64 = 0x00bc_7644; // grid divisor (the `div` + `fild` operand)

const TABLE1: u64 = 0x00bc_7978; // 64 rows × 8 f32

const TABLE2: u64 = 0x00bc_7898; // 5 rows × 8 f32

const MESH_VBSET: u64 = 0x0058_a980; // vertex-buffer accessor (returns ptr; deref'd)

const MESH_APPEND: u64 = 0x0044_9d90; // descriptor append (returns ptr)

const MESH_INIT: u64 = 0x004e_f7a0; // local init

const MESH_FINISH: u64 = 0x0041_9e30; // finalize

#[test]
fn geo_4ed810() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // A readable scratch the mesh accessors "return" (the binary deref's `[eax]`).
    let sret = emu.alloc(0x40);
    emu.stub(MESH_INIT, 0, 0); // thiscall, no stack args
    emu.stub(MESH_VBSET, sret as u32, 8); // thiscall, 8 stack args; returns ptr (deref'd)
    emu.stub(MESH_APPEND, sret as u32, 3); // thiscall, 3 stack args
    emu.stub(MESH_FINISH, 0, 0); // thiscall, no stack args

    let mut checked = 0u32;
    let mut grids = 0u32;
    for &n in &[1u32, 2, 3, 4, 7, 8, 16, 64] {
        emu.write_u32(G_GRID_N, n);
        emu.clear_calls();
        emu.call(0x4ed810, 0, 0, &[]);
        let mine = quadgrid(n);
        for (i, row) in mine.table1.iter().enumerate() {
            for (k, &v) in row.iter().enumerate() {
                let bin = emu.read_f32(TABLE1 + (0x20 * i as u64) + 4 * k as u64);
                assert_bits(v, bin, &format!("geo_4ed810 n={n} table1[{i}][{k}]"));
                checked += 1;
            }
        }
        for (j, row) in mine.table2.iter().enumerate() {
            for (k, &v) in row.iter().enumerate() {
                let bin = emu.read_f32(TABLE2 + (0x20 * j as u64) + 4 * k as u64);
                assert_bits(v, bin, &format!("geo_4ed810 n={n} table2[{j}][{k}]"));
                checked += 1;
            }
        }
        grids += 1;
    }
    eprintln!(
        "geo_4ed810: {checked} f32 entries bit-exact across {grids} grid divisors \
         (table1 64×8 fild/fdiv->f32 + table2 5×8 ·0.25->f32)"
    );
    assert!(
        checked > 0 && grids > 1,
        "geo_4ed810: insufficient coverage"
    );
}
