# objectlayer: missiles

`CMissile`, the projectile a spell or a ranged attack sends from a unit: its pool and lists, the
flight from launch to arrival, how it aims at and leads a moving target, and what each outcome does
when it gets there: impact kits, miss words, a ricochet, a reflection, or an arrow left stuck in the
target. The launcher, which fills a missile and queues it on its caster, is `auras`.

Missile_C.cpp runs from `0x61cbf0` to `0x61e930`, its initializers excepted (`initializers`), and
`0x61eec0` follows them. A missile is 0x90 bytes from the pool `0x61cbf0` creates at unit start-up.
It sits on one of four lists: the caster's release queue `+0xac` until it is launched, the flying
list `0xc4de28`, the impact list `0xc4de2c` once it becomes an impact model, or a unit's stuck list
`+0xb0`. Each frame a world-object callback moves a flying missile.

Roots: `0x61cbf0`, `0x61cc40`, `0x61cc70`, `0x61ccd0`, `0x61ce00`, `0x61ce40`, `0x61ceb0`, `0x61e8f0`, `0x61e930`, `0x61eec0`

## The missile object

| offset | member |
|---|---|
| `+0x00` | the model; `+0x7c` the world object that carries it |
| `+0x08`, `+0x10` | the caster's and the target's GUIDs |
| `+0x18`, `+0x1c` | the spell id and the speed in yards per second |
| `+0x20`, `+0x2c` | the position and the destination |
| `+0x38` | the flags, below; `+0x3c` the target attachment, −1 for none |
| `+0x40`, `+0x44` | the flight sound id and its playing sound |
| `+0x48`, `+0x4c` | the outcome code and the code a reflection takes |
| `+0x50`, `+0x74`, `+0x78` | the stuck expiry, the target's reaction time, the time it was queued |
| `+0x54` to `+0x6c` | the GUID array (capacity, count, data) and the reason array |
| `+0x80`, `+0x84`, `+0x88` | the list link (the slot that points here, the next) and the pool handle |

Flags: 0x1 hit, 0x2 impact model, 0x4 fired from a bow, crossbow or thrown weapon, 0x8 missed,
0x10 blocked, 0x20 resisted, 0x40 deflected, 0x80 reflected, 0x100 outcome reported.

Addresses: `0x61cbf0`, `0x61cc40`, `0x61cc70`, `0x61ccf0`, `0x61ccd0`, `0x61cd70`, `0x61ce40`, `0x61ce50`, `0x61d1f0`, `0x61e8f0`, `0xc4de28`, `0xc4de2c`, `0xc4de30`
Confidence: verified
Proof: one
Test: none

`0x61cbf0` allocates the pool record `0xc4de30` and calls `ObjectAllocAddHeap(0x90, 0x40,
"CMissile")`; `0x61cc40` frees it. `0x61cc70` takes a block and runs the constructor `0x61ccf0`,
which zeroes all but `+0x04` and `+0x4c` and sets `+0x3c` to −1, unless the pool's byte `+4` is
set; the handle goes to `+0x88`. `0x61ccd0` runs `0x61cd70`, which closes the model, releases the
world object, unlinks, stops the sound with a 0.15 s fade, empties both arrays and frees them, and
then returns the block. `0x61ce50(head)` unlinks (`0x61d1f0`) and pushes at the list head;
`0x61ce40` is its stack-argument thunk. The launcher `0x60a3d0` stores the arrays (`+0x58`, `+0x5c`;
`+0x68`, `+0x6c`), the sound id from its visual's `+0x28` (`0x60a516`) and flag 0x4 (`0x60a509`),
then stores the time at `+0x78` (`0x60a6e1`) and links the missile on its caster's `+0xac`, the
release queue it waits on until launch. `CGUnit_C::ReleaseMissiles` (`0x60c940`) launches every queued missile
through `0x61ceb0`; the unit's model events call it on `$CSR`, `$CSL` or `$CST` (`0x600149`) and on
`$BWR` (`0x600294`), and `0x60c9b0` calls it otherwise (`auras`, Missiles).
`0x61e8f0`, at shutdown, frees every missile on both global lists.

## Launch and flight time

`0x61ceb0(start)` launches a missile, and relaunches a reflected one: it sets the outcome flags,
places the missile at `start`, moves it to the flying list, settles the target attachment and aims.
For a positive speed it then fits the speed so that the flight ends on schedule: with `d` the
distance and `q` the seconds spent in the caster's release queue, `t = d/speed − q`; for `t > 0` the speed becomes
`min(d/t, 100)`, otherwise 100 with `t = 0`. A blocked or deflected missile at a unit makes the
target play 24 ShieldBlock or 30 Dodge so that half the sequence has run at impact. The model plays
144 InFlight, its loop stretched over `t`, or 0 Stand.

