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

/// The logon object's fields `HandleLogonResult` reads and writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogonResultState {
    /// `+0xc`: set during a reconnect.
    pub reconnect: u8,
    /// `+0x8`: 1 after a failed result, 0 after a success.
    pub failed: u32,
    /// `+0x1ac`: the PIN flag of the challenge.
    pub pin: u8,
    /// `+0x1ad`: the count of failed results 4 and 5 with the PIN, a byte that wraps.
    pub pin_failures: u8,
    /// `+0x54`: the session key, up to 40 bytes.
    pub key: [u8; 40],
}

/// A call `HandleLogonResult` makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogonResultCall {
    /// `0x5b2920` on the realm holder: a login state and a code.
    Report(u32, u32),
    /// A reconnect: the realm array emptied (`0x5b3360` with 0), the holder's slot 5 with 1,
    /// and the link dropped (`0x5bb3b0`).
    Reconnect,
}

/// `HandleLogonResult` (`0x5b2c90`, `(result, key, length)`), slot 10 of the logon object. During
/// a reconnect it only drops the link. Result 0 or `0x0e` clears the failure flag and the PIN
/// count, copies `min(length, 40)` key bytes to `+0x54` and reports state 12 when the link's
/// `+0x78` is set, else 4. Any other result sets the failure flag and reports state 5 with a code,
/// except `0x0a`, state 6. `link_kept` is the link's `+0x78`.
pub fn handle_logon_result(
    st: &mut LogonResultState,
    result: u32,
    key: &[u8],
    link_kept: u32,
) -> LogonResultCall {
    if st.reconnect != 0 {
        return LogonResultCall::Reconnect;
    }
    if result == 0 || result == 0x0e {
        st.failed = 0;
        st.pin_failures = 0;
        let n = key.len().min(0x28);
        st.key[..n].copy_from_slice(&key[..n]);
        return LogonResultCall::Report(if link_kept != 0 { 0x0c } else { 4 }, 0);
    }
    st.failed = 1;
    let code = match result {
        3 => 0x0d,
        4 | 5 => {
            if st.pin == 0 {
                7
            } else {
                st.pin_failures = st.pin_failures.wrapping_add(1);
                if st.pin_failures < 3 {
                    8
                } else {
                    9
                }
            }
        }
        6 => 0x0f,
        7 => 0x10,
        8 => 0x11,
        9 => 0x0e,
        0x0a => return LogonResultCall::Report(6, 0),
        0x0c => 0x12,
        0x0f => 0x13,
        0xff => 0x14,
        _ => 0x0b,
    };
    LogonResultCall::Report(5, code)
}

/// The digits `ComputePinProof` (`0x5b3120`) hashes for the PIN `positions`: each plus `0x30` as a
/// byte, up to the first NUL that makes, since the hash reads a C string. The count is at most
/// 11: the text is a 12-byte stack buffer and its NUL goes at the count.
pub fn pin_digits(positions: &[u8]) -> Vec<u8> {
    positions
        .iter()
        .map(|p| p.wrapping_add(0x30))
        .take_while(|&d| d != 0)
        .collect()
}
