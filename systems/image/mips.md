# image: mip geometry

The sizes of a mip chain. A chain is one block: an array of absolute pointers, one per level,
followed by the pixel data of the levels in order. The functions take the graphics layer's
format number (`fmt`, below); its meaning belongs to `gx`.

Roots: `0x5a49b0`, `0x5a4a00`, `0x5a4a80`, `0x5a4ac0`, `0x5a4af0`, `0x5a4b80`, `0x5a4bb0`, `0x5a8140`, `0x5a80d0`, `0x5a62c0`

## Level sizes

`0x5a49b0(fmt)` returns bits per pixel: 4 for format 0, 8 for 1, 6 and 7, 32 for 2, 16 for 3 to 5
and 0 for anything above 7. `0x5a4a00(level, w, h, fmt)` returns a level's bytes: each dimension is
`max(1, dim >> level)`; formats 0, 1 and 7 raise a dimension of 4 or less to 4; format 9 returns
`wl · hl · 2 + max(1, wl · hl >> 2)`; otherwise `bpp · wl · hl >> 3`.

Addresses: `0x5a49b0`, `0x5a49b5`, `0x5a49d8`, `0x5a4a00`, `0x5a4a2f`, `0x5a4a34`, `0x5a4a48`, `0x5a4a4d`, `0x5a4a6e`, `0x5a4a74`
Confidence: verified
Proof: one
Test: `image::image_bpp_by_format`, `image::image_level_size`

The jump table `0x5a49d8` sends formats 0 to 7 to `0x5a49bc` (4 bits), `0x5a49c2` (8), `0x5a49ce` (32),
`0x5a49c8` (16, for 3 to 5) and `0x5a49c2` (8, for 6 and 7). `0x5a4a2f` tests the
format against 0, 1 and 7, `cmp esi,4; ja` keeps a dimension above 4 and
`mov esi,4` replaces the rest. Format 9 multiplies `wl · hl` by 2 and adds the quarter
(`0x5a4a4d` to `0x5a4a61`). The last line is `imul`, `imul`, `shr eax,3` (`0x5a4a6e`): the
product is not widened, so very large dimensions wrap.

## Chain size and level count

`0x5a4ac0(w, h)` returns the number of levels: 1 plus the number of times `w` or `h` can be halved
while either exceeds 1, each halving clamped to 1. That is `floor(log2(max(w, h))) + 1`. `0x5a4a80` sums
`0x5a4a00` over the first `count` levels. `0x5a4b80(fmt, w, h)` returns that sum for the full chain
plus `4 · count`.

Addresses: `0x5a4ac0`, `0x5a4a80`, `0x5a4b80`, `0x5a4ba4`
Confidence: verified
Proof: one
Test: `image::image_level_count`, `image::image_mip_sum`, `image::image_mip_block_size`

`0x5a4ac0` loops `cmp ecx,1; ja` or `cmp edx,1; jbe` out, `shr` each by 1 with a floor of 1 and
increments `eax` (`0x5a4ac5` to `0x5a4ae8`). `0x5a4b80` calls `0x5a4ac0` then `0x5a4a80` and adds
`count · 4` (`lea eax,[eax+edi*4]`, `0x5a4ba4`).

## Building the pointer table

`0x5a4af0(fmt, w, h, file, line)` allocates a chain with `SMemAlloc` using the caller's file and line,
fills the pointer table and returns it; `0x5a4bb0(fmt, w, h, base)` fills a table the caller
supplies. Entry `i` is `base + 4 · count + Σ levelSize(j) for j < i`.

Addresses: `0x5a4af0`, `0x5a4b2f`, `0x5a4b30`, `0x5a4b4f`, `0x5a4bb0`, `0x5a4bdb`, `0x5a4be0`
Confidence: verified
Proof: one
Test: `image::image_mip_alloc_populate`, `image::image_mip_populate_existing`

`0x5a4af0` takes `fmt` in `ecx`, `w` in `edx` and `h`, the file and the line on the stack. It calls
`0x5a4a80` for the size, adds `4 · count` and calls `0x6462e0` with that size, the caller's file
and line and flags 0 (`0x5a4b30`). The tests
run both functions whole with `SMemAlloc` returning a fixed block.

## Level byte size and pitch

`0x5a8140(fmt, level, &size, &pitch)` is the decoded size of a level for the formats the decoders
write: with `wl`, `hl` the dimensions shifted by the level and floored at 1 and `area = wl · hl`,
format 2 gives `4 · area` with pitch `4 · wl`, formats 3 to 5 give `2 · area` with pitch `2 · wl`,
format 9 gives `2 · area + max(1, area >> 2)` with pitch `2 · wl`, and any other format
gives 0 and returns 0.

Addresses: `0x5a8140`, `0x5a81f8`, `0x5a817e`, `0x5a8186`, `0x5a81df`, `0x5a81f1`
Confidence: verified
Proof: one
Test: `image::image_compute_mip_byte_size`

`0x5a8140` reads `+0x10` and `+0x14` of its BLP record, shifts, floors at 1 and subtracts 2 from the
format (`0x5a817e`); a result above 7 goes to `0x5a81df`. The jump table `0x5a81f8` sends formats
2 to `0x5a818d`, 3 to 5 to `0x5a81ab` and 9 to `0x5a81c5`, 6 to 8 to `0x5a81df`.

## Pixel pitch of a source format

`0x5a62c0(class, dim)` returns the row bytes of a conversion class for a width: the width, raised
to 4 for the DXT classes 5 to 7 when below 5, shifted right by a per-class amount and
multiplied by a per-class factor.

Addresses: `0x5a62c0`, `0x85c868`, `0x85c88c`
Confidence: verified
Proof: one
Test: `image::image_dxt_pitch`

The shifts at `0x85c868` are 0, 0, 0, 0, 0, 2, 2, 2, 0 and the factors at `0x85c88c` are 0, 4, 2, 2, 2, 8,
16, 16, 2 for classes 0 to 8. The decoder uses it for the source row bytes of a level
(`4 · wl` for class 1, `(wl >> 2) · 8` for DXT1).

## Box filter downsample

`0x5a4c30(dst, dstW, dstH, src, srcW, srcH)` halves or reduces a BGRA image by averaging
each `bw × bh` source block (`bw = srcW / dstW`, `bh = srcH / dstH`, integer divisions), weighting
the colour channels by each pixel's alpha.

Addresses: `0x5a4c30`
Confidence: verified
Proof: one
Test: `image::image_box_downsample`

When the block's alpha sum `ΣA` is non-zero the output channels are `Σ(c · A) / ΣA` and the alpha is
`ΣA / (bw · bh)`; when it is zero the channels are the plain average `Σc / (bw · bh)` and the alpha is
0. All divisions are signed 64-bit (`__alldiv`) and the low byte of each quotient is kept. The
function does no bounds checks. Its caller is the texture service's `0x44af3a`.
