//! The weather driver arithmetic against the exe. `ray_box_clip` (`0x6dc5a0`) runs whole.
//! `0x67be40` and `0x67c150` are mostly calls, so their arithmetic runs as the exe's own
//! instructions between two addresses (a slice) or, for a value the exe leaves wide in a register
//! into a compare, as an island: the slice's bytes copied from the image with a `fstp` appended
//! that captures the value (as a double where the exe keeps it wide).
//!
//! Host double (`docs/METHOD.md#the-harness`): `wx_67be40_owned_fp_difftest` runs `fpatan` at
//! `0x67bede`. The emulator computes each in host `double`, so where its result reaches a compared
//! value the host's math stands in for the x87's, and the test shows the code around it.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::wx_driver::{
    ray_box_clip, wx_abs_diff, wx_manager_planar_len, wx_pos_delta, wx_vec3_sqlen,
    wx_velocity_average, wx_velocity_sum, wx_wind_angle, wx_wind_mag2, wx_wind_mag_live_sqrt,
    WX_DOT_THRESHOLD, WX_DRIFT_EPS,
};
use common::{assert_f32, fuzz_f32, next};

/// The wrap added to a negative heading.
const G_C7B554: u64 = 0x00c7_b554;
/// The previous camera position.
const G_C7B4FC: u64 = 0x00c7_b4fc;
/// The previous listener position.
const G_C7B544: u64 = 0x00c7_b544;

/// Build a copied-byte island: read `len` bytes from the emulator's mapped `.text` at `src` (the
/// binary's own slice — byte-exact, no hand transcription), append `fstp dword [slot]`, write the
/// island to a scratch page, run it from a clean FP state, and return the captured f32 at `slot`.
/// The caller sets any registers/memory the slice reads (esi/ecx/ebp + the operand memory) first.
/// Run the copied bytes `src..src+len` then a `fstp dword` into `slot` and return the stored value.
fn island_f32(emu: &mut Emu, island: u64, slot: u64, src: u64, len: usize) -> f32 {
    let mut code = emu.read(src, len);
    code.push(0xd9);
    code.push(0x1d);
    code.extend_from_slice(&(slot as u32).to_le_bytes());
    emu.write(island, &code);
    emu.run(island, island + code.len() as u64);
    emu.read_f32(slot)
}

/// Compare two wide values bit for bit, any two NaN equal.
fn assert_f64(binary: f64, ours: f64, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={binary:e} ours={ours:e}"
    );
}

/// [`island_f32`] with a `fstp qword`, which keeps all 53 bits of a value the exe leaves wide.
fn island_f64(emu: &mut Emu, island: u64, slot: u64, src: u64, len: usize) -> f64 {
    let mut code = emu.read(src, len);
    code.push(0xdd);
    code.push(0x1d);
    code.extend_from_slice(&(slot as u32).to_le_bytes());
    emu.write(island, &code);
    emu.run(island, island + code.len() as u64);
    let mut b = [0u8; 8];
    b.copy_from_slice(&emu.read(slot, 8));
    f64::from_bits(u64::from_le_bytes(b))
}

