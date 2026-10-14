# ui: Model frames, camera and paint

How a `Model` frame picks the camera its scene is drawn through, what the paint installs around the
scene (projection, viewport, depth clear, root transform), and how model units become layout units.

A `CSimpleModel` is a frame of 0x3dc bytes that holds one M2 model at `+0x318` and draws it in a
private scene. `PlayerModel`, `DressUpModel` and `TabardModel` extend it (`playermodel`); the model's
own animation, camera objects and particles belong to the model system (`models`, `camera`). The
interface's coordinate units are those of `screencoord`: the screen spans `0.8 × 0.6` layout units at
4:3, and `uiscale` says how the frame scale reaches the transform.

Roots: `0x76c8e0`, `0x76cec0`, `0x76ce00`, `0x76d240`, `0x76d1a0`, `0x76d080`, `0x76d0d0`

## Model frame fields

A model frame keeps its scene at `+0x314`, the model at `+0x318`, the camera at `+0x31c`, a pending
camera index at `+0x320`, a light at `+0x324`, a position, facing and scale at `+0x390` to `+0x3a0`,
fog at `+0x3a4` to `+0x3b0`, the model's bounding box at `+0x3b4`, and two script slots. The constructor
zeroes the fields above; the pending camera index is 0, a request for camera 0, the scale and the fog
far distance are 1.0, and the fog colour is 0xffffffff.

| offset | field |
|---|---|
| `+0x314` | scene |
| `+0x318`, `+0x31c`, `+0x320` | model, camera, pending camera index |
| `+0x324` | light (0x6c bytes) |
| `+0x390` to `+0x398`, `+0x39c`, `+0x3a0` | position, facing, scale |
| `+0x3a4` to `+0x3b0` | fog flags, colour, near, far |
| `+0x3b4` to `+0x3c8` | model bounding box, minimum then maximum |
| `+0x3cc`, `+0x3d4` | `OnUpdateModel` and `OnAnimFinished` slots |

Addresses: `0x76c8e0`, `0x76c910`, `0x76c92d`, `0x76c938`, `0x76c950`, `0x76ca03`, `0x76c98f`, `0x76c995`, `0x6eed18`, `0x76cfc0`, `0x81c608`, `0x81c5d8`
Confidence: verified
Proof: one
Test: none

The constructor stores the primary vtable `0x81c608` at `+0x0` and the geometry vtable `0x81c5d8` at
`+0x24`; geometry methods run with `ecx = frame + 0x24`, so their `+N` is frame `+0x24 + N`. The
factory allocates 0x3dc bytes (`0x6eed18`) and `0x76cfc0` creates the scene on first use and caches
it at `+0x314`; every model frame has its own scene and clock. The zero stores are `0x76c8fe` to
`0x76c986`; the pending index is `0x76c910`, the scale `0x76c938`, the far distance `0x76c950` and the
fog colour `0x76ca03`.

## Choosing the camera

A model frame draws only when its pending camera index is −1. The constructor leaves 0, and when the
model is ready the frame applies it: it looks up raw camera number `idx` of the model, ignoring the
file's camera lookup table, and installs it. An index at or past the camera count installs no camera,
so the paint takes the orthographic leg. Installing any camera, including none, sets the index to −1.
`SetCamera(n)` truncates `n` and does the same; a negative `n` is past the count. With no model, or one
whose data is not resident, the call only stores the index.

Addresses: `0x76cec0`, `0x76cedb`, `0x76ceeb`, `0x76ce80`, `0x76cead`, `0x76ce00`, `0x76ce3e`, `0x7134d0`, `0x713590`, `0x76e0e0`, `0x76e182`, `0x76d5f0`, `0x76d7f0`
Confidence: verified
Proof: one
Test: none

`0x76cec0` stores the index and returns when `+0x318` is null (`0x76cece`) or `0x7103d0` finds no
resident data (`0x76cedb`). Otherwise `0x7134d0` returns the model header's camera count (`+0x124`) and
the unsigned compare `jae` at `0x76ceeb` sends an out-of-range index to `0x76ce80(NULL)`; else
`0x713590` returns the `CCamera` at `[model+0x3c4] + idx·0x84 + 0x80`. `0x76ce80` is the only writer of
`+0x31c`: it takes a reference, releases the old camera, stores, and writes −1 at `0x76cead`. The
model-ready hook `0x76ce00` applies a pending index (`0x76ce3e`); the paint tests it at `0x76d5f0`.

