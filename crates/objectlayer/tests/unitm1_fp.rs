//! The unit method kernels of `src/unitm1_fp.rs` against the exe:
//! - `0x604140`, `[0x6041cb, 0x6041dc)`: the one-yard lift of the pet dismiss sound;
//! - `CGUnit_C::UpdateBreathState` (`0x607710`) whole, with the liquid query `0x670630` writing a
//!   chosen depth, the position getter and the line-of-sight leaf stubbed, reading bit 0x20 of
//!   `+0xc58`;
//! - `CGUnit_C::UpdateNamePlate` (`0x6086e0`) whole with a plate present, reading whether
//!   `0x608870` ran;
//! - `CGUnit_C::ShowChatBubble` (`0x608ac0`), `[0x608b74, 0x608ba5)`, read as `test ah,0x41`;
//! - `CGUnit_C::DrawSelectionCircle` (`0x608e00`), `[0x608e14, 0x608e6a)`, reading the four floats
//!   it builds;
//! - `CGUnit_C::ScaleQuestMarker` (`0x607570`) whole, with its gates and the vector source
//!   stubbed, reading the matrix `0x710620` receives.
//!
//! Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step either
//! side, to its random inputs.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::unitm1_fp::*;
use common::*;

const ATTACK_RANGE: u64 = 0x607710;
const NAMEPLATE_POS: u64 = 0x6086e0;
const GLOW: u64 = 0x607570;

#[test]
fn pet_dismiss_pos_lift_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x6041_4000u32;

    let case = |e: &mut Emu, z: f32, ctx: &str| {
        e.write_u32(ebp - 4, z.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20); // room for the `push -1`
        e.run(0x6041cb, 0x6041dc); // fld; push; fadd 1.0; lea; fstp
        let bin = e.read_f32(ebp - 4);
        assert_f32(bin, pet_dismiss_pos_lift(z), ctx);
    };

    case(&mut e, 0.0, "zero");
    case(&mut e, 5.5, "pos");
    case(&mut e, -42.25, "neg");
    case(&mut e, f32::INFINITY, "inf");
    for &v in &specials(&[]) {
        case(&mut e, v, "special");
    }
    for i in 0..3000 {
        let z = if i % 2 == 0 {
            frand(&mut s, -1e6, 1e6)
        } else {
            frand_bits(&mut s)
        };
        case(&mut e, z, &format!("fuzz{i}"));
    }
}

// `0x607710`, whole: the ground raycast range gate of `UpdateBreathState`. The raycast `0x670630` is
// stubbed to fill `[ebp-4]` with the distance and return 1, and the line-of-sight leaf `0x67e9c0` to
// return 0. The position virtual returns a pointer to a vec3 that is read at `[eax+8]`, so a stub
// of its vtable slot returns one. Bit 0 of `this+0xc58` is the result.
const GROUND_RAYCAST: u64 = 0x670630; // fills [ebp-4]=dist via arg0 write-through, returns 1
const LOS_LEAF: u64 = 0x67e9c0; // returns al, stubbed to 0

