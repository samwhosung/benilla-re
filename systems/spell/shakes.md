# spell: shakes

The camera shake a spell visual kit plays: which node owns it, when it fires, and where it shakes from.

A `SpellVisualKit` record names a camera shake group in its dword `+0x38`; `0x6ecb40` plays the group's
shakes at a position (`visuals`, Camera shake groups). The kit player `0x60edf0` (`objectlayer`, Playing a visual
kit) decides when and where. The one-time arm pass of the first effect node plays the shake from that node's
position, and a kit that creates no node plays it from the tail. The producers are the kit player
and the two `$SHK` animation event arms (`objectlayer`, dynamic objects and game objects).

Roots: `0x6ecb40`

## Kit shake owner

A kit with `+0x38` above 0 plays exactly one shake group per play. The kit player raises the shake bit 0x10
in its local flags at entry; each effect spawner snapshots the flags into the new node's `+0x2c`, then clears
0x11 and sets 0x400 in the local, so only the first node created carries 0x10. That node's one-time arm
pass plays the group on its first frame. A kit that creates no node plays the group at the end of the play,
at the supplied position or the unit's own. The stage 2 node of the kit player is created already armed, so
its kit plays from the tail too.

Addresses: `0x60ee2e`, `0x60ee36`, `0x60eda6`, `0x60edc7`, `0x60edd5`, `0x60edd8`, `0x61fdd0`, `0x61fe5b`, `0x620580`, `0x62058f`, `0x620595`, `0x620be0`, `0x620df5`, `0x620e02`, `0x620e0e`, `0x620e11`, `0x60f3ca`, `0x60f3d4`, `0x60f424`, `0x61ffa0`, `0x61fffd`, `0x60f4c1`, `0x60f4c8`, `0x60f4cf`, `0x60f4d6`, `0x60f4e6`, `0x6ecb40`, `0x60edf0`, `0x60ed40`
Confidence: verified
Proof: one
Test: none

`0x60ee2e cmp [esi+0x38],ecx; je` then `0x60ee36 mov [ebp-0xc],0x10`. `0x60eda6` passes the flags to
`0x61fdd0`, which stores them at `0x61fe5b`; `0x60edd5 and edx,0xffffffee; or edx,0x400` clears the shake bit
after every call of the spawner `0x60ed40`. Bit 0x10 of a node's `+0x2c` has one reader, `0x620df5`. The
per-frame update `0x620580` calls the arm pass `0x620be0` unless `+0x2c` has 0x800 or 0x100 (`0x62058f test
ah,9`); the pass sets 0x800 first and plays the group at `0x620e11` with `&node+0x48` when the node's kit has
`+0x38` above 0. The tail at `0x60f4c1` runs when no node was recorded (`[ebp-0x1c]` is 0) and plays at
the extra position `[ebx+0x14]`, else the unit's position (vtable `+0x14`). The stage 2 node (`0x60f3ca`,
the flag 0x1000 test at `0x60f3d4`) goes through `0x61ffa0`, which ors 0x800 in (`0x61fffd`), so the arm pass
skips it.

## Kit shake position fault

When the first node of a kit is bone-attached, its shake plays at the world origin. The node's position
`+0x48` is zero from its constructor; the arm pass reads it at `0x620e0e` and fills it from the attachment
only afterwards (`0x62101d`). The spawner of every bone slot (`0x61fdd0`, nine of the kit's ten slots)
never writes it. A node planted in the world (`0x61fcf0`, the kit's `+0x30` effect) holds its spawn position
already. The camera drops a shake whose position is more than 80 yards from it (`uicamera`, effects), so
the origin shake plays only for a camera within 80 yards of the origin.

Addresses: `0x61f490`, `0x61f4c1`, `0x61fdd0`, `0x61fcf0`, `0x61fc4a`, `0x61fd4b`, `0x61fec0`, `0x61ff11`, `0x61ff4f`, `0x620580`, `0x6205f1`, `0x620be0`, `0x620e0e`, `0x62101d`, `0x712d50`, `0x511d40`, `0x5116e0`
Confidence: verified
Proof: one
Test: none

A scan of the listing over `0x600000` to `0x630000` for stores and `lea` of displacement `0x48` off
a node register finds the writes of a node's `+0x48`: the constructor `0x61f4c1`, the planted spawners
`0x61fc4a` and `0x61fd4b`, the loop-sound spawner `0x61ff11` and `0x61ff4f`, the per-frame refresh `0x6205f1`
(only for a node with bit 0x4000) and the arm pass's own fill `0x62101d`. `0x61fdd0` (`[0x61fdd0, 0x61feb9)`)
holds none. `0x62101d` follows the vtable call `0x712d50` that resolves the attachment, and it sits 0x20e
bytes after the shake call. The bit 0x4000 that would make the refresh run first is set only inside the
kit's `+0x30` block (`0x60f0a4`), after that slot's own record check.