/// `0x6dc5a0` whole: the point inside, a segment crossing each axis, both ends outside on one side,
/// the entry plane on a face, parallel segments, NaN coordinates, a point box, the hit point at
/// `bmin - 1e-5` and `bmax + 1e-5` and the f32 neighbours either side (the exe compares them wide),
/// and a fuzz.
#[test]
fn ray_box_clip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let bx = emu.alloc(0x18);
    let qp = emu.alloc(0x10);
    let pp = emu.alloc(0x10);
    let mut s = 0x7261_7962_u32;

    let run =
        |emu: &mut Emu, p: [f32; 3], q: [f32; 3], bmin: [f32; 3], bmax: [f32; 3], tag: &str| {
            for i in 0..3u64 {
                emu.write_u32(bx + 4 * i, bmin[i as usize].to_bits());
                emu.write_u32(bx + 0xc + 4 * i, bmax[i as usize].to_bits());
                emu.write_u32(qp + 4 * i, q[i as usize].to_bits());
                emu.write_u32(pp + 4 * i, p[i as usize].to_bits());
            }
            let binary = emu.call(0x6dc5a0, bx as u32, qp as u32, &[pp as u32]);
            let ours = ray_box_clip(p, q, bmin, bmax);
            assert_eq!(
                binary, ours,
                "{tag}: P={p:?} Q={q:?} min={bmin:?} max={bmax:?}"
            );
        };

    let bmin = [-1.0f32, -2.0, -3.0];
    let bmax = [1.0f32, 2.0, 3.0];
    run(
        &mut emu,
        [10.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        bmin,
        bmax,
        "Q-inside",
    );
    run(
        &mut emu,
        [-10.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        bmin,
        bmax,
        "cross-x",
    );
    run(
        &mut emu,
        [0.0, -10.0, 0.0],
        [0.0, 10.0, 0.0],
        bmin,
        bmax,
        "cross-y",
    );
    run(
        &mut emu,
        [5.0, 5.0, 5.0],
        [3.0, 3.0, 3.0],
        bmin,
        bmax,
        "both-outside-miss",
    );
    run(
        &mut emu,
        [-5.0, 0.0, 0.0],
        [-3.0, 0.0, 0.0],
        bmin,
        bmax,
        "both-below-x",
    );
    run(
        &mut emu,
        [5.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
        bmin,
        bmax,
        "both-above-x",
    );
    run(
        &mut emu,
        [10.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        bmin,
        bmax,
        "Q-on-maxface",
    );
    run(
        &mut emu,
        [10.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        bmin,
        bmax,
        "Q-on-minface",
    );
    run(
        &mut emu,
        [5.0, 0.0, 0.0],
        [5.0, 0.0, 0.0],
        bmin,
        bmax,
        "parallel-out-x",
    );
    run(
        &mut emu,
        [0.5, 0.0, 0.0],
        [0.5, 5.0, 0.0],
        bmin,
        bmax,
        "parallel-in-x",
    );
    run(
        &mut emu,
        [-2.0, -3.0, -4.0],
        [0.0, 0.0, 0.0],
        bmin,
        bmax,
        "diag-corner",
    );
    run(
        &mut emu,
        [f32::NAN, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        bmin,
        bmax,
        "nan-Px",
    );
    run(
        &mut emu,
        [0.0, 0.0, 0.0],
        [f32::NAN, 0.0, 0.0],
        bmin,
        bmax,
        "nan-Qx",
    );
    run(
        &mut emu,
        [-5.0, 0.0, 0.0],
        [5.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "point-box",
    );

    let eps = f64::from(f32::from_bits(0x3727_c5ac));
    let around = |centre: f64| -> Vec<f32> {
        let base = (centre as f32).to_bits() as i32;
        (-4..=4)
            .map(|d| f32::from_bits((base + d) as u32))
            .collect()
    };
    let box_min = [0.0f32, 1.0, 0.0];
    let box_max = [1.0f32, 2.0, 1.0];
    for p1 in around(1.0 - eps) {
        run(
            &mut emu,
            [0.0, p1, 0.5],
            [-1.0, 1.0, 0.5],
            box_min,
            box_max,
            "hit point near bmin - 1e-5",
        );
    }
    for p1 in around(2.0 + eps) {
        run(
            &mut emu,
            [0.0, p1, 0.5],
            [-1.0, 2.0, 0.5],
            box_min,
            box_max,
            "hit point near bmax + 1e-5",
        );
    }
    for _ in 0..800 {
        let c = |s: &mut u32| (f64::from(next(s) >> 8) / f64::from(1u32 << 24) * 8.0 - 4.0) as f32;
        let half = |s: &mut u32| (f64::from(next(s) >> 8) / f64::from(1u32 << 24) * 3.0) as f32;
        let cx = c(&mut s);
        let cy = c(&mut s);
        let cz = c(&mut s);
        let bmin = [cx - half(&mut s), cy - half(&mut s), cz - half(&mut s)];
        let bmax = [cx + half(&mut s), cy + half(&mut s), cz + half(&mut s)];
        let mut p = [c(&mut s) * 1.5, c(&mut s) * 1.5, c(&mut s) * 1.5];
        let mut q = [c(&mut s) * 1.5, c(&mut s) * 1.5, c(&mut s) * 1.5];
        match (next(&mut s) >> 8) % 6 {
            0 => q[(next(&mut s) >> 8) as usize % 3] = bmax[(next(&mut s) >> 8) as usize % 3],
            1 => p[(next(&mut s) >> 8) as usize % 3] = q[(next(&mut s) >> 8) as usize % 3],
            2 => q[(next(&mut s) >> 8) as usize % 3] = f32::NAN,
            3 => p[(next(&mut s) >> 8) as usize % 3] = fuzz_f32(&mut s),
            _ => {}
        }
        run(&mut emu, p, q, bmin, bmax, "fuzz");
    }
}

/// `0x67be40` slices: the wind's squared speed and its wide square root, the heading (within one
/// unit in the last place), the transport's squared and planar speed, and the two drift
/// differences, including the drift threshold exactly.
#[test]
fn wx_67be40_owned_fp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x100;
    let esi = emu.alloc(0x100);
    let island = emu.alloc(0x40);
    let slot = emu.alloc(0x10);
    let mut s = 0x7769_6e64_u32;

    let mag = |emu: &mut Emu, x: f32, y: f32| {
        let stage = |emu: &mut Emu| {
            emu.set_reg(RegisterX86::EBP, frame);
            emu.set_reg(RegisterX86::EAX, u64::from(x.to_bits()));
            emu.set_reg(RegisterX86::EDX, esi);
            emu.write_u32(frame - 0x10, y.to_bits());
        };
        stage(emu);
        emu.write_u32(frame - 0x8, 0xdead_beef);
        emu.run(0x67be8a, 0x67bea4);
        let bin_mag2 = emu.read_f32(frame - 0x8);
        assert_f32(bin_mag2, wx_wind_mag2(x, y), &format!("mag2 x={x} y={y}"));

        stage(emu);
        let bin_sqrt = island_f64(emu, island, slot, 0x67be8a, 0x67bea6 - 0x67be8a);
        assert_f64(
            bin_sqrt,
            wx_wind_mag_live_sqrt(x, y),
            &format!("mag-live-sqrt x={x} y={y}"),
        );
    };

    let mut angle_red = 0u32;
    let ang = |emu: &mut Emu, mag2_f32: f32, x: f32, y: f32, wrap: f32, red: &mut u32| {
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.write_u32(frame - 0x8, mag2_f32.to_bits());
        emu.write_u32(frame - 0x14, x.to_bits());
        emu.write_u32(frame - 0x10, y.to_bits());
        emu.write_u32(G_C7B554, wrap.to_bits());
        emu.write_u32(esi + 0x78, 0xdead_beef);
        emu.run(0x67bec7, 0x67bf09);
        let binary = emu.read_f32(esi + 0x78);
        let ours = wx_wind_angle(mag2_f32, x, y, wrap);
        assert!(
            f32_within_1ulp(binary, ours),
            "angle beyond 1 ULP m2={mag2_f32} x={x} y={y} w={wrap}: binary={:#010x} ours={:#010x}",
            binary.to_bits(),
            ours.to_bits()
        );
        if binary.to_bits() != ours.to_bits() {
            *red += 1;
        }
    };

    let sqlen = |emu: &mut Emu, v: [f32; 3]| {
        emu.set_reg(RegisterX86::EAX, esi);
        emu.write_u32(esi + 0x5c, v[0].to_bits());
        emu.write_u32(esi + 0x60, v[1].to_bits());
        emu.write_u32(esi + 0x64, v[2].to_bits());
        let binary = island_f64(emu, island, slot, 0x67bf41, 0x67bf5a - 0x67bf41);
        assert_f64(binary, wx_vec3_sqlen(v), &format!("sqlen {v:?}"));
    };

    let planar = |emu: &mut Emu, p: f32, q: f32| {
        emu.set_reg(RegisterX86::ESI, esi);
        emu.write_u32(esi + 0x5c, p.to_bits());
        emu.write_u32(esi + 0x60, q.to_bits());
        let binary = island_f32(emu, island, slot, 0x67bf6d, 0x67bf7f - 0x67bf6d);
        assert_f32(
            binary,
            wx_manager_planar_len(p, q),
            &format!("planar p={p} q={q}"),
        );
    };

    let absd = |emu: &mut Emu, a0: f32, b0: f32, a1: f32, b1: f32| {
        emu.set_reg(RegisterX86::ESI, esi);
        emu.write_u32(esi, b0.to_bits());
        emu.write_u32(esi + 8, a0.to_bits());
        let bin0 = island_f64(emu, island, slot, 0x67c0fb, 0x67c102 - 0x67c0fb);
        assert_f64(
            bin0,
            wx_abs_diff(a0, b0),
            &format!("absdiffA a={a0} b={b0}"),
        );
        emu.write_u32(esi + 0xc, b1.to_bits());
        emu.write_u32(esi + 0x14, a1.to_bits());
        let bin1 = island_f64(emu, island, slot, 0x67c10f, 0x67c117 - 0x67c10f);
        assert_f64(
            bin1,
            wx_abs_diff(a1, b1),
            &format!("absdiffB a={a1} b={b1}"),
        );
    };

    for &(x, y) in &[
        (0.0f32, 0.0),
        (1.0, 0.0),
        (0.0, 1.0),
        (0.5, -0.5),
        (3.0, 4.0),
        (1e-20, 1e-20),
    ] {
        mag(&mut emu, x, y);
    }
    for &(m, x, y) in &[
        (2.0f32, 1.0, 1.0),
        (2.0, 1.0, -1.0),
        (2.0, -1.0, -1.0),
        (0.5, 0.0, 0.0),
    ] {
        ang(&mut emu, m, x, y, 6.2831855, &mut angle_red);
    }
    for &v in &[
        [1.0f32, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        [-2.0, 0.0, 0.0],
        [0.7, 0.7, 0.7],
        [10.0, 0.0, 0.0],
    ] {
        sqlen(&mut emu, v);
    }
    sqlen(&mut emu, [WX_DOT_THRESHOLD.sqrt(), 0.0, 0.0]);
    for &(p, q) in &[(3.0f32, 4.0), (0.0, 0.0), (-1.0, 1.0), (1e10, 1e10)] {
        planar(&mut emu, p, q);
    }
    absd(&mut emu, WX_DRIFT_EPS, 0.0, 0.0, WX_DRIFT_EPS);
    absd(&mut emu, 1.0, 1.0, 2.0, 5.0);

    for _ in 0..300 {
        let (x, y) = (fuzz_f32(&mut s), fuzz_f32(&mut s));
        mag(&mut emu, x, y);
        ang(
            &mut emu,
            fuzz_f32(&mut s).abs(),
            x,
            y,
            fuzz_f32(&mut s),
            &mut angle_red,
        );
        sqlen(
            &mut emu,
            [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)],
        );
        planar(&mut emu, fuzz_f32(&mut s), fuzz_f32(&mut s));
        absd(
            &mut emu,
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
        );
    }
}

/// Whether two headings are within one unit in the last place, two NaN counting as equal: `fpatan`
/// is stood in by the host's `atan2`, which can differ from the x87 by one. The f32 values are
/// mapped onto a monotonic integer scale so the distance is right across sign and zero.
fn f32_within_1ulp(a: f32, b: f32) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    let mono = |x: f32| -> i64 {
        let bits = x.to_bits() as i32;
        if bits < 0 {
            i64::from(i32::MIN.wrapping_sub(bits))
        } else {
            i64::from(bits)
        }
    };
    (mono(a) - mono(b)).abs() <= 1
}

/// `0x67c150` slices: the camera and listener position steps, the two velocity averages over lists
/// of 0 to 4 steps, the three `fabs` of the drift test, and the wind's squared speed.
#[test]
fn wx_67c150_owned_fp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400) + 0x200;
    let ebx = emu.alloc(0x100);
    let listmem = emu.alloc(0x800);
    let island = emu.alloc(0x40);
    let slot = emu.alloc(0x10);
    let mut s = 0x706f_7364_u32;

    let posdelta = |emu: &mut Emu,
                    pos: [f32; 3],
                    prev: [f32; 3],
                    begin: u64,
                    end: u64,
                    pos_slots: [i64; 3],
                    prev_g: u64,
                    tag: &str| {
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x100);
        for i in 0..3usize {
            emu.write_u32((frame as i64 + pos_slots[i]) as u64, pos[i].to_bits());
            emu.write_u32(prev_g + 4 * i as u64, prev[i].to_bits());
        }
        for k in 0..3u64 {
            emu.write_u32(frame - 0x38 + 4 * k, 0xdead_beef);
        }
        emu.run(begin, end);
        let binary = [
            emu.read_f32(frame - 0x38),
            emu.read_f32(frame - 0x34),
            emu.read_f32(frame - 0x30),
        ];
        let ours = wx_pos_delta(pos, prev);
        for i in 0..3 {
            assert_f32(
                binary[i],
                ours[i],
                &format!("{tag}: delta[{i}] pos={pos:?} prev={prev:?}"),
            );
        }
    };

    let velocity = |emu: &mut Emu,
                    nodes: &[[f32; 3]],
                    counts: &[u32],
                    head_off: u64,
                    out_base_off: u64,
                    begin: u64,
                    end: u64| {
        emu.set_reg(RegisterX86::EBX, ebx);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x180);
        emu.set_reg(RegisterX86::EDX, ebx + 0x50);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.set_reg(RegisterX86::ECX, 0);
        let stride = 0x20u64;
        for (i, n) in nodes.iter().enumerate() {
            let base = listmem + stride * i as u64;
            let next = if i + 1 < nodes.len() {
                (listmem + stride * (i as u64 + 1)) as u32
            } else {
                0
            };
            emu.write_u32(base + 4, next);
            emu.write_u32(base + 8, n[0].to_bits());
            emu.write_u32(base + 0xc, n[1].to_bits());
            emu.write_u32(base + 0x10, n[2].to_bits());
            emu.write_u32(base + 0x14, counts[i]);
        }
        let head = if nodes.is_empty() { 0 } else { listmem as u32 };
        emu.write_u32(ebx + head_off, head);
        let out = ebx + out_base_off;
        for k in 0..3u64 {
            emu.write_u32(out + 4 * k, 0xdead_beef);
        }
        emu.write_u32(frame - 0x30, 0xdead_beef);
        emu.run(begin, end);
        let bin0 = emu.read_f32(out);
        let bin1 = emu.read_f32(out + 4);
        let bin2 = emu.read_f32(frame - 0x30);
        let (sums, sum2_f32) = wx_velocity_sum(nodes);
        let count_total: u32 = counts.iter().copied().sum();
        let ours = wx_velocity_average(sums, sum2_f32, count_total);
        assert_f32(
            bin0,
            ours[0],
            &format!("vel[0] nodes={nodes:?} counts={counts:?}"),
        );
        assert_f32(
            bin1,
            ours[1],
            &format!("vel[1] nodes={nodes:?} counts={counts:?}"),
        );
        assert_f32(
            bin2,
            ours[2],
            &format!("vel[2] nodes={nodes:?} counts={counts:?}"),
        );
    };

    let epsabs = |emu: &mut Emu, v: f32, src: u64, len: usize, off: i64| {
        emu.set_reg(RegisterX86::EBP, frame);
        emu.write_u32((frame as i64 + off) as u64, v.to_bits());
        let binary = island_f32(emu, island, slot, src, len);
        assert_f32(binary, v.abs(), &format!("epsabs v={v} @ {src:#x}"));
    };

    let dot = |emu: &mut Emu, v: [f32; 3]| {
        emu.set_reg(RegisterX86::ECX, ebx);
        emu.write_u32(ebx, v[0].to_bits());
        emu.write_u32(ebx + 4, v[1].to_bits());
        emu.write_u32(ebx + 8, v[2].to_bits());
        let binary = island_f64(emu, island, slot, 0x67c65c, 0x67c674 - 0x67c65c);
        assert_f64(binary, wx_vec3_sqlen(v), &format!("dot {v:?}"));
    };

    let cam_slots = [-0x14i64, -0x10, -0xc];
    let lis_slots = [-0x20i64, -0x1c, -0x18];
    for &(pos, prev) in &[
        ([1.0f32, 2.0, 3.0], [0.0f32, 0.0, 0.0]),
        ([100.0, -50.0, 0.25], [99.0, -50.0, 0.25]),
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
    ] {
        posdelta(
            &mut emu, pos, prev, 0x67c37b, 0x67c3aa, cam_slots, G_C7B4FC, "cam",
        );
        posdelta(
            &mut emu, pos, prev, 0x67c458, 0x67c487, lis_slots, G_C7B544, "lis",
        );
    }

    velocity(&mut emu, &[], &[], 0x40, 0x50, 0x67c4cd, 0x67c584);
    velocity(
        &mut emu,
        &[[1.0, 2.0, 3.0]],
        &[1],
        0x40,
        0x50,
        0x67c4cd,
        0x67c584,
    );
    velocity(
        &mut emu,
        &[[1.0, 2.0, 3.0], [0.5, -0.5, 0.25], [10.0, 0.0, -1.0]],
        &[2, 3, 1],
        0x40,
        0x50,
        0x67c4cd,
        0x67c584,
    );
    velocity(
        &mut emu,
        &[[2.0, 4.0, 6.0], [1.0, 1.0, 1.0]],
        &[1, 1],
        0x4c,
        0x5c,
        0x67c587,
        0x67c620,
    );

    for &v in &[
        0.0f32,
        1.0,
        -1.0,
        WX_DRIFT_EPS,
        -WX_DRIFT_EPS,
        1e30,
        f32::NAN,
    ] {
        epsabs(&mut emu, v, 0x67c620, 8, -0x2c);
        epsabs(&mut emu, v, 0x67c638, 5, -0x28);
        epsabs(&mut emu, v, 0x67c64a, 5, -0x24);
    }
    for &v in &[
        [1.0f32, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        [WX_DOT_THRESHOLD.sqrt(), 0.0, 0.0],
        [-3.0, 4.0, 0.0],
    ] {
        dot(&mut emu, v);
    }

    for _ in 0..200 {
        let pos = [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)];
        let prev = [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)];
        posdelta(
            &mut emu, pos, prev, 0x67c37b, 0x67c3aa, cam_slots, G_C7B4FC, "cam-fuzz",
        );
        let nn = ((next(&mut s) >> 8) % 5) as usize;
        let nodes: Vec<[f32; 3]> = (0..nn)
            .map(|_| [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)])
            .collect();
        let counts: Vec<u32> = (0..nn).map(|_| (next(&mut s) >> 8) % 7).collect();
        velocity(&mut emu, &nodes, &counts, 0x40, 0x50, 0x67c4cd, 0x67c584);
        dot(
            &mut emu,
            [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)],
        );
        epsabs(&mut emu, fuzz_f32(&mut s), 0x67c620, 8, -0x2c);
    }
}
