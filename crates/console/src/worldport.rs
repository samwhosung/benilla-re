//! The `worldport` console command, `CCommand_WorldTeleport` (`0x404dc0`): what it reads from its
//! argument text and the active player, and what it prints or sends (`systems/console/commands.md`,
//! "World port command" and "World port message").
//!
//! The text is split by [`benilla_re_storm::str_tokenize`] with [`DELIMS`], the map is read by
//! [`benilla_re_storm::str_to_unsigned`], and the coordinates by a float reader the caller
//! supplies (`SStrToFloat`, `0x64aaa0`, whose powers of ten come from the C runtime's `pow`). The
//! x87 runs at 53-bit precision, so a coordinate is the reader's `f64` rounded to `f32` at the
//! `fstp dword`, and the facing is the reader's `f64` times the float π/180 in `f64`, then rounded.

use benilla_re_storm::{str_to_unsigned, str_tokenize};

/// The delimiters of the argument text, `0x82ecc8`: tab, CR, LF, quote and space, no comma.
pub const DELIMS: &[u8] = b"\t\r\n\" ";

/// The token buffer, 0x40 bytes: tokens are cut at 63.
pub const TOKEN_SIZE: usize = 0x40;

/// The usage line, `0x82efb0`, printed in class 4.
pub const USAGE: &[u8] = b"Usage: worldport <continentID> [x y z] [facing]";

/// The format of the bad-map line, `0x82ef98`, printed in class 3 with the signed map.
pub const BAD_WORLD: &[u8] = b"Bad world number: %i\n";

/// The float π/180, `s_degreesToRadians` (`0x7ffaac`).
pub const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);

/// What the active player's object answers: its position (vtable slot 5, `+0x14`) and its facing
/// (slot 6, `+0x18`), the defaults of the coordinates.
#[derive(Clone, Copy, Debug)]
pub struct Player {
    pub position: [f32; 3],
    pub facing: f32,
}

/// The `Map.dbc` id map (`0xc0daa8`, `0xc0daac`): the largest id (−1 until the table loads) and,
/// for each id from 0, whether its entry is non-null.
#[derive(Clone, Copy, Debug)]
pub struct Maps<'a> {
    pub largest_id: i32,
    pub has_row: &'a [bool],
}

/// The message `worldport` sends after opcode 8 and the clock: the map as a u32, then the three
/// coordinates and the facing in radians as floats.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Teleport {
    pub map: u32,
    pub position: [f32; 3],
    pub facing: f32,
}

/// What a call does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    /// No active player: nothing is read or printed, and the handler returns 1.
    NoPlayer,
    /// No token: [`USAGE`] in class 4, and the handler returns 0.
    Usage,
    /// The map, as a signed integer, is negative, above the largest id or without a row:
    /// [`BAD_WORLD`] in class 3 with it, and the handler returns 0.
    BadWorld(i32),
    /// The message is sent and the handler returns 1.
    Teleport(Teleport),
}

/// `CCommand_WorldTeleport` (`0x404dc0`) on its argument text `args` (the bytes before the first
/// NUL). `to_float` is `SStrToFloat` on one token.
///
/// The order is the exe's: the player, the first token (empty prints the usage), the map read, the
/// four coordinate tokens, then the map test. A coordinate token that exists replaces its default,
/// text that is no number reading as 0.
pub fn worldport(
    args: &[u8],
    player: Option<&Player>,
    maps: &Maps<'_>,
    to_float: impl Fn(&[u8]) -> f64,
) -> Outcome {
    let Some(player) = player else {
        return Outcome::NoPlayer;
    };
    let mut at = 0;
    let mut token = || {
        let t = str_tokenize(&args[at.min(args.len())..], DELIMS, TOKEN_SIZE);
        at += t.next;
        t.text
    };
    let first = token();
    if first.is_empty() {
        return Outcome::Usage;
    }
    let map = str_to_unsigned(&first);
    let mut position = player.position;
    for slot in &mut position {
        let t = token();
        if !t.is_empty() {
            *slot = to_float(&t) as f32;
        }
    }
    let mut facing = player.facing;
    let t = token();
    if !t.is_empty() {
        facing = (to_float(&t) * f64::from(DEGREES_TO_RADIANS)) as f32;
    }
    let id = map as i32;
    let known =
        id >= 0 && id <= maps.largest_id && maps.has_row.get(id as usize).copied().unwrap_or(false);
    if !known {
        return Outcome::BadWorld(id);
    }
    Outcome::Teleport(Teleport {
        map,
        position,
        facing,
    })
}
