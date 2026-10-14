//! The PIN grid of the logon screen (`systems/net/login.md`).

/// `DecodePinGridOrder` (`0x5b1e70`, the loop `0x5b1e93` to `0x5b1ebd`): the order of the digits
/// 0 to 9 the server's seed encodes. From the pool `0..=9`, for `n` from 10 down to 1 it takes
/// `r = seed % n`, outputs the pool's `r`th digit, removes it (a `memmove` of the tail) and sets
/// `seed /= n`.
pub fn pin_grid_shuffle(mut seed: u32) -> [u8; 10] {
    let mut digits: Vec<u8> = (0..10u8).collect();
    let mut out = [0u8; 10];
    for (i, slot) in out.iter_mut().enumerate() {
        let n = (10 - i) as u32;
        let r = (seed % n) as usize;
        seed /= n;
        *slot = digits.remove(r);
    }
    out
}
