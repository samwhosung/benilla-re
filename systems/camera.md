# camera

The camera object and the matrices it gives the graphics device.

A camera (`CCamera`, allocation tag `HCAMERA`) is a 0x170-byte object holding nine properties:
eye, target, distance, far and near clip, field of view, roll, pitch and yaw. Each property is a
`CBaseManaged` object of its own; an angle keeps its cosine and sine. The camera builds a
perspective projection and a view for the device, and code outside reads and writes its
properties by id through four accessors.

The code is two object files from `0x7abbc0` through `0x7adf70`: the property machinery, then the
class with two free transforms the interface uses, an orthographic setup for a rectangle and a
screen-to-world unprojection. The particle density setting beside them is in the next file,
`models`' particle registry; its entry is in this doc.

Roots: `0x7ac930`, `0x7acf10`, `0x7ada40`, `0x7ad2c0`, `0x7ad7f0`, `0x7abf80`, `0x7ac0b0`,
`0x7ac250`, `0x7ac3a0`, and the static initializers `0x7abbc0`, `0x7ac5a0`, `0x7ac5c0`, `0x7ac5f0`,
`0x7ac610`, `0x7adf20`, `0x7adf40`.

## Properties

The constructor builds nine properties inside the camera and registers each under an id in a
nine-slot array:

| id | property | object | value | default |
|---|---|---|---|---|
| 7 | eye | `+0x20`, vector | `+0x3c` | (100, 0, 0) |
| 8 | target | `+0x48`, vector | `+0x64` | (0, 0, 0) |
| 1 | distance | `+0x70`, scalar | `+0x8c` | 100 |
| 2 | far clip | `+0x90`, scalar | `+0xac` | 5000 |
| 3 | near clip | `+0xb0`, scalar | `+0xcc` | 8 |
| 0 | pitch | `+0xd0`, angle | `+0xec` | 0 |
| 4 | field of view | `+0xf8`, angle | `+0x114` | π/2 |
| 5 | roll | `+0x120`, angle | `+0x13c` | 0 |
| 6 | yaw | `+0x148`, angle | `+0x164` | 0 |

Addresses: `0x7ac930`, `0x7abc90`, `0x7abcf0`, `0x7ac640`, `0x7ac810`
Confidence: verified
Proof: one
Test: none

The ids are the middle of the three values pushed before each call to `0x7abc90` (vectors, type
3) and `0x7abcf0` (scalars and angles, type 6) from `0x7aca54` to `0x7acae8`. The defaults are
`0x81d7b0` (100.0) for eye x and distance, `0x81d7b4` (5000.0), `0x81d7b8` (8.0), and
`0xcf5738` for the field of view, which `0x7ac620` sets to π·0.5. Names follow the uses:
`0x7ac640` aims along target − eye and passes `+0xcc` and `+0xac` to the perspective builder
as near and far, which it requires in that order. Pitch and yaw are named from `0x7ac810`,
whose direction is `(cos +0xec · cos +0x164, cos +0xec · sin +0x164, sin +0xec)`.

## Angle properties

Setting an angle stores it reduced by `0x7abbe0`, then recomputes its cosine at `+0x20` and its
sine at `+0x24` of the property, every time. `0x7abbe0` returns `x − 2π·trunc(x/2π)` for
`x ≥ 0` and `x − 2π·(trunc(x/2π) − 1)` for `x < 0`, computed at the client's 53-bit precision.
For `|x|` below 81.68141 the result lies in `[0, 2π]`: 2π maps to 2π, −2π to 0, and −0.0 stays
−0.0. The stored angle changes, and the property is marked changed, only when it differs from the
old value or is NaN.

Addresses: `0x7abbe0`, `0x7acbc0`, `0x7acc40`, `0xcf5730`, `0xcf5734`
Confidence: verified
Proof: two
Test: `camera::camera_normalize_2pi`, `camera::camera_angle_cossin`

