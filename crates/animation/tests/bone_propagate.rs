//! `CM2Model::EnsureAnimated` (`0x714000`) run whole
//! (`systems/models/animation.md#ensure-animated`) on a chain of instances with their scene,
//! attachment records and instance matrices. `Animate` (`0x714260`) is a stub that returns (`ret
//! 0x10`, four stack dwords) and records its calls, and `0x7bc6a0` runs for real; the recursive
//! call `0x714023` is not stubbed.
//!
//! Compared bit for bit with `m2_bone_propagate`: (a) the 16-dword world matrix `+0xfc` of every
//! instance; (b) the sequence of `Animate` calls, with the composed matrix read back through the
//! recorded pointer and the colour and alpha arguments; (c) chains two and three deep, with the
//! ancestors' frames set and not.
//!
//! Cases: the frame test; no parent; the compose path; the parent's world matrix as the path for an
//! index of 0xffff, a parent that is not loaded, and a parent not yet animated; an instance that is
//! not loaded, which makes no call.

mod common;

use benilla_re_animation::bone_propagate::{
    m2_bone_propagate, AnimateCall, AnimateLeg, AttachmentRecord, Instance, InstanceGraph,
};
use benilla_re_difftest::{Emu, Wow};
use common::{assert_f32, fuzz_f32, next};

// The offsets within the arena, which every case rewrites.
const SHARED_OFF: u64 = 0x0000; // the scene: +0x10 the frame, +0x9c the matrix
const MD_OFF: u64 = 0x0200; // +0x130 leads to the header
const MD2_OFF: u64 = 0x0400; // the header: +0x108 leads to the records
const RECORDS_OFF: u64 = 0x0600; // the attachment records, stride 0x30
const POOL_OFF: u64 = 0x0a00; // the bone matrices, stride 0x40
const BONES_OFF: u64 = 0x1000; // the instances
const BONE_STRIDE: u64 = 0x400;

// The harness's stack region (`crates/difftest/src/lib.rs`): the colour arguments of the call
// with no parent are written here.
const STK_LO: u64 = 0x7000_0000;
const STK_HI: u64 = 0x7010_0000;

struct Scene {
    instances: Vec<Instance>,
    scene_frame: u32,
    scene_matrix: [f32; 16],
    records: Vec<AttachmentRecord>,
    pool: Vec<[f32; 16]>,
}

fn write_mat(emu: &mut Emu, va: u64, m: &[f32; 16]) {
    for (i, &v) in m.iter().enumerate() {
        emu.write_u32(va + (i as u64) * 4, v.to_bits());
    }
}

fn read_mat(emu: &Emu, va: u64) -> [f32; 16] {
    let mut m = [0f32; 16];
    for (i, e) in m.iter_mut().enumerate() {
        *e = emu.read_f32(va + (i as u64) * 4);
    }
    m
}

fn fuzz_mat(s: &mut u32) -> [f32; 16] {
    let mut m = [0f32; 16];
    for e in m.iter_mut() {
        *e = fuzz_f32(s);
    }
    m
}

/// Write the whole scene into the emulator; return each instance's VA.
fn materialize(emu: &mut Emu, arena: u64, scene: &Scene) -> Vec<u64> {
    let shared = arena + SHARED_OFF;
    let md = arena + MD_OFF;
    let md2 = arena + MD2_OFF;
    let records = arena + RECORDS_OFF;
    let pool = arena + POOL_OFF;

    emu.write_u32(shared + 0x10, scene.scene_frame);
    write_mat(emu, shared + 0x9c, &scene.scene_matrix);
    emu.write_u32(md + 0x130, md2 as u32);
    emu.write_u32(md2 + 0x108, records as u32);
    for (i, r) in scene.records.iter().enumerate() {
        let rva = records + (i as u64) * 0x30;
        emu.write_u32(rva + 4, u32::from(r.bone));
        emu.write_u32(rva + 8, r.offset[0].to_bits());
        emu.write_u32(rva + 0xc, r.offset[1].to_bits());
        emu.write_u32(rva + 0x10, r.offset[2].to_bits());
    }
    for (j, p) in scene.pool.iter().enumerate() {
        write_mat(emu, pool + (j as u64) * 0x40, p);
    }

    let vas: Vec<u64> = (0..scene.instances.len() as u64)
        .map(|i| arena + BONES_OFF + i * BONE_STRIDE)
        .collect();
    for (i, b) in scene.instances.iter().enumerate() {
        let v = vas[i];
        emu.write_u32(v + 0x10, b.loaded);
        emu.write_u32(v + 0x2c, shared as u32);
        emu.write_u32(v + 0x30, md as u32);
        emu.write_u32(v + 0x40, b.frame);
        emu.write_u32(v + 0x94, pool as u32);
        write_mat(emu, v + 0xbc, &b.base_matrix);
        write_mat(emu, v + 0xfc, &b.world_matrix);
        emu.write_u32(v + 0x19c, b.alpha.to_bits());
        emu.write_u32(v + 0x1cc, b.parent.map_or(0, |p| vas[p] as u32));
        emu.write_u32(v + 0x1d4, b.attachment_index);
    }
    vas
}

