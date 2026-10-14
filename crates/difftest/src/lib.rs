//! Runs `WoW.exe` 1.12.1.5875's own code in the Unicorn x86-32 emulator on chosen inputs, so a
//! reimplementation can be compared with the client bit for bit (`METHOD.md#the-harness`).
//!
//! The exe is the reader's own and is never in the repo. [`Wow::load`] finds it at
//! `BENILLA_RE_WOW_EXE`, else at `WoW/WoW.exe` in the repo, refuses any other build by its SHA-256,
//! and returns `None` with one skip line when it is absent; with `BENILLA_RE_REQUIRE_EXE=1` it
//! panics instead, so a skip can never pass as a test.

use object::{Object, ObjectSection};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use unicorn_engine::{Arch, HookType, Mode, Prot, Unicorn};
pub use unicorn_engine::{MemType, RegisterX86};

/// Stack region, mapped above the image (`SizeOfImage` 0x906000 from base 0x400000).
const STACK_BASE: u64 = 0x7000_0000;
const STACK_SIZE: u64 = 0x0010_0000;
/// Scratch region for the structures a function reads through pointers, and for the bump arenas
/// that stand in for the client's allocators.
const SCRATCH_BASE: u64 = 0x7200_0000;
const SCRATCH_SIZE: u64 = 0x0100_0000;
/// Unmapped return address: emulation halts when a `ret` pops it into EIP.
const RET_SENTINEL: u64 = 0x6f6f_6f6f;
/// Instructions per emulation before it counts as a runaway. A count, not a time, so the limit is
/// the same under any load; the largest healthy case runs about 10M instructions.
const EMU_INSTR_BUDGET: usize = 200_000_000;

/// Names the exe in place of `WoW/WoW.exe`.
pub const EXE_VARIABLE: &str = "BENILLA_RE_WOW_EXE";
/// Names the install's `Data` folder in place of `WoW/Data`.
pub const DATA_VARIABLE: &str = "BENILLA_RE_WOW_DATA";
/// Set (to `1`), a test that cannot find the exe or the install panics instead of skipping.
pub const REQUIRE_VARIABLE: &str = "BENILLA_RE_REQUIRE_EXE";
/// Names a file to which every executed basic-block address is appended once.
pub const COVERAGE_VARIABLE: &str = "BENILLA_RE_COVERAGE_OUT";

/// The build every finding is read from, as `tools/exe.env` gives it to every tool.
const IDENTITY: &str = include_str!("../../../tools/exe.env");

fn identity(key: &str) -> &'static str {
    IDENTITY
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("tools/exe.env sets {key}"))
}

/// The repo root: the nearest directory above this crate that holds `METHOD.md` and `symbols/`.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|dir| dir.join("METHOD.md").is_file() && dir.join("symbols").is_dir())
        .expect("the crate lives inside the benilla-re repo")
        .to_path_buf()
}

/// Whether a test that cannot find the exe or the install must fail instead of skipping.
pub fn required() -> bool {
    std::env::var(REQUIRE_VARIABLE).is_ok_and(|value| !value.is_empty() && value != "0")
}

/// Prints the one skip line for the running test, or panics when skipping is forbidden.
fn skip(reason: &str) {
    let thread = std::thread::current();
    let test = thread.name().unwrap_or("test");
    assert!(
        !required(),
        "{test}: {reason}, and {REQUIRE_VARIABLE} forbids skipping"
    );
    eprintln!("skip {test}: {reason}");
}

/// The file a fixture variable names, read; `None` with one skip line when it is unset or
/// unreadable. A fixture comes from a run of the real client or a third party and is never in the
/// repo, so it skips even under `BENILLA_RE_REQUIRE_EXE`, and no row names a test that needs one.
pub fn fixture(variable: &str) -> Option<Vec<u8>> {
    let thread = std::thread::current();
    let test = thread.name().unwrap_or("test");
    let Some(path) = std::env::var_os(variable).filter(|p| !p.is_empty()) else {
        eprintln!("skip {test}: {variable} is not set");
        return None;
    };
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(error) => {
            eprintln!(
                "skip {test}: {variable} = {}: {error}",
                Path::new(&path).display()
            );
            None
        }
    }
}

/// The install's `Data` folder: `BENILLA_RE_WOW_DATA`, else `WoW/Data` in the repo. `None` with
/// one skip line when it is absent; under `BENILLA_RE_REQUIRE_EXE` a missing install panics.
pub fn data_dir() -> Option<PathBuf> {
    let dir = match std::env::var_os(DATA_VARIABLE) {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => repo_root().join("WoW").join("Data"),
    };
    if dir.is_dir() {
        Some(dir)
    } else {
        skip(&format!("no install data at {}", dir.display()));
        None
    }
}

/// Where the exe is: `BENILLA_RE_WOW_EXE` when set, else `WoW/WoW.exe` in the repo.
pub fn exe_path() -> PathBuf {
    match std::env::var_os(EXE_VARIABLE) {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => repo_root().join("WoW").join("WoW.exe"),
    }
}

/// The exe's bytes once identified, or the path where none was found. Read once per process.
static EXE: OnceLock<Result<Vec<u8>, PathBuf>> = OnceLock::new();

