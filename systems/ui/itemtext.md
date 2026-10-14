# ui: the item text window

How a readable item or object (a letter, a book, a page of text) is opened, paged and closed, and what the Lua functions
`ItemTextGetItem` to `CloseItemText` read from it. The server messages that start and end a read are
`objectlayer/playermessages` ("Reading items"); the page material is `ui/questmaterial`.

The code is `ItemTextFrame.cpp`, `0x4e3140` to `0x4e3b70`: open or close `0x4e32e0`, show `0x4e35b0`, the page moves
`0x4e3870` and `0x4e38b0`, and the eight Lua functions `0x4e38f0` to `0x4e3b70`.

Roots: `0x4e32e0`, `0x4e35b0`, `0x4e3870`, `0x4e38b0`, `0x4e38f0`, `0x4e3940`

## What is kept

The window keeps the GUID of the open object (`0xbc3fa0`), the page index (0xbc3fc4), the id of a single item text
(0xbc3fbc), a list of page text ids (data pointer 0xbc3fd0, a growable array) and the text of the current page, up to
0x1f40 bytes (0xbc3fe8). An object with a single item text has page id 0 in the list's first entry and the item text id in
the single id. An object with page text has the first page id in the list's first entry and 0 in the single id. The events
are ITEM_TEXT_BEGIN (293), ITEM_TEXT_READY (295) and ITEM_TEXT_CLOSED (296), signalled as 0x125, 0x127 and 0x128.

Addresses: `0xbc3fa0`, `0x4e3372`, `0x4e3379`, `0x4e3423`, `0x4e342f`, `0x4e3431`, `0x4e349f`, `0x4e3677`, `0x4e3328`, `0xbc3fbc`, `0xbc3fc4`, `0xbc3fd0`, `0xbc3fe8`
Confidence: verified
Proof: one
Test: none

`0x4e3372` to `0x4e338f` zero the single id, the page index, the first list entry, the second list entry and the first byte of the text when a new object is adopted. `0x4e342f` stores the virtual-slot id (`0x74` of the object) in
the first list entry and `0x4e3431` the item's own text id in the single id. `0x4e349f` signals BEGIN, `0x4e3677` and the other signal sites READY, `0x4e3328` CLOSED.

## Opening and closing

`0x4e32e0(guid, open)` is the one routine for both. A GUID that is not the open one clears the interaction target and the
page state and adopts the new GUID; a zero GUID then signals ITEM_TEXT_CLOSED. With `open` clear and the GUID the open one it
clears the target and signals CLOSED. Otherwise it resolves the object, takes its page text id from the object's virtual at
`+0x74` and, for an item, its text id from the item field block (`+0x9c`), and returns with no event when both are 0. It
requests the page record or the item text record, with `0x4e3280` as the arrival callback, and when a record is at hand sets
the interaction target and signals BEGIN.

Addresses: `0x4e32e0`, `0x4e3240`, `0x4e3280`, `0x4e32b0`, `0x4e3301`, `0x4e330c`, `0x4e3350`, `0x4e33b0`, `0x4e33ec`, `0x4e3402`, `0x4e347c`, `0x4e349a`, `0x4930d0`, `0x4931c0`, `0x564730`, `0x569980`
Confidence: verified
Proof: one
Test: none

`0x4e3240` calls `0x4e32e0` with a zero GUID and `open` 0, which `CloseItemText` also does (`0x4e3b96`). `0x4e3280` and `0x4e32b0` are the arrival callbacks: with a true arrival flag and the open GUID they reopen with `open` 1.
`0x4e33ec` calls the object's virtual `+0x74`; `0x4e3402` loads `+0x9c` of the item field block. `0x4e3465` asks the page cache (`0xc0e174`) and `0x4e347c` the item text cache (`0xc0e0c0`). `0x4e349a` calls `SetInteractionTarget`.
An item whose field flags (`+0x3c` of the block) lack bit 2 and that has no item text id is first read through its container: `0x4e3547` to `0x4e3565` search the container's slots for the item and `0x4e3586` calls `ReadItemAt`, which sends the read request. Every other object goes straight to the show routine `0x4e35b0` (`0x4e359b`).

