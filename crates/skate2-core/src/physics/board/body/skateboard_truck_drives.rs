//! Fresh `CreateDrives` (0x82b37ea8) retains two mutable payload slots across
//! insertion and subsequent contact-gated steering updates. This is not hook
//! creation or a general repeated `CreateDrivesWithOffset` implementation.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "fresh truck-drive suffix awaits live skater publication"
    )
)]

#[path = "skateboard_truck_wipeout.rs"]
mod wipeout;
use crate::{
    simulation_drive_pool::{
        SimulationDriveDynamics, SimulationDriveDynamicsParams, SimulationDriveFrames,
    },
    skateboard_attributes::{SkateboardAttributeClass as Class, SkateboardAttributeRead},
    skateboard_joints::SkateboardTruckTransforms,
    world::try_simulation_quaternion_candidate,
};
use std::{
    fmt,
    sync::{Arc, RwLock},
};

#[cfg(test)]
#[path = "skateboard_truck_drives_tests.rs"]
mod tests;

const DECK_PROFILE: u64 = 0x103f_dc19_514d_4246;
const STRENGTH: u64 = 0xa9f9_72c5_5a52_8cb7;
const DAMPING: u64 = 0x71be_814c_d51a_993f;
const SPEED_DAMPING: u64 = 0xaa32_fea0_0e7d_0921;
const SPRING: u64 = 0x22f5_4336_9220_c8c7;
const LINEAR_OVERRIDE: u64 = 0x7210_da40_dfb1_8272;
const FORCED_STRENGTH: u64 = 0xb926_e54f_9681_34e0;
const FORCED_DAMPING: u64 = 0x0b99_673c_3f05_4766;
const FORCED_SPRING: u64 = 0x2374_d23f_a26b_3a7b;
const STRENGTH_SCALE: f32 = f32::from_bits(0x4560_fffe);
const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];
const ZERO_PARAMS: SimulationDriveDynamicsParams = SimulationDriveDynamicsParams {
    spring: 0.0,
    damping: 0.0,
    strength: 0.0,
    raw_type: 0,
};

pub(crate) trait SkateboardTruckDriveNumeric {
    type Error;
    fn sine_cosine(&mut self, angle: f32) -> Result<(f32, f32), Self::Error>;
    fn quaternion_reciprocal_square_root_estimate(
        &mut self,
        magnitudes: [f32; 4],
    ) -> Result<[f32; 4], Self::Error>;
}

/// Host libm and reciprocal-root seeds, not a Xenon bit-equivalence claim.
pub(crate) struct HostSkateboardTruckDriveNumeric;
impl SkateboardTruckDriveNumeric for HostSkateboardTruckDriveNumeric {
    type Error = std::convert::Infallible;
    fn sine_cosine(&mut self, angle: f32) -> Result<(f32, f32), Self::Error> {
        Ok(angle.sin_cos())
    }
    fn quaternion_reciprocal_square_root_estimate(
        &mut self,
        magnitudes: [f32; 4],
    ) -> Result<[f32; 4], Self::Error> {
        Ok(magnitudes.map(|value| value.sqrt().recip()))
    }
}

/// The concrete admission/session retains each accepted handle BEFORE returning
/// true. False is native capacity null; host errors must not be converted to it.
/// No rollback or implicit handle cleanup is owned by this payload component.
pub(crate) trait SkateboardTruckDriveAdmission {
    type Error;
    fn try_add_drive(
        &mut self,
        parent: usize,
        child: usize,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkateboardTruckDrivePhase {
    Fresh,
    PreFrames,
    ZeroCountDynamics,
    FirstInsertion,
    SecondInsertion,
    PostDynamics,
    PostFrames,
    OptionalLinear,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkateboardTruckDrivePayload {
    Frames,
    Dynamics,
}

#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardTruckDriveError<A, N, D> {
    Attribute(A),
    Numeric(N),
    Admission(D),
    NotFresh {
        phase: SkateboardTruckDrivePhase,
    },
    NotComplete {
        phase: SkateboardTruckDrivePhase,
    },
    UnsupportedDeckProfile {
        value: u8,
    },
    UnsupportedLinearOverride {
        value: u8,
    },
    Poisoned {
        slot: usize,
        payload: SkateboardTruckDrivePayload,
    },
}
impl<A: fmt::Display, N: fmt::Display, D: fmt::Display> fmt::Display
    for SkateboardTruckDriveError<A, N, D>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attribute(error) => write!(f, "truck-drive attribute: {error}"),
            Self::Numeric(error) => write!(f, "truck-drive numeric: {error}"),
            Self::Admission(error) => write!(f, "truck-drive admission: {error}"),
            Self::NotFresh { phase } => {
                write!(f, "truck-drive creation already attempted: {phase:?}")
            }
            Self::NotComplete { phase } => {
                write!(f, "truck-drive construction incomplete: {phase:?}")
            }
            Self::UnsupportedDeckProfile { value } => {
                write!(f, "unsupported fresh deck profile byte {value}")
            }
            Self::UnsupportedLinearOverride { value } => {
                write!(f, "unsupported linear drive override byte {value}")
            }
            Self::Poisoned { slot, payload } => {
                write!(f, "truck-drive slot {slot} {payload:?} lock poisoned")
            }
        }
    }
}
impl<A: std::error::Error + 'static, N: std::error::Error + 'static, D: std::error::Error + 'static>
    std::error::Error for SkateboardTruckDriveError<A, N, D>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Attribute(error) => Some(error),
            Self::Numeric(error) => Some(error),
            Self::Admission(error) => Some(error),
            Self::NotFresh { .. }
            | Self::NotComplete { .. }
            | Self::UnsupportedDeckProfile { .. }
            | Self::UnsupportedLinearOverride { .. }
            | Self::Poisoned { .. } => None,
        }
    }
}

