# animation: unit tracks

How a unit's animation code arms the bone slots of `playback`. A unit has up to three tracks: bone 0 of its
body model, a split key bone of the body model (`+0xd5c`, the upper body), and bone 0 of its mount model.
`CGUnit_C::PlayAnimation` (`0x5fe2f0`) reads the three tracks' states, routes the requested animation onto
one or two of them, and plays each through `PlayTrack` (`0x5fdba0`). When a sequence ends, the model calls the
unit back and the unit releases the key bone or picks the next base animation. `objectlayer/unitanim` holds the
animation sets and the choice of the base animation; this doc holds the routing, the hand-overs and the release.

Roots: none

## Track states

`GetTrackStates(mount, body, keyBone)` (`0x5fe1f0`) fills three 7-dword states with `GetKeyBoneAnimState` (`0x711fe0`):
the mount model's bone 0 when a mount exists and is loaded, the body model's bone 0, and the body model's key bone when
`+0xd5c` is not −1. A state holds the bone's requested id `+0xf8`, variation `+0xfc`, position
`ftol(float(now − lo) · speed) + offset`, speed, `lo`, `hi` and latch `+0xc0`. A state whose latch is nonzero, or whose track is
absent, is rewritten to id −1, variation −1, position 0 and speed 1.0. A track reads as armed from the latch, not from `+0xa4`.

Addresses: `0x5fe1f0`, `0x5fe210`, `0x5fe22b`, `0x5fe232`, `0x5fe239`, `0x5fe26b`, `0x5fe2ba`, `0x5fe2c1`, `0x711fe0`, `0x712024`, `0x71203b`, `0x71204a`, `0x712058`, `0x71205e`, `0x712067`, `0x71206a`, `0x712073`, `0x71207c`
Confidence: verified
Proof: one
Test: none

`0x5fe22b` and `0x5fe2ba` store a latch of 1 for a missing mount or key bone. The rewrite at `0x5fe239` to `0x5fe24d`
repeats at `0x5fe277` and `0x5fe2c8`. `0x711fe0` reads the state at `[+0x90] + bone·0x118` (bone 0 for key bone −1, else the key
bone lookup) and waits for the model to load when it has not (`0x711fef`). It never reads `+0xa4`.

## Playing a track

`PlayTrack` (`0x5fdba0`) plays one state. It first sets the variation to 0 when the id is 0 or a ready stance (25 to 29) and the unit
has an attack target (`+0xc48`), a channel or precast, or a cast, spell-ready, swing or ready stance as the current upper animation. It
sets the speed to 1.0 for a death id, and the variation to 0 when the model resolves the id to another animation. It returns when the
id equals the current id or is −1, or the key bone is missing. Otherwise it plays the id on the body model with
the state's variation, position, speed and blend flag `+0x18`, as `primary`, on bone −1 or `+0xd5c`.

Addresses: `0x5fdba0`, `0x5fdbb6`, `0x5fdbdf`, `0x5fdbf2`, `0x5fdc19`, `0x5fdc28`, `0x5fdc32`, `0x5fdc3b`, `0x5fdc96`, `0x5fdc9e`, `0x5fdca2`, `0x5fdcab`, `0x5fdcb4`, `0x5fdccd`, `0x5fdcf2`, `0x7121a0`, `0x711a20`
Confidence: verified
Proof: one
Test: none

The channel test is `[[+0x110]+0x228]` (the channel spell field) or `+0xd58` bit 0x400. The position is passed as the offset
argument, which back-dates the new window (`playback#play-window`), so a state read from `0x711fe0` resumes at its frame. The dedup
compares the state id with the id passed in, taken from the states read at the start of the request. The resolved id is the dword at
`+0x34` of the `0x711a20` record (`0x5fdc9b`).

## Lock bits

After a play, `PlayTrack` sets bits of the unit's animation state word `+0xd58` by the id played: 39 sets 0x4, 121 sets 0x80000,
127 sets 0x8 and 192 and 200 set 0x40000. Then, when bit 0x20000 is set and the id is a swing or a spell cast, it sets 0x2000.
The bits 0x10, 0x20 and 0x40, the enables of the mount, key bone and bone 0 arms, are set by the constructor (`0x70`) and cleared
only between `0x5fcae6` and `0x5fcb30` of the sequence-end handler, which sets 0x70 again before it returns.

