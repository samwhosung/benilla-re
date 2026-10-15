//! The logon link's commands (`Grunt.cpp`; `systems/net/login.md`, `systems/net/logonlink.md`):
//! the challenge it writes and the replies it reads, on the data store of [`crate::DataStore`].
//!
//! A command handler returns 0 when the store does not yet hold the whole command, 1 when the
//! command is malformed, and 2 when it was consumed.

use crate::DataStore;

/// `0x5b90d0`: whether the store holds `n` unread bytes: the cursor is not past the size and
/// `size - cursor >= n`.
pub fn can_read(s: &DataStore, n: u32) -> bool {
    s.read <= s.size && s.size - s.read >= n
}

/// A stack argument of a call out of a handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arg {
    Val(u32),
    /// A pointer into the store, as the position it points at.
    Pos(u32),
}

/// A call a handler makes, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkCall {
    /// `ClientLink::SetState` (`0x5bb430`): the state stored at `+0x80`.
    State(u32),
    /// `0x5d3650` on the SRP6 client at `+0x84`: the positions of `N`, `g`, the salt and `B`,
    /// with the two lengths (the salt and `B` are 32 bytes).
    Srp {
        n: u32,
        n_len: u32,
        g: u32,
        g_len: u32,
        salt: u32,
        b: u32,
    },
    /// The log line `0x6dd240` writes for a logon result of 16 or more.
    LogResult(u32),
    /// A slot of the logon object at `+0x198`, by index, with its arguments.
    Owner(u32, Vec<Arg>),
}

/// `ClientLink::CmdAuthLogonChallenge` (`0x5baa20`): the reply to the logon challenge. `srp` is
/// what `0x5d3650` returns, 0 for success; `b` receives `B`, which the link keeps at `+0x1dc`.
pub fn cmd_auth_logon_challenge(
    s: &mut DataStore,
    srp: u32,
    b: &mut [u8; 32],
) -> (u32, Vec<LinkCall>) {
    let mut calls = Vec::new();
    let byte = |s: &mut DataStore| u32::from(s.get(1).map_or(0, |v| v[0]));
    if !can_read(s, 2) {
        return (0, calls);
    }
    if byte(s) != 0 {
        return (1, calls);
    }
    let result = byte(s);
    if result != 0 {
        if s.read > s.size {
            return (1, calls);
        }
        calls.push(LinkCall::State(2));
        let r = if result >= 0x10 {
            calls.push(LinkCall::LogResult(result));
            0xff
        } else {
            result
        };
        calls.push(LinkCall::Owner(
            10,
            vec![Arg::Val(r), Arg::Val(0), Arg::Val(0)],
        ));
        return (2, calls);
    }
    if !can_read(s, 0x21) {
        return (0, calls);
    }
    let pb = s.get_in_situ(32).unwrap_or(0);
    let g_len = byte(s);
    if !can_read(s, g_len + 1) {
        return (0, calls);
    }
    let pg = s.get_in_situ(g_len).unwrap_or(0);
    let n_len = byte(s);
    if !can_read(s, n_len + 0x30) {
        return (0, calls);
    }
    let pn = s.get_in_situ(n_len).unwrap_or(0);
    let salt = s.get_in_situ(32).unwrap_or(0);
    let crc = s.get_in_situ(16).unwrap_or(0);
    if !can_read(s, 1) {
        return (0, calls);
    }
    let flag = byte(s);
    let (mut extra, mut digest) = (0, Arg::Val(0));
    if flag != 0 {
        if !can_read(s, 0x14) {
            return (0, calls);
        }
        extra = u32::from_le_bytes(s.get(4).unwrap_or_default().try_into().unwrap_or([0; 4]));
        digest = s.get_in_situ(16).map_or(Arg::Val(0), Arg::Pos);
    }
    if s.read > s.size {
        return (1, calls);
    }
    for (i, v) in b.iter_mut().enumerate() {
        *v = s.byte(pb + i as u32);
    }
    calls.push(LinkCall::Srp {
        n: pn,
        n_len,
        g: pg,
        g_len,
        salt,
        b: pb,
    });
    if srp != 0 {
        calls.push(LinkCall::State(2));
        calls.push(LinkCall::Owner(
            10,
            vec![Arg::Val(5), Arg::Val(0), Arg::Val(0)],
        ));
        return (2, calls);
    }
    calls.push(LinkCall::State(4));
    calls.push(LinkCall::Owner(
        6,
        vec![Arg::Val(flag), Arg::Val(extra), digest],
    ));
    calls.push(LinkCall::Owner(5, vec![Arg::Pos(crc)]));
    (2, calls)
}

