# objectlayer: fields

The update fields: how many each type has, how they split into per-class sections, and the
watchers through which the rest of the client learns that a field changed.

An object's fields are one array of dwords (`classes`); the update packet writes them in its
create passes and runs the watchers in its notify pass (`update`). A watcher, a `CMirrorHandler`
record, covers a byte range of one class section and calls a function when the bytes there differ
from a copy taken before the write. Watchers live on 10,256 per-type lists at `0xb23310`, one per
section and field, and on per-object lists inside each object. The copies sit in a second array
after the fields, which holds only the fields a type watches; nine small object files, one per
type and one for the tables, give each type's count of them and the field indices they cover.

Roots: `0x467e70`, `0x467fb0`, `0x468070`, `0x468240`, `0x47f840`, `0x4656e0`, `0x465690`

## Field counts and sections

A type's fields are its sections in hierarchy order: object fields 0 to 5; then item 6 to 47 and
container 48 to 121; or unit 6 to 187 and player 188 to 1281; or game object 6 to 25, dynamic
object 6 to 15, corpse 6 to 37. Each section's end is a cumulative count in `0xb4143c`, and a
section's byte offset in the array is 0, 0x18 for every class section after the object's, 0xc0 for
the container's and 0x2f0 for the player's. Another player has only the first 486 fields.

Addresses: `0x465a80`, `0x4656e0`, `0x465748`, `0x465760`, `0x465690`, `0x4656b4`, `0xb4143c`, `0x464260`
Confidence: verified
Proof: one
Test: `objectlayer::updateblock_stride_difftest`, `objectlayer::descriptor_section_difftest`, `objectlayer::updatefield_count_difftest`

`0x4656e0(object, section)` maps the type mask minus 3 through the byte table `0x465760` to the
jump table `0x465748`: masks 3 and 7 go 0 → 1 → 2; 9 and 0x19 go 0 → 3 → 4; 0x21 gives 0 → 5,
0x41 0 → 6, 0x81 0 → 7; anything else returns 8. `0x464260` stores the ends 6, 48, 122, 188, 1282,
26, 16, 38. `0x465690(section)` returns 0, 0x18, 0xc0, 0x18, 0x2f0, 0x18, 0x18, 0x18 through
`0x4656b4`. `0x465a80` gives the counts (`update`).

## Watched fields

Each type watches a fixed subset of its fields and keeps 4 bytes per watched field after its field
array: object 3, item 35, container 107, unit 170, the active player 1108 and another player 465,
game object 8, dynamic object 3, corpse 6. A table per section lists the watched field indices;
a field's slot in the copy is its position in that table plus the counts of the sections before
it, times 4. The player's table has a part only the active player watches.

Addresses: `0x47f2b0`, `0x47f2c0`, `0x47e5d0`, `0x47e5e0`, `0x47e090`, `0x47e0a0`, `0x47db10`, `0x47db20`, `0x47d540`, `0x47d550`, `0x47d560`, `0x47d000`, `0x47d010`, `0x47c9e0`, `0x47c9f0`, `0x47c4a0`, `0x47c4b0`, `0x466830`, `0x838f50`, `0x838f6c`, `0x838f90`, `0x839430`
Confidence: verified
Proof: one
Test: `objectlayer::player_dynfield_base_difftest`, `objectlayer::field_dynamic_offset_difftest`

`0x47f2b0` returns 3; `0x47e5d0` adds 0x20, `0x47e090` 0x48 to that, `0x47db10` adds 0xa7 to 3,
`0x47d540` 0x3aa and `0x47d550` 0x127 to the unit's, `0x47d000` adds 5, `0x47c9e0` jumps to
`0x47f2b0`, `0x47c4a0` adds 3. Each scan function returns the index of `ecx` in its table, or a
default past it: `0x47f2c0` over {3, 4, 5} (default 6), `0x47c4b0` over `0x838f50` {29, 30, 32},
`0x47d010` over `0x838f6c` {3, 8, 17, 18, 20}, `0x47d560` over `0x838f90` (295 entries) then, for
the active player, `0x839430` (643). `0x47c9f0` returns 10 for any field. `0x466830(type, offset)`
adds the scan's result to the earlier sections' counts and returns `4·index + (offset & 3)`.

## Watcher records

A watcher is a 0x30-byte `CMirrorHandler`: the byte offset of its range in the field array
(`+0x1c`), its offset in the copy (`+0x20`), its byte length (`+0x24`), the callback (`+0x10`)
and its argument (`+0x14`), a kind (`+0x28`), a busy flag (`+0x2c`) and a delete-pending flag
(`+0x2d`). Removing a watcher whose callback is running only marks it; the notify pass frees it
when the callback returns.

