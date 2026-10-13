//! Spawning and substeps (`systems/models/particles.md#spawn-count`,
//! `systems/models/particles.md#substeps`, `systems/models/particles.md#emitter-classes`,
//! `systems/models/particles.md#model-particles`): slices of the spawn, the particle and model
//! particle inits, the advance and the preallocation, each run to a store it reads back.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::particles::spawn::{
    count_subtract, integrate_count, normalize_basis9, normalize_quat4, pos_integrate,
    prealloc_count, prealloc_product_f64, rel_delta, spawn_flush_gate, spawn_random_axes,
    spawn_random_scale, spawn_sign_flip, spawn_transform, substep_count_bias, substep_floor_arg,
    substep_remainder, substep_tangent, t_scalar, weighted_point, BIAS_512, COUNT_MAX, FLUSH_EPS,
    ONE, PERIOD, ZERO,
};
use common::{assert_f32, island_bytes_match, next, SLICE_ESP};

const G0_GLOBAL: u64 = 0x00cf_58e8;

struct DrvRng(u32);

impl DrvRng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// A finite f32 in a mixed range, from the LCG's high bits.
    fn f32_mixed(&mut self) -> f32 {
        let pick = (self.next() >> 8) % 6;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => (m * 2.0 - 1.0) as f32,
            1 => (m * 200.0 - 100.0) as f32,
            2 => (m * 2.0) as f32,
            3 => (m * 0.01) as f32,
            4 => (m * 20.0 - 10.0) as f32,
            _ => (m * 4.0 - 2.0) as f32,
        }
    }
    /// A distance value spanning the R1 clamp regions (around 50, where t crosses 1.0 and 0.25).
    fn dist(&mut self) -> f32 {
        let pick = (self.next() >> 8) % 4;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => (m * 100.0) as f32,        // 0..100: spans both clamps
            1 => (m * 40.0 + 30.0) as f32,  // 30..70: near the high clamp / mid band
            2 => (m * 20.0 + 80.0) as f32,  // 80..100: low clamp region (t<0.25)
            _ => (m * 200.0 - 50.0) as f32, // wide incl. negatives (t>1.0)
        }
    }
    /// A random mantissa in `[1.0, 2.0)`, exactly as the binary forms it (`& 0x7fffff |
    /// 0x3f800000`).
    fn r_unit(&mut self) -> f32 {
        f32::from_bits((self.next() & 0x007f_ffff) | 0x3f80_0000)
    }
}

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

fn frand_mixed(s: &mut u32) -> f32 {
    let pick = (lcg(s) >> 8) % 5;
    let m = f64::from(lcg(s) >> 8) / f64::from(1u32 << 24);
    let v = match pick {
        0 => m * 2.0 - 1.0,            // [-1, 1)
        1 => m * 100.0,                // moderate
        2 => -(m * 100.0),             // negative
        3 => m * 1.0e6 - 5e5,          // large
        _ => (m * 2.0 - 1.0) * 1.0e-6, // tiny (near the flush-eps boundary)
    };
    v as f32
}

fn rnd_unit(s: &mut u32) -> f32 {
    f32::from_bits((lcg(s) & 0x007f_ffff) | 0x3f80_0000)
}

struct FinRng(u32);

impl FinRng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// A finite f32 in a mixed range, from the LCG's high bits.
    fn f32_mixed(&mut self) -> f32 {
        let pick = (self.next() >> 8) % 5;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => (m * 2.0 - 1.0) as f32,
            1 => (m * 20.0 - 10.0) as f32,
            2 => (m * 200.0 - 100.0) as f32,
            3 => (m * 0.01) as f32,
            _ => (m * 4.0 - 2.0) as f32,
        }
    }
    /// A random mantissa in `[1.0, 2.0)`, exactly as the binary forms it (`& 0x7fffff |
    /// 0x3f800000`).
    fn r_unit(&mut self) -> f32 {
        f32::from_bits((self.next() & 0x007f_ffff) | 0x3f80_0000)
    }
}

const PREALLOC_K_VA: u64 = 0x0081_d868;

/// `0x7b5556..0x7b55be`: the distance factor at its two clamps and NaN.
#[test]
fn partspawndrv_t_scalar_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x400); // ESI → emitter; field_a8 at [esi+0xa8]

    let run_case = |e: &mut Emu, g0: f32, field_a8: f32, ctx: &str| {
        e.write_u32(G0_GLOBAL, g0.to_bits());
        e.write_u32(this + 0xa8, field_a8.to_bits());
        // The slice's `mov esi,ecx` @0x7b5565 sets ESI from ECX — pass `this` in ECX (thiscall).
        e.set_reg(RegisterX86::ECX, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b5556, 0x7b55be);
        let want = t_scalar(g0, field_a8);
        assert_f32(e.read_f32(ebp - 0x4), want, ctx);
    };

    // ==threshold probes — LOW clamp (0.25): g0=87.5 ⇒ t==0.25 exactly (kept, not clamped).
    run_case(&mut e, 87.5, 3.0, "t low-clamp boundary t==0.25 (kept)");
    // just past: g0=88.0 ⇒ (88-50)*0.02=0.76 ⇒ t=0.24 < 0.25 ⇒ clamp to 0.25.
    run_case(&mut e, 88.0, 3.0, "t low-clamp t<0.25 (clamped)");
    // just before: g0=87.0 ⇒ t=0.26 > 0.25 ⇒ kept.
    run_case(&mut e, 87.0, 3.0, "t low-clamp t>0.25 (kept)");
    // ==threshold probes — HIGH clamp (1.0): g0=50.0 ⇒ t==1.0 exactly (clamp path, harmless).
    run_case(&mut e, 50.0, -2.5, "t high-clamp boundary t==1.0");
    // g0=49.0 ⇒ (49-50)*0.02=-0.02 ⇒ t=1.02 > 1.0 ⇒ clamp to 1.0.
    run_case(&mut e, 49.0, -2.5, "t high-clamp t>1.0 (clamped)");
    // g0=60.0 ⇒ t=0.8 < 1.0 ⇒ kept.
    run_case(&mut e, 60.0, -2.5, "t mid t<1.0 (kept)");
    // field_a8 == 0 → vel 0 regardless of clamp.
    run_case(&mut e, 70.0, 0.0, "t field_a8==0");
    // A NaN distance passes the low test and fails the high one: the factor is 1.
    run_case(&mut e, f32::NAN, 3.0, "NaN distance (factor 1)");

    let mut rng = DrvRng(0x7b55_5600);
    for i in 0..4000u32 {
        let g0 = rng.dist();
        let field_a8 = rng.f32_mixed();
        run_case(&mut e, g0, field_a8, &format!("t_scalar fuzz[{i}]"));
    }
}

