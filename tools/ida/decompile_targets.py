"""Decompile selected functions from the database currently open in IDA.

Addresses are read from `artifacts/ida-targets.txt`, one hexadecimal address per
line. Derived pseudocode remains below the ignored artifacts directory.
"""

from __future__ import annotations

import json
import traceback
from pathlib import Path

import ida_funcs
import ida_hexrays
import ida_idaapi
import ida_name
import ida_pro
import idautils


PROJECT = Path(__file__).resolve().parents[2]
TARGETS = PROJECT / "artifacts" / "ida-targets.txt"
OUTPUT = PROJECT / "artifacts" / "decomp"


def symbol_at(address: int) -> dict[str, object]:
    function = ida_funcs.get_func(address)
    start = function.start_ea if function else address
    name = ida_funcs.get_func_name(start) or ida_name.get_name(address)
    return {
        "address": address,
        "function_start": start,
        "name": name,
        "demangled": ida_name.demangle_name(name, ida_name.MNG_LONG_FORM) if name else None,
    }


def direct_callees(function: ida_funcs.func_t) -> list[dict[str, object]]:
    seen = set()
    result = []
    for instruction in idautils.FuncItems(function.start_ea):
        for target in idautils.CodeRefsFrom(instruction, False):
            target_function = ida_funcs.get_func(target)
            key = target_function.start_ea if target_function else target
            if key not in seen:
                seen.add(key)
                result.append(symbol_at(target))
    return result


def direct_callers(function: ida_funcs.func_t) -> list[dict[str, object]]:
    seen = set()
    result = []
    for reference in idautils.CodeRefsTo(function.start_ea, False):
        caller = ida_funcs.get_func(reference)
        key = caller.start_ea if caller else reference
        if key not in seen:
            seen.add(key)
            result.append(symbol_at(reference))
    return result


def safe_filename(address: int, name: str) -> str:
    cleaned = "".join(character if character.isalnum() else "_" for character in name)
    return f"{address:08x}_{cleaned[:100]}"


def main() -> None:
    previous_timeout = ida_idaapi.set_script_timeout(0)
    OUTPUT.mkdir(parents=True, exist_ok=True)
    try:
        if not ida_hexrays.init_hexrays_plugin():
            raise RuntimeError("PowerPC decompiler is unavailable")
        addresses = [
            int(line.strip(), 0)
            for line in TARGETS.read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.lstrip().startswith("#")
        ]
        index = []
        for address in addresses:
            function = ida_funcs.get_func(address)
            if function is None:
                index.append(
                    {
                        "address": address,
                        "decompilation_error": "no function contains target address",
                    }
                )
                continue
            identity = symbol_at(function.start_ea)
            stem = safe_filename(function.start_ea, str(identity["name"]))
            record = {
                **identity,
                "end": function.end_ea,
                "size": function.end_ea - function.start_ea,
                "callers": direct_callers(function),
                "callees": direct_callees(function),
            }
            try:
                pseudocode = str(ida_hexrays.decompile(function.start_ea))
                (OUTPUT / f"{stem}.cpp").write_text(pseudocode + "\n", encoding="utf-8")
                record["pseudocode"] = f"{stem}.cpp"
            except ida_hexrays.DecompilationFailure as error:
                record["decompilation_error"] = str(error)
            (OUTPUT / f"{stem}.json").write_text(
                json.dumps(record, indent=2, sort_keys=True), encoding="utf-8"
            )
            index.append(record)
        (OUTPUT / "index.json").write_text(
            json.dumps(index, indent=2, sort_keys=True), encoding="utf-8"
        )
        (OUTPUT / "state.txt").write_text("complete\n", encoding="utf-8")
        print(f"Skate 2 target decompilation complete: {len(index)} functions")
    except Exception:
        error = traceback.format_exc()
        (OUTPUT / "state.txt").write_text("failed\n" + error, encoding="utf-8")
        print(error)
        raise
    finally:
        ida_idaapi.set_script_timeout(previous_timeout)


if __name__ == "__main__":
    exit_code = 0
    try:
        main()
    except Exception:
        exit_code = 1
        raise
    finally:
        ida_pro.qexit(exit_code)
