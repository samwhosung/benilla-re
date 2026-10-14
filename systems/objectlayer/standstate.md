# objectlayer: sitting and standing

How the local player changes its stand state: the one setter and its gates, the callers it has, the
text-emote checks in front of it, every place that stands a seated unit, and how a seated body
turns. The stand state is byte 0 of `UNIT_FIELD_BYTES_1` (descriptor `+0x210`): 0 standing, 1 sitting,
2 sitting in a chair, 3 sleeping, 7 dead, 8 kneeling, 9 submerged. The send, the local apply and
the animation that follows are in `interaction`, `units` and `sheathing`; the movement flag bits
are in `collision`.

Roots: `0x5ed430`, `0x60be30`, `0x5ef560`, `0x47db40`, `0x5fa550`

## The sit gate

`0x5ed430(state)` sends `CMSG_STANDSTATECHANGE` (0x101, a u32 state) and applies the state at once
through `0x6127b0`. It returns with no packet, no apply, no message and no sound when any of these
holds, tested in this order: the unit is not the local player; its mount display (field 133) is above
0; `UNIT_FIELD_FLAGS` bit 20 is set; its health is 0 or below; `UNIT_DYNAMIC_FLAGS` bit 5 is set; it
channels (field 144) or has `+0xd58` bit 0x400 set; `+0xc8c` is nonzero. Then the movement flags
decide: state 0 reads none; state 3 refuses on 0x30 and then on 0x20000f; any other state refuses on
0x20000f.

Addresses: `0x5ed430`, `0x5ed440`, `0x5ed44b`, `0x5ed46c`, `0x5ed487`, `0x5ed497`, `0x5ed4a9`, `0x5ed4b2`, `0x5ed4c3`, `0x5ed4cc`, `0x5ed4d8`, `0x5ed4e6`, `0x5ed4ec`, `0x5ed4f8`, `0x5ed501`, `0x5ed53f`, `0x5ed566`, `0x4181f0`, `0x7c6d90`, `0x7c6dd0`, `0x7c6e50`, `0x7c6e80`, `0x7c61f0`
Confidence: verified
Proof: one
Test: none

| state | refused while the movement word `+0x40` has |
|---|---|
| 0 | nothing |
| 1, 2, 8, any other | 0x20000f: forward, backward, strafe left or right, swimming |
| 3 | 0x20003f: those and a turn |

`0x5ed4ec` jumps into the second test, so state 3 takes both. The bits are those of `StartMove`,
`StartStrafe`, `StartTurn` (0x10 left, 0x20 right) and `StartSwimming` (0x200000). Falling (0x2000),
pitch (0x40, 0x80) and root are in neither mask: a jump from a standstill does not block a sit and
a running one does, by 0x1. `0x5ed566` is the only exit of a refusal; the function calls `0x468550`
and nothing else before it. `0x4181f0` grows the buffer by 4 for the state. The movement object is
the unit's own, at `+0x118`. There is no charm or possess test.

## The setter's callers

Six call sites reach `0x5ed430`: the `SitOrStand` script function (`0x48b953` with 1 when the
descriptor byte is 0, `0x48b95d` with 0 otherwise), `SendTextEmote` (`0x5ef652`), the wrapper `0x60be30`
(`0x60be45`), `StartAttack` (`0x5ecc9b`, with 0 when the descriptor byte is nonzero) and the idle
handler of the world frame (`0x482f91`, with 1). No other reference to its address exists in the
image. `0x60be30(state)` calls it for a player type only, and every one of its nine callers passes
0. The emote state leg sends only an emote state whose bit is in the mask at `0x85ff10` (0x10b: states
0, 1, 3 and 8).

Addresses: `0x48b920`, `0x48b953`, `0x48b95d`, `0x5ef560`, `0x5ef63e`, `0x5ef64f`, `0x5ef652`, `0x60be30`, `0x60be45`, `0x5ecb70`, `0x5ecc90`, `0x5ecc9b`, `0x482ea0`, `0x482f91`, `0x85ff10`
Confidence: verified
Proof: one
Test: none

The scan covers every `e8` and `e9` in the image and the little-endian dword of the address in
every section: six calls, no jumps, no other occurrence. The `Emotes` rows with kind 1 (`+0x10`),
read from the shipped file, have state parameters (`+0x14`) 26 STATE_STAND 0, 13 STATE_SIT 1, 12
STATE_SLEEP 3, 68 STATE_KNEEL 8, 373 STATE_SUBMERGED 0, 65 STATE_DEAD 7 and 313 STATE_AT_EASE 2: the
mask admits the first five and refuses 7 and 2. State 2 is therefore not sent by any client path.
The idle sit (after 5 minutes without input) goes through the same gate, so a swimming player is not
seated; its own tests are in `ui/idle`.

## The emote checks in front

A text emote passes `0x47db40` before it reaches the setter. That check sets its out flag for an
emote row with flag 0x4000 when the movement word has any of 0x20ff, which includes turning,
pitch and falling and not swimming. `SendTextEmote` then asks `0x5fa550`; when that returns nonzero,
as it does for an ordinary player, it prints `ERR_NOEMOTEWHILERUNNING` (message 0x139) and sends
nothing. When it returns 0 (see below) the emote goes on. A refusal by the setter after that sends `CMSG_TEXT_EMOTE` (0x104) all the same.

