//! Retained `Skeleton::UpdateRecords` (0x82b54118), after collision and volume
//! states. Native debug collaborators remain explicit stops. Numeric services
//! retain their Xenon estimate/reduction boundary; snapshots exclude alias races.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained record readers await full Actor scheduling"
    )
)]

use crate::{
    animation::AnimationMatrix,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
    skeleton_collision_data::SkeletonCollisionData,
    skeleton_normal_displacement::SkeletonNormalDisplacementError,
    skeleton_physics_history::SkeletonPhysicsRecord,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[cfg(test)]
#[path = "skeleton_post_physics_records_tests.rs"]
mod tests;

const ZERO: SkeletonRootVector = SkeletonRootVector {
    xyz: [0.0; 3],
    w: Some(0.0),
};
const HOOK_PARTS: [usize; 8] = [23, 17, 18, 16, 21, 22, 20, 1];

pub(crate) trait SkeletonPostPhysicsRecordServices {
    type Error;
    fn ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn use_normals_for_proxy_offset(&mut self) -> Result<u8, Self::Error>;
    fn scale_hook_to_hips_y(&mut self) -> Result<f32, Self::Error>;
    fn draw_hook_to_hips(&mut self) -> Result<u8, Self::Error>;
}

#[derive(Clone, Copy)]
pub(crate) struct SkeletonPostPhysicsRecordInput<'a, F = u32> {
    pub(crate) physics: &'a SkeletonPhysicsRecord,
    pub(crate) adjusted_volumes: &'a [AnimationMatrix; 24],
    pub(crate) anim_to_world: &'a SkeletonRootMatrix,
    pub(crate) proxy_target: Option<SkeletonRootVector>,
    pub(crate) time_delta: f32,
    pub(crate) flags_8c8: F,
    pub(crate) collision: &'a SkeletonCollisionData,
}

impl<'a, F> SkeletonPostPhysicsRecordInput<'a, F> {
    pub(crate) fn map_flags<G>(
        self,
        map: impl FnOnce(F) -> G,
    ) -> SkeletonPostPhysicsRecordInput<'a, G> {
        SkeletonPostPhysicsRecordInput {
            physics: self.physics,
            adjusted_volumes: self.adjusted_volumes,
            anim_to_world: self.anim_to_world,
            proxy_target: self.proxy_target,
            time_delta: self.time_delta,
            flags_8c8: map(self.flags_8c8),
            collision: self.collision,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkeletonHookDebugStage {
    NormalBeforeDisplacements,
    NormalBeforeHookStore,
    ProxyAfterHookStore,
}

#[derive(Debug, PartialEq)]
pub(crate) enum SkeletonPostPhysicsRecordError<S, N, R> {
    MissingProxyTarget,
    Service(S),
    Numeric(N),
    NormalDisplacement {
        part: usize,
        error: SkeletonNormalDisplacementError<N, R>,
    },
    Debug(SkeletonHookDebugStage),
}
impl<
    S: std::fmt::Display,
    N: std::fmt::Display + std::fmt::Debug,
    R: std::fmt::Display + std::fmt::Debug,
> std::fmt::Display for SkeletonPostPhysicsRecordError<S, N, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingProxyTarget => f.write_str("Skeleton proxy target is unavailable"),
            Self::Service(error) => write!(f, "Skeleton post-physics service: {error}"),
            Self::Numeric(error) => write!(f, "Skeleton post-physics numeric: {error}"),
            Self::NormalDisplacement { part, error } => {
                write!(f, "Skeleton Part {part} normal displacement: {error}")
            }
            Self::Debug(stage) => write!(
                f,
                "Skeleton hook debug collaborator is unavailable at {stage:?}"
            ),
        }
    }
}
impl<S: std::error::Error + 'static, N: std::error::Error + 'static, R: std::error::Error + 'static>
    std::error::Error for SkeletonPostPhysicsRecordError<S, N, R>
{
}

#[derive(Debug)]
struct ComLaneKnowledge {
    current: bool,
    previous: bool,
    velocity: bool,
}

/// Non-Clone owner; no second mass, pose, root or collision authority.
#[derive(Debug)]
pub(crate) struct SkeletonPostPhysicsRecords {
    errors: [SkeletonRootVector; 24],
    centre_of_mass: [f32; 4],
    previous_centre_of_mass: [f32; 4],
    centre_of_mass_velocity: [f32; 4],
    com_lanes: ComLaneKnowledge,
    hook_to_hips: SkeletonRootVector,
    max_displacement: Option<f32>,
    proxy_enabled: bool,
}

