//! The five InventoryType-to-body-slot jump-table dispatchers of the character component.
//!
//! Each is a two-level dispatch: `if arg > MAX { default }; slot = byte_lut[arg]; jmp
//! [dword_table + 4*slot]`. A byte LUT folds the 19, 25 or 27 sparse InventoryType ids onto a small
//! dense set of handler slots, and a dword jump table routes each slot to its handler. The
//! mapping `arg -> slot -> handler target` is the LUT and the table, reproduced here from the
//! image at the cited addresses; the handlers themselves are not modelled. For every in-range
//! argument the test compares the slot register and the resolved jump target of the exe's own code
//! with these tables.

/// A two-level InventoryType dispatcher: a per-arg byte LUT (`lut[arg]` is the slot) feeding a dword
/// jump table (`table[slot]` is the handler address). `max` is the inclusive arg bound; an arg
/// above it takes the early default and does not jump. The mapping is `arg -> lut[arg] ->
/// table[lut[arg]]`.
pub struct Dispatch {
    /// Address of the function entry.
    pub fn_va: u32,
    /// Address of the `jmp dword ptr [4*slot + table_va]` instruction, where the test's slice halts.
    pub jmp_va: u32,
    /// Address of the dword jump table.
    pub table_va: u32,
    /// Address of the byte index LUT.
    pub lut_va: u32,
    /// Inclusive maximum arg the bound check admits (`cmp arg, MAX; jg/ja default`).
    pub max: u32,
    /// The bound check's signedness: `true` is `jg` (`0x478bd0`, `0x478cb0`), `false` is `ja`
    /// (`0x478f20`, `0x479010`). A signed bound does not catch a high-bit arg (`0xffffffff` is -1),
    /// which then indexes the LUT at a wild address; [`Dispatch::in_range`] reflects it.
    pub signed_bound: bool,
    /// Address the early-default branch (`jg` or `ja`) lands on.
    pub default_va: u32,
    /// The byte index LUT: `lut[arg]` is the jump-table slot. Its length is `max + 1`.
    pub lut: &'static [u8],
    /// The dword jump table: `table[slot]` is the handler address the `jmp` resolves to.
    pub table: &'static [u32],
}

impl Dispatch {
    /// `arg -> slot` (the `movzx <reg>, byte ptr [arg + lut_va]`), for `arg <= max`.
    #[inline]
    pub fn slot(&self, arg: u32) -> u8 {
        self.lut[arg as usize]
    }
    /// `arg -> handler address` (the full `jmp [4*slot + table_va]` resolution), for `arg <= max`.
    #[inline]
    pub fn target(&self, arg: u32) -> u32 {
        self.table[self.slot(arg) as usize]
    }
    /// Whether `arg` reaches the jump table, so the dispatcher indexes `lut[arg]`. The unsigned
    /// bound (`ja`) is taken for `arg > max`; the signed bound (`jg`) only for `(arg as i32) > max`,
    /// so a high-bit arg passes the signed guard and the exe reads the LUT out of bounds, a case
    /// the tests do not probe. An arg out of range takes the early default.
    #[inline]
    pub fn in_range(&self, arg: u32) -> bool {
        if self.signed_bound {
            (arg as i32) <= (self.max as i32)
        } else {
            arg <= self.max
        }
    }
}

/// `0x478bd0`: handlers `push N; call 0x478960`; arg <= 0x12.
/// LUT at `0x478c98` (19 bytes), jump table at `0x478c68` (12 dwords; slot 0x0b = the no-op default
/// `pop ebp; ret 4`). Slot register: `eax`.
pub const D_478BD0: Dispatch = Dispatch {
    fn_va: 0x478bd0,
    jmp_va: 0x478be6,
    table_va: 0x478c68,
    lut_va: 0x478c98,
    max: 0x12,
    signed_bound: true,
    default_va: 0x478c62,
    lut: &[
        0x00, 0x0b, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x0b, 0x0b, 0x0b, 0x0b, 0x09,
        0x0b, 0x0b, 0x0b, 0x0a,
    ],
    table: &[
        0x478bed, 0x478bf8, 0x478c03, 0x478c0e, 0x478c19, 0x478c24, 0x478c2f, 0x478c3a, 0x478c45,
        0x478c5b, 0x478c50, 0x478c62,
    ],
};

