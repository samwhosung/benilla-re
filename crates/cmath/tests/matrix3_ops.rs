//! The 3×3 determinant `0x7bc040`, which takes the matrix by value on the stack, and the row
//! scales `0x7be6d0` and `0x7be670`, each run whole. The inputs are the edge fills, a mix, and
//! random non-NaN bit patterns. A NaN result matches any NaN.

mod common;

use benilla_re_cmath::matrix3 as m3;
use common::*;

#[test]
fn det3() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mats = synth_n(9);
    for m in &mats {
        let bits: Vec<u32> = m.iter().map(|x| x.to_bits()).collect();
        let got = d.emu.call_f32(0x7bc040, 0, 0, &bits); // 3×3 matrix by value on the stack
        assert_f32(got, m3::det3(arr(m.as_slice())), "det3");
    }
    eprintln!("det3: {} cases bit-exact vs WoW.exe @ 0x7bc040", mats.len());
}

#[test]
fn mat3_scale_rows() {
    check_vecscale_inplace("mat3_scale_rows", &[0x7be6d0], 9, 3, |m, v| {
        m3::mat3_scale_rows(arr(m), arr(v)).to_vec()
    });
}

#[test]
fn mat3_scale_rows3() {
    // 3 by-value scalars on the stack (ret 0xc); in-place read-back.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mats = synth_n(9);
    let sc = synth_scalars();
    for (k, m) in mats.iter().enumerate() {
        let s = [
            sc[k % sc.len()],
            sc[(k + 3) % sc.len()],
            sc[(k + 7) % sc.len()],
        ];
        d.put(d.a, m);
        d.emu.call(
            0x7be670,
            d.a as u32,
            0,
            &[s[0].to_bits(), s[1].to_bits(), s[2].to_bits()],
        );
        assert_slice(
            &d.read_n(d.a, 9),
            &m3::mat3_scale_rows3(arr(m.as_slice()), &s),
            "mat3_scale_rows3",
        );
    }
    eprintln!(
        "mat3_scale_rows3: {} cases bit-exact vs WoW.exe @ 0x7be670",
        mats.len()
    );
}
