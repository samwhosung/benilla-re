//! Differential tests of the arithmetic of the minimap ping functions `0x4eaf00`,
//! `MinimapApplyPing` (`0x4ee330`) and `PingLocation` (`0x4eeca0`), each run against `WoW.exe` in
//! the emulator.
//!
//! Each owned FP slice is diffed either as a direct **instruction slice** (`Emu::run` over just the
//! owned arith, frame/object cells synthesized) or — where the source interleaves delegated st0
//! producers (the frame-timer `0x6da9b0`, the Lua getters `0x6f3620`, the `0.8·x` helper
//! `0x41ae60`, the camera vtable getters) we can't faithfully drive in isolation — as a
//! **copied-byte island**: the EXACT source FP-op bytes (each byte-verified `==` its real source
//! address below, so the diff is never circular), with a `fld qword [cell]` prepended to inject
//! the delegated st0 result as a synthesized `f64` input and a `fstp [cell]` appended to capture
//! the store. Every in-body arithmetic opcode (the arithmetic opcodes) lands in a diffed slice; `fld`/`fst`/
//! `fstp`/`fxch`/`fild` and the `fcomp` clamp compares (NOT arith) ride along / are excluded.
//!
//! No `==`-boundary probe is emitted: NO `fcomp`/`fcom`/`comiss` lands inside any diffed slice
//! (0x4eaf00's clamp compares are downstream of its owned arith; the other two carry none), so
//! there is no constant-vs-value branch whose rounding an owned-arith result gates.
//!
//! Fuzz reads the LCG's HIGH bits.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::minimap_ping::{
    bbox_edges, delta_sub, ping_double, proj_add_offset, proj_bias8, proj_div_scale, proj_negate,
    proj_over_unit, scale_project,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}
fn rng64(s: &mut u32, lo: f64, hi: f64) -> f64 {
    lo + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * (hi - lo)
}

struct Frame {
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x80; // room for [ebp-0x50]..[ebp+0x20]
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { ebp, esp }
}

// ===========================================================================================
// 0x4eaf00 — 4 edge values for a bbox clamp: `fld [G]; f{sub,add} [w]; fstp [m32]` ×4. Diffed as
// a direct slice 0x4eaf24..0x4eaf4e (no object-graph deref in the owned arith — only frame args).
// fsub ×2 (0x4eaf2a/36) + fadd ×2 (0x4eaf3f/48). PRIMARY: glue_4ea_4eaf00_bbox_edges.
// ===========================================================================================
#[test]
fn glue_4ea_4eaf00_bbox_edges() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, x: f32, y: f32, w: f32| -> (f32, f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x10, x.to_bits());
        emu.write_u32(f.ebp + 0xc, y.to_bits());
        emu.write_u32(f.ebp + 0x14, w.to_bits());
        emu.run(0x4eaf24, 0x4eaf4e); // fld;fsub;fstp ×2 then fld;fadd;fstp ×2 — halt at fld [bbox]
        (
            emu.read_f32(f.ebp - 0x14), // x - w
            emu.read_f32(f.ebp - 0x10), // y - w
            emu.read_f32(f.ebp - 0xc),  // x + w
            emu.read_f32(f.ebp - 0x8),  // y + w
        )
    };

    let mut total = 0u32;
    let probes = [
        0.0f32, 1.0, -1.0, 0.5, 1024.0, -1024.0, 1.0e6, -7.5, 0.031, 3.4e38,
    ];
    for &x in &probes {
        for &y in &probes {
            for &w in &probes {
                let got = run(&mut emu, x, y, w);
                let want = bbox_edges(x, y, w);
                assert_eq!(got.0.to_bits(), want.0.to_bits(), "x-w x={x} y={y} w={w}");
                assert_eq!(got.1.to_bits(), want.1.to_bits(), "y-w x={x} y={y} w={w}");
                assert_eq!(got.2.to_bits(), want.2.to_bits(), "x+w x={x} y={y} w={w}");
                assert_eq!(got.3.to_bits(), want.3.to_bits(), "y+w x={x} y={y} w={w}");
                total += 4;
            }
        }
    }
    let mut s = 0x4eaf_0000u32;
    for _ in 0..3000 {
        let x = rng(&mut s, -1.0e7, 1.0e7);
        let y = rng(&mut s, -1.0e7, 1.0e7);
        let w = rng(&mut s, -5.0e6, 5.0e6);
        let got = run(&mut emu, x, y, w);
        let want = bbox_edges(x, y, w);
        assert_eq!(
            got.0.to_bits(),
            want.0.to_bits(),
            "fuzz x-w x={x} y={y} w={w}"
        );
        assert_eq!(
            got.1.to_bits(),
            want.1.to_bits(),
            "fuzz y-w x={x} y={y} w={w}"
        );
        assert_eq!(
            got.2.to_bits(),
            want.2.to_bits(),
            "fuzz x+w x={x} y={y} w={w}"
        );
        assert_eq!(
            got.3.to_bits(),
            want.3.to_bits(),
            "fuzz y+w x={x} y={y} w={w}"
        );
        total += 4;
    }
    eprintln!("glue_4ea_4eaf00_bbox_edges: {total} bit-exact (fsub×2 + fadd×2)");
}

