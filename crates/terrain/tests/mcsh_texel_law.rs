//! `IsInChunkShadow` (`0x69b350`, `systems/terrain/queries.md#shadow-test`) addressing the shadow
//! map of the install's own ADT files: the chunk its indices pick is the MCIN slot whose footprint
//! holds the point, and points known to stand in shadow land on set bits. The arithmetic is
//! `tile_index_conversion_difftest`'s; this checks the addressing against real data.

use std::path::Path;

use benilla_re_difftest::data_dir;
use benilla_re_terrain::worldcoord::{
    tile_index_coarse, tile_index_fine, tile_neg_axis, MAP_CENTER_F32, TILE_SCALE_COARSE,
    TILE_SCALE_FINE,
};
use wow_mpq::PatchChain;

/// The MCNK header's offset of the shadow map (`+0x2c`), from the MCNK record's start; the client
/// keeps the pointer at chunk `+0xf1c` (`systems/terrain/chunk.md`).
const HDR_OFS_MCSH: usize = 0x2c;

/// The archives an ADT can come from: the terrain archive, then the patches over it in order.
fn mount(dir: &Path) -> PatchChain {
    let mut chain = PatchChain::new();
    chain
        .add_archive(dir.join("terrain.MPQ"), 0)
        .expect("mount terrain.MPQ");
    let mut patches: Vec<_> = std::fs::read_dir(dir)
        .expect("read Data")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(str::to_ascii_lowercase)
                .is_some_and(|n| n.starts_with("patch") && n.ends_with(".mpq"))
        })
        .collect();
    patches.sort_by_key(|p| p.file_name().map(|n| n.to_ascii_lowercase()));
    for (i, patch) in patches.iter().enumerate() {
        chain
            .add_archive(patch, 100 + i as i32)
            .expect("mount a patch");
    }
    chain
}

fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

