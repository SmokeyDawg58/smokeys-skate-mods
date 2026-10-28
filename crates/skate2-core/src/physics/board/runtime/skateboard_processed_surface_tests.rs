use super::*;
use crate::{
    attribute_runtime::{AttributeRuntimeVault, tests::graph},
    skateboard_slide_collision_attributes::SlideCollisionCurve,
};
use std::sync::Arc;

fn owner() -> (
    SkateboardProcessedSurface,
    SkateboardSlideCollisionVaultAttributes,
) {
    let vault = Arc::new(graph(
        SkateboardSlideCollisionVaultAttributes::SURFACE_CLASS,
        vec![],
    ));
    let attributes = SkateboardSlideCollisionVaultAttributes::bind(&vault);
    (SkateboardProcessedSurface::bind(&attributes), attributes)
}
#[derive(Default)]
struct Reads {
    events: Vec<&'static str>,
    count: usize,
    fail: usize,
}
impl Reads {
    fn step(&mut self, name: &'static str) -> Result<(), usize> {
        self.events.push(name);
        self.count += 1;
        if self.count == self.fail {
            Err(self.count)
        } else {
            Ok(())
        }
    }
}
impl SkateboardSlideCollisionAttributeRead for Reads {
    type Global = ();
    type Pointer = ();
    type Surface = SkateboardSelectedSurface;
    type Layout = ();
    type Error = usize;
    fn capture_global(&mut self, kind: SlideCollisionGlobal) -> Result<(), usize> {
        assert_eq!(kind, SlideCollisionGlobal::Wheels);
        self.step("capture")
    }
    fn lookup(&mut self, (): &(), key: u64) -> Result<(), usize> {
        assert_eq!(key, WHEEL_RESTITUTION);
        self.step("lookup")
    }
    fn float(&mut self, (): &()) -> Result<f32, usize> {
        self.step("restitution")?;
        Ok(f32::from(u16::try_from(self.count).unwrap()))
    }
    fn graph(&mut self, (): &(), _: SlideCollisionCurve, _: f32) -> Result<f32, usize> {
        panic!()
    }
    fn surface_float(
        &mut self,
        _: &SkateboardSelectedSurface,
        field: SlideSurfaceField,
    ) -> Result<f32, usize> {
        self.layout_float(&(), field)
    }
    fn capture_surface_layout(&mut self, _: &SkateboardSelectedSurface) -> Result<(), usize> {
        self.step("layout")
    }
    fn layout_float(&mut self, (): &(), field: SlideSurfaceField) -> Result<f32, usize> {
        match field {
            SlideSurfaceField::WheelDynamicFriction => self.step("dynamic")?,
            SlideSurfaceField::WheelStaticFriction => self.step("static")?,
            _ => panic!(),
        }
        Ok(f32::from(u16::try_from(self.count).unwrap()))
    }
}
#[test]
fn material_constructor_reads_five_restitutions_before_dynamic_then_static() {
    let (mut owner, _) = owner();
    let mut reads = Reads::default();
    owner.try_initialize_materials(&mut reads).unwrap();
    assert_eq!(
        reads.events,
        ["capture", "lookup", "restitution"]
            .repeat(4)
            .into_iter()
            .chain([
                "capture",
                "lookup",
                "layout",
                "restitution",
                "dynamic",
                "static"
            ])
            .chain(["layout", "dynamic", "static"].repeat(4))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        owner.material_prefix()[0],
        AssemblyPartMaterial {
            static_friction: 18.0,
            dynamic_friction: 17.0,
            restitution: 3.0
        }
    );
    assert_eq!(
        owner.material_prefix()[4],
        AssemblyPartMaterial {
            static_friction: 30.0,
            dynamic_friction: 29.0,
            restitution: 16.0
        }
    );
    let before = reads.events.clone();
    assert_eq!(
        owner.try_initialize_materials(&mut reads),
        Err(SkateboardSurfaceConstructionError::AlreadyStarted)
    );
    assert_eq!(reads.events, before);
}

