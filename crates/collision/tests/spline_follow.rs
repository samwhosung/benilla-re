//! Stretches of `CMovement::FollowSpline` (`0x7c5490`) run as slices on a frame laid out around
//! `ebp`, `esi` the mover and `edi` its `+0x5c`: `0x7c5509` to `0x7c551c`, `0x7c562c` to
//! `0x7c566d`, `0x7c55a5` to `0x7c55ca`, `0x7c569c` to `0x7c57c6` (which calls the C runtime's
//! `acos` at `0x741820`), `0x7c57d9` to `0x7c5841` and `0x7c5841` to `0x7c5871`.
//!
//! Host double (`docs/METHOD.md#the-harness`): `spline_follow_facing_wrap` runs `fpatan` at
//! `0x7c55ab`. `spline_follow_turn` runs `fpatan` at `0x741866`. The emulator computes each in host
//! `double`, so where its result reaches a compared value the host's math stands in for the x87's,
//! and the test shows the code around it.

#![allow(clippy::unusual_byte_groupings)]

use benilla_re_collision::spline_follow::{
    facing_wrap, frac1, frac2, mat3_mul_vec, relative_out, turn, TurnResult, TWO_PI_F32,
};
use benilla_re_difftest::{Emu, RegisterX86, Wow};

/// The frame: `ebp`, the mover and `edi`.
struct Frame {
    ebp: u64,
    this: u64,
    edi: u64,
}

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// A random `f32` in `[-scale, scale)`.
fn fz(s: &mut u32, scale: f32) -> f32 {
    ((next(s) >> 9) as f32 / (1u32 << 23) as f32) * (2.0 * scale) - scale
}
/// True one time in `m`.
fn coin(s: &mut u32, m: u32) -> bool {
    (next(s) >> 8).is_multiple_of(m)
}

/// Asserts the same bits, or NaN on both sides.
fn assert_f32(b: f32, o: f32, what: &str) {
    assert_eq!(
        b.to_bits(),
        o.to_bits(),
        "{what}: binary={:#010x} ({b}) ours={:#010x} ({o})",
        b.to_bits(),
        o.to_bits()
    );
}

/// Maps the frame and sets 2π at `[0xcf5d00]`.
fn setup(emu: &mut Emu) -> Frame {
    let frame = emu.alloc(0x200);
    let this = emu.alloc(0x200);
    Frame {
        ebp: frame + 0x100,
        this,
        edi: this + 0x5c,
    }
}

/// Sets the registers and runs `begin` to `end`.
fn run_slice(emu: &mut Emu, f: &Frame, begin: u64, end: u64) {
    emu.set_reg(RegisterX86::EBP, f.ebp);
    emu.set_reg(RegisterX86::ESI, f.this);
    emu.set_reg(RegisterX86::EDI, f.edi);

    emu.set_reg(RegisterX86::ESP, 0x7000_0000 + 0x000f_0000);
    emu.run(begin, end);
}

/// Writes an `f32`.
fn wf(emu: &mut Emu, va: u64, v: f32) {
    emu.write_u32(va, v.to_bits());
}

/// `0x7c5509`: the first fraction.
#[test]
fn spline_follow_frac1() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = setup(&mut emu);

    let mut s = 0x5c54_9001u32;
    let mut checked = 0u32;

    let cases: Vec<(i32, i32)> = {
        let mut v = vec![
            (0, 1),
            (1, 1),
            (500, 1000),
            (999, 1000),
            (1, 1000),
            (1000, 1000),
            (123_456, 1000),
            (i32::MAX, 1000),
        ];
        for _ in 0..400 {
            let denom = 1 + (next(&mut s) % 100_000) as i32;
            let numer = (next(&mut s) % (denom as u32 * 2)) as i32;
            v.push((numer, denom));
        }
        v
    };

    for (numer, denom) in cases {
        emu.write_u32(f.ebp.wrapping_add(8), numer as u32);
        emu.set_reg(RegisterX86::ECX, denom as u32 as u64);
        run_slice(&mut emu, &f, 0x7c5509, 0x7c551c);
        let b = emu.read_f32(f.ebp.wrapping_add(8));
        let o = frac1(numer, denom);
        assert_f32(b, o, &format!("frac1 {numer}/{denom}"));
        checked += 1;
    }
    eprintln!("spline_follow_frac1: {checked} cases");
}