/// `0x7b5644..0x7b56bb`, flag 0x1000: the accumulator, the spawn count and the offset.
#[test]
fn partspawndrv_integrate_count_r3path_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x400); // ESI
    let edi = e.alloc(0x100); // EDI; src at [edi+0x30+0/4/8]

    #[allow(clippy::too_many_arguments)]
    let run_case =
        |e: &mut Emu, vel: f32, dt: f32, pos: f32, src: [f32; 3], origin: [f32; 3], ctx: &str| {
            e.write_u32(ebp - 0x4, vel.to_bits());
            e.write_u32(ebp + 0x8, dt.to_bits());
            e.write_u32(this + 0x8, pos.to_bits());
            for (k, &s) in src.iter().enumerate() {
                e.write_u32(edi + 0x30 + 4 * k as u64, s.to_bits());
            }
            for (k, &o) in origin.iter().enumerate() {
                e.write_u32(this + 0x248 + 4 * k as u64, o.to_bits());
            }
            // [esi+0x1ac]: bits 0,1 SET (pass the two `je 0x7b5877`), bit 0xC SET (→ `test ah,0x10`
            // ZF=0 → `je 0x7b5670` NOT taken → R3 path). Start at 0x7b5644 so `test ah,0x10` runs.
            e.write_u32(this + 0x1ac, 0x1003);
            e.set_reg(RegisterX86::ESI, this);
            e.set_reg(RegisterX86::EDI, edi);
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.run(0x7b5644, 0x7b56bb);
            let count = e.get_reg(RegisterX86::EAX) as u32; // __ftol result (low 32 bits of the count)

            assert_f32(
                e.read_f32(this + 0x8),
                pos_integrate(vel, dt, pos),
                &format!("{ctx} newpos"),
            );
            let d = rel_delta(src, origin);
            assert_f32(e.read_f32(ebp - 0x24), d[0], &format!("{ctx} dx"));
            assert_f32(e.read_f32(ebp - 0x20), d[1], &format!("{ctx} dy"));
            assert_f32(e.read_f32(ebp - 0x1c), d[2], &format!("{ctx} dz"));
            assert_eq!(
                count,
                integrate_count(vel, dt, pos) as u32,
                "{ctx} count: binary={count:#x} ours={:#x}",
                integrate_count(vel, dt, pos)
            );
        };

    // ==threshold count probes (newpos + 0.5 at integer boundaries; __ftol truncates toward zero).
    run_case(
        &mut e,
        0.0,
        0.0,
        2.5,
        [1.0, 2.0, 3.0],
        [0.0, 0.0, 0.0],
        "count newpos==2.5 (→3)",
    );
    run_case(
        &mut e,
        0.0,
        0.0,
        2.49,
        [1.0, 2.0, 3.0],
        [0.0, 0.0, 0.0],
        "count newpos==2.49 (→2)",
    );
    run_case(
        &mut e,
        0.0,
        0.0,
        -0.5,
        [1.0, 2.0, 3.0],
        [0.0, 0.0, 0.0],
        "count newpos==-0.5 (→0)",
    );
    run_case(
        &mut e,
        0.0,
        0.0,
        -1.5,
        [1.0, 2.0, 3.0],
        [0.0, 0.0, 0.0],
        "count newpos==-1.5 (→-1)",
    );
    run_case(
        &mut e,
        2.0,
        3.0,
        0.0,
        [5.0, -4.0, 3.0],
        [1.0, 2.0, -1.0],
        "count vel*dt=6",
    );
    run_case(&mut e, 0.0, 0.0, 0.0, [0.0; 3], [0.0; 3], "all-zero");

    let mut rng = DrvRng(0x7b56_5d00);
    for i in 0..4000u32 {
        let vel = rng.f32_mixed();
        let dt = rng.f32_mixed();
        let pos = rng.f32_mixed();
        let src = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        let origin = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        run_case(&mut e, vel, dt, pos, src, origin, &format!("r2a fuzz[{i}]"));
    }
}

