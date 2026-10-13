//! A particle's step (`systems/models/particles.md#particle-motion`,
//! `systems/models/particles.md#model-particles`, `systems/models/particles.md#particle-life`):
//! the step, the model particle's turn and the quaternion product called whole, the age as a slice.

#![allow(clippy::vec_init_then_push)] // the cases are listed one by one with their reasons

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::particles::motion::{
    particle_integrate, particle_quat_mul, particle_spin_integrate, particle_time_advance,
    EmitterIntegrate, Particle, Spin,
};
use common::{assert_f32, frand, next, SLICE_ESP};

struct SimRng(u32);

impl SimRng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// A finite, sanely-scaled f32 drawn from the high bits across signs/magnitudes.
    fn f32_finite(&mut self) -> f32 {
        let pick = (self.next() >> 8) % 5;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        let v = match pick {
            0 => m * 2.0 - 1.0,            // [-1, 1)
            1 => m * 100.0,                // moderate
            2 => -(m * 100.0),             // negative
            3 => m * 1.0e6 - 5e5,          // large
            _ => (m * 2.0 - 1.0) * 1.0e-6, // tiny
        };
        v as f32
    }
}

fn fuzz_bits(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 6;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32,       // [-1, 1)
        1 => (m * 2000.0 - 1000.0) as f32, // mid range
        2 => (m * 1.0e6) as f32,           // large
        3 => (-(m * 1.0e6)) as f32,        // large negative
        4 => (m * 1.0e-6) as f32,          // tiny
        _ => f32::from_bits(next(s)),      // raw bits (incl. NaN/Inf/denormal)
    }
}

/// `0x7b29c0` whole.
#[test]
fn particle_quat_mul_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let qa = emu.alloc(0x10);
    let qb = emu.alloc(0x10);
    let qo = emu.alloc(0x10);

    let structured: &[([f32; 4], [f32; 4])] = &[
        ([0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, 1.0]), // identity ⊗ identity
        ([1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]), // i ⊗ j = k
        ([0.0, 0.0, 0.0, 1.0], [1.0, 2.0, 3.0, 4.0]),
        ([0.5, -0.5, 0.5, -0.5], [-0.25, 0.75, -0.125, 0.6]),
    ];
    let mut rng = SimRng(0x7b29_c001);
    let q = |r: &mut SimRng| {
        [
            r.f32_finite(),
            r.f32_finite(),
            r.f32_finite(),
            r.f32_finite(),
        ]
    };
    let fuzz = (0..400).map(|_| (q(&mut rng), q(&mut rng)));
    for (a, b) in structured.iter().copied().chain(fuzz) {
        for k in 0..4 {
            emu.write(qa + 4 * k as u64, &a[k].to_le_bytes());
            emu.write(qb + 4 * k as u64, &b[k].to_le_bytes());
        }
        emu.call(0x7b29c0, qo as u32, qa as u32, &[qb as u32]);
        let bin = [
            emu.read_f32(qo),
            emu.read_f32(qo + 4),
            emu.read_f32(qo + 8),
            emu.read_f32(qo + 0xc),
        ];
        let ours = particle_quat_mul(a, b);
        for k in 0..4 {
            assert_f32(
                bin[k],
                ours[k],
                &format!("particle_quat_mul[{k}] a={a:?} b={b:?}"),
            );
        }
    }
}

