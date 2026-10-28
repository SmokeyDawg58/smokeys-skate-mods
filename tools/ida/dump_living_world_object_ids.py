"""Dump named living-world registration and object-ID lifecycle functions."""

from __future__ import annotations

import traceback
from pathlib import Path

import ida_auto
import ida_bytes
import ida_funcs
import ida_hexrays
import ida_lines
import ida_name
import ida_pro
import idautils


OUTPUT = Path(__file__).resolve().parents[2] / "artifacts" / "living-world-object-ids.txt"
TERMS = (
    "RegisterObject",
    "RegisterEntity",
    "LWEntityFactory<Sk8::DynamicObject",
    "LWEntityFactory_VDynamicObject",
    "LWEntityCensusMan<Sk8::DynamicObject",
    "LWEntityCensusMan_VDynamicObject",
    "LWDynamicObjectCensusMan",
    "??0LWDynamicObjectMan",
    "CreateEntityMan@LWDynamicObjectMan",
    "CreateEntityFactory@LWDynamicObjectMan",
    "LWDynamicObjectFactory",
    "??0LWBasicEntity",
    "NextEntityID",
    "AddObjectToPriorityIndexTracking",
    "OnSpawnComplete@DynamicObject",
    "Spawn@LWDynamicObjectCensusMan",
    "SpawnDynamicObject@CensusMan",
    "RequestCull@LWBasicEntity",
    "OnCullRequested@DynamicObject",
    "cMsgRequestCull",
)


def clean_disassembly(address: int) -> str:
    rendered = ida_lines.generate_disasm_line(
        address, ida_lines.GENDSM_FORCE_CODE
    ) or ""
    return ida_lines.tag_remove(rendered)


def main() -> None:
    ida_auto.auto_wait()
    if not ida_hexrays.init_hexrays_plugin():
        raise RuntimeError("PowerPC decompiler is unavailable")

    selected: dict[int, str] = {}
    matched_names: list[tuple[int, str, str]] = []
    for address, name in idautils.Names():
        demangled = ida_name.demangle_name(name, ida_name.MNG_LONG_FORM) or name
        if any(term in demangled or term in name for term in TERMS):
            matched_names.append((address, name, demangled))
            function = ida_funcs.get_func(address)
            if function is not None:
                selected[function.start_ea] = ida_name.get_name(function.start_ea)
    for reference in idautils.CodeRefsTo(0x82D38228, False):
        function = ida_funcs.get_func(reference)
        if function is not None:
            selected[function.start_ea] = ida_name.get_name(function.start_ea)
    # Follow registrations of the request-cull Hermes handler template back to
    # their owning manager functions.
    for reference in idautils.CodeRefsTo(0x828E4B90, False):
        function = ida_funcs.get_func(reference)
        if function is not None:
            selected[function.start_ea] = ida_name.get_name(function.start_ea)
    # Store initialization callers reveal the block size/count assigned to
    # each living-world entity type.
    for reference in idautils.CodeRefsTo(0x82E4FA38, False):
        function = ida_funcs.get_func(reference)
        if function is not None:
            selected[function.start_ea] = ida_name.get_name(function.start_ea)
    for caller_address in (
        0x82D37778,
        0x82D3E308,
        0x8271A5E8,
        0x82E67B10,
    ):
        caller = ida_funcs.get_func(caller_address)
        if caller is None:
            continue
        for instruction in idautils.FuncItems(caller.start_ea):
            for target in idautils.CodeRefsFrom(instruction, False):
                function = ida_funcs.get_func(target)
                if function is not None:
                    selected[function.start_ea] = ida_name.get_name(function.start_ea)
    # The generic factory bodies are identical-code-folded under other
    # specializations; the dynamic-object constructor is included explicitly.
    for target in (
        0x82E66BB8,
        0x82D24B90,
        0x82D3DC58,
        0x82D35FC8,
        0x82D2DFF0,
        0x82D2BBA8,
        0x82D2BC28,
        0x82E5E9C0,
        0x82E605A8,
        0x8271A5E8,
        0x826F5890,
        0x82E67B10,
        0x82D36EB0,
        0x82D36F30,
        0x82A6DD98,
        0x82A6DE90,
        0x82A6DF20,
        0x82A6E450,
        0x82E4FA38,
    ):
        function = ida_funcs.get_func(target)
        if function is not None:
            selected[function.start_ea] = ida_name.get_name(function.start_ea)

    lines = ["MATCHED NAMES"]
    for address, name, demangled in matched_names:
        words = [
            (f"{word:08x}", ida_name.get_name(word))
            for word in (
                ida_bytes.get_dword(address + offset) for offset in range(0, 64, 4)
            )
        ]
        lines.append(
            f"{address:08x} {name} :: {demangled} :: first64={words!r}"
        )
    for address, name in sorted(selected.items()):
        function = ida_funcs.get_func(address)
        if function is None:
            continue
        lines.append(f"\nFUNCTION {address:08x}..{function.end_ea:08x} {name}")
        try:
            lines.append("\nPSEUDOCODE\n" + str(ida_hexrays.decompile(address)))
        except ida_hexrays.DecompilationFailure as error:
            lines.append(f"\nPSEUDOCODE FAILED: {error}")
        lines.append("\nDISASSEMBLY")
        for instruction in idautils.Heads(address, function.end_ea):
            if ida_bytes.is_code(ida_bytes.get_flags(instruction)):
                lines.append(f"{instruction:08x}  {clean_disassembly(instruction)}")

    OUTPUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {len(selected)} object-ID functions to {OUTPUT}")


exit_code = 0
try:
    main()
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