/// `0x7b5644..0x7b57db`: the accumulator and the spawn count, past `i32`, past `i64` and NaN.
#[test]
fn partspawndrv_integrate_count_jepath_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x400); // ESI

    let run_case = |e: &mut Emu, vel: f32, dt: f32, pos: f32, ctx: &str| {
        e.write_u32(ebp - 0x4, vel.to_bits());
        e.write_u32(ebp + 0x8, dt.to_bits());
        e.write_u32(this + 0x8, pos.to_bits());
        // [esi+0x1ac]: bits 0,1 SET (pass the two `je 0x7b5877`), bit 0xC CLEAR (→ `test ah,0x10`
        // ZF=1 → `je 0x7b5670` taken → 0x7b57d0). Start at 0x7b5644 so `test ah,0x10` runs.
        e.write_u32(this + 0x1ac, 0x3);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b5644, 0x7b57db);
        let count = e.get_reg(RegisterX86::EAX) as u32; // __ftol result (low 32 bits of the count)
        assert_f32(
            e.read_f32(this + 0x8),
            pos_integrate(vel, dt, pos),
            &format!("{ctx} newpos"),
        );
        assert_eq!(
            count,
            integrate_count(vel, dt, pos) as u32,
            "{ctx} count: binary={count:#x} ours={:#x}",
            integrate_count(vel, dt, pos)
        );
    };

    // same ==threshold count probes via the je path.
    run_case(&mut e, 0.0, 0.0, 2.5, "count newpos==2.5 (→3)");
    run_case(&mut e, 0.0, 0.0, 2.49, "count newpos==2.49 (→2)");
    run_case(&mut e, 0.0, 0.0, -0.5, "count newpos==-0.5 (→0)");
    run_case(&mut e, 0.0, 0.0, -2.5, "count newpos==-2.5 (→-2)");
    run_case(&mut e, 4.0, 0.25, 0.5, "count vel*dt+pos=1.5 (→2)");
    run_case(&mut e, 0.0, 0.0, 0.0, "all-zero (→0)");
    run_case(
        &mut e,
        1.0e30,
        1.0e10,
        0.0,
        "count past i64 (the indefinite, low dword 0)",
    );
    run_case(&mut e, 3.0e9, 1.0, 0.0, "count past i32 (the low dword)");
    run_case(&mut e, f32::NAN, 1.0, 0.0, "NaN count (0)");

    let mut rng = DrvRng(0x7b57_d000);
    for i in 0..4000u32 {
        let vel = rng.f32_mixed();
        let dt = rng.f32_mixed();
        let pos = rng.f32_mixed();
        run_case(&mut e, vel, dt, pos, &format!("r2b fuzz[{i}]"));
    }
}

/// `0x7b5676..0x7b56b0`: the offset from the last position.
#[test]
fn partspawndrv_rel_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x400); // ESI; origin at [esi+0x248/24c/250]
    let edi = e.alloc(0x100); // EDI; src at [edi+0x30+0/4/8]

    let run_case = |e: &mut Emu, src: [f32; 3], origin: [f32; 3], ctx: &str| {
        // start at 0x7b5676: the int copies `mov eax,[edi+0x30]; ...; mov [ebp-0x18],eax` (etc.)
        // load src FROM [edi+0x30+k] (writing [ebp-0x14]/[ebp-0x10] from ECX/EDX inside the slice),
        // so supply src via [edi+0x30+k], not the [ebp-..] scratch (which the slice overwrites).
        for (k, &s) in src.iter().enumerate() {
            e.write_u32(edi + 0x30 + 4 * k as u64, s.to_bits());
        }
        for (k, &o) in origin.iter().enumerate() {
            e.write_u32(this + 0x248 + 4 * k as u64, o.to_bits());
        }
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b5676, 0x7b56b0);
        let d = rel_delta(src, origin);
        assert_f32(e.read_f32(ebp - 0x24), d[0], &format!("{ctx} dx"));
        assert_f32(e.read_f32(ebp - 0x20), d[1], &format!("{ctx} dy"));
        assert_f32(e.read_f32(ebp - 0x1c), d[2], &format!("{ctx} dz"));
    };

    run_case(&mut e, [0.0; 3], [0.0; 3], "rel all-zero");
    run_case(
        &mut e,
        [1.0, 2.0, 3.0],
        [1.0, 2.0, 3.0],
        "rel src==origin (→0)",
    );
    run_case(&mut e, [5.0, -3.0, 7.0], [-1.0, 4.0, -2.0], "rel mixed");

    let mut rng = DrvRng(0x7b56_8600);
    for i in 0..4000u32 {
        let src = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        let origin = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        run_case(&mut e, src, origin, &format!("rel fuzz[{i}]"));
    }
}

/// `0x7b56f3..0x7b5730`: a spawn point between the last and the current position.
#[test]
fn partspawndrv_weighted_point_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x400); // ESI; origin at [esi+0x248/24c/250]

    let run_case = |e: &mut Emu, r: f32, d: [f32; 3], origin: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x8, r.to_bits());
        e.write_u32(ebp - 0x24, d[0].to_bits());
        e.write_u32(ebp - 0x20, d[1].to_bits());
        e.write_u32(ebp - 0x1c, d[2].to_bits());
        for (k, &o) in origin.iter().enumerate() {
            e.write_u32(this + 0x248 + 4 * k as u64, o.to_bits());
        }
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b56f3, 0x7b5730);
        // after `sub esp,0xc`: out_x=[esp], out_y=[esp+4], out_z=[esp+8] = SLICE_ESP-0xc + 0/4/8.
        let base = SLICE_ESP - 0xc;
        let out = weighted_point(r, d, origin);
        assert_f32(e.read_f32(base), out[0], &format!("{ctx} out_x"));
        assert_f32(e.read_f32(base + 4), out[1], &format!("{ctx} out_y"));
        assert_f32(e.read_f32(base + 8), out[2], &format!("{ctx} out_z"));
    };

    // boundary r: exactly 1.0 (w=0 → out==origin), and just below 2.0 (w≈1 → out≈origin+d).
    run_case(
        &mut e,
        1.0,
        [5.0, -3.0, 7.0],
        [1.0, 2.0, 3.0],
        "r==1.0 (w=0)",
    );
    let r_near2 = f32::from_bits(0x3fff_ffff); // just below 2.0
    run_case(
        &mut e,
        r_near2,
        [5.0, -3.0, 7.0],
        [1.0, 2.0, 3.0],
        "r~2.0 (w~1)",
    );
    run_case(&mut e, 1.5, [0.0; 3], [4.0, 5.0, 6.0], "d==0 (→origin)");

    let mut rng = DrvRng(0x7b56_f300);
    for i in 0..4000u32 {
        let r = rng.r_unit();
        let d = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        let origin = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        run_case(&mut e, r, d, origin, &format!("r4 fuzz[{i}]"));
    }
}

