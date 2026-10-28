"""Symbolic scalar-FP run of a straight-line PPC function: every FP register
holds an expression tree, so each store shows its exact (fused) operation
order. Memory reads from argument pointers become named leaves.

Usage: python vmx128_symbolic_fp.py <bytes.txt>, where the file holds the
function bytes as IDA `get_bytes` prints them (the __main__ block is set up
for ICE::MulMatrix 0x82a33778: r3 dst, r4 a, r5 b)."""
import sys


def bits(w, lo, n):
    return (w >> lo) & ((1 << n) - 1)


def simm16(w):
    v = w & 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


def run(code_bytes, base, regs, names):
    """regs: initial GPR values (symbolic base tags as (tag, offset));
    names: tag -> function(offset) -> leaf name."""
    r = dict(regs)
    f = {}
    mem = {}
    stores = []
    words = [int.from_bytes(code_bytes[i:i + 4], 'big') for i in range(0, len(code_bytes), 4)]
    for i, w in enumerate(words):
        addr = base + 4 * i
        op = w >> 26
        d, a, b, c = bits(w, 21, 5), bits(w, 16, 5), bits(w, 11, 5), bits(w, 6, 5)
        if op in (48, 52):  # lfs / stfs
            tag, off = r[a]
            ea = (tag, off + simm16(w))
            if op == 48:
                f[d] = mem.get(ea) or names[tag](ea[1])
            else:
                mem[ea] = f[d]
                stores.append((ea, f[d]))
            continue
        if op == 59:
            xo = bits(w, 1, 5)
            if xo == 25:
                f[d] = ('*', f[a], f[c])
            elif xo == 29:
                f[d] = ('fma', f[a], f[c], f[b])
            elif xo == 28:
                f[d] = ('fms', f[a], f[c], f[b])
            elif xo == 21:
                f[d] = ('+', f[a], f[b])
            elif xo == 20:
                f[d] = ('-', f[a], f[b])
            elif xo == 18:
                f[d] = ('/', f[a], f[b])
            else:
                raise SystemExit(f'{addr:08x}: op59 xo {xo}')
            continue
        if op == 63:
            xo = bits(w, 1, 10)
            if xo == 72:
                f[d] = f[b]
            elif xo == 40:
                f[d] = ('neg', f[b])
            else:
                raise SystemExit(f'{addr:08x}: op63 xo {xo}')
            continue
        if op == 14 and a == 1:  # addi rX, r1, k: stack pointer arithmetic
            r[d] = ('sp', r[1][1] + simm16(w))
            continue
        # prologue/epilogue/branches and other integer ops are ignored
    return stores


def show(e):
    if isinstance(e, str):
        return e
    op = e[0]
    if op == '*':
        return f'({show(e[1])}*{show(e[2])})'
    if op == 'fma':
        return f'fma({show(e[1])},{show(e[2])},{show(e[3])})'
    if op == 'fms':
        return f'fms({show(e[1])},{show(e[2])},{show(e[3])})'
    if op in '+-/':
        return f'({show(e[1])}{op}{show(e[2])})'
    if op == 'neg':
        return f'-{show(e[1])}'
    return str(e)


if __name__ == '__main__':
    data = bytes(int(x, 16) for x in open(sys.argv[1]).read().split())
    lane = 'xyzw'
    names = {
        'a': lambda off: f'a{off // 16}{lane[(off % 16) // 4]}',
        'b': lambda off: f'b{off // 16}{lane[(off % 16) // 4]}',
        'sp': lambda off: f'stack{off}',
    }
    stores = run(data, 0x82a33778, {1: ('sp', 0), 3: ('dst', 0), 4: ('a', 0), 5: ('b', 0)}, names)
    for (tag, off), e in stores:
        if tag == 'dst':
            print(f'dst{off // 16}{lane[(off % 16) // 4]} = {show(e)}')