#[test]
fn attack_range_in_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000);
    let vt = e.alloc(0x100);
    let posvec = e.alloc(0x20); // the vtable getter's returned vec3 (eax)
    let distbuf = e.alloc(0x10); // the dist the raycast writes (payload for the write-through)
                                 // vtable slot +0x14 → a stub that returns eax = posvec (the vec3 ptr the body reads [eax+8]).
    let vtslot_stub = e.alloc(0x10);
    e.write(vtslot_stub, &[0xc3]); // ret (we set EAX via the stub hook below)
    e.write_u32(this, vt as u32);
    e.write_u32(vt + 0x14, vtslot_stub as u32);
    // The raycast 0x670630 writes the hit-distance to arg0 (= &[ebp-4]); stub write-through with our
    // distbuf payload, return 1 (hit). It is __thiscall with 3 stack-arg out-ptrs → ret 0xc (pop 3).
    e.stub_write_through(GROUND_RAYCAST, 0, distbuf, 4, 1, 3);
    // The vtable getter ([edx+0x14]) returns eax = posvec; it has 1 stack arg (the out-ptr). Stub it.
    e.stub(vtslot_stub, posvec as u32, 1);
    // The LoS leaf 0x67e9c0 → 0 (the je at 0x6077bf is taken; that path only writes +0xc18, no float).
    e.stub(LOS_LEAF, 0, 0);

    let in_bit = |e: &Emu| (e.read_u32(this + 0xc58) & 0x20) != 0;

    let case = |e: &mut Emu, dist: f32, vz: f32, base: f32, scale: f32, ctx: &str| {
        e.write_u32(distbuf, dist.to_bits());
        e.write_u32(posvec + 8, vz.to_bits());
        e.write_u32(this + 0x90, base.to_bits());
        e.write_u32(this + 0xbc, scale.to_bits());
        e.write_u32(this + 0xc58, 0xffff_ffff); // pre-set so the `and -0x61` clear is observable
        e.write_u32(this + 0xe0, 0); // arg the leaves don't use here
        e.clear_calls();
        let _ = e.call(ATTACK_RANGE, this as u32, 0, &[0x1000]); // nowTime arg
        assert_eq!(in_bit(e), attack_range_in(dist, vz, base, scale), "{ctx}");
    };

    // (5+base*scale) < (dist - vz) sets 0x20.
    case(&mut e, 100.0, 0.0, 1.0, 1.0, "far → in (5+1<100)");
    case(&mut e, 3.0, 0.0, 1.0, 1.0, "near → not (5+1>3)");
    case(&mut e, 7.0, 0.0, 1.0, 1.0, "boundary-ish");
    case(&mut e, f32::NAN, 0.0, 1.0, 1.0, "nan dist → not set");
    // `5.0 + reach == dist - vz` (6 against 6) and a step either side, then NaN, infinities and
    // zeros in each input.
    for d in [6.0f32, next_up(6.0), next_down(6.0)] {
        case(&mut e, d, 0.0, 1.0, 1.0, "threshold step");
    }
    for &v in &specials(&[6.0]) {
        case(&mut e, v, 0.0, 1.0, 1.0, "special dist");
        case(&mut e, 100.0, v, 1.0, 1.0, "special vz");
        case(&mut e, 100.0, 0.0, v, 1.0, "special base");
        case(&mut e, 100.0, 0.0, 1.0, v, "special scale");
    }
    let mut seen_set = false;
    let mut seen_unset = false;
    let mut s = 0x6077_1000u32;
    for i in 0..3000 {
        let dist = frand(&mut s, -2.0, 60.0);
        let vz = frand(&mut s, -10.0, 10.0);
        let base = frand(&mut s, 0.0, 8.0);
        let scale = frand(&mut s, 0.0, 3.0);
        e.write_u32(distbuf, dist.to_bits());
        e.write_u32(posvec + 8, vz.to_bits());
        e.write_u32(this + 0x90, base.to_bits());
        e.write_u32(this + 0xbc, scale.to_bits());
        e.write_u32(this + 0xc58, 0xffff_ffff);
        e.write_u32(this + 0xe0, 0);
        e.clear_calls();
        let _ = e.call(ATTACK_RANGE, this as u32, 0, &[0x1000]);
        let b = in_bit(&e);
        assert_eq!(b, attack_range_in(dist, vz, base, scale), "fuzz{i}");
        seen_set |= b;
        seen_unset |= !b;
    }
    assert!(seen_set, "0x20 (in-range) never set by fuzz");
    assert!(seen_unset, "0x20 never left unset by fuzz");
}

