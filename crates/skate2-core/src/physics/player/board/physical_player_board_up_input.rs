//! Actual same-board input reads for `Skateboard::UpdateUpVector`.
//! The enclosing continuation certifies the completed Body stage and selects
//! the current retained processed owner. No source values are prefetched here.

use super::{
    physical_board_forces::PhysicalPlayerBoardForceError,
    physical_island::SimulationPhysicalBodyError,
    physical_phys_out::PhysicalPlayerPhysOutOwnerError,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    skateboard_contact_surface::SkateboardContactSurface,
    skateboard_update_up_vector::SkateboardUpdateUpInput,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{
    fmt,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub(crate) enum PhysicalPlayerBoardUpInputError {
    Authority(PhysicalPlayerBoardForceError),
    Owner(SimulationPhysicalBodyError),
    Pool(SimulationSkaterPoolError),
    Pair(PhysicalPlayerPhysOutOwnerError),
    ForeignProcessedOutput,
    ForeignContactState,
    ForeignToken,
    PoisonedContactState,
    InvalidWheel(usize),
    Missing(&'static str),
}
impl fmt::Display for PhysicalPlayerBoardUpInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authority(error) => write!(f, "up input board authority: {error}"),
            Self::Owner(error) => write!(f, "up input physical owner: {error}"),
            Self::Pool(error) => write!(f, "up input board pool: {error}"),
            Self::Pair(error) => write!(f, "up input output pair: {error}"),
            Self::ForeignProcessedOutput => {
                f.write_str("up input processed owner is not bound to this pair's output")
            }
            Self::ForeignContactState => {
                f.write_str("up input contact state belongs to another board")
            }
            Self::ForeignToken => f.write_str("up input token belongs to another input occurrence"),
            Self::PoisonedContactState => f.write_str("up input contact state lock poisoned"),
            Self::InvalidWheel(wheel) => write!(f, "up input wheel {wheel} is outside four wheels"),
            Self::Missing(field) => write!(f, "up input {field} is unready"),
        }
    }
}

pub(crate) struct PhysicalPlayerUpBodyToken {
    identity: Arc<()>,
    board: SimulationSkaterAssemblyHandle,
}
pub(crate) struct PhysicalPlayerUpCollisionToken {
    identity: Arc<()>,
    contact: Arc<Mutex<SkateboardContactSurface>>,
}
pub(crate) struct PhysicalPlayerUpProcessedToken {
    identity: Arc<()>,
}

pub(crate) struct PhysicalPlayerBoardUpInput<'a> {
    world: &'a PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    processed: &'a SkeletonProcessedContact,
    contact: Arc<Mutex<SkateboardContactSurface>>,
    identity: Arc<()>,
}

impl<'a> PhysicalPlayerBoardUpInput<'a> {
    /// Host occurrence validation only. The mutable borrow permits the existing
    /// authority getter, then becomes immutable for all source-stage reads.
    /// The caller must not hold the contact mutex while running this adapter.
    pub(crate) fn try_new(
        world: &'a mut PhysicalAssemblyWorld,
        board: &'a SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        processed: &'a SkeletonProcessedContact,
        contact: &Arc<Mutex<SkateboardContactSurface>>,
    ) -> Result<Self, PhysicalPlayerBoardUpInputError> {
        let bound = world
            .try_with_pair_phys_out(board, skeleton, |output| {
                processed.prior_output().matches_output_occurrence(output)
            })
            .map_err(PhysicalPlayerBoardUpInputError::Pair)?;
        if !bound {
            return Err(PhysicalPlayerBoardUpInputError::ForeignProcessedOutput);
        }
        let same = world
            .try_with_board_force_runtime(board, |runtime| {
                Arc::ptr_eq(&runtime.contact_surface, contact)
            })
            .map_err(PhysicalPlayerBoardUpInputError::Authority)?;
        if !same {
            return Err(PhysicalPlayerBoardUpInputError::ForeignContactState);
        }
        Ok(Self {
            world,
            board,
            processed,
            contact: Arc::clone(contact),
            identity: Arc::new(()),
        })
    }

