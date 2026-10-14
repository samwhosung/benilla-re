//! The rect and anchor getters and setters of a region (`GetCenter`, `GetLeft`, `GetRight`,
//! `GetTop`, `GetBottom`, `GetWidth`, `GetHeight`, `GetPoint`, `SetWidth`, `SetHeight`, `SetPoint`)
//! against the exe: each converts through the layout scale, run as the slice of instructions from
//! the `call 0x41ad70` to the store of the result, including the three leaf helpers.
//!
//! Probes: the scale 1.0 and the exact 0.8, zero and equal coordinates, and a coordinate whose
//! product with 1024 lands on a tie in `f32`.

#![allow(clippy::float_cmp, clippy::too_many_arguments)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::ui_scale::{
    center_to_logical, device_to_logical, get_bottom, get_center, get_height, get_left, get_point,
    get_right, get_top, get_width, logical_to_device, point_to_logical, set_height, set_point,
    set_width, UI_SCALE,
};
use fpkit::{read_f64, rng};

struct Frame {
    this: u64,
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let this = emu.alloc(0x300);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { this, ebp, esp }
}
fn set_frame(emu: &mut Emu, f: &Frame) {
    emu.set_reg(RegisterX86::ESI, f.this);
    emu.set_reg(RegisterX86::ECX, f.this);
    emu.set_reg(RegisterX86::EDI, f.this);
    emu.set_reg(RegisterX86::EBP, f.ebp);
    emu.set_reg(RegisterX86::ESP, f.esp);
}
// Leaf-composition kernels — device_to_logical / logical_to_device / point_to_logical
// (diffed via the GetLeft / SetWidth / GetPoint slices below; this section also exercises the
// kernels directly as a sanity floor before the per-row slices.)

// Getters: device_to_logical slice — call 0x41ad70; fmul[1024]; fld coord; (center math) fdiv
// scale; fmulp; fstp[esp]; call 0x41ae40; fstp qword[esp]. Reads the f64 at the final ESP.

/// Run a getter slice `[start, end)` that fills `[ebp-coord_off]` with `coord` and `[ebp-4]` with
/// `scale`, halting after the `fstp qword [esp]`; returns the pushed f64.
fn run_getter(
    emu: &mut Emu,
    f: &Frame,
    start: u64,
    end: u64,
    coord_off: u64,
    coord: f32,
    scale: f32,
) -> f64 {
    set_frame(emu, f);
    emu.write_u32(f.ebp - coord_off, coord.to_bits());
    emu.write_u32(f.ebp - 0x4, scale.to_bits());
    emu.run(start, end);
    let esp = emu.get_reg(RegisterX86::ESP);
    read_f64(emu, esp)
}

#[test]
fn luabind_7a1_getters() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (start, end, coord_off, our-fn) for the four scale-divided rect getters.
    // GetLeft [ebp-0x10], GetRight [ebp-8], GetTop [ebp-0xc], GetBottom [ebp-0x14].
    struct G {
        name: &'static str,
        start: u64,
        end: u64,
        off: u64,
        ours: fn(f32, f32) -> f64,
    }
    let getters = [
        G {
            name: "GetLeft",
            start: 0x7a1a54,
            end: 0x7a1a76,
            off: 0x10,
            ours: get_left,
        },
        G {
            name: "GetRight",
            start: 0x7a1b74,
            end: 0x7a1b96,
            off: 0x8,
            ours: get_right,
        },
        G {
            name: "GetTop",
            start: 0x7a1c94,
            end: 0x7a1cb6,
            off: 0xc,
            ours: get_top,
        },
        G {
            name: "GetBottom",
            start: 0x7a1db4,
            end: 0x7a1dd6,
            off: 0x14,
            ours: get_bottom,
        },
    ];

    let mut total = 0u32;
    for g in &getters {
        // deterministic boundaries: coord==0 (=> 0.0), scale==1.0 (no-op divide), and the exact
        // 0.8 UI scale; plus a coord whose 1024·coord/scale lands on an f32 tie.
        for (coord, scale) in [
            (0.0f32, 1.0f32),
            (1.0, 1.0),
            (512.0, UI_SCALE),
            (768.0, 0.8),
            (-256.0, 1.25),
            (1.0 / 1024.0, 1.0), // 1024·coord == 1.0 exactly
        ] {
            let bin = run_getter(&mut emu, &f, g.start, g.end, g.off, coord, scale);
            let mine = (g.ours)(coord, scale);
            assert_eq!(
                mine.to_bits(),
                bin.to_bits(),
                "{} coord={coord} scale={scale}",
                g.name
            );
            total += 1;
        }
    }
    let mut s = 0x7a1a_5400u32;
    for g in &getters {
        for _ in 0..1500 {
            let coord = rng(&mut s, -2048.0, 2048.0);
            let scale = rng(&mut s, 0.1, 4.0);
            let bin = run_getter(&mut emu, &f, g.start, g.end, g.off, coord, scale);
            let mine = (g.ours)(coord, scale);
            assert_eq!(mine.to_bits(), bin.to_bits(), "{} fuzz", g.name);
            // confirm the kernel matches the named row (independent path coverage).
            assert_eq!(device_to_logical(coord, scale).to_bits(), bin.to_bits());
            total += 1;
        }
    }
    eprintln!(
        "luabind_7a1_getters: {total} bit-exact (GetLeft/Right/Top/Bottom device_to_logical)"
    );
}

