//! The logon: the PIN grid, the session key, the random pool, the proofs and the SRP6 session
//! (`systems/net/login.md`).

mod common;

use common::*;

/// NUL-free buffers for the logon hashes, whose inputs the exe measures with `strlen`: edge strings,
/// every length up to 70 and a fuzz.
fn login_inputs() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![b"".to_vec(), b"a".to_vec(), b"Jefe".to_vec()];
    for len in 0..=70usize {
        v.push((0..len).map(|i| (i * 7 + 3) as u8 | 1).collect());
    }
    let mut lcg = Lcg(0x1234_5678);
    for _ in 0..500 {
        let len = (lcg.next() >> 8) as usize % 256;
        v.push((0..len).map(|_| (lcg.next() >> 16) as u8 | 1).collect());
    }
    v
}

/// The modulus N of the vanilla logon, 32 bytes little-endian as the server sends it, the top byte
/// `0xb7` last, so its bit length is 256.
const SRP_N: [u8; 32] = [
    0x89, 0x4b, 0x64, 0x5e, 0x89, 0xe1, 0x53, 0x5b, 0xbd, 0xad, 0x5b, 0x8b, 0x29, 0x06, 0x50, 0x53,
    0x08, 0x01, 0xb1, 0x8e, 0xbf, 0xbf, 0x5e, 0x8f, 0xab, 0x3c, 0x82, 0x87, 0x2a, 0x3e, 0x9b, 0xb7,
];

/// Runs `DecodePinGridOrder` (`0x5b1e70`) whole over edge and fuzzed seeds and compares the 10
/// bytes it writes at `+0x1ae`.
#[test]
fn pin_shuffle() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mut seeds: Vec<u32> = vec![0, 1, 9, 10, u32::MAX, 3_628_799, 3_628_800, 0x1234_5678];
    let mut s: u32 = 0xc0ff_ee01;
    for _ in 0..500 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        seeds.push(s);
    }
    for &seed in &seeds {
        // The 10-byte output is at `this + 0x1ae`, which `io` (0x800 bytes) holds.
        d.emu.write(d.io + 0x1ae, &[0u8; 10]);
        d.emu.call(0x5b1e70, d.io as u32, 0, &[seed]);
        let got = d.read_bytes(d.io + 0x1ae, 10);
        assert_eq!(
            got,
            benilla_re_net::pin_grid_shuffle(seed).to_vec(),
            "pin_shuffle mismatch on seed {seed:#x}"
        );
    }
}

/// Runs `Srp6InterleaveSessionKey` (`0x5d3360`) whole over each secret, with the SHA-1 of
/// `SHA1_Init` (`0x5d1db0`) running inside it, and compares the 40-byte key.
#[test]
fn srp6_interleave() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let secrets = synth_secrets();
    for s in &secrets {
        d.emu.write(d.io, s);
        // The output is cleared so that a short write cannot match stale bytes.
        d.emu.write(d.out, &[0u8; 40]);
        d.emu
            .call(0x5d3360, d.out as u32, d.io as u32, &[s.len() as u32]);
        let got = d.read_bytes(d.out, 40);
        assert_eq!(
            got,
            benilla_re_net::srp6_interleave(s).to_vec(),
            "srp6_interleave mismatch on {}-byte secret",
            s.len()
        );
    }
}