// `0x6086e0`, whole, with an emitter present (`this+0xe60 != 0`): when `dx² + dy²` of the new
// position against the old is not 0, or `[0xc4da34] & 2` is set, it overwrites `[node+0x508]` and
// `[node+0x50c]` and calls `0x608870`; the update shows as `[node+0x508]` taking the new value.
// `0x608870` (the sorted insert), `0x4840c0` and `0x7cbb40` (the `[0xc4da34] & 4` branch, left
// clear) are stubbed.
const NP_INSERT: u64 = 0x608870;
const NP_REFRESH: u64 = 0x4840c0;
const NP_FLAG4: u64 = 0x7cbb40;
const GLOBAL_C4DA34: u64 = 0xc4da34;

#[test]
fn nameplate_pos_changed_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000);
    let node = e.alloc(0x600); // the emitter node (this+0xe60); +0x508/+0x50c = old pos
    let newpos = e.alloc(0x10);
    e.write_u32(this + 0xe60, node as u32);
    e.stub(NP_INSERT, 0, 0); // __thiscall, no stack args — ONLY the UPDATE path calls it
    e.stub(NP_REFRESH, 0, 0);
    e.stub(NP_FLAG4, 0, 1); // the [0xc4da34]&4 branch (we keep bit2 clear so it never fires)

    // The UPDATE side effect is the (only) call to 0x608870; reading it (not the [node+0x508]
    // value) makes the new==old case unambiguous — the binary writes the same value either way, but
    // only the update path calls 0x608870.
    let case = |e: &mut Emu, old: [f32; 2], new: [f32; 2], flag2: bool, ctx: &str| -> bool {
        e.write_u32(node + 0x508, old[0].to_bits());
        e.write_u32(node + 0x50c, old[1].to_bits());
        e.write_u32(newpos, new[0].to_bits());
        e.write_u32(newpos + 4, new[1].to_bits());
        // [0xc4da34]: bit2 (0x4) clear (skip 0x7cbb40), bit1 (0x2) = flag2.
        e.write_u32(GLOBAL_C4DA34, if flag2 { 0x2 } else { 0x0 });
        e.clear_calls();
        let _ = e.call(NAMEPLATE_POS, this as u32, 0, &[newpos as u32, 0xdead]);
        let did_update = e.calls().iter().any(|c| c.addr == NP_INSERT);
        assert_eq!(did_update, nameplate_pos_changed(old, new, flag2), "{ctx}");
        did_update
    };

    // moved → update; same → no update; flag2 forces update even when same.
    assert!(case(&mut e, [1.0, 1.0], [2.0, 2.0], false, "moved"));
    assert!(!case(
        &mut e,
        [1.0, 1.0],
        [1.0, 1.0],
        false,
        "same → no update"
    ));
    assert!(case(
        &mut e,
        [1.0, 1.0],
        [1.0, 1.0],
        true,
        "same + flag2 → update"
    ));
    assert!(case(&mut e, [0.0, 0.0], [0.0, 1e-30], false, "tiny y move"));
    for &v in &specials(&[]) {
        case(&mut e, [1.0, 1.0], [v, 1.0], false, "special new x");
        case(&mut e, [1.0, 1.0], [1.0, v], false, "special new y");
        case(&mut e, [v, 1.0], [1.0, 1.0], false, "special old x");
        case(&mut e, [1.0, v], [1.0, 1.0], false, "special old y");
        case(&mut e, [v, v], [v, v], false, "special both");
        case(&mut e, [v, v], [v, v], true, "special both flag2");
    }
    let mut seen_upd = false;
    let mut seen_noupd = false;
    let mut s = 0x6086_e000u32;
    for i in 0..3000 {
        let old = [frand(&mut s, -100.0, 100.0), frand(&mut s, -100.0, 100.0)];
        // half the time, new == old (no-update path); half, a real move.
        let same = (next(&mut s) >> 8).is_multiple_of(2);
        let new = if same {
            old
        } else {
            [frand(&mut s, -100.0, 100.0), frand(&mut s, -100.0, 100.0)]
        };
        let flag2 = (next(&mut s) >> 8).is_multiple_of(3);
        let upd = case(&mut e, old, new, flag2, &format!("fuzz{i}"));
        seen_upd |= upd;
        seen_noupd |= !upd;
    }
    assert!(seen_upd, "update path never taken");
    assert!(seen_noupd, "no-update (same-pos) path never taken");
}

