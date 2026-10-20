//! The raid roster against `WoW.exe` in the emulator (`systems/ui/raid.md`):
//!
//! * `raid_rebuild_difftest`: `0x4ba5f0(guids, flags, online, count)` run whole.
//! * `raid_pending_names_difftest`: the name answer `0x4ba1b0`, which calls `0x4bad90`, and the
//!   own-row fill `0x4ba140`, each run whole.
//!
//! Every callee that reaches outside the roster is a stub that logs its arguments, and the logs
//! are compared in order: `FireTutorial` `0x4b5390`, `CancelCallback` `0x55f870`, `GetRecord`
//! `0x55f080`, `DisplayError` `0x496720`, `ClntObjMgrObjectPtr` `0x468460`, the token flag
//! calls `0x60c4f0` and `0x60c4a0`, `ReleaseTarget` `0x493910`, the data store puts `0x418190`,
//! `0x418370` and `0x418070` with `ClientServices::Send` `0x5ab630`, `FrameScript_SignalEvent`
//! `0x703e50`, `OsGetAsyncTimeMs` `0x42c010` and `ShowReadyCheckSummary` `0x4bb310`. Each one's
//! row says what its body does; none of their bodies writes the roster records, the count, the
//! pending count or the deadline. `ClntObjMgrGetActivePlayer` (`0x468550`), `__alloca_probe`
//! (`0x40a280`) and `CDataStore::InternalDestroy` (`0x417bd0`) run as they are. The name records
//! the `GetRecord` stub returns are distinct addresses that the code under test only passes on.
//!
//! The rosters before the call hold 0 to 40 members drawn from a small GUID pool, GUID 0 among
//! them, with every byte of every record random, rows past the count included; the lists overlap
//! them, repeat GUIDs, and carry every flag dword and online byte regime; names, objects, pets,
//! the leader, the active player, the pending count and the deadline vary independently.
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::raid_roster::{
    fill_own_row, on_roster_name, rebuild, NameCallback, NameOwner, RaidHost, Roster, Row,
    BLOCK_LEN, ROWS,
};

const REBUILD: u64 = 0x4ba5f0;
const NAME_ANSWER: u64 = 0x4ba1b0;
const FILL_OWN_ROW: u64 = 0x4ba140;

const FIRE_TUTORIAL: u64 = 0x4b5390;
const CANCEL_CALLBACK: u64 = 0x55f870;
const GET_RECORD: u64 = 0x55f080;
const DISPLAY_ERROR: u64 = 0x496720;
const OBJECT_PTR: u64 = 0x468460;
const REMOVE_TOKEN_FLAGS: u64 = 0x60c4f0;
const ADD_TOKEN_FLAGS: u64 = 0x60c4a0;
const RELEASE_TARGET: u64 = 0x493910;
const PUT_U32: u64 = 0x418190;
const PUT_GUID: u64 = 0x418370;
const PUT_U8: u64 = 0x418070;
const SEND: u64 = 0x5ab630;
const SIGNAL_EVENT: u64 = 0x703e50;
const CLOCK: u64 = 0x42c010;
const READY_SUMMARY: u64 = 0x4bb310;

const ROSTER: u64 = 0xb712a8;
const COUNT: u64 = 0xb713e0;
const PENDING: u64 = 0xb713f0;
const DEADLINE: u64 = 0xb713f4;
const LEADER: u64 = 0xbc75f8;
const OBJECT_MANAGER: u64 = 0xb41414;
const NAME_CACHE: u32 = 0xc0e228;
const RECORD_LEN: usize = 0x160;

const CB_ADDED: u32 = 0x4bacd0;
const CB_REMOVED: u32 = 0x4bad30;
const CB_ROSTER_NAME: u32 = 0x4ba1b0;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Effect {
    Tutorial(u32),
    Cancel(u64, NameCallback),
    Name(u64, NameOwner, NameCallback),
    Error(u32, Option<u64>),
    Object(u32, u64),
    Remove(u64, u32),
    Add(u64, u32),
    Release(u32, u64),
    Send(Vec<u8>),
    Event(u32),
    Clock,
    Summary,
}