/// `0x7c562c`: the clamped fraction, at and around 0 and 1.
#[test]
fn spline_follow_frac2() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = setup(&mut emu);

    let mut s = 0x5c54_9002u32;
    let mut checked = 0u32;
    let (mut below, mut inside, mut above, mut eq0, mut eq1) = (0u32, 0u32, 0u32, 0u32, 0u32);

    let cases: Vec<(i32, i32)> = {
        let mut v = vec![
            (0, 1000),
            (1000, 1000),
            (-5, 1000),
            (2000, 1000),
            (500, 1000),
            (-1, 1),
            (1, 1),
            (3, 2),
        ];
        for _ in 0..400 {
            let denom = 1 + (next(&mut s) % 50_000) as i32;

            let numer = (next(&mut s) % (denom as u32 * 3)) as i32 - denom;
            v.push((numer, denom));
        }
        v
    };

    for (numer, denom) in cases {
        emu.write_u32(f.ebp.wrapping_add(8), numer as u32);
        emu.set_reg(RegisterX86::EAX, denom as u32 as u64);
        run_slice(&mut emu, &f, 0x7c562c, 0x7c566d);
        let b = emu.read_f32(f.ebp.wrapping_add(8));
        let o = frac2(numer, denom);
        assert_f32(b, o, &format!("frac2 {numer}/{denom}"));
        let q = numer as f64 / denom as f64;
        if q < 0.0 {
            below += 1;
        } else if q > 1.0 {
            above += 1;
        } else {
            inside += 1;
        }
        if numer == 0 {
            eq0 += 1;
        }
        if numer == denom {
            eq1 += 1;
        }
        checked += 1;
    }
    assert!(
        below > 0 && inside > 0 && above > 0 && eq0 > 0 && eq1 > 0,
        "all clamp legs + boundaries"
    );
    eprintln!(
        "spline_follow_frac2: {checked} cases ({below} <0, {inside} in, {above} >1, {eq0} ==0, {eq1} ==1)"
    );
}

/// `0x7c55a5`: `+0x1c`.
#[test]
fn spline_follow_facing_wrap() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = setup(&mut emu);

    emu.write_u32(0xcf5d00, TWO_PI_F32.to_bits());

    let mut s = 0x5c54_9003u32;
    let mut checked = 0u32;
    let (mut neg, mut pos) = (0u32, 0u32);

    let probes: Vec<(f32, f32)> = vec![
        (0.0, 1.0),
        (-0.0, 1.0),
        (0.0, -1.0),
        (-1.0, -1.0),
        (-1.0, 0.0),
        (1.0, 0.0),
    ];
    let mut cases = probes;
    for _ in 0..600 {
        cases.push((fz(&mut s, 4.0), fz(&mut s, 4.0)));
    }

    for (y, x) in cases {
        wf(&mut emu, f.ebp.wrapping_sub(0x58), y);
        wf(&mut emu, f.ebp.wrapping_sub(0x5c), x);
        wf(&mut emu, f.this + 0x1c, 0.0);
        run_slice(&mut emu, &f, 0x7c55a5, 0x7c55ca);
        let b = emu.read_f32(f.this + 0x1c);
        let o = facing_wrap(y, x);
        assert_f32(b, o, &format!("facing_wrap atan2({y},{x})"));
        if (y as f64).atan2(x as f64) < 0.0 {
            neg += 1;
        } else {
            pos += 1;
        }
        checked += 1;
    }
    assert!(neg > 0 && pos > 0, "both wrap branches");
    eprintln!("spline_follow_facing_wrap: {checked} cases ({neg} wrapped, {pos} unwrapped)");
}

/// Writes the turn's inputs to their frame slots.
fn turn_inputs(emu: &mut Emu, f: &Frame, dir: [f32; 2], sampled: [f32; 2], xform: [f32; 2]) {
    wf(emu, f.ebp.wrapping_sub(0x1c), dir[0]);
    wf(emu, f.ebp.wrapping_sub(0x18), dir[1]);

    wf(emu, f.ebp.wrapping_sub(0x14), sampled[0]);
    wf(emu, f.ebp.wrapping_sub(0x10), sampled[1]);
    wf(emu, f.ebp.wrapping_sub(0x98), xform[0]);
    wf(emu, f.ebp.wrapping_sub(0x94), xform[1]);
}