fn read_exe() -> Result<Vec<u8>, PathBuf> {
    let path = exe_path();
    let Ok(bytes) = std::fs::read(&path) else {
        return Err(path);
    };
    let digest: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let size: u64 = identity("EXE_SIZE").parse().expect("EXE_SIZE is a decimal");
    assert!(
        bytes.len() as u64 == size && digest == identity("EXE_SHA256"),
        "refused {}: {} bytes, SHA-256 {digest}; the findings are read from 1.12.1.5875, \
         {size} bytes, SHA-256 {}",
        path.display(),
        bytes.len(),
        identity("EXE_SHA256"),
    );
    Ok(bytes)
}

/// The image of `WoW.exe` 1.12.1.5875.
#[derive(Clone, Copy)]
pub struct Wow {
    bytes: &'static [u8],
}

impl Wow {
    /// The reader's exe, identified by its hash. `None`, after one skip line, when it is absent;
    /// panics when it is another build, or when it is absent and skipping is forbidden.
    pub fn load() -> Option<Wow> {
        match EXE.get_or_init(read_exe) {
            Ok(bytes) => Some(Wow { bytes }),
            Err(path) => {
                skip(&format!(
                    "no WoW.exe at {}; set {EXE_VARIABLE} or copy it to WoW/WoW.exe",
                    path.display()
                ));
                None
            }
        }
    }
}

/// Where executed block addresses go when `BENILLA_RE_COVERAGE_OUT` is set.
static COVERAGE: OnceLock<Option<std::sync::Mutex<CoverageSink>>> = OnceLock::new();

struct CoverageSink {
    file: std::fs::File,
    seen: std::collections::HashSet<u64>,
}

/// Appends `addr` to the coverage file the first time it is seen; a no-op when recording is off.
fn record_coverage(addr: u64) {
    let sink = COVERAGE.get_or_init(|| {
        let path = std::env::var_os(COVERAGE_VARIABLE)?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()?;
        Some(std::sync::Mutex::new(CoverageSink {
            file,
            seen: std::collections::HashSet::new(),
        }))
    });
    if let Some(m) = sink {
        let mut s = m.lock().unwrap_or_else(|e| e.into_inner());
        if s.seen.insert(addr) {
            use std::io::Write;
            let _ = writeln!(s.file, "{addr:#x}");
        }
    }
}

/// `.text`'s bytes and its base address, kept from the first [`Emu::new`] for [`scan_island`].
static TEXT_IMAGE: OnceLock<(Vec<u8>, u64)> = OnceLock::new();
/// Positions in `.text`, bucketed by first byte, for the island byte match.
static TEXT_INDEX: OnceLock<Vec<Vec<u32>>> = OnceLock::new();
/// Island blocks already scanned, keyed by address and an 8-byte prefix.
static ISLAND_SEEN: OnceLock<std::sync::Mutex<std::collections::HashSet<u64>>> = OnceLock::new();

/// True the first time `key` is seen, so each island block is scanned once.
fn island_first_seen(key: u64) -> bool {
    let seen = ISLAND_SEEN.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()));
    let mut s = seen.lock().unwrap_or_else(|e| e.into_inner());
    s.insert(key)
}

/// Records as covered the `.text` addresses an island block copies: a test that runs client bytes
/// copied into scratch executes them at a scratch address, so every 8-byte window of the block is
/// matched back against `.text`. A run shorter than 8 bytes is not recognized.
fn scan_island(block: &[u8]) {
    let Some((text, va)) = TEXT_IMAGE.get() else {
        return;
    };
    if block.len() < 8 {
        return;
    }
    let index = TEXT_INDEX.get_or_init(|| {
        let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); 256];
        let n = text.len();
        for (i, &b) in text.iter().enumerate() {
            if i + 8 <= n {
                buckets[b as usize].push(i as u32);
            }
        }
        buckets
    });
    for w in block.windows(8) {
        for &off in &index[w[0] as usize] {
            let o = off as usize;
            if &text[o..o + 8] == w {
                record_coverage(va + u64::from(off));
            }
        }
    }
}

/// A call to a stubbed function: the stub's address and the first eight stack arguments
/// (`args[0]` = `[esp+4]`). `ecx` and `edx` are read with [`Emu::get_reg`].
#[derive(Clone, Copy, Debug)]
pub struct StubCall {
    pub addr: u64,
    pub args: [u32; 8],
}

/// The registers [`Emu::snapshot`] carries; the x87 data stack is not among them.
const SNAP_REGS: &[RegisterX86] = &[
    RegisterX86::EAX,
    RegisterX86::EBX,
    RegisterX86::ECX,
    RegisterX86::EDX,
    RegisterX86::ESI,
    RegisterX86::EDI,
    RegisterX86::EBP,
    RegisterX86::ESP,
    RegisterX86::EIP,
    RegisterX86::EFLAGS,
    RegisterX86::FPCW,
    RegisterX86::MXCSR,
];

/// The XMM registers zeroed at every entry. x86-32 conventions pass floats on the stack, so a
/// fresh call site sees no live XMM value.
const SSE_REGS: &[RegisterX86] = &[
    RegisterX86::XMM0,
    RegisterX86::XMM1,
    RegisterX86::XMM2,
    RegisterX86::XMM3,
    RegisterX86::XMM4,
    RegisterX86::XMM5,
    RegisterX86::XMM6,
    RegisterX86::XMM7,
];

/// MXCSR's control bits, all but the six sticky exception flags. The exe executes no `ldmxcsr`
/// and its CRT's `_controlfp` touches only the x87 word, so MXCSR stays at the OS default `0x1F80`
/// (round to nearest, every exception masked, no FTZ or DAZ) for the process's life.
const MXCSR_CONTROL_MASK: u64 = 0xFFFF_FFC0;
const MXCSR_DEFAULT: u64 = 0x1F80;