## Stale camera after SetModel

A second `SetModel` leaves the frame's camera as it was. The new model is drawn through the old
model's camera, frozen at the eye and target published on the old model's last paint, until `SetCamera`
is called. The pending index is not re-armed, so the model-ready hook skips the camera lookup.
`ClearModel` leaves the camera too.

Addresses: `0x76cd30`, `0x76cce0`, `0x76d950`, `0x76db20`, `0x76ce00`, `0x76ce44`, `0x76ce47`
Confidence: verified
Proof: one
Test: none

`SetModel` reaches `0x76cd30` through slot 37 (`0x76cce0`) and slot 36; `ClearModel` calls the same slot
with null. `0x76cd30` takes a reference on the new model (`0x710390`), releases the old one
(`0x7103a0`), stores `+0x318`, passes the frame's alpha to the model and registers three callbacks; it
reads neither `+0x31c` nor `+0x320`. In `0x76ce00` the compare of `+0x320` with −1 (`0x76ce44`) takes
the `je` at `0x76ce47` past the lookup. The old model is destroyed with its reference, so the camera
object lives only on the frame's own reference and no scene republishes it.

## The character pane camera

A `PlayerModel`, `DressUpModel` or `TabardModel` replaces the camera when its model is ready (a
`TabardModel` then moves the target, `playermodel#tabard-camera-target`). It tries
raw camera 1; if the model has one, it installs a frozen copy of it, made after the frame's facing is
set to 0 and one evaluation pass has run. Otherwise it builds a fixed camera: eye (5.5555558, 0, 2.4166667),
target the model's bounding-box centre, field of view 0.5, near clip 0.027777778, and the default far
clip 5000.0. Either way the frame ends with a camera and never takes the orthographic leg.

Addresses: `0x505b30`, `0x505890`, `0x50589c`, `0x5058a9`, `0x5058b5`, `0x5058cd`, `0x5058df`, `0x505906`, `0x505917`, `0x50591e`, `0x505935`, `0x50596a`, `0x50597e`, `0x7acf10`, `0x713680`, `0x7ac930`, `0x7ac250`, `0x7ac3a0`
Confidence: verified
Proof: one
Test: none

The ready hook `0x505b30` calls `0x76ce00`, then `0x505890`. That tries camera 1 (`0x50589c`) and tests
`+0x31c` (`0x5058a9`). With none, `0x713680` gives the box centre, `0x7ac930` allocates a camera and
`0x7ac3a0` and `0x7ac250` set its field of view (`0x3f000000`), target, eye (`0x40b1c71d`, 0,
`0x401aaaab`) and near clip (`0x3ce38e39`). With one, it saves the facing, stores 0 (`0x50592b`), calls
slot 38, links the model, runs `0x707680` (`0x50596a`), restores the facing and clones the camera with
`0x7acf10` (`0x50597e`); the clone holds the source camera's evaluated values as constants
(`camera#lifetime`). The facing is zeroed so the copy is taken in the model's unrotated frame.

## Camera publish and scale

An authored camera's eye and target reach the frame's camera through the model's root transform, so the
camera and the geometry take the same scale and position: on a live camera `SetModelScale`,
`SetPosition` and the frame scale change nothing in the picture. A frozen camera (character panes) keeps
the transform of its snapshot, so there a later scale or position change shows. The fixed fallback
camera is never republished and keeps its model-space eye, so there the scale shows too.

Addresses: `0x718960`, `0x718b66`, `0x718ba9`, `0x718bb6`, `0x718bc7`, `0x718c21`, `0x718c35`, `0x71439b`, `0x707680`, `0x7076c6`, `0x707752`, `0x74a7c0`
Confidence: verified
Proof: one
Test: none

