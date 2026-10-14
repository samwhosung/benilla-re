//! The float leaves of the rain, snow and sand drivers against the exe. Each runs the instructions
//! of one stretch of its function between two addresses, on a frame laid out in the emulator, and
//! compares the stored value bit for bit; the emitters, whose three copies sit at different stack
//! offsets, are each run at all their sites. The ground height lookups `0x67c820` and `0x67ca90`
//! run whole.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::mist::{cell_index_fill, tile_cell_fill, Tile, FLT_MAX_SENTINEL};
use benilla_re_lighting::wx_leaf_fp::{
    emit_count_clamp, emit_phase_advance, emit_phase_inc, height_sample_bias, rain_drop_t,
    rain_patter_vec, rain_wind_emits_patter, rain_wind_mag2, recenter_tile_coords,
    snow_patter_value, update_frame_quota, update_interval_div, RainDrop,
};
use common::{assert_f32, fuzz_f32, next, straddling_y};

/// Run one count-clamp site with the slots and request written to its frame and compare the stored
/// count.
#[allow(clippy::too_many_arguments)]
fn diff_count_clamp(
    emu: &mut Emu,
    label: &str,
    begin: u64,
    until: u64,
    frame: u64,
    slots_off: i64,
    req_off: i64,
    n_off: i64,
    slots: i64,
    requested: f32,
) {
    let ebp = frame + 0x200;
    let at = |o: i64| (ebp as i64 + o) as u64;
    emu.write_u32(at(slots_off), slots as u32);
    emu.write_u32(at(slots_off + 4), 0);
    emu.write_u32(at(req_off), requested.to_bits());
    emu.write_u32(at(n_off), 0xdead_beef);
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.run(begin, until);
    let binary = emu.read_u32(at(n_off)) as i32;
    let ours = emit_count_clamp(slots, requested);
    assert_eq!(
        binary, ours,
        "{label} slots={slots} requested={requested}: binary={binary} ours={ours}"
    );
}

/// The emit count at the rain, snow and sand sites (`0x6754cc`, `0x677e1c`, `0x67995c`): `slots`
/// against `requested` strictly below, equal and above, an exact odd count (a tie that rounds
/// down), NaN and infinite requests, requests beyond `i32`, and a fuzz.
#[test]
fn wl_emit_count_clamp() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let mut s = 0x636c_6d70_u32;

    let sites: [(&str, u64, u64, i64, i64, i64); 3] = [
        ("rain 0x6754a0", 0x6754cc, 0x6754f2, -0x10, 0x10, 0x8),
        ("snow 0x677df0", 0x677e1c, 0x677e42, -0x14, 0xc, -0x8),
        ("sand 0x679930", 0x67995c, 0x679982, -0x14, 0xc, -0x8),
    ];

    let run_all = |emu: &mut Emu, slots: i64, requested: f32| {
        for &(label, begin, until, so, ro, no) in &sites {
            diff_count_clamp(
                emu, label, begin, until, frame, so, ro, no, slots, requested,
            );
        }
    };

    run_all(&mut emu, 4096, 4096.0);
    let edges: [(i64, f32); 8] = [
        (0, 10.0),
        (100, 99.5),
        (100, 100.0),
        (100, 100.5),
        (0x1800, 1.0),
        (1, 0.4),
        (5, 5.5),
        (6, 5.5),
    ];
    for &(slots, req) in &edges {
        run_all(&mut emu, slots, req);
    }
    let special: [(i64, f32); 14] = [
        (7, 100.0),
        (6, 100.0),
        (3, 4096.0),
        (5, 5.0),
        (0x1800, 1.0e30),
        (100, f32::NAN),
        (100, f32::INFINITY),
        (100, f32::NEG_INFINITY),
        (100, -3.0e9),
        (100, 3.0e9),
        (0, 0.5),
        (0, 1.5),
        (0, -0.0),
        (1, 1.5),
    ];
    for &(slots, req) in &special {
        run_all(&mut emu, slots, req);
    }
    for _ in 0..200 {
        let slots = i64::from((next(&mut s) >> 8) % 0x1801);
        let requested = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 6144.0) as f32;
        run_all(&mut emu, slots, requested);
    }
}

