#!/usr/bin/env python3
"""tables/glue_surface.tsv: every Lua function the glue screens' state has in `_G` before
GlueXML loads.

The glue builder `0x46a7b0` starts from the base set `0x7039e0`: a rebuild resets the Lua state
(`0x703b80` calls `0x7039e0`, at `0x46a87b`), and the first build (`0x46a505`, passing 0) skips
that reset and takes the state the start-up init left. It then calls the glue registrar
`0x46abb0`, which calls seven registrar functions (the last by a tail jump) and does nothing
else. Each registrar is a loop over a table of (name, function) pairs: `mov edx,[esi+T+4]`,
`mov ecx,[esi+T]`, `call 0x704120`, `add esi,8`, `cmp esi,8*N`, `jb`. The generator reads the
registrars from the glue registrar's calls and each loop's table T and count N from the
listing, and takes the rows of bindings.tsv whose `table` is T; the count must equal N.

`0x7039e0` is the base set both states run. Its own loop installs the base array (`0x811e28`,
bindings.tsv's `baselib` rows). It opens the `string`, `table`, `math` and `bit` libraries by
`luaL_openlib 0x6f4dc0`, each opener a `push <table>`, `mov edx,<name>`, `call` (fields of a
library table, not globals, so not rows here). `luaopen_math` also sets the global `__pow` to
the function at `0x7fb050`. A compat chunk (Lua source in the image, passed to `0x704cd0` at
`0x703b26`) then binds bare names. The generator reads the chunk's text: `local a = lib` names
a library, `name = a.field` binds the library table's `field` (a row taking that binding's `fn`
and table; a field the table has no binding for, `PI = math.pi`, is a number and no row) and
`name = function ...` is a Lua closure that converts degrees around the `math` function of the
same name (`fn` is `lua`, `table_va` empty, `arity` `?`).

Columns:
  scope      `base` (0x7039e0), `shared` (a registrar the in-game state also calls), `glue`.
  registrar  the function that installs the row; `0x7039e0` for the base set, its aliases and
             `__pow`.
  name       the global; the key.
  fn, table_va, arity  as bindings.tsv has them; a compat alias carries the library binding it
             names; `__pow` has no table.
  world_fn   a glue row's in-game namesake: the binding of the same name in any `global` table
             other than these registrars', `|`-joined; empty for none. The in-game state never
             sees a glue row, but the name is shared.
  note       `shape-differs: ...` where both shapes are exact and the argument count or the
             return arity differs between the glue binding and its in-game namesake.

Rows are sorted by name.
"""

import re

from pathlib import Path

from lib import Disasm, Image, addr, note, parse_args, read_tsv, write_table

GLUE_BUILDER = 0x46A7B0
GLUE_REGISTRAR = 0x46ABB0
BASE_INIT = 0x7039E0
OPENLIB = 0x6F4DC0
MATH_OPEN_NAME = 'math'
LOCAL = re.compile(r'^local\s+(\w+)\s*=\s*(\w+)\s*$')
ALIAS = re.compile(r'^(\w+)\s*=\s*(\w+)\.(\w+)\s*$')
CLOSURE = re.compile(r'^(\w+)\s*=\s*function\b')
CHUNK_CALL = 0x703B26


def loop(dis, start):
    """The (table, rows) of the `esi` loop that begins at or after `start`: T is the lowest
    `[esi+T]` operand, N the `cmp esi,8*N` before the `jb` back to the loop head."""
    k = dis.at[start]
    head = None
    offs = []
    while True:
        a, _, mn, ops = dis.ins[k]
        m = re.fullmatch(r'e[a-d]x,DWORD PTR \[esi\+0x([0-9a-f]+)\]', ops) if mn == 'mov' else None
        if m:
            offs.append(int(m.group(1), 16))
            head = head if head is not None else a
        if mn == 'jb' and head is not None and int(ops, 16) == head:
            cmp = dis.ins[k - 1]
            if cmp[2] == 'cmp' and cmp[3].startswith('esi,0x'):
                return min(offs), int(cmp[3][6:], 16) // 8
            break
        k += 1
        if k - dis.at[start] > 120:
            break
    raise SystemExit('glue_surface: no pair loop at %s' % addr(start))


def calls_of(dis, start, kinds=('call', 'jmp')):
    """The bare-address call and jump targets of the straight-line code at `start`, up to a ret
    or a tail jmp (inclusive)."""
    k, out = dis.at[start], []
    while True:
        a, _, mn, ops = dis.ins[k]
        if mn in kinds and re.fullmatch(r'0x[0-9a-f]+', ops):
            out.append(int(ops, 16))
            if mn == 'jmp':
                return out
        if mn == 'ret':
            return out
        k += 1


def openers(dis, img):
    """The libraries `0x7039e0` opens: {name: table} from each opener's `push <table>` and
    `mov edx,<name>` before the `luaL_openlib` call."""
    libs, openers_ = {}, {}
    for t in calls_of(dis, BASE_INIT, ('call',)):
        k = dis.at.get(t)
        if k is None:
            continue
        table = name = None
        for j in range(k, k + 12):
            _, _, mn, ops = dis.ins[j]
            if mn == 'push' and ops.startswith('0x') and table is None and ops != '0x0':
                table = int(ops, 16)
            m = re.fullmatch(r'edx,0x([0-9a-f]+)', ops) if mn == 'mov' else None
            if m and name is None:
                name = img.cstr(int(m.group(1), 16))
            if mn == 'call':
                if int(ops, 16) == OPENLIB and table and name:
                    libs[name], openers_[name] = table, t
                break
    return libs, openers_


