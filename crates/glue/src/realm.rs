//! The realm list's load statistics and bands (`systems/glue/realms.md`).

/// 0.6, the float `0x3f19999a` at `0x8038d8`.
pub const STDDEV_MULT: f32 = f32::from_bits(0x3f19_999a);

/// `RebuildRealmList` (`0x46e510`): `(mean, deviation)` of the populations of every realm the client
/// knows, in order (`systems/glue/realms.md#population-statistics`). The sum is `s = f32(s + p)`,
/// the mean `f32(s / n)`, the accumulator `a = f32((p - mean)² + a)` from the stored mean, and the
/// deviation `f32(sqrt(a / (n - 1)) · 0.6)`, each step at 53 bits. With `n <= 1` the result is
/// `(1.0, 0.0)`. NaN populations give NaN for both.
pub fn realm_load_stats(populations: &[f32]) -> (f32, f32) {
    let n = populations.len();
    if n <= 1 {
        return (1.0, 0.0);
    }
    let mut sum: f32 = 0.0;
    for &pop in populations {
        sum = (f64::from(sum) + f64::from(pop)) as f32;
    }
    let mean = (f64::from(sum) / n as f64) as f32;
    let mut acc: f32 = 0.0;
    for &pop in populations {
        let d = f64::from(pop) - f64::from(mean);
        acc = (d * d + f64::from(acc)) as f32;
    }
    let stddev = ((f64::from(acc) / (n - 1) as f64).sqrt() * f64::from(STDDEV_MULT)) as f32;
    (mean, stddev)
}

/// `GetRealmLoadBand` (`0x46ec60`): the first that applies of flags `0x20` (-3.0), `0x40` (-2.0),
/// `0x80` (2.0), `mean - dev > pop` (-1.0), `mean + dev < pop` (1.0), else 0.0 (NaN included). The
/// sums are at 53 bits and the compares strict, so a population on an edge reads 0.0
/// (`systems/glue/realms.md#the-load-band`).
pub fn realm_load_classify(flags: u8, population: f32, mean: f32, stddev: f32) -> f32 {
    if flags & 0x20 != 0 {
        return -3.0;
    }
    if flags & 0x40 != 0 {
        return -2.0;
    }
    if flags & 0x80 != 0 {
        return 2.0;
    }
    if (f64::from(mean) - f64::from(stddev)) > f64::from(population) {
        return -1.0;
    }
    if (f64::from(mean) + f64::from(stddev)) < f64::from(population) {
        return 1.0;
    }
    0.0
}