/// Run one phase-increment site and compare the pushed argument.
#[allow(clippy::too_many_arguments)]
fn diff_phase_inc(
    emu: &mut Emu,
    label: &str,
    begin: u64,
    until: u64,
    frame: u64,
    stack: u64,
    m_off: i64,
    g_off: i64,
    phase_off: u64,
    cnt_off: u64,
    m: f32,
    g: f32,
    phase: f32,
) {
    let ebp = frame + 0x200;
    let obj = frame + 0x40000;
    let at = |o: i64| (ebp as i64 + o) as u64;
    emu.write_u32(at(m_off), m.to_bits());
    emu.write_u32(at(g_off), g.to_bits());
    emu.write_u32(obj + phase_off, phase.to_bits());
    emu.write_u32(obj + cnt_off, 0);
    if m_off != 0x8 {
        emu.write_u32(at(0x8), obj as u32);
    }
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.set_reg(RegisterX86::EDI, obj);
    emu.set_reg(RegisterX86::ESI, obj);
    let esp0 = stack + 0x200;
    emu.set_reg(RegisterX86::ESP, esp0);
    emu.run(begin, until);
    let esp = emu.get_reg(RegisterX86::ESP);
    let binary = emu.read_f32(esp);
    let ours = emit_phase_inc(m, g, phase);
    assert_f32(binary, ours, &format!("{label} m={m} g={g} phase={phase}"));
}

/// The spawn phase `((m - 1) * g + phase)` at the three emitters (`0x67551c`, `0x677f21`,
/// `0x679a5c`).
#[test]
fn wl_emit_phase_inc() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x60000);
    let stack = emu.alloc(0x400);
    let mut s = 0x7068_6173_u32;

    let sites: [(&str, u64, u64, i64, i64, u64, u64); 3] = [
        (
            "rain 0x6754a0",
            0x67551c,
            0x675542,
            0x8,
            -0x8,
            0x36018,
            0x3600c,
        ),
        (
            "snow 0x677df0",
            0x677f21,
            0x677f5b,
            0xc,
            -0xc,
            0x30018,
            0x3000c,
        ),
        (
            "sand 0x679930",
            0x679a5c,
            0x679a91,
            0xc,
            -0xc,
            0x30018,
            0x3000c,
        ),
    ];

    let run_all = |emu: &mut Emu, m: f32, g: f32, phase: f32| {
        for &(label, begin, until, mo, go, po, co) in &sites {
            diff_phase_inc(
                emu, label, begin, until, frame, stack, mo, go, po, co, m, g, phase,
            );
        }
    };

    run_all(&mut emu, 1.0, 0.016, 1234.5);
    let edges: [(f32, f32, f32); 6] = [
        (1.0, 0.0, 0.0),
        (2.0, 0.016, 1000.0),
        (1.5, 0.033, -500.0),
        (1.0, -1.0, 7.0),
        (1.999, 0.016, 1e6),
        (1.25, 1.0e-6, 1e-20),
    ];
    for &(m, g, p) in &edges {
        run_all(&mut emu, m, g, p);
    }
    for _ in 0..150 {
        let m = f32::from_bits((next(&mut s) & 0x7fffff) | 0x3f80_0000);
        let g = fuzz_f32(&mut s);
        let phase = fuzz_f32(&mut s);
        run_all(&mut emu, m, g, phase);
    }
}

/// Run one phase-advance site and compare the stored phase.
#[allow(clippy::too_many_arguments)]
fn diff_phase_advance(
    emu: &mut Emu,
    label: &str,
    begin: u64,
    until: u64,
    frame: u64,
    g_off: i64,
    phase_off: u64,
    phase: f32,
    g: f32,
) {
    let ebp = frame + 0x200;
    let edi = frame + 0x40000;
    let at = |o: i64| (ebp as i64 + o) as u64;
    emu.write_u32(at(g_off), g.to_bits());
    emu.write_u32(edi + phase_off, phase.to_bits());
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.set_reg(RegisterX86::EDI, edi);
    emu.run(begin, until);
    let binary = emu.read_f32(edi + phase_off);
    let ours = emit_phase_advance(phase, g);
    assert_f32(binary, ours, &format!("{label} phase={phase} g={g}"));
}

