//! The module image loader `0x7a79d0` run whole, with the loader's unloader `0x7a7d80` and entry
//! lookup `0x7a7de0`, against `benilla_re_warden::image`. The process calls (`VirtualAlloc`,
//! `LoadLibraryA`, `GetProcAddress`, `VirtualProtect`, `FlushInstructionCache`, `VirtualFree`,
//! `GetModuleHandleA`, `FreeLibrary`) are stubs that log their arguments; the loader's own code, the
//! C runtime's `sprintf` in the error message included, runs.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_warden::image::{self, LoadError, Loaded};
use common::{build, CountingHost, Event, Rng, FIRST, STRIDE};

const LOADER: u64 = 0x7a79d0;
const UNLOAD: u64 = 0x7a7d80;
const ENTRY: u64 = 0x7a7de0;
const FATAL: u64 = 0x64d0e0;

const IAT_VIRTUAL_ALLOC: u64 = 0x7ff2bc;
const IAT_LOAD_LIBRARY: u64 = 0x7ff2b4;
const IAT_GET_PROC: u64 = 0x88249c;
const IAT_PROTECT: u64 = 0x7ff35c;
const IAT_CURRENT_PROCESS: u64 = 0x7ff390;
const IAT_FLUSH: u64 = 0x7ff320;
const IAT_VIRTUAL_FREE: u64 = 0x7ff2f8;
const IAT_MODULE_HANDLE: u64 = 0x7ff340;
const IAT_FREE_LIBRARY: u64 = 0x7ff27c;

const HANDLE: u32 = 0x6000_0001;
const PROCESS: u32 = 0xffff_ffff;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Ev {
    Alloc(u32),
    LoadLibrary(Vec<u8>),
    ProcName(u32, Vec<u8>),
    ProcOrdinal(u32, u32),
    Protect(u32, u32, u32),
    CurrentProcess,
    Flush(u32, u32, u32),
    Free(u32, u32, u32),
    ModuleHandle(Vec<u8>),
    FreeLibrary(u32),
    Fatal(Vec<u8>),
}

