//! The coin row's text parts against the exe (`systems/ui/loot.md`, Coin row text): `LootInitialize`
//! (`0x6c63a0`) runs whole and fills the multipliers at `0xce86c4`, then `SplitCopper` (`0x6c6260`)
//! runs whole on amounts at and around each multiple, negatives and the extremes, and
//! `GetCurrencyKey` (`0x6c6230`) on every index to 8 and the extremes, its string read back.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::loot_coin::{currency_key, currency_multipliers, split_copper};

const LOOT_INITIALIZE: u64 = 0x6c63a0;
const SPLIT_COPPER: u64 = 0x6c6260;
const CURRENCY_KEY: u64 = 0x6c6230;
const MULTIPLIERS: u64 = 0xce86c4;

#[test]
fn loot_coin_split_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.call(LOOT_INITIALIZE, 0, 0, &[]);
    let m = currency_multipliers();
    for (i, &v) in m.iter().enumerate() {
        assert_eq!(
            e.read_u32(MULTIPLIERS + 4 * i as u64) as i32,
            v,
            "multiplier {i}"
        );
    }
    let out = e.alloc(0x10);
    let mut amounts: Vec<i32> = vec![
        0,
        1,
        99,
        100,
        101,
        9999,
        10_000,
        10_001,
        10_099,
        10_100,
        -1,
        -100,
        -10_000,
        i32::MIN,
        i32::MAX,
    ];
    let mut s = 0x6c62_6000u32;
    for i in 0..20_000u32 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        amounts.push(match i % 3 {
            0 => (s >> 8) as i32 % 2_000_000,
            1 => s as i32,
            _ => ((s >> 16) as i32 - 0x8000) * 100,
        });
    }
    let mut n = 0u32;
    for &a in &amounts {
        e.write(out, &[0xa5; 12]);
        e.call(SPLIT_COPPER, a as u32, out as u32, &[]);
        let bin = [0, 1, 2].map(|i| e.read_u32(out + 4 * i) as i32);
        assert_eq!(bin, split_copper(a), "amount {a}");
        n += 1;
    }
    for index in (0..=8u32).chain([u32::MAX, 0x8000_0000]) {
        let p = e.call(CURRENCY_KEY, index, 0, &[]);
        let bin = e.read(u64::from(p), 16);
        let text = bin.split(|&b| b == 0).next().unwrap_or_default();
        assert_eq!(text, currency_key(index).as_bytes(), "index {index}");
        n += 1;
    }
    eprintln!("loot_coin_split: {n} cases bit-exact vs WoW.exe @ 0x6c63a0, 0x6c6260, 0x6c6230");
}
