#!/usr/bin/env python3
"""Recover local build inputs from an unmodified Smokey Update 42 game EXE.

Nothing extracted by this helper belongs in the Git repository. The source
bytes remain in the copy of the game the player already has.
"""
from __future__ import annotations

import argparse
import hashlib
from pathlib import Path

EXE_HASH = "16de0392bda3e1444f39f21f0e51f679a61f580bd7c29c2722d80d378ab8ba60"
# Exact Update 42 / Skate 2 0.2.9 binary offsets, lengths, and SHA-256 checks.
ASSETS = (
    ("skate3-player.pack", 56184464, 26313708, "e371fad65cfa0e5a2638e821d46f47b8b6bbfaf01ec4db8b772c45de0b78380a"),
    ("skate3-darkcatch.bnk", 51184004, 53970, "014b08e617c57c002c8bea0e41a3e44d2c1063a967ae8d929fe9c5293dd74e6e"),
    ("skate3-landing-notifier.bin", 46959998, 143324, "3bd41540838783d3e7fd36a11881875e9077bb4091f675e76a6265cd206accf4"),
    ("pool-ground.bin", 45521520, 149268, "c458289cb9d92628ab8121091adaad77e49eadf3ce5dd12d7cdb710b0f672f4c"),
)


def digest_file(path: Path) -> str:
    sha = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            sha.update(chunk)
    return sha.hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path, help="Original Update 42 skate2-pc.exe")
    parser.add_argument("output", type=Path, help="Private build-input folder outside this repository")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[1]
    out = args.output.resolve()
    if out.is_relative_to(repo):
        parser.error("Choose an output folder outside the source repository.")
    if digest_file(args.executable) != EXE_HASH:
        parser.error("That is not the verified Update 42 executable. Use the matching release EXE.")
    # Verify every slice before creating any files, then leave existing user
    # files alone unless they are already identical.
    slices = []
    with args.executable.open("rb") as executable:
        for name, offset, size, expected in ASSETS:
            executable.seek(offset)
            data = executable.read(size)
            if len(data) != size or hashlib.sha256(data).hexdigest() != expected:
                raise ValueError(f"Invalid embedded input: {name}")
            slices.append((name, data, expected))
    if out.exists():
        for name, _, expected in slices:
            path = out / name
            if path.exists() and (not path.is_file() or digest_file(path) != expected):
                raise FileExistsError(f"Refusing to overwrite a different file: {path}")
    out.mkdir(parents=True, exist_ok=True)
    for name, data, _ in slices:
        path = out / name
        if not path.exists():
            with path.open("xb") as output:
                output.write(data)
    print(f"Verified four local build inputs in {out}.")


if __name__ == "__main__":
    main()