/// `0x7b586e..0x7b5877`: the accumulator less the count.
#[test]
fn partspawndrv_count_subtract_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x400); // ESI; pos at [esi+8]

    let run_case = |e: &mut Emu, pos: f32, count: i32, ctx: &str| {
        e.write_u32(ebp - 0xc, count as u32); // low dword of the fild qword
        e.write_u32(ebp - 0x8, 0); // high dword = 0
        e.write_u32(this + 0x8, pos.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b586e, 0x7b5877);
        assert_f32(e.read_f32(this + 0x8), count_subtract(pos, count), ctx);
    };

    run_case(&mut e, 3.5, 3, "pos=3.5 count=3 (→0.5)");
    run_case(&mut e, 0.0, 0, "pos=0 count=0 (→0)");
    run_case(&mut e, 100.0, 99, "pos=100 count=99 (→1)");
    run_case(&mut e, -2.5, 0, "pos=-2.5 count=0 (→-2.5)");

    let mut rng = DrvRng(0x7b58_6e00);
    for i in 0..4000u32 {
        let pos = rng.f32_mixed();
        let count = ((rng.next() >> 8) % 1000) as i32; // emit counts are small non-negative ints
        run_case(&mut e, pos, count, &format!("tl fuzz[{i}]"));
    }
}

/// `0x7b22d5..0x7b2301`: the random age and its clamp, -0, NaN and a product under the least `f32`.
#[test]
fn spawn_random_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pp = e.alloc(0x80); // edi (the particle record; out at +0x1c)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, ru: f32, scale: f32, ctx: &str| {
        e.write_u32(ebp - 0x4, ru.to_bits()); // [ebp-4] = the random-mantissa dword
        e.write_u32(ebp + 0xc, scale.to_bits()); // [ebp+0xc] = scale (param_3)
        e.set_reg(RegisterX86::EDI, pp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b22d5, 0x7b2301); // fld [ebp-4]; fsub 1.0; fmul scale; clamp; fstp [edi+0x1c]
        let bin = e.read_f32(pp + 0x1c);
        let ours = spawn_random_scale(ru, scale);
        assert_f32(bin, ours, ctx);
    };

    // probes: scale=0 (out=0), rnd=1.0 (rnd-1=0 → out=0), exacts, negative scale (clamp side).
    run_case(&mut e, 1.5, 0.0, "scale=0");
    run_case(&mut e, f32::from_bits(0x3f800000), 10.0, "rnd=1.0 (out=0)");
    run_case(&mut e, 1.5, 4.0, "exacts (0.5*4=2)");
    run_case(&mut e, 1.5, -4.0, "neg scale (clamp to 0)");
    run_case(&mut e, f32::from_bits(0x3fffffff), 100.0, "rnd~2.0");
    run_case(&mut e, 1.5, -0.0, "scale -0 (kept, compares equal)");
    run_case(
        &mut e,
        f32::from_bits(0x3f80_0001),
        f32::from_bits(0x8000_0001),
        "a negative product that rounds to -0 compares below 0 (+0)",
    );
    run_case(&mut e, 1.5, f32::NAN, "NaN scale (kept)");
    run_case(&mut e, 1.0, f32::INFINITY, "0 times infinity (NaN, kept)");

    let mut s = 0x7b22_b0b0u32;
    for i in 0..4000u32 {
        let ru = rnd_unit(&mut s);
        // scale spans signs so the `<0` clamp fires both ways.
        let scale = frand_mixed(&mut s);
        run_case(&mut e, ru, scale, &format!("randscale fuzz[{i}]"));
    }
}