Addresses: `0x467f00`, `0x467f17`, `0x467f7c`, `0x467f9c`, `0x467ff0`, `0x468019`, `0x468023`, `0x468057`, `0x465570`, `0x4655f9`, `0x46566a`
Confidence: verified
Proof: one
Test: none

`0x467f00(list, offset, copyOffset, length, callback, argument, kind)` allocates 0x30 bytes tagged
`.?AUCMirrorHandler@@` (`0x467f17`), links the record at the list's end and stores the offset at
`+0x1c`, then `+0x20`, `+0x24`, `+0x10`, `+0x14`, `+0x28` (`0x467f7c` on), and zeros `+0x2c` and
`+0x2d` (`0x467f9c`). `0x467ff0(list, callback, argument)` finds records matching `+0x10` and
`+0x14` (`0x468019`); a busy one (`0x468023`) gets `+0x2d = 1` (`0x468057`), any other is unlinked
and freed. `0x465570` checks `+0x2d` after the callback (`0x4655f9`) and frees there (`0x46566a`).

## Registering a watcher

Other systems register watchers on a field of a type, or on a field of one object. A type watcher
takes the section, the range's byte offset in it and its length, the callback, its argument and
a kind, and goes on the global list of that section and field. An object watcher also takes the GUID, finds the live
object, and goes on that object's own list for the field, held inside the object. Each has a
matching removal. Item and corpse code are among the callers of the type registration.

Addresses: `0x468070`, `0x4680c0`, `0x468098`, `0x4680aa`, `0x468240`, `0x467e70`, `0x467e86`, `0x467ebe`, `0x467edd`, `0x467fb0`, `0x465850`, `0x46594c`
Confidence: verified
Proof: one
Test: `objectlayer::updatefield_addr_difftest`

`0x468070(section, offset, length, callback, argument, kind)` gets the copy offset from `0x4680c0` and
calls `0x467f00` on `0xb23310 + 12·(1282·section + offset/4)` (`0x468098` to `0x4680aa`);
`0x468240` removes from the same list. `0x467e70(section, offset, lo, hi, length, callback, argument, kind)` looks the GUID up
(`0x467e86`), computes the copy offset with `0x466830` for the object's type (`0x467ebe`) and
links the record on `0x465850(object, section, field)` (`0x467edd`), which returns a 12-byte list
inside the object chosen by the section through `0x46594c`. `0x467fb0` removes. The callers: 45
sites of `0x467e70` and 49 of `0x468070`.

## When a watcher runs

During a values block's create pass, before writing, the client copies the current bytes of every
watcher covering a sent field into the copy array. During the notify pass it walks the fields
again; for each sent field (every field, when the create notify asks for it, `update`) each watcher covering it whose
watched bytes now differ from the copy is called once, with `ecx` the range's byte offset in its
section, `edx` its length, and on the stack the GUID low and high dwords, a pointer to the old bytes
in the copy, and the registered argument. Unchanged ranges call nothing.

Addresses: `0x466590`, `0x466679`, `0x466686`, `0x4667a0`, `0x466816`, `0x465330`, `0x465429`, `0x46544a`, `0x465463`, `0x4654a4`, `0x465970`, `0x465a61`, `0x4657e0`, `0x465570`, `0x4655bb`, `0x4655c7`, `0x4655dc`
Confidence: verified
Proof: one
Test: `objectlayer::apply_update_fields_difftest`

`0x466679` splices a field's lists into a pending list with `0x465970`, which sets each record's
countdown `+0x18` to the dwords its range spans, `(length + (offset & 3) + 3) >> 2` (`0x465a61`);
`0x466686` calls `0x4667a0`, `rep movs` of `+0x24` bytes from `[+0x8] + [+0x1c]` to
`[+0xc] + [+0x20]` (`0x466816`). The notify pass steps the countdowns with `0x4657e0` (`0x465429`),
splices (`0x46544a`, `0x465463`) and calls `0x465570` (`0x4654a4`). `0x465570` sets the countdown
to 1, so a record leaves at the next step; `repz cmps` (`0x4655bb`) compares, and on a difference
pushes the argument and the copy pointer (`0x4655c7`), then the GUID, and calls `+0x10`
(`0x4655dc`).

## Watcher tables

`0x47f840` builds the runtime tables the copy offsets come from, copying each type's static
field-index blocks with `0x47f960`, and runs once at start-up from the manager's shared setup.
Several functions of the tables' file have no caller.

Addresses: `0x47f840`, `0x47f960`, `0x467d40`, `0x47f9c0`, `0x47fae0`, `0x47fc10`, `0x47fc40`, `0x47fc60`, `0x47fd40`
Confidence: inferred
Proof: none
Test: none

`0x467d40` calls `0x47f840`; `0x47f960` copies records of five dwords. The table layout is not
read here; the unreached functions are in `overview`.