/// `0x7b2680` whole on each branch, the cull order and NaN, then a fuzz.
#[test]
fn particle_integrate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let emitter = emu.alloc(0x300);
    let part = emu.alloc(0x40);

    // Write an emitter record into emulator memory.
    let write_emitter = |emu: &mut Emu, e: &EmitterIntegrate| {
        emu.write(emitter + 0x180, &e.gravity.to_le_bytes());
        emu.write_u32(emitter + 0x1ac, e.flag_word);
        emu.write(emitter + 0x1e0, &e.drag.to_le_bytes());
        emu.write(emitter + 0x1e4, &e.accel[0].to_le_bytes());
        emu.write(emitter + 0x1e8, &e.accel[1].to_le_bytes());
        emu.write(emitter + 0x1ec, &e.accel[2].to_le_bytes());
        emu.write(emitter + 0x1f0, &e.lifespan_gate.to_le_bytes());
        emu.write(emitter + 0x278, &e.drift[0].to_le_bytes());
        emu.write(emitter + 0x27c, &e.drift[1].to_le_bytes());
        emu.write(emitter + 0x280, &e.drift[2].to_le_bytes());
    };
    let write_particle = |emu: &mut Emu, p: &Particle| {
        for k in 0..3 {
            emu.write(part + 4 * k as u64, &p.pos[k].to_le_bytes());
            emu.write(part + 0x10 + 4 * k as u64, &p.vel[k].to_le_bytes());
        }
        // flag byte at +0xd; age at +0x1c. Keep surrounding bytes zero.
        emu.write_u32(part + 0xc, u32::from(p.flag) << 8);
        emu.write(part + 0x1c, &p.age.to_le_bytes());
    };
    let read_particle = |emu: &Emu| -> Particle {
        let r = |o: u64| emu.read_f32(part + o);
        Particle {
            pos: [r(0), r(4), r(8)],
            vel: [r(0x10), r(0x14), r(0x18)],
            flag: (emu.read_u32(part + 0xc) >> 8) as u8,
            age: r(0x1c),
        }
    };

    let base_emit = EmitterIntegrate {
        gravity: 9.8,
        flag_word: 0,
        drag: 0.0,
        accel: [1.0, 2.0, -3.0],
        lifespan_gate: 100.0,
        drift: [0.1, -0.2, 0.3],
    };
    let base_part = Particle {
        pos: [10.0, 20.0, 30.0],
        vel: [1.0, -1.0, 2.0],
        flag: 0,
        age: 5.0,
    };

    // Structured cases over each branch.
    let mut cases: Vec<(EmitterIntegrate, Particle, f32)> = Vec::new();
    cases.push((base_emit, base_part, 0.1)); // baseline: accel on, no flags, no drag
    cases.push((
        EmitterIntegrate {
            lifespan_gate: 1.0,
            ..base_emit
        },
        base_part,
        0.1,
    )); // age≥gate
    cases.push((
        EmitterIntegrate {
            flag_word: 0x4_0000,
            ..base_emit
        },
        base_part,
        0.1,
    )); // drift, flag bit0=0
    cases.push((
        EmitterIntegrate {
            flag_word: 0x4_0000,
            ..base_emit
        },
        Particle {
            flag: 1,
            ..base_part
        },
        0.1,
    )); // drift, clear bit
    cases.push((
        EmitterIntegrate {
            drag: 0.5,
            ..base_emit
        },
        base_part,
        0.1,
    )); // drag (sub 1.0)
    cases.push((
        EmitterIntegrate {
            drag: 100.0,
            ..base_emit
        },
        base_part,
        0.1,
    )); // drag clamps to 1.0
    cases.push((
        EmitterIntegrate {
            flag_word: 0x800,
            ..base_emit
        },
        base_part,
        0.1,
    )); // away-cull
    cases.push((
        EmitterIntegrate {
            flag_word: 0x800,
            ..base_emit
        },
        Particle {
            pos: [-10.0, -20.0, -30.0],
            ..base_part
        },
        0.1,
    )); // cull, opposite pos
    cases.push((
        base_emit,
        Particle {
            vel: [1e-9, -1e-9, 5e-9],
            ..base_part
        },
        0.1,
    )); // denormal flush
    cases.push((
        base_emit,
        Particle {
            vel: [0.0, -0.0, 0.0],
            ..base_part
        },
        0.1,
    )); // zero velocity (==0.0 keeps)
    cases.push((
        EmitterIntegrate {
            flag_word: 0x4_0800,
            drag: 0.3,
            ..base_emit
        },
        base_part,
        0.05,
    )); // all blocks
        // The cull sum is (x + y) + z: the terms 1, -1 and 2^-60 cull, summed z first they would
        // not.
    cases.push((
        EmitterIntegrate {
            gravity: 0.0,
            flag_word: 0x800,
            drag: 0.0,
            accel: [0.0; 3],
            lifespan_gate: 0.0,
            drift: [0.0; 3],
        },
        Particle {
            pos: [0.0, -2.0, 0.0],
            vel: [1.0, 1.0, f32::from_bits(0x3080_0000)],
            flag: 0,
            age: 5.0,
        },
        1.0,
    ));
    // A NaN age passes the wind time; a NaN cull sum keeps the particle.
    cases.push((
        base_emit,
        Particle {
            age: f32::NAN,
            ..base_part
        },
        0.1,
    ));
    cases.push((
        EmitterIntegrate {
            flag_word: 0x800,
            ..base_emit
        },
        Particle {
            vel: [f32::NAN, 1.0, 1.0],
            ..base_part
        },
        0.1,
    ));
    // A NaN drag factor is not capped.
    cases.push((
        EmitterIntegrate {
            drag: f32::NAN,
            ..base_emit
        },
        base_part,
        0.1,
    ));

    let mut rng = SimRng(0x7b26_8001);
    let fr = |r: &mut SimRng| r.f32_finite();
    for _ in 0..400 {
        let e = EmitterIntegrate {
            gravity: fr(&mut rng),
            flag_word: rng.next(),
            drag: if (rng.next() >> 8) & 1 == 0 {
                0.0
            } else {
                fr(&mut rng)
            },
            accel: [fr(&mut rng), fr(&mut rng), fr(&mut rng)],
            lifespan_gate: fr(&mut rng),
            drift: [fr(&mut rng), fr(&mut rng), fr(&mut rng)],
        };
        let p = Particle {
            pos: [fr(&mut rng), fr(&mut rng), fr(&mut rng)],
            vel: [fr(&mut rng), fr(&mut rng), fr(&mut rng)],
            flag: (rng.next() >> 8) as u8,
            age: fr(&mut rng),
        };
        cases.push((e, p, fr(&mut rng)));
    }

    for (e, p, dt) in cases {
        write_emitter(&mut emu, &e);
        write_particle(&mut emu, &p);
        let bin_eax = emu.call(0x7b2680, emitter as u32, 0, &[part as u32, dt.to_bits()]);
        let bin_part = read_particle(&emu);

        let mut our_part = p;
        let our_eax = particle_integrate(&e, &mut our_part, dt);

        assert_eq!(
            bin_eax, our_eax,
            "particle_integrate EAX (e={e:?} p={p:?} dt={dt})"
        );
        let ctx = format!("particle_integrate field (flag={:#x} dt={dt})", e.flag_word);
        for k in 0..3 {
            assert_f32(bin_part.pos[k], our_part.pos[k], &format!("{ctx} pos[{k}]"));
            assert_f32(bin_part.vel[k], our_part.vel[k], &format!("{ctx} vel[{k}]"));
        }
        assert_eq!(bin_part.flag, our_part.flag, "{ctx} flag");
    }
}