/// See [`Emu::snapshot`].
pub struct EmuSnapshot {
    regions: Vec<(u64, Vec<u8>)>,
    regs: Vec<(RegisterX86, u64)>,
    scratch_next: u64,
    bump_next: u64,
    bump_stride: u64,
    bump_end: u64,
}

/// The emulator's own state, carried as Unicorn's user data so stub hooks can write to it.
#[derive(Default)]
pub struct EmuState {
    pub calls: Vec<StubCall>,
    /// The next address the bump allocator stubs hand out, and the fixed-stride stubs' stride.
    bump_next: u64,
    bump_stride: u64,
    /// The exclusive end of the [`Emu::stub_alloc_sized`] arena; 0 when none is registered.
    bump_end: u64,
    /// The last unmapped access, `(address, pc, kind)`, cleared at the start of each run.
    last_fault: Option<(u64, u64, MemType)>,
}

/// An emulator with every section of `WoW.exe` mapped at its virtual address, a stack and a
/// scratch arena. Registers, memory and the stub call log persist across [`Emu::call`] and
/// [`Emu::run`]; a test rewrites its inputs and clears the log per case.
pub struct Emu<'a> {
    uc: Unicorn<'a, EmuState>,
    scratch_next: u64,
    /// The `fstp dword [slot]` return trampoline of [`Emu::call_f32`], built on first use.
    tramp: Option<(u64, u64)>,
    /// The `fstp qword [slot]` return trampoline of [`Emu::call_f64`], built on first use.
    tramp64: Option<(u64, u64)>,
}

