#!/usr/bin/env python3
"""Naming leads for the function rows, from a decompilation of a later client.

    tools/namelead.py PATH/TO/DECOMP [-o dumps/namelead.tsv] [--measure]

Reads: PATH/TO/DECOMP, a local checkout of a C++ decompilation of a later build of the client in
which each reconstructed function carries a `// OFFSET: 0x...` comment with its address in that
build (the 3.3.5a decompilation at github.com/SatyPardus/wowdecomp is one), read as text: nothing
in it is built or run. Beside it, symbols/functions.tsv, tables/calls.tsv, the exe (its strings
and data tables) and the listing, dumps/disasm.txt (options --exe and --disasm).

Writes: dumps/namelead.tsv (-o, refused anywhere but under dumps/), one line per lead. With
--measure, also dumps/namelead-measure.tsv, the leads of the measured run below.

The output is leads, never findings (docs/METHOD.md, The binary decides). A name enters symbols/
only through a reading at the bytes, under docs/CONTRIBUTING.md, Adding a name, and a lead is a
hint for where to look. The file is derived from a third party's work that carries no license, so
it stays under dumps/, which git ignores: no name, address, string or file from it is committed.

Four methods pair a row with a decompiled function:

  string    The string literals a row's instructions name by address (a format, a CVar or command
            name, an error text, a file name) against the literals in a decompiled body. A shared
            string weighs 1/(rows that name it x bodies that hold it). Source paths are left out:
            the decompilation writes __FILE__.
  register  A string beside a function address: a data table of (name, function) pairs, and a
            call whose arguments are a string and function addresses, against `{ "name", Func }`
            initializers and calls with a literal and function names in the source. A table pairs
            with the decompiled table that shares most of its names.
  calls     The names of a row's named callees and callers against the names a decompiled body
            calls and the bodies that call it, each weighed by log(functions / bodies that hold
            it).
  align     When a paired row and its decompiled function have every callee (or caller) matched
            by pair or by name but one on each side, those two are a lead: one vote is tier C,
            two B, three A.

The string and register leads, and then the calls and align leads, at tier A or B are fed back as
names and pairs, and calls and align run again until a round adds no pair (a guard of 100 rounds
says so when it stops the loop). A lead's tier is A or B
when the measure below supports it, C otherwise; C is a list of candidates, not a lead to read.

--measure takes a fifth of the named rows (by a hash of the address), runs every method as if
they had no names, and prints, per method and tier, how the leads for those rows compare with
their names: in full (`Script::Name` against `Script_Name`), by the last part (`Class::Name`
against `Script_Name`), or not. Since this repo's names and the later build's differ in style, it
also counts the rows whose name is exactly one decompiled function's and how often the lead is
that function: the precision. Names this repo makes mechanically (initializers, thunks, exception
code) are left out, and so are the rows tools/namelead-read.tsv lists, named from a lead: their
names would confirm their own leads. The rows measured are easier than the unnamed ones, so the
measure is an upper bound.

What it cannot see: a function with no string, no registration and no named neighbour (most small
helpers and unreached code); code the later build inlined, split or rewrote, where a string or a
neighbour points at the caller or the callee instead; vtable slots, as the classes measured have
vtables of another length and order in the two builds; and library code the decompilation does
not hold.

Columns: address, the row's name (empty when it has none), method, tier, score, margin over the
next candidate, shared (strings, pairs, names or votes), lead name, lead file, later-build
address, file (`match` when a source path the row passes names the lead's file, `differ` when it
names another, empty without one) and the evidence, truncated.
"""

import argparse
import math
import os
import re
import struct
import sys
import zlib
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / 'tables'))
from lib import ROOT, Disasm, Functions, Image, addr, note, read_tsv  # noqa: E402

# ---------------------------------------------------------------------------------------------
# Names

MECHANICAL = re.compile(r'^(Init|SEH_|Register.*Exit$|Destroy.*Exit$)|Thunk$|AtExit$|'
                        r'[0-9a-f]{6}(Exit|Thunk)?$')
PLACEHOLDER = re.compile(r'^(unk_|sub_|FUN_)|::unk_', re.I)


def norm(name):
    """A name without its decoration, separators and case: `Script::Name`, `Script_Name` and
    `_ScriptName@4` all give `scriptname`; a destructor's `~` is kept apart as `dtor`."""
    name = re.sub(r'@\d+$', '', name).lstrip('_?').replace('~', 'dtor')
    name = re.sub(r'::DeletingDestructor$', '::dtor', name)
    return re.sub(r'[^0-9a-z]', '', name.lower())


