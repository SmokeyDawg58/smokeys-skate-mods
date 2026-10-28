//! Actual pair-owned Ground detector adapter, scheduled after Skeleton.Post.
use crate::{
    attribute_runtime::AttributeRuntimeData,
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    dynamic_assembly_session::{
        part_transform::DynamicAssemblyPartTransformError,
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
            SimulationSkeletonJumpPairError, board_construction::SimulationConstructedBoardError,
        },
    },
    input_cvar_runtime::{InputCvarError, ground_wipeout::GroundDrawTarget},
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_ground_wipeout::{
        self as detector, CollisionFlag, GroundWipeoutSource, Matrix, Scalar, Vector,
        attributes::Attribute,
    },
    physical_player_wipeout::ground_access::GroundOwnerError,
    reckoning_numeric::{ReckoningNumeric, try_acos, try_length},
    simulation_assembly_backend::constraint_session::physical_board_forces::PhysicalPlayerBoardForceError,
    skateboard_attributes::{SkateboardAttributeError, SkateboardAttributeRead},
    skateboard_contact_surface::SkateboardContactSurface,
    skeleton_collision_data::SkeletonCollisionData,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::{Arc, Mutex};
#[path = "../air/physical_player_air_bad_landing.rs"]
mod bad_landing;
#[derive(Debug)]
pub(crate) enum GroundWipeoutRuntimeError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    AirEffectiveTransform(
        crate::dynamic_assembly_session::skater_pool::board_entry::SkateboardEntrySessionError,
    ),
    Pair(SimulationSkeletonJumpPairError),
    Pool(SimulationSkaterPoolError),
    Board(PhysicalPlayerBoardForceError),
    BoardAttributes(SimulationConstructedBoardError),
    Transform(DynamicAssemblyPartTransformError),
    Owner(GroundOwnerError),
    Cvar(InputCvarError),
    Attribute(SkateboardAttributeError),
    Numeric(N),
    Overlay(DebugOverlayError),
    DebugBranchPending(&'static str),
    BalanceSpeedAssertion(u32),
    AiPathFollowing(
        crate::skateboard_ai_path_following::AiPathFollowingError<
            crate::dynamic_assembly_session::skater_pool::controller_target::ControllerBoardTargetError,
            N,
        >,
    ),
}
impl<N: std::fmt::Debug> std::fmt::Display for GroundWipeoutRuntimeError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "Ground wipeout missing {v}"),
            Self::Poisoned => f.write_str("Ground wipeout owner poisoned"),
            Self::AirEffectiveTransform(error) => {
                write!(f, "Air wipeout effective transform: {error:?}")
            }
            Self::ForeignOwner => f.write_str("Ground wipeout foreign owner"),
            Self::Pair(e) => write!(f, "Ground wipeout pair: {e:?}"),
            Self::Pool(e) => write!(f, "Ground wipeout pool: {e:?}"),
            Self::Board(e) => write!(f, "Ground wipeout board: {e:?}"),
            Self::BoardAttributes(e) => write!(f, "Ground wipeout retained attributes: {e:?}"),
            Self::Transform(e) => write!(f, "Ground wipeout transform: {e:?}"),
            Self::Owner(e) => write!(f, "Ground wipeout ledger: {e}"),
            Self::Cvar(e) => write!(f, "Ground wipeout cvar: {e:?}"),
            Self::Attribute(e) => write!(f, "Ground wipeout attribute: {e:?}"),
            Self::Numeric(e) => write!(f, "Ground wipeout numeric: {e:?}"),
            Self::Overlay(e) => write!(f, "Ground wipeout overlay: {e:?}"),
            Self::DebugBranchPending(v) => write!(f, "Ground wipeout debug payload pending: {v}"),
            Self::BalanceSpeedAssertion(bits) => {
                write!(f, "Ground wipeout balance speed assertion {bits:08x}")
            }
            Self::AiPathFollowing(error) => write!(f, "Ground.Post {error}"),
        }
    }
}
#[path = "../air/physical_player_air_wipeout_runtime.rs"]
mod air_wipeout;
#[path = "physical_player_ground_animation_post.rs"]
mod animated_ground;
#[path = "../offboard/physical_player_offboard_air_wipeout.rs"]
mod offboard_air_wipeout;

