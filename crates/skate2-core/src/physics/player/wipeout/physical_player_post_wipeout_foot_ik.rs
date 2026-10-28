//! Child of `skeleton_ik_state`: actual `DoPostPhysicsFootIK82b55bf0` and helpers.
use super::SkeletonIkState;
use crate::{
    ragdoll::SkeletonPartIndex as Part,
    reckoning_numeric::{ReckoningNumeric, cross_full4},
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_data::SkeletonData,
    skeleton_ik::SkeletonIkDataIn,
    skeleton_ik_geometry::SkeletonIkGeometry,
    skeleton_ik_line_frame::{normalize_safe, try_calculate_matrix_from_line},
    skeleton_ik_targets::SkeletonIkTargets,
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
    skeleton_two_bone::{
        TwoBoneChain, TwoBoneIkError, TwoBoneIkOutcome, TwoBoneLimits, try_solve_two_bone,
    },
};
const ZERO: Vector = Vector::known([0.0; 4]);
#[cfg(test)]
#[path = "physical_player_post_wipeout_foot_ik_active_tests.rs"]
mod active_tests;
const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

/// All row reads/writes refer to the existing live `PhysRecord`; Part writes
/// precede its matching `PhysRecord` write. No atomic pair replacement.
pub(crate) trait FootIkSource {
    type Error;
    fn flags_8c4(&mut self) -> Result<u32, Self::Error>;
    fn state_timer(&mut self) -> Result<f32, Self::Error>;
    /// Global `physics_animation.default` key3b2641b64fcf3485, live.
    fn minimum_deck_up_y(&mut self) -> Result<f32, Self::Error>;
    /// Own `physics_skeletonik.default` key693453cdb831119b, live `Vector4`.
    fn deck_bounds_margin(&mut self) -> Result<Vector, Self::Error>;
    /// Global animation.default keys38e91dfbcb5e99f3 (bail),8e12e37ff8c71792.
    fn feet_relative_height(&mut self, wipeout: bool) -> Result<f32, Self::Error>;
    fn record(&mut self, part: Part) -> Result<Matrix, Self::Error>;
    fn set_part(&mut self, part: Part, matrix: &Matrix) -> Result<(), Self::Error>;
    fn set_record(&mut self, part: Part, matrix: &Matrix) -> Result<(), Self::Error>;
    fn draw_post_feet_ik(&mut self) -> Result<bool, Self::Error>;
    fn draw_post_phys_ik(&mut self) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum FootIkError<S, N, R> {
    Source(S),
    Numeric(N),
    TwoBone(TwoBoneIkError<N, R>),
    Missing(&'static str),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display
    for FootIkError<S, N, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "post foot IK source: {e:?}"),
            Self::Numeric(e) => write!(f, "post foot IK numeric: {e:?}"),
            Self::TwoBone(e) => write!(f, "post foot IK {e}"),
            Self::Missing(v) => write!(f, "post foot IK missing {v}"),
        }
    }
}
type ResultIk<T, S, N, R> = Result<T, FootIkError<S, N, R>>;

