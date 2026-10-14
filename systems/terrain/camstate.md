# terrain: camera state

What the scene works out each frame about the camera's surroundings: the environment it stands in, the
liquid it is under or over, and the map object that holds it. These feed the scene driver in `scene`,
the lights in `lights` and the effects that follow the camera.

Roots: `0x6821f0`, `0x6809c0`

## Camera environment

`0x6809c0` samples the liquid at the camera's probe point: the camera's x and y, and the lowest z of the camera and
the four near corners of the frustum. Inside a map object group (`0xc7b748`) it asks the group's
liquid, with the point moved into the group's space; outdoors it asks the terrain query
`0x6723d0`. The depth `0xc7bc60` is the liquid height less the probe z, 0 when dry. The state
`0xc7f288` becomes the liquid type, 0xf when dry. A change to type 0 or 1 scatters the drift
cloud and sets the size base 1/36, to type 2 scatters it and sets 1/9, to type 3 clears the
cloud's enable byte; leaving liquid forces a lighting rebuild.

Addresses: `0x6809c0`, `0x6809e2`, `0x6809e8`, `0x6809f2`, `0x680a03`, `0x680a26`, `0x680a3e`, `0x680a74`, `0x680a87`, `0x680a9d`, `0x680aad`, `0x680abe`, `0x680ae7`, `0x680ba4`, `0x680afa`, `0x680b0d`, `0x680b1d`, `0x680b4c`, `0x680b5c`, `0x680b75`, `0x680b7f`, `0x680b8b`, `0x6b9f10`, `0x6723d0`, `0x68e720`, `0x68e670`, `0x6d2210`
Confidence: verified
Proof: one
Test: `terrain::camera_probe_z_delta_a_difftest`, `terrain::camera_probe_z_delta_b_difftest`

