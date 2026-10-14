# lighting: weather

The weather manager: the message that sets a type and a grade, the two ramps that carry the grade
to the effects and the sky, the per-frame driver that follows the player, the cached ground grid
the particles land on, and the gate that hides weather indoors. The manager is the object at
`[0xc6326c]`; its effects, the particles, are `precipitation`.

Roots: `0x67b740`, `0x67b900`, `0x67baf0`, `0x67bac0`, `0x67be40`, `0x67c720`, `0x67bdd0`, `0x67bc40`

## Weather manager

The world builds one manager at start-up and frees it at shutdown. It holds the grade and density
ramps, the pending type, one effect per type, the ground grid and the player's motion:

| offset | holds |
|---|---|
| `+0x0`, `+0x4`, `+0x8` | grade: target, ramp start value, current |
| `+0xc`, `+0x10`, `+0x14` | density: target, ramp start value, current |
| `+0x18`, `+0x1c` | grade and density ramp start, in milliseconds |
| `+0x20` | pending type, −1 for none |
| `+0x24`, `+0x25` | bytes: ramp now, smooth |
| `+0x28`, `+0x2c`, `+0x30` | rain, snow and sand effects |
| `+0x34` | ground grid |
| `+0x38` to `+0x4c` | the player's and the transport's motion samples |
| `+0x50`, `+0x5c` | the player's and the transport's velocity |
| `+0x68`, `+0x74` | the player's position; byte: its `UNIT_FIELD_FLAGS` bit 0x100000 |
| `+0x78`, `+0x7c` | heading and speed |
| `+0x80` | byte: driven at least once |

Addresses: `0x67b740`, `0x67b900`, `0x66f9dc`, `0x66fbb0`, `0x67c900`, `0x67c760`, `0x67bc10`
Confidence: verified
Proof: one
Test: none

`0x66f9dc` constructs it and `0x66fbb0` destroys it. `0x67b740` zeroes the fields, sets `+0x20`
to −1, builds the grid with `0x67c900` into `+0x34`, and makes both sample lists empty.
`0x67b900` tears the effects down, deletes the grid and frees every sample. `0x67c760` returns
the grid. `0x67bc10` reports the live type: 1 with rain, 2 with snow, 3 with sand, else 0.

## Weather settings

`weatherDensity` (0 to 3) picks the effect scale K: 0.1, 0.33, 0.66 or 1.0, and a change sets the
manager's ramp-now byte so the effects take the new scale on the next frame. `useWeatherShaders`
at 0 sends every effect to its fixed-function draw. The `Internal` option `force weather type on`,
read at start-up, sets a type of 1 to 3 at grade 1.0 with no ramp and locks the weather: every later
message and every reset is ignored.

Addresses: `0x67b870`, `0x67b7f1`, `0x67b81d`, `0x8680ec`, `0xc7b588`, `0xc7b584`, `0x67b839`, `0x67b857`, `0x67bac0`, `0x67bc40`
Confidence: verified
Proof: one
Test: none

`0x67b870` stores 0x3dcccccd, 0x3ea8f5c3, 0x3f28f5c3 or 0x3f800000 in `0x8680ec`, writes the
range error for any other value, and sets `[0xc6326c]+0x24` when the manager exists. The image
holds 0.66 there before the setting runs. `0x67b839` reads the option and `0x67b857` calls
`0x67baf0(type, 1.0, 0)`, then stores 1 in `0xc7b584`; `0x67baf0` and `0x67bac0` return at once
while it is set. Each effect's constructor clears its shader byte when `[0xc7b588]+0x28` is 0, and `0x67bc40`
returns the live effect's shader byte for the console's weather report.

## Weather message

The weather message, 0x2f4, carries a u32 type, a float grade, a u32 sound and a byte; the byte at
0 means smooth. SetWeather ignores a type outside 0 to 3. A type that differs from the live one
becomes pending. The grade clamps to [0, 1] and becomes the target; the ramp starts from the old
target, not from the current grade. Without smooth the current grade jumps to the target and the
ramp-now byte is set. The density target is the grade clamped to [0, 0.25], its start the ramp's
start value clamped the same way, and both ramps restart now.

Addresses: `0x48f690`, `0x48fa99`, `0x48faa5`, `0x67baf0`, `0x6734f0`, `0x673530`, `0x673570`, `0x6735b0`
Confidence: verified
Proof: one
Test: none