def tail(name):
    """The last part of a qualified name, without a `Script_` prefix, normalized."""
    last = re.sub(r'@\d+$', '', name).split('::')[-1].lstrip('_')
    if last.startswith('~') or last == 'DeletingDestructor':
        return 'dtor'
    last = re.sub(r'^(Script_|Script)', '', last) or last
    return norm(last)


def agreement(lead, name):
    if norm(lead) == norm(name):
        return 'full'
    if tail(lead) == tail(name):
        return 'tail'
    return 'differ'


# ---------------------------------------------------------------------------------------------
# The decompilation, read as text

LEX = re.compile(r'//[^\n]*|/\*.*?\*/|(?:u8|[LuU])?R"([^(\s]*)\(.*?\)\1"|'
                 r'(?:u8|[LuU])?"(?:[^"\\\n]|\\.)*"|\'(?:[^\'\\\n]|\\.){1,4}\'', re.S)
ESC = re.compile(r'\\(x[0-9a-fA-F]+|[0-7]{1,3}|.)', re.S)
SIMPLE = {'n': '\n', 't': '\t', 'r': '\r', '0': '\0', 'a': '\a', 'b': '\b', 'f': '\f', 'v': '\v'}
TAG = re.compile(r'^[ \t]*// OFFSET: *\[?(0x[0-9a-fA-F]+)', re.M)
NAME = re.compile(r'((?:[A-Za-z_~][\w]*(?:<[^<>()]*>)?::)*(?:operator\s*\S+?|~?[A-Za-z_]\w*))'
                  r'\s*(?:<[^<>()]*>)?\s*$')
SCOPE = re.compile(r'\b(class|struct|namespace|union)\s+(?:__declspec\([^)]*\)\s*)?'
                   r'([A-Za-z_][\w:]*)?[^;{}()]*$')
CALL = re.compile(r'(?<![\w.>:])((?:[A-Za-z_]\w*::)*~?[A-Za-z_]\w*)\s*\(')
METHOD_CALL = re.compile(r'(?:->|\.)\s*([A-Za-z_]\w*)\s*\(')
KEYWORDS = {'if', 'while', 'for', 'switch', 'return', 'sizeof', 'static_cast', 'reinterpret_cast',
            'const_cast', 'dynamic_cast', 'catch', 'new', 'delete', 'defined', 'alignof',
            'decltype', 'typeid', '__declspec', 'throw', 'case', 'do', 'else', '__asm',
            'offsetof', 'assert', 'noexcept', 'static_assert', 'alignas'}
PAIR = re.compile(r'\{\s*\x01(\d+)\x01\s*,\s*(?:\([^()]*\)\s*)?&?\s*'
                  r'((?:[A-Za-z_]\w*::)*[A-Za-z_]\w*)\s*[,}]')
ARRAY = re.compile(r'\]\s*=\s*\{')
SOURCE_EXT = ('.cpp', '.c', '.h', '.hpp', '.inl', '.cc')


def unescape(body):
    def one(m):
        e = m.group(1)
        if e[0] in 'xX' and len(e) > 1:
            return chr(int(e[1:], 16) & 0xFF)
        if e[0] in '01234567':
            return chr(int(e, 8) & 0xFF)
        return SIMPLE.get(e, e)
    return ESC.sub(one, body)


class Lexed:
    """A source file with its comments blanked and each string literal (adjacent ones joined)
    replaced by a marker `\\x01<index>\\x01` padded to the literal's length."""

    def __init__(self, text):
        out, last, lits = [], 0, []
        for m in LEX.finditer(text):
            tok = m.group(0)
            out.append(text[last:m.start()])
            last = m.end()
            if tok.startswith('/'):
                out.append(re.sub(r'[^\n]', ' ', tok))
                continue
            if tok.startswith("'"):
                out.append("'" + ' ' * (len(tok) - 2) + "'")
                continue
            wide = tok[0] in 'LuU' and not tok.startswith('u8') and tok[1] in '"R'
            if re.match(r'(?:u8|[LuU])?R"', tok):
                q = tok.index('"')
                delim = tok[q + 1:tok.index('(', q)]
                value = tok[tok.index('(', q) + 1:len(tok) - len(delim) - 2]
            else:
                value = unescape(tok[tok.index('"') + 1:-1])
            # join with the literal before when only blanks lie between
            if lits and lits[-1][2] is not None and not wide and re.fullmatch(
                    r'\s*', text[lits[-1][1]:m.start()]):
                start, _, prev = lits[-1]
                lits[-1] = (start, m.end(), prev + value)
                out.append(' ' * len(tok))
                continue
            lits.append((m.start(), m.end(), None if wide else value))
            marker = '\x01%d\x01' % (len(lits) - 1)
            out.append(marker + ' ' * max(0, len(tok) - len(marker)))
        out.append(text[last:])
        self.text = ''.join(out)
        self.lits = [v for _, _, v in lits]