/// GetCenter 0x7a1810 — two center blocks: midpoint (hi-lo)*0.5+lo kept LIVE f64 into the scale.
/// X-block fld[ebp-8]=right, fsub[ebp-0x10]=left; Y-block fld[ebp-0xc]=top, fsub[ebp-0x14]=bottom.
#[test]
fn luabind_7a1_get_center() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // X-block [0x7a18e8, 0x7a1916): hi=[ebp-8], lo=[ebp-0x10]. Y-block [0x7a1923, 0x7a1951):
    // hi=[ebp-0xc], lo=[ebp-0x14]. Each uses scale=[ebp-4].
    let run_block = |emu: &mut Emu,
                     start: u64,
                     end: u64,
                     hi_off: u64,
                     lo_off: u64,
                     hi: f32,
                     lo: f32,
                     scale: f32|
     -> f64 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - hi_off, hi.to_bits());
        emu.write_u32(f.ebp - lo_off, lo.to_bits());
        emu.write_u32(f.ebp - 0x4, scale.to_bits());
        emu.run(start, end);
        let esp = emu.get_reg(RegisterX86::ESP);
        read_f64(emu, esp)
    };

    let mut total = 0u32;
    for (right, left, top, bottom, scale) in [
        (0.0f32, 0.0f32, 0.0f32, 0.0f32, 1.0f32),
        (100.0, 0.0, 50.0, 0.0, 1.0),
        (1024.0, 0.0, 768.0, 0.0, 0.8),
        (1.0, 1.0, 1.0, 1.0, 1.25), // hi==lo => center==hi
        (-100.0, -200.0, 5.0, -5.0, UI_SCALE),
        (3.0, 2.0, 1.0, 0.0, 1.0), // (3-2)*0.5+2 = 2.5 ; (1-0)*0.5+0 = 0.5
    ] {
        let bx = run_block(&mut emu, 0x7a18e8, 0x7a1916, 0x8, 0x10, right, left, scale);
        let by = run_block(&mut emu, 0x7a1923, 0x7a1951, 0xc, 0x14, top, bottom, scale);
        let (mx, my) = get_center(right, left, top, bottom, scale);
        assert_eq!(
            mx.to_bits(),
            bx.to_bits(),
            "center X r={right} l={left} s={scale}"
        );
        assert_eq!(
            my.to_bits(),
            by.to_bits(),
            "center Y t={top} b={bottom} s={scale}"
        );
        // independent: the live-f64 center kernel.
        assert_eq!(
            center_to_logical(f64::from(right), f64::from(left), scale).to_bits(),
            bx.to_bits()
        );
        total += 2;
    }
    let mut s = 0x7a18_1000u32;
    for _ in 0..2000 {
        let right = rng(&mut s, -2048.0, 2048.0);
        let left = rng(&mut s, -2048.0, 2048.0);
        let top = rng(&mut s, -2048.0, 2048.0);
        let bottom = rng(&mut s, -2048.0, 2048.0);
        let scale = rng(&mut s, 0.1, 4.0);
        let bx = run_block(&mut emu, 0x7a18e8, 0x7a1916, 0x8, 0x10, right, left, scale);
        let by = run_block(&mut emu, 0x7a1923, 0x7a1951, 0xc, 0x14, top, bottom, scale);
        let (mx, my) = get_center(right, left, top, bottom, scale);
        assert_eq!(mx.to_bits(), bx.to_bits(), "center X fuzz");
        assert_eq!(my.to_bits(), by.to_bits(), "center Y fuzz");
        total += 2;
    }
    eprintln!(
        "luabind_7a1_get_center: {total} bit-exact (X+Y midpoint·1024/scale /0.8, live-f64 center)"
    );
}

