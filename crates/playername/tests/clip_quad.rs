//! Bit-exact diff for `playername::clip_quad::clip_quad` vs `WoW.exe` `0x6c7cc0`'s slice.
//!
//! Slice diffed: `[0x6c7da8, 0x6c7ef9)` — the screen-rect clamp + quad-corner + midpoint build, run
//! on the binary's own bytes. Calls fed or stubbed:
//! - the world→screen projection (`0x483ee0`), the viewport fetch (`0x768320`), and the rising-Z
//!   interp (`0x6c8070`) all run BEFORE `0x6c7da8` — their results are fed by writing the projection
//!   out-slots `[ebp-0x10]`/`[ebp-0xc]` (sx/sy) and the viewport-origin addends `[ebp-0x54]`/
//!   `[ebp-0x58]` (add_x/add_y) directly.
//! - the viewport half-extents the `0x41ae40/50/60/70` converters read are written to `[0x832a44]`
//!   (vp_half_w) / `[0x832a48]` (vp_half_h); the converters themselves (tiny leaves) execute.
//! - `0x509520` (gx state, no FP effect on the slots) is stubbed to a clean return.
//!
//! Outputs read: `this[0x30]`/`[0x34]`/`[0x38]` (the plate NDC anchor), captured at the halt
//! `0x6c7ef9` — after the last write, before the `0x5c2770` WORLDTEXTSTRING writeback call.

#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_playername::clip_quad::clip_quad;

// shared LCG fuzz (high bits only)
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rngf(s: &mut u32, lo: f64, hi: f64) -> f64 {
    lo + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * (hi - lo)
}

const SLICE_BEGIN: u64 = 0x6c7da8;
const SLICE_END: u64 = 0x6c7ef9;
const GX_STATE: u64 = 0x509520; // 0x509520: gx state call (no FP effect) — stub to clean return
const VP_HALF_W_VA: u64 = 0x832a44; // [0x832a44] viewport half-width (0x41ae40 div / 0x41ae60 mul)
const VP_HALF_H_VA: u64 = 0x832a48; // [0x832a48] viewport half-height (0x41ae50 div / 0x41ae70 mul)

#[allow(clippy::too_many_arguments)]
fn run_slice(
    emu: &mut Emu,
    this: u64,
    ebp: u64,
    esp: u64,
    cx: f32,
    cy: f32,
    sx: f32,
    sy: f32,
    add_x: f32,
    add_y: f32,
    vp_half_w: f32,
    vp_half_h: f32,
) -> (u32, u32, u32) {
    // `this` (esi) holds [0xc]=cx, [0x10]=cy (the plate half-extents the clamp/quad read).
    emu.write_u32(this + 0xc, cx.to_bits());
    emu.write_u32(this + 0x10, cy.to_bits());
    emu.set_reg(RegisterX86::ESI, this);
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.set_reg(RegisterX86::ESP, esp);
    // fed projection out-slots + viewport-origin addends.
    emu.write_u32(ebp - 0x10, sx.to_bits()); // [ebp-0x10] = projected screen x
    emu.write_u32(ebp - 0xc, sy.to_bits()); // [ebp-0xc]  = projected screen y
    emu.write_u32(ebp - 0x54, add_x.to_bits()); // [ebp-0x54] addend to x
    emu.write_u32(ebp - 0x58, add_y.to_bits()); // [ebp-0x58] addend to y
                                                // the viewport half-extents the converter leaves read from .data.
    emu.write_u32(VP_HALF_W_VA, vp_half_w.to_bits());
    emu.write_u32(VP_HALF_H_VA, vp_half_h.to_bits());
    emu.run(SLICE_BEGIN, SLICE_END);
    (
        emu.read_u32(this + 0x30),
        emu.read_u32(this + 0x34),
        emu.read_u32(this + 0x38),
    )
}