The publish pass of `0x718960` forms eye = base position + position track, and passes it through the
model's matrix at `+0xfc` (`0x718ba9`) and then through the scene's inverse view at `+0xdc`
(`0x718bb6`), storing it as property 7 (`0x718bc7`); the target does the same (`0x718c21`) and the roll
track is stored unchanged (`0x718c35`). `+0xfc` is `+0xbc` times the scene view, written by the animate
step (`0x71439b`); `0x707680` reads the view back from the device (`0x7076c6`) and inverts it into
`+0xdc` (`0x707752`). So the view cancels and the published eye is the authored eye times the root
transform alone, whose scale is uniform.

## The facing cancels

On a plain `Model` with a live authored camera, `SetFacing` does not rotate the picture. The camera's up
vector is exactly (0, 0, 1) while the published roll is 0, which is the axis the root
transform rotates about, so the camera frame turns with the geometry. The only trace is one frame of lag
at a change: the paint reads the eye and target published on the previous paint before it rebuilds the
root transform. With a frozen or stale camera, a nonzero published roll, or no camera, the facing shows.

Addresses: `0x7ac640`, `0x7ac778`, `0x7ac7d0`, `0x76d411`, `0x76d421`, `0x76d42d`, `0x76d5ea`, `0x76d619`, `0x76d1fe`, `0x707839`, `0x70dd70`
Confidence: verified
Proof: one
Test: none

`0x7ac640` builds up as `(sin yaw · sin roll, −cos yaw · sin roll, cos roll)` (`camera#view-and-projection`).
Nothing writes a camera's yaw (property 6, `+0x164`), so it stays 0 and up is `(0, −sin roll, cos
roll)`, and the published roll is the camera's roll track unchanged (`0x718c35`), so up is (0, 0, 1)
whenever that track is 0. Which models carry a nonzero roll track is install data and not derived here.
`0x76d1fe` sets the rotation axis to (0, 0, 1). The paint reads the eye and target (`0x76d411`,
`0x76d421`), builds the view (`0x76d42d`), rebuilds the root transform through slot 38 (`0x76d5ea`) and
only then runs the scene, whose publish (`0x707839`) stores the new eye. The model's matrix starts as
the identity (`0x70dd70`), so the first paint after a load lags too.

## The paint

The paint of a model frame runs from the ARTWORK batch of its layer. It returns at once without a
model or a resolved rectangle. It saves the device projection, modelview and viewport, builds a
projection over the frame's raw rectangle (camera leg or orthographic leg), sets the viewport to the
rectangle, clears the depth buffer inside it, rebuilds the root transform, and, if the pending camera
index is −1 and the model's data is resident, runs the scene and draws passes 0 and 1. It then restores
the three saved states.

Addresses: `0x76d240`, `0x76d24c`, `0x76d27d`, `0x76d3a7`, `0x76d3b2`, `0x76d3cd`, `0x76d404`, `0x76d42d`, `0x76d448`, `0x76d5d7`, `0x76d5e1`, `0x76d5ea`, `0x76d5f0`, `0x76d603`, `0x76d619`, `0x76d629`, `0x76d639`, `0x76d644`, `0x76d64f`, `0x76d66c`, `0x768320`, `0x7ada40`, `0x7ad7f0`, `0x58b090`, `0x58b0b0`, `0x58b060`, `0x58af60`, `0x58a970`, `0x708900`, `0x710450`
Confidence: verified
Proof: one
Test: none

`0x76d27d` reads the resolved rectangle `{bottom, left, top, right}` through `0x768320`, which fails
until the layout has resolved. The saves are `0x76d3a7`, `0x76d3b2` and `0x76d3cd`. A camera at `+0x31c`
takes the camera leg: `0x7abf80` reads eye and target, `0x7ada40` (`camera#view-and-projection`) builds
the projection over the raw rectangle; a null camera takes `0x7ad7f0` with the point (left, bottom)
(`camera#orthographic-setup`). `0x58a970(2)` is the device clear with the depth bit only. After slot 38,
`0x76d5f0` and `0x710450` gate `0x707680` and `0x708900` with 0 then 1. The restores are `0x76d644`,
`0x76d64f` and `0x76d66c`.

## Viewport and clamp