/// The phase advance `g + phase` after the loop at the three emitters.
#[test]
fn wl_emit_phase_advance() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x60000);
    let mut s = 0x6164_7663_u32;

    let sites: [(&str, u64, u64, i64, u64); 3] = [
        ("rain 0x6754a0", 0x675686, 0x675695, -0x8, 0x36018),
        ("snow 0x677df0", 0x677fd0, 0x677fdf, -0xc, 0x30018),
        ("sand 0x679930", 0x679ae5, 0x679af4, -0xc, 0x30018),
    ];

    let run_all = |emu: &mut Emu, phase: f32, g: f32| {
        for &(label, begin, until, go, po) in &sites {
            let edi = frame + 0x40000;
            emu.set_reg(RegisterX86::ESI, edi);
            diff_phase_advance(emu, label, begin, until, frame, go, po, phase, g);
        }
    };

    run_all(&mut emu, 1000.0, 0.0);
    let edges: [(f32, f32); 5] = [
        (0.0, 0.016),
        (1e6, 0.5),
        (-500.0, 0.033),
        (12345.6, -0.01),
        (1e-20, 1e-20),
    ];
    for &(p, g) in &edges {
        run_all(&mut emu, p, g);
    }
    for _ in 0..150 {
        let phase = fuzz_f32(&mut s);
        let g = fuzz_f32(&mut s);
        run_all(&mut emu, phase, g);
    }
}

/// The phase advance fed back on itself for thousands of frames, where rounding at each step shows
/// (a ramp from 0, a large phase that absorbs small steps, a negative step).
#[test]
fn wl_emit_phase_advance_trajectory() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x60000);
    let ebp = frame + 0x200;
    let edi = frame + 0x40000;
    const PHASE_OFF: u64 = 0x36018;

    let configs: [(&str, f32, f32, usize); 3] = [
        ("phase ramp from 0", 0.05, 0.0, 8000),
        ("large phase (g near f32-absorption)", 0.5, 1.0e6, 4000),
        ("negative-g drift down", -0.033, 500.0, 8000),
    ];

    for (label, g, phase0, steps) in configs {
        emu.write_u32(ebp - 8, g.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::EDI, edi);

        let r = emu.trajectory(
            steps,
            vec![phase0.to_bits()],
            |emu, st| {
                emu.write_u32(edi + PHASE_OFF, st[0]);
                emu.run(0x675686, 0x675695);
                vec![emu.read_u32(edi + PHASE_OFF)]
            },
            |st| vec![emit_phase_advance(f32::from_bits(st[0]), g).to_bits()],
        );
        r.assert_match(&format!("emit_phase_advance trajectory [{label}]"));
    }
}

/// `-(2.0 / f14) + f20` at `0x675548`: zero, negative and infinite divisors and a fuzz.
#[test]
fn wl_rain_drop_t() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let drop = emu.alloc(0x40);
    let mut s = 0x6472_7074_u32;
    let ebp = frame + 0x200;

    let run = |emu: &mut Emu, f14: f32, f20: f32, ctx: &str| {
        emu.write_u32(drop + 0x14, f14.to_bits());
        emu.write_u32(drop + 0x20, f20.to_bits());
        emu.write_u32(ebp + 8, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESI, drop);
        emu.set_reg(RegisterX86::EDI, frame + 0x800);
        emu.run(0x675548, 0x67555f);
        let binary = emu.read_f32(ebp + 8);
        let ours = rain_drop_t(f14, f20);
        assert_f32(binary, ours, ctx);
    };

    run(&mut emu, 2.0, 5.0, "probe f14=2.0");
    let edges: [(f32, f32); 8] = [
        (1.0, 0.0),
        (4.0, 100.0),
        (-2.0, 50.0),
        (0.0, 1.0),
        (-0.0, 1.0),
        (f32::INFINITY, 1.0),
        (0.5, -10.0),
        (1000.0, 0.001),
    ];
    for (i, &(a, b)) in edges.iter().enumerate() {
        run(&mut emu, a, b, &format!("edge {i} ({a},{b})"));
    }
    for case in 0..400 {
        let a = fuzz_f32(&mut s);
        let b = fuzz_f32(&mut s);
        run(&mut emu, a, b, &format!("case {case} ({a},{b})"));
    }
}

