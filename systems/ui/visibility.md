# ui: visibility, alpha and scale

How a frame is shown and hidden, when its `OnShow` and `OnHide` scripts run, and how alpha and
scale pass from a frame to its children.

A `CSimpleFrame` has two flags: shown (`+0xd0`), what the frame asks for, and visible (`+0xd4`),
whether it is shown and its parent is visible. Only a visible frame is in a level of its stratum
(`strata`), lays itself out, takes input and draws. The frame's slot 0x88 makes it visible and
slot 0x84 makes it invisible; both walk the frame's regions and children. Alpha and scale are
stored per frame and pushed down to the regions and children when they change. All of this is in
`CSimpleFrame.cpp` (`0x769090` to `0x76c4d0`).

Roots: `0x76ae10`, `0x76ad50`, `0x76b260`, `0x76b290`, `0x76a690`, `0x76ac10`, `0x76ac50`, `0x76ac90`

## Showing a frame

Making a frame visible (slot 0x88) requires it to be shown and its parent, if any, to be visible;
otherwise it answers 0 and changes nothing. A frame that is not yet visible then, unless it is
loading, ends the layout deferral of itself and of its title region; becomes visible; joins its
level; shows its regions; makes each child visible in turn, which each child does only if it is
shown itself; raises itself when it is toplevel; and runs its show slot 0x30. It answers 1, also
when it was visible already.

Addresses: `0x76ae10`, `0x76ae13`, `0x76ae23`, `0x76ae2d`, `0x76ae3b`, `0x76ae49`, `0x76ae58`, `0x76ae6c`, `0x76ae7b`, `0x76ae85`, `0x76aeac`, `0x76aed5`, `0x76aee0`, `0x76aeec`, `0x76aef5`, `0x775750`
Confidence: verified
Proof: one
Test: `ui::effective_visible_show`

`0x76ae10` tests `+0xd0` (`0x76ae13`) and the parent `+0x9c`'s `+0xd4` (`0x76ae23`, `0x76ae2d`),
then `+0xd4` (`0x76ae3b`); with `+0x114` clear it calls `0x768060(0)` on the frame's layout part
and on `+0xa8` (`0x76ae58`, `0x76ae6c`). It stores 1 at `+0xd4` (`0x76ae7b`), calls
`0x764780(frame, 0)` (`0x76ae85`), 0x77fcb0 for each region of `+0x1b8` (`0x76aeac`) and slot
0x88 of each child of `+0x300` (`0x76aed5`), `0x76a5b0` when bit 1 of `+0xb4` is set
(`0x76aee0`), and slot 0x30 (`0x76aef5`). The Lua `Show` (`0x775750`) stores 1 at `+0xd0` and
calls slot 0x88.

`ui::effective_visible_show` runs `0x76ae10` whole, with `0x764780` and `OnShow` stubbed and empty child lists.

## Hiding a frame

Making a frame invisible (slot 0x84) acts only on a visible frame. Unless it is loading, it defers
the layout of itself and its title region; it clears visible, leaves its level, hides its regions,
makes every child invisible, and runs its hide slot 0x34. A child keeps its own shown flag, so it
returns when the parent is shown again. The Lua `Hide` clears shown first.

Addresses: `0x76ad50`, `0x76ad53`, `0x76ad61`, `0x76ad70`, `0x76ad84`, `0x76ad93`, `0x76ad9d`, `0x76adbf`, `0x76adee`, `0x76adfd`, `0x775810`, `0x7758b1`, `0x7758bb`
Confidence: verified
Proof: one
Test: `ui::effective_visible_hide`

`0x76ad50` tests `+0xd4` (`0x76ad53`); with `+0x114` clear it calls `0x768060(1)` on its layout
part and `+0xa8` (`0x76ad70`, `0x76ad84`); it stores 0 at `+0xd4` (`0x76ad93`), calls
`0x764920(frame, 0)` (`0x76ad9d`), 0x77fc60 for each region (`0x76adbf`), slot 0x84 of each
child (`0x76adee`) and slot 0x34 (`0x76adfd`). The Lua `Hide` stores 0 at `+0xd0` (`0x7758b1`)
and calls slot 0x84 (`0x7758bb`).

