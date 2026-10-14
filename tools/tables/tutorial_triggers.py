#!/usr/bin/env python3
"""tables/tutorial_triggers.tsv: every place the client fires or acknowledges a tutorial.

A tutorial has a number from 0 to 49 (the interface sees it plus one). Two functions take it in
ecx:

    fire  0x4b5390  ecx = number, edx = delay in milliseconds; set the fire-once bit and signal
                    TUTORIAL_TRIGGER now (delay 0) or after the delay
    ack   0x4b54c0  ecx = number; set both bits and tell the server

A site is a call or a tail jmp to either. The generator walks the control-flow graph backward from
each site (the walk tables/event_sites.py uses) and collects every definition of ecx, and for a fire
site of edx, that reaches it; `?` is a value the walk cannot name (a table read or a parameter).
Two control sites are checked on every run.
"""

import csv
import sys

from lib import Disasm, Image, ROOT, addr, note, parse_args, write_table
import event_sites

FIRE, ACK = 0x4B5390, 0x4B54C0
CONTROLS = {0x514A97: ({3}, {10000}), 0x4D74AE: ({12, 13}, None)}


def function_starts():
    out = []
    with open(ROOT / 'symbols' / 'functions.tsv', newline='') as f:
        for row in csv.DictReader(f, delimiter='\t'):
            out.append((int(row['address'], 16), int(row['size'] or 0)))
    out.sort()
    return out


def containing(starts, va):
    lo, hi = 0, len(starts)
    while lo < hi:
        mid = (lo + hi) // 2
        if starts[mid][0] <= va:
            lo = mid + 1
        else:
            hi = mid
    s, n = starts[lo - 1]
    return s if s <= va < s + n else None


def names(vals, bad):
    return '?' if bad else '|'.join(str(v) for v in sorted(vals))


def main():
    args = parse_args(__doc__, disasm=True)
    Image(args.exe)
    dis = Disasm(args.disasm)
    cfg = event_sites.Cfg(dis)
    starts = function_starts()
    rows, res = [], {}
    for i, (a, mn, ops) in enumerate(cfg.ins):
        if mn in ('call', 'jmp') and event_sites.IMM.match(ops) and int(ops, 16) in (FIRE, ACK):
            fire = int(ops, 16) == FIRE
            ids, bad = event_sites.resolve(cfg, i, 'SignalEvent')
            delay = None
            if fire:
                dv, db = event_sites.resolve(cfg, i, 'SignalEvent', reg='edx')
                delay = (dv, db)
            res[a] = (ids, bad, delay)
            fn = containing(starts, a)
            rows.append([addr(a), addr(fn) if fn else '', 'fire' if fire else 'ack',
                         names(ids, bad), names(*delay) if delay else ''])
    for a, (want_ids, want_delay) in CONTROLS.items():
        ids, bad, delay = res[a]
        if bad or ids != want_ids or (want_delay is not None and (delay[1] or delay[0] != want_delay)):
            sys.exit('control failed at 0x%x: %s' % (a, res[a]))
    note('%d sites, %d fire, %d ack, 2 controls passed'
         % (len(rows), sum(r[2] == 'fire' for r in rows), sum(r[2] == 'ack' for r in rows)))
    write_table(['site', 'fn', 'verb', 'tutorials', 'delay_ms'], rows, args.out)


if __name__ == '__main__':
    sys.exit(main())