/// Reads the turn's stores.
fn turn_outputs(emu: &Emu, f: &Frame) -> TurnResult {
    TurnResult {
        dir: [
            emu.read_f32(f.ebp.wrapping_sub(0x1c)),
            emu.read_f32(f.ebp.wrapping_sub(0x18)),
        ],
        delta: [
            emu.read_f32(f.ebp.wrapping_sub(0x8)),
            emu.read_f32(f.ebp.wrapping_sub(0x4)),
        ],
        angle: emu.read_f32(f.ebp.wrapping_add(8)),
    }
}

/// Runs the turn slice and compares it with `turn`.
fn check_turn(
    emu: &mut Emu,
    f: &Frame,
    dir: [f32; 2],
    sampled: [f32; 2],
    xform: [f32; 2],
    tag: &str,
) {
    turn_inputs(emu, f, dir, sampled, xform);
    run_slice(emu, f, 0x7c569c, 0x7c57c6);
    let b = turn_outputs(emu, f);
    let o = turn(dir, sampled, xform);
    assert_f32(b.dir[0], o.dir[0], &format!("{tag} dir.x"));
    assert_f32(b.dir[1], o.dir[1], &format!("{tag} dir.y"));
    assert_f32(b.delta[0], o.delta[0], &format!("{tag} delta.x"));
    assert_f32(b.delta[1], o.delta[1], &format!("{tag} delta.y"));
    assert_f32(b.angle, o.angle, &format!("{tag} angle"));
}

/// `0x7c569c`: the half-angle and both unit vectors, with the clamps reached.
#[test]
fn spline_follow_turn() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = setup(&mut emu);

    let mut s = 0x5c54_9004u32;
    let mut checked = 0u32;
    let (mut deg_dir, mut deg_delta, mut crossneg, mut crosspos) = (0u32, 0u32, 0u32, 0u32);

    let eps = f32::from_bits(0x3480_0000);

    type TurnProbe = ([f32; 2], [f32; 2], [f32; 2], &'static str);
    let probes: Vec<TurnProbe> = vec![
        ([0.0, 0.0], [1.0, 0.0], [0.0, 0.0], "zero-dir"),
        ([1.0, 0.0], [0.5, 0.5], [0.5, 0.5], "zero-delta"),
        ([eps, 0.0], [1.0, 0.0], [0.0, 1.0], "dir-len==eps"),
        ([1.0, 0.0], [eps, 0.0], [0.0, 0.0], "delta-len==eps"),
        ([1.0, 0.0], [1.0, 0.0], [0.0, 0.0], "parallel"),
        ([1.0, 0.0], [-1.0, 0.0], [0.0, 0.0], "antiparallel"),
        ([1.0, 0.0], [0.0, 1.0], [0.0, 0.0], "orthogonal+"),
        ([1.0, 0.0], [0.0, -1.0], [0.0, 0.0], "orthogonal-"),
    ];
    for (dir, sampled, xform, tag) in probes {
        check_turn(&mut emu, &f, dir, sampled, xform, tag);
        checked += 1;
    }

    for _ in 0..1200 {
        let dir = if coin(&mut s, 7) {
            [fz(&mut s, 1.0) * 1e-6, fz(&mut s, 1.0) * 1e-6]
        } else {
            [fz(&mut s, 3.0), fz(&mut s, 3.0)]
        };
        let sampled = [fz(&mut s, 4.0), fz(&mut s, 4.0)];
        let xform = if coin(&mut s, 7) {
            [sampled[0], sampled[1]]
        } else {
            [fz(&mut s, 4.0), fz(&mut s, 4.0)]
        };
        check_turn(&mut emu, &f, dir, sampled, xform, "fuzz");

        let dl = [
            (sampled[0] as f64) - (xform[0] as f64),
            (sampled[1] as f64) - (xform[1] as f64),
        ];
        let dlen = (dl[0] * dl[0] + dl[1] * dl[1]).sqrt();
        let dirlen = ((dir[0] as f64).powi(2) + (dir[1] as f64).powi(2)).sqrt();
        if dirlen.abs() < (eps as f64) {
            deg_dir += 1;
        }
        if dlen.abs() < (eps as f64) {
            deg_delta += 1;
        }

        let cross = dl[1] * (dir[0] as f64) - (dir[1] as f64) * dl[0];
        if cross < 0.0 {
            crossneg += 1;
        } else {
            crosspos += 1;
        }
        checked += 1;
    }
    assert!(
        deg_dir > 0 && deg_delta > 0 && crossneg > 0 && crosspos > 0,
        "normalize-skip + both cross-sign branches exercised"
    );
    eprintln!(
        "spline_follow_turn: {checked} cases ({deg_dir} deg-dir, {deg_delta} deg-delta, {crossneg} cross<0, {crosspos} cross>=0)"
    );
}