/// `0x7b28e0` whole, its `0x7b29c0` and `0x7b2680` included, about the 1e-4 spin.
#[test]
fn particle_spin_integrate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let emitter = emu.alloc(0x300);
    let obj = emu.alloc(0x40);

    let write_emitter = |emu: &mut Emu, e: &EmitterIntegrate| {
        emu.write(emitter + 0x180, &e.gravity.to_le_bytes());
        emu.write_u32(emitter + 0x1ac, e.flag_word);
        emu.write(emitter + 0x1e0, &e.drag.to_le_bytes());
        emu.write(emitter + 0x1e4, &e.accel[0].to_le_bytes());
        emu.write(emitter + 0x1e8, &e.accel[1].to_le_bytes());
        emu.write(emitter + 0x1ec, &e.accel[2].to_le_bytes());
        emu.write(emitter + 0x1f0, &e.lifespan_gate.to_le_bytes());
        emu.write(emitter + 0x278, &e.drift[0].to_le_bytes());
        emu.write(emitter + 0x27c, &e.drift[1].to_le_bytes());
        emu.write(emitter + 0x280, &e.drift[2].to_le_bytes());
    };

    let emit = EmitterIntegrate {
        gravity: 9.8,
        flag_word: 0,
        drag: 0.0,
        accel: [0.5, 0.5, 0.5],
        lifespan_gate: 100.0,
        drift: [0.0, 0.0, 0.0],
    };

    let structured: &[(Spin, Particle, f32)] = &[
        // (spin, particle, dt) — spin below / above the 1e-4 gate
        (
            Spin {
                orient: [0.0, 0.0, 0.0, 1.0],
                spin: [0.0, 0.0, 0.0],
            },
            Particle {
                pos: [1.0, 2.0, 3.0],
                vel: [0.1, 0.2, 0.3],
                flag: 0,
                age: 1.0,
            },
            0.1,
        ),
        (
            Spin {
                orient: [0.0, 0.0, 0.0, 1.0],
                spin: [1.0, 0.0, 0.0],
            },
            Particle {
                pos: [1.0, 2.0, 3.0],
                vel: [0.1, 0.2, 0.3],
                flag: 0,
                age: 1.0,
            },
            0.1,
        ),
        (
            Spin {
                orient: [0.2, -0.3, 0.4, 0.5],
                spin: [0.7, -1.2, 0.9],
            },
            Particle {
                pos: [5.0, -5.0, 0.0],
                vel: [-1.0, 1.0, -2.0],
                flag: 0,
                age: 2.0,
            },
            0.05,
        ),
        (
            Spin {
                orient: [0.0, 0.0, 0.0, 1.0],
                spin: [1e-5, 1e-5, 1e-5],
            }, // below gate
            Particle {
                pos: [0.0, 0.0, 0.0],
                vel: [0.0, 0.0, 0.0],
                flag: 0,
                age: 0.0,
            },
            0.1,
        ),
        // |spin| = 9.99999990e-5 in the register is above 1e-4 (`fcomp` at 0x7b2911 after `fst`), so
        // it turns, though the stored f32 equals 1e-4.
        (
            Spin {
                orient: [0.0, 0.0, 0.0, 1.0],
                spin: [
                    f32::from_bits(0x3894_4a1e),
                    f32::from_bits(0x3894_4ae5),
                    0.0,
                ],
            },
            Particle {
                pos: [0.0, 0.0, 0.0],
                vel: [0.0, 0.0, 0.0],
                flag: 0,
                age: 0.0,
            },
            1.0,
        ),
        // |spin|² summed (x² + y²) + z² stays at 1e-4, so it does not turn; summed z first it
        // would.
        (
            Spin {
                orient: [0.0, 0.0, 0.0, 1.0],
                spin: [
                    f32::from_bits(0x38d1_b717),
                    f32::from_bits(0x2b47_0c09),
                    f32::from_bits(0x2b51_3e17),
                ],
            },
            Particle {
                pos: [0.0, 0.0, 0.0],
                vel: [0.0, 0.0, 0.0],
                flag: 0,
                age: 0.0,
            },
            1.0,
        ),
    ];

    let mut rng = SimRng(0x7b28_e001);
    let fr = |r: &mut SimRng| r.f32_finite();
    let mut cases: Vec<(Spin, Particle, f32)> = structured.to_vec();
    for _ in 0..300 {
        cases.push((
            Spin {
                orient: [fr(&mut rng), fr(&mut rng), fr(&mut rng), fr(&mut rng)],
                spin: [fr(&mut rng), fr(&mut rng), fr(&mut rng)],
            },
            Particle {
                pos: [fr(&mut rng), fr(&mut rng), fr(&mut rng)],
                vel: [fr(&mut rng), fr(&mut rng), fr(&mut rng)],
                flag: 0,
                age: fr(&mut rng),
            },
            fr(&mut rng),
        ));
    }

    write_emitter(&mut emu, &emit);
    for (spin, p, dt) in cases {
        // Lay the combined object: pos +0x00, vel +0x10, orient +0x20, spin +0x30.
        for k in 0..3 {
            emu.write(obj + 4 * k as u64, &p.pos[k].to_le_bytes());
            emu.write(obj + 0x10 + 4 * k as u64, &p.vel[k].to_le_bytes());
            emu.write(obj + 0x30 + 4 * k as u64, &spin.spin[k].to_le_bytes());
        }
        for k in 0..4 {
            emu.write(obj + 0x20 + 4 * k as u64, &spin.orient[k].to_le_bytes());
        }
        emu.write_u32(obj + 0xc, u32::from(p.flag) << 8);
        emu.write(obj + 0x1c, &p.age.to_le_bytes());

        let bin_eax = emu.call(0x7b28e0, emitter as u32, 0, &[obj as u32, dt.to_bits()]);
        let bin_orient = [
            emu.read_f32(obj + 0x20),
            emu.read_f32(obj + 0x24),
            emu.read_f32(obj + 0x28),
            emu.read_f32(obj + 0x2c),
        ];
        let bin_pos = [
            emu.read_f32(obj),
            emu.read_f32(obj + 4),
            emu.read_f32(obj + 8),
        ];
        let bin_vel = [
            emu.read_f32(obj + 0x10),
            emu.read_f32(obj + 0x14),
            emu.read_f32(obj + 0x18),
        ];

        let mut our_spin = spin;
        let mut our_p = p;
        let our_eax = particle_spin_integrate(&emit, &mut our_spin, &mut our_p, dt);

        assert_eq!(
            bin_eax, our_eax,
            "spin_integrate EAX (spin={spin:?} dt={dt})"
        );
        for (k, &bo) in bin_orient.iter().enumerate() {
            assert_f32(
                bo,
                our_spin.orient[k],
                &format!("spin_integrate orient[{k}] spin={spin:?}"),
            );
        }
        for k in 0..3 {
            assert_f32(
                bin_pos[k],
                our_p.pos[k],
                &format!("spin_integrate pos[{k}]"),
            );
            assert_f32(
                bin_vel[k],
                our_p.vel[k],
                &format!("spin_integrate vel[{k}]"),
            );
        }
    }
}

