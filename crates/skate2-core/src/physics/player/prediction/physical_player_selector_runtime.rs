//! Completed-query consumption on the same retained pair, Vault and World queries.
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance},
    collision_line_numeric::HostCollisionLineNumeric,
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_construction::SimulationConstructedBoardError,
    },
    grind_data_runtime::GrindDataQueryResult,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_grind_manager::{
        math::GrindNum,
        view::{GrindAttributes, GrindCvars, GrindError, error, lanes},
    },
    physical_player_trajectory_owners::{
        OnboardTrajectoryOwner,
        selector_grind::{
            SelectorGrindProcessed, SelectorGrindWorld, calculate_grind_penalty,
            calculate_grind_score,
        },
        selector_update::{SelectorError, SelectorGrindPolicy, SelectorProgress, SelectorSource},
    },
    physics::PointGraph,
    reckoning_attributes::{
        ReckoningAttributeError, ReckoningAttributeRead, ReckoningAttributeScope,
        ReckoningVaultAttributes,
    },
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    simulation_assembly_backend::constraint_session::{
        grind_world::PhysicalGrindServices,
        ground_pre_solve::GroundPreparationGrindPolicy,
        physical_grind::WorldLines,
        world_line_query::{SimulationWorldLineError, SimulationWorldLineQueries},
    },
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    skateboard_foot_attributes::{
        SkateboardFootAttributeError, SkateboardFootAttributeRead, SkateboardFootVaultAttributes,
        wallride::WallRideTrajectoryAttributes,
    },
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum SelectorLiveError {
    Missing(&'static str),
    Poisoned,
    Attribute(ReckoningAttributeError),
    Layout(AttributeRuntimeError),
    Foot(SkateboardFootAttributeError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    Query(SimulationWorldLineError<ReckoningHostNumericError>),
    Grind(GrindError),
}
impl std::fmt::Display for SelectorLiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "selector missing {name}"),
            Self::Poisoned => f.write_str("selector lock poisoned"),
            Self::Attribute(e) => write!(f, "selector attribute: {e}"),
            Self::Layout(e) => write!(f, "selector layout: {e}"),
            Self::Foot(e) => write!(f, "selector feet: {e}"),
            Self::Cvar(e) => write!(f, "selector cvar: {e:?}"),
            Self::Overlay(e) => write!(f, "selector overlay: {e:?}"),
            Self::Query(e) => write!(f, "selector query: {e}"),
            Self::Grind(e) => write!(f, "selector {e}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum SelectorRuntimeError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Pair(SimulationSkeletonJumpPairError),
    Board(SimulationConstructedBoardError),
    Complete(SelectorError<SelectorLiveError, N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for SelectorRuntimeError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "selector runtime missing {name}"),
            Self::Poisoned => f.write_str("selector runtime lock poisoned"),
            Self::ForeignOwner => f.write_str("selector foreign owner"),
            Self::Pair(e) => write!(f, "selector pair: {e:?}"),
            Self::Board(e) => write!(f, "selector board: {e:?}"),
            Self::Complete(e) => write!(f, "selector completion: {e}"),
        }
    }
}
struct Live<'a> {
    #[cfg(feature = "skate3-runtime")]
    mod_grind_lock: Option<f32>,
    processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    attributes: &'a mut ReckoningVaultAttributes,
    trajectory: &'a WallRideTrajectoryAttributes,
    feet: &'a mut SkateboardFootVaultAttributes,
    cvars: &'a InputCvarReferences,
    overlay: &'a DebugOverlaySlot,
    queries: Option<&'a mut SimulationWorldLineQueries>,
    dmos: Option<&'a SimulationDmoAssemblyPool>,
    ground: &'a StaticGroundPool,
    grind_services: &'a PhysicalGrindServices,
    grinds: &'a GrindAttributes,
    /// `physicsdeck` default, `VGlobalAttributeCollection` +0x94.
    deck: &'a AttributeRuntimeInstance,
}
fn word(layout: Option<&AttributeRuntimeData>, offset: usize) -> f32 {
    layout.map_or(0.0, |data| {
        f32::from_be_bytes(
            data.bytes()[offset..offset + 4]
                .try_into()
                .expect("captured24c"),
        )
    })
}
impl SelectorSource for Live<'_> {
    type Error = SelectorLiveError;
    type Layout = Option<AttributeRuntimeData>;
    fn minimum_frames(&mut self) -> Result<i32, Self::Error> {
        let pointer = self
            .attributes
            .lookup(ReckoningAttributeScope::Global, 0x3077_4f8a_272f_682b)
            .map_err(Self::Error::Attribute)?;
        self.attributes
            .signed_word(&pointer)
            .map_err(Self::Error::Attribute)
    }
    fn layout(&mut self) -> Result<Self::Layout, Self::Error> {
        self.trajectory.capture().map_err(Self::Error::Layout)
    }
    fn scalar(&mut self, layout: &Self::Layout, offset: usize) -> Result<f32, Self::Error> {
        Ok(word(layout.as_ref(), offset))
    }
    fn graph(
        &mut self,
        layout: &Self::Layout,
        offset: usize,
        count: usize,
        x: f32,
    ) -> Result<f32, Self::Error> {
        match count {
            8 => Ok(PointGraph::<8> {
                x: std::array::from_fn(|i| word(layout.as_ref(), offset + i * 4)),
                y: std::array::from_fn(|i| word(layout.as_ref(), offset + 32 + i * 4)),
            }
            .evaluate(x)),
            4 => Ok(PointGraph::<4> {
                x: std::array::from_fn(|i| word(layout.as_ref(), offset + i * 4)),
                y: std::array::from_fn(|i| word(layout.as_ref(), offset + 16 + i * 4)),
            }
            .evaluate(x)),
            _ => Err(Self::Error::Missing("native graph size")),
        }
    }
    fn feet(&mut self, key: u64) -> Result<f32, Self::Error> {
        let pointer = self.feet.lookup(key).map_err(Self::Error::Foot)?;
        self.feet.float(&pointer).map_err(Self::Error::Foot)
    }
    fn central(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_select_central_trajectory()
            .map_err(Self::Error::Cvar)
    }
    fn raw_debug(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_show_raw_trajectory()
            .map_err(Self::Error::Cvar)
    }
    fn wall_debug(&mut self) -> Result<bool, Self::Error> {
        self.cvars.try_show_wall_info().map_err(Self::Error::Cvar)
    }
    fn overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
    fn up(&mut self) -> Result<V, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .prior_output()
            .reckoning_up()
            .map(|v| v.value)
            .ok_or(Self::Error::Missing("up220"))
    }
    fn transition(&mut self) -> Result<f32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .input_controls()
            .transition
            .ok_or(Self::Error::Missing("transition95c"))
    }
    fn ground_normal(&mut self) -> Result<V, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .prior_output()
            .ground_normal()
            .map(|v| v.value)
            .ok_or(Self::Error::Missing("ground1d0"))
    }
    fn ground_velocity(&mut self) -> Result<V, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .ground_position_fields_mut()
            .damped_ground_velocity
            .ok_or(Self::Error::Missing("groundVelocity200"))
    }
    fn line(&mut self, start: V, end: V) -> Result<Option<(V, V)>, Self::Error> {
        let result = self
            .queries
            .as_mut()
            .ok_or(Self::Error::Missing("World queries"))?
            .try_line_test(
                self.ground,
                self.dmos.ok_or(Self::Error::Missing("DMO collection"))?,
                crate::collision_line_collection::CollisionLineInput {
                    start,
                    end,
                    fatness: 0.0,
                },
                "TrajectorySelector::CalculateWallRideScore",
                &mut crate::collision_line_numeric::HostCollisionLineNumeric,
            )
            .map_err(Self::Error::Query)?;
        Ok(result.map(|hit| (hit.position, hit.normal)))
    }
    fn grind_scoring_allowed(&mut self) -> Result<bool, Self::Error> {
        let p = self.processed.lock().map_err(|_| Self::Error::Poisoned)?;
        let flags = p
            .known_flags_8c8()
            .ok_or(Self::Error::Missing("flags8c8"))?;
        if flags & 0x4000_0000 == 0 {
            return Ok(true);
        }
        let (ai, known) = p.physics_state_fields().ai_flags;
        if known & 0x0400_0000 == 0 {
            return Err(Self::Error::Missing("AI valid trajectory6b0"));
        }
        if ai & 0x0400_0000 == 0 {
            return Ok(true);
        }
        if known & 0x0800_0000 == 0 {
            return Err(Self::Error::Missing("AI trajectory enters grind6b0"));
        }
        Ok(ai & 0x0800_0000 != 0)
    }
    fn avoid_grinds(&mut self) -> Result<bool, Self::Error> {
        let p = self.processed.lock().map_err(|_| Self::Error::Poisoned)?;
        let flags = p
            .physics_state_fields()
            .flags_8cc
            .ok_or(Self::Error::Missing("flags8cc"))?;
        Ok(flags & 0x0800_0000 != 0)
    }
    fn grind_score(
        &mut self,
        selector: &mut OnboardTrajectoryOwner,
        index: usize,
        landing_velocity: V,
    ) -> Result<f32, Self::Error> {
        let processed = {
            let p = self.processed.lock().map_err(|_| Self::Error::Poisoned)?;
            let deck = p
                .deck_transform()
                .ok_or(Self::Error::Missing("deck transform40"))?;
            SelectorGrindProcessed {
                deck_translation: lanes(deck.columns()[3]),
                centre_of_mass: p.prior_output().centre_of_mass().map(|c| lanes(c.value)),
            }
        };
        let layout = self.trajectory.capture().map_err(Self::Error::Layout)?;
        let services = self.grind_services;
        let cvars = self.cvars;
        let mut box_test = |bounds, result: &mut GrindDataQueryResult| {
            services
                .try_box_test(bounds, result, false, cvars)
                .map_err(error)
        };
        let mut line_numeric = HostCollisionLineNumeric;
        let mut lines = WorldLines {
            queries: self
                .queries
                .as_deref_mut()
                .ok_or(Self::Error::Missing("World queries"))?,
            dmos: self.dmos.ok_or(Self::Error::Missing("DMO collection"))?,
            ground: self.ground,
            numeric: &mut line_numeric,
        };
        let mut world = SelectorGrindWorld {
            box_test: &mut box_test,
            lines: &mut lines,
            grinds: self.grinds,
            trajectory: self.trajectory,
            layout: layout.as_ref(),
            deck: self.deck,
            cvars: GrindCvars(cvars),
            overlay: self.overlay,
            processed,
            #[cfg(feature = "skate3-runtime")]
            mod_grind_lock: self.mod_grind_lock,
        };
        let mut numeric = HostReckoningNumeric;
        let mut sin_cos = HostReckoningSinCosNumeric;
        calculate_grind_score(
            selector,
            index,
            lanes(landing_velocity),
            &mut world,
            &mut GrindNum {
                n: &mut numeric,
                s: &mut sin_cos,
            },
        )
        .map_err(Self::Error::Grind)
    }
    fn grind_penalty(
        &mut self,
        selector: &OnboardTrajectoryOwner,
        index: usize,
    ) -> Result<f32, Self::Error> {
        let layout = self.trajectory.capture().map_err(Self::Error::Layout)?;
        calculate_grind_penalty(selector, index, layout.as_ref(), &mut HostReckoningNumeric)
            .map_err(Self::Error::Grind)
    }
}
impl PhysicalAssemblyWorld {
    /// Explicit policy required by every caller; no default parked branch.
    pub(crate) fn try_complete_pair_selector_prefix<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        numeric: &mut N,
        policy: GroundPreparationGrindPolicy,
    ) -> Result<SelectorProgress, SelectorRuntimeError<N::Error>> {
        use SelectorRuntimeError as E;
        let policy = match policy {
            GroundPreparationGrindPolicy::ParkedByUser => {
                SelectorGrindPolicy::ParkedForFirstSkatingTest
            }
            GroundPreparationGrindPolicy::RunRecoveredPrefix => SelectorGrindPolicy::Native,
        };
        let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
        let owner = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut attributes = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .reckoning()
            .ok_or(E::Missing("Reckoning"))?
            .cloned_attribute_bindings();
        let vault = &owner.inputs().vault;
        let grinds = GrindAttributes::bind(vault);
        let deck = vault.instance(0x9ed2_91a2_7445_3fc0, 0xd7ed_bd36_2d7d_2152);
        #[cfg(feature = "skate3-runtime")]
        let mod_grind_lock = owner.skate3_tuning.lock().map_err(|_| E::Poisoned)?.as_ref().map(|tuning|tuning.grind_lock_distance);
        pool.try_with_ground_control_board(&self.backend, board, |runtime, _| {
            let mut trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
            let selector = trajectories
                .onboard
                .as_mut()
                .ok_or(E::Missing("onboard selector"))?;
            if !selector.bindings.matches(&owner, &processed) {
                return Err(E::ForeignOwner);
            }
            selector
                .try_complete_query_prefix(
                    &mut Live {
                        #[cfg(feature = "skate3-runtime")]
                        mod_grind_lock,
                        processed: &processed,
                        attributes: &mut attributes,
                        trajectory: &runtime.wallride_trajectory,
                        feet: &mut runtime.foot_attributes,
                        cvars: &owner.inputs().cvars,
                        overlay: &self.debug_overlay,
                        queries: self.world_line_queries.as_mut(),
                        dmos: self.dmos.as_ref(),
                        ground,
                        grind_services: &self.grind_services,
                        grinds: &grinds,
                        deck: &deck,
                    },
                    numeric,
                    policy,
                )
                .map_err(E::Complete)
        })
        .map_err(E::Board)?
    }
}
