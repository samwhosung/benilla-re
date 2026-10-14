# ui: the nameplate frame

A unit's nameplate is a `Button` frame that the world frame owns: addons reach it through `WorldFrame:GetChildren()`. This doc is the frame, its parts, its pool, its size, its scripts and the rules for its level text, colours and health bar,
and the four Lua functions that switch plates on. Which units get a plate, its projection and its seating are `objectlayer/unitnames`; the hover and click are `mouse` and `hover`.

Roots: `0x7cb250`, `0x7cb6d0`, `0x7cb950`, `0x7cbaa0`, `0x7cbb40`, `0x7cc3d0`, `0x489450`, `0x489460`, `0x489470`, `0x489480`

## A plate is a button

`0x6087f0` allocates a plate (0x518 bytes) and `0x7cb250` constructs it with the world frame as its parent. The base constructors run `SetParent`, which appends the plate to the world frame's child list, gives it the parent's strata (`WORLD`, 0), its
level plus one (1) and scale 1.0. Its type is `Button`: the class overrides six slots of the button vtable (`0x00`, `0x38`, `0x3c`, `0x4c`, `0x50`, `0x94`) and none of the identity slots. It has no name: `GetName()` is nil.

Addresses: `0x6087f0`, `0x6087f8`, `0x60881d`, `0x7cb250`, `0x7cb256`, `0x7cb26d`, `0x7cb29a`, `0x81de50`, `0x607062`, `0x60706a`, `0x607ef9`, `0x6086e0`, `0x76ab10`, `0x76aa20`, `0x7786a0`, `0xb4b2bc`, `0x76c650`, `0x46ff70`, `0x7a1390`
Confidence: verified
Proof: one
Test: none

The parent comes from `0x60f600`'s argument (`0x607062` loads `0xb4b2bc`, the world frame, and `0x607ef9` forwards the same), through `0x6086e0`, `0x6087f0` and `0x7cb256` to the button constructor. Only `0x76c650` stores the name word (`+0x98`)
and its five callers are the XML attribute readers, so `0x46ff70`, the name getter, returns null for a plate and `0x7a1390` pushes nil. The table `0x81de50` differs from the button's at six slots; the constructor stores it at `0x7cb29a`.

## The parts

`GetRegions()` of a plate returns six regions in creation order: the border texture (layer ARTWORK), the glow texture (HIGHLIGHT), the name font string (OVERLAY), the level font string (OVERLAY), the skull texture (OVERLAY) and the raid icon texture (ARTWORK).
`GetChildren()` returns one frame, the health bar, a `StatusBar` whose own regions are its fill texture alone. The bar is at frame level 0, one below the plate, so the border draws over the fill.

Addresses: `0x7cb250`, `0x7cb2c2`, `0x7cb311`, `0x7cb329`, `0x7cb33e`, `0x7cb35a`, `0x7cb438`, `0x7cb4ea`, `0x7cb50b`, `0x7cb5ad`, `0x76fc40`, `0x770d30`, `0x77fd10`, `0x783140`, `0x76a4f0`, `0x7cc340`, `0x782d80`
Confidence: verified
Proof: one
Test: none

The two font strings are built on layer 2 and moved to layer 3 by `0x77fd10(plate, 3, 1)` (`0x7cb438`, `0x7cb4ea`). The bar is a `0x348`-byte `CSimpleStatusBar` subclass built by `0x7cc340`; its type slots equal the stock bar's, so Lua cannot tell the two apart.
`0x7cb33e` sets its level to the plate's less one (absolute 0). The glow is drawn only while the highlight layer is enabled, which the base enter handler does. The fill texture comes from `0x783140` with a null parent and is linked to the bar by `0x77fd10`.

## Pool and lifetime

A plate is never removed from the world frame's child list. `0x608a10` releases a unit's plate by moving it to the tail of the free list, zeroing its shown byte (`+0xd0`) and hiding it (slot `+0x84`); `0x6087f0` hands out the head of the free list without running the constructor, or constructs a new plate when the list is empty.
The free list is first in, first out and has no size limit. Hiding is the only change Lua sees: `OnHide` fires, `IsShown()` is false and the index in `GetChildren()` stays.

Addresses: `0x608a10`, `0x608a5a`, `0x608a7a`, `0x608a80`, `0x6087f0`, `0x6087f5`, `0x608800`, `0x608828`, `0x6086e0`, `0x604ef0`, `0x402016`, `0xc4d918`, `0xc4d920`, `0xc4d91c`
Confidence: verified
Proof: one
Test: none