#[test]
fn selected_material_keeps_its_exact_slot_and_rejects_equal_foreign_owner() {
    let (mut first, _) = owner();
    let (mut second, _) = owner();
    first
        .try_initialize_materials(&mut Reads::default())
        .unwrap();
    second
        .try_initialize_materials(&mut Reads::default())
        .unwrap();
    let mut a = SkateboardSurfaceProjection::default();
    let mut b = SkateboardSurfaceProjection::default();
    first.dispatch_from_published_type(1, &mut a).unwrap();
    second.dispatch_from_published_type(1, &mut b).unwrap();
    let a = a.material.unwrap();
    let b = b.material.unwrap();
    assert_eq!(**a.material_arc(), **b.material_arc());
    assert!(first.owns_material(&a));
    assert!(!first.owns_material(&b));
    assert!(!a.same_as(&b));
    let mut again = SkateboardSurfaceProjection::default();
    first.dispatch_from_published_type(1, &mut again).unwrap();
    assert!(a.same_as(&again.material.unwrap()));
    let value = **a.material_arc();
    drop(first);
    assert_eq!(**a.material_arc(), value);
}
#[test]
fn constructor_failures_preserve_partial_materials_and_dispatch_selected_prefix() {
    for failure in [6, 15, 18, 23] {
        let (mut owner, _) = owner();
        let mut reads = Reads {
            fail: failure,
            ..Reads::default()
        };
        assert_eq!(
            owner.try_initialize_materials(&mut reads),
            Err(SkateboardSurfaceConstructionError::Attribute(failure))
        );
        assert_eq!(
            owner.material_prefix()[0].restitution.to_bits(),
            3.0_f32.to_bits()
        );
        if failure == 6 {
            assert_eq!(owner.material_prefix()[1], MATERIAL_DEFAULT);
        }
        if failure == 15 {
            assert_eq!(
                owner.material_prefix()[4].restitution.to_bits(),
                0.4_f32.to_bits()
            );
        }
        if failure == 18 {
            assert_eq!(
                owner.material_prefix()[0].static_friction.to_bits(),
                0.5_f32.to_bits()
            );
            assert_eq!(
                owner.material_prefix()[0].dynamic_friction.to_bits(),
                0.3_f32.to_bits()
            );
        }
        if failure == 23 {
            assert_eq!(
                owner.material_prefix()[1].dynamic_friction.to_bits(),
                20.0_f32.to_bits()
            );
            assert_eq!(
                owner.material_prefix()[2].dynamic_friction.to_bits(),
                0.3_f32.to_bits()
            );
        }
        let prefix = owner.material_prefix();
        let before = reads.events.clone();
        assert_eq!(
            owner.try_initialize_materials(&mut reads),
            Err(SkateboardSurfaceConstructionError::AlreadyStarted)
        );
        assert_eq!(owner.material_prefix(), prefix);
        assert_eq!(reads.events, before);
        let mut output = SkateboardSurfaceProjection::default();
        assert_eq!(
            owner.dispatch_from_published_type(6, &mut output),
            Err(SkateboardSurfaceDispatchError::MaterialsNotReady)
        );
        assert_eq!(output.physics_type, Some(3));
        assert!(output.selected.is_some());
        assert!(output.material.is_none());
    }
}
struct Writes {
    events: Vec<u32>,
    fail: usize,
    output: SkateboardSurfaceProjection,
}
impl SkateboardSurfacePublication for Writes {
    type Error = usize;
    fn publish_physics_type(&mut self, value: u32) -> Result<(), usize> {
        self.events.push(value);
        if self.events.len() == self.fail {
            return Err(self.fail);
        }
        self.output.physics_type = Some(value);
        Ok(())
    }
    fn publish_collection(&mut self, value: SkateboardSelectedSurface) -> Result<(), usize> {
        self.events.push(100);
        if self.events.len() == self.fail {
            return Err(self.fail);
        }
        self.output.selected = Some(value);
        Ok(())
    }
    fn publish_material(&mut self, value: SkateboardSelectedMaterial) -> Result<(), usize> {
        self.events.push(200);
        if self.events.len() == self.fail {
            return Err(self.fail);
        }
        self.output.material = Some(value);
        Ok(())
    }
}
#[test]
fn dispatch_normalizes_then_publishes_collection_before_material() {
    let (mut owner, mut attributes) = owner();
    owner.try_initialize_materials(&mut attributes).unwrap();
    for raw in [0, 1, 2, 3, 4, 5, 6, 7, u32::MAX] {
        let normalized = match raw {
            1..=5 => raw,
            6 => 3,
            _ => 1,
        };
        let mut writes = Writes {
            events: vec![],
            fail: 0,
            output: SkateboardSurfaceProjection::default(),
        };
        owner
            .dispatch_from_published_type(raw, &mut writes)
            .unwrap();
        let mut expected = vec![raw];
        if raw != normalized {
            expected.push(normalized);
        }
        expected.extend([100, 200]);
        assert_eq!(writes.events, expected);
        assert_eq!(writes.output.physics_type, Some(normalized));
        assert_eq!(
            attributes
                .surface_float(
                    writes.output.selected.as_ref().unwrap(),
                    SlideSurfaceField::BrakingScalar
                )
                .unwrap()
                .to_bits(),
            0
        );
    }
    let mut writes = Writes {
        events: vec![],
        fail: 4,
        output: SkateboardSurfaceProjection::default(),
    };
    assert_eq!(
        owner.dispatch_from_published_type(6, &mut writes),
        Err(SkateboardSurfaceDispatchError::Publication(4))
    );
    assert_eq!(writes.output.physics_type, Some(3));
    assert!(writes.output.selected.is_some());
    assert!(writes.output.material.is_none());
}