Addresses: `0x5fdcf7`, `0x5fdcfa`, `0x5fdd01`, `0x5fdd08`, `0x5fdd0f`, `0x5fdd1c`, `0x5fdd29`, `0x5fdd34`, `0x5fdd43`, `0x5fdd6b`, `0x5fafdc`, `0x5fcae6`, `0x5fcb30`
Confidence: verified
Proof: one
Test: none

The index is `id − 0x27` against a byte table at `0x5fdd90` of 162 entries, all 4 except entry 0 (case 0), 82 (1), 88 (2), 153 and
161 (3); the jump table at `0x5fdd7c` is `0x5fdd29`, `0x5fdd1c`, `0x5fdd34`, `0x5fdd0f`, `0x5fdd43`. A census of the stores to `+0xd58`
shows each other store changing other bits only: the masks used are `0xffff7fff`, `0xffffdbff`, `0xfffffffb`, `0xfffffff7`, `0xffffbfff`,
`0xffffff7f`, `0xfffffe7f`, `0xfffffbff`, `0xfffdffff`, `0xfff7dfff` and the `or` immediates. So the three enable bits a request reads are
always set.

## Request set-up

`PlayAnimation(id)` returns at once unless the unit is created, not reactivating and has a body model. While a model is not
loaded it stores the id in `+0xd64` and returns. A unit with health 0 or below plays only death ids, the state bits 0xc0000
refuse any id, and bit 0x2000 allows only death ids. A request for 57 or 58 becomes 118 when neither hand holds a weapon, else
58 when the main-hand item is two-handed, else 57. The pass then reads the three states, takes the key bone's id as the
current id when it is armed (flag at `ebp-0x10`) and bone 0's otherwise, and builds the request state: variation −1, offset 0,
speed 1.0, blend 1.

Addresses: `0x5fe2f0`, `0x5fe2fe`, `0x5fe30d`, `0x5fe34c`, `0x5fe361`, `0x5fe377`, `0x5fe381`, `0x5fe39c`, `0x5fe3a7`, `0x5fe3bc`, `0x5fe3cc`, `0x5fe3e9`, `0x5fe3f0`, `0x5fe400`, `0x5fe41d`, `0x5fe422`, `0x5fe43c`, `0x5fe69e`, `0x5fec1b`, `0x5fec23`, `0x5fc390`, `0x614e90`, `0x614ef0`, `0x7103d0`
Confidence: verified
Proof: one
Test: none

`0x5fe305 je 0x5fec29` and `0x5fe314 jne 0x5fec29` are the created and reactivating tests (`IsCreated` bit 0x4, `IsReactivating` bit
0x2 of `+0xe4`). `0x5fec1b` stores `+0xd64` unless the id is −1. `0x5fe381 cmp [eax+0x40],edi; jg` passes a unit with health above 0
(`+0x110` field cache, `+0x40`). `0x5fe3cc` and `0x5fe3db` read the weapon in hand 0 and 1 through the unit's slot `+0x98`;
`0x5fe3fa neg; sbb; neg; add 0x39` gives 57 or 58 from `0x5fc390`.

## Move phase and rate

A move animation starts where the one it replaces stands. For a request in the move set (`0x5fee80`) whose sequence has a move
speed above 0, the rate is the mover's current speed over that move speed. When the sequence also has a length, and the playing
base animation on the mount or body has a move speed above 0 and a length, the offset is
`((position · newLength) / currentLength) mod newLength` in unsigned steps, `position` the playing state's position. Other
requests keep offset 0 and rate 1.0.

Addresses: `0x5fe4be`, `0x5fe50a`, `0x5fe515`, `0x5fe526`, `0x5fe53c`, `0x5fe541`, `0x5fe54d`, `0x5fe556`, `0x5fe56f`, `0x5fe5d8`, `0x5fe5dd`, `0x5fe5f0`, `0x5fe5fa`, `0x5fe606`, `0x5fe60b`, `0x5fe60d`, `0x5fe611`, `0x711a20`, `0x7c4c90`
Confidence: verified
Proof: one
Test: none

