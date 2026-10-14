//! The arithmetic of `UpdateMapHighlight` (`0x4a7fa0`, `systems/ui/worldmap.md#hover-and-click`)
//! run from the exe as instruction slices.
//!
//! The slices are the zone rectangle normalization and hit test, the texture height and its
//! power-of-two round-up, and the UV scale and offset of the area-grid and zone branches. The
//! test writes the inputs into the registers and memory a slice reads, halts at its end and reads
//! the stored `f32` or the pushed numbers back, exactly. Every draw comes off the high bits of an
//! LCG.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::worldmap_highlight::{
    highlight_dim_ratio as worldmap_zone_dim_ratio, highlight_uv_area as worldmap_uv,
    highlight_uv_zone as worldmap_uv_zone, normalize_zone_rect as worldmap_normalize,
    point_in_zone_rect as worldmap_point_in_rect, round_up_texture_dim as worldmap_pot_dim,
    texture_height_from_area as worldmap_pot_extent_area,
    texture_height_from_rect as worldmap_pot_extent_rect,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

fn dbits(c: &benilla_re_difftest::StubCall) -> f64 {
    f64::from_bits((u64::from(c.args[1]) << 32) | u64::from(c.args[0]))
}

fn pushes(emu: &Emu) -> Vec<f64> {
    emu.calls()
        .iter()
        .filter(|c| c.addr == 0x6f3810)
        .map(dbits)
        .collect()
}

// ===========================================================================================
// worldmap_highlight (0x4a7fa0) — sub-primitives: overlay_hit / pot_roundup / worldmap_uv
// ===========================================================================================
fn patch_branch(emu: &mut Emu, addr: u64, eax: u32, halt: u64) {
    // `mov eax, imm32 ; jmp halt`  (B8 imm32 E9 rel32) — 10 bytes.
    let mut c = vec![0xb8u8];
    c.extend_from_slice(&eax.to_le_bytes());
    c.push(0xe9);
    let rel = (halt as i64 - (addr as i64 + 10)) as i32;
    c.extend_from_slice(&rel.to_le_bytes());
    emu.write(addr, &c);
}

