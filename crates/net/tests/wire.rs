//! The world connection's cipher and size header (`systems/net/wire.md`).

mod common;

use common::*;

/// Cipher vectors `(key, j, prev, payload)`: states at the wrap of `j` (39) and with `prev` 0 and 0xff,
/// and a fuzz of keys, states and payloads across the 40-byte period.
fn cipher_vectors() -> Vec<([u8; 40], u8, u8, Vec<u8>)> {
    let mut v: Vec<([u8; 40], u8, u8, Vec<u8>)> = Vec::new();
    let base = |fill: u8| {
        let mut k = [0u8; 40];
        for (i, b) in k.iter_mut().enumerate() {
            *b = fill ^ (i as u8);
        }
        k
    };
    v.push((base(0), 0, 0, b"hello world".to_vec()));
    v.push((base(0xff), 39, 0xff, vec![0u8; 50])); // j wraps past 39
    v.push((base(0x5a), 38, 0x80, (0..100u8).collect()));
    let mut s: u32 = 0x2468_ace0;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..400 {
        let mut k = [0u8; 40];
        for b in k.iter_mut() {
            *b = (next() >> 16) as u8;
        }
        let j0 = ((next() >> 16) % 40) as u8;
        let prev0 = (next() >> 16) as u8;
        let len = (next() >> 8) as usize % 96;
        let payload: Vec<u8> = (0..len).map(|_| (next() >> 16) as u8).collect();
        v.push((k, j0, prev0, payload));
    }
    v
}

/// Runs the cipher loop of `WowConnection::Send`, `0x5b5842` to `0x5b5896`, and compares the
/// ciphertext and the advanced state with `cipher_send`.
#[test]
fn cipher_send() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let vecs = cipher_vectors();
    for (key, j0, prev0, payload) in &vecs {
        let (ct_bin, j_bin, prev_bin) = d.cipher_send_slice(key, *j0, *prev0, payload);
        let mut ct_rust = payload.clone();
        let (j_rust, prev_rust) = benilla_re_net::cipher_send(key, *j0, *prev0, &mut ct_rust);
        assert_eq!(
            ct_bin,
            ct_rust,
            "cipher_send ciphertext mismatch (len {})",
            payload.len()
        );
        assert_eq!(
            (j_bin, prev_bin),
            (j_rust, prev_rust),
            "cipher_send state mismatch"
        );
    }
}

/// Enciphers with the `WowConnection::Send` loop (`0x5b5842` to `0x5b5896`), deciphers the result with
/// `cipher_recv` from the same state, and expects the plaintext and the state of the send loop.
#[test]
fn cipher_recv_roundtrip() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let vecs = cipher_vectors();
    for (key, j0, prev0, payload) in &vecs {
        let (ct_bin, j_bin, prev_bin) = d.cipher_send_slice(key, *j0, *prev0, payload);
        let mut pt = ct_bin.clone();
        let (j_rust, prev_rust) = benilla_re_net::cipher_recv(key, *j0, *prev0, &mut pt);
        assert_eq!(
            &pt,
            payload,
            "cipher_recv did not recover plaintext (len {})",
            payload.len()
        );
        assert_eq!(
            (j_rust, prev_rust),
            (j_bin, prev_bin),
            "cipher_recv state mismatch vs binary send"
        );
    }
}

/// Runs the cipher loop of `WowConnection::ReadFramedMessages`, `0x5b6848` to `0x5b68a8`, on arbitrary
/// ciphertext and compares the plaintext and the advanced state with `cipher_recv`.
#[test]
fn cipher_recv_direct() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let vecs = cipher_vectors();
    for (key, j0, prev0, cipher) in &vecs {
        // The payload bytes are taken as incoming ciphertext.
        let (pt_bin, j_bin, prev_bin) = d.cipher_recv_slice(key, *j0, *prev0, cipher);
        let mut pt_rust = cipher.clone();
        let (j_r, prev_r) = benilla_re_net::cipher_recv(key, *j0, *prev0, &mut pt_rust);
        assert_eq!(
            pt_bin,
            pt_rust,
            "cipher_recv_direct plaintext mismatch (len {})",
            cipher.len()
        );
        assert_eq!(
            (j_bin, prev_bin),
            (j_r, prev_r),
            "cipher_recv_direct state mismatch (len {})",
            cipher.len()
        );
    }
}

