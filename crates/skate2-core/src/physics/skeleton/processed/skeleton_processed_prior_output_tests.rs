use super::*;
use crate::physical_player_phys_out::{PhysOutOrigin, PhysOutRemainder, PhysOutRemainderStage};

struct ConstructorFixture;
impl PhysOutRemainder for ConstructorFixture {
    type Error = ();
    fn run(&mut self, _: PhysOutRemainderStage) -> Result<(), ()> {
        Ok(())
    }
}
#[derive(Default)]
struct Gaps {
    events: Vec<SkeletonPriorOutputGap>,
    fail: Option<SkeletonPriorOutputGap>,
    lock: Option<SkeletonPriorOutputGap>,
}
impl SkeletonPriorOutputGaps for Gaps {
    type Error = &'static str;
    fn run(
        &mut self,
        gap: SkeletonPriorOutputGap,
        output: &mut PhysicalPlayerPhysOut,
    ) -> Result<(), Self::Error> {
        self.events.push(gap);
        if self.lock == Some(gap) {
            output.lock_after_update1().unwrap();
        }
        if self.fail == Some(gap) {
            Err("gap")
        } else {
            Ok(())
        }
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut output = PhysicalPlayerPhysOut::new();
    output.try_construct(&mut ConstructorFixture).unwrap();
    output
}

#[test]
fn ground_takeoff_updates_the_existing_velocity_view_without_replacing_other_fields() {
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    let com = processed.prior_output().centre_of_mass();
    let gravity = processed.gravity();
    let velocity = SkeletonRootVector {
        xyz: [3.0, 4.0, -5.0],
        w: None,
    };
    processed.publish_ground_launch_velocity(velocity);
    assert_eq!(
        processed.prior_output().velocity().unwrap(),
        PhysOutValue {
            value: velocity,
            origin: PhysOutOrigin::Publication,
        }
    );
    assert_eq!(processed.prior_output().centre_of_mass(), com);
    assert_eq!(processed.gravity(), gravity);
}

#[test]
fn spatial_reset_and_late_copy_failure_preserve_exact_origins() {
    let mut processed = SkeletonProcessedContact::new();
    assert!(processed.gravity().is_none());
    assert!(processed.prior_output().centre_of_mass().is_none());
    processed.reset();
    assert_eq!(processed.gravity().unwrap().value.to_bits(), 0xc11c_cccd);
    assert_eq!(processed.gravity().unwrap().origin, PhysOutOrigin::Reset);
    let mut output = output();
    let mut gaps = Gaps {
        lock: Some(SkeletonPriorOutputGap::BeforeCentreOfMassVelocity),
        ..Gaps::default()
    };
    assert_eq!(
        processed.try_copy_prior_output(&mut output, &mut gaps),
        Err(SkeletonPriorOutputError::Output(
            PhysOutAccessError::LockedForWrite
        ))
    );
    assert_eq!(
        processed.prior_output().centre_of_mass().unwrap().origin,
        PhysOutOrigin::Constructor
    );
    assert_eq!(
        processed
            .prior_output()
            .centre_of_mass_velocity()
            .unwrap()
            .origin,
        PhysOutOrigin::Reset
    );
    assert!(!processed.prior_output().selected_copies_complete());
}

#[test]
fn selected_copy_order_retains_constructor_provenance_without_locking() {
    use SkeletonPriorOutputGap::{
        BeforeAngularVelocity, BeforeCentreOfMass, BeforeCentreOfMassVelocity, BeforeDeckSpeed,
        BeforeDeckVelocity, BeforeGroundNormal, BeforeReckoningUp, BeforeSurfaceSpeed,
        BeforeSurfaceVelocity, BeforeZSpeed,
    };
    let mut processed = SkeletonProcessedContact::new();
    let mut output = output();
    let mut gaps = Gaps::default();
    processed
        .try_copy_prior_output(&mut output, &mut gaps)
        .unwrap();
    assert_eq!(
        gaps.events,
        [
            BeforeDeckVelocity,
            BeforeSurfaceVelocity,
            BeforeZSpeed,
            BeforeSurfaceSpeed,
            BeforeDeckSpeed,
            BeforeAngularVelocity,
            BeforeCentreOfMass,
            BeforeCentreOfMassVelocity,
            BeforeReckoningUp,
            BeforeGroundNormal
        ]
    );
    assert_eq!(
        processed.prior_output().ground_normal().unwrap().origin,
        PhysOutOrigin::Constructor
    );
    assert!(processed.prior_output().selected_copies_complete());
    assert_eq!(output.locked_for_write(), Some(false));
    assert!(processed.ready(0).is_none());
}

#[test]
fn late_gap_failure_retains_only_executed_copy_prefix() {
    let mut processed = SkeletonProcessedContact::new();
    let mut output = output();
    let mut gaps = Gaps {
        fail: Some(SkeletonPriorOutputGap::BeforeReckoningUp),
        ..Gaps::default()
    };
    assert_eq!(
        processed.try_copy_prior_output(&mut output, &mut gaps),
        Err(SkeletonPriorOutputError::Gap("gap"))
    );
    assert!(processed.prior_output().angular_velocity().is_some());
    assert!(processed.prior_output().reckoning_up().is_none());
    assert!(processed.prior_output().ground_normal().is_none());
    assert!(!processed.prior_output().selected_copies_complete());
}

#[test]
fn each_late_getter_rechecks_same_output_lock() {
    let mut processed = SkeletonProcessedContact::new();
    let mut output = output();
    let mut gaps = Gaps {
        lock: Some(SkeletonPriorOutputGap::BeforeSurfaceVelocity),
        ..Gaps::default()
    };
    assert_eq!(
        processed.try_copy_prior_output(&mut output, &mut gaps),
        Err(SkeletonPriorOutputError::Output(
            PhysOutAccessError::LockedForWrite
        ))
    );
    assert!(processed.prior_output().velocity().is_some());
    assert!(processed.prior_output().surface_velocity().is_none());
    assert_eq!(gaps.events.len(), 2);
}

#[test]
fn published_unknown_w_and_negative_zero_survive_copy_and_invalidation() {
    let mut processed = SkeletonProcessedContact::new();
    let mut output = output();
    let guard = output.sample_write_guard().unwrap();
    output
        .publish_motion_vector(
            &guard,
            PhysOutMotionVector::DeckVelocity,
            SkeletonRootVector {
                xyz: [1.0, -0.0, 4.0],
                w: None,
            },
        )
        .unwrap();
    output
        .publish_motion_scalar(&guard, PhysOutMotionScalar::ZSpeed, -0.0)
        .unwrap();
    processed
        .try_copy_prior_output(&mut output, &mut Gaps::default())
        .unwrap();
    let state = processed.prior_output();
    assert_eq!(state.velocity().unwrap().value.w, None);
    assert_eq!(state.velocity().unwrap().origin, PhysOutOrigin::Publication);
    assert_eq!(
        state.z_speed().unwrap().value.to_bits(),
        (-0.0_f32).to_bits()
    );
    processed.prior_output.invalidate();
    assert!(!processed.prior_output().selected_copies_complete());
    assert_eq!(
        processed.prior_output().z_speed().unwrap().value.to_bits(),
        (-0.0_f32).to_bits()
    );
}

#[test]
fn restarted_copy_failure_keeps_previous_late_fields_but_clears_readiness() {
    let mut processed = SkeletonProcessedContact::new();
    let mut output = output();
    processed
        .try_copy_prior_output(&mut output, &mut Gaps::default())
        .unwrap();
    let previous = processed.prior_output().ground_normal();
    let mut gaps = Gaps {
        fail: Some(SkeletonPriorOutputGap::BeforeDeckVelocity),
        ..Gaps::default()
    };
    processed
        .try_copy_prior_output(&mut output, &mut gaps)
        .unwrap_err();
    assert_eq!(processed.prior_output().ground_normal(), previous);
    assert!(!processed.prior_output().selected_copies_complete());
}

#[test]
fn foreign_output_is_rejected_before_gaps_and_binding_survives_reset() {
    let mut processed = SkeletonProcessedContact::new();
    let mut first = output();
    processed
        .try_copy_prior_output(&mut first, &mut Gaps::default())
        .unwrap();
    processed.reset();
    let reset_velocity = processed.prior_output().velocity().unwrap();
    assert_eq!(
        reset_velocity.origin,
        crate::physical_player_phys_out::PhysOutOrigin::Reset
    );
    assert_eq!(reset_velocity.value, SkeletonRootVector::known([0.0; 4]));
    let mut foreign = output();
    let mut gaps = Gaps::default();
    assert_eq!(
        processed.try_copy_prior_output(&mut foreign, &mut gaps),
        Err(SkeletonPriorOutputError::ForeignOutput)
    );
    assert!(gaps.events.is_empty());
    assert_eq!(processed.prior_output().velocity(), Some(reset_velocity));
    assert!(!processed.prior_output().selected_copies_complete());
    processed
        .try_copy_prior_output(&mut first, &mut gaps)
        .unwrap();
}

struct ToolkitFixture;
impl crate::skeleton_processed_contact::SkeletonProcessedContactBoard for ToolkitFixture {
    type Error = ();
    fn mass(&mut self) -> Result<f32, ()> {
        Ok(1.0)
    }
    fn deck_transform(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootMatrix, ()> {
        Ok(
            crate::skeleton_root_transforms::SkeletonRootMatrix::from_columns([
                SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]),
                SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
                SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]),
                SkeletonRootVector::known([0.0; 4]),
            ]),
        )
    }
    fn flags_8c4(&self) -> u32 {
        0
    }
}
impl crate::skeleton_processed_contact::toolkit::SkeletonProcessedToolkitInputRead
    for ToolkitFixture
{
    type Error = ();
    fn flags_8c4(&mut self) -> Result<u32, ()> {
        Ok(0)
    }
    fn z_speed(&mut self) -> Result<f32, ()> {
        Ok(1.0)
    }
}
impl crate::skeleton_processed_contact::toolkit::SkeletonProcessedToolkitJumpInputRead
    for ToolkitFixture
{
    type Error = ();
    fn ground_normal(&mut self) -> Result<SkeletonRootVector, ()> {
        Ok(SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]))
    }
}
impl crate::skeleton_processed_contact::toolkit::SkeletonProcessedToolkitJumpBoard
    for ToolkitFixture
{
    type Error = ();
    fn jump_normal(&mut self) -> Result<SkeletonRootVector, ()> {
        Ok(SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]))
    }
    fn set_jump_normal(&mut self, value: SkeletonRootVector) -> Result<(), ()> {
        assert_eq!(value, SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]));
        Ok(())
    }
}
impl crate::skeleton_processed_contact::toolkit::SkeletonProcessedToolkitJumpCvars
    for ToolkitFixture
{
    type Error = ();
    type FloatTarget = ();
    type BoolTarget = ();
    fn capture_float(
        &mut self,
        _: crate::skeleton_processed_contact::toolkit::ToolkitJumpFloat,
    ) -> Result<(), ()> {
        Ok(())
    }
    fn float(&mut self, (): &()) -> Result<f32, ()> {
        Ok(0.5)
    }
    fn capture_draw_jump(&mut self) -> Result<(), ()> {
        Ok(())
    }
    fn draw_jump(&mut self, (): &()) -> Result<bool, ()> {
        Ok(false)
    }
    fn require_overlay(&mut self) -> Result<(), ()> {
        panic!("draw disabled")
    }
}

