#!/usr/bin/env python3
"""tables/spell_failed.tsv: the cast failure reasons, one row per reason byte, with the
GlobalStrings key the client shows for it.

0x6e23e0 maps a reason byte r (0 to 0x90) to a string through a jump table of `mov eax,imm32;
ret` pairs; any larger byte returns the lowest string. The strings are 4-byte-padded C strings
laid end to end from 0x86faa0 (`SPELL_FAILED_UNKNOWN`) up to 0x870b64 (`SPELL_FAILED_AFFECTING_COMBAT`,
reason 0): the string at the k-th place from the top is reason k, and the lowest is the key for any
byte above 0x90, 0x91 in this table. The generator reads the run and checks that it holds 146
strings starting at the two ends it names, and that the jump table's immediates agree with the
tiling, and that the jump table at 0x6e2764 sends reason r to that pair: the pair for reason r, at 0x6e23f6 + 6r, holds the address of the string r places below the top.
"""

import sys

from lib import Image, addr, note, parse_args, write_table

LOW, TOP = 0x86FAA0, 0x870B64
JUMPS = 0x6E23F6
TABLE = 0x6E2764
LAST = 0x90


def main():
    args = parse_args(__doc__)
    img = Image(args.exe)
    found, va = [], LOW
    while va <= TOP:
        text = img.cstr(va, 100)
        if text is None:
            sys.exit('no string at 0x%x' % va)
        found.append((va, text))
        va += (len(text) + 1 + 3) // 4 * 4
    if found[-1][0] != TOP or len(found) != LAST + 2:
        sys.exit('the run does not tile 0x%x to 0x%x' % (LOW, TOP))
    rows = []
    for r in range(LAST + 2):
        va, text = found[len(found) - 1 - r]
        rows.append(('0x%02x' % r, text, addr(va)))
    # the jump table: case r is a `mov eax,imm32; ret` at JUMPS + 6r whose immediate is the string
    for r in range(LAST + 1):
        at = JUMPS + 6 * r
        if img.u8(at) != 0xB8 or img.u32(at + 1) != int(rows[r][2], 16):
            sys.exit('reason 0x%x: the switch does not return its string' % r)
    # the dispatch table: slot r holds the address of the pair for reason r
    for r in range(LAST + 1):
        if img.u32(TABLE + 4 * r) != JUMPS + 6 * r:
            sys.exit('reason 0x%x: the jump table does not pick its pair' % r)
    note('%d reasons, the switch agrees with the run' % len(rows))
    write_table(['reason', 'key', 'address'], rows, args.out)


if __name__ == '__main__':
    sys.exit(main())
