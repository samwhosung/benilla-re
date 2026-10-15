//! The module decoder `0x6ca360` run whole: the exe's own RC4, SHA-1, RSA, zlib and allocation
//! wrappers, with the allocator stubbed and the loader `0x7a79d0` stubbed to log what it gets (the
//! loader has its own test). Against `benilla_re_warden::module::decode`.

mod common;

use benilla_re_difftest::{Emu, StubCall, Wow};
use benilla_re_warden::module::{self, DecodeError, TAIL};
use benilla_re_warden::{signature_name, NAME_TABLE_VA};
use common::Rng;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

const DECODER: u64 = 0x6ca360;
const LOADER: u64 = 0x7a79d0;
const S_MEM_ALLOC: u64 = 0x6462e0;
const S_MEM_REALLOC: u64 = 0x646320;
/// The C runtime's `malloc`, which zlib's allocator `0x7348f0` calls, and zlib's `zcfree`.
const MALLOC: u64 = 0x40bc91;
const ZCFREE: u64 = 0x734910;
const S_MEM_FREE: u64 = 0x646430;
const VERIFY: u64 = 0x66cf10;
const S_STR_UPPER: u64 = 0x64b460;
/// The source path the client's own allocations carry.
const PATH: u32 = 0x86f074;
const BLOB_VA: u64 = 0x86ddb0;
const BLOB_LEN: usize = 0x12b3;
const KEY_VA: u64 = 0x86f064;
const MODULUS_VA: u64 = 0x811330;
const EXPONENT_VA: u64 = 0x811430;

struct Fixture {
    emu: Emu<'static>,
    arena: u64,
    blob: Vec<u8>,
    key: [u8; 16],
    name: Vec<u8>,
    modulus: Vec<u8>,
    exponent: u32,
}

fn fixture(wow: &Wow, stub_verify: bool, loader_result: u32) -> Fixture {
    let mut emu = Emu::new(wow);
    let blob = emu.read(BLOB_VA, BLOB_LEN);
    let key: [u8; 16] = emu.read(KEY_VA, 16).try_into().unwrap();
    let name = signature_name(&emu.read(u64::from(NAME_TABLE_VA), 0x20));
    let modulus = emu.read(MODULUS_VA, 0x100);
    let exponent = emu.read_u32(EXPONENT_VA);
    let arena = emu.alloc(0x40_0000);
    emu.stub_alloc_sized(S_MEM_ALLOC, arena, arena + 0x40_0000, 0, 4);
    emu.stub_alloc_sized(MALLOC, arena, arena + 0x40_0000, 0, 0);
    emu.stub_realloc_sized(S_MEM_REALLOC, 5);
    emu.stub(ZCFREE, 0, 0);
    emu.stub(S_MEM_FREE, 1, 4);
    emu.stub(LOADER, loader_result, 2);
    // SStrUpper is the C runtime's `_strupr` under a lock the emulator has no kernel for; the
    // stub writes what it leaves in the name buffer, the upper-cased name.
    let upper = emu.alloc(0x20);
    let mut up = name.to_ascii_uppercase();
    up.push(0);
    emu.write(upper, &up);
    emu.stub_write_through(S_STR_UPPER, 0, upper, up.len(), 0, 1);
    if stub_verify {
        emu.stub(VERIFY, 1, 1);
    }
    Fixture {
        emu,
        arena,
        blob,
        key,
        name,
        modulus,
        exponent,
    }
}

struct Outcome {
    eax: u32,
    /// The sizes of the client's own allocations.
    allocs: Vec<u32>,
    frees: usize,
    /// What the loader gets: the buffer's contents and the length argument.
    loaded: Option<(Vec<u8>, u32)>,
}