impl SkeletonPostPhysicsRecords {
    /// Move `SkeletonState82b50d08`'s already-constructed error rows into the
    /// SAME continuing post-record owner during Body handoff. No reset/update.
    #[expect(
        clippy::large_types_passed_by_value,
        reason = "constructor transfers its actual owned array into continuing storage"
    )]
    pub(crate) fn retain_constructor_errors(&mut self, errors: [[f32; 4]; 24]) {
        self.errors = errors.map(SkeletonRootVector::known);
    }

    /// State ctor 0x82b50d08 clears errors. Skeleton ctor stores zero COM,
    /// previous COM, velocity and hook at 0x82b5732c..0x82b5739c, and false
    /// proxyEnabled at 0x82b574a8. No `MaxDisplacement` ctor store was found.
    /// Init repeats proxy=false at 0x82b62b08; native true mode setters are
    /// not yet connected, so there is intentionally no external boolean setter.
    pub(crate) fn new() -> Self {
        Self {
            errors: [ZERO; 24],
            centre_of_mass: [0.0; 4],
            previous_centre_of_mass: [0.0; 4],
            centre_of_mass_velocity: [0.0; 4],
            com_lanes: ComLaneKnowledge {
                current: true,
                previous: true,
                velocity: true,
            },
            hook_to_hips: ZERO,
            max_displacement: None,
            proxy_enabled: false,
        }
    }
    pub(crate) fn errors(&self) -> &[SkeletonRootVector; 24] {
        &self.errors
    }
    pub(crate) fn centre_of_mass(&self) -> [f32; 4] {
        self.centre_of_mass
    }
    pub(crate) fn centre_of_mass_typed(&self) -> SkeletonRootVector {
        SkeletonRootVector {
            xyz: self.centre_of_mass[..3].try_into().unwrap(),
            w: self.com_lanes.current.then_some(self.centre_of_mass[3]),
        }
    }
    pub(crate) fn centre_of_mass_velocity_typed(&self) -> SkeletonRootVector {
        SkeletonRootVector {
            xyz: self.centre_of_mass_velocity[..3].try_into().unwrap(),
            w: self
                .com_lanes
                .velocity
                .then_some(self.centre_of_mass_velocity[3]),
        }
    }
    pub(crate) fn previous_centre_of_mass(&self) -> [f32; 4] {
        self.previous_centre_of_mass
    }
    pub(crate) fn centre_of_mass_velocity(&self) -> [f32; 4] {
        self.centre_of_mass_velocity
    }
    pub(crate) fn hook_to_hips(&self) -> SkeletonRootVector {
        self.hook_to_hips
    }
    pub(crate) fn max_displacement(&self) -> Option<f32> {
        self.max_displacement
    }
    pub(crate) fn proxy_enabled(&self) -> bool {
        self.proxy_enabled
    }

    /// SAME Skeleton+3e43 store in `SetGroundCollision`543f4 and
    /// `CollideWithWorldOnly`54394, not a separate controller flag.
    pub(crate) fn disable_proxy_for_collision(&mut self) {
        self.proxy_enabled = false;
    }
    pub(crate) fn enable_proxy_for_collision(&mut self) {
        self.proxy_enabled = true;
    }

    /// 0x82b54118..0x82b54214, including both full non-debug hook callers.
    #[expect(
        clippy::type_complexity,
        reason = "independent source service and numeric errors"
    )]
    pub(crate) fn try_update<S, N, R>(
        &mut self,
        input: SkeletonPostPhysicsRecordInput<'_>,
        services: &mut S,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), SkeletonPostPhysicsRecordError<S::Error, N::Error, R::Error>>
    where
        S: SkeletonPostPhysicsRecordServices,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    {
        self.try_update_with_flags(
            input.map_flags(|flags| move || Ok(flags)),
            services,
            numeric,
            rounding,
        )
    }

    /// 0x82b54118..0x82b54214, including both full non-debug hook callers.
    #[expect(
        clippy::type_complexity,
        reason = "independent source service and numeric errors"
    )]
    pub(crate) fn try_update_with_flags<S, N, R, F>(
        &mut self,
        input: SkeletonPostPhysicsRecordInput<'_, F>,
        services: &mut S,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), SkeletonPostPhysicsRecordError<S::Error, N::Error, R::Error>>
    where
        S: SkeletonPostPhysicsRecordServices,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
        F: FnOnce() -> Result<u32, S::Error>,
    {
        let target = input
            .proxy_target
            .ok_or(SkeletonPostPhysicsRecordError::MissingProxyTarget)?;
        self.calculate_errors(
            input.physics,
            input.adjusted_volumes,
            input.anim_to_world.columns(),
            target,
        );
        self.previous_centre_of_mass = self.centre_of_mass;
        self.com_lanes.previous = self.com_lanes.current;
        let seed = numeric
            .reciprocal_estimate(input.time_delta)
            .map_err(SkeletonPostPhysicsRecordError::Numeric)?;
        self.centre_of_mass = input.physics.centre_of_mass();
        self.com_lanes.current = input
            .physics
            .vector_typed(
                crate::skeleton_physics_history::lane_knowledge::RecordVectorField::CentreOfMass,
            )
            .w
            .is_some();
        let reciprocal = refine_records_reciprocal(input.time_delta, seed);
        self.centre_of_mass_velocity = std::array::from_fn(|lane| {
            (self.centre_of_mass[lane] - self.previous_centre_of_mass[lane]) * reciprocal
        });
        self.com_lanes.velocity = self.com_lanes.current && self.com_lanes.previous;
        if (input.flags_8c8)().map_err(SkeletonPostPhysicsRecordError::Service)? & 0x1000 != 0 {
            self.previous_centre_of_mass = self.centre_of_mass;
            self.com_lanes.previous = self.com_lanes.current;
        }
        self.try_update_hook(input.collision, services, numeric, rounding)
    }

    /// `SkeletonState::CalculateErrors`, 0x82b3d408..0x82b3d498. Part0 uses
    /// proxy position/target; Parts1..23 use physical transform P, not positions.
    fn calculate_errors(
        &mut self,
        physics: &SkeletonPhysicsRecord,
        adjusted: &[AnimationMatrix; 24],
        world: &[SkeletonRootVector; 4],
        target: SkeletonRootVector,
    ) {
        let proxy = physics.vector_typed(
            crate::skeleton_physics_history::lane_knowledge::RecordVectorField::ProxyVolumePosition,
        );
        self.errors[0] = proxy.zip(target, |a, b| a - b);
        for (part, matrix) in adjusted.iter().enumerate().skip(1) {
            let position = physics
                .transform_typed(part)
                .expect("native part range")
                .columns()[3];
            let predicted = transform_point(world, [matrix[12], matrix[13], matrix[14]]);
            self.errors[part] = position.zip(predicted, |a, b| a - b);
        }
    }

    #[expect(
        clippy::type_complexity,
        reason = "independent source service and numeric errors"
    )]
    fn try_update_hook<S, N, R>(
        &mut self,
        collision: &SkeletonCollisionData,
        services: &mut S,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), SkeletonPostPhysicsRecordError<S::Error, N::Error, R::Error>>
    where
        S: SkeletonPostPhysicsRecordServices,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    {
        self.max_displacement = Some(
            maximum_length(&self.errors, numeric)
                .map_err(SkeletonPostPhysicsRecordError::Numeric)?,
        );
        if self.proxy_enabled {
            let ground = services
                .ground_normal()
                .map_err(SkeletonPostPhysicsRecordError::Service)?;
            let normals = services
                .use_normals_for_proxy_offset()
                .map_err(SkeletonPostPhysicsRecordError::Service)?;
            let mut hook = self.errors[0];
            if normals != 0 {
                collision
                    .try_calculate_normal_displacement(hook, ground, &mut hook, numeric, rounding)
                    .map_err(|error| SkeletonPostPhysicsRecordError::NormalDisplacement {
                        part: 0,
                        error,
                    })?;
            }
            let scale = services
                .scale_hook_to_hips_y()
                .map_err(SkeletonPostPhysicsRecordError::Service)?;
            hook.xyz[1] *= scale;
            self.hook_to_hips = hook;
            check_debug(services, SkeletonHookDebugStage::ProxyAfterHookStore)?;
        } else {
            check_debug(services, SkeletonHookDebugStage::NormalBeforeDisplacements)?;
            let ground = services
                .ground_normal()
                .map_err(SkeletonPostPhysicsRecordError::Service)?;
            let mut adjusted = [ZERO; 8];
            for (slot, part) in HOOK_PARTS.into_iter().enumerate() {
                collision
                    .try_calculate_normal_displacement(
                        self.errors[part],
                        ground,
                        &mut adjusted[slot],
                        numeric,
                        rounding,
                    )
                    .map_err(|error| SkeletonPostPhysicsRecordError::NormalDisplacement {
                        part,
                        error,
                    })?;
            }
            let hook = select_adjusted_hook(&adjusted, numeric)
                .map_err(SkeletonPostPhysicsRecordError::Numeric)?;
            check_debug(services, SkeletonHookDebugStage::NormalBeforeHookStore)?;
            self.hook_to_hips = hook;
        }
        Ok(())
    }
}