struct Rig {
    emu: Emu<'static>,
    region: u64,
    ids: Vec<(u64, &'static str)>,
}

const REGION: u64 = 0x1_0000;

fn rig(wow: &Wow, library_fails: bool, alloc_fails: bool) -> Rig {
    let mut emu = Emu::new(wow);
    emu.map(0, 0x1000);
    let region = emu.alloc(REGION);
    let mut ids = Vec::new();
    let mut slot = |emu: &mut Emu<'static>, iat: u64, name: &'static str| {
        let at = emu.alloc(0x10);
        emu.write_u32(iat, at as u32);
        ids.push((at, name));
        at
    };
    let at = slot(&mut emu, IAT_VIRTUAL_ALLOC, "alloc");
    emu.stub(at, if alloc_fails { 0 } else { region as u32 }, 4);
    let at = slot(&mut emu, IAT_LOAD_LIBRARY, "load");
    if library_fails {
        emu.stub(at, 0, 1);
    } else {
        emu.stub_alloc(at, u64::from(FIRST), u64::from(STRIDE), 1);
    }
    let at = slot(&mut emu, IAT_GET_PROC, "proc");
    emu.stub_alloc(at, u64::from(FIRST), u64::from(STRIDE), 2);
    let at = slot(&mut emu, IAT_PROTECT, "protect");
    emu.stub(at, 1, 4);
    let at = slot(&mut emu, IAT_CURRENT_PROCESS, "process");
    emu.stub(at, PROCESS, 0);
    let at = slot(&mut emu, IAT_FLUSH, "flush");
    emu.stub(at, 1, 3);
    let at = slot(&mut emu, IAT_VIRTUAL_FREE, "free");
    emu.stub(at, 1, 3);
    let at = slot(&mut emu, IAT_MODULE_HANDLE, "handle");
    emu.stub(at, HANDLE, 1);
    let at = slot(&mut emu, IAT_FREE_LIBRARY, "freelib");
    emu.stub(at, 1, 1);
    // SErrDisplayAppFatal is cdecl and does not return in the client.
    emu.stub(FATAL, 0, 0);
    ids.push((FATAL, "fatal"));
    Rig { emu, region, ids }
}

fn cstr(emu: &Emu, at: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut p = u64::from(at);
    loop {
        let b = emu.read(p, 1)[0];
        if b == 0 {
            return out;
        }
        out.push(b);
        p += 1;
    }
}

impl Rig {
    /// The stub calls since the last clear, as events.
    fn events(&self) -> Vec<Ev> {
        let name = |addr: u64| self.ids.iter().find(|(a, _)| *a == addr).unwrap().1;
        self.emu
            .calls()
            .iter()
            .map(|c| match name(c.addr) {
                "alloc" => {
                    assert_eq!((c.args[0], c.args[2], c.args[3]), (0, 0x1000, 4));
                    Ev::Alloc(c.args[1])
                }
                "load" => Ev::LoadLibrary(cstr(&self.emu, c.args[0])),
                "proc" => {
                    if c.args[1] < 0x1_0000 {
                        Ev::ProcOrdinal(c.args[0], c.args[1])
                    } else {
                        Ev::ProcName(c.args[0], cstr(&self.emu, c.args[1]))
                    }
                }
                "protect" => Ev::Protect(c.args[0], c.args[1], c.args[2]),
                "process" => Ev::CurrentProcess,
                "flush" => Ev::Flush(c.args[0], c.args[1], c.args[2]),
                "free" => Ev::Free(c.args[0], c.args[1], c.args[2]),
                "handle" => Ev::ModuleHandle(cstr(&self.emu, c.args[0])),
                "freelib" => Ev::FreeLibrary(c.args[0]),
                "fatal" => Ev::Fatal(cstr(&self.emu, c.args[0])),
                other => panic!("unexpected stub {other}"),
            })
            .collect()
    }

