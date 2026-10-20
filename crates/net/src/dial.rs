//! How an address text becomes a dial (`systems/net/dial.md`): the logon server's realmlist text
//! resolved to a list of socket addresses by the resolver thread (`0x43b890`), the world server's
//! `host:port` text split and parsed by `NetClient::Connect` (`0x537820`), and the address the
//! connection stores for it (`0x5b6e40`).
//!
//! The system calls `inet_addr` and `gethostbyname` are the platform's, so the models take them as
//! parameters.

use benilla_re_storm::str_to_int;

/// `_atol` (`0x414411`) and `_atoi` (`0x41449c`, which calls it) of a C string, with the C locale
/// the client runs in: leading white space (tab to carriage return, and space) is skipped, one
/// `-` or `+` is read, then decimal digits accumulate in wrapping 32-bit arithmetic until the
/// first non-digit; a `-` negates last. No digit gives 0.
pub fn atol(text: &[u8]) -> i32 {
    let text = &text[..text.iter().position(|&b| b == 0).unwrap_or(text.len())];
    let mut i = 0;
    while i < text.len() && matches!(text[i], 9..=13 | 32) {
        i += 1;
    }
    let sign = text.get(i).copied().unwrap_or(0);
    if sign == b'-' || sign == b'+' {
        i += 1;
    }
    let mut v: u32 = 0;
    while i < text.len() && text[i].is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add(u32::from(text[i] - b'0'));
        i += 1;
    }
    if sign == b'-' {
        v.wrapping_neg() as i32
    } else {
        v as i32
    }
}

/// The default world port, 9090 (`0x2382`), which `NetClient::Connect` (`0x537820`) uses for an
/// address with no `:`.
pub const WORLD_PORT: u32 = 9090;

/// The host text and port dword `NetClient::Connect` (`0x537820`) passes to its dial.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldDial {
    /// The address copied into a 1,024-byte buffer (1,023 bytes and the NUL), cut at its first `:`.
    pub host: Vec<u8>,
    /// `_atoi` of the text after the `:` (0 for none), or 9090 for an address with no `:`. The
    /// connection keeps its low 16 bits (`0x5b6e94`).
    pub port: u32,
}

/// `NetClient::Connect` (`0x537820`) in state 2: the address text split at the first `:`.
pub fn world_dial(address: &[u8]) -> WorldDial {
    let address = &address[..address
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(address.len())];
    let copy = &address[..address.len().min(0x3ff)];
    match copy.iter().position(|&b| b == b':') {
        Some(i) => WorldDial {
            host: copy[..i].to_vec(),
            port: atol(&copy[i + 1..]) as u32,
        },
        None => WorldDial {
            host: copy.to_vec(),
            port: WORLD_PORT,
        },
    }
}

/// The address `WowConnection::ConnectToHost` (`0x5b6e40`) stores at `+0x64` for a host text:
/// `inet_addr`'s result unless it is `0xffffffff` or 0, else the first address `gethostbyname`
/// returns, its four bytes read as a little-endian dword, else 0. `first_address` is that lookup,
/// `None` when it fails. The hostent's address type and length are not read.
pub fn host_address(
    host: &[u8],
    inet_addr: impl Fn(&[u8]) -> u32,
    first_address: impl Fn(&[u8]) -> Option<u32>,
) -> u32 {
    let a = inet_addr(host);
    if a != 0xffff_ffff && a != 0 {
        return a;
    }
    first_address(host).unwrap_or(0)
}

/// A 16-byte `sockaddr_in`: the family 2, the port in network order, the address as stored and
/// eight zero bytes.
pub fn sockaddr_in(address: u32, port: u16) -> [u8; 16] {
    let mut s = [0u8; 16];
    s[0] = 2;
    s[2..4].copy_from_slice(&port.to_be_bytes());
    s[4..8].copy_from_slice(&address.to_le_bytes());
    s
}

/// `0x43f3d0` on a piece of the resolver's list that begins with a digit. With a `:` in it, the
/// text before it (at most 31 bytes) goes to `inet_addr` and `SStrToInt` of the text after it is
/// the port; without one, the whole piece goes to `inet_addr` and `port` is the request's. The
/// address `inet_addr` returns is stored whatever it is, `0xffffffff` for a text it cannot read.
/// The port is the low 16 bits.
pub fn digit_piece(piece: &[u8], port: u32, inet_addr: impl Fn(&[u8]) -> u32) -> [u8; 16] {
    let piece = &piece[..piece.iter().position(|&b| b == 0).unwrap_or(piece.len())];
    match piece.iter().position(|&b| b == b':') {
        Some(i) => {
            let text = &piece[..i.min(0x1f)];
            let port = str_to_int(&piece[i + 1..]) as u32;
            sockaddr_in(inet_addr(text), port as u16)
        }
        None => sockaddr_in(inet_addr(piece), port as u16),
    }
}

/// What `gethostbyname` returns for a name, as the resolver reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hostent {
    /// `h_addrtype` (`+8`); 2 is `AF_INET`.
    pub addr_type: u16,
    /// `h_length` (`+0xa`).
    pub length: u16,
    /// The four bytes each `h_addr_list` entry points to, as a little-endian dword.
    pub addrs: Vec<u32>,
}

/// The list the resolver thread (`0x43b890`) builds, and whether it marks the request done.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The `sockaddr_in` entries, in the order the pieces and their addresses came.
    pub entries: Vec<[u8; 16]>,
    /// Whether the request ends marked done (`+0x20`). A host entry with no address leaves the
    /// thread without marking it.
    pub done: bool,
}

/// The resolver thread (`0x43b890`) over a list text and a port: it takes pieces up to `;`, each
/// at most 1,023 bytes, and ends, done, at the first empty one. A piece whose first byte is a
/// digit (signed compare, so a byte of `0x80` and up is not) goes to [`digit_piece`]. Any other
/// piece goes whole to `gethostbyname`; a name it does not find, or finds with an address type
/// other than 2 or a length other than 4, adds nothing, and each address of one it finds adds an
/// entry with the request's port.
pub fn resolve_list(
    text: &[u8],
    port: u16,
    inet_addr: impl Fn(&[u8]) -> u32,
    gethostbyname: impl Fn(&[u8]) -> Option<Hostent>,
) -> Resolved {
    let text = &text[..text.iter().position(|&b| b == 0).unwrap_or(text.len())];
    let mut entries = Vec::new();
    let mut rest = Some(text);
    loop {
        let piece: &[u8] = match rest {
            Some(r) => match r.iter().position(|&b| b == b';') {
                Some(i) => {
                    rest = Some(&r[i + 1..]);
                    &r[..i]
                }
                None => {
                    rest = None;
                    r
                }
            },
            None => &[],
        };
        let piece = &piece[..piece.len().min(0x3ff)];
        let Some(&first) = piece.first() else {
            return Resolved {
                entries,
                done: true,
            };
        };
        // `cmp al,0x30; jl` and `cmp al,0x39; jg` compare signed, so a byte of 0x80 and up is not a digit.
        if first.is_ascii_digit() {
            entries.push(digit_piece(piece, u32::from(port), &inet_addr));
            continue;
        }
        let Some(h) = gethostbyname(piece) else {
            continue;
        };
        if h.addr_type != 2 || h.length != 4 {
            continue;
        }
        if h.addrs.is_empty() {
            return Resolved {
                entries,
                done: false,
            };
        }
        for a in h.addrs {
            entries.push(sockaddr_in(a, port));
        }
    }
}
