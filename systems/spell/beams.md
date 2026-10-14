# spell: beams

Which targets a kit's beam procedure draws to, and how long the list lasts.

A chain beam (Chain Lightning, Drain Life, Mind Flay) starts from a kit procedure of kind 0 or 12
(`objectlayer`, Kit character procedures). The procedure picks the beam's targets from the unit's channel
or its chain-target array, makes the beams (`visuals`, Chain effect creation) and drops the list. The
array is filled by the spell go and the chain-target message (`go`, `handlers`).

Roots: `0x6ecb90`

## Beam targets and owner

The procedure first makes a bare effect on the unit for the kit, which owns the beams so that they end with
it. The targets are the unit's channel object alone when the kit's spell is its channel spell, the channel
object guid is nonzero and the chain-target count is at most 1 (unsigned); otherwise the chain-target array
with its count. No targets draws nothing. The chain id, the strand count and the persistence flag are the low
bytes of the kit's first three parameters, the flag being whether the third is nonzero. The chain-target
count is set to 0 on every exit, so a list serves one play.

Addresses: `0x60da79`, `0x60da8a`, `0x60da95`, `0x60dac7`, `0x60dacf`, `0x60dad4`, `0x60dadd`, `0x60dae5`, `0x60daf2`, `0x60dafd`, `0x60db01`, `0x60db11`, `0x60db19`, `0x60db43`, `0x60db6d`, `0x60db72`, `0x6ecb90`, `0x620090`, `0x61f490`, `0x6210e0`, `0x605780`
Confidence: verified
Proof: one
Test: none

`0x60da8a` allocates 0x90 bytes and `0x60da95` runs the effect constructor; `0x60dac7` calls `0x620090(duration,
spell, kit, &guid)` with the unit's guid (`[esi+8]`), and `0x60dacf` releases the local reference. `0x60dadd`
compares the kit's spell (`[ebx+8]`) with `[desc+0x228]`; `0x60dae5` or-s the two dwords of `[desc+0x38]`, the
channel object, and a zero jumps to the array arm; `0x60dafd cmp edx,ecx; jbe 0x60db11` takes the channel
object (pointer `desc+0x38`, count 1) when `[esi+0xd48]` is at most 1. `0x60db01` loads the count and, from
`0x60db0b`, the array `[esi+0xd4c]`; a zero count or pointer leaves for `0x60db72`. The parameters are read
with the `+512.0` trick (`0x60db19`, `0x60db25`, `0x60db3b`, `0x60db50`): `0x60db43 setne` makes the
flag. `0x60db72` stores 0 in `[esi+0xd48]` and is the target of the three skips and of the fall-through after
the call at `0x60db6d`. The array is filled by `0x605780` (`go`, Caster state at go).