`ui::effective_visible_hide` runs `0x76ad50` whole, with `0x764920` and `OnHide` stubbed and empty child lists.

## Show and hide scripts

The show slot runs the frame's `OnShow` script and the hide slot its `OnHide` script, each only
when the script is set and the frame is not loading. A frame loads from XML with the loading flag
`+0x114` set: shows and hides during the load run no script and leave layout alone. At the end of
the load the flag is cleared, the deferral of the frame, its title region and its regions is
ended, the `OnLoad` script runs, and then, when the frame is visible and still not loading,
`OnShow` runs once.

Addresses: `0x76b260`, `0x76b270`, `0x76b27b`, `0x76b290`, `0x76b2a0`, `0x76b2ab`, `0x769770`, `0x7697c2`, `0x7697cc`, `0x7697e0`, `0x769807`, `0x76a2f0`
Confidence: verified
Proof: one
Test: none

`0x76b260` tests the `OnShow` slot `+0x130` and `+0x114` (`0x76b270`) before `0x702690`
(`0x76b27b`); `0x76b290` does the same with `+0x138`. The load's first step `0x769770` stores 1 at
`+0x114` (`0x7697c2`) and defers the frame, `+0xa8` and each region of `+0x1b8` through
`0x768060(1)` (`0x7697cc` to `0x769807`). The last step `0x76a2f0` stores 0 at `+0x114`, ends
the deferrals when `+0xd4` is set, loads the child frames, runs `+0x118` (`OnLoad`) and then
`+0x130` when `+0xd4` is set and `+0x114` is still 0.

## Walking a show or hide

Neither walk copies a list. Each reads the next link of the current node after the call on it, so a
handler that hides or unlinks a later sibling changes what is visited, and one that frees the
current node breaks the walk. The visible flag is written before the walk, so a handler already
sees the new state, a `Show` of a frame already visible answers 1 and runs nothing, and a `Hide` from
a sibling's `OnShow` clears that later sibling's shown flag, which makes its own show refuse. The
region walks reach no script. Called on the same frame mid-walk, the opposite verb runs its whole body.

Addresses: `0x76ae10`, `0x76ae1d`, `0x76ae35`, `0x76ae43`, `0x76ae7b`, `0x76aea0`, `0x76aeac`, `0x76aeb1`, `0x76aed5`, `0x76aedb`, `0x76aef5`, `0x76ad50`, `0x76ad5b`, `0x76ad93`, `0x76adbf`, `0x76adc4`, `0x76adee`, `0x76adf4`, `0x76adfd`, `0x77fc60`, `0x77fcb0`, `0x77fcf9`, `0x76a860`, `0x76a910`, `0x768060`, `0x775810`, `0x7758b1`, `0x7758bb`
Confidence: verified
Proof: one
Test: none

The region loop is `call 0x77fcb0` then `mov ebx,[ebx+4]` (`0x76aeac`, `0x76aeb1`), and the child loop is the
virtual call at `0x76aed5` then `mov ebx,[ebx+4]` (`0x76aedb`), with the head re-read from
`[esi+0x1b8]` and `[esi+0x300]` and the terminator tested at each step. `0x77fcb0` registers a region
when its shown byte `+0xc4` is set, its parent is visible and it is not yet registered (`0x77fcbd` to `0x77fcd5`), and
`0x77fc60` removes it; each calls `0x768060`, `0x76a860` or `0x76a910` and nothing else. The gates return
0 for a frame that is not shown (`0x76ae1d`) or whose parent is not visible (`0x76ae35`). A visible
frame is the only one `0x76ae43` lets pass. The Lua `Hide` clears `+0xd0` (`0x7758b1`) before the virtual call.

