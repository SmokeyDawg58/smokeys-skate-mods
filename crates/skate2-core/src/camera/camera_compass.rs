//! Actual anchor/compass constructor owners used by `GameCameraManager82e37c08`.
#[path = "camera_compass_runtime.rs"]
pub(crate) mod runtime;

use crate::{
    camera_attributes::CameraAttributes,
    camera_subject::{ZERO, identity},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::{
        PhysOutMotionVector, PhysOutSystemVector, PhysicalPlayerPhysOut,
        construct::ConcretePhysOutError,
    },
    reckoning_numeric::ReckoningNumeric,
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum CameraAnchorError<E> {
    OwnerLock,
    Component(ConcretePhysOutError<std::convert::Infallible>),
    Cvar(InputCvarError),
    Numeric(E),
    UnwrittenGrindPosition,
}

pub(crate) enum CameraAnchorKind {
    Skateboard {
        stance_offset: f32,
    },
    Hips,
    Air,
    Grind {
        raw_position: Option<SkeletonRootVector>,
    },
    World,
    DampedCentreOfMass,
}
pub(crate) struct CameraAnchor {
    pub(crate) kind: CameraAnchorKind,
    pub(crate) position: SkeletonRootVector,
    pub(crate) velocity: SkeletonRootVector,
    pub(crate) angular_velocity: Option<SkeletonRootVector>,
    pub(crate) filtered_position: SkeletonRootVector,
    /// Outer None is unwritten; Some(None) can represent a later null producer.
    #[expect(
        clippy::option_option,
        reason = "Retain distinct native unwritten, written null, and written nonnull pointer states"
    )]
    pub(crate) phys_out: Option<Option<Arc<PhysicalPlayerAssemblyOwner>>>,
}
pub(crate) struct CameraAnchors {
    pub(crate) points: [CameraAnchor; 6],
    pub(crate) stance_offset: f32,
}
impl CameraAnchors {
    /// `UpdateAnchorPoints82e31d58`: one teleport sample before the six updates.
    pub(crate) fn try_update<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        filtered_position: SkeletonRootVector,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), CameraAnchorError<N::Error>> {
        let teleported = owner
            .phys_out()
            .lock()
            .map_err(|_| CameraAnchorError::OwnerLock)?
            .try_state_output()
            .map_err(CameraAnchorError::Component)?
            .has_just_teleported;
        for point in &mut self.points {
            point.phys_out = Some(Some(Arc::clone(owner)));
            point.filtered_position = filtered_position;
            if let CameraAnchorKind::Skateboard { stance_offset } = &mut point.kind {
                *stance_offset = self.stance_offset;
            }
            let output = owner
                .phys_out()
                .lock()
                .map_err(|_| CameraAnchorError::OwnerLock)?;
            point.try_update(dt, teleported, &output, cvars, numeric)?;
        }
        Ok(())
    }
    /// `AnchorPointManager82e33e68` writes only position, velocity, filtered
    /// position in each base; angular velocity and `PhysOut` are untouched.
    pub(crate) fn constructed() -> Self {
        let kinds = [
            CameraAnchorKind::Skateboard { stance_offset: 0.0 },
            CameraAnchorKind::Hips,
            CameraAnchorKind::Air,
            CameraAnchorKind::Grind { raw_position: None },
            CameraAnchorKind::World,
            CameraAnchorKind::DampedCentreOfMass,
        ];
        Self {
            points: kinds.map(|kind| CameraAnchor {
                kind,
                position: ZERO,
                velocity: ZERO,
                angular_velocity: None,
                filtered_position: ZERO,
                phys_out: None,
            }),
            stance_offset: 0.0,
        }
    }
}
impl CameraAnchor {
    /// Grind's prefix `82e31c10` precedes the common `AnchorPoint::Update82e31a08`.
    fn try_update<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        teleported: bool,
        output: &PhysicalPlayerPhysOut,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), CameraAnchorError<N::Error>> {
        use CameraAnchorError::{Component, Cvar, Numeric, UnwrittenGrindPosition};
        if let CameraAnchorKind::Grind { raw_position } = &mut self.kind {
            let offset = if teleported {
                *raw_position = Some(
                    output
                        .try_camera_board_output()
                        .map_err(Component)?
                        .deck_position,
                );
                true
            } else {
                let grinds = output.try_camera_grinds_output().map_err(Component)?;
                if grinds.is_grinding {
                    *raw_position = Some(grinds.cam_target_root);
                    true
                } else {
                    false
                }
            };
            if offset {
                let raw = raw_position.ok_or(UnwrittenGrindPosition)?;
                let up = output
                    .try_camera_system_vector(PhysOutSystemVector::Up)
                    .map_err(Component)?;
                *raw_position = Some(up.zip(raw, |a, b| a.mul_add(0.5, b)));
            } else {
                let raw = raw_position.ok_or(UnwrittenGrindPosition)?;
                let velocity = output
                    .try_camera_motion_vector(PhysOutMotionVector::DeckVelocity)
                    .map_err(Component)?;
                *raw_position = Some(velocity.zip(raw, |a, b| a.mul_add(dt, b)));
            }
        }
        let old_position = self.position;
        self.position = match &self.kind {
            CameraAnchorKind::Skateboard { .. } => {
                let target = cvars.try_capture_camera_standing_offset().map_err(Cvar)?;
                let up = output
                    .try_camera_system_vector(PhysOutSystemVector::Up)
                    .map_err(Component)?;
                let position = output
                    .try_camera_board_output()
                    .map_err(Component)?
                    .deck_position;
                let offset = target.read().map_err(Cvar)?;
                up.zip(position, |a, b| a.mul_add(offset, b))
            }
            CameraAnchorKind::Hips => output
                .try_camera_system_vector(PhysOutSystemVector::CentreOfMass)
                .map_err(Component)?,
            CameraAnchorKind::Air => {
                let axis = output
                    .try_skeleton_output()
                    .map_err(Component)?
                    .skeleton_root_matrix
                    .columns()[1];
                let position = output
                    .try_camera_system_vector(PhysOutSystemVector::CentreOfMass)
                    .map_err(Component)?;
                let scaled = axis.map(|lane| lane * f32::from_bits(0x3ea8_f5c3));
                position.zip(scaled, |a, b| a - b)
            }
            CameraAnchorKind::Grind { raw_position } => {
                raw_position.ok_or(UnwrittenGrindPosition)?
            }
            CameraAnchorKind::World => ZERO,
            CameraAnchorKind::DampedCentreOfMass => {
                output
                    .try_camera_system_output()
                    .map_err(Component)?
                    .damped_centre_of_mass
            }
        };
        self.velocity = if teleported {
            ZERO
        } else {
            let delta = self.position.zip(old_position, |a, b| a - b);
            let seed = numeric.reciprocal_estimate(dt).map_err(Numeric)?;
            let reciprocal = refine_drive_reciprocal(dt, seed);
            delta.map(|lane| reciprocal * lane)
        };
        self.angular_velocity = Some(match self.kind {
            CameraAnchorKind::Skateboard { .. }
            | CameraAnchorKind::Air
            | CameraAnchorKind::Grind { .. } => output
                .try_camera_motion_vector(PhysOutMotionVector::AngularVelocity)
                .map_err(Component)?,
            _ => ZERO,
        });
        Ok(())
    }
}
#[expect(
    clippy::struct_excessive_bools,
    reason = "distinct native compass input fields"
)]
pub(crate) struct CameraCompassInput {
    pub(crate) incline_normal: SkeletonRootVector,
    pub(crate) landing_normal: SkeletonRootVector,
    pub(crate) transform: SkeletonRootMatrix,
    pub(crate) angular_velocity: SkeletonRootVector,
    pub(crate) trajectory_heading: SkeletonRootVector,
    pub(crate) trajectory_start: SkeletonRootVector,
    pub(crate) trajectory_end: SkeletonRootVector,
    pub(crate) grind_direction: SkeletonRootVector,
    pub(crate) hips_direction: SkeletonRootVector,
    pub(crate) trajectory_fraction: f32,
    pub(crate) on_ground: bool,
    pub(crate) override_velocity: bool,
    pub(crate) zero_velocity: bool,
    pub(crate) wiping_out: bool,
    pub(crate) wipeout_over: bool,
    pub(crate) grinding: bool,
    pub(crate) mirrored: bool,
}
impl CameraCompassInput {
    /// `CameraCompassInput82e31f40`; incline normal is ZERO, landing is J.
    fn constructed() -> Self {
        let forward = SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]);
        Self {
            incline_normal: ZERO,
            landing_normal: SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
            transform: identity(),
            angular_velocity: ZERO,
            trajectory_heading: forward,
            trajectory_start: ZERO,
            trajectory_end: ZERO,
            grind_direction: forward,
            hips_direction: forward,
            trajectory_fraction: 0.0,
            on_ground: true,
            override_velocity: false,
            zero_velocity: false,
            wiping_out: false,
            wipeout_over: false,
            grinding: false,
            mirrored: false,
        }
    }
}
pub(crate) struct CameraCompass {
    pub(crate) input: CameraCompassInput,
    /// Velocity, incline, trajectory and offboard headings in source order.
    pub(crate) output: [f32; 4],
    pub(crate) parameters: Arc<Mutex<CameraAttributes>>,
    pub(crate) ground_velocity_heading: f32,
    pub(crate) velocity_heading: f32,
    pub(crate) incline_heading: f32,
    pub(crate) trajectory_heading: f32,
    pub(crate) offboard_heading: f32,
    pub(crate) subject_velocity: SkeletonRootVector,
    pub(crate) deadzone_velocity: SkeletonRootVector,
    pub(crate) last_position: SkeletonRootVector,
    pub(crate) subject_on_ground: bool,
    pub(crate) wipeout_over: bool,
    pub(crate) teleported_last_frame: bool,
    pub(crate) on_ground_counter: f32,
    pub(crate) time_since_direction_change: f32,
    pub(crate) deadzone_size: f32,
    // Native Vector2 storage is a full VMX vector; all constructor lanes are zero.
    pub(crate) adaptive_position: SkeletonRootVector,
    pub(crate) adaptive_velocity: SkeletonRootVector,
    pub(crate) adaptive_size: f32,
}
impl CameraCompass {
    /// `CameraCompass82e342a8`; parameters retained without being read.
    pub(crate) fn constructed(parameters: Arc<Mutex<CameraAttributes>>) -> Self {
        Self {
            input: CameraCompassInput::constructed(),
            output: [0.0; 4],
            parameters,
            ground_velocity_heading: 0.0,
            velocity_heading: 0.0,
            incline_heading: 0.0,
            trajectory_heading: 0.0,
            offboard_heading: 0.0,
            subject_velocity: ZERO,
            deadzone_velocity: ZERO,
            last_position: ZERO,
            subject_on_ground: false,
            wipeout_over: false,
            teleported_last_frame: true,
            on_ground_counter: 0.0,
            time_since_direction_change: 0.0,
            deadzone_size: 0.0,
            adaptive_position: ZERO,
            adaptive_velocity: ZERO,
            adaptive_size: 0.0,
        }
    }
}