/// `ClientLink::CmdRealmList` (`0x5ba8d0`): a u16 size, then that many bytes: a u32, a count, the
/// records (a u32, a byte, two C strings read to a capacity of 256, a float, three bytes) and a
/// u16. When the bytes read after the size field equal it, the logon object's slot 13 gets the
/// store with its cursor at the count, and the cursor ends at the end; the second value is that
/// cursor at the call.
pub fn cmd_realm_list(s: &mut DataStore) -> (u32, Option<u32>) {
    let mut scratch = [0u8; 0x100];
    if !can_read(s, 2) {
        return (0, None);
    }
    let size = u32::from(u16::from_le_bytes(
        s.get(2).unwrap_or_default().try_into().unwrap_or([0; 2]),
    ));
    if !can_read(s, size) {
        return (0, None);
    }
    let start = s.read;
    s.get(4);
    let count_at = s.read;
    let count = s.get(1).map_or(0, |v| v[0]);
    for _ in 0..count {
        s.get(4);
        s.get(1);
        s.get_cstring(&mut scratch, false);
        s.get_cstring(&mut scratch, false);
        s.get(4);
        s.get(1);
        s.get(1);
        s.get(1);
        if s.read > s.size {
            return (1, None);
        }
    }
    s.get(2);
    let end = s.read;
    if end > s.size || end.wrapping_sub(start) != size {
        return (1, None);
    }
    (2, Some(count_at))
}

/// What `ClientLink::CmdXferInitiate` (`0x5baf70`) passes to the logon object's slot 9: the
/// name with its NUL, the size as two dwords, and the digest's position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XferOffer {
    pub name: Vec<u8>,
    pub size: (u32, u32),
    pub digest: u32,
}

/// `ClientLink::CmdXferInitiate` (`0x5baf70`): a name length byte, the name, a u64 size and a
/// 16-byte digest.
pub fn cmd_xfer_initiate(s: &mut DataStore) -> (u32, Option<XferOffer>) {
    if !can_read(s, 1) {
        return (0, None);
    }
    let n = s.get(1).map_or(0, |v| v[0]) as usize;
    if !can_read(s, n as u32 + 0x18) {
        return (0, None);
    }
    let mut name = vec![0u8; n];
    s.get_data(&mut name, false);
    name.push(0);
    let size = s.get(8).unwrap_or_default();
    let digest = s.get_in_situ(16).unwrap_or(0);
    if s.read > s.size {
        return (1, None);
    }
    let lo = u32::from_le_bytes(size[0..4].try_into().unwrap());
    let hi = u32::from_le_bytes(size[4..8].try_into().unwrap());
    (
        2,
        Some(XferOffer {
            name,
            size: (lo, hi),
            digest,
        }),
    )
}

/// The client's half of the logon challenge record that `ClientLink::PutChallengeBody` reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChallengeRecord {
    /// `+0x8`, `+0xc`, `+0x10`: the game, platform and system tags.
    pub game: u32,
    pub platform: u32,
    pub system: u32,
    /// `+0x14` to `+0x16`: the version bytes.
    pub version: [u8; 3],
    /// `+0x18`: the build.
    pub build: u16,
    /// `+0x1c`: the locale.
    pub locale: u32,
    /// `+0x20`: the time-zone bias.
    pub timezone: u32,
}

/// `ClientLink::PutChallengeBody` (`0x5bb460`): appends a u16 placeholder, the record's fields in
/// the order game, version bytes, build, platform, system, locale, time zone, then the link's
/// address (`+0x7c`), the account's length as a byte and the account's first `length & 0xff`
/// bytes; then sets the placeholder to the bytes after it, `(size - start - 2) & 0xffff`. The
/// senders copy the account to `+0x19c` with `SStrCopy` bounded at `0x40` (`0x5bb55a`,
/// `0x5bb6ea`), so it is at most 63 bytes and the length byte never wraps in the client.
pub fn put_challenge_body(s: &mut DataStore, r: &ChallengeRecord, address: u32, account: &[u8]) {
    let start = s.size;
    s.put(&0u16.to_le_bytes());
    s.put(&r.game.to_le_bytes());
    s.put(&r.version[0..1]);
    s.put(&r.version[1..2]);
    s.put(&r.version[2..3]);
    s.put(&r.build.to_le_bytes());
    s.put(&r.platform.to_le_bytes());
    s.put(&r.system.to_le_bytes());
    s.put(&r.locale.to_le_bytes());
    s.put(&r.timezone.to_le_bytes());
    s.put(&address.to_le_bytes());
    let len = account.len() as u8;
    s.put(&[len]);
    s.put_array(Some(&account[..len as usize]), u32::from(len));
    let rest = (s.size.wrapping_sub(start) & 0xffff).wrapping_sub(2) as u16;
    s.set_at(start, &rest.to_le_bytes());
}

