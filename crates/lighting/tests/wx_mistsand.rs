//! The arithmetic of `0x67a1b0`, `0x67a7a0` and `0x679e50` against the exe, each stretch run as the
//! exe's own instructions between two addresses on a frame laid out in the emulator. A value the
//! exe leaves wide from a call (the ramp result `0x67add0` in `0x679e50`) is loaded into an island
//! from a double. The three span tests of `0x67a7a0` run in place to the loop's exit and read the
//! exe's result from the loop counter.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::wx_mistsand::{
    mist_basis_count, mist_gate_rejects, mist_node_accumulate, mist_node_spacing, mist_tex_scroll,
    sand_clamp_span, sand_clamp_value, sand_color_repack, sand_ramp_arg, sand_transform, MistBasis,
    SandTransform,
};
use common::next;

/// Compare two f32 bit for bit, any two NaN equal.
fn assert_field(binary: f32, ours: f32, tag: &str, field: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{tag}.{field}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// A fuzzed f32: random bits, and finite values of several scales and signs.
fn fuzz(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 6;
    let bits = next(s);
    match pick {
        0 => f32::from_bits(bits),
        1 => (f64::from(bits >> 4) / f64::from(1u32 << 20) - 128.0) as f32,
        2 => (f64::from(bits >> 8) / f64::from(1u32 << 23) * 2000.0 - 1000.0) as f32,
        3 => (f64::from(bits >> 8) / f64::from(1u32 << 23) * 2.0 - 1.0) as f32,
        4 => f32::from_bits(0x3f80_0000 | (bits >> 9)),
        _ => -f32::from_bits(0x3f00_0000 | (bits >> 9)),
    }
}

/// A fuzzed f64: random bits, and small finite values.
fn fuzz_f64(s: &mut u32) -> f64 {
    let hi = next(s);
    let lo = next(s);
    match (next(s) >> 8) % 4 {
        0 => f64::from_bits((u64::from(hi) << 32) | u64::from(lo)),
        1 => f64::from(hi) / f64::from(1u32 << 24) * 4.0 - 2.0,
        2 => f64::from(hi as i32) / 1.0e6,
        _ => f64::from(fuzz(s)),
    }
}

/// Build an island that loads each of `inputs` as a double onto the x87 stack and jumps to
/// `target`; returns its entry.
fn build_island(emu: &mut Emu, inputs: &[f64], target: u64) -> u64 {
    let mut code: Vec<u8> = Vec::new();
    for &v in inputs {
        let slot = emu.alloc(8);
        emu.write(slot, &v.to_bits().to_le_bytes());
        code.push(0xdd);
        code.push(0x05);
        code.extend_from_slice(&(slot as u32).to_le_bytes());
    }
    let island = emu.alloc((code.len() + 5) as u64);
    let jmp_addr = island + code.len() as u64;
    let rel = (target as i64 - (jmp_addr as i64 + 5)) as i32;
    code.push(0xe9);
    code.extend_from_slice(&rel.to_le_bytes());
    emu.write(island, &code);
    island
}

/// `0x67a1b0` from `0x67a411` to `0x67a437`: the texture scroll for ordinary, equal, NaN, infinite
/// and huge inputs, and a fuzz.
#[test]
fn mist_tex_scroll_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x6d69_7374_u32;

    let run = |emu: &mut Emu, dy: f32, dx: f32, tag: &str| {
        emu.write_u32(frame - 0xc, dy.to_bits());
        emu.write_u32(frame - 0x14, dx.to_bits());
        emu.write_u32(frame - 0x28, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x67a411, 0x67a437);
        let binary = emu.read_f32(frame - 0x28);
        let ours = mist_tex_scroll(dy, dx);
        assert_field(binary, ours, tag, "scroll");
    };

    run(&mut emu, 1.0, 0.0, "one");
    run(&mut emu, 0.0, 0.0, "zero");
    run(&mut emu, 2.0, 1.0, "diff");
    run(&mut emu, f32::NAN, 0.0, "nan");
    run(&mut emu, f32::INFINITY, f32::INFINITY, "inf-inf");
    run(&mut emu, 0.333_333_34, 0.1, "subf32");
    for _ in 0..400 {
        run(&mut emu, fuzz(&mut s), fuzz(&mut s), "fuzz");
    }
}