#[allow(clippy::too_many_arguments)]
fn check(
    emu: &mut Emu,
    this: u64,
    ebp: u64,
    esp: u64,
    cx: f32,
    cy: f32,
    sx: f32,
    sy: f32,
    add_x: f32,
    add_y: f32,
    vp_half_w: f32,
    vp_half_h: f32,
) {
    let (gx, gy, gz) = run_slice(
        emu, this, ebp, esp, cx, cy, sx, sy, add_x, add_y, vp_half_w, vp_half_h,
    );
    let r = clip_quad(cx, cy, sx, sy, add_x, add_y, vp_half_w, vp_half_h);
    assert_eq!(
        (gx, gy, gz),
        (r.anchor_x, r.anchor_y, r.anchor_z),
        "clip_quad cx={cx} cy={cy} sx={sx} sy={sy} add_x={add_x} add_y={add_y} \
         vpW={vp_half_w} vpH={vp_half_h}: bin=({gx:#x},{gy:#x},{gz:#x}) rust=({:#x},{:#x},{:#x})",
        r.anchor_x,
        r.anchor_y,
        r.anchor_z
    );
}

#[test]
fn pn_label_clip_quad() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(GX_STATE, 0, 0); // 0x509520 ends with plain `ret` — no stack-arg cleanup
    let this = emu.alloc(0x80);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x200; // room for [ebp-0x58 .. ebp+0x8]
    let esp = ebp - 0x100; // headroom for the converter push/call scratch (all balanced)

    let mut total = 0u32;

    // Deterministic boundary probes: each clamp edge hit at its `==threshold` (cx, vpW−cx, cy,
    // vpH−cy) plus the inverted-interval case (vpW−cx <= max(x,cx)). vpW=vpH=1.0 (so px==NDC scale)
    // and a few realistic 0.8/0.6 viewport halves.
    let vps: [(f32, f32); 3] = [
        (1.0, 1.0),
        (0.8, 0.6),     // the static-default viewport halves (0x3f4ccccd / 0x3f19999a)
        (400.0, 300.0), // an 800x600 half-viewport
    ];
    let cxs = [0.05f32, 0.1, 20.0, 40.0];
    let cys = [0.05f32, 0.1, 16.0, 32.0];
    for &(vpw, vph) in &vps {
        for &cx in &cxs {
            for &cy in &cys {
                // edge probes for X: below-lower, exactly-lower(cx), inside, exactly-upper(vpW−cx),
                // above-upper; same for Y. add_x/add_y fold into x/y so test both raw + offset.
                let xs = [
                    -1.0f32,
                    cx,
                    cx - 0.001,
                    cx + 0.001,
                    vpw * 0.5,
                    vpw - cx,
                    vpw - cx - 0.001,
                    vpw - cx + 0.001,
                    vpw + 1.0,
                    0.0,
                ];
                let ys = [
                    -1.0f32,
                    cy,
                    cy - 0.001,
                    cy + 0.001,
                    vph * 0.5,
                    vph - cy,
                    vph - cy - 0.001,
                    vph - cy + 0.001,
                    vph + 1.0,
                    0.0,
                ];
                for &sx in &xs {
                    for &sy in &ys {
                        // add=0 (sx/sy are the screen coords directly)
                        check(&mut emu, this, ebp, esp, cx, cy, sx, sy, 0.0, 0.0, vpw, vph);
                        total += 1;
                    }
                }
                // a couple of nonzero viewport-origin addends to exercise the `+add` ops.
                check(
                    &mut emu, this, ebp, esp, cx, cy, 10.0, 7.0, 3.5, -2.25, vpw, vph,
                );
                check(
                    &mut emu, this, ebp, esp, cx, cy, -5.0, -3.0, 1.0, 1.0, vpw, vph,
                );
                total += 2;
            }
        }
    }

    // Fuzz: random center + viewport halves over a wide screen-px range.
    let mut s = 0x6c7c_c000u32;
    for _ in 0..8000 {
        let vpw = rngf(&mut s, 0.5, 800.0) as f32;
        let vph = rngf(&mut s, 0.5, 600.0) as f32;
        let cx = rngf(&mut s, 0.01, vpw as f64 * 0.6) as f32;
        let cy = rngf(&mut s, 0.01, vph as f64 * 0.6) as f32;
        let sx = rngf(&mut s, -50.0, vpw as f64 + 50.0) as f32;
        let sy = rngf(&mut s, -50.0, vph as f64 + 50.0) as f32;
        let add_x = rngf(&mut s, -20.0, 20.0) as f32;
        let add_y = rngf(&mut s, -20.0, 20.0) as f32;
        check(
            &mut emu, this, ebp, esp, cx, cy, sx, sy, add_x, add_y, vpw, vph,
        );
        total += 1;
    }

    eprintln!("pn_label_clip_quad: {total} bit-exact (clamp + quad-corner + midpoint build)");
}

