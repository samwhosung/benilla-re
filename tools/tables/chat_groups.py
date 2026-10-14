#!/usr/bin/env python3
"""tables/chat_groups.tsv: the 68 message groups a chat window can show, in table order.

The table at 0x805fb0 is 68 records of 12 bytes: a name pointer, a default-on flag and the
file version that added the group (the chat settings loader at 0x498a60 and the window start-up
at 0x4982c0 read the last two). A window's flag byte i (+0x20 of its record) belongs to group i.
"""

from lib import Image, addr, note, parse_args, write_table

TABLE, STRIDE, COUNT = 0x805FB0, 12, 68


def main():
    args = parse_args(__doc__)
    img = Image(args.exe)
    rows = []
    for i in range(COUNT):
        base = TABLE + i * STRIDE
        rows.append([i, img.cstr(img.u32(base), 64), img.u32(base + 4), img.u32(base + 8)])
    note('%d message groups' % len(rows))
    write_table(['index', 'name', 'default_on', 'added_in'], rows, args.out)


if __name__ == '__main__':
    main()