impl<'a> Emu<'a> {
    /// Maps every section of the exe at its address (file bytes written, the virtual tail zeroed),
    /// then the stack and the scratch arena, and pins the x87 and SSE control words.
    pub fn new(wow: &Wow) -> Emu<'a> {
        let pe = object::read::pe::PeFile32::parse(wow.bytes).expect("parse WoW.exe PE32");
        let mut uc = Unicorn::new_with_data(Arch::X86, Mode::MODE_32, EmuState::default())
            .expect("unicorn new x86-32");
        for sec in pe.sections() {
            let va = sec.address();
            let vsize = sec.size();
            if vsize == 0 {
                continue;
            }
            let base = va & !0xfff;
            let map_size = ((va + vsize + 0xfff) & !0xfff) - base;
            uc.mem_map(base, map_size, Prot::ALL)
                .unwrap_or_else(|e| panic!("map section @ {va:#x} ({vsize:#x}): {e:?}"));
            if let Ok(data) = sec.data() {
                uc.mem_write(va, data).expect("write section bytes");
                if sec.name().map(|n| n == ".text").unwrap_or(false) {
                    let _ = TEXT_IMAGE.set((data.to_vec(), va));
                }
            }
        }
        uc.mem_map(STACK_BASE, STACK_SIZE, Prot::ALL)
            .expect("map stack");
        uc.mem_map(SCRATCH_BASE, SCRATCH_SIZE, Prot::ALL)
            .expect("map scratch");
        // The client's x87 control word is 0x023F; 0x027F differs only in reserved bit 6. WinMain (0x404100) calls
        // __control87(0x9001F, 0xFFFFFFFF), 53-bit precision (METHOD.md#the-harness).
        uc.reg_write(RegisterX86::FPCW, 0x027F)
            .expect("set FPCW=PC_53");
        uc.reg_write(RegisterX86::MXCSR, MXCSR_DEFAULT)
            .expect("set MXCSR=OS-default");
        // With BENILLA_RE_COVERAGE_OUT set, record every executed block; one run in scratch is an
        // island of copied client bytes, whose .text source is recovered by byte match.
        if std::env::var_os(COVERAGE_VARIABLE).is_some() {
            uc.add_block_hook(0, u64::MAX, |uc, addr, size| {
                if (SCRATCH_BASE..SCRATCH_BASE + SCRATCH_SIZE).contains(&addr) {
                    let mut sig = [0u8; 8];
                    if uc.mem_read(addr, &mut sig).is_ok() {
                        let key =
                            addr.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ u64::from_le_bytes(sig);
                        if island_first_seen(key) {
                            let n = (size as usize).clamp(8, 256);
                            let mut buf = vec![0u8; n];
                            if uc.mem_read(addr, &mut buf).is_ok() {
                                scan_island(&buf);
                            }
                        }
                    }
                } else {
                    record_coverage(addr);
                }
            })
            .expect("add coverage block hook");
        }
        // Records the address, pc and kind of an unmapped access; returning false leaves it
        // unhandled, so the run aborts exactly as it would without the hook.
        uc.add_mem_hook(
            HookType::MEM_READ_UNMAPPED
                | HookType::MEM_WRITE_UNMAPPED
                | HookType::MEM_FETCH_UNMAPPED,
            0,
            u64::MAX,
            |uc, mem_type, addr, _size, _value| {
                let pc = uc.reg_read(RegisterX86::EIP).unwrap_or(0);
                uc.get_data_mut().last_fault = Some((addr, pc, mem_type));
                false
            },
        )
        .expect("add unmapped-access fault hook");
        Emu {
            uc,
            scratch_next: SCRATCH_BASE,
            tramp: None,
            tramp64: None,
        }
    }

    /// Maps `size` zeroed bytes at `va` (page aligned), for state the exe reaches at a fixed
    /// address: the thread information block that `fs:0` reads is page 0.
    pub fn map(&mut self, va: u64, size: u64) {
        self.uc
            .mem_map(va, size, Prot::ALL)
            .unwrap_or_else(|e| panic!("map {va:#x} ({size:#x}): {e:?}"));
    }

    /// Reserves `n` zeroed, 16-byte-aligned bytes in the scratch arena and returns their address.
    pub fn alloc(&mut self, n: u64) -> u64 {
        let p = self.scratch_next;
        self.scratch_next = (p + n + 0xf) & !0xf;
        assert!(
            self.scratch_next <= SCRATCH_BASE + SCRATCH_SIZE,
            "scratch arena exhausted"
        );
        p
    }

    pub fn write(&mut self, va: u64, bytes: &[u8]) {
        self.uc.mem_write(va, bytes).expect("mem_write");
    }
    /// Reads `n` bytes at `va`. A read of `.text` counts as coverage of that address, since a test
    /// reads client code to check it or to copy it into an island.
    pub fn read(&self, va: u64, n: usize) -> Vec<u8> {
        let mut b = vec![0u8; n];
        self.uc.mem_read(va, &mut b).expect("mem_read");
        if let Some((text, tva)) = TEXT_IMAGE.get() {
            let end = tva + text.len() as u64;
            if va >= *tva && va < end {
                record_coverage(va);
            }
        }
        b
    }
    pub fn write_u32(&mut self, va: u64, v: u32) {
        self.write(va, &v.to_le_bytes());
    }
    pub fn read_u32(&self, va: u64) -> u32 {
        let mut b = [0u8; 4];
        self.uc.mem_read(va, &mut b).expect("mem_read");
        u32::from_le_bytes(b)
    }
    /// Reads a 32-bit float by its bits.
    pub fn read_f32(&self, va: u64) -> f32 {
        f32::from_bits(self.read_u32(va))
    }

    /// Sets a register before a [`Emu::run`] or [`Emu::call`]. `FPCW` is already `0x027F`; set it
    /// only to run at another precision on purpose.
    pub fn set_reg(&mut self, reg: RegisterX86, value: u64) {
        self.uc.reg_write(reg, value).expect("reg_write");
    }
    pub fn get_reg(&self, reg: RegisterX86) -> u64 {
        self.uc.reg_read(reg).expect("reg_read")
    }

    /// Replaces the function at `addr`: sets `EAX = ret_eax`, logs the call, and returns as the
    /// callee would, popping the return address and `pop_args` stack dwords (its `ret 4*pop_args`;
    /// 0 for `__cdecl`).
    pub fn stub(&mut self, addr: u64, ret_eax: u32, pop_args: u32) {
        self.uc
            .add_code_hook(addr, addr, move |uc, _addr, _size| {
                let esp = uc.reg_read(RegisterX86::ESP).unwrap();
                let rd = |va: u64| -> u32 {
                    let mut b = [0u8; 4];
                    uc.mem_read(va, &mut b).ok();
                    u32::from_le_bytes(b)
                };
                let ret = u64::from(rd(esp));
                let mut args = [0u32; 8];
                for (i, a) in args.iter_mut().enumerate() {
                    *a = rd(esp + 4 + 4 * i as u64);
                }
                uc.get_data_mut().calls.push(StubCall { addr, args });
                uc.reg_write(RegisterX86::EAX, u64::from(ret_eax)).unwrap();
                uc.reg_write(RegisterX86::ESP, esp + 4 + 4 * u64::from(pop_args))
                    .unwrap();
                uc.set_pc(ret).unwrap();
            })
            .expect("add stub hook");
    }

    /// Stubs a function that returns through a pointer argument: copies `len` bytes from
    /// `payload_va` to the pointer in stack argument `arg_idx` (0 = `[esp+4]`), then returns like
    /// [`Emu::stub`]. Any executable address works, including a scratch vtable slot.
    pub fn stub_write_through(
        &mut self,
        addr: u64,
        arg_idx: u32,
        payload_va: u64,
        len: usize,
        ret_eax: u32,
        pop_args: u32,
    ) {
        self.uc
            .add_code_hook(addr, addr, move |uc, _addr, _size| {
                let esp = uc.reg_read(RegisterX86::ESP).unwrap();
                let rd = |va: u64| -> u32 {
                    let mut b = [0u8; 4];
                    uc.mem_read(va, &mut b).ok();
                    u32::from_le_bytes(b)
                };
                let ret = u64::from(rd(esp));
                let mut args = [0u32; 8];
                for (i, a) in args.iter_mut().enumerate() {
                    *a = rd(esp + 4 + 4 * i as u64);
                }
                let dest = u64::from(args[arg_idx as usize]);
                let mut payload = vec![0u8; len];
                uc.mem_read(payload_va, &mut payload).ok();
                uc.mem_write(dest, &payload).ok();
                uc.get_data_mut().calls.push(StubCall { addr, args });
                uc.reg_write(RegisterX86::EAX, u64::from(ret_eax)).unwrap();
                uc.reg_write(RegisterX86::ESP, esp + 4 + 4 * u64::from(pop_args))
                    .unwrap();
                uc.set_pc(ret).unwrap();
            })
            .expect("add stub hook");
    }

    /// Stubs a function to return the `f64` at `value_va` in `st0`, logging its stack arguments.
    /// For code that calls an x87 transcendental (`pow`, `atan2`), whose result differs by CPU:
    /// the test injects the value and checks the arguments. The hook jumps to a scratch island
    /// `fld qword [value_va]; ret 4*pop_args`, since a hook cannot push the x87 stack itself.
    pub fn stub_f64(&mut self, addr: u64, value_va: u64, pop_args: u32) {
        let island = self.alloc(0x10);
        let mut code = vec![0xddu8, 0x05]; // fld qword ptr [disp32]
        code.extend_from_slice(&(value_va as u32).to_le_bytes());
        if pop_args == 0 {
            code.push(0xc3); // ret
        } else {
            code.push(0xc2); // ret imm16
            code.extend_from_slice(&u16::try_from(pop_args * 4).expect("pop_args").to_le_bytes());
        }
        self.write(island, &code);
        self.uc
            .add_code_hook(addr, addr, move |uc, _addr, _size| {
                let esp = uc.reg_read(RegisterX86::ESP).unwrap();
                let mut args = [0u32; 8];
                for (i, a) in args.iter_mut().enumerate() {
                    let mut b = [0u8; 4];
                    uc.mem_read(esp + 4 + 4 * i as u64, &mut b).ok();
                    *a = u32::from_le_bytes(b);
                }
                uc.get_data_mut().calls.push(StubCall { addr, args });
                uc.set_pc(island).unwrap();
            })
            .expect("add stub_f64 hook");
    }

    /// A copy of the emulator: every mapped region, the integer and control registers, and the
    /// scratch and bump cursors. Build a state once, snapshot it, and [`Emu::restore`] before each
    /// case. The x87 data stack is not carried, so a snapshot is taken between calls.
    pub fn snapshot(&mut self) -> EmuSnapshot {
        let regions = self
            .uc
            .mem_regions()
            .expect("mem_regions")
            .iter()
            .map(|r| {
                let n = (r.end - r.begin + 1) as usize;
                let mut b = vec![0u8; n];
                self.uc.mem_read(r.begin, &mut b).expect("snapshot read");
                (r.begin, b)
            })
            .collect();
        let regs = SNAP_REGS
            .iter()
            .map(|&r| (r, self.uc.reg_read(r).unwrap()))
            .collect();
        EmuSnapshot {
            regions,
            regs,
            scratch_next: self.scratch_next,
            bump_next: self.uc.get_data().bump_next,
            bump_stride: self.uc.get_data().bump_stride,
            bump_end: self.uc.get_data().bump_end,
        }
    }

    /// Restores an [`EmuSnapshot`]: every region, the registers and the cursors; clears the call
    /// log and resets the x87 stack.
    pub fn restore(&mut self, s: &EmuSnapshot) {
        for (base, bytes) in &s.regions {
            self.uc.mem_write(*base, bytes).expect("restore write");
        }
        for &(r, v) in &s.regs {
            self.uc.reg_write(r, v).expect("restore reg");
        }
        self.scratch_next = s.scratch_next;
        self.uc.get_data_mut().bump_next = s.bump_next;
        self.uc.get_data_mut().bump_stride = s.bump_stride;
        self.uc.get_data_mut().bump_end = s.bump_end;
        self.uc.get_data_mut().calls.clear();
        self.reset_fp_entry();
    }

    /// Stubs an allocator: each call returns `base + n*stride`, logs the call, and returns via
    /// `ret 4*pop_args`. One bump allocator is active at a time.
    pub fn stub_alloc(&mut self, addr: u64, base: u64, stride: u64, pop_args: u32) {
        self.uc.get_data_mut().bump_next = base;
        self.uc.get_data_mut().bump_stride = stride;
        self.uc
            .add_code_hook(addr, addr, move |uc, _addr, _size| {
                let esp = uc.reg_read(RegisterX86::ESP).unwrap();
                let rd = |va: u64| -> u32 {
                    let mut b = [0u8; 4];
                    uc.mem_read(va, &mut b).ok();
                    u32::from_le_bytes(b)
                };
                let ret = u64::from(rd(esp));
                let mut args = [0u32; 8];
                for (i, a) in args.iter_mut().enumerate() {
                    *a = rd(esp + 4 + 4 * i as u64);
                }
                let st = uc.get_data_mut();
                let node = st.bump_next;
                st.bump_next += st.bump_stride;
                st.calls.push(StubCall { addr, args });
                uc.reg_write(RegisterX86::EAX, node).unwrap();
                uc.reg_write(RegisterX86::ESP, esp + 4 + 4 * u64::from(pop_args))
                    .unwrap();
                uc.set_pc(ret).unwrap();
            })
            .expect("add alloc-stub hook");
    }

    /// Stubs a `realloc(ptr, new_size, …)` on the [`Emu::stub_alloc`] arena: a fresh block, with
    /// `new_size` bytes copied from the old one unless `ptr` is 0. Copying `new_size` bytes keeps
    /// the old prefix; reading past the old block's end only reads more of the arena.
    pub fn stub_realloc(&mut self, addr: u64, pop_args: u32) {
        self.uc
            .add_code_hook(addr, addr, move |uc, _addr, _size| {
                let esp = uc.reg_read(RegisterX86::ESP).unwrap();
                let rd = |va: u64| -> u32 {
                    let mut b = [0u8; 4];
                    uc.mem_read(va, &mut b).ok();
                    u32::from_le_bytes(b)
                };
                let ret = u64::from(rd(esp));
                let ptr = u64::from(rd(esp + 4));
                let new_size = rd(esp + 8);
                let mut args = [0u32; 8];
                for (i, a) in args.iter_mut().enumerate() {
                    *a = rd(esp + 4 + 4 * i as u64);
                }
                let st = uc.get_data_mut();
                let node = st.bump_next;
                st.bump_next += st.bump_stride;
                st.calls.push(StubCall { addr, args });
                if ptr != 0 && new_size != 0 {
                    let mut buf = vec![0u8; new_size as usize];
                    uc.mem_read(ptr, &mut buf).ok();
                    uc.mem_write(node, &buf).ok();
                }
                uc.reg_write(RegisterX86::EAX, node).unwrap();
                uc.reg_write(RegisterX86::ESP, esp + 4 + 4 * u64::from(pop_args))
                    .unwrap();
                uc.set_pc(ret).unwrap();
            })
            .expect("add realloc-stub hook");
    }

    /// Stubs an allocator that bumps by the requested size (stack argument `size_arg`, rounded up
    /// to 16) within `[base, end)`, and panics when the arena runs out rather than overlap. Shares
    /// the bump cursor with the other allocator stubs.
    pub fn stub_alloc_sized(
        &mut self,
        addr: u64,
        base: u64,
        end: u64,
        size_arg: usize,
        pop_args: u32,
    ) {
        self.uc.get_data_mut().bump_next = base;
        self.uc.get_data_mut().bump_end = end;
        self.uc
            .add_code_hook(addr, addr, move |uc, _addr, _size| {
                let esp = uc.reg_read(RegisterX86::ESP).unwrap();
                let rd = |va: u64| -> u32 {
                    let mut b = [0u8; 4];
                    uc.mem_read(va, &mut b).ok();
                    u32::from_le_bytes(b)
                };
                let ret = u64::from(rd(esp));
                let mut args = [0u32; 8];
                for (i, a) in args.iter_mut().enumerate() {
                    *a = rd(esp + 4 + 4 * i as u64);
                }
                let st = uc.get_data_mut();
                let node = (st.bump_next + 0xf) & !0xf;
                let sz = u64::from(args[size_arg]).max(1);
                st.bump_next = node + ((sz + 0xf) & !0xf);
                assert!(
                    st.bump_next <= st.bump_end,
                    "sized bump arena exhausted: {sz:#x}-byte request at {node:#x} past {:#x}",
                    st.bump_end
                );
                st.calls.push(StubCall { addr, args });
                uc.reg_write(RegisterX86::EAX, node).unwrap();
                uc.reg_write(RegisterX86::ESP, esp + 4 + 4 * u64::from(pop_args))
                    .unwrap();
                uc.set_pc(ret).unwrap();
            })
            .expect("add sized-alloc-stub hook");
    }

    /// Stubs a `(ptr, new_size, …)` reallocator on the [`Emu::stub_alloc_sized`] arena, which must
    /// be registered first: a block of `new_size` bytes, the old contents copied unless `ptr` is 0.
    pub fn stub_realloc_sized(&mut self, addr: u64, pop_args: u32) {
        self.uc
            .add_code_hook(addr, addr, move |uc, _addr, _size| {
                let esp = uc.reg_read(RegisterX86::ESP).unwrap();
                let rd = |va: u64| -> u32 {
                    let mut b = [0u8; 4];
                    uc.mem_read(va, &mut b).ok();
                    u32::from_le_bytes(b)
                };
                let ret = u64::from(rd(esp));
                let ptr = u64::from(rd(esp + 4));
                let new_size = rd(esp + 8);
                let mut args = [0u32; 8];
                for (i, a) in args.iter_mut().enumerate() {
                    *a = rd(esp + 4 + 4 * i as u64);
                }
                let st = uc.get_data_mut();
                let node = (st.bump_next + 0xf) & !0xf;
                let sz = u64::from(new_size).max(1);
                st.bump_next = node + ((sz + 0xf) & !0xf);
                assert!(
                    st.bump_next <= st.bump_end,
                    "sized bump arena exhausted: {sz:#x}-byte realloc at {node:#x} past {:#x}",
                    st.bump_end
                );
                st.calls.push(StubCall { addr, args });
                if ptr != 0 && new_size != 0 {
                    let mut buf = vec![0u8; new_size as usize];
                    uc.mem_read(ptr, &mut buf).ok();
                    uc.mem_write(node, &buf).ok();
                }
                uc.reg_write(RegisterX86::EAX, node).unwrap();
                uc.reg_write(RegisterX86::ESP, esp + 4 + 4 * u64::from(pop_args))
                    .unwrap();
                uc.set_pc(ret).unwrap();
            })
            .expect("add sized-realloc-stub hook");
    }

    /// The stub calls since the last [`Emu::clear_calls`], in order.
    pub fn calls(&self) -> &[StubCall] {
        &self.uc.get_data().calls
    }
    pub fn clear_calls(&mut self) {
        self.uc.get_data_mut().calls.clear();
    }
    /// Rewinds the [`Emu::stub_alloc`] cursor to `base`.
    pub fn reset_bump(&mut self, base: u64) {
        self.uc.get_data_mut().bump_next = base;
    }

    /// Puts the FPU and SSE state a fresh call site sees: every x87 slot empty and the status word
    /// zero (the control word is kept), XMM0 to XMM7 zero, and MXCSR back at `0x1F80`.
    fn reset_fp_entry(&mut self) {
        self.uc.reg_write(RegisterX86::FPSW, 0).expect("reset FPSW");
        self.uc
            .reg_write(RegisterX86::FPTAG, 0xffff)
            .expect("reset FPTAG");
        for &r in SSE_REGS {
            self.uc.reg_write_long(r, &[0u8; 16]).expect("zero XMM");
        }
        self.uc
            .reg_write(RegisterX86::MXCSR, MXCSR_DEFAULT)
            .expect("re-pin MXCSR");
    }

    /// Panics unless the run stopped at its halt address, so a runaway or an early stop is never
    /// read back as a result, and unless MXCSR's control bits are unchanged (the exe has no
    /// `ldmxcsr`; the sticky exception flags are ignored).
    fn check_completed(&self, expected_pc: u64, what: &str) {
        let pc = self.uc.reg_read(RegisterX86::EIP).unwrap();
        if pc != expected_pc {
            panic!(
                "emulation stopped at {pc:#x}, expected halt at {expected_pc:#x} \
                 (instruction budget exhausted = runaway, or bad setup): {what}"
            );
        }
        let mxcsr = self.uc.reg_read(RegisterX86::MXCSR).unwrap();
        if mxcsr & MXCSR_CONTROL_MASK != MXCSR_DEFAULT {
            panic!(
                "MXCSR control word changed to {mxcsr:#x} (control bits expected {MXCSR_DEFAULT:#x}; \
                 WoW.exe executes no ldmxcsr — a stray ldmxcsr or bad setup): {what}"
            );
        }
    }

    /// The last run's unmapped access, `(address, pc, kind)`, or `None`.
    pub fn last_fault(&self) -> Option<(u64, u64, MemType)> {
        self.uc.get_data().last_fault
    }

    fn clear_fault(&mut self) {
        self.uc.get_data_mut().last_fault = None;
    }

    /// `" [fault …]"` for a failed run's panic message, or `""` when no unmapped access happened.
    fn fault_suffix(&self) -> String {
        match self.uc.get_data().last_fault {
            Some((addr, pc, kind)) => format!(" [fault {kind:?} addr={addr:#x} pc={pc:#x}]"),
            None => String::new(),
        }
    }

    /// Runs the exe's bytes from `begin` until EIP reaches `until`, which is not executed, with
    /// the registers and memory the test set. The x87 stack is emptied first.
    pub fn run(&mut self, begin: u64, until: u64) {
        self.reset_fp_entry();
        self.clear_fault();
        if let Err(e) = self.uc.emu_start(begin, until, 0, EMU_INSTR_BUDGET) {
            panic!(
                "emu_start {begin:#x}..{until:#x}: {e:?}{}",
                self.fault_suffix()
            );
        }
        self.check_completed(until, "run slice");
    }

    /// Calls a function: sets `ECX` and `EDX`, pushes `stack_args` right to left under a sentinel
    /// return address, runs until the function returns, and yields `EAX`. Works for `__cdecl`,
    /// `__stdcall`, `__thiscall` and `__fastcall`, since the halt does not depend on `ret N`.
    pub fn call(&mut self, fn_va: u64, ecx: u32, edx: u32, stack_args: &[u32]) -> u32 {
        let mut esp = STACK_BASE + STACK_SIZE - 0x1000;
        for &a in stack_args.iter().rev() {
            esp -= 4;
            self.write_u32(esp, a);
        }
        esp -= 4;
        self.write_u32(esp, RET_SENTINEL as u32);
        self.uc.reg_write(RegisterX86::ESP, esp).unwrap();
        self.uc.reg_write(RegisterX86::ECX, u64::from(ecx)).unwrap();
        self.uc.reg_write(RegisterX86::EDX, u64::from(edx)).unwrap();
        self.reset_fp_entry();
        self.clear_fault();
        if let Err(e) = self.uc.emu_start(fn_va, RET_SENTINEL, 0, EMU_INSTR_BUDGET) {
            panic!("emu_start @ {fn_va:#x}: {e:?}{}", self.fault_suffix());
        }
        self.check_completed(RET_SENTINEL, "call");
        self.uc.reg_read(RegisterX86::EAX).unwrap() as u32
    }

    /// Calls a function that returns a float in `st0`, as [`Emu::call`], and reads it back as a
    /// caller's `fstp dword` would store it: the return lands on a trampoline `fstp dword [slot]`.
    pub fn call_f32(&mut self, fn_va: u64, ecx: u32, edx: u32, stack_args: &[u32]) -> f32 {
        let (tramp, slot) = match self.tramp {
            Some(t) => t,
            None => {
                let tramp = self.alloc(0x10);
                let slot = self.alloc(0x10);
                // fstp dword ptr [disp32]: D9 /3, modrm 0x1d.
                let mut code = vec![0xd9u8, 0x1d];
                code.extend_from_slice(&(slot as u32).to_le_bytes());
                self.write(tramp, &code);
                self.tramp = Some((tramp, slot));
                (tramp, slot)
            }
        };
        let mut esp = STACK_BASE + STACK_SIZE - 0x1000;
        for &a in stack_args.iter().rev() {
            esp -= 4;
            self.write_u32(esp, a);
        }
        esp -= 4;
        self.write_u32(esp, tramp as u32);
        self.uc.reg_write(RegisterX86::ESP, esp).unwrap();
        self.uc.reg_write(RegisterX86::ECX, u64::from(ecx)).unwrap();
        self.uc.reg_write(RegisterX86::EDX, u64::from(edx)).unwrap();
        self.reset_fp_entry();
        self.clear_fault();
        // Halt after the trampoline's 6-byte fstp.
        if let Err(e) = self.uc.emu_start(fn_va, tramp + 6, 0, EMU_INSTR_BUDGET) {
            panic!("emu_start (f32) @ {fn_va:#x}: {e:?}{}", self.fault_suffix());
        }
        self.check_completed(tramp + 6, "call_f32");
        self.read_f32(slot)
    }

    /// Calls a function that leaves its result in `st0` and reads the full 53-bit value back as
    /// `f64`, through a trampoline `fstp qword [slot]`. For a result its caller uses unrounded.
    pub fn call_f64(&mut self, fn_va: u64, ecx: u32, edx: u32, stack_args: &[u32]) -> f64 {
        let (tramp, slot) = match self.tramp64 {
            Some(t) => t,
            None => {
                let tramp = self.alloc(0x10);
                let slot = self.alloc(0x10);
                // fstp qword ptr [disp32]: DD /3, modrm 0x1d.
                let mut code = vec![0xddu8, 0x1d];
                code.extend_from_slice(&(slot as u32).to_le_bytes());
                self.write(tramp, &code);
                self.tramp64 = Some((tramp, slot));
                (tramp, slot)
            }
        };
        let mut esp = STACK_BASE + STACK_SIZE - 0x1000;
        for &a in stack_args.iter().rev() {
            esp -= 4;
            self.write_u32(esp, a);
        }
        esp -= 4;
        self.write_u32(esp, tramp as u32);
        self.uc.reg_write(RegisterX86::ESP, esp).unwrap();
        self.uc.reg_write(RegisterX86::ECX, u64::from(ecx)).unwrap();
        self.uc.reg_write(RegisterX86::EDX, u64::from(edx)).unwrap();
        self.reset_fp_entry();
        self.clear_fault();
        if let Err(e) = self.uc.emu_start(fn_va, tramp + 6, 0, EMU_INSTR_BUDGET) {
            panic!("emu_start (f64) @ {fn_va:#x}: {e:?}{}", self.fault_suffix());
        }
        self.check_completed(tramp + 6, "call_f64");
        let mut b = [0u8; 8];
        self.uc.mem_read(slot, &mut b).expect("mem_read f64 slot");
        f64::from_bits(u64::from_le_bytes(b))
    }

    /// Runs two closed loops from `seed` for `steps` steps, the exe's code (`bin_step`) and the
    /// reimplementation (`kern_step`), each fed its own previous output, and reports the first
    /// step where their carried state differs. For an accumulator, whose reachable states a
    /// single-shot fuzz may never sample and where a small error compounds. State is carried as
    /// raw words, so the comparison is exact for any float layout.
    pub fn trajectory(
        &mut self,
        steps: usize,
        seed: Vec<u32>,
        mut bin_step: impl FnMut(&mut Self, &[u32]) -> Vec<u32>,
        mut kern_step: impl FnMut(&[u32]) -> Vec<u32>,
    ) -> Trajectory {
        let mut b = seed.clone();
        let mut k = seed;
        for step in 0..steps {
            b = bin_step(self, &b);
            k = kern_step(&k);
            if b != k {
                return Trajectory::Diverged {
                    step,
                    bin: b,
                    kern: k,
                };
            }
        }
        Trajectory::Match { steps }
    }
}

