# collision: climbing

What happens when a grounded mover walks into a steep face: when it rises over the obstacle and when
it slides along it. The walk resolver and its helpers are `collision/walking`, the sweep and its
volume are `collision/contact`, and the step height H is `collision/mover#collision-size`; this doc
states the law the three make together and the bytes that decide each branch.

Roots: none

## Rise sources

A walking mover gets a vertical response from one function, the rise along a face `0x635c00`,
called from the walkable arm (`0x636ada`, following the face's slope) and the step-allowed arm
(`0x636bf6`, the probe returned 2). The push-out `0x635d80`, used after a probe verdict of 1, writes z = 0, so a
blocked step never rises. In the step-allowed arm `0x636610` classifies the contacts. When every
contact is a mover bevel plane the rise follows the cone: a diagonal of up to 61.6°, scaled by
the cosine of the approach. Otherwise the steep face's own normal gives a huge rise, the clamp
sets it to H and the segment length to nearly 0: a vertical pop of H.

Addresses: `0x635c00`, `0x636ada`, `0x636bf6`, `0x636411`, `0x635d80`, `0x636b99`, `0x636610`, `0x636a99`, `0x636b41`, `0x636bbf`, `0x636c14`
Confidence: verified
Proof: one
Test: `collision::speed_select_difftest`, `collision::walk_helpers_difftest`

Relative-call scans over `.text`: `0x635c00` has the two sites above and `0x636411`, inside the
probe, whose position restore undoes it, `0x635d80` the one at
`0x636b99`, `0x636610` three (`0x6363c5`, `0x636a99`, `0x636b41`), and none has a stored address.
`0x636bbf or eax,0x4000000` sets the flag before `0x636bf6`, so `0x635c00` takes its
rise-below-zero and rise-above-H cases there. `0x636c14` scales the segment budget by the new
length over the old, which is denormal after the clamp. For a bevel-only manifold, `0x635c00`
replaces the face normal by the negated contact vector (`collision/walking#rise-along-a-face`),
whose z is +0.4756366, so the divisor is finite.

## Manifold identity

The contact planes a sweep reports are copies of the mover's own volume planes; the face hit is a
record of the world contact list. `0x632ba0` commits a contact by copying 16 bytes of the prism
plane to the manifold (`0x632e57` to `0x632e6e`), so the bevel test of `0x636610` asks whether the
earliest feature was a foot-cone plane. The slope class comes from the world record: the walk
resolver indexes `0xc4e534` (stride `0x34`) with the hit index and passes that normal to
`0x6334a0`. The world normal picks the arm; the mover plane picks the climb direction. `0x631e70`
only validates: near geometry it returns 1.

Addresses: `0x632ba0`, `0x632e57`, `0x632e6e`, `0x632e74`, `0x636610`, `0x636635`, `0x63664a`, `0x6334a0`, `0x636a58`, `0x636aa1`, `0x636aab`, `0xc4e534`, `0x631e70`
Confidence: verified
Proof: one
Test: none

`0x632e59` loads from the plane (`ebx`) and `0x632e5d` to `0x632e6e` store its four dwords at the manifold
slot (`esi`); `0x632e74` sets the count. The record address at `0x636aa1` is `index·0x34 + [0xc4e534]`.
`0x631e70` tests both corners against the cached box `0xc4e5a0` (`collision/contact#facet-gather`).

## Step-up certification

For a steep face that `0x6334a0` rejects, the resolver runs the probe `0x636100` and its verdict
decides: 1 slides, 2 climbs. The probe lifts by up to H, sweeps the reach `s` square into the face
from that height, and a face still hit with progress at most `r` returns 1; a 1 goes to the
push-out before the rise function runs. So a mover climbs only an obstacle that stops blocking
within the lift, however its edge meets the cone; every taller one (a wall, a trunk, a pinch
between two) slides. The probe's up sweep is not blocked by a vertical wall.

Addresses: `0x636100`, `0x6362a2`, `0x6362cb`, `0x6362ed`, `0x636306`, `0x636311`, `0x636516`, `0x636b6e`, `0x636b73`, `0x636b7f`, `0x632dd5`, `0x632de0`, `0x63276d`, `0x632778`, `0x632f3a`, `0x632bc9`, `0x8029d4`, `0x80c5c4`
Confidence: verified
Proof: one
Test: `collision::multipass_difftest`

`0x636b73 sub eax,0; je` sends 0 to the exit, and `sub eax,2; je 0x636bb9` sends 2 to the rise
function; other values (1) fall to the push-out call `0x636b99`. The up sweep passes the face
tests twice: `0x632dd5 fcomp 0.0; test ah,0x41; jnp` skips each mover plane with `dot ≤ 0`, so on
a vertical sweep only the top cap leads, and `0x63276d fcomp -1e-5; test ah,0x41; je` skips each
world face with `dot > -1e-5`, which includes `n.z = 0`. With nothing contributing, the best-t seed
(`0x632bc9`) is written unchanged (`0x632f3a`), so the lift equals H.

## Pop and settle

The pop is two segments. After the clamp `0x636c20` to `0x636cbe` build `newvel = dir·len + R`
with `len` near 0 and `R = (0, 0, H)`, so the first segment is vertical, length H, direction
(0, 0, 1). The horizontal heading resumes after that segment is swept, with length `rem/ms·dist`,
the share of `dist` left. The step reach `s` never leaves the probe: its two uses are sweep lengths
whose position effect is restored. The settle then sweeps down `d·1.8494 + 1/36` plus H (the flag the
arm just set adds H), so the mover ends where the down sweep rests.

Addresses: `0x636c14`, `0x636c20`, `0x636c88`, `0x636c9c`, `0x636cbe`, `0x63692a`, `0x63698b`, `0x636d10`, `0x636dcd`, `0x636dfc`, `0x636221`, `0x636327`, `0x6365c0`, `0x6365df`, `0x636193`
Confidence: verified
Proof: one
Test: none

`0x636c88 fst` stores the new length, `0x636c9c` to `0x636cbe` multiply the three components by
its reciprocal. The reach (`ebx`) is the length argument of the sweeps at `0x636221` and
`0x636327`, and `0x6365c0` and `0x6365df` copy the saved position back before every exit. The
cap `0x636d10` limits the total rise to `+0x80 + H` (`collision/walking#deflection-on-a-hit`).
`0x636dfc test eax,0x4000000` adds H at `0x636e00` and `0x636e05`.

## Climb regimes

A grounded mover walking into a steep face has three outcomes. The obstacle blocks the lifted
sweep: slide, no rise. The blocking edge lies in the foot cone's band, the bottom `r·1.8493990`
of the volume (`r` is `+0xb0`): the diagonal ride, capped at the engagement floor plus H. The
obstacle is above the band but clears within H: the pop, a one-frame step onto the top or over a
thin obstacle. H is `+0xb8`, 1.0 when the unit's scale equals its model scale, and 2.0 for a unit
`0x5fa550` rejects.

Addresses: `0x617430`, `0x6174b0`, `0x5fa550`, `0x80c740`, `0x635c00`, `0x636100`, `0x636d10`
Confidence: inferred
Proof: none
Test: none

The regime boundary between ride and pop depends on which face `0x632ba0` selects first for an edge
inside the band; this is read from the table of `collision/contact#face-prisms` and not traced
through a fixture. No function on the path tests the angle between the move and the face: the
tests compare a normal's z (`0x6334a0`, `0x636610`, the probe), never `dot(move, normal)` against
a threshold.

## Frame height outcome

A step-up frame ends at `entry z + rise − achieved down`, never held at the elevated height. Which
of the three results holds follows the swept volume. With the foot point a horizontal distance δ
behind a flat face of height `h` that the volume reaches at `min(u/1.8494, r)` at height `u`, the
obstacle wins when `1.8494·δ < h`. For `h` above the band `r·1.8494` and at most H the contact is
the vertical box side, so the mover lands on the top in one frame; for `h` inside the band the
contact is the cone against the top edge and the mover gains `1.8494` times each frame's advance
until it stands on the obstacle.

Addresses: `0x636e52`, `0x636e5b`, `0x631be0`, `0x632ba0`, `0x80c740`
Confidence: inferred
Proof: none
Test: none

The sum `entry z + rise − achieved down` follows from the seven relative position writes
(`collision/steps#walk-position-writes`). Which face the swept volume meets first at the worked
configuration (flat face, head-on, level ground) is composed from the face tables of
`0x631be0` and the earliest-contact rule of `0x632ba0`, and not traced against a fixture.
