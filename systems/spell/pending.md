# spell: pending

The pending cast record field by field, and the target mask bits that fill it.

The 256-byte pending cast `0xceac48` (`cast`, The pending cast record) holds what the next cast request
sends: the cast's source, caster and spell, then the targets the binders accumulate while the player
chooses them. This topic gives each field with its writers, found by a scan of the listing for every
reference to the record, and the bit of the target mask each binder sets. The request clears the record
and fills the first three fields (`cast`); the binders (`targeting`) and the arm fill the rest; the send
writes it to the wire (`cast`, Cast request packets).

Roots: `0x6e54f0`

## Pending cast fields

Each field below is written where the table says, besides three zeroings of the whole record: the system start
`0x6e11e0`, the request's `rep stos` (`0x6e4f80`) and the re-arm's (`0x6e746a`). The start also stores -1 at
`+0x4c`. The bytes `+0x28`, `+0x2c`, `+0x48` to `+0x6c` and `+0xf0` to `+0xf8` are written only by those
zeroings and that store, and nothing reads them.

| offset | size | field | written by |
|---|---|---|---|
| `+0x00` | 8 | source: the casting item's guid, else the caster's | request `0x6e4fa1`, re-arm `0x6e747d` |
| `+0x08` | 8 | the caster's guid | request `0x6e4fb1`, re-arm `0x6e748f` |
| `+0x10` | 4 | spell id | request `0x6e4f88`, re-arm `0x6e7472` |
| `+0x14` | 2 | target mask | the binders (next entry), `0x6e5b51` reads it for the rollback |
| `+0x18` | 8 | unit, game object or corpse target | the unit and object binders, `0x6e545b` for an unknown object |
| `+0x20` | 8 | item target, or the trade slot 6 | `0x6e5f3e`, `0x6e6925` |
| `+0x30` | 12 | source point | `0x6e6107` |
| `+0x3c` | 12 | destination point | `0x6e6132` |
| `+0x70` | 128 | text target | `0x6e5311` |

Addresses: `0xceac48`, `0x6e11e0`, `0x6e121e`, `0x6e127f`, `0x6e4f80`, `0x6e4fa1`, `0x6e4fb1`, `0x6e4f88`, `0x6e747d`, `0x6e748f`, `0x6e7472`, `0x6e746a`, `0x6e5b51`, `0x6e545b`, `0x6e5f3e`, `0x6e6925`, `0x6e6107`, `0x6e6132`, `0x6e5311`
Confidence: verified
Proof: one
Test: none

The scan finds, by offset, the references to `0xceac48` to `0xceacb8` in the listing. `+0x00` and `+0x04`
are stored by the start (`0x6e121e`, `0x6e1223`), the request (`0x6e4fa1`, `0x6e4fa6`) and the re-arm
(`0x6e747d`, `0x6e7482`). `+0x08` and `+0x0c` by the request (`0x6e4fb1`, `0x6e4fbf`) and the re-arm
(`0x6e748f`, `0x6e7494`). `+0x20` and `+0x24` have three stores each: the start's, `0x6e5f3e` and `0x6e6925`
(`0x6e5f43`, `0x6e692f`). `+0x30` to `+0x44` are written in pairs by the start and the location binder
(`0x6e6107` to `0x6e614b`). The text offset `+0x70` has one reference, the print at `0x6e5311`.
`+0x28`, `+0x2c` (`0x6e1270`, `0x6e1275`), `+0x48` and `+0x4c` (`0x6e127a`, `0x6e127f`), `+0x50`, `+0x54`
(`0x6e122d`, `0x6e1232`), `+0x58` to `+0x64` (`0x6e1260`, `0x6e125b`, `0x6e1251`, `0x6e1256`), `+0x68`,
`+0x6c` (`0x6e128e`, `0x6e1293`) and `+0xf0` to `+0xf8` (`0x6e1265`, `0x6e126a`, `0x6e1289`) have the start as
their only reference besides the two `rep stos` (`0x6e4f80`, `0x6e746a`). The text field's bytes are copied by `SStrPrintf` and sent as 128 bytes.

## Target mask bits

The mask at `+0x14` is a 16-bit word. A binder sets its bit when it binds, and the send writes the guids and
points the bits name. Nine bits are ever set: 0x2 unit, 0x10 item, 0x20 source point, 0x40 destination
point, 0x200 corpse of an attackable relation, 0x800 game object, 0x1000 trade item, 0x2000 text and 0x8000
corpse of an assistable relation. Bits 0x1, 0x4, 0x8, 0x80, 0x100, 0x400 and 0x4000 are never set. The
binder's rollback restores the whole word. The send never reads the mask itself; the target block writer
tests it (`cast`, Cast request packets).

Addresses: `0x6e5cd0`, `0x6e5d14`, `0x6e5d5a`, `0x6e5dad`, `0x6e5e13`, `0x6e5e92`, `0x6e5efa`, `0x6e5f2e`, `0x6e6119`, `0x6e6144`, `0x6e5325`, `0x6e5f69`, `0x6e5fc7`, `0x6e600c`, `0x6e6915`, `0x6e1237`, `0x6e609d`, `0x6e602c`, `0x6e54f0`, `0x7e4e10`, `0x6067d0`
Confidence: verified
Proof: one
Test: none

The scan finds the word at `0xceac5c` stored whole at `0x6e1237` (zero) and `0x6e609d` (the restore), read at
`0x6e5b51` (the save) and `0x6e602c` (`test dword,0x8202`), and or-ed by byte or word at: bit 0x2
`0x6e5cd0`, `0x6e5d14`, `0x6e5d5a`, `0x6e5dad`, `0x6e5e13`, `0x6e5e92`, `0x6e5efa`; 0x10 `0x6e5f2e`; 0x20
`0x6e6119`; 0x40 `0x6e6144`. The high byte `0xceac5d` is or-ed at `0x6e5325` (0x2000), `0x6e5f69` (0x800),
`0x6e5fc7` (0x8000), `0x6e600c` (0x200) and `0x6e6915` (0x1000). The corpse split is `0x6067d0`: nonzero
selects 0x8000 (`0x6e5fb5`), zero 0x200 (`0x6e5ffb`). The byte patterns `5e ac ce 00` and `5f ac ce 00` occur
nowhere, with `5c`/`5d` as the control. `0x6e54f0` has no reference to the word.
