"""Dump symbol-imported collision and dynamic-object model layouts."""

from __future__ import annotations

import json
import traceback
from pathlib import Path

import ida_auto
import ida_pro
import ida_typeinf


PROJECT = Path(__file__).resolve().parents[2]
OUTPUT = PROJECT / "artifacts" / "collision-types.json"
WANTED = {
    "pegasus::tCModelData",
    "pegasus::tCModelData::tMeshData",
    "pegasus::tInstance",
    "rw::collision::Aggregate",
    "rw::collision::Feature",
    "rw::collision::FeatureEdge",
    "rw::collision::GPInstance",
    "rw::collision::KDTreeMappedArray",
    "rw::collision::SimpleMappedArray",
    "rw::collision::Volume",
    "rw::collision::rwc_FeatureIntersectionPrism",
    "rw::physics::Assembly",
    "rw::physics::AssemblyDefinition",
    "rw::physics::PartDefinition",
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
    return record


def main() -> None:
    ida_auto.auto_wait()
    found: dict[str, object] = {}
    for tif in ida_typeinf.get_idati().numbered_types():
        name = tif.get_type_name()
        if name in WANTED:
            found[name] = describe(tif)
    OUTPUT.write_text(json.dumps(found, indent=2, sort_keys=True), encoding="utf-8")
    print(f"dumped {len(found)} of {len(WANTED)} collision types to {OUTPUT}")


exit_code = 0
try:
    main()
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