#[test]
fn failed_prior_copy_invalidates_completed_toolkit_without_erasing_its_fields() {
    let mut processed = SkeletonProcessedContact::new();
    let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
    processed
        .try_sample_toolkit_prefix(&mut ToolkitFixture)
        .unwrap();
    processed
        .try_continue_toolkit_directions(&mut ToolkitFixture, &mut numeric)
        .unwrap();
    processed
        .try_continue_toolkit_jump_normal(
            &mut ToolkitFixture,
            &mut ToolkitFixture,
            &mut ToolkitFixture,
            &mut numeric,
        )
        .unwrap();
    assert!(processed.ready(0).is_some());
    assert!(processed.toolkit_state().directions_complete());
    assert!(processed.toolkit_state().jump_tail_complete());
    let direction = processed.toolkit_state().body_z_direction();
    let jump = processed.toolkit_state().processed_jump_normal();
    let mut gaps = Gaps {
        fail: Some(SkeletonPriorOutputGap::BeforeZSpeed),
        ..Gaps::default()
    };
    processed
        .try_copy_prior_output(&mut output(), &mut gaps)
        .unwrap_err();
    assert!(processed.ready(0).is_none());
    assert!(!processed.toolkit_state().directions_complete());
    assert!(!processed.toolkit_state().jump_tail_complete());
    assert_eq!(processed.toolkit_state().body_z_direction(), direction);
    assert_eq!(processed.toolkit_state().processed_jump_normal(), jump);
    assert!(processed.prior_output().surface_velocity().is_some());
}
