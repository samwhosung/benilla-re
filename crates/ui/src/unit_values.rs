//! Arithmetic of the unit, pet, guild and auction scripts that push or compare a number.
//! The entries are in `systems/ui/unitlevels.md`, `systems/ui/honor.md`, `systems/ui/petbar.md`,
//! `systems/ui/auctionlists.md`, `systems/ui/guild.md` and `systems/ui/rested.md`.

use benilla_re_cmath::x87::ftol;

/// `UnitLevel` (`0x517fc0`, `0x5180bc` to `0x5180dd`): the level byte clamps to 100, then
/// `n * 5.0 * 0.01 + 0.5` stays live in `st0` (the three constants are `f32`: `0x808bf4`,
/// `0x8029d0`, `0x7ffa24`) and `__ftol` truncates it.
pub fn level_round(b: u8) -> i32 {
    let n = u32::from(b.min(100));
    let v = (f64::from(n) * 5.0) * f64::from(f32::from_bits(0x3c23_d70a))
        + f64::from(f32::from_bits(0x3f00_0000));
    benilla_re_cmath::x87::ftol(v)
}

/// `GetPVPRankProgress` (`0x51aacb` to `0x51aad4`) and `GetInspectPVPRankProgress` (`0x51ab11` to
/// `0x51ab1a`): the rank byte times the `f32` 1/255 at `0x8026c8`, pushed as an f64.
pub fn rank_progress(rank: u8) -> f64 {
    crate::color::channel_over_255_f32(rank)
}

/// `GetXPExhaustion` (`0x48d47e` to `0x48d487`): the rested experience (an `i64` with the high dword
/// forced to 0) times the `f32` factor at `+0x8` of the faction record, pushed as an f64.
pub fn xp_exhaustion(n: i64, factor: f32) -> f64 {
    n as f64 * f64::from(factor)
}

/// `GetPetHappiness` (`0x4be9ae` to `0x4be9be`): the stat field `+0x30 + 4 * index` times the
/// `f32` 100.0 at `0x806b10`, pushed as an f64.
pub fn pet_happiness_scale(field: f32) -> f64 {
    f64::from(field) * f64::from(f32::from_bits(0x42c8_0000))
}

/// `CancelAuction` (`0x4d0338` to `0x4d0346`): 5% of the current bid, `fild` of the bid times the
/// `f32` 0.05 at `0x807024`, truncated.
pub fn cancel_auction_min_increment(bid: i64) -> i32 {
    ftol(bid as f64 * f64::from(f32::from_bits(0x3d4c_cccd)))
}

/// `GetGuildRosterLastOnline` (`0x4d1508` to `0x4d15a8`): the `LastOnline` float `t` of the member
/// (`+0x150`) broken into years, months, days and hours, all at 53 bits:
///
/// - `years = trunc(t * (1/365))`, with 1/365 the `f32` at `0x80733c`;
/// - `rem = t - years * 365.0` (`0x807338`);
/// - `months = trunc(rem * (1/30))`, with 1/30 the `f32` at `0x807334`;
/// - `days_rem = rem - months * 30.0` (`0x807330`), stored as an `f32` at `0x4d1544`
///   and reloaded;
/// - `days = trunc(days_rem)`, `hours = trunc((days_rem - days) * 24.0)` (`0x80732c`).
pub fn guild_last_online(t: f32) -> (i32, i32, i32, i32) {
    let k = |bits: u32| f64::from(f32::from_bits(bits));
    let tf = f64::from(t);
    let years = ftol(tf * k(0x3b33_8cfa));
    let rem = tf - f64::from(years) * k(0x43b6_8000);
    let months = ftol(rem * k(0x3d08_8889));
    let days_rem = (rem - f64::from(months) * k(0x41f0_0000)) as f32;
    let days = ftol(f64::from(days_rem));
    let hours = ftol((f64::from(days_rem) - f64::from(days)) * k(0x41c0_0000));
    (years, months, days, hours)
}
