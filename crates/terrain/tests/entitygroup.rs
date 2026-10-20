//! What an entity's map object group says about it, each run whole against the exe:
//! `CWorld::IsEntityOutdoors` (`0x6706f0`) with `FindMapObjectGroupAt` (`0x69d4f0`) and
//! `CMapObj::GetLoadedGroup` (`0x6a4a00`) unstubbed, so the walk of the attachment list, the group
//! lookup and the flag read are the exe's own; and `SetEntityLightFlags` (`0x6a87f0`) with
//! `0x6a4a00` unstubbed. The structures are built in the scratch arena at the offsets the code
//! reads.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_terrain::entitygroup::{
    claim_group_light, is_entity_outdoors, Attachment, GROUP_EXTERIOR, GROUP_EXTERIOR_LIT,
    GROUP_OUTDOORS,
};
use common::next;

const IS_OUTDOORS: u64 = 0x6706f0;
const CLAIM: u64 = 0x6a87f0;

/// Most records one case attaches.
const MAX_ATTACH: usize = 4;

/// What one attachment record is built from.
#[derive(Clone, Copy, Debug)]
enum Spec {
    /// A target whose tag has bit 0x10 clear; the tag is `tag`.
    Other { tag: u32 },
    /// A group node (tag `0x10 | extra`) with the key `key`, behind a map object whose loaded
    /// byte is `root` and a group whose parsed byte is `parsed` and whose flags are `flags`.
    Group {
        extra: u32,
        key: u32,
        root: u8,
        parsed: u8,
        flags: u32,
    },
}

impl Spec {
    fn attachment(self) -> Attachment {
        match self {
            Spec::Other { .. } => Attachment::NotGroupNode,
            Spec::Group { root, parsed, .. } if root == 0 || parsed == 0 => {
                Attachment::GroupNotLoaded
            }
            Spec::Group { flags, .. } => Attachment::Group { flags },
        }
    }
}

/// The blocks a case is built in, reused for every case.
struct Rig {
    entity: u64,
    record: [u64; MAX_ATTACH],
    node: [u64; MAX_ATTACH],
    member: [u64; MAX_ATTACH],
    object: [u64; MAX_ATTACH],
    model: [u64; MAX_ATTACH],
    group: [u64; MAX_ATTACH],
}

const ENTITY_SIZE: usize = 0x100;
const RECORD_SIZE: usize = 0x40;
const NODE_SIZE: usize = 0x90;
const MEMBER_SIZE: usize = 0x20;
const OBJECT_SIZE: usize = 0x130;
const MODEL_SIZE: usize = 0x400;
const GROUP_SIZE: usize = 0x170;

impl Rig {
    fn new(e: &mut Emu) -> Rig {
        let mut r = Rig {
            entity: e.alloc(ENTITY_SIZE as u64),
            record: [0; MAX_ATTACH],
            node: [0; MAX_ATTACH],
            member: [0; MAX_ATTACH],
            object: [0; MAX_ATTACH],
            model: [0; MAX_ATTACH],
            group: [0; MAX_ATTACH],
        };
        for i in 0..MAX_ATTACH {
            r.record[i] = e.alloc(RECORD_SIZE as u64);
            r.node[i] = e.alloc(NODE_SIZE as u64);
            r.member[i] = e.alloc(MEMBER_SIZE as u64);
            r.object[i] = e.alloc(OBJECT_SIZE as u64);
            r.model[i] = e.alloc(MODEL_SIZE as u64);
            r.group[i] = e.alloc(GROUP_SIZE as u64);
        }
        r
    }

    /// Clears every block, then builds the entity: `+0x18` the link offset of its records, `+0x20`
    /// the list head `head`, `+0x90` its flags; the records chained from `head` ending in
    /// `terminator`, each pointing at its target.
    fn build(
        &self,
        e: &mut Emu,
        link: u32,
        flags: u32,
        head: Option<u32>,
        terminator: u32,
        specs: &[Spec],
    ) {
        e.write(self.entity, &[0; ENTITY_SIZE]);
        for i in 0..MAX_ATTACH {
            e.write(self.record[i], &[0; RECORD_SIZE]);
            e.write(self.node[i], &[0; NODE_SIZE]);
            e.write(self.member[i], &[0; MEMBER_SIZE]);
            e.write(self.object[i], &[0; OBJECT_SIZE]);
            e.write(self.model[i], &[0; MODEL_SIZE]);
            e.write(self.group[i], &[0; GROUP_SIZE]);
        }
        e.write_u32(self.entity + 0x18, link);
        e.write_u32(self.entity + 0x90, flags);
        e.write_u32(
            self.entity + 0x20,
            head.unwrap_or(if specs.is_empty() {
                terminator
            } else {
                self.record[0] as u32
            }),
        );
        for (i, spec) in specs.iter().enumerate() {
            let next_link = if i + 1 < specs.len() {
                self.record[i + 1] as u32
            } else {
                terminator
            };
            e.write_u32(self.record[i] + u64::from(link) + 4, next_link);
            e.write_u32(self.record[i] + 8, self.node[i] as u32);
            match *spec {
                Spec::Other { tag } => e.write_u32(self.node[i] + 8, tag),
                Spec::Group {
                    extra,
                    key,
                    root,
                    parsed,
                    flags,
                } => {
                    e.write_u32(self.node[i] + 8, 0x10 | extra);
                    e.write_u32(self.node[i] + 0x20, self.member[i] as u32);
                    e.write_u32(self.node[i] + 0x7c, key);
                    e.write_u32(self.member[i] + 8, self.object[i] as u32);
                    e.write_u32(self.object[i] + 0x118, self.model[i] as u32);
                    e.write(self.model[i] + 0x1d4, &[root]);
                    e.write_u32(
                        self.model[i] + 0x1f4 + 4 * u64::from(key),
                        self.group[i] as u32,
                    );
                    e.write(self.group[i] + 0x164, &[parsed]);
                    e.write_u32(self.group[i] + 0x10, flags);
                }
            }
        }
    }
}

