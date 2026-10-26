//! Six record readers run whole in the emulator against `read_record`: `CharHairGeosetsRec::Read`
//! (`0x5753b0`), `CharSectionsRec::Read` (`0x575540`), `CharacterFacialHairStylesRec::Read`
//! (`0x575a80`), `SpellRec::Read` (`0x583750`), `WeaponImpactSoundsRec::Read` (`0x587450`) and
//! `LightParamsRec::Read` (`0x589030`).
//!
//! `SFile::Read` (`0x648460`) is a stub over a list of reads: each call takes the next entry,
//! writes its bytes at the destination and returns 1, or writes nothing and returns 0 for a failed
//! one; it checks the file handle, the size the layout gives and the three zero arguments.
//! `ConsoleWrite` (`0x63cb50`) is a stub that keeps the message it is given. The record is filled
//! with a sentinel first, so a column the reader does not write shows. The cases are every column
//! failing alone, a null string block, offsets that wrap the base, and a fuzz.

use benilla_re_dbc::record::{
    read_record, record_size, Column, CHARACTER_FACIAL_HAIR_STYLES, CHAR_HAIR_GEOSETS,
    CHAR_SECTIONS, LIGHT_PARAMS, SPELL, WEAPON_IMPACT_SOUNDS,
};
use benilla_re_difftest::{Emu, RegisterX86, Wow};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

const SFILE_READ: u64 = 0x648460;
const CONSOLE_WRITE: u64 = 0x63cb50;
const FILE: u32 = 0x5f11_e000;
const SENTINEL: u8 = 0xa5;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s >> 8
}

fn run(reader: u64, name: &str, layout: &[Column]) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let queue: Rc<RefCell<VecDeque<Option<Vec<u8>>>>> = Rc::default();
    let logged: Rc<RefCell<Vec<Vec<u8>>>> = Rc::default();
    let sizes: Rc<Vec<usize>> = Rc::new(layout.iter().map(|&(s, _)| s).collect());
    {
        let (queue, sizes) = (queue.clone(), sizes.clone());
        let mut call = 0usize;
        emu.stub_fn(SFILE_READ, 6, move |io| {
            assert_eq!(io.arg(0), FILE, "file");
            assert_eq!(
                io.arg(2) as usize,
                sizes[call % sizes.len()],
                "size of read {call}"
            );
            assert_eq!((io.arg(3), io.arg(4), io.arg(5)), (0, 0, 0));
            call += 1;
            match queue
                .borrow_mut()
                .pop_front()
                .expect("a read past the layout")
            {
                Some(bytes) => {
                    io.write(u64::from(io.arg(1)), &bytes);
                    1
                }
                None => 0,
            }
        });
    }
    {
        let logged = logged.clone();
        emu.stub_fn(CONSOLE_WRITE, 0, move |io| {
            assert_eq!(io.reg(RegisterX86::EDX), 0, "colour class");
            let text = io.read_cstr(io.reg(RegisterX86::ECX));
            logged.borrow_mut().push(text);
            0
        });
    }
    let size = record_size(layout);
    let rec = emu.alloc(size as u64);
    let error = format!("Error reading {name}Rec").into_bytes();

    let check = |emu: &mut Emu, reads: Vec<Option<Vec<u8>>>, base: u32| {
        emu.write(rec, &vec![SENTINEL; size]);
        *queue.borrow_mut() = reads.iter().cloned().collect();
        logged.borrow_mut().clear();
        let got = emu.call(reader, rec as u32, 0, &[FILE, base]) & 0xff;
        let mut want_rec = vec![SENTINEL; size];
        let slices: Vec<Option<&[u8]>> = reads.iter().map(|r| r.as_deref()).collect();
        let want = read_record(layout, &slices, base, &mut want_rec);
        assert!(queue.borrow().is_empty(), "{name}: every column read");
        assert_eq!(got, u32::from(want), "{name}: result, base {base:#x}");
        assert_eq!(
            emu.read(rec, size),
            want_rec,
            "{name}: record, base {base:#x}"
        );
        let log = logged.borrow();
        if want {
            assert!(log.is_empty(), "{name}: nothing logged");
        } else {
            assert_eq!(*log, vec![error.clone()], "{name}: the error line");
        }
    };

    let mut seed = reader as u32;
    let fill = |seed: &mut u32| -> Vec<Option<Vec<u8>>> {
        layout
            .iter()
            .map(|&(s, _)| Some((0..s).map(|_| next(seed) as u8).collect()))
            .collect()
    };
    for base in [0x1234_0000, 0, 1, 0xffff_fff0] {
        for _ in 0..8 {
            let reads = fill(&mut seed);
            check(&mut emu, reads, base);
        }
        // Every column failing alone, and all of them.
        for fail in 0..layout.len() {
            let mut reads = fill(&mut seed);
            reads[fail] = None;
            check(&mut emu, reads, base);
        }
        check(&mut emu, vec![None; layout.len()], base);
    }
    // Offsets that wrap the base, and a fuzz of bases and failures.
    for _ in 0..200 {
        let mut reads = fill(&mut seed);
        for (r, &(_, string)) in reads.iter_mut().zip(layout) {
            if string && next(&mut seed).is_multiple_of(2) {
                *r = Some(
                    (0xffff_ff00u32 + next(&mut seed) % 0x100)
                        .to_le_bytes()
                        .to_vec(),
                );
            }
            if next(&mut seed).is_multiple_of(64) {
                *r = None;
            }
        }
        let base = match next(&mut seed) % 4 {
            0 => 0,
            _ => next(&mut seed) << 8 | next(&mut seed) & 0xff,
        };
        check(&mut emu, reads, base);
    }
}

#[test]
fn char_hair_geosets_read_difftest() {
    run(0x5753b0, "CharHairGeosets", CHAR_HAIR_GEOSETS);
}

#[test]
fn char_sections_read_difftest() {
    run(0x575540, "CharSections", CHAR_SECTIONS);
}

#[test]
fn character_facial_hair_styles_read_difftest() {
    run(
        0x575a80,
        "CharacterFacialHairStyles",
        CHARACTER_FACIAL_HAIR_STYLES,
    );
}

#[test]
fn spell_read_difftest() {
    run(0x583750, "Spell", SPELL);
}

#[test]
fn weapon_impact_sounds_read_difftest() {
    run(0x587450, "WeaponImpactSounds", WEAPON_IMPACT_SOUNDS);
}

#[test]
fn light_params_read_difftest() {
    run(0x589030, "LightParams", LIGHT_PARAMS);
}
