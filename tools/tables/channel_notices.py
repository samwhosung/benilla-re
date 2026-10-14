#!/usr/bin/env python3
"""tables/channel_notices.tsv: the 32 notices of the channel notify message (opcode 0x99).

The handler 0x49bf80 reads a u8 notice and a channel name, then jumps through the 32-entry table
at 0x49c60c. Each arm reads its own fields, picks the token that becomes the event's first
argument and the chat type (`edi`: 0x0a unless the arm sets 0x0f, 0x10, 0x12 or 0x13), and all
arms but one end in the same two calls, `0x64a5a0` (copy the token) and `0x49a870` (the chat
line). The generator follows each arm's code up to the shared tail at 0x49c4df, collecting the
data store reads (`GetGuid`, `GetInt32`, `GetCString`, `GetUInt8`), every token string
pushed (joined by `|` where the arm picks one at run time) and the chat type the arm leaves in
`edi`. An arm that pushes no token has none.
"""

import sys

from lib import Disasm, Image, addr, note, parse_args, write_table

TABLE, COUNT = 0x49C60C, 32
TAIL = 0x49C4DF
END = 0x49C609          # the handler's last arm ends before the table
READS = {0x4190B0: 'guid', 0x418E30: 'u32', 0x4191B0: 'cstring', 0x418CB0: 'u8',
         0x418EB0: 'u32'}
TYPES = {0x0A: 'SYSTEM', 0x0F: 'CHANNEL_JOIN', 0x10: 'CHANNEL_LEAVE', 0x12: 'CHANNEL_NOTICE',
         0x13: 'CHANNEL_NOTICE_USER'}


def main():
    args = parse_args(__doc__, disasm=True)
    img, dis = Image(args.exe), Disasm(args.disasm)
    rows = []
    for notice in range(COUNT):
        arm = img.u32(TABLE + 4 * notice)
        seen, work = set(), [arm]
        reads, tokens, kind = [], {}, 0x0A
        reaches_tail = False
        while work:
            a = work.pop()
            while a not in seen:
                seen.add(a)
                k = dis.at.get(a)
                if k is None:
                    break
                _, size, mn, ops = dis.ins[k]
                if a == TAIL:
                    reaches_tail = True
                    kind = 0x13
                if a >= TAIL + 5 and a < 0x49C5C0:
                    break
                if mn == 'mov' and ops.startswith('edi,0x'):
                    kind = int(ops[4:], 16)
                elif mn == 'push' and ops.startswith('0x8'):
                    s = img.cstr(int(ops, 16), 40)
                    if s and s.isupper():
                        tokens[a] = s
                elif mn == 'call':
                    t = int(ops, 16) if ops.startswith('0x') else None
                    if t in READS:
                        reads.append((a, READS[t]))
                    if t == 0x49A870:
                        break
                elif mn == 'jmp' and ops.startswith('0x'):
                    a = int(ops, 16)
                    continue
                elif mn.startswith('j') and ops.startswith('0x'):
                    t = int(ops, 16)
                    if t < TAIL + 5:
                        work.append(t)
                elif mn == 'ret':
                    break
                a += size
        wire = ' '.join(kind_ for _, kind_ in sorted(reads))
        token = '|'.join(tokens[a] for a in sorted(tokens))
        rows.append([notice, addr(arm), token, TYPES.get(kind, hex(kind)) if token or notice in (0, 1) else '', wire])
    note('%d notices' % len(rows))
    write_table(['notice', 'arm', 'token', 'chat_type', 'reads'], rows, args.out)


if __name__ == '__main__':
    main()