/// Runs `DrawRandomPoolBytes` (`0x5d1cf0`) whole from chosen pool states and counts, with the
/// `SHA1_Init` family running inside it, and compares the bytes drawn and all four pool fields.
#[test]
fn sha1_prng_next() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let state = d.emu.alloc(0x40);
    let obuf = d.emu.alloc(0x400);
    let mut cases: Vec<(usize, usize)> = Vec::new();
    for &rem in &[0usize, 1, 5, 19, 20] {
        for len in 0..=64usize {
            cases.push((rem, len));
        }
        for &len in &[100usize, 128, 200, 256, 333] {
            cases.push((rem, len));
        }
    }
    let mut lcg = Lcg(0x2468_ace0);
    for _ in 0..300 {
        cases.push(((lcg.next() % 21) as usize, (lcg.next() % 400) as usize));
    }
    for (rem, len) in cases.clone() {
        let (a, b, pool) = (lcg.arr20(), lcg.arr20(), lcg.arr20()); // a and b differ, so a swap of their reads shows
        d.emu.write(state, &a);
        d.emu.write(state + 0x14, &b);
        d.emu.write(state + 0x28, &pool);
        d.emu.write_u32(state + 0x3c, rem as u32);
        d.emu
            .call(0x5d1cf0, state as u32, 0, &[obuf as u32, len as u32]);
        let mut p = benilla_re_net::Sha1Prng::from_state(a, b, pool, rem);
        let mut want = vec![0u8; len];
        p.generate(&mut want);
        let (ra, rb, rpool, rrem) = p.state();
        assert_eq!(
            d.read_bytes(obuf, len),
            want,
            "prng output (rem={rem}, len={len})"
        );
        assert_eq!(d.read_bytes(state, 20), ra.to_vec(), "prng state.a");
        assert_eq!(d.read_bytes(state + 0x14, 20), rb.to_vec(), "prng state.b");
        assert_eq!(
            d.read_bytes(state + 0x28, 20),
            rpool.to_vec(),
            "prng state.pool (rem={rem}, len={len})"
        );
        assert_eq!(
            d.emu.read_u32(state + 0x3c) as usize,
            rrem,
            "prng state.remaining"
        );
    }
}

/// Runs `SeedRandomPool` (`0x5d1c70`) whole with `_time` (`0x73f049`) and `_srand` (`0x7400d8`)
/// stubbed to do nothing, so the time reads as the mapped zero, and `_rand` (`0x7400e5`) stubbed to
/// return a fixed value. It compares the pool with `Sha1Prng::seed(0, seed, RAND)`.
#[test]
fn sha1_prng_seed() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let state = d.emu.alloc(0x40);
    const RAND: u32 = 0x1357_9bdf;
    d.emu.stub(0x73f049, 0, 0); // _time writes nothing: the time slot stays the mapped zero
    d.emu.stub(0x7400d8, 0, 0); // _srand
    d.emu.stub(0x7400e5, RAND, 0); // _rand returns a fixed value
    for &seed in &[0u32, 1, 0xdead_beef, 0xabcd_1234, 0xffff_ffff] {
        d.emu.write(state, &[0xa5u8; 0x40]); // poison, so that the zeroing of the block and the count shows
        d.emu.call(0x5d1c70, state as u32, 0, &[seed]);
        let (wa, wb, wpool, wrem) = benilla_re_net::Sha1Prng::seed(0, seed, RAND).state();
        assert_eq!(
            d.read_bytes(state, 20),
            wa.to_vec(),
            "prng_seed a (time must be mapped-zero)"
        );
        assert_eq!(d.read_bytes(state + 0x14, 20), wb.to_vec(), "prng_seed b");
        assert_eq!(
            d.read_bytes(state + 0x28, 20),
            wpool.to_vec(),
            "prng_seed pool"
        );
        assert_eq!(
            d.emu.read_u32(state + 0x3c) as usize,
            wrem,
            "prng_seed remaining"
        );
    }
}