/// The objects, names and clock of one case, shared by both sides.
#[derive(Clone, Default)]
struct World {
    names: HashSet<u64>,
    /// GUID to (type mask, CHARM, SUMMON).
    objects: HashMap<u64, (u32, [u64; 2])>,
    active: u64,
    clock: u32,
}

impl World {
    fn object(&self, mask: u32, guid: u64) -> Option<[u64; 2]> {
        self.objects
            .get(&guid)
            .filter(|o| o.0 & mask != 0)
            .map(|o| o.1)
    }
}

struct Model<'w> {
    world: &'w World,
    log: Vec<Effect>,
}

impl RaidHost for Model<'_> {
    fn fire_tutorial(&mut self, id: u32) {
        self.log.push(Effect::Tutorial(id));
    }
    fn cancel_name(&mut self, guid: u64, callback: NameCallback) {
        self.log.push(Effect::Cancel(guid, callback));
    }
    fn name_record(&mut self, guid: u64, owner: NameOwner, callback: NameCallback) -> bool {
        self.log.push(Effect::Name(guid, owner, callback));
        self.world.names.contains(&guid)
    }
    fn display_error(&mut self, id: u32, name_of: Option<u64>) {
        self.log.push(Effect::Error(id, name_of));
    }
    fn active_player(&mut self) -> u64 {
        self.world.active
    }
    fn object(&mut self, mask: u32, guid: u64) -> Option<[u64; 2]> {
        self.log.push(Effect::Object(mask, guid));
        self.world.object(mask, guid)
    }
    fn remove_token_flags(&mut self, guid: u64, flags: u32) {
        self.log.push(Effect::Remove(guid, flags));
    }
    fn add_token_flags(&mut self, guid: u64, flags: u32) {
        self.log.push(Effect::Add(guid, flags));
    }
    fn release_target(&mut self, send_selection: u32, guid: u64) {
        self.log.push(Effect::Release(send_selection, guid));
    }
    fn send(&mut self, message: &[u8]) {
        self.log.push(Effect::Send(message.to_vec()));
    }
    fn signal_event(&mut self, event: u32) {
        self.log.push(Effect::Event(event));
    }
    fn now_ms(&mut self) -> u32 {
        self.log.push(Effect::Clock);
        self.world.clock
    }
    fn ready_check_summary(&mut self) {
        self.log.push(Effect::Summary);
    }
}

/// What the stubs see and log.
#[derive(Default)]
struct Shared {
    world: World,
    log: Vec<Effect>,
    store: Vec<u8>,
    /// Object address to GUID and GUID to object address.
    object_guid: HashMap<u32, u64>,
    object_va: HashMap<u64, u32>,
    /// Name record address to GUID.
    record_guid: HashMap<u32, u64>,
    guid_list: u32,
    rows: Vec<u32>,
}

fn guid_of(lo: u32, hi: u32) -> u64 {
    u64::from(lo) | (u64::from(hi) << 32)
}

fn callback_of(va: u32) -> NameCallback {
    match va {
        CB_ADDED => NameCallback::MemberAdded,
        CB_REMOVED => NameCallback::MemberRemoved,
        CB_ROSTER_NAME => NameCallback::RosterName,
        _ => panic!("unknown name callback {va:#x}"),
    }
}

fn owner_of(s: &Shared, va: u32) -> NameOwner {
    if let Some(i) = s.rows.iter().position(|&r| r == va) {
        return NameOwner::Row(i);
    }
    let off = va.wrapping_sub(s.guid_list);
    assert!(
        off.is_multiple_of(8) && off < 8 * ROWS as u32,
        "owner {va:#x}"
    );
    NameOwner::ListEntry((off / 8) as usize)
}

