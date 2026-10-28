//! Ordinary finite path of `ShakeEffect::Apply82e43700`.
//! Keyframe W remains unknown; all published results select only keyframe XYZ.
use crate::{
    camera_attributes::CameraParameterPublication,
    camera_man::CameraShake,
    camera_parameters::CameraParameters,
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric},
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};

#[derive(Debug)]
pub(crate) enum ShakeApplyError {
    OwnerLock,
    MissingParameter(&'static str),
    Numeric(ReckoningHostNumericError),
    PhaseFraction(u32),
    PhaseConversion(u32),
    KeyframeCount(u32),
    MissingKeyframe(usize),
    SetterNaN(usize),
}
impl std::fmt::Display for ShakeApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("camera shake Apply owner lock"),
            Self::MissingParameter(name) => write!(f, "camera shake parameter {name} missing"),
            Self::Numeric(error) => write!(f, "camera shake numeric error: {error}"),
            Self::PhaseFraction(bits) => write!(f, "camera shake phase fraction {bits:08x}"),
            Self::PhaseConversion(bits) => write!(f, "camera shake phase conversion {bits:08x}"),
            Self::KeyframeCount(count) => write!(f, "camera shake keyframe count {count}"),
            Self::MissingKeyframe(index) => write!(f, "camera shake keyframe {index} missing"),
            Self::SetterNaN(lane) => write!(f, "camera shake offset NaN lane {lane}"),
        }
    }
}

#[derive(Clone, Copy)]
struct Parameters {
    amplitude_curve: Matrix,
    frequency_curve: Matrix,
    amplitude_min: f32,
    amplitude_max: f32,
    amplitude_top_speed: f32,
    frequency_min: f32,
    frequency_max: f32,
    frequency_top_speed: f32,
    one_shot_frequency: f32,
    one_shot_decay: f32,
    fps: f32,
    translation: f32,
    rotation: f32,
    dutch: f32,
}
impl Parameters {
    fn read(p: &CameraParameters) -> Result<Self, ShakeApplyError> {
        fn need<T: Copy>(value: Option<T>, name: &'static str) -> Result<T, ShakeApplyError> {
            value.ok_or(ShakeApplyError::MissingParameter(name))
        }
        Ok(Self {
            amplitude_curve: need(p.shake_amplitude_curve, "shake_amplitude_curve")?,
            frequency_curve: need(p.shake_frequency_curve, "shake_frequency_curve")?,
            amplitude_min: need(p.shake_amplitude_min, "shake_amplitude_min")?,
            amplitude_max: need(p.shake_amplitude_max, "shake_amplitude_max")?,
            amplitude_top_speed: need(
                p.shake_amplitude_top_skater_speed,
                "shake_amplitude_top_skater_speed",
            )?,
            frequency_min: need(p.shake_frequency_min, "shake_frequency_min")?,
            frequency_max: need(p.shake_frequency_max, "shake_frequency_max")?,
            frequency_top_speed: need(
                p.shake_frequency_top_skater_speed,
                "shake_frequency_top_skater_speed",
            )?,
            one_shot_frequency: need(p.shake_one_shot_frequency, "shake_one_shot_frequency")?,
            one_shot_decay: need(p.shake_one_shot_decay, "shake_one_shot_decay")?,
            fps: need(p.shake_data_fps, "shake_data_fps")?,
            translation: need(
                p.shake_translation_multiplier,
                "shake_translation_multiplier",
            )?,
            rotation: need(p.shake_rotation_multiplier, "shake_rotation_multiplier")?,
            dutch: need(p.shake_dutch_multiplier, "shake_dutch_multiplier")?,
        })
    }
}