## Showing a page

`0x4e35b0(guid, translate)` does nothing unless the GUID is the open one and the object exists. With a single item text
it expands the cached record text for the player, masks profanity and signals READY. With page text it looks up the record
for the current page, stores the record's next page id (`+0x400`) as the following list entry, expands its text into a local buffer and signals READY after
writing the page text. An item whose field flags have bit 2 set shows the expanded text unchanged. Any other item or an object that is a game object has its text rewritten in the object's
language, with the player's skill in that language (from `GetIndexedSpellSkill`) only when `translate` is set.

Addresses: `0x4e35b0`, `0x4e3612`, `0x4e3666`, `0x4e3672`, `0x4e3711`, `0x4e3723`, `0x4e3744`, `0x4e3762`, `0x4e3779`, `0x4e37e9`, `0x4e37fc`, `0x4e3837`, `0x4e3856`, `0x4e385b`, `0x506f70`, `0x4a1a60`, `0x49b560`, `0x5ec720`, `0x5f5930`
Confidence: verified
Proof: one
Test: none

`0x4e3666` expands the record into the text buffer with a 0x1f40 limit and `0x4e3672` masks it. `0x4e3711` reads the next page id at record `+0x400` and `0x4e3723` stores it after the current index. `0x4e3744` expands into a 0x400 byte local buffer. `0x4e3762` tests bit 2 of the item field flags and `0x4e3779` copies the text.
`0x4e37e9` takes an item's language from its template (`+0x1a0`); `0x4e37fc` reads a game object's. `0x4e3856` calls `RewriteInLanguage` with the language and the skill, which `0x4e37b3` and `0x4e3808` start at 0 and `0x4e3837` and `0x4e37e0` fetch only when `translate` is set.

## Moving between pages

`ItemTextNextPage()` and `ItemTextPrevPage()` act only when an object is open and it has page text (no single item text): next
moves when the list has a page id after the current one, previous when the page index is above 0. Each moves the index by
one and shows the page again with `translate` set. `ItemTextGetPage()` returns the page index plus 1. `ItemTextHasNextPage()`
returns 1 when the open object has page text and a next page id, else nil. `ItemTextGetText()` returns the page text buffer.

Addresses: `0x4e3870`, `0x4e387d`, `0x4e388d`, `0x4e3890`, `0x4e38b0`, `0x4e38bd`, `0x4e38d5`, `0x4e38db`, `0x4e3ad0`, `0x4e3b00`, `0x4e3b10`, `0x4e3b50`, `0x4e3b60`
Confidence: verified
Proof: one
Test: none

`0x4e387d` and `0x4e38bd` test the single id, `0x4e388d` the page index and `0x4e38d5` the next list entry (`cmp [list + (page + 1) * 4], 0; jle`). `0x4e3ad0` increments the index before pushing, `0x4e3b19` and `0x4e3b25` test the list, and `0x4e3b50` and `0x4e3b60` call the two routines.

## The source of the text

`ItemTextGetItem()` returns the open object's name (the virtual at `+0x70`), or nil with no open object. `ItemTextGetCreator()`
returns the name of the item's creator from the player name cache, or nil for no open item, a zero creator GUID or an
uncached name. `CloseItemText()` closes through `0x4e32e0` with a zero GUID.

Addresses: `0x4e38f0`, `0x4e3912`, `0x4e391d`, `0x4e3940`, `0x4e3974`, `0x4e3982`, `0x4e3988`, `0x4e39b9`, `0x4e3b70`, `0x55f080`
Confidence: verified
Proof: one
Test: none

`0x4e38f0` resolves the GUID with any object type and `0x4e391d` calls the virtual at `+0x70`; `0x4e3940` needs an item (type mask 2), reads the creator GUID at the field block's `+0x10` and `+0x14` (`0x4e3988` ors them and a zero ends in the nil at `0x4e39d9`) and asks the name cache (`0xc0e228`) through `0x55f080` with `0x4e32b0` as the callback.
