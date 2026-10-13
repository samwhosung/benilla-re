//! The WMO chunks of `systems/models/wmo.md#root-file-chunks` and
//! `systems/models/wmo.md#group-file-chunks`, and the material textures of
//! `systems/models/wmo.md#material-textures`, checked over the install's `.wmo` files.

mod common;

use benilla_re_difftest::data_dir;
use common::{list_files, mount};

const MVER: u32 = 0x4d56_4552; // "MVER" (reversed 4CC, read LE)
const MOHD: u32 = 0x4d4f_4844; // "MOHD" — root header
const MOGP: u32 = 0x4d4f_4750; // "MOGP" — group header (container)
const MOMT: u32 = 0x4d4f_4d54; // "MOMT" — materials, 64 B each
const MOGI: u32 = 0x4d4f_4749; // "MOGI" — group info, 32 B each
const MOTX: u32 = 0x4d4f_5458; // "MOTX" — texture-name string block

fn rd(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// The first top-level chunk `tag`: where its data starts and its size.
fn find_chunk(b: &[u8], tag: u32) -> Option<(usize, usize)> {
    let len = b.len();
    let mut pos = 0usize;
    while pos + 8 <= len {
        let t = rd(b, pos);
        if !t.to_le_bytes().iter().all(|c| c.is_ascii_uppercase()) {
            return None;
        }
        let size = rd(b, pos + 4) as usize;
        if t == tag {
            return Some((pos + 8, size));
        }
        pos += 8 + size;
    }
    None
}

#[derive(PartialEq, Debug)]
enum Kind {
    Root,
    Group,
}

/// The chunk walk of `0x6c3a60`: tags while an 8-byte header fits, each four capital letters, the
/// last chunk not required to end with the file; `MVER` first with size 4 and version 17, then `MOHD` (a
/// root) or `MOGP` (a group); `MOMT` a multiple of 64 bytes (`shr 6`) and `MOGI` of 32 (`shr 5`).
fn validate_wmo(b: &[u8]) -> Result<Kind, String> {
    let len = b.len();
    let mut chunks: Vec<(u32, usize, usize)> = Vec::new(); // (tag, size, data_pos)
    let mut pos = 0usize;
    while pos + 8 <= len {
        let tag = rd(b, pos);
        if !tag.to_le_bytes().iter().all(|c| c.is_ascii_uppercase()) {
            return Err(format!(
                "chunk tag {tag:08x} @ {pos} is not a 4CC (desync/corrupt)"
            ));
        }
        let size = rd(b, pos + 4) as usize;
        chunks.push((tag, size, pos + 8));
        pos += 8 + size;
    }
    if chunks.is_empty() {
        return Err("no chunks".into());
    }
    let (t0, s0, d0) = chunks[0];
    if t0 != MVER {
        return Err(format!("first chunk {t0:08x} != MVER"));
    }
    if s0 != 4 {
        return Err(format!("MVER size {s0} != 4"));
    }
    if d0 + 4 > b.len() {
        return Err("MVER version field past EOF".into());
    }
    let ver = rd(b, d0);
    if ver != 17 {
        return Err(format!("MVER version {ver} != 17"));
    }
    let kind = match chunks.get(1).map(|c| c.0) {
        Some(MOHD) => Kind::Root,
        Some(MOGP) => Kind::Group,
        Some(other) => return Err(format!("second chunk {other:08x} is neither MOHD nor MOGP")),
        None => return Err("only MVER, no header chunk".into()),
    };
    for &(tag, size, _) in &chunks {
        if tag == MOMT && size % 64 != 0 {
            return Err(format!("MOMT size {size} not a multiple of 64"));
        }
        if tag == MOGI && size % 32 != 0 {
            return Err(format!("MOGI size {size} not a multiple of 32"));
        }
    }
    Ok(kind)
}

/// Every `.wmo` of the install passes the chunk walk, as a root or a group.
#[test]
fn wmo_corpus_satisfies_chunk_contract() {
    let Some(dir) = data_dir() else {
        return;
    };
    let mut chain = mount(&dir, "wmo.MPQ");
    let paths = list_files(&mut chain, ".wmo");
    assert!(!paths.is_empty(), "expected .wmo files in wmo.MPQ");

    let (mut roots, mut groups, mut empty) = (0usize, 0usize, 0usize);
    let mut rejected: Vec<String> = Vec::new();
    for p in &paths {
        let bytes = chain
            .read_file(p)
            .unwrap_or_else(|e| panic!("read {p}: {e}"));
        if bytes.is_empty() {
            empty += 1;
            continue;
        }
        match validate_wmo(&bytes) {
            Ok(Kind::Root) => roots += 1,
            Ok(Kind::Group) => groups += 1,
            Err(why) => rejected.push(format!("{p}: {why} (len={})", bytes.len())),
        }
    }
    assert_eq!(empty, 0, "a listed .wmo reads back empty");
    assert!(
        rejected.is_empty(),
        "{} .wmo files fail the chunk walk: {:?}",
        rejected.len(),
        &rejected[..rejected.len().min(25)]
    );
    assert!(roots + groups > 1000, "expected a large .wmo corpus");
}

/// In every root, each material's two texture offsets (`+0x0c`, `+0x18`, read by `0x6c3db0`) land
/// on the start of a name in `MOTX`, 0 being the empty name.
#[test]
fn wmo_momt_texture_offsets_resolve_into_motx() {
    let Some(dir) = data_dir() else {
        return;
    };
    let mut chain = mount(&dir, "wmo.MPQ");
    let paths = list_files(&mut chain, ".wmo");

    let mut materials = 0usize;
    let mut bad: Vec<String> = Vec::new();
    for p in &paths {
        let Ok(b) = chain.read_file(p) else { continue };
        if b.is_empty() || find_chunk(&b, MOHD).is_none() {
            continue;
        }
        let (Some((momt, momt_sz)), Some((motx, motx_sz))) =
            (find_chunk(&b, MOMT), find_chunk(&b, MOTX))
        else {
            continue;
        };
        for i in 0..(momt_sz / 64) {
            let base = momt + i * 64;
            materials += 1;
            for (field, label) in [(0x0c, "texture1"), (0x18, "texture2")] {
                let off = rd(&b, base + field) as usize;
                if off >= motx_sz {
                    bad.push(format!(
                        "{p} material {i} {label}: offset {off} >= MOTX {motx_sz}"
                    ));
                } else if off != 0 && b[motx + off - 1] != 0 {
                    bad.push(format!(
                        "{p} material {i} {label}: offset {off} not a string start"
                    ));
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} texture offsets miss MOTX: {:?}",
        bad.len(),
        &bad[..bad.len().min(20)]
    );
    assert!(materials > 1000, "expected many WMO materials");
}
