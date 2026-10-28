//! Actual board `PostPhysics` through ground-normal publication (82b37498).
//! Time-since-wheels and truck continuation still follow this selected endpoint.
use super::{
    board_normal_services::{
        PhysicalPlayerBoardNormalServiceError, PhysicalPlayerBoardNormalServices,
    },
    board_post_services::{
        PhysicalPlayerBoardPostServiceError, PhysicalPlayerBoardPostServices, SkateboardPostCvars,
    },
    physical_board_forces::PhysicalPlayerBoardForceError,
    physical_contact_list::SimulationPhysicalContactListError,
    physical_wheel_lines::PhysicalPlayerBoardPostPrefixError,
};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skateboard_contact_surface::{
        CompletedSkateboardContacts, SkateboardContactFrameError, SkateboardContactSurfaceCvars,
        SkateboardContactSurfaceError,
        motion::SkateboardContactMotionError,
        normal_attributes::SkateboardGroundNormalAttributeError,
        normal_tail::{SkateboardContactNormalError, SkateboardContactNormalOutcome},
        prefix::SkateboardContactPrefixError,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{convert::Infallible, sync::Arc};

#[derive(Debug)]
pub(crate) enum PhysicalPlayerBoardPostError<N, C, R> {
    Authority(PhysicalPlayerBoardForceError),
    Prefix(PhysicalPlayerBoardPostPrefixError<N>),
    Frame(SkateboardContactFrameError),
    List(SimulationPhysicalContactListError),
    Refused(&'static str),
    Begin(SkateboardContactPrefixError<Infallible, &'static str>),
    Contact(SkateboardContactPrefixError<PhysicalPlayerBoardPostServiceError<N, C>, &'static str>),
    Motion(SkateboardContactMotionError<PhysicalPlayerBoardPostServiceError<N, C>>),
    Vote(SkateboardContactSurfaceError<Infallible, C>),
    Normal(
        SkateboardContactNormalError<
            SkateboardGroundNormalAttributeError,
            PhysicalPlayerBoardNormalServiceError<C>,
            N,
            R,
        >,
    ),
}
struct VoteCvars<'a, C>(&'a mut C);
impl<N: std::fmt::Debug, C: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display
    for PhysicalPlayerBoardPostError<N, C, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Authority(e) => write!(f, "board authority: {e:?}"),
            Self::Prefix(e) => write!(f, "board entry: {e}"),
            Self::Frame(e) => write!(f, "board frame: {e}"),
            Self::List(e) => write!(f, "board list: {e:?}"),
            Self::Refused(e) => write!(f, "board post stage: {e}"),
            Self::Begin(e) => write!(f, "board begin: {e}"),
            Self::Contact(e) => write!(f, "board contact: {e}"),
            Self::Motion(e) => write!(f, "board motion: {e:?}"),
            Self::Vote(e) => write!(f, "board surface: {e:?}"),
            Self::Normal(e) => write!(f, "board normal: {e}"),
        }
    }
}
impl<C: SkateboardPostCvars> SkateboardContactSurfaceCvars for VoteCvars<'_, C> {
    type Error = C::Error;
    fn forced_world_surface(&mut self) -> Result<u32, Self::Error> {
        self.0.forced_surface()
    }
}
impl PhysicalAssemblyWorld {
    #[expect(
        clippy::too_many_lines,
        clippy::type_complexity,
        reason = "single ordered contact producer with explicit failure prefixes"
    )]
    pub(crate) fn try_update_board_post_ground_normals<C, N, R>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        timestep: f32,
        up: SkeletonRootVector,
        cvars: &mut C,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<
        SkateboardContactNormalOutcome,
        PhysicalPlayerBoardPostError<N::Error, C::Error, R::Error>,
    >
    where
        C: SkateboardPostCvars,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    {
        use PhysicalPlayerBoardPostError as E;
        let (state, attributes) = self
            .try_with_board_force_runtime(board, |runtime| {
                (
                    Arc::clone(&runtime.contact_surface),
                    Arc::clone(&runtime.ground_normal_attributes),
                )
            })
            .map_err(E::Authority)?;
        let frame = Arc::clone(
            self.contact_spies
                .as_ref()
                .ok_or(E::Refused("missing solved frame"))?,
        );
        let (debug, schedule) = {
            let construction = self
                .board_construction(board)
                .map_err(|_| E::Refused("board construction owner"))?
                .ok_or(E::Refused("missing board construction"))?;
            (
                Arc::clone(
                    construction
                        .debug_colors()
                        .ok_or(E::Refused("missing actual debug objects"))?,
                ),
                Arc::clone(construction.wheel_schedule()),
            )
        };
        if !schedule
            .lock()
            .map_err(|_| E::Refused("wheel schedule poisoned"))?
            .matches(&frame)
        {
            return Err(E::Refused(
                "wheel Start/End does not belong to current solved frame",
            ));
        }
        let mut state = state
            .lock()
            .map_err(|_| E::Refused("contact state poisoned"))?;
        // Validate replay/current body before mutating the actual activation pool.
        {
            let completed = CompletedSkateboardContacts::try_new(
                &frame,
                &self.physical_contacts,
                &self.contacts,
                self.backend.collision_body_pool(),
                board,
            )
            .map_err(E::Frame)?;
            if !state.begin_post_frame(&completed) {
                return Err(E::Refused("contact frame already attempted"));
            }
        }
        self.try_board_post_physics_prefix(board, timestep, numeric)
            .map_err(E::Prefix)?;
        self.try_get_physical_collisions(board).map_err(E::List)?;
        {
            let completed = CompletedSkateboardContacts::try_new(
                &frame,
                &self.physical_contacts,
                &self.contacts,
                self.backend.collision_body_pool(),
                board,
            )
            .map_err(E::Frame)?;
            let mut debug = debug
                .lock()
                .map_err(|_| E::Refused("debug state poisoned"))?;
            if !debug.belongs_to(
                self.board(board)
                    .map_err(|_| E::Refused("board definition owner"))?
                    .template()
                    .collision(),
            ) {
                return Err(E::Refused("debug objects belong to another definition"));
            }
            state.begin_post_selection(&mut *debug).map_err(E::Begin)?;
            let mut services = PhysicalPlayerBoardPostServices {
                world: self,
                board,
                numeric,
                cvars,
            };
            for contact in completed.contacts() {
                state
                    .process_post_contact(contact, up, &mut services, &mut *debug)
                    .map_err(E::Contact)?;
            }
            state
                .try_finish_line_fallback(&mut services)
                .map_err(E::Motion)?;
            state
                .try_update_contact_motion(timestep, &mut services)
                .map_err(E::Motion)?;
        }
        state
            .finish::<Infallible, _>(&mut VoteCvars(cvars))
            .map_err(E::Vote)?;
        let best = state
            .post_prefix
            .best_part
            .map_or(-1, |part| i32::try_from(part).expect("seven Parts"));
        let mut attributes = attributes
            .lock()
            .map_err(|_| E::Refused("normal attributes poisoned"))?;
        let result = state
            .try_finish_ground_normals(
                best,
                up,
                &mut *attributes,
                &mut PhysicalPlayerBoardNormalServices::new(self, board, cvars),
                numeric,
                rounding,
            )
            .map_err(E::Normal)?;
        state.normal_frame = Some(frame);
        state.post_timestep = Some(timestep);
        Ok(result)
    }
}
