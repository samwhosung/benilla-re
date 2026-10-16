//! The vertex shader id of the shared model (`systems/models/shared.md#vertex-shader-id`):
//! `CM2Shared::GetVertexShaderId` (`0x71de50`) called whole.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_models::shared::{stage_digit, vertex_shader_id};
use common::Lcg;

/// A 16-bit value from the cases a lookup entry or a count takes: the thresholds 0, 1, 2, 3, the
/// top value, a value near the transform count, and any.
fn pick16(rng: &mut Lcg, near: u32) -> u16 {
    match rng.next() >> 29 {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 0xffff,
        5 => near.wrapping_sub(1) as u16,
        6 => near as u16,
        _ => (rng.next() >> 8) as u16,
    }
}

/// `0x71de50`, whole: the cache at `[this+4]` whose `+4` holds the granted features, the header
/// at `[this+0x130]` with its transform count `+0x74` and the material, coordinate and transform
/// arrays `+0x88`, `+0xa0` and `+0xb0`, the batch (texture count `+0xe`, material `+0xa`,
/// coordinate and transform starts `+0x12` and `+0x16`) and the submesh word `+0x10`.
#[test]
fn vertex_shader_id_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let cache = e.alloc(0x10);
    let this = e.alloc(0x140);
    let header = e.alloc(0xc0);
    let materials = e.alloc(0x40);
    let coords = e.alloc(0x40);
    let transforms = e.alloc(0x40);
    let batch = e.alloc(0x20);
    let submesh = e.alloc(0x40);
    e.write_u32(this + 4, cache as u32);
    e.write_u32(this + 0x130, header as u32);
    e.write_u32(header + 0x88, materials as u32);
    e.write_u32(header + 0xa0, coords as u32);
    e.write_u32(header + 0xb0, transforms as u32);

    let mut rng = Lcg(0x71de_5000);
    let mut ids = std::collections::BTreeSet::new();
    let mut digit_cases = [[0u32; 3]; 2];
    for case in 0..60_000u32 {
        let features = if case % 3 == 0 {
            rng.next()
        } else {
            rng.next() & 0x10
        };
        let transform_count = match rng.next() >> 30 {
            0 => 0,
            1 => 1,
            2 => rng.next() >> 28,
            _ => rng.next(),
        };
        let material_index = (rng.next() >> 8) % 8;
        let material_flags = (rng.next() >> 8) as u16;
        let influences = pick16(&mut rng, 4);
        let count = pick16(&mut rng, 2);
        let coord_start = (rng.next() >> 8) % 8;
        let transform_start = (rng.next() >> 8) % 8;
        let mut coord = [0u16; 16];
        let mut transform = [0u16; 16];
        for i in 0..16 {
            coord[i] = pick16(&mut rng, 3);
            transform[i] = pick16(&mut rng, transform_count);
            e.write(coords + 2 * i as u64, &coord[i].to_le_bytes());
            e.write(transforms + 2 * i as u64, &transform[i].to_le_bytes());
        }
        e.write_u32(cache + 4, features);
        e.write_u32(header + 0x74, transform_count);
        e.write(
            materials + 4 * u64::from(material_index),
            &u32::from(material_flags).to_le_bytes(),
        );
        e.write(batch + 0xa, &(material_index as u16).to_le_bytes());
        e.write(batch + 0xe, &count.to_le_bytes());
        e.write(batch + 0x12, &(coord_start as u16).to_le_bytes());
        e.write(batch + 0x16, &(transform_start as u16).to_le_bytes());
        e.write(submesh + 0x10, &influences.to_le_bytes());

        let binary = e.call(0x71de50, this as u32, 0, &[batch as u32, submesh as u32]);

        let digits = [0, 1].map(|k| {
            stage_digit(
                k,
                count,
                coord[(coord_start + k) as usize],
                transform[(transform_start + k) as usize],
                transform_count,
            )
        });
        for (k, d) in digits.iter().enumerate() {
            digit_cases[k][*d as usize] += 1;
        }
        let ours = vertex_shader_id(features & 0x10 != 0, material_flags, influences, digits);
        assert_eq!(
            binary, ours,
            "case {case}: features {features:#x} flags {material_flags:#x} influences {influences} \
             count {count} digits {digits:?}"
        );
        ids.insert(binary);
    }
    // Every digit of both stages was reached, and the ids were many.
    for (k, row) in digit_cases.iter().enumerate() {
        assert!(row.iter().all(|&n| n > 100), "stage {k} digits {row:?}");
    }
    assert!(ids.len() > 1000, "{} distinct ids", ids.len());
}