/// `0x7b5a8b..0x7b5a99`: the age stored by `fst`.
#[test]
fn particle_time_advance_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let particle = e.alloc(0x40); // EBX → particle base; field at +0x1c
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, t: f32, field_1c: f32, ctx: &str| {
        e.write_u32(particle + 0x1c, field_1c.to_bits()); // [ebx+0x1c] = field (fadd + fst slot)
        e.write_u32(ebp + 0x8, t.to_bits()); // [ebp+8] = t (param_2, the fld source)
        e.set_reg(RegisterX86::EBX, particle);
        e.set_reg(RegisterX86::EAX, 0); // add ebx,eax keeps ebx = particle base
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        // run(0x7b5a8b,0x7b5a99): fld [ebp+8]; add ebx,eax; fadd [ebx+0x1c]; mov; fst [ebx+0x1c].
        // Halt at 0x7b5a99 (fcom), PAST the fst-no-pop; read the stored f32.
        e.run(0x7b5a8b, 0x7b5a99);
        let bin = e.read_f32(particle + 0x1c);
        let ours = particle_time_advance(t, field_1c);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 0.0, 0.0, "0+0");
    run_case(&mut e, 0.016, 5.0, "dt + age");
    run_case(&mut e, -1.0, 1.0, "t-1 cancels");
    run_case(&mut e, 1.0e30, 1.0e30, "overflow → inf");
    run_case(&mut e, f32::NAN, 1.0, "NaN");

    let mut s = 0x7b5a_1010u32;
    for i in 0..5000u32 {
        // mix: small time-steps + ages (the realistic band) and wide fuzz.
        let (t, field) = match (next(&mut s) >> 8) % 2 {
            0 => (frand(&mut s, 0.0, 1.0), frand(&mut s, 0.0, 1000.0)),
            _ => (fuzz_bits(&mut s), fuzz_bits(&mut s)),
        };
        run_case(&mut e, t, field, &format!("timeadv fuzz[{i}]"));
    }
}
