# ui: the world pick

A mouse press in the world picks one object or one point under the cursor, and what the pick may return depends on the spell that awaits a target. This doc is the pick's flag word and how its two rays use it, the candidate list and the two filters over it, the local player as a candidate, the hover while a spell awaits a target, and what moving does to targeting. The click legs and the cursor pictures are `mouseover` and `worldcursor`; the flag word of the pending spell and its binders are `spell/targeting`.

Roots: `0x481050`, `0x481190`, `0x480df0`, `0x480780`, `0x480610`, `0x4815d0`, `0x4828d0`

## The pick flags

`0x481050` builds the flag word. Without a pending spell it is 0 for a missing active mover, 1 when the mover can auto-interact and no cursor payload is held, and 3 when its movement word `+0x40` also has bit 0x10000000 set and bit 0x200000 clear; with an active player, 0x5c is ORed in. With a pending spell the word comes from the spell's target word alone: 3 for a location, 4 for a game object, 0x18 for a unit, then for a unit spell 0x20 (self), 0x10000 (group), 0x20000 (assist), 0x40000 (enemy), and 0x200040 or 0x100000 (dead units or living ones).
Corpse spells add 0x20040 and 0x40040.

Addresses: `0x481050`, `0x481055`, `0x4810ac`, `0x4810b5`, `0x4810be`, `0x4810e6`, `0x4810f3`, `0x481101`, `0x48110d`, `0x481119`, `0x481122`, `0x481125`, `0x481134`, `0x481143`, `0x481152`, `0x481163`, `0x481169`, `0x481178`, `0x6e48a0`, `0x6e6320`, `0x6e62d0`, `0x6e6180`, `0x6e61a0`, `0x6e61e0`, `0x6e6200`, `0x6e6210`, `0x6e6230`, `0x6e6240`, `0x6e6250`
Confidence: verified
Proof: one
Test: none

The unit branch is skipped (`0x481114`) when `0x6e6180` fails, so 0x20 and the relation bits need a unit target, and a location-only spell gives exactly 3. The word is built in one place and used by two callers of `0x481190`: the hover (`0x48184a`) and the press (`0x481f47`). `0x6e61a0` is true for the target word & 0x850e when the spell lacks `AttributesEx` 0x80000 (`spell/targeting#flag-word-predicates`).

## What the pick returns

`0x481190(cursor position, out)` returns 0 at once when the flag word is 0. Otherwise it stores the ray's origin at `out+0x18` and far point at `out+0x24` on every call, and calls `0x480df0`. That traces the terrain segment (`0x672170`; collision mask 0x1000114, or 0x10f0114 with flag 0x2) and, only when the flag word has any of the bits 0x7c, the object candidates (`0x480a50`). The nearer wins; a tie or an unordered compare goes to the object.
An object hit returns 2 with its GUID at `out+0` and its point and distance; a terrain hit returns flag bit 0 as the state (1 or 0); no hit returns 0.

Addresses: `0x481190`, `0x4812c1`, `0x4812c8`, `0x481321`, `0x481335`, `0x48134c`, `0x481351`, `0x481377`, `0x480df0`, `0x480df9`, `0x480e19`, `0x480e20`, `0x480e3d`, `0x480e7b`, `0x480e92`, `0x480eb4`, `0x480ebf`, `0x480ec5`, `0x480f67`, `0x480f8b`, `0x481026`, `0x672170`, `0x480a50`, `0x481f00`, `0x481f47`
Confidence: verified
Proof: one
Test: `ui::glue_geom_480_480a50_480df0_dist_scale`, `ui::glue_geom_480_480df0_normalize`

`test ah,5` after the `fcomp` at `0x480eb7` leaves the terrain branch only for a terrain distance strictly less than the object's. The pick runs at the press only (`input#what-a-press-latches`). A result of 2 stores the GUID in the world frame's hover cache (`+0x348`); any lower result clears it and releases the frame's second node list (`0x480990` at `0x481362`).
A hit point is the ray origin plus the origin-to-far vector scaled to the distance (`0x4841c0`).

## Casts that want a place

A spell that wants only a location gives the flag word 3, which has no bit of 0x7c, so the object candidates are never tested: a click on a unit reaches the terrain behind it, and the state is 1. Bit 0 comes only from the location predicate, so for a spell that wants a unit a terrain hit without an object returns 0 and the click takes the nothing leg (`mouseover#the-nothing-leg`), which binds no target.