/// `0x7b234a..0x7b240f`: the velocity and its flushes.
#[test]
fn spawn_transform_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emitter = e.alloc(0x300); // esi (base at +0x258/0x25c/0x260)
    let pp = e.alloc(0x80); // edi (out at +0x10/0x14/0x18)
    let srcbuf = e.alloc(0x20); // eax (src vector at +0/4/8)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, scalar: f32, src: [f32; 3], base: [f32; 3], ctx: &str| {
        e.write_u32(ebp + 0xc, scalar.to_bits()); // [ebp+0xc] = fVar2 (scalar)
        for k in 0..3u64 {
            e.write_u32(srcbuf + 4 * k, src[k as usize].to_bits());
            e.write_u32(emitter + 0x258 + 4 * k, base[k as usize].to_bits());
        }
        e.set_reg(RegisterX86::EAX, srcbuf); // eax = pfVar8 (src vector)
        e.set_reg(RegisterX86::ESI, emitter);
        e.set_reg(RegisterX86::EDI, pp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b234a, 0x7b240f); // fmul×3; fadd×3; 3× flush gates → [edi+0x10/0x14/0x18]
        let bin = [
            e.read_f32(pp + 0x10),
            e.read_f32(pp + 0x14),
            e.read_f32(pp + 0x18),
        ];
        let raw = spawn_transform(scalar, src, base);
        let ours = [
            spawn_flush_gate(raw[0]),
            spawn_flush_gate(raw[1]),
            spawn_flush_gate(raw[2]),
        ];
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} out[{k}]"));
        }
    };

    // probes: zero src (out=base), unit, the f32-spill asymmetry (comp2 only), flush-eps boundary.
    run_case(&mut e, 2.0, [0.0; 3], [1.0, 2.0, 3.0], "zero-src");
    run_case(&mut e, 1.0, [3.0, 4.0, 5.0], [0.0; 3], "unit-scalar");
    run_case(
        &mut e,
        0.1,
        [0.3, 0.3, 0.3],
        [-0.03, -0.03, -0.03],
        "near-cancel",
    );
    // deterministic flush boundary: drive an output to exactly < eps and to >= eps.
    let tiny = f32::from_bits(0x322b_cc76); // just below FLUSH_EPS
    run_case(&mut e, 1.0, [tiny, tiny, tiny], [0.0; 3], "flush below-eps");
    run_case(
        &mut e,
        1.0,
        [FLUSH_EPS, FLUSH_EPS, FLUSH_EPS],
        [0.0; 3],
        "flush ==eps (keep)",
    );
    run_case(
        &mut e,
        1.0,
        [0.0, 0.0, 0.0],
        [0.0; 3],
        "exact zero (gate: !=0 false)",
    );

    let mut s = 0x7b23_4a4au32;
    let mut flushed = 0u32;
    for i in 0..5000u32 {
        let scalar = frand_mixed(&mut s);
        // bias some products into the flush band so the gate fires.
        let src = if (lcg(&mut s) >> 8).is_multiple_of(3) {
            [
                frand_mixed(&mut s) * 1.0e-8,
                frand_mixed(&mut s) * 1.0e-8,
                frand_mixed(&mut s) * 1.0e-8,
            ]
        } else {
            [
                frand_mixed(&mut s),
                frand_mixed(&mut s),
                frand_mixed(&mut s),
            ]
        };
        let base = [
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
        ];
        let raw = spawn_transform(scalar, src, base);
        for r in raw {
            if r != 0.0 && r.abs() < FLUSH_EPS {
                flushed += 1;
            }
        }
        run_case(&mut e, scalar, src, base, &format!("xform fuzz[{i}]"));
    }
    assert!(
        flushed > 0,
        "the flush-to-zero gate must fire at least once"
    );
}

/// The bytes `0x7b58f8..0x7b5904` and `0x7b591e..0x7b592a`, copied: the two quotients `floor`
/// takes.
#[test]
fn substep_floor_arg_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // 0x7b58f8..0x7b5904 (12 bytes): fld [ebp+8]; sub esp,8; fdiv [0x81d828]. Append `fstp qword
    // [out]` (instead of the binary's `fstp qword [esp]; call floor`) so we capture the f64
    // quotient.
    const SRC: u64 = 0x7b58f8;
    const LEN: usize = 0x7b5904 - 0x7b58f8;
    let mut code = e.read(SRC, LEN);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    code.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl, SRC, LEN);
    let end = isl + code.len() as u64;

    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, x: f32, ctx: &str| {
        e.write_u32(ebp + 0x8, x.to_bits()); // [ebp+8] = x (param_2)
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let lo = e.read_u32(out);
        let hi = e.read_u32(out + 4);
        let bin = f64::from_bits(u64::from(lo) | (u64::from(hi) << 32));
        let ours = substep_floor_arg(x);
        assert_eq!(
            bin.to_bits(),
            ours.to_bits(),
            "{ctx}: binary={:#018x} ours={:#018x}",
            bin.to_bits(),
            ours.to_bits()
        );
    };

    run_case(&mut e, 0.5, "0.5/0.1");
    run_case(&mut e, 0.1, "0.1/0.1");
    run_case(&mut e, 0.0, "0/0.1");
    run_case(&mut e, 1.0, "1/0.1");
    let mut s = 0x7b58_f8f8u32;
    for i in 0..3000u32 {
        run_case(
            &mut e,
            frand_mixed(&mut s).abs(),
            &format!("floorarg fuzz[{i}]"),
        );
    }

    // Second fdiv site 0x7b5924 (the `[esi+0xac]/0.1` form). Diffed DIRECTLY (not clone-inherited):
    // its own bytes 0x7b591e..0x7b592a (`fld [esi+0xac]; fdiv [0x81d828]`) run as a byte-island
    // with an appended `fstp qword [out2]`. The `fdiv` opcode (`d8 35 28 d8 81 00`) is
    // byte-identical to 0x7b58fe; only the `fld` source differs, so `substep_floor_arg` covers
    // both.
    const SRC2: u64 = 0x7b591e;
    const LEN2: usize = 0x7b592a - 0x7b591e;
    let mut code2 = e.read(SRC2, LEN2);
    let out2 = e.alloc(0x10);
    let isl2 = e.alloc(0x80);
    code2.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [out2]
    code2.extend_from_slice(&(out2 as u32).to_le_bytes());
    e.write(isl2, &code2);
    island_bytes_match(&e, isl2, SRC2, LEN2);
    let end2 = isl2 + code2.len() as u64;
    let emitter = e.alloc(0x300); // esi (field at +0xac)

    let run_case2 = |e: &mut Emu, x: f32, ctx: &str| {
        e.write_u32(emitter + 0xac, x.to_bits());
        e.set_reg(RegisterX86::ESI, emitter);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(isl2, end2);
        let lo = e.read_u32(out2);
        let hi = e.read_u32(out2 + 4);
        let bin = f64::from_bits(u64::from(lo) | (u64::from(hi) << 32));
        let ours = substep_floor_arg(x);
        assert_eq!(
            bin.to_bits(),
            ours.to_bits(),
            "{ctx}: binary={:#018x} ours={:#018x}",
            bin.to_bits(),
            ours.to_bits()
        );
    };
    run_case2(&mut e, 0.5, "site2 0.5/0.1");
    run_case2(&mut e, 0.0, "site2 0/0.1");
    let mut s2 = 0x7b59_1e1eu32;
    for i in 0..3000u32 {
        run_case2(
            &mut e,
            frand_mixed(&mut s2).abs(),
            &format!("floorarg2 fuzz[{i}]"),
        );
    }
}

