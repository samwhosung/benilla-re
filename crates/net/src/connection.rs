//! The world connection's callback bracket (`WowConnection.cpp`; `systems/net/threads.md`).

/// The connection's fields the bracket keeps, under its lock at `+0x9c`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CallbackBracket {
    /// `+0xb4`: how many callbacks are running.
    pub depth: u32,
    /// `+0xb8`: the thread that entered last.
    pub thread: u32,
    /// `+0xbc`: a value held back for `+0x14` until the callback ends; 0 for none.
    pub deferred: u32,
    /// `+0x14`.
    pub field14: u32,
}

impl CallbackBracket {
    /// `WowConnection::BeginCallback` (`0x5b7260`): one more callback, entered on `thread`
    /// (`0x64bbd0`).
    pub fn begin(&mut self, thread: u32) {
        self.depth = self.depth.wrapping_add(1);
        self.thread = thread;
    }

    /// `WowConnection::EndCallback` (`0x5b7290`): one fewer; a held value moves to `+0x14` and
    /// the hold clears, whatever the depth.
    pub fn end(&mut self) {
        self.depth = self.depth.wrapping_sub(1);
        if self.deferred != 0 {
            self.field14 = self.deferred;
            self.deferred = 0;
        }
    }
}

/// `ClientServices::CheckCharacterName` (`0x5abd50`): the result `ValidateCharacterName`
/// (`0x6c9ae0`) gives for the name in the current locale, plus `0x45`, so that 0 is
/// `CHAR_NAME_NO_NAME` and 13 `CHAR_NAME_SUCCESS`.
pub fn check_character_name(validator: u32) -> u32 {
    validator.wrapping_add(0x45)
}