`0x711a20` returns the sequence length at `+0x4` and the move speed at `+0x8` (`objectlayer/unitanim#animation-rate`). The rate
`GetCurrentSpeed / moveSpeed` is stored at `0x5fe54d` before the length test `0x5fe547 cmp [ebp-0x160],edi`, so a zero length keeps
the rate and skips the phase. `0x5fe5f8 jbe` skips a zero current length; `0x5fe60b div esi` and `0x5fe60f div ecx` are the
unsigned steps.

## Combat fast path

When the current upper animation and the requested id are both swing ids (`IsSwingAnimation`, the 22 ids of
`objectlayer/unitanim#animation-sets`), the request is not played. The pass doubles the playing track with
`SetKeyBoneAnimSpeed(keyBone, 2.0)`, on the key bone when it is armed and on bone 0 otherwise, stores the request in `+0xd60`,
and returns. In any other case it sets `+0xd60` to −1 and goes on. The sequence-end handler clears `+0xd60` when the animation
that ended is a swing and the cached id is not.

Addresses: `0x5fe43c`, `0x5fe44b`, `0x5fe456`, `0x5fe45f`, `0x5fe46e`, `0x5fe474`, `0x5fe47a`, `0x5fe480`, `0x5fe48b`, `0x5fe48e`, `0x5fe496`, `0x5fcc10`, `0x712910`, `0x5fc967`, `0x5fc972`, `0x5fc981`
Confidence: verified
Proof: one
Test: none

`0x5fe43c test byte [ebx+0xd58],0x20` is always true (`Lock bits`). The call pushes `0x40000000` (2.0) and the key bone, `+0xd5c` when
the flag at `ebp-0x10` is set and −1 otherwise (`0x5fe461 je 0x5fe46b`). `0x5fd360` reads `+0xd60` when it is not −1
(`objectlayer/unitanim#choosing-the-base-animation`). The early `ret 4` at `0x5fe48b` also skips the hand-off of the pending trail
value `+0xd20` to the weapon trails (`0x60e550`) that follows `0x5fe48e`.

## Mask rules

The pass decides whether the request plays on the key bone (mask) or on bone 0. It masks a request in the upper-body set
(`0x5fed90`) while the unit is turning (`+0xd58 & 0x1800`), moving or swimming (`+0x9e8 & 0x20003f`) or not in stand state 0
(vtable slot `+0xa4`); a jump id (37 to 39, 187) while the current upper animation is a swing or a ready stance (25 to 29); a
swing id while the mover has the jumping flag `0x2000`; and any request while a channel or precast is active and the unit moves
or swims (`& 0x20000f`). A unit with none of these plays on bone 0.

Addresses: `0x5fe6a1`, `0x5fe6ae`, `0x5fe6c8`, `0x5fe6d7`, `0x5fe6dc`, `0x5fe6ec`, `0x5fe6f8`, `0x5fe704`, `0x5fe70d`, `0x5fe716`, `0x5fe721`, `0x5fe72f`, `0x5fe733`, `0x5fe73f`, `0x5fe74b`
Confidence: verified
Proof: one
Test: none

The route flag `esi` is 1 at `0x5fe73f` (the key enable `ebp-4` is 1) and 0 at `0x5fe74b`. The channel or precast flag in `edi` starts at 1
(`0x5fe692`) and is cleared when `[[+0x110]+0x228]` is 0 and `+0xd58 & 0x400` is clear (`0x5fe6b4` to `0x5fe6c1`). The swing id test is
`IsSwingAnimation` on the request (`0x5fe718`) and the mover flags are `[[+0x118]+0x40]` (`0x5fe727`). A jump id is 37, 38, 39 or 187
(`0x5fed70` at `0x5fe6f8`), and its test reads the current upper id at `ebp-8`.

## Mask overrides

Three rules clear the route flag after those rules. A landing id (39, 187) plays on bone 0. A turning unit (mover flags `& 0x30` or
`+0xd58 & 0x1800`) that has a channel, a precast or an attack target and is not strafing (mover flags `& 0xc`) plays on bone 0. A
request of 57, 58 or 118, a death id, or state bit 0x20000 plays on bone 0 and also turns the key arm off and bone 0's on.