/// Runs `Srp6SetCredentials` (`0x5d3590`) whole on NUL-free strings, since the exe reads their length
/// with `scasb`, and compares the digests at `+0x5c` and `+0x70`.
#[test]
fn srp6_credential_hash() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let this = d.emu.alloc(0x100);
    let user_va = d.emu.alloc(0x80);
    let pass_va = d.emu.alloc(0x80);
    // Bytes 1 to 255 only: the exe measures the strings with `scasb`, so a NUL would end them.
    let mut vecs: Vec<(Vec<u8>, Vec<u8>)> = vec![
        (b"".to_vec(), b"".to_vec()),
        (b"ADMIN".to_vec(), b"ADMIN".to_vec()),
        (b"TEST".to_vec(), b"PASSWORD".to_vec()),
        (vec![b'U'; 64], vec![b'P'; 200]),
    ];
    let mut lcg = Lcg(0x1234_5678);
    for _ in 0..400 {
        let ul = (lcg.next() >> 8) as usize % 48;
        let pl = (lcg.next() >> 8) as usize % 64;
        vecs.push((
            (0..ul)
                .map(|_| (lcg.next() >> 16) as u8 % 0xff + 1)
                .collect(),
            (0..pl)
                .map(|_| (lcg.next() >> 16) as u8 % 0xff + 1)
                .collect(),
        ));
    }
    for (user, pass) in &vecs {
        let (mut u, mut p) = (user.clone(), pass.clone());
        u.push(0);
        p.push(0);
        d.emu.write(user_va, &u);
        d.emu.write(pass_va, &p);
        d.emu.write(this + 0x5c, &[0u8; 20]);
        d.emu.write(this + 0x70, &[0u8; 20]);
        d.emu
            .call(0x5d3590, this as u32, 0, &[user_va as u32, pass_va as u32]);
        let want = benilla_re_net::srp6_credential_hash(user, pass);
        assert_eq!(
            d.read_bytes(this + 0x5c, 20),
            want.username.to_vec(),
            "SHA1(USER)"
        );
        assert_eq!(
            d.read_bytes(this + 0x70, 20),
            want.credential.to_vec(),
            "SHA1(USER:PASS)"
        );
    }
}

/// Runs `HashCdKeyProof` (`0x5bba80`) whole over pairs of buffers and a random 20-byte tail.
#[test]
fn srp6_file_hmac() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let record = d.emu.alloc(0x20);
    let chunk_va = d.emu.alloc(0x800);
    let key_va = d.emu.alloc(0x800);
    let out = d.emu.alloc(0x20);
    let bufs = synth_buffers();
    let n = bufs.len();
    let mut lcg = Lcg(0x0bad_f00d);
    for i in 0..n {
        let (chunk, key) = (&bufs[i], &bufs[n - 1 - i]);
        if chunk.len() >= 0x800 || key.len() >= 0x800 {
            continue;
        }
        let tail = lcg.arr20();
        d.emu.write(record + 0x0c, &tail);
        d.emu.write(chunk_va, chunk);
        d.emu.write(key_va, key);
        d.emu.write(out, &[0u8; 20]);
        d.emu.call(
            0x5bba80,
            0,
            0,
            &[
                record as u32,
                chunk_va as u32,
                chunk.len() as u32,
                key_va as u32,
                key.len() as u32,
                out as u32,
            ],
        );
        assert_eq!(
            d.read_bytes(out, 20),
            benilla_re_net::srp6_file_hmac(chunk, key, &tail).to_vec(),
            "srp6_file_hmac (chunk {} / key {})",
            chunk.len(),
            key.len()
        );
    }
}

/// Runs `ChecksumExecutables` (`0x5b1170`) whole with `OsFileExists` (`0x42a4e0`) stubbed to return
/// 0, so that every file is skipped and the message is empty, over keys of up to 64 bytes. This
/// checks the HMAC framing only; `checksum_executables_known_answer` checks which files are read.
#[test]
fn hmac_sha1() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let key_va = d.emu.alloc(0x80);
    let out = d.emu.alloc(0x40);
    d.emu.stub(0x42a4e0, 0, 0); // OsFileExists returns 0: no file is read
    for input in login_inputs() {
        if input.len() > 64 {
            continue;
        }
        d.emu.write(key_va, &input);
        d.emu.write(out, &[0u8; 20]);
        d.emu
            .call(0x5b1170, key_va as u32, input.len() as u32, &[out as u32]);
        assert_eq!(
            d.read_bytes(out, 20),
            benilla_re_net::hmac_sha1(&input, &[]).to_vec(),
            "hmac_sha1 ({}-byte key)",
            input.len()
        );
    }
}