fn assert_mat(bin: &[f32; 16], ours: &[f32; 16], ctx: &str) {
    for i in 0..16 {
        assert_f32(bin[i], ours[i], &format!("{ctx}: world[{i}]"));
    }
}

/// Materialize, run the binary, run the transcription, diff worlds + the Animate-call sequence.
fn run_scene(emu: &mut Emu, arena: u64, scene: &mut Scene, test_idx: usize, ctx: &str) {
    let shared = arena + SHARED_OFF;
    let vas = materialize(emu, arena, scene);

    // --- the binary ---
    emu.clear_calls();
    emu.call(0x714000, vas[test_idx] as u32, 0, &[]);
    let bin_worlds: Vec<[f32; 16]> = vas.iter().map(|&v| read_mat(emu, v + 0xfc)).collect();
    // snapshot the recorded Animate calls (args) before the borrow of `emu` is needed again.
    let bin_calls: Vec<[u32; 8]> = emu.calls().iter().map(|c| c.args).collect();

    // --- the transcription ---
    let mut recorded: Vec<AnimateCall> = Vec::new();
    {
        let mut graph = InstanceGraph {
            instances: &mut scene.instances,
            scene_frame: scene.scene_frame,
            scene_matrix: scene.scene_matrix,
            attachments: &scene.records,
            palette: &scene.pool,
        };
        let mut rec = |c: AnimateCall| recorded.push(c);
        m2_bone_propagate(&mut graph, test_idx, &mut rec);
    }

    // (a) every instance's world matrix.
    for (i, (bin, ours)) in bin_worlds.iter().zip(scene.instances.iter()).enumerate() {
        assert_mat(bin, &ours.world_matrix, &format!("{ctx}: instance {i}"));
    }

    // (b) the Animate-call sequence (count + per-call args, in order).
    assert_eq!(
        bin_calls.len(),
        recorded.len(),
        "{ctx}: Animate call count (binary {} vs ours {})",
        bin_calls.len(),
        recorded.len()
    );
    for (i, (bin, ours)) in bin_calls.iter().zip(recorded.iter()).enumerate() {
        let c = format!(
            "{ctx}: call {i} ({:?} instance {})",
            ours.leg, ours.instance
        );
        match ours.leg {
            AnimateLeg::Identity => {
                // The matrix is `[this+0x2c]+0x9c`, the alpha is 1.0, and the colour arguments are
                // pointers on the stack.
                assert_eq!(bin[0], (shared + 0x9c) as u32, "{c}: src");
                assert_eq!(bin[3], 1.0f32.to_bits(), "{c}: alpha");
                for (k, &a) in bin.iter().enumerate().take(3).skip(1) {
                    let a = u64::from(a);
                    assert!(
                        (STK_LO..STK_HI).contains(&a),
                        "{c}: out-dest arg{k} {a:#x} not on the stack"
                    );
                }
                assert_ne!(bin[1], bin[2], "{c}: rot/trans out-dests must differ");
            }
            AnimateLeg::Simple => {
                // The matrix is `[parent]+0xfc`, the colours are `&[parent+0x1a0]` and
                // `&[parent+0x1ac]`, and the alpha is `[parent+0x19c]`.
                let p = scene.instances[ours.instance]
                    .parent
                    .expect("simple leg has a parent");
                let pva = vas[p];
                assert_eq!(bin[0], (pva + 0xfc) as u32, "{c}: src=[parent]+0xfc");
                assert_eq!(bin[1], (pva + 0x1a0) as u32, "{c}: rotOut");
                assert_eq!(bin[2], (pva + 0x1ac) as u32, "{c}: transOut");
                assert_eq!(
                    bin[3],
                    ours.alpha.expect("simple alpha").to_bits(),
                    "{c}: alpha=[parent+0x19c]"
                );
            }
            AnimateLeg::Compose => {
                // The matrix is the composed copy at `[ebp-0x58]`, a stack pointer; its 16 dwords
                // are read back and compared with the transcription's. The other arguments are the
                // parent's (`edx` is the parent at the call): `&[parent+0x1a0]`, `&[parent+0x1ac]`
                // and `[parent+0x19c]`.
                let ptr = u64::from(bin[0]);
                assert!(
                    (STK_LO..STK_HI).contains(&ptr),
                    "{c}: composed-src ptr {ptr:#x} not on the stack"
                );
                let bin_m = read_mat(emu, ptr);
                let ours_m = ours.src_matrix.expect("compose carries its src matrix");
                assert_mat(&bin_m, &ours_m, &format!("{c}: composed src"));
                let p = scene.instances[ours.instance]
                    .parent
                    .expect("compose leg has a parent");
                let pva = vas[p];
                assert_eq!(bin[1], (pva + 0x1a0) as u32, "{c}: rotOut=&[parent+0x1a0]");
                assert_eq!(
                    bin[2],
                    (pva + 0x1ac) as u32,
                    "{c}: transOut=&[parent+0x1ac]"
                );
                assert_eq!(
                    bin[3],
                    ours.alpha.expect("compose alpha").to_bits(),
                    "{c}: alpha=[parent+0x19c]"
                );
            }
        }
    }
}