// ===========================================================================================
// 0x4ee330 — screen-delta debug log. Owned arith in two parts:
//  * delta_sub: direct slice 0x4ee43a..0x4ee456 (set EBX=input vec, EAX=pos vec). fsub ×2.
//  * scale_project: COPIED-BYTE ISLAND of verbatim 0x4ee45b..0x4ee482 (fadd st,st; fdivr [1.0];
//    fmul st,st(1); fxch; fmul [ebp-4]; the two qword stores; fld;fchs), prepended with
//    `fld qword [Rcell]` to inject the frame-timer st0 (f64). fadd ×1 + fdivr ×1 + fmul ×2 +
//    fchs ×1. The verbatim 39 bytes are byte-verified == source. PRIMARY: glue_4ea_4ee330_scale.
// ===========================================================================================
#[test]
fn glue_4ea_4ee330_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let invec = emu.alloc(0x10); // [ebx]/[ebx+4] = bx,by
    let posvec = emu.alloc(0x10); // [eax]/[eax+4] = px,py

    // ---- delta_sub: slice 0x4ee43a..0x4ee456 (two fsub) -----------------------------------
    let run_delta = |emu: &mut Emu, bx: f32, by: f32, px: f32, py: f32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EBX, invec);
        emu.set_reg(RegisterX86::EAX, posvec);
        emu.write_u32(invec, bx.to_bits());
        emu.write_u32(invec + 4, by.to_bits());
        emu.write_u32(posvec, px.to_bits());
        emu.write_u32(posvec + 4, py.to_bits());
        emu.run(0x4ee43a, 0x4ee456); // fld[ebx];fsub;fstp[ebp-8] ; fld[ebx+4];fsub;fstp[ebp-4]
        (emu.read_f32(f.ebp - 0x8), emu.read_f32(f.ebp - 0x4))
    };

    // ---- scale_project: copied-byte island ------------------------------------------------
    // prepend `fld qword [Rcell]` (DD 05 + disp32) then verbatim 0x4ee45b..0x4ee482.
    let rcell = emu.alloc(8);
    let isl = emu.alloc(0x40);
    let mut code = vec![0xddu8, 0x05];
    code.extend_from_slice(&(rcell as u32).to_le_bytes());
    let mid_off = code.len();
    let src_mid: [u8; 39] = [
        0xdc, 0xc0, // fadd st(0),st            (#3)
        0x83, 0xec, 0x10, // sub esp,0x10            (int noise)
        0x8d, 0x45, 0xcc, // lea eax,[ebp-0x34]      (int noise)
        0xd8, 0x3d, 0xd8, 0xf9, 0x7f, 0x00, // fdivr [0x7ff9d8]=1.0    (#4)
        0xd9, 0x45, 0xf8, // fld [ebp-0x8]=dx
        0xd8, 0xc9, // fmul st,st(1)           (#5)
        0xd9, 0xc9, // fxch st(1)
        0xd8, 0x4d, 0xfc, // fmul [ebp-0x4]=dy       (#6)
        0xd9, 0x5d, 0xfc, // fstp [ebp-0x4]  (s*dy f32)
        0xdd, 0x5c, 0x24, 0x08, // fstp qword [esp+8]  (dx*s f64)
        0xd9, 0x45, 0xfc, // fld [ebp-0x4]
        0xd9, 0xe0, // fchs                    (#7)
        0xdd, 0x1c, 0x24, // fstp qword [esp]    (-(s*dy) f64)
    ];
    code.extend_from_slice(&src_mid);
    emu.write(isl, &code);
    let halt = isl + code.len() as u64;
    // byte-verify the verbatim middle == source 0x4ee45b..0x4ee482 (else circular).
    assert_eq!(
        &emu.read(isl + mid_off as u64, 39),
        &emu.read(0x4ee45b, 39),
        "scale_project island != source 0x4ee45b"
    );

    let run_scale = |emu: &mut Emu, dx: f32, dy: f32, r: f64| -> (f32, f64, f64) {
        // island-entry esp = S; `sub esp,0x10` => esp=S-0x10; qword stores at S-8 and S-0x10.
        let s_esp = f.esp;
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, s_esp);
        emu.write_u32(f.ebp - 0x8, dx.to_bits());
        emu.write_u32(f.ebp - 0x4, dy.to_bits());
        emu.write(rcell, &r.to_bits().to_le_bytes());
        emu.run(isl, halt);
        let sdy_f32 = emu.read_f32(f.ebp - 0x4);
        let mut q = [0u8; 8];
        q.copy_from_slice(&emu.read(s_esp - 8, 8));
        let dxs_f64 = f64::from_bits(u64::from_le_bytes(q));
        q.copy_from_slice(&emu.read(s_esp - 0x10, 8));
        let neg_f64 = f64::from_bits(u64::from_le_bytes(q));
        (sdy_f32, dxs_f64, neg_f64)
    };

    let mut total = 0u32;
    // delta_sub probes + fuzz
    let dp = [
        0.0f32, 1.0, -1.0, 100.5, -100.5, 1.0e6, -1.0e6, 0.031, 3.4e38,
    ];
    for &bx in &dp {
        for &px in &dp {
            let (gdx, gdy) = run_delta(&mut emu, bx, bx, px, px);
            let (wdx, wdy) = delta_sub(bx, bx, px, px);
            assert_eq!(gdx.to_bits(), wdx.to_bits(), "delta dx bx={bx} px={px}");
            assert_eq!(gdy.to_bits(), wdy.to_bits(), "delta dy bx={bx} px={px}");
            total += 2;
        }
    }
    let mut s = 0x4ee3_3000u32;
    for _ in 0..2000 {
        let bx = rng(&mut s, -1.0e6, 1.0e6);
        let by = rng(&mut s, -1.0e6, 1.0e6);
        let px = rng(&mut s, -1.0e6, 1.0e6);
        let py = rng(&mut s, -1.0e6, 1.0e6);
        let (gdx, gdy) = run_delta(&mut emu, bx, by, px, py);
        let (wdx, wdy) = delta_sub(bx, by, px, py);
        assert_eq!(gdx.to_bits(), wdx.to_bits(), "delta fuzz dx");
        assert_eq!(gdy.to_bits(), wdy.to_bits(), "delta fuzz dy");
        total += 2;
    }

    // scale_project probes (incl. 1/(2R) near-zero and large) + fuzz
    let sp: [(f32, f32, f64); 7] = [
        (1.0, 1.0, 0.5),  // 2R=1, s=1
        (3.0, -4.0, 8.0), // 2R=16, s=0.0625
        (0.0, 0.0, 1.0),  // dx=dy=0
        (123.0, -45.0, 0.016_666_667),
        (1.0e5, -2.0e5, 1.0e-3),
        (-7.5, 7.5, 33.333_333_333),
        (1.0, -1.0, 1.0e-9), // tiny 2R -> huge s
    ];
    for &(dx, dy, r) in &sp {
        let (gs, gx, gn) = run_scale(&mut emu, dx, dy, r);
        let (ws, wx, wn) = scale_project(dx, dy, r);
        assert_eq!(
            gs.to_bits(),
            ws.to_bits(),
            "scale sdy dx={dx} dy={dy} r={r}"
        );
        assert_eq!(
            gx.to_bits(),
            wx.to_bits(),
            "scale dxs dx={dx} dy={dy} r={r}"
        );
        assert_eq!(
            gn.to_bits(),
            wn.to_bits(),
            "scale neg dx={dx} dy={dy} r={r}"
        );
        total += 3;
    }
    let mut s = 0x4ee3_3001u32;
    for _ in 0..2000 {
        let dx = rng(&mut s, -1.0e5, 1.0e5);
        let dy = rng(&mut s, -1.0e5, 1.0e5);
        let r = rng64(&mut s, -1.0e3, 1.0e3);
        if r.abs() < 1.0e-6 {
            continue; // avoid 1/0 (faithful but uninteresting NaN/inf parity already probed)
        }
        let (gs, gx, gn) = run_scale(&mut emu, dx, dy, r);
        let (ws, wx, wn) = scale_project(dx, dy, r);
        assert_eq!(gs.to_bits(), ws.to_bits(), "scale fuzz sdy");
        assert_eq!(gx.to_bits(), wx.to_bits(), "scale fuzz dxs");
        assert_eq!(gn.to_bits(), wn.to_bits(), "scale fuzz neg");
        total += 3;
    }
    eprintln!(
        "glue_4ea_4ee330_scale: {total} bit-exact (fsub×2 slice + island fadd/fdivr/fmul×2/fchs)"
    );
}