Addresses: `0x5fe68b`, `0x5fe6a3`, `0x5fe750`, `0x5fe75b`, `0x5fe765`, `0x5fe76e`, `0x5fe77b`, `0x5fe781`, `0x5fe788`, `0x5fe794`, `0x5fe79d`, `0x5fe7a9`, `0x5fe7ae`, `0x5fea3a`, `0x5fea5d`, `0x5fea74`
Confidence: verified
Proof: one
Test: none

`0x5fe750 call 0x5fec40; je` leaves ids 39 and 187 with `esi = 0` (`0x5fe759`). The attack target is the 64-bit value at `+0xc48`
(`0x5fe765 call 0x47bf60; or eax,edx`). `0x5fe7a9` clears `esi` and `ebp-4` and sets the bone 0 enable `ebp-0x14`; the same copy
repeats at `0x5fea3a` to `0x5fea74` with the request copied to bone 0's working state.

## Routing a request

Unmounted, a request of 61 goes to the key bone. With the route flag set, an emote (not 69) goes to the key bone, and so does any other
request while the current upper animation is not a swing, a ready stance, a cast or a spell-ready stance and the sheathe state `+0xd40`
is not 2. Otherwise a move id, a ready stance or Stand (0) goes to bone 0 (`Hand-overs`) and any other request goes to the key bone,
with bone 0 playing 41 when the unit swims in place. With the flag clear a request goes to bone 0, except in the hand-overs.

Addresses: `0x5fe834`, `0x5fe848`, `0x5fe852`, `0x5fe860`, `0x5fe86f`, `0x5fe89f`, `0x5fe8ae`, `0x5fe8c8`, `0x5fe8ed`, `0x5fe912`, `0x5fe930`, `0x5fe93d`, `0x5fead7`, `0x5feb00`, `0x5feb89`, `0x5febcb`, `0x5febea`, `0x5fec06`, `0x5fec0d`, `0x5fdf80`
Confidence: verified
Proof: one
Test: none

Blocks `0x5fe89f` and `0x5fe860` copy the request into the key bone's working state (`ebp-0x5c`) and set `ebp-4`; `0x5fe930` copies it into
bone 0's (`ebp-0x3c`). `0x5fe8c8` to `0x5fe8ed` also copy it to the key bone and, while swimming without moving (`& 0x200000`, no
direction bits), write 41 into bone 0's working state. The mount arm at `0x5febcb` is a direct `PlayAnimation(−1, id, variation, offset,
speed, blend, 1)` on `+0xdc`, gated by `+0xd58` bit 0x10. `0x5fec0d` ends with the sheathe update. The arms run in the order mount, key bone, bone 0, each skipped when its working id equals the
current one or is −1.

## Mounted routing

A mounted unit sends a request in the mount set (`0x5fec80`, 33 ids) to the mount model's working state. A request outside that set
but in the upper-body set goes to the key bone's working state. In every case bone 0's working state becomes Mount (91), variation −1,
offset 0, speed 1.0, blend 0, unless bone 0 already plays 91, and the rest of the routing is skipped.

Addresses: `0x5fe7b5`, `0x5fe7bd`, `0x5fe7c1`, `0x5fe7d8`, `0x5fe7dd`, `0x5fe7ea`, `0x5fe7fe`, `0x5fe803`, `0x5fe809`, `0x5fe810`, `0x5fe816`, `0x5fe82f`, `0x5fe940`, `0x5fec80`, `0x5fed90`
Confidence: verified
Proof: one
Test: none

`0x5fe7bd je 0x5fe834` takes an unmounted unit past this block. `0x5fe7d8` and `0x5fe7fe` are the 7-dword copies of the request at `ebp-0x78`
into the mount's (`ebp-0x98`) and the key bone's (`ebp-0x5c`) working states. `0x5fe803 mov ecx,[ebp-0xb4]; mov eax,0x5b; cmp` compares
bone 0's current id with 91, and `0x5fe816` to `0x5fe82c` store 91, −1, 0, 1.0 and 0 at `ebp-0x3c` to `ebp-0x24`.

## Hand-overs

Two blocks move a playing animation between tracks at its current frame. `0x5fe919` copies bone 0's current state to the key bone's
working state, with blend 0, and gives the request to bone 0. It runs when the route flag is clear, the request is a move id and bone 0
plays a swing or a cast. It also runs with the flag set when the current upper animation is a swing, a ready stance, a cast or a
spell-ready stance (or the sheathe state is 2), the request is a move id, a ready stance or Stand, and the key bone is not armed.

Addresses: `0x5fe8ae`, `0x5fe8b0`, `0x5fe8bb`, `0x5fe8c4`, `0x5fe912`, `0x5fe915`, `0x5fe919`, `0x5fe927`, `0x5fe92e`, `0x5fe930`, `0x5feae0`, `0x5feae6`, `0x5feaed`, `0x5feaf3`, `0x5feafa`
Confidence: verified
Proof: one
Test: none

With the key bone already armed, `0x5fe912` sends a mask-path move request to bone 0 alone (`0x5fe930`). The unmasked entries
`0x5feae6 call 0x5fcc10` with `0x5feaed jne 0x5fe919`, and `0x5feaf3 call 0x5fcbb0` with the `jne 0x5fe919` after it, test bone 0's current id with no check of the key bone.
`0x5fe927` sets the key enable and `0x5fe92e rep movsd` copies the seven dwords of bone 0's current state (`ebp-0xb4`) to `ebp-0x5c`. The
sets are in `objectlayer/unitanim#animation-sets`.