#[derive(Clone, Copy)]
pub(crate) struct FootIkInput<'a> {
    pub(crate) data_in: &'a SkeletonIkDataIn,
    pub(crate) targets: &'a SkeletonIkTargets,
    pub(crate) geometry: &'a SkeletonIkGeometry,
    pub(crate) deck: &'a Matrix,
    pub(crate) wipeout_starting: bool,
}
impl SkeletonIkState {
    /// Caller must satisfy `PostWipeout`'s actual grounded/contact/offboard/
    /// teleport selection. This helper has no cvar-enabled default skip.
    pub(crate) fn try_post_wipeout_foot_ik<
        S: FootIkSource,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        input: FootIkInput<'_>,
        source: &mut S,
        numeric: &mut N,
        rounding: &mut R,
    ) -> ResultIk<(), S::Error, N::Error, R::Error> {
        use FootIkError as E;
        if source.flags_8c4().map_err(E::Source)? & 0x40000 != 0
            && source.state_timer().map_err(E::Source)? > f32::from_bits(0x3d23_d70a)
        {
            return Ok(());
        }
        let up = input.deck.columns()[1];
        let local_up = direction(&input.data_in.world_to_animation, up);
        let minimum = source.minimum_deck_up_y().map_err(E::Source)?;
        if minimum > local_up.xyz[1] {
            return Ok(());
        }
        for entry in 0..2 {
            if self.parts_on_deck[entry] != 0 && !input.wipeout_starting {
                let part_world = input.deck.compose(&self.part_to_deck[entry]);
                let parent = input.targets.parents()[entry].ok_or(E::Missing("IK parent"))?;
                // Parent record position is captured before the parent-to-world store.
                let parent_position = source.record(parent).map_err(E::Source)?.columns()[3];
                self.part_to_world[entry] = part_world;
                self.parent_to_world[entry] = input.deck.compose(&self.parent_to_deck[entry]);
                let delta = subtract(self.parent_to_world[entry].columns()[3], parent_position);
                if two_bone(
                    input.targets.data(),
                    parent,
                    delta,
                    source,
                    numeric,
                    rounding,
                )? {
                    let part = input.targets.parts()[entry].ok_or(E::Missing("IK part"))?;
                    commit(source, part, &self.part_to_world[entry])?;
                    commit(source, parent, &self.parent_to_world[entry])?;
                }
                debug_phys(source)?;
            } else if !self.external_target_set[entry] {
                let part = input.targets.parts()[entry].ok_or(E::Missing("IK part"))?;
                let original = source.record(part).map_err(E::Source)?;
                let old_position = original.columns()[3];
                let parent = input.targets.parents()[entry].ok_or(E::Missing("IK parent"))?;
                let dot = numeric
                    .dot3(scratch(original.columns()[1]), scratch(up))
                    .map_err(E::Numeric)?;
                let parent_position = source.record(parent).map_err(E::Source)?.columns()[3];
                let mut new_position = ZERO;
                if dot > 0.0
                    && intersect_deck(
                        input.geometry,
                        input.deck,
                        old_position,
                        parent_position,
                        self.part_positions[entry],
                        input.wipeout_starting,
                        &mut new_position,
                        source,
                    )?
                {
                    let movement = subtract(new_position, old_position);
                    let projection = numeric
                        .dot3(scratch(up), scratch(movement))
                        .map_err(E::Numeric)?;
                    let delta = up.map(|v| v * projection);
                    if two_bone(
                        input.targets.data(),
                        parent,
                        delta,
                        source,
                        numeric,
                        rounding,
                    )? {
                        commit(source, part, &with_position(original, new_position))?;
                        let parent_matrix = source.record(parent).map_err(E::Source)?;
                        let target = add(parent_matrix.columns()[3], delta);
                        commit(source, parent, &with_position(parent_matrix, target))?;
                    }
                    debug_phys(source)?;
                } else {
                    debug_phys(source)?;
                }
            }
        }
        Ok(())
    }
}

pub(super) fn commit<S: FootIkSource, N, R>(
    source: &mut S,
    part: Part,
    matrix: &Matrix,
) -> ResultIk<(), S::Error, N, R> {
    source.set_part(part, matrix).map_err(FootIkError::Source)?;
    source.set_record(part, matrix).map_err(FootIkError::Source)
}
fn debug_phys<S: FootIkSource, N, R>(source: &mut S) -> ResultIk<(), S::Error, N, R> {
    if source.draw_post_phys_ik().map_err(FootIkError::Source)? {
        source.require_overlay().map_err(FootIkError::Source)?;
    }
    Ok(())
}
fn debug_feet<S: FootIkSource, N, R>(source: &mut S) -> ResultIk<(), S::Error, N, R> {
    if source.draw_post_feet_ik().map_err(FootIkError::Source)? {
        source.require_overlay().map_err(FootIkError::Source)?;
    }
    Ok(())
}

/// `DoesBoneIntersectDeck82b53398`. Native scratch reuses one slot for the X
/// then Z extent tests; treating decompiler's reused v44 as one value is wrong.
#[allow(clippy::too_many_arguments)]
#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "native BO4 bound and height branches include unordered operands"
)]
fn intersect_deck<S: FootIkSource, N, R>(
    geometry: &SkeletonIkGeometry,
    deck: &Matrix,
    bone: Vector,
    parent: Vector,
    target: Vector,
    wipeout: bool,
    new_position: &mut Vector,
    source: &mut S,
) -> ResultIk<bool, S::Error, N, R> {
    use FootIkError as E;
    let inverse = deck.inverse();
    let bone_local = point(&inverse, bone);
    let parent_local = point(&inverse, parent);
    let mut midpoint = add(parent_local, bone_local).map(|v| v * 0.5);
    midpoint.xyz[1] = bone_local.xyz[1];
    let width = geometry
        .box_half_width()
        .ok_or(E::Missing("deck half width"))?;
    let total_length = geometry
        .total_half_length()
        .ok_or(E::Missing("deck total half length"))?;
    let margin = source.deck_bounds_margin().map_err(E::Source)?;
    let bounds = add(Vector::known([width, 0.0, total_length, 0.0]), margin);
    // VMX vminfp propagates NaN from either operand. Its payload cannot
    // escape the subsequent !LT bounds rejection, but Rust min suppression
    // would incorrectly admit an unordered foot position.
    let minimum = |a: f32, b: f32| {
        if a.is_nan() || b.is_nan() {
            f32::NAN
        } else {
            a.abs().min(b.abs())
        }
    };
    let x = minimum(bone_local.xyz[0], midpoint.xyz[0]);
    let z = minimum(bone_local.xyz[2], midpoint.xyz[2]);
    if !(x < bounds.xyz[0]) || !(z < bounds.xyz[2]) {
        debug_feet(source)?;
        return Ok(false);
    }
    let mut height = source.feet_relative_height(wipeout).map_err(E::Source)?;
    let box_length = geometry
        .box_half_length()
        .ok_or(E::Missing("deck box half length"))?;
    if z > box_length {
        let extra = total_length - box_length;
        let distance = z - box_length;
        let selected = if extra - distance >= 0.0 {
            distance
        } else {
            extra
        };
        height = selected.mul_add(
            geometry.end_slope().ok_or(E::Missing("deck end slope"))?,
            height,
        );
    }
    if !(height > midpoint.xyz[1]) {
        debug_feet(source)?;
        return Ok(false);
    }
    // Native first copies target before replacing Y and then world transform.
    *new_position = target;
    new_position.xyz[1] = height;
    *new_position = point(deck, *new_position);
    debug_feet(source)?;
    Ok(true)
}

