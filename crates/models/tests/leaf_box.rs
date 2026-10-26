//! `AaBspQueryLeafBox` (`0x6bca50`, `systems/models/collisionquery.md#group-box-gather`) run whole,
//! `BoxContainsTriangle` (`0x6b8b70`) included, over runs of faces on one context: the face flag
//! bytes, the visited and kept stacks and their counts, and the status word are compared after
//! every call.

#![allow(clippy::needless_range_loop)] // index loops follow the client layout

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_models::aabsp::{aabsp_query_leaf_box, LeafBoxQuery, VISITED_CAPACITY};
use common::Lcg;

const FN_VA: u64 = 0x6bca50;
const VISITED_VA: u64 = 0x00ce_26e8;
const VISITED_COUNT_VA: u64 = 0x00ce_26e0;
const KEPT_VA: u64 = 0x00cd_e648;
const KEPT_COUNT_VA: u64 = 0x00ce_66fc;

const FACES: usize = 24;
const VERTS: usize = 16;

fn coord(rng: &mut Lcg) -> f32 {
    match rng.next() >> 30 {
        0 => (rng.next() % 9) as f32 - 4.0,
        1 => rng.f32_mixed(),
        _ => ((rng.next() >> 8) as f32 / (1u32 << 24) as f32) * 8.0 - 4.0,
    }
}

#[test]
fn aabsp_query_leaf_box_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ctx = e.alloc(0x18);
    let status_va = e.alloc(4);
    let faces_va = e.alloc(2 * FACES as u64);
    let verts_va = e.alloc(12 * VERTS as u64);
    let idx_va = e.alloc(6 * FACES as u64);
    let box_va = e.alloc(0x18);
    let mut rng = Lcg(0x6bca_5000);
    for case in 0..400 {
        let mut faces: Vec<u8> = (0..2 * FACES).map(|_| rng.byte() & 0x7f).collect();
        let vertices: Vec<[f32; 3]> = (0..VERTS)
            .map(|_| [coord(&mut rng), coord(&mut rng), coord(&mut rng)])
            .collect();
        let indices: Vec<u16> = (0..3 * FACES)
            .map(|_| (rng.next() % VERTS as u32) as u16)
            .collect();
        let lo = [coord(&mut rng), coord(&mut rng), coord(&mut rng)];
        let aabb = [lo[0], lo[1], lo[2], lo[0] + 2.0, lo[1] + 2.0, lo[2] + 2.0];
        let mask = [0x84u8, 0x82, 0xc6, 0x80][(rng.next() >> 30) as usize];
        let null_status = case % 7 == 3;
        // Start the visited stack empty, or near or at its capacity.
        let start = match case % 4 {
            0 | 1 => 0,
            2 => VISITED_CAPACITY - 3,
            _ => VISITED_CAPACITY,
        };
        let status0 = rng.next() & !1;

        e.write(faces_va, &faces);
        for (i, v) in vertices.iter().enumerate() {
            for k in 0..3 {
                e.write_u32(verts_va + 12 * i as u64 + 4 * k as u64, v[k].to_bits());
            }
        }
        for (i, &w) in indices.iter().enumerate() {
            e.write(idx_va + 2 * i as u64, &w.to_le_bytes());
        }
        for (k, &f) in aabb.iter().enumerate() {
            e.write_u32(box_va + 4 * k as u64, f.to_bits());
        }
        e.write_u32(status_va, status0);
        e.write_u32(ctx, if null_status { 0 } else { status_va as u32 });
        e.write_u32(ctx + 0x4, faces_va as u32);
        e.write_u32(ctx + 0x8, verts_va as u32);
        e.write_u32(ctx + 0xc, idx_va as u32);
        e.write_u32(ctx + 0x10, box_va as u32);
        e.write_u32(ctx + 0x14, u32::from(mask) | 0xabcd_0000);
        e.write_u32(VISITED_COUNT_VA, start as u32);
        e.write_u32(KEPT_COUNT_VA, 0);

        let mut q = LeafBoxQuery {
            status: (!null_status).then_some(status0),
            faces: &mut faces,
            vertices: &vertices,
            indices: &indices,
            aabb,
            mask,
            visited: vec![0; start],
            kept: Vec::new(),
        };
        // Each face once, some twice, so the visited bit itself rejects.
        let order: Vec<u16> = (0..FACES as u16).chain([0, 5, 7]).collect();
        for &face in &order {
            e.call(FN_VA, ctx as u32, 0, &[u32::from(face)]);
            aabsp_query_leaf_box(&mut q, face);
            let ctxs = format!("case {case} face {face}");
            assert_eq!(e.read(faces_va, 2 * FACES), *q.faces, "{ctxs}: flags");
            assert_eq!(
                e.read_u32(VISITED_COUNT_VA) as usize,
                q.visited.len(),
                "{ctxs}: visited"
            );
            assert_eq!(
                e.read_u32(KEPT_COUNT_VA) as usize,
                q.kept.len(),
                "{ctxs}: kept"
            );
            if !null_status {
                assert_eq!(Some(e.read_u32(status_va)), q.status, "{ctxs}: status");
            }
        }
        for (i, &f) in q.visited.iter().enumerate().skip(start) {
            let w = e.read(VISITED_VA + 2 * i as u64, 2);
            assert_eq!(
                u16::from_le_bytes([w[0], w[1]]),
                f,
                "case {case}: visited[{i}]"
            );
        }
        for (i, &f) in q.kept.iter().enumerate() {
            let w = e.read(KEPT_VA + 2 * i as u64, 2);
            assert_eq!(
                u16::from_le_bytes([w[0], w[1]]),
                f,
                "case {case}: kept[{i}]"
            );
        }
    }
}
