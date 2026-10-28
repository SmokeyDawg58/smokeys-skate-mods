"""Export reconstruction metadata from the database currently open in IDA.

Run with File -> Script file (Alt+F7). Outputs are intentionally written below
the repository's ignored `artifacts` directory.
"""

from __future__ import annotations

import json
import os
import traceback
from pathlib import Path

import ida_auto
import ida_entry
import ida_funcs
import ida_ida
import ida_idaapi
import ida_loader
import ida_name
import ida_nalt
import ida_segment
import ida_typeinf
import idautils


DEFAULT_OUTPUT = Path(__file__).resolve().parents[2] / "artifacts" / "ida-export"


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True), encoding="utf-8")


def export_segments(output: Path) -> int:
    records = []
    for index in range(ida_segment.get_segm_qty()):
        segment = ida_segment.getnseg(index)
        records.append(
            {
                "name": ida_segment.get_segm_name(segment),
                "class": ida_segment.get_segm_class(segment),
                "start": segment.start_ea,
                "end": segment.end_ea,
                "size": segment.end_ea - segment.start_ea,
                "permissions": segment.perm,
                "bitness": segment.bitness,
            }
        )
    write_json(output / "segments.json", records)
    return len(records)


def export_entries(output: Path) -> int:
    records = []
    for index in range(ida_entry.get_entry_qty()):
        ordinal = ida_entry.get_entry_ordinal(index)
        address = ida_entry.get_entry(ordinal)
        records.append(
            {
                "ordinal": ordinal,
                "address": address,
                "name": ida_entry.get_entry_name(ordinal),
            }
        )
    write_json(output / "entries.json", records)
    return len(records)


def export_imports(output: Path) -> int:
    modules = []
    total = 0
    for module_index in range(ida_nalt.get_import_module_qty()):
        imports = []

        def visit(address: int, name: str | None, ordinal: int) -> bool:
            imports.append({"address": address, "name": name, "ordinal": ordinal})
            return True

        ida_nalt.enum_import_names(module_index, visit)
        total += len(imports)
        modules.append(
            {
                "name": ida_nalt.get_import_module_name(module_index),
                "imports": imports,
            }
        )
    write_json(output / "imports.json", modules)
    return total


def export_functions(output: Path) -> int:
    count = 0
    with (output / "functions.jsonl").open("w", encoding="utf-8") as stream:
        for address in idautils.Functions():
            function = ida_funcs.get_func(address)
            if function is None:
                continue
            name = ida_funcs.get_func_name(address)
            demangled = ida_name.demangle_name(name, ida_name.MNG_LONG_FORM)
            record = {
                "start": function.start_ea,
                "end": function.end_ea,
                "size": function.end_ea - function.start_ea,
                "name": name,
                "demangled": demangled,
                "flags": function.flags,
            }
            stream.write(json.dumps(record, sort_keys=True) + "\n")
            count += 1
    return count


def export_types(output: Path) -> int:
    type_library = ida_typeinf.get_idati()
    count = 0
    with (output / "types.jsonl").open("w", encoding="utf-8") as stream:
        for ordinal in range(1, ida_typeinf.get_ordinal_limit(type_library)):
            name = ida_typeinf.get_numbered_type_name(type_library, ordinal)
            if not name:
                continue
            type_info = ida_typeinf.tinfo_t()
            if not type_info.get_numbered_type(type_library, ordinal):
                continue
            record: dict[str, object] = {
                "ordinal": ordinal,
                "name": name,
                "declaration": str(type_info),
                "size": type_info.get_size(),
                "is_udt": type_info.is_udt(),
                "is_enum": type_info.is_enum(),
            }
            if type_info.is_udt():
                details = ida_typeinf.udt_type_data_t()
                if type_info.get_udt_details(details, ida_typeinf.GTD_CALC_LAYOUT):
                    record["members"] = [
                        {
                            "name": member.name,
                            "bit_offset": member.offset,
                            "bit_size": member.size,
                            "type": str(member.type),
                        }
                        for member in details
                    ]
            elif type_info.is_enum():
                details = ida_typeinf.enum_type_data_t()
                if type_info.get_enum_details(details):
                    record["members"] = [
                        {"name": member.name, "value": member.value}
                        for member in details
                    ]
            stream.write(json.dumps(record, sort_keys=True) + "\n")
            count += 1
    return count


def main() -> None:
    previous_timeout = ida_idaapi.set_script_timeout(0)
    output = Path(os.environ.get("SKATE2_IDA_EXPORT", DEFAULT_OUTPUT))
    output.mkdir(parents=True, exist_ok=True)
    marker = output / "export-state.txt"
    marker.write_text("running\n", encoding="utf-8")
    try:
        ida_auto.auto_wait()
        manifest = {
            "input_file": ida_nalt.get_input_file_path(),
            "image_base": ida_nalt.get_imagebase(),
            "min_ea": ida_ida.inf_get_min_ea(),
            "max_ea": ida_ida.inf_get_max_ea(),
            "segments": export_segments(output),
            "entries": export_entries(output),
            "imports": export_imports(output),
            "functions": export_functions(output),
            "types": export_types(output),
        }
        write_json(output / "manifest.json", manifest)
        marker.write_text("complete\n", encoding="utf-8")
        print(f"Skate 2 metadata export complete: {output}")
    except Exception:
        error = traceback.format_exc()
        marker.write_text("failed\n" + error, encoding="utf-8")
        print(error)
        raise
    finally:
        ida_idaapi.set_script_timeout(previous_timeout)


if __name__ == "__main__":
    main()