## Demotion

`0x5feb5c` copies the key bone's current state to bone 0's working state with blend 1, sets the key bone's working id to −1, which
`PlayTrack` skips, and drops the request. It runs with the route flag clear and the key bone armed, when the current upper animation
is a swing or ready stance and the request is a ready stance or Stand, or it is a cast or spell-ready stance and the request is a
spell-ready stance or Stand. With the key bone not armed in those cases the request is dropped and nothing changes.

Addresses: `0x5feb00`, `0x5feb05`, `0x5feb0e`, `0x5feb17`, `0x5feb19`, `0x5feb22`, `0x5feb26`, `0x5feb28`, `0x5feb31`, `0x5feb3e`, `0x5feb40`, `0x5feb49`, `0x5feb51`, `0x5feb56`, `0x5feb5c`, `0x5feb71`, `0x5feb74`, `0x5feb77`, `0x5feb7e`, `0x5fe930`
Confidence: verified
Proof: one
Test: none

`0x5feb51 mov eax,[ebp-0x10]; test; je 0x5fe93d` leaves both arms alone when the key bone is not armed. Any other case ends at
`0x5fe930`, which gives the request to bone 0. `0x5feb74` stores 1 in bone 0's blend field (`ebp-0x24`), `0x5feb77` stores −1 in the key
bone's id (`ebp-0x5c`) and `0x5feb7e` 1 in its blend field.

## Stationary kit pin

`ApplyEffectAnimation` (`0x5fde80`) runs unless the request is a death id. It walks the unit's effect list `+0xb4` and reads the
unit's movement flags `+0x9e8 & 0x20000f` once. For a unit that is not moving or swimming, an effect whose kit id `[kit+8]` is positive
and whose flags `+0x2c` have bit 0x1000 makes the working states play that id with variation −1, and drops the request; the key bone's
state only when it is armed. An effect with flag 0x4 and no pin sets the three working speeds to 0 and a variation of −1 to 0 and ends the walk, so a pin behind it does not apply; the play goes on with its working states.

Addresses: `0x5fde80`, `0x5fde96`, `0x5fde9c`, `0x5fdebd`, `0x5fdec2`, `0x5fded6`, `0x5fdee5`, `0x5fdef3`, `0x5fdf3a`, `0x5fdf43`, `0x5fdf6b`, `0x5feab5`, `0x5feaba`, `0x5feac4`, `0x5feacf`, `0x60eecd`, `0x60edf0`
Confidence: verified
Proof: one
Test: none

