//! The module built into the exe, decoded and loaded by this crate, mapped into the emulator and
//! run: its entry, start, packet and unload slots against `benilla_re_warden::builtin`. The
//! client's seven services are stubs that log their arguments.

mod common;

use benilla_re_difftest::{Emu, StubCall, Wow};
use benilla_re_storm::md5;
use benilla_re_warden::builtin::{derive_keys, Rc4};
use benilla_re_warden::image::{self, Host};
use benilla_re_warden::{module, signature_name, NAME_TABLE_VA};
use common::Rng;
use sha1::{Digest, Sha1};

const BLOB_VA: u64 = 0x86ddb0;
const BLOB_LEN: usize = 0x12b3;
const KEY_VA: u64 = 0x86f064;
const MODULUS_VA: u64 = 0x811330;
const EXPONENT_VA: u64 = 0x811430;

struct Base(u32);

impl Host for Base {
    fn alloc(&mut self, _size: u32) -> Option<u32> {
        Some(self.0)
    }
    fn load_library(&mut self, _name: &[u8]) -> Option<u32> {
        None
    }
    fn proc_by_name(&mut self, _library: u32, _name: &[u8]) -> u32 {
        0
    }
    fn proc_by_ordinal(&mut self, _library: u32, _ordinal: u32) -> u32 {
        0
    }
}

struct Rig {
    emu: Emu<'static>,
    iface: u32,
    vtable: [u32; 4],
    slots: [u64; 7],
    /// Where service 1's code logs: the call count, then the last id and key pointers.
    load_log: u64,
}

/// Service 1 answers `first` to its first call and `then` to every later one; `take` is what
/// service 6 hands out, or none.
fn rig(wow: &Wow, first: u32, then: u32, take: Option<&[u8]>) -> Rig {
    let mut emu = Emu::new(wow);
    let blob = emu.read(BLOB_VA, BLOB_LEN);
    let key: [u8; 16] = emu.read(KEY_VA, 16).try_into().unwrap();
    let name = signature_name(&emu.read(u64::from(NAME_TABLE_VA), 0x20));
    let modulus = emu.read(MODULUS_VA, 0x100);
    let exponent = emu.read_u32(EXPONENT_VA);
    let p = module::decode(&blob, &key, &name, &modulus, exponent).expect("the built-in module");

    let base = emu.alloc(0x6100);
    let loaded = image::load(&p, &mut Base(base as u32)).expect("the image loads");
    emu.write(base, &loaded.image);
    let entry = image::export_address(&loaded.image, loaded.base, 1)
        .unwrap()
        .expect("export 1");

    let slots: [u64; 7] = std::array::from_fn(|_| emu.alloc(0x10));
    let arena = emu.alloc(0x10_0000);
    emu.stub(slots[0], 1, 2);
    // Service 1 is machine code, so that its answer can change between calls:
    // `mov eax,[next]; inc [count]; mov ecx,[esp+4]; mov [id],ecx; mov ecx,[esp+8]; mov [key],ecx;
    //  mov dword [next],then; ret 8`.
    let load_log = emu.alloc(0x20);
    let (count, id_at, key_at, next) = (load_log, load_log + 4, load_log + 8, load_log + 12);
    emu.write_u32(next, first);
    let mut code = vec![0xa1];
    code.extend_from_slice(&(next as u32).to_le_bytes());
    code.extend_from_slice(&[0xff, 0x05]);
    code.extend_from_slice(&(count as u32).to_le_bytes());
    code.extend_from_slice(&[0x8b, 0x4c, 0x24, 0x04, 0x89, 0x0d]);
    code.extend_from_slice(&(id_at as u32).to_le_bytes());
    code.extend_from_slice(&[0x8b, 0x4c, 0x24, 0x08, 0x89, 0x0d]);
    code.extend_from_slice(&(key_at as u32).to_le_bytes());
    code.extend_from_slice(&[0xc7, 0x05]);
    code.extend_from_slice(&(next as u32).to_le_bytes());
    code.extend_from_slice(&then.to_le_bytes());
    code.extend_from_slice(&[0xc2, 0x08, 0x00]);
    emu.write(slots[1], &code);
    emu.stub(slots[2], 1, 3);
    emu.stub_alloc_sized(slots[3], arena, arena + 0x10_0000, 0, 1);
    emu.stub(slots[4], 1, 1);
    emu.stub(slots[5], 1, 2);
    match take {
        None => emu.stub(slots[6], 0, 2),
        Some(bytes) => {
            let payload = emu.alloc(bytes.len() as u64);
            emu.write(payload, bytes);
            emu.stub_write_through(slots[6], 0, payload, bytes.len(), 1, 2);
        }
    }
    let vt = emu.alloc(0x20);
    for (i, s) in slots.iter().enumerate() {
        emu.write_u32(vt + 4 * i as u64, *s as u32);
    }
    let client = emu.alloc(8);
    emu.write_u32(client, vt as u32);

    emu.clear_calls();
    let iface = emu.call(u64::from(entry), client as u32, 0, &[]);
    assert_ne!(iface, 0, "the entry returns the module's object");
    let table = u64::from(emu.read_u32(u64::from(iface)));
    let vtable = std::array::from_fn(|i| emu.read_u32(table + 4 * i as u64));
    assert_eq!(table, base + 0x3000, "the object's table is in the image");
    assert_eq!(
        emu.calls()
            .iter()
            .map(|c| (c.addr, c.args[0]))
            .collect::<Vec<_>>(),
        vec![(slots[3], 0x214)],
        "the entry allocates the object"
    );
    Rig {
        emu,
        iface,
        vtable,
        slots,
        load_log,
    }
}

