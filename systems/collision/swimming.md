# collision: swimming

How a swimming mover moves once it swims: the vertical motion, the resting waterline, and which
units keep the height the integrator computes. The decision to swim is `objectlayer/swim`; the swim
commands and the swim start and stop applies are `collision/handlers`.

Roots: none

## Swim vertical motion

A swimmer's vertical motion comes from its pitch, and gravity does not act on it. The swim
velocity direction is `(cosY·cosP, sinY·cosP, sinP)`: `0x7c5880` stores `sinP` at `+0x64` under
flag 0x200000 when the pitch base `+0x54` is at least 2^-20 in magnitude. A spline-driven
swimmer takes its pitch from the path, `asin(dir.z)`, stored at `+0x20`; an own-input swimmer
takes it from the pitch commands. The resolve dispatch sends any mover with 0x200800 to the
free-move resolver `0x634640` before it tests falling, so the fall integrator `0x635b00` never runs
for a swimmer: a swimmer without pitch keeps its depth. The resolve entry runs only while the flags
hold a direction bit or falling, so an idle swimmer is not moved.

Addresses: `0x7c5880`, `0x7c58b3`, `0x7c58c1`, `0x7c58cc`, `0x7c5905`, `0x7c5490`, `0x7c55dd`, `0x7c55e9`, `0x7c55ee`, `0x7405a0`, `0x634040`, `0x634100`, `0x6341b6`, `0x6341bc`, `0x634640`, `0x635b00`
Confidence: verified
Proof: one
Test: `collision::velocity_basis_difftest`

`0x7c55dd test [esi+0x40],0x200000; je` guards `0x7c55e6 fld [ebp-0x54]` (the path's direction z),
the call of `__CIasin` (`0x7c55e9`) and `fstp [esi+0x20]`. `0x7c58b3` tests swimming and
`0x7c58cc jnp` skips the pitched basis for a pitch under 2^-20. `0x7c5905 fstp [ecx+0x64]` is the
third component. `0x6341b6 test ecx,0x200800; je 0x6341e3` goes on to the falling test only when neither bit is
set, and the fall through reaches the free-move call at `0x6341be`. `0x634100 test [esi+0x40],0x200f; je 0x6342c4` skips the resolve.

## Swim waterline

A swimmer rests with its feet at the liquid surface minus `0.75·h` and rises no higher, `h`
being the collision height `+0xb4`. The sweep against the liquid list scales the volume's height to
`0.75·h`, and the volume's top cap is its only plane facing up, with no plane closing it below;
liquid triangles reach that list with their normals negated, so a rising swimmer meets the surface as
a ceiling. It is a hard stop met by the sweep in each resolved step, not a spring. The depth at which
the swim decision enters is the same `0.75·h`.

Addresses: `0x632ba0`, `0x632c5a`, `0x632c64`, `0x632c6f`, `0x631be0`, `0x631440`, `0x6314cc`, `0x633e76`, `0x633e90`, `0x633eb3`, `0xc4e564`, `0x8012cc`
Confidence: inferred
Proof: none
Test: none

`0x632c5a test eax,0x200000`, `0x632c64 cmp ebx,0xc4e564` and `0x632c6f fmul [0x8012cc]` scale the
height only for a swimmer sweeping the liquid list (`collision/contact#earliest-contact-sweep`). The
volume planes are `collision/contact#volume-planes` (plane 4 is the top cap at `p.z + H`). The query
fills `0xc4e564` with mask `0x30000` and negates each normal and distance (`0x633e90`, `0x633eb3`).
The resting height follows from these planes and from no force acting on a swimmer; the resting
depth in the water is not traced through a fixture, and whether the 1/720 skin applies to the
liquid pass is not read.

## Swim height commit

Whether a resolved step keeps the vertical part of its displacement depends on the mover. The commit
`0x616cb0` keeps `d.z` when a live flying spline is set (spline present, bit `0x4` clear, spline flag
`0x200`) or when the mover has 0x200800 (swimming or fixed height), and zeroes it otherwise so the
body follows the ground. Spline flag `0x200` is one bit for flying and swimming, so a creature on a
swim spline keeps its height as a flying one does, and the dispatch sends it to the free-move
resolver by the same test.

Addresses: `0x616cb0`, `0x616cec`, `0x616cf1`, `0x616cf5`, `0x616cfa`, `0x616d03`, `0x6341a0`, `0x6341ad`, `0x6341b1`, `0x6341b6`, `0x7c5490`
Confidence: verified
Proof: one
Test: none

`0x616cf1 test al,4; jne` skips to the 0x200800 test, `0x616cf5 test ah,2; jne 0x616d0a` keeps `d.z`,
and `0x616d03 mov [ebp-4],0` zeroes it. `0x6341ad test al,4` and `0x6341b1 test ah,2` make the same
spline test for the dispatch. The spline sampler `0x7c5490` writes the full three-dimensional step
and does not clear z itself.