fn install_stubs(emu: &mut Emu, shared: &Rc<RefCell<Shared>>) {
    let s = shared.clone();
    emu.stub_fn(FIRE_TUTORIAL, 0, move |io| {
        assert_eq!(io.reg(RegisterX86::EDX), 0, "tutorial delay");
        let id = io.reg(RegisterX86::ECX) as u32;
        s.borrow_mut().log.push(Effect::Tutorial(id));
        0
    });
    let s = shared.clone();
    emu.stub_fn(CANCEL_CALLBACK, 4, move |io| {
        assert_eq!(io.reg(RegisterX86::ECX) as u32, NAME_CACHE);
        assert_eq!(io.arg(3), 0, "cancel data");
        let e = Effect::Cancel(guid_of(io.arg(0), io.arg(1)), callback_of(io.arg(2)));
        s.borrow_mut().log.push(e);
        0
    });
    let s = shared.clone();
    emu.stub_fn(GET_RECORD, 6, move |io| {
        assert_eq!(io.reg(RegisterX86::ECX) as u32, NAME_CACHE);
        assert_eq!((io.arg(4), io.arg(5)), (0, 0), "request data and dedupe");
        let guid = guid_of(io.arg(0), io.arg(1));
        let mut sh = s.borrow_mut();
        let owner = owner_of(&sh, io.arg(2));
        sh.log
            .push(Effect::Name(guid, owner, callback_of(io.arg(3))));
        if !sh.world.names.contains(&guid) {
            return 0;
        }
        let va = 0x6000_0000 + 0x10 * sh.record_guid.len() as u32;
        sh.record_guid.insert(va, guid);
        va
    });
    let s = shared.clone();
    emu.stub_fn(DISPLAY_ERROR, 0, move |io| {
        let id = io.arg(0);
        let mut sh = s.borrow_mut();
        let name_of = match id {
            0x196 | 0x197 => Some(*sh.record_guid.get(&io.arg(1)).expect("a name record")),
            _ => None,
        };
        sh.log.push(Effect::Error(id, name_of));
        0
    });
    let s = shared.clone();
    emu.stub_fn(OBJECT_PTR, 3, move |io| {
        let mask = io.reg(RegisterX86::ECX) as u32;
        let guid = guid_of(io.arg(0), io.arg(1));
        let mut sh = s.borrow_mut();
        sh.log.push(Effect::Object(mask, guid));
        if sh.world.object(mask, guid).is_some() {
            sh.object_va[&guid]
        } else {
            0
        }
    });
    for (addr, add) in [(REMOVE_TOKEN_FLAGS, false), (ADD_TOKEN_FLAGS, true)] {
        let s = shared.clone();
        emu.stub_fn(addr, 1, move |io| {
            let mut sh = s.borrow_mut();
            let guid = sh.object_guid[&(io.reg(RegisterX86::ECX) as u32)];
            let flags = io.arg(0);
            sh.log.push(if add {
                Effect::Add(guid, flags)
            } else {
                Effect::Remove(guid, flags)
            });
            0
        });
    }
    let s = shared.clone();
    emu.stub_fn(RELEASE_TARGET, 2, move |io| {
        let e = Effect::Release(
            io.reg(RegisterX86::ECX) as u32,
            guid_of(io.arg(0), io.arg(1)),
        );
        s.borrow_mut().log.push(e);
        0
    });
    let s = shared.clone();
    emu.stub_fn(PUT_U32, 1, move |io| {
        let v = io.arg(0);
        s.borrow_mut().store.extend_from_slice(&v.to_le_bytes());
        0
    });
    let s = shared.clone();
    emu.stub_fn(PUT_GUID, 2, move |io| {
        let v = guid_of(io.arg(0), io.arg(1));
        s.borrow_mut().store.extend_from_slice(&v.to_le_bytes());
        0
    });
    let s = shared.clone();
    emu.stub_fn(PUT_U8, 1, move |io| {
        let v = io.arg(0) as u8;
        s.borrow_mut().store.push(v);
        0
    });
    let s = shared.clone();
    emu.stub_fn(SEND, 0, move |_| {
        let mut sh = s.borrow_mut();
        let message = std::mem::take(&mut sh.store);
        sh.log.push(Effect::Send(message));
        0
    });
    let s = shared.clone();
    emu.stub_fn(SIGNAL_EVENT, 0, move |io| {
        let event = io.reg(RegisterX86::ECX) as u32;
        s.borrow_mut().log.push(Effect::Event(event));
        0
    });
    let s = shared.clone();
    emu.stub_fn(CLOCK, 0, move |_| {
        let mut sh = s.borrow_mut();
        sh.log.push(Effect::Clock);
        sh.world.clock
    });
    let s = shared.clone();
    emu.stub_fn(READY_SUMMARY, 0, move |_| {
        s.borrow_mut().log.push(Effect::Summary);
        0
    });
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    fn chance(&mut self, percent: u32) -> bool {
        self.below(100) < percent
    }
    fn u64(&mut self) -> u64 {
        guid_of(self.next(), self.next())
    }
}

