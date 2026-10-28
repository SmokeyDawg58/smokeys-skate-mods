"""Recover exact animation-attribute string keys from their PDB-named initializers."""

from __future__ import annotations

import json
import re
import traceback
from pathlib import Path

import ida_auto
import ida_bytes
import ida_funcs
import ida_pro
import idautils


PROJECT = Path(__file__).resolve().parents[2]
OUTPUT = PROJECT / "artifacts" / "physics-animation-attribute-keys.json"
INITIALIZER = re.compile(r"dynamic initializer for '([^']+)'")
CONTACT_AND_JUMP_KEYS = {
    "kPushContactName",
    "kBrakeContactName",
    "kRightHandGrabName",
    "kLeftHandGrabName",
    "kRightHandName",
    "kJumpName",
}


def referenced_strings(function_start: int) -> list[dict[str, object]]:
    function = ida_funcs.get_func(function_start)
    if function is None:
        return []
    values: dict[int, str] = {}
    for instruction in idautils.FuncItems(function_start):
        for target in idautils.DataRefsFrom(instruction):
            raw = ida_bytes.get_strlit_contents(target, -1, 0)
            if raw:
                values[target] = raw.decode("utf-8", errors="replace")
    return [
        {"address": address, "value": value}
        for address, value in sorted(values.items())
    ]


def main() -> None:
    ida_auto.auto_wait()
    records = []
    for address in idautils.Functions():
        name = ida_funcs.get_func_name(address)
        match = INITIALIZER.search(name)
        if not match or not (
            match.group(1).endswith("Att") or match.group(1) in CONTACT_AND_JUMP_KEYS
        ):
            continue
        strings = referenced_strings(address)
        if strings:
            records.append(
                {
                    "symbol": match.group(1),
                    "initializer": address,
                    "strings": strings,
                }
            )
    records.sort(key=lambda record: str(record["symbol"]).casefold())
    OUTPUT.write_text(json.dumps(records, indent=2), encoding="utf-8")
    print(f"dumped {len(records)} animation attribute initializers to {OUTPUT}")


exit_code = 0
try:
    main()
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