impl Rig {
    fn calls_to(&self, slot: usize) -> Vec<StubCall> {
        self.emu
            .calls()
            .iter()
            .copied()
            .filter(|c| c.addr == self.slots[slot])
            .collect()
    }

    /// How many times service 1 has run, and the id and key of its last call.
    fn loads(&self) -> (u32, Vec<u8>, Vec<u8>) {
        let at = self.load_log;
        (
            self.emu.read_u32(at),
            self.emu.read(u64::from(self.emu.read_u32(at + 4)), 16),
            self.emu.read(u64::from(self.emu.read_u32(at + 8)), 16),
        )
    }

    fn init(&mut self, session_key: &[u8]) {
        let k = self.emu.alloc(0x30);
        self.emu.write(k, session_key);
        self.emu.clear_calls();
        self.emu.call(
            u64::from(self.vtable[0]),
            self.iface,
            0,
            &[k as u32, session_key.len() as u32],
        );
    }

    /// Delivers `wire` bytes to slot 2; returns the module's result.
    fn packet(&mut self, wire: &[u8]) -> u32 {
        let p = self.emu.alloc(wire.len() as u64 + 16);
        self.emu.write(p, wire);
        let out = self.emu.alloc(8);
        self.emu.clear_calls();
        self.emu.call(
            u64::from(self.vtable[2]),
            self.iface,
            0,
            &[p as u32, wire.len() as u32, out as u32],
        )
    }

    /// The bytes sent to the server since the last clear, each call's.
    fn sent(&self) -> Vec<Vec<u8>> {
        self.calls_to(0)
            .iter()
            .map(|c| self.emu.read(u64::from(c.args[0]), c.args[1] as usize))
            .collect()
    }
}

fn random_bytes(rng: &mut Rng, n: usize) -> Vec<u8> {
    (0..n).map(|_| rng.next() as u8).collect()
}

/// The server's side of a stream: encrypts under the receive state, and decrypts a reply under
/// the send state.
struct Streams {
    send: Rc4,
    recv: Rc4,
}