fn f32le(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

/// `0x69b350`'s chunk index on one axis.
fn coarse(p: f32) -> i32 {
    tile_index_coarse(tile_neg_axis(p))
}

/// `0x69b350`'s texel index on one axis.
fn fine(p: f32) -> i32 {
    tile_index_fine(tile_neg_axis(p))
}

/// One ADT, per MCIN slot.
struct Tile {
    /// The shadow map, by MCIN slot.
    mcsh: Vec<Vec<u8>>,
    /// The MCNK header's corner, `+0x68` and `+0x6c`, in world x and y.
    corner: Vec<(f32, f32)>,
    /// Each MDDF entry's unique id and world x and y as `PlaceDoodadDef` (`0x694bc0`) sets them:
    /// x = 17066.666 − `+0x10`, y = 17066.666 − `+0x08` (`0x694c8b` to `0x694cbe`).
    doodads: Vec<(u32, f32, f32)>,
}

fn load(chain: &mut PatchChain, col: u32, row: u32) -> Option<Tile> {
    // the file name's first index is the column, from y; the second the row, from x
    let adt = chain
        .read_file(&format!(r"World\Maps\Azeroth\Azeroth_{col}_{row}.adt"))
        .ok()?;
    let mhdr = 8 + u32le(&adt, 4) as usize + 8;
    let mcin = mhdr + u32le(&adt, mhdr + 4) as usize + 8;
    let mut mcsh = vec![Vec::new(); 256];
    let mut corner = vec![(0.0, 0.0); 256];
    for slot in 0..256 {
        let rec = u32le(&adt, mcin + slot * 16) as usize;
        assert_eq!(&adt[rec..rec + 4], b"KNCM", "MCNK magic at slot {slot}");
        let hdr = rec + 8;
        corner[slot] = (f32le(&adt, hdr + 0x68), f32le(&adt, hdr + 0x6c));
        let ofs = u32le(&adt, hdr + HDR_OFS_MCSH) as usize;
        if ofs != 0 && &adt[rec + ofs..rec + ofs + 4] == b"HSCM" {
            let n = u32le(&adt, rec + ofs + 4) as usize;
            mcsh[slot] = adt[rec + ofs + 8..rec + ofs + 8 + n.min(512)].to_vec();
        }
    }
    let mddf = mhdr + u32le(&adt, mhdr + 0x1c) as usize;
    assert_eq!(&adt[mddf..mddf + 4], b"FDDM", "MDDF magic");
    let (base, n) = (mddf + 8, u32le(&adt, mddf + 4) as usize / 36);
    let c = MAP_CENTER_F32;
    let doodads = (0..n)
        .map(|i| {
            let e = base + i * 36;
            (
                u32le(&adt, e + 4),
                c - f32le(&adt, e + 0x10),
                c - f32le(&adt, e + 8),
            )
        })
        .collect();
    Some(Tile {
        mcsh,
        corner,
        doodads,
    })
}

/// `0x69b350` within one tile: the texel's row and column and whether its bit is set; `None` when
/// the chunk is not in the tile.
fn sample(t: &Tile, x: f32, y: f32) -> Option<(i32, i32, bool)> {
    let (cx, cy) = (coarse(x), coarse(y));
    let slot = ((cx & 0xf) << 4) + (cy & 0xf);
    let (row, col) = (fine(x) & 0x3f, fine(y) & 0x3f);
    let payload = t.mcsh.get(slot as usize)?;
    if payload.len() < 512 {
        return Some((row, col, false));
    }
    let byte = payload[row as usize * 8 + col as usize / 8];
    Some((row, col, (byte >> (col % 8)) & 1 == 1))
}

#[test]
fn mcsh_texel_law_against_real_adt() {
    let Some(dir) = data_dir() else {
        return;
    };
    let mut chain = mount(&dir);
    let tile = load(&mut chain, 34, 45).expect("Azeroth_34_45.adt");

    // every chunk of this tile has a shadow map, so no verdict below is lit for want of one
    assert_eq!(
        tile.mcsh.iter().filter(|m| m.len() == 512).count(),
        256,
        "all 256 MCNKs of Azeroth_34_45 carry a 512-byte MCSH payload"
    );

    // The MCIN slot is the client's chunk index (x << 4) + y. The probe is half a chunk inside the
    // corner: on the corner itself 0.03, a little under 1/33.333, puts the point in the chunk before.
    for slot in 0..256usize {
        let (sub_x, sub_y) = (slot / 16, slot % 16);
        let (cx, cy) = tile.corner[slot];
        let (px, py) = (cx - 16.0, cy - 16.0);
        assert_eq!(
            ((coarse(px) & 0xf) as usize, (coarse(py) & 0xf) as usize),
            (sub_x, sub_y),
            "MCIN slot {slot} (corner {cx},{cy}) is not chunk ({sub_x},{sub_y})"
        );
    }

    // points standing in shadow on this tile: (x, y, row, column, byte, bit)
    let vectors = [
        (-7210.91_f32, -1071.78_f32, 20, 9, 161, 1),
        (-7213.78, -1071.14, 26, 8, 209, 0),
        (-7223.22, -1072.16, 44, 10, 353, 2),
        (-7231.83, -1072.14, 61, 10, 489, 2),
        (-6938.98, -1312.63, 10, 24, 83, 0),
    ];
    for (x, y, row, col, byte, bit) in vectors {
        assert_eq!(
            ((coarse(x) >> 4) & 0x3f, (coarse(y) >> 4) & 0x3f),
            (45, 34),
            "({x},{y}) is not in tile row 45 / col 34"
        );
        let (r, c, shadowed) = sample(&tile, x, y).expect("chunk loaded");
        assert_eq!((r, c), (row, col), "texel for ({x},{y})");
        assert_eq!(
            (r as usize * 8 + c as usize / 8, c % 8),
            (byte, bit),
            "byte/bit for ({x},{y})"
        );
        assert!(
            shadowed,
            "({x},{y}) reads shadowed in the real MCSH payload"
        );
    }

    // 1.92 is exactly 64 times 0.03 as floats, so a texel index refines its chunk index
    assert_eq!(
        f64::from(TILE_SCALE_FINE),
        f64::from(TILE_SCALE_COARSE) * 64.0
    );
}

/// A doodad near a tile's edge is listed in both tiles' MDDF, which is why the shadow lookup goes
/// through the global tile table (`0xc96318`) and `PlaceDoodadDef` (`0x694bc0`) finds an existing
/// def by its unique id (`0x694bd4`): the texel under it is only in the tile it stands in.
#[test]
fn border_doodads_are_duplicated_across_tiles() {
    let Some(dir) = data_dir() else {
        return;
    };
    let mut chain = mount(&dir);
    let owner = load(&mut chain, 34, 45).expect("Azeroth_34_45.adt");
    let west = load(&mut chain, 33, 45).expect("Azeroth_33_45.adt");

    // id 76967 stands at (-7223.22, -1072.16), in tile column 34, and is listed in column 33 too
    let in_west = west
        .doodads
        .iter()
        .find(|d| d.0 == 76967)
        .expect("in 33_45");
    let in_owner = owner
        .doodads
        .iter()
        .find(|d| d.0 == 76967)
        .expect("in 34_45");
    assert_eq!(
        (in_west.1, in_west.2),
        (in_owner.1, in_owner.2),
        "the duplicate carries the same placement origin"
    );
    assert_eq!(
        (coarse(in_west.1) >> 4) & 0x3f,
        45,
        "the doodad's own tile row"
    );
    assert_eq!(
        (coarse(in_west.2) >> 4) & 0x3f,
        34,
        "it stands in tile col 34, not the col-33 list that also holds it"
    );
    assert!(sample(&owner, in_west.1, in_west.2).expect("chunk").2);
}