const TEST_AH_41: u8 = 0x41;

#[test]
fn bubble_in_range_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let avec = e.alloc(0x20);
    let bvec = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40; // the body's fstp st(0) noise touches no [ebp-x]; ebp set for safety
    let mut s = 0x608a_c000u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(avec + 4 * k as u64, a[k].to_bits());
            e.write_u32(bvec + 4 * k as u64, b[k].to_bits());
        }
        e.set_reg(RegisterX86::EAX, avec);
        e.set_reg(RegisterX86::EDI, bvec);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x608b74, 0x608ba5); // dist² compute + fcomp 400 + fnstsw + test ah,0x41
        let bin_ah = ah(e) & TEST_AH_41;
        let bin_bubble = bin_ah != 0; // je not taken ⟺ (ah&0x41)!=0 ⟺ bubble
        assert_eq!(
            bin_bubble,
            bubble_in_range(a, b),
            "{ctx}: ah&0x41={bin_ah:#04x}"
        );
        seen.insert(bin_ah);
    };

    // dist² < 400 (C0), > 400 (0), == 400 (C3 -> 0x40), and NaN (C0|C3 -> 0x41).
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "coincident → in (C0)",
    );
    case(&mut e, [100.0, 0.0, 0.0], [0.0, 0.0, 0.0], "far → out (0)");
    case(&mut e, [20.0, 0.0, 0.0], [0.0, 0.0, 0.0], "20 → ==400 (C3)");
    case(&mut e, [f32::NAN, 0.0, 0.0], [0.0, 0.0, 0.0], "nan → C0|C3");
    // `dist^2 == 400` and a step either side, then NaN, infinities and zeros in each coordinate.
    for x in [20.0f32, next_up(20.0), next_down(20.0)] {
        case(&mut e, [x, 0.0, 0.0], [0.0; 3], "threshold step");
    }
    for &v in &specials(&[20.0]) {
        for k in 0..3 {
            let mut a = [20.0f32, 0.0, 0.0];
            a[k] = v;
            case(&mut e, a, [0.0; 3], "special a");
            let mut b = [0.0f32; 3];
            b[k] = v;
            case(&mut e, [20.0, 0.0, 0.0], b, "special b");
        }
    }
    for i in 0..6000 {
        // Mix near-boundary (so both arms fire densely) and far.
        let scale = if (next(&mut s) >> 8).is_multiple_of(2) {
            22.0
        } else {
            5.0
        };
        let a = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        let b = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        case(&mut e, a, b, &format!("fuzz{i}"));
    }
    assert!(seen.contains(&0x00), "out-of-range (ah&0x41==0) never seen");
    assert!(seen.contains(&0x01), "in-range (C0) never seen");
}

