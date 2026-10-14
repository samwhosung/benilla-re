# ui: the chat bubble

The speech bubble over a unit that has just spoken: `CGChatBubbleFrame`, `ChatBubbleFrame.cpp`, the
code from `0x4b0940` to `0x4b1870`. A bubble is a 0x358-byte frame whose parent is the world frame,
one text string, a backdrop and a tail. The unit side that spawns and releases one (`0x608ac0`,
`0x608c00`) is in `objectlayer/unitnames`; the chat line that calls it is in `chatevents`. This
file holds the gates, the lifetime, the list the bubbles live on, the text and the placement.

Roots: `0x4b1060`, `0x4b1460`, `0x4b0f80`, `0x4b0f40`, `0x4b1600`, `0x4b1870`, `0x4b1590`

## Spawning a bubble

`0x608ac0(kind, text)` is called once in the image, by the chat line function (`0x49acd9`), when
a line that passes `ui/chatfilter` has a speaker that resolves to a unit of the client. A unit gets a bubble when, in order: it has no
nameplate (`+0xe60` is 0), the text is non-empty, it has a scene node (`+0xe0`), the CVar is on
(`ChatBubblesParty` for kind 1, `ChatBubbles` for every other kind; defaults 0 and 1), the active
player resolves, and the squared distance to the active player is at most 400 or NaN. The speaker may be
the active player. A bubble that is not drawn by the scene does not stop the spawn.

Addresses: `0x608ac0`, `0x608ae5`, `0x608b0d`, `0x608b1d`, `0x608b2a`, `0x608b4b`, `0x608ba0`, `0x608bb4`, `0x608bbf`, `0x608be6`, `0x49acd9`
Confidence: verified
Proof: one
Test: none

A line the spam filter catches returns before the bubble (`0x49ab23` to `0x49afd7`). The distance test is
`0x608ba0 test ah,0x41; je`, which falls through on an unordered compare.

`0x608ac0` makes nine calls: the CVar lookup `0x63de30`, the active player and its lookup, two
position slots, the colour `0x49e990`, the release `0x608c00` and the build `0x4b1870` then
`0x4b1600`. `0x608b0d` compares the kind with 1 and picks the name `ChatBubblesParty` or `ChatBubbles`;
the CVar's value at `+0x28` must be non-zero (`0x608b2d`). `0x608ba0 test ah,0x41; je` bails when the
distance is above 400. `0x608bb4` releases the unit's old bubble first, so a new line replaces it;
the new bubble is stored at `+0xe64` (`0x608bc7`) and built with the unit's GUID (`0x608be6`). The
`0x608c00` release is also called by the bubble walk, the unit teardown (`0x5fb627`, `0x5fbbb0`) and
the plate set-up (`0x608752`).

## Lifetime

A bubble lives its duration and then fades out: the duration counts words, and a line spoken by the
active player lasts less. For a line of text `t`, the duration is 0 for an empty text, else
`base + per·k`, with `k` the number of runs of spaces or tabs that follow a character other than a
space or tab. Someone else's bubble has `base` 2750 and `per` 750; the active player's has 1500
and 500. A leading run adds nothing and a trailing run adds one: `"hi"` lasts 2750, `"hi "` 3500,
`"a b c"` 4250.

Addresses: `0x4b1810`, `0x4b1812`, `0x4b1822`, `0x4b1832`, `0x4b1840`, `0x4b1842`, `0x4b1850`, `0x4b1856`, `0x4b17b8`, `0x4b17cf`, `0x4b17d4`
Confidence: verified
Proof: one
Test: none

`0x4b1810` is `__fastcall (text, isSelf)`. The flag `isSelf` sets `esi` to 750 or, by
`neg; sbb; and 0xffffff06; add 0x2ee`, to 500, and `edx` to `0x7d0` plus `esi` or, by `and
0xfffffc18`, 1000 plus `esi`. The scan keeps a flag that starts set: a separator byte adds `esi`
only when the flag is clear (`0x4b1850`, `0x4b1856`), and every byte sets the flag to whether it was
a separator (`0x4b1859`). `0x4b17ae` compares the active player's GUID with the speaker's. The text
counted is the stripped buffer of `0x4b1600` (`0x4b17c9`), so a line stripped to nothing lasts 0.
`0x4b17d4` stores the result at `+0x340` and `0x4b17a8` the start time at `+0x338`.

## Fades

A bubble fades in over 250 ms and out over 250 ms, at the end of its life and whenever its speaker
leaves range. Both ramps compute the alpha from their own start time `+0x33c`: in `255·t/250`, out
`255 − 255·t/250`, integer division; past 250 ms the fade in sets 255 and the fade out hides the
frame. Neither reads the alpha the frame holds, so a fade-out that interrupts a fade-in starts from
255. The state word `+0x320` holds 1 fading in, 2 fading out, 4 the final fade-out and 8 a new bubble.