BRACE = re.compile(r'[{}]')
DECL_END = re.compile(r'[;{]')


def matching(text, i):
    """The index of the brace that closes the one at i, or None."""
    depth = 0
    for m in BRACE.finditer(text, i):
        depth += 1 if m.group(0) == '{' else -1
        if depth == 0:
            return m.start()
    return None


def scopes(text):
    """For each `{` of a class, struct or namespace, its (open, close, name)."""
    out, stack, last = [], [], 0
    for m in re.finditer(r'[{};]', text):
        c = m.group(0)
        if c == '{':
            head = text[last:m.start()]
            s = SCOPE.search(head)
            stack.append((m.start(), s.group(2) if s and s.group(2) else None,
                          s.group(1) if s else None))
        elif c == '}' and stack:
            o, name, kind = stack.pop()
            if name:
                out.append((o, m.start(), name, kind))
        last = m.end()
    return out


def args_of(text, i):
    """The top-level arguments of the call whose `(` is at i, as stripped strings."""
    depth, start, out = 0, i + 1, []
    for k in range(i, min(len(text), i + 4000)):
        c = text[k]
        if c in '([{':
            depth += 1
        elif c in ')]}':
            depth -= 1
            if depth == 0:
                out.append(text[start:k].strip())
                return out
        elif c == ',' and depth == 1:
            out.append(text[start:k].strip())
            start = k + 1
    return None


class Decomp:
    """The decompiled functions: name, file, later-build address, strings, calls; and the
    registrations the source makes."""

    def __init__(self, root):
        root = Path(root)
        self.fns = []
        self.pairs = []        # (name, function identifier, table key, file)
        self.reg_calls = []    # (first string argument, identifier arguments, file)
        files = 0
        for dirpath, dirnames, filenames in os.walk(root):
            dirnames[:] = [d for d in dirnames if d != '.git']
            for fn in filenames:
                if not fn.endswith(SOURCE_EXT):
                    continue
                path = Path(dirpath) / fn
                text = path.read_text(encoding='utf-8', errors='replace')
                if '// OFFSET:' not in text and '"' not in text:
                    continue
                files += 1
                self.read_file(text, str(path.relative_to(root)))
        if not self.fns:
            sys.exit('namelead: no `// OFFSET:` functions under %s' % root)
        self.by_full = defaultdict(list)
        self.by_last = defaultdict(list)
        for k, f in enumerate(self.fns):
            self.by_full[norm(f['name'])].append(k)
            self.by_last[f['name'].split('::')[-1]].append(k)
        # the call graph, where a call names one function
        self.callees = [set() for _ in self.fns]
        self.callers = [set() for _ in self.fns]
        for k, f in enumerate(self.fns):
            cls = f['name'].rsplit('::', 1)[0] if '::' in f['name'] else None
            for c in f['calls']:
                t = None
                if cls and '::' not in c:
                    t = self.resolve(cls + '::' + c, f['file'])
                if t is None:
                    t = self.resolve(c, f['file'])
                if t is not None and t != k:
                    self.callees[k].add(t)
            for m in f['methods']:
                ks = self.by_last.get(m, [])
                if len(ks) == 1 and ks[0] != k:
                    self.callees[k].add(ks[0])
            for t in self.callees[k]:
                self.callers[t].add(k)
        note('decompilation: %d files read, %d functions, %d name pairs, %d registration calls'
             % (files, len(self.fns), len(self.pairs), len(self.reg_calls)))

    def read_file(self, raw, rel):
        lx = Lexed(raw)
        text, lits = lx.text, lx.lits
        scope = scopes(text)
        # the tags are comments, blanked in `text`: find them in the raw file at the same offsets
        tags = list(TAG.finditer(raw))
        for t in tags:
            pos = t.end()
            # the declaration starts after the tag and any comment lines that follow it
            m = DECL_END.search(text, pos)
            while m:
                head = text[pos:m.start()]
                if head.count('(') == head.count(')'):
                    break
                m = DECL_END.search(text, m.end())
            if not m or m.group(0) == ';':
                continue
            head = text[pos:m.start()]
            paren = head.find('(')
            if paren < 0:
                continue
            nm = NAME.search(head[:paren])
            if not nm:
                continue
            name = re.sub(r'\s+', '', nm.group(1))
            name = re.sub(r'<[^<>]*>', '', name)
            if name in KEYWORDS:
                continue
            end = matching(text, m.start())
            if end is None:
                continue
            if '::' not in name:
                outer = [s for s in scope if s[0] < pos < s[1] and s[3] in ('class', 'struct')]
                if outer:
                    name = max(outer, key=lambda s: s[0])[2] + '::' + name
            body = text[m.start():end + 1]
            strings = set()
            for k in re.findall(r'\x01(\d+)\x01', body):
                v = lits[int(k)]
                if v is not None and len(v) >= 2:
                    strings.add(v)
            calls = set()
            for c in CALL.finditer(body):
                n = c.group(1)
                if n.split('::')[-1] not in KEYWORDS:
                    calls.add(n)
            methods = set(METHOD_CALL.findall(body))
            self.fns.append({'name': name, 'file': rel, 'offset': t.group(1).lower(),
                             'strings': strings, 'calls': calls, 'methods': methods,
                             'placeholder': bool(PLACEHOLDER.search(name))})
            # registration calls inside the body: a literal and a function name among the args
            for c in CALL.finditer(body):
                a = args_of(body, c.end() - 1)
                if not a or len(a) < 2:
                    continue
                first = next((lits[int(x.group(1))] for x in
                              (re.fullmatch(r'\x01(\d+)\x01\s*', s) for s in a) if x), None)
                if not first:
                    continue
                ids = [s.lstrip('&').strip() for s in a
                       if re.fullmatch(r'&?\s*(?:[A-Za-z_]\w*::)*[A-Za-z_]\w*', s)]
                if ids:
                    self.reg_calls.append((first, ids, rel))
        # name tables: `{ "name", Function }` pairs, grouped by the array that holds them
        arrays = []
        for a in ARRAY.finditer(text):
            o = a.end() - 1
            e = matching(text, o)
            if e is not None:
                arrays.append((o, e))
        for p in PAIR.finditer(text):
            v = lits[int(p.group(1))]
            if not v:
                continue
            key = next(('%s@%d' % (rel, o) for o, e in arrays if o < p.start() < e),
                       '%s@%d' % (rel, p.start()))
            self.pairs.append((v, p.group(2), key, rel))

    def resolve(self, ident, rel):
        """The decompiled function an identifier names: the one in the same file first."""
        ks = self.by_full.get(norm(ident), [])
        same = [k for k in ks if self.fns[k]['file'] == rel]
        if len(same) == 1:
            return same[0]
        if len(ks) == 1:
            return ks[0]
        return None