/// PPC fsel's two selections: ordered ordinary values map to [0,1].
fn source_bound(value: f32) -> f32 {
    let lower = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

/// First two lanes of native `BezierLite::Evaluate829b0c60`. Other curve lanes
/// are independent and unused by `EvaluateYatX829b1e88`'s scalar return.
#[expect(
    clippy::needless_range_loop,
    reason = "native control and lane evaluation order is explicit"
)]
fn evaluate(curve: Matrix, t: f32) -> ([f32; 2], [f32; 2]) {
    const BASIS: [[f32; 4]; 4] = [
        [-1.0, 3.0, -3.0, 1.0],
        [3.0, -6.0, 3.0, 0.0],
        [-3.0, 3.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
    ];
    let columns = curve.columns();
    let mut coefficient = [[0.0; 2]; 4];
    for polynomial in 0..4 {
        for lane in 0..2 {
            let mut value = columns[0].xyz[lane] * BASIS[polynomial][0];
            for control in 1..4 {
                value = columns[control].xyz[lane].mul_add(BASIS[polynomial][control], value);
            }
            coefficient[polynomial][lane] = value;
        }
    }
    let t2 = t * t;
    let t3 = t2 * t;
    let three_t2 = 3.0 * t2;
    let two_t = 2.0 * t;
    let mut result = [0.0; 2];
    let mut derivative = [0.0; 2];
    for lane in 0..2 {
        let mut y = coefficient[0][lane] * t3;
        y = coefficient[1][lane].mul_add(t2, y);
        y = coefficient[2][lane].mul_add(t, y);
        result[lane] = coefficient[3][lane].mul_add(1.0, y);
        let mut dy = coefficient[0][lane] * three_t2;
        dy = coefficient[1][lane].mul_add(two_t, dy);
        dy = coefficient[2][lane].mul_add(1.0, dy);
        derivative[lane] = coefficient[3][lane].mul_add(0.0, dy);
    }
    (result, derivative)
}

fn evaluate_y_at_x(curve: Matrix, x: f32) -> f32 {
    let mut t = 0.5;
    let mut result_y = 0.0;
    for iteration in 0..10 {
        let (result, derivative) = evaluate(curve, t);
        result_y = result[1];
        if iteration == 9 {
            break;
        }
        let error = x - result[0];
        if error.abs() <= 0.01 || derivative[0].abs() <= f32::from_bits(0x3400_0000) {
            break;
        }
        t += error / derivative[0];
    }
    result_y
}

/// Source multiply before FMA. Unknown keyframe W is deliberately not read.
fn interpolate_xyz(current: Vector, next: Vector, other: f32, fraction: f32) -> [f32; 3] {
    std::array::from_fn(|lane| {
        let base = current.xyz[lane] * other;
        next.xyz[lane].mul_add(fraction, base)
    })
}

fn sin_cos(angle: f32) -> (f32, f32) {
    match try_sin_cos(angle, &mut HostReckoningSinCosNumeric) {
        Ok(value) => value,
        Err(never) => match never {},
    }
}

#[expect(
    clippy::many_single_char_names,
    reason = "native rotation basis algebra"
)]
fn scratch_basis(angles: [f32; 3]) -> [[f32; 4]; 4] {
    let (sx, cx) = sin_cos(angles[0]);
    let (sy, cy) = sin_cos(angles[1]);
    let (sz, cz) = sin_cos(angles[2]);
    let p = cx * cz;
    let q = sx * sz;
    let r = cx * sz;
    let s = sx * cz;
    let x = cy * cz;
    let y = sy * s - r;
    let z = sy.mul_add(p, q);
    [
        [x, cy * sz, -sy, x],
        [y, sy.mul_add(q, p), cy * sx, y],
        [z, sy * r - s, cy * cx, z],
        [0.0; 4],
    ]
}

fn world_offset(input: Matrix, local: [f32; 3]) -> Vector {
    let i = input.columns();
    let first = i[0].map(|lane| lane.mul_add(local[0], 0.0));
    let second = i[1].zip(first, |lane, sum| lane.mul_add(local[1], sum));
    i[2].zip(second, |lane, sum| lane.mul_add(local[2], sum))
}

fn compose(input: Matrix, scratch: [[f32; 4]; 4]) -> Matrix {
    let i = input.columns();
    Matrix::from_columns(std::array::from_fn(|column| {
        let first = if column == 3 {
            i[0].zip(i[3], |axis, translation| {
                axis.mul_add(scratch[column][0], translation)
            })
        } else {
            i[0].map(|axis| axis * scratch[column][0])
        };
        let second = i[1].zip(first, |axis, sum| axis.mul_add(scratch[column][1], sum));
        i[2].zip(second, |axis, sum| axis.mul_add(scratch[column][2], sum))
    }))
}

