"""Dump selected symbol-imported character layouts from the headless IDA database."""

from __future__ import annotations

import json
import traceback
from pathlib import Path

import ida_auto
import ida_pro
import ida_typeinf


PROJECT = Path(__file__).resolve().parents[2]
OUTPUT = PROJECT / "artifacts" / "character-types.json"
WANTED = {
    "Character::CharacterEntity",
    "Character::CharacterEntity::CharacterLod",
    "Character::CharacterEntity::PartToRModel",
    "Character::CharacterEntity::RModelDataToCASAssetIDs",
    "Character::tComponent",
    "Character::tModel",
    "Character::tLod",
    "Character::tMaterialInstance",
    "Character::tMaterial",
    "Character::tShaderParam",
    "assman::cAssetID",
    "pegasus::tRModelData",
    "pegasus::tInstance",
    "Sk8::Render::cModelInstance",
    "Andale::HierarchyData",
    "renderengine::IndexBuffer",
    "renderengine::IndexBuffer::Parameters",
    "renderengine::VertexBuffer",
    "renderengine::VertexBuffer::Parameters",
    "D3DIndexBuffer",
    "D3DVertexBuffer",
    "D3DResource",
    "renderengine::VertexFormat",
}


def describe(tif: ida_typeinf.tinfo_t) -> dict[str, object]:
    record: dict[str, object] = {
        "declaration": tif.dstr(),
        "size": tif.get_size(),
    }
    members = ida_typeinf.udt_type_data_t()
    if tif.get_udt_details(members):
        record["members"] = [
            {
                "name": member.name,
                "offset_bits": member.offset,
                "size_bits": member.size,
                "type": member.type.dstr(),
            }
            for member in members
        ]
    enum = ida_typeinf.enum_type_data_t()
    if tif.get_enum_details(enum):
        record["values"] = [
            {"name": member.name, "value": member.value} for member in enum
        ]
    return record


def main() -> None:
    ida_auto.auto_wait()
    found: dict[str, object] = {}
    til = ida_typeinf.get_idati()
    for tif in til.numbered_types():
        name = tif.get_type_name()
        if name in WANTED:
            found[name] = describe(tif)
    OUTPUT.write_text(json.dumps(found, indent=2, sort_keys=True), encoding="utf-8")
    print(f"dumped {len(found)} of {len(WANTED)} character types to {OUTPUT}")


exit_code = 0
try:
    main()
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