# ---------------------------------------------------------------------------------------------
# The client: strings, registrations and calls per row

HEX = re.compile(r'0x[0-9a-f]+')
SRC_PATH = re.compile(r'^[A-Za-z]:\\.*\.(?:cpp|c|h|inl)$', re.I)


def cstring(img, va, cap=400):
    """The NUL-terminated string at va in a data section, printable ASCII with tabs and line
    breaks, at least two characters and one letter; else None."""
    if img.section_of(va) not in ('.rdata', '.data'):
        return None
    o = va - img.base
    e = img.mem.find(b'\0', o, o + cap)
    if e < 0 or e - o < 2:
        return None
    s = bytes(img.mem[o:e])
    if not all(32 <= c < 127 or c in (9, 10, 13) for c in s):
        return None
    if not re.search(rb'[A-Za-z]', s):
        return None
    return s.decode('ascii')


class Client:
    def __init__(self, exe, disasm):
        self.img = Image(exe)
        self.dis = Disasm(disasm)
        self.fns = Functions()
        self.rows = self.fns.rows
        self.strings = defaultdict(set)   # row address -> strings
        self.paths = defaultdict(set)     # row address -> source file basenames
        self.reg_calls = []               # (first string argument, ordinal, row address)
        self.pairs = []                   # (string, row address, table key)
        self.callees = defaultdict(set)
        self.callers = defaultdict(set)
        self._scan_code()
        self._scan_data()
        for r in read_tsv(ROOT / 'tables' / 'calls.tsv'):
            f = int(r['function'], 16)
            for c in r['callees'].split('|'):
                c = int(c, 16)
                self.callees[f].add(c)
                self.callers[c].add(f)

    def _scan_code(self):
        img, dis, at = self.img, self.dis, self.fns.at
        cache = {}

        def string(v):
            if v not in cache:
                cache[v] = cstring(img, v)
            return cache[v]
        for row in self.rows:
            start, end = row['address'], row['address'] + row['size']
            k = dis.at.get(start)
            args = []     # immediates since the last call, closest last
            while k is not None and k < len(dis.ins) and dis.ins[k][0] < end:
                _, _, mn, ops = dis.ins[k]
                k += 1
                vals = [int(h, 16) for h in HEX.findall(ops)]
                if mn == 'call':
                    if len(vals) == 1 and vals[0] in at:
                        self._reg(args)
                    args = []
                    continue
                if mn.startswith('j') or mn.startswith('ret'):
                    args = []
                    continue
                for v in vals:
                    s = string(v)
                    if s is None:
                        continue
                    if SRC_PATH.match(s):
                        self.paths[start].add(s.rsplit('\\', 1)[-1].lower())
                    else:
                        self.strings[start].add(s)
                if len(vals) == 1 and (mn == 'push' or (mn == 'mov' and re.match(
                        r'^(ecx|edx),0x', ops))):
                    reg = ops.split(',')[0] if mn == 'mov' else None
                    args.append((reg, vals[0]))

    def _reg(self, args):
        """A call whose arguments hold a string and a function address: the arguments in order
        are ecx, edx (fastcall) and then the pushes, last pushed first."""
        regs = {r: v for r, v in args if r}
        pushes = [v for r, v in args if not r][::-1]
        ordered = [regs[r] for r in ('ecx', 'edx') if r in regs] + pushes
        first = next((s for s in (cstring(self.img, v) for v in ordered) if s), None)
        if not first:
            return
        fns = [v for v in ordered if v in self.fns.at]
        for n, f in enumerate(fns):
            self.reg_calls.append((first, n, f))

    def _scan_data(self):
        img, at = self.img, self.fns.at
        for sec in ('.rdata', '.data'):
            lo, hi, rsize = img.sections[sec]
            hi = min(hi, lo + rsize)
            last_key, last_o = None, None
            for o in range(lo, hi - 8, 4):
                a, b = struct.unpack_from('<II', img.mem, o - img.base)
                if b in at and img.section_of(a) in ('.rdata', '.data'):
                    s = cstring(img, a)
                    if s and not SRC_PATH.match(s):
                        # a table ends at a null pair or a gap
                        if (last_o is None or o - last_o > 32
                                or img.bytes(last_o + 8, 8) == bytes(8)):
                            last_key = addr(o)
                        last_o = o
                        self.pairs.append((s, b, last_key))


