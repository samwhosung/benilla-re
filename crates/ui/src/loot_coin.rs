//! The coin row's text parts (`systems/ui/loot.md`, Coin row text).

/// The copper value of copper, silver and gold, as `LootInitialize` (`0x6c63a0`) fills the table at
/// `0xce86c4`: 1, then each entry 100 times the one before.
pub fn currency_multipliers() -> [i32; 3] {
    let mut m = [1i32; 3];
    for i in 1..3 {
        m[i] = m[i - 1].wrapping_mul(100);
    }
    m
}

/// `SplitCopper` (`0x6c6260`): copper, silver and gold in that order. Gold, then silver, takes
/// `remainder / multiplier` when the remainder is at least the multiplier (signed), and the
/// remainder goes on; otherwise that part is 0. Copper is what is left.
pub fn split_copper(copper: i32) -> [i32; 3] {
    let m = currency_multipliers();
    let mut parts = [0i32; 3];
    let mut rest = copper;
    for i in (1..3).rev() {
        if rest >= m[i] {
            parts[i] = rest / m[i];
            rest %= m[i];
        }
    }
    parts[0] = rest;
    parts
}

/// `GetCurrencyKey` (`0x6c6230`): the global string key of currency unit `index`.
pub fn currency_key(index: u32) -> &'static str {
    match index {
        0 => "COPPER",
        1 => "SILVER",
        2 => "GOLD",
        _ => "ERROR_CAPS",
    }
}