/// GetWidth 0x7a1e00 / GetHeight 0x7a2030 — width/height = (hi-lo) f32, then point_to_logical
/// (NO scale divide: fmul[1024]; fmul[ebp-4]=width; fstp; /0.8). Slice starts at the fsub compute.
#[test]
fn luabind_7a1_get_width_height() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // GetWidth [0x7a1ee6, 0x7a1f0c): hi=[ebp-8], lo=[ebp-0x10]. (writes width into [ebp-4]).
    // GetHeight [0x7a2116, 0x7a213e): hi=[ebp-0xc], lo=[ebp-0x14].
    let run =
        |emu: &mut Emu, start: u64, end: u64, hi_off: u64, lo_off: u64, hi: f32, lo: f32| -> f64 {
            set_frame(emu, &f);
            emu.write_u32(f.ebp - hi_off, hi.to_bits());
            emu.write_u32(f.ebp - lo_off, lo.to_bits());
            emu.run(start, end);
            let esp = emu.get_reg(RegisterX86::ESP);
            read_f64(emu, esp)
        };

    let mut total = 0u32;
    for (hi, lo) in [
        (0.0f32, 0.0f32),
        (100.0, 0.0),
        (1024.0, 0.0),       // width == 1024 => 1024·1024 ...
        (1.0, 1.0),          // width == 0
        (1.0 / 1024.0, 0.0), // 1024·width == 1.0 exactly
        (-5.0, -10.0),       // width == 5
    ] {
        let bw = run(&mut emu, 0x7a1ee6, 0x7a1f0c, 0x8, 0x10, hi, lo);
        let bh = run(&mut emu, 0x7a2116, 0x7a213e, 0xc, 0x14, hi, lo);
        assert_eq!(
            get_width(hi, lo).to_bits(),
            bw.to_bits(),
            "GetWidth hi={hi} lo={lo}"
        );
        assert_eq!(
            get_height(hi, lo).to_bits(),
            bh.to_bits(),
            "GetHeight hi={hi} lo={lo}"
        );
        // independent: width = (hi-lo) f32 then point_to_logical (no scale).
        let width = (f64::from(hi) - f64::from(lo)) as f32;
        assert_eq!(point_to_logical(width).to_bits(), bw.to_bits());
        total += 2;
    }
    let mut s = 0x7a1e_0000u32;
    for _ in 0..2000 {
        let hi = rng(&mut s, -2048.0, 2048.0);
        let lo = rng(&mut s, -2048.0, 2048.0);
        let bw = run(&mut emu, 0x7a1ee6, 0x7a1f0c, 0x8, 0x10, hi, lo);
        let bh = run(&mut emu, 0x7a2116, 0x7a213e, 0xc, 0x14, hi, lo);
        assert_eq!(get_width(hi, lo).to_bits(), bw.to_bits(), "GetWidth fuzz");
        assert_eq!(get_height(hi, lo).to_bits(), bh.to_bits(), "GetHeight fuzz");
        total += 2;
    }
    eprintln!("luabind_7a1_get_width_height: {total} bit-exact (width/height = (hi-lo)·1024 /0.8, no scale)");
}

/// GetPoint 0x7a2340 — two anchor coords (edi+4 / edi+8), each point_to_logical (NO scale divide).
/// The slice starts at call 0x41ad70 with the coord already loaded by `fmul [edi+coord]`; so we
/// put the coord in [edi+4]/[edi+8] (edi = this). Path A (0x7a2483/0x7a24ac) and path B
/// (0x7a24e7/0x7a250b) are byte-identical in the FP slice — both diffed.
#[test]
fn luabind_7a1_get_point() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (start, end, coord member offset). The slice is call 0x41ad70; fmul[1024]; fmul[edi+off];
    // fstp[esp]; call 0x41ae40; fstp qword[esp]. End = the `mov ecx,esi` after fstp qword.
    let blocks: [(u64, u64, u64, &str); 4] = [
        (0x7a2483, 0x7a24a0, 0x4, "A.x"),
        (0x7a24a7, 0x7a24c4, 0x8, "A.y"),
        (0x7a24e7, 0x7a2504, 0x4, "B.x"),
        (0x7a250b, 0x7a2528, 0x8, "B.y"),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, off: u64, coord: f32| -> f64 {
        set_frame(emu, &f);
        emu.write_u32(f.this + off, coord.to_bits());
        emu.run(start, end);
        let esp = emu.get_reg(RegisterX86::ESP);
        read_f64(emu, esp)
    };

    let mut total = 0u32;
    for &(start, end, off, label) in &blocks {
        for coord in [0.0f32, 1.0, -1.0, 512.0, 1.0 / 1024.0, -2048.5] {
            let bin = run(&mut emu, start, end, off, coord);
            assert_eq!(
                point_to_logical(coord).to_bits(),
                bin.to_bits(),
                "GetPoint {label} coord={coord}"
            );
            total += 1;
        }
        let mut s = 0x7a23_4000u32 ^ (start as u32);
        for _ in 0..800 {
            let coord = rng(&mut s, -4096.0, 4096.0);
            let bin = run(&mut emu, start, end, off, coord);
            assert_eq!(
                point_to_logical(coord).to_bits(),
                bin.to_bits(),
                "GetPoint {label} fuzz"
            );
            total += 1;
        }
    }
    // get_point composes the two coords; check the pair against the A-path blocks.
    for (x, y) in [(0.0f32, 0.0f32), (10.0, -20.0), (512.0, 768.0)] {
        let bx = run(&mut emu, 0x7a2483, 0x7a24a0, 0x4, x);
        let by = run(&mut emu, 0x7a24a7, 0x7a24c4, 0x8, y);
        let (mx, my) = get_point(x, y);
        assert_eq!(mx.to_bits(), bx.to_bits(), "get_point.x");
        assert_eq!(my.to_bits(), by.to_bits(), "get_point.y");
        total += 2;
    }
    eprintln!(
        "luabind_7a1_get_point: {total} bit-exact (4 anchor coords point_to_logical, paths A+B)"
    );
}