fn check_debug<S: SkeletonPostPhysicsRecordServices, N, R>(
    services: &mut S,
    stage: SkeletonHookDebugStage,
) -> Result<(), SkeletonPostPhysicsRecordError<S::Error, N, R>> {
    if services
        .draw_hook_to_hips()
        .map_err(SkeletonPostPhysicsRecordError::Service)?
        != 0
    {
        return Err(SkeletonPostPhysicsRecordError::Debug(stage));
    }
    Ok(())
}

fn known(value: [f32; 4]) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [value[0], value[1], value[2]],
        w: Some(value[3]),
    }
}
fn transform_point(world: &[SkeletonRootVector; 4], point: [f32; 3]) -> SkeletonRootVector {
    let lane = |a, b, c, d| point[2].mul_add(c, point[1].mul_add(b, point[0].mul_add(a, d)));
    SkeletonRootVector {
        xyz: std::array::from_fn(|i| {
            lane(
                world[0].xyz[i],
                world[1].xyz[i],
                world[2].xyz[i],
                world[3].xyz[i],
            )
        }),
        w: world[0]
            .w
            .zip(world[1].w)
            .zip(world[2].w)
            .zip(world[3].w)
            .map(|(((a, b), c), d)| lane(a, b, c, d)),
    }
}
fn square<N: ReckoningNumeric>(
    numeric: &mut N,
    value: SkeletonRootVector,
) -> Result<f32, N::Error> {
    // dot3 explicitly ignores W; this scratch does not establish retained W.
    let lanes = [
        value.xyz[0],
        value.xyz[1],
        value.xyz[2],
        value.w.unwrap_or(0.0),
    ];
    numeric.dot3(lanes, lanes)
}