Addresses: `0x61ceb0`, `0x61cee4`, `0x61cf13`, `0x61cfae`, `0x61cfda`, `0x61cffa`, `0x61d0c3`, `0x61d02e`, `0x61d0b0`, `0x61d10e`, `0x61d18e`
Confidence: verified
Proof: one
Test: none

`0x61cee4`: a blocked missile takes attachment 0. `0x61cf13`: for a target unit the attachment is
kept when the target's model has it, else 15, else 19, else −1; no target gives −1. The distance is
`sqrt((dz² + dy²) + dx²)`, stored as a float (`0x61cfd7`); `0x61cfda` divides the unrounded
distance by the speed, and `0x61cfdd` rounds `d/speed` to a float before the queue time,
`(now − +0x78)·0.001` (`0x61cfe5`), is subtracted. `0x61cffa` tests `t > 0` on the unrounded difference, and `0x61d0c3` divides the stored
float `d` by the stored float `t`. `0x61d0c3`: `100.0`
(0x80c854) compared below the quotient keeps a NaN quotient and caps a larger one. `0x61d02e`: the
lead is `ftol(t·1000)` minus half of `GetSequenceInfo(anim)` `+4`, the sequence length; a positive
lead goes to `+0x74`, otherwise the unit plays the animation at once when it resolves to its own
sequence (`+0x34` nonzero, `0x61d0e8`). `0x61d0fa`: a `t` of exactly 0 plays Stand. `0x61d18e`:
for a sequence with flag 0x1 the speed is `length·0.001/t`, else 1.0, through
`CM2Model::PlayAnimation(-1, 144, 0, 0, speed, 1, 1)`.

## Outcome flags

At each launch a missile that is not a hit takes a flag from its outcome code: 1, 3 and 6 set 0x8
(missed); 2, 7 and 8 set 0x20 (resisted); 4 sets 0x40 and rewrites the code to 9; 5 sets 0x10
(blocked); 11 sets 0x80 (reflected). Codes 9 and 10, 0, and codes above 11 set nothing.

Addresses: `0x61d720`, `0x61d76c`, `0x61d784`
Confidence: verified
Proof: one
Test: none

`0x61d723` returns for flag 0x1. The switch takes `code − 1`, bounded by `cmp 0xa`, through the bytes
`0x61d784` (`00 01 00 02 03 00 01 01 05 05 04`) into the six cases at `0x61d76c`; case 5 is the
bare `ret`. The parry case stores 9 into `+0x48` (`0x61d756`).

## Aiming and leading

`0x61d230` sets the destination. The aim point is the target's attachment, or its position raised
by 0.75 of `+0x90`·`+0xbc`. A missed missile flies half a second past the aim point, turned by a
random angle up to 3° about the horizontal axis across its path. A missile at a unit leads it: with
`v` half the target's velocity, capped first at the missile's speed `s`, and `r` the aim point
from the missile, it takes the least positive root of `(|v|² − s²)t² + 2(v·r)t + |r|² = 0` and
aims at `aim + v·t`; with no positive root, a negative discriminant or a near-zero quadratic term
it aims at the point itself.

Addresses: `0x61d230`, `0x61d2c1`, `0x61d33a`, `0x61d36b`, `0x61d39f`, `0x61d3d5`, `0x61d41b`, `0x61d4cc`, `0x61d51c`, `0x61d589`, `0x61d5ef`, `0x61d61e`, `0x61d63b`, `0x61d6ad`, `0x61eec0`
Confidence: verified
Proof: one
Test: none

The target resolves with mask 1. `0x61d2c1` multiplies by 0.75 (0x8012cc). Missed: `d` = aim −
position; `|d|` below 2^-22, or a unit `z` within 2^-22 of 1, returns unchanged. `0x61d36b`: the
path `V = u·(|d| + s·0.5)`. `0x61d39f`: the axis `(−dx·dz, −dy·dz, dx² + dy²)`. `0x61d3d5`: a
`CRandom::uint32` draw `r` gives `f` in [1, 2) from its low 23 bits, then `2 − f` for `r < 0` and
`f − 2` otherwise; `0x61d41b` takes `sin(f·π/2)·0.0523599` (π/60). The destination is position
plus `BuildAxisRotation(axis, angle)` applied to `V`. A target that is not a unit (`0x61d4cc`) is
aimed at directly. `0x61d51c`: `0x61eec0` returns the mover's `+0x84` times its `+0x5c` direction;
`speed² < |v|²` normalizes `v` (when `|v|` is at least 2^-22) and scales it to `s`; `0x61d589`
halves it. `0x61d61e` gives up on a discriminant below 0 and `0x61d624` on `a²` below 1e-5. `t₁ = (−b − √D)·(0.5/a)`, `t₂ = (√D − b)·(0.5/a)`, each kept when above 0 and below the
best so far, from `FLT_MAX`; `0x61d6ad` keeps the aim point while the best is still `FLT_MAX`. The lead point rounds `v·t` to a
float in x and y before adding, while z adds the unrounded product (`0x61d6e5` to `0x61d702`).

