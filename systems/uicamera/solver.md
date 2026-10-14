# uicamera: the near and far solver and the water bands

How the follow camera keeps its pivot and its orbit distance clear of walls, ceilings and water: the per-frame
outer call, the solver that shortens the pivot height and the distance, the two water bands that move the
solver's limits, and the pitch the camera takes when it crosses the surface.

This is `Camera.cpp`. The update that calls the solver is in `uicamera/driver`; the CVars are `uicamera/cvars`;
the swept queries the solver issues belong to the world (`collision`).

Roots: `0x511bc0` (per frame), `0x50e570` (solver), `0x511ad0` (bands), `0x511a80` (listener state)

## The per-frame outer call

The world frame calls `0x511bc0(frame, now)` for the camera each update. It stores the near clip, the CVar `nearclip`,
at `+0x38` and the far clip, the CVar `farclip`, at `+0x3c`. If a cinematic is playing (`+0x48` nonzero) it runs the
cinematic update `0x50f060`. Otherwise it resolves the target GUID at `+0x88` and calls the driver. Either way it then
calls `0x511a80`, which asks the world for the liquid type at the camera and, when that differs from the
cached one at `+0x250`, updates the listener's submerged sound flag; it returns 1.

Addresses: `0x511bc0`, `0x50f060`, `0x511a80`, `0x672470`, `0x458650`, `0xbe1078`, `0xbe0cc0`
Confidence: verified
Proof: one
Test: none

`0x511bdc` stores the near clip and `0x511be8` the far clip. `0x511bea` tests `+0x48` and `0x511bf0` calls `0x50f060`. `0x511c2e` calls `0x50e8d0`.
`0x511a8b` calls the liquid type getter `0x672470`; `0x511aa3` to `0x511ab5` update the cache and set mode bit 1. The type 0xf
means none (`0x511aa6`).

## The solver

`0x50e570(pivot, &distance, &height, offset, band)` gives the largest distance and pivot height that clear the
world. Both start from the larger of target and live values: the distance from `+0x198` and `+0xec`, the height
from `+0x1c8` and `+0xfc`. The trace mask is 0x100171, plus 0xf0000 when `cameraWaterCollision` is set; the same
register enables the water bands below. A vertical probe upward from the pivot plus a base height of 5/6 yard
(more in the surface band) shortens the height, which is then clamped between that base and a cap. The cap is normally the live height `+0xfc`. A height within 2^-20 of the near clip skips the probe and the band limits.

Addresses: `0x50e570`, `0x50e5ec`, `0x50e629`, `0x50e6f6`, `0x8089cc`, `0x808a04`
Confidence: verified
Proof: one
Test: `ui::cam_50e570`

`0x50e57a` to `0x50e5cc` take the larger of each pair; `0x50e5e3` compares height minus the near clip with 9.5e-7; `0x50e623` jumps to the clamp at `0x50e754` when it is not greater, past the probe and the band limits (`0x50e629` to `0x50e684`). `0x50e607` stores the live height as the cap `[ebp-0xc]`. `0x50e5ec` to `0x50e61a` build the trace mask (`neg esi; sbb esi,esi; and esi,0xf0000; add esi,0x100171`). `0x50e687` raises the
probe start by the base `[ebp-4]` (5/6, `0x8089cc`), `0x50e690` takes the height minus the base and `0x50e695` raises it to at least 1/9 (`0x808a04`).
`0x50e6f6` is the vertical probe; a hit scales the length, and `0x50e717` to `0x50e743` back it off by 1/9 when it was shortened. `0x50e756` to
`0x50e776` clamp the height to the base and the cap. `0x50e778` to `0x50e793` start the boom at the pivot plus the clamped height; `0x50e816` is the
sweep, `0x50e861` the clip; `0x50e871` to `0x50e8a4` back off; `0x50e8a9` limits the distance to the live value. The solver has one caller, `0x50ed59`.

`ui::cam_50e570` runs `0x50e570` whole with `0x50de00` and `0x50de90` real, and the two ray queries `0x672170` and the sweep `0x686db0` replaced.

## The boom sweep

A sweep from the pivot, raised by the solved height, against the view direction shortens the distance, and the near box clip (below) can shorten it again. After a hit the distance backs off 1/9 yard; it is never
below 0 or above the live distance. The answer is a mask: 0x20000 when the vertical probe hit and 0x10000 when
the sweep did or the clip did.

Addresses: `0x50e816`, `0x50e861`, `0x672170`, `0x50de90`
Confidence: verified
Proof: one
Test: none