`0x7abbe0`: `fld [0xcf5730]; fmul x`, `__ftol` (`0x40a2b0`, truncation), `fild;
fmul [0xcf5734]`; the sign test is `fcomp 0.0; test ah,5; jp 0x7abc14`, which falls through only
for `x < 0`, into `fsub [0xcf5734]` (`d8 25`); then `fsubr x` (`d8 6d 08`) gives `x − t`.
`ret 4`. The globals hold 2π and 1/2π as floats (`0x40c90fdb`, `0x3e22f983`), set by the static
initializers `0x7ac5b0` (π + π) and `0x7ac5d0` (1.0 / 2π). Their product is `1 − 1.25·10⁻⁸`,
so just above a multiple of 2π the truncated turn count can come out one short. That first
happens at 13 turns: 81.68141 gives one float above 2π, −81.68141 gives −4.8·10⁻⁷, and the
excursion grows with `|x|`. In `0x7acbc0` and `0x7acc40`, `fsincos` leaves the cosine in `st0`,
and the first `fstp` writes `+0x20` (`0x7acc28`, `0x7acc8c`). The change test is
`test ah,0x44; jnp` around the store. `camera_normalize_2pi` runs `0x7abbe0` whole, and
`camera_angle_cossin` runs `0x7acc87` to `0x7acc90`.

## Property updates

A property holds a type, flags and a producer (callback, context, ordering key) before its
value. Slot 3 stores the value, setting flag 0x08 when it changes. Slot 1 stores the producer's
result for a copy of the value; slot 2 raises error 0x57 if that result differs. Both clear flag
0x04. The getters `0x7abf80` (vectors) and `0x7ac0b0` (scalars and angles) run slot 2 or slot 1
on a property with flag 0x04, slot 2 when flag 0x02 is set, before copying its value out.
The setters `0x7ac250` and `0x7ac3a0` refuse a verify-only property, detach its producer, and call
slot 3. No property in this build ever has a producer or flag 0x04: each is a plain stored value.

Addresses: `0x7adac0`, `0x7adae0`, `0x7adb20`, `0x7adcd0`, `0x7add20`, `0x7add70`, `0x7abf80`, `0x7ac0b0`, `0x7ac250`, `0x7ac3a0`, `0x7abc20`, `0x7abd10`, `0x7ad1b0`, `0x7ac410`
Confidence: verified
Proof: one
Test: none

The vtables are `0x81d7c8` (scalar), `0x81d7d8` (vector) and `0x81d7fc` (angle), slots 0 to 3:
destroy, evaluate, verify, set. The accessors raise error 0x57 for a null camera, an id past the
slot count, an empty slot or a wrong type. `0x7ac250` keeps each component whose bit is set in
its mask and takes the rest from its argument. `0x7abc20` stores the type and flags and fills
the slot, adding the property to the update list by its `+0x18` key when flag 0x01 is set
(`0x7abd10`). `0x7abc90` and `0x7abcf0` call it with types 3 and 6; the camera registers every
property with flags 0. Within the object file, the only store of flag 0x04 is `0x7ac459`, in
`0x7ac410`, which also attaches a producer and is unreached; `0x7ad1b0`, called only by the
clone, and the setters store a null producer. The base property vtable `0x81d7f0` holds its
destructor and two empty slots, `0x7acb30` and `0x7acb40`.

The slot array is allocated with the tag `.PAVCBaseManaged@@` (`0x87d5e8`), the type name of
`CBaseManaged *`: the properties, not the camera, are the `CBaseManaged` objects.

## Lifetime

`0x7ac930` allocates and builds a camera with the defaults and returns its handle from
`0x41af10`. `0x7acf10` does the same and then copies each property's value from a source camera
through the property's set slot. Destruction runs vtable `0x81d7c0` slot 0, `0x7acd80`: it
destroys the properties, unlinks them, frees the slot array, and frees the camera when bit 0 of
its argument is set.

Addresses: `0x7ac930`, `0x7acf10`, `0x7acd80`, `0x7ace80`, `0x7ad1d0`, `0x7adb90`, `0x41af10`
Confidence: verified
Proof: one
Test: none

