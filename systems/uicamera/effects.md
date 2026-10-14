# uicamera: shake, bobbing, terrain tilt and the drunk view

The effects the follow camera adds on top of the view it solves: spell and footstep shakes, the head bob of first
person, the tilt the camera takes from sloping ground, the wide field of view of a drunk character, and the pitch
the camera hands back to a swimming player.

This is `Camera.cpp`. The per-frame driver that calls them is `uicamera/driver`; the channels they ease through are
`uicamera/transitions`; the CVars are `uicamera/cvars`. Which shake ids a spell or a footstep adds is
`spell/visuals#camera-shake-groups`.

Roots: `0x511d40` (add a shake), `0x511760` (shake walk), `0x511920` (bob), `0x50d900` (slope probe), `0x50dbc0` (tilt arming), `0x511250` (drunk view), `0x5103e0` (facing commit)

## Spawning a shake

`0x511d40(id, position)` adds one shake. An id below 0 or above the table's highest id adds nothing, and so does a
hole in the table. The row of `CameraShakes.dbc` gives the record its type, axis, amplitude, frequency, duration,
pre-roll and decay. The amplitude is multiplied by 1/36 once, so everything after is in yards. The record keeps the
position and the spawn time and nothing else: no source object, no id. It is appended to the tail of the camera's
list at `+0x25c`.

Addresses: `0x511d40`, `0x511c60`, `0xc0dfc8`, `0x540f70`, `0x574cd0`
Confidence: verified
Proof: one
Test: none

The table store is loaded by `0x540f70` from the name `0x574cd0` returns (`DBFilesClient\CameraShakes.dbc`); `0xc0dfc8` holds the rows, `0xc0dfd0` the id index and `0xc0dfd4` the highest id. `0x511d4a` accepts an id equal
to the highest id. `0x511d78` multiplies the amplitude by 1/36 (0x7ff9d0). `0x511c60` allocates 0x34 zeroed bytes
and stores the arguments at `+0x08` type, `+0x0c` axis, `+0x10` amplitude, `+0x14` frequency, `+0x18` duration, `+0x1c` pre-roll and `+0x20` decay, the position at `+0x24` and the clock at `+0x30`
(`0x511d0c`).

## Walking the shakes

Each frame `0x511760(eye, offset)` adds the shakes to the offset. It does nothing, and ages nothing, when the list is empty
or the followed object is not a unit. A record's time is the milliseconds since its spawn times 0.001 plus its
pre-roll; at or past its duration the record is unlinked and freed. A live record's amplitude is cut by the distance
falloff and the record is skipped when out of range. For each axis 0 to 2 the record with the largest amplitude wins,
the older record on a tie; the others add nothing that frame. Each winner is evaluated with the followed unit's world facing.

Addresses: `0x511760`, `0x600cb0`, `0x513a00`, `0x646430`, `0x42c010`
Confidence: verified
Proof: one
Test: none

`0x511785` reads the clock and `0x5117aa` resolves the followed GUID with the unit mask 8. `0x5117ba` takes the facing from `0x600cb0`. `0x5117d1` to `0x5117e9` zero the three best-amplitude, best-time and best-record slots. `0x511816` to `0x511822` form the
time as an unsigned difference; `0x51182d` keeps the record while the time is below the duration, else `0x511834` unlinks it and `0x511860` frees it. The axis is not range checked (`0x5118a8`).
The compare at `0x5118ab` is a strict greater. `0x5118d6` to `0x511905` call the evaluator for each filled slot.

## Evaluating one shake

A shake's value is `a = A * sin(2π * t * f)` with `A` the cut amplitude, `t` the record time and `f` the frequency. When
the type is exactly 1, `a` is multiplied by `exp(-t * k)` with `k` the decay rate; any other type is flat until the
duration ends. Axis 0 adds `(a cos φ, a sin φ, 0)` to the offset, axis 1 adds the same turned by π/2, and axis 2 adds
`a` to the height. `φ` is the world facing of the followed unit, so turning rotates a shake in flight. Another axis
adds nothing.

Addresses: `0x511610`
Confidence: verified
Proof: one
Test: none