/// The command bytes of the link's table (`0x85e278`), in table order: logon challenge, logon
/// proof, reconnect challenge, reconnect proof, realm list, transfer initiate, transfer data.
pub const LINK_COMMANDS: [u8; 7] = [0x00, 0x01, 0x02, 0x03, 0x10, 0x30, 0x31];

/// How `ClientLink::OnData` ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OnDataEnd {
    /// The store keeps the bytes from the end of the last consumed command.
    Keep,
    /// An unknown command byte or a malformed command: the byte it logs, the position and length
    /// of the hex dump (none when the bytes cannot be borrowed), and a disconnect.
    Bad {
        command: u8,
        dump: Option<(u32, u32)>,
    },
}

/// `CDataStore::Reset` (`0x401d20`, slot 6 of the plain store): an `alloc` of `0xffffffff` (a
/// borrowed buffer) is dropped, a window not at 0 is moved there through slot 3, and the size
/// and cursor become 0 and `0xffffffff`.
pub fn reset(s: &mut DataStore) {
    if s.alloc == 0xffff_ffff {
        s.alloc = 0;
    }
    if s.base > 0 {
        s.set_at(0, &[]);
    }
    s.size = 0;
    s.read = 0xffff_ffff;
}

/// `ClientLink::OnData` (`0x5bb0e0`) on the link's plain store: appends `data`, sets the cursor
/// to 0 (slot 7), and reads commands while a byte remains: an unknown byte is bad; a known one
/// calls its handler, `handler(index, store)`, whose 0 stops, 1 is bad, 2 marks the cursor as
/// consumed, and any other value reads on. Stopping keeps the bytes from the mark: they are
/// borrowed, the store is reset and they are appended again. A bad command logs its byte, dumps
/// up to 128 bytes from the cursor, resets the store and disconnects.
pub fn on_data(
    s: &mut DataStore,
    data: &[u8],
    mut handler: impl FnMut(usize, &mut DataStore) -> u32,
) -> OnDataEnd {
    s.put_array(Some(data), data.len() as u32);
    s.read = 0;
    let mut mark = 0u32;
    let bad = loop {
        if !can_read(s, 1) {
            break None;
        }
        let command = s.get(1).map_or(0, |v| v[0]);
        let Some(i) = LINK_COMMANDS.iter().position(|&c| c == command) else {
            break Some(command);
        };
        match handler(i, s) {
            0 => break None,
            1 => break Some(command),
            2 => mark = s.read,
            _ => {}
        }
    };
    match bad {
        None => {
            let n = s.size.wrapping_sub(mark);
            s.read = mark;
            let at = s.get_in_situ(n);
            let kept: Vec<u8> = match at {
                Some(p) => (0..n).map(|i| s.byte(p + i)).collect(),
                None => Vec::new(),
            };
            s.read = 0xffff_ffff;
            reset(s);
            if n != 0 {
                s.put_array(Some(&kept), n);
            }
            OnDataEnd::Keep
        }
        Some(command) => {
            let mut n = s.size.wrapping_sub(s.read);
            if n >= 0x80 {
                n = 0x80;
            }
            let at = s.get_in_situ(n);
            let dump = match at {
                Some(p) if n != 0 => Some((p, n)),
                _ => None,
            };
            s.read = 0xffff_ffff;
            reset(s);
            OnDataEnd::Bad { command, dump }
        }
    }
}

/// `Grunt::Pending::Expire` (`0x5b8650`, `(force)`) on a pending list whose nodes carry the
/// times `times` (`+0x10`, 0 for none), in list order: the index of the one node it unlinks,
/// hands to the owner's slot 0 and frees, or `None`. Without `force` the walk skips a node of
/// time 0 and stops at the first whose age `now - time` is below `timeout` (`+4`); with it the
/// first node goes. A moved node's link then belongs to the local list, so the walk reads its
/// end there and stops: one node goes per call.
pub fn pending_expire(times: &[u32], force: bool, now: u32, timeout: u32) -> Option<usize> {
    for (i, &t) in times.iter().enumerate() {
        if !force {
            if t == 0 {
                continue;
            }
            if now.wrapping_sub(t) < timeout {
                return None;
            }
        }
        return Some(i);
    }
    None
}