/// The rain wind gate (`0x6755a9` to `0x6755ce`): the squared length 2.0 exactly, either side of
/// it, NaN and infinite winds, and a fuzz; the flags the exe leaves in `ah` are compared with the
/// transcription.
#[test]
fn wl_rain_wind_gate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    const MGR_GLOBAL: u64 = 0xc6326c;
    let mgr = emu.alloc(0x90);
    emu.write_u32(MGR_GLOBAL, mgr as u32);
    let mut s = 0x7769_6e67_u32;

    let run = |emu: &mut Emu, x: f32, y: f32, z: f32, ctx: &str| {
        emu.write_u32(mgr + 0x5c, x.to_bits());
        emu.write_u32(mgr + 0x60, y.to_bits());
        emu.write_u32(mgr + 0x64, z.to_bits());
        emu.set_reg(RegisterX86::EAX, mgr);
        emu.run(0x6755a9, 0x6755ce);
        let ah = ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8;
        let binary_emit = (ah & 0x41) != 0;
        let mag2 = rain_wind_mag2(x, y, z);
        let ours_emit = rain_wind_emits_patter(mag2);
        assert_eq!(
            binary_emit, ours_emit,
            "{ctx} mag2={mag2} ah={ah:#04x}: binary_emit={binary_emit} ours_emit={ours_emit}"
        );
    };

    run(&mut emu, 1.0, 1.0, 0.0, "probe mag2==2.0");
    let edges: [(f32, f32, f32); 8] = [
        (0.0, 0.0, 0.0),
        (1.0, 0.0, 0.0),
        (1.5, 0.0, 0.0),
        (1.41, 0.0, 0.0),
        (0.8, 0.8, 0.8),
        (f32::NAN, 1.0, 0.0),
        (f32::INFINITY, 0.0, 0.0),
        (-1.0, -1.0, 0.0),
    ];
    for (i, &(x, y, z)) in edges.iter().enumerate() {
        run(&mut emu, x, y, z, &format!("edge {i} ({x},{y},{z})"));
    }
    for case in 0..400 {
        let (x, y, z) = match (next(&mut s) >> 8) % 3 {
            0 => {
                let c =
                    |s: &mut u32| (f64::from(next(s) >> 8) / f64::from(1u32 << 24) * 2.0) as f32;
                (c(&mut s), c(&mut s), c(&mut s))
            }
            1 => (fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)),
            _ => (
                f32::from_bits(next(&mut s)),
                f32::from_bits(next(&mut s)),
                f32::from_bits(next(&mut s)),
            ),
        };
        run(&mut emu, x, y, z, &format!("case {case}"));
    }
}

/// The rain patter point (`0x6755d9` to `0x67562f`): a zero delta, extremes and a fuzz, where the x
/// product stays wide and the y and z products round twice.
#[test]
fn wl_rain_patter_vec() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let drop = emu.alloc(0x40);
    let stack = emu.alloc(0x400);
    let mut s = 0x7074_7672_u32;
    let ebp = frame + 0x200;
    let out = |o: i64| (ebp as i64 + o) as u64;

    let run = |emu: &mut Emu, d: &RainDrop, ctx: &str| {
        emu.write_u32(drop, d.pos[0].to_bits());
        emu.write_u32(drop + 4, d.pos[1].to_bits());
        emu.write_u32(drop + 8, d.pos[2].to_bits());
        emu.write_u32(drop + 0xc, d.dir[0].to_bits());
        emu.write_u32(drop + 0x10, d.dir[1].to_bits());
        emu.write_u32(drop + 0x14, d.dir[2].to_bits());
        emu.write_u32(drop + 0x1c, d.f1c.to_bits());
        emu.write_u32(drop + 0x20, d.f20.to_bits());
        for o in [-0x20i64, -0x1c, -0x18] {
            emu.write_u32(out(o), 0xdead_beef);
        }
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESI, drop);
        emu.set_reg(RegisterX86::EBX, frame + 0x100000);
        emu.write_u32(frame + 0x100000 + 0x2400c, 0);
        emu.set_reg(RegisterX86::ESP, stack + 0x200);
        emu.run(0x6755d9, 0x67562f);
        let ours = rain_patter_vec(d);
        let bx = emu.read_f32(out(-0x20));
        let by = emu.read_f32(out(-0x1c));
        let bz = emu.read_f32(out(-0x18));
        assert_f32(bx, ours[0], &format!("{ctx} out.x"));
        assert_f32(by, ours[1], &format!("{ctx} out.y"));
        assert_f32(bz, ours[2], &format!("{ctx} out.z"));
    };

    run(
        &mut emu,
        &RainDrop {
            pos: [11.0, 22.0, 33.0],
            dir: [1.0, 2.0, 3.0],
            f1c: 7.5,
            f20: 7.5,
        },
        "probe delta==0",
    );
    let edges: [RainDrop; 4] = [
        RainDrop {
            pos: [0.0, 0.0, 0.0],
            dir: [1.0, 1.0, 1.0],
            f1c: 0.0,
            f20: 1.0,
        },
        RainDrop {
            pos: [-100.0, 200.0, -300.0],
            dir: [0.5, -0.5, 2.0],
            f1c: -1.0,
            f20: 1.0,
        },
        RainDrop {
            pos: [1.000_000_1, 1.000_000_1, 1.000_000_1],
            dir: [1.000_000_1, 1.000_000_1, 1.000_000_1],
            f1c: 0.0,
            f20: 1.000_000_1,
        },
        RainDrop {
            pos: [3.3, -3.3, 3.3],
            dir: [7.7, -7.7, 7.7],
            f1c: 2.2,
            f20: 9.9,
        },
    ];
    for (i, d) in edges.iter().enumerate() {
        run(&mut emu, d, &format!("edge {i}"));
    }
    for case in 0..400 {
        let d = RainDrop {
            pos: [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)],
            dir: [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)],
            f1c: fuzz_f32(&mut s),
            f20: fuzz_f32(&mut s),
        };
        run(&mut emu, &d, &format!("case {case}"));
    }
}