/// Runs `HashPinWithSalts` (`0x5b31e0`) whole over random 16-byte salt pairs and NUL-free data.
#[test]
fn checksum_proof() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let (k1_va, k2_va) = (d.emu.alloc(0x20), d.emu.alloc(0x20));
    let data_va = d.emu.alloc(0x400);
    let out = d.emu.alloc(0x40);
    let mut lcg = Lcg(0x0bad_c0de);
    for data in login_inputs() {
        if data.len() >= 0x400 {
            continue;
        }
        let k1: [u8; 16] = std::array::from_fn(|_| (lcg.next() >> 16) as u8);
        let k2: [u8; 16] = std::array::from_fn(|_| (lcg.next() >> 16) as u8);
        d.emu.write(k1_va, &k1);
        d.emu.write(k2_va, &k2);
        d.emu.write(data_va, &data);
        d.emu.write(data_va + data.len() as u64, &[0u8]); // the NUL the exe's length scan stops at
        d.emu.write(out, &[0u8; 20]);
        d.emu.call(
            0x5b31e0,
            k1_va as u32,
            k2_va as u32,
            &[data_va as u32, out as u32],
        );
        assert_eq!(
            d.read_bytes(out, 20),
            benilla_re_net::checksum_proof(&k1, &k2, &data).to_vec(),
            "checksum_proof ({}-byte data)",
            data.len()
        );
    }
}

/// Runs `Srp6ClientComputeProof` (`0x5d3650`) whole with its callees, Storm's BigNumber and the
/// SHA-1 code, running for real, on the logon modulus N and g = 7 and fuzzed challenges. The pool at
/// `0xc2a428` is set to a chosen state and its draw reproduced with `Sha1Prng`. Storm's allocator
/// cannot run without the start-up that builds its free tree, so `SMemAlloc` (`0x6462e0`),
/// `SMemReAlloc` (`0x646320`) and `SMemFree` (`0x646430`) are stubbed to a bump arena that keeps the
/// old block across a reallocation. It compares `A`, `K` and `M1`.
#[test]
fn srp6_client_session() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let base = d.emu.alloc(0x120); // the SRP object: A at 0, K at 0x20, M1 at 0x48, H(USER) at 0x5c, p at 0x70, the proof hash at 0x84
    let n_va = d.emu.alloc(0x40);
    let g_va = d.emu.alloc(0x10);
    let salt_va = d.emu.alloc(0x40);
    let b_va = d.emu.alloc(0x40);
    let arena = d.emu.alloc(0xc0_0000); // the bump arena, in 0x1000-byte blocks
    const PRNG_STATE: u64 = 0xc2a428; // the random pool, the last argument

    // Storm's allocator needs the free tree that start-up builds, so its three entries, all
    // `__stdcall`, are stubbed to a bump arena:
    //   SMemAlloc   0x6462e0(size, name, line, flag): the next block (4 arguments)
    //   SMemReAlloc 0x646320(ptr, size, name, line, flag): a new block holding the old contents
    //   SMemFree    0x646430(ptr, name, line, flag): nothing (4 arguments)
    // The reallocation copies, since not every BigNumber growth falls back when it fails.
    d.emu.stub_alloc(0x6462e0, arena, 0x1000, 4);
    d.emu.stub_realloc(0x646320, 5);
    d.emu.stub(0x646430, 1, 4);

    d.emu.write(n_va, &SRP_N);
    d.emu.write(g_va, &[7u8]);

    let mut lcg = Lcg(0x5117_b00c);
    for _ in 0..8 {
        let salt: [u8; 32] = std::array::from_fn(|_| (lcg.next() >> 16) as u8);
        let p: [u8; 20] = lcg.arr20();
        let h_i: [u8; 20] = lcg.arr20();
        // 0 < B < N, little-endian: byte 31 is the top, and N's is 0xb7, so a top byte of 1 to 0xb6
        // gives B < N and B != 0.
        let mut b = [0u8; 32];
        for x in b.iter_mut().take(31) {
            *x = (lcg.next() >> 16) as u8;
        }
        b[31] = ((lcg.next() >> 16) as u8 % 0xb6) + 1;
        // A pool state with nothing remaining, so the draw is a refill, reproduced with `Sha1Prng`.
        let pa = lcg.arr20();
        let pb = lcg.arr20();
        let ppool = lcg.arr20();
        let mut a_rand = [0u8; 32];
        benilla_re_net::Sha1Prng::from_state(pa, pb, ppool, 0).generate(&mut a_rand);

        // The SRP object: zeroed, with the digests `Srp6SetCredentials` stores, and the proof hash
        // at +0x84 started, as the exe does before the call.
        d.emu.write(base, &[0u8; 0x120]);
        d.emu.write(base + 0x5c, &h_i);
        d.emu.write(base + 0x70, &p);
        d.emu.call(0x5d1db0, (base + 0x84) as u32, 0, &[]); // SHA1_Init
        d.emu.write(PRNG_STATE, &pa);
        d.emu.write(PRNG_STATE + 0x14, &pb);
        d.emu.write(PRNG_STATE + 0x28, &ppool);
        d.emu.write_u32(PRNG_STATE + 0x3c, 0);
        d.emu.write(salt_va, &salt);
        d.emu.write(b_va, &b);

        let rc = d.emu.call(
            0x5d3650,
            base as u32,
            0,
            &[
                n_va as u32,
                32,
                g_va as u32,
                1,
                salt_va as u32,
                0x20,
                b_va as u32,
                0x20,
                PRNG_STATE as u32,
            ],
        );
        assert_eq!(rc, 0, "srp6_client_session returned {rc:#x} (expected 0)");

        let want = benilla_re_net::srp6_client_session(&SRP_N, &[7], &salt, &b, &p, &h_i, &a_rand);
        assert_eq!(d.read_bytes(base, 32), want.a_pub.to_vec(), "A mismatch");
        assert_eq!(d.read_bytes(base + 0x20, 40), want.k.to_vec(), "K mismatch");
        assert_eq!(
            d.read_bytes(base + 0x48, 20),
            want.m1.to_vec(),
            "M1 mismatch"
        );
        d.emu.reset_bump(arena); // the arena is reused by the next case
    }
}

