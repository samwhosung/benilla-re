//! `0x728b80`, `__thiscall(ecx = this, [esp+4] = idx)` returning `al`, against
//! [`minigame_move_legal`], called whole: it calls nothing. It reads the board at `[ecx+0x22+idx]`.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_minigame::minigame_move_legal;

const MOVE_LEGAL: u64 = 0x728b80;

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

#[test]
fn minigame_move_legal_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x80);
    let board_base = this + 0x22;

    let mut s = 0x728b_8001u32;
    let mut checked = 0u64;
    // Random 10-byte boards against every index from -3 to 12: both bounds, and index 9.
    for _ in 0..3000 {
        let mut board = [0u8; 10];
        for b in &mut board {
            *b = match lcg(&mut s) % 4 {
                0 | 1 => 0xFF,                   // empty
                2 => 0x00,                       // a piece
                _ => (lcg(&mut s) & 0xff) as u8, // anything
            };
        }
        for (i, b) in board.iter().enumerate() {
            emu.write(board_base + i as u64, &[*b]);
        }
        for idx in -3i32..=12 {
            let bin = emu.call(MOVE_LEGAL, this as u32, 0, &[idx as u32]) & 0xff;
            let ours = u32::from(minigame_move_legal(idx, &board));
            assert_eq!(
                bin, ours,
                "move_legal(idx={idx}, board={board:?}): binary={bin} ours={ours}"
            );
            checked += 1;
        }
    }
    eprintln!("minigame_move_legal: {checked} cases bit-exact vs WoW.exe @ 0x728b80");
}
