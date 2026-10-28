//! Connected trajectory publications and native incline updates.
use crate::{
    camera_man::CameraMan,
    camera_subject::CameraSubject,
    input_cvar_runtime::InputCvarError,
    reckoning_numeric::{ReckoningNumeric, try_acos, try_length},
};
#[path = "camera_man_air_collision.rs"]
mod air_collision;
#[path = "camera_man_incline.rs"]
pub(crate) mod incline;
#[path = "camera_man_trajectory_math.rs"]
pub(crate) mod math;
use math::{TremorError, flat_basis, landing_incline, signed_fraction};

#[derive(Debug)]
pub(crate) enum CameraTrajectoryError<E> {
    OwnerLock,
    MissingSubject,
    MissingAnchor(u32),
    Numeric(E),
    Tremor(TremorError<E>),
    Cvar(InputCvarError),
    Query(crate::camera_collision::query::CameraQueryError),
    DebugOverlay(crate::debug_overlay::DebugOverlayError),
}

impl<E: std::fmt::Debug> std::fmt::Display for CameraTrajectoryError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("camera trajectory owner lock poisoned"),
            Self::MissingSubject => f.write_str("camera trajectory has no Subject"),
            Self::MissingAnchor(index) => write!(f, "camera trajectory anchor {index} is absent"),
            Self::Numeric(error) => write!(f, "camera trajectory numeric boundary: {error:?}"),
            Self::Tremor(error) => write!(f, "{error}"),
            Self::Cvar(error) => write!(f, "camera incline cvar: {error:?}"),
            Self::Query(error) => write!(f, "camera air collision: {error}"),
            Self::DebugOverlay(error) => write!(f, "camera frontside debug overlay: {error}"),
        }
    }
}

/// `GetIsSubjectOnGround82e3b4c8`; NaN end time takes the GT-clear return-true.
pub(crate) fn is_on_ground(subject: &CameraSubject, rig_state: u32) -> bool {
    if rig_state == 2 {
        return true;
    }
    if subject.flags.starting_to_jump || subject.flags.trajectory_valid {
        return false;
    }
    subject
        .trajectory_end_time
        .partial_cmp(&f32::from_bits(0x3ecc_cccd))
        != Some(std::cmp::Ordering::Greater)
}

fn height_blend_finished(elapsed: f32, duration: f32) -> bool {
    // Raw45248 branches only ordered LT; an unordered value continues.
    elapsed.partial_cmp(&duration) != Some(std::cmp::Ordering::Less)
}

