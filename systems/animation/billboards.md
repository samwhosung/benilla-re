# animation: billboards

The last stage of a bone in the bone loop: a bone whose flags carry a billboard bit has its palette
matrix turned to face the camera. The stage reads the palette entry the bone has just been given
(`pose`), keeps the length of each row and the position of the bone's pivot, and in the billboard cases replaces
the directions of the rows. It runs in the space of the palette, which for a scene-driven instance is the
view the scene passes as the root parent matrix (`models/scene#animate-pass`), so the camera axes are
constants there. The code is `0x7151f9` through `0x71594e` of `CM2Model::Animate`, with its jump table at
`0x718788` and index bytes at `0x71879c`.

Roots: none

## Billboard cases

The stage runs for a bone whose flags `& 0x78` are not 0. The selector is `(flags & 0x78) − 8`; a
selector above 0x38 takes the default. The index bytes map 0 to case 0, 8 to case 1, 0x18 to case 2,
0x38 to case 3 and every other selector to case 4.

| flags `& 0x78` | case | target |
|---|---|---|
| 0x08 | 0: all three rows replaced | `0x7152f8` |
| 0x10 | 1: row 0 kept | `0x715531` |
| 0x20 | 2: row 1 kept | `0x715639` |
| 0x40 | 3: row 2 kept | `0x715742` |
| 0x18, 0x28, 0x30, 0x38, 0x48 and above | 4: directions kept | `0x715868` |

Addresses: `0x7151f9`, `0x7151fc`, `0x7151ff`, `0x7152d0`, `0x7152d3`, `0x7152e4`, `0x7152ea`, `0x7152f1`, `0x71594e`, `0x718788`, `0x71879c`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

The table at `0x718788` holds `0x7152f8`, `0x715531`, `0x715639`, `0x715742`, `0x715868`. The 57 index bytes
at `0x71879c` are 4 except byte 0 (0), byte 8 (1), byte 24 (2) and byte 56 (3). `0x7152d0 add eax,-8` and
`cmp eax,0x38; ja` send a selector above 0x38 to `0x715868`. Flags 0 never reaches it, since `0x7151ff je 0x71594e` leaves first. The flags are the
combined flags of `pose`, so a bit set in the state's `+0xf4` also selects a case.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Row lengths and pivot

Before the case runs, the stage measures the palette entry `P` (`[+0x94] + bone·0x40`): the lengths
`L_k = f32(sqrt((x² + y²) + z²))` of rows 0, 1, 2, and the position of the bone's pivot `b` under the
entry: `p_j = f32(S_j + P[3][j])` for j = 0, 1, 2, where `S_j` adds the three products `b_i·P[i][j]` in the
order given below. The lengths and the pivot position are stored on the frame (`ebp-0x98` to `ebp-0x90`, `ebp-0xa4` to `ebp-0x9c`) for the tail. The sums run at 53 bits and
each result is narrowed once.

Addresses: `0x715205`, `0x71520b`, `0x71522e`, `0x715234`, `0x715255`, `0x71525b`, `0x71527c`, `0x715282`, `0x71529a`, `0x7152a0`, `0x7152b9`, `0x7152bf`, `0x7152de`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

`0x715205` reloads `esi` with the palette base and `0x71520f` adds the entry offset. Each length is
`fld` of the row's three floats, three squares summed as `(x² + y²) + z²`, `fsqrt`, one `fstp dword`. The
pivot terms: `p_0 = ((b2·P[2][0] + b1·P[1][0]) + b0·P[0][0]) + P[3][0]` (`0x715282` to `0x71529a`),
`p_1 = ((b0·P[0][1] + b2·P[2][1]) + b1·P[1][1]) + P[3][1]` and
`p_2 = ((b0·P[0][2] + b2·P[2][2]) + b1·P[1][2]) + P[3][2]`. The pivot `b` is the bone record's `+0x60`
to `+0x68`; the sum adds the translation row, so it is a point.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Spherical case

Case 0 writes the three rows of `P` and falls into the tail. When the flags `& 0x280` stored by the
local matrix route are 0, the rows are the constants `(0, 0, −1)`, `(1, 0, 0)` and `(0, 1, 0)`. When they
are not, the rows are built from the bone's local matrix `R` (`ebp-0xe4`, after scale and override):
row `k` is `(R[k][1], R[k][2], −R[k][0])`, divided by its length unless that is below 2^-22. A
local matrix that is the identity gives the constant rows.

