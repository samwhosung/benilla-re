# ui: portrait textures

The three Lua bindings that put an item or bag icon into a round portrait, and the engine routine they share, which
loads an image, replaces its alpha with a circular mask and keeps the result in a cache. This is `PortraitButton.cpp`
(`0x525d30`, `0x5258a0`), `ContainerFrame.cpp` and `PaperDollInfoFrame.cpp`. The unit-frame portrait bake `0x524f60`
reads the same mask and is `ui/model`'s.

Roots: `0x4fa4f0`, `0x48d780`, `0x4c9150`

## Bag portrait

`SetBagPortaitTexture(texture, id)` (the client's spelling) is `0x4fa4f0`. Argument 1 must be a texture table, else it raises
the frame script object errors; argument 2 must be a number, else
`Usage: SetBagPortaitTexture(texture, slot)`. The id is truncated and reduced by 1.
A result below 0 (id 0, the backpack, and the keyring) returns without touching the texture; 10 or more raises
`Invalid slot in SetBagPortaitTexture`. Otherwise the texture is cleared, and the container at cache index id - 1
(`0x4f93e0`) is resolved with mask 4. An empty slot, a missing container or a closed bank (ids 5 to 10) leaves the
texture cleared; else the path `<StringLookups row 3>\<InventoryIcon_1>` goes to the sink. Nothing is returned.

Addresses: `0x4fa4f0`, `0x4fa5b9`, `0x4fa5c2`, `0x4fa5c8`, `0x4fa5e9`, `0x4fa631`, `0x4fa641`, `0x4f93e0`, `0x53f810`, `0x5d8af0`, `0x770300`
Confidence: verified
Proof: one
Test: none

`0x4fa5b9 js` takes the silent return, `0x4fa5c2 jge` the raise, and `0x4fa5c8` clears with `SetTexture(0)`. `0x4f93e0`
returns the GUID at `0xbdd060 + 8n`, zero for n outside 0 to 9, and for n of 4 or more while the banker GUID
(`0xbdd038`) is zero. `0x4fa5e9` looks the GUID up with mask 4. `0x53f810(3)` returns the string of row 3 of the
StringLookups store; `0x4fa600` uses a `\` separator when it is not empty, and `0x4fa631` formats `%s%s%s` into
a 260-byte buffer. `0x5d8af0` returns the inventory icon name (`objectlayer/items#inventory-icons`).

## Portrait by name

`SetPortraitToTexture(target, path)` is `0x48d780`. The target is a name (a string or a number, turned into a string and
looked up as a global that must be a texture) or a texture table; the failures raise
`SetPortraitToTexture(): Couldn't find texture named '%s'`, `Couldn't find 'this' in texture object` and
`Wrong object type, expected texture`. A table where a name was wanted reports the name as `(null)`. A second argument
that is not a string or number returns silently; the texture is left alone and no error is raised. Otherwise the
path goes to the sink `0x525d30`.

Addresses: `0x48d780`, `0x48d789`, `0x48d7bf`, `0x48d7f5`, `0x48d87f`, `0x48d886`, `0x48d898`, `0x76c760`
Confidence: verified
Proof: one
Test: none

`0x48d789` is `lua_isstring`, which accepts a number. `0x48d7bf` calls `0x76c760(name, textureToken)`, a global
lookup by name that requires a table, a non-null `this` and the texture class. A table argument 1 takes
`0x48d7f5 lua_type == 5` and the same three checks. `0x48d886 je 0x48d89d` returns without a message when the second
argument is neither string nor number.

## Inventory portrait

`SetInventoryPortaitTexture(texture, unit, slot)` is `0x4c9150`. After the three texture checks it clears the texture,
then needs argument 2 to be a string (`Usage: SetInventoryPortaitTexture(texure, unit, slot)`) and argument 3 to be a
slot the reader accepts (`Invalid inventory slot in SetInventoryPortaitTexture`). It then reads the unit token from
argument 1, the texture table, which is never a string, so the token is null and the function returns with the
texture cleared. The unit and slot code after that, which finds a player, an ammo item or the item in the slot and
builds the same icon path as the bag portrait, cannot be reached with a table as argument 1.

Addresses: `0x4c9150`, `0x4c91ed`, `0x4c9200`, `0x4c9227`, `0x4c924c`, `0x4c9255`, `0x4c9316`, `0x515940`, `0x515970`
Confidence: verified
Proof: one
Test: none

`0x4c9247 mov edx,1` passes index 1 to `lua_tostring` (`0x6f3690`), where the usage string and the checks at
`0x4c91f2` and `0x4c9220` use indices 2 and 3. A table converts to null, and `0x515970` returns a zero GUID for a null
token (`0x515992`), so `0x515940` returns 0 and `0x4c925e` leaves. The stock interface never calls this binding.

## Portrait sink

`0x525d30(texture, name)` sets a texture to the circular portrait of an image. An empty name does nothing. The cache
`0xc0ce30` is looked up by the Storm hash of the name (`0x64b3f0`, which folds case and `/`) with a case-sensitive
compare; a hit sets the cached texture. A miss inserts the entry first, then loads `<name>.blp` (appending `.blp`
whatever the name ends in) and decodes mip 0 as 32-bit. It keeps exactly 64 by 64 texels (4096), copied unchanged,
replaces every texel's alpha byte with the mask byte at the same index, creates a 64 by 64 texture named `Portrait2`,
and sets it. Names that differ only in case get separate entries.

Addresses: `0x525d30`, `0x525d52`, `0x525d8f`, `0x525d9d`, `0x525dc4`, `0x525de8`, `0x525e37`, `0x525e4e`, `0x525e97`, `0x525eb0`, `0x525efb`, `0x525f19`, `0x525f58`, `0x64b3f0`, `0x64a480`, `0x5a7b40`, `0x5a7f00`
Confidence: verified
Proof: one
Test: none

`0x525d43` and `0x525d4c` return for a null or empty name; `0x525d8f` compares the hash and `0x525d9d` calls
`0x64a480` (a byte compare, with no folding) on the key. `0x525de8` inserts through `0x527010`, `0x525e37 0x5a7b40`
opens the file, and `0x525e4e 0x526930(0x1000, 1)` pins the pixel array to 4096 dwords whatever the file holds. The
copy at `0x525eb0` to `0x525edc` is an identity of the four bytes; `0x525efb` fetches the mask for size 0x40 and
`0x525f19 mov [edi+ecx*4+3],dl` overwrites byte 3. `0x525f58` creates the texture with the callback `0x525ae0` that
hands over the array without a copy. The node is 0x2c bytes (`0x5264e0`), zeroed from +4 to +0x28.

## Portrait load failure

When the file does not open (`0x525e3e je 0x525f6b`) the entry is already cached with a texture handle of 0, and the
target is not touched. Every later call with the same name hits that entry and sets texture 0, releasing the target's
texture. So a missing image leaves the portrait as it was the first time and blanks it afterwards.

Addresses: `0x525d30`, `0x525de8`, `0x525e3e`, `0x525f6b`, `0x525dc4`, `0x770300`, `0x5264e0`
Confidence: verified
Proof: one
Test: none

The entry is inserted at `0x525de8` before the open and its handle `+0x18` is written only at `0x525f68`, after a
successful create; `0x526509` to `0x526524` zero the node. `0x770300(0)` cannot take its equal-pointer early out
(`ret 4`), so it always stores null at `[this+0xcc]` and notifies.

## The circular mask

`0x5258a0(size)` returns the alpha template for portraits, one byte per texel, `size` squared of them. Size 0x40 reads
`Interface\CharacterFrame\TempPortraitAlphaMaskSmall.tga`, and when that fails `...Small.blp` (the blue byte of each
decoded texel); size 0x80 reads `TempPortraitAlphaMask` the same way. If both fail it fills 255 bytes with the low byte
of the texel count and caches the result; the template is never retried. Both callers, the 2D sink and the 3D unit
portrait bake `0x524f60`, pass 0x40. A block of DXT colour 0xffff decodes to 248, 252, 248, because the client shifts
the 5 and 6 bit channels left instead of scaling them to 255.

Addresses: `0x5258a0`, `0x5258e6`, `0x525919`, `0x525944`, `0x525a30`, `0x525a72`, `0x52576f`, `0x525efb`, `0x5a633c`, `0x524f60`
Confidence: verified
Proof: one
Test: none

`0x5258e6 sete al` picks the slot for 0x40, `0x525919` stores the count before any file is read, and `0x52594f` and
`0x5259fa` open the `.tga` and the `.blp`. `0x525a30 mov cl,[ecx+eax*4]` takes decoded byte 0. The failure fill is
`0x525a72`, `0x3f` dwords, a word and a byte. The expansion at `0x5a633c` to `0x5a636b` builds `0xfff8fcf8`. In the
shipped interface archive the small mask is a 64 by 64 file whose blue channel is a filled disc with an
antialiased rim, so the interior alpha is 248.