/// The snow patter value (`0x677f61`): zero, -0, the smallest positive value, NaN, infinity and a
/// fuzz.
#[test]
fn wl_snow_patter_value() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let flake = emu.alloc(0x40);
    let mut s = 0x736e_7076_u32;
    let ebp = frame + 0x200;

    let run = |emu: &mut Emu, field: f32, ctx: &str| {
        emu.write_u32(flake + 0x1c, field.to_bits());
        emu.write_u32(ebp + 0xc, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESI, flake);
        emu.set_reg(RegisterX86::EDI, frame + 0x800);
        emu.run(0x677f61, 0x677f8b);
        let binary = emu.read_f32(ebp + 0xc);
        let ours = snow_patter_value(field);
        assert_f32(binary, ours, ctx);
    };

    run(&mut emu, 0.0, "probe field==0.0");
    run(&mut emu, -0.0, "probe field==-0.0");
    run(&mut emu, f32::from_bits(0x0000_0001), "probe tiny +eps");
    let edges: [f32; 7] = [1.0, -1.0, 0.25, 100.0, -100.0, f32::NAN, f32::INFINITY];
    for (i, &x) in edges.iter().enumerate() {
        run(&mut emu, x, &format!("edge {i} x={x}"));
    }
    for case in 0..400 {
        let x = match (next(&mut s) >> 8) % 3 {
            0 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0) as f32,
            1 => fuzz_f32(&mut s),
            _ => f32::from_bits(next(&mut s)),
        };
        run(&mut emu, x, &format!("case {case} x={x}"));
    }
}

/// Run one frame-quota site and compare the stored quota.
#[allow(clippy::too_many_arguments)]
fn diff_dt_accum(
    emu: &mut Emu,
    label: &str,
    begin: u64,
    until: u64,
    frame: u64,
    field_off: u64,
    slot_off: i64,
    g: f32,
    field: f32,
) {
    const G_GLOBAL: u64 = 0xc62510;
    let ebp = frame + 0x200;
    let obj = frame + 0x800;
    emu.write_u32(G_GLOBAL, g.to_bits());
    emu.write_u32(obj + field_off, field.to_bits());
    let slot = (ebp as i64 + slot_off) as u64;
    emu.write_u32(slot, 0xdead_beef);
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.set_reg(RegisterX86::ESI, obj);
    emu.run(begin, until);
    let binary = emu.read_f32(slot);
    let ours = update_frame_quota(g, field);
    assert_f32(binary, ours, &format!("{label} g={g} field={field}"));
}

/// The frame quota at the rain and sand updates (`0x675863`, `0x679c77`): the cap exactly, one ulp
/// either side, zero, negative and NaN frame times, and a fuzz.
#[test]
fn wl_update_dt_accum() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x1000);
    let mut s = 0x6474_6163_u32;

    let sites: [(&str, u64, u64, u64, i64); 2] = [
        ("rain 0x6756a0", 0x675863, 0x67588d, 0xcc, -0xc),
        ("sand 0x679b00", 0x679c77, 0x679ca1, 0x9c, -0x8),
    ];

    let run_all = |emu: &mut Emu, g: f32, field: f32| {
        for &(label, begin, until, fo, so) in &sites {
            diff_dt_accum(emu, label, begin, until, frame, fo, so, g, field);
        }
    };

    let cap = f32::from_bits(0x3c88_893b);
    run_all(&mut emu, cap, 1000.0);
    run_all(&mut emu, f32::from_bits(0x3c88_893a), 1000.0);
    run_all(&mut emu, f32::from_bits(0x3c88_893c), 1000.0);
    let edges: [(f32, f32); 6] = [
        (0.0, 100.0),
        (0.008, 60.0),
        (1.0, 60.0),
        (-1.0, 60.0),
        (f32::NAN, 60.0),
        (0.01, -250.0),
    ];
    for &(g, field) in &edges {
        run_all(&mut emu, g, field);
    }
    for _ in 0..200 {
        let g = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 0.05) as f32;
        let field = fuzz_f32(&mut s);
        run_all(&mut emu, g, field);
    }
}

