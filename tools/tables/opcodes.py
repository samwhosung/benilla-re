#!/usr/bin/env python3
"""tables/opcodes.tsv: every network opcode the client registers a handler for.

The client registers inbound-message handlers two ways, and both are scanned.

Path A, the wrapper 0x5ab650 (fastcall: ecx = opcode, edx = handler, [ebp+8] = context). Each
site loads the two arguments as adjacent immediates,

    mov edx, <handler>    ; ba imm32
    mov ecx, <opcode>     ; b9 imm32
    [0..n global stores the scheduler hoisted in here]
    call 0x5ab650

The scan anchors on the adjacent ba/b9 pair, five bytes apart: an unanchored back-scan for the
first 0xba byte mistakes the immediate of `mov ecx, 0x1ba` for an opcode.

Path B, a direct thiscall into NetClient::SetMessageHandler 0x537a60, used by the modules that
already hold the connection:

    push <ctx>; push <handler imm32>; push <opcode imm8|imm32>; mov ecx, <this>; call 0x537a60

0x5ab660 is the wrapper's own call into 0x537a60, not a registration. Six path A sites carry
a global store between the pair and the call; they are listed below by the address of each mov,
and the scan checks that it finds the same pair.

The name column is the opcode's name in vmangos's `Opcodes_1_12_1.h`, from the committed input
tools/tables/opcode_names.tsv (opcode, name, the line in that file). The client holds no names. An
opcode vmangos does not list has an empty name.

Both handler tables hold 828 entries (0x537600 zero-fills 0x33c dwords at each base), so 0x33b is
the last expressible opcode.
"""

import sys

from lib import Image, addr, note, opcode_names, parse_args, write_table

REGISTRAR = 0x5AB650
INNER = 0x537A60
WRAPPER_INNER_CALL = 0x5AB660
BACK = 48

# call site -> (mov edx, mov ecx), the six sites with a store between the pair and the call
HAND_RESOLVED = {
    0x49861A: (0x498604, 0x498609),
    0x4BC6B6: (0x4BC694, 0x4BC699),
    0x4C9EF6: (0x4C9EE2, 0x4C9EE7),
    0x4DE3EF: (0x4DE3E0, 0x4DE3E5),
    0x5D47A6: (0x5D4784, 0x5D4789),
    0x6CA347: (0x6CA338, 0x6CA33D),
}


def main():
    args = parse_args(__doc__)
    img = Image(args.exe)
    lo, hi = img.text
    u8, u32 = img.u8, img.u32

    rows, rejects = [], []
    adjacent = hand = 0
    for site in img.call_sites(REGISTRAR):
        pair = None
        for back in range(10, BACK + 1):
            j = site - back
            if u8(j) == 0xBA and u8(j + 5) == 0xB9:
                pair = (j, j + 5)
                break
            if u8(j) == 0xB9 and u8(j + 5) == 0xBA:
                pair = (j + 5, j)
                break
        if pair is None:
            rejects.append(site)
            continue
        if site - max(pair) == 5:
            adjacent += 1
        elif HAND_RESOLVED.get(site) == pair:
            hand += 1
        else:
            rejects.append(site)
            continue
        rows.append((u32(pair[1] + 1), u32(pair[0] + 1), site, 'A'))

    direct = [s for s in img.call_sites(INNER) if s != WRAPPER_INNER_CALL]
    for site in direct:
        # The ecx load sits after the pushes (mov ecx,reg 2 bytes; mov ecx,imm32 5;
        # mov ecx,[reg+disp32] 6) or before all three of them (0 bytes).
        for movlen in (2, 5, 6, 0):
            k = site - movlen
            if u8(k - 2) == 0x6A:
                op, head = u8(k - 1), k - 2
            elif u8(k - 5) == 0x68:
                op, head = u32(k - 4), k - 5
            else:
                continue
            if u8(head - 5) == 0x68:
                handler = u32(head - 4)
                if lo <= handler < hi:
                    rows.append((op, handler, site, 'B'))
                    break
        else:
            rejects.append(site)

    rows.sort()
    ops = [r[0] for r in rows]
    for s in rejects:
        note('unresolved site 0x%x' % s)
    note('%d rows: %d adjacent, %d store-between, %d direct; %d distinct opcodes, '
         '%d distinct handlers; %d unresolved'
         % (len(rows), adjacent, hand, len(direct), len(set(ops)),
            len({r[1] for r in rows}), len(rejects)))
    names = opcode_names()
    write_table(['opcode', 'name', 'handler', 'site', 'path'],
                [('0x%03x' % op, names.get('0x%03x' % op, ''), addr(h), addr(s), p)
                 for op, h, s, p in rows], args.out)
    return 1 if rejects else 0


if __name__ == '__main__':
    sys.exit(main())