/// `0x67a7a0` from `0x67a7f3` to `0x67a872`: the steps and node count for 3-4-5, a unit basis, a
/// zero basis (`inv` infinite), a zero length, NaN, huge and sub-f32 inputs, and a fuzz.
#[test]
fn mist_basis_count_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x200);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x800) + 0x400;
    let mut s = 0x6261_7369_u32;

    let run = |emu: &mut Emu, a: f32, b: f32, q: f32, sv: f32, tag: &str| {
        emu.write_u32(obj + 0x114, a.to_bits());
        emu.write_u32(obj + 0x118, b.to_bits());
        emu.write_u32(frame + 8, q.to_bits());
        emu.write_u32(frame - 4, sv.to_bits());
        emu.write_u32(frame - 0x14, 0xdead_beef);
        emu.write_u32(frame - 0x10, 0xdead_beef);
        emu.write_u32(frame - 0xc, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x67a7f3, 0x67a872);
        let binary = MistBasis {
            step_a: emu.read_f32(frame - 0x14),
            step_b: emu.read_f32(frame - 0x10),
            step_z: emu.read_f32(frame - 0xc),
            count: emu.get_reg(RegisterX86::EAX) as u32 as i32,
        };
        let ours = mist_basis_count(a, b, q, sv);
        assert_field(binary.step_a, ours.step_a, tag, "step_a");
        assert_field(binary.step_b, ours.step_b, tag, "step_b");
        assert_field(binary.step_z, ours.step_z, tag, "step_z");
        assert_eq!(
            binary.count, ours.count,
            "{tag}.count: binary={} ours={}",
            binary.count, ours.count
        );
    };

    run(&mut emu, 3.0, 4.0, 2.0, 1.0, "3-4");
    run(&mut emu, 1.0, 0.0, 1.0, 1.0, "unit");
    run(&mut emu, 0.0, 0.0, 1.0, 1.0, "zero-len");
    run(&mut emu, 1.0, 1.0, 1.0, 0.0, "s0");
    run(&mut emu, f32::NAN, 1.0, 1.0, 1.0, "nan-a");
    run(&mut emu, 1.0e20, 1.0e20, 1.0, 1.0e20, "huge");
    run(&mut emu, 0.333_333_34, 0.111_111_1, 0.7, 2.5, "subf32");
    for _ in 0..400 {
        run(
            &mut emu,
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            "fuzz",
        );
    }
}

/// `0x67a7a0` from `0x67a8a4` to `0x67a8cb`: the position step for zero, half, NaN and huge values
/// and a fuzz.
#[test]
fn mist_node_accumulate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x200);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x6163_6375_u32;

    let run = |emu: &mut Emu, pos: [f32; 3], step: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x20, pos[0].to_bits());
        emu.write_u32(frame - 0x1c, pos[1].to_bits());
        emu.write_u32(frame - 0x18, pos[2].to_bits());
        emu.write_u32(frame - 0x14, step[0].to_bits());
        emu.write_u32(frame - 0x10, step[1].to_bits());
        emu.write_u32(frame - 0xc, step[2].to_bits());
        emu.write_u32(obj + 0x104, 0x10);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.set_reg(RegisterX86::EBX, obj + 4);
        emu.run(0x67a8a4, 0x67a8cb);
        let binary = [
            emu.read_f32(frame - 0x20),
            emu.read_f32(frame - 0x1c),
            emu.read_f32(frame - 0x18),
        ];
        let ours = mist_node_accumulate(pos, step);
        assert_field(binary[0], ours[0], tag, "px");
        assert_field(binary[1], ours[1], tag, "py");
        assert_field(binary[2], ours[2], tag, "pz");
    };

    run(&mut emu, [0.0, 0.0, 0.0], [1.0, 2.0, 3.0], "fromzero");
    run(&mut emu, [1.0, 1.0, 1.0], [0.5, 0.5, 0.5], "half");
    run(&mut emu, [f32::NAN, 0.0, 0.0], [1.0, 1.0, 1.0], "nan");
    run(&mut emu, [1.0e30, 0.0, 0.0], [1.0e30, 0.0, 0.0], "huge");
    for _ in 0..300 {
        let pos = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let step = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        run(&mut emu, pos, step, "fuzz");
    }
}