struct Source<'a, N> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    skeleton: &'a SimulationSkaterAssemblyHandle,
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    numeric: &'a mut N,
    overlay: &'a DebugOverlaySlot,
}
impl<N: ReckoningNumeric> Source<'_, N> {
    /// Shared retained physics_wipeout/default reader for the OffBoard callee.
    fn wipeout_pointer(
        &mut self,
        key: u64,
    ) -> Result<Option<AttributeRuntimeData>, GroundWipeoutRuntimeError<N::Error>> {
        self.world
            .skaters
            .as_mut()
            .ok_or(GroundWipeoutRuntimeError::Missing("skater pool"))?
            .try_with_ground_force_board(&self.world.backend, self.board, |_, _, attributes| {
                attributes.lookup(
                    crate::skateboard_attributes::SkateboardAttributeClass::Wipeout,
                    key,
                )
            })
            .map_err(GroundWipeoutRuntimeError::BoardAttributes)?
            .map_err(GroundWipeoutRuntimeError::Attribute)
    }
    fn wipeout_float(
        &mut self,
        pointer: &Option<AttributeRuntimeData>,
    ) -> Result<f32, GroundWipeoutRuntimeError<N::Error>> {
        self.world
            .skaters
            .as_mut()
            .ok_or(GroundWipeoutRuntimeError::Missing("skater pool"))?
            .try_with_ground_force_board(&self.world.backend, self.board, |_, _, attributes| {
                attributes.float(pointer)
            })
            .map_err(GroundWipeoutRuntimeError::BoardAttributes)?
            .map_err(GroundWipeoutRuntimeError::Attribute)
    }
    fn wipeout_hash(&mut self, key: u64) -> Result<f32, GroundWipeoutRuntimeError<N::Error>> {
        let pointer = self.wipeout_pointer(key)?;
        self.wipeout_float(&pointer)
    }
    fn processed<T>(
        &self,
        read: impl FnOnce(&SkeletonProcessedContact) -> Option<T>,
    ) -> Result<T, GroundWipeoutRuntimeError<N::Error>> {
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| GroundWipeoutRuntimeError::Poisoned)?
                .processed(),
        );
        let p = processed
            .lock()
            .map_err(|_| GroundWipeoutRuntimeError::Poisoned)?;
        read(&p).ok_or(GroundWipeoutRuntimeError::Missing(
            "selected Processed field",
        ))
    }
    fn collision<T>(
        &self,
        read: impl FnOnce(&SkeletonCollisionData) -> T,
    ) -> Result<T, GroundWipeoutRuntimeError<N::Error>> {
        Ok(read(
            &self
                .world
                .skeleton(self.skeleton)
                .map_err(GroundWipeoutRuntimeError::Pool)?
                .state()
                .collision,
        ))
    }
    fn contacts(
        &mut self,
    ) -> Result<Arc<Mutex<SkateboardContactSurface>>, GroundWipeoutRuntimeError<N::Error>> {
        self.world
            .try_with_board_force_runtime(self.board, |r| Arc::clone(&r.contact_surface))
            .map_err(GroundWipeoutRuntimeError::Board)
    }
}
impl<N: ReckoningNumeric> GroundWipeoutSource for Source<'_, N> {
    type Error = GroundWipeoutRuntimeError<N::Error>;
    type AttributePointer = (Attribute, Option<AttributeRuntimeData>);
    type DrawTarget = GroundDrawTarget;
    fn state(&mut self) -> Result<i32, Self::Error> {
        self.owner
            .wipeout()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .try_ground_state()
            .map_err(Self::Error::Owner)
    }
    fn set_state(&mut self, v: i32) -> Result<(), Self::Error> {
        self.owner
            .wipeout()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .try_store_ground_state(v)
            .map_err(Self::Error::Owner)
    }
    fn off_balance_total(&mut self) -> Result<f32, Self::Error> {
        self.owner
            .wipeout()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .try_ground_balance()
            .map_err(Self::Error::Owner)
    }
    fn set_off_balance_total(&mut self, v: f32) -> Result<(), Self::Error> {
        self.owner
            .wipeout()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .try_store_ground_balance(v)
            .map_err(Self::Error::Owner)
    }
    fn invincible_timer(&mut self) -> Result<f32, Self::Error> {
        self.owner
            .wipeout()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .try_ground_invincibility()
            .map_err(Self::Error::Owner)
    }
    fn set_invincible_timer(&mut self, v: f32) -> Result<(), Self::Error> {
        self.owner
            .wipeout()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .try_store_ground_invincibility(v)
            .map_err(Self::Error::Owner)
    }
    fn scalar(&mut self, f: Scalar) -> Result<f32, Self::Error> {
        match f {
            Scalar::TimeDelta => self.processed(|p| p.physics_state_fields().time_delta),
            Scalar::SkaterContact => self.collision(|c| c.skater_contact),
            Scalar::VehicleContact => self.collision(|c| c.vehicle_contact),
            Scalar::MaxDisplacement => self
                .world
                .skeleton(self.skeleton)
                .map_err(Self::Error::Pool)?
                .post_physics_records()
                .max_displacement()
                .ok_or(Self::Error::Missing("Skeleton maxDisplacement")),
            Scalar::DeckSpeed => self.processed(|p| p.prior_output().deck_speed().map(|v| v.value)),
            Scalar::UpY => {
                self.processed(|p| p.prior_output().reckoning_up().map(|v| v.value.xyz[1]))
            }
        }
    }
    fn processed_flag(&mut self, o: u32, m: u32) -> Result<bool, Self::Error> {
        self.processed(|p| match o {
            0x8c4 => {
                let (v, k) = p.known_flags_8c4();
                (k & m == m).then_some(v & m != 0)
            }
            0x8c8 => p.known_flags_8c8().map(|v| v & m != 0),
            0x8cc => p.physics_state_fields().flags_8cc.map(|v| v & m != 0),
            _ => None,
        })
    }
    fn collision_flag(&mut self, f: CollisionFlag) -> Result<bool, Self::Error> {
        self.collision(|c| match f {
            CollisionFlag::HitVehicle => c.hit_vehicle != 0,
            CollisionFlag::CompliantJustCollided => c.compliant_just_collided != 0,
            CollisionFlag::OpposingContacts => c.opposing_contacts != 0,
        })
    }
    fn board_contact_flag(&mut self, m: u32) -> Result<bool, Self::Error> {
        self.contacts()?
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .collision_state
            .flags
            .read(m)
            .map(|v| v != 0)
            .ok_or(Self::Error::Missing("board contact named bit"))
    }
    fn num_parts_in_contact(&mut self) -> Result<u8, Self::Error> {
        Ok(self
            .world
            .board_construction(self.board)
            .map_err(Self::Error::Pool)?
            .and_then(crate::skateboard_body_construction::SkateboardBodyConstruction::post_fields)
            .ok_or(Self::Error::Missing("board post fields"))?
            .num_parts)
    }
    fn vector(&mut self, f: Vector) -> Result<SkeletonRootVector, Self::Error> {
        match f {
            Vector::HookToHips => Ok(self
                .world
                .skeleton(self.skeleton)
                .map_err(Self::Error::Pool)?
                .post_physics_records()
                .hook_to_hips()),
            Vector::MaxContactVelocity => self
                .contacts()?
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .post_prefix
                .max_velocity
                .ok_or(Self::Error::Missing("body290 maxContactVel")),
            Vector::MostVerticalContactNormal => self.collision(|c| SkeletonRootVector {
                xyz: *c.most_vertical_contact_normal.xyz(),
                w: c.most_vertical_contact_normal.known_lanes().map(|v| v[3]),
            }),
        }
    }
    fn matrix(&mut self, f: Matrix) -> Result<SkeletonRootMatrix, Self::Error> {
        match f {
            Matrix::BoardTransform => self
                .world
                .board(self.board)
                .map_err(Self::Error::Pool)?
                .try_live_part_transform_typed(&self.world.backend, 6)
                .map_err(Self::Error::Transform),
            Matrix::ProcessedSkateTransform => {
                self.processed(|p| p.skateboard_orientation().copied())
            }
            Matrix::WorldToAnimation => Ok(*self
                .world
                .skeleton(self.skeleton)
                .map_err(Self::Error::Pool)?
                .root_transforms()
                .world_to_animation()),
        }
    }
    fn contact_force(&mut self, i: usize) -> Result<f32, Self::Error> {
        self.collision(|c| c.contact_force[i])
    }
    fn attribute(&mut self, a: Attribute) -> Result<Self::AttributePointer, Self::Error> {
        self.world
            .skaters
            .as_mut()
            .ok_or(Self::Error::Missing("skater pool"))?
            .try_with_ground_force_board(&self.world.backend, self.board, |_, _, attributes| {
                detector::attributes::lookup(attributes, a)
            })
            .map_err(Self::Error::BoardAttributes)?
            .map(|pointer|(a,pointer)).map_err(Self::Error::Attribute)
    }
    fn attribute_value(&mut self, p: &Self::AttributePointer) -> Result<f32, Self::Error> {
        #[cfg(feature = "skate3-runtime")]
        if p.0 == Attribute::GroundXZ {
            if let Some(tuning) = &*self.owner.skate3_tuning.lock().map_err(|_| Self::Error::Poisoned)? {
                return Ok(tuning.ground_wipeout_xz);
            }
        }
        self.world
            .skaters
            .as_mut()
            .ok_or(Self::Error::Missing("skater pool"))?
            .try_with_ground_force_board(&self.world.backend, self.board, |_, _, attributes| {
                attributes.float(&p.1)
            })
            .map_err(Self::Error::BoardAttributes)?
            .map_err(Self::Error::Attribute)
    }
    fn capture_draw_target(&mut self) -> Result<GroundDrawTarget, Self::Error> {
        self.owner
            .inputs()
            .cvars
            .try_capture_ground_wipeout_draw()
            .map_err(Self::Error::Cvar)
    }
    fn read_draw_target(&mut self, t: &GroundDrawTarget) -> Result<bool, Self::Error> {
        t.read().map_err(Self::Error::Cvar)
    }
    fn wipeout_on_collision(&mut self) -> Result<bool, Self::Error> {
        self.owner
            .inputs()
            .cvars
            .try_ground_wipeout_on_collision()
            .map_err(Self::Error::Cvar)
    }
    fn draw_squashed(&mut self) -> Result<(), Self::Error> {
        self.overlay.require().map_err(Self::Error::Overlay)?;
        Err(Self::Error::DebugBranchPending("squashed"))
    }
    fn draw_displacement(&mut self) -> Result<(), Self::Error> {
        Err(Self::Error::DebugBranchPending("displacement"))
    }
    fn draw_contact_force(&mut self, _: f32, _: f32) -> Result<(), Self::Error> {
        self.overlay.require().map_err(Self::Error::Overlay)?;
        Err(Self::Error::DebugBranchPending("contact force"))
    }
    fn require_positive_balance_speed(&mut self, speed: f32) -> Result<(), Self::Error> {
        Err(Self::Error::BalanceSpeedAssertion(speed.to_bits()))
    }
    fn dot(&mut self, a: SkeletonRootVector, b: SkeletonRootVector) -> Result<f32, Self::Error> {
        self.numeric.dot_vector(a, b).map_err(Self::Error::Numeric)
    }
    fn length(&mut self, v: SkeletonRootVector) -> Result<f32, Self::Error> {
        try_length(v, self.numeric).map_err(Self::Error::Numeric)
    }
    fn acos(&mut self, v: f32) -> Result<f32, Self::Error> {
        try_acos(v, self.numeric).map_err(Self::Error::Numeric)
    }
    /// Existing host VMX selection boundary; does not establish exceptional parity.
    fn minimum(&mut self, a: f32, b: f32) -> Result<f32, Self::Error> {
        Ok(a.min(b))
    }
    fn maximum(&mut self, a: f32, b: f32) -> Result<f32, Self::Error> {
        Ok(a.max(b))
    }
    fn request(&mut self, i: usize) -> Result<(), Self::Error> {
        self.owner
            .wipeout()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .try_ground_request(i)
            .map_err(Self::Error::Owner)
    }
}
impl PhysicalAssemblyWorld {
    /// Wipeout::CheckForOffBoardWipeout 0x82ddee24..f014 after its first
    /// speed threshold. Reuse the same retained Skeleton collision leaves as
    /// Ground, but keep the OffBoard-specific vehicle threshold and ordering.
    pub(crate) fn try_check_offboard_high_speed_wipeout<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        let mut source = Source {
            world: self,
            board,
            skeleton,
            owner,
            numeric,
            overlay,
        };
        let max_displacement = source.wipeout_hash(0xc1d6_4bcc_60f0_8410)?;
        let vehicle_scalar = if source.collision_flag(CollisionFlag::HitVehicle)? {
            let scalar = source.wipeout_hash(0xd3e5_2d70_accc_4432)?;
            let contact_threshold = source.wipeout_hash(0x155f_79ee_f944_8122)?;
            if source.scalar(Scalar::VehicleContact)? > contact_threshold {
                source.request(7)?;
            }
            scalar
        } else {
            1.0
        };
        if detector::leaves::squashed(&mut source, vehicle_scalar * max_displacement)? {
            source.request(18)?;
            return Ok(());
        }
        let skeleton_max_displacement = source.wipeout_hash(0xb2b0_46b5_00ea_daee)?;
        if detector::leaves::displacement(&mut source, skeleton_max_displacement * vehicle_scalar)?
        {
            source.request(1)?;
            return Ok(());
        }
        // Native looks up both pointers before loading either float.
        let arms = source.wipeout_pointer(0x6167_b209_2124_3e65)?;
        let body = source.wipeout_pointer(0x9022_1e4c_63a2_99c5)?;
        let arm_limit = source.wipeout_float(&arms)?;
        let body_limit = source.wipeout_float(&body)?;
        if detector::leaves::contact_force(
            &mut source,
            vehicle_scalar * body_limit,
            arm_limit * vehicle_scalar,
        )? {
            source.request(0)?;
            if owner
                .inputs()
                .cvars
                .try_wipeout_display_debug_info()
                .map_err(E::Cvar)?
            {
                return Err(E::DebugBranchPending("OffBoard contact force"));
            }
        }
        Ok(())
    }
    /// Wipeout::CheckForPlantWipeout 0x82dde908 (Boneless/FootPlant UpdatePostPhysics).
    /// State 3 without the Ground balance reset, then the shared Skeleton leaves
    /// with the unscaled plant thresholds, then the deck test with Trick X/Z,Y.
    pub(crate) fn try_check_plant_wipeout<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::ForeignOwner);
        }
        let mut source = Source {
            world: self,
            board,
            skeleton,
            owner,
            numeric,
            overlay,
        };
        if source.state()? != 3 {
            source.set_state(3)?;
        }
        #[cfg(feature = "skate3-runtime")]
        if owner.physics_state().lock().map_err(|_|E::Poisoned)?.current
            ==crate::physical_player_physics_state::CurrentPhysicalState::FootPlant {
            let active={
                let mut foot=owner.footplant().lock().map_err(|_|E::Poisoned)?;
                foot.try_manager(owner,skeleton).map_err(|_|E::Missing("owned footplant manager"))?
                    .source3.as_ref().is_some_and(|source|source.active)
            };
            if active {
                use skate3_core::player::wipeout::{PlantInput,PlantContact,check_plant_input};
                use super::skate3_footplant_binding::lanes;
                let maximum_pose_error=source.scalar(Scalar::MaxDisplacement)?;
                let pose_error=lanes(source.vector(Vector::HookToHips)?);
                let mut regions_force=[0.;8];
                for part in [0,1,5,4,7,6,2,3] {regions_force[part]=source.contact_force(part)?;}
                let contact=if source.num_parts_in_contact()?!=0 {
                    let world_to_animation=source.matrix(Matrix::WorldToAnimation)?.columns().map(lanes);
                    let closing_velocity=lanes(source.vector(Vector::MaxContactVelocity)?);
                    Some(PlantContact {world_to_animation,closing_velocity})
                } else {None};
                let input=PlantInput {maximum_pose_error,pose_error,regions_force,contact};
                let mut requests=Vec::new();
                {
                    let mut foot=owner.footplant().lock().map_err(|_|E::Poisoned)?;
                    let manager=foot.try_manager(owner,skeleton).map_err(|_|E::Missing("owned footplant manager"))?;
                    let settings=&manager.source3.as_ref().ok_or(E::Missing("source footplant manager"))?.plant_wipeout;
                    check_plant_input(settings,&input,|reason,value|requests.push((reason,value)));
                }
                // Preserve ordered triggers, including a repeated force reason;
                // the native request counter counts calls, not unique bits.
                for (reason,value) in requests {
                    owner.wipeout().lock().map_err(|_|E::Poisoned)?
                        .try_wipeout_request_value(reason,value).map_err(E::Owner)?;
                    crate::diagnostics::test_log::event("skate3_footplant_collision",format_args!(
                        "reason={reason} value={value} native_s2_solver=true"));
                }
                return Ok(());
            }
        }
        let squash = source.wipeout_hash(Attribute::Squash.key())?;
        if detector::leaves::squashed(&mut source, squash)? {
            source.request(18)?;
        } else {
            let displacement = source.wipeout_hash(0xaa44_05c5_949d_1845)?;
            if detector::leaves::displacement(&mut source, displacement)? {
                source.request(1)?;
            } else {
                // Native looks up both pointers before loading either float.
                let arms = source.wipeout_pointer(0xd031_7181_ce40_bfbc)?;
                let body = source.wipeout_pointer(0x68e0_e915_d4f3_af66)?;
                let arm_limit = source.wipeout_float(&arms)?;
                let body_limit = source.wipeout_float(&body)?;
                if detector::leaves::contact_force(&mut source, body_limit, arm_limit)? {
                    source.request(0)?;
                    if owner
                        .inputs()
                        .cvars
                        .try_wipeout_display_debug_info()
                        .map_err(E::Cvar)?
                    {
                        // 0x82ddeb84: DebugOverlay Get asserts; the print is BLR825187c0.
                        overlay.require().map_err(E::Overlay)?;
                    }
                }
            }
        }
        let y = source.wipeout_pointer(Attribute::TrickY.key())?;
        let xz = source.wipeout_pointer(Attribute::TrickXZ.key())?;
        let y = source.wipeout_float(&y)?;
        let xz = source.wipeout_float(&xz)?;
        detector::leaves::deck_acceleration(&mut source, xz, y)
    }
    /// Wipeout::CheckForGrindWipeout 0x82ddf248, the grind types'
    /// UpdatePostPhysics 0x82d85e78 (and Nonspecific's 0x82d86768 under
    /// kDoWipeoutsTest): state 5, the Skeleton displacement leaf or else the
    /// contact-force leaf, then deck acceleration (fixed Y 100) and flipped.
    pub(crate) fn try_check_grind_wipeout<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::ForeignOwner);
        }
        let mut source = Source {
            world: self,
            board,
            skeleton,
            owner,
            numeric,
            overlay,
        };
        if source.state()? != 5 {
            source.set_state(5)?;
        }
        let displacement = source.wipeout_hash(0x55b7_bac4_51c5_4a53)?;
        if detector::leaves::displacement(&mut source, displacement)? {
            source.request(1)?;
        } else {
            // Native looks up both pointers before loading either float.
            let arms = source.wipeout_pointer(0x744b_b592_dd7e_0cfb)?;
            let body = source.wipeout_pointer(0xd50a_c973_07b0_117c)?;
            let arm_limit = source.wipeout_float(&arms)?;
            let body_limit = source.wipeout_float(&body)?;
            if detector::leaves::contact_force(&mut source, body_limit, arm_limit)? {
                source.request(0)?;
            }
        }
        let xz = source.wipeout_hash(0x9f1c_2ef3_0c74_9332)?;
        detector::leaves::deck_acceleration(&mut source, xz, 100.0)?;
        let angle = source.wipeout_hash(0xe3f9_4cf1_02b1_b35f)?;
        detector::leaves::deck_flipped(&mut source, angle)
    }
    /// Ground.Post82d8e998: detector, then the AI tail it shares with FollowPath.
    pub(crate) fn try_update_ground_postphysics<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        self.try_check_ground_wipeout(board, skeleton, owner, overlay, numeric)?;
        self.try_update_follow_path_postphysics(board, owner, numeric)
    }
    /// `Skateboard::UpdateAIPathFollowing` on a fresh processed AI input whose DoAIAdjust bit is
    /// set: Ground.Post's tail, and all of FollowPath.UpdatePostPhysics 0x82d91138.
    pub(crate) fn try_update_follow_path_postphysics<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut N,
    ) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let (bits, known) = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .ai_flags;
        if known & 0x8000_0000 == 0 {
            return Err(E::Missing("Processed AI flags6b0 high bit"));
        }
        if bits & 0x8000_0000 != 0 {
            self.try_update_ai_path_following(board, owner, numeric)
                .map_err(E::AiPathFollowing)?;
        }
        Ok(())
    }
    /// Parent must schedule after SAME Skeleton.Post; this validates owner/pair
    /// but is not proof that earlier source callbacks have run this tick.
    pub(crate) fn try_check_ground_wipeout<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::ForeignOwner);
        }
        detector::try_check_ground(&mut Source {
            world: self,
            board,
            skeleton,
            owner,
            numeric,
            overlay,
        })
    }
}

#[path = "../runtime/physical_player_landing_state_wipeout.rs"]
pub(crate) mod landing_state;