impl Streams {
    fn new(session: &[u8]) -> Streams {
        let (ka, kb) = derive_keys(session);
        Streams {
            send: Rc4::new(&ka),
            recv: Rc4::new(&kb),
        }
    }
    fn encrypt_for_client(&mut self, mut plain: Vec<u8>) -> Vec<u8> {
        self.recv.crypt(&mut plain);
        plain
    }
    fn decrypt_from_client(&mut self, mut wire: Vec<u8>) -> Vec<u8> {
        self.send.crypt(&mut wire);
        wire
    }
}

#[test]
fn builtin_cipher_and_messages() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rng = Rng(0xb017_1100);
    let session = random_bytes(&mut rng, 40);
    let mut st = Streams::new(&session);

    // The module is asked for and the cache has it: service 1 answers 1 and the reply is the byte 1.
    let mut r = rig(&wow, 1, 1, None);
    r.init(&session);
    assert_eq!(
        r.calls_to(6).len(),
        1,
        "start asks service 6 for a handover"
    );
    assert_eq!(r.calls_to(6)[0].args[0], r.iface + 0xc);
    let id = [0x11u8; 16];
    let key = random_bytes(&mut rng, 16);
    let mut plain = vec![0u8];
    plain.extend_from_slice(&id);
    plain.extend_from_slice(&key);
    plain.extend_from_slice(&700u32.to_le_bytes());
    assert_eq!(r.packet(&st.encrypt_for_client(plain)) & 0xff, 1);
    let (count, got_id, got_key) = r.loads();
    assert_eq!((count, got_id, got_key), (1, id.to_vec(), key.clone()));
    let wire = r.sent().remove(0);
    assert_eq!(st.decrypt_from_client(wire), [1], "present: the byte 1");

    // The hash request: the byte 2, SHA-1 and MD5 of the string up to its first zero.
    for text in [&b"seed"[..], b"", b"a\0b", &[0x55; 200][..]] {
        let mut plain = vec![2u8, text.len() as u8];
        plain.extend_from_slice(text);
        assert_eq!(
            r.packet(&st.encrypt_for_client(plain)) & 0xff,
            1,
            "{text:?}"
        );
        let reply = st.decrypt_from_client(r.sent().remove(0));
        let upto = text.iter().position(|&b| b == 0).unwrap_or(text.len());
        let mut want = vec![2u8];
        want.extend_from_slice(&Sha1::digest(&text[..upto]));
        want.extend_from_slice(&md5(&text[..upto]));
        assert_eq!(reply, want, "{text:?}");
    }

    // An opcode of 3 or more fails the packet and sends nothing.
    assert_eq!(r.packet(&st.encrypt_for_client(vec![3u8, 0, 0])) & 0xff, 0);
    assert!(r.sent().is_empty());
    // The 3 bytes it left are counted as already deciphered: the next packet's first 3 bytes are
    // taken as they arrive, and the stream does not advance over them.
    assert_eq!(r.packet(&[2u8, 1, b'z']) & 0xff, 1);
    let reply = st.decrypt_from_client(r.sent().remove(0));
    assert_eq!(&reply[1..21], &Sha1::digest(b"z")[..]);

    // Unload: both RC4 states leave through service 5, and the object is freed.
    r.emu.clear_calls();
    r.emu.call(u64::from(r.vtable[1]), r.iface, 0, &[]);
    let stash = r.calls_to(5);
    assert_eq!(stash.len(), 1);
    assert_eq!((stash[0].args[0], stash[0].args[1]), (r.iface + 0xc, 0x208));
    let bytes = r.emu.read(u64::from(stash[0].args[0]), 0x208);
    let mut want = st.send.to_bytes();
    want.extend_from_slice(&st.recv.to_bytes());
    assert_eq!(&bytes[..0x204], &want[..], "send state, then receive state");
    assert_eq!(&bytes[0x204..], [0, 0, 0, 0], "no unconsumed bytes");
    assert_eq!(r.calls_to(4).last().map(|c| c.args[0]), Some(r.iface));

    // The next module takes the states over and the streams go on.
    let mut r2 = rig(&wow, 1, 1, Some(&bytes));
    r2.init(&session);
    let mut plain = vec![2u8, 3];
    plain.extend_from_slice(b"xyz");
    assert_eq!(r2.packet(&st.encrypt_for_client(plain)) & 0xff, 1);
    let reply = st.decrypt_from_client(r2.sent().remove(0));
    assert_eq!(reply[0], 2);
    assert_eq!(&reply[1..21], &Sha1::digest(b"xyz")[..]);

    // Without a handover the module starts its streams afresh, from the key.
    let mut r3 = rig(&wow, 1, 1, None);
    r3.init(&session);
    let mut fresh = Streams::new(&session);
    let mut plain = vec![2u8, 1, b'q'];
    assert_eq!(
        r3.packet(&fresh.encrypt_for_client(std::mem::take(&mut plain))) & 0xff,
        1
    );
    let reply = fresh.decrypt_from_client(r3.sent().remove(0));
    assert_eq!(&reply[1..21], &Sha1::digest(b"q")[..]);
}