# ---------------------------------------------------------------------------------------------
# Methods

def best(scores):
    """The best candidate, its score and the margin over the next."""
    ranked = sorted(scores.items(), key=lambda kv: -kv[1])
    top, s = ranked[0]
    second = ranked[1][1] if len(ranked) > 1 else 0.0
    return top, s, s - second


def by_string(client, dec):
    holders = defaultdict(set)
    for k, f in enumerate(dec.fns):
        for s in f['strings']:
            holders[s].add(k)
    namers = defaultdict(set)
    for a, ss in client.strings.items():
        for s in ss:
            namers[s].add(a)
    leads = {}
    best_for_d = defaultdict(lambda: (0.0, None))
    raw = {}
    for a, ss in client.strings.items():
        scores, shared = defaultdict(float), defaultdict(list)
        for s in ss:
            hs = holders.get(s)
            if not hs or len(hs) > 40:
                continue
            w = 1.0 / (len(namers[s]) * len(hs))
            for k in hs:
                scores[k] += w
                shared[k].append(s)
        if not scores:
            continue
        k, s, margin = best(scores)
        raw[a] = (k, s, margin, shared[k])
        if s > best_for_d[k][0]:
            best_for_d[k] = (s, a)
    for a, (k, s, margin, shared) in raw.items():
        mutual = best_for_d[k][1] == a
        unique = sum(1 for x in shared if len(namers[x]) == 1 and len(holders[x]) == 1)
        if mutual and unique >= 2 and margin >= 1.0:
            tier = 'A'
        elif mutual and unique >= 1 and margin >= 0.5 and max(len(x) for x in shared) >= 6:
            tier = 'B'
        else:
            tier = 'C'
        leads[a] = dict(k=k, score=s, margin=margin, shared=len(shared), tier=tier,
                        evidence=' | '.join(sorted(shared, key=len, reverse=True)[:3]))
    return leads