def main():
    args = parse_args(__doc__, disasm=True)
    dis, img = Disasm(args.disasm), Image(args.exe)
    bindings = read_tsv(Path(args.tables) / 'bindings.tsv')
    by_table = {}
    for r in bindings:
        by_table.setdefault(int(r['table'], 16), []).append(r)

    callers = {}
    for r in read_tsv(Path(args.tables) / 'calls.tsv'):
        for c in r['callees'].split('|'):
            callers.setdefault(int(c, 16), set()).add(int(r['function'], 16))
    glue_lo, glue_hi = 0x46A000, 0x476000

    out = {}

    def take(registrar, table, rows, scope):
        got = by_table.get(table, [])
        if len(got) != rows:
            raise SystemExit('glue_surface: table %s: the registrar loops %d rows, bindings.tsv '
                             'has %d' % (addr(table), rows, len(got)))
        for r in got:
            out[r['name']] = dict(r=r, scope=scope, registrar=registrar, fn=r['fn'],
                                  table=r['table'], arity=r['arity'])

    take(BASE_INIT, *loop(dis, BASE_INIT), 'base')
    registrars = calls_of(dis, GLUE_REGISTRAR)
    if len(registrars) != 7:
        raise SystemExit('glue_surface: %s calls %d registrars, not 7' % (addr(GLUE_REGISTRAR),
                                                                          len(registrars)))
    ours = set()
    for reg in registrars:
        table, rows = loop(dis, reg)
        ours.add(table)
        outside = {c for c in callers.get(reg, ()) if not glue_lo <= c < glue_hi}
        take(reg, table, rows, 'shared' if outside else 'glue')

    # The base set's Lua side: the libraries, the compat chunk's aliases and `__pow`.
    libs, opener = openers(dis, img)
    if set(libs) != {'string', 'table', 'math', 'bit'}:
        raise SystemExit('glue_surface: libraries opened: %s' % sorted(libs))
    fields = {(lib, r['name']): r for lib, t in libs.items() for r in by_table.get(t, [])}
    chunk_call = dis.ins[dis.at[CHUNK_CALL]]
    if chunk_call[3] != '0x704cd0':
        raise SystemExit('glue_surface: control failed: %s is no call to 0x704cd0' % addr(CHUNK_CALL))
    text = chunk_text(img)
    local = {}
    aliases, closures = [], 0
    for line in text.split('\n'):
        line = line.strip()
        m = LOCAL.match(line)
        if m:
            local[m.group(1)] = m.group(2)
            continue
        m = ALIAS.match(line)
        if m and local.get(m.group(2), m.group(2)) in libs:
            r = fields.get((local.get(m.group(2), m.group(2)), m.group(3)))
            if r is not None:
                aliases.append((m.group(1), r['fn'], r['table'], r['arity']))
            continue
        m = CLOSURE.match(line)
        if m:
            aliases.append((m.group(1), 'lua', '', '?'))
            closures += 1
    pow_fn = pow_function(dis, img, opener['math'])
    pow_row = next(r for r in by_table[libs['math']] if int(r['fn'], 16) == pow_fn)
    aliases.append(('__pow', addr(pow_fn), '', pow_row['arity']))
    for name, fn, table, arity in aliases:
        if name in out:
            raise SystemExit('glue_surface: %s is a compat alias and a registered name' % name)
        out[name] = dict(r=None, scope='base', registrar=BASE_INIT, fn=fn, table=table,
                         arity=arity)

    world = {}
    for r in bindings:
        if r['table_kind'] == 'global' and int(r['table'], 16) not in ours:
            world.setdefault(r['name'], []).append(r)

    rows, differs = [], 0
    for name in sorted(out):
        o = out[name]
        r = o['r']
        world_fn, text = '', ''
        if o['scope'] == 'glue' and name in world:
            w = world[name]
            world_fn = '|'.join(x['fn'] for x in w)
            notes = []
            for x in w:
                if r['argc_conf'] == 'exact' and x['argc_conf'] == 'exact' and r['argc'] != x['argc']:
                    notes.append('argc %s here, %s in-game (%s)' % (r['argc'], x['argc'], x['fn']))
                if (r['arity_conf'] == 'exact' and x['arity_conf'] == 'exact'
                        and r['arity'] != x['arity']):
                    notes.append('arity %s here, %s in-game (%s)' % (r['arity'], x['arity'], x['fn']))
            if notes:
                text = 'shape-differs: ' + '; '.join(notes)
                differs += 1
        rows.append([name, o['scope'], addr(o['registrar']), o['fn'], o['table'], o['arity'],
                     world_fn, text])
    count = {s: sum(1 for r in rows if r[1] == s) for s in ('base', 'shared', 'glue')}
    note('%d functions: %s; %d closures; %d shape differences' % (len(rows), count, closures, differs))
    write_table(['name', 'scope', 'registrar', 'fn', 'table_va', 'arity', 'world_fn', 'note'],
                rows, args.out)


def chunk_text(img):
    """The compat chunk's text: the string that opens with the table library's aliases."""
    start = img.mem.find(b'local tab = table')
    if start < 0:
        raise SystemExit('glue_surface: the compat chunk is not in the image')
    end = img.mem.find(b'\0', start)
    return bytes(img.mem[start:end]).decode('latin1')


def pow_function(dis, img, opener):
    """The function `luaopen_math` stores under the global `__pow`: the `mov edx,<function>`
    that follows the `mov edx,<"__pow">`."""
    k = dis.at[opener]
    seen = False
    while dis.ins[k][2] != 'ret':
        _, _, mn, ops = dis.ins[k]
        m = re.fullmatch(r'edx,0x([0-9a-f]+)', ops) if mn == 'mov' else None
        if m:
            v = int(m.group(1), 16)
            if seen:
                return v
            seen = img.cstr(v) == '__pow'
        k += 1
    raise SystemExit('glue_surface: no __pow in luaopen_math')


main()