`0x511613` to `0x51162a` form `a` with 2π (0x80295c). `0x51162f` to `0x511650` multiply by the exponential: `fldl2e` scales the exponent by log2(e), so `f2xm1` and `fscale` give `e^-(t*k)`. `0x511653` switches on the axis; `0x511677` adds π/2 (0x808acc). `0x511692` and `0x5116af` are `fsincos`, which leaves the cosine in `st(0)` and stores it
first; `0x5116b5` to `0x5116cb` add `a cos` and `a sin` to x and y, and `0x51166a` adds `a` to z.
Every store pops, so each intermediate rounds to a float.

## Distance falloff

`0x5116e0(eye, shakePosition, &amplitude, 9, 81, 80, 6400)` scales the amplitude by distance. Past 80 yards (squared
distance over 6400) it answers 0 and the record is skipped for the frame. Within 9 yards (squared distance of 81 or less) it
answers 1 with the amplitude untouched. Between, the amplitude is multiplied by `0.7 ^ ((d - 9) / 9)` with `d` the
distance, and it answers 1. The base is the float 0.7 widened to a double, 0.699999988079071.

Addresses: `0x5116e0`, `0x73f940`, `0xbe103c`, `0xbe1060`
Confidence: verified
Proof: one
Test: none

`0xbe103c` and `0xbe1060` hold 9.0 squared and 80.0 squared, stored by the static initialisers `0x50b2c0` and `0x50b2f0` from 0x808a0c and 0x808a10; the 80.0 passed as the sixth argument is not read. `0x51171a` and `0x51172d` branch on `test ah,0x41`, so a distance of exactly 80 or exactly 9
takes the cheaper branch. `0x511746` calls the power routine with the exponent on top; the pow base is the double at 0x808ad0.

## Where a shake lands

The shake offset is added to the camera's eye position in world space. It never turns the view or changes the field of
view. The driver zeroes the offset every frame and computes it only for a unit that is neither swimming nor on an
airborne or aquatic server spline; on a skipped frame no shake ages or ends. The offset also takes the bob offset, and the
solver tests the shaken eye against the world. Setting the camera's target to none frees every record; destroying the camera
unlinks them without freeing any.

Addresses: `0x50e8d0`, `0x50d0f0`, `0x50c460`, `0x50de00`, `0x511d40`
Confidence: verified
Proof: one
Test: none

`0x50ea72` to `0x50ea80` zero the offset. `0x50ea87` and `0x50ea8b` skip the block when the spline flag test of `0x50e957` to `0x50e96f` (a spline not finished, flag 0x200) or the swimming flag `0x50e98b` holds. `0x50eafc` is the only call of `0x511760`; `0x50eb0f` to `0x50eb3b` add the bob. `0x50edcc` places the eye through `0x50de00` with the offset added to
each component. The null-target arm of `0x50d0f0` (`0x50d2ac` to `0x50d310`) frees the list; `0x50c48f` to `0x50c4aa` in the destructor only call the unlink.

## Bobbing

First-person bobbing sways the eye as the character moves. The sway runs only while the bob bit is set and `0x5105e0`
admits the unit (`uicamera/cvars#bobbing-and-pivot-tests`). With `speed` the movement speed of the followed unit,
`r = clamp(speed / 7.2, 0.5, 1.5) * cameraBobbingFrequency` and `t` the seconds since the bob began,
the side sway is `a = cameraBobbingLRAmplitude / 36 * sin(2π r t)` along the unit's left axis, `(a cos(φ + π/2), a sin(φ + π/2))`,
and the vertical sway is `cameraBobbingUDAmplitude / 36 * sin(2π * 2r * t)`. The result is written to `+0x110` to `+0x118`.

Addresses: `0x511920`, `0x7c4c90`, `0x5105e0`, `0x511170`, `0x5110f0`
Confidence: verified
Proof: one
Test: none

`0x511951` resolves the followed unit with the unit mask; `0x511959` calls the gate and a false gate writes nothing. `0x5119a6` calls the speed routine with 0; `0x5119ab` divides by 7.2 (0x808a08). The clamp branches at `0x5119b1` to `0x5119d3` raise a ratio below 0.5 (0x8089a0) and lower one above 1.5 (0x80899c); a speed of 0 gives 0.5 and the bob halves instead of stopping. `0x5119e8` subtracts
the start time `+0x238` and `0x511a01` scales by 0.001. The phase is the product `r * t`, so a speed change jumps the phase. `0x511a56` doubles the rate for the vertical term.
The bob starts and stops on the edge of the movement commands: `uicamera/transitions#bob-decay`.

## Starting and stopping the bob