Both allocate `0x170` bytes with the tag `HCAMERA` (`0x87d5d8`). `0x7ac930` has two call sites
outside the object file and `0x7acf10` one. The clone returns 0 and raises error 0x57 for a null
source. Each property copy is `0x7ad1b0(0, 0, 0)` and then slot 3 with the source's value
address (`0x7ad08e` to `0x7ad19a`).

The constructors are `0x7ad1d0` and `0x7adc50` for the camera's base (slot array `0x7adb90`,
update list) and `0x7acb10`, `0x7ada80`, `0x7adc80` and `0x7acbc0` for the base, scalar, vector
and angle properties. Each property's slot 0 (`0x7acb50`, `0x7ade00`, `0x7ade70`, `0x7acca0`)
unlinks it and frees it when bit 0 of its argument is set; `0x7ace30` unlinks without freeing.
`0x7acd10` is slot 0 of the camera's base vtable `0x81d7e8`; `0x7ace80` and `0x7adb70` tear the
base down. List steps go through `0x7ac570`, `0x7adf70` and `0x7adee0`. Slot 1 of both camera
vtables is the shared `0x442a70`, which returns 0.

## View and projection

`0x7ada40` checks the camera and calls `0x7ac640`, which sets the device's two matrices from the
camera and a viewport rectangle `r` of four floats. The projection is `0x5c3cc0` with the field
of view, the aspect `(r[3] − r[1]) / (r[2] − r[0])`, near and far; it scales x by
`1/(aspect · tan θ)` and y by `1/tan θ`, with `θ = fov / (2·√(aspect² + 1))`. The view is
`0x5c3e70` looking from the origin along `target − eye`, with up
`(sin yaw · sin roll, −cos yaw · sin roll, cos roll)`. The eye position is not in the view.

Addresses: `0x7ada40`, `0x7ac640`, `0x5c3cc0`, `0x5c3e70`, `0x58b040`, `0x58b050`
Confidence: verified
Proof: one
Test: `camera::camera_view_lookat`, `gxumath::perspective_difftest`, `gxumath::lookat_v1_difftest`

The aspect is `fdivp st(1),st` (`de f9`, `0x7ac679`), `st(1)/st(0)`. In `0x5c3cc0`, `0x5c3d43`
divides the angle by `√(aspect² + 1)` before `fmul 0.5` and `fptan`, and `0x5c3d5b`/`0x5c3d62`
store the two scales. It refuses a field of view outside `(0, π)`, a non-positive aspect or
`near ≥ far` with error 0x57 and leaves the matrix alone. With the default π/2 at an aspect of
4/3, `θ` is 27°, a vertical field of 54°. Up reads `+0x16c·+0x144`, `−(+0x168·+0x144)` and
`+0x140` (`0x7ac778` to `0x7ac7dc`). `0x58b040` and `0x58b050` pass the matrix to the device's
vtable slots `+0x70` and `+0x74`. The test runs `0x7ac640` to `0x7ac7df`, with `0x5c3cc0` stubbed.

`gxumath::perspective_difftest` runs `0x5c3cc0` whole and `gxumath::lookat_v1_difftest` runs `0x5c3e70` whole.

## Unprojection

`0x7ad2c0` maps a screen fraction `(a, b)`, each in `[0, 1]` from the bottom-left, to a point on
the near plane and a point on the far plane, written through `ecx` and `edx`. It reads two device
matrices, builds the eight frustum corners `c0` to `c7` with `0x5c43b0`, and interpolates: near is
`lerp(lerp(c0, c1, b), lerp(c3, c2, b), a)`, far the same over `c4`, `c5`, `c7`, `c6`. It does
not read a camera.

Addresses: `0x7ad2c0`, `0x58b090`, `0x58b0b0`, `0x5c43b0`
Confidence: verified
Proof: one
Test: `camera::camera_frustum_split`, `gxumath::frustum_corners_difftest`