def by_register(client, dec):
    """Name tables and registration calls on both sides, keyed by the string."""
    leads = {}
    # tables: pair each client table with the decompiled table sharing most names
    dtables = defaultdict(dict)
    for v, ident, key, rel in dec.pairs:
        k = dec.resolve(ident, rel)
        if k is not None:
            dtables[key][v] = k
    dnames = defaultdict(set)
    for key, m in dtables.items():
        for v in m:
            dnames[v].add(key)
    ctables = defaultdict(dict)
    for s, a, key in client.pairs:
        ctables[key][s] = a
    for key, m in ctables.items():
        overlap = defaultdict(int)
        for v in m:
            for dk in dnames.get(v, ()):
                overlap[dk] += 1
        if not overlap:
            continue
        ranked = sorted(overlap.items(), key=lambda kv: -kv[1])
        dk, n = ranked[0]
        tied = len(ranked) > 1 and ranked[1][1] == n
        for v, a in m.items():
            if v in dtables[dk]:
                k = dtables[dk][v]
                tier = 'A' if n >= 3 and not tied else ('B' if not tied else 'C')
                leads.setdefault(a, dict(k=k, score=float(n), margin=float(
                    n - (ranked[1][1] if len(ranked) > 1 else 0)), shared=n, tier=tier,
                    evidence='table %s: "%s"' % (key, v)))
    # registration calls: the first string argument and the n-th function argument
    dcalls = defaultdict(set)
    for v, idents, rel in dec.reg_calls:
        ks = [k for k in (dec.resolve(i, rel) for i in idents) if k is not None]
        for n, k in enumerate(ks):
            dcalls[(v, n)].add(k)
    ccalls = defaultdict(set)
    for v, n, a in client.reg_calls:
        ccalls[(v, n)].add(a)
    for key, as_ in ccalls.items():
        ks = dcalls.get(key)
        if not ks:
            continue
        tier = 'A' if len(as_) == 1 and len(ks) == 1 and len(key[0]) >= 4 else 'C'
        for a in as_:
            if a in leads:
                continue
            k = sorted(ks)[0]
            leads[a] = dict(k=k, score=1.0 / (len(as_) * len(ks)), margin=0.0, shared=1,
                            tier=tier, evidence='call "%s" arg %d' % key)
    return leads


def neighbour_tokens_client(client, a, names):
    toks = set()
    for c in client.callees.get(a, ()):
        n = names.get(c)
        if n:
            toks.add('c:' + norm(n))
            toks.add('ct:' + tail(n))
    for p in client.callers.get(a, ()):
        n = names.get(p)
        if n:
            toks.add('p:' + norm(n))
            toks.add('pt:' + tail(n))
    return toks


def decomp_tokens(dec):
    toks = [set() for _ in dec.fns]
    for k, f in enumerate(dec.fns):
        cls = f['name'].rsplit('::', 1)[0] if '::' in f['name'] else None
        for c in f['calls']:
            toks[k].add('c:' + norm(c))
            toks[k].add('ct:' + tail(c))
            if cls and '::' not in c:
                toks[k].add('c:' + norm(cls + '::' + c))
        for m in f['methods']:
            toks[k].add('ct:' + tail(m))
        for t in dec.callers[k]:
            toks[k].add('p:' + norm(dec.fns[t]['name']))
            toks[k].add('pt:' + tail(dec.fns[t]['name']))
    return toks


class Calls:
    """The calls method: a row's named neighbours against a decompiled function's."""

    def __init__(self, dec):
        self.dec = dec
        self.dtoks = decomp_tokens(dec)
        self.post = defaultdict(list)
        for k, ts in enumerate(self.dtoks):
            for t in ts:
                self.post[t].append(k)

    def run(self, client, names, rows):
        N = len(self.dec.fns)
        leads, raw = {}, {}
        best_for_d = defaultdict(lambda: (0.0, None))
        for a in rows:
            ts = neighbour_tokens_client(client, a, names)
            if len(ts) < 2:
                continue
            scores, shared = defaultdict(float), defaultdict(list)
            for t in ts:
                ks = self.post.get(t)
                if not ks or len(ks) > 60:
                    continue
                w = math.log(N / len(ks))
                for k in ks:
                    scores[k] += w
                    shared[k].append(t)
            if not scores:
                continue
            k, s, margin = best(scores)
            raw[a] = (k, s, margin, shared[k])
            if s > best_for_d[k][0]:
                best_for_d[k] = (s, a)
        for a, (k, s, margin, shared) in raw.items():
            mutual = best_for_d[k][1] == a
            full = len({t for t in shared if t[:2] in ('c:', 'p:')})
            if mutual and full >= 3 and margin >= 8.0:
                tier = 'A'
            elif mutual and full >= 2 and margin >= 4.0:
                tier = 'B'
            else:
                tier = 'C'
            leads[a] = dict(k=k, score=s, margin=margin, shared=len(shared), tier=tier,
                            evidence=' '.join(sorted(shared)[:6]))
        return leads