## When the handlers run

`OnShow` and `OnHide` run to completion inside the `Show` or `Hide` call, at the transition of the
effective visibility, descendants first in child order and the frame's own last; a call that changes
nothing runs nothing. This holds for every frame class: 27 vtables hold the transitions, 23 verbatim
and four through wrappers that release a model or refuse an ownerless tooltip, and no class queues
a notify. The same holds for a `Show` or `Hide` that code issues inside the per-frame pass: the tooltip's
fade-out hide, the minimap's hover tooltip and the chat bubbles. Nested calls stop at 200 levels
of the Lua C stack, with the message `C stack overflow`, and at 225 with an error status.

Addresses: `0x775750`, `0x7757f1`, `0x7757fb`, `0x76b260`, `0x76b290`, `0x76b27b`, `0x76b2ab`, `0x76a3b3`, `0x6f65a0`, `0x6f65ae`, `0x6f65bf`, `0x6f65c9`, `0x6f65d6`, `0x530a60`, `0x530a80`, `0x530b20`, `0x530be9`, `0x4eb0c0`, `0x4eb880`, `0x4b1180`, `0x4b1186`, `0x505ce0`, `0x505d00`, `0x46fa60`, `0x77a750`, `0x77a780`, `0x788440`, `0x7791e0`, `0x789c90`, `0x78bd60`
Confidence: verified
Proof: one
Test: none

A dword census of `0x76ad50` and `0x76ae10` finds 23 `.rdata` slots each, always adjacent, and the
wrappers `0x505ce0`, `0x505d00`, `0x530a60` and `0x530a80` tail-jump into them (`0x505cf0`, `0x505d44`,
`0x505d5e`, `0x530a73`, `0x530ab5`). The notify slots `0x76b260` and `0x76b290` fire only at
`0x76b27b`, `0x76b2ab` and the post-load `0x76a3b3`. The overrides add work around the base: the edit box fires
Lua `OnShow` before autofocus and `OnHide` before clearing focus, the message scroll frame fires it first, and
the button, slider and colour select clear state before Lua runs. `0x6f65a0` increments the call depth and
compares with `0xc8` (raising `C stack overflow`) and `0xe1` (the throw of status 5). The pass calls a hide
at `0x530be9` (tooltip), `0x4eb0c0` and `0x4eb880` (minimap) and `0x4b1186` (chat bubble).

## A second show after load

A frame that its own `OnLoad` or a child's makes visible after the load flag is cleared gets an
`OnShow` from that show and a second from the load's end, which fires whenever the frame is visible
then; a hide during the load is dropped, while one after the flag is cleared fires `OnHide`. A
scroll frame builds its scroll child inside its own load, so script runs while the scroll frame's
flag is still set and its own show and hide fire nothing.

Addresses: `0x76a2f0`, `0x76a2ff`, `0x76a36f`, `0x76a387`, `0x76a38c`, `0x76a396`, `0x76a3a6`, `0x76a3b3`, `0x769770`, `0x7697c2`, `0x786bc0`, `0x786bf4`, `0x6ee280`, `0x6ee4d6`, `0x6ee4ea`
Confidence: verified
Proof: one
Test: none

`+0x114` has three writers: the constructor, `0x7697c2` (1) and `0x76a2ff` (0); `0x6ee280` runs the
pre-load, the load and the post-load in sequence. After clearing, `0x76a2f0` builds the children
(`0x76a36f`), fires `OnLoad` (`0x76a387`) and then `OnShow` when `+0xd4` (`0x76a38c`), the slot
(`0x76a396`) and a clear `+0x114` (`0x76a3a6`) hold; nothing reads `+0x138` there. `0x786bc0` loads the
base then builds the `ScrollChild` through `0x6ee280` (`0x786bf4`).

## Reparenting by script

