//! The SHA-1 random pool that supplies the SRP6 ephemeral (`systems/net/login.md#random-pool`).
//!
//! The pool is 64 bytes: the seed digest `a` at `+0x00`, a copy `b` at `+0x14`, the 20-byte output
//! block at `+0x28` and at `+0x3c` the count of unused output bytes, which are the last `remaining`
//! bytes of the block.

use crate::sha1;

/// The pool's four fields as the exe lays them out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sha1Prng {
    a: [u8; 20],
    b: [u8; 20],
    pool: [u8; 20],
    remaining: usize,
}

impl Sha1Prng {
    /// `SeedRandomPool` (`0x5d1c70`, `__thiscall` with the pool in `ecx`, the seed on the stack):
    /// `a` is the SHA-1 of the twelve bytes `time ‖ seed ‖ rand`, each a little-endian `u32`, where
    /// `time` is written by `0x73f049` and `rand` is the C runtime's `rand()` after `srand(seed)`.
    /// `b` is a copy of `a`, the block is zeroed and `remaining` is 0. The three inputs are
    /// arguments here, since two of them come from the clock and the generator.
    pub fn seed(time: u32, seed: u32, rand: u32) -> Sha1Prng {
        let mut buf = [0u8; 12];
        buf[0..4].copy_from_slice(&time.to_le_bytes());
        buf[4..8].copy_from_slice(&seed.to_le_bytes());
        buf[8..12].copy_from_slice(&rand.to_le_bytes());
        let d = sha1(&buf);
        Sha1Prng {
            a: d,
            b: d,
            pool: [0u8; 20],
            remaining: 0,
        }
    }

    /// `DrawRandomPoolBytes` (`0x5d1cf0`, `__thiscall` with the pool in `ecx`, the output pointer
    /// and the count on the stack). A count of 0 does nothing. While fewer bytes remain than are
    /// wanted it copies the remaining tail of the block, refills the block ([`Sha1Prng::refill`])
    /// and sets `remaining` to 20, going on while more than 20 are wanted; then it copies the
    /// wanted bytes from the block's unused part and subtracts them from `remaining`.
    pub fn generate(&mut self, out: &mut [u8]) {
        let mut len = out.len();
        if len == 0 {
            return;
        }
        let mut pos = 0usize;
        if self.remaining < len {
            loop {
                let r = self.remaining;
                out[pos..pos + r].copy_from_slice(&self.pool[20 - r..20]);
                pos += r;
                len -= r;
                self.refill();
                self.remaining = 20;
                if len <= 20 {
                    break;
                }
            }
        }
        if len != 0 {
            let r = self.remaining;
            out[pos..pos + len].copy_from_slice(&self.pool[20 - r..20 - r + len]);
            self.remaining = r - len;
        }
    }

    /// The refill: the block becomes `SHA1(a ‖ block ‖ b)`, three updates of 20 bytes from `+0x00`,
    /// `+0x28` and `+0x14`.
    fn refill(&mut self) {
        let mut buf = [0u8; 60];
        buf[0..20].copy_from_slice(&self.a);
        buf[20..40].copy_from_slice(&self.pool);
        buf[40..60].copy_from_slice(&self.b);
        self.pool = sha1(&buf);
    }

    /// A pool with the given fields, to run [`Sha1Prng::generate`] from a chosen state.
    /// `remaining` is at most 20.
    pub fn from_state(a: [u8; 20], b: [u8; 20], pool: [u8; 20], remaining: usize) -> Sha1Prng {
        debug_assert!(remaining <= 20);
        Sha1Prng {
            a,
            b,
            pool,
            remaining,
        }
    }

    /// The fields `(a, b, pool, remaining)`.
    pub fn state(&self) -> ([u8; 20], [u8; 20], [u8; 20], usize) {
        (self.a, self.b, self.pool, self.remaining)
    }
}