/// `0x67a7a0` from `0x67a8e4` to the exit at `0x67a933`: each of the three span tests against its
/// limit, with the span exactly at the limit, one to three f32 steps either side of it (the sum is
/// not f32, so the exe's wide compare differs from a narrowed one), NaN, infinite and fuzzed
/// inputs; the exe's reject or accept is read from the loop counter.
#[test]
fn mist_gate_diff_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rec = emu.alloc(0x40);
    let obj = emu.alloc(0x200);
    let frame = emu.alloc(0x40) + 0x20;
    let stack = emu.alloc(0x100) + 0x80;
    let mut s = 0x6761_7465_u32;
    emu.write_u32(obj + 0x104, 4);
    emu.write_u32(frame, 0);
    emu.write_u32(frame + 4, 0x67a933);

    let sites: [(u64, f32); 3] = [(0xc, 1.0), (0x8, 0.75), (0x4, 0.5)];
    let mut rejected_seen = 0u32;
    let mut run = |emu: &mut Emu, lo: f32, hi: f32, tag: &str| {
        for &(off, limit) in &sites {
            for k in [0u64, 0x4, 0x8, 0xc] {
                emu.write_u32(rec + k, lo.to_bits());
            }
            emu.write_u32(rec + off, hi.to_bits());
            emu.set_reg(RegisterX86::ECX, 0);
            emu.set_reg(RegisterX86::EDX, rec);
            emu.set_reg(RegisterX86::ESI, obj);
            emu.set_reg(RegisterX86::EBP, frame);
            emu.set_reg(RegisterX86::ESP, stack);
            emu.run(0x67a8e4, 0x67a933);
            let rejected = emu.get_reg(RegisterX86::ECX) == 0;
            assert_eq!(
                rejected,
                mist_gate_rejects(hi, lo, limit),
                "{tag}: gate at +{off:#x} (limit {limit}) lo={lo} hi={hi}"
            );
            if rejected {
                rejected_seen += 1;
            }
        }
    };

    run(&mut emu, 0.0, 1.0, "span equals 1");
    run(&mut emu, 0.0, 0.75, "span equals 0.75");
    run(&mut emu, 0.0, 0.5, "span equals 0.5");
    run(&mut emu, 0.0, 1.000_000_1, "just over 1");
    run(&mut emu, 1.0, 1.0, "empty span");
    run(&mut emu, f32::NAN, 1.0, "NaN lo");
    run(&mut emu, 0.0, f32::NAN, "NaN hi");
    run(&mut emu, f32::INFINITY, f32::INFINITY, "infinite span");
    for _ in 0..200 {
        let lo = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 10.0 - 5.0) as f32;
        for limit in [1.0f64, 0.75, 0.5] {
            let wide = f64::from(lo) + limit;
            let base = (wide as f32).to_bits() as i32;
            for d in -3..=3 {
                run(
                    &mut emu,
                    lo,
                    f32::from_bits((base + d) as u32),
                    "span around the limit",
                );
            }
        }
    }
    for _ in 0..200 {
        run(&mut emu, fuzz(&mut s), fuzz(&mut s), "fuzz");
    }
    assert!(
        rejected_seen > 100,
        "the gates reject in many cases: {rejected_seen}"
    );
}