fn row_bytes(r: &Row) -> Vec<u8> {
    let mut b = Vec::with_capacity(RECORD_LEN);
    b.extend_from_slice(&r.guid.to_le_bytes());
    b.extend_from_slice(&r.subgroup.to_le_bytes());
    b.extend_from_slice(&r.rank.to_le_bytes());
    b.extend_from_slice(&r.block);
    b.extend_from_slice(&r.ready.to_le_bytes());
    b.extend_from_slice(&r.tail.to_le_bytes());
    b
}

fn rd32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

fn row_from(b: &[u8]) -> Row {
    Row {
        guid: guid_of(rd32(b, 0), rd32(b, 4)),
        subgroup: rd32(b, 8),
        rank: rd32(b, 0xc),
        block: b[0x10..0x10 + BLOCK_LEN].try_into().unwrap(),
        ready: rd32(b, 0x158),
        tail: rd32(b, 0x15c),
    }
}

/// The emulator's side of one case: the records, the globals and the objects in memory.
struct Rig {
    rows: Vec<u64>,
    guid_list: u64,
    flag_list: u64,
    online_list: u64,
    objects: Vec<u64>,
    manager: u64,
}

fn rig(emu: &mut Emu, shared: &Rc<RefCell<Shared>>) -> Rig {
    let rows: Vec<u64> = (0..ROWS).map(|_| emu.alloc(RECORD_LEN as u64)).collect();
    for (i, &r) in rows.iter().enumerate() {
        emu.write_u32(ROSTER + 4 * i as u64, r as u32);
    }
    let objects: Vec<u64> = (0..64).map(|_| emu.alloc(0x140)).collect();
    let rig = Rig {
        rows,
        guid_list: emu.alloc(8 * ROWS as u64),
        flag_list: emu.alloc(4 * ROWS as u64),
        online_list: emu.alloc(ROWS as u64),
        objects,
        manager: emu.alloc(0xe0),
    };
    let mut sh = shared.borrow_mut();
    sh.guid_list = rig.guid_list as u32;
    sh.rows = rig.rows.iter().map(|&r| r as u32).collect();
    drop(sh);
    rig
}

/// Writes the roster and the world into the emulator and resets the logs.
fn load(emu: &mut Emu, rig: &Rig, shared: &Rc<RefCell<Shared>>, roster: &Roster, world: &World) {
    for (row, &va) in roster.rows.iter().zip(&rig.rows) {
        emu.write(va, &row_bytes(row));
    }
    emu.write_u32(COUNT, roster.count);
    emu.write_u32(PENDING, roster.pending);
    emu.write_u32(DEADLINE, roster.deadline);
    emu.write(LEADER, &roster.leader.to_le_bytes());
    if world.active == 0 && roster.count.is_multiple_of(2) {
        emu.write_u32(OBJECT_MANAGER, 0);
    } else {
        emu.write(rig.manager + 0xc0, &world.active.to_le_bytes());
        emu.write_u32(OBJECT_MANAGER, rig.manager as u32);
    }
    let mut sh = shared.borrow_mut();
    sh.world = world.clone();
    sh.log.clear();
    sh.store.clear();
    sh.record_guid.clear();
    sh.object_guid.clear();
    sh.object_va.clear();
    let mut guids: Vec<u64> = world.objects.keys().copied().collect();
    guids.sort_unstable();
    for (k, guid) in guids.into_iter().enumerate() {
        let va = rig.objects[k];
        let fields = va + 0x120;
        let (_, pets) = world.objects[&guid];
        emu.write_u32(va + 0x110, fields as u32);
        emu.write(fields, &pets[0].to_le_bytes());
        emu.write(fields + 8, &pets[1].to_le_bytes());
        sh.object_guid.insert(va as u32, guid);
        sh.object_va.insert(guid, va as u32);
    }
}

