//! Left-focus APT HUD projection:826082b0, ViewMan::Init82848f30 and
//! ViewCamera::SetPerspectiveMatrixRadians82840770. The inverse APT view
//! cancels the same view at submission; retain the two rotated basis axes.
use crate::reckoning_sin_cos::{HostReckoningSinCosNumeric, sine::try_sine, try_sin_cos};

/// The degrees-to-radians factor the 3D matrices multiply by.
const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);

#[derive(Clone, Copy, Debug)]
pub struct FocusLeftProjection {
    x_axis: [f32; 3],
    y_axis: [f32; 3],
    distance: f32,
    projection: [f32; 2],
}
impl FocusLeftProjection {
    pub fn new(angle_degrees: f32, distance: f32) -> Self {
        Self::turned((180.0 - angle_degrees) * DEGREES_TO_RADIANS, distance)
    }
    /// The view half-turned about X (`pi`) and turned about Y by `turn`
    /// radians, `distance` in front of the camera.
    fn turned(turn: f32, distance: f32) -> Self {
        let mut numeric = HostReckoningSinCosNumeric;
        let (sx, cx) = try_sin_cos(f32::from_bits(0x40490fdb), &mut numeric).unwrap();
        let (sy, cy) = try_sin_cos(turn, &mut numeric).unwrap();
        let rotated_y = [0.0, cx, sx];
        let rotated_z = [0.0, -sx, cx];
        let original_x = [1.0, 0.0, 0.0];
        let x_axis = std::array::from_fn(|i| original_x[i] * cy - rotated_z[i] * sy);
        let fov = f32::from_bits(0x3f860a92) * f32::from_bits(0x3f22f983);
        let horizontal = (f64::from(fov) * 1.777777777777778) as f32;
        Self {
            x_axis,
            y_axis: rotated_y,
            distance,
            projection: [
                (-1.0 / f64::from(horizontal)) as f32,
                (1.0 / f64::from(fov)) as f32,
            ],
        }
    }
    /// `SetApt3DFocusRightMatrix` (`0x82608748`): the same view rotated by
    /// `180 + angle` instead of `180 - angle` (the `g3dHUDRight` container).
    pub fn focus_right(angle_degrees: f32, distance: f32) -> Self {
        Self::new(-angle_degrees, distance)
    }
    /// `SetApt3DSpinMatrix` (`0x82608be0`) for the `g3dSpinContainer` and
    /// `g3dHUDSpin` screens: the same view turned by the spin angle itself
    /// (`AptIntegration::mCurrentSpinAngle`, degrees), without the HUD's
    /// landing jolt.
    pub fn spin(angle_degrees: f32, distance: f32) -> Self {
        Self::turned(angle_degrees * DEGREES_TO_RADIANS, distance)
    }
    /// Homogeneous position, with depth testing disabled by this HUD layer.
    pub fn project(&self, point: [f32; 2]) -> [f32; 4] {
        let x = point[0] - 640.0;
        let y = point[1] - 360.0;
        let v: [f32; 3] = std::array::from_fn(|i| self.y_axis[i].mul_add(y, self.x_axis[i] * x));
        [
            v[0] * self.projection[0],
            v[1] * self.projection[1],
            0.0,
            v[2] + self.distance,
        ]
    }
}

/// `kfDistanceOf3D`'s default: the 3D HUD's distance from the camera.
pub const DISTANCE_OF_3D: f32 = 624.0;
/// `kfSpinStartAngle`, `kfSpinDistance` (the angle the intro turns to),
/// `kfSpinFrame`, `kfSpinAmbience` and `kfSpinRotationDelta` defaults (their
/// `cVarFloatRef` initialisers `0x82f57db0`, `0x82f57cf0`, `0x82f57d50`,
/// `0x82f57e10`, `0x82f57e70`).
pub const SPIN_START_ANGLE: f32 = 90.0;
pub const SPIN_DISTANCE: f32 = 550.0;
pub const SPIN_FRAMES: f32 = 20.0;
pub const SPIN_AMBIENCE: f32 = f32::from_bits(0x3d23_d70a);
pub const SPIN_ROTATION_DELTA: f32 = f32::from_bits(0x3c23_d70a);

/// `AptIntegration`'s `eSpinMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpinMode {
    #[default]
    None,
    Intro,
    Idle,
    Outro,
}

