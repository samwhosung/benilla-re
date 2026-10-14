# objectlayer: unit body each frame

What a unit does with its body every frame: the tick the world runs for each unit, the update
behind vtable slot 14, the body's turn toward the display facing and the spine twist that covers
the rest, the breath and bleed timers, the water ripples and swimming, and a drunk
player's sway. The facings themselves (`+0xc94` body, `+0xc98` display) are
[unit movement](unitmovement.md#facing).

Roots: `0x60a7f0`, `0x607ed0`, `0x5fac70`

## The unit tick

The world's per-object callback 0x482cd0 calls `0x60a7f0(time)` for each unit. It releases
missiles held past 2 seconds, stops the unit's finished sounds, ends the active player's loading
screen once its transport is ready, drops the nameplate of a unit whose node is hidden, moves the
unit's sound emitter, runs the bleed timer, and, for the active mover, feeds a drunk player's
sway to the auto-move or to the facing and pitch.

Addresses: `0x60a7f0`, `0x60a814`, `0x60a8dc`, `0x60a93e`, `0x60a9bc`, `0x60aa0e`, `0x60aab7`, `0x60ab0a`
Confidence: verified
Proof: one
Test: `objectlayer::u5_mount_level_fraction_difftest`

`0x60a814` calls `0x60c9b0` when missiles hang on `+0xac` 2000 ms after `+0xa8`. With a loaded
model each of the sounds `+0xb1c`, `+0xb20`, `+0xb28` stops when 0x7a5870 returns true or
0x7a5810 false. At `0x60a8dc` the active player during a loading screen looks up its transport (slot 9) as a
game object; a missing one or one whose model reports ready (0x670910) ends the screen. A node
that 0x6704c0 reports hidden releases the nameplate (`0x60a93e`); `+0xcbc` goes to the position
through 0x462000; bit 0x2 of `+0xc58` runs `0x60a260`. At `0x60a9bc` a player mover's drunkenness
is `min(+0x1d of the player block, 100)/100` and the sway `0x60ab20(time, drunkenness)`. While
auto-moving, both go to `0x6106b0` (`0x60aa0e`). Otherwise, with drunkenness above 0, moving
(flags 0xf) and input ready, `0x60aab7` sets the facing to facing plus sway wrapped into 0 to 2π
(skipped while turning unless 0x5151b0 allows it); a swimmer (0x200000) not pitching by keys
(0xc0) gets pitch + 4·sway, clamped to ±π/2 (`0x60ab0a`). Slices: u5_mount_level_fraction_difftest `0x60a9bc`-`0x60a9de`.

## The per-frame update

Vtable slot 14 is `0x607ed0(time)`. It checks the breath every 10 seconds, shows or hides the
nameplate, flashes the attacked target's selection colour, applies tints and fades, runs the
object's slot 14, draws the auto-move target, plays queued emotes, leaves a water ripple, steps
the unit's effects, steers the display facing unless the camera holds it, then turns the body
and twists the spine.

Addresses: `0x607ed0`, `0x607eee`, `0x607ef9`, `0x607f60`, `0x60800c`, `0x60802e`, `0x608046`, `0x608070`
Confidence: verified
Proof: one
Test: none

`0x607eee` calls `0x607710` once `+0xc18` has passed; `0x607ef9` calls `0x60f600`. At `0x607f60`
the flash runs only on the player's target while the player attacks (`0x60ecb0`) and may attack
it ([unit display](unitdisplay.md#selection-circle-and-colour)); clearing bit 0x10 or setting it
refreshes the name (0x6c77f0). Then `0x60cb50` and `0x60cd30` (tint and fade), `0x60800c` the
object's slot 14 (0x614a90), `0x611400` while auto-moving, `0x60bce0`, `0x60802e` the ripple
`0x5fa760(0)`, `0x608046` each effect on the list `+0xb4` through 0x620580, and `0x608070` the
display facing `0x600cd0` unless the active camera follows this unit.

## Body turn and spine twist

A live unit on foot turns its body `+0xc94` toward the display facing `+0xc98`: a quarter of the
gap each frame while strafing, toward a side angle; at once when moving straight or falling;
otherwise at a rate, with anything past 90° taken at once. Bones 4 and 6 twist through the
remaining gap, 45° each at most. An idle turning body plays the shuffle animations.

Addresses: `0x608075`, `0x6080d6`, `0x60814d`, `0x6081c8`, `0x608293`, `0x608351`, `0x6083e6`, `0x6084d3`
Confidence: verified
Proof: one
Test: `objectlayer::u4_lean_smooth_difftest`

Dead or swimming (movement flags 0x200000), the body takes the display at once (`0x608075`).
Strafing (0xc) aims at the display plus 90°, or 45° when also moving forward or back, the sign by
strafe side and backward (`0x6080d6`). Moving (0x2003) snaps. Within 1e-5 (0x80c5c8) bones 4 and 6 are reset
(0x711f10, `0x60814d`). Past π/2 the excess is taken at once; then, not strafing and not turning by
keys (`+0xd58` bit 0x1), `0x6081c8` adds at most 8 times the turn rate `+0xa44` times the seconds since `+0xcb0`. With a body
model, no mount and bone 4 (bit 0x80), `0x608293` turns bone 4 by half the rest, all of it while
auto-moving, at most π/4; with bone 6 (bit 0x100) `0x608351` turns it by what remains, at most
π/4. At `0x6083e6` bits 0x800 and 0x1000 clear; a unit not moving, falling, swimming or looting,
standing, with no slot 41 state, sets 0x800 or 0x1000 by the turn's sign; `0x6084d3` re-picks the
animation through `0x5fce30` when the wanted shuffle, 11 for left and 12 for right, or none,
differs from the playing one. Slices: u4_lean_smooth_difftest `0x60809a`-`0x6080f6`.

## Low health and bleeding

`UNIT_FIELD_HEALTH` sets bit 0x2 of `+0xc58` through `0x5fa710` when the unit has blood levels;
`0x5fa700` clears it. While set, the tick's `0x60a260` reschedules `+0xc40` 2 to 5 seconds ahead.
Nothing else reads `+0xc40`: the timer draws from the client random and shows nothing.

Addresses: `0x5fa700`, `0x5fa710`, `0x60a260`, `0x60a29b`, `0x60a2ad`
Confidence: verified
Proof: one
Test: none

`0x5fa710` sets the bit only with `+0xb48`; it runs from the constructor, slot 2 and the health
watcher, and `0x5fa700` from slot 1 and the health watcher. `0x60a260(time)` needs a `UnitBlood` row (`0x60a390`);
once `time` reaches `+0xc40` it stores `time + 2000 + MulHigh32(random, 3000)` (`0x60a29b`), then
advances the random state twice more (`0x60a2ad`, the second inline) and drops both draws. The
only other write to `+0xc40` is the constructor's.

## Breath

`0x607710(time)` sets the breath state every 10 seconds: bit 0x20 of `+0xc58` when a liquid
surface lies more than 5 yards above the unit's top, else bit 0x40 in an area whose flags have
bit 0. The model event `$BTH` turns them into a breath effect: drunk, bubbles or a cold breath.

Addresses: `0x607710`, `0x60777b`, `0x6077b8`, `0x5fffad`, `0x5fffe7`
Confidence: verified
Proof: one
Test: `objectlayer::attack_range_in_difftest`

`0x607710` clears 0x60, queries the liquid at the node (0x670630), and compares the surface minus
the position's z with 5 (0x80c44c) plus the height `+0x90`·`+0xbc` (`0x60777b`). Else
`0x6077b8` asks 0x67e9c0, which reads the `AreaTable` row behind the node's `WMOAreaTable` row
(`+0x28`) and returns its flags `+0x10` bit 0, or its parent's unless the area has bit 0x2; vmangos
calls bit 0 the snow flag. Either way `+0xc18` becomes `time + 10000`. In `0x5ffbd0`, `$BTH`
(`0x5fffad`) does nothing while 0x468570 returns 1 or for model data with flag 0x2; at `0x5fffe7` a player with drunkenness of
at least 0.5 (0x80c488) gets effect kind 7, else bit 0x20 kind 2, else bit 0x40 kind 3, through
0x61fae0, which hangs kinds 2, 3 and 7 at attachment 17 by its table 0x80c968.

## Water ripples and swimming

`0x5fa760(kind)` leaves a ripple where the unit stands in shallow water: deeper than the surface
by less than twice the unit's height, or 1 yard. A moving unit's ripples point along its motion
and come faster with speed; a still or turning one's are random and come every 400 to 450 ms.

Addresses: `0x5fa760`, `0x5fa7c4`, `0x5fa888`, `0x5fab41`, `0x5fab78`
Confidence: verified
Proof: one
Test: none

The ripple needs liquid of a type at the node (0x670630, 0x670690) and a surface above the feet by
less than max(2·`+0xa5c`, 1), `+0xa5c` being the collision height. At `0x5fa7c4` a nonzero argument resets `+0xc78` and picks kind 3
for 201, else 0; zero picks 2 when moving (0xf), 1 when turning (0x30), else 0. Byte table
0x860a4c (0, 0, 1, 0) marks kind 2 as moving (`0x5fa888`): size from 2.5 over the speed `+0x84`
capped at 20, direction the facing turned 45°, 90°, 135° or 180° by strafe side and backward,
next ripple after 625/speed ms times the depth factor. Kinds 0, 1 and 3 take a random direction and the next at `time + 400 +
rand(50)` (`0x5fab78`). Sizes mix the scale, 0.65 (0x80c4f0), 3.75 (0x80c4ec), 12 (0x80c4e8),
1/36, three random draws and, when the surface is within half the threshold, a depth factor;
speeds under 0.0001 (0x80c4f4) count as still. `0x5fab41` calls 0x672490 with the active-mover
flag.

## Liquid and swimming

After each mover step `0x5fac70` refreshes the ground's terrain type and hands the depth to the
swim decision.
`0x60df70` and `0x60dff0` start and stop swimming: the active mover asks, others change at once
or queue the change. Two tests read the depth: deeper than half the unit's height, and a point
just under a surface.

Addresses: `0x5fac70`, `0x5fa730`, `0x60df70`, `0x60dff0`, `0x605650`, `0x60a740`
Confidence: verified
Proof: one
Test: `objectlayer::diff_negate_z`, `objectlayer::occlusion_z_gate_difftest`, `objectlayer::u4_airborne_reach_gate_difftest`

`0x5fa730`, and `CGUnit_C::PostInit` at `0x5fba95`, store at `+0xc60` the world entity's `+0xdc`
through `0x6706d0`, or −1: the id `AttachEntityToGround` takes from the WMO face or the ground
effect (`0x6a8bfb`). Its readers index the `TerrainType` store 0xc0d67c with it, through its id
map 0xc0d684 (`0x5fc20f`, `0x623749`), or pass it on as the footstep and footprint terrain
(`0x62341d`, `0x5fc06e`). `0x5fac70` passes the surface minus
the z, or 0, and the liquid flag to 0x6030c0, then 0x613ef0(0). 0x6030c0 calls
`0x60df70(time, force)` or `0x60dff0(time, force)`: the active mover requests the change through
its `CMovement` (a start also sets tutorial flag 0x1b), others apply it at once when forced or
when a spline without flag 0x4 runs, else queue it. `0x605650` returns whether the depth tops half
`+0xa5c`; `0x60a740(point)` whether a liquid surface lies 0.01 below to 2/3 yard above the point. Slices: diff_negate_z `0x5facd4`-`0x5facdd`; occlusion_z_gate_difftest `0x60a7a3`-`0x60a7c7`; u4_airborne_reach_gate_difftest `0x6056ac`-`0x6056ca`.

## Drunk sway

`0x60ab20(time, drunkenness)` is the sway angle: the drunkenness times 0.75° (0x80c4bc) times the
mean of three cosines at slow, unrelated rates. `0x60aba0` clamps a pitch to ±π/2.

Addresses: `0x60ab20`, `0x60aba0`
Confidence: verified
Proof: one
Test: `objectlayer::cosine_pulse_difftest`

With `t = time · (π/180) · drunkenness`, the sway is `drunkenness · 0.0130900 · (cos 0.093t +
cos 0.154t + cos 0.195t) · 0.333333`, the factors at 0x80c5e0, 0x80c5dc, 0x80c5d8 and 0x80c5d4
(0x3eaaaa9f, just under ⅓); it is 0 when the drunkenness is 0. `0x60aba0` uses −π/2 at 0x80c5e4.
