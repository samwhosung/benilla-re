//! The index strips: the liquid strip, the hole-masked chunk strip and the low-detail tile's
//! indices (`systems/terrain/liquid.md#liquid-strips`, `systems/terrain/lod.md#low-detail-mesh`).

/// `CChunkLiquid::BuildStrip` (`0x68d9b0`): over the 8×8 cell flags (row stride 8), a restartable
/// strip through every cell whose low nibble is `liquid_type`: a run opens with `(i, i, i+9)`, each
/// cell adds `(i+1, i+10)`, and a run closes by repeating its last index. Vertices are 9 to a row.
#[allow(clippy::explicit_counter_loop)] // the counters are the client's cursors
pub fn liquid_strip_indices(cell_flags: &[u8; 64], liquid_type: u8) -> Vec<u16> {
    let mut out = Vec::new();
    let mut running = false;
    let mut last = 0i32;
    for row in 0..8i32 {
        let mut i = row * 9;
        let mut below = i + 9;
        let mut below1 = i + 10;
        for col in 0..8i32 {
            if cell_flags[(row * 8 + col) as usize] & 0xf == liquid_type {
                if !running {
                    out.push(i as u16);
                    out.push(i as u16);
                    out.push(below as u16);
                    running = true;
                }
                out.push((i + 1) as u16);
                out.push(below1 as u16);
                last = below1;
            } else if running {
                out.push(last as u16);
                running = false;
            }
            i += 1;
            below += 1;
            below1 += 1;
        }
        if running {
            out.push(last as u16);
            running = false;
        }
    }
    out
}

/// `BuildLiquidCellStrip` (`0x6b61a0`): the same strip over a `rows`×`cols` cell map, vertex rows
/// `stride` apart from vertex `base`. A cell takes part when its low nibble is not 0xf and its
/// 0x80 bit is clear or `include_flagged` is set. The client adds the count to an in-out.
#[allow(clippy::explicit_counter_loop)] // the counters are the client's cursors
pub fn hole_strip_indices(
    cells: &[u8],
    stride: u16,
    cols: i32,
    rows: i32,
    base: i32,
    include_flagged: bool,
) -> Vec<u16> {
    let mut out = Vec::new();
    let mut running = false;
    let mut last = 0i32;
    let mut c = 0usize;
    for row in 0..rows {
        let mut i = i32::from(stride) * row + base;
        let mut below = i32::from(stride) + i;
        let mut below1 = below;
        for _ in 0..cols {
            below1 += 1;
            let cell = cells[c];
            let participates = (cell & 0xf) != 0xf && ((cell & 0x80) == 0 || include_flagged);
            if participates {
                if !running {
                    out.push(i as u16);
                    out.push(i as u16);
                    out.push(below as u16);
                    running = true;
                }
                out.push((i + 1) as u16);
                out.push(below1 as u16);
                last = below1;
            } else if running {
                out.push(last as u16);
                running = false;
            }
            i += 1;
            below += 1;
            c += 1;
        }
        if running {
            out.push(last as u16);
            running = false;
        }
    }
    out
}

/// `FillLowDetailIndices` (`0x6bd9b0`): four triangles per quad of the 16×16 tile, each around its
/// centre vertex (from 0x121) over the 17×17 outer grid, 12 indices a quad in the client's order
/// with `i16` arithmetic.
#[allow(clippy::explicit_counter_loop)] // the counters are the client's cursors
pub fn wdl_strip_indices() -> [u16; 16 * 16 * 12] {
    let mut out = [0u16; 16 * 16 * 12];
    let mut center: i16 = 0x121;
    let mut tl: i16 = 0; // the quad's top-left outer vertex
    let mut k = 0usize;
    for _ in 0..16 {
        let mut br: i16 = tl + 0x12; // its bottom-right
        for _ in 0..16 {
            let tl0 = tl;
            out[k] = center as u16;
            out[k + 1] = (br - 0x11) as u16;
            out[k + 2] = tl0 as u16;
            out[k + 3] = center as u16;
            out[k + 4] = br as u16;
            out[k + 5] = (br - 0x11) as u16;
            out[k + 6] = center as u16;
            out[k + 7] = (br - 1) as u16;
            out[k + 8] = br as u16;
            out[k + 9] = center as u16;
            out[k + 10] = tl0 as u16;
            out[k + 11] = (br - 1) as u16;
            k += 12;
            center += 1;
            br += 1;
            tl = tl0 + 1;
        }
        tl += 1; // a row ends two past its last quad's top-left
    }
    out
}
