//! `CCamera::ClipNearBox` (`0x50de90`) and `CCamera::SolveLimits` (`0x50e570`) run from the exe.
//!
//! The camera object, a vtable and the world queries are built in emulator memory. The clip runs
//! whole with its collision and matrix callees stubbed and the swept query `0x686db0` replaced by an
//! island that writes a chosen contact list. The solver runs whole as well: the inner clip and
//! `CCamera::PlaceEye` run for real, and the two ray queries `0x672170` are a counter island that
//! returns a chosen hit and fraction for the height probe and then for the boom sweep. The last
//! test pins the consequence for the pivot height of an eased target. Every draw comes off the high
//! bits of an LCG, and each branch is tallied and required to be hit.
#![allow(
    clippy::float_cmp,
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::vec_init_then_push
)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::camera_solver::{
    clip_near_box as clip, solve_limits as solve, ClipResult, SolverFields, Sweep,
};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// f32 in `[lo, hi)` from the LCG high bits.
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

/// Compare two `f32`s by bits; both-NaN passes (transcendental routing matched, payload unspeced).
fn eqf(bin: u32, mine: f32, ctx: &str) {
    if f32::from_bits(bin).is_nan() && mine.is_nan() {
        return;
    }
    assert_eq!(
        bin,
        mine.to_bits(),
        "{ctx}: binary {:#010x} ({}) vs rust {:#010x} ({mine})",
        bin,
        f32::from_bits(bin),
        mine.to_bits()
    );
}

/// Overwrite a function's prologue with raw island bytes (for boundary fns whose `st0`/pointer
/// return the `Emu::stub` mechanism cannot synthesize).
fn island(emu: &mut Emu, addr: u64, bytes: &[u8]) {
    emu.write(addr, bytes);
}
// ===========================================================================================
// 0x50de90 — swept-box velocity clip (thiscall, full machinery, sweep stubbed)
// ===========================================================================================
const BE0CCC: u64 = 0x00be_0ccc; // singleton-init flag (set bit0 to skip the init block)
const BE0F58: u64 = 0x00be_0f58; // collision volume count
const BE0F5C: u64 = 0x00be_0f5c; // collision volume base ptr