/// Runs the span computation and cipher loop of `WowConnection::SendRaw`, `0x5b5df1` to `0x5b5e79`,
/// over every `span_extra` and cipher vector, and over frame lengths either side of the 2- and
/// 3-byte header boundary (`0x8001` and `0x8002`).
#[test]
fn encipher_send_frame() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let base = cipher_vectors();

    // The frame length is the payload length. `span_extra` runs over the opcode widths and to 255,
    // where the span is clamped to the whole short buffer.
    for span_extra in [0u8, 2, 4, 6, 40, 255] {
        for (key, j0, prev0, payload) in &base {
            let flen = payload.len() as u32;
            let (ct_bin, j_bin, prev_bin) =
                d.encipher_send_frame_slice(key, *j0, *prev0, span_extra, payload, flen);
            let mut ct_rust = payload.clone();
            let (j_r, prev_r) =
                benilla_re_net::encipher_send_frame(key, *j0, *prev0, span_extra, &mut ct_rust);
            assert_eq!(
                ct_bin,
                ct_rust,
                "ct mismatch (span_extra {span_extra}, len {})",
                payload.len()
            );
            assert_eq!(
                (j_bin, prev_bin),
                (j_r, prev_r),
                "state mismatch (span_extra {span_extra}, len {})",
                payload.len()
            );
        }
    }

    // `hdr_len` is 3 when `len - 2 > 0x7fff`, so `0x8001` has a 2-byte header and `0x8002` a 3-byte
    // one. Only the first `span_extra + 3` bytes are touched, so a 32-byte prefix serves with a large
    // `frame_len`.
    let key = [0x4du8; 40];
    let plain: Vec<u8> = (0..32u8).collect();
    for &flen in &[0x20u32, 0x7fff, 0x8000, 0x8001, 0x8002, 0x9000, 0x1_0000] {
        for span_extra in [0u8, 4, 6] {
            let (ct_bin, j_bin, prev_bin) =
                d.encipher_send_frame_slice(&key, 5, 0x9c, span_extra, &plain, flen);
            let mut ct_rust = vec![0u8; flen as usize];
            ct_rust[..plain.len()].copy_from_slice(&plain);
            let (j_r, prev_r) =
                benilla_re_net::encipher_send_frame(&key, 5, 0x9c, span_extra, &mut ct_rust);
            assert_eq!(
                ct_bin[..],
                ct_rust[..plain.len()],
                "hdr-boundary ct mismatch (flen {flen:#x}, span_extra {span_extra})"
            );
            assert_eq!(
                (j_bin, prev_bin),
                (j_r, prev_r),
                "hdr-boundary state mismatch (flen {flen:#x}, span_extra {span_extra})"
            );
        }
    }
}

/// Runs the form selection and byte writes of `WowConnection::AllocSendBuffer`, `0x5b5495` to
/// `0x5b54f0`, across the 2- and 3-byte boundary and the whole length domain, and compares the
/// header bytes with `header_encode`.
#[test]
fn header_encode() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mut cases: Vec<u32> = vec![
        0,
        1,
        2,
        0x7f,
        0x80,
        0x100,
        0x7ffe,
        0x7fff,
        0x8000,
        0x8001,
        0xffff,
        0x10000,
        0x10001,
        0x123456,
        0x7ffffe,
        0x7fffff,
        0x8000_0000,
        0xabcd_ef01,
        0xffff_ffff,
    ];
    for k in 0..0x80u32 {
        cases.push((k << 16) | 0x1234); // the first byte of the 3-byte form, 0 to 0x7f
    }
    let mut lcg = Lcg(0x9e37_79b9);
    for _ in 0..600 {
        cases.push(lcg.next());
    }
    for &n in &cases {
        let emitted = d.header_encode_slice(n);
        assert_eq!(
            emitted,
            benilla_re_net::header_encode(n),
            "header mismatch on n={n:#x}"
        );
    }
}

