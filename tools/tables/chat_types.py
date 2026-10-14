#!/usr/bin/env python3
"""tables/chat_types.tsv: the 94 chat types, one row each, in the order the client indexes them.

The type's name and default colour are the 67-byte records at 0x804710 (a 64-byte name, then
red, green and blue). The event the type fires is the dword at 0x8062e0 + 4 * index, read by
0x49b0b0 after it bounds the index below 94 (`cmp edx,0x5e`); the event's name is the entry of
tables/events.tsv with that id, empty where the id has no name.
"""

import csv
import sys

from lib import ROOT, Image, addr, note, parse_args, write_table

TYPES, STRIDE, COUNT = 0x804710, 0x43, 94
EVENTS = 0x8062E0


def main():
    args = parse_args(__doc__)
    img = Image(args.exe)
    names = {}
    with open(ROOT / 'tables' / 'events.tsv', encoding='utf-8') as f:
        for row in csv.DictReader(f, delimiter='\t'):
            names[int(row['id'])] = row['name']
    rows = []
    for i in range(COUNT):
        base = TYPES + i * STRIDE
        name = img.cstr(base, 64)
        if name is None:
            sys.exit('chat type %d has no name' % i)
        event = img.u32(EVENTS + 4 * i)
        rows.append([i, name, '%02x%02x%02x' % tuple(img.bytes(base + 0x40, 3)), event,
                     names.get(event, '')])
    note('%d chat types' % len(rows))
    write_table(['index', 'name', 'rgb', 'event_id', 'event'], rows, args.out)


if __name__ == '__main__':
    main()