`0x5fde9c and edi,0x20000f` is read outside the node loop. `0x5fdebd test ch,0x10` is bit 12. `0x5fdec2 test [eax+0x2c],bl` with `bl = 4`
jumps to the freeze at `0x5fdf3a`, which falls to the `xor eax,eax` at `0x5fdf6b`: the walk returns 0 and `0x5feaba` plays the working states. A moving unit reaches the freeze test for every effect it walks, since `0x5fdeac` jumps past the pin test. The hit copies the three current states to the working states, then stores `[[node+0x1c]+8]` at offset 0 and −1 at offset 4
of each. The hit enables bone 0's arm, and the key arm when it is armed. `0x60eecd or ecx,0x1000` is in the kit attach routine `0x60edf0`, case 2 of its jump table.

## Sequence end

A unit's model calls back when a sequence ends (`0x6078d0` to `0x5fc920`). For a bone with a key bone id other than −1 and 26, an id
other than 15 (HandsClosed), and not a sheathe stage that continues, the unit calls `PlayAnimation(keyBone, −1, −1, 0, 1.0, 1, 1)`,
which fades the held pose out over 150 ms (`playback#stopping-a-bone`) and leaves the bone to inherit bone 0's time. For key bone −1
or 26 it clears bits 0x70 of `+0xd58`, sets 0x10 or 0x40, restores the sheathe when bit 0x8000 is set, picks the base animation again
through `PlayFollowOnAnimation` and sets 0x70. A reason-1 call from a replaced play runs only the handler's common prefix and
the small table of `objectlayer/unitanim#when-a-sequence-ends`.

Addresses: `0x5fc920`, `0x5fc9ff`, `0x5fca62`, `0x5fca65`, `0x5fca6a`, `0x5fca6f`, `0x5fca78`, `0x5fca7d`, `0x5fca98`, `0x5fcaad`, `0x5fcab8`, `0x5fcacb`, `0x5fcad7`, `0x5fcae6`, `0x5fcb2b`, `0x5fcb30`, `0x6078d0`, `0x607800`, `0x711bb0`, `0x70a280`, `0x719370`, `0x7194bc`, `0x7194f5`, `0x7194fe`
Confidence: verified
Proof: one
Test: none

The handler gets `(model, keyBone, id, reason, overshoot)`; `0x5fc9fc` tests the reason (`0x5fca62` is reason 0). The call at `0x5fcacb` pushes
`1, 1, 0x3f800000, 0, −1, −1` and the key bone, so `primary` and `blend` are 1. For ids 89 and 90 on key bones 2 and 3 (`0x5fca82`) the
unit tests state bit 0x200000 or 0x100000 and, when it is clear, calls the sheathe helper `0x611960` or `0x6118a0`; a nonzero result
holds the bone. `0x719370` sets the latch (`0x7194bc`) before it queues the callback with reason 0 (`0x7194f5 push 0`) through
`0x70a280` (`0x7194fe`).

## Variation cycle

A unit's base animation draws a new variation each time its window ends. The window end sets the bone's latch and queues the
callback (`Sequence end`), which for bone 0 picks the base animation again; the pick calls `PlayAnimation` with variation −1, and the
finished track reads as id −1 (`Track states`), so `PlayTrack` does not dedup an id equal to the finished one. `PlayTrack` leaves
the variation −1 except in the cases that set it to 0, the head variation: an attack target, a channel or precast, or a combat, cast
or ready current animation, for Stand and the ready stances. A window is `span · repeats` wide, so a looping sequence plays its
repeat count before the draw.

Addresses: `0x7194a4`, `0x7194bc`, `0x7194fe`, `0x607800`, `0x6078d0`, `0x5fcad7`, `0x5fcb2b`, `0x5fc3f0`, `0x5fe697`, `0x5fe2f0`, `0x5fdc28`, `0x5fdcab`, `0x5fdcf2`
Confidence: verified
Proof: one
Test: none

The watchdog test is `0x71949e mov eax,[edi+0xac]; cmp eax,[ebp-4]` against the clamped end, and it does not read the sequence flag.
`0x5fe697 mov [ebp-0x74],0xffffffff` is the request's variation. `0x607800` installs `0x6078d0` on the body model, with the unit's
GUID. The follow-on pick `0x5fc3f0` is `objectlayer/unitanim#when-a-sequence-ends`.