/// Run one interval site and compare the pushed argument.
#[allow(clippy::too_many_arguments)]
fn diff_interval_div(
    emu: &mut Emu,
    label: &str,
    begin: u64,
    until: u64,
    frame: u64,
    stack: u64,
    field_off: u64,
    g: f32,
    field: f32,
) {
    let ebp = frame + 0x200;
    let obj = frame + 0x800;
    let scratch = frame + 0x2000;
    emu.write_u32(0xc62510, g.to_bits());
    emu.write_u32(obj + field_off, field.to_bits());
    emu.write_u32((ebp as i64 - 8) as u64, scratch as u32);
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.set_reg(RegisterX86::ESI, obj);
    emu.set_reg(RegisterX86::EAX, scratch);
    emu.set_reg(RegisterX86::EDX, scratch + 0x100);
    emu.set_reg(RegisterX86::EBX, scratch + 0x200);
    emu.set_reg(RegisterX86::EDI, scratch + 0x300);
    let esp0 = stack + 0x200;
    emu.set_reg(RegisterX86::ESP, esp0);
    emu.run(begin, until);
    let esp = emu.get_reg(RegisterX86::ESP);
    let binary = emu.read_f32(esp);
    let ours = update_interval_div(field);
    assert_f32(binary, ours, &format!("{label} field={field}"));
}

/// The interval `6144 / field` at the rain and sand updates (`0x675944`, `0x679da8`): positive,
/// zero, -0, negative and infinite fields and a fuzz.
#[test]
fn wl_update_interval_div() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x4000);
    let stack = emu.alloc(0x400);
    for addr in [0x67d6a0u64, 0x67df40, 0x67dca0, 0x67d740, 0x67e0a0] {
        emu.stub(addr, 0, 0);
    }
    let mut s = 0x6964_6976_u32;

    let sites: [(&str, u64, u64, u64); 2] = [
        ("rain 0x6756a0", 0x675944, 0x675976, 0xcc),
        ("sand 0x679b00", 0x679da8, 0x679dda, 0x9c),
    ];

    let run_all = |emu: &mut Emu, field: f32| {
        for &(label, begin, until, fo) in &sites {
            diff_interval_div(emu, label, begin, until, frame, stack, fo, 0.016, field);
        }
    };

    run_all(&mut emu, 6144.0);
    let edges: [f32; 8] = [1.0, 2.0, 0.5, -3.0, 0.0, -0.0, f32::INFINITY, 1024.0];
    for (i, &field) in edges.iter().enumerate() {
        run_all(&mut emu, field);
        let _ = i;
    }
    for _ in 0..400 {
        let field = fuzz_f32(&mut s);
        run_all(&mut emu, field);
    }
}

/// `0x67c7f0` whole with the ground query stubbed to find nothing: `z - 200` for z around 200,
/// infinite and huge.
#[test]
fn wl_height_sample_bias() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let world = emu.alloc(0x20);
    let mut s = 0x6873_6d70_u32;

    emu.stub(0x6b7070, 0, 1);
    let run_miss = |emu: &mut Emu, wz: f32, ctx: &str| {
        emu.write_u32(world, 0);
        emu.write_u32(world + 4, 0);
        emu.write_u32(world + 8, wz.to_bits());
        let binary = emu.call_f32(0x67c7f0, world as u32, 0, &[world as u32]);
        let ours = height_sample_bias(wz);
        assert_f32(binary, ours, ctx);
    };

    run_miss(&mut emu, 200.0, "probe wz==200.0");
    let edges: [f32; 6] = [0.0, 100.0, 300.0, -50.0, f32::INFINITY, 1e30];
    for (i, &wz) in edges.iter().enumerate() {
        run_miss(&mut emu, wz, &format!("miss edge {i} wz={wz}"));
    }
    for case in 0..400 {
        let wz = fuzz_f32(&mut s);
        run_miss(&mut emu, wz, &format!("miss case {case} wz={wz}"));
    }
}

