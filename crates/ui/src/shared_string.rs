//! The shared path strings fonts and backdrops keep (`systems/ui/fontstring.md`, Shared path
//! strings): a handle whose `+8` points at a counted record of a string pool, and `0x41e3a0`, which
//! points the handle at a new string.

/// The record a null string shares (`0x883e78`); its text pointer `+0x1c` is 0.
pub const NULL_RECORD: u32 = 0x0088_3e78;

/// The length from which a string is refused.
pub const MAX_LEN: usize = 0x400;

/// What `0x41e3a0(handle, text)` does, in order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Assign {
    /// The pool getter `0x41ddd0` was called.
    pub asked_pool: bool,
    /// The pool's `0x41e030` was called with the text.
    pub asked_intern: bool,
    /// The record whose count `+4` went up by one, before the release; 0 for none.
    pub add_ref: u32,
    /// The record released through its slot 2, after the count; 0 for none.
    pub release: u32,
    /// The error raised through `SErrSetLastError`, 0 for none.
    pub error: u32,
    /// What the handle's `+8` holds after.
    pub slot: u32,
}

/// `0x41e3a0`: `old` is the handle's `+8`, `text` the string (`None` for a null pointer) without
/// its NUL, `pool` what `0x41ddd0` returns and `interned` what `0x41e030` returns.
pub fn assign_shared_string(old: u32, text: Option<&[u8]>, pool: u32, interned: u32) -> Assign {
    let release = old;
    let Some(text) = text else {
        return Assign {
            add_ref: NULL_RECORD,
            release,
            slot: NULL_RECORD,
            ..Assign::default()
        };
    };
    let len = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    if len >= MAX_LEN {
        return Assign {
            error: 0x57,
            slot: old,
            ..Assign::default()
        };
    }
    if pool == 0 {
        return Assign {
            asked_pool: true,
            error: 0x57,
            slot: old,
            ..Assign::default()
        };
    }
    Assign {
        asked_pool: true,
        asked_intern: true,
        add_ref: interned,
        release,
        error: 0,
        slot: interned,
    }
}