/// `0x478cb0`: handlers `push displayId; push N; call 0x478aa0`; arg <= 0x12.
/// LUT at `0x478da4` (19 bytes), jump table at `0x478d74` (12 dwords; slot 0x0b = default `0x478d6e`).
/// Slot register: `eax`.
pub const D_478CB0: Dispatch = Dispatch {
    fn_va: 0x478cb0,
    jmp_va: 0x478cc6,
    table_va: 0x478d74,
    lut_va: 0x478da4,
    max: 0x12,
    signed_bound: true,
    default_va: 0x478d6e,
    lut: &[
        0x00, 0x0b, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x0b, 0x0b, 0x0b, 0x0b, 0x09,
        0x0b, 0x0b, 0x0b, 0x0a,
    ],
    table: &[
        0x478ccd, 0x478cdc, 0x478ceb, 0x478cfa, 0x478d09, 0x478d18, 0x478d27, 0x478d36, 0x478d45,
        0x478d63, 0x478d54, 0x478d6e,
    ],
};

/// `0x478f20`: handlers `push N; call 0x478960`; arg <= 0x18.
/// LUT at `0x478fec` (25 bytes), jump table at `0x478fb8` (13 dwords; slots 0 and 12 = the no-op
/// default `0x478fb2`). Slot register: `eax`.
pub const D_478F20: Dispatch = Dispatch {
    fn_va: 0x478f20,
    jmp_va: 0x478f36,
    table_va: 0x478fb8,
    lut_va: 0x478fec,
    max: 0x18,
    signed_bound: false,
    default_va: 0x478fb2,
    lut: &[
        0x00, 0x01, 0x00, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x00, 0x00, 0x0c, 0x0c,
        0x00, 0x0a, 0x0c, 0x00, 0x0b, 0x04, 0x0c, 0x0c, 0x0c, 0x00,
    ],
    table: &[
        0x478fb2, 0x478f3d, 0x478f48, 0x478f53, 0x478f5e, 0x478f69, 0x478f74, 0x478f7f, 0x478f8a,
        0x478f95, 0x478fab, 0x478fa0, 0x478fb2,
    ],
};

/// `0x479010`: first calls `0x478f20(arg0)`, then guards `arg1 > 0` (signed) and `arg0 <= 0x1a`
/// before dispatching `arg0`. Handlers `push arg1; push N; call 0x478aa0`, plus three that call
/// other functions (slot 10 calls `0x479e80`, slot 11 `0x479f00`, slot 14 `0x479ec0`). LUT at
/// `0x479158` (27 bytes), jump table at `0x47911c` (15 dwords; slot 0 is the default `0x479116`).
/// The slot register is `ecx`, not `eax`; the dispatch reads `arg0` from `edi` (`[ebp+8]`).
pub const D_479010: Dispatch = Dispatch {
    fn_va: 0x479010,
    jmp_va: 0x47903b,
    table_va: 0x47911c,
    lut_va: 0x479158,
    max: 0x1a,
    signed_bound: false,
    default_va: 0x479116,
    lut: &[
        0x00, 0x01, 0x00, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x00, 0x00, 0x0a, 0x0b,
        0x00, 0x0c, 0x0a, 0x00, 0x0d, 0x04, 0x0a, 0x0e, 0x00, 0x00, 0x00, 0x00,
    ],
    table: &[
        0x479116, 0x479042, 0x479052, 0x479062, 0x479072, 0x479082, 0x479092, 0x4790a2, 0x4790b2,
        0x4790c2, 0x4790f0, 0x4790e2, 0x47910c, 0x4790d2, 0x4790fe,
    ],
};
