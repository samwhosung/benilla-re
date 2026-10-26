//! The add-on record lookups by folder name (`systems/ui/addoninfo.md#finding-a-record-by-name`):
//! `0x51e050` answers the `Notes` metadata, `0x51e180` the URL and `0x51e3e0` the update byte.
//!
//! The client walks a hash keyed by `SStrHashHT` of the name and takes a record whose stored hash
//! equals the name's and whose name `SStrCmpI` finds equal, ignoring ASCII case. With each
//! record stored under its own name's hash, that is the record whose name equals the asked one
//! ignoring ASCII case.

/// One add-on record, as far as the three lookups read it.
#[derive(Clone, Copy, Debug)]
pub struct AddOnRecord<'a> {
    /// The folder name, `+0x14`.
    pub name: &'a [u8],
    /// The metadata pairs, key and value, held in the record's own hash `+0xb4`.
    pub metadata: &'a [(&'a [u8], &'a [u8])],
    /// The URL `+0x30`, `None` for a null pointer.
    pub url: Option<&'a [u8]>,
    /// The security `+0x24`: 0 secure, 1 insecure, 2 banned.
    pub security: u32,
    /// The update byte `+0xc0`.
    pub update: u8,
}

fn same_name(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// The record of `name`, ignoring ASCII case.
pub fn find<'r, 'a>(records: &'r [AddOnRecord<'a>], name: &[u8]) -> Option<&'r AddOnRecord<'a>> {
    records.iter().find(|r| same_name(r.name, name))
}

/// `0x51e050`: the value of the record's `Notes` metadata, the key compared ignoring ASCII case.
pub fn notes<'a>(records: &[AddOnRecord<'a>], name: &[u8]) -> Option<&'a [u8]> {
    let r = find(records, name)?;
    r.metadata
        .iter()
        .find(|(k, _)| same_name(k, b"Notes"))
        .map(|&(_, v)| v)
}

/// `0x51e180`: the URL when it is not null, not empty and the add-on is not banned (security 2).
pub fn url<'a>(records: &[AddOnRecord<'a>], name: &[u8]) -> Option<&'a [u8]> {
    let r = find(records, name)?;
    r.url.filter(|u| !u.is_empty() && r.security != 2)
}

/// `0x51e3e0`: the update byte, 0 for an unknown name.
pub fn update_byte(records: &[AddOnRecord<'_>], name: &[u8]) -> u8 {
    find(records, name).map_or(0, |r| r.update)
}