fn run(f: &mut Fixture, blob: &[u8], key: &[u8; 16]) -> Outcome {
    f.emu.clear_calls();
    f.emu.reset_bump(f.arena);
    let blob_at = f.emu.alloc(blob.len() as u64 + 16);
    f.emu.write(blob_at, blob);
    let key_at = f.emu.alloc(16);
    f.emu.write(key_at, key);
    let eax = f
        .emu
        .call(DECODER, blob_at as u32, blob.len() as u32, &[key_at as u32]);
    let calls: Vec<StubCall> = f.emu.calls().to_vec();
    let allocs = calls
        .iter()
        .filter(|c| c.addr == S_MEM_ALLOC && c.args[1] == PATH)
        .map(|c| c.args[0])
        .collect();
    let frees = calls
        .iter()
        .filter(|c| c.addr == S_MEM_FREE && c.args[1] == PATH)
        .count();
    let loaded = calls.iter().find(|c| c.addr == LOADER).map(|c| {
        (
            f.emu.read(u64::from(c.args[0]), c.args[1] as usize),
            c.args[1],
        )
    });
    Outcome {
        eax,
        allocs,
        frees,
        loaded,
    }
}

fn check(f: &mut Fixture, blob: &[u8], key: &[u8; 16], what: &str) -> Result<Vec<u8>, DecodeError> {
    let want = module::decode(blob, key, &f.name, &f.modulus, f.exponent);
    let got = run(f, blob, key);
    match &want {
        Ok(p) => {
            assert_ne!(got.eax, 0, "{what}: result");
            assert_eq!(
                got.loaded.as_ref().map(|l| &l.0),
                Some(p),
                "{what}: loader input"
            );
            assert_eq!(
                got.loaded.as_ref().map(|l| l.1),
                Some(p.len() as u32),
                "{what}: length"
            );
            let declared = u32::from_le_bytes({
                let mut d = blob.to_vec();
                module::rc4(&f.key, &mut d);
                d[..4].try_into().unwrap()
            });
            assert_eq!(
                got.allocs,
                vec![blob.len() as u32, declared, 12],
                "{what}: allocations"
            );
            assert_eq!(got.frees, 2, "{what}: frees");
        }
        Err(DecodeError::TooShort) => {
            assert_eq!(got.eax, 0, "{what}: result");
            assert!(got.allocs.is_empty(), "{what}: allocations");
            assert_eq!(got.frees, 0, "{what}");
        }
        Err(DecodeError::BadSignature) => {
            assert_eq!(got.eax, 0, "{what}: result");
            assert_eq!(got.allocs, vec![blob.len() as u32], "{what}: allocations");
            assert_eq!(got.frees, 1, "{what}: frees");
            assert!(got.loaded.is_none(), "{what}");
        }
        Err(DecodeError::BadStream) => {
            assert_eq!(got.eax, 0, "{what}: result");
            assert_eq!(got.allocs.len(), 2, "{what}: allocations");
            assert_eq!(got.frees, 2, "{what}: frees");
            assert!(got.loaded.is_none(), "{what}");
        }
    }
    want
}

#[test]
fn decoder_real_signature() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut f = fixture(&wow, false, 1);
    let (blob, key) = (f.blob.clone(), f.key);
    let p = check(&mut f, &blob, &key, "the exe's module").expect("the exe's module decodes");
    assert_eq!(p.len(), 6584);

    // Every length below the minimum, and the minimum.
    for n in [0usize, 1, 0x103, 0x104, 0x107] {
        check(&mut f, &blob[..n], &key, &format!("short {n:#x}")).unwrap_err();
    }
    // A flipped bit in the first byte, the middle, and each end of the signature tail.
    let mut rng = Rng(0x6ca3_6000);
    let n = blob.len();
    let mut spots = vec![0, 1, 4, n / 2, n - TAIL - 1, n - TAIL, n - TAIL + 3, n - 1];
    for _ in 0..4 {
        spots.push(rng.below(n as u32) as usize);
    }
    for at in spots {
        let mut b = blob.clone();
        b[at] ^= 1 << rng.below(8);
        let r = check(&mut f, &b, &key, &format!("flip at {at:#x}"));
        assert_eq!(r.unwrap_err(), DecodeError::BadSignature, "flip at {at:#x}");
    }
    // A shorter blob that still ends where the signature would, and a wrong key.
    let mut k = key;
    k[15] ^= 0x80;
    check(&mut f, &blob, &k, "wrong key").unwrap_err();
    check(&mut f, &blob[..blob.len() - 1], &key, "one byte short").unwrap_err();
}

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

