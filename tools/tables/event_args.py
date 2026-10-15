#!/usr/bin/env python3
"""tables/event_args.tsv: each event name and the arguments its producers hand to Lua.

One row per event name in events.tsv. A producer is one of four things:

  signal             a `FrameScript_SignalEvent 0x703e50` site (id in `ecx`, no arguments): zero
                     Lua values, written `()`.
  signal2            a `FrameScript_SignalEvent2 0x703f50` site: the site's format string, from
                     event_sites.tsv, is the shape. Its ids are the ones event_sites.tsv gives
                     it, computed ones included (the chat type table, a watcher's argument).
  unit-field-bridge  `0x51bbb0` registers a watch for every named event id below the bound of
                     its loop (`cmp esi,0xb6` before the `jl` to the loop head); the watch
                     callback `0x51bd50` calls `0x515e50`, which fans out over the unit tokens
                     and calls `0x703f50(id, "%s", token)`. Shape `%s`.
  token-fanout       a call or tail jump to `0x515e50` with a literal id in `edx`: the same
                     fan-out, shape `%s`. An id computed from a byte (`0x11 + n`, `0x17 + n`) is
                     not read.

The fan-out's own site, `0x515e7a` in `0x515e50`, is the one row of event_sites.tsv whose ids
come through `0x515e50`'s parameter (note `through-0x515e50`). Its ids are credited to the two
fan-out producers above, not to `signal2`, and the generator stops unless they are exactly the
named ids the two producers reach: two readings of one set.

`ids` is every id of the name that a producer reaches or the name table holds, ascending and
`|`-separated. A site that event_sites.tsv gives several ids contributes its shape to each
named id; a `?` id and a glue-space site (`space` = `glue`, whose ids name other events) are
left out. `arg_formats` lists each distinct shape once, sorted, `|`-separated; `?` where no
producer reaches the name.

`conf` is `exact` (every producer's shape is known), `advisory` (a contributing site's format
declares more values than its caller pushes, found as tools/event-args.py finds it; the script
reads whatever lies above the last push) or `none` (no resolved id of a fire site is the name's:
event_sites.py resolves every site's id or stops, and lists apart only the ids a site computes
as K plus an unbounded byte). `note` says which sites declare more than they push, `;`-separated.
"""

import re

from pathlib import Path

from lib import (SIGNAL2, Disasm, addr, declared, note, parse_args, read_tsv,
                 signal2_passed, write_table)

FANOUT = 0x515E50
BRIDGE = 0x51BBB0
NONE = '()'
HEAD = re.compile(r'^cmp\s+esi,0x([0-9a-f]+)$')


def bridge_bound(dis):
    """The id bound of 0x51bbb0's loop: the `cmp esi,N` before the `jl` back to the loop head."""
    k = dis.at[BRIDGE]
    while dis.ins[k][2] != 'ret':
        k += 1
    for j in range(dis.at[BRIDGE], k):
        a, _, mn, ops = dis.ins[j]
        if mn == 'jl' and int(ops, 16) < a:
            m = HEAD.match('%s %s' % dis.ins[j - 1][2:])
            if m:
                return int(m.group(1), 16)
    raise SystemExit('event_args: no loop bound found in %s' % addr(BRIDGE))


def fanout_ids(dis):
    """The literal ids in `edx` at each call or jmp to 0x515e50, with the control that the scan
    reads the wrapper's own callers."""
    ids, calls = set(), 0
    for k, (a, _, mn, ops) in enumerate(dis.ins):
        if mn not in ('call', 'jmp') or ops != '0x%x' % FANOUT:
            continue
        calls += 1
        for j in range(k - 1, max(k - 14, -1), -1):
            _, _, m2, o2 = dis.ins[j]
            if m2 in ('call', 'ret', 'jmp') or m2.startswith('j'):
                break
            if m2 == 'mov' and o2.startswith('edx,'):
                if re.fullmatch(r'edx,0x[0-9a-f]+', o2):
                    ids.add(int(o2[4:], 16))
                break
    if not calls or not ids:
        raise SystemExit('event_args: control failed: no literal-id caller of %s' % addr(FANOUT))
    return ids


def main():
    args = parse_args(__doc__, disasm=True)
    dis = Disasm(args.disasm)
    names = {int(r['id']): r['name'] for r in read_tsv(Path(args.tables) / 'events.tsv')}
    ev = {}

    def add(name, eid, producer, fmt, text=None):
        e = ev.setdefault(name, {'ids': set(), 'producers': set(), 'formats': set(), 'notes': []})
        if eid is not None:
            e['ids'].add(eid)
        if producer:
            e['producers'].add(producer)
            e['formats'].add(fmt)
        if text and text not in e['notes']:
            e['notes'].append(text)

    glue, through = 0, set()
    for r in read_tsv(Path(args.tables) / 'event_sites.tsv'):
        if r['space'] == 'glue':
            glue += 1
            continue
        if 'through-%s' % addr(FANOUT) in r['notes'].split(';'):
            through.update(int(e) for e in r['event_ids'].split('|') if int(e) in names)
            continue
        two = r['dispatcher'] == 'SignalEvent2'
        fmt = (r['arg_format'] or NONE) if two else NONE
        text = None
        if two:
            site = int(r['site'], 16)
            if dis.ins[dis.at[site]][3] != '0x%x' % SIGNAL2:
                raise SystemExit('event_args: control failed: %s is no call to %s'
                                 % (r['site'], addr(SIGNAL2)))
            got = signal2_passed(dis, dis.at[site], declared(r['arg_format']))
            if got is not None and got[0] < declared(r['arg_format']):
                text = '%s declares %d values and passes %d' % (r['site'], declared(r['arg_format']),
                                                                got[0])
        ids = r['event_ids'].split('|')
        for eid, name in zip(ids, (r['event_names'].split('|') + [''] * len(ids))):
            if eid != '?' and name:
                add(name, int(eid), 'signal2' if two else 'signal', fmt, text)

    bound = bridge_bound(dis)
    fanout = {eid for eid in fanout_ids(dis) if eid in names}
    bridge = {eid for eid in names if eid < bound}
    if through != bridge | fanout:
        raise SystemExit('event_args: control failed: the fan-out site gives %s, the producers %s'
                         % (sorted(through ^ (bridge | fanout)), 'differ there'))
    for eid in sorted(bridge):
        add(names[eid], eid, 'unit-field-bridge', '%s')
    for eid in sorted(fanout):
        add(names[eid], eid, 'token-fanout', '%s')
    for eid, name in names.items():
        add(name, eid, None, None)

    rows = []
    for name in sorted(ev):
        e = ev[name]
        conf = 'none' if not e['producers'] else ('advisory' if e['notes'] else 'exact')
        rows.append([name, '|'.join(str(i) for i in sorted(e['ids'])),
                     '|'.join(sorted(e['producers'])), '|'.join(sorted(e['formats'])) or '?',
                     conf, '; '.join(e['notes'])])
    note('%d events, %d with a producer; %d glue-space sites left out; loop bound 0x%x'
         % (len(rows), sum(1 for r in rows if r[2]), glue, bound))
    write_table(['name', 'ids', 'producers', 'arg_formats', 'conf', 'note'], rows, args.out)


main()