#[cfg(test)]
mod anchor_tests {
    use super::*;
    use crate::physical_player_phys_out::construct::PhysOutAllocation;

    struct FailNumeric;
    impl ReckoningNumeric for FailNumeric {
        type Error = &'static str;
        fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
            panic!("anchor has no dot")
        }
        fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            panic!("anchor has no rsqrt")
        }
        fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            Err("estimate")
        }
        fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
            panic!("anchor has no conversion")
        }
    }

    #[test]
    fn reciprocal_failure_keeps_new_position_and_old_velocity_angular() {
        let mut output = PhysicalPlayerPhysOut::new();
        output
            .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        let guard = output.sample_write_guard().unwrap();
        let published = SkeletonRootVector {
            xyz: [2.0, 3.0, 4.0],
            w: None,
        };
        output
            .publish_system_vector(&guard, PhysOutSystemVector::CentreOfMass, published)
            .unwrap();
        let mut anchors = CameraAnchors::constructed();
        let point = &mut anchors.points[1];
        let old = SkeletonRootVector::known([9.0; 4]);
        point.velocity = old;
        point.angular_velocity = Some(old);
        let result = point.try_update(
            0.0,
            false,
            &output,
            &InputCvarReferences::new_static_initialized(),
            &mut FailNumeric,
        );
        assert!(matches!(
            result,
            Err(CameraAnchorError::Numeric("estimate"))
        ));
        assert_eq!(
            point.position.xyz.map(f32::to_bits),
            published.xyz.map(f32::to_bits)
        );
        assert!(point.position.w.is_none());
        assert_eq!(
            point.velocity.known_lanes().unwrap().map(f32::to_bits),
            [9.0_f32.to_bits(); 4]
        );
        assert_eq!(
            point
                .angular_velocity
                .unwrap()
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [9.0_f32.to_bits(); 4]
        );
        // Teleport bypasses the same failing numeric service and reaches angular.
        point
            .try_update(
                0.0,
                true,
                &output,
                &InputCvarReferences::new_static_initialized(),
                &mut FailNumeric,
            )
            .unwrap();
        assert_eq!(
            point.velocity.known_lanes().unwrap().map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(
            point
                .angular_velocity
                .unwrap()
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [0; 4]
        );
    }

    #[test]
    fn world_raw_is_zero_and_unproduced_grind_raw_remains_unknown() {
        let mut output = PhysicalPlayerPhysOut::new();
        output
            .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        let mut anchors = CameraAnchors::constructed();
        let refs = InputCvarReferences::new_static_initialized();
        anchors.points[4].filtered_position = SkeletonRootVector::known([12.0; 4]);
        anchors.points[4]
            .try_update(1.0, true, &output, &refs, &mut FailNumeric)
            .unwrap();
        assert_eq!(
            anchors.points[4]
                .position
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [0; 4]
        );
        assert!(matches!(
            anchors.points[3].try_update(1.0, false, &output, &refs, &mut FailNumeric),
            Err(CameraAnchorError::UnwrittenGrindPosition)
        ));
        assert!(matches!(
            anchors.points[3].kind,
            CameraAnchorKind::Grind { raw_position: None }
        ));
    }
}
