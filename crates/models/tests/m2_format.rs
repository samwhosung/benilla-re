//! The M2 header fixup of `systems/models/shared.md#header-fixup` (`0x71cdf0`) and the particle
//! records of `systems/models/emission.md#particle-record`, checked over the install's `.m2`
//! files: every one passes the fixup's checks, and every emitter's gradient midpoint lies inside
//! (0, 1).

mod common;

use benilla_re_difftest::data_dir;
use common::{list_files, mount};

/// The header's length.
const M2_HEADER_SIZE: usize = 0x154;

/// The arrays `0x71cdf0` checks inline: count, offset, element size and name.
const M2_DESCS: &[(usize, usize, usize, &str)] = &[
    (0x08, 0x0c, 1, "name"),
    (0x14, 0x18, 4, "globalSequences"),
    (0x1c, 0x20, 68, "animations"),
    (0x24, 0x28, 2, "animationLookup"),
    (0x2c, 0x30, 4, "playableAnimationLookup"),
    (0x34, 0x38, 108, "bones"),
];

/// The count of every top-level array, the inline ones and the 26 its helpers check, each helper
/// first testing `offset <= size` (`0x71e160`, `0x71e210`, `0x71e440`).
const M2_ARRAY_DESCS: &[usize] = &[
    0x08, 0x14, 0x1c, 0x24, 0x2c, 0x34, 0x3c, 0x44, 0x4c, 0x54, 0x5c, 0x64, 0x6c, 0x74, 0x7c, 0x84,
    0x8c, 0x94, 0x9c, 0xa4, 0xac, 0xec, 0xf4, 0xfc, 0x104, 0x10c, 0x114, 0x11c, 0x124, 0x12c,
    0x134, 0x13c,
];
/// The array version 0x101 adds (`0x71d284`).
const M2_DESC_V257: usize = 0x144;

fn rd_u32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// `0x71cdf0`'s checks on a file: `Ok(version)` when it passes, else why it fails.
fn m2_accept(b: &[u8]) -> Result<u32, String> {
    let size = b.len();
    if size < M2_HEADER_SIZE {
        return Err(format!("buffer {size} < header 0x154"));
    }
    if &b[0..4] != b"MD20" {
        return Err(format!("bad magic {:08x}", rd_u32(b, 0)));
    }
    let ver = rd_u32(b, 4);
    if ver != 256 && ver != 257 {
        return Err(format!("version {ver} not in {{256,257}}"));
    }
    for &(cf, of, elem, name) in M2_DESCS {
        let off = rd_u32(b, of) as usize;
        let cnt = rd_u32(b, cf) as usize;
        if off > size {
            return Err(format!("{name}: offset {off} > size {size}"));
        }
        // offset + count·elem <= size, without overflow
        let span = cnt.checked_mul(elem).and_then(|s| s.checked_add(off));
        match span {
            Some(end) if end <= size => {}
            _ => return Err(format!("{name}: offset {off} + {cnt}*{elem} > size {size}")),
        }
    }
    // every array's offset within the file, each helper's first test
    for &base in M2_ARRAY_DESCS {
        let off = rd_u32(b, base + 4) as usize;
        if off > size {
            return Err(format!("array desc @{base:#x}: offset {off} > size {size}"));
        }
    }
    if ver == 257 {
        let off = rd_u32(b, M2_DESC_V257 + 4) as usize;
        if off > size {
            return Err(format!(
                "v257 array @{M2_DESC_V257:#x}: offset {off} > size {size}"
            ));
        }
    }
    Ok(ver)
}

