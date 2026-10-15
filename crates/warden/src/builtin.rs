//! The cipher of the module built into the exe (`systems/warden/embedded.md`): two RC4 states
//! keyed from the session key by a SHA-1 stream.

use sha1::{Digest, Sha1};

/// RC4 as the built-in module keeps it: 256 bytes of state, then the two indices, 0x102 bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rc4 {
    pub s: [u8; 256],
    pub i: u8,
    pub j: u8,
}

impl Rc4 {
    pub fn new(key: &[u8]) -> Rc4 {
        let mut s = [0u8; 256];
        for (i, b) in s.iter_mut().enumerate() {
            *b = i as u8;
        }
        let mut j = 0u8;
        for i in 0..256 {
            j = j.wrapping_add(s[i]).wrapping_add(key[i % key.len()]);
            s.swap(i, j as usize);
        }
        Rc4 { s, i: 0, j: 0 }
    }

    pub fn crypt(&mut self, data: &mut [u8]) {
        for b in data.iter_mut() {
            self.i = self.i.wrapping_add(1);
            self.j = self.j.wrapping_add(self.s[self.i as usize]);
            self.s.swap(self.i as usize, self.j as usize);
            let k = self.s[self.s[self.i as usize].wrapping_add(self.s[self.j as usize]) as usize];
            *b ^= k;
        }
    }

    /// The 0x102 bytes in the module's layout.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = self.s.to_vec();
        v.push(self.i);
        v.push(self.j);
        v
    }
}

/// The two RC4 keys the module derives from a session key: `(send, receive)`, 16 bytes each.
///
/// Each half of the key is hashed with SHA-1 into `o1` and `o2`. A 20-byte block starts as zeros;
/// each refill is `SHA-1(o1 ‖ block ‖ o2)`, the first made at start. The stream is the blocks'
/// bytes in order: the first 16 key the send state, the next 16 the receive state.
pub fn derive_keys(session_key: &[u8]) -> ([u8; 16], [u8; 16]) {
    let half = session_key.len() / 2;
    let o1 = Sha1::digest(&session_key[..half]);
    let o2 = Sha1::digest(&session_key[half..]);
    let mut block = [0u8; 20];
    let refill = |block: &mut [u8; 20]| {
        let mut h = Sha1::new();
        h.update(o1);
        h.update(&block[..]);
        h.update(o2);
        block.copy_from_slice(&h.finalize());
    };
    refill(&mut block);
    let mut stream = Vec::new();
    let mut pos = 0;
    while stream.len() < 32 {
        if pos == 20 {
            refill(&mut block);
            pos = 0;
        }
        stream.push(block[pos]);
        pos += 1;
    }
    (
        stream[..16].try_into().unwrap(),
        stream[16..].try_into().unwrap(),
    )
}