/// Reads the five files of a 1.12.1.5875 install from the folder `BENILLA_RE_WOW_DIR` and compares
/// `checksum_executables` with the hash vmangos stores for that build,
/// `95EDB27C7823B363CBDDAB56A392E7CB73FCCA20` (`sql/migrations/20221117065844_logon.sql`), for
/// the salt its realm daemon sends (`src/realmd/AuthSocket.cpp`, line 65). The value is independent
/// of this repo, and the five files differ, so it fixes the order of concatenation. It runs no exe
/// code and skips without the folder. A folder with no files is checked first and always: the result
/// is the HMAC of the empty message, not zeros.
#[test]
fn checksum_executables_known_answer() {
    const SALT: [u8; 16] = [
        0xBA, 0xA3, 0x1E, 0x99, 0xA0, 0x0B, 0x21, 0x57, 0xFC, 0x37, 0x3F, 0xB3, 0x69, 0xCD, 0xD2,
        0xF1,
    ];
    const EXPECTED: [u8; 20] = [
        0x95, 0xED, 0xB2, 0x7C, 0x78, 0x23, 0xB3, 0x63, 0xCB, 0xDD, 0xAB, 0x56, 0xA3, 0x92, 0xE7,
        0xCB, 0x73, 0xFC, 0xCA, 0x20,
    ];

    // With no file present the result is the HMAC of the empty message, a real digest and not
    // twenty zero bytes.
    let empty = std::path::PathBuf::from("no-such-install-dir-for-this-test");
    let none = benilla_re_net::checksum_executables(&SALT, &empty);
    assert_eq!(none, benilla_re_net::hmac_sha1(&SALT, &[]));
    assert_ne!(none, [0u8; 20], "an unreadable scan must not yield zeros");
    assert_ne!(none, EXPECTED);

    let Some(dir) = std::env::var_os("BENILLA_RE_WOW_DIR")
        .map(std::path::PathBuf::from)
        .filter(|p| p.join("WoW.exe").is_file())
    else {
        eprintln!("skip checksum_executables_known_answer: BENILLA_RE_WOW_DIR is not set");
        return;
    };
    let h = benilla_re_net::checksum_executables(&SALT, &dir);
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02X}")).collect::<String>();
    assert_eq!(
        h,
        EXPECTED,
        "ChecksumExecutables over {}: got {} want {}",
        dir.display(),
        hex(&h),
        hex(&EXPECTED)
    );
}