#[test]
fn shadow_quad_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000);
    let desc = e.alloc(0x40);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    e.write_u32(this + 8, desc as u32);
    let mut s = 0x608e_0000u32;

    let case = |e: &mut Emu, scale: f32, m: f32, px: f32, py: f32, pz: f32, ctx: &str| {
        e.write_u32(this + 0xcf0, scale.to_bits());
        e.write_u32(desc + 0x10, m.to_bits());
        e.write_u32(ebp - 0x10, px.to_bits());
        e.write_u32(ebp - 0xc, py.to_bits());
        e.write_u32(ebp - 8, pz.to_bits());
        e.set_reg(RegisterX86::ESI, this); // the body uses esi (= ecx saved at entry) for this
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40); // room for the push at 0x608e45
        e.run(0x608e14, 0x608e6a);
        let bin = [
            e.read_f32(ebp - 0x34), // px - s
            e.read_f32(ebp - 0x30), // py - s
            e.read_f32(ebp - 0x2c), // pz - 2s
            e.read_f32(ebp - 0x1c), // s
        ];
        let ours = shadow_quad(scale, m, px, py, pz);
        for k in 0..4 {
            assert_f32(bin[k], ours[k], &format!("{ctx} [{k}]"));
        }
    };

    case(&mut e, 2.0, 3.0, 1.0, 2.0, 3.0, "simple");
    case(&mut e, 0.0, 0.0, 0.0, 0.0, 0.0, "zero");
    case(&mut e, -1.5, 4.0, 10.0, -10.0, 5.0, "neg scale");
    for &v in &specials(&[]) {
        case(&mut e, v, 2.0, 1.0, 2.0, 3.0, "special scale");
        case(&mut e, 2.0, v, 1.0, 2.0, 3.0, "special m");
        case(&mut e, 2.0, 3.0, v, 2.0, 3.0, "special px");
        case(&mut e, 2.0, 3.0, 1.0, v, 3.0, "special py");
        case(&mut e, 2.0, 3.0, 1.0, 2.0, v, "special pz");
    }
    for i in 0..3000 {
        let f = |s: &mut u32| {
            if next(s) >> 8 & 3 == 0 {
                frand_bits(s)
            } else {
                frand(s, -200.0, 200.0)
            }
        };
        let (scale, m, px, py, pz) = (f(&mut s), f(&mut s), f(&mut s), f(&mut s), f(&mut s));
        case(&mut e, scale, m, px, py, pz, &format!("fuzz{i}"));
    }
}

// `0x607570`, whole: the scale `1 / len` of a vector. The gates `0x712ed0` and `0x7103d0` are stubbed
// to return non-zero, the vector source `0x712de0` to write the vec3 through, and the consumer
// `0x710620` to record its matrix argument; the early exit (`len` 0, 1 or NaN) is `0x710620` not
// being called.
const GLOW_GATE_A: u64 = 0x712ed0; // [esi+0xb2c] visible? → nonzero
const GLOW_GATE_B: u64 = 0x7103d0; // [esi+0xdc]/[esi+0xd8] sub-objects → nonzero (2 stack args)
const GLOW_VEC_SRC: u64 = 0x712de0; // fills the vec3 (write-through), 2 stack args
const GLOW_HANDLE: u64 = 0x712ee0; // returns a handle (passed to 0x712de0); no stack args
const GLOW_CONSUME: u64 = 0x710620; // (this.glow, &matrix) — record args[0]

