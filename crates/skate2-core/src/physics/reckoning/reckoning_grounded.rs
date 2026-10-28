//! `UpdateGroundStates` 0x82de0408 and fixed-heading evolution 0x82ddd468.
use super::{
    J, Reckoning, ReckoningAttributeRead, ReckoningCvars, ReckoningDebug, ReckoningError,
    ReckoningGroundInput, ReckoningNumeric, RunResult, Scope, ZERO, add, blend_fma, column,
    debug_off, fsel, numeric, project_off, replace_control, scale, sub,
};

use super::grounded_input::{
    CapturedGroundDirections, GroundDirectionsRead, GroundInputRead, GroundScalar, GroundVector,
};

impl<A: ReckoningAttributeRead> Reckoning<A> {
    /// Full grounded calculation on the SAME retained filters/attributes/spin
    /// state. Supplied input is held immutable for this exclusive host call.
    /// A typed error retains source-completed state; no transactional rollback.
    pub(crate) fn try_update_grounded<C: ReckoningCvars, N: ReckoningNumeric>(
        &mut self,
        input: &ReckoningGroundInput,
        ground: [f32; 4],
        dynamic: [f32; 4],
        heading: [f32; 4],
        c: &mut C,
        n: &mut N,
    ) -> RunResult<A::Error, C::Error, N::Error> {
        self.dynamic_up = dynamic.into();
        self.heading = heading.into(); // GroundStates wrapper.
        self.try_update_grounded_fixed_heading(input, ground, dynamic, c, n)
    }

    /// Controlled snapshot compatibility; the numerical body is shared with live reads.
    pub(crate) fn try_update_grounded_fixed_heading<C: ReckoningCvars, N: ReckoningNumeric>(
        &mut self,
        input: &ReckoningGroundInput,
        ground: [f32; 4],
        dynamic: [f32; 4],
        c: &mut C,
        n: &mut N,
    ) -> RunResult<A::Error, C::Error, N::Error> {
        self.try_update_grounded_fixed_heading_read(
            input,
            &CapturedGroundDirections {
                ground: ground.into(),
                dynamic: dynamic.into(),
                heading: self.heading,
            },
            c,
            n,
        )
    }

    /// Source wrapper0408 writes dynamic before heading, then enters the shared body.
    pub(crate) fn try_update_grounded_live<
        I: GroundInputRead + GroundDirectionsRead,
        C: ReckoningCvars,
        N: ReckoningNumeric,
    >(
        &mut self,
        input: &I,
        c: &mut C,
        n: &mut N,
    ) -> RunResult<A::Error, C::Error, N::Error> {
        self.try_update_grounded_live_directions(input, input, c, n)
    }

    /// The wrapper's two source-ordered stores also apply when ResetSystems
    /// supplies its own ground/dynamic up and live board heading arguments.
    pub(crate) fn try_update_grounded_live_directions<
        I: GroundInputRead,
        D: GroundDirectionsRead,
        C: ReckoningCvars,
        N: ReckoningNumeric,
    >(
        &mut self,
        input: &I,
        directions: &D,
        c: &mut C,
        n: &mut N,
    ) -> RunResult<A::Error, C::Error, N::Error> {
        self.dynamic_up = directions.dynamic_up().map_err(ReckoningError::Input)?;
        self.heading = directions.heading().map_err(ReckoningError::Input)?;
        self.try_update_grounded_fixed_heading_read(input, directions, c, n)
    }

    /// `PhysState_FollowPath::Update` 0x82d868b8 supplies gJ for the ground, then the live
    /// dynamic up and deck Z.
    pub(crate) fn try_update_grounded_follow_path<
        I: GroundInputRead + GroundDirectionsRead,
        C: ReckoningCvars,
        N: ReckoningNumeric,
    >(
        &mut self,
        input: &I,
        c: &mut C,
        n: &mut N,
    ) -> RunResult<A::Error, C::Error, N::Error> {
        let directions = CapturedGroundDirections {
            ground: J.into(),
            dynamic: input.dynamic_up().map_err(ReckoningError::Input)?,
            heading: input.heading().map_err(ReckoningError::Input)?,
        };
        self.try_update_grounded_live_directions(input, &directions, c, n)
    }

    /// ResetSystems82dfff20 supplies two copies of gJ and the live,
    /// unflipped board transform's third column to UpdateGroundStates.
    pub(crate) fn try_update_grounded_reset_systems<
        I: GroundInputRead,
        C: ReckoningCvars,
        N: ReckoningNumeric,
    >(
        &mut self,
        input: &I,
        heading: crate::skeleton_root_transforms::SkeletonRootVector,
        c: &mut C,
        n: &mut N,
    ) -> RunResult<A::Error, C::Error, N::Error> {
        self.try_update_grounded_live_directions(
            input,
            &CapturedGroundDirections {
                ground: J.into(),
                dynamic: J.into(),
                heading,
            },
            c,
            n,
        )
    }