`SetParent` takes nil, a frame name or a frame table. Nil leaves the frame with no parent, a string or
number names a global frame (no `$parent` expansion), a table must hold a frame, and a missing
argument raises `Couldn't find region named`. A parent that is the frame or one of its descendants
raises `Would create a loop parenting to %s`. The same parent changes nothing; otherwise a visible frame is hidden before and shown after only if the new parent is nil or
visible. It takes the new parent's stratum, level plus one, scale and `0x2000` bit, which `0x76a400` hands to the whole subtree, or 3, 0, 1.0 and 0 for nil. Its children keep their levels.

Addresses: `0x7a1550`, `0x7a15e5`, `0x7a15ee`, `0x7a1606`, `0x7a1637`, `0x7a167c`, `0x7a177f`, `0x7a1781`, `0x7a1785`, `0x7a1794`, `0x7a17dc`, `0x76ab10`, `0x76ab20`, `0x76ab3d`, `0x76ab65`, `0x76ab92`, `0x76aba0`, `0x76abc7`, `0x76abe5`, `0x76abf7`, `0x76a400`, `0x76aba3`, `0x76a470`, `0x76a4f0`, `0x76a554`, `0x76c760`, `0x774560`
Confidence: verified
Proof: one
Test: none

The change is slot 0x14, which no class overrides, so it is `0x76ab10`. It hides the frame at `0x76ab3d` and shows it again only if the new parent is nil or `[parent+0xd4]` is set (`0x76abe5` to `0x76abf7`). It calls `0x76a400` with the parent's `0x2000` bit (`0x76ab92` to `0x76aba0`), or 0 for nil (`0x76abc7`), and `-1` (`draworder`). `0x7a1550` tests the type of argument 2: 0 calls `[vtable+0x14](0)` (`0x7a15ee`); string or number calls
`0x76c760` with the Frame id `[0xcf0c10]` (`0x7a1637`); a table reads element 0 with `lua_rawgeti`
and checks the id (`0x7a16ea`). The loop guard walks the candidate's `+0x9c` chain against the
frame (`0x7a1781` to `0x7a178d`) and raises (`0x7a17dc`) rather than returning. A census of the dword `0x76ab10` in the
image finds it at offset 0x14 of 27 frame-family vtables, each stored by a constructor. `0x76ab20` returns for an
unchanged parent; `0x76a470` returns for an unchanged stratum and otherwise recurses into every child with the new value; `0x76a4f0`
is called with propagate 0 (`0x76ab65`, `0x76aba3`), so `0x76a554` skips the children. A child created at level 1
under a parent that moved to level 5 stays at 1, below its parent.

## Alpha

A frame's alpha is a byte at `+0xc8`, 255 at construction. Setting it stores the byte and, when
it changed, refreshes each region and sets every child's alpha to the same byte, recursively: a
parent's alpha replaces its children's, it does not multiply them. The Lua `SetAlpha` clamps its
number to 0 to 1, NaN going to 1, multiplies by 255 and truncates.

Addresses: `0x76a690`, `0x76a69a`, `0x76a6a2`, `0x76a6d0`, `0x76a702`, `0x774e90`, `0x774f32`, `0x774f49`, `0x774f5e`, `0x774f66`, `0x774f6e`, `0x7ffe58`
Confidence: verified
Proof: one
Test: `ui::set_alpha_scaled`, `ui::set_alpha_value`

`0x76a690` compares the byte with `+0xc8` and returns when equal (`0x76a69a`), stores it
(`0x76a6a2`), calls slot 0x20 of each region with 0 (`0x76a6d0`) and slot 0x28, itself in
`0x81c498`, of each child with the same byte (`0x76a702`). `0x774e90` loads the number,
`fcom 0.0; test ah,0x5; jp` replaces a value below 0 with 0.0 (`0x774f32`), `fcom 1.0; test
ah,0x5; jnp` keeps a value below 1 and replaces the rest, NaN included, with 1.0 (`0x774f49`),
then `fmul [0x7ffe58]` (255.0) and `__ftol` (`0x774f5e`, `0x774f66`) before slot 0x28.