The input control sets the camera's bob bit when a movement command becomes active and clears it when none is, whatever the unit's speed. The commands that count are move forward, move backward, strafe left and right, auto-run, the bits 0x100000 and 0x400000, or the
primary mouse command (bit 0) held with either the secondary mouse command (bit 1) or a turn key (0x300). `0x511170` acts only on an edge; it starts the bob, stamps the time and clears the bit and arms the decay on the opposite edge. The bob is therefore armed even with `cameraBobbing` off; the CVar only gates what `0x5105e0` lets through.

Addresses: `0x514840`, `0x514b70`, `0x511170`, `0x5110f0`, `0x511110`, `0x5105e0`
Confidence: verified
Proof: one
Test: none

`0x514840` (press) and `0x514b70` (release) both end in the same test on the new command word at `+0x04`: `0x5149ad` and `0x514c90` test the mask 0x5010f0 (0x10, 0x20, 0x40, 0x80, 0x1000, 0x100000 and 0x400000); `0x5149b6` to `0x5149c3` and `0x514c97` to `0x514ca6` test bit 0 with bit 1 or 0x300. `0x5149d5` and `0x514cc1` call `0x511170` with the result. `0x511170` calls `0x5110f0` on a rising flag and `0x511110` on a falling one and does nothing otherwise.

## Probing the slope

Every 100 milliseconds `0x50d900(unit, now)` measures the ground ahead of a unit. With the tilt off (`0x5105a0` false) it stores 0 at `+0xa0`. Otherwise it casts a ray 10/3 yards forward along the facing from 5/3 yards above the feet,
backs the hit point off 5/18 yard, and casts a ray 64/9 yards straight down from there. The slope is the ground rise
over the horizontal distance. The slope's size picks the largest of ten table angles (0 to 45 degrees in steps of 5) whose
key it reaches; uphill gives a negative angle. The result is clamped to plus or minus 20 degrees and stored at `+0xa0`.

Addresses: `0x50d900`, `0x5105a0`, `0x672170`
Confidence: verified
Proof: one
Test: none

`0x50d94f` to `0x50d956` throttle on the stamp `+0x188`. `0x50d97e` adds 5/3 (0x808a38) to the foot height, `0x50d9bd` scales the facing vector by 10/3 (0x808a34), and `0x50da3d` and `0x50dac2` are the two traces with the mask 0x151. A miss leaves the end point prefilled in the output, so a forward miss reaches the full 10/3 and a downward miss gives
a depth of 64/9 (0x808ab4) and the maximum tilt. `0x50da42` to `0x50da5d` back off by 0x808ab8 (5/18). `0x50dad9` adds 5/3 back. `0x50db06` selects the sign, and `0x50db20` to `0x50db46` scan the table at 0x808a40 from index 9 down, 8 bytes a row, key then angle in radians; the keys are 0, 0.09, 0.18, 0.27, 0.36, 0.47, 0.58, 0.70, 0.84 and 1.0. `0x50db71` to `0x50dbaa` clamp to plus or minus 0.349066 (0x8089c0). The clamp equals the angle of index 4, so the angles from 25 degrees up are never stored.

## Arming the tilt

`0x50dbc0(unit, now, snap)` turns the probed angle into the ground pitch channel. The state of the unit picks one of ten
rows of the third smoothing family (`uicamera/cvars#smoothing-cvars`) for the style `cameraSmoothStyle`: Taxi, Swim, Fall, Track,
Fear, Move, Strafe, Turn or Idle in that order of test. The target is the row's absorb times the probed angle. A negative factor keeps the live
tilt and cancels the transition. A snap, or a factor under 2^-22, sets the live tilt to the target at once. Otherwise the tilt
eases with `0x512490`, and the duration is kept from `factor * cameraTerrainTiltTimeMin` to `factor * cameraTerrainTiltTimeMax`.

Addresses: `0x50dbc0`, `0x512490`, `0x50f160`, `0x50d0f0`, `0x512e90`
Confidence: verified
Proof: one
Test: none

`0x50dbe9` reads the style. The state tests: `0x50dc06` Taxi (7) when bit 20 of the player flags `[unit+0x110]+0xa0` is set; `0x50dc21` Swim (6) for movement flag 0x200000; `0x50dc2d` Fall (0) for flag 0x2000; `0x50dc4e` Jump (3) after the same flag test failed, so it never runs; `0x50dc60` Track (8) for mode bit 8; `0x50dc6c` Fear (1) for mode bit 12; `0x50dc78` Move (4) for
flags 0x3; `0x50dc84` Strafe (5) for 0xc; `0x50dc8b` Turn (9) for 0x30; else Idle (2). `0x50dc98` to `0x50dca1` index the record `(state + 10 * style) * 12` of the table `0xbe0d00`, which is not bounded for a style of 3. The record holds absorb, delay and factor handles. `0x50f6f6` calls it each frame with snap 0, `0x50d1c0` and `0x513167` call it with a snap on a target change and after a view is set.