    #[expect(
        clippy::too_many_lines,
        reason = "preserve ordered grounded state machine and source-late reads"
    )]
    pub(crate) fn try_update_grounded_fixed_heading_read<
        I: GroundInputRead,
        D: GroundDirectionsRead,
        C: ReckoningCvars,
        N: ReckoningNumeric,
    >(
        &mut self,
        input: &I,
        directions: &D,
        c: &mut C,
        n: &mut N,
    ) -> RunResult<A::Error, C::Error, N::Error> {
        if self.is_flipping {
            return Err(ReckoningError::FlippingAssertion);
        }
        self.body_spin
            .try_update_grounded(
                input
                    .scalar(GroundScalar::BodySpin)
                    .map_err(ReckoningError::Input)?,
                0.0,
                c,
            )
            .map_err(ReckoningError::Cvar)?;
        let com = numeric::try_normalize_safe(
            input
                .vector(GroundVector::ComToDeck)
                .map_err(ReckoningError::Input)?,
            ZERO,
            n,
        )
        .map_err(ReckoningError::Numeric)?;
        let dynamic = directions.dynamic_up().map_err(ReckoningError::Input)?;
        let candidate = add(J, sub(dynamic, com));
        let weight = self
            .graph(
                Scope::Global,
                0x420,
                directions.ground_y().map_err(ReckoningError::Input)?,
            )
            .map_err(ReckoningError::Attribute)?;
        let blended = blend_fma(candidate, weight, scale(self.ground_normal, 1.0 - weight));
        let projection_input =
            numeric::try_normalize_safe(blended, ZERO, n).map_err(ReckoningError::Numeric)?;
        self.dynamic_up = directions.dynamic_up().map_err(ReckoningError::Input)?;
        self.update_ground_normal(directions.ground().map_err(ReckoningError::Input)?, n)?;
        let side_raw = numeric::cross_typed(J, self.ground_normal);
        let side_squared = n
            .dot_vector(side_raw, side_raw)
            .map_err(ReckoningError::Numeric)?;
        if side_squared > f32::from_bits(0x3a83_126f) {
            let seed = n
                .reciprocal_sqrt_estimate(side_squared)
                .map_err(ReckoningError::Numeric)?;
            let side = numeric::normalize_from_squared_and_estimate(side_raw, side_squared, seed);
            let projected_dot = n
                .dot_vector(side, projection_input)
                .map_err(ReckoningError::Numeric)?;
            let projected = project_off(projection_input, side, projected_dot);
            let a = numeric::try_unsigned_angle(projected, self.ground_normal, n)
                .map_err(ReckoningError::Numeric)?;
            let b =
                numeric::try_unsigned_angle(projected, J, n).map_err(ReckoningError::Numeric)?;
            let ground_angle = numeric::try_unsigned_angle(self.ground_normal, J, n)
                .map_err(ReckoningError::Numeric)?;
            let old_up_angle =
                numeric::try_unsigned_angle(self.up, J, n).map_err(ReckoningError::Numeric)?;
            // Source computes this even when its debug consumer is disabled.
            let _old_up_ground_angle = numeric::try_unsigned_angle(self.up, self.ground_normal, n)
                .map_err(ReckoningError::Numeric)?;
            let endpoint = if a < b { self.ground_normal } else { J.into() };
            let chosen = if a < ground_angle && b < ground_angle {
                projected
            } else {
                endpoint
            };
            let alpha = (input
                .scalar(GroundScalar::DeckSpeed)
                .map_err(ReckoningError::Input)?
                + 1.0)
                * f32::from_bits(0x3a83_126f);
            let smoothed = blend_fma(self.up, 1.0 - alpha, scale(chosen, alpha));
            let selected = if old_up_angle > ground_angle || b > old_up_angle {
                chosen
            } else {
                smoothed
            };
            let scaled = ground_angle * f32::from_bits(0x3ea2_f983);
            let lower = fsel(-scaled, 0.0, scaled);
            let graph_input = fsel(1.0 - lower, lower, 1.0);
            let mut target = self
                .graph(Scope::Own, 0x3e0, graph_input)
                .map_err(ReckoningError::Attribute)?;
            let speed_graph = self
                .graph(
                    Scope::Own,
                    0x3a0,
                    input
                        .scalar(GroundScalar::DeckSpeed)
                        .map_err(ReckoningError::Input)?
                        * f32::from_bits(0x3dcc_cccd),
                )
                .map_err(ReckoningError::Attribute)?;
            let _debug_max = fsel(speed_graph - target, speed_graph, target);
            debug_off::<A::Error, _, N::Error>(c, ReckoningDebug::Reckoning)?;
            let _unused = self
                .attributes
                .lookup(Scope::Own, 0xda9e_8318_96b8_6ffc)
                .map_err(ReckoningError::Attribute)?;
            let wheel_pointer = self
                .attributes
                .lookup(Scope::Own, 0xfae8_0499_e7d5_f3e4)
                .map_err(ReckoningError::Attribute)?;
            let wheels = self
                .attributes
                .signed_word(&wheel_pointer)
                .map_err(ReckoningError::Attribute)?;
            if input.wheels().map_err(ReckoningError::Input)? <= wheels {
                target = 0.0;
            }
            let delta = target - self.ground_blend;
            let rate_pointer = self
                .attributes
                .lookup(Scope::Own, 0x6ab1_ba81_9741_4fb3)
                .map_err(ReckoningError::Attribute)?;
            let rate = self
                .attributes
                .float(&rate_pointer)
                .map_err(ReckoningError::Attribute)?;
            let lower = fsel(-rate - delta, -rate, delta);
            let limited = fsel(rate - lower, lower, rate);
            let previous_blend = self.ground_blend;
            self.ground_blend = limited + previous_blend;
            self.up_target = blend_fma(
                self.ground_normal,
                self.ground_blend,
                scale(selected, 1.0 - self.ground_blend),
            );
            self.up_target = numeric::try_normalize_safe(self.up_target, ZERO, n)
                .map_err(ReckoningError::Numeric)?;
        } else {
            self.up_target = self.ground_normal;
        }

        let layout = self
            .attributes
            .capture_layout(Scope::Own)
            .map_err(ReckoningError::Attribute)?;
        replace_control(&mut self.attributes, &layout, 0x4a0, &mut self.slow_filter)
            .map_err(ReckoningError::Attribute)?;
        replace_control(&mut self.attributes, &layout, 0x4b0, &mut self.fast_filter)
            .map_err(ReckoningError::Attribute)?;
        if input.wheels().map_err(ReckoningError::Input)? >= 2
            && input
                .scalar(GroundScalar::Balance)
                .map_err(ReckoningError::Input)?
                == 0.0
        {
            let usage = self
                .attributes
                .evaluate_graph8(
                    &layout,
                    0x110,
                    input
                        .scalar(GroundScalar::AbsoluteBodySpeed)
                        .map_err(ReckoningError::Input)?,
                )
                .map_err(ReckoningError::Attribute)?;
            let side = numeric::cross_typed(
                input
                    .vector(GroundVector::DeckZ)
                    .map_err(ReckoningError::Input)?,
                self.ground_normal,
            );
            let side =
                numeric::try_normalize_safe(side, ZERO, n).map_err(ReckoningError::Numeric)?;
            let deck_y = input
                .vector(GroundVector::DeckY)
                .map_err(ReckoningError::Input)?;
            let dot = n
                .dot_vector(deck_y, side)
                .map_err(ReckoningError::Numeric)?;
            let deck_up = numeric::try_normalize_safe(project_off(deck_y, side, dot), ZERO, n)
                .map_err(ReckoningError::Numeric)?;
            self.up_target = blend_fma(deck_up, usage, scale(self.up_target, 1.0 - usage));
            self.up_target = numeric::try_normalize_safe(self.up_target, ZERO, n)
                .map_err(ReckoningError::Numeric)?;
            debug_off::<A::Error, _, N::Error>(c, ReckoningDebug::Reckoning)?;
        }
        let slow_raw = self.slow_filter.update(self.up_target);
        let slow =
            numeric::try_normalize_safe(slow_raw, ZERO, n).map_err(ReckoningError::Numeric)?;
        let fast_raw = self.fast_filter.update(self.up_target);
        let fast =
            numeric::try_normalize_safe(fast_raw, ZERO, n).map_err(ReckoningError::Numeric)?;
        let acceleration_pointer = self
            .attributes
            .lookup(Scope::Own, 0x8b86_e017_4aee_5d6e)
            .map_err(ReckoningError::Attribute)?;
        let acceleration_scalar = self
            .attributes
            .float(&acceleration_pointer)
            .map_err(ReckoningError::Attribute)?;
        let scaled_speed = input
            .scalar(GroundScalar::DeckSpeed)
            .map_err(ReckoningError::Input)?
            * f32::from_bits(0x3dcc_cccd);
        let max_acceleration = input
            .scalar(GroundScalar::TimeDelta)
            .map_err(ReckoningError::Input)?
            * acceleration_scalar;
        let weight = self
            .graph(Scope::Own, 0x1a0, scaled_speed)
            .map_err(ReckoningError::Attribute)?;
        let desired = numeric::try_normalize_safe(
            blend_fma(fast, weight, scale(slow, 1.0 - weight)),
            ZERO,
            n,
        )
        .map_err(ReckoningError::Numeric)?;
        let max_delta = self
            .graph(Scope::Own, 0x1e0, scaled_speed)
            .map_err(ReckoningError::Attribute)?;
        let difference = sub(desired, self.up);
        let limit = max_delta * f32::from_bits(0x3e4c_cccd);
        let length = numeric::try_length(difference, n).map_err(ReckoningError::Numeric)?;
        let direction =
            numeric::try_normalize_safe(difference, ZERO, n).map_err(ReckoningError::Numeric)?;
        let distance = fsel(length - limit, limit, length);
        let bounded = numeric::try_normalize_safe(blend_fma(direction, distance, self.up), ZERO, n)
            .map_err(ReckoningError::Numeric)?;
        let desired_delta = sub(bounded, self.up);
        let acceleration = sub(desired_delta, self.up_speed);
        let acceleration =
            numeric::try_clamp(acceleration, max_acceleration, n).map_err(|error| match error {
                numeric::ReckoningClampError::Numeric(e) => ReckoningError::Numeric(e),
                numeric::ReckoningClampError::NegativeMaximum { bits } => {
                    ReckoningError::NegativeClampMaximum { bits }
                }
            })?;
        self.up_speed = add(self.up_speed, acceleration);
        let damping_pointer = self
            .attributes
            .lookup(Scope::Global, 0x5366_a548_f61b_bbea)
            .map_err(ReckoningError::Attribute)?;
        let damping = self
            .attributes
            .float(&damping_pointer)
            .map_err(ReckoningError::Attribute)?;
        let axis = column(&self.transform, 0);
        let dot = n
            .dot_vector(self.up_speed, axis)
            .map_err(ReckoningError::Numeric)?;
        let parallel = scale(axis, dot);
        let perpendicular = sub(self.up_speed, parallel);
        self.up_speed = blend_fma(parallel, damping, perpendicular);
        self.up = add(self.up, self.up_speed);
        self.up =
            numeric::try_normalize_safe(self.up, self.up, n).map_err(ReckoningError::Numeric)?;
        let reversal_pointer = self
            .attributes
            .lookup(Scope::Own, 0xaf8a_5d55_853a_d035)
            .map_err(ReckoningError::Attribute)?;
        if n.dot_vector(self.up_speed, desired_delta)
            .map_err(ReckoningError::Numeric)?
            < 0.0
        {
            let damping = self
                .attributes
                .float(&reversal_pointer)
                .map_err(ReckoningError::Attribute)?;
            self.up_speed = scale(self.up_speed, damping);
        }
        if input.flag8c8(1).map_err(ReckoningError::Input)? {
            let effective_z = input
                .vector(GroundVector::EffectiveZ)
                .map_err(ReckoningError::Input)?;
            if n.dot_vector(self.up, effective_z)
                .map_err(ReckoningError::Numeric)?
                < 0.0
            {
                debug_off::<A::Error, _, N::Error>(c, ReckoningDebug::Coffin)?;
                let first = numeric::cross_typed(self.up, effective_z);
                let projected = numeric::cross_typed(effective_z, first);
                self.up = numeric::try_normalize_safe(projected, self.up, n)
                    .map_err(ReckoningError::Numeric)?;
                self.up_speed = scale(self.up_speed, 0.5);
            }
        }
        self.slow_filter.set_value(self.up);
        self.fast_filter.set_value(self.up);
        self.calculate_transform(n)
            .map_err(ReckoningError::Numeric)?;
        self.calculate_dynamic_lean(n)
            .map_err(ReckoningError::Numeric)?;
        self.calculate_tilt(input, c, n)?;
        if std::env::var_os("SKATE2_TRACE_VERT").is_some() {
            eprintln!(
                "vert reckoning: com_to_deck={:?} candidate={:?} ground_normal={:?} up={:?} up_target={:?} ground_blend={} tilt_x={} dynamic_lean={}",
                com.xyz,
                candidate.xyz,
                self.ground_normal.xyz,
                self.up.xyz,
                self.up_target.xyz,
                self.ground_blend,
                self.animation_tilt.xyz[0],
                self.dynamic_lean
            );
        }
        debug_off::<A::Error, _, N::Error>(c, ReckoningDebug::Reckoning)?;
        Ok(())
    }
}
