"""Lists each chunk's handler name and its native calls with constant args.

Symbolic register tracking within a function: GETGLOBAL/GETTABLE(const key)/
LOADK/LOADBOOL/MOVE feed a CALL's callee and arguments. Branches are ignored,
so this is a reading aid, not a decompiler.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from luadump import chunks, decode, OPS


def fmt(v):
    if isinstance(v, float):
        return ('%d' % v) if v == int(v) else repr(v)
    if isinstance(v, str):
        return repr(v)
    return str(v)


def calls(f):
    reg = {}
    out = []
    for pc, ins in enumerate(f.code):
        op, a, b, c, bx, sbx = decode(ins)
        name = OPS[op]
        if name == 'GETGLOBAL':
            reg[a] = f.consts[bx]
        elif name == 'GETTABLE':
            base = reg.get(b)
            key = f.consts[c - 256] if c >= 256 else reg.get(c, '?')
            reg[a] = '%s.%s' % (base, key) if base is not None else '?.%s' % (key,)
        elif name == 'LOADK':
            reg[a] = fmt(f.consts[bx])
        elif name == 'LOADBOOL':
            reg[a] = 'true' if b else 'false'
        elif name == 'LOADNIL':
            for r in range(a, b + 1):
                reg[r] = 'nil'
        elif name == 'MOVE':
            reg[a] = reg.get(b, 'r%d' % b)
        elif name == 'NEWTABLE':
            reg[a] = '{}'
        elif name == 'CALL':
            nargs = (b - 1) if b else None
            args = [str(reg.get(a + 1 + i, '?')) for i in range(nargs or 0)]
            out.append('%s(%s)' % (reg.get(a, '?'), ', '.join(args)))
            reg[a] = 'ret(%s)' % reg.get(a, '?')
        elif name in ('EQ', 'LT', 'LE'):
            lhs = f.consts[b - 256] if b >= 256 else reg.get(b, 'r%d' % b)
            rhs = f.consts[c - 256] if c >= 256 else reg.get(c, 'r%d' % c)
            out.append('  if %s %s %s' % (lhs, {'EQ': '==', 'LT': '<', 'LE': '<='}[name], fmt(rhs)))
    return out


if __name__ == '__main__':
    data = open(sys.argv[1], 'rb').read()
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 10 ** 9
    for n, (start, end, main) in enumerate(chunks(data)):
        if n >= limit:
            break
        handler = None
        for ins in main.code:
            op, a, b, c, bx, sbx = decode(ins)
            if OPS[op] == 'SETGLOBAL':
                handler = main.consts[bx]
        print('@%x %s' % (start, handler))
        for p in main.protos:
            for line in calls(p):
                print('    ' + line)