/// `0x67a7a0` from `0x67a939` to `0x67a949`: `(idx / count) * s` for zero, one and fuzzed indices,
/// a zero count (infinite), and NaN.
#[test]
fn mist_node_spacing_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x200);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x40) + 0x20;
    let mut s = 0x7370_6163_u32;

    let run = |emu: &mut Emu, idx1: i32, count: i32, sv: f32, tag: &str| {
        emu.write_u32(frame + 0xc, idx1 as u32);
        emu.write_u32(obj + 0x104, count as u32);
        emu.write_u32(frame - 4, sv.to_bits());
        emu.write_u32(espbase - 4, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x67a939, 0x67a949);
        let binary = emu.read_f32(espbase - 4);
        let ours = mist_node_spacing(idx1, count, sv);
        assert_field(binary, ours, tag, "spacing");
    };

    run(&mut emu, 1, 4, 1.0, "quarter");
    run(&mut emu, 2, 4, 2.0, "half");
    run(&mut emu, 0, 4, 1.0, "zero-idx");
    run(&mut emu, 1, 1, 1.0, "div1");
    run(&mut emu, 3, 0, 1.0, "div0");
    run(&mut emu, 1, 3, f32::NAN, "nan-s");
    for _ in 0..300 {
        let idx1 = (next(&mut s) >> 8) as i32 % 65;
        let count = ((next(&mut s) >> 8) as i32 % 64) + 1;
        run(&mut emu, idx1, count, fuzz(&mut s), "fuzz");
    }
}

/// `0x679e50` from `0x67a013` to `0x67a022`: the clamp span and value.
#[test]
fn sand_clamp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x636c_6d70_u32;

    let run = |emu: &mut Emu, mid: f32, lo: f32, hi: f32, tag: &str| {
        emu.write_u32(frame - 0x14, mid.to_bits());
        emu.write_u32(frame - 0x18, lo.to_bits());
        emu.write_u32(frame - 8, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x67a013, 0x67a01c);
        let bspan = emu.read_f32(frame - 8);
        assert_field(bspan, sand_clamp_span(mid, lo), tag, "span");

        emu.write_u32(frame - 0xc, hi.to_bits());
        emu.write_u32(frame - 0x18, lo.to_bits());
        emu.write_u32(frame - 4, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x67a01c, 0x67a025);
        let bval = emu.read_f32(frame - 4);
        assert_field(bval, sand_clamp_value(hi, lo), tag, "value");
    };

    run(&mut emu, 1.0, 0.0, 0.5, "basic");
    run(&mut emu, 0.0, 0.0, 0.0, "zero");
    run(&mut emu, f32::NAN, 0.0, 1.0, "nan");
    run(&mut emu, 0.333_333_34, 0.1, 0.7, "subf32");
    for _ in 0..300 {
        run(&mut emu, fuzz(&mut s), fuzz(&mut s), fuzz(&mut s), "fuzz");
    }
}

