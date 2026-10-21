//! `CGPlayer_C::SetAutoLoot` (`0x5df460`): the player's auto-loot byte and the sweep it runs
//! (`systems/objectlayer/interaction.md#the-auto-loot-byte`).
//!
//! The function stores the low byte of its argument at `+0x1d30` and, when that byte is not 0 and
//! the player holds a loot target (`+0x1d28` or `+0x1d2c` not 0), calls the auto-loot sweep
//! `0x4c1fa0` with 0 in `ecx`.

/// `AutoLootAll`, the sweep, called with `ecx` = 0.
pub const SWEEP: u32 = 0x4c1fa0;

/// What `0x5df460` stores and whether it calls the sweep, for the argument `arg` and the player's
/// loot target `target` (both dwords of `+0x1d28`, `+0x1d2c` OR-ed).
pub fn set_auto_loot(arg: u32, target: u64) -> (u8, bool) {
    let byte = arg as u8;
    (byte, byte != 0 && target != 0)
}