## The drunk view

`0x511250(fraction, immediate)` sets the field of view bias that drunkenness adds. The bias target is 89 degrees times the
fraction (3.1241393 - π/2 times the fraction). With a nonzero second argument the bias is set at once; otherwise it
eases through `0x512270` with no delay and a factor of 1. The followed player's drunk level, the byte at `+0x1d` of the
field block `+0xe68`, kept to 100 and times 0.01, is the fraction; any other unit gives 0. The camera's field of view adds
this bias to its base and keeps the sum between 0 and 179 degrees.

Addresses: `0x511250`, `0x50d0f0`, `0x512270`, `0x50d6d0`, `0x5e2a90`
Confidence: verified
Proof: one
Test: none

`0x511253` to `0x51126a` form the target, `0x51126d` branches on the second argument and `0x511298` stores. `0x5112a7` reads the clock for the eased path. `0x50d1de` to `0x50d1fc` in `0x50d0f0` compute the fraction and `0x50d209` calls the setter, immediate when the camera is fresh or its target changed (`0x50d17f` to `0x50d19c`). The object layer calls it when the drunk field changes (`0x5e2b54`, `objectlayer/playerfields#drunk-messages`). The speed CVar `cameraFoVSmoothSpeed` is 0.5 degrees a second by default, so
the full swing takes minutes.

## Committing the facing

`0x5103e0` hands the camera's yaw and pitch to the player. It remembers the yaw at `+0x18c`, sets mode bit 14, forms the
final pitch as the pitch plus the pivot bias kept within plus or minus 89 degrees, and calls the input control's facing and
pitch setters with the clock. In a water band with `cameraDive` set and a sober player, a small pitch is replaced by 0: in the submerged band a pitch between
`cameraSurfacePitch` (a negative value) and 0; in the surface band a pitch between 0 and `cameraSubmergePitch`. In the Track state nothing is replaced.

Addresses: `0x5103e0`, `0x5152e0`, `0x515330`, `0x5143e0`, `0x514400`, `0x514840`
Confidence: verified
Proof: one
Test: none

The mouse-motion handler `0x514400` calls `0x50fee0`, then `0x5103e0` through the gate `0x5145e0` (`0x51447b`); the press of both mouse buttons calls it from `0x514966`. `0x5103fb` and `0x51040c` store the yaw and set bit 14 (0x4000). `0x510412` to `0x510447` clamp to plus or minus 1.5533431 (0x8089dc, 0x8089d8). `0x510471` skips the replacement in the Track state; `0x5104a5` to `0x5104ce` skip it for a drunk player. `0x5104ed` and `0x5104f3` convert the two CVars from degrees. `0x510582` sets the facing and `0x51058e` the pitch.

## Levelling a swimmer

While the unit swims, each frame the driver may set the unit's swim pitch to 0. It does so in a water band with `cameraDive` set,
when the swim pitch is not already zero, neither pitch key is held, the unit is not auto-moving, and the player is not drunk. If the
mouselook command (bit 0) is up, the surface band (with `cameraSubmergePitch` above 0) or the submerged band (with `cameraSurfacePitch` below
0) levels at once. With mouselook held the level happens only when the camera's final pitch lies inside the same small window as in the facing commit.

Addresses: `0x50e8d0`, `0x6103a0`, `0x515330`, `0x5143e0`
Confidence: verified
Proof: one
Test: none

`0x50e9a9` stores the swim pitch `[movement+0x20]` and `0x50e94c` the auto-move result. `0x50eb62` skips unless swimming. `0x50eb7d` to `0x50eba7` skip for a drunk player. `0x50ebb5` to `0x50ebd2` test the bands and `cameraDive`. `0x50ebdd` skips a swim pitch under 2^-22. `0x50ebf1` skips when command bits 0x400 or 0x800 are held and `0x50ebfa` for an auto-moving unit. `0x50ec05` to `0x50ec97` are the window tests, and `0x50ec9d` calls the pitch setter with 0.
