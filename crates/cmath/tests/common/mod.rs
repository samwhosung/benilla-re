//! Shared by the cmath tests: the input sets (`synth_*`), the `Dev` fixture with its three scratch
//! buffers, the bit-exact comparisons, and one driver per calling convention (`check_*`).

#![allow(dead_code)]

pub use benilla_re_difftest::Wow;
use benilla_re_difftest::{Emu, RegisterX86};

/// `n`-`f32` inputs: single-value fills over the edge cases, a mixed vector, and a large LCG fuzz of
/// finite values. NaN excluded as an input (never a real matrix value; `fld`/`fstp` would quiet sNaN).
pub fn synth_n(n: usize) -> Vec<Vec<f32>> {
    let edges = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1e30,
        -1e30,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        123.456,
        -789.012,
    ];
    let mut v: Vec<Vec<f32>> = Vec::new();
    for &e in &edges {
        v.push(vec![e; n]);
    }
    let mut mixed = vec![0f32; n];
    for (i, x) in mixed.iter_mut().enumerate() {
        *x = edges[i % edges.len()];
    }
    v.push(mixed);
    let mut s: u32 = 0x1234_5678;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..1500 {
        let mut m = vec![0f32; n];
        for x in m.iter_mut() {
            let mut f = f32::from_bits(next());
            while f.is_nan() {
                f = f32::from_bits(next());
            }
            *x = f;
        }
        v.push(m);
    }
    v
}

/// Angles for the rotation builders: edges (multiples of π/4) + an LCG fuzz in [−1e4, 1e4]. Bounded
/// well under the x87 `fsincos` 2^63 no-argument-reduction cliff (real rotations are small anyway).
pub fn synth_angles() -> Vec<f32> {
    use std::f32::consts::PI;
    let mut v = vec![
        0.0f32,
        -0.0,
        PI / 4.0,
        PI / 2.0,
        PI,
        2.0 * PI,
        -PI,
        -PI / 2.0,
        1.0,
        -1.0,
        100.0,
        -1234.5,
    ];
    let mut s: u32 = 0xabcd_ef01;
    for _ in 0..1000 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        v.push((s as f32 / u32::MAX as f32 - 0.5) * 2.0e4);
    }
    v
}

/// Diff a 3-angle Euler→3×3 builder: `ecx`=dst, `[ebp+8/0xc/0x10]` = the three angles by value; reads
/// the 9-`f32` matrix from `dst`. Fuzzed over sliding triples of `synth_angles`.
pub fn check_eulerbuild(name: &str, fn_va: u64, rust: impl Fn(&[f32; 3]) -> [f32; 9]) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let angles = synth_angles();
    for w in angles.windows(3) {
        let a = [w[0], w[1], w[2]];
        d.emu.call(
            fn_va,
            d.dst as u32,
            0,
            &[a[0].to_bits(), a[1].to_bits(), a[2].to_bits()],
        );
        assert_slice(&d.out_n(9), &rust(&a), name);
    }
    eprintln!(
        "{name}: {} cases bit-exact vs WoW.exe @ {fn_va:#x}",
        angles.len() - 2
    );
}

/// Scalars for the broadcast and scale functions: edge values and 400 random non-NaN patterns.
pub fn synth_scalars() -> Vec<f32> {
    let mut v = vec![
        0.0f32, -0.0, 1.0, -1.0, 0.5, 2.0, 1e30, -1e-30, 123.456, -789.0,
    ];
    let mut s: u32 = 0x9e37_79b9;
    for _ in 0..400 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let f = f32::from_bits(s);
        if !f.is_nan() {
            v.push(f);
        }
    }
    v
}

pub struct Dev<'a> {
    pub emu: Emu<'a>,
    pub dst: u64,
    pub a: u64,
    pub b: u64,
}
impl<'a> Dev<'a> {
    pub fn new(wow: &Wow) -> Self {
        let mut emu = Emu::new(wow);
        emu.set_reg(RegisterX86::FPCW, 0x027F);
        let dst = emu.alloc(0x80);
        let a = emu.alloc(0x80);
        let b = emu.alloc(0x80);
        Dev { emu, dst, a, b }
    }
    pub fn put(&mut self, base: u64, m: &[f32]) {
        for (i, &f) in m.iter().enumerate() {
            self.emu.write_u32(base + i as u64 * 4, f.to_bits());
        }
    }
    pub fn out_n(&self, n: usize) -> Vec<f32> {
        self.read_n(self.dst, n)
    }
    pub fn read_n(&self, base: u64, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| self.emu.read_f32(base + i as u64 * 4))
            .collect()
    }
}

