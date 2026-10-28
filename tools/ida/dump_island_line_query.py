"""Dump decompiler and raw instruction evidence for the grind island query."""

from __future__ import annotations

import traceback
from pathlib import Path

import ida_auto
import ida_bytes
import ida_funcs
import ida_hexrays
import ida_idaapi
import ida_lines
import ida_name
import ida_pro
import idautils


OUTPUT = Path(__file__).resolve().parents[2] / "artifacts" / "island-line-query.txt"
TARGETS = (
    0x82CF9C80,  # GrindData::BoxTest dynamic-provider wrapper
    0x82CF9CF0,  # GrindData::TrajectoryBoxTest
    0x82CFAC60,  # GrindGeometryAnalyzer::DoGeometryTest
    0x825E9720,  # Dmo::SimManager dynamic grind query
    0x825E9480,  # AABBoxTemplate::Transform used by dynamic grinds
    0x82703FF0,  # LivingWorld::Manager dynamic grind query
    0x82D30FB0,  # LWDynamicObjectMan dynamic grind query
    0x82D237A0,  # LWVehicleMan dynamic grind query
    0x82F8C1B0,  # kFeedDMOGrindSplines static initializer
    0x82F8CFF8,  # kMaxDynamicObjectSize static initializer
    0x82F8B5A8,  # kMaxVechileObjectSize static initializer
    0x82E5EC58,  # LWGrindableObject constructor
    0x82E4D380,  # LWGrindableObject::Init
    0x82E62EA0,  # LWGrindableObject::AddGrindSpline
    0x82E62DA0,  # LWGrindableObject::DeInit
    0x82D17340,  # Vehicle::InitSplines
    0x82D2BF18,  # DynamicObject::InitSplines
    0x825EA6B8,  # Dmo::Registration::DRegDmoLocator
    0x825EAC28,  # Dmo::Registration::RegDmoLocator
    0x825EAB98,  # Dmo::Registration::RegDmoModel
    0x825EA628,  # Dmo::Registration::DRegDmoModel
    0x82D10F10,  # CharacterAssetManager::GetTInstanceWithCollisionVolume
    0x82D39F90,  # LWDynamicObjectModel::Create
    0x82D29F58,  # LWDynamicObjectModel::FindCharacteristic
    0x82D2B5F0,  # generated livingworld_dynamicobject_priority constructor
    0x826FC8B8,  # generated global livingworld constructor
    0x82D337B0,  # LWDynamicObjectModel::CreateFromMesh
    0x82D23AE0,  # LWVehicleModel::Create
    0x82E51010,  # LWModel::Create
    0x82D2DF10,  # LWDynamicObjectLocatorMan::GetStreamedLocator
    0x82D2EE20,  # LWDynamicObjectUtil::GetSimModelIDFromInstanecID
    0x82D3AD38,  # LWDynamicObjectLocatorMan::CreateLocator
    0x82D3BE90,  # LWDynamicObjectLocatorMan::AddLocatorsForStreamedLocatorChunk
    0x82D3D548,  # LWDynamicObjectLocatorMan::AddStreamedLocatorData
    0x82D3D6D0,  # LWDynamicObjectMan::AddStreamedLocatorData policy wrapper
    0x82D5F4F0,  # cProcessArenaAsset::ProcessDMOData
    0x82D5EC38,  # cProcessArenaAsset::ProcessInstanceList
    0x82CE9B00,  # LivingWorld::ModelMan::AddRecipe
    0x82CE9C60,  # LivingWorld::ModelMan::AddDMOModel
    0x82CE9D78,  # LivingWorld::ModelMan::ReceiveRegLWSchemaMsg
    0x82CE8D68,  # LivingWorld::ModelMan::RemoveDMOModel
    0x828107D8,  # cLivingWorldPresModelManager::AddDMOModel
    0x8280DD68,  # cLivingWorldPresModelManager::RemoveDMOModel
    0x828A0420,  # Simulation::RegDMOArena
    0x828A01E0,  # Simulation::RegArena
    0x828A0828,  # Simulation::DRegDMOArena
    0x82BB57C8,  # rw::RwHash32String
    0x82F76880,  # initialize kAssetType_Instance_Marquee
    0x82F768B8,  # initialize kAssetType_Instance_CreateACharacter
    0x82F768F0,  # initialize kAssetType_Instance_LivingWorld
    0x82F76928,  # initialize kAssetType_Instance_Environment
    0x82F76960,  # initialize kAssetType_Instance_Trigger
    0x82F76998,  # initialize kAssetType_Instance_Prop
    0x82F769D0,  # initialize kAssetType_Instance_VisualIndicator
    0x82F76A08,  # initialize kAssetType_Instance_DMO
    0x82F76A40,  # initialize kAssetType_Instance_Spatial
    0x82D1CAB0,  # Vehicle constructor
    0x82D3DC58,  # DynamicObject constructor
    0x829C5610,  # pegasus::tDMOData::Unfix
    0x829C5670,  # pegasus::tDMOData::Fixup
    0x829C56C8,  # pegasus::tGrabData::Unfix
    0x829C5798,  # pegasus::tGrabData::Fixup
    0x829C6738,  # pegasus::tEmbeddedData::Fixup
    0x829C8640,  # pegasus::RegisterArenaTypes
    0x82E4A408,  # PhysicsUtility::LineWorldCollisionTest
    0x827B2538,  # Island::LineCollisionTest[WithVehicles]
    0x827B27D0,  # Island::SetupSimulationIslandsForOnline
    0x827B2880,  # Island::SetupSimulationIslandsForSinglePlayer
    0x827B42C0,  # Island::InitWorld
    0x827B6798,  # Island::AddDMOBody
    0x827B6840,  # Island::AddVehicleBody
    0x827B6B90,  # Island::CleanUpDMOBody
    0x827B7530,  # Island::RemoveDMOBody
    0x827B8728,  # Island::AcquireLineBatch
    0x827BAB18,  # Island constructor
    0x827BBEC8,  # AggregateLineBatch::GetResult
    0x827BB560,  # ClusteredMeshLineBatch::PrepareJobData
    0x827BD9A0,  # ClusteredMeshLineBatch::AddTest
    0x827C1300,  # ClusteredMeshCollection LineTest
    0x827C86D0,  # DynamicPool::AddVolume
    0x827CAF20,  # DynamicPool constructor
    0x828B4530,  # SimpleMappedArray serializer
    0x828B45E8,  # SimpleMappedArray::Fixup
    0x82AC8FD0,  # SimpleMappedArray::LineIntersectionQueryThis
    0x82AC8970,  # VolumeLineQuery::AddPrimitiveRef
    0x82AC8A48,  # VolumeLineQuery::AddVolumeRef
    0x82ADC9C8,  # AggregateVolume::GetBBox
    0x82ACF260,  # SimpleMappedArray::Initialize
    0x82ACF2B8,  # SimpleMappedArray::UpdateThis
    0x82AE1650,  # AssemblyDefinition serializer
    0x82AE17A0,  # Assembly serializer
    0x82AE0BF8,  # Assembly::InitializeComponentsFromDefinition
    0x82AE1878,  # AssemblyDefinition::Fixup
    0x82AE18C0,  # Assembly::Fixup
    0x82AE1908,  # physics::assembly arena callback registration
    0x82AE9DF8,  # PartDefinition::Fixup
    0x82AE9E38,  # PartDefinition arena callback registration
    0x82AE9820,  # Part::Initialize from PartDefinition
    0x82B25480,  # Part::SetTransform
    0x829C6478,  # pegasus::tCModelData::Unfix
    0x829C6518,  # pegasus::tCModelData::Fixup
    0x825EA858,  # Dmo::Body constructor from assembly and locator transform
    0x82DE8090,  # Sk8::Physics::Body::SetPosition
    0x8271C690,  # LivingWorld census evaluates dynamic-object spawns
    0x827208D0,  # LivingWorld CensusMan constructor
    0x82724090,  # LivingWorld census evaluates dynamic-object culls
    0x82726930,  # LivingWorld dynamic-object census wrapper
    0x82726C08,  # LivingWorld census spawns a dynamic object
    0x82727A38,  # LivingWorld complete census evaluation
    0x82D2BBA8,  # DynamicObject::CreateLivingWorldCollision
    0x82D31BA0,  # LWDynamicObjectCensusMan::Spawn
    0x82D2DA38,  # LWDynamicObjectMan compares locator with lowest-priority object
    0x82D2DAA8,  # LWDynamicObjectMan gets lowest-priority object ID
    0x82D32188,  # LWDynamicObjectLocatorMan updates its spatial lookup
    0x82D34B68,  # LWDynamicObjectMan builds a locator priority index
    0x82D38AB0,  # LWDynamicObjectMan builds dynamic-object overlap IDs
    0x82D28A10,  # LWVehicleMan builds vehicle overlap IDs
    0x82D37958,  # LWDynamicObjectMan exact overlap test
    0x82D27BA0,  # LWVehicleMan exact overlap test
    0x827CB4B8,  # LivingWorldView overlap against the DMO pool
    0x827CB540,  # LivingWorldView overlap against the vehicle pool
    0x827CB218,  # DynamicPool broad/narrow-phase volume-overlap query
    0x82E4BC00,  # PhysicsUtility batched volume-overlap narrow phase
    0x82E49858,  # PhysicsUtility overlap functor primitive test
    0x82E4A328,  # SimpleMappedArray primitive instancing for overlap tests
    0x82E4AD80,  # PhysicsUtility overlap functor constructor/padding setup
    0x82AD6188,  # RenderWare GPInstance overlap predicate
    0x82ACDE18,  # SphereVolume GPInstance construction
    0x82ACA1C0,  # sphere/sphere separating direction
    0x82ACDD98,  # GPSphere maximum feature
    0x82AD2350,  # overlap feature-intersection prism
    0x82ACA8A0,  # generic GPInstance separating direction
    0x82ACA230,  # generic separator candidate-axis construction
    0x82ACD0A8,  # CapsuleVolume GPInstance construction
    0x82ACD2C8,  # CylinderVolume GPInstance construction
    0x82AD6FE0,  # BoxVolume GPInstance construction
    0x82AD9748,  # TriangleVolume GPInstance construction
    0x82AC8BE0,  # TriangleVolume local/affine normal construction
    0x82ACDDB8,  # GPSphere interval projection
    0x82ACCE30,  # GPBox interval projection
    0x82AD8218,  # GPCapsule interval projection
    0x82AD87F8,  # GPCylinder interval projection
    0x82ACE7D0,  # GPTriangle interval projection
    0x82AD2C58,  # specialized box/box separating direction
    0x82AD31A0,  # specialized triangle/box separating direction
    0x82ADAF58,  # specialized box/triangle separating direction wrapper
    0x82AE06A0,  # specialized cylinder/volume separating direction
    0x82AE06F8,  # specialized volume/cylinder separating direction
    0x82ACC360,  # GPBox maximum feature
    0x82AD80F0,  # GPCapsule maximum feature
    0x82ACE110,  # GPTriangle maximum feature
    0x82ADD438,  # GPCylinder maximum feature
    0x82AD4610,  # cylinder rim/edge separator candidate
    0x82AD46D0,  # cylinder rim/rim separator candidates
    0x82AD4A00,  # nearest cylinder-rim direction to an edge
    0x82ACAC68,  # line/edge separator candidate
    0x82AD3710,  # cylinder rim/edge closest direction
    0x82AD3B98,  # cylinder rim/rim closest directions
    0x82AD4AC0,  # cylinder specialized separator candidate construction
    0x82ADAFB0,  # cylinder specialized best-candidate selection
    0x82AC86D0,  # FeatureEdge construction
    0x82AC8770,  # FeatureEdge point constraint
    0x82AC8810,  # Feature edge-plane construction
    0x82ACB278,  # triangle feature type from direction
    0x82AC90D8,  # point/edge feature prism
    0x82AC9140,  # edge/edge feature prism
    0x82AC9848,  # point/face feature prism
    0x82AC9980,  # constrain a point to a face prism
    0x82AC9B00,  # edge clipping against a face prism
    0x82AC9CF8,  # face/face non-overlap fallback
    0x82AD0CC0,  # face/edge feature prism
    0x82AD10D8,  # four-edge/three-edge face prism
    0x82AD19B0,  # four-edge/four-edge face prism
    0x82F7A63C,  # initializes RenderWare VEC_EPSILON
    0x82D3AD20,  # LWDynamicObjectLocatorMan queries passive locators in range
    0x82D39670,  # shared 2D KDNode::FindNear specialization
    0x827EDF20,  # shared 2D KDTree::FindNear specialization
    0x82D3AA28,  # dynamic-object collision body initialization
    0x82D3CC58,  # DynamicObject::OnSpawnComplete
    0x82D3CDA0,  # DynamicObject::OnCullRequested
    0x82E50180,  # LWBasicEntity::TestLOSim
    0x82E50580,  # LWCollideableObject::OnLOSimSwitch
    0x82E505D8,  # living-world collision add world body
    0x82E50658,  # living-world collision add skater body
    0x82E50CB0,  # living-world collision body AddToWorld
    0x82F5D7F0,  # initialize kMAXDYNAMICOBJECTS
    0x82F5DBB8,  # initialize kCloseByLocatorSearchRadius
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
    lines: list[str] = []
    for target in TARGETS:
        function = ida_funcs.get_func(target)
        if function is None:
            raise RuntimeError(f"no function at {target:08x}")
        name = ida_name.get_name(function.start_ea)
        lines.append(
            f"\nFUNCTION {function.start_ea:08x}..{function.end_ea:08x} {name}"
        )
        try:
            lines.append("\nPSEUDOCODE\n" + str(ida_hexrays.decompile(target)))
        except ida_hexrays.DecompilationFailure as error:
            lines.append(f"\nPSEUDOCODE FAILED: {error}")
        lines.append("\nDISASSEMBLY")
        for address in idautils.Heads(function.start_ea, function.end_ea):
            if ida_bytes.is_code(ida_bytes.get_flags(address)):
                raw = ""
                if function.start_ea in (
                    0x82AC9140,
                    0x82AC9980,
                    0x82AC9B00,
                    0x82AC9CF8,
                    0x82ACB278,
                    0x82ACC360,
                    0x82ADD438,
                    0x82AD4AC0,
                    0x82ADAFB0,
                    0x82AD4610,
                    0x82AD46D0,
                    0x82AD4A00,
                    0x82ACAC68,
                    0x82AD3710,
                    0x82AD3B98,
                    0x82ACE110,
                    0x82AD0CC0,
                ):
                    raw = f"{ida_bytes.get_bytes(address, 4).hex()}  "
                lines.append(
                    f"{address:08x}  {raw}{clean_disassembly(address)}"
                )
    lines.append("\nDYNAMIC GRIND PROVIDER XREFS")
    for reference in idautils.XrefsTo(0x830A67D0):
        function = ida_funcs.get_func(reference.frm)
        function_name = ida_name.get_name(function.start_ea) if function else ""
        lines.append(
            f"{reference.frm:08x} {function_name} {clean_disassembly(reference.frm)}"
        )
    lines.append("\nLW GRINDABLE OBJECT XREFS")
    for target in (0x82E4D380, 0x82E62EA0, 0x82E62DA0):
        lines.append(f"TARGET {target:08x} {ida_name.get_name(target)}")
        for reference in idautils.XrefsTo(target):
            function = ida_funcs.get_func(reference.frm)
            function_name = ida_name.get_name(function.start_ea) if function else ""
            lines.append(
                f"{reference.frm:08x} {function_name} {clean_disassembly(reference.frm)}"
            )
    lines.append("\nDMO MODEL REGISTRATION XREFS")
    for target in (0x825EAB98, 0x825EA628):
        lines.append(f"TARGET {target:08x} {ida_name.get_name(target)}")
        for reference in idautils.XrefsTo(target):
            function = ida_funcs.get_func(reference.frm)
            function_name = ida_name.get_name(function.start_ea) if function else ""
            lines.append(
                f"{reference.frm:08x} {function_name} {clean_disassembly(reference.frm)}"
            )
    lines.append("\nISLAND BODY REGISTRATION XREFS")
    for target in (0x827B6798, 0x827B6840, 0x827B7530):
        lines.append(f"TARGET {target:08x} {ida_name.get_name(target)}")
        for reference in idautils.XrefsTo(target):
            function = ida_funcs.get_func(reference.frm)
            function_name = ida_name.get_name(function.start_ea) if function else ""
            lines.append(
                f"{reference.frm:08x} {function_name} {clean_disassembly(reference.frm)}"
            )
    lines.append("\nNAMED CONSTANTS")
    named_constants = (
        ("SMALL_FLOAT_9", None),
        ("SMALL_FLOAT_150", None),
        ("VEC_EPSILON_60", None),
        ("EPSILON_131", None),
        ("krGPMinimumAllowedClippingAngle", None),
        ("krGPFeatureSimplificationThreshold", None),
        ("pegasus::kAssetType_Instance_DMO", 0x83080968),
        ("pegasus::kAssetType_Instance_LivingWorld", 0x83080970),
        ("pegasus::kAssetType_Instance_CreateACharacter", 0x83080974),
        ("pegasus::kAssetType_Instance_Trigger", 0x83080978),
        ("pegasus::kAssetType_Instance_Prop", 0x8308097C),
        ("pegasus::kAssetType_Instance_Environment", 0x83080980),
        ("pegasus::kAssetType_Instance_Marquee", 0x83080984),
        ("pegasus::kAssetType_Instance_Spatial", 0x83080988),
        ("pegasus::kAssetType_Instance_VisualIndicator", 0x8308096C),
    )
    for name, known_address in named_constants:
        address = known_address or ida_name.get_name_ea(ida_idaapi.BADADDR, name)
        value = ida_bytes.get_dword(address) if address != ida_idaapi.BADADDR else None
        words = (
            [ida_bytes.get_dword(address + offset) for offset in range(0, 16, 4)]
            if address != ida_idaapi.BADADDR
            else None
        )
        lines.append(
            f"{name} address={address:08x} bits={value!r} words={words!r}"
        )
        if name in {
            "VEC_EPSILON_60",
            "krGPMinimumAllowedClippingAngle",
            "krGPFeatureSimplificationThreshold",
        }:
            for reference in idautils.XrefsTo(address):
                function = ida_funcs.get_func(reference.frm)
                function_name = ida_name.get_name(function.start_ea) if function else ""
                lines.append(
                    f"  xref {reference.frm:08x} {function_name} "
                    f"{clean_disassembly(reference.frm)}"
                )
    lines.append("\nGPBOX TABLES")
    for name, size in (
        ("axis_corner_signs", 32),
        ("axis_corner_index", 32),
        ("other_dir_indices", 24),
    ):
        address = ida_name.get_name_ea(ida_idaapi.BADADDR, name)
        blob = (
            ida_bytes.get_bytes(address, size)
            if address != ida_idaapi.BADADDR
            else None
        )
        lines.append(
            f"{name} address={address:08x} bytes={blob.hex() if blob else None}"
        )
    lines.append("\nGPINSTANCE OVERLAP DISPATCH")
    dispatch_address = ida_name.get_name_ea(
        ida_idaapi.BADADDR, "findBestSepDirFuncs"
    )
    lines.append(f"findBestSepDirFuncs address={dispatch_address:08x}")
    if dispatch_address != ida_idaapi.BADADDR:
        shape_names = ("unused", "sphere", "capsule", "triangle", "box", "cylinder")
        for left_index, left_name in enumerate(shape_names):
            for right_index, right_name in enumerate(shape_names):
                entry_address = dispatch_address + 4 * (left_index * 6 + right_index)
                target = ida_bytes.get_dword(entry_address)
                lines.append(
                    f"{left_name:8s} x {right_name:8s} -> "
                    f"{target:08x} {ida_name.get_name(target)}"
                )
    OUTPUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote island line-query evidence to {OUTPUT}")


exit_code = 0
try:
    main()
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