pub(crate) struct SkateboardTruckDrives {
    occurrence: Arc<()>,
    level: SkateboardTruckTransforms,
    current_back: Option<[f32; 16]>,
    current_front: Option<[f32; 16]>,
    frames: [Arc<RwLock<SimulationDriveFrames>>; 2],
    dynamics: [Arc<RwLock<SimulationDriveDynamics>>; 2],
    count: usize,
    add_results: [Option<bool>; 2],
    phase: SkateboardTruckDrivePhase,
    turn_front: f32,
    turn_back: f32,
    time_front: f32,
    time_back: f32,
    force_turn: bool,
    speed_damping_factor: f32,
}
impl SkateboardTruckDrives {
    /// `SetTruckDamping82b29dd0`: store the sampled speed factor unchanged.
    pub(crate) fn set_speed_damping_factor(&mut self, value: f32) {
        self.speed_damping_factor = value;
    }
    /// Fresh constructor payloads, 0x82b38270. Current transformed matrices
    /// are absent until their corresponding source stores have occurred.
    pub(crate) fn new(level: SkateboardTruckTransforms) -> Self {
        Self {
            occurrence: Arc::new(()),
            level,
            current_back: None,
            current_front: None,
            frames: std::array::from_fn(|_| {
                Arc::new(RwLock::new(SimulationDriveFrames {
                    quaternion_a: [0.0, 0.0, 0.0, 1.0],
                    position_a: [0.0; 4],
                    quaternion_b: [0.0, 0.0, 0.0, 1.0],
                    position_b: [0.0; 4],
                }))
            }),
            dynamics: std::array::from_fn(|_| {
                Arc::new(RwLock::new(SimulationDriveDynamics {
                    linear: ZERO_PARAMS,
                    angular: ZERO_PARAMS,
                }))
            }),
            count: 0,
            add_results: [None; 2],
            phase: SkateboardTruckDrivePhase::Fresh,
            turn_front: 0.0,
            turn_back: 0.0,
            time_front: 0.0,
            time_back: 0.0,
            force_turn: false,
            speed_damping_factor: 0.0,
        }
    }
    pub(crate) fn frames(&self) -> &[Arc<RwLock<SimulationDriveFrames>>; 2] {
        &self.frames
    }
    pub(crate) fn occurrence(&self) -> &Arc<()> {
        &self.occurrence
    }
    pub(crate) fn dynamics(&self) -> &[Arc<RwLock<SimulationDriveDynamics>>; 2] {
        &self.dynamics
    }
    pub(crate) const fn count(&self) -> usize {
        self.count
    }
    pub(crate) const fn add_results(&self) -> [Option<bool>; 2] {
        self.add_results
    }
    pub(crate) const fn phase(&self) -> SkateboardTruckDrivePhase {
        self.phase
    }
    pub(crate) const fn current_back(&self) -> Option<&[f32; 16]> {
        self.current_back.as_ref()
    }
    pub(crate) const fn current_front(&self) -> Option<&[f32; 16]> {
        self.current_front.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn turn_state_for_test(&self) -> [f32; 4] {
        [
            self.turn_front,
            self.turn_back,
            self.time_front,
            self.time_back,
        ]
    }

    /// `SetForceTurnAngle(float,bool,bool)`, 0x82b24cf0. Native front/back
    /// booleans occupy r5/r6. This only mutates retained steering fields;
    /// frame and dynamics publication are separate native calls.
    pub(crate) fn set_force_turn_angle(&mut self, angle: f32, front: bool, back: bool) {
        contact_turn(&mut self.time_front, &mut self.turn_front, angle, front);
        if back {
            self.force_turn = true;
            contact_turn(&mut self.time_back, &mut self.turn_back, angle, true);
        } else {
            contact_turn(&mut self.time_back, &mut self.turn_back, angle, false);
            self.force_turn = true;
        }
    }

    /// Body::TeleportReset 0x82b2e818..24 stores force-turn first, then
    /// positive-zero front/back angles without touching the contact timers.
    pub(crate) fn reset_teleport_turn_fields(&mut self) {
        self.force_turn = true;
        self.turn_front = 0.0;
        self.turn_back = 0.0;
    }

    /// The later OUTER board reset repeats these stores in a different native
    /// order: front angle, force-turn, back angle (0x82b36170..78).
    pub(crate) fn reset_teleport_outer_turn_fields(&mut self) {
        self.turn_front = 0.0;
        self.force_turn = true;
        self.turn_back = 0.0;
    }

    /// `SetTruckDriveFrames`, 0x82b34fa8, on the same published payloads.
    pub(crate) fn try_update_frames<N: SkateboardTruckDriveNumeric>(
        &mut self,
        numeric: &mut N,
    ) -> Result<
        (),
        SkateboardTruckDriveError<std::convert::Infallible, N::Error, std::convert::Infallible>,
    > {
        if self.phase != SkateboardTruckDrivePhase::Complete {
            return Err(SkateboardTruckDriveError::NotComplete { phase: self.phase });
        }
        self.set_frames(numeric)
    }

    /// `SetTruckDriveDynamics`, 0x82b2cb08. Runtime supports both deck
    /// profiles; the fresh construction contract remains separately bounded.
    pub(crate) fn try_update_dynamics<A: SkateboardAttributeRead>(
        &mut self,
        attributes: &mut A,
    ) -> Result<
        (),
        SkateboardTruckDriveError<A::Error, std::convert::Infallible, std::convert::Infallible>,
    > {
        if self.phase != SkateboardTruckDrivePhase::Complete {
            return Err(SkateboardTruckDriveError::NotComplete { phase: self.phase });
        }
        self.set_dynamics(attributes)
    }

    /// `CreateDrivesWithOffset(0)` + `CreateDrives` suffix, 0x82b37b88/0x82b37ea8.
    /// Fresh false-force/true-deck/false-linear profile only. Unsupported
    /// samples refuse at their source sampling site, after any earlier effects.
    /// Host failures retain partial writes/results and permanently refuse retry.
    #[expect(
        clippy::type_complexity,
        reason = "retain the three collaborator error types without erasing their origin"
    )]
    pub(crate) fn try_create_fresh<
        A: SkateboardAttributeRead,
        N: SkateboardTruckDriveNumeric,
        D: SkateboardTruckDriveAdmission,
    >(
        &mut self,
        attributes: &mut A,
        numeric: &mut N,
        admission: &mut D,
    ) -> Result<(), SkateboardTruckDriveError<A::Error, N::Error, D::Error>> {
        use SkateboardTruckDriveError::{
            Admission, Attribute, NotFresh, UnsupportedLinearOverride,
        };
        use SkateboardTruckDrivePhase as Phase;
        if self.phase != Phase::Fresh {
            return Err(NotFresh { phase: self.phase });
        }
        self.phase = Phase::PreFrames;
        self.set_frames::<A::Error, N, D::Error>(numeric)?;
        self.phase = Phase::ZeroCountDynamics;
        self.set_dynamics::<A, N::Error, D::Error>(attributes)?;
        self.phase = Phase::FirstInsertion;
        self.add_results[0] = Some(
            admission
                .try_add_drive(6, 4, &self.frames[0], &self.dynamics[0])
                .map_err(Admission)?,
        );
        self.phase = Phase::SecondInsertion;
        self.add_results[1] = Some(
            admission
                .try_add_drive(6, 5, &self.frames[1], &self.dynamics[1])
                .map_err(Admission)?,
        );
        self.count += 2;
        self.phase = Phase::PostDynamics;
        self.set_dynamics::<A, N::Error, D::Error>(attributes)?;
        self.phase = Phase::PostFrames;
        self.set_frames::<A::Error, N, D::Error>(numeric)?;
        self.phase = Phase::OptionalLinear;
        let pointer = attributes
            .lookup(Class::Trucks, LINEAR_OVERRIDE)
            .map_err(Attribute)?;
        let value = attributes.byte(&pointer).map_err(Attribute)?;
        if value != 0 {
            return Err(UnsupportedLinearOverride { value });
        }
        self.phase = Phase::Complete;
        Ok(())
    }

    fn set_frames<A, N: SkateboardTruckDriveNumeric, D>(
        &mut self,
        numeric: &mut N,
    ) -> Result<(), SkateboardTruckDriveError<A, N::Error, D>> {
        use SkateboardTruckDriveError::{Numeric, Poisoned};
        let back = apply_turn(
            self.level.back,
            numeric.sine_cosine(self.turn_back).map_err(Numeric)?,
        );
        self.current_back = Some(back);
        let front = apply_turn(
            self.level.front,
            numeric.sine_cosine(-self.turn_front).map_err(Numeric)?,
        );
        self.current_front = Some(front);
        for (slot, child) in [back, front].into_iter().enumerate() {
            let relative = relative_frame(IDENTITY, child);
            let qa = try_simulation_quaternion_candidate(IDENTITY, |input| {
                numeric.quaternion_reciprocal_square_root_estimate(input)
            })
            .map_err(Numeric)?;
            {
                let mut frame = self.frames[slot].write().map_err(|_| Poisoned {
                    slot,
                    payload: SkateboardTruckDrivePayload::Frames,
                })?;
                frame.quaternion_a = qa;
                frame.position_a = [0.0; 4];
            }
            let qb = try_simulation_quaternion_candidate(relative, |input| {
                numeric.quaternion_reciprocal_square_root_estimate(input)
            })
            .map_err(Numeric)?;
            let mut frame = self.frames[slot].write().map_err(|_| Poisoned {
                slot,
                payload: SkateboardTruckDrivePayload::Frames,
            })?;
            frame.quaternion_b = qb;
            frame.position_b = [relative[12], relative[13], relative[14], relative[15]];
        }
        Ok(())
    }

    fn set_dynamics<A: SkateboardAttributeRead, N, D>(
        &mut self,
        attributes: &mut A,
    ) -> Result<(), SkateboardTruckDriveError<A::Error, N, D>> {
        use SkateboardTruckDriveError::{Attribute, Poisoned, UnsupportedDeckProfile};
        let pointer = attributes
            .lookup(Class::Deck, DECK_PROFILE)
            .map_err(Attribute)?;
        let value = attributes.byte(&pointer).map_err(Attribute)?;
        if value == 0 && self.phase != SkateboardTruckDrivePhase::Complete {
            return Err(UnsupportedDeckProfile { value });
        }
        for slot in 0..self.count {
            if self.force_turn {
                let strength = attributes
                    .lookup(Class::TrucksDrives, FORCED_STRENGTH)
                    .map_err(Attribute)?;
                let (spring, damping, strength) = if value != 0 {
                    let damping = attributes
                        .lookup(Class::TrucksDrives, FORCED_DAMPING)
                        .map_err(Attribute)?;
                    let spring = attributes
                        .lookup(Class::TrucksDrives, FORCED_SPRING)
                        .map_err(Attribute)?;
                    let spring = attributes.float(&spring).map_err(Attribute)?;
                    let strength = attributes.float(&strength).map_err(Attribute)?;
                    let spring = spring * f32::from_bits(0x426f_ffff);
                    let damping = attributes.float(&damping).map_err(Attribute)?;
                    (spring, damping, strength)
                } else {
                    let strength = attributes.float(&strength).map_err(Attribute)?;
                    (
                        f32::from_bits(0x4561_0000),
                        f32::from_bits(0x4279_999a),
                        strength,
                    )
                };
                let mut dynamics = self.dynamics[slot].write().map_err(|_| Poisoned {
                    slot,
                    payload: SkateboardTruckDrivePayload::Dynamics,
                })?;
                if value != 0 {
                    dynamics.angular.raw_type = 2;
                }
                dynamics.angular.spring = spring;
                dynamics.angular.damping = damping;
                dynamics.angular.strength = strength * STRENGTH_SCALE;
                if value == 0 {
                    dynamics.angular.raw_type = 1;
                }
                continue;
            }
            let strength = attributes
                .lookup(Class::TrucksDrives, STRENGTH)
                .map_err(Attribute)?;
            let base = attributes
                .lookup(Class::TrucksDrives, DAMPING)
                .map_err(Attribute)?;
            let multiplier = attributes
                .lookup(Class::TrucksDrives, SPEED_DAMPING)
                .map_err(Attribute)?;
            let spring = attributes
                .lookup(Class::TrucksDrives, SPRING)
                .map_err(Attribute)?;
            // The false-deck branch reads strength before speed/multiplier/base.
            let early_strength = if value == 0 {
                Some(attributes.float(&strength).map_err(Attribute)?)
            } else {
                None
            };
            let speed = self.speed_damping_factor;
            let multiplier = attributes.float(&multiplier).map_err(Attribute)?;
            let base = attributes.float(&base).map_err(Attribute)?;
            let strength = if let Some(strength) = early_strength {
                strength
            } else {
                attributes.float(&strength).map_err(Attribute)?
            };
            let damping = speed_damping(speed, multiplier, base);
            let spring = attributes.float(&spring).map_err(Attribute)?;
            let mut dynamics = self.dynamics[slot].write().map_err(|_| Poisoned {
                slot,
                payload: SkateboardTruckDrivePayload::Dynamics,
            })?;
            if value != 0 {
                dynamics.angular.raw_type = 1;
            }
            dynamics.angular.spring = spring;
            dynamics.angular.damping = damping;
            dynamics.angular.strength = strength * STRENGTH_SCALE;
            if value == 0 {
                dynamics.angular.raw_type = 1;
            }
        }
        Ok(())
    }
}

