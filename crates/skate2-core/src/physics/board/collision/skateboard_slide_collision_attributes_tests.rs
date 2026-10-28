use super::*;
use crate::attribute_runtime::tests::{collection, definition, graph, node, set_dependency_bytes};

#[test]
fn global_hashes_and_missing_generated_layout_defaults() {
    let vault = Arc::new(graph(SlideCollisionGlobal::Friction.class(), vec![]));
    let mut reader = SkateboardSlideCollisionVaultAttributes::bind(&vault);
    for (kind, name) in [
        (SlideCollisionGlobal::Friction, "physics_friction"),
        (SlideCollisionGlobal::Heading, "physics_heading"),
        (SlideCollisionGlobal::Wheels, "physicswheels"),
        (SlideCollisionGlobal::Collision, "physics_collision"),
    ] {
        assert_eq!(kind.class(), skate2_formats::attrib::string_hash64(name));
    }
    for curve in [
        SlideCollisionCurve::SlideVsNormal,
        SlideCollisionCurve::SlideVsSpeed,
        SlideCollisionCurve::SideFrictionFactor,
        SlideCollisionCurve::CollisionTorqueVsAngle,
    ] {
        let global = reader.capture_global(curve.layout().0).unwrap();
        assert_eq!(reader.graph(&global, curve, f32::NAN).unwrap().to_bits(), 0);
    }
    let global = reader
        .capture_global(SlideCollisionGlobal::Collision)
        .unwrap();
    let pointer = reader.lookup(&global, COLLISION_LIMIT).unwrap();
    assert_eq!(reader.float(&pointer).unwrap().to_bits(), 0);
}

#[test]
fn foreign_global_pointer_and_selected_surface_are_rejected_even_same_vault() {
    let vault = Arc::new(graph(SlideCollisionGlobal::Collision.class(), vec![]));
    let mut first = SkateboardSlideCollisionVaultAttributes::bind(&vault);
    let mut other = SkateboardSlideCollisionVaultAttributes::bind(&vault);
    let global = other
        .capture_global(SlideCollisionGlobal::Collision)
        .unwrap();
    let pointer = other.lookup(&global, COLLISION_LIMIT).unwrap();
    let surface = other.bind_selected_surface(7);
    assert!(matches!(
        first.lookup(&global, COLLISION_LIMIT),
        Err(SkateboardSlideCollisionAttributeError::ForeignOwner)
    ));
    assert!(matches!(
        first.float(&pointer),
        Err(SkateboardSlideCollisionAttributeError::ForeignOwner)
    ));
    assert!(matches!(
        first.surface_float(&surface, SlideSurfaceField::StraightenOutForceTime),
        Err(SkateboardSlideCollisionAttributeError::ForeignOwner)
    ));
    let global = first
        .capture_global(SlideCollisionGlobal::Collision)
        .unwrap();
    assert!(matches!(
        first.graph(&global, SlideCollisionCurve::SlideVsNormal, 0.0),
        Err(SkateboardSlideCollisionAttributeError::WrongGlobal)
    ));
}

#[test]
fn selected_surface_uses_actual_cached_layout_not_global_default_or_generic_parent() {
    let class = SkateboardSlideCollisionVaultAttributes::SURFACE_CLASS;
    let mut vault = graph(class, vec![]);
    let mut bytes = vec![0; 0x500];
    bytes[0xec..0xf0].copy_from_slice(&2.0_f32.to_be_bytes());
    bytes[0x108..0x10c].copy_from_slice(&7.0_f32.to_be_bytes());
    bytes[0xf0..0xf4].copy_from_slice(&(-0.0_f32).to_be_bytes());
    bytes[0x200 + 0xec..0x200 + 0xf0].copy_from_slice(&9.0_f32.to_be_bytes());
    collection(&mut vault, class, 7, 0, Some(0), vec![]);
    collection(&mut vault, class, 8, 0, Some(0x200), vec![]);
    collection(&mut vault, class, 9, 7, None, vec![]);
    set_dependency_bytes(&mut vault, 1, bytes);
    let mut reader = SkateboardSlideCollisionVaultAttributes::bind(&Arc::new(vault));
    let seven = reader.bind_selected_surface(7);
    let eight = reader.bind_selected_surface(8);
    let nine = reader.bind_selected_surface(9);
    assert_eq!(
        reader
            .surface_float(&seven, SlideSurfaceField::BrakingScalar)
            .unwrap()
            .to_bits(),
        7.0_f32.to_bits()
    );
    assert_eq!(
        reader
            .surface_float(&seven, SlideSurfaceField::StraightenOutForceTime)
            .unwrap()
            .to_bits(),
        2.0_f32.to_bits()
    );
    assert_eq!(
        reader
            .surface_float(&eight, SlideSurfaceField::StraightenOutForceTime)
            .unwrap()
            .to_bits(),
        9.0_f32.to_bits()
    );
    assert_eq!(
        reader
            .surface_float(&seven, SlideSurfaceField::SideFrictionScalar)
            .unwrap()
            .to_bits(),
        0x8000_0000
    );
    assert_eq!(
        reader
            .surface_float(&nine, SlideSurfaceField::StraightenOutForceTime)
            .unwrap()
            .to_bits(),
        0
    );
}