#[test]
fn dispatch_retains_distinct_original_collection_owners_for_late_brake_read() {
    use crate::attribute_runtime::tests::{collection, set_dependency_bytes};
    let class = SkateboardSlideCollisionVaultAttributes::SURFACE_CLASS;
    let mut vault = graph(class, vec![]);
    let mut bytes = vec![0; 6 * 0x10c];
    for (index, key) in COLLECTIONS.into_iter().enumerate() {
        let start = index * 0x10c;
        bytes[start + 0x108..start + 0x10c]
            .copy_from_slice(&(f32::from(u16::try_from(index).unwrap()) + 10.0).to_be_bytes());
        collection(
            &mut vault,
            class,
            key,
            0,
            Some(u32::try_from(start).unwrap()),
            vec![],
        );
    }
    set_dependency_bytes(&mut vault, 1, bytes);
    let vault = Arc::new(vault);
    let mut attributes = SkateboardSlideCollisionVaultAttributes::bind(&vault);
    let mut owner = SkateboardProcessedSurface::bind(&attributes);
    owner.try_initialize_materials(&mut attributes).unwrap();
    for (raw, expected) in [
        (0, 11.0_f32),
        (1, 11.0),
        (2, 12.0),
        (3, 13.0),
        (4, 14.0),
        (5, 15.0),
        (6, 13.0),
    ] {
        let mut output = SkateboardSurfaceProjection::default();
        owner
            .dispatch_from_published_type(raw, &mut output)
            .unwrap();
        let token = output.selected.unwrap();
        assert_eq!(
            attributes
                .surface_float(&token, SlideSurfaceField::BrakingScalar)
                .unwrap()
                .to_bits(),
            expected.to_bits()
        );
        let mut foreign = SkateboardSlideCollisionVaultAttributes::bind(&vault);
        assert!(
            foreign
                .surface_float(&token, SlideSurfaceField::BrakingScalar)
                .is_err()
        );
    }
}

/// Existing original corpus can validate construction and same-token brake read.
pub(crate) fn check_original(vault: &Arc<AttributeRuntimeVault>) {
    let mut attributes = SkateboardSlideCollisionVaultAttributes::bind(vault);
    let mut owner = SkateboardProcessedSurface::bind(&attributes);
    owner.try_initialize_materials(&mut attributes).unwrap();
    for (raw, expected) in [
        (1, 0x3f80_0000),
        (2, 0x3f99_999a),
        (3, 0x4000_0000),
        (4, 0x3f00_0000),
        (5, 0x4000_0000),
        (6, 0x4000_0000),
    ] {
        let mut output = SkateboardSurfaceProjection::default();
        owner
            .dispatch_from_published_type(raw, &mut output)
            .unwrap();
        assert_eq!(
            attributes
                .surface_float(
                    output.selected.as_ref().unwrap(),
                    SlideSurfaceField::BrakingScalar
                )
                .unwrap()
                .to_bits(),
            expected
        );
    }
}