#[test]
fn cam_50de90() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // --- the camera object `this` + a synthetic vtable (slots 0, +4, +0xc) -------------------
    let vtable = emu.alloc(0x20);
    let this = emu.alloc(0x80);
    emu.write_u32(this, vtable as u32);
    // slot 0 returns a float10 (feeds an fstp) — `fldz; ret 0xc` island.
    let slot0 = emu.alloc(8);
    island(&mut emu, slot0, &[0xd9, 0xee, 0xc2, 0x0c, 0x00]);
    emu.write_u32(vtable, slot0 as u32);
    // slots +4 / +0xc write through a ptr we ignore — no-op stub islands (thiscall, 1 stack arg).
    let slot4 = emu.alloc(8);
    let slotc = emu.alloc(8);
    emu.write_u32(vtable + 4, slot4 as u32);
    emu.write_u32(vtable + 0xc, slotc as u32);
    emu.stub(slot4, 0, 1);
    emu.stub(slotc, 0, 1);

    // --- stub the middle's collision/matrix callees (their outputs don't affect the owned math) -
    emu.stub(0x4078e0, 0, 1); // ×8 init
    emu.stub(0x4549a0, 0, 3); // global temp-vec set
    emu.stub(0x5c3e70, 0, 2);
    emu.stub(0x5c3cc0, 0, 4);
    emu.stub(0x5c43b0, 0, 1);
    emu.stub(0x686500, 0, 1);
    emu.stub(0x513470, 0, 1);
    emu.stub(0x513330, 0, 1);
    emu.stub(0x672390, 0, 2);
    emu.stub(0x687080, 1, 1); // must return nonzero so the volume loop runs
    emu.stub(0x513570, 0, 0);
    emu.stub(0x4044b0, 0, 4);
    // 0x7bca80 returns a ptr that is dereferenced — return a valid 16-byte scratch.
    let xbuf = emu.alloc(16);
    emu.stub(0x7bca80, xbuf as u32, 1);

    // --- the swept query 0x686db0: island writing (contact-array ptr, count) + a hit flag -----
    // It receives [esp+4]=&arr, [esp+8]=&cnt (fastcall ecx/edx + 2 stack out-params, ret 8).
    // Build a fixed array of pointers to contact structs (each struct+8 = fraction).
    const NMAX: u64 = 4;
    let mut structs = Vec::new();
    for _ in 0..NMAX {
        structs.push(emu.alloc(0x10));
    }
    let arr = emu.alloc(NMAX * 4);
    for (k, &st) in structs.iter().enumerate() {
        emu.write_u32(arr + 4 * k as u64, st as u32);
    }
    let cnt_cell = emu.alloc(4);
    let flag_cell = emu.alloc(4);
    // island: mov eax,[esp+4]; mov [eax],arr; mov eax,[esp+8]; mov ecx,ds:cnt; mov [eax],ecx;
    //         mov eax,ds:flag; ret 8
    let mut sweep = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00];
    sweep.extend_from_slice(&(arr as u32).to_le_bytes());
    sweep.extend_from_slice(&[0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d]);
    sweep.extend_from_slice(&(cnt_cell as u32).to_le_bytes());
    sweep.extend_from_slice(&[0x89, 0x08, 0xa1]);
    sweep.extend_from_slice(&(flag_cell as u32).to_le_bytes());
    sweep.extend_from_slice(&[0xc2, 0x08, 0x00]);
    island(&mut emu, 0x686db0, &sweep);

    // --- globals: skip the singleton init; one collision volume ------------------------------
    emu.write_u32(BE0CCC, emu.read_u32(BE0CCC) | 1);
    emu.write_u32(BE0F58, 1);
    let vbuf = emu.alloc(0x40); // one volume (corners at +0x10..+0x33)
    emu.write_u32(BE0F5C, vbuf as u32);

    let dist_ptr = emu.alloc(4);
    let box_ptr = emu.alloc(12);

    struct C {
        dist: f32,
        nearclip: f32,
        fracs: Vec<f32>,
        hit: bool,
    }
    let mut cases: Vec<C> = Vec::new();
    // early-out (delta < dt).
    cases.push(C {
        dist: 5.0005,
        nearclip: 5.0,
        fracs: vec![],
        hit: false,
    });
    // hit, single contact.
    cases.push(C {
        dist: 10.0,
        nearclip: 2.0,
        fracs: vec![0.4],
        hit: true,
    });
    // hit, multi contact (max).
    cases.push(C {
        dist: 10.0,
        nearclip: 2.0,
        fracs: vec![0.2, 0.7, 0.5],
        hit: true,
    });
    // hit, but clip drives below zero -> floor.
    cases.push(C {
        dist: 3.0,
        nearclip: 0.0,
        fracs: vec![0.99, 0.99, 0.99, 0.99],
        hit: true,
    });
    // hit flag set but zero contacts (maxFrac stays 0 -> dist unchanged, ret 1).
    cases.push(C {
        dist: 8.0,
        nearclip: 1.0,
        fracs: vec![],
        hit: true,
    });
    // miss (no contact -> dist unchanged, ret 0).
    cases.push(C {
        dist: 8.0,
        nearclip: 1.0,
        fracs: vec![0.5],
        hit: false,
    });

    let mut s = 0x0050_de90u32;
    for _ in 0..3000 {
        let n = ((next(&mut s) >> 8) % (NMAX as u32 + 1)) as usize;
        let hit = (next(&mut s) >> 8) & 1 == 0;
        let mut fracs = Vec::new();
        for _ in 0..n {
            fracs.push(rng(&mut s, 0.0, 1.0));
        }
        // mix early-out and normal: occasionally make delta tiny.
        let nearclip = rng(&mut s, -20.0, 20.0);
        let dist = if (next(&mut s) >> 8).is_multiple_of(7) {
            nearclip + rng(&mut s, -0.002, 0.001) // straddle dt
        } else {
            nearclip + rng(&mut s, 0.0, 40.0)
        };
        cases.push(C {
            dist,
            nearclip,
            fracs,
            hit,
        });
    }

    let mut early = 0u32;
    let mut hit_n = 0u32;
    let mut miss = 0u32;
    let mut floored = 0u32;
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(dist_ptr, c.dist.to_bits());
        emu.write_u32(this + 0x38, c.nearclip.to_bits());
        // box position (feeds the boundary middle only).
        for k in 0..3 {
            emu.write_u32(box_ptr + 4 * k, 0);
        }
        // contacts + sweep controls.
        emu.write_u32(cnt_cell, c.fracs.len() as u32);
        emu.write_u32(flag_cell, c.hit as u32);
        for (k, &f) in c.fracs.iter().enumerate() {
            emu.write_u32(structs[k] + 8, f.to_bits());
        }
        emu.clear_calls();
        let ret = emu.call(
            0x50de90,
            this as u32,
            0,
            &[dist_ptr as u32, box_ptr as u32, 0],
        );

        let mine = clip(c.dist, c.nearclip, &c.fracs, c.hit);
        eqf(
            emu.read_u32(dist_ptr),
            mine.dist,
            &format!("50de90 case {i}: dist"),
        );
        assert_eq!(ret, mine.ret, "50de90 case {i}: ret eax");

        let delta = f64::from(c.dist) - f64::from(c.nearclip);
        if delta < 0.001 {
            early += 1;
        } else if c.hit {
            hit_n += 1;
            if mine.dist == 0.0 && !c.fracs.is_empty() {
                floored += 1;
            }
        } else {
            miss += 1;
        }
    }
    eprintln!(
        "cam_50de90: {} bit-exact; early={early} hit={hit_n} miss={miss} floored={floored}",
        cases.len()
    );
    assert!(
        early > 0 && hit_n > 0 && miss > 0 && floored > 0,
        "cam_50de90 branch coverage gap (early={early} hit={hit_n} miss={miss} floored={floored})"
    );
}

