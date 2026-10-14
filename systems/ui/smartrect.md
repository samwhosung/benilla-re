# ui: the screen rectangle placement

How nameplates and combat text keep from overlapping. Every frame the client seats each plate and
each number: it moves the desired rectangle off the rectangles already seated, within a bound, then
claims the final one so later ones avoid it. The two kinds have a claim list of their own and never
interact.

The code is one object file between the `Ui` and `UIUtil` files, `0x508af0` to `0x509ec0`; it names
no source path, and its allocation tags say `BFSNODE` and `CRect` and its CVar `SmartScreenRects`.
The two seats are `0x509ec0` for a nameplate and `0x509520` for combat text. A rectangle is four
floats in layout space (`regions`): top, left, bottom, right, with larger y higher. The CVar
`showsmartrects` registered here is read by nothing. The camera helpers that follow `0x50a060` belong
to other systems.

Roots: `0x509ec0`, `0x509520`, `0x509500`, `0x5094e0`, `0x509420`, `0x509450`, `0x508af0`

## Claim lists

Three lists of rectangles (ids 0 to 2) are kept at `0xbe0b8c`, 16 bytes each: capacity, count,
array, a growth step. List 0 holds the nameplates and list 1 the combat text; list 2 is never
claimed into. Claiming appends a rectangle to the list; resetting sets the count to 0 and keeps the
array. The counts are zeroed at start-up (`0x5094e0`, called from `0x40166b`) and again by the world scene
render: list 0 only in a frame whose `[render+0x3f0]` bit 0 is set (`0x48363c`), list 1 every frame.

Addresses: `0xbe0b8c`, `0xbe0b90`, `0xbe0b94`, `0x508f70`, `0x509660`, `0x509671`, `0x50967e`, `0x50a160`, `0x509500`, `0x509502`, `0x509512`, `0x5094e0`, `0x40166b`, `0x483640`, `0x483774`, `0x509228`
Confidence: verified
Proof: one
Test: none

`0x509660(id, rect)` computes `0xbe0b8c + 16·id`, and when `count + 1` exceeds the capacity grows the
array with `0x50a160` to the new count rounded up to a multiple of the step (16 from the claim that
needs 16, before that the largest power of two not above the new count), then copies the rectangle in at `count` and
increments it. `0x509500` stores 0 in the count for an id of 0 to 2 (`0x509502` to `0x509512`).
`0x5094e0` stores 0 in the count of lists 0 and 1 (its base `0xbe0b90` is the first count), and its one call is at start-up (`0x40166b`). The per-frame resets are
`0x483640` (list 0) and `0x483774` (list 1) in the scene render. A census of the base `0xbe0b8c` finds
three uses: the overlap test (`0x509226`), the reset (`0x50950c`) and the claim (`0x509667`). The
fields are also reached through `0xbe0b90` (`0x5094e0`) and `0xbe0b94`, the array pointer, in `0x509450` (`0x5094ad`) and the
initializer `0x508f70`, which zeroes the capacity, count and array of both lists.

## Freeing the lists

`0x509450` frees what the claims and the search allocate: the worklist, the node pool and the arrays of lists 0 and 1. It is called once, from
`0x481d7d` inside the world frame code `0x481d30`.

Addresses: `0x509450`, `0x509455`, `0x509480`, `0x5094ad`, `0x5094d6`, `0x481d7d`, `0xbe0bfc`, `0xbe0c38`, `0xbe0b94`
Confidence: verified
Proof: one
Test: none

`0x509450` takes the worklist nodes `0xbe0bf8` from the tail `[0xbe0bfc]` and frees each (`0x509455` to `0x509479`), then
the nodes of the pool `0xbe0c38` from its tail (`0x509480` to `0x5094ab`), then walks the two lists from `0xbe0b94`, freeing the array and storing 0 in the
capacity, count and array of each (`0x5094ad` to `0x5094d6`). List 2 has no array to free.

## The placement search

A rectangle is moved to the first free place a breadth-first search finds, trying four pushes per
candidate in an order chosen by where the rectangle sits on the screen. The search starts from the
rectangle clamped inside the screen. It tries up to `⌊K1/width⌋+1` times `⌊K2/height⌋+1`
candidates, takes the oldest first, and answers the first one a push leaves unchanged. If none does,
or the search runs dry, it answers the rectangle it was given.