/// A blob the signature check is bypassed for: the declared size, the stream, a random tail,
/// all RC4-encrypted with `key`.
fn forge(declared: u32, stream: &[u8], key: &[u8; 16], rng: &mut Rng) -> Vec<u8> {
    let mut b = declared.to_le_bytes().to_vec();
    b.extend_from_slice(stream);
    b.extend_from_slice(b"NGIS");
    b.extend((0..0x100).map(|_| rng.next() as u8));
    module::rc4(key, &mut b);
    b
}

#[test]
fn decoder_after_the_signature() {
    let Some(wow) = Wow::load() else {
        return;
    };
    // The signature routine is stubbed to accept, so the stages after it run on blobs the test
    // forges; the expected outcome comes from inflating the same stream.
    let mut f = fixture(&wow, true, 1);
    let mut rng = Rng(0xf04_6e00);
    let key: [u8; 16] = std::array::from_fn(|_| rng.next() as u8);
    for case in 0..40 {
        let n = 1 + rng.below(3000) as usize;
        let data: Vec<u8> = (0..n)
            .map(|i| if i % 7 < 4 { 0 } else { rng.next() as u8 })
            .collect();
        let stream = zlib(&data);
        for (declared, what) in [
            (n as u32, "exact"),
            (n as u32 + 1 + rng.below(100), "declared larger"),
            (n as u32 - 1, "declared smaller"),
        ] {
            let blob = forge(declared, &stream, &key, &mut rng);
            let got = run(&mut f, &blob, &key);
            // What the exe does after the signature: inflate into `declared` bytes, or fail.
            let mut inflated = Vec::new();
            let ok = {
                use flate2::read::ZlibDecoder;
                use std::io::Read;
                ZlibDecoder::new(&stream[..])
                    .take(u64::from(declared) + 1)
                    .read_to_end(&mut inflated)
                    .is_ok()
                    && inflated.len() <= declared as usize
            };
            if ok {
                assert_ne!(got.eax, 0, "case {case} {what}");
                let (bytes, len) = got.loaded.expect("the loader runs");
                assert_eq!(len as usize, inflated.len(), "case {case} {what}");
                assert_eq!(bytes, inflated, "case {case} {what}");
                assert_eq!(got.allocs, vec![blob.len() as u32, declared, 12]);
                assert_eq!(got.frees, 2, "case {case} {what}");
            } else {
                assert_eq!(got.eax, 0, "case {case} {what}");
                assert!(got.loaded.is_none());
                assert_eq!(got.allocs, vec![blob.len() as u32, declared]);
                assert_eq!(got.frees, 2, "case {case} {what}");
            }
        }
    }
    // A damaged stream, and a stream cut short.
    let data: Vec<u8> = (0..500).map(|_| rng.next() as u8).collect();
    let mut stream = zlib(&data);
    let blob = forge(500, &stream[..stream.len() / 2], &key, &mut rng);
    let got = run(&mut f, &blob, &key);
    assert_eq!(got.eax, 0, "cut stream");
    let at = stream.len() / 2;
    stream[at] ^= 0xff;
    let blob = forge(500, &stream, &key, &mut rng);
    let got = run(&mut f, &blob, &key);
    assert_eq!(got.eax, 0, "damaged stream");
}

#[test]
fn decoder_loader_failure() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut f = fixture(&wow, false, 0);
    let (blob, key) = (f.blob.clone(), f.key);
    let got = run(&mut f, &blob, &key);
    // The module decodes and the loader refuses it: null, and both buffers freed under the
    // client's path (the module object goes back through `delete`).
    assert_eq!(got.eax, 0);
    assert!(got.loaded.is_some());
    assert_eq!(got.allocs, vec![blob.len() as u32, 6584, 12]);
    assert_eq!(got.frees, 2);
}