#[test]
fn glow_inv_len_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000);
    let vecbuf = e.alloc(0x90); // the 0x712de0 write-through payload (vec3 + slack)
    e.write_u32(this + 0xb2c, 0x100); // model present
    e.write_u32(this + 0xdc, 0x200); // sub-object present
    e.write_u32(this + 0xd8, 0x300); // sub-object present
    e.stub(GLOW_GATE_A, 1, 0);
    e.stub(GLOW_GATE_B, 1, 2);
    e.stub(GLOW_HANDLE, 0x55, 0);
    // 0x712de0(ecx=edi=[esi+0xd8], stack0=&vec, stack1=handle): write the vec3 (layout v0,v1,v2),
    // 2 stack args → ret 8.
    e.stub_write_through(GLOW_VEC_SRC, 0, vecbuf, 12, 0, 2);
    // 0x710620(ecx=[esi+0xb2c], stack0=&matrix): record args[0]; 1 stack arg → ret 4.
    e.stub(GLOW_CONSUME, 0, 1);

    let case = |e: &mut Emu, v0: f32, v1: f32, v2: f32, ctx: &str| {
        e.write_u32(vecbuf, v0.to_bits());
        e.write_u32(vecbuf + 4, v1.to_bits());
        e.write_u32(vecbuf + 8, v2.to_bits());
        e.clear_calls();
        let _ = e.call(GLOW, this as u32, 0, &[]);
        let consumed = e.calls().iter().find(|c| c.addr == GLOW_CONSUME).copied();
        let ours = glow_inv_len(v0, v1, v2);
        match (consumed, ours) {
            (None, None) => {} // early-out both
            (Some(c), Some(inv)) => {
                let mat = u64::from(c.args[0]);
                let bin_inv = e.read_f32(mat); // [ebp-0x44] = matrix+0 = the inv scale
                assert_f32(bin_inv, inv, ctx);
            }
            (got, exp) => panic!(
                "{ctx}: early-out mismatch: binary 0x710620 called={} ours={:?}",
                got.is_some(),
                exp.map(|_| "Some")
            ),
        }
    };

    // unit length (v=(1,0,0)) → len==1.0 → early out (None).
    case(&mut e, 0.0, 1.0, 0.0, "len==1 → early-out");
    // zero vector → len==0 → early out.
    case(&mut e, 0.0, 0.0, 0.0, "len==0 → early-out");
    // a normalizable vector.
    case(&mut e, 0.0, 3.0, 4.0, "len==5 → 1/5");
    case(&mut e, 2.0, 3.0, 6.0, "len==7 → 1/7");
    // A length of 1 reached by two components, the zero vectors, and NaN, infinities and the
    // thresholds in each component.
    case(&mut e, 0.6, 0.8, 0.0, "len 1 from two");
    case(&mut e, -0.0, -0.0, -0.0, "negative zeros");
    for &v in &specials(&[1.0]) {
        case(&mut e, v, 3.0, 4.0, "special v0");
        case(&mut e, 0.0, v, 0.0, "special v1");
        case(&mut e, 0.0, 0.0, v, "special v2");
        case(&mut e, v, v, v, "special all");
    }
    let mut seen_norm = false;
    let mut seen_early = false;
    let mut s = 0x6075_7000u32;
    for i in 0..3000 {
        let kind = (next(&mut s) >> 8) % 4;
        let (v0, v1, v2) = match kind {
            0 => (0.0, 0.0, 0.0),                     // len 0 (early)
            1 => (frand(&mut s, 0.0, 1.0), 0.0, 0.0), // possibly len 1 (the v0=1 boundary)
            _ => (
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
            ),
        };
        e.write_u32(vecbuf, v0.to_bits());
        e.write_u32(vecbuf + 4, v1.to_bits());
        e.write_u32(vecbuf + 8, v2.to_bits());
        e.clear_calls();
        let _ = e.call(GLOW, this as u32, 0, &[]);
        let consumed = e.calls().iter().find(|c| c.addr == GLOW_CONSUME).copied();
        let ours = glow_inv_len(v0, v1, v2);
        match (consumed, ours) {
            (None, None) => seen_early = true,
            (Some(c), Some(inv)) => {
                seen_norm = true;
                let bin_inv = e.read_f32(u64::from(c.args[0]));
                assert_f32(bin_inv, inv, &format!("fuzz{i}"));
            }
            (g, ex) => panic!(
                "fuzz{i}: branch mismatch binary-called={} ours={:?}",
                g.is_some(),
                ex.map(|_| "Some")
            ),
        }
    }
    // deterministic probe for the len==1 early-out (the fuzz rarely lands exactly on it).
    case(&mut e, 1.0, 0.0, 0.0, "probe len==1");
    // ≠ 1.0 (binary proceeds to normalize) but f32(sqrt) == 1.0 (a prior `len as f32 == 1.0`
    // early-outed). Lands the exact boundary `0x607614 fcomp [1.0]` reads on the live f64.
    case(
        &mut e,
        0.0,
        1.0,
        f32::from_bits(0x3900_0000),
        "fst-no-pop ½ulp: len_live>1 but f32(len)==1 → normalize, not early-out",
    );
    assert!(seen_norm, "normalize path never taken");
    assert!(seen_early, "early-out path never taken");
}
