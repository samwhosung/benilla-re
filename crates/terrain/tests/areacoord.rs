//! The area queries' arithmetic, as slices: `FindNearestLiquidInAreas` (`0x68af10`) from
//! `0x68af1b` to `0x68af63` and from `0x68b031` to `0x68b05d`; and `ClaimCameraMapObject`
//! (`0x6821f0`) from `0x68223c` to `0x682252` and from `0x6822a2` to `0x6822b3`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::areacoord::{
    area_coord_x, area_coord_y, area_param_offset, area_tile_x, area_tile_y, area_window,
    nearer_select_jp, ray_end_z,
};
use common::{assert_f32, next};

/// A stack pointer inside the harness's stack, for slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any `f32`, from random bits: subnormals, infinities and NaN included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// `[0x7ffab4]`, the map's half extent.
const MAP_CENTER: f32 = f32::from_bits(0x4685_5555);

#[test]
fn area_coord_conversion_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80); // the frame below ebp
    let pos = e.alloc(0x10); // edi: x at +0, y at +4
    let ebp = frame + 0x70;
    let slot = |disp: u64| ebp - disp;
    let (s_ix, s_iy) = (slot(0xc), slot(0x8)); // [ebp-0xc] = ix, [ebp-8] = iy
    let mut s = 0x68af_0000u32;

    // 0x68af1b to 0x68af63, edi = pos
    let conv_case = |e: &mut Emu, px: f32, py: f32, ctx: &str| {
        e.write_u32(pos, px.to_bits());
        e.write_u32(pos + 4, py.to_bits());
        e.write_u32(s_ix, 0xdead_beef);
        e.write_u32(s_iy, 0xdead_beef);
        e.set_reg(RegisterX86::EDI, pos);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x68af1b, 0x68af63);
        let bin_ix = e.read_u32(s_ix) as i32;
        let bin_iy = e.read_u32(s_iy) as i32;
        assert_eq!(bin_ix, area_coord_x(px), "{ctx} iX (px={px})");
        assert_eq!(bin_iy, area_coord_y(py), "{ctx} iY (py={py})");
        // the tile is `>> 7 & 0x3f` of the same ix and iy
        assert_eq!((bin_ix >> 7) & 0x3f, area_tile_x(px), "{ctx} tileX");
        assert_eq!((bin_iy >> 7) & 0x3f, area_tile_y(py), "{ctx} tileY");
    };

    // Fixed values first: a tie at the `fistp` and the step between tiles.
    conv_case(&mut e, 0.0, 0.0, "zeros");
    conv_case(&mut e, MAP_CENTER, MAP_CENTER, "at-center (=> 0)");
    conv_case(&mut e, -MAP_CENTER, 2.0 * MAP_CENTER, "span");
    // `0.24·A − 0.5` on a tie of `fistp`, and ix either side of a tile's edge (128 is tile 1, 127
    // tile 0).
    {
        // ix = 128: A near 128.5/0.24, px = C − A
        let a_for_128 = 128.5f32 / 0.24f32;
        let px = MAP_CENTER - a_for_128;
        conv_case(&mut e, px, px, "tile-flip iX~128 boundary");
        // ix = 127: A near 127.5/0.24
        let a_for_127 = 127.5f32 / 0.24f32;
        let px2 = MAP_CENTER - a_for_127;
        conv_case(&mut e, px2, px2, "tile-flip iX~127 boundary");
        // an exact tie on the y path: `(C − py)·0.24 = 1`, so `B − 0.5 = 0.5`
        let py_tie = MAP_CENTER - (1.0f32 / 0.24f32);
        conv_case(&mut e, py_tie, py_tie, "Y fistp tie B~1.0");
    }

    let mut checked = 0u32;
    for i in 0..3000u32 {
        // x and y drawn apart: their stores differ
        let px = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -40000.0, 40000.0)
        };
        let py = if i % 5 == 1 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -40000.0, 40000.0)
        };
        conv_case(&mut e, px, py, &format!("fuzz{i}"));
        checked += 1;
    }

    // 0x68b031 to 0x68b05d, esi = the area
    let area = e.alloc(0x100); // +0x7c and +0x80
    let (s_off0, s_off1) = (slot(0x30), slot(0x2c)); // [ebp-0x30], [ebp-0x2c]
    let off_case = |e: &mut Emu, f0: f32, f1: f32, ctx: &str| {
        e.write_u32(area + 0x7c, f0.to_bits());
        e.write_u32(area + 0x80, f1.to_bits());
        e.write_u32(s_off0, 0xdead_beef);
        e.write_u32(s_off1, 0xdead_beef);
        e.set_reg(RegisterX86::ESI, area);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x68b031, 0x68b05d);
        assert_f32(
            e.read_f32(s_off0),
            area_param_offset(f0),
            &format!("{ctx} off0"),
        );
        assert_f32(
            e.read_f32(s_off1),
            area_param_offset(f1),
            &format!("{ctx} off1"),
        );
    };
    off_case(&mut e, 0.0, 0.0, "zeros");
    off_case(&mut e, 533.3333, 533.3333, "at-offset (=> ~0)");
    off_case(&mut e, -1000.0, 34000.0, "span");
    let mut off_checked = 0u32;
    for i in 0..2000u32 {
        let f0 = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -2000.0, 36000.0)
        };
        let f1 = if i % 5 == 1 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -2000.0, 36000.0)
        };
        off_case(&mut e, f0, f1, &format!("fuzz{i}"));
        off_checked += 1;
    }

    // The window at its edges: the low end stops at 0 for t <= 1, the high at 0x3f for t >= 0x3e.
    assert_eq!(area_window(0), (0, 1), "clamp t=0 lo->0");
    assert_eq!(area_window(1), (0, 2), "clamp t=1 lo->0 (t-1=0)");
    assert_eq!(area_window(2), (1, 3), "clamp t=2 unclamped");
    assert_eq!(area_window(0x3e), (0x3d, 0x3f), "clamp t=0x3e hi=0x3f");
    assert_eq!(
        area_window(0x3f),
        (0x3e, 0x3f),
        "clamp t=0x3f hi->0x3f (t+1=0x40 clamps)"
    );

    eprintln!(
        "area_coord_conversion: {checked} conv + {off_checked} offset cases bit-exact vs WoW.exe \
         @0x68af10 slices [68af1b,68af63)/[68b031,68b05d) + clamp/tile boundary probes"
    );
}

