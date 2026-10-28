"""Find code/data references to high-volume Pegasus TOC type constants."""

from __future__ import annotations

import json
from pathlib import Path

import ida_auto
import ida_bytes
import ida_funcs
import ida_ida
import ida_name
import ida_pro
import ida_segment
import ida_ua
import idautils


OUTPUT = Path(__file__).resolve().parents[2] / "artifacts" / "toc-type-constants.json"
TARGETS = (0x00EB0064, 0x00EB0068, 0x00EB0069, 0x00EB006B)
LOWS = {target & 0xFFFF for target in TARGETS}
HIGH = 0x00EB


def immediates(address: int) -> set[int]:
    instruction = ida_ua.insn_t()
    if not ida_ua.decode_insn(instruction, address):
        return set()
    return {
        int(operand.value) & 0xFFFFFFFF
        for operand in instruction.ops
        if operand.type == ida_ua.o_imm
    }


def identity(address: int) -> dict[str, object]:
    function = ida_funcs.get_func(address)
    start = function.start_ea if function else None
    name = ida_funcs.get_func_name(start) if start is not None else ida_name.get_name(address)
    return {
        "address": address,
        "function_start": start,
        "name": name,
        "disassembly": ida_ua.print_insn_mnem(address),
    }


def code_candidates() -> list[dict[str, object]]:
    records = []
    for start in idautils.Functions():
        recent: list[tuple[int, set[int]]] = []
        for address in idautils.FuncItems(start):
            values = immediates(address)
            if values & LOWS and any(HIGH in prior for _, prior in recent[-8:]):
                record = identity(address)
                record["recent"] = [
                    {"address": prior_address, "immediates": sorted(prior_values)}
                    for prior_address, prior_values in recent[-8:]
                ]
                record["immediates"] = sorted(values)
                records.append(record)
            recent.append((address, values))
    return records


def aligned_words() -> list[dict[str, object]]:
    records = []
    segment = ida_segment.get_first_seg()
    while segment:
        address = (segment.start_ea + 3) & ~3
        while address + 4 <= segment.end_ea:
            value = ida_bytes.get_dword(address)
            if value in TARGETS:
                record = identity(address)
                record["value"] = value
                record["xrefs"] = [identity(reference.frm) for reference in idautils.XrefsTo(address)]
                record["window"] = [
                    {
                        "address": candidate,
                        "value": ida_bytes.get_dword(candidate),
                        "name": ida_name.get_name(candidate),
                    }
                    for candidate in range(address - 32, address + 36, 4)
                ]
                records.append(record)
            address += 4
        segment = ida_segment.get_next_seg(segment.start_ea)
    return records


def main() -> None:
    ida_auto.auto_wait()
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(
        json.dumps(
            {
                "processor_big_endian": bool(ida_ida.inf_is_be()),
                "targets": TARGETS,
                "code_candidates": code_candidates(),
                "aligned_words": aligned_words(),
            },
            indent=2,
        ),
        encoding="utf-8",
    )


exit_code = 0
try:
    main()
except Exception:
    import traceback

    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