/// Setters: SetWidth 0x7a1f20 / SetHeight 0x7a2150 — logical_to_device(luaArg) = (arg/1024) f32 ·0.8
/// stored f32. Slice starts at call 0x41ad70 with luaArg already at [ebp-4]; halts after the
/// final fstp[esp] (f32). SetPoint 0x7a2540 — two such, arg at [ebp-0x10]/[ebp-0xc], result fstp there.
#[test]
fn luabind_7a1_setters() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // SetWidth/SetHeight: arg at [ebp-4]; result is fstp[esp] (f32) at the slice end; read at ESP.
    let run_set_wh = |emu: &mut Emu, start: u64, end: u64, arg: f32| -> f32 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - 0x4, arg.to_bits());
        emu.run(start, end);
        let esp = emu.get_reg(RegisterX86::ESP);
        emu.read_f32(esp)
    };
    // SetPoint x: arg at [ebp-0x10], result fstp [ebp-0x10]. y: arg at [ebp-0xc], result [ebp-0xc].
    let run_set_pt = |emu: &mut Emu, start: u64, end: u64, slot_off: u64, arg: f32| -> f32 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - slot_off, arg.to_bits());
        emu.run(start, end);
        emu.read_f32(f.ebp - slot_off)
    };

    let mut total = 0u32;
    for arg in [
        0.0f32,
        1.0,
        1024.0, // 1024/1024 == 1.0 exactly
        2048.0,
        -512.0,
        0.8,
        1.0 / 0.8,
    ] {
        let bw = run_set_wh(&mut emu, 0x7a1fce, 0x7a1fe9, arg);
        let bh = run_set_wh(&mut emu, 0x7a21fe, 0x7a2219, arg);
        let bpx = run_set_pt(&mut emu, 0x7a28a6, 0x7a28c0, 0x10, arg);
        let bpy = run_set_pt(&mut emu, 0x7a28cc, 0x7a28e6, 0xc, arg);
        assert_eq!(set_width(arg).to_bits(), bw.to_bits(), "SetWidth arg={arg}");
        assert_eq!(
            set_height(arg).to_bits(),
            bh.to_bits(),
            "SetHeight arg={arg}"
        );
        assert_eq!(
            logical_to_device(arg).to_bits(),
            bw.to_bits(),
            "kernel arg={arg}"
        );
        assert_eq!(
            set_point(arg, arg).0.to_bits(),
            bpx.to_bits(),
            "SetPoint.x arg={arg}"
        );
        assert_eq!(
            set_point(arg, arg).1.to_bits(),
            bpy.to_bits(),
            "SetPoint.y arg={arg}"
        );
        total += 4;
    }
    let mut s = 0x7a1f_2000u32;
    for _ in 0..2500 {
        let arg = rng(&mut s, -4096.0, 4096.0);
        let bw = run_set_wh(&mut emu, 0x7a1fce, 0x7a1fe9, arg);
        let bh = run_set_wh(&mut emu, 0x7a21fe, 0x7a2219, arg);
        let bpx = run_set_pt(&mut emu, 0x7a28a6, 0x7a28c0, 0x10, arg);
        let bpy = run_set_pt(&mut emu, 0x7a28cc, 0x7a28e6, 0xc, arg);
        assert_eq!(set_width(arg).to_bits(), bw.to_bits(), "SetWidth fuzz");
        assert_eq!(set_height(arg).to_bits(), bh.to_bits(), "SetHeight fuzz");
        assert_eq!(
            set_point(arg, arg).0.to_bits(),
            bpx.to_bits(),
            "SetPoint.x fuzz"
        );
        assert_eq!(
            set_point(arg, arg).1.to_bits(),
            bpy.to_bits(),
            "SetPoint.y fuzz"
        );
        total += 4;
    }
    eprintln!("luabind_7a1_setters: {total} bit-exact (SetWidth/Height/Point logical_to_device)");
}
