# terrain: trace

The segment trace: whether a segment from a start to an end point hits the map, and at what
fraction. It walks the cells the segment's ground projection crosses and tests each cell's
triangles, after the map objects have had their turn.

The code is the map trace object file, `0x69c0d0` through `0x69d450`, the dispatcher
`0x69bff0` in the query file, and the terrain-only first-hit trace in the collision gather file.
The cell list the walk fills is the pointer `0xc962c8` with the count `0xc89f40`, pairs of
(column, row) cells, the column from y first. The world's entry is `TraceWorldSegment` (`0x672170`).

Roots: none.

## Segment trace

`TraceWorldSegmentBody` (`0x69bff0`) traces a segment under a mask: map objects when the mask has
a bit of 0xf000ff, then the terrain when it has one of 0xf00f0f, each lowering the fraction t
it is given when it hits. On a hit it writes the hit point start + (end − start)·t and returns
1. The point's x is one rounding of (end.x − start.x)·t, then one of its sum with start.x; y is
one rounding of the whole; z rounds end.z − start.z first.

Addresses: `0x69bff0`, `0x69c0f0`, `0x69c320`, `0xc89f20`, `0x69c00b`, `0x69c03e`, `0x69c06c`, `0x69c08c`, `0x69c0a5`
Confidence: verified
Proof: one
Test: `terrain::segment_lerp_difftest`

`ret 0xc`, start in ecx, end in edx, then the hit pointer, the fraction pointer and the mask.
The query counter `0xc89f20` advances. `0x69c00b` and `0x69c03e` test the masks. From
`0x69c06c` the three differences are taken, z stored; `0x69c08c` stores dx·t, and the sums with
the start are stored at `0x69c09d`, `0x69c0a8` (`0x69c0a5` adds start.y to dy·t) and `0x69c0b6`.
The fraction is read once, after both traces.

`terrain::segment_lerp_difftest` runs `0x69c06c` to `0x69c0bf`.

## Terrain walk

`0x69c320(start, end, t, mask)` turns both ends into cells, row from x and column from y as
round(0.24·(17066.666 − c) − 0.5), and fills the cell list by one of four walks: a run along one
column when the y extent is below 2.3841858e-7 or both ends share a column, a run along one row
likewise for x, and otherwise a line walk along the larger extent. It then tests the listed cells with
`0x69c920`.

Addresses: `0x69c320`, `0x69c4e0`, `0x69c570`, `0x69c600`, `0x69c780`, `0x69c920`, `0x8107a4`, `0x86a758`, `0x8029d4`, `0xc962c8`, `0xc89f40`, `0x69c3e3`, `0x69c43c`, `0x69c4c8`
Confidence: verified
Proof: one
Test: `terrain::ray_dda_map_driver`, `terrain::raycast_e2e_map_driver`

`ret 0xc`. The four distances from 17066.666 are stored, the two extents (end − start) stored,
and each cell is the product by 0.24 (`0x8107a4`) stored, less 0.5 (`0x86a758`), `fistp`.
`0x69c3e3` compares |Δy| with 2.3841858e-7 (`0x8029d4`), a NaN not taking the run; `0x69c43c`
takes `0x69c600` when |Δy| is above |Δx| and `0x69c780` otherwise. The count is zeroed before
the walk and `0x69c4c8` passes the segment, t and the mask.

## Cell walks

The four walks list the cells a segment's projection crosses: a straight run of cells from start
to end along one axis, or a line walk that steps the major axis one cell at a time, computes the
minor cell from the line, lists an extra cell at each change of the minor cell, and ends with the
end cell. The per-cell test resolves each cell's area and chunk, skips holes, and intersects the
cell's four triangles, keeping the nearest fraction.

Addresses: `0x69c4e0`, `0x69c570`, `0x69c600`, `0x69c780`, `0x69c920`, `0x810678`, `0x81049c`, `0x7c2c40`
Confidence: inferred
Proof: none
Test: `terrain::dda_axis_run_a`, `terrain::dda_axis_run_b`, `terrain::dda_step_ymajor`, `terrain::dda_step_xmajor`, `terrain::dda_cell_intersect_difftest`, `terrain::raycast_e2e_map_driver`

`0x69c600` and `0x69c780` multiply by 4.1666665 (`0x810678`) and 0.24 and convert with
`fistp`; the runs `0x69c4e0` and `0x69c570` step ±1. `0x69c920` reads the hole word and the
triangle tables and calls the ray-triangle test `0x7c2c40`. The steppers' exact minor-cell
formula and the order of the corner cells are not derived here.
Each pair is the y cell, then the x cell: in `0x69c4e0` the store at `0x69c505` writes the y cell
before `0x69c51f` writes the x cell from `[ecx]`, and `0x69c570` does the same at `0x69c598` and
`0x69c5b0`; `0x69c3f2` to `0x69c3fa` lay the ends out as x cell, y cell.

## Terrain first hit

`0x6aa3e0` is the same walk for the terrain alone, used by queries that want the first hit: it
rebuilds the ray from the start and t, walks the cells and tests them with `0x6aa600`, which
records the hit triangle's plane through the collision code's `0x637480`, and rescales the raw
fraction by t.

Addresses: `0x6aa3e0`, `0x6aa600`, `0x637480`, `0x810ae4`
Confidence: inferred
Proof: none
Test: `terrain::ray_dda_chunk_driver`, `terrain::raycast_e2e_chunk_driver`, `terrain::dda_cell_intersect_first_difftest`

`0x6aa3e0` calls the four walks of `0x69c320` and then `0x6aa600`; the latter calls `0x637480`
with the three vertices. That `0x6aa160` reaches it is read from its calls; the arithmetic is not
derived here.