impl CameraShake {
    /// Ordinary finite Apply output. The PPC reciprocal-estimate seed is an
    /// explicit host numeric policy; exceptional phase operations stop at a
    /// typed boundary after the native multiplier/time publication prefix.
    /// The live keyframe W remains None throughout.
    #[expect(clippy::similar_names, reason = "current and next keyframe pairs")]
    #[expect(
        clippy::too_many_lines,
        reason = "source-ordered Apply mutation schedule"
    )]
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "native branch selects positive and unordered one-shot values"
    )]
    pub(crate) fn try_apply(
        &mut self,
        dt: f32,
        subject_speed: f32,
        subject_distance: f32,
        input: Matrix,
        active: bool,
    ) -> Result<Matrix, ShakeApplyError> {
        let changed = if active {
            dt.mul_add(5.0, self.multiplier)
        } else {
            (-dt).mul_add(5.0, self.multiplier)
        };
        self.multiplier = changed;
        self.multiplier = source_bound(self.multiplier);
        let p = {
            let owner = self
                .parameters
                .lock()
                .map_err(|_| ShakeApplyError::OwnerLock)?;
            let CameraParameterPublication::Fields(fields) = &owner.parameters else {
                return Err(ShakeApplyError::MissingParameter("camera parameters"));
            };
            Parameters::read(fields)?
        };
        let amplitude = if p.amplitude_top_speed == 0.0 {
            p.amplitude_min
        } else {
            let ratio = source_bound(subject_speed / p.amplitude_top_speed);
            let curve = evaluate_y_at_x(p.amplitude_curve, ratio);
            curve.mul_add(p.amplitude_max - p.amplitude_min, p.amplitude_min)
        };
        let amplitude = (self.one_shot_effect + amplitude) * self.multiplier;
        let mut frequency = if p.frequency_top_speed == 0.0 {
            p.frequency_min
        } else {
            let ratio = source_bound(subject_speed / p.frequency_top_speed);
            let curve = evaluate_y_at_x(p.frequency_curve, ratio);
            curve.mul_add(p.frequency_max - p.frequency_min, p.frequency_min)
        };
        if !(self.one_shot_effect <= 0.0) {
            frequency = p.one_shot_frequency;
        }
        let old_time = self.time;
        // 82e43854: vrefp(+1) precedes the native new-time publication.
        let seed = HostReckoningNumeric
            .reciprocal_estimate(1.0)
            .map_err(ShakeApplyError::Numeric)?;
        let frame_step = p.fps * frequency;
        self.time = frame_step.mul_add(dt, old_time);
        let distance = source_bound(
            (f32::from_bits(0x4059_999a) - subject_distance) * f32::from_bits(0x3fd5_5552),
        );
        if std::env::var_os("SKATE2_TRACE_VERT").is_some() {
            eprintln!(
                "vert camera shake: speed={subject_speed} active={active} multiplier={} amplitude={amplitude} frequency={frequency} one_shot={} camera_distance={subject_distance} distance_scale={distance}",
                self.multiplier, self.one_shot_effect
            );
        }
        // 82e438c4/c8: two native traps follow the time store and distance
        // calculation, before the later phase fraction/conversion operations.
        let count = self.keyframes.count;
        if count == 0 {
            return Err(ShakeApplyError::KeyframeCount(count));
        }
        // 82e43854..dc: vrefp of +1 followed by two VMX Newton refinements.
        // Host estimate bits are a declared numeric policy, not a Xenon trace.
        let reciprocal_one = refine_drive_reciprocal(1.0, seed);
        // 82e438f0..43904: old_time is used, not the just-published new_time.
        let scaled_old = old_time * reciprocal_one;
        if !scaled_old.is_finite() || !self.time.is_finite() {
            return Err(ShakeApplyError::PhaseFraction(old_time.to_bits()));
        }
        let truncated_old = scaled_old.trunc(); // vrfiz128, toward zero
        let fraction = (-truncated_old).mul_add(1.0, old_time); // vnmsubfp
        let integer_source = self.time - fraction; // fsubs before fctidz
        if !fraction.is_finite() || !integer_source.is_finite() {
            return Err(ShakeApplyError::PhaseFraction(old_time.to_bits()));
        }
        let phase = f64::from(integer_source);
        if !phase.is_finite()
            || !(-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&phase)
        {
            return Err(ShakeApplyError::PhaseConversion(integer_source.to_bits()));
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked PPC fctidz followed by low unsigned word"
        )]
        let phase = phase.trunc() as i64 as u32;
        let index = (phase % count) as usize;
        let other = 1.0 - fraction; // 82e43928, before keyframe reads
        let next = ((phase % count).wrapping_add(1) % count) as usize;
        let positions = self
            .keyframes
            .positions
            .as_ref()
            .ok_or(ShakeApplyError::MissingKeyframe(index))?;
        let rotations = self
            .keyframes
            .rotations
            .as_ref()
            .ok_or(ShakeApplyError::MissingKeyframe(index))?;
        let rotation0 = rotations
            .get(index)
            .and_then(|v| *v)
            .ok_or(ShakeApplyError::MissingKeyframe(index))?;
        let position0 = positions
            .get(index)
            .and_then(|v| *v)
            .ok_or(ShakeApplyError::MissingKeyframe(index))?;
        let rotation1 = rotations
            .get(next)
            .and_then(|v| *v)
            .ok_or(ShakeApplyError::MissingKeyframe(next))?;
        let position1 = positions
            .get(next)
            .and_then(|v| *v)
            .ok_or(ShakeApplyError::MissingKeyframe(next))?;
        let rotation = interpolate_xyz(rotation0, rotation1, other, fraction);
        let position = interpolate_xyz(position0, position1, other, fraction);
        let mut angles = rotation.map(|value| ((value * amplitude) * distance) * p.rotation);
        angles[2] *= p.dutch;
        let local = position.map(|value| ((value * p.translation) * amplitude) * distance);
        let scratch = scratch_basis(angles);
        self.offset = Vector {
            xyz: local,
            w: None,
        }; // 82e43bf4 transient full vector
        self.offset = world_offset(input, local); // 82e43c04 overwrites unknown W
        let result = compose(input, scratch);
        let remaining = (1.0 - p.one_shot_decay) * self.one_shot_effect;
        self.one_shot_effect = remaining;
        if remaining < f32::from_bits(0x3ba3_d70a) {
            self.one_shot_effect = 0.0;
        }
        Ok(result)
    }
}