/// Runs the header decode of `WowConnection::ReadFramedMessages`, `0x5b6730` to `0x5b6789`, on
/// headers built by `header_encode`, on random bytes, and on incomplete headers, and compares
/// `(frame_len, hdr_len)` with `header_decode`. Each decoded length also round-trips `header_encode`.
#[test]
fn header_decode() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mut checked = 0usize;

    // Headers built by `header_encode` for every `N` up to 0x7fffff that the form can hold: the
    // decode must match the exe and `payload_len` must return `N`.
    let mut lengths: Vec<u32> = vec![
        0, 1, 2, 0x7f, 0x80, 0xff, 0x100, 0x1234, 0x7ffe, 0x7fff, // 2-byte form
        0x8000, 0x8001, 0xffff, 0x1_0000, 0x12_3456, 0x7f_fffe, 0x7f_ffff, // 3-byte form
    ];
    for k in 0..0x80u32 {
        lengths.push((k << 16) | 0x0abc); // the first byte of the 3-byte form, 0 to 0x7f
    }
    for &n in &lengths {
        let hdr = benilla_re_net::header_encode(n);
        let (bin_fl, bin_hl) = d.header_decode_slice(&hdr, hdr.len() as u32);
        let ours = benilla_re_net::header_decode(&hdr);
        assert_eq!((bin_fl, bin_hl), (ours.frame_len, ours.hdr_len), "n={n:#x}");
        assert_eq!(ours.payload_len(), Some(n), "n={n:#x}: payload round-trip");
        assert_eq!(
            ours.frame_len,
            n + ours.hdr_len,
            "n={n:#x}: frame_len = n + hdr_len"
        );
        // Body bytes after the header leave the decode unchanged.
        let mut withbody = hdr.clone();
        withbody.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        let (fl2, hl2) = d.header_decode_slice(&withbody, withbody.len() as u32);
        assert_eq!(
            (fl2, hl2),
            (ours.frame_len, ours.hdr_len),
            "n={n:#x}: trailing-body"
        );
        checked += 1;
    }

    // Random bytes: the decode reads whatever is present.
    let mut lcg = Lcg(0x1357_9bdf);
    for _ in 0..600 {
        let r = lcg.next();
        let bytes = [r as u8, (r >> 8) as u8, (r >> 16) as u8];
        let (bin_fl, bin_hl) = d.header_decode_slice(&bytes, 3);
        let ours = benilla_re_net::header_decode(&bytes);
        assert_eq!(
            (bin_fl, bin_hl),
            (ours.frame_len, ours.hdr_len),
            "fuzz bytes={bytes:02x?}"
        );
        checked += 1;
    }

    // Fewer bytes than the header: the exe stores `frame_len` as 0xffffffff.
    let incompletes: &[(&[u8], u32)] = &[
        (&[], 0),           // no bytes
        (&[0x12], 1),       // one byte; the 2-byte form needs two
        (&[0x80, 0x34], 2), // the 3-byte form, with only two bytes
    ];
    for &(bytes, avail) in incompletes {
        let (bin_fl, bin_hl) = d.header_decode_slice(bytes, avail);
        let ours = benilla_re_net::header_decode(&bytes[..avail as usize]);
        assert_eq!(
            (bin_fl, bin_hl),
            (ours.frame_len, ours.hdr_len),
            "incomplete avail={avail}"
        );
        assert_eq!(bin_fl, 0xffff_ffff, "incomplete avail={avail}: sentinel");
        assert_eq!(ours.payload_len(), None);
        checked += 1;
    }

    assert!(checked > 20);
}