## Flight each frame

`0x61ce00(dt, visible)` advances a flying missile through `0x61e2a0` and fades a missed or deflected
one. A missile of speed 0 has arrived and stays put. Each frame the missile re-aims at a live target
unit unless it missed, plays the target's delayed reaction when its time comes, and moves
`dt·speed` toward the destination. When that step is longer than the remaining distance it arrives
instead, unless it is already an impact model. A moving missile faces its direction of travel with
its up axis in the vertical plane.

Addresses: `0x61ce00`, `0x61e2a0`, `0x61e2b9`, `0x61e2e9`, `0x61e30c`, `0x61e3e8`, `0x61e3f7`, `0x61e45c`, `0x61e4e0`, `0x61e56b`, `0x61e655`, `0x61e753`, `0x61e78f`
Confidence: verified
Proof: one
Test: none

The world-object callback 0x4814c0, which `0x61e2a0` installs (`0x61e661`), calls `0x61ce00` with
the float at `[0xb4b2bc]+0x3f4` and its visibility argument. `0x61e2b9 test ah,0x44; jnp` returns at
once only for a speed equal to 0. `0x61e2e9`: a live target is one that resolves and is a unit.
`0x61e30c`: at `+0x74` the unit plays 24 or 30 as at launch. `0x61e3e8` arrives on `step² >
|d|²`, so a NaN keeps flying. `0x61e4e0`: the side axis is `(−uy, ux, 0)`; at or below 1e-5
squared (`0x61e56b`) the axes become (0, 0, 1), (1, 0, 0) and (0, 1, 0); otherwise up is `u × s`.
`0x61e655` creates the world object once through `CWorld::CreateLightObjectB`, with the model's
box. `0x61e753` moves the sound with velocity `u·speed`; `0x61e78f` starts it once from `+0x40`.

## Fading out

A missed or deflected missile fades as it nears the destination. Within `r` yards, `r` being half
a second of flight for a miss and 0.7 seconds for a deflection, its model alpha is `a²(3 − 2a)`
with `a` the remaining distance over `r`, clamped to [0, 1].

Addresses: `0x61d790`, `0x61d7b9`, `0x61d7e5`, `0x61d7f7`, `0x61d81d`, `0x61d843`
Confidence: verified
Proof: one
Test: none

`0x61d7b9` picks 0.5 for flag 0x8, else 0.7 (0x80c85c). `0x61d7e5` acts only when the squared
distance is below `r²`; equal, larger or NaN leave the alpha alone. `0x61d7f7` sets alpha 0 below 0,
`0x61d81d` alpha 1 above 1, and `0x61d843` computes `(3 − 2a)·a·a`.

## Arrival

`0x61e1d0(target, d)` snaps the model to the destination, stops the flight sound and picks one of
three arrivals: with no live target, `0x61d870` when the spell has a record and a visual; with a
live target and the hit flag, `0x61dc50`; otherwise `0x61dd50`. `d` is the last remaining distance
vector.

Addresses: `0x61e1d0`, `0x61e21c`, `0x61e24c`, `0x61e269`, `0x61e281`
Confidence: verified
Proof: one
Test: none

`0x61e2a0` passes the target only when it resolved as a unit (`0x61e404`). `0x61e21c` takes the
spell record for a positive id and `GetSpellVisualRecord` of it. `0x61e24c` tests the target,
`0x61e269` the hit flag; every other path reaches `0x61e281`.

## Arrival without a target

`0x61d870` plays the visual's `+0x34` kit on the caster as kind 3 at the arrival point. Then for each
GUID it carries: with reasons, a nonzero reason not yet reported floats the miss word, raises
`UNIT_COMBAT` and writes the miss line, unless the spell has Attributes 0x100 (only the line
remains); without reasons, the target plays the impact kit `+0xc` as kind 1 and, while alive with
the spell's aura, the state kit `+0x10` as kind 2. A visual `+0x30` effect model replaces the
missile's model as an impact model; otherwise the missile is freed.

Addresses: `0x61d870`, `0x61d8e7`, `0x61d99d`, `0x61d9c8`, `0x61da14`, `0x61da1b`, `0x61da4d`, `0x61da86`, `0x61da8d`, `0x61dbaa`, `0x61dc20`
Confidence: verified
Proof: one
Test: none