`0x608a10` never calls `SetParent` (`0x76ab10`) or touches the child list, and `0x6087f0` has no count test. `0x604ef0` frees the free list at engine teardown, its only caller `0x402016`. `0x6086e0` marks a reused plate shown (`[+0xd0] = 1`, slot `+0x88`) after the bind `0x7cb6d0`,
which creates nothing.

## Size and anchors

Every bind stores the size 0.1 wide and 0.025 high in layout units (`0x87d9cc`, `0x87d9d0`). A script reads the size in 768-line pixels: `GetWidth` and `GetHeight` are the stored value times `768 * sqrt(a * a + 1)`, with a the screen aspect from `0x41ad10`.
The width is 128 and the height 32 at 4:3, and 156.6 by 39.2 at 16:9. Each seat clears the plate's points and sets one anchor `TOP` of the plate to `BOTTOMLEFT` of the world frame, so an anchor a script sets on the plate is lost at the next seat.

Addresses: `0x7cb6d0`, `0x7cb737`, `0x7cb744`, `0x7a1e00`, `0x7a2030`, `0x7a1ef4`, `0x41ad10`, `0x41ad70`, `0x41ae40`, `0x509ec0`, `0x50a021`, `0x50a044`, `0x608ce0`, `0x768420`, `0x768410`
Confidence: verified
Proof: one
Test: none

`0x7a1e00` pushes `0x41ad70() * 1024 * stored` divided by `0x832a44` (`0x41ae40`), with `0x41ad70` the aspect times 0.75 and `0x832a44` the aspect over `sqrt(a * a + 1)` (`0x41ad10`); the height has the same conversion. `0x509ec0` ends with `0x767ed0` (clear all points) and `0x767c70` with point 1 (`TOP`), the world frame and point 6.
The bind sets the bar's `BOTTOMLEFT` point, the level's `CENTER` point and the glow's all-points again for each unit. The seat runs again only when a plate is created or one's point moves (`objectlayer/unitnames#seating-the-plates`). The raid icon and skull are anchored to the border and the level text.

## Scripts and mouse

The plate fires every script an addon sets, and the engine sets none. The overrides run first and end in the button's handler: `OnUpdate` (`0x7cb950` to `0x76b2c0`), `OnEnter` (`0x7cb850` to `0x779490`) and `OnLeave` (`0x7cb8b0` to `0x7794e0`), and the click `0x7cb910`, which selects the unit through `0x4949f0` and then runs the button's click. `OnShow`, `OnHide`, `OnMouseDown` and `OnMouseUp` are the inherited ones.

Addresses: `0x7cb950`, `0x7cba17`, `0x7cb850`, `0x7cb8a5`, `0x7cb8b0`, `0x7cb8ff`, `0x7cb910`, `0x7cb92a`, `0x7cb936`, `0x7cb637`, `0x779730`, `0x7792d0`, `0x76af00`, `0x778771`, `0x7cba30`, `0x76b320`, `0x6e48a0`, `0x6e6320`, `0x6e6180`
Confidence: verified
Proof: one
Test: none

The plate registers `LeftButtonUp` and `RightButtonUp` (`0x7cb630` stores 0x500), so `OnClick` fires on the release only (`button#registerforclicks-and-click`). The button base enables the mouse (`0x778771`, kind 2); the plate's hit test `0x7cba30` answers 0 while the pending spell wants a location and not a unit.

## Name, flash and dim

The bind `0x7cb6d0` stores the unit GUID (`+0x4e8`, `+0x4ec`), resets the name colour to white, sets the name text from `0x609210` and binds the bar. `0x7cba70` starts a flash: it sets the timer `+0x514` to 5000 milliseconds and the name colour to red. `OnUpdate` counts the timer down by the elapsed seconds times 1000 and,
when it reaches 0 or less, sets the name to yellow while the glow is shown (hovered) and white otherwise. It also sets the plate's alpha to 0xff when no target is set or the unit is the target, else 0x7f.

Addresses: `0x7cb6d0`, `0x7cb70c`, `0x7cba70`, `0x7cba77`, `0x7cba88`, `0x7cb950`, `0x7cb958`, `0x7cb96b`, `0x7cb97c`, `0x7cb9d7`, `0x7cb9f5`, `0x7cb9f9`, `0x7cba0e`, `0xb4e2d8`, `0x6122e0`, `0x609210`
Confidence: verified
Proof: one
Test: none

A negative timer is idle: the bind stores 0, so the first update after a bind leaves the timer negative and resets the name colour. The step is `ftol(elapsed * 1000.0)`. The unit lookup uses mask 8 and a missing unit gives 0x7f. `0x6122e0` flashes the plate of a unit that is hit (`objectlayer/unitnames`).