/// The outcome of [`Emu::trajectory`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Trajectory {
    /// The carried state matched bit for bit at every step.
    Match { steps: usize },
    /// The first step at which the exe's state (`bin`) and the reimplementation's (`kern`) differ.
    Diverged {
        step: usize,
        bin: Vec<u32>,
        kern: Vec<u32>,
    },
}

impl Trajectory {
    /// Panics with the first divergence unless the whole trajectory matched.
    pub fn assert_match(&self, what: &str) {
        if let Trajectory::Diverged { step, bin, kern } = self {
            panic!(
                "{what}: carried state differs at step {step}: binary {bin:#010x?} vs kernel \
                 {kern:#010x?}"
            );
        }
    }
}

/// Runs the first-call selection `0x74fb49(1)` on the emulated processor, so the math slots
/// (`0x876504`, `0x876544`, `0x876588`, `0x876594`) hold what a client on that processor holds,
/// and returns the product slot.
///
/// What the run needs from outside the exe is supplied: page 0, where `fs:0` (the exception
/// chain that `0x74fa8a` links) lives; `0x74f951`, the registry read, which finds neither
/// `DisablePSGP` nor `DisableD3DXPSGP` (as on a machine without them); `0x750aae`, the one-time
/// library load whose result no slot depends on; and `IsProcessorFeaturePresent` (the import
/// at `0x7ff344`), which reports SSE (6) and not 3DNow (7). The processor tests run as the exe
/// runs them: `0x74fa8a` executes `cpuid` on the emulator.
pub fn select_math_slots(emu: &mut Emu) -> u32 {
    emu.map(0, 0x1000);
    emu.stub(0x750aae, 0, 0);
    emu.stub(0x74f951, 0, 4);
    // `mov eax,[esp+4]; cmp eax,6; sete al; movzx eax,al; ret 4`
    let api = emu.alloc(0x10);
    emu.write(
        api,
        &[
            0x8b, 0x44, 0x24, 0x04, 0x83, 0xf8, 0x06, 0x0f, 0x94, 0xc0, 0x0f, 0xb6, 0xc0, 0xc2,
            0x04, 0x00,
        ],
    );
    emu.write_u32(0x7ff344, api as u32);
    emu.call(0x74fb49, 0, 0, &[1]);
    emu.read_u32(0x876504)
}