/// The tile of `0x67ce50` (`0x67ce56` to `0x67ce96`): both signs, and the y values where the
/// product stored as f32 floors differently from the wide product.
#[test]
fn wl_recenter_tile_coords() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let world = emu.alloc(0x20);
    let stack = emu.alloc(0x400);
    let mut s = 0x7263_6e74_u32;
    let ebp = frame + 0x200;

    let run = |emu: &mut Emu, x: f32, y: f32, ctx: &str| {
        emu.write_u32(world, x.to_bits());
        emu.write_u32(world + 4, y.to_bits());
        emu.write_u32(ebp + 8, world as u32);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ECX, frame + 0x800);
        emu.set_reg(RegisterX86::ESP, stack + 0x200);
        emu.run(0x67ce56, 0x67ce96);
        let bx = emu.get_reg(RegisterX86::EDI) as u32 as i32;
        let by = emu.get_reg(RegisterX86::EAX) as u32 as i32;
        let (ox, oy) = recenter_tile_coords(x, y);
        assert_eq!(bx, ox, "{ctx} tile_x");
        assert_eq!(by, oy, "{ctx} tile_y");
    };

    run(&mut emu, 100.0, 200.0, "probe");
    let edges: [(f32, f32); 8] = [
        (0.0, 0.0),
        (33.4, 66.7),
        (-33.4, -66.7),
        (1000.0, 2000.0),
        (-0.0, 0.5),
        (16.66, 16.67),
        (1.0, -1.0),
        (5000.0, -5000.0),
    ];
    for (i, &(x, y)) in edges.iter().enumerate() {
        run(&mut emu, x, y, &format!("edge {i} ({x},{y})"));
    }
    for &(y, _) in straddling_y(benilla_re_lighting::wx_leaf_fp::TILE_SCALE)
        .iter()
        .take(60)
    {
        run(&mut emu, 100.0, y, "y rounded to f32 before the floor");
    }
    for case in 0..400 {
        let c = |s: &mut u32| {
            (f64::from(next(s) >> 8) / f64::from(1u32 << 24) * 34000.0 - 17000.0) as f32
        };
        let x = c(&mut s);
        let y = c(&mut s);
        run(&mut emu, x, y, &format!("case {case}"));
    }
}

/// `0x67c820` whole through `tile_cell_fill`: a point below, left of and inside the tile, a cell
/// already filled, and a fuzz.
#[test]
fn wl_height_cache_lookup_xy() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let tile = emu.alloc(0x1008);
    let world = emu.alloc(0x10);
    let mut s = 0x6378_6c6b_u32;

    emu.stub(0x6b7070, 0, 1);

    let run = |emu: &mut Emu,
               tile_coord: (i32, i32),
               wx: f32,
               wy: f32,
               wz: f32,
               prefill: Option<f32>,
               ctx: &str| {
        let terrain = move |_x: f32, _y: f32| (f64::from(wz) - 200.0) as f32;
        emu.write_u32(tile + 0x1000, tile_coord.0 as u32);
        emu.write_u32(tile + 0x1004, tile_coord.1 as u32);
        for i in 0..0x400u64 {
            emu.write_u32(tile + 4 * i, FLT_MAX_SENTINEL.to_bits());
        }
        let mut cells = [FLT_MAX_SENTINEL; 0x400];
        if let Some(v) = prefill {
            let mut probe = cells;
            let _ = tile_cell_fill(&mut probe, tile_coord, (wx, wy), |_, _| v);
            for (i, (&a, &b)) in cells.iter().zip(probe.iter()).enumerate() {
                if a.to_bits() != b.to_bits() {
                    cells[i] = v;
                    emu.write_u32(tile + 4 * i as u64, v.to_bits());
                    break;
                }
            }
        }
        emu.write_u32(world, wx.to_bits());
        emu.write_u32(world + 4, wy.to_bits());
        emu.write_u32(world + 8, wz.to_bits());
        let binary = emu.call_f32(0x67c820, tile as u32, 0, &[world as u32]);
        let ours = tile_cell_fill(&mut cells, tile_coord, (wx, wy), terrain);
        assert_f32(binary, ours, ctx);
    };

    run(&mut emu, (0, 0), 0.0, 0.0, 50.0, None, "probe origin");
    run(
        &mut emu,
        (5, 5),
        0.0,
        0.0,
        50.0,
        None,
        "below tile -> FLT_MAX",
    );
    run(&mut emu, (0, 0), 100.0, 150.0, 12.5, None, "lazy fill");
    run(&mut emu, (0, 0), 100.0, 150.0, 12.5, Some(-7.25), "cached");
    run(&mut emu, (1, 1), 60.0, 60.0, 3.0, None, "mid cell");

    for case in 0..300 {
        let tc = (
            ((next(&mut s) >> 8) % 64) as i32 - 32,
            ((next(&mut s) >> 8) % 64) as i32 - 32,
        );
        let base_x = tc.0 as f32 * 33.33333;
        let base_y = tc.1 as f32 * 33.33333;
        let wx =
            base_x + (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 1100.0 - 50.0) as f32;
        let wy =
            base_y + (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 1100.0 - 50.0) as f32;
        let wz = fuzz_f32(&mut s);
        let prefill = if (next(&mut s) >> 8) & 1 == 1 {
            Some(fuzz_f32(&mut s))
        } else {
            None
        };
        run(&mut emu, tc, wx, wy, wz, prefill, &format!("case {case}"));
    }
}