/// `0x7c57d9`: the rotated vector at `+0x24`.
#[test]
fn spline_follow_mat3_mul_vec() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = setup(&mut emu);

    let mut s = 0x5c54_9005u32;
    let mut checked = 0u32;

    let m_off: [i64; 9] = [
        -0x8c, -0x88, -0x84, -0x80, -0x7c, -0x78, -0x74, -0x70, -0x6c,
    ];
    let v_off: [i64; 3] = [-0x34, -0x38, -0x3c];

    let probes: Vec<([f32; 9], [f32; 3])> = vec![
        (
            [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0],
        ),
        (
            [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            [1.0, 2.0, 3.0],
        ),
        ([1.0; 9], [1.0, 1.0, 1.0]),
    ];
    for (m, v) in &probes {
        run_matvec(&mut emu, &f, &m_off, &v_off, m, v);
        checked += 1;
    }

    for _ in 0..1000 {
        let m: [f32; 9] = core::array::from_fn(|_| fz(&mut s, 2.0));
        let v: [f32; 3] = core::array::from_fn(|_| fz(&mut s, 4.0));
        run_matvec(&mut emu, &f, &m_off, &v_off, &m, &v);
        checked += 1;
    }
    eprintln!("spline_follow_mat3_mul_vec: {checked} cases");
}

/// Writes the matrix and the vector and runs the product slice.
fn run_matvec(
    emu: &mut Emu,
    f: &Frame,
    m_off: &[i64; 9],
    v_off: &[i64; 3],
    m: &[f32; 9],
    v: &[f32; 3],
) {
    for (i, &off) in m_off.iter().enumerate() {
        wf(emu, (f.ebp as i64 + off) as u64, m[i]);
    }
    for (i, &off) in v_off.iter().enumerate() {
        wf(emu, (f.ebp as i64 + off) as u64, v[i]);
    }

    run_slice(emu, f, 0x7c57d9, 0x7c5841);
    let b: [f32; 3] = [
        emu.read_f32(f.this + 0x24),
        emu.read_f32(f.this + 0x28),
        emu.read_f32(f.this + 0x2c),
    ];
    let o = mat3_mul_vec(m, v);
    assert_f32(b[0], o[0], "matvec out.x");
    assert_f32(b[1], o[1], "matvec out.y");
    assert_f32(b[2], o[2], "matvec out.z");
}

/// `0x7c5841`: the output point.
#[test]
fn spline_follow_relative_out() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = setup(&mut emu);

    let out = emu.alloc(0x10);

    let mut s = 0x5c54_9006u32;
    let mut checked = 0u32;

    let probes: Vec<([f32; 3], [f32; 3])> = vec![
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]),
        ([5.0, -5.0, 0.0], [-5.0, 5.0, 10.0]),
    ];
    let mut cases = probes;
    for _ in 0..500 {
        cases.push((
            [fz(&mut s, 1000.0), fz(&mut s, 1000.0), fz(&mut s, 1000.0)],
            [fz(&mut s, 1000.0), fz(&mut s, 1000.0), fz(&mut s, 1000.0)],
        ));
    }

    for (pos, reference) in cases {
        wf(&mut emu, f.ebp.wrapping_sub(0x2c), pos[0]);
        wf(&mut emu, f.ebp.wrapping_sub(0x28), pos[1]);
        wf(&mut emu, f.ebp.wrapping_sub(0x24), pos[2]);
        wf(&mut emu, f.this + 0x44, reference[0]);
        wf(&mut emu, f.this + 0x48, reference[1]);
        wf(&mut emu, f.this + 0x4c, reference[2]);
        emu.write_u32(f.ebp.wrapping_add(0xc), out as u32);
        run_slice(&mut emu, &f, 0x7c5841, 0x7c5871);
        let b: [f32; 3] = [
            emu.read_f32(out),
            emu.read_f32(out + 4),
            emu.read_f32(out + 8),
        ];
        let o = relative_out(&pos, &reference);
        assert_f32(b[0], o[0], "relative_out.x");
        assert_f32(b[1], o[1], "relative_out.y");
        assert_f32(b[2], o[2], "relative_out.z");
        checked += 1;
    }
    eprintln!("spline_follow_relative_out: {checked} cases");
}