fn contact_turn(timer: &mut f32, retained: &mut f32, angle: f32, contact: bool) {
    if contact {
        if (*timer).partial_cmp(&f32::from_bits(0x3e28_f5c3)) == Some(std::cmp::Ordering::Greater) {
            *retained = angle;
        } else {
            *timer += f32::from_bits(0x3c88_8889);
            let scaled = *timer * f32::from_bits(0x40c1_f07c);
            // PPC fsel: ordered >=0 selects its positive operand, including
            // either signed zero; unordered selects its negative operand.
            let lower = if -scaled >= 0.0 { 0.0 } else { scaled };
            let blend = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
            let weighted = blend * angle;
            *retained = (1.0 - blend).mul_add(*retained, weighted);
        }
    } else {
        if *timer == 0.0 {
            *retained *= f32::from_bits(0x3f7b_a5e3);
        }
        *timer = 0.0;
    }
}

fn speed_damping(speed: f32, multiplier: f32, base: f32) -> f32 {
    speed.mul_add(multiplier, base)
}

fn apply_turn(level: [f32; 16], (sine, cosine): (f32, f32)) -> [f32; 16] {
    let turn = [[1.0, 0.0, 0.0], [0.0, cosine, sine], [0.0, -sine, cosine]];
    std::array::from_fn(|index| {
        let lane = index % 4;
        if index < 12 {
            let column = turn[index / 4];
            let first = level[lane] * column[0];
            let second = level[4 + lane].mul_add(column[1], first);
            level[8 + lane].mul_add(column[2], second)
        } else {
            let first = level[lane].mul_add(0.0, level[12 + lane]);
            let second = level[4 + lane].mul_add(0.0, first);
            level[8 + lane].mul_add(0.0, second)
        }
    })
}

/// `SetDriveFrames2` (0x82b2c998): rigid inverse-parent * child, preserving the
/// inverse translation's Z/Y/X schedule and the relative product's X/Y/Z.
fn relative_frame(parent: [f32; 16], child: [f32; 16]) -> [f32; 16] {
    let inverse: [[f32; 4]; 3] =
        std::array::from_fn(|column| [parent[column], parent[4 + column], parent[8 + column], 0.0]);
    let negative: [f32; 3] = std::array::from_fn(|lane| 0.0 - parent[12 + lane]);
    let origin: [f32; 4] = std::array::from_fn(|lane| {
        let z = inverse[2][lane] * negative[2];
        let y = inverse[1][lane].mul_add(negative[1], z);
        inverse[0][lane].mul_add(negative[0], y)
    });
    std::array::from_fn(|index| {
        let lane = index % 4;
        let base = (index / 4) * 4;
        let first = if index < 12 {
            inverse[0][lane] * child[base]
        } else {
            inverse[0][lane].mul_add(child[base], origin[lane])
        };
        let second = inverse[1][lane].mul_add(child[base + 1], first);
        inverse[2][lane].mul_add(child[base + 2], second)
    })
}