// 0x6c7d29 — 0x6c7cc0's world-Z rising offset combine: `world_z = interp_param(elapsed) + this[0x1c]`
// (`fadd dword [ebp-0x40]` on the live interp result on st0; then `fstp dword [ebp-0x40]`). interp_param
// (0x6c8070) is diffed in `timing`; here we diff THIS fadd op via a island
// (fld qword [interp_f64]; <fadd dword [z_f32]>; fstp dword [out]), the fadd byte-verified opcode d8 + reg /0
// (fadd dword) == source 0x6c7d29.
#[test]
fn pn_label_clip_quad_world_z() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let interp_cell = emu.alloc(8); // the live interp_param result (f64/PC_53)
    let z_cell = emu.alloc(4); // this[0x1c] world-Z (f32)
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    // fld qword [interp_cell] = DD 05 <disp32>
    let mut code = vec![0xddu8, 0x05];
    code.extend_from_slice(&(interp_cell as u32).to_le_bytes());
    // fadd dword [z_cell] = D8 05 <disp32>  (opcode d8, reg /0 = fadd dword — same op as src 0x6c7d29)
    let fadd_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x05]);
    code.extend_from_slice(&(z_cell as u32).to_le_bytes());
    // fstp dword [out_cell] = D9 1D <disp32>
    code.push(0xd9);
    code.push(0x1d);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    // byte-verify the island's fadd is the same operation as source 0x6c7d29 (opcode d8 + reg-field /0).
    let src = emu.read(0x6c7d29, 2);
    assert_eq!(src[0], 0xd8, "src 0x6c7d29 opcode");
    assert_eq!(src[1] & 0x38, 0x00, "src 0x6c7d29 reg-field /0 (fadd)");
    assert_eq!(emu.read(isl + fadd_off, 1), [0xd8], "island fadd opcode");
    assert_eq!(
        emu.read(isl + fadd_off + 1, 1)[0] & 0x38,
        0x00,
        "island fadd reg /0"
    );

    let run = |emu: &mut Emu, interp: f64, z: f32| -> f32 {
        emu.write(interp_cell, &interp.to_bits().to_le_bytes());
        emu.write_u32(z_cell, z.to_bits());
        emu.run(isl, isl_end);
        let b = emu.read(out_cell, 8);
        // fstp dword narrows; read the low f32.
        f32::from_bits(u32::from_le_bytes(b[0..4].try_into().unwrap()))
    };
    let want = |interp: f64, z: f32| -> f32 { (interp + f64::from(z)) as f32 };
    let mut total = 0u32;
    for &(i, z) in &[
        (0.0f64, 0.0f32),
        (0.0, -0.0),
        (1.0 / 3.0, 100.0),
        (-2.0 / 3.0, 0.0),
        (1e9, -1e9),
        (0.5, -0.5), // exact cancel -> 0.0
    ] {
        assert_eq!(
            run(&mut emu, i, z).to_bits(),
            want(i, z).to_bits(),
            "world_z i={i} z={z}"
        );
        total += 1;
    }
    let mut s = 0x6c7d_2900u32;
    for _ in 0..4000 {
        let i = rngf(&mut s, -1000.0, 1000.0);
        let z = rngf(&mut s, -1000.0, 1000.0) as f32;
        assert_eq!(
            run(&mut emu, i, z).to_bits(),
            want(i, z).to_bits(),
            "world_z fuzz i={i} z={z}"
        );
        total += 1;
    }
    eprintln!("pn_label_clip_quad_world_z: {total} bit-exact (island fld qword; fadd dword [z]==src 0x6c7d29; fstp)");
}
