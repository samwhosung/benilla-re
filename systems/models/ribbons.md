# models: ribbons

The ribbon emitters a model carries, `CRibbonEmitter`: a trail of edges left behind a moving
bone, each edge a vertex pair above and below the bone, aged out after the ribbon's lifespan and
drawn as a triangle strip.

The class is 0x168 bytes; its code runs from `0x7b6c10` through `0x7b8710`, with its assignment
and frame helpers just before, `0x7b6500` through `0x7b6990`, in the particle object file. A model
creates one ribbon per ribbon record of its header (`0x70ebd0`) and drives it each frame
(`0x718960`); the draw is `render`.

Roots: none

## Ribbon parameters

Each frame the model passes the ribbon its colour and alpha, packed into bytes as `x·255 + 0.5`,
its heights above and below the bone, its texture slot and its visibility, then places it at its
bone and updates it with the elapsed seconds.

Addresses: `0x718960`, `0x7b7a80`, `0x7b7b10`, `0x7b7b90`, `0x7b7ba0`, `0x7b7b70`, `0x7b7b40`, `0x7b76c0`, `0x7b7e60`
Confidence: verified
Proof: one
Test: none

`0x718960` calls the setters only for tracks with more keys than the first-update gate. The
colour packs three floats with `__ftol` after `fmul 255.0; fadd 0.5` and repacks the alpha byte
`a` as `ftol(a·255·255 + 0.5)`, whose low byte is `a` again (`0x7b7a88`); `0x7b7b90` and `0x7b7ba0`
store `+0x15c` and `+0x160`. The placement passes the bone matrix moved to the record's position
and multiplied by the scene matrix, and the instance's reference frame.

## Ribbon ring

A ribbon keeps its edges in a ring of `n = ceil(ceil(rate)·L) + 2` slots, with `rate` the record's
edges per second (`+0x94`) and `L` its lifespan (`+0x98`) raised to at least 0.25. Each slot holds
an age and a 40-byte vertex pair, bottom then top, each a position and texture coordinates; edges
live from the tail to the head, and the head slot holds the uncommitted edge at the bone. The
index array holds `i mod 2n` for `i` below `4n`, so a draw of the live run reads straight through
the wrap. A new ribbon starts hidden, 10 above and 10 below, with no gravity.

Addresses: `0x7b73a0`, `0x7b73c5`, `0x7b73e3`, `0x7b74e2`, `0x7b7686`, `0x70f7d8`, `0x7b6f00`, `0x73fdf5`, `0x81d878`
Confidence: verified
Proof: one
Test: none

`0x7b73a0` rounds the rate up with `0x73fdf5`, compares 0.25 (`0x81d878`) with the lifespan,
and stores `ftol(ceil(rate·L) + 2.0)` at `+0x8`, `2n` vertices at `+0x38` and `4n` words at
`+0x48`. It stores `1/L` at `+0x58`, the rate at `+0xf4`, `L` at `+0xf8`, 10.0 at `+0x15c` and
`+0x160` and 0 at `+0x164`. The model passes record `+0x94`, `+0x98`, `+0xa0` and `+0xa2` and
the rectangle (0, 0, 1, 1), then `+0x9c` to `0x7b7bb0` and 0 to `0x7b7b40`. `0x7b6f00` adds its
step to an index and wraps it at `+0x8`.

## Node frame

Each frame the ribbon keeps the bone's last and current node: position (matrix row 3), up (row 1)
and forward (row 2). On the first frame, and after it was hidden, both nodes are the current one.
The edge ends of a node are `p − below·up` (bottom) and `p + above·up` (top), and its tangent is
`|current − last|·forward`.

Addresses: `0x7b76c0`, `0x7b7739`, `0x7b7796`, `0x7b6990`, `0x7b7b40`
Confidence: verified
Proof: one
Test: none

`0x7b76c0` copies the matrix, adds its offset argument to row 3, and multiplies by the inverse of
the frame when one is given. With flag 1 set it moves the current node (`+0x14c`, `+0x88`, `+0xa0`)
into the last (`+0x20`, `+0x7c`, `+0x94`); otherwise it seeds the last from the new rows, sets
flag 1 and clears the carry `+0x1c`. `0x7b6990` stores the bottoms at `+0xc4`, `+0xd0`, the tops
at `+0xdc`, `+0xe8`, and the tangents at `+0xac`, `+0xb8`. `0x7b7b40` clears flag 1 when it
hides the ribbon.

## Edge curve