def by_align(client, dec, names, anchors):
    """The callee (or caller) left over when a paired row and its decompiled function have had
    every other neighbour matched, by pair or by name, is a lead for the one left on the other
    side."""
    taken = set(anchors.values())
    votes = defaultdict(lambda: defaultdict(list))
    for a, k in anchors.items():
        for mine, theirs, kind in ((client.callees.get(a, set()), dec.callees[k], 'callee'),
                                   (client.callers.get(a, set()), dec.callers[k], 'caller')):
            left_d = set(theirs)
            left_c = []
            for x in mine:
                if x in anchors and anchors[x] in left_d:
                    left_d.discard(anchors[x])
                    continue
                n = names.get(x)
                hit = [t for t in left_d if n and norm(dec.fns[t]['name']) == norm(n)]
                if hit:
                    left_d.discard(hit[0])
                    continue
                if x in anchors or n:
                    continue
                left_c.append(x)
            left_d = {t for t in left_d if t not in taken}
            if len(left_c) == 1 and len(left_d) == 1:
                votes[left_c[0]][next(iter(left_d))].append('%s of %s' % (kind, addr(a)))
    leads = {}
    for x, cand in votes.items():
        ranked = sorted(cand.items(), key=lambda kv: -len(kv[1]))
        k, why = ranked[0]
        rival = len(ranked[1][1]) if len(ranked) > 1 else 0
        if rival or len(why) < 2:
            tier = 'C'
        elif len(why) >= 3:
            tier = 'A'
        else:
            tier = 'B'
        leads[x] = dict(k=k, score=float(len(why)), margin=float(len(why) - rival),
                        shared=len(why), tier=tier, evidence=', '.join(why[:4]))
    return leads


METHODS = ('string', 'register', 'calls', 'align')
ROUNDS = 100   # a guard: each round adds a pair or ends the loop, so it is never reached
READ = ROOT / 'tools' / 'namelead-read.tsv'


def run(client, dec, names, calls):
    """Every method, with the leads at tier A or B fed back to the calls and align methods as
    names and pairs until no new one comes."""
    methods = {'string': by_string(client, dec), 'register': by_register(client, dec)}
    anchors = {}
    for a, n in names.items():
        ks = dec.by_full.get(norm(n), [])
        if len(ks) == 1:
            anchors[a] = ks[0]
    named = dict(names)
    methods['calls'], methods['align'] = {}, {}
    rows = [r['address'] for r in client.rows]
    rnd = 0
    while True:
        rnd += 1
        if rnd > ROUNDS:
            note('namelead: stopped after %d rounds with pairs still coming' % ROUNDS)
            break
        new = 0
        for m in METHODS:
            for a, ld in methods[m].items():
                f = dec.fns[ld['k']]
                if a in anchors or ld['tier'] not in 'AB' or f['placeholder']:
                    continue
                anchors[a] = ld['k']
                if a not in named:
                    named[a] = f['name']
                new += 1
        if rnd > 1 and not new:
            break
        for m, leads in (('calls', calls.run(client, named, rows)),
                         ('align', by_align(client, dec, named, anchors))):
            for a, ld in leads.items():
                ld['evidence'] = 'round %d: %s' % (rnd, ld['evidence'])
                old = methods[m].get(a)
                if old is None or ld['tier'] <= old['tier']:
                    methods[m][a] = ld
    return methods


def file_check(client, dec, a, k):
    paths = client.paths.get(a)
    if not paths:
        return ''
    base = Path(dec.fns[k]['file']).name.lower()
    return 'match' if base in paths else 'differ'


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('decomp', help='a local checkout of the decompilation')
    ap.add_argument('--exe', default=str(ROOT / 'WoW' / 'WoW.exe'))
    ap.add_argument('--disasm', default=str(ROOT / 'dumps' / 'disasm.txt'))
    ap.add_argument('-o', '--out', default=str(ROOT / 'dumps' / 'namelead.tsv'))
    ap.add_argument('--measure', action='store_true',
                    help='print the agreement of each method with the names the rows have')
    args = ap.parse_args()
    if not Path(args.out).resolve().is_relative_to((ROOT / 'dumps').resolve()):
        sys.exit('namelead: the leads go under dumps/, which git ignores')
    if Path(args.decomp).resolve().is_relative_to(ROOT):
        sys.exit('namelead: the decompilation is read from a checkout outside the repo')

    dec = Decomp(args.decomp)
    client = Client(args.exe, args.disasm)
    names = {r['address']: r['name'] for r in client.rows if r['name']}
    note('client: %d rows, %d with strings, %d name-table pairs, %d registration calls'
         % (len(client.rows), len(client.strings), len(client.pairs), len(client.reg_calls)))

    calls = Calls(dec)
    out, _ = table(run(client, dec, names, calls), client, dec)
    write(args.out, out)
    coverage(out)
    if not args.measure:
        return
    # The measure: a fifth of the named rows lose their names, the methods run again, and the
    # leads for those rows are checked against the names they had.
    # A row named from a lead would confirm its own lead, so the rows read from leads are left out.
    read = {int(r['address'], 16) for r in read_tsv(READ)}
    hidden = {a for a in names if zlib.crc32(addr(a).encode()) % 5 == 0 and a not in read}
    visible = {a: n for a, n in names.items() if a not in hidden}
    h_out, h_ks = table(run(client, dec, visible, calls), client, dec)
    hid = {addr(a) for a in hidden}
    write(Path(args.out).with_name(Path(args.out).stem + '-measure.tsv'),
          [r for r in h_out if r[0] in hid])
    measure(h_out, h_ks, client, dec, hid, {addr(a) for a in read})