The loop at `0x6809e2` takes the four values at `0xc7bce0 + 0xc × i` (the near corners' z): a
value at or below the running z (or NaN) replaces it (`test ah,0x41; je` skips a larger one).
`0x680a3e` moves the probe into the group by `[group + 0xd4]`; `0x680a74` calls `0x6b9f10`
with the group of `0xc7cd88`; the depth at `0x680a87` is the returned height less the
transformed z `[ebp-0x24]`; the outdoor depth at `0x680aad` less the probe z `[ebp-0xc]`. State
0xf is compared at `0x680abe`. The jump at `0x680ae7` indexes `0x680ba4` by `state & 3`: cases 0
and 1 `0x680aee`, 2 `0x680b31`, 3 `0x680b6f`; the first two need the type to differ from
`0xc7f288` and world flag 0x2000000.

## Camera liquid state

`0x672470(depth out)` returns the liquid type at the camera, 0xf when dry, and stores the depth in
the out argument when it is nonnull. Two writers set the state: `0x6806d3` once at start-up (0xf) and
`0x680b81` every frame in `0x6809c0`. Nine calls and seven direct reads use it, and
nothing else: the order of the two model transparent passes around the water (`0x4836d1`), the sky
dome and the clear colour (`0x6812a4`, `0x68123f`), the drift cloud (`0x66feec`, `0x6701ec`), the
light selection (`0x6d226e`), the magma fog snap (`0x6ceec1`), the glare fade (`0x6cf810`), the sound
reverb (`0x511a8b`) and the effect passes (`0x6cb16d`, `0x6cb5c6`, `0x6cc5c2`, `0x6cc63f`).

Addresses: `0x672470`, `0x672474`, `0x67247b`, `0x6806d3`, `0x680b81`, `0x680ac3`, `0x4836d1`, `0x4836d6`, `0x511a8b`, `0x511a92`, `0x511aa3`, `0x6cb16d`, `0x6cb5c6`, `0x6cc5c2`, `0x6cc63f`, `0x6ceec1`, `0x6ceec8`, `0x6cf810`, `0x6d226e`, `0x6d2277`, `0x6d2817`, `0x66feec`, `0x6701ec`, `0x68123f`, `0x6812a4`, `0x6ba16c`, `0x6ba153`, `0xc7f288`, `0xc7bc60`
Confidence: verified
Proof: one
Test: none

Values: 0xf dry, 0 water, 1 ocean, 2 magma, 3 slime. Type 1 comes only from the terrain query:
the group query's byte table (`0x6ba16c`: 0, 2, 1, 1) sends it to the bare miss `0x6ba153`.
`0x672470` takes no position, so a caller cannot ask about another point.
The state has no hysteresis: each frame stores a fresh value. The consumers mask the word differently: `0x6d2277` takes the low
nibble, `0x6d2817` tests the whole dword for 1, and `0x6ceec8` keeps 0xf and masks the other
values with 3. `0x4836d1` compares with 0xf: dry draws the second transparent pass, the water, the
weather and the first pass; submerged draws the first pass, the weather, the water and the second.
`0x511a8b` discards the depth, compares the type with the camera's stored one (`+0x250`) and, on a
change, tells the sound engine whether the camera is submerged (`0x458650`). `0x6cb16d` and `0x6cb5c6`
raise a blur floor to 0x54 when submerged. Only `0x511a8b` and `0x6cf810` pass an out pointer, and only the glare fade uses the depth. The
census is by raw scan of calls and addresses; the getter's address appears as a data word nowhere.

## Camera down-ray

`0x6821f0` finds the map object that holds the camera and stores it in `0xc7b748`. With world flag
0x100 set, it traces a segment from the camera 1760 yards down the z axis against the terrain
(`0x69c320`) and against the map objects (`0x6be250`), both fractions starting at 1.0. A map
object hit counts when the terrain was missed or its fraction is not larger than the terrain's;
strictly nearer terrain wins. A counted hit becomes `0xc7b748` and its model and group join the
window list.

Addresses: `0x6821f0`, `0x6821f6`, `0x682201`, `0x68223f`, `0x682264`, `0x682278`, `0x682291`, `0x6822a5`, `0x6822af`, `0x682345`, `0x68234f`, `0x68235f`, `0x69c320`, `0x6be250`, `0x6820b0`
Confidence: verified
Proof: one
Test: `terrain::world_query_consumer_difftest`

`0x682201` tests bit 8 of the world flags; the segment's end z is the camera z less 1760.0
(`0x68223f`, constant `0x80fec8`). `0x6822a5` compares the terrain fraction `[ebp-0xc]` with the
object fraction `[ebp-0x10]` (`test ah,5; jp` keeps the hit for a larger, equal or NaN
terrain fraction) and `0x6822af` drops it otherwise. `0x6822b3` to `0x6822f6` format a debug text
into a 64-byte buffer. `0x682345` stores the claim; `0x68234f` and `0x68235f` append the model and
the group index (when not 0xffff) to the window list `0xc7cd84`.

## Camera group set

Each frame `0x6821f0` rebuilds the claim `0xc7b748` and the group set (capacity `0xc7cd80`, count
`0xc7cd84`, data `0xc7cd88`). Both are zeroed on entry, before the map objects flag is tested, so
no frame keeps the last one: a frame without the flag, without a map object floor below the camera,
under nearer terrain or over an exterior group leaves them empty. A counted hit appends the probe's
group and then its other group when that is not 0xffff, each once, so the set holds 0, 1 or 2 groups.
Nothing else fills it, and the liquid probe, the interior fog and the inside seed all read it.

Addresses: `0x6821f0`, `0x682201`, `0x682207`, `0x68221b`, `0x682303`, `0x682345`, `0x68234a`, `0x68234f`, `0x682357`, `0x68235f`, `0x6820b0`, `0x6820d0`, `0x66fc4a`, `0x68206f`, `0x682020`, `0xc7b748`, `0xc7cd84`, `0xc7cd88`
Confidence: verified
Proof: one
Test: none

`0x682201` and `0x682207` store 0 in the claim and the count; `0x68221b` leaves for a clear flag with them
zero, and `0x682303 je` and `0x68234a je` skip the appends. `0x6820b0` scans the data for the value before it
appends (`0x6820d0`), and its two calls are `0x68234f` and `0x68235f`; `0x682357 cmp ecx,0xffff` guards the
second. The claim has four writers: the start-up reset `0x66fc4a`, the two above and `0x68206f`, in
`0x682020`, which no call, jump or stored address reaches (the census finds `0x685d70`'s two stored
addresses as its control).

## Camera probe group

The liquid probe `0x6809c0` samples one group. With the claim set, it moves the probe point into the
instance's space and asks `0x6b9f10` about the group at `0xc7cd88[0]`, never reading the count. The
state starts at 0xf: an unresolved group, a group without flag 0x1000, a hole and a point above the
surface all leave it there, and the terrain query `0x6723d0` runs only with no claim. A camera inside a
map object therefore never reads the terrain's liquid. The type stored is `0x6b9f10`'s, so the water
tile 4 reads 0, and the depth is the surface less the point's z, both in the instance's space.

Addresses: `0x6809c0`, `0x680a03`, `0x680a1f`, `0x680a26`, `0x680a30`, `0x680a3e`, `0x680a43`, `0x680a59`, `0x680a60`, `0x680a74`, `0x680a79`, `0x680a87`, `0x680a8f`, `0x680a9d`, `0x680ab8`, `0x680abb`, `0x680b81`, `0x6b9f10`, `0x6723d0`, `0x6a4a00`, `0xc7cd88`, `0xc7b748`
Confidence: verified
Proof: one
Test: none

`0x680a1f` seeds the type 0xf and `0x680a26` the depth 0. `0x680a30 je 0x680a8f` is the only way to the
terrain leg (`0x680a9d`); `0x680a43` and `0x680a48` load `data[0]`. Every exit of the map object leg joins at
`0x680abb` with the type unchanged, which `0x680b81` stores. The call at `0x680a74` ignores the result
and the type decides (`0x680a7c`). The depth of the map object leg subtracts `[ebp-0x24]`, the probe's z
after the `+0xd4` matrix (`0x680a87`); the terrain leg subtracts the world z `[ebp-0xc]` (`0x680aad`).