#[test]
fn scalar_pointer_retains_parent_payload_and_malformed_found_value_is_not_default() {
    let class = SlideCollisionGlobal::Collision.class();
    let mut vault = graph(
        class,
        vec![
            definition(COLLISION_LIMIT, 4, 4),
            definition(COLLISION_RESPONSE, 4, 1),
        ],
    );
    collection(
        &mut vault,
        class,
        7,
        0,
        None,
        vec![
            node(COLLISION_LIMIT, 0x40, 0x8000_0000),
            node(COLLISION_RESPONSE, 0x40, 0x7f00_0000),
        ],
    );
    collection(
        &mut vault,
        class,
        SkateboardSlideCollisionVaultAttributes::DEFAULT_COLLECTION,
        7,
        None,
        vec![],
    );
    let mut reader = SkateboardSlideCollisionVaultAttributes::bind(&Arc::new(vault));
    let global = reader
        .capture_global(SlideCollisionGlobal::Collision)
        .unwrap();
    let limit = reader.lookup(&global, COLLISION_LIMIT).unwrap();
    let malformed = reader.lookup(&global, COLLISION_RESPONSE).unwrap();
    assert_eq!(reader.float(&limit).unwrap().to_bits(), 0x8000_0000);
    assert!(matches!(
        reader.float(&malformed),
        Err(SkateboardSlideCollisionAttributeError::Runtime(
            AttributeRuntimeError::ScalarSize { size: 1 }
        ))
    ));
}

#[test]
fn source_curve_offsets_and_counts_use_real_cached_data() {
    for curve in [
        SlideCollisionCurve::SlideVsNormal,
        SlideCollisionCurve::SlideVsSpeed,
        SlideCollisionCurve::SideFrictionFactor,
        SlideCollisionCurve::CollisionTorqueVsAngle,
    ] {
        let (kind, size, offset, count) = curve.layout();
        let mut vault = graph(kind.class(), vec![]);
        let mut bytes = vec![0; size];
        for i in 0..count {
            bytes[offset + i * 4..offset + i * 4 + 4]
                .copy_from_slice(&f32::from(u16::try_from(i).unwrap()).to_be_bytes());
            bytes[offset + (i + count) * 4..offset + (i + count) * 4 + 4]
                .copy_from_slice(&f32::from(u16::try_from(i * i).unwrap()).to_be_bytes());
        }
        collection(
            &mut vault,
            kind.class(),
            SkateboardSlideCollisionVaultAttributes::DEFAULT_COLLECTION,
            0,
            Some(0),
            vec![],
        );
        set_dependency_bytes(&mut vault, 1, bytes);
        let mut reader = SkateboardSlideCollisionVaultAttributes::bind(&Arc::new(vault));
        let global = reader.capture_global(kind).unwrap();
        assert_eq!(
            reader.graph(&global, curve, 1.5).unwrap().to_bits(),
            2.5_f32.to_bits()
        );
        assert_eq!(
            reader.graph(&global, curve, f32::NAN).unwrap().to_bits(),
            f32::from(u16::try_from((count - 1) * (count - 1)).unwrap()).to_bits()
        );
    }
}

