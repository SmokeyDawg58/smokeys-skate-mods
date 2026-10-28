"""Dump animation database paths from the analyzed Skate 2 executable."""

from __future__ import annotations

import json
import traceback
from pathlib import Path

import ida_auto
import ida_pro
import idautils


PROJECT = Path(__file__).resolve().parents[2]
OUTPUT = PROJECT / "artifacts" / "animation-paths.json"


def main() -> None:
    ida_auto.auto_wait()
    found: list[dict[str, object]] = []
    strings = idautils.Strings()
    strings.setup(strtypes=[0, 1], ignore_instructions=True)
    for item in strings:
        value = str(item)
        if ".abin" in value.casefold():
            found.append({"address": int(item.ea), "value": value})
    OUTPUT.write_text(json.dumps(found, indent=2), encoding="utf-8")
    print(f"dumped {len(found)} animation paths to {OUTPUT}")


exit_code = 0
try:
    main()
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
