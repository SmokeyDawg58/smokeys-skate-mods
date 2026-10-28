"""Tiny Xenon emulator for straight-line VMX128 helper functions.

Feed it the raw bytes of a function (IDA `get_bytes`) and the mnemonic of
each instruction (IDA listing), set GPRs and memory, run, and read the
results back. Validate on a function whose result is known first (as was
done with Scene::ComputeTransform 0x828652f0 before decoding
ICESceneBridge::ICEMatrixFromSkatePointAndNormal 0x826cc6a8).

Mnemonics come from IDA's listing; operands are decoded from the raw words
(xenia's VX128 layouts), since IDA misreports VA high bits. f32 arithmetic is
rounded per operation; fused VMX ops round once. vrsqrtefp is taken as an
exact reciprocal square root.
"""
import math
import struct


def f32(x):
    return struct.unpack('>f', struct.pack('>f', x))[0]


def bits(w, lo, n):
    return (w >> lo) & ((1 << n) - 1)


def vd(w):
    return bits(w, 21, 5) | (bits(w, 2, 2) << 5)


def va(w):
    return bits(w, 16, 5) | (bits(w, 5, 1) << 5) | (bits(w, 10, 1) << 6)


def vb(w):
    return bits(w, 11, 5) | (bits(w, 0, 2) << 5)


def simm16(w):
    v = w & 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


class Machine:
    def __init__(self):
        self.mem = {}
        self.r = [0] * 32
        self.f = [0.0] * 32
        self.v = [bytes(16)] * 128

    def rd(self, addr, n):
        return bytes(self.mem.get(addr + i, 0) for i in range(n))

    def wr(self, addr, data):
        for i, b in enumerate(data):
            self.mem[addr + i] = b

    def rf(self, addr):
        return struct.unpack('>f', self.rd(addr, 4))[0]

    def wf(self, addr, x):
        self.wr(addr, struct.pack('>f', f32(x)))

    def vec(self, i):
        return list(struct.unpack('>4f', self.v[i]))

    def setvec(self, i, xs):
        self.v[i] = struct.pack('>4f', *[f32(x) for x in xs])

    def setvec_bytes(self, i, b):
        self.v[i] = bytes(b)

    def run(self, code, listing, stop_mnems=('b',)):
        for addr in sorted(code):
            w = code[addr]
            text = listing.get(addr, '')
            mn = text.split()[0] if text else ''
            op = w >> 26
            rD = bits(w, 21, 5)
            rA = bits(w, 16, 5)
            if mn in ('mflr', 'bl', 'stfd', 'lfd', 'mtlr', 'blr', 'stwu'):
                continue
            if mn == 'b':
                break
            if op == 15:  # addis / lis
                self.r[rD] = ((self.r[rA] if rA else 0) + (simm16(w) << 16)) & 0xFFFFFFFF
            elif op == 14:  # addi / li
                self.r[rD] = ((self.r[rA] if rA else 0) + simm16(w)) & 0xFFFFFFFF
            elif op == 48:  # lfs
                self.f[rD] = self.rf((self.r[rA] + simm16(w)) & 0xFFFFFFFF)
            elif op == 52:  # stfs
                self.wf((self.r[rA] + simm16(w)) & 0xFFFFFFFF, self.f[rD])
            elif op == 36:  # stw
                self.wr((self.r[rA] + simm16(w)) & 0xFFFFFFFF, struct.pack('>I', self.r[rD]))
            elif mn == 'fmr':
                self.f[rD] = self.f[bits(w, 11, 5)]
            elif mn in ('lvx128', 'stvx128', 'lvlx128'):
                ea = ((self.r[rA] if rA else 0) + self.r[bits(w, 11, 5)]) & 0xFFFFFFFF
                d = vd(w)
                if mn == 'lvx128':
                    self.v[d] = self.rd(ea & ~15, 16)
                elif mn == 'stvx128':
                    self.wr(ea & ~15, self.v[d])
                else:
                    n = 16 - (ea & 15)
                    self.v[d] = self.rd(ea, n) + bytes(16 - n)
            elif mn == 'vspltisw128':
                s = bits(w, 16, 5)
                s = s - 32 if s & 16 else s
                self.v[vd(w)] = struct.pack('>4i', s, s, s, s)
            elif mn == 'vcsxwfp128':
                ints = struct.unpack('>4i', self.v[vb(w)])
                scale = 2.0 ** -bits(w, 16, 5)
                self.setvec(vd(w), [i * scale for i in ints])
            elif mn == 'vspltw128':
                k = bits(w, 16, 2)
                word = self.v[vb(w)][4 * k:4 * k + 4]
                self.v[vd(w)] = word * 4
            elif mn == 'vpermwi128':
                perm = bits(w, 16, 5) | (bits(w, 6, 3) << 5)
                src = self.v[vb(w)]
                out = b''
                for i in range(4):
                    k = (perm >> (6 - 2 * i)) & 3
                    out += src[4 * k:4 * k + 4]
                self.v[vd(w)] = out
            elif mn == 'vperm128':
                src = self.v[va(w)] + self.v[vb(w)]
                ctl = self.v[bits(w, 6, 3)]
                self.v[vd(w)] = bytes(src[c & 31] for c in ctl)
            elif mn == 'vsldoi128':
                src = self.v[va(w)] + self.v[vb(w)]
                sh = bits(w, 6, 4)
                self.v[vd(w)] = src[sh:sh + 16]
            elif mn == 'vor128':
                a, b = self.v[va(w)], self.v[vb(w)]
                self.v[vd(w)] = bytes(x | y for x, y in zip(a, b))
            elif mn in ('vsubfp128', 'vaddfp128', 'vmulfp128'):
                a, b = self.vec(va(w)), self.vec(vb(w))
                f = {'vsubfp128': lambda x, y: x - y, 'vaddfp128': lambda x, y: x + y,
                     'vmulfp128': lambda x, y: x * y}[mn]
                self.setvec(vd(w), [f(x, y) for x, y in zip(a, b)])
            elif mn == 'vmsum3fp128':
                a, b = self.vec(va(w)), self.vec(vb(w))
                d = f32(f32(f32(a[0] * b[0]) + f32(a[1] * b[1])) + f32(a[2] * b[2]))
                self.setvec(vd(w), [d] * 4)
            elif mn == 'vrsqrtefp128':
                self.setvec(vd(w), [1.0 / math.sqrt(x) if x > 0 else float('inf') for x in self.vec(vb(w))])
            elif mn in ('vmaddfp', 'vnmsubfp'):
                A = self.vec(bits(w, 16, 5))
                B = self.vec(bits(w, 11, 5))
                C = self.vec(bits(w, 6, 5))
                if mn == 'vmaddfp':
                    self.setvec(rD, [a * c + b for a, b, c in zip(A, B, C)])
                else:
                    self.setvec(rD, [-(a * c - b) for a, b, c in zip(A, B, C)])
            else:
                raise SystemExit(f'unhandled {addr:08x} {w:08x} {text}')


def load_listing(path):
    listing = {}
    for line in open(path):
        a, _, text = line.partition(' ')
        if a.strip():
            listing[int(a, 16)] = text.strip()
    return listing


def load_code(base, path):
    data = bytes(int(x, 16) for x in open(path).read().split())
    return {base + i: int.from_bytes(data[i:i + 4], 'big') for i in range(0, len(data), 4)}