`0x48f690` reads the four fields and passes `sete` of the byte as smooth (`0x48fa99`) to
`0x67baf0` (`0x48faa5`). `0x67baf0` writes `+0x25`, compares the type with `0x67bc10`, stores the
old `+0x0` in `+0x4` and the grade in `+0x0`, and, without smooth, the grade in `+0x4` and `+0x8`
and 1 in `+0x24`. It reads `OsGetAsyncTimeMs` into `+0x18` and `+0x1c`. The four console commands
`0x6734f0` to `0x6735b0` call it too.

## Weather ramps

Each ramp is a clamped linear move over a time that scales with the distance:
g = from + (to − from)·clamp(s/(|to − from + 0.001|·10), 0, 1), s the seconds since the ramp
started, so a full move takes about 10 s. The effects receive x = max((g − 0.25)·4/3, 0), so no
precipitation falls below a grade of 0.25. The density ramps the same way with the span
|(to − from)·4 + 0.001|, so its 0.25 range also takes about 10 s; it goes to the light state's
weather density and rebuilds the light table.

Addresses: `0x67bc70`, `0x67bc9b`, `0x67bca7`, `0x67bcb3`, `0x67bcb9`, `0x67bcc0`, `0x67bcc8`, `0x67bcce`, `0x67bcfc`, `0x67bd09`, `0x67bd16`, `0x67bd3e`, `0x67bd62`, `0x67bd65`, `0x67bd72`, `0x674ba0`, `0x801360`, `0x801624`, `0x80b058`, `0x80306c`
Confidence: verified
Proof: one
Test: none

`0x67bc70(from, to)` takes the milliseconds since `+0x18` times 0.001, adds 0.001 inside the
`fabs`, multiplies by 10 and divides (`0x67bcb9`, `DE F9`, the elapsed time over the span).
`0x674ba0` returns `from` for t < 0 and caps t at 1. `+0x8` takes g; `0x67bcc8` and `0x67bcce`
apply −0.25 and ×4/3, and x reaches `0x6749e0`, `0x6776c0` and `0x679380` for the live effects.
The density leg multiplies its span by 4 (`0x67bd3e`), stores `+0x14`, copies it to
`0x6d48b0()+0x40` and calls `0x6d2210` (`daynight#storm-weight`). The ramp clears `+0x24`.

## Weather driver

Each frame the driver samples the player's motion, then takes the heading from the player's
velocity when its horizontal speed is at least 1 and the unit flag is clear, else from the
player's facing; the speed is the player's movement speed while the transport is at most √2 (v² ≤ 2, NaN
included), else the transport's horizontal speed. A pending type with live particles and smooth on fades the
old effect: the grade ramps toward min(target, 0.25) and each effect is told to stop. Otherwise
the effects are torn down and the pending type is built. A grade or density still off its
target ramps toward the full target.

Addresses: `0x67be40`, `0x66ff06`, `0x67be4b`, `0x67bea6`, `0x67bebf`, `0x67bede`, `0x67bef0`, `0x67beff`, `0x67bf1b`, `0x67bf5a`, `0x67bf66`, `0x67bf85`, `0x67bfa9`, `0x67bfb9`, `0x67bfde`, `0x67bfec`, `0x67c01b`, `0x67c052`, `0x67c085`, `0x67c0b8`, `0x67c0cf`, `0x67c0e3`, `0x67c102`, `0x67c138`, `0x67c13e`, `0x67c6e0`, `0x67ba50`, `0xc7b554`
Confidence: verified
Proof: one
Test: none

`0x67bede` is `atan2(vy, vx)`, plus `[0xc7b554]` (2π) below 0 (`0x67bef0`); `0x67beff` calls the
facing slot. `0x67bf5a` compares the transport's squared speed with 2.0 and `0x67bf66` tests `ah,0x41`; `0x67bf85` reads
`[[player+0x118]+0x84]`. `0x67c6e0` counts live particles. The fade sets bytes `+0x1` and `+0x58`
of each effect (`0x67bfec`). Teardown `0x67ba50`; types 3, 2, 1 build sand `0x679140(40, 25)`,
snow `0x677420(90, 60)` and rain `0x674620(130, 75)`; then `+0x18` = now, `+0x4` = `+0x8`,
`+0x20` = −1 (`0x67c0cf`). `0x67c0e3` recentres the grid on the camera; `0x67c102` tests 2⁻²².

## Player motion

The driver keeps two lists of position steps, the player's and the transport's, each step with
the milliseconds since the last frame, newest last. First, walking a list from its oldest step,
each step at or after the one where the running sum reaches 150 ms frees one step from the front.
Then the frame's step is added, and each velocity is the summed step over (summed milliseconds +
1)·0.001 seconds. When the player has no movement velocity and the transport is at most √2 (v² ≤ 2, NaN
included), the player's velocity is zero at once.

