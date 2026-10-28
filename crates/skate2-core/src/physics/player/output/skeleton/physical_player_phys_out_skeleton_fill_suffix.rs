//! Unregistered Skeleton Fill 82b589b8..59848; child IK/audio producers are mandatory.
use super::{
    super::{PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut},
    ConcretePhysOutError,
};
use crate::{
    physics::PointGraph,
    reckoning_numeric::{
        ReckoningNumeric, try_normalize_safe_with_refinement, try_signed_angle, try_unsigned_angle,
        wrap_turns_to_radians,
    },
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::convert::Infallible;

/// Each method reads the SAME retained owner at the indicated native call site.
/// Implementations must execute the actual children, never substitute empty output.
pub(crate) trait SkeletonFillSuffixSource<N: ReckoningNumeric> {
    type Error;
    fn angular_velocity(&mut self, assembly_part: usize)
    -> Result<SkeletonRootVector, Self::Error>;
    fn hook_to_hips(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn proxy_in_collision(&mut self) -> Result<bool, Self::Error>;
    fn physics_velocity(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error>;
    fn effective_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error>;
    fn part_transform(&mut self, part: usize) -> Result<SkeletonRootMatrix, Self::Error>;
    fn fill_ik(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        numeric: &mut N,
    ) -> Result<(), Self::Error>;
    fn reckoning_up(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn processed_body_z(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn compliant_just_collided(&mut self) -> Result<bool, Self::Error>;
    fn fill_audio_collision(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        numeric: &mut N,
    ) -> Result<(), Self::Error>;
    fn ik_contact_offset(&mut self, index: usize) -> Result<f32, Self::Error>;
    /// Global+ec key2f1f013c99079afb, plain index0/default4.
    fn feet_compression_threshold(&mut self) -> Result<f32, Self::Error>;
    /// Fresh Global+ec layout+50/+60, exactly four points.
    fn feet_compression_curve(&mut self) -> Result<PointGraph<4>, Self::Error>;
    fn want_respawn(&mut self) -> Result<bool, Self::Error>;
    fn clear_want_respawn(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub(crate) enum SkeletonFillSuffixError<S, N> {
    Source(S),
    Numeric(N),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
    InvalidPublishedLane { offset: usize, lane: usize },
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for SkeletonFillSuffixError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "Skeleton Fill source: {e:?}"),
            Self::Numeric(e) => write!(f, "Skeleton Fill numeric: {e:?}"),
            Self::Access(e) => write!(f, "Skeleton Fill access: {e:?}"),
            Self::Component(e) => write!(f, "Skeleton Fill component: {e:?}"),
            Self::InvalidPublishedLane { offset, lane } => {
                write!(f, "Skeleton Fill NaN at {offset:x} lane {lane}")
            }
        }
    }
}

/// Two native VNMSUB/FMA refinements followed by the source zero select.
fn length(squared: f32, mut inverse: f32) -> f32 {
    for _ in 0..2 {
        let square = inverse * inverse;
        let half = inverse * 0.5;
        let residual = -squared.mul_add(square, -1.0);
        inverse = half.mul_add(residual, inverse);
    }
    let result = squared * inverse;
    if squared == 0.0 { 0.0 } else { result }
}

/// NormalizeSafe824b1988/1998: negate the rounded fused residual, then FMA.
/// This is the actual shoulder call59638's policy, not the drive refinement.
fn shoulder_refinement(squared: f32, seed: f32) -> f32 {
    let residual = shoulder_residual(squared, seed);
    let first = (seed * 0.5).mul_add(residual, seed);
    let residual = shoulder_residual(squared, first);
    (first * 0.5).mul_add(residual, first)
}
fn shoulder_residual(squared: f32, seed: f32) -> f32 {
    -(squared.mul_add(seed * seed, -1.0))
}

/// Raw dots are armR, armL, legL, legR; estimates are armL,legL,legR,armR.
fn limb_speeds<N: ReckoningNumeric>(
    velocities: [SkeletonRootVector; 4],
    com: SkeletonRootVector,
    numeric: &mut N,
) -> Result<[f32; 4], N::Error> {
    let relative = velocities.map(|v| v.zip(com, |a, b| a - b));
    let mut squared = [0.0; 4];
    for index in 0..4 {
        squared[index] = numeric.dot_vector(relative[index], relative[index])?;
    }
    let first = numeric.reciprocal_sqrt_estimate(squared[1])?;
    let second = numeric.reciprocal_sqrt_estimate(squared[2])?;
    let third = numeric.reciprocal_sqrt_estimate(squared[3])?;
    let lengths = [
        length(squared[1], first),
        length(squared[2], second),
        length(squared[3], third),
    ];
    let last = numeric.reciprocal_sqrt_estimate(squared[0])?;
    Ok([lengths[1], lengths[2], lengths[0], length(squared[0], last)])
}

/// `Math::AngleBetweenVectorsOnPlane82b3a360`. Gate uses original length product,
/// not projected lengths; zero/NaN product returns the source constant zero.
pub(crate) fn plane_angle<N: ReckoningNumeric>(
    a: SkeletonRootVector,
    b: SkeletonRootVector,
    normal: SkeletonRootVector,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let nb = numeric.dot_vector(normal, b)?;
    let na = numeric.dot_vector(normal, a)?;
    let bb = numeric.dot_vector(b, b)?;
    let aa = numeric.dot_vector(a, a)?;
    let projected_a = a.zip(normal.map(|v| v * na), |v, p| v - p);
    let projected_b = b.zip(normal.map(|v| v * nb), |v, p| v - p);
    if aa * bb > f32::from_bits(0x3780_0000) {
        try_signed_angle(projected_a, projected_b, normal, numeric)
    } else {
        Ok(0.0)
    }
}

impl PhysicalPlayerPhysOut {
    /// `captured_com_velocity` MUST be the actual 585b0 value carried from the
    /// prefix. Neither a fresh source query nor an arbitrary current output is valid.
    #[expect(
        clippy::too_many_lines,
        reason = "one source-ordered output suffix with late writes and mandatory child calls"
    )]
    pub(crate) fn try_fill_skeleton_suffix<S: SkeletonFillSuffixSource<N>, N: ReckoningNumeric>(
        &mut self,
        source: &mut S,
        captured_com_velocity: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), SkeletonFillSuffixError<S::Error, N::Error>> {
        use SkeletonFillSuffixError as E;
        macro_rules! check {
            ($guard:expr,$component:ident) => {{
                self.check_write_guard(&$guard).map_err(E::Access)?;
                self.require_complete_component(PhysOutComponent::$component)
                    .map_err(E::Component)?;
            }};
        }
        macro_rules! store {
            ($guard:expr,$field:ident,$value:expr) => {{
                check!($guard, Skeleton);
                self.concrete
                    .skeleton
                    .as_mut()
                    .expect("guarded Skeleton")
                    .$field = $value;
            }};
        }
        // These are assembly slots23/20/16, NOT the previous query's23/19/15.
        for (part, destination) in [(23, 0), (20, 1), (16, 2)] {
            let guard = self.sample_write_guard().map_err(E::Access)?;
            let value = source.angular_velocity(part).map_err(E::Source)?;
            check!(guard, Skeleton);
            let skeleton = self.concrete.skeleton.as_mut().expect("guarded Skeleton");
            match destination {
                0 => skeleton.skeleton_torso_angular_vel = value,
                1 => skeleton.skeleton_right_foot_angular_vel = value,
                _ => skeleton.skeleton_left_foot_angular_vel = value,
            }
        }
        let guard = self.sample_write_guard().map_err(E::Access)?;
        check!(guard, Skeleton); //8a78..98 before hook source load8aa4
        let hook = source.hook_to_hips().map_err(E::Source)?;
        self.concrete
            .skeleton
            .as_mut()
            .expect("guarded Skeleton")
            .skeleton_hook_to_hips = hook;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let proxy = source.proxy_in_collision().map_err(E::Source)?;
        store!(guard, is_proxy_in_collision, proxy);
        // Physics record velocities base26d0: source load order2750,2710,27e0,2820.
        let arm_right = source.physics_velocity(8).map_err(E::Source)?;
        let guard = self.sample_write_guard().map_err(E::Access)?; //8b24, held across all numeric work
        let arm_left = source.physics_velocity(4).map_err(E::Source)?;
        let leg_left = source.physics_velocity(17).map_err(E::Source)?;
        let leg_right = source.physics_velocity(21).map_err(E::Source)?;
        let speeds = limb_speeds(
            [arm_right, arm_left, leg_left, leg_right],
            captured_com_velocity,
            numeric,
        )
        .map_err(E::Numeric)?;
        store!(guard, average_left_leg_speed_local, speeds[0]);
        let guard = self.sample_write_guard().map_err(E::Access)?;
        store!(guard, average_right_leg_speed_local, speeds[1]);
        let guard = self.sample_write_guard().map_err(E::Access)?;
        store!(guard, average_left_arm_speed_local, speeds[2]);
        let guard = self.sample_write_guard().map_err(E::Access)?;
        store!(guard, average_right_arm_speed_local, speeds[3]);
        let effective = source.effective_transform().map_err(E::Source)?;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        store!(guard, effective_anim_z, effective.columns()[2]);
        let guard = self.sample_write_guard().map_err(E::Access)?;
        check!(guard, Skeleton);
        let effective = source.effective_transform().map_err(E::Source)?;
        self.concrete
            .skeleton
            .as_mut()
            .expect("guarded Skeleton")
            .effective_skeleton_root_matrix = effective;
        // Twenty-one source self-equality assertions, each with a fresh write guard.
        for offset in [0x10, 0x20, 0x30, 0x40, 0x90, 0xa0, 0xb0] {
            for lane in 0..3 {
                let guard = self.sample_write_guard().map_err(E::Access)?;
                check!(guard, Skeleton);
                let s = self.concrete.skeleton.as_ref().expect("guarded Skeleton");
                let vector = match offset {
                    0x10 => s.skeleton_head_look,
                    0x20 => s.skeleton_head_pos,
                    0x30 => s.skeleton_hips_look,
                    0x40 => s.skeleton_hips_pos,
                    0x90 => s.skeleton_foot_left_pos,
                    0xa0 => s.skeleton_foot_right_pos,
                    _ => s.skeleton_hips_vel,
                };
                if vector.xyz[lane].is_nan() {
                    return Err(E::InvalidPublishedLane { offset, lane });
                }
            }
        }
        let hips = source.part_transform(23).map_err(E::Source)?;
        source.fill_ik(self, numeric).map_err(E::Source)?;
        let up = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
        let lean = try_unsigned_angle(hips.columns()[0], up, numeric).map_err(E::Numeric)?;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        store!(guard, skeleton_lean, lean);
        let dot = numeric
            .dot_vector(up, hips.columns()[0])
            .map_err(E::Numeric)?;
        let projected = up.zip(hips.columns()[0].map(|v| v * dot), |v, p| v - p);
        let twist = try_signed_angle(projected, hips.columns()[1], hips.columns()[0], numeric)
            .map_err(E::Numeric)?;
        let twist = wrap_turns_to_radians(
            (twist + f32::from_bits(0x4049_0fdb)) * f32::from_bits(0x3e22_f983),
        );
        let guard = self.sample_write_guard().map_err(E::Access)?;
        store!(guard, skeleton_twist, twist);
        let left = source.part_transform(6).map_err(E::Source)?;
        let right = source.part_transform(10).map_err(E::Source)?;
        let across = right.columns()[3].zip(left.columns()[3], |r, l| r - l);
        let across = try_normalize_safe_with_refinement(
            across,
            SkeletonRootVector::known([0.0; 4]),
            numeric,
            shoulder_refinement,
        )
        .map_err(E::Numeric)?;
        let normal = source.reckoning_up().map_err(E::Source)?;
        let body_z = source.processed_body_z().map_err(E::Source)?;
        let shoulder = plane_angle(body_z, across, normal, numeric).map_err(E::Numeric)?;
        let shoulder = wrap_turns_to_radians(shoulder * f32::from_bits(0x3e22_f983));
        let guard = self.sample_write_guard().map_err(E::Access)?;
        store!(guard, shoulder_twist, shoulder);
        self.try_publish_skeleton_contact::<S, N>(source)?;
        source
            .fill_audio_collision(self, numeric)
            .map_err(E::Source)?;
        let left = source.ik_contact_offset(0).map_err(E::Source)?;
        let right = source.ik_contact_offset(1).map_err(E::Source)?;
        let selected = if left - right >= 0.0 { right } else { left }; //FSEL, including unordered else
        let threshold = source.feet_compression_threshold().map_err(E::Source)?;
        let compression = if selected < threshold {
            source
                .feet_compression_curve()
                .map_err(E::Source)?
                .evaluate(selected)
        } else {
            0.0
        };
        let guard = self.sample_write_guard().map_err(E::Access)?;
        store!(guard, feet_ik_compression, compression);
        if source.want_respawn().map_err(E::Source)? {
            let guard = self.sample_write_guard().map_err(E::Access)?;
            check!(guard, State);
            self.concrete
                .state
                .as_mut()
                .expect("guarded State")
                .physics_requested_teleport = true;
            source.clear_want_respawn().map_err(E::Source)?;
        }
        Ok(())
    }

    ///596d8 samples latch before the596e4 collision byte, then596e8 checks it.
    fn try_publish_skeleton_contact<S: SkeletonFillSuffixSource<N>, N: ReckoningNumeric>(
        &mut self,
        source: &mut S,
    ) -> Result<(), SkeletonFillSuffixError<S::Error, N::Error>> {
        use SkeletonFillSuffixError as E;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let collided = source.compliant_just_collided().map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Collision)
            .map_err(E::Component)?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded Collision")
            .skeleton_in_contact = collided;
        Ok(())
    }
}

#[cfg(test)]
#[path = "physical_player_phys_out_skeleton_fill_suffix_tests.rs"]
mod tests;