The range test accepts 1.0: for `a = 1`, `fcomp` sets only C3, `test ah,0x41` leaves one bit, and
`jp 0x7ad317` falls through (`0x7ad2f5`); a value above 1 or NaN gives error 0x57. `0x58b0b0` copies
the top of the stack at device `+0x1ac8`, `0x58b090` the matrix at device `+0xf50`. `0x5c43b0`
builds `c0` to `c3` from the clip-space corners `(−1,−1)`, `(−1,1)`, `(1,1)`, `(1,−1)` at z = −1 and
`c4` to `c7` at z = +1; the camera's perspective puts the near plane at −1 (its depth term is
`−2·near·far/(far − near)`, `0x8038dc`). The near point is stored at `0x7ad6a4` and the far point at
`0x7ad7c4`. It has one call site. The vector work goes through `0x4078e0` (construct with one
value), `0x4549a0` (set), `0x621160` (add), `0x4841f0` (subtract) and `0x5f8cf0` (scale), and the
corner array is destroyed by `0x4044b0` with the empty `0x4621a0`. The test runs `0x7ad2c0` whole
with `0x5c43b0`, `0x58b090`, `0x58b0b0` and `0x4078e0` stubbed.

`gxumath::frustum_corners_difftest` runs `0x5c43b0` whole.

## Orthographic setup

`0x7ad7f0` takes a rectangle `r` in `ecx` and a point `p` in `edx`. With `cx = (r[1] + r[3])/2`
and `cy = (r[0] + r[2])/2`, it sets an orthographic projection over the rectangle moved to that
centre, depth ±500, Z negated, and a view translated by `(p[0] − cx, p[1] − cy, 0)`. When
`0x589ca0` returns 1 it also moves the projection by half the rectangle's size over the screen's,
from `0x58a250`. Its stack argument is unused.

Addresses: `0x7ad7f0`, `0x5c3d90`, `0x7bdca0`, `0x7bdc40`, `0x589ca0`, `0x58a250`
Confidence: verified
Proof: one
Test: `camera::camera_ortho`, `terrain::liquid_tex_matrix_difftest`, `gxumath::ortho_difftest`, `cmath::mat34_scale3_rows`

`0x81d7bc` is 500.0, negated for the near side (`0x7ad8dd`). The Z flip is `0x7bdca0` with
`(1, 1, −1)` (`0x7ad8fe` to `0x7ad913`). The shift subtracts `0.5·(r[3] − r[1])/(s[3] − s[1])`
from the matrix's x translation and adds `0.5·(r[2] − r[0])/(s[2] − s[0])` to its y
(`0x7ad946` to `0x7ad976`), `s` being the four floats `0x58a250` writes. Both rectangle readers,
this and `0x7ac640`, take x from `r[1]` and `r[3]`. It has four call sites. `0x589ca0` returns
the device's dword at `+0x1fc` through a tail call to `0x592120`. The test runs `0x7ad7f0`
whole with `0x589ca0` returning 0, and the shift as `0x7ad946` to `0x7ad979`.

`gxumath::ortho_difftest` runs `0x5c3d90` whole and `cmath::mat34_scale3_rows` runs `0x7bdca0` whole.

## Particle density

`0x7adfb0` clamps its argument to `[0, 1]` and stores it in `0x87d5fc`, whose default is 1.0:
below 0 gives 0, and 1 or more, or NaN, gives 1. `0x7ae000` returns it. The `particleDensity`
and `spellEffectLevel` options write it.

Addresses: `0x7adfb0`, `0x7ae000`, `0x87d5fc`, `0x688fb0`, `0x689510`
Confidence: verified
Proof: one
Test: `camera::camera_clamp01`

Two `fcomp`, each `test ah,5; jp`: the first falls through to the 0 store only for `x < 0`, the
second to the `x` store only for `x < 1`. The option handlers `0x688fb0` and `0x689510` are
registered with the names `particleDensity` (default `1.0`) and `spellEffectLevel` (default `2`),
and are the only callers of `0x7adfb0` (three call sites). `0x7ae000` has two call sites. The test
runs `0x7adfb0` whole.

## Object files and initializers