An edge at `t` between the last node (`t = 0`) and the current one (`t = 1`) puts each of its two
vertices at `s·P0 + t·P1 + s·t·(T0 − T1)`, with `s = 1 − t`, `P0` and `P1` that end of the last
and current node, and `T0` and `T1` their tangents. The edge takes its age and, when committed,
advances the head.

Addresses: `0x7b6c30`, `0x7b6c50`, `0x7b6c6e`, `0x7b6d96`, `0x7b6ec3`, `0x7b6ee7`
Confidence: verified
Proof: one
Test: none

`0x7b6c30(age, t, commit)` builds `t·(P1 − s·T1)` and `s·(P0 + t·T0)` for the bottom from `+0xd0`,
`+0xb8`, `+0xc4`, `+0xac` and stores the sum at `0x7b6d96`; the top repeats it with `+0xe8` and
`+0xdc`. `s` exists in two precisions: the bottom's `s·T1` multiplies the unrounded `1 − t` left on
the FPU stack (`fld st(0)` at `0x7b6c50` and `0x7b6c6e`), and every other `s` is its float copy at
`ebp−4` or `ebp−8`. It stores the age in `+0xc` at the head and calls `0x7b6f00(head, commit)`.

## Ribbon update

An update clamps `dt` into [0, L] and drops tail edges whose age plus `dt` exceeds `L`. A visible,
started ribbon then accumulates `e = dt·rate + carry` and commits `floor(e)` edges at `t_j = (j −
carry)·r` with `r = 1/(e − carry)` rounded to a float, aged `−t_j·dt`, keeps `e − floor(e)` as the
carry, and writes the head edge at `t = 1`, age 0. Every committed edge then moves `gravity·(2·age +
dt)·dt` up in z, ages by `dt`, and takes `u = u0 + age·w/L`, `v` the cell's bottom and top.

Addresses: `0x7b7e60`, `0x7b7e7d`, `0x7b7e98`, `0x7b7ee0`, `0x7b7f16`, `0x7b7f40`, `0x7b7f48`, `0x7b7f83`, `0x7b7ff6`, `0x7b8080`
Confidence: verified
Proof: one
Test: none

The clamp loads `+0xf8` and replaces `dt` by 0 when negative and by `L` when above it. Emission
needs flags 4 and 1 and a zero second argument; `0x7b7f40` loops `ftol(floor(e − 1)) + 1` times and
`0x7b7f83` floors for the carry; `0x7b7f16` stores `r` as a float and `0x7b7f48` multiplies `j −
carry` by it. The aging loop adds `(2·age + dt)·dt·+0x164` to both z values (`+0x8`, `+0x1c`), then
`dt` to the age, and stores `age·+0x5c·+0x58 + +0x70`. Flag 8 is set and 0x10 cleared at the end.

## Ribbon texture cells

A ribbon's cell `c` of `R` rows and `C` columns spans `u0 = (c mod C)/C` to `u0 + 1/C` and
`v0 = (c / C)/R` to `v0 + 1/R`; bottom vertices take `v0`, top ones `v0 + 1/R`. Over its life an
edge's `u` runs from `u0` to `u0 + 1/C`. Setting the slot recomputes the cell only when it changes.

Addresses: `0x7b6f30`, `0x7b7b70`, `0x7b7530`
Confidence: verified
Proof: one
Test: none

`0x7b7530` stores `(1 − 0)/C` at `+0x5c` and `(1 − 0)/R` at `+0x60` from the rectangle; `0x7b6f30`
divides `+0x158` by `+0x144` and stores `u0` at `+0x70`, `v0` at `+0x6c`, `u0 + w` at `+0x78` and
`v0 + h` at `+0x74`.

## Ribbon draw

A ribbon with edges draws the live run as one indexed strip of `2·count + 2` indices from twice
the tail, through its frame moved by minus its offset, with one colour for every vertex. Each
material sets five render states from its flag bits 0 to 4, its blend, and its texture.

Addresses: `0x7b80c0`, `0x7b81ad`, `0x7b81f0`
Confidence: verified
Proof: one
Test: none

`0x7b80c0` returns 0 when tail equals head. The count is head − tail, or capacity − tail + head
when wrapped, and the indices start at `+0x50 + 4·tail`. The colour stream is `+0x12c` with
stride 0. Flags bits 0 to 4 go to states 0xe, 0xf, 0x10, 0x12 and 0x14, the blend `+0x4` to state
7, and texture `+0x114[i]` to stage 0.