fn read_back(emu: &Emu, rig: &Rig, leader: u64) -> Roster {
    Roster {
        rows: rig
            .rows
            .iter()
            .map(|&va| row_from(&emu.read(va, RECORD_LEN)))
            .collect(),
        count: emu.read_u32(COUNT),
        pending: emu.read_u32(PENDING),
        deadline: emu.read_u32(DEADLINE),
        leader,
    }
}

fn pick_count(r: &mut Rng) -> usize {
    match r.below(10) {
        0 => 0,
        1 => 40,
        2 => 39,
        _ => 1 + r.below(8) as usize,
    }
}

fn random_world(r: &mut Rng, pool: &[u64]) -> World {
    let mut w = World::default();
    for &g in pool {
        if g != 0 && r.chance(50) {
            w.names.insert(g);
        }
        if g == 0 {
            continue;
        }
        if r.chance(55) {
            let pet = |r: &mut Rng| {
                if r.chance(50) {
                    0
                } else {
                    pool[r.below(pool.len() as u32) as usize]
                }
            };
            let pets = [pet(r), pet(r)];
            w.objects.insert(g, (0x18, pets));
        } else if r.chance(40) {
            w.objects.insert(g, (0x8, [r.u64(), r.u64()]));
        }
    }
    w.active = if r.chance(15) {
        0
    } else {
        pool[r.below(pool.len() as u32) as usize]
    };
    w.clock = match r.below(4) {
        0 => 0,
        1 => 1,
        _ => r.next(),
    };
    w
}

fn random_roster(r: &mut Rng, pool: &[u64], world: &World) -> Roster {
    let count = pick_count(r);
    let rows = (0..ROWS)
        .map(|i| {
            let mut block = [0u8; BLOCK_LEN];
            for b in block.iter_mut() {
                *b = r.next() as u8;
            }
            let guid = if i < count || r.chance(60) {
                pool[r.below(pool.len() as u32) as usize]
            } else {
                r.u64()
            };
            Row {
                guid,
                subgroup: r.next(),
                rank: r.next(),
                block,
                ready: match r.below(4) {
                    0 | 1 => 0,
                    2 => 1,
                    _ => r.next(),
                },
                tail: r.next(),
            }
        })
        .collect();
    Roster {
        rows,
        count: count as u32,
        pending: match r.below(4) {
            0 | 1 => 0,
            2 => 1,
            _ => r.below(6),
        },
        deadline: if r.chance(50) { 0 } else { r.next() | 1 },
        leader: match r.below(4) {
            0 => world.active,
            1 => 0,
            _ => pool[r.below(pool.len() as u32) as usize],
        },
    }
}

fn guid_pool(r: &mut Rng) -> Vec<u64> {
    let mut pool = vec![0u64];
    for k in 0..14 {
        pool.push(match k % 3 {
            0 => u64::from(r.next() | 1),
            1 => r.u64() | 1 << 40,
            _ => u64::from(r.below(64) + 1),
        });
    }
    pool.sort_unstable();
    pool.dedup();
    pool
}