    fn run(&mut self, blob: &[u8]) -> (u32, u64) {
        self.emu.clear_calls();
        self.emu.reset_bump(u64::from(FIRST));
        self.emu.write(self.region, &vec![0u8; REGION as usize]);
        let obj = self.emu.alloc(0xc);
        self.emu.write(obj, &[0u8; 0xc]);
        let at = self.emu.alloc(blob.len() as u64 + 16);
        self.emu.write(at, blob);
        let eax = self
            .emu
            .call(LOADER, obj as u32, 0, &[at as u32, blob.len() as u32]);
        (eax, obj)
    }
}

fn model_events(host: &CountingHost, loaded: &Loaded) -> Vec<Ev> {
    let mut v = Vec::new();
    for e in &host.events {
        v.push(match e {
            Event::Alloc(n) => Ev::Alloc(*n),
            Event::LoadLibrary(n) => Ev::LoadLibrary(n.clone()),
            Event::ProcName(l, n) => Ev::ProcName(*l, n.clone()),
            Event::ProcOrdinal(l, o) => Ev::ProcOrdinal(*l, *o),
        });
    }
    for p in &loaded.protects {
        v.push(Ev::Protect(p.address, p.size, p.protection));
        if p.flush {
            v.push(Ev::CurrentProcess);
            v.push(Ev::Flush(PROCESS, p.address, p.size));
        }
    }
    if let Some((a, n)) = loaded.release {
        v.push(Ev::Free(a, n, 0x8000));
    }
    v
}

fn host_events(host: &CountingHost) -> Vec<Ev> {
    host.events
        .iter()
        .map(|e| match e {
            Event::Alloc(n) => Ev::Alloc(*n),
            Event::LoadLibrary(n) => Ev::LoadLibrary(n.clone()),
            Event::ProcName(l, n) => Ev::ProcName(*l, n.clone()),
            Event::ProcOrdinal(l, o) => Ev::ProcOrdinal(*l, *o),
        })
        .collect()
}

#[test]
fn loader_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = rig(&wow, false, false);
    let base = rig.region as u32;
    let mut rng = Rng(0x7a79_d0d0);
    let mut cases = 0;
    for case in 0..120 {
        let libraries = rng.below(4);
        let b = build(&mut rng, libraries);
        let mut host = CountingHost::new(base, false);
        let model =
            image::load(&b.blob, &mut host).unwrap_or_else(|e| panic!("case {case}: {e:?}"));
        let (eax, obj) = rig.run(&b.blob);
        assert_eq!(eax, 1, "case {case}: loader result");
        let got_image = rig.emu.read(rig.region, model.image.len());
        assert_eq!(got_image, model.image, "case {case}: image");
        assert_eq!(
            (
                rig.emu.read_u32(obj),
                rig.emu.read_u32(obj + 4),
                rig.emu.read_u32(obj + 8)
            ),
            (model.base, b.size, model.libraries),
            "case {case}: module object"
        );
        assert_eq!(
            rig.events(),
            model_events(&host, &model),
            "case {case}: calls"
        );

        // The entry lookup, whole, for ordinals below, inside and past the table.
        for ordinal in 0..8u32 {
            let got = rig.emu.call(ENTRY, obj as u32, 0, &[ordinal]);
            let want = image::export_address(&model.image, model.base, ordinal)
                .unwrap()
                .unwrap_or(0);
            assert_eq!(got, want, "case {case}: export {ordinal}");
        }

        // The unloader, whole.
        rig.emu.clear_calls();
        rig.emu.call(UNLOAD, obj as u32, 0, &[]);
        let mut want: Vec<Ev> = Vec::new();
        for name in image::unload_names(&model.image, model.libraries).unwrap() {
            want.push(Ev::ModuleHandle(name));
            want.push(Ev::FreeLibrary(HANDLE));
        }
        want.push(Ev::Free(model.base, b.size, 0x8000));
        assert_eq!(rig.events(), want, "case {case}: unload calls");
        assert_eq!(
            (
                rig.emu.read_u32(obj),
                rig.emu.read_u32(obj + 4),
                rig.emu.read_u32(obj + 8)
            ),
            (0, 0, 0),
            "case {case}: object after unload"
        );
        cases += 1;
    }
    assert_eq!(cases, 120);
}

#[test]
fn loader_library_missing() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = rig(&wow, true, false);
    let base = rig.region as u32;
    let mut rng = Rng(0x1b0a_d001);
    for case in 0..30 {
        let libraries = 1 + rng.below(3);
        let b = build(&mut rng, libraries);
        let mut host = CountingHost::new(base, true);
        let err = image::load(&b.blob, &mut host).unwrap_err();
        assert_eq!(err, LoadError::LibraryMissing { libraries: 0 });
        let (eax, obj) = rig.run(&b.blob);
        assert_eq!(eax, 0, "case {case}");
        // The loader asked once, then the unloader freed the allocation.
        let mut want = host_events(&host);
        want.push(Ev::Free(base, b.size, 0x8000));
        assert_eq!(rig.events(), want, "case {case}");
        assert_eq!(
            (rig.emu.read_u32(obj), rig.emu.read_u32(obj + 4)),
            (0, 0),
            "case {case}: object"
        );
    }
}

#[test]
fn loader_alloc_failed() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = rig(&wow, false, true);
    let mut rng = Rng(0xa110_c000);
    let b = build(&mut rng, 1);
    let (eax, obj) = rig.run(&b.blob);
    assert_eq!(eax, 0);
    assert_eq!(rig.events(), vec![Ev::Alloc(b.size)]);
    assert_eq!(rig.emu.read_u32(obj), 0);
}