/// `DoTwoBoneIk_PostPhysics82b536c0`: same solver, actual `PhysRecord`-derived
/// bone locations; limits0/180, overridefalse, one attempt. No prephysics clamp.
pub(super) fn two_bone<S: FootIkSource, N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    data: &SkeletonData,
    selected: Part,
    delta: Vector,
    source: &mut S,
    numeric: &mut N,
    rounding: &mut R,
) -> ResultIk<bool, S::Error, N::Error, R::Error> {
    use FootIkError as E;
    let parent = data.parents()[selected.index()].ok_or(E::Missing("parent"))?;
    let grandparent = data.parents()[parent.index()].ok_or(E::Missing("grandparent"))?;
    let end = bone_position(data, selected, source)?;
    let middle = bone_position(data, parent, source)?;
    let root = bone_position(data, grandparent, source)?;
    let mut solved_middle = ZERO;
    let mut solved_end = add(end, delta);
    let outcome = try_solve_two_bone(
        &TwoBoneChain { root, middle, end },
        &mut solved_middle,
        &mut solved_end,
        TwoBoneLimits {
            minimum_degrees: 0.0,
            maximum_degrees: 180.0,
            override_maximum_with_original: false,
        },
        1,
        numeric,
        rounding,
    )
    .map_err(E::TwoBone)?;
    debug_feet(source)?;
    if outcome != TwoBoneIkOutcome::Reachable {
        return Ok(false);
    }
    let old_normal = cross(subtract(end, middle), subtract(root, middle));
    let new_normal = cross(
        subtract(solved_end, solved_middle),
        subtract(root, solved_middle),
    );
    let normal = normalize_safe(add(old_normal, new_normal), numeric).map_err(E::Numeric)?;
    let mut lower = Matrix::known(IDENTITY);
    let mut upper = Matrix::known(IDENTITY);
    let mapped = try_calculate_matrix_from_line(
        [middle, end],
        normal,
        [solved_middle, solved_end],
        normal,
        &mut lower,
        numeric,
    )
    .map_err(E::Numeric)?
        && try_calculate_matrix_from_line(
            [root, middle],
            normal,
            [root, solved_middle],
            normal,
            &mut upper,
            numeric,
        )
        .map_err(E::Numeric)?;
    if mapped {
        // Source captures BOTH old matrices before publishing the lower Part.
        let parent_matrix = source.record(parent).map_err(E::Source)?;
        let grandparent_matrix = source.record(grandparent).map_err(E::Source)?;
        let parent_matrix = lower.compose(&parent_matrix);
        let grandparent_matrix = upper.compose(&grandparent_matrix);
        commit(source, parent, &parent_matrix)?;
        commit(source, grandparent, &grandparent_matrix)?;
    }
    debug_feet(source)?;
    Ok(mapped)
}
fn bone_position<S: FootIkSource, N, R>(
    data: &SkeletonData,
    part: Part,
    source: &mut S,
) -> ResultIk<Vector, S::Error, N, R> {
    let record = source.record(part).map_err(FootIkError::Source)?;
    let mapping = Matrix::known(data.volume_to_bone()[part.index()]);
    Ok(point(&record, mapping.columns()[3]))
}
fn add(a: Vector, b: Vector) -> Vector {
    a.zip(b, |a, b| a + b)
}
fn subtract(a: Vector, b: Vector) -> Vector {
    a.zip(b, |a, b| a - b)
}
fn scratch(a: Vector) -> [f32; 4] {
    [a.xyz[0], a.xyz[1], a.xyz[2], a.w.unwrap_or(0.0)]
}
fn cross(a: Vector, b: Vector) -> Vector {
    let value = cross_full4(scratch(a), scratch(b));
    Vector {
        xyz: [value[0], value[1], value[2]],
        w: a.w.zip(b.w).map(|_| value[3]),
    }
}
fn point(matrix: &Matrix, vector: Vector) -> Vector {
    let columns = matrix.columns();
    let along_x = columns[0].zip(columns[3], |x, position| x.mul_add(vector.xyz[0], position));
    let along_y = columns[1].zip(along_x, |y, value| y.mul_add(vector.xyz[1], value));
    columns[2].zip(along_y, |z, value| z.mul_add(vector.xyz[2], value))
}
fn direction(matrix: &Matrix, vector: Vector) -> Vector {
    let columns = matrix.columns();
    let along_x = columns[0].map(|x| x * vector.xyz[0]);
    let along_y = columns[1].zip(along_x, |y, value| y.mul_add(vector.xyz[1], value));
    columns[2].zip(along_y, |z, value| z.mul_add(vector.xyz[2], value))
}
fn with_position(m: Matrix, p: Vector) -> Matrix {
    let mut columns = *m.columns();
    columns[3] = p;
    Matrix::from_columns(columns)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Source {
        flags: u32,
        timer: Option<f32>,
        minimum: f32,
        events: Vec<&'static str>,
    }
    impl FootIkSource for Source {
        type Error = &'static str;
        fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
            self.events.push("flags");
            Ok(self.flags)
        }
        fn state_timer(&mut self) -> Result<f32, Self::Error> {
            self.events.push("timer");
            self.timer.ok_or("timer unavailable")
        }
        fn minimum_deck_up_y(&mut self) -> Result<f32, Self::Error> {
            self.events.push("minimum");
            Ok(self.minimum)
        }
        fn deck_bounds_margin(&mut self) -> Result<Vector, Self::Error> {
            Err("margin")
        }
        fn feet_relative_height(&mut self, _: bool) -> Result<f32, Self::Error> {
            Err("height")
        }
        fn record(&mut self, _: Part) -> Result<Matrix, Self::Error> {
            Err("record")
        }
        fn set_part(&mut self, _: Part, _: &Matrix) -> Result<(), Self::Error> {
            Err("set part")
        }
        fn set_record(&mut self, _: Part, _: &Matrix) -> Result<(), Self::Error> {
            Err("set record")
        }
        fn draw_post_feet_ik(&mut self) -> Result<bool, Self::Error> {
            Err("debug feet")
        }
        fn draw_post_phys_ik(&mut self) -> Result<bool, Self::Error> {
            Err("debug phys")
        }
        fn require_overlay(&mut self) -> Result<(), Self::Error> {
            Err("overlay")
        }
    }
    fn run(source: &mut Source) -> Result<(), String> {
        let mut targets = SkeletonIkTargets::before_skeleton_data_init();
        let mut state = SkeletonIkState::new(&mut targets);
        let identity = Matrix::known(IDENTITY);
        let data = SkeletonIkDataIn {
            animation_to_world: identity,
            world_to_animation: identity,
            physics_skateboard: identity,
            inverse_skateboard: identity,
            hips: identity,
            skateboard: identity,
            toe_contact_normal: ZERO,
            hips_offset: ZERO,
        };
        state
            .try_post_wipeout_foot_ik(
                FootIkInput {
                    data_in: &data,
                    targets: &targets,
                    geometry: &SkeletonIkGeometry::new(),
                    deck: &identity,
                    wipeout_starting: false,
                },
                source,
                &mut crate::reckoning_numeric::HostReckoningNumeric,
                &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
            )
            .map_err(|e| e.to_string())
    }
    #[test]
    fn timer_is_lazy_and_ordered_nan_does_not_skip_live_attribute() {
        let mut source = Source {
            flags: 0,
            timer: None,
            minimum: 2.0,
            events: vec![],
        };
        run(&mut source).unwrap();
        assert_eq!(source.events, ["flags", "minimum"]);
        source.flags = 0x40000;
        source.timer = Some(0.05);
        source.events.clear();
        run(&mut source).unwrap();
        assert_eq!(source.events, ["flags", "timer"]);
        source.timer = Some(f32::NAN);
        source.events.clear();
        run(&mut source).unwrap();
        assert_eq!(source.events, ["flags", "timer", "minimum"]);
    }
    #[test]
    fn nan_threshold_reaches_real_mapping_requirement() {
        let mut source = Source {
            flags: 0,
            timer: None,
            minimum: f32::NAN,
            events: vec![],
        };
        assert!(run(&mut source).unwrap_err().contains("IK part"));
        assert_eq!(source.events, ["flags", "minimum"]);
    }
}
