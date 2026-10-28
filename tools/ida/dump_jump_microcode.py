"""Dump Hex-Rays microcode for the grounded-jump function, then exit."""

from __future__ import annotations

import traceback
from pathlib import Path

import ida_auto
import ida_funcs
import ida_hexrays
import ida_pro


TARGET = 0x82B315A8
OUTPUT = Path(__file__).resolve().parents[2] / "artifacts" / "jump-microcode.txt"


def dump_maturity(function, maturity: int) -> list[str]:
    failure = ida_hexrays.hexrays_failure_t()
    ranges = ida_hexrays.mba_ranges_t(function)
    mba = ida_hexrays.gen_microcode(
        ranges,
        failure,
        None,
        ida_hexrays.DECOMP_WARNINGS | ida_hexrays.DECOMP_NO_CACHE,
        maturity,
    )
    if mba is None:
        raise RuntimeError(f"microcode generation failed: {failure.str}")
    lines = [f"MATURITY {maturity}"]
    block = mba.blocks
    while block is not None:
        lines.append(
            f"\nBLOCK {block.serial} {block.start:08x}..{block.end:08x} "
            f"pred={list(block.predset)} succ={list(block.succset)}"
        )
        instruction = block.head
        while instruction is not None:
            lines.append(f"{instruction.ea:08x}  {instruction.dstr()}")
            instruction = instruction.next
        block = block.nextb
    return lines


exit_code = 0
try:
    ida_auto.auto_wait()
    if not ida_hexrays.init_hexrays_plugin():
        raise RuntimeError("Hex-Rays is unavailable")
    function = ida_funcs.get_func(TARGET)
    if function is None:
        raise RuntimeError(f"no function at {TARGET:08x}")
    lines: list[str] = []
    for maturity in (
        ida_hexrays.MMAT_PREOPTIMIZED,
        ida_hexrays.MMAT_CALLS,
        ida_hexrays.MMAT_GLBOPT3,
    ):
        lines.extend(dump_maturity(function, maturity))
        lines.append("\n" + "=" * 80 + "\n")
    OUTPUT.write_text("\n".join(lines), encoding="utf-8")
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