Addresses: `0x5097a0`, `0x5097f0`, `0x5097f7`, `0x5097ff`, `0x509820`, `0x509836`, `0x50983f`, `0x509869`, `0x5098a8`, `0x5098ee`, `0x509916`, `0x509943`, `0x509a04`, `0x509a0c`, `0x509a25`, `0x509a3c`, `0x509a70`, `0x509b00`, `0x509dd0`, `0xbe0bf8`, `0xbe0bfc`, `0xbe0c38`, `0x8087e0`, `0x808870`, `0xbe0bf4`
Confidence: verified
Proof: one
Test: none

`0x5097a0(bucket, rect)` calls `0x509bf0` in clamp mode and `0x509d80` for the region and enqueues
the clamped seed as a 28-byte node (`0x509b00`: a pooled or `SMemAlloc(0x1c)` node, strategy field −1) at the
head of the list `0xbe0bf8`. Each pass takes the tail `[0xbe0bfc]` (`0x50983f`), so the order is
first in, first out. For each of four tries it looks up the strategy `[0x8087e0 + 16·region + 4·try]`,
skips the try when `0x509bf0` in test mode flags the node, calls `[0x808870 + 4·strategy]` and
compares the four floats with the node's (`jp` at `0x509916`). Unchanged, it answers the candidate
(`0x509a04`); changed, it enqueues it. The test `cmp eax,[…]` at `0x509874` never fires. `0x509a70`
recycles the list onto the pool `0xbe0c38` on every exit.

## Pushes by region

The screen is divided into nine regions by which edges the rectangle crosses, and the region fixes
the order of the four pushes: up, right, down, left for a rectangle inside the screen, and for one
beyond an edge the pushes that do not go further out.

| region | edges crossed | order of pushes |
|---|---|---|
| 0 | none | up, right, down, left |
| 1 | near the bottom | left, up, right, up |
| 2 | near the top | left, down, right, down |
| 3 | beyond the right | up, left, down, left |
| 4 | beyond the left | up, right, down, right |
| 5 | right and bottom | up, left, up, left |
| 6 | left and bottom | up, right, up, right |
| 7 | right and top | down, left, down, left |
| 8 | left and top | down, right, down, right |

Addresses: `0x509bf0`, `0x509c01`, `0x509c0e`, `0x509c16`, `0x509c26`, `0x509c78`, `0x509c8d`, `0x509c9d`, `0x509cb0`, `0x509ceb`, `0x509d80`, `0x509d82`, `0x509d86`, `0x509d9d`, `0x509dae`, `0x509dbf`, `0x8087cc`, `0x8087d0`, `0x8087d4`, `0x8087d8`
Confidence: verified
Proof: one
Test: none

`0x509bf0` builds the flag word: 1 when the left edge is below 0.0, 2 when the right edge is above
`K1 − 0.0`, 8 when the bottom is below 0.01875 (`0x8087d0`) and 4 when the top is above `K2 − 0.0375`
(`0x8087cc`). In clamp mode it moves the whole rectangle, keeping its size: the top clamp
takes precedence over the bottom, the left over the right. `0x509d80` is a priority tree: bit 1 gives 8
with bit 4, else 6 with bit 8, else 4; else bit 4 gives 7 with bit 2, else 2; else bit 2 gives 5 with
bit 8, else 3; else bit 8 gives 1, else 0. A dominant bit hides the lower ones, so flags 3, 11, 12 and
14 give regions 4, 6, 2 and 7.

## Pushes and the overlap test

A push moves a rectangle in one direction by the depth of the first claimed rectangle it overlaps,
so it lands edge to edge with it, and does nothing when it overlaps none. Overlap is strict: touching
rectangles do not overlap, and no gap is added.

Addresses: `0x5091c0`, `0x509300`, `0x509360`, `0x5093c0`, `0x509220`, `0x509241`, `0x509279`, `0x5092e8`, `0x50928f`, `0x5092a4`, `0x5092ba`, `0x5092d0`, `0x5091d4`, `0x5091dd`, `0x50931d`, `0x50937d`, `0x5093dd`
Confidence: verified
Proof: one
Test: none

| strategy | function | direction | change | depth |
|---|---|---|---|---|
| 0 | `0x5091c0` | up | top and bottom plus `d` | claimed top minus the bottom |
| 1 | `0x509300` | left | left and right minus `d` | the right minus claimed left |
| 2 | `0x509360` | right | left and right plus `d` | claimed right minus the left |
| 3 | `0x5093c0` | down | top and bottom minus `d` | the top minus claimed bottom |

`0x509220(id, rect, direction, &d)` walks the list and tests `r.right > c.left`, `r.left < c.right`,
`r.top > c.bottom` and `r.bottom < c.top` with `fcomp` and `test ah,0x41`, so an unordered value counts as
overlapping; on the first hit it jumps through `[0x5092e8 + 4·direction]` to the depth and answers 1.
Each strategy passes its direction (0 to 3), applies the depth when the answer is 1 and stores the
rectangle to its output.

