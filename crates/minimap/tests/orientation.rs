//! The orientation renormalize (`0x6da180`) against the exe, whole function, with the two bracketing
//! calls stubbed (`0x670570` derives the basis, `0x7bdd60` applies the facing rotation); both
//! branches, the normalize and the already-unit skip.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_minimap::orientation::orientation_renormalize;
use common::{assert_f32, fuzz, next};

const ORIENT_RENORM: u64 = 0x6da180; // void(void)
const BASIS_DERIVE: u64 = 0x670570; // edx=0xceaa20, push 0xcea9e0 — 1 stack dword
const FACING_ROTATE: u64 = 0x7bdd60; // thiscall, 3 stack dwords
const A20: u64 = 0x00ce_aa20; // row0
const A30: u64 = 0x00ce_aa30; // row1
const A40: u64 = 0x00ce_aa40; // row2
const A50: u64 = 0x00ce_aa50; // translation row (zeroed)

const ROW: [u64; 3] = [A20, A30, A40];

fn set_basis(e: &mut Emu, b: &[f32; 9]) {
    for (r, &base) in ROW.iter().enumerate() {
        for c in 0..3 {
            e.write_u32(base + 4 * c as u64, b[r * 3 + c].to_bits());
        }
    }
}
fn read_basis(e: &Emu) -> [f32; 9] {
    let mut b = [0f32; 9];
    for (r, &base) in ROW.iter().enumerate() {
        for c in 0..3 {
            b[r * 3 + c] = e.read_f32(base + 4 * c as u64);
        }
    }
    b
}

fn run_case(e: &mut Emu, b: [f32; 9], ctx: &str) {
    set_basis(e, &b);
    // dirty the translation row so we observe the zeroing
    for k in 0..3 {
        e.write_u32(A50 + 4 * k, 0xdead_beef);
    }
    e.call(ORIENT_RENORM, 0, 0, &[]);
    let bin = read_basis(e);
    let ours = orientation_renormalize(b);
    for k in 0..9 {
        assert_f32(bin[k], ours[k], &format!("{ctx} basis[{k}]"));
    }
    for k in 0..3 {
        assert_eq!(e.read_u32(A50 + 4 * k), 0, "{ctx} translation[{k}] zeroed");
    }
}

#[test]
fn orientation_renormalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(BASIS_DERIVE, 0, 1);
    e.stub(FACING_ROTATE, 0, 3);
    let mut s: u32 = 0x6da1_0001;
    for i in 0..4000u32 {
        // far-from-unit bases → the normalize branch
        let b = [
            fuzz(&mut s, 2.0),
            fuzz(&mut s, 2.0),
            fuzz(&mut s, 2.0),
            fuzz(&mut s, 2.0),
            fuzz(&mut s, 2.0),
            fuzz(&mut s, 2.0),
            fuzz(&mut s, 2.0),
            fuzz(&mut s, 2.0),
            fuzz(&mut s, 2.0),
        ];
        run_case(&mut e, b, &format!("renorm[{i}]"));
        let _ = next(&mut s);
    }
    // skip branch: row0 already unit (|s0−1| < eps) → basis unchanged
    run_case(
        &mut e,
        [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        "skip.identity",
    );
    run_case(
        &mut e,
        [0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        "skip.unit-row0",
    );
}