/// `0x7b590f..0x7b591e`: the remainder.
#[test]
fn substep_remainder_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, param2: f32, count: f32, ctx: &str| {
        e.write_u32(ebp + 0x8, param2.to_bits()); // [ebp+8] = param_2
        e.write_u32(ebp + 0xc, count.to_bits()); // [ebp+0xc] = floor_count (param_3)
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b590f, 0x7b591e); // fld 0.1; fmul count; fsubr param_2; fstp [ebp+8]
        let bin = e.read_f32(ebp + 0x8);
        let ours = substep_remainder(param2, count);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 0.55, 5.0, "0.55 - 0.1*5");
    run_case(&mut e, 0.0, 0.0, "zeros");
    run_case(&mut e, 1.0, 9.0, "1 - 0.9");
    let mut s = 0x7b59_0f0fu32;
    for i in 0..3000u32 {
        let param2 = frand_mixed(&mut s).abs();
        let count = (frand_mixed(&mut s).abs() * 50.0).floor();
        run_case(&mut e, param2, count, &format!("rem fuzz[{i}]"));
    }
}

/// `0x7b595f..0x7b5974`: the count's bias.
#[test]
fn substep_count_bias_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, param3: f32, ctx: &str| {
        e.write_u32(ebp + 0xc, param3.to_bits()); // [ebp+0xc] = param_3
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b595f, 0x7b5974); // fld param_3; fadd 512.0; fstp [ebp+0xc]
        let bin = e.read_f32(ebp + 0xc);
        let ours = substep_count_bias(param3);
        assert_f32(bin, ours, ctx);
    };

    // param_3 is in [0,255] in practice (clamped above), but the fadd is pure; probe the band +
    // edges.
    run_case(&mut e, 0.0, "0+512");
    run_case(&mut e, 1.0, "1+512");
    run_case(&mut e, 255.0, "255+512");
    run_case(&mut e, COUNT_MAX, "COUNT_MAX+512");
    let mut s = 0x7b59_5f5fu32;
    for i in 0..3000u32 {
        let param3 = frand_mixed(&mut s).abs() % 256.0;
        run_case(&mut e, param3, &format!("bias fuzz[{i}]"));
    }
}

/// `0x7b598a..0x7b59bd`: the substep displacement.
#[test]
fn substep_tangent_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emitter = e.alloc(0x300); // esi (axis at +0x26c/0x270/0x274)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, n: u32, axis: [f32; 3], ctx: &str| {
        let n1 = i64::from(n) + 1;
        e.write_u32(ebp - 0x8, n1 as u32); // [ebp-8] lo = n+1
        e.write_u32(ebp - 0x4, (n1 >> 32) as u32); // [ebp-4] hi = 0
        for k in 0..3u64 {
            e.write_u32(emitter + 0x26c + 4 * k, axis[k as usize].to_bits());
        }
        e.set_reg(RegisterX86::ESI, emitter);
        // ecx = [esi+0x278] (set by `lea ecx,[esi+0x278]`@7b597c, just before the slice): the
        // in-slice `mov [ecx],edx`/`mov [ecx+4],edx` copy out[0]/out[1] there. We diff the f32
        // spills at [ebp-0xc/-8/-4]; ECX just needs to be mapped so the copies don't fault.
        e.set_reg(RegisterX86::ECX, emitter + 0x278);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b598a, 0x7b59bd); // fild n+1; fdivr 1.0; (fld st0; fmul axis[i]; fstp)×3
        let bin = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
            e.read_f32(ebp - 0x4),
        ];
        let ours = substep_tangent(n1, axis);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} out[{k}]"));
        }
    };

    // n is in [0,255] (the masked substep count); n+1 in [1,256].
    run_case(&mut e, 0, [3.0, 4.0, 5.0], "n=0 (frac=1)");
    run_case(&mut e, 1, [3.0, 4.0, 5.0], "n=1 (frac=0.5)");
    run_case(&mut e, 255, [1.0, -1.0, 0.0], "n=255 (frac=1/256)");
    run_case(&mut e, 9, [0.0, 0.0, 0.0], "zero-axis");
    let mut s = 0x7b59_8a8au32;
    for i in 0..3000u32 {
        let n = (lcg(&mut s) >> 8) & 0xff; // 0..255
        let axis = [
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
        ];
        run_case(&mut e, n, axis, &format!("tangent fuzz[{i}]"));
    }
}

/// The constants of the spawn and the substeps.
#[test]
fn particlespawn2_constants() {
    assert_eq!(ONE.to_bits(), 0x3f80_0000);
    assert_eq!(ZERO.to_bits(), 0x0000_0000);
    assert_eq!(PERIOD.to_bits(), 0x3dcc_cccd);
    assert_eq!(BIAS_512.to_bits(), 0x4400_0000);
}