## The nameplate seat

A nameplate is seated top anchored under its point: the desired rectangle hangs from the projected
point, is normalised, searched against list 0, its top clamped to `[halfH, K2 − halfH]` and its
centre to `[halfW, K1 − halfW]`, and claimed. The frame is then anchored by its top to the root's
bottom left at the clamped centre and top. The claimed rectangle can differ from the drawn one.

Addresses: `0x509ec0`, `0x509ed0`, `0x509eda`, `0x509f37`, `0x509f49`, `0x509f51`, `0x509f81`, `0x509fb2`, `0x50a010`, `0x50a01a`, `0x50a021`, `0x50a044`, `0x509e20`, `0x509720`, `0x608ce0`, `0x608d37`, `0x7ffa24`
Confidence: verified
Proof: one
Test: none

`0x509ec0(id, plate, root, point, flag)` reads the plate's width and height through layout slots 7 and
8 (`0x509ed0`, `0x509eda`), builds `top = point.y`, `left = point.x − halfW`, `bottom = point.y − height`,
`right = point.x + halfW`, normalises it with `0x509e20`, solves with `0x509720` and clamps. The second normalisation (`0x50a010`) changes only the claimed copy, and the anchor uses the unnormalised clamped
values (`0x50a026`), so a plate pinned to the bottom claims a rectangle up to `halfH` above where it
draws. `0x50a021` clears the points and `0x50a044` calls `0x767c70(1 or 4, root, 6, x, y, 1)`, TOP for
a zero flag and CENTER for a nonzero one. `0x509720` returns the changed flag, the centre x
`(left + right)/2` and the top y of the solved rectangle. The caller `0x608ce0` calls the seat only
for a plate that passed its on-screen test.

## Normalising a rectangle

`0x509e20` moves a rectangle onto the screen `[0, K1] × [0, K2]` keeping its size, with the four clamps in the order top, bottom, left, right. The rectangle
lands wholly inside only when it fits the screen. A rectangle taller than `K2` ends at the bottom clamp (bottom 0, top its height), and a wider
one at the right clamp (right `K1`). A NaN edge clamps nothing.

Addresses: `0x509e20`, `0x509e51`, `0x509e55`, `0x509e5a`, `0x509e6b`, `0x509e73`, `0x509e83`, `0x509e8b`, `0x509e9c`, `0x509ea1`
Confidence: verified
Proof: one
Test: none

`0x509e20` takes `h = top − bottom` and `w = right − left` (`0x509e29` to `0x509e3c`) and `K1`, `K2` from `0x41ae60` and `0x41ae70` with 1.0. The top clamp
compares `K2` with the top and, when the top is above or equal to it (`fcom`, `test ah,0x41; jp`, which falls through only when exactly one of C0 and C3 is set), stores top `K2` and bottom `K2 − h` (`0x509e51` to `0x509e5f`). The bottom clamp, a compare with 0.0 and `test ah,5`, acts only on a
negative bottom and stores bottom 0 and top `h` (`0x509e6b` to `0x509e7e`), so after a rectangle taller than the screen it undoes the top clamp. The left clamp
stores left 0 and right `w` (`0x509e83` to `0x509e96`); the right clamp acts when the right is above `K1`, stores right `K1` and left `K1 − w`
(`0x509e9c` to `0x509eb2`) and wins for a wider rectangle. Each compare with a NaN takes its skip branch.

## The combat text seat

A combat number is seated the same way against list 1: the rectangle is the string's half extents
around its clamped screen point, normalised, searched, clamped in the same bounds, claimed, and the
string is positioned at the centre of the returned rectangle. The call is made every frame for
each string with no option gate, and an off-screen string is destroyed by its caller before it gets here.

Addresses: `0x509520`, `0x50952d`, `0x509558`, `0x50956f`, `0x50957e`, `0x5095a1`, `0x50964e`, `0x509645`, `0x6c7cc0`, `0x6c7ea3`, `0x6c7d96`, `0x6c7d9f`, `0x483ee0`
Confidence: verified
Proof: one
Test: none

`0x509520(id, rect)` takes the half extents `|right − left|·0.5` and `|top − bottom|·0.5` and the
height, normalises, calls `0x509720(…, id, …)`, clamps the solved centre x to `[halfW, K1 − halfW]` and
top to `[halfH, K2 − halfH]`, rebuilds the rectangle with its top at the clamped value, normalises and
claims it with `0x509660` (`0x50964e`), writing the rectangle back to the caller. Its one caller is `0x6c7ea3` in
`0x6c7cc0`, which returns before it when the projection `0x483ee0` fails (`0x6c7d9f`). The caller turns the returned
rectangle's centre back with `0x41ae40` and `0x41ae50`, so a string whose blocker has gone returns to its
desired place next frame.

