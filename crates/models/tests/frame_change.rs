//! `CParticleEmitter2::TransformParticles` (`0x7b5e60`, `systems/models/particles.md#frame-change`)
//! run whole on an emitter with two children, `TransformPointInPlace` (`0x7bcc60`) and the
//! recursion included: every word of every particle pool is compared, live or not.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_models::particles::draw::transform_particles;
use common::Lcg;

const FN_VA: u64 = 0x7b5e60;
const POOL: usize = 8;

struct Emitter {
    model: bool,
    live: Vec<u32>,
    pool: Vec<f32>,
}

/// Writes an emitter at `at`: `+0x28` the model flag, `+0x3c` or `+0x4c` the pool, `+0x5c` and
/// `+0x64` the live list, `+0x7c` and `+0x80` the children.
fn write_emitter(e: &mut Emu, at: u64, em: &Emitter, children: &[u64]) -> u64 {
    let pool = e.alloc(4 * em.pool.len() as u64);
    for (k, &f) in em.pool.iter().enumerate() {
        e.write_u32(pool + 4 * k as u64, f.to_bits());
    }
    let live = e.alloc(4 * em.live.len().max(1) as u64);
    for (k, &i) in em.live.iter().enumerate() {
        e.write_u32(live + 4 * k as u64, i);
    }
    e.write_u32(at + 0x28, u32::from(em.model));
    // The pool the other mode reads holds junk that must never be touched.
    let junk = e.alloc(0x40);
    e.write_u32(at + 0x3c, if em.model { junk } else { pool } as u32);
    e.write_u32(at + 0x4c, if em.model { pool } else { junk } as u32);
    e.write_u32(at + 0x5c, live as u32);
    e.write_u32(at + 0x64, em.live.len() as u32);
    e.write_u32(at + 0x7c, children.len() as u32);
    for (k, &c) in children.iter().enumerate() {
        e.write_u32(at + 0x80 + 4 * k as u64, c as u32);
    }
    pool
}

fn emitter(rng: &mut Lcg) -> Emitter {
    let model = rng.next() & 1 == 1;
    let stride = if model { 16 } else { 8 };
    let n = 1 + (rng.next() % POOL as u32) as usize;
    let pool: Vec<f32> = (0..n * stride).map(|_| rng.f32_mixed()).collect();
    let live: Vec<u32> = (0..n as u32).filter(|_| rng.next() >> 30 != 0).collect();
    Emitter { model, live, pool }
}

#[test]
fn transform_particles_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut rng = Lcg(0x7b5e_6000);
    let mut ident = [0f32; 16];
    for k in [0, 5, 10, 15] {
        ident[k] = 1.0;
    }
    for i in 0..1500 {
        let m: [f32; 16] = match i {
            0 => ident,
            1 => {
                let mut t = ident;
                t[12] = 5.0;
                t[13] = -7.5;
                t[14] = 1e30;
                t
            }
            2 => [0.0; 16],
            _ => std::array::from_fn(|_| rng.f32_mixed()),
        };
        let mut ems = [emitter(&mut rng), emitter(&mut rng), emitter(&mut rng)];
        if i == 3 {
            ems[0].live.clear();
        }
        let mat = e.alloc(0x40);
        for (k, &f) in m.iter().enumerate() {
            e.write_u32(mat + 4 * k as u64, f.to_bits());
        }
        let parent = e.alloc(0x290);
        let c1 = e.alloc(0x290);
        let c2 = e.alloc(0x290);
        let n_children = (i % 3) as usize;
        let kids = [c1, c2];
        let p_pool = write_emitter(&mut e, parent, &ems[0], &kids[..n_children]);
        let c1_pool = write_emitter(&mut e, c1, &ems[1], &[]);
        let c2_pool = write_emitter(&mut e, c2, &ems[2], &[]);

        e.call(FN_VA, parent as u32, 0, &[mat as u32]);

        let pools = [p_pool, c1_pool, c2_pool];
        for (j, em) in ems.iter_mut().enumerate() {
            if j == 0 || j <= n_children {
                transform_particles(&m, &em.live, em.model, &mut em.pool);
            }
            for (k, &f) in em.pool.iter().enumerate() {
                let bin = f32::from_bits(e.read_u32(pools[j] + 4 * k as u64));
                assert!(
                    bin.to_bits() == f.to_bits() || (bin.is_nan() && f.is_nan()),
                    "case {i}: emitter {j} word {k}: binary={:#010x} ours={:#010x}",
                    bin.to_bits(),
                    f.to_bits()
                );
            }
        }
    }
}