#[test]
fn builtin_download_in_pieces() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rng = Rng(0xd011_70ad);
    let session = random_bytes(&mut rng, 40);

    // (first answer of service 1, later answers, the hash is right)
    for (first, then, good_hash) in [(0u32, 1u32, true), (0, 1, false), (0, 0, true)] {
        let mut st = Streams::new(&session);
        let mut r = rig(&wow, first, then, None);
        r.init(&session);
        let data = random_bytes(&mut rng, 900);
        let id: [u8; 16] = if good_hash { md5(&data) } else { [7u8; 16] };
        let key = random_bytes(&mut rng, 16);
        let mut plain = vec![0u8];
        plain.extend_from_slice(&id);
        plain.extend_from_slice(&key);
        plain.extend_from_slice(&(data.len() as u32).to_le_bytes());
        r.packet(&st.encrypt_for_client(plain));
        assert_eq!(
            st.decrypt_from_client(r.sent().remove(0)),
            [0],
            "absent: the byte 0, and the download is allocated"
        );
        let alloc = r.calls_to(3);
        assert_eq!(alloc.len(), 1);
        assert_eq!(alloc[0].args[0], data.len() as u32 + 0x80);

        // Pieces; the last completes the size.
        let mut at = 0;
        let mut last = None;
        while at < data.len() {
            let n = (1 + rng.below(300) as usize).min(data.len() - at);
            let mut plain = vec![1u8];
            plain.extend_from_slice(&(n as u16).to_le_bytes());
            plain.extend_from_slice(&data[at..at + n]);
            let ok = r.packet(&st.encrypt_for_client(plain)) & 0xff;
            at += n;
            if at < data.len() {
                assert!(r.sent().is_empty(), "no reply before the last piece");
                assert_eq!(ok, 1);
            } else {
                last = Some(r.sent());
            }
        }
        let last = last.unwrap();
        if !good_hash {
            // A hash that does not match restarts the download and asks again.
            assert_eq!(last.len(), 1);
            assert_eq!(st.decrypt_from_client(last[0].clone()), [0]);
            assert!(r.calls_to(2).is_empty(), "nothing is stored");
            assert_eq!(r.loads().0, 1, "service 1 ran once");
            continue;
        }
        let stores = r.calls_to(2);
        assert_eq!(stores.len(), 1, "stored once");
        assert_eq!(r.emu.read(u64::from(stores[0].args[0]), 16), id);
        assert_eq!(stores[0].args[2], data.len() as u32);
        assert_eq!(r.emu.read(u64::from(stores[0].args[1]), data.len()), data);
        let (count, got_id, got_key) = r.loads();
        assert_eq!((count, got_id, got_key), (2, id.to_vec(), key.clone()));
        if then == 1 {
            assert_eq!(last.len(), 1);
            assert_eq!(
                st.decrypt_from_client(last[0].clone()),
                [1],
                "loaded: the byte 1"
            );
        } else {
            assert!(last.is_empty(), "a load that fails sends nothing");
        }
    }
}