The code is two object files, each opening its run of static initializers in the C runtime's table
with +∞: the property machinery from `0x7abbc0` through `0x7ac5d0`, and the class from `0x7ac5f0`
through `0x7adf70`. Seven run from consecutive slots `0x82d978` to `0x82d990`: the first file's
`0x7abbc0` (+∞ into `0xcf572c`), `0x7ac5a0` (2π into `0xcf5734`) and `0x7ac5c0` (1/2π into
`0xcf5730`); then the second file's `0x7ac5f0` (+∞ into `0xcf573c`), `0x7adf20` (2π into
`0xcf5744`), `0x7adf40` (1/2π into `0xcf5740`) and `0x7ac610` (π/2 into `0xcf5738`). Each is a
`jmp` thunk to its body 16 bytes on. The next slot holds `0x7adf90` (+∞ into `0xcf5748`), which
opens the particle registry's file.

Addresses: `0x7abbc0`, `0x7abbd0`, `0xcf572c`, `0x7abbe0`, `0x7ac5d0`, `0x7ac5f0`, `0x7adf70`, `0x7ac5a0`, `0x7ac5b0`, `0x7ac5c0`, `0x7ac5d0`, `0x7ac5f0`, `0x7ac600`, `0x7ac610`, `0x7ac620`, `0x7adf20`, `0x7adf30`, `0x7adf40`, `0x7adf50`, `0x7adf90`, `0x7adfa0`, `0xcf5748`, `0x7adfb0`, `0x7ae000`, `0x7af1b0`, `0x827000`
Confidence: verified
Proof: one
Test: `camera::camera_normalize_2pi`

Each file includes the shared header, so its run opens with its +∞ (`systems/README`, Object files).
Slot `0x82d978` holds `0x7abbc0`, which copies the +∞ at 0x81d7a0; `0x7ac5a0` doubles the π 4 bytes
on. `0x7ac5f0` copies the +∞ at 0x81d7a8, and `0x7adf20` and `0x7ac610` read the π at 0x81d7ac.
`0x7abbe0` reads `0xcf5730` and `0xcf5734`, which only the first file's initializers write.
`0x7adf90` copies the +∞ at 0x81d80c, and the registry's 2π `0x7af1b0`, at slot `0x82d998`, doubles
the π at 0x81d810, so the code between, `0x7adfb0` and `0x7ae000`, is the registry's. The class's
file ends past its last initializer with `0x7adf70`, whose one caller is the class's `0x7acd2e`. The
code below `0x7abbc0` allocates with the sound-file cache's tags. Only `0xcf5730`, `0xcf5734` and
`0xcf5738` (read by `0x7ac930` and `0x7acf10`) are read; the other four are written and never read.
The test runs `0x7ac5b0` and `0x7ac5d0` whole.

## Unreached functions

Nothing live reaches these functions: `0x7ac810` and `0x7ac8a0`, which place the eye or the
target from the angles and the distance; the update pass `0x7abdf0` and its wrapper `0x7ada60`;
`0x7ac410`, which attaches a producer, and its seven type wrappers; and the accessors for
property types the camera never registers. Their rows, of class `unreached`, keep a one-line
meaning each, marked `inferred` because no derivation of it is kept.

Addresses: `0x7ac810`, `0x7ac8a0`, `0x7ada60`, `0x7abdf0`, `0x7abc50`, `0x7abc70`, `0x7abcb0`, `0x7abcd0`, `0x7abe50`, `0x7abea0`, `0x7abf10`, `0x7abff0`, `0x7ac060`, `0x7ac110`, `0x7ac150`, `0x7ac1a0`, `0x7ac1f0`, `0x7ac300`, `0x7ac350`, `0x7ac3f0`, `0x7ac410`, `0x7ac4b0`, `0x7ac4d0`, `0x7ac4f0`, `0x7ac510`, `0x7ac530`, `0x7ac550`
Confidence: verified
Proof: one
Test: none

The census is a scan for `call`/`jmp` targets and for each address as a dword anywhere in the
image; the same scan finds the vtable slot `0x7acb30` at `0x81d7f4`. `0x7abdf0` is called only
from `0x7ada60`, and `0x7ac410` only from the seven type wrappers `0x7ac3f0` and `0x7ac4b0` to
`0x7ac550`; every other function listed has no reference at all.