const SCALARS: [(SlideCollisionGlobal, u64, usize, u32); 5] = [
    (
        SlideCollisionGlobal::Collision,
        COLLISION_LIMIT,
        0x5c170,
        0x4100_0000,
    ),
    (
        SlideCollisionGlobal::Collision,
        COLLISION_LOCAL_Y,
        0x5c130,
        0xbd4c_cccd,
    ),
    (
        SlideCollisionGlobal::Collision,
        COLLISION_RESPONSE,
        0x5c0b0,
        0x3f80_0000,
    ),
    (
        SlideCollisionGlobal::Collision,
        COLLISION_NORMAL_SPEED,
        0x5c1c0,
        0x3f80_0000,
    ),
    (
        SlideCollisionGlobal::Wheels,
        WHEEL_HARDNESS_BLEND,
        0x5b648,
        0x4000_0000,
    ),
];
const CURVES: [(SlideCollisionCurve, usize, usize); 4] = [
    (SlideCollisionCurve::SlideVsNormal, 0x9bd00, 16),
    (SlideCollisionCurve::SlideVsSpeed, 0x9be20, 8),
    (SlideCollisionCurve::SideFrictionFactor, 0x9ce90, 16),
    (SlideCollisionCurve::CollisionTorqueVsAngle, 0x9b450, 8),
];

/// Actual mounted provider compared with independent original inline words and
/// source curve samples. Original bytes remain outside the repository.
pub(crate) fn check_original(vault: &Arc<AttributeRuntimeVault>, archive: &[u8]) {
    let mut reader = SkateboardSlideCollisionVaultAttributes::bind(vault);
    for (kind, key, offset, bits) in SCALARS {
        let global = reader.capture_global(kind).unwrap();
        let pointer = reader.lookup(&global, key).unwrap();
        assert!(pointer.data.is_some());
        assert_eq!(reader.float(&pointer).unwrap().to_bits(), bits);
        let at = 256 + offset;
        assert_eq!(&archive[at..at + 8], &key.to_be_bytes());
        assert_eq!(&archive[at + 8..at + 12], &bits.to_be_bytes());
    }
    for (curve, offset, count) in CURVES {
        let global = reader.capture_global(curve.layout().0).unwrap();
        let at = 851_840 + offset;
        let word =
            |i: usize| f32::from_be_bytes(archive[at + i * 4..at + i * 4 + 4].try_into().unwrap());
        for i in 0..count {
            let x = word(i);
            let expected = word(i + count);
            let actual = reader.graph(&global, curve, x).unwrap();
            assert_eq!(actual.to_bits(), expected.to_bits(), "{curve:?} knot{i}");
        }
    }
}
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn mounted_original_globals_match_independent_db_archive() {
    let vault = crate::attribute_runtime_corpus::mounted_physics_attributes_with_classes(&[
        SlideCollisionGlobal::Friction.class(),
        SlideCollisionGlobal::Heading.class(),
        SlideCollisionGlobal::Wheels.class(),
        SlideCollisionGlobal::Collision.class(),
    ]);
    let root = std::env::var("SKATE2_GAME_ROOT").unwrap();
    let archive = skate2_formats::mount::SKATE2_ARCHIVES
        .iter()
        .find(|a| a.relative_path == "data\\db\\db.big")
        .unwrap();
    let bytes = std::fs::read(archive.path_under(std::path::Path::new(&root))).unwrap();
    check_original(&vault, &bytes);
}

#[test]
fn captured_surface_layout_preserves_owner_and_defers_malformed_storage_until_read() {
    let class = SkateboardSlideCollisionVaultAttributes::SURFACE_CLASS;
    let mut vault = graph(class, vec![]);
    collection(&mut vault, class, 7, 0, Some(240), vec![]);
    let vault = Arc::new(vault);
    let mut reader = SkateboardSlideCollisionVaultAttributes::bind(&vault);
    let mut other = SkateboardSlideCollisionVaultAttributes::bind(&vault);
    let surface = reader.bind_selected_surface(7);
    let captured = reader.capture_surface_layout(&surface).unwrap();
    assert!(matches!(
        other.layout_float(&captured, SlideSurfaceField::WheelDynamicFriction),
        Err(SkateboardSlideCollisionAttributeError::ForeignOwner)
    ));
    assert!(matches!(
        reader.layout_float(&captured, SlideSurfaceField::WheelDynamicFriction),
        Err(SkateboardSlideCollisionAttributeError::Runtime(_))
    ));
    assert!(matches!(
        surface.try_float(SlideSurfaceField::BrakingScalar),
        Err(SkateboardSlideCollisionAttributeError::Runtime(_))
    ));
}