`0x50e778` to `0x50e793` start the boom at the pivot plus the clamped height; `0x50e816` is the sweep (it sets 0x10000 at `0x50e82f`) and `0x50e861` the clip (it sets 0x10000 at `0x50e86a`); `0x50e871`
to `0x50e8a4` back off; `0x50e8a9` limits the distance to the live value. The solver has one caller, `0x50ed59`.

## The near box clip

`0x50de90(&distance, trialEye, flags)` shortens the orbit distance so the near end of the view volume clears geometry. When the distance is below the near clip plus 0.001 it sets the distance to 0 and answers 1; a distance far under the near clip takes the same exit. Otherwise it builds the view and the projection for the trial eye, takes the eight corners of the volume between the near plane and the trial distance, collects the world polygons that overlap that box
and, for the polygons that clip it, finds the largest depth along the box. It reduces the distance by that depth times the distance beyond the near clip, never below 0, and answers 1 when any polygon clipped.

Addresses: `0x50de90`, `0x5c3e70`, `0x5c3cc0`, `0x5c43b0`, `0x686500`, `0x672390`, `0x687080`, `0x686db0`
Confidence: inferred
Proof: none
Test: `ui::cam_50de90`

`0x50dea3` to `0x50debe` take the near case: `0x50deb4` jumps on above or equal, so the test is one-sided. `0x50deef` and `0x50defa` read the camera's vtable slots 3 and 1. `0x50e099` builds the look-at, `0x50e0b7` and `0x50e0c3` the projection (`uicamera/transitions#field-of-view`), `0x50e0db` the corners. `0x50e21d` makes the box, `0x50e28e` and `0x50e2e9` query and test the world with the flags, `0x50e434` clips each polygon, and `0x50e482` to `0x50e49c` shorten the distance. The depth variable and the clip routine were read for their flow, not decoded.

`ui::cam_50de90` runs `0x50de90` whole, with its collision and matrix callees stubbed and the swept query `0x686db0` replaced by an island that writes a chosen contact list.

## The water bands

`0x511ad0(unit)` classes where the water surface lies against the pivot target and sets one of two mode bits, 0x100000
(surface) or 0x200000 (submerged); both are cleared first. With `d` the height of the liquid surface above the unit's
position and `H` the target pivot height (`+0x1c8`): `d` below `H + 2/9` is the surface band, `d` from there up to `H + 5/9`
is the submerged band, and above that is neither. A unit with no liquid under it gets neither. It returns `d`, or 0.
In the surface band the solver's base height becomes `d + 2/9` and its cap at least that; in the submerged band
the cap is `d - 5/6`, at least 5/6.

Addresses: `0x511ad0`, `0x670630`, `0x8089d0`, `0x8089d4`
Confidence: verified
Proof: one
Test: `ui::glue_511_511ad0_residual`

`0x511ae3` clears both bits (`and ecx,0xffcfffff`). `0x511b39` calls the liquid query `0x670630`; `0x511b5b` forms `d` against the unit's z. `0x511b61` compares with
`+0x1c8`: below it goes to the surface band at `0x511ba4`. `0x511b76` compares the excess with `0x8089d0` (2/9) and `0x511b83` with `0x8089d4` (5/9); `0x511b90`
sets the submerged bit. In the solver `0x50e638` to `0x50e65d` build the surface limits and `0x50e666` to `0x50e684` the submerged cap from `[ebp+0x18]`, the argument that holds `d`.

## Crossing the surface

When the camera passes from the submerged band to the surface band the pitch is set to the CVar
`cameraSurfaceFinalPitch`, 5 degrees by default, and from the surface band to the submerged band to
`cameraSubmergeFinalPitch`, also 5. The pitch goes in directly in mouselook and otherwise through the pitch
transition. Nothing is done unless `cameraWaterCollision` is nonzero.

Addresses: `0x50ecae`, `0x50eb35`, `0x50eb3f`, `0x50ecd0`, `0x50eced`, `0x50ed18`, `0x50ed34`, `0x512830`
Confidence: verified
Proof: one
Test: none

`0x50eb35` saves the previous submerged bit and `0x50eb3f` the previous surface bit before `0x50eb45` classes the new bands. `0x50ecae` tests the CVar. `0x50ecc2` and
`0x50ecc8` select the surface pitch when the previous frame was submerged and the new one surface; `0x50ecdb` to `0x50ecf2` the submerged one in the opposite case.
`0x50ecfb` converts to radians; a zero pitch is skipped (`0x50ed0c`). `0x50ed18` and `0x50ed1e` store it in `+0xf4` and `+0x1e0` in mouselook, `0x50ed34` calls the
pitch setter `0x512830` otherwise.