Addresses: `0x7152f8`, `0x7152fb`, `0x7152fd`, `0x715303`, `0x715333`, `0x715342`, `0x715347`, `0x71537b`, `0x715477`, `0x71547c`, `0x7154a3`, `0x7154b6`, `0x71552c`, `0x715868`, `0x4549f0`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

`0x7152fd je 0x71547c` takes the constant rows, written as immediates through `ebp-0x138` to `ebp-0x154`:
`0xbf800000` at `0x7154a3`, `0x3f800000` at `0x7154b6` and `0x715506`. The built rows copy `R[k][1]`
(`ebp-0xe0`, `-0xd0`, `-0xc0`), `R[k][2]` (`-0xdc`, `-0xcc`, `-0xbc`) and the negated `R[k][0]` (`-0xe4`,
`-0xd4`, `-0xc4`), then `0x4549f0`, `fsqrt`, `fcomp 2^-22; test ah,5; jnp`, and `fdivr 1.0`. All three
cases end at `0x715868`.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Cylindrical cases

Cases 1 to 3 keep one row of `P` and rebuild the other two in the plane that has no z component. Each
normalizes a row only when its length is not below 2^-22. Case 1 normalizes row 0, sets row 1 to
`(r0y, −r0x, 0)` normalized, and row 2 to `row1 × row0`. Case 2 normalizes row 1, sets row 0 to
`(−r1y, r1x, 0)` normalized, and row 2 to `row1 × row0`. Case 3 normalizes row 2, sets row 1 to
`(r2y, −r2x, 0)` normalized, and row 0 to `row2 × row1`, with
`a × b = (ay·bz − az·by, az·bx − ax·bz, ax·by − ay·bx)` narrowed once per component.

Addresses: `0x715531`, `0x715533`, `0x715569`, `0x7155a5`, `0x7155dd`, `0x715634`, `0x715639`, `0x715674`, `0x7156b0`, `0x7156e6`, `0x71573d`, `0x715742`, `0x715794`, `0x71580e`, `0x715864`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

Cases 1 and 2 call `0x4549f0` for both normalizations (`(x² + y²) + z²`), case 3 sums inline as
`(z² + y²) + x²` (`0x71574b` to `0x715759`, `0x7157c7` to `0x7157d5`). Case 1's row 2 is
`x = r0z·r1y − r0y·r1z` (`0x7155dd`), which is `row1 × row0`; case 3's row 0 is `x = r2y·r1z − r2z·r1y`
(`0x71580e`), which is `row2 × row1`. Case 2's `fchs` is on the first component (`0x715679`), unlike
cases 1 and 3, which negate the second (`0x71557d`, `0x7157a3`).
`animation::m2_animate_difftest` runs `0x714260` whole.

## Row scale and translation

Case 4, and the end of the other cases, scales each row `k` of `P` by `L_k` component by component,
zeroes the fourth column, sets `P[3][3]` to 1.0, and sets the translation row so that the pivot stays at
`p`: `t_j = f32(p_j − ((P[1][j]·b1 + P[2][j]·b2) + P[0][j]·b0))`. The entries
`P[0][0]`, `P[0][1]`, `P[1][0]` and `P[2][0]` enter the sum as the 53-bit products, the other five as the
floats just stored.

Addresses: `0x715868`, `0x71586e`, `0x715872`, `0x71587b`, `0x715895`, `0x7158ec`, `0x715907`, `0x71591f`, `0x715925`, `0x71593c`, `0x715945`, `0x71594b`, `0x71594e`, `0x715957`, `0x715960`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

Each product `L_k·P[k][i]` is `fmul` then `fst` or `fstp dword`. The x products of rows 0, 1 and 2
and the y product of row 0 stay on the x87 stack as 53-bit values; the other entries are spilled
to floats at `ebp-0xc`, `ebp+0x10`, `ebp+8`, `ebp+0x14` and `ebp-0x18`. `t_0` sums the rows'
x values, `t_1` the y values (row 0's at 53 bits) and `t_2` the spills; each is `fsubr` from `p_j` and
stored at `+0x30`, `+0x34`, `+0x38`. The loop then adds 1 to the bone index and compares it with
header `+0x34` (`0x715957`).
`animation::m2_animate_difftest` runs `0x714260` whole.
