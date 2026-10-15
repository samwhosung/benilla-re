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
        let got_image = rig.emu.read(rig.region, b.size as usize);
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

#[test]
fn loader_long_relocation_leaves_the_image() {
    // A relocation entry whose first byte has bit 7 set reads four big-endian bytes with the top
    // bit kept: an offset of at least 0x80000000, past any image.
    let mut rng = Rng(0x10a6_f0f0);
    let mut b = build(&mut rng, 0);
    let reloc_rva = u32::from_le_bytes(b.blob[8..12].try_into().unwrap());
    b.blob[12..16].copy_from_slice(&1u32.to_le_bytes());
    let sections = u32::from_le_bytes(b.blob[0x24..0x28].try_into().unwrap()) as usize;
    // The stream sits in the first copy run, which starts at the first section's address.
    let first_run = 0x28 + 12 * sections;
    let at = first_run + 2 + (reloc_rva - 0x1000) as usize;
    b.blob[at..at + 4].copy_from_slice(&[0x80, 0x00, 0x10, 0x00]);
    let mut host = CountingHost::new(0x7200_0000, false);
    assert_eq!(
        image::load(&b.blob, &mut host).unwrap_err(),
        LoadError::OutOfImage
    );
}
