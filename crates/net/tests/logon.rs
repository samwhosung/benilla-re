//! The logon object's result handler and PIN proof (`systems/net/login.md#logon-results`,
//! `systems/net/logonlink.md#pin-proof`), run whole beside the models.

mod common;

use benilla_re_difftest::Emu;
use benilla_re_net::{
    checksum_proof, handle_logon_result, pin_digits, LogonResultCall, LogonResultState, Sha1Prng,
};
use common::{Lcg, Wow};

const HANDLE_LOGON_RESULT: u64 = 0x5b2c90;
const COMPUTE_PIN_PROOF: u64 = 0x5b3120;
/// `ReportLoginStatus` (`__thiscall`, 3 arguments), which logs the state's and code's names and
/// passes them on; the stub logs the state and code instead.
const REPORT: u64 = 0x5b2920;
const SET_REALM_COUNT: u64 = 0x5b3360;
const DROP_LINK: u64 = 0x5bb3b0;

struct Rig<'a> {
    e: Emu<'a>,
    login: u64,
    link: u64,
    slot4: u64,
    slot5: u64,
}

impl<'a> Rig<'a> {
    fn new(wow: &'a Wow) -> Rig<'a> {
        let mut e = Emu::new(wow);
        let login = e.alloc(0x200);
        let link = e.alloc(0x100);
        let holder = e.alloc(0x20);
        let vt = e.alloc(0x40);
        let slot4 = e.alloc(0x10);
        let slot5 = e.alloc(0x10);
        e.write_u32(login + 0x1ec, link as u32);
        e.write_u32(login + 0x1f0, holder as u32);
        e.write_u32(holder, vt as u32);
        e.write_u32(vt + 0x10, slot4 as u32);
        e.write_u32(vt + 0x14, slot5 as u32);
        e.stub(slot4, 0, 5);
        e.stub(slot5, 0, 1);
        e.stub(REPORT, 0, 3);
        e.stub(SET_REALM_COUNT, 0, 1);
        e.stub(DROP_LINK, 0, 0);
        Rig {
            e,
            login,
            link,
            slot4,
            slot5,
        }
    }
}

/// `HandleLogonResult` whole for every result byte and results past 255, with and without the
/// PIN, PIN failure counts at 0, 1, 2 and the byte's wrap, keys of 0 to 64 bytes, the link's
/// kept dword set or not, and during a reconnect: the state and code reported, the failure flag,
/// the count and the key.
#[test]
fn logon_result() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let key_at = r.e.alloc(0x40);
    let mut g = Lcg(0x10e0_0001);
    let mut results: Vec<u32> = (0..=0x10f).collect();
    results.extend([0xffff_ffff, 0x8000_0000, 0x1_0000, 0x1_000e]);
    for &result in &results {
        for case in 0..8 {
            let mut st = LogonResultState {
                reconnect: if case == 7 {
                    1 + (g.next() >> 24) as u8 % 255
                } else {
                    0
                },
                failed: g.next() >> 31,
                pin: [0u8, 1, 0x80][case % 3],
                pin_failures: [0u8, 1, 2, 0xff, 0x7f][(g.next() >> 16) as usize % 5],
                key: [0xee; 40],
            };
            let n = [0usize, 1, 20, 39, 40, 41, 64][(g.next() >> 16) as usize % 7];
            let key: Vec<u8> = (0..n).map(|_| (g.next() >> 16) as u8).collect();
            let kept = if g.next() >> 31 == 0 { 0 } else { g.next() | 1 };
            r.e.write(r.login + 0xc, &[st.reconnect]);
            r.e.write_u32(r.login + 8, st.failed);
            r.e.write(r.login + 0x1ac, &[st.pin, st.pin_failures]);
            r.e.write(r.login + 0x54, &st.key);
            r.e.write_u32(r.link + 0x78, kept);
            r.e.write(key_at, &key);
            r.e.clear_calls();
            r.e.call(
                HANDLE_LOGON_RESULT,
                r.login as u32,
                0,
                &[result, key_at as u32, n as u32],
            );
            let call = handle_logon_result(&mut st, result, &key, kept);
            let ctx = format!("result {result:#x} case {case} key {n}");
            let calls: Vec<_> = r.e.calls().iter().map(|c| (c.addr, c.args)).collect();
            match call {
                LogonResultCall::Report(state, code) => {
                    assert_eq!(calls.len(), 1, "{ctx}: one call");
                    assert_eq!(calls[0].0, REPORT, "{ctx}: report");
                    assert_eq!(&calls[0].1[..3], &[state, code, 0], "{ctx}: state, code");
                }
                LogonResultCall::Reconnect => {
                    let addrs: Vec<u64> = calls.iter().map(|c| c.0).collect();
                    assert_eq!(
                        addrs,
                        vec![SET_REALM_COUNT, r.slot5, DROP_LINK],
                        "{ctx}: calls"
                    );
                    assert_eq!((calls[0].1[0], calls[1].1[0]), (0, 1), "{ctx}: arguments");
                }
            }
            assert_eq!(r.e.read_u32(r.login + 8), st.failed, "{ctx}: failed");
            assert_eq!(
                r.e.read(r.login + 0x1ad, 1)[0],
                st.pin_failures,
                "{ctx}: PIN count"
            );
            assert_eq!(r.e.read(r.login + 0x54, 40), st.key.to_vec(), "{ctx}: key");
        }
    }
}

