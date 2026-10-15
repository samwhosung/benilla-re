//! The realm list the logon object fills from the realm list packet (`Login.cpp`;
//! `systems/net/logonlink.md#realm-record-fill`).
//!
//! The holder keeps an array at `+0x4`: its allocated count at `+0x4`, its count at `+0x8` and
//! its records at `+0xc`, each [`REALM_RECORD`] bytes; and the packet's tail u16 at `+0x10`.

use crate::DataStore;

/// The size of a realm record.
pub const REALM_RECORD: usize = 0x134;

/// The characters a realm name may not hold: each becomes a space.
pub const NAME_FORBIDDEN: &[u8] = b"\"*/:<>?\\|";

/// The realm holder's state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealmHolder {
    /// `+0x4` and `+0x8`.
    pub alloc: u32,
    pub count: u32,
    /// The records `+0xc` points at, `count` of them.
    pub records: Vec<[u8; REALM_RECORD]>,
    /// `+0x10`.
    pub tail: [u8; 2],
}

/// `IsFileNameChar` (`0x5b24b0`, `(c, keep_space)`): 0 for a forbidden character, and for a space
/// or a dot when `keep_space` is not 0; 1 for every other character, those below `0x20` or above
/// `0x7c` and the negative ones included.
pub fn is_file_name_char(c: u8, keep_space: u8) -> u8 {
    let i = (c as i8 as i32) - 0x20;
    if !(0..=0x5c).contains(&i) {
        return 1;
    }
    if NAME_FORBIDDEN.contains(&c) {
        0
    } else if c == b' ' || c == b'.' {
        u8::from(keep_space == 0)
    } else {
        1
    }
}

/// Cleans a realm name in place: each forbidden character becomes a space, then trailing spaces,
/// dots and forbidden characters are cut by a NUL.
pub fn clean_realm_name(name: &mut [u8]) {
    let mut n = 0;
    while name[n] != 0 {
        if NAME_FORBIDDEN.contains(&name[n]) {
            name[n] = b' ';
        }
        n += 1;
    }
    while n > 0 && is_file_name_char(name[n - 1], 1) == 0 {
        n -= 1;
    }
    name[n] = 0;
}

/// Reads up to `count` records into `records`, stopping after a record whose reads ran past the
/// end, which is left as read and not cleaned. A clean record has its population rewritten:
/// 600.0 becomes 0.0 with flag `0x20` set in the byte at `+8`, 200.0 becomes 0.001
/// (`0x3a83126f`) with `0x40`, 400.0 becomes 8.0 with `0x80`.
fn read_records(s: &mut DataStore, records: &mut [[u8; REALM_RECORD]]) {
    for (i, r) in records.iter_mut().enumerate() {
        r[0] = i as u8;
        if let Some(v) = s.get(4) {
            r[4..8].copy_from_slice(&v);
        }
        if let Some(v) = s.get(1) {
            r[8] = v[0];
        }
        s.get_cstring(&mut r[9..0x109], false);
        s.get_cstring(&mut r[0x109..0x129], false);
        if let Some(v) = s.get(4) {
            r[0x12c..0x130].copy_from_slice(&v);
        }
        for k in 0..3 {
            if let Some(v) = s.get(1) {
                r[0x130 + k] = v[0];
            }
        }
        if s.read > s.size {
            return;
        }
        let pop = u32::from_le_bytes(r[0x12c..0x130].try_into().unwrap());
        let (to, flag) = match pop {
            0x4416_0000 => (0, 0x20),
            0x4348_0000 => (0x3a83_126f, 0x40),
            0x43c8_0000 => (0x4100_0000, 0x80),
            _ => (pop, 0),
        };
        if flag != 0 {
            r[0x12c..0x130].copy_from_slice(&to.to_le_bytes());
            r[8] |= flag;
        }
        clean_realm_name(&mut r[9..0x109]);
    }
}

impl RealmHolder {
    fn clear(&mut self) {
        self.alloc = 0;
        self.count = 0;
        self.records.clear();
    }

    fn resize(&mut self, n: u32) {
        self.alloc = n;
        self.count = n;
        self.records = vec![[0u8; REALM_RECORD]; n as usize];
    }

    /// The count byte, the records, the tail u16 into `+0x10`, then the slot 5 status: 0 when the
    /// store was read exactly to its end, else 4 with the array emptied.
    fn fill(&mut self, s: &mut DataStore) -> u32 {
        let count = s.get(1).map_or(0, |v| v[0]);
        if u32::from(count) != self.count {
            if count == 0 {
                self.clear();
            } else {
                self.resize(u32::from(count));
            }
        }
        let mut records = std::mem::take(&mut self.records);
        read_records(s, &mut records);
        self.records = records;
        if let Some(v) = s.get(2) {
            self.tail = [v[0], v[1]];
        }
        if s.read == s.size {
            0
        } else {
            self.clear();
            4
        }
    }

    /// `HandleRealmList` (`0x5b2230`) with the store, or `None` for a null store or one whose
    /// cursor is `0xffffffff`, which goes to [`RealmHolder::handle_result`] with 4. The array is
    /// emptied first when it holds records. Returns slot 5's argument. The store holds at least
    /// one unread byte: with none, the count is read from stale stack.
    pub fn handle_realm_list(&mut self, s: Option<&mut DataStore>) -> u32 {
        let Some(s) = s.filter(|s| s.read != 0xffff_ffff) else {
            return self.handle_result(4, None);
        };
        if self.count != 0 {
            self.clear();
        }
        self.fill(s)
    }

    /// `HandleRealmListResult` (`0x5b2560`, `(result, store)`): empties a non-empty array, then
    /// passes a non-zero result to slot 5; a zero result fills the array from the store as
    /// [`RealmHolder::handle_realm_list`] does.
    pub fn handle_result(&mut self, result: u32, s: Option<&mut DataStore>) -> u32 {
        if self.count != 0 {
            self.clear();
        }
        match (result, s) {
            (0, Some(s)) => self.fill(s),
            (r, _) => r,
        }
    }
}