/// The answer of the exe and of the transcription for one built case.
fn check_outdoors(e: &mut Emu, r: &Rig, null: bool, flags: u32, specs: &[Spec], ctx: &str) {
    let ecx = if null { 0 } else { r.entity as u32 };
    let bin = e.call(IS_OUTDOORS, ecx, 0, &[]) & 0xff;
    let atts: Vec<Attachment> = specs.iter().map(|s| s.attachment()).collect();
    let ours = is_entity_outdoors((!null).then_some((flags, atts.as_slice())));
    assert_eq!(
        bin,
        u32::from(ours),
        "{ctx}: flags {flags:#x}, null {null}, specs {specs:?}"
    );
}

/// One spec from a number: the four kinds the walk tells apart.
fn spec_of(kind: u32, flags: u32, key: u32, s: &mut u32) -> Spec {
    match kind {
        0 => Spec::Other {
            tag: next(s) & !0x10,
        },
        1 => Spec::Group {
            extra: next(s) & !0x10,
            key,
            root: 0,
            parsed: 1,
            flags,
        },
        2 => Spec::Group {
            extra: next(s) & !0x10,
            key,
            root: 1,
            parsed: 0,
            flags,
        },
        _ => Spec::Group {
            extra: next(s) & !0x10,
            key,
            root: [1, 2, 0x80, 0xff][(next(s) >> 8) as usize % 4],
            parsed: [1, 2, 0x80, 0xff][(next(s) >> 8) as usize % 4],
            flags,
        },
    }
}