/// `0x67ca90` whole through `cell_index_fill`: the first cell, the last cell of a tile, the next
/// tile, a filled cell, and a fuzz over the 160 by 160 grid and tile origins of both signs.
#[test]
fn wl_height_cache_lookup_cell() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let grid = emu.alloc(0x100);
    let cellxy = emu.alloc(0x10);
    let mut tiles = [0u64; 25];
    for t in &mut tiles {
        *t = emu.alloc(0x1008);
    }
    let mut s = 0x6363_656c_u32;

    emu.stub(0x6b7070, 0, 1);

    let run = |emu: &mut Emu,
               cellx: i32,
               celly: i32,
               origin: (i32, i32),
               world_z: f32,
               prefill: Option<f32>,
               ctx: &str| {
        let terrain = move |_x: f32, _y: f32| (f64::from(world_z) - 200.0) as f32;
        emu.write_u32(grid + 0x18, origin.0 as u32);
        emu.write_u32(grid + 0x1c, origin.1 as u32);
        for (i, &t) in tiles.iter().enumerate() {
            emu.write_u32(grid + 0x28 + 4 * i as u64, t as u32);
            for c in 0..0x400u64 {
                emu.write_u32(t + 4 * c, FLT_MAX_SENTINEL.to_bits());
            }
        }
        let empty = Tile {
            coord: (0, 0),
            cells: [FLT_MAX_SENTINEL; 0x400],
        };
        let mut backing: Vec<Tile> = vec![empty; 25];
        if let Some(v) = prefill {
            let mut probe = backing.clone();
            let _ = cell_index_fill(cellx, celly, origin, &mut probe, |_, _| v);
            'outer: for (ti, (a, b)) in backing.iter().zip(probe.iter()).enumerate() {
                for (ci, (&x, &y)) in a.cells.iter().zip(b.cells.iter()).enumerate() {
                    if x.to_bits() != y.to_bits() {
                        backing[ti].cells[ci] = v;
                        emu.write_u32(tiles[ti] + 4 * ci as u64, v.to_bits());
                        break 'outer;
                    }
                }
            }
        }
        emu.write_u32(cellxy, cellx as u32);
        emu.write_u32(cellxy + 4, celly as u32);
        let binary = emu.call_f32(
            0x67ca90,
            grid as u32,
            0,
            &[cellxy as u32, world_z.to_bits()],
        );
        let ours = cell_index_fill(cellx, celly, origin, &mut backing, terrain);
        assert_f32(binary, ours, ctx);
    };

    run(&mut emu, 0, 0, (0, 0), 200.0, None, "probe (0,0)");
    run(&mut emu, 31, 31, (2, 3), 50.0, None, "intra-tile max");
    run(&mut emu, 64, 64, (0, 0), 50.0, None, "next tile");
    run(&mut emu, 10, 20, (1, 1), 12.0, Some(-3.5), "cached");

    for case in 0..300 {
        let cellx = ((next(&mut s) >> 8) % 160) as i32;
        let celly = ((next(&mut s) >> 8) % 160) as i32;
        let origin = (
            ((next(&mut s) >> 8) % 64) as i32 - 32,
            ((next(&mut s) >> 8) % 64) as i32 - 32,
        );
        let wz = fuzz_f32(&mut s);
        let prefill = if (next(&mut s) >> 8) & 1 == 1 {
            Some(fuzz_f32(&mut s))
        } else {
            None
        };
        run(
            &mut emu,
            cellx,
            celly,
            origin,
            wz,
            prefill,
            &format!("case {case}"),
        );
    }
}
