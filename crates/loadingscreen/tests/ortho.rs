//! The letterbox rect against the exe: the slice `0x406ac4` to `0x406b3d` of `0x406a60`, run in a
//! set-up `ebp` frame. The four letterbox locals start at their defaults, the aspect global is set,
//! and the six arguments pushed for the `0x58af60` call are read off the stack.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_loadingscreen::ortho::aspect_fit_ortho;
use common::{assert_f32, fuzz, next};

const SLICE_BEGIN: u64 = 0x406ac4; // call 0x41ad70 (aspect getter = fld [0x832a4c])
const SLICE_END: u64 = 0x406b3d; // call 0x58af60 (ortho-set) — args on the stack
const ASPECT: u64 = 0x0083_2a4c; // f32 ScreenW/H

#[test]
fn aspect_fit_ortho_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s: u32 = 0x406a_0001;
    for i in 0..6000u32 {
        // aspect in [0.5, 2.0): covers a > 1 (pillarbox), a < 1 (letterbox), near a == 1
        let aspect = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 1.5 + 0.5) as f32;
        let _ = fuzz(&mut s, 1.0);
        e.write_u32(ASPECT, aspect.to_bits());
        // defaults: [ebp-4]=w=1, [ebp-8]=xlo=0, [ebp-c]=ylo=0, [ebp-0x10]=h=1
        e.write_u32(ebp - 0x4, 1.0f32.to_bits());
        e.write_u32(ebp - 0x8, 0);
        e.write_u32(ebp - 0xc, 0);
        e.write_u32(ebp - 0x10, 1.0f32.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.set_reg(RegisterX86::FPCW, 0x027F);
        e.run(SLICE_BEGIN, SLICE_END);
        let esp = e.get_reg(RegisterX86::ESP);
        let bin: Vec<f32> = (0..6).map(|k| e.read_f32(esp + 4 * k)).collect();
        let ours = aspect_fit_ortho(aspect);
        for k in 0..6 {
            assert_f32(
                bin[k],
                ours[k],
                &format!("aspect_fit_ortho[{i}] a={aspect} arg{k}"),
            );
        }
    }
}