impl CameraMan {
    /// `UpdateTrajectoryData82e44dd8`; unrecovered air callbacks retain their prefix.
    pub(crate) fn try_update_trajectory<N: ReckoningNumeric>(
        &mut self,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        overlay: &crate::debug_overlay::DebugOverlaySlot,
        services: &mut crate::simulation_assembly_backend::constraint_session::physical_world_line::camera::CameraLineServices<'_>,
        numeric: &mut N,
    ) -> Result<(), CameraTrajectoryError<N::Error>> {
        use CameraTrajectoryError as E;
        let owner = self.subject.as_ref().ok_or(E::MissingSubject)?;
        let subject = owner.lock().map_err(|_| E::OwnerLock)?;
        self.landing_relative_height =
            subject.trajectory_end_position.xyz[1] - subject.trajectory_start_position.xyz[1];
        self.maximum_prediction_height =
            subject.trajectory_apex_position.xyz[1] - subject.trajectory_start_position.xyz[1];
        self.maximum_prediction_height_time = Some(subject.trajectory_apex_time);
        self.launch_position = subject.trajectory_start_position;
        self.trajectory_height_distance_ratio = if self.launch_distance == 0.0 {
            0.0
        } else {
            self.maximum_prediction_height / self.launch_distance
        };
        self.landing_normal = subject.trajectory_end_normal;
        self.absolute_launch_incline = try_acos(
            signed_fraction(subject.trajectory_start_normal.xyz[1]),
            numeric,
        )
        .map_err(E::Numeric)?;
        self.flags &= !8;
        if is_on_ground(&subject, self.rig.subject_state) || !subject.flags.trajectory_valid {
            self.flags &= !1;
        } else {
            if self.flags & 1 == 0 {
                let anchor = subject
                    .anchor_positions
                    .get(self.current_anchor as usize)
                    .ok_or(E::MissingAnchor(self.current_anchor))?;
                let delta = self
                    .rig
                    .positioner
                    .position
                    .zip(*anchor, |position, anchor| position - anchor);
                self.launch_distance = try_length(delta, numeric).map_err(E::Numeric)?;
            }
            self.flags |= 1;
            self.absolute_landing_incline =
                try_acos(signed_fraction(self.landing_normal.xyz[1]), numeric)
                    .map_err(E::Numeric)?;
            if self.absolute_launch_incline > f32::from_bits(0x3f9c_61aa)
                && self.absolute_landing_incline > f32::from_bits(0x3f06_0a92)
            {
                self.flags |= 8;
            }
            self.transfer_angle = transfer_angle(
                subject.trajectory_start_normal,
                subject.trajectory_end_normal,
                numeric,
            )
            .map_err(E::Numeric)?;
            if cvars.try_camera_show_transfer_angle().map_err(E::Cvar)? {
                overlay.require().map_err(E::DebugOverlay)?;
            }
        }
        let basis = flat_basis(&subject, numeric).map_err(E::Numeric)?;
        self.landing_incline =
            landing_incline(basis, self.landing_normal, numeric).map_err(E::Numeric)?;
        let on_ground = is_on_ground(&subject, self.rig.subject_state);
        if on_ground {
            if self.flags & 0x20 == 0 && !self.rig.positioner.flags.colliding {
                self.shake
                    .try_tremor(self.incline_normal, self.previous_subject_velocity, numeric)
                    .map_err(E::Tremor)?;
                self.landing_relative_height = 0.0;
            }
        } else if height_blend_finished(
            self.rig.blend_height_elapsed,
            self.rig.blend_height_duration,
        ) && subject.flags.trajectory_valid
            && !subject.flags.offboard
        {
            if self.landing_relative_height > -3.0 {
                let remaining = subject.trajectory_end_time - subject.trajectory_elapsed;
                if self.second_flags & 0x80 != 0
                    || air_collision::check_air_collision(
                        &self.rig.positioner,
                        &self.rig.collision,
                        &subject,
                        cvars,
                        services,
                    )?
                {
                    self.rig.flags |= 0x40;
                } else {
                    //82e45324/330 EQ branches alone skip: unordered references continue.
                    if self.rig.reference_height_at_launch != f32::MAX
                        && self.rig.reference_height != f32::MAX
                    {
                        self.rig.blend_to_height =
                            self.rig.reference_height + self.landing_relative_height;
                        self.rig.blend_from_height = self.rig.reference_height;
                        self.rig.blend_height_duration = remaining;
                        self.rig.blend_height_elapsed = 0.0;
                    }
                }
            }
            self.flags |= 4;
        }
        self.flags = (self.flags & !0x20) | (u8::from(on_ground) << 5);
        self.previous_subject_velocity = self.rig.subject_velocity;
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_AIR_CAMERA").is_some()) {
            std::eprintln!("air-camera ground={} valid={} offboard={} state={} board_y={} start_y={} end_y={} anchor_y={} reference={} launch_reference={} elapsed={} total={} blend_elapsed={} blend_duration={} blend_from={} blend_to={} rig_flags={:x} camera_flags={:x}",
                on_ground,subject.flags.trajectory_valid,subject.flags.offboard,self.rig.subject_state,
                subject.board_position.xyz[1],subject.trajectory_start_position.xyz[1],subject.trajectory_end_position.xyz[1],
                self.rig.anchor_matrix.columns()[3].xyz[1],self.rig.reference_height,self.rig.reference_height_at_launch,
                subject.trajectory_elapsed,subject.trajectory_end_time,self.rig.blend_height_elapsed,self.rig.blend_height_duration,
                self.rig.blend_from_height,self.rig.blend_to_height,self.rig.flags,self.second_flags);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{height_blend_finished, is_on_ground};
    use crate::camera_subject::CameraSubject;
    #[test]
    fn unordered_duration_continues_but_ordered_unfinished_duration_skips() {
        assert!(height_blend_finished(f32::NAN, 1.0));
        assert!(height_blend_finished(0.0, f32::NAN));
        assert!(height_blend_finished(1.0, 1.0));
        assert!(!height_blend_finished(0.0, 1.0));
    }
    #[test]
    fn on_ground_preserves_grinding_short_circuit_and_unordered_end_time() {
        let mut subject = CameraSubject::try_construct().unwrap();
        subject.trajectory_end_time = f32::NAN;
        assert!(is_on_ground(&subject, 0));
        subject.flags.trajectory_valid = true;
        assert!(!is_on_ground(&subject, 0));
        assert!(is_on_ground(&subject, 2));
    }
}

/// UpdateTransferAngle82e41040. Scalar Math::Angle::Normalize is distinct from
/// CameraAngle::Normalize; preserve initial Normalize0 before both conditional Euler calls.
fn transfer_angle<N: ReckoningNumeric>(
    start: crate::skeleton_root_transforms::SkeletonRootVector,
    end: crate::skeleton_root_transforms::SkeletonRootVector,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    use crate::{
        camera_compass::runtime::math::euler_angles, reckoning_numeric::try_normalize_angle,
    };
    let initial = try_normalize_angle(0.0, numeric)?;
    let mut start_yaw = initial;
    let mut end_yaw = initial;
    // Raw VMX comparisons VD57/VA58/VB60 and VD52/VA53/VB55: .99 > Y.
    let start_sloped = f32::from_bits(0x3f7d70a4) > start.xyz[1];
    if start_sloped {
        start_yaw = try_normalize_angle(euler_angles(start, numeric)?.1, numeric)?;
    }
    let end_sloped = f32::from_bits(0x3f7d70a4) > end.xyz[1];
    if end_sloped {
        end_yaw = try_normalize_angle(euler_angles(end, numeric)?.1, numeric)?;
    }
    if start_sloped && end_sloped {
        Ok(try_normalize_angle(end_yaw - start_yaw, numeric)?.abs())
    } else {
        Ok(f32::from_bits(0x40490fdb))
    }
}
#[cfg(test)]
#[test]
fn transfer_angle_keeps_flat_and_unordered_fallback_and_opposing_slopes() {
    use crate::{
        reckoning_numeric::HostReckoningNumeric, skeleton_root_transforms::SkeletonRootVector as V,
    };
    let mut n = HostReckoningNumeric;
    for y in [1.0, f32::from_bits(0x3f7d70a4), f32::NAN] {
        let flat = V {
            xyz: [0.0, y, 0.0],
            w: None,
        };
        assert_eq!(
            transfer_angle(flat, flat, &mut n).unwrap().to_bits(),
            0x40490fdb
        );
    }
    let a = V::known([1.0, 0.0, 0.0, 0.0]);
    let b = V::known([-1.0, 0.0, 0.0, 0.0]);
    assert!((transfer_angle(a, b, &mut n).unwrap() - std::f32::consts::PI).abs() < 1e-5);
    assert_eq!(transfer_angle(a, a, &mut n).unwrap(), 0.0);
}