/// `0x7b243e..0x7b24ca`: the spawn matrix normalize, about the tolerance and NaN.
#[test]
fn partfinalize_normalize_basis9_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100); // EBP frame; copied struct lives at [ebp-0x34]
    let ebp = frame + 0x80;
    let src = e.alloc(0x40); // ESI → 9-float source struct
    const OFFS: [u64; 9] = [0, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28];

    let run_case = |e: &mut Emu, s: [f32; 9], ctx: &str| {
        for (i, &o) in OFFS.iter().enumerate() {
            e.write_u32(src + o, s[i].to_bits());
        }
        e.set_reg(RegisterX86::ESI, src);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b243e, 0x7b24ca);
        let ours = normalize_basis9(s);
        for (k, &want) in ours.iter().enumerate() {
            assert_f32(
                e.read_f32(ebp - 0x34 + 4 * k as u64),
                want,
                &format!("{ctx} out[{k}]"),
            );
        }
    };

    // ==threshold probes: len² of first three == 1.0 (skip) and == 1 + 2^-22 (normalize).
    // len²==1.0 exactly via (1,0,0,...): distance 0 < eps → SKIP, unchanged.
    run_case(
        &mut e,
        [1.0, 0.0, 0.0, 9.0, -8.0, 7.0, -6.0, 5.0, -4.0],
        "basis9 len2==1 skip",
    );
    // len²==1+2^-22 exactly via c0=1.0, c1=2^-11: distance EXACTLY eps → NORMALIZE (not skip).
    let two_m11 = f32::from_bits(0x3a00_0000); // 2^-11; (2^-11)² = 2^-22
    run_case(
        &mut e,
        [1.0, two_m11, 0.0, 3.0, -2.0, 1.0, 0.5, -0.25, 4.0],
        "basis9 len2==1+eps normalize",
    );
    // len²==1+2^-24 (distance 2^-24 < eps) → SKIP, unchanged.
    let two_m12 = f32::from_bits(0x3980_0000); // 2^-12
    run_case(
        &mut e,
        [1.0, two_m12, 0.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0],
        "basis9 len2 just-inside skip",
    );
    // clean normalize: len²=4 → recip 0.5, all 9 scaled.
    run_case(
        &mut e,
        [0.0, 0.0, 2.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        "basis9 len2==4",
    );
    run_case(&mut e, [0.0; 9], "basis9 all-zero (len2==0)");

    let mut rng = FinRng(0x7b24_3e00);
    for i in 0..3000u32 {
        let mut s = [0f32; 9];
        for f in s.iter_mut() {
            *f = rng.f32_mixed();
        }
        run_case(&mut e, s, &format!("basis9 fuzz[{i}]"));
    }
}

/// `0x7b24d8..0x7b2540`: the quaternion normalize.
#[test]
fn partfinalize_normalize_quat4_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let q = e.alloc(0x20); // ESI → quaternion [x,y,z,w]

    let run_case = |e: &mut Emu, v: [f32; 4], ctx: &str| {
        for (i, &c) in v.iter().enumerate() {
            e.write_u32(q + 4 * i as u64, c.to_bits());
        }
        e.set_reg(RegisterX86::ESI, q);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b24d8, 0x7b2540);
        let ours = normalize_quat4(v);
        for (k, &want) in ours.iter().enumerate() {
            assert_f32(e.read_f32(q + 4 * k as u64), want, &format!("{ctx} q[{k}]"));
        }
    };

    // ==threshold probes (4-component len²): exactly 1.0 → skip; exactly 1+2^-22 → normalize.
    run_case(&mut e, [1.0, 0.0, 0.0, 0.0], "quat4 len2==1 skip");
    let two_m11 = f32::from_bits(0x3a00_0000); // 2^-11
    run_case(
        &mut e,
        [1.0, two_m11, 0.0, 0.0],
        "quat4 len2==1+eps normalize",
    );
    let two_m12 = f32::from_bits(0x3980_0000); // 2^-12; dist 2^-24 < eps
    run_case(&mut e, [1.0, two_m12, 0.0, 0.0], "quat4 just-inside skip");
    run_case(&mut e, [2.0, 0.0, 0.0, 0.0], "quat4 len2==4");
    run_case(&mut e, [1.0, 1.0, 1.0, 1.0], "quat4 len2==4 mixed");
    run_case(&mut e, [0.0; 4], "quat4 all-zero (len2==0)");

    let mut rng = FinRng(0x7b24_d800);
    for i in 0..3000u32 {
        let v = [
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
        ];
        run_case(&mut e, v, &format!("quat4 fuzz[{i}]"));
    }
}

/// `0x7b257f..0x7b25dd`: the spin from three draws.
#[test]
fn partfinalize_spawn_random_axes_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x400); // EBX → emitter struct (params at +0x1c8/0x1cc/0x1d4/0x1dc/0x1ac)
    let out = e.alloc(0x100); // EDI → output base; the slice does `add edi,0x30`

    let run_case = |e: &mut Emu, r: [f32; 3], min: f32, range: f32, d4: f32, dc: f32, ctx: &str| {
        // r2 → [ebp+0x10], r1 → [ebp+0xc], r0 → [ebp-0x4]
        e.write_u32(ebp + 0x10, r[2].to_bits());
        e.write_u32(ebp + 0xc, r[1].to_bits());
        e.write_u32(ebp - 0x4, r[0].to_bits());
        e.write_u32(this + 0x1c8, min.to_bits());
        e.write_u32(this + 0x1cc, range.to_bits());
        e.write_u32(this + 0x1d4, d4.to_bits());
        e.write_u32(this + 0x1dc, dc.to_bits());
        e.write_u32(this + 0x1ac, 0); // high bit clear → `jns` taken → skip sign block
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::EDI, out);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b257f, 0x7b25dd);
        let ours = spawn_random_axes(r, min, range, d4, dc);
        assert_f32(e.read_f32(out + 0x30), ours[0], &format!("{ctx} axis0"));
        assert_f32(e.read_f32(out + 0x34), ours[1], &format!("{ctx} axis1"));
        assert_f32(e.read_f32(out + 0x38), ours[2], &format!("{ctx} axis2"));
    };

    // boundary r values: exactly 1.0 (→ (r-1)=0) and just-below 2.0.
    run_case(&mut e, [1.0, 1.0, 1.0], 10.0, 4.0, 3.0, 7.0, "axes r==1.0");
    let r_near2 = f32::from_bits(0x3fff_ffff); // just below 2.0
    run_case(
        &mut e,
        [r_near2, r_near2, r_near2],
        -100.0,
        50.0,
        2.5,
        -3.5,
        "axes r~2.0",
    );

    let mut rng = FinRng(0x7b25_7f00);
    for i in 0..3000u32 {
        let r = [rng.r_unit(), rng.r_unit(), rng.r_unit()];
        let (min, range, d4, dc) = (
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
        );
        run_case(&mut e, r, min, range, d4, dc, &format!("axes fuzz[{i}]"));
    }
}