    fn validate(&self, identity: &Arc<()>) -> Result<(), PhysicalPlayerBoardUpInputError> {
        if Arc::ptr_eq(identity, &self.identity) {
            Ok(())
        } else {
            Err(PhysicalPlayerBoardUpInputError::ForeignToken)
        }
    }

    fn validate_body(
        &self,
        body: &PhysicalPlayerUpBodyToken,
    ) -> Result<(), PhysicalPlayerBoardUpInputError> {
        self.validate(&body.identity)?;
        if body.board.same_as(self.board) {
            Ok(())
        } else {
            Err(PhysicalPlayerBoardUpInputError::ForeignToken)
        }
    }
}

impl SkateboardUpdateUpInput for PhysicalPlayerBoardUpInput<'_> {
    type Error = PhysicalPlayerBoardUpInputError;
    type Body = PhysicalPlayerUpBodyToken;
    type Collision = PhysicalPlayerUpCollisionToken;
    type Processed = PhysicalPlayerUpProcessedToken;

    fn capture_body(&mut self) -> Result<Self::Body, Self::Error> {
        self.world
            .physical_body_owner_for_contacts(self.board)
            .map_err(Self::Error::Owner)?;
        // Resolve the actual board member at both native Body getter sites.
        self.world.board(self.board).map_err(Self::Error::Pool)?;
        Ok(PhysicalPlayerUpBodyToken {
            identity: Arc::clone(&self.identity),
            board: self.board.clone(),
        })
    }

    fn num_wheels(&mut self, body: &Self::Body) -> Result<u8, Self::Error> {
        self.validate_body(body)?;
        self.world
            .board_construction(&body.board)
            .map_err(Self::Error::Pool)?
            .and_then(|construction| construction.post_fields())
            .map(|fields| fields.num_wheels)
            .ok_or(Self::Error::Missing("actual Body wheel count"))
    }

    fn capture_collision(&mut self) -> Result<Self::Collision, Self::Error> {
        Ok(PhysicalPlayerUpCollisionToken {
            identity: Arc::clone(&self.identity),
            contact: Arc::clone(&self.contact),
        })
    }

    fn in_contact(
        &mut self,
        collision: &Self::Collision,
        wheel: usize,
    ) -> Result<bool, Self::Error> {
        self.validate(&collision.identity)?;
        if wheel >= 4 {
            return Err(Self::Error::InvalidWheel(wheel));
        }
        collision
            .contact
            .lock()
            .map_err(|_| Self::Error::PoisonedContactState)?
            .collision_state
            .in_contact[wheel]
            .ok_or(Self::Error::Missing("wheel contact flag"))
    }

    fn acceleration(
        &mut self,
        collision: &Self::Collision,
        wheel: usize,
    ) -> Result<SkeletonRootVector, Self::Error> {
        self.validate(&collision.identity)?;
        if wheel >= 4 {
            return Err(Self::Error::InvalidWheel(wheel));
        }
        collision
            .contact
            .lock()
            .map_err(|_| Self::Error::PoisonedContactState)?
            .collision_state
            .acceleration[wheel]
            .ok_or(Self::Error::Missing("retained wheel acceleration"))
    }

    fn capture_processed(&mut self) -> Result<Self::Processed, Self::Error> {
        // Capturing the borrowed current processed owner does not read speed.
        Ok(PhysicalPlayerUpProcessedToken {
            identity: Arc::clone(&self.identity),
        })
    }

    fn surface_speed(&mut self, processed: &Self::Processed) -> Result<f32, Self::Error> {
        self.validate(&processed.identity)?;
        // The source field may legitimately retain constructor-origin prior
        // motion. Its provenance is not relabeled as newly solved motion.
        self.processed
            .prior_output()
            .surface_speed()
            .map(|value| value.value)
            .ok_or(Self::Error::Missing("processed prior-output surface speed"))
    }

    fn average_normal(&mut self, body: &Self::Body) -> Result<SkeletonRootVector, Self::Error> {
        self.validate_body(body)?;
        self.contact
            .lock()
            .map_err(|_| Self::Error::PoisonedContactState)?
            .collision_state
            .average_normal
            .ok_or(Self::Error::Missing("actual Body average normal"))
    }
}
