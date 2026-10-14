//! The taxi-route geometry against the exe: the endpoint quads (`0x407900`) run whole, the segment
//! count as a slice of `0x4075a0`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_loadingscreen::route::{route_endpoint_quads, route_tessellate_segs};
use common::{assert_f32, fuzz, next};

const ROUTE_ENDPOINT_QUADS: u64 = 0x407900; // __fastcall(a* ecx, b* edx)
const VBUF_PTR: u64 = 0x0088_28e8; // DYNAMICELEMENTVERT* (route vertex buffer)
const COLOR: u64 = 0x0088_28a4; // u32 vertex tint

#[test]
fn route_tessellate_segs_difftest() {
    // The slice `0x4077f4` to `0x407814` of `0x4075a0`: the spline arc length goes in at [ebp-0x54]
    // and `ebx` is 1 (the max operand); it runs through `__ftol` and the max, and segs is read at
    // [ebp-0x4].
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s: u32 = 0x4075_0001;
    // Arc lengths whose product with 66.6667 is NaN, infinite, past `i32` or past `i64`: `__ftol`
    // returns the low dword of its qword, 0 for the indefinite, and the max raises it to 1.
    const WIDE: [f32; 9] = [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.0e30,
        -1.0e30,
        1.0e8,
        1.0e17,
        -1.0e8,
        6.4e7,
    ];
    for i in 0..6000u32 + WIDE.len() as u32 {
        // arc length >= 0, in a range that keeps arclen * 66.6667 in i32
        let mut arclen = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 4000.0) as f32;
        if let Some(&a) = WIDE.get((i as usize).wrapping_sub(6000)) {
            arclen = a;
        }
        e.write_u32(ebp - 0x54, arclen.to_bits());
        e.set_reg(RegisterX86::EBX, 1);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.set_reg(RegisterX86::FPCW, 0x027F);
        e.run(0x4077f4, 0x407814);
        let bin = e.read_u32(ebp - 0x4) as i32;
        assert_eq!(
            bin,
            route_tessellate_segs(arclen),
            "route_tessellate_segs[{i}] arclen={arclen}"
        );
    }
}

#[test]
fn route_endpoint_quads_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let buf = e.alloc(0x100); // 8 verts × 6 dwords = 0xc0
    let pa = e.alloc(0x10);
    let pb = e.alloc(0x10);
    e.write_u32(VBUF_PTR, buf as u32);
    let mut s: u32 = 0x4079_0001;
    for i in 0..4000u32 {
        let a = [fuzz(&mut s, 1.0), fuzz(&mut s, 1.0)];
        let b = [fuzz(&mut s, 1.0), fuzz(&mut s, 1.0)];
        let color = next(&mut s);
        e.write_u32(pa, a[0].to_bits());
        e.write_u32(pa + 4, a[1].to_bits());
        e.write_u32(pb, b[0].to_bits());
        e.write_u32(pb + 4, b[1].to_bits());
        e.write_u32(COLOR, color);
        e.write(buf, &[0u8; 0xc0]);
        e.call(ROUTE_ENDPOINT_QUADS, pa as u32, pb as u32, &[]);
        let ours = route_endpoint_quads(a, b);
        for (vi, v) in ours.iter().enumerate() {
            let base = buf + (vi * 6) as u64 * 4;
            for (k, &o) in v.iter().enumerate() {
                assert_f32(
                    e.read_f32(base + 4 * k as u64),
                    o,
                    &format!("endpoint[{i}] v{vi}[{k}]"),
                );
            }
            assert_eq!(e.read_u32(base + 20), color, "endpoint[{i}] v{vi} color");
        }
    }
}
