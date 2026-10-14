# ui: add-on housekeeping

Four small routines of `AddOns.cpp` that sit outside the loading of add-ons (`ui/addons`): two keep the interface folders and the per-character add-on state tidy, one writes the version-check switch, and one reads the interface number of the stock FrameXML. The first three run from the glue code; the last has no caller.

Roots: `0x51fb50`, `0x51f0b0`, `0x51f1a0`

## Loose interface folders

`0x51fb50` runs when the glue resumes (`CGlueMgr::Resume`). For each of Interface\GlueXML and Interface\FrameXML that exists as a folder it formats `%s.old`, calls `0x42aa80` on that name, moves the folder to it with `OsMoveFile` and notes that something moved. It then walks the Interface\AddOns\Blizzard* folders with the callback `0x51fbf0`, which moves a folder aside to `%s.old` when it holds a `.toc` of its own name. When anything moved it calls `0x646fc0`.

Addresses: `0x51fb50`, `0x51fbf0`, `0x42a820`, `0x42aa80`, `0x646fc0`
Confidence: verified
Proof: one
Test: none

The callback receives the folder name at `[ecx+8]` and a pointer to the note byte in `edx`, builds `Interface\AddOns\<name>\<name>.toc`, tests it with `OsFileExists`, formats `Interface\AddOns\<name>` and its `.old` twin, moves the first to the second and stores 1 in the note byte. It returns 0 so the walk continues.

## Character add-on state

`0x51f0b0(name)` drops the nodes of the add-on state list at 0xbe1bd8. A node holds a character name at +8 and the enable table and lists of that character; with a name it removes the first node whose name matches without regard to case, with a null name every node. Removing frees the name, empties the node's tables and lists and frees the node. It is called by `LoadCharSelectRoster` and `ApplyCharacterRename`.

Addresses: `0x51f0b0`, `0x4724d0`, `0x4725f0`
Confidence: verified
Proof: one
Test: none

`0x51f0e9` to `0x51f0fa` compare the names with `SStrCmpI`; a match, or a null name, reaches the frees at `0x51f0fe`. After a named match `0x51f15f` leaves the loop (`test edi,edi; jne`); a null name continues with the next node saved at `[ebp-4]`.

## Version check switch

`0x51f1a0(flag)` stores the text 1 or 0 in the CVar checkAddonVersion through `CVar::Set`, 1 for a nonzero flag. `Script::SetAddonVersionCheck` calls it (`glue/script`, Add-on list).

Addresses: `0x51f1a0`, `0x63df50`, `0x46d9e0`
Confidence: verified
Proof: one
Test: none

`0x51f1a6` tests `cl`; the two branches push the strings `1` and `0` before the call. The CVar record is the dword at 0x882630, the one `tables/cvars.tsv` lists for checkAddonVersion, whose default is 1.

## Stock interface number

`0x51d7e0` returns the interface number of the stock FrameXML, reading it with `0x51d800` the first time. `0x51d800` loads Interface\FrameXML\FrameXML.toc, skips a UTF-8 mark, tokenizes it by lines and, at the first `## Interface:` line, stores the number after it in the dword at 0xbe1b48. Nothing calls `0x51d7e0` and nothing else reads that dword, so both are unreached.

Addresses: `0x51d7e0`, `0x51d800`, `0x648620`, `0x64ae50`
Confidence: verified
Proof: one
Test: none

The census finds no `call`, `jmp` or stored address that names `0x51d7e0`, and `0x51d800` is called only by it. A positive control: the same scan names `0x51f0b0` from its two callers.
