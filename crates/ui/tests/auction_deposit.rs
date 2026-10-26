//! The auction deposit against the exe (`systems/ui/auctionlists.md`, Deposit): `ComputeAuctionDeposit`
//! (`0x644bc0`) runs whole, `__ftol` (`0x40a2b0`) as the exe has it. The cases cover every run time on
//! the table and its neighbours, rates and values whose product wraps 32 bits, and products at the
//! multiples of 100.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::auction_deposit::auction_deposit;

const DEPOSIT: u64 = 0x644bc0;

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
}

#[test]
fn auction_deposit_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let minutes = [
        0u32,
        1,
        60,
        119,
        120,
        121,
        240,
        479,
        480,
        481,
        960,
        1439,
        1440,
        1441,
        2880,
        u32::MAX,
    ];
    let edges = [
        0u32,
        1,
        5,
        15,
        25,
        99,
        100,
        101,
        9999,
        10_000,
        0x7fff_ffff,
        0x8000_0000,
        u32::MAX,
    ];
    let mut g = Lcg(0x644b_c000);
    let mut n = 0u32;
    for i in 0..40_000u32 {
        let rate = if i % 3 == 0 {
            edges[(g.next() >> 8) as usize % edges.len()]
        } else {
            g.next() >> (g.next() >> 27)
        };
        let value = if i % 5 == 0 {
            edges[(g.next() >> 8) as usize % edges.len()]
        } else {
            g.next() >> (g.next() >> 27)
        };
        let m = minutes[(g.next() >> 8) as usize % minutes.len()];
        let bin = e.call(DEPOSIT, rate, value, &[m]);
        assert_eq!(
            bin,
            auction_deposit(rate, value, m),
            "rate {rate} value {value} minutes {m}"
        );
        n += 1;
    }
    eprintln!("auction_deposit: {n} cases bit-exact vs WoW.exe @ 0x644bc0");
}