## Level text and skull

`0x7cbb40` refreshes the level text and the skull from the displayed level L (`unitlevels#the-level-on-a-plate`) and the active player's level P; with no player it does nothing. A world boss (rank 3) shows the skull and no level. A hostile unit (reaction 1 or less) with P at most L minus 10 shows the skull and no level. Every other unit shows the level and no skull. A shown level is written as decimal text in the difficulty colour of P and L.

Addresses: `0x7cbb40`, `0x7cbb7a`, `0x7cbb84`, `0x7cbbc0`, `0x7cbbcc`, `0x7cbc24`, `0x7cbc27`, `0x7cbc29`, `0x7cbc97`, `0x7cbcba`, `0x7cb82f`, `0x7cbd50`, `0x7cbce0`, `0x5f0700`, `0x605620`, `0x6061e0`, `0x7cbe20`, `0x4bb190`
Confidence: verified
Proof: one
Test: none

Hiding a part is `[part+0xc4] = 0` and `0x77fc60`; showing is `[part+0xc4] = 1` and `0x77fcb0`. The skull rule is the `jg` at `0x7cbc29` on P against L - 10; the trivial-unit test at `0x7cbbcc` ahead of it selects the same outcome, level and no skull. `0x7cbe20(index)` hides the raid icon for an index of 8 or more and otherwise crops the 4 by 2 atlas: column `index & 3`, row `index >> 2`, each cell a quarter wide and a quarter high.
The bind passes it the slot `0x4bb190` finds for the unit's GUID (`0x7cb82f`).

## Bar colour and values

`0x7cbaa0` sets the bar colour: red `0xffff0000` when the unit's reaction toward the player is 1 or less, else blue `0xff0000ff` for a player unit, green `0xff00ff00` when the reaction is 4 or more, else yellow `0xffffff00`. `0x7cc3d0` sets the bar range 0 to `UNIT_FIELD_MAXHEALTH` and the value to `UNIT_FIELD_HEALTH`, so `GetValue()` is the health and not a fraction;
watchers on both fields (`0x7cc510`) keep them current through `0x7cc570`.

Addresses: `0x7cbaa0`, `0x7cbad4`, `0x7cbb02`, `0x7cbb2e`, `0x7cc3d0`, `0x7cc41b`, `0x7cc42c`, `0x7cc442`, `0x7cc510`, `0x7cc570`, `0x7cc5d0`, `0x7cc450`, `0x7cc4e0`, `0x783380`, `0x81df88`, `0x7cb120`, `0x7cb160`, `0x7cb1a0`, `0x7cb1e0`, `0x7cb220`, `0xcf6128`, `0xcf60c8`, `0xcf60d0`, `0xcf60dc`, `0xcf60e8`
Confidence: verified
Proof: one
Test: none

The bar's `SetValue` override (`0x7cc450`) recolours it red at a fill fraction of 0.2 or less and green above (`0x783380`, the threshold at `0x81df88`) while its flag `+0x340` is set; the colour override (`0x7cc4e0`) clears the flag, and the bind sets a colour at once, so a plate keeps its reaction colour.
The four colours are written by the static initialisers `0x7cb120`, `0x7cb160`, `0x7cb1a0` and `0x7cb1e0`.

## The nameplate switches

`ShowNameplates()` and `HideNameplates()` set and clear bit 0x1 of `0xc4da34`; `ShowFriendNameplates()` and `HideFriendNameplates()` bit 0x8. None reads an argument, none returns a value, and there is no getter and no CVar. The plate gate asks bit 0x1 of an attackable unit and bit 0x8 of any other (`0x60f6f3`). Starting the game systems (`0x6033c0`, from `ClientInitializeGame`) clears both bits (`0x60394d`, `0x603954`).

Addresses: `0x489450`, `0x489452`, `0x489460`, `0x489470`, `0x489480`, `0x6054f0`, `0x605510`, `0x605530`, `0x605550`, `0x60f6f3`, `0x6033c0`, `0x60394d`, `0x603954`, `0xc4da34`
Confidence: verified
Proof: one
Test: none

Bits 0x2 and 0x4 are per-frame latches. `0x483611` sets 0x2 for a frame in which a plate was created or moved and `0x483627` and `0x48362e` clear both after the frame; 0x2 makes `0x6086e0` re-sort every plate and 0x4 makes it refresh the level text and the skull through `0x7cbb40` (`0x60875f`, `0x60878e`).
The latch 0x4 is set by a change of the player's level (`0x5edb93` through `0x605550`) and of its drunkenness (`0x5e2b42`).