/// First reduction of both hook callers, 0x82b457c0 and 0x82b4d6d0.
/// Strict greater picks Part17 on the initial tie, then preserves that winner.
fn maximum_length<N: ReckoningNumeric>(
    errors: &[SkeletonRootVector; 24],
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let mut lengths = [0.0; 8];
    for (slot, part) in HOOK_PARTS.into_iter().enumerate() {
        lengths[slot] = square(numeric, errors[part])?;
    }
    let mut winner = if lengths[0] > lengths[1] {
        errors[23]
    } else {
        errors[17]
    };
    for slot in 2..8 {
        let current = square(numeric, winner)?;
        if lengths[slot] > current {
            winner = errors[HOOK_PARTS[slot]];
        }
    }
    let length_squared = square(numeric, winner)?;
    // Native estimate executes even for zero, before the final zero selection.
    let seed = numeric.reciprocal_sqrt_estimate(length_squared)?;
    let length = length_squared * refine_drive_reciprocal_sqrt(length_squared, seed);
    Ok(if length_squared == 0.0 { 0.0 } else { length })
}

/// Second normal-hook reduction 0x82b45a3c..0x82b45adc. Its initial tie
/// selects Part23. Preserve primitive call order and delayed Part16 selection.
#[expect(
    clippy::similar_names,
    reason = "native Part identifiers expose the source reduction order"
)]
fn select_adjusted_hook<N: ReckoningNumeric>(
    values: &[SkeletonRootVector; 8],
    numeric: &mut N,
) -> Result<SkeletonRootVector, N::Error> {
    let part17 = square(numeric, values[1])?;
    let part18 = square(numeric, values[2])?;
    let part16 = square(numeric, values[3])?;
    let part21 = square(numeric, values[4])?;
    let part23 = square(numeric, values[0])?;
    let mut winner = if part17 > part23 {
        values[1]
    } else {
        values[0]
    };
    if part18 > square(numeric, winner)? {
        winner = values[2];
    }
    let select16 = part16 > square(numeric, winner)?;
    let part22 = square(numeric, values[5])?;
    let part20 = square(numeric, values[6])?;
    let part1 = square(numeric, values[7])?;
    if select16 {
        winner = values[3];
    }
    for (length, slot) in [(part21, 4), (part22, 5), (part20, 6), (part1, 7)] {
        if length > square(numeric, winner)? {
            winner = values[slot];
        }
    }
    Ok(winner)
}

// Source541a0/a8: final negation follows the fused rounding.
fn records_reciprocal_residual(input: f32, seed: f32) -> f32 {
    -(seed.mul_add(input, -1.0))
}
fn refine_records_reciprocal(input: f32, seed: f32) -> f32 {
    let first = seed.mul_add(records_reciprocal_residual(input, seed), seed);
    first.mul_add(records_reciprocal_residual(input, first), first)
}
#[cfg(test)]
#[path = "skeleton_post_record_reciprocal_tests.rs"]
mod reciprocal_tests;