// ===========================================================================================
// 0x50e570 CCamera::SolveLimits, run whole
// ===========================================================================================
// Only the collision queries are replaced: the two `0x672170` ray queries (the height probe, then
// the boom sweep) and the `0x686db0` sweep inside the real `0x50de90`. The clip `0x50de90` and the
// eye placement `0x50de00` run for real; the direction getter (vtable +4) is an island that writes
// its vector through the out parameter and returns it in `eax`. `0x672170` is a counter island:
// call N in {0, 1} writes the fraction of entry N into its `&frac` out parameter (`[esp+0xc]`) and
// returns the hit flag of entry N. The height probe always runs first (the cases keep
// `height_seed - near_clip > 2^-20`), so it is entry 0 and the boom sweep entry 1. The expected
// `boom_clip` is computed with `clip_near_box` over the same stubbed `0x686db0` contacts.
const BE1088: u64 = 0x00be_1088; // the object whose +0x28 selects the water bits of the trace mask

#[test]
fn cam_50e570() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // --- camera `this` + vtable (slot0 float10 getter, slot+4 dir getter, slot+0xc matrix no-op) --
    let vtable = emu.alloc(0x20);
    let this = emu.alloc(0x200);
    emu.write_u32(this, vtable as u32);
    // slot 0: `fldz; ret` (float10 getter). It is __cdecl — `0x50de90` reuses its three pushed args
    // for the very next call (`0x5c3cc0`, disasm 50e0b0-c3), so the getter must clean ZERO stack
    // bytes. A `ret 0xc` here would drift esp by 0xc and make 0x50de90's `pop edi/esi/ebx` (which run
    // before its `mov esp,ebp`) restore zeroed locals — silently clobbering the caller's `ebx`
    // (= `near_out`) that `0x50e570` dereferences right after, faulting. `ret` (clean 0) is faithful.
    let slot0 = emu.alloc(8);
    island(&mut emu, slot0, &[0xd9, 0xee, 0xc3]);
    emu.write_u32(vtable, slot0 as u32);
    // slot +4: direction getter — writes dir through the out-param AND returns &dir in eax (both
    // 0x50e570's direct call / 0x50de00 use eax; 0x50de90 uses the out-param). dir is arbitrary
    // (it only feeds the stubbed sweeps / box geometry, never the owned near/far algebra).
    let dir_p = emu.alloc(12);
    emu.write_u32(dir_p, 0.40f32.to_bits());
    emu.write_u32(dir_p + 4, 0.30f32.to_bits());
    emu.write_u32(dir_p + 8, 0.866f32.to_bits());
    let slot4 = emu.alloc(8);
    emu.stub_write_through(slot4, 0, dir_p, 12, dir_p as u32, 1);
    emu.write_u32(vtable + 4, slot4 as u32);
    // slot +0xc: matrix writer, no-op (1 stack arg).
    let slotc = emu.alloc(8);
    emu.stub(slotc, 0, 1);
    emu.write_u32(vtable + 0xc, slotc as u32);

    // --- the inner 0x50de90's boundary callees (outputs don't affect the owned clip) -------------
    emu.stub(0x4078e0, 0, 1);
    emu.stub(0x4549a0, 0, 3);
    emu.stub(0x5c3e70, 0, 2);
    emu.stub(0x5c3cc0, 0, 4);
    emu.stub(0x5c43b0, 0, 1);
    emu.stub(0x686500, 0, 1);
    emu.stub(0x513470, 0, 1);
    emu.stub(0x513330, 0, 1);
    emu.stub(0x672390, 0, 2);
    emu.stub(0x687080, 1, 1);
    emu.stub(0x513570, 0, 0);
    emu.stub(0x4044b0, 0, 4);
    let xbuf = emu.alloc(16);
    emu.stub(0x7bca80, xbuf as u32, 1);
    // singleton init already done; one collision volume.
    emu.write_u32(0xbe0ccc, emu.read_u32(0xbe0ccc) | 1);
    emu.write_u32(0xbe0f58, 1);
    let vbuf = emu.alloc(0x40);
    emu.write_u32(0xbe0f5c, vbuf as u32);

    // --- 0x686db0 (inside 0x50de90): writes (contact-array ptr, count) + a hit flag --------------
    const NMAX: u64 = 4;
    let mut structs = Vec::new();
    for _ in 0..NMAX {
        structs.push(emu.alloc(0x10));
    }
    let arr = emu.alloc(NMAX * 4);
    for (k, &st) in structs.iter().enumerate() {
        emu.write_u32(arr + 4 * k as u64, st as u32);
    }
    let cnt_cell = emu.alloc(4);
    let flag686 = emu.alloc(4);
    {
        let mut sweep = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00];
        sweep.extend_from_slice(&(arr as u32).to_le_bytes());
        sweep.extend_from_slice(&[0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d]);
        sweep.extend_from_slice(&(cnt_cell as u32).to_le_bytes());
        sweep.extend_from_slice(&[0x89, 0x08, 0xa1]);
        sweep.extend_from_slice(&(flag686 as u32).to_le_bytes());
        sweep.extend_from_slice(&[0xc2, 0x08, 0x00]);
        island(&mut emu, 0x686db0, &sweep);
    }

    // --- 0x672170 (far then near): counter island over a 2-entry {frac:f32, hit:u32} table -------
    let ctr = emu.alloc(4);
    let sweeptbl = emu.alloc(16); // [far.frac, far.hit, near.frac, near.hit]
    {
        let c32 = (ctr as u32).to_le_bytes();
        let t32 = (sweeptbl as u32).to_le_bytes();
        let mut isl: Vec<u8> = Vec::new();
        isl.extend_from_slice(&[0x8b, 0x0d]); // mov ecx, ds:[ctr]
        isl.extend_from_slice(&c32);
        isl.extend_from_slice(&[0x8d, 0x14, 0xcd]); // lea edx, [ecx*8 + sweeptbl]
        isl.extend_from_slice(&t32);
        isl.extend_from_slice(&[0x8b, 0x44, 0x24, 0x0c]); // mov eax, [esp+0xc]  (&frac out-param)
        isl.extend_from_slice(&[0x8b, 0x0a, 0x89, 0x08]); // mov ecx,[edx]; mov [eax],ecx
        isl.extend_from_slice(&[0xa1]); // mov eax, ds:[ctr]
        isl.extend_from_slice(&c32);
        isl.extend_from_slice(&[0x83, 0xf0, 0x01]); // xor eax, 1
        isl.extend_from_slice(&[0xa3]); // mov ds:[ctr], eax
        isl.extend_from_slice(&c32);
        isl.extend_from_slice(&[0x8b, 0x42, 0x04]); // mov eax, [edx+4]  (hit -> al)
        isl.extend_from_slice(&[0xc2, 0x10, 0x00]); // ret 0x10
        emu.write(0x672170, &isl);
    }

    // g-flag object.
    let gobj = emu.alloc(0x40);
    emu.write_u32(BE1088, gobj as u32);

    let in_vec = emu.alloc(12);
    let near_out = emu.alloc(4);
    let far_out = emu.alloc(4);
    let off_p = emu.alloc(12); // param_5 (pull-back offset) — feeds 0x50de00 -> stubbed, owned-inert
    for k in 0..3 {
        emu.write_u32(off_p + 4 * k, 0);
    }

    const EPS_DIST: f64 = f32::from_bits(0x3580_0000) as f64; // 0x8026bc
    const DT: f64 = f32::from_bits(0x3a83_126f) as f64; // 1e-3

    struct C {
        distance_target: f32,
        distance_live: f32,
        height_target: f32,
        height_live: f32,
        nearclip: f32,
        mode: u32,
        in_vec: [f32; 3],
        cvar: f32,
        g_flag: bool,
        height_sweep: Sweep,
        boom_sweep: Sweep,
        contacts: Vec<f32>,
        hit686: bool,
        far_active: bool, // false => family B (both chains skipped; no 0x672170 calls)
    }
    let mut cases: Vec<C> = Vec::new();
    let mut s = 0x0050_e570u32;

    // Family A — far ALWAYS active (so far=index0, near=index1). near active/inactive varies.
    for _ in 0..6000 {
        let nearclip = rng(&mut s, -5.0, 2.0);
        let ncnt = ((next(&mut s) >> 8) % (NMAX as u32 + 1)) as usize;
        let mut contacts = Vec::new();
        for _ in 0..ncnt {
            contacts.push(rng(&mut s, 0.0, 1.0));
        }
        // near seed sometimes near/below nearclip (inactive), sometimes well above (active).
        let near_seed_hi = if (next(&mut s) >> 8).is_multiple_of(3) {
            nearclip + rng(&mut s, -0.5, 0.0005) // mostly inactive / straddle ε
        } else {
            nearclip + rng(&mut s, 0.5, 25.0)
        };
        cases.push(C {
            distance_target: near_seed_hi - rng(&mut s, 0.0, 3.0),
            distance_live: near_seed_hi,
            height_target: nearclip + rng(&mut s, 5.0, 30.0),
            height_live: nearclip + rng(&mut s, 5.0, 30.0),
            nearclip,
            mode: [0u32, 0x100000, 0x200000, 0x300000][((next(&mut s) >> 8) % 4) as usize],
            in_vec: [
                rng(&mut s, -10.0, 10.0),
                rng(&mut s, -10.0, 10.0),
                rng(&mut s, -10.0, 10.0),
            ],
            cvar: rng(&mut s, -2.0, 12.0),
            g_flag: (next(&mut s) >> 8) & 1 == 0,
            height_sweep: Sweep {
                hit: (next(&mut s) >> 8) & 1 == 0,
                frac: rng(&mut s, 0.0, 1.0),
            },
            boom_sweep: Sweep {
                hit: (next(&mut s) >> 8) & 1 == 0,
                frac: rng(&mut s, 0.0, 1.0),
            },
            contacts,
            hit686: (next(&mut s) >> 8) & 1 == 0,
            far_active: true,
        });
    }
    // Family B — both chains skipped (all four limits within ε of nearclip): no sweep calls at all.
    for _ in 0..600 {
        let nearclip = rng(&mut s, -5.0, 20.0);
        cases.push(C {
            distance_target: nearclip,
            distance_live: nearclip,
            height_target: nearclip,
            height_live: nearclip,
            nearclip,
            mode: [0u32, 0x100000, 0x200000][((next(&mut s) >> 8) % 3) as usize],
            in_vec: [0.0, 0.0, 0.0],
            cvar: rng(&mut s, -2.0, 12.0),
            g_flag: (next(&mut s) >> 8) & 1 == 0,
            height_sweep: Sweep {
                hit: false,
                frac: 1.0,
            },
            boom_sweep: Sweep {
                hit: false,
                frac: 1.0,
            },
            contacts: vec![],
            hit686: false,
            far_active: false,
        });
    }

    let mut far_hit = 0u32;
    let mut near_hit = 0u32;
    let mut clip_used = 0u32;
    let mut f100 = 0u32;
    let mut f200 = 0u32;
    let mut near_inactive = 0u32;
    let mut fam_b = 0u32;
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(this + 0x198, c.distance_target.to_bits());
        emu.write_u32(this + 0xec, c.distance_live.to_bits());
        emu.write_u32(this + 0x1c8, c.height_target.to_bits());
        emu.write_u32(this + 0xfc, c.height_live.to_bits());
        emu.write_u32(this + 0x38, c.nearclip.to_bits());
        emu.write_u32(this + 0x90, c.mode);
        for k in 0..3 {
            emu.write_u32(in_vec + 4 * k as u64, c.in_vec[k].to_bits());
        }
        emu.write_u32(gobj + 0x28, u32::from(c.g_flag));
        // sweep table: far=index0, near=index1.
        emu.write_u32(ctr, 0);
        emu.write_u32(sweeptbl, c.height_sweep.frac.to_bits());
        emu.write_u32(sweeptbl + 4, u32::from(c.height_sweep.hit));
        emu.write_u32(sweeptbl + 8, c.boom_sweep.frac.to_bits());
        emu.write_u32(sweeptbl + 12, u32::from(c.boom_sweep.hit));
        // 0x686db0 contacts.
        emu.write_u32(cnt_cell, c.contacts.len() as u32);
        emu.write_u32(flag686, u32::from(c.hit686));
        for (k, &f) in c.contacts.iter().enumerate() {
            emu.write_u32(structs[k] + 8, f.to_bits());
        }
        emu.write_u32(near_out, 0x7f80_0000); // poison
        emu.write_u32(far_out, 0x7f80_0000);

        let mask = emu.call(
            0x50e570,
            this as u32,
            0,
            &[
                in_vec as u32,
                near_out as u32,
                far_out as u32,
                off_p as u32,
                c.cvar.to_bits(),
            ],
        );

        // --- expected: replicate the near pre-clip seed + sweep scaling, then the proven cam_50de90.
        let fields = SolverFields {
            distance_target: c.distance_target,
            distance_live: c.distance_live,
            height_target: c.height_target,
            height_live: c.height_live,
            near_clip: c.nearclip,
            mode: c.mode,
        };
        let near_seed = if c.distance_target <= c.distance_live {
            c.distance_live
        } else {
            c.distance_target
        };
        let mut boom_clip = ClipResult {
            dist: near_seed,
            ret: 0,
        };
        let mut reached_clip = false;
        if EPS_DIST < f64::from(near_seed) - f64::from(c.nearclip) {
            let mut np = near_seed;
            if c.boom_sweep.hit {
                np = (f64::from(c.boom_sweep.frac) * f64::from(np)) as f32;
            }
            if EPS_DIST < f64::from(np) {
                boom_clip = clip(np, c.nearclip, &c.contacts, c.hit686);
                reached_clip = true;
            }
        }
        let want = solve(
            fields,
            c.cvar,
            c.g_flag,
            c.height_sweep,
            c.boom_sweep,
            boom_clip,
        );

        eqf(
            emu.read_u32(near_out),
            want.distance,
            &format!("50e570 case {i}: distance"),
        );
        eqf(
            emu.read_u32(far_out),
            want.height,
            &format!("50e570 case {i}: height"),
        );
        assert_eq!(mask, want.mask, "50e570 case {i}: mask eax");

        // coverage
        let near_active = EPS_DIST < f64::from(near_seed) - f64::from(c.nearclip);
        if c.far_active {
            if c.height_sweep.hit {
                far_hit += 1;
            }
            if near_active && c.boom_sweep.hit {
                near_hit += 1;
            }
            if reached_clip {
                clip_used += 1;
            }
            if !near_active {
                near_inactive += 1;
            }
            if c.g_flag && c.mode & 0x100000 != 0 {
                f100 += 1;
            }
            if c.g_flag && c.mode & 0x100000 == 0 && c.mode & 0x200000 != 0 {
                f200 += 1;
            }
        } else {
            fam_b += 1;
        }
        let _ = DT;
    }
    eprintln!(
        "cam_50e570: {} bit-exact (far_hit={far_hit} near_hit={near_hit} clip_used={clip_used} \
         flag100000={f100} flag200000={f200} near_inactive={near_inactive} far_skip={fam_b})",
        cases.len()
    );
    assert!(
        far_hit > 0
            && near_hit > 0
            && clip_used > 0
            && f100 > 0
            && f200 > 0
            && near_inactive > 0
            && fam_b > 0,
        "50e570 coverage gap"
    );
}