Addresses: `0x4810fc`, `0x480e7b`, `0x480e9d`, `0x480f67`, `0x481116`, `0x6e6320`
Confidence: verified
Proof: one
Test: none

A location cast's click reaches the terrain leg and commits through the location binder (`spell/targeting#binding-a-clicked-location`); a unit cast's click on an object reaches the select leg, whose `SetSelection` binds the target and returns without changing the selection (`spell/targeting#binding-a-clicked-object`).

## The candidate list

The world frame keeps a list of nodes, one per object drawn this frame, at `+0x318`/`+0x31c`. The per-frame render callback `0x481540` hands each object to `0x4815d0`, which asks the object's virtual at `+0x40` whether it renders this frame, and inserts a node only when the answer is nonzero and the object is not release-pending. A node holds the model (`+0x8`, with a reference), the GUID (`+0x18`) and the object's height (`+0x10`).
`0x482ca0`, once per world frame, empties this list and a second one at `+0x320` (`0x480990` releases each node's model and returns the node to the free list) and then ticks every unit and game object.

Addresses: `0x481540`, `0x48158a`, `0x482cd0`, `0x4815d0`, `0x4815ff`, `0x48161d`, `0x48162e`, `0x481653`, `0x481694`, `0x4816f3`, `0x48171d`, `0x481720`, `0x48172a`, `0x481739`, `0x480990`, `0x482ca0`, `0x48322f`, `0x484220`, `0x710390`, `0x710c50`
Confidence: verified
Proof: one
Test: none

`0x481540` is installed on the model by address (`objectlayer/createpose`); for an object that is released, or whose slot `+0x78` answers 0, it hides the model instead. A node comes from the free list at `+0x330` or from a 0x20-byte allocation. An object that does not render this frame has no node, so it cannot be picked, whatever the flag word.
`0x482ca0` is called from the world render (`0x48322f`), which is also where the per-object tick `0x482cd0` runs.

## The candidate filters

`0x480a50` walks the list and runs `0x480780(node, flags)` on each node. A node that passes is registered as a hit-test instance (`0x480d90`, for the model and its children) and the scene ray (`0x7089c0`) then returns the nearest hit's node.
`0x480780` admits a unit with flag 0x8 and a player with 0x10, and sends both to `0x480610`. It admits a game object with 0x4 and a corpse with 0x40, each also needing `0x6e62e0` or `0x6e6260` respectively while a spell awaits a target. Every other type is refused.

Addresses: `0x480a50`, `0x480b7f`, `0x480bac`, `0x480bfe`, `0x480c11`, `0x480c2e`, `0x480780`, `0x4807b2`, `0x4807ba`, `0x4807c8`, `0x4807de`, `0x4807f4`, `0x480816`, `0x480850`, `0x48083c`, `0x480c90`, `0x480d10`, `0x480d00`, `0x480d90`, `0x480da1`, `0x7089b0`, `0x7089c0`, `0x5f8800`, `0x6003a0`, `0x5d6e20`, `0x713cb0`
Confidence: verified
Proof: one
Test: `ui::glue_geom_480_480a50_480df0_dist_scale`, `ui::glue_geom_480_480a50_coord_delta`

`0x480d90` calls `SetHitTestMode` with mode 1, the node as the user value and a value from `0x480c90`: 1 for an interactable game object, 3 for a living unit, 2 for a lootable dead unit or corpse, else 0, and −1 for the object picked last. The type byte table at `0x480850` maps unit, player, game object and corpse to four cases of the jump table `0x48083c`; the fifth entry refuses. `0x480c90` uses the table `0x480d10` over the type less 9: unit and player, game object, corpse. The ray returns the node of the nearest triangle and the pick takes the GUID from it. `0x7e51fe` is a second caller of `0x480a50`, the glare test (`lighting/celestial`).

## What a unit must pass

`0x480610(unit, flags)` runs these tests in order, and admits the unit when none refuses. Without flag 0x20 it refuses the active player's own model and a unit under the active player's control: unit-flag 0x01000000 set and its charmer (else its creator) the active player. With any of 0x10000, 0x20000 and 0x40000 set it needs one of them to hold against the active player: same party, may assist, may attack.
With either of 0x100000 and 0x200000 set it needs a living unit to have the first and a dead one the second. While a spell awaits a target it refuses a unit with unit-flag 0x100.

Addresses: `0x480610`, `0x48062c`, `0x480634`, `0x480638`, `0x48064f`, `0x480658`, `0x480686`, `0x4806ab`, `0x4806b8`, `0x4806c4`, `0x4806ce`, `0x4806da`, `0x4806e4`, `0x4806fd`, `0x48070e`, `0x480737`, `0x480746`, `0x48074c`, `0x5fa6d0`, `0x606c20`, `0x6066f0`, `0x606980`
Confidence: verified
Proof: one
Test: none

`test cl,0x20` at `0x48062c` and the `jne` at `0x480634` jump over both control tests to `0x480686`. Health `> 0` is `setg` at `0x480710`. The relation tests are calls with the active player as `this`: `0x606c20` (same party), `0x6066f0` (assist) and `0x606980` (attack). The flag at `0x480746` is bit 8 of the unit-flag word at `+0xa0`.

## The local player as a target

The active player is picked only while a spell awaits a target that gives flag 0x20, and only if its model renders this frame. Without a pending spell 0x20 is never set, so `0x480638` refuses it. The model's render virtual (`CGPlayer_C::OnModelUpdate`, `0x5ec8f0`) answers 0 for the active player when `0xc4d74c` is 0, and the self fade sets that word to whether the fade alpha is nonzero when the camera follows the player.
At first person the alpha is 0, so the player has no node and is not pickable.

Addresses: `0x5ec8f0`, `0x5ec91a`, `0x5ec923`, `0x5ec8c0`, `0x5ec8e0`, `0x482c10`, `0x482c3a`, `0x482c41`, `0x482c88`, `0x50f8fd`, `0x405220`, `0x405229`, `0x404a1d`, `0x5e2058`, `0xc4d74c`
Confidence: verified
Proof: one
Test: `ui::textglue_482_bool_inv255`

`0xc4d74c` has three writers: a store of 1 at `0x5e2058`, the toggle `0x5ec8c0` (the console command `showplayer`, `0x405220`, registered at `0x404a1d`) and the setter `0x5ec8e0`, called from `0x482c41`. `0x482c10` runs every time the camera applies the self fade (`0x50f8fd`; `uicamera/driver#the-self-fade`). The same alpha, over 255, is the model's alpha (`0x482c88`). The relation tests of `0x480610` still apply to the player.

Bit-exact coverage: `ui::textglue_482_bool_inv255` covers `0x482c7f` to `0x482c88` of `0x482c10`.

## The hover while targeting

`0x4828d0`, with a spell awaiting a target, asks `0x6e6460` about the picked object and sets the cursor to Cast (2) or UnableCast (0x16), then jumps to the publish at `0x4829d4`. It never runs the object's eligibility virtual (`+0x54`) that the other path needs, so any object the pick admitted becomes the mouseover. Mouselook publishes a null GUID instead.

Addresses: `0x4828d0`, `0x4828da`, `0x4828f4`, `0x4828fd`, `0x482910`, `0x482923`, `0x482928`, `0x482932`, `0x482982`, `0x4829d4`, `0x4829e0`, `0x482090`, `0x523c20`, `0x6e6460`
Confidence: verified
Proof: one
Test: none

`0x4828da` tests the world frame's bit 1 at `+0x38c` (mouselook), `0x4828f4` the pending spell. The publish `0x482090` is the one gate (`mouseover#publishing-the-mouseover`). A unit refused by `0x480610` or `0x480780` never gets here.

## Moving keeps targeting

A press that changes a movement bit cancels an in-flight spell only when no spell awaits a target. `0x515090` reads the in-flight spell (`0x6e3d10`), returns past the cancel when there is none, and tests the pending-target state (`0x6e48a0`) before it checks the spell's interrupt flag and cancels with reason 0x1c.

Addresses: `0x515090`, `0x5150ce`, `0x5150f2`, `0x5150fb`, `0x5150fd`, `0x515102`, `0x51511e`, `0x51512a`, `0x6e3d10`, `0x6e48a0`, `0x6e4940`
Confidence: verified
Proof: one
Test: none

`0x5150ce` tests the movement bits 0x10f0 first. `0x5150fd` calls `0x6e48a0` and `0x515104` skips the cancel when it answers true. The cancel at `0x51512a` is the one listed in `spell/cancel`.