/// `0x7b2624..0x7b2648`: the spin's signs.
#[test]
fn partfinalize_spawn_sign_flip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let out = e.alloc(0x100); // EDI → axes at [edi+0/4/8]

    // sign0 is selected by the entry address: 0x7b2624 = fld +1.0, 0x7b262c = fld −1.0.
    let run_case = |e: &mut Emu, axes: [f32; 3], s0: f32, s1: f32, s2: f32, ctx: &str| {
        e.write_u32(out, axes[0].to_bits());
        e.write_u32(out + 4, axes[1].to_bits());
        e.write_u32(out + 8, axes[2].to_bits());
        e.write_u32(ebp + 0x10, s1.to_bits()); // [edi+4] *= [ebp+0x10]
        e.write_u32(ebp + 0xc, s2.to_bits()); // [edi+8] *= [ebp+0xc]
        e.set_reg(RegisterX86::EDI, out);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        let start = if s0 > 0.0 { 0x7b2624 } else { 0x7b262c };
        e.run(start, 0x7b2648);
        let ours = spawn_sign_flip(axes, [s0, s1, s2]);
        assert_f32(e.read_f32(out), ours[0], &format!("{ctx} axis0"));
        assert_f32(e.read_f32(out + 4), ours[1], &format!("{ctx} axis1"));
        assert_f32(e.read_f32(out + 8), ours[2], &format!("{ctx} axis2"));
    };

    // all 8 sign combinations over a fixed axis triple, plus fuzzed magnitudes.
    let mut rng = FinRng(0x7b26_2400);
    for combo in 0..8u32 {
        let s0 = if combo & 1 != 0 { 1.0 } else { -1.0 };
        let s1 = if combo & 2 != 0 { 1.0 } else { -1.0 };
        let s2 = if combo & 4 != 0 { 1.0 } else { -1.0 };
        for i in 0..200u32 {
            let axes = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
            run_case(
                &mut e,
                axes,
                s0,
                s1,
                s2,
                &format!("sign combo{combo} fuzz[{i}]"),
            );
        }
    }
}

/// `0x7b1dd9..0x7b1df0` and the copied product: the count through `__ftol`, past `i32`, past `i64`
/// and NaN.
#[test]
fn prealloc_sizing_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // the binary's own K (read from .rdata in emulator memory).
    let k = e.read_f32(PREALLOC_K_VA);

    // build the copied-byte island: WoW.exe[0x7b1dd9..0x7b1deb] (fld;fmul;fmul) + fstp dword [out].
    const PROD_VA: u64 = 0x7b1dd9;
    const PROD_LEN: usize = 0x7b1deb - 0x7b1dd9; // 0x12 = 18
    let src_bytes = e.read(PROD_VA, PROD_LEN);
    let p = e.alloc(0x200); // edi: the obj; +0xa8/+0xac the two scale floats
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    let mut code = src_bytes.clone();
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    e.write(isl, &code);
    // The copied product bytes are the exe's own.
    assert_eq!(
        e.read(isl, PROD_LEN),
        src_bytes,
        "prealloc island bytes must equal WoW.exe[0x7b1dd9..0x7b1deb]"
    );

    let run_case = |e: &mut Emu, ac: f32, a8: f32, ctx: &str| {
        e.write_u32(p + 0xac, ac.to_bits());
        e.write_u32(p + 0xa8, a8.to_bits());

        // (1) island: capture the exact f32 product.
        e.set_reg(RegisterX86::EDI, p);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(isl, isl_end);
        let bin_prod = e.read_f32(out);
        let our_prod = prealloc_product_f64(ac, a8, k) as f32;
        assert_f32(bin_prod, our_prod, &format!("{ctx} f32 product"));

        // (2) real slice through __ftol: the truncated int.
        e.set_reg(RegisterX86::EDI, p);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(0x7b1dd9, 0x7b1df0); // fld;fmul;fmul;call __ftol → halt at `push eax`
        let bin_count = e.get_reg(RegisterX86::EAX) as u32 as i32;
        let our_count = prealloc_count(ac, a8, k);
        assert_eq!(
            bin_count, our_count,
            "{ctx} __ftol count: binary={bin_count} ours={our_count} (ac={ac} a8={a8} k={k})"
        );
    };

    // structured: zeros, ones, in-band products, negatives (ftol toward zero), small fractionals.
    run_case(&mut e, 0.0, 0.0, "zeros");
    run_case(&mut e, 1.0, 1.0, "ones");
    run_case(&mut e, 100.0, 50.0, "mid");
    run_case(&mut e, -3.0, 7.0, "neg product (trunc toward 0)");
    run_case(&mut e, 0.4, 0.6, "sub-1 product");
    run_case(&mut e, 12345.0, 1.0, "large-ish");
    run_case(
        &mut e,
        1.0e30,
        1.0e10,
        "past i64 (the indefinite, low dword 0)",
    );
    run_case(&mut e, 3.0e9, 1.0, "past i32 (the low dword)");
    run_case(&mut e, f32::NAN, 1.0, "NaN (0)");

    let mut s = 0x7b1d_d0d0u32;
    for _ in 0..3000u32 {
        let m0 = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        let m1 = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        let ac = ((m0 * 4000.0 - 2000.0) as f32).abs(); // a count-like magnitude
        let a8 = (m1 * 200.0 - 100.0) as f32; // mixed sign, modest
        run_case(&mut e, ac, a8, "fuzz");
    }
}