HEADER = ['address', 'name', 'method', 'tier', 'score', 'margin', 'shared', 'lead', 'lead_file',
          'lead_address', 'file', 'evidence']


def write(path, out):
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    with open(path, 'w', encoding='utf-8', newline='\n') as fh:
        fh.write('\t'.join(HEADER) + '\n')
        for r in out:
            fh.write('\t'.join(r) + '\n')
    note('wrote %d leads to %s' % (len(out), path))


def table(methods, client, dec):
    """The leads as output rows, with the decompiled function each names."""
    out, ks = [], []
    for m, leads in methods.items():
        if not leads:
            sys.exit('namelead: method %s found no leads; a side was not read' % m)
        for a, ld in sorted(leads.items()):
            f = dec.fns[ld['k']]
            if f['placeholder']:
                continue
            row = client.fns.at[a]
            out.append([addr(a), row['name'], m, ld['tier'], '%.2f' % ld['score'],
                        '%.2f' % ld['margin'], str(ld['shared']), f['name'], f['file'],
                        f['offset'], file_check(client, dec, a, ld['k']),
                        re.sub(r'[\t\n\r]', ' ', ld['evidence'])[:160]])
            ks.append(ld['k'])
    order = sorted(range(len(out)), key=lambda i: (out[i][0], out[i][2]))
    return [out[i] for i in order], [ks[i] for i in order]


def known_pairs(client, dec, read):
    """The rows whose own name, normalized, is the name of exactly one decompiled function: the
    pairs a method's lead can be checked against whatever the naming style. The rows named from a
    lead (`read`) are not among them."""
    out = {}
    for r in client.rows:
        if (r['name'] and not MECHANICAL.search(r['name'])
                and addr(r['address']) not in read):
            ks = dec.by_full.get(norm(r['name']), [])
            if len(ks) == 1:
                out[addr(r['address'])] = ks[0]
    return out




def coverage(out):
    unnamed = defaultdict(lambda: defaultdict(set))
    for r in out:
        if not r[1]:
            unnamed[r[2]][r[3]].add(r[0])
            unnamed['any'][r[3]].add(r[0])
    print('unnamed rows with a lead (each row at its best tier for the column):')
    for m in METHODS + ('any',):
        seen, cells = set(), []
        for t in 'ABC':
            got = unnamed[m][t] - seen
            seen |= got
            cells.append('%s %5d' % (t, len(got)))
        print('  %-9s %s' % (m, '  '.join(cells)))


def measure(out, ks, client, dec, hidden, read):
    known = known_pairs(client, dec, read)
    stats = defaultdict(lambda: defaultdict(int))
    for r, k in zip(out, ks):
        a, name, m, tier = r[0], r[1], r[2], r[3]
        if a not in hidden or MECHANICAL.search(name):
            continue
        for key in ((m, tier), ('any', tier)):
            s = stats[key]
            s['n'] += 1
            s[agreement(r[7], name)] += 1
            if a in known:
                s['kn'] += 1
                s['kok'] += known[a] == k
    print('measured on %d named rows run as unnamed, against their names (full, last part,'
          % len(hidden))
    print('differ) and, for the %d rows whose name is one decompiled function\'s, against that'
          ' function:' % len(known))
    for key in sorted(stats, key=lambda k: ((METHODS + ('any',)).index(k[0]), k[1])):
        s = stats[key]
        print('  %-9s %s  n %5d  full %5d  tail %5d  differ %5d  names %.3f  |  '
              'known %5d  right %5d  precision %.3f'
              % (key[0], key[1], s['n'], s['full'], s['tail'], s['differ'],
                 (s['full'] + s['tail']) / s['n'], s['kn'], s['kok'],
                 s['kok'] / s['kn'] if s['kn'] else float('nan')))


if __name__ == '__main__':
    main()
