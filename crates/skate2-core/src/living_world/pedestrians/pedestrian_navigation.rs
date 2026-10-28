//! Retained Navigation base82e6ba80/Init82e75358. Mesh seeking and A* remain
//! distinct owners; this base never substitutes interpolation for their paths.
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    reckoning_numeric::{ReckoningNumeric, try_normalize_angle, try_normalize_unsafe},
    skeleton_root_transforms::SkeletonRootVector,
};
#[path = "pedestrian_navigation_mesh.rs"]
pub(crate) mod mesh;
#[path = "pedestrian_navigation_road.rs"]
pub(crate) mod road;
#[path = "pedestrian_navigation_wander.rs"]
pub(crate) mod wander;
#[path = "pedestrian_navigation_rig.rs"]
pub(crate) mod rig;

pub(crate) const NAVIGATION_PARAMETERS: u64 = 0x4a7a_c010_4ee5_1208;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NavigationTargetMode {
    #[default]
    Auto,
    Explicit,
    ExplicitNoSeekNoAstar,
}

#[derive(Debug)]
pub(crate) enum NavigationError<E> {
    Attribute(AttributeRuntimeError),
    MissingNavigationParameters,
    Numeric(E),
}

pub(crate) struct PedestrianNavigation {
    pub(crate) parameters: AttributeRuntimeInstance,
    pub(crate) position: [f32; 4],
    pub(crate) heading: f32,
    pub(crate) seek_destination: [f32; 4],
    pub(crate) path_target: [f32; 4],
    pub(crate) seek_direction: [f32; 4],
    pub(crate) seek_flag: bool,
    pub(crate) seek_speed: f32,
    pub(crate) ramped_seek_speed: f32,
    pub(crate) speed_ramping: bool,
    pub(crate) max_speed_increment: f32,
    pub(crate) seek_turn: f32,
    pub(crate) seek_heading: f32,
    pub(crate) arrival_radius: f32,
    pub(crate) arrival_y_threshold: f32,
    pub(crate) strafe_target: [f32; 4],
    pub(crate) use_strafe_target: bool,
    pub(crate) rotate_to_target: bool,
    pub(crate) target_mode: NavigationTargetMode,
    pub(crate) velocity_override: Option<f32>,
}
impl PedestrianNavigation {
    /// Init binds the entity's full RefSpec; no replacement navigation tuning.
    pub(crate) fn initialize(
        entity: &AttributeRuntimeInstance,
        position: [f32; 4],
        heading: f32,
    ) -> Result<Self, NavigationError<std::convert::Infallible>> {
        let parameters = entity
            .try_ref_spec_instance(NAVIGATION_PARAMETERS)
            .map_err(NavigationError::Attribute)?
            .ok_or(NavigationError::MissingNavigationParameters)?;
        Ok(Self {
            parameters,
            position,
            heading,
            seek_destination: position,
            path_target: position,
            seek_direction: [0.; 4],
            seek_flag: false,
            seek_speed: 0.,
            ramped_seek_speed: 0.5,
            speed_ramping: false,
            max_speed_increment: 0.5,
            seek_turn: 0.,
            seek_heading: heading,
            arrival_radius: 1.,
            arrival_y_threshold: 1.,
            strafe_target: [0.; 4],
            use_strafe_target: false,
            rotate_to_target: true,
            target_mode: NavigationTargetMode::Auto,
            velocity_override: None,
        })
    }
    /// ResetNavigation82e6bcd0 preserves constructor/tuning fields.
    pub(crate) fn reset(&mut self, position: [f32; 4], heading: f32) {
        self.position = position;
        self.heading = heading;
        self.seek_destination = position;
        self.path_target = position;
        self.seek_direction = [0.; 4];
        self.strafe_target = [0.; 4];
        self.seek_flag = false;
        self.seek_turn = 0.;
        self.seek_speed = 0.;
        self.arrival_radius = 1.;
        self.arrival_y_threshold = 1.;
        self.seek_heading = heading;
        self.use_strafe_target = false;
        self.ramped_seek_speed = 0.5;
        self.speed_ramping = false;
        self.target_mode = NavigationTargetMode::Auto;
    }
    /// Update82e6bb70 raw instructions: compare against the increment BEFORE
    /// dt multiplication, then add/subtract increment*dt. No invented clamp.
    pub(crate) fn update_speed(&mut self, dt: f32) {
        let difference = self.seek_speed - self.ramped_seek_speed;
        self.ramped_seek_speed = if difference.abs() >= self.max_speed_increment {
            let increment = self.max_speed_increment * dt;
            if difference >= 0. {
                self.ramped_seek_speed + increment
            } else {
                self.ramped_seek_speed - increment
            }
        } else {
            self.seek_speed
        };
    }
    pub(crate) fn speed(&self) -> f32 {
        if self.speed_ramping {
            self.ramped_seek_speed
        } else {
            self.seek_speed
        }
    }
    pub(crate) fn set_strafe_target(&mut self, target: [f32; 4]) {
        self.strafe_target = target;
        self.use_strafe_target = true;
    }
    /// HasArrivedXZ82e75468: inclusive XZ circle, strict vertical threshold.
    /// Entity position is supplied because the native getter reads the entity,
    /// not Navigation's potentially earlier position publication.
    pub(crate) fn has_arrived_xz<N: ReckoningNumeric>(
        &self,
        entity_position: [f32; 4],
        target: [f32; 4],
        radius: f32,
        numeric: &mut N,
    ) -> Result<bool, N::Error> {
        let delta = [
            entity_position[0] - target[0],
            0.,
            entity_position[2] - target[2],
            0.,
        ];
        if numeric.dot3(delta, delta)? - radius * radius > 0. {
            return Ok(false);
        }
        Ok(self.arrival_y_threshold > (entity_position[1] - target[1]).abs())
    }
    /// UpdateSeekDirection82e6bdd8 uses unsafe normalize then retains the old
    /// direction when every absolute XYZ component fails its epsilon compare.
    pub(crate) fn update_seek_direction<N: ReckoningNumeric>(
        &mut self,
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        let delta: [f32; 4] = std::array::from_fn(|i| self.path_target[i] - self.position[i]);
        let direction = try_normalize_unsafe(delta, numeric)?;
        if direction[..3]
            .iter()
            .any(|v| v.abs() > f32::from_bits(0x34000000))
        {
            self.seek_direction = direction;
        }
        Ok(())
    }
    /// SetPathTarget82e7daf0/SetSeekDestination82e7db70 share original Euler
    /// math and angle normalization already owned by the camera math module.
    pub(crate) fn set_path_target<N: ReckoningNumeric>(
        &mut self,
        target: [f32; 4],
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        self.path_target = target;
        let delta =
            SkeletonRootVector::known(std::array::from_fn(|i| target[i] - self.position[i]));
        self.seek_heading = crate::camera_compass::runtime::math::euler_angles(delta, numeric)?.1;
        self.seek_turn = try_normalize_angle(self.seek_heading - self.heading, numeric)?;
        self.update_seek_direction(numeric)
    }
    pub(crate) fn set_seek_destination<N: ReckoningNumeric>(
        &mut self,
        target: [f32; 4],
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        self.seek_destination = target;
        self.set_path_target(target, numeric)
    }
}