#[test]
fn raid_rebuild_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let shared = Rc::new(RefCell::new(Shared::default()));
    install_stubs(&mut emu, &shared);
    let rig = rig(&mut emu, &shared);
    let mut r = Rng(0x4ba5_f000);
    let mut cases = 0u32;
    for case in 0..6000u32 {
        let pool = guid_pool(&mut r);
        let world = random_world(&mut r, &pool);
        let before = random_roster(&mut r, &pool, &world);
        let n = pick_count(&mut r);
        let guids: Vec<u64> = (0..n)
            .map(|j| {
                let old = before.count as usize;
                if old > 0 && r.chance(55) {
                    before.rows[r.below(old as u32) as usize].guid
                } else if j + 1 == n && r.chance(30) {
                    0
                } else {
                    pool[r.below(pool.len() as u32) as usize]
                }
            })
            .collect();
        let flags: Vec<u32> = (0..n)
            .map(|_| match r.below(4) {
                0 => r.below(8),
                1 => 0x80 | r.below(8),
                _ => r.next(),
            })
            .collect();
        let online: Vec<u8> = (0..n)
            .map(|_| [0u8, 1, 0x80, 0xff][r.below(4) as usize])
            .collect();

        load(&mut emu, &rig, &shared, &before, &world);
        for (j, &g) in guids.iter().enumerate() {
            emu.write(rig.guid_list + 8 * j as u64, &g.to_le_bytes());
            emu.write_u32(rig.flag_list + 4 * j as u64, flags[j]);
        }
        emu.write(rig.online_list, &online);
        emu.call(
            REBUILD,
            rig.guid_list as u32,
            rig.flag_list as u32,
            &[rig.online_list as u32, n as u32],
        );
        let got = read_back(&emu, &rig, before.leader);
        let got_log = std::mem::take(&mut shared.borrow_mut().log);

        let mut want = before.clone();
        let mut model = Model {
            world: &world,
            log: Vec::new(),
        };
        rebuild(&mut want, &guids, &flags, &online, &mut model);
        let ctx = format!("case {case}: old {} new {n}", before.count);
        assert_eq!(got_log, model.log, "{ctx}: calls");
        assert_eq!(got.count, want.count, "{ctx}: count");
        assert_eq!(got.pending, want.pending, "{ctx}: pending");
        assert_eq!(got.deadline, want.deadline, "{ctx}: deadline");
        for i in 0..ROWS {
            assert_eq!(got.rows[i], want.rows[i], "{ctx}: row {i}");
        }
        cases += 1;
    }
    eprintln!("raid_rebuild: {cases} cases bit-exact vs WoW.exe @ 0x4ba5f0");
}

#[test]
fn raid_pending_names_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let shared = Rc::new(RefCell::new(Shared::default()));
    install_stubs(&mut emu, &shared);
    let rig = rig(&mut emu, &shared);
    let mut r = Rng(0x4bad_9000);
    let mut cases = 0u32;
    // The answer: every pending count regime, the wrap at 0 included.
    for pending in [0u32, 1, 2, 3, 40, 0x7fff_ffff, 0x8000_0000, u32::MAX] {
        let pool = guid_pool(&mut r);
        let world = random_world(&mut r, &pool);
        let mut before = random_roster(&mut r, &pool, &world);
        before.pending = pending;
        load(&mut emu, &rig, &shared, &before, &world);
        emu.call(NAME_ANSWER, 0, 0, &[r.next(), r.next()]);
        let got = read_back(&emu, &rig, before.leader);
        let got_log = std::mem::take(&mut shared.borrow_mut().log);
        let mut want = before.clone();
        let mut model = Model {
            world: &world,
            log: Vec::new(),
        };
        on_roster_name(&mut want, &mut model);
        assert_eq!(got_log, model.log, "pending {pending}: calls");
        assert_eq!(got, want, "pending {pending}: roster");
        cases += 1;
    }
    // The own-row fill: zero GUIDs below, at and past the count, or none.
    for case in 0..3000u32 {
        let pool = guid_pool(&mut r);
        let world = random_world(&mut r, &pool);
        let mut before = random_roster(&mut r, &pool, &world);
        for row in before.rows.iter_mut() {
            if row.guid == 0 && r.chance(50) {
                row.guid = 1;
            }
        }
        for _ in 0..r.below(3) {
            let i = r.below(ROWS as u32) as usize;
            before.rows[i].guid = 0;
        }
        load(&mut emu, &rig, &shared, &before, &world);
        emu.call(FILL_OWN_ROW, 0, 0, &[]);
        let got = read_back(&emu, &rig, before.leader);
        let got_log = std::mem::take(&mut shared.borrow_mut().log);
        let mut want = before.clone();
        let mut model = Model {
            world: &world,
            log: Vec::new(),
        };
        fill_own_row(&mut want, &mut model);
        assert_eq!(got_log, model.log, "case {case}: calls");
        assert_eq!(got, want, "case {case}: roster");
        cases += 1;
    }
    eprintln!("raid_pending_names: {cases} cases bit-exact vs WoW.exe @ 0x4ba1b0, 0x4ba140");
}
