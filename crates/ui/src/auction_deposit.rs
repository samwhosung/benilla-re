//! The auction deposit (`systems/ui/auctionlists.md`, Deposit).

/// The three run times, in minutes, the deposit accepts: the table at `0x80e4b0`.
pub const RUN_TIMES: [u32; 3] = [120, 480, 1440];

/// `ComputeAuctionDeposit` (`0x644bc0`): 0 for a run time off the table, else `rate · value` in 32
/// bits divided by 100, times `minutes / 120`, both divisions unsigned. The product is taken on the
/// x87 from a 64-bit integer and a 32-bit one and truncated by `__ftol`, whose low dword is the
/// result; it is below 2³², so the x87 step is exact.
pub fn auction_deposit(rate: u32, value: u32, minutes: u32) -> u32 {
    if !RUN_TIMES.contains(&minutes) {
        return 0;
    }
    let base = rate.wrapping_mul(value) / 100;
    let periods = minutes / 120;
    (u64::from(base) * u64::from(periods)) as u32
}