/// An instance with everything zeroed except a fuzzed base matrix, world matrix and alpha; the
/// caller sets the fields that shape the graph.
fn fuzz_instance(s: &mut u32) -> Instance {
    Instance {
        loaded: 0,
        frame: 0,
        parent: None,
        attachment_index: 0,
        base_matrix: fuzz_mat(s),
        world_matrix: fuzz_mat(s),
        alpha: fuzz_f32(s),
    }
}

fn fuzz_records(s: &mut u32, n: usize, pool_len: usize) -> Vec<AttachmentRecord> {
    (0..n)
        .map(|_| AttachmentRecord {
            bone: ((next(s) >> 16) % pool_len as u32) as u16,
            offset: [fuzz_f32(s), fuzz_f32(s), fuzz_f32(s)],
        })
        .collect()
}

const LIMIT: u32 = 7;

#[test]
fn m2_bone_propagate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x714260, 0, 4); // `Animate`: `ret 0x10`, four stack dwords.
    let arena = emu.alloc(0x2000);

    let mut s = 0x7140_0000u32;
    let mut checked = 0u32;

    // (1) The frame equals the scene's: no work, and the world matrix is kept.
    {
        let mut b = fuzz_instance(&mut s);
        b.frame = LIMIT; // == scene_frame
        let mut scene = Scene {
            instances: vec![b],
            scene_frame: LIMIT,
            scene_matrix: fuzz_mat(&mut s),
            records: fuzz_records(&mut s, 4, 4),
            pool: vec![fuzz_mat(&mut s); 4],
        };
        run_scene(&mut emu, arena, &mut scene, 0, "done-guard");
        checked += 1;
    }

    // (2) No parent: one identity call, and the world matrix is `0x7bc6a0` of the base matrix
    // and the scene matrix.
    {
        let mut b = fuzz_instance(&mut s);
        b.frame = 0;
        let mut scene = Scene {
            instances: vec![b],
            scene_frame: LIMIT,
            scene_matrix: fuzz_mat(&mut s),
            records: fuzz_records(&mut s, 4, 4),
            pool: vec![fuzz_mat(&mut s); 4],
        };
        run_scene(&mut emu, arena, &mut scene, 0, "identity");
        checked += 1;
    }

    // (3) The compose path: a child of a root already animated this frame, both loaded, with a
    // valid attachment index.
    {
        let mut root = fuzz_instance(&mut s);
        root.frame = LIMIT; // pre-done ⇒ recursion short-circuits AND enables child's compose
        root.loaded = 1;
        let mut child = fuzz_instance(&mut s);
        child.parent = Some(0);
        child.loaded = 1;
        child.attachment_index = 2; // < records.len()
        let pool = vec![
            fuzz_mat(&mut s),
            fuzz_mat(&mut s),
            fuzz_mat(&mut s),
            fuzz_mat(&mut s),
        ];
        let mut scene = Scene {
            instances: vec![root, child],
            scene_frame: LIMIT,
            scene_matrix: fuzz_mat(&mut s),
            records: fuzz_records(&mut s, 6, pool.len()),
            pool,
        };
        run_scene(&mut emu, arena, &mut scene, 1, "compose");
        checked += 1;
    }

    // (4) The three reasons for the parent's world matrix, with the parent already animated so
    // the recursion returns. An instance that is not loaded makes no call.
    for (tag, idx_ffff, this_flag, parent_flag) in [
        ("simple idx==0xffff", true, 1u32, 1u32),
        ("skip this.loaded==0", false, 0, 1),
        ("simple parent.loaded==0", false, 1, 0),
    ] {
        let mut root = fuzz_instance(&mut s);
        root.frame = LIMIT;
        root.loaded = parent_flag;
        let mut child = fuzz_instance(&mut s);
        child.parent = Some(0);
        child.loaded = this_flag;
        child.attachment_index = if idx_ffff { 0xffff } else { 1 };
        let mut scene = Scene {
            instances: vec![root, child],
            scene_frame: LIMIT,
            scene_matrix: fuzz_mat(&mut s),
            records: fuzz_records(&mut s, 4, 4),
            pool: vec![fuzz_mat(&mut s); 4],
        };
        run_scene(&mut emu, arena, &mut scene, 1, tag);
        checked += 1;
    }

    // (c) Three deep, none animated: the identity call, then the parent's world matrix twice.
    // A fresh chain has no parent animated this frame, so the others take that path.
    {
        let mut gp = fuzz_instance(&mut s);
        gp.loaded = 1;
        let mut p = fuzz_instance(&mut s);
        p.parent = Some(0);
        p.loaded = 1;
        p.attachment_index = 1;
        let mut this = fuzz_instance(&mut s);
        this.parent = Some(1);
        this.loaded = 1;
        this.attachment_index = 2;
        let mut scene = Scene {
            instances: vec![gp, p, this],
            scene_frame: LIMIT,
            scene_matrix: fuzz_mat(&mut s),
            records: fuzz_records(&mut s, 4, 4),
            pool: vec![fuzz_mat(&mut s); 4],
        };
        run_scene(&mut emu, arena, &mut scene, 2, "recursion-3deep-none-done");
        checked += 1;
    }

    // (c) The middle instance animated: `this` recurses into `p`, which returns, so `gp` is not
    // reached and `this` composes from `p`.
    {
        let mut gp = fuzz_instance(&mut s);
        gp.loaded = 1;
        let mut p = fuzz_instance(&mut s);
        p.parent = Some(0);
        p.frame = LIMIT; // done ⇒ short-circuits, gp untouched, enables this's compose
        p.loaded = 1;
        let mut this = fuzz_instance(&mut s);
        this.parent = Some(1);
        this.loaded = 1;
        this.attachment_index = 3;
        let pool = vec![fuzz_mat(&mut s); 5];
        let mut scene = Scene {
            instances: vec![gp, p, this],
            scene_frame: LIMIT,
            scene_matrix: fuzz_mat(&mut s),
            records: fuzz_records(&mut s, 5, pool.len()),
            pool,
        };
        run_scene(&mut emu, arena, &mut scene, 2, "recursion-middle-done");
        checked += 1;
    }

    // Two-instance chains with fuzzed matrices, cycling the inputs that choose the path.
    for case in 0..120u32 {
        let pre_done = case % 2 == 0; // parent pre-done (enables compose / short-circuit)
        let this_flag = u32::from(case % 3 != 0);
        let parent_flag = u32::from(case % 5 != 0);
        let idx = if case % 7 == 0 {
            0xffff
        } else {
            (next(&mut s) >> 16) % 4
        };

        let mut root = fuzz_instance(&mut s);
        root.frame = if pre_done { LIMIT } else { 0 };
        root.loaded = parent_flag;
        let mut child = fuzz_instance(&mut s);
        child.parent = Some(0);
        child.loaded = this_flag;
        child.attachment_index = idx;
        let pool = vec![fuzz_mat(&mut s); 4];
        let mut scene = Scene {
            instances: vec![root, child],
            scene_frame: LIMIT,
            scene_matrix: fuzz_mat(&mut s),
            records: fuzz_records(&mut s, 4, pool.len()),
            pool,
        };
        run_scene(&mut emu, arena, &mut scene, 1, &format!("fuzz case {case}"));
        checked += 1;
    }

    eprintln!(
        "m2_bone_propagate: {checked} instance-graph propagations bit-exact vs WoW.exe @ 0x714000"
    );
}