/// Every `.m2` of the install passes the fixup, version 0x100 or 0x101.
#[test]
fn m2_corpus_satisfies_accept_contract() {
    let Some(dir) = data_dir() else {
        return;
    };
    let mut chain = mount(&dir, "model.MPQ");
    let m2_paths = list_files(&mut chain, ".m2");
    assert!(!m2_paths.is_empty(), "expected .m2 files in model.MPQ");

    let mut accepted = 0usize;
    let (mut v256, mut v257) = (0usize, 0usize);
    let mut empty = 0usize;
    let mut rejected: Vec<String> = Vec::new();
    for p in &m2_paths {
        let bytes = chain
            .read_file(p)
            .unwrap_or_else(|e| panic!("read {p}: {e}"));
        if bytes.is_empty() {
            empty += 1;
            continue;
        }
        match m2_accept(&bytes) {
            Ok(256) => {
                v256 += 1;
                accepted += 1;
            }
            Ok(257) => {
                v257 += 1;
                accepted += 1;
            }
            Ok(_) => accepted += 1,
            Err(why) => rejected.push(format!("{p}: {why} (len={})", bytes.len())),
        }
    }
    assert_eq!(empty, 0, "a listed .m2 reads back empty");
    assert!(
        rejected.is_empty(),
        "{} .m2 files fail the fixup: {:?}",
        rejected.len(),
        &rejected[..rejected.len().min(25)]
    );
    assert!(
        accepted > 1000 && v256 + v257 == accepted,
        "expected a large corpus of versions 0x100 and 0x101, got {accepted} ({v256} + {v257})"
    );
}

/// Every emitter's midpoint `+0x14c` lies inside (0, 1), so no gradient span is empty; the falling
/// cell ranges and the cells past the grid, read from the same records, show the reads land.
///
/// The particle array is `+0x13c` (count) and `+0x140` (offset), read at `0x710099` and `0x70f9a0`,
/// the record 0x1f8 bytes (`0x7100a0`); `+0x14c` the midpoint (`0x70fd71`, `0x70fd88`); the head
/// cells `+0x168`, `+0x16a`, `+0x16e`, `+0x170` and the tail cells `+0x174` to `+0x17a`
/// (`0x70fe7f` to `0x70fed0`); rows `+0x30` and columns `+0x32` (`0x70fd2e`); the type `+0x2c`.
#[test]
fn m2_corpus_particle_cell_ramp_population() {
    let Some(dir) = data_dir() else {
        return;
    };
    let mut chain = mount(&dir, "model.MPQ");
    let m2_paths = list_files(&mut chain, ".m2");
    let rd_u16 = |b: &[u8], at: usize| u16::from_le_bytes(b[at..at + 2].try_into().unwrap());

    const STRIDE: usize = 0x1f8;
    let mut emitters = 0usize;
    let mut mid_non_interior: Vec<String> = Vec::new();
    let mut inverted_emitters = 0usize;
    let mut past_atlas_emitters = 0usize;
    for p in &m2_paths {
        let Ok(b) = chain.read_file(p) else { continue };
        if b.is_empty() || m2_accept(&b).is_err() {
            continue;
        }
        let cnt = rd_u32(&b, 0x13c) as usize;
        let off = rd_u32(&b, 0x140) as usize;
        // the fixup checks `offset <= size` here; the whole array is bounded before it is read
        match cnt.checked_mul(STRIDE).and_then(|s| s.checked_add(off)) {
            Some(end) if end <= b.len() => {}
            _ => continue,
        }
        for i in 0..cnt {
            let r = off + i * STRIDE;
            emitters += 1;
            let mid = f32::from_bits(rd_u32(&b, r + 0x14c));
            if !mid.is_finite() || mid <= 0.0 || mid >= 1.0 {
                mid_non_interior.push(format!("{p} emitter[{i}] mid={mid}"));
            }
            let pairs = [
                (rd_u16(&b, r + 0x168), rd_u16(&b, r + 0x16a)),
                (rd_u16(&b, r + 0x16e), rd_u16(&b, r + 0x170)),
                (rd_u16(&b, r + 0x174), rd_u16(&b, r + 0x176)),
                (rd_u16(&b, r + 0x178), rd_u16(&b, r + 0x17a)),
            ];
            if pairs.iter().any(|&(s, t)| t < s) {
                inverted_emitters += 1;
            }
            let cells = u32::from(rd_u16(&b, r + 0x30)) * u32::from(rd_u16(&b, r + 0x32));
            if cells > 0 && pairs.iter().any(|&(s, t)| u32::from(s.max(t)) >= cells) {
                past_atlas_emitters += 1;
            }
        }
    }
    assert!(emitters > 1000, "expected many emitters, got {emitters}");
    assert!(inverted_emitters > 0, "no falling cell range found");
    assert!(past_atlas_emitters > 0, "no cell past the grid found");
    assert!(
        mid_non_interior.is_empty(),
        "{} emitters have a midpoint outside (0, 1): {:?}",
        mid_non_interior.len(),
        &mid_non_interior[..mid_non_interior.len().min(10)]
    );
}