Addresses: `0x47db40`, `0x47dbab`, `0x47dbb3`, `0x47dbbc`, `0x5ef5c3`, `0x5ef5d0`, `0x5ef5d7`, `0x5ef5e0`, `0x5ef5e5`, `0x5ef674`, `0x5ef688`, `0x5fa550`, `0x5fa566`, `0x496720`
Confidence: verified
Proof: one
Test: none

So `/sit` while swimming still broadcasts its line and does not sit, `/sit` while turning at the
keyboard or falling shows the message although the key would sit, and the two masks are nearly
complements: 0x20ff blocks turns and falls, 0x20000f blocks swimming. `0x5fa550` returns 0 when
`UNIT_FIELD_FLAGS` bit 1 is clear and 0xc00004 is set, follows the unit named at descriptor `+0x10`
when bit 24 is set, and else returns 1 for a player with no GUID there and bit 0 clear. The `0x30` test of the
setter is shadowed for `/sleep`: it is reached only when `0x5fa550` returns 0.
The message id indexes the table at `0xb4b498` (20 bytes each) to the string `ERR_NOEMOTEWHILERUNNING`.

## What stands a seated unit

A seated unit stands, through `0x60be30(0)` or `0x5ed430(0)`, at the sites below. Each tests only for
a nonzero stand state, so sitting (1), the chair (2) and sleeping (3) behave the same.

Addresses: `0x513d1f`, `0x514e0c`, `0x514ef4`, `0x514fb3`, `0x6020d6`, `0x60ad98`, `0x60de65`, `0x60e14b`, `0x61121b`, `0x5ecc9b`, `0x514da0`, `0x514e80`, `0x514f50`, `0x6018f0`, `0x60abe0`, `0x611130`, `0x60f5b0`
Confidence: verified
Proof: one
Test: none

| site | when |
|---|---|
| `0x513d1f` | the jump command, once `0x60f5b0` allows a jump |
| `0x514e0c` | the forward axis total is nonzero (autorun, forward, backward, both mouse buttons) |
| `0x514ef4` | the strafe axis total is nonzero; the turn keys count as strafe under mouselook |
| `0x514fb3` | the turn total is nonzero and mouselook (command bit 0x1) is clear |
| `0x60e14b` | a movement message of the standing class is sent (`unitmovement`) |
| `0x61121b` | the click-to-move commit |
| `0x5ecc9b` | `StartAttack` on a seated unit |
| `0x6020d6` | the monster move handler, after it sets a spline up (not in the near case) |
| `0x60ad98` | a display change, with the displayed and native displays equal, health above 0 and a nonzero stand byte |
| `0x60de65` | a facing change while sleeping (state 3) |

## The mouse and a seated unit

A mouse motion stands nobody. A right-button release stands a unit when its click dispatches
(`0x514ae0` returns 0) and the pick reaches an attack, or a walk with `AutoInteract` on. With it
off, which is the default outside locale index 1, only the attack stands the unit.

Addresses: `0x5145e0`, `0x514ae0`, `0x5ecc9b`, `0x5e0378`, `0x6102b0`, `0x610100`, `0x60fcc0`, `0x492dca`, `0x611130`, `0x60f900`
Confidence: verified
Proof: one
Test: none

A motion turns the camera and hands the facing to the body only when `0x5145e0` holds, which
needs the stand state 0 (`input`). An attackable target reaches `StartAttack` with no other
condition. Every walk that ends in `0x611130`, from `0x6102b0`, `0x610100` or `0x60fcc0`, needs
`AutoInteract` (`unitmovement`, `units`). With it off a press on the ground gives pick state 0 and the
nothing leg stops at its test. With it on, the ground leg's walk (`0x5e0378`) and a press on empty
sky (`0x492dca`) stand the unit (`ui/mouseover`).

## A seated body turns

A unit sitting or in a chair turns freely and stays seated. `0x60de30` stands only from state 3,
and the per-frame facing step `0x6103d0` reaches it through `0x60e940`, whose gate `0x5144e0` tests
health, control flags and far sight and no stand state. The movement message for that turn is
`MSG_MOVE_SET_FACING`, which the standing class of `0x60e0a0` leaves out.

Addresses: `0x60de30`, `0x60de56`, `0x60de5c`, `0x60de65`, `0x60de00`, `0x60de1f`, `0x60e940`, `0x60e94e`, `0x5144e0`, `0x6103d0`, `0x610502`, `0x6108af`, `0x60aab7`, `0x60e0a0`, `0x619860`, `0x615fc6`
Confidence: verified
Proof: one
Test: none

`0x6103d0` wraps its angle to [0, 2π) through `0x455b30` (`0x6104f3`) and calls `0x60e940`
(`0x610502`); its one caller is `0x6108af` in the unit's per-frame tick. `0x60e940` raises bit 0 of
`0xc4da48` around `0x60de00` (`0x60e964`, `0x60e975`), which calls `0x60de30` (`0x60de1f`). The
turn integrator reaches `0x60de30` at `0x60aab7`, behind a movement-flag test with no stand test. The
hand-off `0x5152e0` also calls it (`0x51531e`) and carries the stand test.
