"""Attrib::StringHash64 (lookup8 with the executable's seed), as
skate2_formats::attrib::string_hash64.

    python attrib_hash.py Name Location ChallengeType
    python attrib_hash.py --find 0x8022ba64cb8e431d Radius SignUpRadius ...
"""
import sys

M = (1 << 64) - 1
SEED = 0xABCDEF0011223344


def _mix64(a, b, c):
    a = ((a - b - c) & M) ^ (c >> 43)
    b = ((b - c - a) & M) ^ ((a << 9) & M)
    c = ((c - a - b) & M) ^ (b >> 8)
    a = ((a - b - c) & M) ^ (c >> 38)
    b = ((b - c - a) & M) ^ ((a << 23) & M)
    c = ((c - a - b) & M) ^ (b >> 5)
    a = ((a - b - c) & M) ^ (c >> 35)
    b = ((b - c - a) & M) ^ ((a << 49) & M)
    c = ((c - a - b) & M) ^ (b >> 11)
    a = ((a - b - c) & M) ^ (c >> 12)
    b = ((b - c - a) & M) ^ ((a << 18) & M)
    c = ((c - a - b) & M) ^ (b >> 22)
    return a, b, c


def hash64(text, seed=SEED):
    key = text.encode()
    if not key:
        return 0
    n = len(key)
    a = b = seed
    c = 0x9E3779B97F4A7C13
    while len(key) >= 24:
        a = (a + int.from_bytes(key[:8], 'little')) & M
        b = (b + int.from_bytes(key[8:16], 'little')) & M
        c = (c + int.from_bytes(key[16:24], 'little')) & M
        a, b, c = _mix64(a, b, c)
        key = key[24:]
    c = (c + n) & M
    for i, byte in enumerate(key):
        if i <= 7:
            a = (a + (byte << (i * 8))) & M
        elif i <= 15:
            b = (b + (byte << ((i - 8) * 8))) & M
        else:
            c = (c + (byte << ((i - 15) * 8))) & M
    return _mix64(a, b, c)[2]


assert hash64('Name') == 0x942AB8AEE4B414ED

if __name__ == '__main__':
    args = sys.argv[1:]
    if args[:1] == ['--find']:
        target = int(args[1], 16)
        hits = [name for name in args[2:] if hash64(name) == target]
        print(hits or 'no candidate matches')
    else:
        for name in args:
            print('0x%016x %s' % (hash64(name), name))
