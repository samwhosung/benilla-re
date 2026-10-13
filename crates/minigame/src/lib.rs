//! `Minigame_C.cpp` and the board file linked after it, the client side of a board minigame the
//! server runs. The client's one computation is whether a move is legal; turns and wins are the
//! server's.

/// `0x728b80`, in the board file, called by `0x728740`, the class's vtable slot 1: a move is legal
/// when `0 <= idx <= 9` and `board[idx] == 0xFF`, an empty cell.
///
/// The bounds are `test; jl` and `cmp 9; jg`. The cell index is then rebuilt through `idiv 3`:
/// `r = idx/3`, `c = idx%3`, `(c + 2r) + r`, which is `idx`. `board` is read at `[ecx+0x22+idx]`;
/// `idx == 9` reads one byte past the nine cells, so `board` holds at least 10 bytes.
#[must_use]
pub fn minigame_move_legal(idx: i32, board: &[u8]) -> bool {
    if !(0..=9).contains(&idx) {
        return false;
    }
    let r = idx / 3;
    let c = idx % 3;
    let lin = (c + 2 * r) + r;
    board[lin as usize] == 0xFF
}