#[test]
fn entity_outdoors_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rig = Rig::new(&mut e);
    let mut s = 0x6706_f000u32;
    let mut cases = 0u32;

    // A null entity answers 1 whatever the registers hold.
    rig.build(&mut e, 0, 0, None, 1, &[]);
    check_outdoors(&mut e, &rig, true, 0, &[], "null");
    cases += 1;

    // Every list of up to four records over the four kinds, the bit-15 value of each group
    // record (loaded or not) taking every combination, with the entity flag bit 0 clear and set
    // and three link offsets. A record the walk passes over has bit 15 set or clear at random,
    // so a read of it answers wrongly.
    for link in [0u32, 0x10, 0x1c] {
        for len in 0..=MAX_ATTACH {
            for code in 0..4u32.pow(len as u32) {
                let kinds: Vec<u32> = (0..len).map(|i| (code >> (2 * i)) & 3).collect();
                let groups = kinds.iter().filter(|&&k| k != 0).count() as u32;
                for bits in 0..(1u32 << groups) {
                    let mut g = 0;
                    let specs: Vec<Spec> = kinds
                        .iter()
                        .enumerate()
                        .map(|(i, &k)| {
                            let flags = if k != 0 && (bits >> g) & 1 != 0 {
                                next(&mut s) | GROUP_OUTDOORS
                            } else {
                                next(&mut s) & !GROUP_OUTDOORS
                            };
                            if k != 0 {
                                g += 1;
                            }
                            spec_of(k, flags, i as u32 + 1, &mut s)
                        })
                        .collect();
                    for flag_bit0 in [0u32, 1] {
                        let flags = (next(&mut s) & !1) | flag_bit0;
                        let term = [1u32, 0, 0x1001][(next(&mut s) >> 8) as usize % 3];
                        rig.build(&mut e, link, flags, None, term, &specs);
                        check_outdoors(&mut e, &rig, false, flags, &specs, "lists");
                        cases += 1;
                    }
                }
            }
        }
    }

    // An empty list through each head the walk ends on: 0, and odd values.
    for head in [0u32, 1, 3, 0xffff_ffff] {
        for flags in [1u32, 0xffff_ffff, 0x101] {
            rig.build(&mut e, 0x10, flags, Some(head), 1, &[]);
            check_outdoors(&mut e, &rig, false, flags, &[], "heads");
            cases += 1;
        }
    }

    // One group with the flags a single bit, every bit but one, none and all: only bit 15
    // decides, with the indoors bit set, whatever the exterior bits say.
    let mut flag_sets: Vec<u32> = vec![0, u32::MAX];
    for b in 0..32 {
        flag_sets.push(1 << b);
        flag_sets.push(!(1u32 << b));
    }
    for gf in flag_sets {
        for ef in [1u32, 0x2001, 0xffff_ffff, 0xffff_fffe, 0] {
            let spec = Spec::Group {
                extra: 0,
                key: 0,
                root: 1,
                parsed: 1,
                flags: gf,
            };
            rig.build(&mut e, 0, ef, None, 1, &[spec]);
            check_outdoors(&mut e, &rig, false, ef, &[spec], "flag bits");
            assert_eq!(
                is_entity_outdoors(Some((1, &[Attachment::Group { flags: gf }]))),
                gf & 0x8000 != 0
            );
            cases += 1;
        }
    }

    // Keys across the model's group array.
    for key in [0u32, 1, 2, 7, 0x20, 0x60] {
        for flags in [0u32, GROUP_OUTDOORS] {
            let spec = Spec::Group {
                extra: 0x20,
                key,
                root: 1,
                parsed: 1,
                flags,
            };
            rig.build(&mut e, 0, 1, None, 1, &[spec]);
            check_outdoors(&mut e, &rig, false, 1, &[spec], "keys");
            cases += 1;
        }
    }

    // Random lists.
    for _ in 0..4000 {
        let len = (next(&mut s) >> 8) as usize % (MAX_ATTACH + 1);
        let specs: Vec<Spec> = (0..len)
            .map(|_| {
                let flags = next(&mut s);
                let kind = (next(&mut s) >> 8) % 4;
                spec_of(kind, flags, (next(&mut s) >> 8) % 0x60, &mut s)
            })
            .collect();
        let flags = next(&mut s);
        let link = [0u32, 0x10, 0x1c][(next(&mut s) >> 8) as usize % 3];
        let term = [1u32, 0, 0x1001][(next(&mut s) >> 8) as usize % 3];
        rig.build(&mut e, link, flags, None, term, &specs);
        check_outdoors(&mut e, &rig, false, flags, &specs, "random");
        cases += 1;
    }
    eprintln!("entity_outdoors: {cases} cases bit-exact vs WoW.exe @ {IS_OUTDOORS:#x}");
}

#[test]
fn entity_light_flags_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rig = Rig::new(&mut e);
    let mut s = 0x6a87_f000u32;
    let mut cases = 0u32;

    let mut group_flags: Vec<u32> = vec![
        0,
        u32::MAX,
        GROUP_EXTERIOR,
        GROUP_EXTERIOR_LIT,
        GROUP_EXTERIOR | GROUP_EXTERIOR_LIT,
        GROUP_OUTDOORS,
        0x2000,
        0x48,
    ];
    for b in 0..32 {
        group_flags.push(1 << b);
        group_flags.push(!(1u32 << b));
    }
    for _ in 0..64 {
        group_flags.push(next(&mut s));
    }
    let entity_flags = [0u32, 1, 0xffff_fffe, 0xffff_ffff, 0x2000, 0x10];
    for &gf in &group_flags {
        for &ef in &entity_flags {
            for light in [0u8, 1, 2, 4, 6, 0xf9, 0xff] {
                let key = (next(&mut s) >> 8) % 0x40;
                let spec = Spec::Group {
                    extra: 0,
                    key,
                    root: 1,
                    parsed: 1,
                    flags: gf,
                };
                rig.build(&mut e, 0, ef, None, 1, &[spec]);
                // The word at +0xc and the bytes after it, to show the claim touches one byte.
                let word_c = 0xa5a5_a500 | u32::from(light);
                e.write_u32(rig.entity + 0xc, word_c);
                e.call(
                    CLAIM,
                    rig.entity as u32,
                    rig.object[0] as u32,
                    &[rig.node[0] as u32],
                );
                let (flags90, byte_c) = claim_group_light(ef, light, gf);
                assert_eq!(
                    e.read_u32(rig.entity + 0x90),
                    flags90,
                    "+0x90: group {gf:#x}, entity {ef:#x}"
                );
                assert_eq!(
                    e.read_u32(rig.entity + 0xc),
                    (word_c & !0xff) | u32::from(byte_c),
                    "+0xc: group {gf:#x}, entity {ef:#x}, light {light:#x}"
                );
                cases += 1;
            }
        }
    }
    eprintln!("entity_light_flags: {cases} cases bit-exact vs WoW.exe @ {CLAIM:#x}");
}
