#!/usr/bin/env python3
"""tables/verb_events.tsv: the events a Lua verb fires, keyed by the verb.

A row says the client signals `event` on the call path of the registered Lua binding `verb`:

  body    a signal site inside the verb's own function extent.
  helper  a signal site inside a function whose every caller is a verb (at most 8 callers); `via`
          is that function. A function with any caller that is not a verb (a packet handler, the
          cursor, the tutorial signal) fires a state event and is left out, and so is a hub
          (`ClearCursor` -> CURSOR_UPDATE, 49 callers).
  traced  a site reached through a shared dispatcher, read by hand (TRACED below); `via` is the
          dispatcher. The pet orders' workers (0x4bd4f0 to 0x4bd5f0) and `CastPetAction` enter
          0x4bd1d0, which calls 0x4bc940 (the reaction arm, signalling at 0x4bc956) and 0x4bc960
          (the Stay and Follow arm, signalling at 0x4bc979), both before the send at 0x4bd444.
          A right-click dispatcher that is no verb also calls 0x4bd1d0, so the walk cannot claim
          it. The Attack arm (command 2, jump-table slot 6 at 0x4bd3a3) latches 0x161 and signals
          at 0x4bd438, inside 0x4bd1d0 itself, once the attack check passes; `PetAttack` reaches
          it through 0x4bd630, and `CastPetAction` through the same dispatcher. The generator
          checks that each traced site is a signal site in `via` or a function `via` calls, and
          that the verb reaches `via` through calls.tsv.

A verb is a row of bindings.tsv whose function lies outside the glue object files (0x46a000 to
0x476000) and whose name does not start with `Get`: a getter's fire is a cache-miss re-query
announced when the answer lands, a state event. Where a function has several names the first in
table order names it. A site is a call or tail jump to `FrameScript_SignalEvent 0x703e50` or
`SignalEvent2 0x703f50` in event_sites.tsv, outside the glue space, for each id it lists that has a
name and is 0xb6 or more: the ids below are the unit-field bridge and token fan-out, state events
(see event_args.tsv). A site belongs to the function row whose extent holds it; a site in no row's
extent is left out. A function's callers are the rows of calls.tsv whose callees hold it.

`sites` lists the signal sites, ascending, `|`-separated. A pair with a body fire and a helper
fire is `body`. A floor, not a census: a verb with no row may still fire an event, because only
the literal signal sites are read and `helper` stops one call deep.
"""

import collections

from pathlib import Path

from lib import (Disasm, Functions, addr, note, parse_args, read_tsv, write_table)

GLUE = (0x46A000, 0x476000)
UNIT_WINDOW_FIELDS = 0xB6
MAX_HELPER_CALLERS = 8
SIGNALS = ('0x703e50', '0x703f50')

# (verb, event, via, sites)
TRACED = [
    ('CastPetAction', 'PET_BAR_UPDATE', 0x4BD1D0, (0x4BC956, 0x4BC979, 0x4BD438)),
    ('PetAttack', 'PET_BAR_UPDATE', 0x4BD1D0, (0x4BD438,)),
    ('PetAggressiveMode', 'PET_BAR_UPDATE', 0x4BD1D0, (0x4BC956,)),
    ('PetDefensiveMode', 'PET_BAR_UPDATE', 0x4BD1D0, (0x4BC956,)),
    ('PetFollow', 'PET_BAR_UPDATE', 0x4BD1D0, (0x4BC979,)),
    ('PetPassiveMode', 'PET_BAR_UPDATE', 0x4BD1D0, (0x4BC956,)),
    ('PetWait', 'PET_BAR_UPDATE', 0x4BD1D0, (0x4BC979,)),
]


def main():
    args = parse_args(__doc__, disasm=True)
    dis, fns = Disasm(args.disasm), Functions()
    callees = {int(r['function'], 16): [int(c, 16) for c in r['callees'].split('|')]
               for r in read_tsv(Path(args.tables) / 'calls.tsv')}
    callers = collections.defaultdict(set)
    for src, targets in callees.items():
        for t in targets:
            callers[t].add(src)

    verbs = {}
    for r in read_tsv(Path(args.tables) / 'bindings.tsv'):
        fn = int(r['fn'], 16)
        if GLUE[0] <= fn < GLUE[1] or r['name'].startswith('Get'):
            continue
        verbs.setdefault(fn, r['name'])
    if len(verbs) < 1000:
        raise SystemExit('verb_events: bindings.tsv gave only %d verbs' % len(verbs))

    sites = []
    for r in read_tsv(Path(args.tables) / 'event_sites.tsv'):
        va = int(r['site'], 16)
        if r['space'] == 'glue':
            continue
        if dis.ins[dis.at[va]][3] not in SIGNALS:
            raise SystemExit('verb_events: control failed: %s is no signal call' % r['site'])
        ids = r['event_ids'].split('|')
        for eid, name in zip(ids, r['event_names'].split('|') + [''] * len(ids)):
            if eid != '?' and name and int(eid) >= UNIT_WINDOW_FIELDS:
                sites.append((va, name))

    table = {}

    def add(verb, event, shape, via, site):
        row = table.setdefault((verb, event), {'shape': shape, 'via': via, 'sites': set()})
        if shape == 'body':
            row['shape'], row['via'] = 'body', '-'
        row['sites'].add(site)

    outside = 0
    for va, event in sites:
        row = fns.holding(va)
        if row is None:
            outside += 1
            continue
        fn = row['address']
        if fn in verbs:
            add(verbs[fn], event, 'body', '-', va)
            continue
        cs = callers.get(fn, set())
        if cs and len(cs) <= MAX_HELPER_CALLERS and all(c in verbs for c in cs):
            for c in cs:
                add(verbs[c], event, 'helper', addr(fn), va)

    by_name = {n: fn for fn, n in verbs.items()}
    signal_sites = {va for va, _ in sites}
    for verb, event, via, traced in TRACED:
        reach, frontier = {by_name[verb]}, {by_name[verb]}
        for _ in range(3):
            frontier = {t for f in frontier for t in callees.get(f, ())} - reach
            reach |= frontier
        if via not in reach:
            raise SystemExit('verb_events: control failed: %s does not reach %s' % (verb, addr(via)))
        for site in traced:
            held = fns.holding(site)
            if site not in signal_sites or held is None or held['address'] != via and held['address'] not in callees.get(via, ()):
                raise SystemExit('verb_events: control failed: %s is no signal site in %s or a function it calls'
                                 % (addr(site), addr(via)))
            add(verb, event, 'traced', addr(via), site)

    rows = [[v, e, r['shape'], r['via'], '|'.join(addr(s) for s in sorted(r['sites']))]
            for (v, e), r in sorted(table.items())]
    note('%d pairs, %d verbs, %d events; %d sites in no function extent'
         % (len(rows), len({r[0] for r in rows}), len({r[1] for r in rows}), outside))
    write_table(['verb', 'event', 'shape', 'via', 'sites'], rows, args.out)


main()