The viewport is the frame's rectangle as fractions of the screen: each edge is divided by the screen's
layout extent, and a fraction at or past 1 becomes 1, at or below 0 becomes 0, and NaN becomes 1. The
fractions are multiplied by the enclosing viewport's right and top fractions only, which is exact when
that viewport starts at the origin, as the interface's does. The projection is built over the unclamped
rectangle, so a frame partly off the screen is squashed into the visible strip, not cropped. If the
width or the height fraction is under 2^-22 the paint draws nothing.

Addresses: `0x76d45b`, `0x76d46e`, `0x76d473`, `0x76d4a4`, `0x76d574`, `0x76d587`, `0x76d58d`, `0x76d5a0`, `0x76d5a6`, `0x41ade0`, `0x58af60`, `0x8029d4`
Confidence: verified
Proof: one
Test: none

`0x41ade0` divides x by the screen's x scale and y by its y scale (`0x76d45b`, `0x76d46e`). Each clamp
block is `fcomp 1.0; test ah,5; jp` then `fcomp 0.0; test ah,0x41; jne` (`0x76d473` to `0x76d56d`);
NaN sets C0, C2 and C3, so the first `jp` is taken and the value is stored as 1.0. The size test
(`0x76d574`, `0x76d58d`) compares `|left − right|` and `|bottom − top|` with `fcomp`; only a strictly
smaller value sets C0 and takes the `jnp`, so an equal width draws and a NaN draws. `0x76d5a6` to
`0x76d5d4` multiply by the saved `x1` and `y1`. `0x58af60` rejects equal or inverted edges and clamps
again.

## Clip and depth clear

Nothing a model frame draws lands outside its rectangle. The orthographic projection maps exactly the
rectangle to the clip volume, and the viewport is the same rectangle, so the rectangle is the clip on
either backend. The Direct3D 9 viewport call bounds the depth clear; the OpenGL path also sets a
scissor equal to the viewport. The device's default API is `direct3d`. The depth clear runs on every
paint, inside the viewport only; the frame's pixels are drawn into the back buffer.

Addresses: `0x7ad7f0`, `0x5c3d90`, `0x5a1100`, `0x5a11bb`, `0x59c730`, `0x59c7d5`, `0x59c801`, `0x59c6d0`, `0x59b910`, `0x5a17d0`, `0x58a970`, `0x592530`, `0x63a833`
Confidence: verified
Proof: one
Test: `gx::viewport_matches_binary`

`0x7ad7f0` recentres the rectangle on its centre and calls `0x5c3d90(−w/2, w/2, −h/2, h/2, −500, 500)`,
so x = ±w/2 maps to clip ±1. `0x5a1100` builds a `D3DVIEWPORT9` (X = trunc(x0·W), Y = trunc((1 − y1)·H))
and calls `IDirect3DDevice9::SetViewport`, vtable `+0xbc` (`0x5a11bb`); the D3D render-state writes
never include the clipping state. `0x59c730` passes the same four truncated integers to `glViewport`
and `glScissor` (`0x59c7d5`, `0x59c801`) and runs before every draw (`0x59c6d0`) and clear
(`0x59b910`). Clear slot 0x6c is `0x59b910` for GL and `0x5a17d0` for D3D. `gxApi` registers with the
value `direct3d` (`0x63a833`). The test runs the truncation slice `0x59c786` to `0x59c7cc` of `0x59c730`.

## Root transform