Bit-exact coverage: `ui::set_alpha_scaled` covers `0x774f32` to `0x774f66` of `0x774e90`; `ui::set_alpha_value` covers `0x774f32` to `0x774f66` of `0x774e90`.

## Scale

A frame keeps its own scale at `+0xb8` and its parent's effective scale at `+0xbc`, both 1.0 at
construction. Setting either ignores a change below 2⁻²² unless forced, then recomputes the
effective scale, `+0xbc · +0xb8` rounded to a float. When that differs from the layout scale by
2⁻²² or more, or when forced, it becomes the layout scale of the frame, is passed as the layout
scale of each region, and as the parent scale of each child.

Addresses: `0x76ac10`, `0x76ac1a`, `0x76ac3b`, `0x76ac44`, `0x76ac50`, `0x76ac7b`, `0x76ac90`, `0x76ac97`, `0x76aca2`, `0x76aca8`, `0x76acad`, `0x76acd5`, `0x76ad06`, `0x76ad31`, `0x775140`, `0x7751e1`, `0x7751e9`, `0x7751f1`, `0x77520b`
Confidence: verified
Proof: one
Test: `ui::effective_scale`, `ui::glue769_76ac50_setter_delta_abs`, `ui::glue_767_76ac10_rsub_change_detect`

`0x76ac10` compares the new scale with `+0xb8` (`0x76ac1a`), stores it (`0x76ac3b`) and calls slot
0x8c, `0x76ac90`, with the force byte (`0x76ac44`); `0x76ac50` does the same with `+0xbc`.
`0x76ac90` computes `fld [esi+0xbc]; fmul [esi+0xb8]; fstp` (`0x76ac97` to `0x76aca8`), compares
it with the layout scale `+0x7c` (`0x76acad`), calls `0x768360(scale, force)` (`0x76acd5`), slot
0x10 of each region's layout part (`0x76ad06`) and `0x76ac50(scale, force)` for each child
(`0x76ad31`). The Lua `SetScale` (`0x775140`) reads its argument as a double (`0x7751e1`) and
compares it with 0.0 (`0x7751e9`); `0x7751f1` tests C0 alone, so a value below 0 or a NaN raises
an error (`0x77520b`), and 0 is stored through `0x76ac10`.

`ui::effective_scale` runs `0x76ac90` whole, with the layout scale setter `0x768360` stubbed and empty child lists.

## Parent change

Giving a frame a new parent first leaves the old parent's children and, when visible, makes the
frame invisible. The frame then takes the parent's stratum, level plus one and effective scale,
or stratum 3, level 0 and scale 1.0 without a parent, and the parent's draw-itself bit; it joins
the parent's children; and, when it was visible and the parent is visible or absent, it is made
visible again.

Addresses: `0x76ab10`, `0x76aa20`, `0x76aab0`, `0x76a400`
Confidence: verified
Proof: one
Test: none

`0x76ab10` returns when `+0x9c` already holds the parent; it calls `0x76aab0` on the old parent,
slot 0x84 when `+0xd4` was set, stores `+0x9c`, then `0x76a470`, `0x76a4f0(level + 1, 0)` and
`0x76ac50(parent+0x7c, 0)`, or 3, 0 and 1.0; `0x76a400(bit 0xd of the parent's +0xb4, -1)`;
`0x76aa20` on the parent; and slot 0x88 under the stated condition. `0x76aa20` links a 0xc-byte
node tagged `.?AUSIMPLEFRAMENODE@@` holding the child into `+0x2fc`; `0x76aab0` frees the node of
a child. `0x76a400(a, b)` sets or clears bit 0x2000 by `a` and, when `b` is not negative, bit
0x4000 by `b`; when `a` is set it marks the frame's own five layers in `+0x1f8` through `0x76b530`;
then it calls itself with `(a, -1)` on each child.