#[test]
fn worldmap_highlight() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let ebp_base = emu.alloc(0x80);
    let ebp = ebp_base + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;

    // ---------- (a) overlay_hit: normalize (4a8041..) + point-in-rect, exits patched ----------
    let halt = emu.alloc(0x10);
    patch_branch(&mut emu, 0x4a80cb, 1, halt); // rect HIT reached
    patch_branch(&mut emu, 0x4a80ea, 0, halt); // any MISS
    let zone = emu.alloc(0x48);

    let mut hit_n = 0u32;
    let mut miss_n = 0u32;
    let mut degen_n = 0u32;
    let mut hit_total = 0u32;

    let run_hit = |emu: &mut Emu, e34, e38, e3c, e40, p1: f32, p2: f32| -> (bool, [f32; 4]) {
        emu.write_u32(zone + 0x34, e34 as u32);
        emu.write_u32(zone + 0x38, e38 as u32);
        emu.write_u32(zone + 0x3c, e3c as u32);
        emu.write_u32(zone + 0x40, e40 as u32);
        emu.write_u32(ebp - 0x8, p1.to_bits());
        emu.write_u32(ebp - 0xc, p2.to_bits());
        emu.set_reg(RegisterX86::ECX, zone);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x4a8041, halt);
        let hit = emu.get_reg(RegisterX86::EAX) as u32 == 1;
        let norm = [
            emu.read_f32(ebp - 0x24),
            emu.read_f32(ebp - 0x28),
            emu.read_f32(ebp - 0x1c),
            emu.read_f32(ebp - 0x20),
        ];
        (hit, norm)
    };

    let mut s = 0x4a7f_a000u32;
    for _ in 0..3000 {
        // random rect ints (sorted so the normalized rect is non-degenerate most of the time).
        let mut xa = (next(&mut s) >> 8) as i32 % 668;
        let mut xb = (next(&mut s) >> 8) as i32 % 668;
        let mut ya = (next(&mut s) >> 8) as i32 % 1002;
        let mut yb = (next(&mut s) >> 8) as i32 % 1002;
        if xa > xb {
            std::mem::swap(&mut xa, &mut xb);
        }
        if ya > yb {
            std::mem::swap(&mut ya, &mut yb);
        }
        // occasionally force a degenerate edge (e34==e3c) to exercise the C3-only reject.
        if next(&mut s) >> 8 & 7 == 0 {
            xb = xa;
        }
        // edges: e34/e3c = X (recip 1/668), e38/e40 = Y (recip 1/1002).
        let (e34, e3c, e38, e40) = (xa, xb, ya, yb);
        let [a, b, c, d] = worldmap_normalize(e34, e38, e3c, e40);
        // cursor: half the time inside [a,c]x[b,d], half random.
        let inside = next(&mut s) >> 8 & 1 == 0;
        let (p1, p2) = if inside {
            (rng(&mut s, a, c.max(a)), rng(&mut s, b, d.max(b)))
        } else {
            (rng(&mut s, -0.2, 1.2), rng(&mut s, -0.2, 1.2))
        };
        let (bin_hit, norm) = run_hit(&mut emu, e34, e38, e3c, e40, p1, p2);
        assert_eq!(norm[0].to_bits(), a.to_bits(), "normalize a e38={e38}");
        assert_eq!(norm[1].to_bits(), b.to_bits(), "normalize b e34={e34}");
        assert_eq!(norm[2].to_bits(), c.to_bits(), "normalize c e40={e40}");
        assert_eq!(norm[3].to_bits(), d.to_bits(), "normalize d e3c={e3c}");
        let mine = worldmap_point_in_rect(p1, p2, a, b, c, d);
        assert_eq!(
            bin_hit, mine,
            "overlay_hit p1={p1} p2={p2} rect=[{a},{b},{c},{d}]"
        );
        if a == c || b == d {
            degen_n += 1;
        }
        if bin_hit {
            hit_n += 1;
        } else {
            miss_n += 1;
        }
        hit_total += 1;
    }
    eprintln!("  overlay_hit: {hit_total} bit-exact (hit={hit_n} miss={miss_n} degen={degen_n})");
    assert!(hit_n > 0 && miss_n > 0, "overlay_hit: need both hit/miss");
    assert!(degen_n > 0, "overlay_hit: need a degenerate-edge reject");

    // ---------- (b) pot_roundup: dim power-of-2 round-up (4a833c..4a83ac) ----------
    let mut pot_total = 0u32;
    let mut pow2_n = 0u32;
    let mut nonpow2_n = 0u32;
    let mut gt256_n = 0u32;
    let run_pot = |emu: &mut Emu, n: u32| -> u32 {
        emu.write_u32(ebp - 0xc, n);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x4a833c, 0x4a83ac);
        emu.get_reg(RegisterX86::EDI) as u32
    };
    let mut det: Vec<u32> = (0u32..=260).collect();
    det.extend([300, 320, 384, 512, 700, 1000, 1024, 4096, 65535]);
    let mut s = 0x4a83_3c00u32;
    for _ in 0..1000 {
        det.push((next(&mut s) >> 8) % 70000);
    }
    for n in det {
        let bin = run_pot(&mut emu, n);
        assert_eq!(bin, worldmap_pot_dim(n), "pot_roundup n={n}");
        let lo = n & 0xff;
        if lo != 0 && lo & (lo - 1) == 0 {
            pow2_n += 1;
        } else if lo != 0 {
            nonpow2_n += 1;
        }
        if n > 256 {
            gt256_n += 1;
        }
        pot_total += 1;
    }
    eprintln!(
        "  pot_roundup: {pot_total} bit-exact (pow2={pow2_n} nonpow2={nonpow2_n} >256={gt256_n})"
    );
    assert!(
        pow2_n > 0 && nonpow2_n > 0 && gt256_n > 0,
        "pot_roundup: branch coverage"
    );

    // ---------- (b0) worldmap_pot_extent: the aspect→POT-dim derivation that PRODUCES n (4a82ed/
    // 4a8319 .. 4a833c). Two branches: rect (edi-struct f32 edges, K=128) + area-grid (ebx zone-record
    // ints, K=256). The slices end at 0x4a833c (after the `mov [ebp-0xc],eax` store = pot_roundup entry).
    // The fst-no-pop numerator (LIVE f64) vs fstp denominator (f32) is exercised by a large extent where
    // f64(Y0-Y1) != f32(Y0-Y1) (an `fst` without a pop; an f32-narrowed transcription would diverge here).
    let rectbuf = emu.alloc(0x20);
    let run_rect = |emu: &mut Emu, x0: f32, x1: f32, y0: f32, y1: f32| -> i32 {
        emu.write_u32(rectbuf + 0x10, x0.to_bits());
        emu.write_u32(rectbuf + 0x14, x1.to_bits());
        emu.write_u32(rectbuf + 0x18, y0.to_bits());
        emu.write_u32(rectbuf + 0x1c, y1.to_bits());
        emu.write_u32(ebp - 0xc, 0xdead_beef);
        emu.set_reg(RegisterX86::EDI, rectbuf);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x4a8319, 0x4a833c);
        emu.read_u32(ebp - 0xc) as i32
    };
    let zonebuf = emu.alloc(0x20);
    let run_area = |emu: &mut Emu, e8: i32, ec: i32, e10: i32, e14: i32| -> i32 {
        emu.write_u32(zonebuf + 0x8, e8 as u32);
        emu.write_u32(zonebuf + 0xc, ec as u32);
        emu.write_u32(zonebuf + 0x10, e10 as u32);
        emu.write_u32(zonebuf + 0x14, e14 as u32);
        emu.write_u32(ebp - 0xc, 0xdead_beef);
        emu.set_reg(RegisterX86::EBX, zonebuf);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x4a82ed, 0x4a833c);
        emu.read_u32(ebp - 0xc) as i32
    };
    let mut ext_total = 0u32;
    let mut s = 0x4a82_ed00u32;
    // rect: worldmap aspects + a LARGE extent where f64(Y0-Y1) != f32(Y0-Y1) (live-f64 vs f32).
    let rect_cases: &[(f32, f32, f32, f32)] = &[
        (256.0, 0.0, 256.0, 0.0),
        (668.0, 0.0, 1002.0, 0.0),
        (1024.0, 0.0, 768.0, 0.0),
        (100.0, 50.0, 400.0, 100.0),
        (33_554_432.0, 0.0, 33_554_432.0, 1.0), // den=2^25; num=2^25-1 live(f64) vs 2^25 if f32 → dim differs
    ];
    for &(x0, x1, y0, y1) in rect_cases {
        let bin = run_rect(&mut emu, x0, x1, y0, y1);
        assert_eq!(
            bin,
            worldmap_pot_extent_rect(x0, x1, y0, y1),
            "pot_extent rect ({x0},{x1},{y0},{y1})"
        );
        ext_total += 1;
    }
    for _ in 0..400 {
        let x0 = ((next(&mut s) >> 12) % 4000 + 1) as f32; // non-degenerate positive extents
        let y0 = ((next(&mut s) >> 12) % 4000 + 1) as f32;
        let bin = run_rect(&mut emu, x0, 0.0, y0, 0.0);
        assert_eq!(
            bin,
            worldmap_pot_extent_rect(x0, 0.0, y0, 0.0),
            "pot_extent rect fuzz x0={x0} y0={y0}"
        );
        ext_total += 1;
    }
    // area: DENOM=([ebx+0xc]-[ebx+0x8])+1 (kept >0), NUM=([ebx+0x14]-[ebx+0x10])+1.
    for _ in 0..200 {
        let e8 = ((next(&mut s) >> 16) % 500) as i32;
        let ec = e8 + 1 + ((next(&mut s) >> 16) % 2000) as i32;
        let e10 = ((next(&mut s) >> 16) % 500) as i32;
        let e14 = e10 + ((next(&mut s) >> 16) % 2000) as i32;
        let bin = run_area(&mut emu, e8, ec, e10, e14);
        let (denom, num) = ((ec - e8) + 1, (e14 - e10) + 1);
        assert_eq!(
            bin,
            worldmap_pot_extent_area(denom, num),
            "pot_extent area ({e8},{ec},{e10},{e14})"
        );
        ext_total += 1;
    }
    eprintln!(
        "  pot_extent: {ext_total} bit-exact (rect K=128 + area K=256, incl. fst-no-pop live-f64)"
    );

    // ---------- (c)+(d) worldmap_uv: aspect-fit + UV scale/offset (4a83b9..4a858e) ----------
    let overlay = emu.alloc(0x28);
    emu.stub(0x6f3810, 0, 2);
    let mut uv_total = 0u32;
    let mut wle_n = 0u32;
    let mut wgt_n = 0u32;
    let run_uv = |emu: &mut Emu,
                  w: f32,
                  h: f32,
                  e8: i32,
                  e10: i32,
                  c18: f32,
                  c1c: f32,
                  scale: f32|
     -> Vec<f64> {
        emu.write_u32(overlay + 0x8, e8 as u32);
        emu.write_u32(overlay + 0x10, e10 as u32);
        emu.write_u32(overlay + 0x18, c18.to_bits());
        emu.write_u32(overlay + 0x1c, c1c.to_bits());
        emu.write_u32(overlay + 0x20, scale.to_bits());
        emu.write_u32(ebp - 0x4, w.to_bits());
        emu.write_u32(ebp - 0x8, h.to_bits());
        emu.write_u32(ebp - 0x14, 0); // force the area-grid path (not the overlay branch)
        emu.set_reg(RegisterX86::EBX, overlay);
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x4a83b9, 0x4a858e);
        pushes(emu)
    };
    let mut s = 0x4a83_b900u32;
    for _ in 0..3000 {
        let w = rng(&mut s, 1.0, 800.0);
        let h = rng(&mut s, 1.0, 800.0);
        let e8 = (next(&mut s) >> 8) as i32 % 64;
        let e10 = (next(&mut s) >> 8) as i32 % 64;
        let c18 = rng(&mut s, -50.0, 50.0);
        let c1c = rng(&mut s, -50.0, 50.0);
        let scale = rng(&mut s, 0.1, 4.0);
        let p = run_uv(&mut emu, w, h, e8, e10, c18, c1c, scale);
        assert_eq!(
            p.len(),
            6,
            "worldmap_uv: expected 6 pushes, got {}",
            p.len()
        );
        let uv = worldmap_uv(w, h, e8, e10, c18, c1c, scale);
        let want = [
            uv.aspect[0],
            uv.aspect[1],
            uv.u_scale,
            uv.v_scale,
            uv.u_off,
            uv.v_off,
        ];
        for (i, (g, e)) in p.iter().zip(want.iter()).enumerate() {
            assert_eq!(
                g.to_bits(),
                e.to_bits(),
                "worldmap_uv idx={i} w={w} h={h} scale={scale}"
            );
        }
        if w <= h {
            wle_n += 1;
        } else {
            wgt_n += 1;
        }
        uv_total += 1;
    }
    eprintln!("  worldmap_uv: {uv_total} bit-exact (w<=h={wle_n} w>h={wgt_n})");
    assert!(
        wle_n > 0 && wgt_n > 0,
        "worldmap_uv: need both aspect branches"
    );

    // ---------- (e) zone-search UV: rect inverse-lerp (4a850b..4a858e) ----------
    // The sibling of the area-grid branch above, reached when the zone selector `zone+0x8 != 0`.
    // `eax` = the matched parent/continent-map rect, `edx` = the hovered zone rect; both carry
    // floats at +0x10/+0x14/+0x18/+0x1c. We set eax/edx directly + the f32 w/h at [ebp-0x4]/-0x8
    // (the same values 0x4a8319 stores into them) and run the inverse-lerp slice in isolation.
    let parent_e = emu.alloc(0x24);
    let zone_e = emu.alloc(0x24);
    let mut zuv_total = 0u32;
    let run_zuv = |emu: &mut Emu, parent: [f32; 4], zone: [f32; 4]| -> Vec<f64> {
        for (i, &v) in parent.iter().enumerate() {
            emu.write_u32(parent_e + 0x10 + (i as u64) * 4, v.to_bits());
        }
        for (i, &v) in zone.iter().enumerate() {
            emu.write_u32(zone_e + 0x10 + (i as u64) * 4, v.to_bits());
        }
        // [ebp-0x4]=w, [ebp-0x8]=h (the zone-rect extents, f32 — exactly 0x4a8319's stores).
        let w = (f64::from(zone[0]) - f64::from(zone[1])) as f32;
        let h = (f64::from(zone[2]) - f64::from(zone[3])) as f32;
        emu.write_u32(ebp - 0x4, w.to_bits());
        emu.write_u32(ebp - 0x8, h.to_bits());
        emu.set_reg(RegisterX86::EAX, parent_e);
        emu.set_reg(RegisterX86::EDX, zone_e);
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x4a850b, 0x4a858e);
        pushes(emu)
    };
    let mut s = 0x4a85_0b00u32;
    for _ in 0..3000 {
        let parent = [
            rng(&mut s, -300.0, 300.0),
            rng(&mut s, -300.0, 300.0),
            rng(&mut s, -300.0, 300.0),
            rng(&mut s, -300.0, 300.0),
        ];
        let zone = [
            rng(&mut s, -300.0, 300.0),
            rng(&mut s, -300.0, 300.0),
            rng(&mut s, -300.0, 300.0),
            rng(&mut s, -300.0, 300.0),
        ];
        let p = run_zuv(&mut emu, parent, zone);
        assert_eq!(p.len(), 4, "zone uv: expected 4 pushes, got {}", p.len());
        let uv = worldmap_uv_zone(parent, zone);
        let want = [uv.u_scale, uv.v_scale, uv.u_off, uv.v_off];
        for (i, (g, e)) in p.iter().zip(want.iter()).enumerate() {
            assert_eq!(
                g.to_bits(),
                e.to_bits(),
                "zone uv idx={i} parent={parent:?} zone={zone:?}"
            );
        }
        zuv_total += 1;
    }
    eprintln!("  worldmap_uv_zone: {zuv_total} bit-exact");

    // ---------- (f) zone-search dim ratio: (i64)dim / (i32)potdim (4a84a0..4a84bf) ----------
    let mut ratio_total = 0u32;
    let run_ratio = |emu: &mut Emu, dim: u32, potdim: i32| -> Vec<f64> {
        emu.write_u32(ebp - 0xc, dim);
        emu.set_reg(RegisterX86::EDI, u64::from(potdim as u32));
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x4a84a0, 0x4a84c4);
        pushes(emu)
    };
    let mut det: Vec<(u32, i32)> = vec![
        (1, 1),
        (64, 64),
        (100, 128),
        (255, 256),
        (513, 1024),
        (1000, 1024),
        (4096, 4096),
    ];
    let mut s = 0x4a84_a000u32;
    for _ in 0..1000 {
        // dims/potdims are the aspect-ftol and its pot round-up — small positive ints in practice.
        det.push((
            1 + (next(&mut s) >> 8) % 70000,
            1 + ((next(&mut s) >> 8) % 70000) as i32,
        ));
    }
    for (dim, potdim) in det {
        let p = run_ratio(&mut emu, dim, potdim);
        assert_eq!(
            p.len(),
            1,
            "zone dim ratio: expected 1 push, got {}",
            p.len()
        );
        assert_eq!(
            p[0].to_bits(),
            worldmap_zone_dim_ratio(dim, potdim).to_bits(),
            "zone dim ratio dim={dim} potdim={potdim}"
        );
        ratio_total += 1;
    }
    eprintln!("  worldmap_uv_zone dim ratio: {ratio_total} bit-exact");
}