Each paint rebuilds the model's root matrix as `translate(layout scale · position) · rotate(facing about
+Z) · scale(s)` with `s = G48 · (5/3) · modelScale · layoutScale`, where G48 is the screen's y scale and
layoutScale is the frame's effective scale. Nothing is cached: the 4×4 is overwritten whole every time.
The first act of the transform step is to run the frame's `OnUpdateModel` handler when it has one.

Addresses: `0x76d1a0`, `0x76d1a9`, `0x76d1bc`, `0x76d1c6`, `0x76d1cb`, `0x76d1d1`, `0x76d1dd`, `0x76d1fe`, `0x76d205`, `0x76d235`, `0x710650`, `0x41ae70`, `0x80655c`, `0x702690`
Confidence: verified
Proof: one
Test: none

`0x41ae70(1.0)` returns G48 (`0x76d1c6`); `0x76d1cb` multiplies by the float 1.6666666 (`0x3fd55555`),
`0x76d1d1` loads `+0x3a0` and `0x76d1dd` multiplies by `+0x7c`. The position is `+0x7c` times the three
floats at `+0x390` (`0x76d205`). `0x710650(pos, facing, axis (0,0,1), (s, s, s))` writes the identity
and then translate, rotate and scale into `model+0xbc` (`0x76d235`). 5/3 is √((4/3)² + 1), so G48 · 5/3 is
exactly 1 at the 4:3 reference and √(a_ref²+1)/√(a²+1) at aspect a. `0x76d1a9` calls `0x702690` on the
`OnUpdateModel` slot.

## Model unit to pixel

On the orthographic leg one model unit is `1280 · modelScale · layoutScale` FrameXML units, that is
`(5/3) · modelScale · layoutScale · screenHeight` device pixels, at every aspect. The screen is 768
FrameXML units tall and `768a` wide at scale 1. Model (0, 0) lies at the rectangle's bottom-left; +X is
screen right, +Y screen up, and +Z is depth only, so `SetFacing` is a roll in the screen plane. The only
sign flip in the chain negates Z.

Addresses: `0x6f1eb0`, `0x6f1ee3`, `0x6f1eeb`, `0x6f1ef0`, `0x6f1efd`, `0x7a1e00`, `0x41ad70`, `0x41ae40`, `0x41ae60`, `0x764284`, `0x7ad913`, `0x7bdca0`, `0x48b4d0`, `0x48b480`, `0x7ffd68`
Confidence: verified
Proof: one
Test: none

A FrameXML size N becomes `N / (0.75a · 1024)` times the x scale G44 (`0x6f1eeb` to `0x6f1efd`), which is
`N / (768·√(a²+1))` layout units; `Frame:GetWidth` (`0x7a1e00`) is the inverse. The screen root's
rectangle is G44 by G48 layout units (`0x764284`). A layout unit is therefore `768·√(a²+1)` FrameXML units
and the ortho maps one model unit to one layout unit after the root scale: `s · 768·√(a²+1)` = `768 · 5/3 ·
modelScale · layoutScale`. `GetScreenHeight` (`0x48b4d0`) composes the same two scales and returns
`768 / uiScale` at every aspect. `0x7bdca0(M, (1, 1, −1))` scales rows 0 to 2 only.

## Implicit size

A `Model` frame with a width or height of exactly 0.0 takes that size from its model's bounding box:
`max − min` of the box in model units, handed to the layout as layout units with no conversion. With
no model or a model whose data is not resident the size stays 0.0, and the size getter never forces a
load. A negative or nonzero authored size is returned as is. The geometry then fills `G48 · (5/3) ·
modelScale` of the rectangle it implies, anchored at the bottom-left: exactly 1.0 at 4:3.

Addresses: `0x76d080`, `0x76d0d0`, `0x76d08c`, `0x76d097`, `0x76d09a`, `0x76d0a6`, `0x76d0b5`, `0x76d0bb`, `0x76ce00`, `0x76ce57`, `0x76ce69`, `0x713640`, `0x713661`, `0x7103d0`, `0x768420`
Confidence: verified
Proof: one
Test: none

The geometry vtable overrides slots 7 and 8 with `0x76d080` and `0x76d0d0`. `0x768420` gives the
authored width; `fcom` with 0 and `test ah,0x44; jp` fall through only on equality (`0x76d097`,
`0x76d09a`), then `+0x2f4` from the geometry base (frame `+0x318`) must be nonzero and `0x7103d0(0, 0)`
true. The result is `[+0x39c] − [+0x390]` from the base, frame `+0x3c0 − +0x3b4`
(`0x76d0b5`, `0x76d0bb`); the height uses `+0x3c4 − +0x3b8`. `0x76ce00` copies six dwords from
`0x713640` (`0x76ce57` to `0x76ce69`), which reads `MD20 + 0xb4`, then invalidates the layout.

## Model scale from XML

The `scale` attribute exists on `Model` elements only, and it sets the model scale at `+0x3a0`, never the
frame scale. A value that is not greater than 0, or is NaN, raises `Frame %s: Invalid model scale: %s`
and leaves the scale unchanged. Other frame types ignore the attribute. The game builds nine minimap
model frames and a world-map arrow with the same constructor and stores a scale into each after
construction: 0.6 into eight minimap widgets, and `G48 · 5/3` into the minimap's player arrow and the
world-map arrow, which the root transform multiplies in again, giving `s = (G48 · 5/3)² · layoutScale`,
0.668 at 16:9, while the aspect is unchanged since the widget was built.

Addresses: `0x76cac0`, `0x76cb36`, `0x76cb54`, `0x76cb5c`, `0x76cb61`, `0x76cb69`, `0x76de72`, `0x76c938`, `0x4a7a80`, `0x4a7ae3`, `0x4edbc0`, `0x4edeba`, `0x4edf67`, `0x4edfec`
Confidence: verified
Proof: one
Test: none

The stored scale is computed once at construction (`0x41ae70(1.0)` times `5/3`, stored at `0x4edfec` and
`0x4a7ae3`); the paint reads G48 afresh, so after an aspect change the square fails.

The attribute name string occurs once in the image, with one reference at `0x76cb36`. `0x76cac0` runs
with the geometry base as `this`, so its `fstp [edi+0x37c]` at `0x76cb61` is frame `+0x3a0`; the check
`fcom 0.0; test ah,0x41; jne` (`0x76cb54`, `0x76cb5c`) sends zero, negative and NaN to the error at
`0x76cb69`. `SetModelScale` stores `+0x3a0` at `0x76de72`. In the minimap's constructor, `0x76c8e0` is
called at `0x4ede72` (five widgets, loop count 5), `0x4edf1f` (three) and `0x4edfae` (one); the first
two loops store the float 0.6 (`0x4edeba`, `0x4edf67`), the last stores `0x41ae70(1.0)` times 5/3
(`0x4edfec`). The world-map arrow's creator `0x4a7a80` stores the same product at `0x4a7ae3`; the minimap's constructor is `0x4edbc0`.
## Particles in a model frame

A particle in a model frame is drawn in eye space: the emitter installs an identity view and its quad
half-extent is added to the transformed position, so a particle's on-screen size is `2 · half · 768·√(a²+1)`
FrameXML units and carries neither the model scale nor the frame scale, unlike the geometry. The
half-extent takes the emitter's scale only under runtime flag 0x200. The emitters' distance density
factor is 1 for every model frame, and `particleDensity` applies as in the world. The emitters receive seconds: the
milliseconds the frame added are drained from the model at its next update and multiplied by 0.001, and each
emitter's pool holds `trunc(lifespan · rate · 1.15)` particles.

Addresses: `0x7b2a50`, `0x7b2ba3`, `0x7b2ba6`, `0x7b2bab`, `0x7b2bb4`, `0x7b3d20`, `0x7b3e41`, `0x7b5160`, `0x7b5219`, `0x7b55a3`, `0x7b55a8`, `0x7b55b5`, `0x70fb4a`, `0x707680`, `0x707752`, `0x7193dd`, `0x718c67`, `0x718c89`, `0x801360`, `0x7b1dd0`, `0x7b1de5`, `0x81d868`
Confidence: verified
Proof: one
Test: none

`0x7b2ba3` tests bit 0x200 of the emitter flags; clear, `je 0x7b2bb4` skips the multiply by `+0x264`
(`0x7b2bab`), which `0x7b5160` sets to the length of the matrix's first row (`0x7b5219`). The corners
are added after `0x7bca80` has moved the position. Every reader of `+0x264` that sizes a quad is behind
the same bit. The density factor is `clamp(1 − (dist − 50)·0.02, 0.25, 1)` of the distance from the
emitter to `scene + 0x10c`, the translation of the inverse view; with the view `translate(−w/2, −h/2,
0)` of the orthographic leg (the eye argument is 0) that point is the rectangle's centre, within a
fraction of a layout unit of every emitter, so the factor is 1 (`models#spawn-count`). `0x7193dd` adds the milliseconds to the model's counter at `+0x88`; `0x718c67` to `0x718c92` zero the counter and store the count times `0x801360` as the step in seconds, which is the step `0x7b5550` receives. `0x7b1dd0` multiplies lifespan, rate and `0x81d868` (1.15) and truncates (`0x7b1de5`); a spawn stops when the free list is empty.