#[test]
fn loader_header_too_large() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = rig(&wow, false, false);
    let mut rng = Rng(0x0bad_4ead);
    // The threshold exactly, the top value and random ones above it.
    let mut sizes = vec![0x8000_0000u32, 0x8000_0001, 0xffff_ffff];
    for _ in 0..13 {
        sizes.push(0x8000_0000 | rng.next());
    }
    for size in sizes {
        let mut blob: Vec<u8> = (0..0x40).map(|_| rng.next() as u8).collect();
        blob[..4].copy_from_slice(&size.to_le_bytes());
        let mut host = CountingHost::new(rig.region as u32, false);
        let LoadError::HeaderTooLarge { message } = image::load(&blob, &mut host).unwrap_err()
        else {
            panic!("model accepted size {size:#x}");
        };
        assert!(host.events.is_empty());
        let (eax, _) = rig.run(&blob);
        assert_eq!(eax, 0, "size {size:#x}");
        assert_eq!(
            rig.events(),
            vec![Ev::Fatal(message.into_bytes())],
            "size {size:#x}"
        );
    }
}

/// A module blob built by hand: ten header dwords, section entries `(address, size, protection)`,
/// and one copy run that fills `flat` from the first section's address to its end.
fn blob(header: [u32; 9], sections: &[(u32, u32, u32)], flat: &[u8]) -> Vec<u8> {
    let start = sections[0].0 as usize;
    let mut out = Vec::new();
    for h in header {
        out.extend_from_slice(&h.to_le_bytes());
    }
    out.extend_from_slice(&(sections.len() as u32).to_le_bytes());
    for &(a, n, p) in sections {
        for v in [a, n, p] {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    let run = flat.len() - start;
    assert!(run <= 0xffff);
    out.extend_from_slice(&(run as u16).to_le_bytes());
    out.extend_from_slice(&flat[start..]);
    out
}

fn put(flat: &mut [u8], at: u32, v: u32) {
    flat[at as usize..at as usize + 4].copy_from_slice(&v.to_le_bytes());
}

/// An address in page 0, which the rig maps: a relocation can reach it by wrapping past 2^32.
const PAGE0: u32 = 0x800;

#[test]
fn loader_relocation_forms() {
    // Header: size, unread, relocation stream, count, export table, export count, first ordinal,
    // import table, import count (the section count follows).
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = rig(&wow, false, false);
    let base = rig.region as u32;
    let size = 0x8200u32;
    let reloc = 0x100u32;
    // The long form is `0x80000000` or more, and the client adds the base modulo 2^32: an offset
    // that lands in page 0.
    let long = PAGE0.wrapping_sub(base);
    assert!(long >= 0x8000_0000);
    let mut flat = vec![0u8; size as usize];
    let mut stream = Vec::new();
    stream.extend_from_slice(&[0x00, 0x80]); // short: 0x80
    stream.extend_from_slice(&[0x7f, 0x10]); // short, first byte 0x7f: 0x80 + 0x7f10
    stream.extend_from_slice(&long.to_be_bytes()); // long: page 0
    stream.extend_from_slice(&[0x00, 0x20]); // short after a long: page 0 + 0x20
    flat[reloc as usize..reloc as usize + stream.len()].copy_from_slice(&stream);
    let inside = [(0x80u32, 0x1111_1111u32), (0x7f90, 0x2222_2222)];
    for &(at, v) in &inside {
        put(&mut flat, at, v);
    }
    let outside = [(PAGE0, 0x3333_3333u32), (PAGE0 + 0x20, 0x4444_4444)];
    for &(at, v) in &outside {
        rig.emu.write_u32(u64::from(at), v);
    }
    let b = blob(
        [size, 0, reloc, 4, 0, 0, 0, 0, 0],
        &[(0x40, 0x100, 2)],
        &flat,
    );
    let (eax, obj) = rig.run(&b);
    assert_eq!(eax, 1);
    for &(at, v) in &inside {
        assert_eq!(
            rig.emu.read_u32(rig.region + u64::from(at)),
            v.wrapping_add(base),
            "image dword {at:#x}"
        );
    }
    for &(at, v) in &outside {
        assert_eq!(
            rig.emu.read_u32(u64::from(at)),
            v.wrapping_add(base),
            "page 0 dword {at:#x}"
        );
    }
    // The stream itself and every other dword of the image are untouched.
    let got = rig.emu.read(rig.region, size as usize);
    let mut want = flat.clone();
    want[..0x28].copy_from_slice(&b[..0x28]);
    for &(at, v) in &inside {
        put(&mut want, at, v.wrapping_add(base));
    }
    assert_eq!(got[0x28..], want[0x28..]);
    assert_eq!(
        (rig.emu.read_u32(obj), rig.emu.read_u32(obj + 4)),
        (base, size)
    );
    assert_eq!(
        rig.events(),
        vec![
            Ev::Alloc(size),
            Ev::Protect(base + 0x40, 0x100, 2),
            Ev::Free(base + reloc, size - reloc, 0x8000)
        ]
    );
}

#[test]
fn loader_run_past_the_image_end() {
    // A copy run is not clamped to the image size: bytes past `size` land in the rest of the
    // page `VirtualAlloc` committed. The model keeps those pages.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = rig(&wow, false, false);
    let base = rig.region as u32;
    let size = 0x1800u32;
    let mut flat: Vec<u8> = (0..size + 0x100).map(|i| (i * 7 + 1) as u8).collect();
    flat[..0x40].fill(0);
    let b = blob(
        [size, 0, 0x1000, 0, 0, 0, 0, 0, 0],
        &[(0x40, 0x100, 2)],
        &flat,
    );
    let (eax, _) = rig.run(&b);
    assert_eq!(eax, 1);
    let got = rig.emu.read(rig.region, 0x2000);
    assert_eq!(got[0x1800..0x1900], flat[0x1800..0x1900]);
    assert!(got[0x1900..].iter().all(|&x| x == 0));
    assert_eq!(got[0x40..0x1800], flat[0x40..0x1800]);
    let mut host = CountingHost::new(base, false);
    let model = image::load(&b, &mut host).unwrap();
    assert_eq!(model.image.len(), 0x2000);
    assert_eq!(model.image[0x28..], got[0x28..]);
}

#[test]
fn loader_library_fails_after_some_load() {
    // `LoadLibraryA` fails on its `k`th call (k = 0: never). The module object keeps the count of
    // libraries loaded so far, and the unloader frees that many, last first.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = rig(&wow, false, false);
    let base = rig.region as u32;
    let size = 0x1000u32;
    let names = ["a.dll", "b.dll", "c.dll"];
    let mut flat = vec![0u8; size as usize];
    for (i, n) in names.iter().enumerate() {
        let at = 0x140 + 0x10 * i as u32;
        flat[at as usize..at as usize + n.len()].copy_from_slice(n.as_bytes());
        put(&mut flat, 0x100 + 8 * i as u32, at);
        put(&mut flat, 0x104 + 8 * i as u32, 0x200 + 0x10 * i as u32);
        put(&mut flat, 0x200 + 0x10 * i as u32, 0x170); // by name
        put(&mut flat, 0x204 + 0x10 * i as u32, 0x8000_0003); // by ordinal 3
    }
    flat[0x170..0x174].copy_from_slice(b"Foo\0");
    let b = blob(
        [size, 0, 0x300, 0, 0, 0, 0, 0x100, 3],
        &[(0x40, 0x100, 2)],
        &flat,
    );
    // The stub: count the call, log the name pointer, return `0x70000000 + 0x1000 * n`, or 0 on
    // call `k`.
    let counter = rig.emu.alloc(4) as u32;
    let log = rig.emu.alloc(0x20) as u32;
    for k in 0..=4u32 {
        // Each stub is new code: the emulator keeps what it has translated.
        let code = rig.emu.alloc(0x40);
        let mut c = vec![0xa1];
        c.extend_from_slice(&counter.to_le_bytes()); // mov eax,[counter]
        c.push(0x40); // inc eax
        c.push(0xa3);
        c.extend_from_slice(&counter.to_le_bytes()); // mov [counter],eax
        c.extend_from_slice(&[0x8b, 0x4c, 0x24, 0x04]); // mov ecx,[esp+4]
        c.extend_from_slice(&[0x89, 0x0c, 0x85]);
        c.extend_from_slice(&(log - 4).to_le_bytes()); // mov [eax*4+log-4],ecx
        c.push(0x3d);
        c.extend_from_slice(&k.to_le_bytes()); // cmp eax,k
        c.extend_from_slice(&[0x74, 0x0b]); // je fail
        c.extend_from_slice(&[0xc1, 0xe0, 0x0c]); // shl eax,12
        c.push(0x05);
        c.extend_from_slice(&0x7000_0000u32.to_le_bytes()); // add eax,0x70000000
        c.extend_from_slice(&[0xc2, 0x04, 0x00]); // ret 4
        c.extend_from_slice(&[0x31, 0xc0, 0xc2, 0x04, 0x00]); // fail: xor eax,eax; ret 4
        rig.emu.write(code, &c);
        rig.emu.write_u32(IAT_LOAD_LIBRARY, code as u32);
        rig.emu.write_u32(u64::from(counter), 0);
        let (eax, obj) = rig.run(&b);
        let calls = rig.emu.read_u32(u64::from(counter));
        let loaded = if k == 0 { 3 } else { k - 1 };
        let asked = if k == 0 { 3 } else { k.min(3) };
        // The log is indexed from 1 at `log - 4 + 4n`; read it back by call number.
        let asked_names: Vec<Vec<u8>> = (1..=asked)
            .map(|n| cstr(&rig.emu, rig.emu.read_u32(u64::from(log - 4 + 4 * n))))
            .collect();
        assert_eq!(calls, asked, "k={k}: calls");
        let want_names: Vec<Vec<u8>> = (0..asked as usize)
            .map(|i| names[i].as_bytes().to_vec())
            .collect();
        assert_eq!(asked_names, want_names, "k={k}: library names");
        let mut want = vec![Ev::Alloc(size)];
        for i in 0..loaded {
            let handle = 0x7000_0000 + 0x1000 * (i + 1);
            want.push(Ev::ProcName(handle, b"Foo".to_vec()));
            want.push(Ev::ProcOrdinal(handle, 3));
        }
        if loaded == 3 {
            want.push(Ev::Protect(base + 0x40, 0x100, 2));
            want.push(Ev::Free(base + 0x300, size - 0x300, 0x8000));
        } else {
            for i in (0..loaded as usize).rev() {
                want.push(Ev::ModuleHandle(names[i].as_bytes().to_vec()));
                want.push(Ev::FreeLibrary(HANDLE));
            }
            want.push(Ev::Free(base, size, 0x8000));
        }
        assert_eq!(rig.events(), want, "k={k}: calls made");
        assert_eq!(eax, u32::from(loaded == 3), "k={k}: result");
        if loaded == 3 {
            assert_eq!(rig.emu.read_u32(obj + 8), 3, "k={k}: library count");
        } else {
            assert_eq!(
                (
                    rig.emu.read_u32(obj),
                    rig.emu.read_u32(obj + 4),
                    rig.emu.read_u32(obj + 8)
                ),
                (0, 0, 0),
                "k={k}: object after unload"
            );
        }
        // Each library that loaded has its slots filled, in call order; the others keep theirs.
        for i in 0..3u32 {
            let slot = rig.region + 0x200 + 0x10 * u64::from(i);
            if i < loaded {
                assert_eq!(rig.emu.read_u32(slot), FIRST + STRIDE * 2 * i, "k={k}");
                assert_eq!(rig.emu.read_u32(slot + 4), FIRST + STRIDE * (2 * i + 1));
            } else {
                assert_eq!(rig.emu.read_u32(slot), 0x170, "k={k}");
                assert_eq!(rig.emu.read_u32(slot + 4), 0x8000_0003);
            }
        }
    }
}