Addresses: `0x4b0ea0`, `0x4b0ee0`, `0x4b0c30`, `0x4b0d03`, `0x4b0d21`, `0x4b0d2e`, `0x4b0d3e`, `0x4b0d5b`, `0x4b0d78`, `0x4b0d80`, `0x4b0d88`, `0x4b0da7`
Confidence: verified
Proof: one
Test: none

`0x4b0ea0` returns when `+0x320` has bit 1 or 4 (`0x4b0ea9`); otherwise it clears bit 2, sets bit
1, stamps `+0x33c`, writes `[+0xd0] = 1` and tail-jumps to the frame's show slot `+0x88`.
`0x4b0ee0(final)` returns when bit 4 is set; with a non-zero argument it sets bit 4; unless bit 2 is
set it clears bit 1, sets bit 2 and stamps `+0x33c`. `0x4b0c30` starts the final fade-out when the
frame is not already fading out and `now − +0x338 > +0x340 + 250` (`0x4b0d21`, signed). The ramp
divides by 250 as a multiply by `0x10624dd3` and a shift by 4 (`0x4b0d61`). When a fade-out passes
250 ms the update returns 0 for a final one (`0x4b0d82`), and otherwise clears bits 2 and 4,
writes `[+0xd0] = 0` and calls the hide slot `+0x84` (`0x4b0d9f`).

## The update gates

Every frame `0x4b0c30(player, speaker)` decides a bubble's fate. It destroys the bubble when the
player is null, the speaker has no scene node (`+0xe0`), no model (`+0xd8`) or an unloaded model
data (`0x7103d0` with 0). It then tests whether the scene drew the speaker in the last paint
(`0x6704c0` on the node) and the squared distance to the player. Drawn and strictly under 400, a
bubble whose alpha is not 255 fades in; otherwise one with a non-zero alpha fades out and is hidden,
and returns when the speaker is drawn and near again. A speaker the scene does not draw loses its bubble from view; a
line spoken from exactly 20 yards spawns and then fades.

Addresses: `0x4b0c30`, `0x4b0c40`, `0x4b0c57`, `0x4b0c5f`, `0x4b0c6b`, `0x4b0cab`, `0x4b0cb2`, `0x4b0cca`, `0x4b0cd2`, `0x4b0cd7`, `0x4b0cec`, `0x4b0cf7`, `0x6704c0`, `0x7103d0`, `0x4b0e8e`
Confidence: verified
Proof: one
Test: none

`0x4b0c57`, `0x4b0c5f` and `0x4b0c72` jump to `0x4b0e8e` (return 0), which the walk takes as destroy.
`0x6704c0` is `node != 0 && [node+0x94] == 0`; the scene writes that word 1 for every node each
paint and clears it for the nodes it draws (`terrain/scene`). `0x4b0cca fcomp [0x806798]` with
`test ah,5; jnp` enters the in-range leg on C0 only, strictly below 400 (the spawn admits equal,
`0x608ba0`). The out-of-range leg fades out with `0x4b0ee0(0)` when the alpha byte `+0xc8` is
non-zero; the in-range leg calls `0x4b0ea0` when it is not 255 (`0x4b0cf7`), which a fade
already running refuses. `0x7103d0(0, 0)` returns 0 only when the model's `+0x10` is 0.

## The bubble lists

The live bubbles are on a list kept farthest first and the retired ones on a free list that is
reused first in, first out. The nearest bubble draws on top: the walk gives the frame levels 2,
3, 4 in list order. A new bubble is placed by its distance to the camera and is placed again,
with all the others, whenever the camera moves. Each walk retires a bubble whose speaker does not
resolve and destroys one the update rejects. A frame taken from the free list was hidden, so its
fade-in shows it and runs `OnShow`.

Addresses: `0x4b1060`, `0x4b1360`, `0x4b13a0`, `0x4b1870`, `0x4b187e`, `0x4b1590`, `0x4b1198`, `0x4b11c0`, `0x4b12de`, `0x4b1309`, `0x4b17da`, `0x4b17f7`, `0x4b0f40`, `0x4b0f80`, `0x4b0940`, `0x4b0bb0`, `0xb6eff0`, `0xb6effc`, `0xb6f038`
Confidence: verified
Proof: one
Test: none