// ===========================================================================================
// 0x4eeca0 'PingLocation' — world→screen projection. The owned arith interleaves delegated st0
// producers we cannot faithfully drive in isolation (frame-timer 0x6da9b0, Lua getters 0x6f3620,
// 0.8·x helper 0x41ae60, camera vtable getters), so each owned leg is a COPIED-BYTE ISLAND: the
// EXACT source FP-op bytes (each byte-verified == its source address), with `fld qword [cell]`
// injecting the delegated st0 result (f64) where the source op consumes one, and `fstp [out]`
// capturing the store. Covers all 13 arithmetic ops:
//   #1 fadd st,st (0x4eecf8); #2/#5 fmul[1024] (0x4eedd9/0x4eee04); #3/#6 fdivr (0x4eede0/0x4eee0b);
//   #4 fchs (0x4eedeb); #7/#9 fdivr (0x4eee24/0x4eee34); #8/#10 fmul[ebp-0x10] (0x4eee2b/0x4eee3a);
//   #11/#12 fadd[off] (0x4eee4e/0x4eee61); #13 fadd[8.0] (0x4eeee9).
// PRIMARY: glue_4ea_4eeca0_ping_projection.
// ===========================================================================================
#[test]
fn glue_4ea_4eeca0_ping_projection() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let inj = emu.alloc(8); // injected st0 cell (f64)
    let off = emu.alloc(0x10); // [eax]/[eax+4] offset vector for blocks E
    let out = emu.alloc(8); // f32 store dest
    let mut total = 0u32;

    // Helper: build an island = [prepend `fld qword [inj]`?] + verbatim FP-op bytes + `fstp [out]`,
    // byte-verifying each verbatim op-span against its real source address. Returns (island, halt).
    // `ops` = list of (literal bytes, Some(src_va) to verify | None for our own fstp/fld-inject).

    // ---- #1 ping_double: fld qword[inj]; fadd st,st; fstp [out] ---------------------------
    {
        let isl = emu.alloc(0x20);
        let mut c = vec![0xddu8, 0x05];
        c.extend_from_slice(&(inj as u32).to_le_bytes()); // inject R
        let arith_off = c.len();
        c.extend_from_slice(&[0xdc, 0xc0]); // fadd st,st  (#1)
        c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
        c.extend_from_slice(&(out as u32).to_le_bytes());
        emu.write(isl, &c);
        let halt = isl + c.len() as u64;
        assert_eq!(
            &emu.read(isl + arith_off as u64, 2),
            &emu.read(0x4eecf8, 2),
            "#1 fadd st,st island != src"
        );
        let run = |emu: &mut Emu, r: f64| -> f32 {
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.write(inj, &r.to_bits().to_le_bytes());
            emu.run(isl, halt);
            emu.read_f32(out)
        };
        for r in [0.0f64, 0.5, 1.0, -1.0, 1.0e6, 1.0 / 3.0, 33.333_333_333] {
            assert_eq!(
                run(&mut emu, r).to_bits(),
                ping_double(r).to_bits(),
                "ping_double r={r}"
            );
            total += 1;
        }
        let mut s = 0x4eec_a000u32;
        for _ in 0..2000 {
            let r = rng64(&mut s, -1.0e4, 1.0e4);
            assert_eq!(
                run(&mut emu, r).to_bits(),
                ping_double(r).to_bits(),
                "ping_double fuzz r={r}"
            );
            total += 1;
        }
    }

    // ---- #2,#3 / #5,#6 proj_over_unit: fld qword[inj]=k1; fmul [0x7ffd68]=1024; fdivr [ebp-N]=c;
    //      fstp [out].  Block A divisor numerator @ [ebp-0x8]; block B @ [ebp-0x4]. -------------
    for (slot, srcfmul, srcfdivr, divr_bytes, lbl) in [
        (-0x8i64, 0x4eedd9u64, 0x4eede0u64, [0xd8u8, 0x7d, 0xf8], "A"),
        (-0x4, 0x4eee04, 0x4eee0b, [0xd8, 0x7d, 0xfc], "B"),
    ] {
        let isl = emu.alloc(0x20);
        let mut c = vec![0xddu8, 0x05];
        c.extend_from_slice(&(inj as u32).to_le_bytes()); // inject k1
        let off_fmul = c.len();
        c.extend_from_slice(&[0xd8, 0x0d, 0x68, 0xfd, 0x7f, 0x00]); // fmul [0x7ffd68]=1024
        let off_fdivr = c.len();
        c.extend_from_slice(&divr_bytes); // fdivr [ebp-N]=c
        c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
        c.extend_from_slice(&(out as u32).to_le_bytes());
        emu.write(isl, &c);
        let halt = isl + c.len() as u64;
        assert_eq!(
            &emu.read(isl + off_fmul as u64, 6),
            &emu.read(srcfmul, 6),
            "{lbl} fmul!=src"
        );
        assert_eq!(
            &emu.read(isl + off_fdivr as u64, 3),
            &emu.read(srcfdivr, 3),
            "{lbl} fdivr!=src"
        );
        let run = |emu: &mut Emu, c_in: f32, k1: f64| -> f32 {
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.write(inj, &k1.to_bits().to_le_bytes());
            emu.write_u32((f.ebp as i64 + slot) as u64, c_in.to_bits());
            emu.run(isl, halt);
            emu.read_f32(out)
        };
        // k1 is the delegated 0x41ad70 result (=1.0); probe that and nearby values.
        for &k1 in &[1.0f64, 0.5, 2.0, 1.000_000_1] {
            for &c_in in &[0.0f32, 1.0, -1.0, 512.0, -512.0, 1024.0, 1.0e6, 0.031] {
                assert_eq!(
                    run(&mut emu, c_in, k1).to_bits(),
                    proj_over_unit(c_in, k1).to_bits(),
                    "{lbl} over_unit c={c_in} k1={k1}"
                );
                total += 1;
            }
        }
        let mut s = (srcfmul as u32).wrapping_mul(0x1001);
        for _ in 0..1500 {
            let c_in = rng(&mut s, -5.0e5, 5.0e5);
            let k1 = rng64(&mut s, 0.25, 4.0);
            assert_eq!(
                run(&mut emu, c_in, k1).to_bits(),
                proj_over_unit(c_in, k1).to_bits(),
                "{lbl} over_unit fuzz c={c_in} k1={k1}"
            );
            total += 1;
        }
    }

    // ---- #4 proj_negate: fld qword[inj]=p; fchs; fstp [out] -------------------------------
    {
        let isl = emu.alloc(0x20);
        let mut c = vec![0xddu8, 0x05];
        c.extend_from_slice(&(inj as u32).to_le_bytes()); // inject p (0.8*arg)
        let arith_off = c.len();
        c.extend_from_slice(&[0xd9, 0xe0]); // fchs  (#4)
        c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
        c.extend_from_slice(&(out as u32).to_le_bytes());
        emu.write(isl, &c);
        let halt = isl + c.len() as u64;
        assert_eq!(
            &emu.read(isl + arith_off as u64, 2),
            &emu.read(0x4eedeb, 2),
            "#4 fchs!=src"
        );
        let run = |emu: &mut Emu, p: f64| -> f32 {
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.write(inj, &p.to_bits().to_le_bytes());
            emu.run(isl, halt);
            emu.read_f32(out)
        };
        for p in [0.0f64, 1.0, -1.0, 0.8, -0.8, 1.0e6, 1.0e-9] {
            assert_eq!(
                run(&mut emu, p).to_bits(),
                proj_negate(p).to_bits(),
                "negate p={p}"
            );
            total += 1;
        }
        let mut s = 0x4eed_eb00u32;
        for _ in 0..1500 {
            let p = rng64(&mut s, -1.0e4, 1.0e4);
            assert_eq!(
                run(&mut emu, p).to_bits(),
                proj_negate(p).to_bits(),
                "negate fuzz p={p}"
            );
            total += 1;
        }
    }

    // ---- #7,#8 / #9,#10 proj_div_scale: fld qword[inj]=V; fdivr [ebp-Nn]=n; fmul [ebp-0x10]=m;
    //      fstp [out].  Block C numerator @ [ebp-0x4]; block D @ [ebp-0x8]. ---------------------
    for (nslot, srcfdivr, srcfmul, divr_bytes, lbl) in [
        (-0x4i64, 0x4eee24u64, 0x4eee2bu64, [0xd8u8, 0x7d, 0xfc], "C"),
        (-0x8, 0x4eee34, 0x4eee3a, [0xd8, 0x7d, 0xf8], "D"),
    ] {
        let isl = emu.alloc(0x20);
        let mut c = vec![0xddu8, 0x05];
        c.extend_from_slice(&(inj as u32).to_le_bytes()); // inject V
        let off_fdivr = c.len();
        c.extend_from_slice(&divr_bytes); // fdivr [ebp-Nn]=n
        let off_fmul = c.len();
        c.extend_from_slice(&[0xd8, 0x4d, 0xf0]); // fmul [ebp-0x10]=m
        c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
        c.extend_from_slice(&(out as u32).to_le_bytes());
        emu.write(isl, &c);
        let halt = isl + c.len() as u64;
        assert_eq!(
            &emu.read(isl + off_fdivr as u64, 3),
            &emu.read(srcfdivr, 3),
            "{lbl} fdivr!=src"
        );
        assert_eq!(
            &emu.read(isl + off_fmul as u64, 3),
            &emu.read(srcfmul, 3),
            "{lbl} fmul!=src"
        );
        let run = |emu: &mut Emu, n: f32, v: f64, m: f32| -> f32 {
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.write(inj, &v.to_bits().to_le_bytes());
            emu.write_u32((f.ebp as i64 + nslot) as u64, n.to_bits());
            emu.write_u32(f.ebp - 0x10, m.to_bits());
            emu.run(isl, halt);
            emu.read_f32(out)
        };
        for &n in &[0.0f32, 1.0, -1.0, 0.5, 100.0] {
            for &v in &[1.0f64, 2.0, -3.0, 0.25, 1280.0] {
                for &m in &[0.0f32, 1.0, 2.0, 0.016_666_668] {
                    assert_eq!(
                        run(&mut emu, n, v, m).to_bits(),
                        proj_div_scale(n, v, m).to_bits(),
                        "{lbl} div_scale n={n} v={v} m={m}"
                    );
                    total += 1;
                }
            }
        }
        let mut s = (srcfdivr as u32).wrapping_mul(0x2003);
        for _ in 0..1500 {
            let n = rng(&mut s, -1.0e4, 1.0e4);
            let v = rng64(&mut s, 0.1, 2.0e3);
            let m = rng(&mut s, -1.0e3, 1.0e3);
            assert_eq!(
                run(&mut emu, n, v, m).to_bits(),
                proj_div_scale(n, v, m).to_bits(),
                "{lbl} div_scale fuzz"
            );
            total += 1;
        }
    }

    // ---- #11 / #12 proj_add_offset: fld [ebp-N]=p; fadd [eax+D]=off; fstp [out] -----------
    for (pslot, offdisp, srcfld, srcfadd, fld_bytes, fadd_bytes, lbl) in [
        (
            -0x4i64,
            0u64,
            0x4eee4bu64,
            0x4eee4eu64,
            [0xd9u8, 0x45, 0xfc],
            vec![0xd8u8, 0x00],
            "E11",
        ),
        (
            -0x8,
            4,
            0x4eee5e,
            0x4eee61,
            [0xd9, 0x45, 0xf8],
            vec![0xd8, 0x40, 0x04],
            "E12",
        ),
    ] {
        let isl = emu.alloc(0x20);
        let mut c = Vec::new();
        let off_fld = c.len();
        c.extend_from_slice(&fld_bytes); // fld [ebp-N]=p
        let off_fadd = c.len();
        c.extend_from_slice(&fadd_bytes); // fadd [eax+D]=off
        c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
        c.extend_from_slice(&(out as u32).to_le_bytes());
        emu.write(isl, &c);
        let halt = isl + c.len() as u64;
        assert_eq!(
            &emu.read(isl + off_fld as u64, 3),
            &emu.read(srcfld, 3),
            "{lbl} fld!=src"
        );
        assert_eq!(
            &emu.read(isl + off_fadd as u64, fadd_bytes.len()),
            &emu.read(srcfadd, fadd_bytes.len()),
            "{lbl} fadd!=src"
        );
        let run = |emu: &mut Emu, p: f32, o: f32| -> f32 {
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.set_reg(RegisterX86::EAX, off);
            emu.write_u32((f.ebp as i64 + pslot) as u64, p.to_bits());
            emu.write_u32(off + offdisp, o.to_bits());
            emu.run(isl, halt);
            emu.read_f32(out)
        };
        for &p in &[0.0f32, 1.0, -1.0, 512.0, -7.5, 1.0e6] {
            for &o in &[0.0f32, 1.0, -1.0, 320.0, 240.0, -0.031] {
                assert_eq!(
                    run(&mut emu, p, o).to_bits(),
                    proj_add_offset(p, o).to_bits(),
                    "{lbl} add_offset p={p} o={o}"
                );
                total += 1;
            }
        }
        let mut s = (srcfld as u32).wrapping_mul(0x3005);
        for _ in 0..1500 {
            let p = rng(&mut s, -1.0e6, 1.0e6);
            let o = rng(&mut s, -2000.0, 2000.0);
            assert_eq!(
                run(&mut emu, p, o).to_bits(),
                proj_add_offset(p, o).to_bits(),
                "{lbl} add_offset fuzz"
            );
            total += 1;
        }
    }

    // ---- #13 proj_bias8: fld [ebp-0x10]=z; fadd [0x807b10]=8.0; fstp [out] -----------------
    {
        let isl = emu.alloc(0x20);
        let mut c = Vec::new();
        let off_fld = c.len();
        c.extend_from_slice(&[0xd9, 0x45, 0xf0]); // fld [ebp-0x10]=z
        let off_fadd = c.len();
        c.extend_from_slice(&[0xd8, 0x05, 0x10, 0x7b, 0x80, 0x00]); // fadd [0x807b10]=8.0
        c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
        c.extend_from_slice(&(out as u32).to_le_bytes());
        emu.write(isl, &c);
        let halt = isl + c.len() as u64;
        assert_eq!(
            &emu.read(isl + off_fld as u64, 3),
            &emu.read(0x4eeee3, 3),
            "#13 fld!=src"
        );
        assert_eq!(
            &emu.read(isl + off_fadd as u64, 6),
            &emu.read(0x4eeee9, 6),
            "#13 fadd!=src"
        );
        let run = |emu: &mut Emu, z: f32| -> f32 {
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.write_u32(f.ebp - 0x10, z.to_bits());
            emu.run(isl, halt);
            emu.read_f32(out)
        };
        // the binary re-zeroes z to 0.0 on this path; probe 0.0 plus a spread for the +8 op.
        for z in [0.0f32, 1.0, -1.0, -8.0, 8.0, 1024.0, -1.0e6, 0.031] {
            assert_eq!(
                run(&mut emu, z).to_bits(),
                proj_bias8(z).to_bits(),
                "bias8 z={z}"
            );
            total += 1;
        }
        let mut s = 0x4eee_e900u32;
        for _ in 0..1500 {
            let z = rng(&mut s, -1.0e6, 1.0e6);
            assert_eq!(
                run(&mut emu, z).to_bits(),
                proj_bias8(z).to_bits(),
                "bias8 fuzz z={z}"
            );
            total += 1;
        }
    }

    eprintln!("glue_4ea_4eeca0_ping_projection: {total} bit-exact (13 FP_ARITH ops via copied-byte islands)");
}