/// The spin of the `g3dSpinContainer`/`g3dHUDSpin` screens, one for the
/// whole front end: `ScreenManager`'s clip sequence (`0x0e86`) calls
/// `Game.On3DIntroStart` for such a screen's `intro` and `On3DIntroExit`
/// for its `outro`; `AptIntegration::Update` (`0x826014c0`) advances it
/// once per front-end update, before the movies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spin {
    pub mode: SpinMode,
    /// `mCurrentSpinAngle`, in degrees.
    pub angle: f32,
    /// `mSpinTarget`.
    target: f32,
    /// `mCurrentSpinDelta`.
    delta: f32,
    /// `mIdleSpinAngle`.
    idle_angle: f32,
}

impl Default for Spin {
    /// `AptIntegration::Initialize` (`0x8263e6e8`).
    fn default() -> Self {
        Self { mode: SpinMode::None, angle: 0.0, target: SPIN_START_ANGLE, delta: 0.0, idle_angle: 0.0 }
    }
}

impl Spin {
    /// `GameCmdTranslator::sMethod_On3DIntroStart` (`0x8261af98`): from the
    /// start angle to `kfSpinDistance` over `kfSpinFrame` updates.
    pub fn intro_start(&mut self) {
        self.mode = SpinMode::Intro;
        self.angle = SPIN_START_ANGLE;
        self.target = SPIN_DISTANCE;
        self.delta = (SPIN_DISTANCE - SPIN_START_ANGLE) / SPIN_FRAMES;
        self.idle_angle = 0.0;
    }
    /// `sMethod_On3DIntroExit` (`0x8261b070`): back to the start angle over
    /// `kfSpinFrame` updates.
    pub fn intro_exit(&mut self) {
        self.mode = SpinMode::Outro;
        self.target = SPIN_START_ANGLE;
        self.delta = (SPIN_START_ANGLE - self.angle) / SPIN_FRAMES;
    }
    /// One `AptIntegration::Update`: the intro and outro step to their
    /// target; idle sways by `XMVectorSin` of the idle angle times
    /// `kfSpinAmbience`.
    pub fn update(&mut self) {
        match self.mode {
            SpinMode::None => {}
            SpinMode::Intro => {
                self.angle = self.delta + self.angle;
                if self.angle >= self.target {
                    self.angle = self.target;
                    self.mode = SpinMode::Idle;
                }
            }
            SpinMode::Idle => {
                let sine = try_sine(self.idle_angle, &mut HostReckoningSinCosNumeric).unwrap_or_else(|e| match e {});
                self.angle -= sine * SPIN_AMBIENCE;
                self.idle_angle = SPIN_ROTATION_DELTA + self.idle_angle;
                self.target = SPIN_START_ANGLE;
            }
            SpinMode::Outro => {
                self.angle = self.delta + self.angle;
                if self.angle <= self.target {
                    self.angle = self.target;
                    self.mode = SpinMode::None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stage_centre_and_left_focus_perspective() {
        let p = FocusLeftProjection::new(11.25, 624.);
        assert_eq!(p.project([640., 360.]), [0., 0., 0., 624.]);
        let left = p.project([100., 100.]);
        let right = p.project([1180., 100.]);
        assert!(left[0] < 0. && right[0] > 0.);
        assert!(left[1] > 0. && right[1] > 0.);
        assert!(left[3] < right[3]);
    }

    #[test]
    fn spin_intro_turns_in_twenty_updates_then_sways_and_returns() {
        let mut spin = Spin::default();
        spin.intro_start();
        for _ in 0..19 {
            spin.update();
            assert_eq!(spin.mode, SpinMode::Intro);
        }
        spin.update();
        assert_eq!((spin.mode, spin.angle), (SpinMode::Idle, SPIN_DISTANCE));
        for _ in 0..300 {
            spin.update();
            assert!((SPIN_DISTANCE - 8.01..=SPIN_DISTANCE).contains(&spin.angle), "{}", spin.angle);
        }
        spin.intro_exit();
        for _ in 0..20 {
            spin.update();
        }
        assert_eq!((spin.mode, spin.angle), (SpinMode::None, SPIN_START_ANGLE));
        // Edge-on at the start angle; facing the camera at 180 degrees.
        let edge = FocusLeftProjection::spin(SPIN_START_ANGLE, DISTANCE_OF_3D).project([1280., 360.]);
        assert!(edge[0].abs() < 1e-3, "{edge:?}");
        let facing = FocusLeftProjection::spin(180., DISTANCE_OF_3D);
        assert_eq!(facing.project([640., 360.]), [0., 0., 0., DISTANCE_OF_3D]);
    }
}