/// Bit-exact for finite/inf (incl. inf propagation + overflow). A NaN result — from inf−inf / inf·0 /
/// 0÷0 — is accepted as any NaN: the x87 indefinite (sign=1) vs IEEE (sign=0) differ only in the NaN
/// sign/payload, not observable fidelity. A finite-vs-NaN mismatch still fails.
pub fn assert_slice(binary: &[f32], ours: &[f32], ctx: &str) {
    assert_eq!(binary.len(), ours.len());
    for i in 0..binary.len() {
        if binary[i].is_nan() && ours[i].is_nan() {
            continue;
        }
        assert_eq!(
            binary[i].to_bits(),
            ours[i].to_bits(),
            "{ctx}[{i}]: binary={:#010x} ours={:#010x}",
            binary[i].to_bits(),
            ours[i].to_bits()
        );
    }
}

/// Diff a binary matrix op `dst = f(a, b)` — `Emu::call(fn, dst, a, &[b_ptr])` — over fuzzed `n`-`f32`
/// pairs, across every `fn_va` that shares the op (duplicate instantiations).
pub fn check_binop(
    name: &str,
    fn_vas: &[u64],
    n: usize,
    rust: impl Fn(&[f32], &[f32]) -> Vec<f32>,
) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mats = synth_n(n);
    let mut checked = 0u64;
    for &fn_va in fn_vas {
        for w in mats.windows(2) {
            d.put(d.a, &w[0]);
            d.put(d.b, &w[1]);
            d.emu.call(fn_va, d.dst as u32, d.a as u32, &[d.b as u32]);
            assert_slice(&d.out_n(n), &rust(&w[0], &w[1]), name);
            checked += 1;
        }
    }
    eprintln!("{name}: {checked} cases bit-exact vs WoW.exe @ {fn_vas:#x?}");
}

/// Diff a binary scalar op `dst = f(a, s)` — `Emu::call(fn, dst, a, &[s.to_bits()])` — over fuzzed
/// `n`-`f32` matrices × scalars, across every `fn_va` that shares the op.
pub fn check_scalarop(
    name: &str,
    fn_vas: &[u64],
    n: usize,
    rust: impl Fn(&[f32], f32) -> Vec<f32>,
) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mats = synth_n(n);
    let scalars = synth_scalars();
    let mut checked = 0u64;
    for &fn_va in fn_vas {
        for (k, m) in mats.iter().enumerate() {
            let s = scalars[k % scalars.len()];
            d.put(d.a, m);
            d.emu.call(fn_va, d.dst as u32, d.a as u32, &[s.to_bits()]);
            assert_slice(&d.out_n(n), &rust(m, s), name);
            checked += 1;
        }
    }
    eprintln!("{name}: {checked} cases bit-exact vs WoW.exe @ {fn_vas:#x?}");
}

/// Diff an in-place scalar scale `m[i] ·= s` — `ecx`=matrix (read back from the same buffer), `s` by
/// value on the stack.
pub fn check_scale_inplace(
    name: &str,
    fn_vas: &[u64],
    n: usize,
    rust: impl Fn(&[f32], f32) -> Vec<f32>,
) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mats = synth_n(n);
    let scalars = synth_scalars();
    let mut checked = 0u64;
    for &fn_va in fn_vas {
        for (k, m) in mats.iter().enumerate() {
            let s = scalars[k % scalars.len()];
            d.put(d.a, m);
            d.emu.call(fn_va, d.a as u32, 0, &[s.to_bits()]);
            assert_slice(&d.read_n(d.a, n), &rust(m, s), name);
            checked += 1;
        }
    }
    eprintln!("{name}: {checked} cases bit-exact vs WoW.exe @ {fn_vas:#x?}");
}

/// Diff an in-place vector-driven scale `m ·= f(v)` — `ecx`=matrix (read back), the `vlen`-`f32` vector
/// passed by pointer as the stack arg.
pub fn check_vecscale_inplace(
    name: &str,
    fn_vas: &[u64],
    n: usize,
    vlen: usize,
    rust: impl Fn(&[f32], &[f32]) -> Vec<f32>,
) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mats = synth_n(n);
    let vecs = synth_n(vlen);
    let mut checked = 0u64;
    for &fn_va in fn_vas {
        for (k, m) in mats.iter().enumerate() {
            let v = &vecs[k % vecs.len()];
            d.put(d.a, m);
            d.put(d.b, v);
            d.emu.call(fn_va, d.a as u32, 0, &[d.b as u32]);
            assert_slice(&d.read_n(d.a, n), &rust(m, v), name);
            checked += 1;
        }
    }
    eprintln!("{name}: {checked} cases bit-exact vs WoW.exe @ {fn_vas:#x?}");
}

/// Diff a matrix comparison returning a bool in `EAX` (ecx=a, edx=b). Tests equal pairs `(m, m)`,
/// near-equal `(m, m')` (one element perturbed), and unequal `(m, other)`.
pub fn check_cmp(name: &str, fn_vas: &[u64], n: usize, rust: impl Fn(&[f32], &[f32]) -> bool) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mats = synth_n(n);
    let mut checked = 0u64;
    for &fn_va in fn_vas {
        for (k, m) in mats.iter().enumerate() {
            let mut perturbed = m.clone();
            perturbed[k % n] = f32::from_bits(perturbed[k % n].to_bits() ^ 1); // flip 1 ulp
            for other in [m, &perturbed, &mats[(k + 1) % mats.len()]] {
                d.put(d.a, m);
                d.put(d.b, other);
                let got = d.emu.call(fn_va, d.a as u32, d.b as u32, &[]);
                assert_eq!(
                    got,
                    u32::from(rust(m, other)),
                    "{name}: binary={got} ours={}",
                    u32::from(rust(m, other))
                );
                checked += 1;
            }
        }
    }
    eprintln!("{name}: {checked} comparisons bit-exact vs WoW.exe @ {fn_vas:#x?}");
}

