//! Contact update 0x82b55560 and its four direct helpers. This child borrows
//! the retained state, targets and `DataIn`. Actual world query capability stays
//! lazy; an unavailable query is an error, never a fabricated geometric miss.

use super::SkeletonIkState;
use crate::{
    reckoning_numeric::ReckoningNumeric,
    skeleton_ik::SkeletonIkDataIn,
    skeleton_ik_targets::SkeletonIkTargets,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[cfg(test)]
#[path = "skeleton_ik_contact_tests.rs"]
mod tests;

const TARGET_MIN: f32 = f32::from_bits(0xbdcc_cccd);
const TARGET_MAX: f32 = f32::from_bits(0x3dcc_cccd);
const STEP_MIN: f32 = f32::from_bits(0xbc75_c28f);
const STEP_MAX: f32 = f32::from_bits(0x3c75_c28f);
const CONTACT_HEIGHT: f32 = f32::from_bits(0x3da8_f5c3);
const CONTACT_DISTANCE: f32 = f32::from_bits(0x3ccc_cccd);
const PUSH_DISTANCE: f32 = f32::from_bits(0x3cf5_c28f);
const PUSH_OFFSET: f32 = f32::from_bits(0xbc23_d70a);
const LINE_START_FRACTION: f32 = f32::from_bits(0x3e99_999a);

/// Borrows the actual processed owner. Initial request flags and the later
/// valid-contact flags are distinct reads, not an eager tuning snapshot.
pub(crate) trait SkeletonIkContactProcessed {
    fn flags_8c4(&self) -> u32;
    fn contact_bone_id(&self) -> Option<usize>;
    fn effective_transform(&self) -> &SkeletonRootMatrix;
}

use crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError;
/// Fallible live reads; controlled snapshots adapt without changing the kernel.
pub(crate) trait SkeletonIkContactRead {
    fn try_flags_8c4(&self, mask: u32) -> Result<u32, GroundedInputError>;
    fn try_contact_bone_id(&self) -> Result<Option<usize>, GroundedInputError>;
    fn try_effective_transform(&self) -> Result<SkeletonRootMatrix, GroundedInputError>;
}
impl<T: SkeletonIkContactProcessed> SkeletonIkContactRead for T {
    fn try_flags_8c4(&self, mask: u32) -> Result<u32, GroundedInputError> {
        Ok(self.flags_8c4() & mask)
    }
    fn try_contact_bone_id(&self) -> Result<Option<usize>, GroundedInputError> {
        Ok(self.contact_bone_id())
    }
    fn try_effective_transform(&self) -> Result<SkeletonRootMatrix, GroundedInputError> {
        Ok(*self.effective_transform())
    }
}

pub(crate) struct SkeletonIkContactInput<'a, P: SkeletonIkContactRead> {
    pub(crate) data_in: &'a SkeletonIkDataIn,
    pub(crate) targets: &'a SkeletonIkTargets,
    pub(crate) processed: &'a P,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkeletonIkContactLineQuery {
    pub(crate) label: &'static str,
    pub(crate) start: SkeletonRootVector,
    pub(crate) end: SkeletonRootVector,
    pub(crate) fatness: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SkeletonIkContactLineResult {
    Hit { position: SkeletonRootVector },
    Miss,
}

/// Runtime cvar VALUES are read at the individual source load sites. Initial
/// defaults belong to the source packet; this trait provides no default values.
/// The query implementation retains the actual island/world owner capability.
pub(crate) trait SkeletonIkContactServices {
    type Error;
    fn draw_ik_contact_debug(&mut self) -> Result<bool, Self::Error>;
    fn max_ik_contact_penetration_offset(&mut self) -> Result<f32, Self::Error>;
    fn sideways_offset(&mut self) -> Result<f32, Self::Error>;
    /// Snapshot services have no separate target identity; live services capture it here.
    fn capture_sideways_offset_draw(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn sideways_offset_draw(&mut self) -> Result<bool, Self::Error>;
    fn max_ik_contact_offset(&mut self) -> Result<f32, Self::Error>;
    fn toe_contact_threshold(&mut self) -> Result<f32, Self::Error>;
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error>;
    fn try_line_query(
        &mut self,
        query: &SkeletonIkContactLineQuery,
    ) -> Result<SkeletonIkContactLineResult, Self::Error>;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkContactError<S, N> {
    Service(S),
    Processed(GroundedInputError),
    Numeric(N),
    ContactNotValid { entry: usize, state: u32 },
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for SkeletonIkContactError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IK contact: {self:?}")
    }
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::error::Error for SkeletonIkContactError<S, N> {}

impl SkeletonIkState {
    /// 0x82b55560..0x82b55bec. No initialization, all-feet, W, or query
    /// preflight is added. The enclosing actual IK owner controls readiness.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "short-lived bundle only borrows retained owners"
    )]
    pub(crate) fn try_update_contact<P, S, N>(
        &mut self,
        input: SkeletonIkContactInput<'_, P>,
        services: &mut S,
        numeric: &mut N,
    ) -> Result<(), SkeletonIkContactError<S::Error, N::Error>>
    where
        P: SkeletonIkContactRead,
        S: SkeletonIkContactServices,
        N: ReckoningNumeric,
    {
        use SkeletonIkContactError as Error;
        let request = input
            .processed
            .try_flags_8c4(0x4200_0000)
            .map_err(Error::Processed)?
            & 0x4200_0000
            != 0;
        self.tell_anim_dont_push_or_brake = false;
        for entry in 0..2 {
            // +204 is the HIERARCHY bone ID, distinct from target Part +460.
            let matches = self.modes[entry] == 1
                && input
                    .processed
                    .try_contact_bone_id()
                    .map_err(Error::Processed)?
                    == input.targets.bone_ids()[entry];
            if matches {
                match self.contact_states[entry] {
                    0 | 2 => {
                        let not_valid = self.contact_states[entry] == 2;
                        self.clamp_old_contact_target(entry);
                        self.try_add_in_contact_offset(entry, input.data_in, services)
                            .map_err(Error::Service)?;
                        if not_valid && request {
                            self.abort_contact_push();
                            trace_push_abort("no previous contact", entry, self, None);
                        }
                        if services.draw_ik_contact_debug().map_err(Error::Service)? {
                            services.require_debug_overlay().map_err(Error::Service)?;
                            // The separate "No Contact" and "Aborting Push"
                            // draws each acquire a singleton, with one cvar read.
                            if not_valid && request {
                                services.require_debug_overlay().map_err(Error::Service)?;
                            }
                        }
                    }
                    1 => self.try_valid_contact(entry, request, &input, services)?,
                    _ => {} // Unnamed raw states go directly to refresh.
                }
                self.contact_states[entry] = 0;
                self.try_contact_line_test(entry, input.processed, services, numeric)?;
            } else {
                let current = self.contact_offsets[entry];
                self.contact_states[entry] = 0;
                let difference = -current;
                self.contact_target_offsets[entry] = 0.0;
                self.contact_offsets[entry] = bound(difference, STEP_MIN, STEP_MAX) + current;
                self.try_add_in_contact_offset(entry, input.data_in, services)
                    .map_err(Error::Service)?;
            }
            // Common debug follows query/offset work and precedes the next foot.
            if services.draw_ik_contact_debug().map_err(Error::Service)? {
                services.require_debug_overlay().map_err(Error::Service)?;
                // Native formatting reads these only after singleton acquisition;
                // the actual draw target is a proven BLR stub.
                let _ = (
                    self.contact_offsets[entry],
                    self.contact_target_offsets[entry],
                );
            }
        }
        Ok(())
    }

    fn clamp_old_contact_target(&mut self, entry: usize) {
        let target = bound(self.contact_target_offsets[entry], TARGET_MIN, TARGET_MAX);
        self.contact_target_offsets[entry] = target;
        let current = self.contact_offsets[entry];
        self.contact_offsets[entry] = bound(target - current, STEP_MIN, STEP_MAX) + current;
    }

    fn abort_contact_push(&mut self) {
        self.tell_anim_dont_push_or_brake = true;
        self.skateboard_dont_push = true;
    }

    fn try_valid_contact<P, S, N>(
        &mut self,
        entry: usize,
        request: bool,
        input: &SkeletonIkContactInput<'_, P>,
        services: &mut S,
    ) -> Result<(), SkeletonIkContactError<S::Error, N>>
    where
        P: SkeletonIkContactRead,
        S: SkeletonIkContactServices,
    {
        use SkeletonIkContactError as Error;
        let inverse = input.data_in.inverse_skateboard.columns();
        let contact = self.contact_positions[entry];
        let x = multiply_add(inverse[0], contact.xyz[0], inverse[3]);
        // BOOL value at557d4 precedes the remaining Y/Z FMAs and height add.
        let draw = services.draw_ik_contact_debug().map_err(Error::Service)?;
        let xy = multiply_add(inverse[1], contact.xyz[1], x);
        let xyz = multiply_add(inverse[2], contact.xyz[2], xy);
        let height = xyz.xyz[1] + CONTACT_HEIGHT;
        if draw {
            services.require_debug_overlay().map_err(Error::Service)?;
        }
        let maximum = services.max_ik_contact_offset().map_err(Error::Service)?;
        if height > maximum {
            self.clamp_old_contact_target(entry);
            self.try_add_in_contact_offset(entry, input.data_in, services)
                .map_err(Error::Service)?;
            let abort = if request {
                true
            } else {
                let threshold = services.toe_contact_threshold().map_err(Error::Service)?;
                input.data_in.toe_contact_normal.xyz[1] > threshold
            };
            if abort {
                self.abort_contact_push();
                trace_push_abort("contact above reach", entry, self, Some(height));
            }
        } else {
            let flags = input
                .processed
                .try_flags_8c4(0x0180_0000)
                .map_err(Error::Processed)?;
            if flags & 0x0180_0000 == 0 {
                let target = bound(height, TARGET_MIN, TARGET_MAX);
                self.contact_target_offsets[entry] = target;
                let current = self.contact_offsets[entry];
                self.contact_offsets[entry] = bound(target - current, STEP_MIN, STEP_MAX) + current;
            } else {
                let current = self.contact_offsets[entry];
                let difference = -current;
                self.contact_target_offsets[entry] = 0.0;
                self.contact_offsets[entry] = bound(difference, STEP_MIN, STEP_MAX) + current;
            }
            let distance = self.get_dist_to_contact(entry, input.data_in)?;
            self.try_force_offset_for_penetration(entry, distance, services)
                .map_err(Error::Service)?;
            self.try_add_in_contact_offset(entry, input.data_in, services)
                .map_err(Error::Service)?;
            let far = distance > PUSH_DISTANCE;
            let low = self.contact_offsets[entry] < PUSH_OFFSET;
            if request && far && low {
                self.abort_contact_push();
                trace_push_abort("foot above contact", entry, self, Some(distance));
                if services.draw_ik_contact_debug().map_err(Error::Service)? {
                    services.require_debug_overlay().map_err(Error::Service)?;
                }
            }
        }
        Ok(())
    }

    /// 0x82b49538. The available debug singleton takes one check; native
    /// repeated null/assert continuations are not host retry behavior.
    fn try_add_in_contact_offset<S: SkeletonIkContactServices>(
        &mut self,
        entry: usize,
        data: &SkeletonIkDataIn,
        services: &mut S,
    ) -> Result<(), S::Error> {
        if services.draw_ik_contact_debug()? {
            services.require_debug_overlay()?;
        }
        let world_offset = contact_world_offset(&data.skateboard, self.contact_offsets[entry]);
        let mut primary = *self.part_to_world[entry].columns();
        primary[3] = add(primary[3], world_offset);
        self.part_to_world[entry] = SkeletonRootMatrix::from_columns(primary);
        // Unconditional parent load/add/store after primary, no ID gate.
        let mut parent = *self.parent_to_world[entry].columns();
        parent[3] = add(parent[3], world_offset);
        self.parent_to_world[entry] = SkeletonRootMatrix::from_columns(parent);
        Ok(())
    }

    /// 0x82b49698. Only the open negative interval (-maximum,0) corrects.
    fn try_force_offset_for_penetration<S: SkeletonIkContactServices>(
        &mut self,
        entry: usize,
        distance: f32,
        services: &mut S,
    ) -> Result<(), S::Error> {
        if distance < 0.0 {
            let maximum = services.max_ik_contact_penetration_offset()?;
            if distance > -maximum {
                self.contact_offsets[entry] -= distance;
                if services.draw_ik_contact_debug()? {
                    services.require_debug_overlay()?;
                }
            }
        }
        Ok(())
    }

    /// 0x82b53220. Prospective offset BEFORE caller's world-P addition.
    fn get_dist_to_contact<S, N>(
        &self,
        entry: usize,
        data: &SkeletonIkDataIn,
    ) -> Result<f32, SkeletonIkContactError<S, N>> {
        let state = self.contact_states[entry];
        if state != 1 {
            return Err(SkeletonIkContactError::ContactNotValid { entry, state });
        }
        let offset = contact_world_offset(&data.skateboard, self.contact_offsets[entry]);
        let contact = point(&data.inverse_skateboard, self.contact_positions[entry]);
        let prospective = add(self.part_to_world[entry].columns()[3], offset);
        let local = point(&data.inverse_skateboard, prospective);
        let adjusted = local.xyz[1] - CONTACT_DISTANCE;
        Ok(adjusted - contact.xyz[1])
    }

    /// 0x82b49760. Caller state0 and preceding world-P/flags survive every
    /// refusal. Full result position commits before state1; miss retains it.
    fn try_contact_line_test<P, S, N>(
        &mut self,
        entry: usize,
        processed: &P,
        services: &mut S,
        numeric: &mut N,
    ) -> Result<(), SkeletonIkContactError<S::Error, N::Error>>
    where
        P: SkeletonIkContactRead,
        S: SkeletonIkContactServices,
        N: ReckoningNumeric,
    {
        use SkeletonIkContactError as Error;
        let effective_matrix = processed
            .try_effective_transform()
            .map_err(Error::Processed)?;
        let effective = effective_matrix.columns();
        let down = scale(effective[1], -1.0);
        let sign = if entry == 0 { 1.0 } else { -1.0 };
        let relative = subtract(self.part_to_world[entry].columns()[3], effective[3]);
        let offset = services.sideways_offset().map_err(Error::Service)?;
        let signed_offset = offset * sign;
        services
            .capture_sideways_offset_draw()
            .map_err(Error::Service)?;
        let dot = numeric
            .dot3(scratch(relative), scratch(effective[0]))
            .map_err(Error::Numeric)?;
        // Target pointer is read before dot natively, but BOOL VALUE after it.
        let draw = services.sideways_offset_draw().map_err(Error::Service)?;
        let projection = scale(effective[0], dot);
        let plane = subtract(relative, projection);
        let shifted = multiply_add(effective[0], signed_offset, plane);
        let point = add(shifted, effective[3]);
        if draw {
            services.require_debug_overlay().map_err(Error::Service)?;
        }
        let fraction = scale(down, LINE_START_FRACTION);
        let start = subtract(point, fraction);
        let end = add(start, down);
        let query = SkeletonIkContactLineQuery {
            label: "SkeletonIK::ContactLineTest",
            start,
            end,
            fatness: 0.0,
        };
        match services.try_line_query(&query).map_err(Error::Service)? {
            SkeletonIkContactLineResult::Hit { position } => {
                self.contact_positions[entry] = position;
                self.contact_states[entry] = 1;
            }
            SkeletonIkContactLineResult::Miss => self.contact_states[entry] = 2,
        }
        Ok(())
    }
}

fn bound(value: f32, lower: f32, upper: f32) -> f32 {
    let first = if lower - value >= 0.0 { lower } else { value };
    if upper - first >= 0.0 { first } else { upper }
}
fn contact_world_offset(matrix: &SkeletonRootMatrix, value: f32) -> SkeletonRootVector {
    let columns = matrix.columns();
    let x = scale(columns[0], 0.0);
    let xy = multiply_add(columns[1], value, x);
    multiply_add(columns[2], 0.0, xy)
}
fn point(matrix: &SkeletonRootMatrix, value: SkeletonRootVector) -> SkeletonRootVector {
    let columns = matrix.columns();
    let x = multiply_add(columns[0], value.xyz[0], columns[3]);
    let xy = multiply_add(columns[1], value.xyz[1], x);
    multiply_add(columns[2], value.xyz[2], xy)
}
fn scratch(value: SkeletonRootVector) -> [f32; 4] {
    [
        value.xyz[0],
        value.xyz[1],
        value.xyz[2],
        value.w.unwrap_or(0.0),
    ]
}
fn add(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| left.xyz[lane] + right.xyz[lane]),
        w: left.w.zip(right.w).map(|(left, right)| left + right),
    }
}
fn subtract(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| left.xyz[lane] - right.xyz[lane]),
        w: left.w.zip(right.w).map(|(left, right)| left - right),
    }
}
fn scale(value: SkeletonRootVector, scalar: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: value.xyz.map(|lane| lane * scalar),
        w: value.w.map(|lane| lane * scalar),
    }
}
fn multiply_add(
    value: SkeletonRootVector,
    scalar: f32,
    addend: SkeletonRootVector,
) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| value.xyz[lane].mul_add(scalar, addend.xyz[lane])),
        w: value
            .w
            .zip(addend.w)
            .map(|(value, addend)| value.mul_add(scalar, addend)),
    }
}

fn trace_push_abort(reason: &str, entry: usize, state: &SkeletonIkState, distance: Option<f32>) {
    if std::env::var_os("SKATE2_TRACE_PUSH_INTERRUPTION").is_some() {
        eprintln!("push interruption IK: reason={reason} foot={entry} contact_state={} offset={} target={} distance={distance:?} foot_pos={:?} hit={:?}", state.contact_states[entry], state.contact_offsets[entry], state.contact_target_offsets[entry], state.part_to_world[entry].columns()[3].xyz, state.contact_positions[entry].xyz);
    }
}