/// `ComputePinProof` whole for PINs of 0 to 11 positions, positions that make every byte and a
/// NUL, and several clocks and times: the 16-byte client salt it draws into `+0x1c8`, the proof at
/// `+0x1d8`, and the holder's slot 4 with state 9 and its two names.
#[test]
fn logon_pin_proof() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    const CLOCK: u32 = 0x0012_d687;
    const RAND: u32 = 0x0000_5a17;
    let time_at = r.e.alloc(0x10);
    let positions_at = r.e.alloc(0x20);
    r.e.stub(0x42c010, CLOCK, 0);
    r.e.stub_write_through(0x73f049, 0, time_at, 4, 0, 0);
    r.e.stub(0x7400d8, 0, 0);
    r.e.stub(0x7400e5, RAND, 0);
    let mut g = Lcg(0x0919_0001);
    for case in 0..600u32 {
        let time = g.next();
        r.e.write_u32(time_at, time);
        let count = (case % 12) as usize;
        let positions: Vec<u8> = (0..count)
            .map(|_| match g.next() >> 28 {
                0 => 0xd0,
                1 => (g.next() >> 24) as u8,
                _ => (g.next() >> 16) as u8 % 10,
            })
            .collect();
        let pin_salt: [u8; 16] = std::array::from_fn(|_| (g.next() >> 16) as u8);
        r.e.write(positions_at, &positions);
        r.e.write(r.login + 0x1b8, &pin_salt);
        r.e.write(r.login + 0x1c8, &[0xee; 0x24]);
        r.e.clear_calls();
        r.e.call(
            COMPUTE_PIN_PROOF,
            r.login as u32,
            0,
            &[count as u32, positions_at as u32],
        );
        let mut pool = Sha1Prng::seed(time, CLOCK, RAND);
        let mut salt = [0u8; 16];
        pool.generate(&mut salt);
        let proof = checksum_proof(&pin_salt, &salt, &pin_digits(&positions));
        let ctx = format!("case {case} positions {positions:02x?}");
        assert_eq!(
            r.e.read(r.login + 0x1c8, 16),
            salt.to_vec(),
            "{ctx}: client salt"
        );
        assert_eq!(
            r.e.read(r.login + 0x1d8, 20),
            proof.to_vec(),
            "{ctx}: proof"
        );
        let calls: Vec<_> = r.e.calls().iter().filter(|c| c.addr == r.slot4).collect();
        assert_eq!(calls.len(), 1, "{ctx}: slot 4");
        assert_eq!(&calls[0].args[..3], &[9, 0, 0], "{ctx}: state 9");
        let text = |p: u32| r.e.read(u64::from(p), 0x20);
        assert!(
            text(calls[0].args[3]).starts_with(b"LOGIN_STATE_CHECKSUM\0"),
            "{ctx}: name"
        );
        assert!(
            text(calls[0].args[4]).starts_with(b"LOGIN_OK\0"),
            "{ctx}: code name"
        );
    }
}
