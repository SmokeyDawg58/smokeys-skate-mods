"""Match recovered 64-bit attribute hashes against IDA's string catalogue.

This is a research aid, not runtime code.  It reproduces the game's
``Attrib::StringHash64`` function so a hash observed at an attribute lookup can
be tested against every string discovered in the executable.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


SEED = 0xABCD_EF00_1122_3344
MASK = (1 << 64) - 1


def _mix(a: int, b: int, c: int) -> tuple[int, int, int]:
    a = ((a - b - c) & MASK) ^ (c >> 43)
    b = ((b - c - a) & MASK) ^ ((a << 9) & MASK)
    c = ((c - a - b) & MASK) ^ (b >> 8)
    a = ((a - b - c) & MASK) ^ (c >> 38)
    b = ((b - c - a) & MASK) ^ ((a << 23) & MASK)
    c = ((c - a - b) & MASK) ^ (b >> 5)
    a = ((a - b - c) & MASK) ^ (c >> 35)
    b = ((b - c - a) & MASK) ^ ((a << 49) & MASK)
    c = ((c - a - b) & MASK) ^ (b >> 11)
    a = ((a - b - c) & MASK) ^ (c >> 12)
    b = ((b - c - a) & MASK) ^ ((a << 18) & MASK)
    c = ((c - a - b) & MASK) ^ (b >> 22)
    return a & MASK, b & MASK, c & MASK


def string_hash64(value: str) -> int:
    data = value.encode("utf-8")
    if not data:
        return 0
    original_length = len(data)
    a = b = SEED
    c = 0x9E37_79B9_7F4A_7C13
    while len(data) >= 24:
        a = (a + int.from_bytes(data[0:8], "little")) & MASK
        b = (b + int.from_bytes(data[8:16], "little")) & MASK
        c = (c + int.from_bytes(data[16:24], "little")) & MASK
        a, b, c = _mix(a, b, c)
        data = data[24:]
    c = (c + original_length) & MASK
    for index, byte in enumerate(data):
        if index < 8:
            a = (a + (byte << (index * 8))) & MASK
        elif index < 16:
            b = (b + (byte << ((index - 8) * 8))) & MASK
        else:
            c = (c + (byte << ((index - 15) * 8))) & MASK
    return _mix(a, b, c)[2]


def parse_hash(value: str) -> int:
    return int(value.replace("_", ""), 16)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("catalogue", type=Path)
    parser.add_argument("hashes", nargs="+", type=parse_hash)
    args = parser.parse_args()
    targets = set(args.hashes)
    matches: dict[int, list[tuple[int, str]]] = {target: [] for target in targets}
    with args.catalogue.open(encoding="utf-8") as catalogue:
        for line in catalogue:
            item = json.loads(line)
            value = item["value"]
            hashed = string_hash64(value)
            if hashed in targets:
                matches[hashed].append((item["address"], value))
    for target in args.hashes:
        found = matches[target]
        if not found:
            print(f"{target:016X}: no exact string match")
        for address, value in found:
            print(f"{target:016X}: 0x{address:08X} {value!r}")


if __name__ == "__main__":
    main()