`0x4b1060` compares the camera position (`0x4818f0` plus 8) with the cache `0xb6eff0` (valid when `0xb6effc` is set). For each bubble from the head `0xb6f038` it looks the speaker up by the GUID at `+0x348`; a miss
hides the bubble and appends it to the free list (`0x4b1198`), otherwise `0x4b0c30` runs and a 0
return calls `0x608c00` (`0x4b11c0`). A bubble with bit 8 set, or every bubble when the camera
moved, is taken out and put back by `0x4b1360` after `+0x350` is set to its squared camera
distance (`0x4b12c0`): the bubble goes before the first live one whose `+0x350` is strictly smaller
(`0x4b13a3`). `0x4b12de` starts the level at 2. `0x4b1590`, the release, hides the frame and appends
it to the free list; `0x4b1870` takes the free list's head without running the constructor
`0x4b0940` (`0x4b187e`) and builds only for an empty list. `0x4b17da` sets `+0x320` to 8, `0x4b17f7` calls the fade-in, which shows the frame through slot `+0x88` (`0x76ae10`); that runs the
`OnShow` notify (`0x76aef5`) only for a frame that was not visible. The destructor `0x4b0bb0` unlinks a bubble and frees it on flag bit 0. `0x4b0f80` retires the live bubbles at shutdown (`0x490c3f`) and `0x4b0f40` frees the free list (`0x491218`).

## Bubble text

The bubble shows the line without its escapes, wrapped at 0.2 and no narrower than twice the border
unit. The text goes through the grammar pass `0x7716f0` and then a copy into a 0x800-byte buffer that
keeps `||`, drops every other `|` with everything up to the next `[`, copies from a `[` to the next `]`
without the brackets, and skips a `|h` and a `|r` after the `]`. `|cff1eff00|Hitem:2589:0:0:0|h[Linen
Cloth]|h|r x5` shows as `Linen Cloth x5`; `a|b` shows as `a`; a `|` with no `[` after it ends the text.

Addresses: `0x4b1600`, `0x4b1611`, `0x4b162b`, `0x4b1633`, `0x4b1645`, `0x4b1654`, `0x4b165a`, `0x4b166c`, `0x4b1675`, `0x4b1689`, `0x4b16ac`, `0x4b16c0`, `0x4b16e7`, `0x4b172c`, `0x4b175d`, `0x4b1799`
Confidence: verified
Proof: one
Test: none

The copy stops at `ebp-1` (`0x4b162b`, 0x7ff bytes). A `||` writes both bytes (`0x4b1639`). Any other `|` runs `SStrChr` for `[` (`0x4b1649`) and a miss ends the text
(`0x4b1652`). Between a `[` and a `]` every byte but `[` is copied, `|` included, with no bound
(`0x4b165a` to `0x4b1664`). After the `]` the code skips `|h` (`0x4b1675`) and then `|r` (`0x4b1689`),
each optional. The buffer is set with markup on (`0x4b16c0`, `SetText(buf, 1)`) after the colour of
the chat type (`0x4b16ac`). The width and height are reset to 0 (`0x4b16d3`, `0x4b16e4`); the measured
width is capped at `0x3e4ccccd` (0.2, `0x4b175d`), floored at twice the border unit, and set. For
the locale index 1 (`koKR`, `0x4b16e7`) the string's justify is changed to `LEFT` first.

## Bubble frame

The bubble is a backdrop around a font string with a tail below it. The fill is
`Interface\Tooltips\ChatBubble-Background`, tiled, the edge `ChatBubble-Backdrop` with all eight
pieces on, and the tail a square `ChatBubble-Tail`. A border unit `u = K1·16 / (S·1024)`, 0.0125 at 4:3 (`screencoord`),
gives the insets, the edge size, the tile size and the tail's sides. The text uses the font of
the global string `NAMEPLATE_FONT`, height 0.01. The frame hugs the text with 0.01 to spare on
each side; the tail's top right sits on the frame's bottom, raised `u/4`.

Addresses: `0x4b0940`, `0x4b09bf`, `0x4b09c6`, `0x4b09ec`, `0x4b0a12`, `0x4b0a2f`, `0x4b0a35`, `0x4b0a9d`, `0x4b0ac2`, `0x4b0af1`, `0x4b0b30`, `0x4b0b42`, `0x4b0b6b`, `0x4b0b94`
Confidence: verified
Proof: one
Test: none

