"""Dump PDB-imported Andale database, hierarchy, pose, and clip layouts."""

from __future__ import annotations

import json
import traceback
from pathlib import Path

import ida_auto
import ida_pro
import ida_typeinf


PROJECT = Path(__file__).resolve().parents[2]
OUTPUT = PROJECT / "artifacts" / "andale-types.json"
TERMS = (
    "DataBase",
    "Hierarchy",
    "DataHeader",
    "Pose",
    "Clip",
    "FileHeader",
    "AnimationPart",
    "SQT",
    "ExtractType",
    "Compressed",
    "Quantize",
    "AnimData",
    "AnimationHeader",
    "VBR",
    "BlendSpace",
    "PhaseBlend",
    "Sequence",
    "Selector",
    "PhysicsParam",
    "NVector",
    "NMatrix",
    "VolumeSet",
    "Construction",
    "LocomotionStage",
    "AttributeHeader",
    "AnimOutPhysIn",
    "PhysInAnimOut",
    "AnimOutTrickData",
    "AnimIn",
    "PhysIn",
    "ProcessedPhysIn",
    "AnimOut",
    "GestureData",
    "WipeoutData",
    "AIPhysicsInput",
    "ExternalPlayer",
    "Trajectory",
    "GroundAnimation",
    "SkateboardJump",
    "Skateboard",
    "PredictionResults",
    "PredictionCollision",
    "TrajectoryResult",
    "VolumeLineResult",
    "BatchFlags",
    "LineBatch",
    "IIsland",
    "ClusteredMeshData",
    "Triangle",
    "GrindData",
    "GrindPrimitive",
    "GrindGeometry",
    "GrindSurfaceInfo",
    "CollTestResult",
    "GrindTrajectoryResults",
    "PossibleGrindOrientations",
    "eGrindGeometryType",
    "tAudioSurfaceType",
    "tPhysicsSurfaceType",
    "Octree",
)


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
    for tif in ida_typeinf.get_idati().numbered_types():
        name = tif.get_type_name()
        if (
            name.startswith(
                (
                    "Andale::",
                    "AnimCmdSys::",
                    "ANIM_CODEC::",
                    "Sk8::Physics::",
                    "Sk8::Skater::",
                    "pegasus::",
                )
            )
            and any(term in name for term in TERMS)
        ) or name.startswith(
            (
                "Attrib::Gen::physics_trajectory",
                "Attrib::Gen::physics_reckoning",
                "Attrib::Gen::physicsdeck",
            )
        ):
            found[name] = describe(tif)
    OUTPUT.write_text(json.dumps(found, indent=2, sort_keys=True), encoding="utf-8")
    print(f"dumped {len(found)} Andale types to {OUTPUT}")


exit_code = 0
try:
    main()
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
