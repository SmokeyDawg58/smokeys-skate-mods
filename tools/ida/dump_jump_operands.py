"""Dump raw IDA operand fields for the optimized grounded-jump vector code."""

from __future__ import annotations

import json
import traceback
from pathlib import Path

import ida_auto
import ida_bytes
import ida_funcs
import ida_idp
import ida_pro
import ida_ua
import idautils


TARGET = 0x82B315A8
OUTPUT = Path(__file__).resolve().parents[2] / "artifacts" / "jump-operands.jsonl"
RANGES = ((0x82B316A8, 0x82B3191C), (0x82B31E54, 0x82B3209C))


def in_ranges(address: int) -> bool:
    return any(start <= address < end for start, end in RANGES)


exit_code = 0
try:
    ida_auto.auto_wait()
    function = ida_funcs.get_func(TARGET)
    if function is None:
        raise RuntimeError(f"no function at {TARGET:08x}")
    with OUTPUT.open("w", encoding="utf-8") as output:
        for address in idautils.Heads(function.start_ea, function.end_ea):
            if not in_ranges(address):
                continue
            instruction = ida_ua.insn_t()
            if ida_ua.decode_insn(instruction, address) <= 0:
                continue
            operands = []
            for operand in instruction.ops:
                if operand.type == ida_ua.o_void:
                    break
                operands.append(
                    {
                        "type": operand.type,
                        "dtype": operand.dtype,
                        "reg": operand.reg,
                        "reg_name": ida_idp.get_reg_name(operand.reg, operand.dtype),
                        "value": operand.value,
                        "address": operand.addr,
                        "specflag1": operand.specflag1,
                        "specflag2": operand.specflag2,
                        "specflag3": operand.specflag3,
                        "specflag4": operand.specflag4,
                    }
                )
            item = {
                "address": address,
                "bytes": ida_bytes.get_bytes(address, instruction.size).hex(),
                "itype": instruction.itype,
                "mnemonic": instruction.get_canon_mnem(),
                "operands": operands,
            }
            output.write(json.dumps(item) + "\n")
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