/// Diff a transform `dst = f(a, b)` where `a` (`na`-`f32`) is the `edx` operand and `b` (`nb`-`f32`)
/// the `[ebp+8]` operand; reads `nout` `f32` from dst. `Emu::call(fn, dst, a_ptr, &[b_ptr])`.
pub fn check_vecmat(
    name: &str,
    fn_vas: &[u64],
    na: usize,
    nb: usize,
    nout: usize,
    rust: impl Fn(&[f32], &[f32]) -> Vec<f32>,
) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let avs = synth_n(na);
    let bvs = synth_n(nb);
    let mut checked = 0u64;
    for &fn_va in fn_vas {
        for (k, av) in avs.iter().enumerate() {
            let bv = &bvs[(k + 1) % bvs.len()];
            d.put(d.a, av);
            d.put(d.b, bv);
            d.emu.call(fn_va, d.dst as u32, d.a as u32, &[d.b as u32]);
            assert_slice(&d.out_n(nout), &rust(av, bv), name);
            checked += 1;
        }
    }
    eprintln!("{name}: {checked} cases bit-exact vs WoW.exe @ {fn_vas:#x?}");
}

/// Diff a matrix builder `out = f(in, scalar)`: `__thiscall` `ecx`=in (`n_in` f32), `[ebp+8]`=out
/// ptr, `[ebp+0xc]`=scalar by value; reads `n_out` f32 from out. `scalars` is passed explicitly so a
/// branch keyed on the scalar (e.g. an `|s−1|<eps` guard) can be probed at its boundary.
pub fn check_mat_scalar_build(
    name: &str,
    fn_vas: &[u64],
    n_in: usize,
    n_out: usize,
    scalars: &[f32],
    rust: impl Fn(&[f32], f32) -> Vec<f32>,
) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mats = synth_n(n_in);
    let mut checked = 0u64;
    for &fn_va in fn_vas {
        for (k, m) in mats.iter().enumerate() {
            let s = scalars[k % scalars.len()];
            d.put(d.a, m);
            d.emu
                .call(fn_va, d.a as u32, 0, &[d.dst as u32, s.to_bits()]);
            assert_slice(&d.out_n(n_out), &rust(m, s), name);
            checked += 1;
        }
    }
    eprintln!("{name}: {checked} cases bit-exact vs WoW.exe @ {fn_vas:#x?}");
}

/// Diff a matrix builder `out = f(in)`: `__thiscall` `ecx`=in (`n_in` f32), `[ebp+8]`=out ptr; reads
/// `n_out` f32 from out.
pub fn check_mat_inout(
    name: &str,
    fn_vas: &[u64],
    n_in: usize,
    n_out: usize,
    rust: impl Fn(&[f32]) -> Vec<f32>,
) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mats = synth_n(n_in);
    let mut checked = 0u64;
    for &fn_va in fn_vas {
        for m in &mats {
            d.put(d.a, m);
            d.emu.call(fn_va, d.a as u32, 0, &[d.dst as u32]);
            assert_slice(&d.out_n(n_out), &rust(m), name);
            checked += 1;
        }
    }
    eprintln!("{name}: {checked} cases bit-exact vs WoW.exe @ {fn_vas:#x?}");
}

pub fn assert_f32(binary: f32, ours: f32, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// Diff a unary op that returns by value via a hidden pointer: `ecx`=input `a` (`na`-`f32`), the dst
/// pointer is the stack arg `[ebp+8]`; reads `nout` `f32` from dst. `Emu::call(fn, a_ptr, 0, &[dst])`.
pub fn check_unary(
    name: &str,
    fn_vas: &[u64],
    na: usize,
    nout: usize,
    rust: impl Fn(&[f32]) -> Vec<f32>,
) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mats = synth_n(na);
    let mut checked = 0u64;
    for &fn_va in fn_vas {
        for m in &mats {
            d.put(d.a, m);
            d.emu.call(fn_va, d.a as u32, 0, &[d.dst as u32]);
            assert_slice(&d.out_n(nout), &rust(m), name);
            checked += 1;
        }
    }
    eprintln!("{name}: {checked} cases bit-exact vs WoW.exe @ {fn_vas:#x?}");
}

/// The first `N` entries of `s` as an array reference.
pub fn arr<const N: usize>(s: &[f32]) -> &[f32; N] {
    s.try_into().unwrap()
}