`0x4b0940` stores the vtable `0x806708` and the layout table `0x8066dc` at `+0x24`, makes a backdrop
(0x68 bytes, `0x77e5f0`) with the two paths (`0x41e3a0`), sets its edge mask `+0x3c` to 0xff and
tile flag `+0x40` to 1, and computes `u` as `0x41ae60(16.0 / (0x41ad70() · 1024.0))` four times for
the insets (`+0x50` to `+0x5c`), the edge size `+0x48` and the tile size `+0x4c`. `0x76a5d0` installs
the backdrop. The tail texture is made at `0x4b0a47` and sized `u` by `u`; `0x767c70(2, frame+0x24,
7, 0, y, 1)` seats it with `y = 0x41ae60(4.0 / (S·1024))`. The font string is made by `0x770d30` and kept at
`+0x31c`; `0x7714f0(NAMEPLATE_FONT, 0.01, 0, 0)` sets its font. The two frame anchors are
`SetPoint(0, text, 0, −0.01, 0.01)` (`0x4b0b6b`) and `SetPoint(8, text, 8, 0.01, −0.01)` (`0x4b0b94`);
the frame has no anchor on the world.

## The bubble's world point

A bubble follows its speaker's position at a height fixed when the line was spoken: the Stand
animation's box height times the unit's scale, plus 0.7. Each frame `0x4b0c30` stores
`(x, y, z + h + 0.7)` of the speaker at `+0x32c`; `h` is `+0x354`, set once from the speaker's model
and not updated, so the bubble does not rise and fall with the animation. The overhead name's height is a
different quantity, read from a live attachment (`objectlayer/unitnames`).

Addresses: `0x4b0c30`, `0x4b0dc6`, `0x4b0dd1`, `0x4b0ddc`, `0x4b0de4`, `0x4b0df3`, `0x4b0e38`, `0x4b0e3d`, `0x4b0e49`, `0x4b0e57`, `0x4b0e6f`, `0x4b0e7b`, `0x4b1799`, `0x711a20`
Confidence: verified
Proof: one
Test: none

The latch guard is `|+0x354| < 2^-22` (`0x8029d4`, `0x4b0dd1`, `test ah,5; jp` skips the query for any
other value). The query runs only if the speaker has a model (`0x4b0de4`) whose data is resident
(`0x4b0df3`); it calls `0x711a20(model, 0, 0, out)`, which for animation 0 copies the file's
sequence box of the Stand animation (record `+0x24` to `+0x3c`, stride 0x44) to `out+0x0c`; the term
is `out.max.z − out.min.z` times the speaker's scale `[+0x90]` (`0x4b0e3d` to `0x4b0e49`). The only
clear of `+0x354` is `0x4b1799` in the build `0x4b1600`; the constructor does not touch it and the
allocator does not zero a reused frame, so each line queries again. The z sum is formed at 53 bits
(`fld`, `fadd`, `fadd`, one `fstp` at `0x4b0e7b`). The census of `+0x354` finds four operands in the file: the
reads `0x4b0dc6` and `0x4b0e5d` and the stores `0x4b0e49` and `0x4b1799`. A box under 2^-22 high
is queried again every frame.

## Placing the bubble

Each paint projects every bubble's world point, snaps it up to whole device pixels and, when the
snapped point changed, seats the text with `SetPoint(BOTTOM, world frame, BOTTOMLEFT, x, y)`, so the
tail hangs below the point. A point the projector rejects as nearer than the near plane stays (0, 0)
and the bubble sits in the screen's bottom left corner until it is hidden; a point past the screen
edge is placed as computed.

Addresses: `0x4b1460`, `0x4b14a0`, `0x4b14e4`, `0x4b14f2`, `0x4b14fb`, `0x4b1507`, `0x4b1525`, `0x4b153d`, `0x4b157a`, `0x483ee0`, `0x7666a0`, `0x766750`, `0x767c70`, `0x768ed0`, `0x48377b`
Confidence: verified
Proof: one
Test: none

`0x4b1460`, called by the frame driver `0x483460` at `0x48377b`, runs `0x4b14a0` over the live list
and tail-jumps to `0x768ed0`, the layout pass. `0x4b14a0` zeroes its point (`0x4b14e4`), projects
`+0x32c` with `0x483ee0` and drops the verdict, then snaps x with `0x7666a0` and y with `0x766750`.
Each snap returns a value under 2^-22 unchanged and else computes `p = (v / K) · pixels` with the device
width (`rect[3] − rect[1]`) or height (`rect[2] − rect[0]`), adds `0.9999899864196777` when `p` is above 0,
truncates (`0x40a2b0`) and returns `K · n / pixels`, so positive values round up. `0x4b1525` and
`0x4b153d` call `SetPoint` only when `|Δx| ≥ 2^-22` or `|Δy| ≥ 2^-22` against the cached point
`+0x324`, `+0x328`. The call pushes 1, y, x, 6, the world frame region `+0x24`, 7 on the text's layout part
(`+0x31c` plus `0x24`). No term depends on time except the fades.