/// `0x679e50` from `0x67a04e` to `0x67a0a2`: the three components, whose x, y and z sums are
/// narrowed at different points.
#[test]
fn sand_transform_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mbase = emu.alloc(0x40) + 0x20;
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7872_666d_u32;

    #[allow(clippy::too_many_arguments)]
    let run = |emu: &mut Emu, c: f32, m: [f32; 3], t: [f32; 3], o: [f32; 3], tag: &str| {
        emu.write_u32(frame - 4, c.to_bits());
        emu.write_u32(mbase - 8, m[0].to_bits());
        emu.write_u32(mbase - 4, m[1].to_bits());
        emu.write_u32(mbase, m[2].to_bits());
        emu.write_u32(mbase - 0x14, t[0].to_bits());
        emu.write_u32(mbase - 0x10, t[1].to_bits());
        emu.write_u32(mbase - 0xc, t[2].to_bits());
        emu.write_u32(frame - 0x34, o[0].to_bits());
        emu.write_u32(frame - 0x30, o[1].to_bits());
        emu.write_u32(frame - 0x2c, o[2].to_bits());
        emu.write_u32(frame - 0x40, 0xdead_beef);
        emu.write_u32(frame - 0x3c, 0xdead_beef);
        emu.write_u32(frame - 0x38, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, mbase);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x67a04e, 0x67a0a5);
        let binary = SandTransform {
            o40: emu.read_f32(frame - 0x40),
            o3c: emu.read_f32(frame - 0x3c),
            o38: emu.read_f32(frame - 0x38),
        };
        let ours = sand_transform(c, m[0], m[1], m[2], t[0], t[1], t[2], o[0], o[1], o[2]);
        assert_field(binary.o40, ours.o40, tag, "o40");
        assert_field(binary.o3c, ours.o3c, tag, "o3c");
        assert_field(binary.o38, ours.o38, tag, "o38");
    };

    run(
        &mut emu,
        1.0,
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "ident",
    );
    run(
        &mut emu,
        0.5,
        [2.0, 3.0, 4.0],
        [1.0, 1.0, 1.0],
        [0.5, 0.5, 0.5],
        "basic",
    );
    run(
        &mut emu,
        f32::NAN,
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "nan-c",
    );
    run(
        &mut emu,
        0.333_333_34,
        [0.111_111_1, 0.142_857_15, 0.166_666_67],
        [0.7, 0.3, 0.9],
        [0.123_456_79, 0.987_654_3, 0.555_555_6],
        "subf32",
    );
    for _ in 0..400 {
        let m = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let t = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let o = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        run(&mut emu, fuzz(&mut s), m, t, o, "fuzz");
    }
}

/// `0x679e50` from `0x67a0a5` to `0x67a0b1`: the span less 0.2.
#[test]
fn sand_ramp_arg_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let espslot = emu.alloc(0x40) + 0x20;
    let mut s = 0x726d_7061_u32;

    let run = |emu: &mut Emu, v: f32, tag: &str| {
        emu.write_u32(frame - 8, v.to_bits());
        emu.write_u32(espslot, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espslot);
        emu.run(0x67a0a5, 0x67a0b1);
        let binary = emu.read_f32(espslot);
        assert_field(binary, sand_ramp_arg(v), tag, "arg");
    };

    run(&mut emu, 0.2, "eq-bias");
    run(&mut emu, 1.0, "one");
    run(&mut emu, 0.0, "zero");
    run(&mut emu, f32::NAN, "nan");
    run(&mut emu, 0.333_333_34, "subf32");
    for _ in 0..300 {
        run(&mut emu, fuzz(&mut s), "fuzz");
    }
}

/// `0x679e50` from `0x67a0cf` to `0x67a0eb` with the `0x67add0` result loaded as a double: the
/// colour word for doubles over the whole bit space and the span.
#[test]
fn sand_color_repack_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let scratch = emu.alloc(0x20);
    let mut s = 0x636f_6c72_u32;

    let run = |emu: &mut Emu, e: f64, v: f32, tag: &str| {
        emu.write_u32(frame - 8, v.to_bits());
        emu.write_u32(frame - 0x40, 0);
        emu.write_u32(frame - 0x38, 0);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::EDI, scratch);
        let island = build_island(emu, &[e], 0x67a0cf);
        emu.run(island, 0x67a0eb);
        let binary = emu.read_f32(frame - 8);
        let ours = sand_color_repack(e, v);
        assert_field(binary, ours, tag, "repack");
    };

    run(&mut emu, 1.0, 1.0, "one");
    run(&mut emu, 0.0, 1.0, "zero");
    run(&mut emu, 0.5, 2.0, "half");
    run(&mut emu, -1.0, 1.0, "neg");
    run(&mut emu, f64::INFINITY, 0.0, "inf-zero");
    run(&mut emu, f64::NAN, 1.0, "nan");
    run(&mut emu, 0.1, f32::from_bits(0x3f8c_cccd), "subf32");
    for _ in 0..300 {
        run(&mut emu, fuzz_f64(&mut s), fuzz(&mut s), "fuzz");
    }
}