`0x61d870` takes the spell and visual records. The GUIDs resolve as units; `0x61d99d` tests the
reason count `+0x68`. `0x61d9c8` gates `ShowMissWord` and `0x494720` on the record and
Attributes; `0x61da14` calls `0x62bab0(spell, reason, caster, target, 0)`. `0x61dc20(unit, spell)`
scans the 48 aura dwords from `[unit+0x110]+0xa4`. `0x61da86` sets 0x100 after the loop. `0x61da8d`
reads the `SpellVisualEffectName` record and needs a nonempty model path; the new model keeps the
old base translation, speed 0, joins the world object and the impact list, and `0x61dbaa` stores
`now + (end − start) + 500` from key bone −1 at `+0x50`, which nothing reads for that list, with
flag 0x2. The old model is closed either way.

## Hit arrival

`0x61dc50(target, spell, visual)` plays the target's wound animation, or its death animation at no
health, then the impact kit `+0xc` as kind 1 and, while alive with the spell's aura, the state kit
`+0x10` as kind 2. A missile from a bow, crossbow or thrown weapon with an attachment sticks in a
target that has a model, for 5,000 ms. It floats no word and writes no line.

Addresses: `0x61dc50`, `0x61dc58`, `0x61dcac`, `0x61dcde`, `0x61dcf5`, `0x61dd10`
Confidence: verified
Proof: one
Test: none

`0x61dc58` tests `[[target+0x110]+0x40] > 0` and calls `CGUnit_C::PlayWoundAnimation(0)` or
`CGUnit_C::PlayDeathAnimation`. The kits are looked up in the `SpellVisualKit` store by id, signed.
`0x61dcf5` needs `+0x3c` not −1 and `0x61dd0c` the unit's `+0xdc` or `+0xd8`; `0x61dd10` tests flag
0x4 and calls `0x61e7c0(target, 5000)`, returning 1 when it sticks; every other path frees the
missile and returns 0.

## Misses, resists and reflections

`0x61dd50(target, d)` reports the outcome once at a target: the miss word and `UNIT_COMBAT` unless
the spell has Attributes 0x100, then the miss line. A target not blocking, missing or deflecting
plays 30 Dodge. A block sticks the missile in the target for 5,000 ms; a resist swaps its model for
`Spells\Resist_Immune_Effect.M2` stuck for its animation plus 500 ms. A deflection ricochets at half
speed with 198 Deflect; a reflection turns it back at its caster. A miss frees it.

Addresses: `0x61dd50`, `0x61dd75`, `0x61dd85`, `0x61ddd1`, `0x61ddf7`, `0x61de53`, `0x61de61`, `0x61df00`, `0x61df13`, `0x61df9a`, `0x61e050`, `0x61e109`, `0x61e15f`, `0x61e17b`
Confidence: verified
Proof: one
Test: none

`0x61dd75` skips the report without a target, `0x61dd85` once 0x100 is set; `0x61ddd1` writes the
line with `+0x48` even without a record. `0x61ddf7` tests 0x58; the Dodge plays when the unit's
model resolves it. `0x61de53`: 0x10 calls `0x61e7c0(target, 5000)`; `0x61de61`: 0x20 builds the new
model at the old base transform. `0x61df00` frees a missed missile. Deflected: `v = −d` normalized,
freed when `|v|` or `||v| − |vz||` is below 2^-22; two draws give angles `f·π/2` with `f` as in
aiming; `w` is `(vy, −vx, 0)` turned about `v` by the first, and `v` is turned about `w` by the
second (`0x61e050`). `0x61e109` halves the speed and sets the destination `0.7·speed` along `v`;
`0x61e15f` swaps 0x40 for 0x8. `0x61e17b`: a reflection copies the caster GUID over the target,
clears 0x180, takes `+0x4c` as its code (0 sets the hit flag) and relaunches from its position.

## Stuck missiles

`0x61e7c0(unit, ms)` sticks a missile into a unit at its attachment and keeps it there for `ms`. It
plays 191 Impact, keeps its world pose relative to the attachment, and joins the unit's stuck list,
ordered by expiry. The unit's update frees each one whose time has come; the impact list is freed
model by model, once no emitter of the model is still emitting or holds a live particle.

Addresses: `0x61e7c0`, `0x61e7da`, `0x61e806`, `0x61e830`, `0x61e84c`, `0x61e88a`, `0x61e8c7`, `0x61e930`
Confidence: verified
Proof: one
Test: none

`0x61e7da` returns 0 when the unit's model lacks `+0x3c`. The insertion walks `+0xb0` from its
head while `node+0x50 − expiry` is negative (`0x61e806`). `0x61e84c` attaches the model; the world
object is released. The pose is `inverse(attachment matrix, scale)·base` (`0x61e8c7`), the scale
`+0x94`·`+0x90` times the mount's `+0x9c` or the unit's `+0x98`. Flag 0x2 is set. Object slot 14,
`0x614a90`, frees the head of `+0xb0` while `now − expiry` is not negative; `0x61e930`, from the
world frame, frees each impact-list missile with no model or `CM2Model::HasLiveParticles` false
(`effects`, Stopping and draining).