Addresses: `0x67c150`, `0x67c180`, `0x67c2b5`, `0x67c2cf`, `0x67c3c8`, `0x67c4a2`, `0x67c680`, `0x67d740`, `0x61eec0`, `0xc7b4a8`
Confidence: verified
Proof: one
Test: `objectlayer::missile_scale_vec_difftest`

`0x67c680` tests the transport's v² against 2.0 with `test ah,0x41` before the zeroing. `0x67c150`
frees the head of a list for each step once the running sum reaches 0x96 (`0x67c180`);
the sum is not reduced. `0x67d740` with mode 2 links a new step before the terminator, at the end. The player's
position comes from slot `+0x14`, the transport's when `[player+0x118]+0x38` holds a GUID
(`0x67c2cf`). Transport steps go to the list at `+0x44` (`0x67c3c8`), the player's to `+0x38`
(`0x67c4a2`); their sums give `+0x5c` and `+0x50`. `0x61eec0` is the movement speed times its
direction; `0xc7b4a8` keeps the last frame's milliseconds.

## Ground grid

Particles land on a cached height grid: 5 × 5 tiles around the camera's chunk, each tile one chunk
of 32 × 32 cells, 1/0.96 yards a cell. A cell starts empty (FLT_MAX) and is filled on first use
with the surface height within 200 below the point, or the point's height less 200 when nothing is
hit. A point outside the grid has no height. When the camera changes chunk the grid recentres,
keeping the tiles it still covers.

Addresses: `0x67c900`, `0x67c7b0`, `0x67c770`, `0x67c820`, `0x67ca00`, `0x67ca90`, `0x67cb60`, `0x67ce50`, `0x67cfe0`, `0x67c980`, `0x67c9b0`, `0x67d820`, `0x67c7f0`, `0x6b7070`, `0x810038`, `0x80ff70`, `0x80ff8c`, `0x80ff90`, `0x810008`
Confidence: verified
Proof: one
Test: none

The table `0x810038` holds `0x67cfe0` (bounds), `0x67ce50` (recentre), `0x67cb60` (segment),
`0x67ca00` (height), `0x67d820` and `0x67c980`. `0x67ca00` takes the chunk as floor(0.03·p) and
`0x67c820` the cell as the low byte of bits(0.96·d + 512) >> 14. `0x67c7f0` calls `0x6b7070` with
200 and falls back to z − 200 (`0x810008`). `0x67ce50` moves matching tiles, then refills the
rest with `0x67c7b0`. How `0x67cb60` walks a segment is not read.

## Weather visibility

Weather draws only when the frame's visibility byte is set. The map object frame clears it; the
world sets it when the camera is outside every map object group, and the portal walk sets it on
reaching a group whose flags hold 0x148, or 0x40000 with a set byte in its record. Each effect's
packet update runs before the test, so drops keep falling unseen indoors; the mist spawns and
moves only in its draw, after the test, so it stops while unseen.

Addresses: `0xca80c4`, `0x6b38c1`, `0x68111a`, `0x6811d4`, `0x6b42d0`, `0x6b42d9`, `0x6b42e0`, `0x6b42f4`, `0x67c720`, `0x677380`, `0x677388`, `0x6790b0`, `0x6790b8`, `0x67a520`, `0x67a528`, `0x67ae20`, `0x67b14d`, `0x67b24d`
Confidence: verified
Proof: one
Test: none

`0x6b38c1` stores 0; `0x68111a` branches to `0x6811d4` when the camera's group claim is 0. The
portal walk tests `[group+0x10]` with 0x148 (`0x6b42d0`) and 0x40000 (`0x6b42e0`). `0x67c720`
returns until `+0x80` is set, then calls the three dispatchers; each runs its update first and
returns at `0x677388`, `0x6790b8` or `0x67a528` when the byte is 0, before its tail call to
`0x67ae20`, which accumulates the mist rate (`0x67b14d`) and spawns (`0x67b24d`).

## Weather reset

Loading a world and starting the game reset the weather unless it is forced: the effects are torn
down, both ramps zeroed and nothing is pending. `0x67bdd0` copies the live effect's box, six floats,
for the scene.

Addresses: `0x67bac0`, `0x401591`, `0x401c0b`, `0x67bdd0`, `0x681307`
Confidence: verified
Proof: one
Test: none

`0x67bac0` calls `0x67ba50`, zeroes `+0x0` to `+0x14` and stores −1 in `+0x20`; its callers are
`0x401591` and `0x401c0b`. `0x67bdd0` returns 1 after copying `+0x4` to `+0x18` of the first live
effect; `0x681307` calls it.