## Per frame order

Per frame the scene render clears list 0 when the nameplate pass runs (below), seats every nameplate in the active list's order, clears list 1,
and the combat text pass then seats the strings in their slot order. The first seated rectangle is not
moved; later ones move off it. The nameplate pass runs only when its dirty bit `[render+0x3f0]` bit 0
is set, and it clears the bit; a frame with no new or moved plate does not clear or re-seat.

Addresses: `0x483460`, `0x483635`, `0x483640`, `0x483647`, `0x48364c`, `0x483774`, `0x4840c0`, `0x60874b`, `0x6087dd`, `0x6c7900`, `0x482e7a`, `0x608ce0`, `0x509420`, `0x50943f`, `0xbe0c18`
Confidence: verified
Proof: one
Test: none

`0x483635` tests `[edi+0x3f0]` bit 0 and skips the reset and the walk when clear (`0x48363c`); `0x48364c`
clears it. `0x4840c0` sets it and has two callers, `0x60874b` (plate created) and `0x6087dd` (stored point
changed). `0x6c7900` seats the strings after `0x483774` clears list 1. The callers of the solver are
exactly `0x509731`, of `0x509720` `0x50957e` and `0x509f49`, and of `0x509660` `0x50964e` and
`0x50a01a`. `0x509420` registers `showsmartrects`, and its handle `0xbe0c18` has one reference, the store at
`0x50943f`.

## Static initializers

The file's initializers store 2π and 1/2π, the +∞ constant, eighteen squared constants and two more
that nothing reads, and the empty state of the first two claim lists, the worklist and the node pool.

Addresses: `0xbe0b18`, `0xbe0b7c`, `0x508af0`, `0x508b00`, `0x508b10`, `0x508b20`, `0x508b90`, `0x508ba0`, `0x508f50`, `0x508f60`, `0x508f70`, `0x508fa0`, `0x508fb0`, `0x508fe0`, `0x508ff0`, `0x509020`, `0x509030`, `0xbe0b68`, `0xbe0b38`, `0xbe0bbc`, `0xbe0b1c`, `0xbe0b20`, `0xbe0b2c`, `0xbe0b34`, `0xbe0b3c`, `0xbe0b44`, `0xbe0b48`, `0xbe0b4c`, `0xbe0b50`, `0xbe0b54`, `0xbe0b60`, `0xbe0b64`, `0xbe0b6c`, `0xbe0b70`, `0xbe0b74`, `0xbe0b78`, `0x508b40`, `0x508b50`, `0x508b70`, `0x508b80`, `0x508bc0`, `0x508bd0`, `0x508bf0`, `0x508c00`, `0x508c20`, `0x508c30`, `0x508c50`, `0x508c60`, `0x508c80`, `0x508c90`, `0x508cb0`, `0x508cc0`, `0x508ce0`, `0x508cf0`, `0x508d10`, `0x508d20`, `0x508d40`, `0x508d50`, `0x508d70`, `0x508d80`, `0x508da0`, `0x508db0`, `0x508dd0`, `0x508de0`, `0x508e00`, `0x508e10`, `0x508e20`, `0x508e30`, `0x508e50`, `0x508e60`, `0x508e80`, `0x508e90`, `0x508eb0`, `0x508ec0`, `0x508ee0`, `0x508ef0`, `0x508f10`, `0x508f20`, `0x508f40`, `0x5090d0`, `0x5090e0`, `0x509110`, `0x509120`, `0x80871c`, `0xbe0b24`, `0xbe0b40`
Confidence: verified
Proof: one
Test: none

The first pair stores `2·π` (the float at `0x8086d4`) at `0xbe0b68` and the second `1.0 / [0xbe0b68]` at `0xbe0b38`; `0x508f50` copies the +∞ at `0x808748` to `0xbe0bbc`. Each other
initializer is a 5-byte `jmp` and a 19-byte body of the form `fld c; fmul c; fstp g`. A dword census of
`0xbe0b18` to `0xbe0b7c` finds only these stores and the one read of `0xbe0b68`; the list base
`0xbe0b8c` has the three uses above, and its fields `0xbe0b90` and `0xbe0b94` the further uses listed under the claim lists. `0x508f70` zeroes the first two lists, `0x508ff0` makes the
worklist `0xbe0bf8` empty, and the exit handlers `0x508fb0` and `0x509030` free the arrays and the nodes.