// ===========================================================================================
// 0x50e570: the direction of the pivot height
// ===========================================================================================
// `*height` is the follow pivot height above the feet (`pivot.z = feet.z + height`, `0x50ed5e` to
// `0x50ed6e`). Its seed is the larger of `height_target`, the transition's target, and
// `height_live`, the eased live value. A target above the live value does not lift the pivot at
// once: the height chain subtracts the 5/6 base, probes that
// corridor, adds 5/6 back, and clamps to `[local8, local10]`, where `local10` defaults to
// `height_live`. Unobstructed, the output follows the eased live value in both directions, and the
// larger seed only widens the corridor probed above the head. The run above checks the algebra;
// this test checks that consequence on its own.
#[test]
fn pivot_glide_direction_pin() {
    let nohit = Sweep {
        hit: false,
        frac: 1.0,
    };
    let noclip = ClipResult { dist: 0.0, ret: 0 };
    // distance_target == distance_live == nearclip keeps the near/distance chain inactive; mode = 0 (no taxi/swim).
    let mk = |live: f32, tgt: f32| SolverFields {
        distance_target: 0.1,
        distance_live: 0.1,
        height_target: tgt,
        height_live: live,
        near_clip: 0.1,
        mode: 0,
    };
    // (live, target): rising = mid-glide shorter->taller morph; falling = taller->shorter.
    let cases: [(f32, f32); 6] = [
        (0.9, 2.2),
        (1.0, 2.0),
        (1.3, 2.2), // rising: target above the gliding live
        (2.2, 0.9),
        (2.0, 1.0),
        (1.4, 0.9), // falling: target below the gliding live
    ];
    for (live, tgt) in cases {
        let got = solve(mk(live, tgt), 0.0, false, nohit, nohit, noclip);
        // The pivot height rides the LIVE channel (to the chain's f32 round-trip), never the
        // in-flight target: |far - live| within 2 ulp, and far is NOT pulled toward the target.
        let ulp = live * f32::EPSILON;
        assert!(
            (got.height - live).abs() <= 2.0 * ulp,
            "pivot must ride the live channel: live={live} tgt={tgt} far={}",
            got.height
        );
        assert!(
            (got.height - tgt).abs() > 0.5 * (tgt - live).abs(),
            "pivot must not chase the raw target: live={live} tgt={tgt} far={}",
            got.height
        );
        // And the RISING case clamps at live exactly (the [5/6, height_live] upper clamp binds).
        if tgt > live {
            assert_eq!(
                got.height, live,
                "rising-target clamp must bind exactly at live"
            );
        }
    }
}