#[test]
fn world_query_consumer_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x60;
    let slot = |disp: u64| ebp - disp;
    let mut s = 0x6821_0000u32;

    // 0x68223c to 0x682252, on [ebp-0x20]
    let s_z = slot(0x20);
    let endz_case = |e: &mut Emu, eye_z: f32, ctx: &str| {
        e.write_u32(s_z, eye_z.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x68223c, 0x682252);
        assert_f32(e.read_f32(s_z), ray_end_z(eye_z), ctx);
    };
    for &z in &[
        0.0f32,
        1760.0,
        -1760.0,
        100.5,
        1e6,
        -1e6,
        f32::from_bits(0x4685_5555),
    ] {
        endz_case(&mut e, z, "edge");
    }
    let mut checked = 0u32;
    for i in 0..3000u32 {
        let z = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -5000.0, 5000.0)
        };
        endz_case(&mut e, z, &format!("fuzz{i}"));
        checked += 1;
    }

    // 0x6822a2 to 0x6822b3: [ebp-0xc] the terrain's fraction, [ebp-0x10] the object's, the byte
    // [ebp-1] entering as 1; it is cleared when the terrain's is below (`jp`).
    let s_terr = slot(0xc);
    let s_wmo = slot(0x10);
    let s_byte_dw = slot(0x4); // [ebp-1] is the top byte of [ebp-4]
    let select_case = |e: &mut Emu, terrain_t: f32, wmo_t: f32, ctx: &str| {
        e.write_u32(s_terr, terrain_t.to_bits());
        e.write_u32(s_wmo, wmo_t.to_bits());
        e.write_u32(s_byte_dw, 0x0100_0000); // [ebp-1] = 1
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6822a2, 0x6822b3);
        let bin = ((e.read_u32(s_byte_dw) >> 24) & 0xff) as u8;
        assert_eq!(
            bin,
            nearer_select_jp(1, terrain_t, wmo_t),
            "{ctx} (t={terrain_t} w={wmo_t})"
        );
    };
    // the tie and both orders
    select_case(&mut e, 0.5, 0.5, "TIE t==w (jp taken -> keep 1)");
    select_case(&mut e, 0.3, 0.7, "terrain nearer (cleared -> 0)");
    select_case(&mut e, 0.7, 0.3, "wmo nearer (kept -> 1)");
    select_case(&mut e, 0.0, 0.0, "both 0 tie");
    select_case(&mut e, -0.0, 0.0, "-0 vs +0 tie");
    select_case(
        &mut e,
        f32::from_bits(0x7fc0_0000),
        0.5,
        "NaN terrain (kept)",
    );
    let mut sel_checked = 0u32;
    let mut nearer_terrain = 0u32;
    let mut nearer_wmo = 0u32;
    for i in 0..3000u32 {
        // many equal pairs, chosen by the high bits, so the tie runs often
        let (t, w) = if (next(&mut s) >> 8).is_multiple_of(3) {
            let v = frand(&mut s, 0.0, 1.0);
            (v, v) // a tie
        } else {
            (frand(&mut s, 0.0, 1.0), frand(&mut s, 0.0, 1.0))
        };
        select_case(&mut e, t, w, &format!("fuzz{i}"));
        if t < w {
            nearer_terrain += 1;
        } else {
            nearer_wmo += 1;
        }
        sel_checked += 1;
    }
    assert!(
        nearer_terrain > 0 && nearer_wmo > 0,
        "both select orderings exercised (terrain<wmo={nearer_terrain}, terrain>=wmo={nearer_wmo})"
    );
    eprintln!(
        "world_query_consumer: {checked} ray-end + {sel_checked} select cases bit-exact vs WoW.exe \
         @0x6821f0 slices [68223c,682252)/[6822a2,6822b3) (jp branch, ties={nearer_wmo})"
    );
}