/// `ServerLink::CmdGruntAuthVerify` (`0x5b9630`), a server's handler: a length byte, then that
/// many bytes of name, 16 and 20 bytes, all read and dropped. 0 while the store lacks the
/// length byte or the rest, 1 for a length of `0x20` or more, which reads no further; else 2,
/// or 1 if the reads ran past the size.
pub fn cmd_grunt_auth_verify(s: &mut DataStore) -> u32 {
    if !can_read(s, 1) {
        return 0;
    }
    let len = s.get(1).map_or(0, |v| v[0]);
    if len >= 0x20 {
        return 1;
    }
    if !can_read(s, u32::from(len) + 0x24) {
        return 0;
    }
    for n in [usize::from(len), 16, 20] {
        let mut buf = vec![0u8; n];
        s.get_data(&mut buf, false);
    }
    if s.size < s.read {
        1
    } else {
        2
    }
}

/// What `ServerLink::CmdGruntHello` (`0x5b8c40`) reads from the realm source's slot 4: the
/// 0x40-byte record at the reply's heart.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HelloInfo {
    /// `+0`, `+4` (bit 0 adds a log line), `+0xc` (0 for the link's own `+0x7c`), `+0x10`.
    pub id: u32,
    pub flags: u8,
    pub addr: u32,
    pub port: u16,
    /// `+0x12`, `+0x13`, `+0x14`, `+0x16`, `+0x18`, `+0x1c`.
    pub b12: u8,
    pub b13: u8,
    pub b14: u8,
    pub w16: u16,
    pub d18: u32,
    pub b1c: u8,
    /// `+0x1d`: a C string of up to 34 bytes.
    pub name: Vec<u8>,
}

/// What the handler does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelloEffect {
    pub result: u32,
    /// The u32 stored at `+0x74`.
    pub value: Option<u32>,
    /// The reply sent through `ServerLink::Send` (`0x5b9b00`), after which the state `+0x90`
    /// is 3.
    pub reply: Option<Vec<u8>>,
}

/// `ServerLink::CmdGruntHello` (`0x5b8c40`), command `0x20` of the server's table: 0 while the
/// store lacks 4 bytes; else the u32 goes to `+0x74` and the mode `+0x104` decides. Mode 1
/// replies `20 00`. Mode 0 asks the realm source (`info`, `None` when its slot 4 returns 0,
/// which returns 1 with no reply) and replies `20 0e`, a u16 length of what follows, the name's
/// length byte and bytes, the address (or `own_addr`), the port, the id, the flag byte, the
/// three bytes, the u16, the u32, the byte and `tail` (`+0x78`). Any other mode returns 1. A
/// reply returns 2.
pub fn cmd_grunt_hello(
    s: &mut DataStore,
    mode: u32,
    info: Option<&HelloInfo>,
    own_addr: u32,
    tail: u32,
) -> HelloEffect {
    if !can_read(s, 4) {
        return HelloEffect {
            result: 0,
            value: None,
            reply: None,
        };
    }
    let value = s.get(4).map(|v| u32::from_le_bytes(v.try_into().unwrap()));
    let none = |result| HelloEffect {
        result,
        value,
        reply: None,
    };
    let mut r = vec![0x20];
    match (mode, info) {
        (1, _) => r.push(0),
        (0, Some(i)) => {
            r.extend([0x0e, 0, 0]);
            r.push(i.name.len() as u8);
            r.extend(&i.name);
            r.extend((if i.addr == 0 { own_addr } else { i.addr }).to_le_bytes());
            r.extend(i.port.to_le_bytes());
            r.extend(i.id.to_le_bytes());
            r.extend([i.flags, i.b12, i.b13, i.b14]);
            r.extend(i.w16.to_le_bytes());
            r.extend(i.d18.to_le_bytes());
            r.push(i.b1c);
            r.extend(tail.to_le_bytes());
            let n = (r.len() - 4) as u16;
            r[2..4].copy_from_slice(&n.to_le_bytes());
        }
        _ => return none(1),
    }
    HelloEffect {
        result: 2,
        value,
        reply: Some(r),
    }
}
