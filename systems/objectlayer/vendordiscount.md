# objectlayer: the vendor discount

What `0x612b80` returns for a vendor and a player, and the three places that use it. The prices it
scales are `ui/merchant` (repair) and `ui/taxiroute` (fares); the reaction it reads is `reaction`.

Roots: `0x612b80`

## The discount

`0x612b80(vendor, player)` is the sum of three terms: 0.1 when the vendor's reaction toward the player is 5 or
more; 0.05 when the vendor has the PvP flag and the player's honor rank byte is 6 or more; 0.05 more when that
byte is 8 or more. The sum is left on the FPU stack unrounded, so the values are 0, 0.05, 0.1, 0.15 and 0.2. At 53-bit x87 precision 0.15 is not a float;
at 24-bit precision the sum rounds to a float.

Addresses: `0x612b80`, `0x612b86`, `0x612b89`, `0x612b8a`, `0x612b8c`, `0x612b93`, `0x612b98`, `0x612b9b`, `0x612b9d`, `0x612ba5`, `0x612ba8`, `0x612bae`, `0x612bba`, `0x612bbc`, `0x612bc2`, `0x612bc7`, `0x612bc9`, `0x612bcb`, `0x612bcf`, `0x612bd5`, `0x612bd7`, `0x612bdd`, `0x612be2`, `0x801620`, `0x8029c8`
Confidence: verified
Proof: one
Test: `objectlayer::vendor_price_discount_difftest`

`0x612b89` pushes the player and `0x612b8a` keeps the vendor as `this`, so the call is the vendor's reaction toward
the player. `0x612b98` is `cmp eax,5; jl`, and `0x801620` is the float 0.1. The honor terms do not depend on the
reaction: `0x612ba5` falls into the flag test. `0x612bae` and `0x612bba` test bit 12 of the vendor's flags `+0xa0`,
read directly with no owner lookup. `0x612bc2` reads byte `+0x1f` of the player block at `+0xe68`, the current honor
rank (`ui/honor`); `0x612bc7` returns for 0, `0x612bcb` for below 6, `0x612bcf` adds `0x8029c8` (0.05) and
`0x612bd5` returns for below 8, else `0x612bd7` adds it again. The compares are unsigned. A reaction of 5 or more
means a standing of Honored or better with a faction that has a reputation slot, a forced reaction of 5 or
more, or the persuaded rise from 4 (`reaction#ranks-that-surface`).

## Its users

Three call sites reach `0x612b80`, and no address of it is stored: the repair cost of one item (`0x4faf8a`) and the
two legs of the fare of a taxi node (`0x4dc45d`, `0x4dc4a2`). Each passes the open vendor or flight master, found
with the unit mask 8, and the active player. The trainer's service cost and a merchant's item prices are pushed
as the server sent them, with no discount.

Addresses: `0x4faf30`, `0x4faf8a`, `0x4dc3d0`, `0x4dc45d`, `0x4dc4a2`, `0x4fb150`, `0x4d92f0`
Confidence: verified
Proof: one
Test: none

The scan finds three `e8` calls and no dword of the address anywhere in the image. `0x4fb150` and `0x4d92f0` contain
three `fild` and three `fstp` each and no multiplication. How the products round is in `ui/merchant#item-repair-cost`
and `ui/taxiroute#fares`: the repair multiplier is rounded to a float, the taxi one is not.
