//! `PhysState_Grind_Common` and its five types (400..404), and
//! `PhysState_Nonspecific` (701), on their retained remaining-state payloads.
//! Grind: Enter 0x82d85098 (Boardslide 0x82d85e80), Exit 0x82d8fa10, Update
//! 0x82d9b3e8 with `UpdateGrindReckoning` 0x82d85298, `ManageSubstate`
//! 0x82d901a8, `UpdateSubstate` 0x82d90230, the per-type `Update_Contact`
//! (0x82d9fe48..0x82da00d0) and `Update_InvoluntaryExit` (0x82d85eb8,
//! 0x82d86098, 0x82d86198, 0x82d86478, 0x82d865e0). Nonspecific: Enter
//! 0x82d86710, Update 0x82d9bfc0, `UpdateSkateboard` 0x82d91030.
#[path = "physical_player_grind_state_forces.rs"]
pub(crate) mod forces;
#[path = "physical_player_grind_state_orient.rs"]
pub(crate) mod orient;

use super::{
    antiflip_source::AntiFlipRuntime, ground_control_complete::SteeringInput,
    ground_pre_solve::HostGroundPreparationNumeric, physical_grind::apply_effects,
};
use crate::{
    actor_construction::ActorConstruction,
    debug_overlay::DebugOverlaySlot,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle as Handle,
        board_construction::ground_control::Source as PhysicsErrorSource,
    },
    input_cvar_runtime::{
        InputCvarReferences, grind_states::GrindCvar as C, reckoning::ReckoningCvarAdapter,
    },
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{GrindCommon, StateKind, StatePayload},
    },
    physical_player_grind_manager::{
        math::{GrindNum, V4, ZERO, abs1, cross, scale},
        suffix::SuffixEffects,
        types::{FAT_RAIL, Investigation, LEDGE},
        view::{GrindAttributes, GrindCvars, GrindView, HEAD_TRACKING_LEAD, lanes, root},
    },
    physical_player_physics_state::GrindKind,
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    skateboard_foot_forces::SkateboardFootForce,
    skateboard_force_runtime::antiflip::try_antiflip,
    skateboard_slide_collision::SkateboardSlideCollisionServices,
    skeleton_processed_contact::{SkeletonProcessedContact, ground_force_input::GroundForceInput},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector as V},
    static_ground_pool::StaticGroundPool,
};
use forces::{Frame, TranslationStep};
use orient::Rows;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) struct GrindStateError(pub(crate) String);
impl std::fmt::Display for GrindStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "grind state: {}", self.0)
    }
}
pub(crate) fn error(e: impl std::fmt::Debug) -> GrindStateError {
    GrindStateError(format!("{e:?}"))
}
fn need<T>(value: Option<T>, field: &'static str) -> Result<T, GrindStateError> {
    value.ok_or_else(|| GrindStateError(format!("missing {field}")))
}
/// A computed vector whose native W lane is not claimed.
fn partial(v: V4) -> V {
    V {
        xyz: [v[0], v[1], v[2]],
        w: None,
    }
}
fn matrix(rows: &Rows) -> SkeletonRootMatrix {
    SkeletonRootMatrix::from_columns(rows.map(partial))
}

/// The Boardslide tail (+d0 impulse, +e0 apply, +e1 preparing).
#[derive(Clone, Copy)]
struct Boardslide {
    wipeout_impulse: Option<V4>,
    apply_wipeout_impulse: bool,
    preparing_to_jump: bool,
}
/// `PhysState_Grind_Common` +20..+c0, retained between frames.
#[derive(Clone, Copy)]
struct Fields {
    dir: V4,
    normal: V4,
    skew: V4,
    leaving: bool,
    substate: i32,
    precontact: Rows,
    already_jumped: bool,
    jumped_this_frame: bool,
    frames_in_grind: i32,
    inv_exit_frames: i32,
    crouch: f32,
    jump_vel: V4,
    boardslide: Option<Boardslide>,
}
struct Tail<'a> {
    impulse: &'a mut Option<V4>,
    apply: &'a mut Option<bool>,
    preparing: &'a mut Option<bool>,
}

fn with_payload<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &Handle,
    skeleton: &Handle,
    kind: GrindKind,
    operation: impl FnOnce(&mut GrindCommon, Option<Tail<'_>>) -> Result<T, GrindStateError>,
) -> Result<T, GrindStateError> {
    let mut states = owner.remaining_states().lock().map_err(error)?;
    let payload = states
        .states
        .get_mut(&StateKind::grind(kind))
        .filter(|state| state.published)
        .and_then(|state| state.value.as_mut())
        .ok_or_else(|| GrindStateError("retained grind state unavailable".into()))?;
    let (common, tail) = match (kind, payload) {
        (GrindKind::Boardslide, StatePayload::GrindBoardslide(state)) => {
            let state = &mut **state;
            (
                &mut state.common,
                Some(Tail {
                    impulse: &mut state.wipeout_impulse,
                    apply: &mut state.apply_wipeout_impulse,
                    preparing: &mut state.preparing_to_jump,
                }),
            )
        }
        (GrindKind::FiftyFifty, StatePayload::Grind5050(state))
        | (GrindKind::Tipslide, StatePayload::GrindTipslide(state))
        | (GrindKind::FiveO, StatePayload::Grind5O(state))
        | (GrindKind::Backslash, StatePayload::GrindBackslash(state)) => (&mut **state, None),
        _ => return Err(GrindStateError("foreign grind payload".into())),
    };
    let binding = &common.base.physical;
    if !binding.board.same_as(board)
        || !binding.skeleton.same_as(skeleton)
        || !binding
            .owner
            .upgrade()
            .is_some_and(|bound| Arc::ptr_eq(&bound, owner))
    {
        return Err(GrindStateError("foreign grind binding".into()));
    }
    operation(common, tail)
}

fn load(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &Handle,
    skeleton: &Handle,
    kind: GrindKind,
) -> Result<Fields, GrindStateError> {
    with_payload(owner, board, skeleton, kind, |s, tail| {
        need(
            s.grind_jumper.as_ref().and_then(Option::as_ref),
            "grind state GrindJumper link",
        )?;
        Ok(Fields {
            dir: need(s.grind_dir, "grind dir")?,
            normal: need(s.grind_normal, "grind normal")?,
            skew: need(s.grind_skew, "grind skew")?,
            leaving: need(s.leaving_grind, "leaving grind")?,
            substate: need(s.grind_substate, "grind substate")?,
            precontact: need(s.precontact_target_orientation, "precontact orientation")?,
            already_jumped: need(s.already_jumped, "already jumped")?,
            jumped_this_frame: need(s.jumped_this_frame, "jumped this frame")?,
            frames_in_grind: need(s.frames_in_grind, "frames in grind")?,
            inv_exit_frames: need(s.inv_exit_frames, "involuntary exit frames")?,
            crouch: need(s.crouch, "grind crouch")?,
            jump_vel: need(s.jump_vel, "grind jump velocity")?,
            boardslide: match tail {
                Some(tail) => Some(Boardslide {
                    wipeout_impulse: *tail.impulse,
                    apply_wipeout_impulse: need(*tail.apply, "boardslide wipeout impulse flag")?,
                    preparing_to_jump: need(*tail.preparing, "boardslide preparing to jump")?,
                }),
                None => None,
            },
        })
    })
}

fn store(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &Handle,
    skeleton: &Handle,
    kind: GrindKind,
    fields: &Fields,
) -> Result<(), GrindStateError> {
    with_payload(owner, board, skeleton, kind, |s, tail| {
        s.grind_dir = Some(fields.dir);
        s.grind_normal = Some(fields.normal);
        s.grind_skew = Some(fields.skew);
        s.leaving_grind = Some(fields.leaving);
        s.grind_substate = Some(fields.substate);
        s.precontact_target_orientation = Some(fields.precontact);
        s.already_jumped = Some(fields.already_jumped);
        s.jumped_this_frame = Some(fields.jumped_this_frame);
        s.frames_in_grind = Some(fields.frames_in_grind);
        s.inv_exit_frames = Some(fields.inv_exit_frames);
        s.crouch = Some(fields.crouch);
        s.jump_vel = Some(fields.jump_vel);
        if let (Some(tail), Some(b)) = (tail, fields.boardslide) {
            *tail.impulse = b.wipeout_impulse;
            *tail.apply = Some(b.apply_wipeout_impulse);
            *tail.preparing = Some(b.preparing_to_jump);
        }
        Ok(())
    })
}

/// What the grind `FillPhysOut` publishes, read under the state's lock.
pub(crate) struct GrindFill {
    pub(crate) kind: GrindKind,
    pub(crate) crouch: f32,
    pub(crate) leaving: bool,
    pub(crate) dir: V4,
    pub(crate) normal: V4,
    pub(crate) skew: V4,
    pub(crate) substate: i32,
    pub(crate) jumped_this_frame: bool,
    pub(crate) jump_vel: V4,
    pub(crate) wipeout_impulse: Option<V4>,
}
pub(crate) fn grind_fill(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &Handle,
    skeleton: &Handle,
    kind: GrindKind,
) -> Result<GrindFill, GrindStateError> {
    let fields = load(owner, board, skeleton, kind)?;
    let wipeout_impulse = match fields.boardslide {
        Some(b) if b.apply_wipeout_impulse => Some(need(b.wipeout_impulse, "boardslide impulse")?),
        _ => None,
    };
    Ok(GrindFill {
        kind,
        crouch: fields.crouch,
        leaving: fields.leaving,
        dir: fields.dir,
        normal: fields.normal,
        skew: fields.skew,
        substate: fields.substate,
        jumped_this_frame: fields.jumped_this_frame,
        jump_vel: fields.jump_vel,
        wipeout_impulse,
    })
}

fn processed_of(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<Arc<Mutex<SkeletonProcessedContact>>, GrindStateError> {
    Ok(Arc::clone(
        owner.reckoning().lock().map_err(error)?.processed(),
    ))
}

impl PhysicalAssemblyWorld {
    /// `SkateboardBody::SetPartAngularDrag` 0x82b24aa0 on one board Part.
    fn set_grind_part_drag(
        &mut self,
        board: &Handle,
        part: usize,
        drag: f32,
    ) -> Result<(), GrindStateError> {
        let body = self
            .board(board)
            .map_err(error)?
            .try_part_contact_spy_observation(&self.backend, part)
            .map_err(error)?;
        self.backend
            .try_set_part_angular_drag(body, drag * f32::from_bits(0x426f_ffff))
            .map_err(error)
    }
    /// `ClearWheelSpin` 0x82d85228.
    fn clear_grind_wheel_spin(&mut self, board: &Handle) -> Result<(), GrindStateError> {
        for part in 0..4 {
            self.set_grind_part_drag(board, part, 0.0)?;
        }
        Ok(())
    }
    /// `ManageWheelSpin` 0x82d85188: `IsPartColliding` (CollisionInfo
    /// `inContact`) selects `kAngularWheelDamping`, else zero.
    fn manage_grind_wheel_spin(
        &mut self,
        board: &Handle,
        refs: &InputCvarReferences,
    ) -> Result<(), GrindStateError> {
        for part in 0..4 {
            let contacts = self
                .try_with_board_force_runtime(board, |runtime| Arc::clone(&runtime.contact_surface))
                .map_err(error)?;
            let colliding = need(
                contacts.lock().map_err(error)?.collision_state.in_contact[part],
                "board Part inContact",
            )?;
            let drag = if colliding {
                GrindCvars(refs).f(C::kAngularWheelDamping).map_err(error)?
            } else {
                0.0
            };
            self.set_grind_part_drag(board, part, drag)?;
        }
        Ok(())
    }

    /// `PhysState_Grind_Common::Enter` (and the Boardslide tail clears).
    pub(crate) fn try_enter_pair_grind(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        kind: GrindKind,
        refs: &InputCvarReferences,
    ) -> Result<(), GrindStateError> {
        with_payload(owner, board, skeleton, kind, |_, _| Ok(()))?;
        self.try_disable_board_animation(board).map_err(error)?;
        need(self.skaters.as_mut(), "skater pool")?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                // Skeleton+3e41 mUsePhysicsTruckTilt, then DisableFeetCollision.
                normal.enable_physics_truck_tilt();
                normal.disable_feet_collisions();
            })
            .map_err(error)?;
        let damping = GrindCvars(refs).f(C::kAngularDamping).map_err(error)?;
        self.try_set_board_angular_damping(board, Some(damping))
            .map_err(error)?;
        let deck = {
            let processed = processed_of(owner)?;
            let p = processed.lock().map_err(error)?;
            need(p.deck_transform(), "Processed deck transform")?
                .columns()
                .map(lanes)
        };
        with_payload(owner, board, skeleton, kind, |s, tail| {
            s.grind_substate = Some(1);
            s.leaving_grind = Some(false);
            s.already_jumped = Some(false);
            s.jumped_this_frame = Some(false);
            s.precontact_target_orientation = Some(deck);
            s.frames_in_grind = Some(0);
            s.crouch = Some(0.0);
            s.inv_exit_frames = Some(0);
            s.jump_vel = Some(ZERO);
            if let Some(tail) = tail {
                *tail.apply = Some(false);
                *tail.preparing = Some(false);
            }
            Ok(())
        })
    }

    /// `PhysState_Grind_Common::Exit`.
    pub(crate) fn try_exit_pair_grind(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        kind: GrindKind,
    ) -> Result<(), GrindStateError> {
        with_payload(owner, board, skeleton, kind, |s, _| {
            s.leaving_grind = Some(false);
            Ok(())
        })?;
        need(self.skaters.as_mut(), "skater pool")?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.try_enable_feet_collisions()
            })
            .map_err(error)?
            .map_err(error)?;
        self.try_set_board_angular_damping(board, None)
            .map_err(error)?;
        self.try_disable_board_animation(board).map_err(error)?;
        self.clear_grind_wheel_spin(board)
    }

    /// `PhysState_Grind_Common::Update`.
    #[expect(clippy::too_many_arguments, reason = "actual pair and frame services")]
    pub(crate) fn try_update_pair_grind(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        kind: GrindKind,
        actor: &ActorConstruction,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<PhysicalPlayerReckoningPhase, GrindStateError> {
        let fields = load(owner, board, skeleton, kind)?;
        let processed = processed_of(owner)?;
        let (view, inv, deck) = {
            let p = processed.lock().map_err(error)?;
            (
                GrindView::try_read(&p).map_err(error)?,
                need(
                    p.grind_investigation().cloned(),
                    "Processed grind investigation",
                )?,
                *need(p.deck_transform(), "Processed deck transform")?.columns(),
            )
        };
        let overlay = self.debug_overlay.clone();
        let mut update = Update {
            world: self,
            owner,
            board,
            skeleton,
            actor,
            ground,
            refs,
            numeric,
            overlay,
            attributes: GrindAttributes::bind(&owner.inputs().vault),
            processed,
            kind,
            inv,
            view,
            deck,
            s: fields,
        };
        update.s.jumped_this_frame = false;
        let phase = update.update_reckoning()?;
        update.manage_substate()?;
        update.update_substate()?;
        update.after_substate()?;
        let fields = update.s;
        store(owner, board, skeleton, kind, &fields)?;
        Ok(phase)
    }
}

struct Update<'a> {
    world: &'a mut PhysicalAssemblyWorld,
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    board: &'a Handle,
    skeleton: &'a Handle,
    actor: &'a ActorConstruction,
    ground: &'a StaticGroundPool,
    refs: &'a InputCvarReferences,
    numeric: &'a mut HostGroundPreparationNumeric,
    overlay: DebugOverlaySlot,
    attributes: GrindAttributes,
    processed: Arc<Mutex<SkeletonProcessedContact>>,
    kind: GrindKind,
    inv: Investigation,
    view: GrindView,
    /// Processed +40..+70, W knowledge retained.
    deck: [V; 4],
    s: Fields,
}

impl Update<'_> {
    fn f(&self, cvar: C) -> Result<f32, GrindStateError> {
        GrindCvars(self.refs).f(cvar).map_err(error)
    }
    fn frame(&self) -> Frame {
        Frame {
            dir: self.s.dir,
            normal: self.s.normal,
            skew: self.s.skew,
        }
    }
    /// `Skateboard::ApplyWorldSpaceForceToDeck(force, Processed+70)`: the arm
    /// from the live deck Part's position, read at each call.
    fn deck_force(&mut self, force: V4) -> Result<(), GrindStateError> {
        let world = &mut *self.world;
        let session = need(world.skaters.as_ref(), "skater pool")?
            .session(self.board)
            .map_err(error)?;
        let part = session
            .try_live_part_transform_typed(&world.backend, 6)
            .map_err(error)?;
        let observation = session
            .try_part_contact_spy_observation(&world.backend, 6)
            .map_err(error)?;
        world
            .backend
            .try_apply_skateboard_world_force(
                observation,
                partial(force),
                self.deck[3],
                part.columns()[3],
            )
            .map_err(error)
    }
    fn maybe_force(&mut self, force: Option<V4>) -> Result<(), GrindStateError> {
        match force {
            Some(force) => self.deck_force(force),
            None => Ok(()),
        }
    }
    fn wipeout_request(&self, index: usize) -> Result<(), GrindStateError> {
        self.owner
            .wipeout()
            .lock()
            .map_err(error)?
            .try_wipeout_request_value(index, 0.0)
            .map_err(error)
    }

    /// `UpdateGrindReckoning`.
    fn update_reckoning(&mut self) -> Result<PhysicalPlayerReckoningPhase, GrindStateError> {
        self.s.dir = self.inv.grind_dir;
        self.s.normal = self.inv.grind_normal;
        self.s.skew = cross(self.s.dir, self.s.normal);
        let smoothing = if self.inv.exit_tilt_angle_deg > 0.0 {
            self.f(C::kGrindReckoningSmoothnessOnExit)?
        } else {
            self.f(C::kGrindReckoningSmoothnessInGrind)?
        };
        self.owner
            .reckoning()
            .lock()
            .map_err(error)?
            .try_update_grind_live(
                &self.processed,
                root(self.inv.target_up_vector),
                self.deck[2],
                smoothing,
                false,
                &mut ReckoningCvarAdapter(self.refs),
                &mut self.numeric.reckoning,
            )
            .map_err(error)
    }

    /// `ManageSubstate` with `ShouldExit` 0x82d85d90.
    fn manage_substate(&mut self) -> Result<(), GrindStateError> {
        match self.s.substate {
            1 => {
                if self.should_exit()? {
                    self.s.substate = 2;
                }
                Ok(())
            }
            2 => Ok(()),
            other => Err(GrindStateError(format!(
                "ManageSubstate assertion: substate {other}"
            ))),
        }
    }
    fn should_exit(&self) -> Result<bool, GrindStateError> {
        if self.inv.balance_requests_auto_exit {
            return Ok(true);
        }
        // UserTriggeredExit: 8cc bit31; Boardslide 0x82d85fd8 on a ledge only.
        let user = self.view.flags_8cc >> 31 != 0
            && (self.kind != GrindKind::Boardslide || self.inv.geometry.geometry_type == LEDGE);
        if user {
            return Ok(true);
        }
        let minimum = GrindCvars(self.refs).i(C::kMinGrindFrames).map_err(error)?;
        if !(self.s.frames_in_grind > minimum) {
            return Ok(false);
        }
        Ok(self.view.deck_speed < self.f(C::kGrindCommonExitSpeed)?)
    }

    /// `GetVerticalJump` of each type.
    fn vertical_jump(&self) -> Result<f32, GrindStateError> {
        self.f(match self.kind {
            GrindKind::Boardslide => C::kBoardslideJumpVert,
            GrindKind::Tipslide => C::kTipslideJumpVert,
            _ => C::kGrindCommonJumpVert,
        })
    }
    /// `GetGeometrySideJumpScalar` of each type.
    fn side_jump_scalar(&self) -> Result<f32, GrindStateError> {
        match self.kind {
            GrindKind::Boardslide => {
                if self.inv.geometry.geometry_type == LEDGE {
                    self.f(C::kBoardslideSideJumpScalar)
                } else {
                    Ok(0.0)
                }
            }
            GrindKind::FiftyFifty => self.f(C::k5050SideJumpScalar),
            GrindKind::Tipslide => self.f(C::kTipslideSideJumpScalar),
            GrindKind::FiveO => self.f(C::k5_OSideJumpScalar),
            GrindKind::Backslash => self.f(C::kBackslashSideJumpScalar),
        }
    }

    /// `ApplyJump` 0x82d85d00: `GrindJumper::LaunchSkateboard`, ending with its
    /// `Skateboard::SetVelocity`.
    fn apply_jump(&mut self) -> Result<V4, GrindStateError> {
        let vertical = self.vertical_jump()?;
        let side = self.side_jump_scalar()?;
        launch(
            self.world,
            self.owner,
            self.board,
            &self.processed,
            &self.overlay,
            &self.view,
            self.refs,
            self.numeric,
            side,
            vertical,
        )
    }

    /// Toolkit_CalcCollisionForce with its new velocity; a hit queues force 15.
    fn collision_force(&mut self) -> Result<Option<V>, GrindStateError> {
        collision_force(
            self.world,
            self.owner,
            self.board,
            &self.processed,
            &mut self.numeric.reckoning,
            true,
        )
    }

    /// `UpdateSubstate`.
    fn update_substate(&mut self) -> Result<(), GrindStateError> {
        let mut velocity: Option<V> = None;
        let mut animate = false;
        if let Some(new_velocity) = self.collision_force()? {
            velocity = Some(new_velocity);
        } else if self.view.bit_8c4(0x0040_0000).map_err(error)? {
            let jump = self.apply_jump()?;
            self.s.already_jumped = true;
            self.s.jumped_this_frame = true;
            self.s.jump_vel = jump;
            velocity = Some(root(jump));
            animate = true;
        } else if !self.s.already_jumped {
            if self.inv.should_engage {
                let engage = root(self.inv.engagement_velocity);
                self.world
                    .try_set_board_velocity(self.board, &engage)
                    .map_err(error)?;
                velocity = Some(engage);
            } else {
                match self.s.substate {
                    1 => self.update_contact()?,
                    2 => self.update_involuntary_exit()?,
                    other => {
                        return Err(GrindStateError(format!(
                            "UpdateSubstate assertion: substate {other}"
                        )));
                    }
                }
            }
        }
        if let Some(velocity) = velocity {
            // Skeleton+3ce0 predicted board position (fused vel*dt + deck T),
            // then +3dec valid.
            let dt = self.view.dt;
            let t = self.deck[3];
            let position = V {
                xyz: std::array::from_fn(|i| velocity.xyz[i].mul_add(dt, t.xyz[i])),
                w: velocity.w.zip(t.w).map(|(v, t)| v.mul_add(dt, t)),
            };
            need(self.world.skaters.as_mut(), "skater pool")?
                .try_with_jump_pair(
                    &self.world.backend,
                    self.board,
                    self.skeleton,
                    |normal, _| {
                        normal
                            .root_transforms_mut()
                            .publish_board_prediction(position);
                    },
                )
                .map_err(error)?;
        }
        skeleton_skateboard(
            self.world,
            self.board,
            self.skeleton,
            self.ground,
            self.refs,
            &self.processed,
            self.numeric,
            animate,
        )
    }

    /// The per-type `Update_Contact`.
    fn update_contact(&mut self) -> Result<(), GrindStateError> {
        match self.kind {
            GrindKind::Boardslide => {
                self.orient_boardslide()?;
                self.boardslide_translation()?;
                self.pin()?;
                self.friction(
                    C::kBoardslideFriction_ThinRail,
                    C::kBoardslideFriction_FatRail,
                    C::kBoardslideFriction_Ledge,
                )?;
                self.coping()?;
                self.assist_exit_lean()?;
                self.world.manage_grind_wheel_spin(self.board, self.refs)
            }
            GrindKind::FiftyFifty => {
                self.friction(
                    C::k5050Friction_ThinRail,
                    C::k5050Friction_FatRail,
                    C::k5050Friction_Ledge,
                )?;
                self.alignment(C::k5050CenterForce, None, C::k5050TruckDown)?;
                self.coping()?;
                self.pin()?;
                self.assist_exit_lean()?;
                self.orient_5050()
            }
            GrindKind::Tipslide => {
                self.orient_tilted(
                    C::kTipslideTiltAngleDeg,
                    C::kTipslideMatBlend,
                    C::kTipslideNoise,
                )?;
                let force = {
                    let frame = self.frame();
                    let mut num = GrindNum {
                        n: &mut self.numeric.reckoning,
                        s: &mut self.numeric.sin_cos,
                    };
                    forces::stability_nudge(
                        &self.view,
                        &self.inv,
                        &frame,
                        &GrindCvars(self.refs),
                        &mut num,
                    )
                    .map_err(error)?
                };
                self.deck_force(force)?;
                self.friction(
                    C::kTipslideFriction_ThinRail,
                    C::kTipslideFriction_FatRail,
                    C::kTipslideFriction_Ledge,
                )?;
                self.coping()?;
                self.pin()?;
                self.assist_exit_lean()?;
                self.world.manage_grind_wheel_spin(self.board, self.refs)
            }
            GrindKind::FiveO => {
                self.orient_5_o()?;
                self.alignment(
                    C::k5_OCenterForce,
                    Some(C::k5_OCenterToTruck),
                    C::k5_OTruckDown,
                )?;
                self.pin()?;
                self.friction(
                    C::k5_OFriction_ThinRail,
                    C::k5_OFriction_FatRail,
                    C::k5_OFriction_Ledge,
                )?;
                self.coping()?;
                self.assist_exit_lean()
            }
            GrindKind::Backslash => {
                self.orient_tilted(
                    C::kBackslashTiltAngleDeg,
                    C::kBackslashMatBlend,
                    C::kBackslashNoise,
                )?;
                self.alignment(
                    C::kBackslashCenterForce,
                    Some(C::kBackslashDeckToGrindPoint),
                    C::kBackslashGrindPointDown,
                )?;
                self.friction(
                    C::kBackslashFriction_ThinRail,
                    C::kBackslashFriction_FatRail,
                    C::kBackslashFriction_Ledge,
                )?;
                self.coping()?;
                self.pin()?;
                self.assist_exit_lean()?;
                self.world.manage_grind_wheel_spin(self.board, self.refs)
            }
        }
    }

    /// The per-type `Update_InvoluntaryExit`.
    fn update_involuntary_exit(&mut self) -> Result<(), GrindStateError> {
        let geometry = self.inv.geometry.geometry_type;
        #[expect(clippy::cast_sign_loss, reason = "native unsigned cmplwi")]
        let rail = (geometry as u32) < 2;
        match self.kind {
            GrindKind::Boardslide => {
                if geometry == LEDGE {
                    let lift = self.f(C::kBoardslideExitLift)?;
                    let speed = self.f(C::kBoardslideExitNudgeSpeed)?;
                    let force = self.f(C::kBoardslideExitNudgeForce)?;
                    self.exit_nudge(force, speed, lift, false, false)?;
                } else {
                    need(self.s.boardslide.as_mut(), "Boardslide tail")?.apply_wipeout_impulse =
                        true;
                    let speed = self.f(C::kBoardslideWipeoutDeckSpeed)?;
                    need(self.s.boardslide.as_mut(), "Boardslide tail")?.wipeout_impulse =
                        Some(scale(self.inv.grind_dir, speed));
                    // BoardslideStopOnRail.
                    self.wipeout_request(10)?;
                }
                self.world.clear_grind_wheel_spin(self.board)
            }
            GrindKind::FiftyFifty => {
                let (lift, force) = if rail {
                    self.f(C::k5050ExitLift)?;
                    self.f(C::k5050ExitNudgeForce)?;
                    (
                        self.f(C::k5050ExitLift_Rail)?,
                        self.f(C::k5050ExitNudgeForce_Rail)?,
                    )
                } else {
                    (self.f(C::k5050ExitLift)?, self.f(C::k5050ExitNudgeForce)?)
                };
                let speed = self.f(C::k5050ExitNudgeSpeed)?;
                self.exit_nudge(force, speed, lift, true, false)
            }
            GrindKind::Tipslide => {
                let lift = self.f(C::kTipslideExitLift)?;
                let speed = self.f(C::kTipslideExitNudgeSpeed)?;
                let force = self.f(C::kTipslideExitNudgeForce)?;
                self.exit_nudge(force, speed, lift, false, true)?;
                self.world.clear_grind_wheel_spin(self.board)
            }
            GrindKind::FiveO => {
                let (lift, force) = if rail {
                    self.f(C::k5_OExitLift)?;
                    self.f(C::k5_OExitNudgeForce)?;
                    (
                        self.f(C::k5_OExitLift_Rail)?,
                        self.f(C::k5_OExitNudgeForce_Rail)?,
                    )
                } else {
                    (self.f(C::k5_OExitLift)?, self.f(C::k5_OExitNudgeForce)?)
                };
                let mut deck_side = false;
                if geometry == FAT_RAIL {
                    let sideness = {
                        let mut num = GrindNum {
                            n: &mut self.numeric.reckoning,
                            s: &mut self.numeric.sin_cos,
                        };
                        abs1(num.dot(self.view.deck_z(), self.s.skew).map_err(error)?)
                    };
                    deck_side = sideness > self.f(C::k5_OMinDecksideSideness)?;
                }
                let speed = self.f(C::k5_OExitNudgeSpeed)?;
                self.exit_nudge(force, speed, lift, true, deck_side)
            }
            GrindKind::Backslash => {
                let fat = geometry == FAT_RAIL;
                let speed = self.f(C::kBackslashExitNudgeSpeed)?;
                let lift = self.f(C::kBackslashExitLift)?;
                let force = self.f(C::kBackslashExitNudgeForce)?;
                self.exit_nudge(force, speed, lift, fat, fat)?;
                self.world.clear_grind_wheel_spin(self.board)
            }
        }
    }

    fn friction(&mut self, thin: C, fat: C, ledge: C) -> Result<(), GrindStateError> {
        let ledge = self.f(ledge)?;
        let fat = self.f(fat)?;
        let thin = self.f(thin)?;
        let force = {
            let frame = self.frame();
            let mut num = GrindNum {
                n: &mut self.numeric.reckoning,
                s: &mut self.numeric.sin_cos,
            };
            forces::friction(
                &self.view,
                &self.inv,
                &frame,
                thin,
                fat,
                ledge,
                &GrindCvars(self.refs),
                &mut num,
            )
            .map_err(error)?
        };
        self.maybe_force(force)
    }
    fn coping(&mut self) -> Result<(), GrindStateError> {
        let force = {
            let mut num = GrindNum {
                n: &mut self.numeric.reckoning,
                s: &mut self.numeric.sin_cos,
            };
            forces::coping(&self.view, &self.inv, &GrindCvars(self.refs), &mut num)
                .map_err(error)?
        };
        self.maybe_force(force)
    }
    fn pin(&mut self) -> Result<(), GrindStateError> {
        let force = forces::pin(&self.inv, &self.frame(), &GrindCvars(self.refs)).map_err(error)?;
        self.maybe_force(force)
    }
    fn assist_exit_lean(&mut self) -> Result<(), GrindStateError> {
        let force = forces::assist_exit_lean(&self.inv, &self.attributes, &GrindCvars(self.refs))
            .map_err(error)?;
        self.maybe_force(force)
    }
    fn truck_compensation(&mut self) -> Result<(), GrindStateError> {
        let force = forces::truck_compensation(&self.inv, &self.frame(), &GrindCvars(self.refs))
            .map_err(error)?;
        self.deck_force(force)
    }
    fn alignment(
        &mut self,
        strength: C,
        dir_dist: Option<C>,
        down_dist: C,
    ) -> Result<(), GrindStateError> {
        let down = self.f(down_dist)?;
        let dir = match dir_dist {
            Some(cvar) => self.f(cvar)?,
            None => 0.0,
        };
        let strength = self.f(strength)?;
        let force = {
            let frame = self.frame();
            let mut num = GrindNum {
                n: &mut self.numeric.reckoning,
                s: &mut self.numeric.sin_cos,
            };
            forces::alignment(
                &self.view,
                &self.inv,
                &frame,
                strength,
                dir,
                down,
                &self.attributes,
                &GrindCvars(self.refs),
                &mut num,
            )
            .map_err(error)?
        };
        self.maybe_force(force)
    }
    fn exit_nudge(
        &mut self,
        strength: f32,
        speed: f32,
        lift: f32,
        add_vertical: bool,
        use_deck_side: bool,
    ) -> Result<(), GrindStateError> {
        let force = {
            let frame = self.frame();
            let mut num = GrindNum {
                n: &mut self.numeric.reckoning,
                s: &mut self.numeric.sin_cos,
            };
            forces::exit_nudge(
                &self.view,
                &self.inv,
                &frame,
                strength,
                speed,
                lift,
                add_vertical,
                use_deck_side,
                &mut num,
            )
            .map_err(error)?
        };
        self.maybe_force(force)
    }
    fn boardslide_translation(&mut self) -> Result<(), GrindStateError> {
        if self.view.bit_8c4(0x0020_0000).map_err(error)? {
            need(self.s.boardslide.as_mut(), "Boardslide tail")?.preparing_to_jump = true;
        }
        let preparing = need(self.s.boardslide, "Boardslide tail")?.preparing_to_jump;
        let steps = {
            let frame = self.frame();
            let mut num = GrindNum {
                n: &mut self.numeric.reckoning,
                s: &mut self.numeric.sin_cos,
            };
            forces::boardslide_translation(
                &self.view,
                &self.inv,
                &frame,
                preparing,
                &GrindCvars(self.refs),
                &mut num,
            )
            .map_err(error)?
        };
        for step in steps {
            match step {
                TranslationStep::Force(force) => self.deck_force(force)?,
                TranslationStep::TranslationControl => {
                    let strength = self.f(C::kBoardslideTranslationForce)?;
                    let force = forces::translation_control(&self.view, &self.frame(), strength);
                    self.deck_force(force)?;
                }
                TranslationStep::Overlay => {
                    self.overlay.require().map_err(error)?;
                }
            }
        }
        Ok(())
    }

    /// `ReorientSkateboard(target, fraction, noise)`.
    fn reorient(
        &mut self,
        target: &Rows,
        fraction: f32,
        noise: f32,
    ) -> Result<(), GrindStateError> {
        let (to, from) = {
            let mut num = GrindNum {
                n: &mut self.numeric.reckoning,
                s: &mut self.numeric.sin_cos,
            };
            let to = orient::orthonormalize(target, &mut num).map_err(error)?;
            let from = orient::orthonormalize(&self.view.deck, &mut num).map_err(error)?;
            (to, from)
        };
        let (mut result, _) = crate::skeleton_affine_interpolation::try_slerp_typed(
            &matrix(&from),
            &matrix(&to),
            fraction,
            &mut self.numeric.reckoning,
            &mut self.numeric.sin_cos,
        )
        .map_err(error)?;
        // noise > 1 is a debug-build FailAssert only.
        if noise > 0.0 {
            let draws = [
                self.actor.try_motion_random_int().map_err(error)?,
                self.actor.try_motion_random_int().map_err(error)?,
                self.actor.try_motion_random_int().map_err(error)?,
            ];
            let rows = result.columns().map(lanes);
            let mut num = GrindNum {
                n: &mut self.numeric.reckoning,
                s: &mut self.numeric.sin_cos,
            };
            let noisy = orient::noise(
                &rows,
                draws,
                self.view.deck_speed,
                noise,
                &GrindCvars(self.refs),
                &mut num,
            )
            .map_err(error)?;
            result = matrix(&noisy);
        }
        self.world
            .try_set_controller_board_target_orientation(self.board, &result)
            .map_err(error)?;
        self.world
            .try_enable_board_angular_drive_only(
                self.board,
                1.0,
                self.refs,
                &mut self.numeric.reckoning,
            )
            .map_err(error)
    }
    fn orientation_control(&mut self, strength: f32) -> Result<Rows, GrindStateError> {
        let flipped = self.view.bit_8c4(0x0010_0000).map_err(error)?;
        let mut num = GrindNum {
            n: &mut self.numeric.reckoning,
            s: &mut self.numeric.sin_cos,
        };
        orient::orientation_control(
            &self.view.deck,
            self.s.normal,
            self.inv.truck_control_lr,
            strength,
            self.inv.truck_control_ud,
            strength,
            flipped,
            &mut num,
        )
        .map_err(error)
    }
    /// 5050 `OrientBoard`.
    fn orient_5050(&mut self) -> Result<(), GrindStateError> {
        let epsilon = self.f(C::k5050ControlEpsilon)?;
        let target = if abs1(self.inv.truck_control_ud) > epsilon {
            let strength = self.f(C::k5050ControlStrength)?;
            self.orientation_control(strength)?
        } else {
            let frame = self.frame();
            let mut num = GrindNum {
                n: &mut self.numeric.reckoning,
                s: &mut self.numeric.sin_cos,
            };
            orient::target_5050(&self.view, &frame, &mut num).map_err(error)?
        };
        let blend = self.f(C::k5050ControlMatBlend)?;
        self.reorient(&target, blend, 0.0)?;
        self.truck_compensation()
    }
    /// 5_O `OrientBoard`.
    fn orient_5_o(&mut self) -> Result<(), GrindStateError> {
        let strength = self.f(C::k5_OControlStrength)?;
        let target = self.orientation_control(strength)?;
        let noise = self.f(C::k5_ONoise)?;
        let blend = self.f(C::k5_OMatBlend)?;
        self.reorient(&target, blend, noise)?;
        self.truck_compensation()
    }
    /// Tipslide / Backslash `OrientBoard`.
    fn orient_tilted(&mut self, tilt: C, blend: C, noise: C) -> Result<(), GrindStateError> {
        let tilt = self.f(tilt)?;
        let target = {
            let frame = self.frame();
            let mut num = GrindNum {
                n: &mut self.numeric.reckoning,
                s: &mut self.numeric.sin_cos,
            };
            orient::target_tilted(&self.view, &self.inv, &frame, tilt, &mut num).map_err(error)?
        };
        let noise = self.f(noise)?;
        let blend = self.f(blend)?;
        self.reorient(&target, blend, noise)
    }
    /// Boardslide `OrientBoardToNormal`.
    fn orient_boardslide(&mut self) -> Result<(), GrindStateError> {
        let target = {
            let frame = self.frame();
            let mut num = GrindNum {
                n: &mut self.numeric.reckoning,
                s: &mut self.numeric.sin_cos,
            };
            orient::target_boardslide(&self.view, &frame, &mut num).map_err(error)?
        };
        let noise = self.f(C::kBoardslideNoise)?;
        let blend = self.f(C::kBoardslideMatBlend)?;
        self.reorient(&target, blend, noise)
    }

    /// The Update tail after `UpdateSubstate`.
    fn after_substate(&mut self) -> Result<(), GrindStateError> {
        self.s.inv_exit_frames = if self.s.substate == 2 {
            self.s.inv_exit_frames.wrapping_add(1)
        } else {
            0
        };
        let maximum = GrindCvars(self.refs)
            .i(C::kMaxInvExitFrames)
            .map_err(error)?;
        if self.s.inv_exit_frames > maximum {
            // FailureToInvExit.
            self.wipeout_request(17)?;
        }
        let (deck_z, velocity) = {
            let p = self.processed.lock().map_err(error)?;
            (
                lanes(need(p.deck_transform(), "Processed deck transform")?.columns()[2]),
                need(p.prior_output().velocity(), "Processed velocity")?.value,
            )
        };
        let facing = {
            let mut num = GrindNum {
                n: &mut self.numeric.reckoning,
                s: &mut self.numeric.sin_cos,
            };
            abs1(num.dot(deck_z, self.s.normal).map_err(error)?)
        };
        self.s.crouch = self.f(C::kGrindCrouchScalar)? * facing;
        self.world
            .try_update_constructed_board_steering(
                self.board,
                0.0,
                &mut SteeringInput(&self.processed),
            )
            .map_err(error)?;
        let lead = self.attributes.scalar(HEAD_TRACKING_LEAD).map_err(error)?;
        let location = self.inv.primitive_location;
        let target = V {
            xyz: std::array::from_fn(|i| velocity.xyz[i].mul_add(lead, location[i])),
            w: velocity.w.map(|w| w.mul_add(lead, location[3])),
        };
        need(self.world.skaters.as_mut(), "skater pool")?
            .try_set_skeleton_head_target(&self.world.backend, self.skeleton, self.owner, target)
            .map_err(error)?;
        self.s.frames_in_grind = self.s.frames_in_grind.wrapping_add(1);
        Ok(())
    }
}

/// `GrindJumper::LaunchSkateboard` through the manager's jumper, its suffix
/// writes, then its closing `Skateboard::SetVelocity`.
#[expect(
    clippy::too_many_arguments,
    reason = "actual owners of one native call"
)]
fn launch(
    world: &mut PhysicalAssemblyWorld,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &Handle,
    processed: &Arc<Mutex<SkeletonProcessedContact>>,
    overlay: &DebugOverlaySlot,
    view: &GrindView,
    refs: &InputCvarReferences,
    numeric: &mut HostGroundPreparationNumeric,
    side: f32,
    vertical: f32,
) -> Result<V4, GrindStateError> {
    let mut effects = SuffixEffects::default();
    let result = {
        let mut grind = owner.grind().lock().map_err(error)?;
        let jumper = need(
            grind.as_mut().and_then(|g| g.jumper.as_mut()),
            "GrindJumper",
        )?;
        let mut num = GrindNum {
            n: &mut numeric.reckoning,
            s: &mut numeric.sin_cos,
        };
        jumper
            .try_launch_skateboard(
                side,
                vertical,
                view,
                &GrindCvars(refs),
                &mut num,
                &mut effects,
            )
            .map_err(error)?
    };
    apply_effects(owner, processed, overlay, &effects).map_err(error)?;
    world
        .try_set_board_velocity(board, &root(result))
        .map_err(error)?;
    Ok(result)
}

/// `Toolkit_CalcCollisionForce(force, torque, newVel)`; a hit queues
/// `AddSkateboardForce(15)` and returns the new velocity.
fn collision_force(
    world: &mut PhysicalAssemblyWorld,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &Handle,
    processed: &Arc<Mutex<SkeletonProcessedContact>>,
    numeric: &mut crate::reckoning_numeric::HostReckoningNumeric,
    with_velocity: bool,
) -> Result<Option<V>, GrindStateError> {
    need(world.skaters.as_mut(), "skater pool")?
        .try_with_ground_force_board(&world.backend, board, |_, runtime, _| {
            let reckoning = owner.reckoning().lock().map_err(error)?;
            let mut input = GroundForceInput {
                processed,
                reckoning: reckoning.reckoning(),
            };
            let mut force = SkateboardFootForce {
                world_space_force: V::known([0.0; 4]),
                local_space_position: V::known([0.0; 4]),
            };
            let mut torque = V {
                xyz: [f32::NAN; 3],
                w: None,
            };
            let mut velocity = V::known([0.0; 4]);
            let hit = SkateboardSlideCollisionServices {
                input: &mut input,
                attributes: &mut runtime.slide_collision_attributes,
                numeric,
            }
            .try_collision(
                &mut force,
                &mut torque,
                with_velocity.then_some(&mut velocity),
            )
            .map_err(error)?;
            if hit {
                runtime.queue.add(15, force);
            }
            Ok::<_, GrindStateError>(hit.then_some(velocity))
        })
        .map_err(error)?
}

/// `Skeleton::UpdateAnimatedSkateboard(p, false)` when `animate`, else
/// `UpdateGroundSkateboard(p)` followed by `Skateboard::UpdatePhysicsError`.
#[expect(clippy::too_many_arguments, reason = "actual pair and frame services")]
fn skeleton_skateboard(
    world: &mut PhysicalAssemblyWorld,
    board: &Handle,
    skeleton: &Handle,
    ground: &StaticGroundPool,
    refs: &InputCvarReferences,
    processed: &Arc<Mutex<SkeletonProcessedContact>>,
    numeric: &mut HostGroundPreparationNumeric,
    animate: bool,
) -> Result<(), GrindStateError> {
    use crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralContinuation as Next;
    if animate {
        let progress = world
            .try_update_pair_animated_skeleton(
                board,
                skeleton,
                ground,
                refs,
                false,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
                &mut numeric.spawn_math,
            )
            .map_err(error)?;
        if progress.next != Next::Complete {
            return Err(GrindStateError("incomplete animated Skeleton".into()));
        }
        return Ok(());
    }
    let progress = world
        .try_update_pair_ground_skateboard(
            board,
            skeleton,
            ground,
            refs,
            &mut numeric.reckoning,
            &mut numeric.sin_cos,
            &mut numeric.spawn_math,
        )
        .map_err(error)?;
    if progress.next != Next::Complete {
        return Err(GrindStateError("incomplete ground Skeleton".into()));
    }
    let backend = &world.backend;
    need(world.skaters.as_mut(), "skater pool")?
        .try_with_ground_force_board(backend, board, |session, runtime, _| {
            runtime
                .steering
                .try_update_physics_error(
                    &mut PhysicsErrorSource {
                        processed,
                        board: session,
                        backend,
                    },
                    &mut numeric.reckoning,
                )
                .map_err(error)
        })
        .map_err(error)?
}

/// `PhysState_Nonspecific`.
impl PhysicalAssemblyWorld {
    fn with_nonspecific<T>(
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        operation: impl FnOnce(
            &mut crate::physical_player_assembly::remaining_states::Nonspecific,
        ) -> Result<T, GrindStateError>,
    ) -> Result<T, GrindStateError> {
        let mut states = owner.remaining_states().lock().map_err(error)?;
        let Some(StatePayload::Nonspecific(state)) = states
            .states
            .get_mut(&StateKind::Nonspecific)
            .filter(|state| state.published)
            .and_then(|state| state.value.as_mut())
        else {
            return Err(GrindStateError("retained Nonspecific unavailable".into()));
        };
        let binding = &state.base.physical;
        if !binding.board.same_as(board)
            || !binding.skeleton.same_as(skeleton)
            || !binding
                .owner
                .upgrade()
                .is_some_and(|bound| Arc::ptr_eq(&bound, owner))
        {
            return Err(GrindStateError("foreign Nonspecific binding".into()));
        }
        operation(state)
    }

    /// Enter 0x82d86710.
    pub(crate) fn try_enter_pair_nonspecific(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
    ) -> Result<(), GrindStateError> {
        Self::with_nonspecific(owner, board, skeleton, |s| {
            s.jumped_this_frame = Some(false);
            s.jump_vel = Some(ZERO);
            Ok(())
        })?;
        need(self.skaters.as_mut(), "skater pool")?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.enable_physics_truck_tilt();
            })
            .map_err(error)?;
        owner
            .reckoning()
            .lock()
            .map_err(error)?
            .try_reset_body_spin()
            .map_err(error)?;
        self.try_disable_board_animation(board).map_err(error)
    }

    /// Update 0x82d9bfc0.
    #[expect(clippy::too_many_arguments, reason = "actual pair and frame services")]
    pub(crate) fn try_update_pair_nonspecific(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<PhysicalPlayerReckoningPhase, GrindStateError> {
        Self::with_nonspecific(owner, board, skeleton, |s| {
            s.jumped_this_frame = Some(false);
            Ok(())
        })?;
        let processed = processed_of(owner)?;
        let view = GrindView::try_read(&*processed.lock().map_err(error)?).map_err(error)?;
        let overlay = self.debug_overlay.clone();
        if view.bit_8c4(0x0040_0000).map_err(error)? {
            let vertical = GrindCvars(refs).f(C::kGrindCommonJumpVert).map_err(error)?;
            let side = GrindCvars(refs)
                .f(C::kNonspecificSideJumpScalar)
                .map_err(error)?;
            let jump = launch(
                self, owner, board, &processed, &overlay, &view, refs, numeric, side, vertical,
            )?;
            Self::with_nonspecific(owner, board, skeleton, |s| {
                s.jump_vel = Some(jump);
                s.jumped_this_frame = Some(true);
                Ok(())
            })?;
            skeleton_skateboard(
                self, board, skeleton, ground, refs, &processed, numeric, true,
            )?;
            self.try_set_board_velocity(board, &root(jump))
                .map_err(error)?;
        } else {
            let animated = self
                .skeleton(skeleton)
                .map_err(error)?
                .try_is_skateboard_animated()
                .map_err(error)?
                && view.flags_8c8 & 0x0080_0000 != 0;
            skeleton_skateboard(
                self, board, skeleton, ground, refs, &processed, numeric, animated,
            )?;
            self.nonspecific_skateboard(owner, board, refs, &processed, &overlay, numeric)?;
        }
        let (normal, heading) = {
            let p = processed.lock().map_err(error)?;
            let deck = need(p.deck_transform(), "Processed deck transform")?.columns();
            (deck[1], deck[2])
        };
        let smoothing = GrindCvars(refs)
            .f(C::kNonspecificReckoningSmoothness)
            .map_err(error)?;
        owner
            .reckoning()
            .lock()
            .map_err(error)?
            .try_update_grind_live(
                &processed,
                normal,
                heading,
                smoothing,
                false,
                &mut ReckoningCvarAdapter(refs),
                &mut numeric.reckoning,
            )
            .map_err(error)
    }

    /// `UpdateSkateboard` 0x82d91030.
    fn nonspecific_skateboard(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        refs: &InputCvarReferences,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        overlay: &DebugOverlaySlot,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<(), GrindStateError> {
        self.try_update_constructed_board_steering(board, 0.0, &mut SteeringInput(processed))
            .map_err(error)?;
        let backend = &self.backend;
        need(self.skaters.as_mut(), "skater pool")?
            .try_with_ground_force_board(backend, board, |session, runtime, _| {
                runtime
                    .steering
                    .try_update_physics_error(
                        &mut PhysicsErrorSource {
                            processed,
                            board: session,
                            backend,
                        },
                        &mut numeric.reckoning,
                    )
                    .map_err(error)
            })
            .map_err(error)??;
        let anti = need(self.skaters.as_mut(), "skater pool")?
            .try_with_ground_control_board(&self.backend, board, |runtime, _| {
                let mut value = V::known([0.0; 4]);
                try_antiflip(
                    &mut AntiFlipRuntime {
                        processed,
                        feet: &runtime.foot_attributes,
                        references: refs,
                        overlay,
                    },
                    &mut numeric.reckoning,
                    &mut value,
                )
                .map_err(error)?;
                Ok::<_, GrindStateError>(value)
            })
            .map_err(error)??;
        need(self.skaters.as_mut(), "skater pool")?
            .try_apply_board_angular_displacement(
                &mut self.backend,
                board,
                anti,
                &mut numeric.reckoning,
            )
            .map_err(error)?;
        // ApplyPlayerWeight 0x82d86790.
        let weight = GrindCvars(refs).f(C::kPlayerWeight).map_err(error)?;
        let (up, point) = {
            let p = processed.lock().map_err(error)?;
            (
                need(p.prior_output().reckoning_up(), "Processed up vector")?.value,
                need(p.deck_transform(), "Processed deck transform")?.columns()[3],
            )
        };
        let force = scale(
            lanes(up),
            crate::physical_player_grind_manager::math::neg1(weight),
        );
        let session = need(self.skaters.as_ref(), "skater pool")?
            .session(board)
            .map_err(error)?;
        let part = session
            .try_live_part_transform_typed(&self.backend, 6)
            .map_err(error)?;
        let observation = session
            .try_part_contact_spy_observation(&self.backend, 6)
            .map_err(error)?;
        self.backend
            .try_apply_skateboard_world_force(observation, partial(force), point, part.columns()[3])
            .map_err(error)?;
        if GrindCvars(refs).b(C::kDoCollisionResponse).map_err(error)? {
            collision_force(self, owner, board, processed, &mut numeric.reckoning, true)?;
        }
        Ok(())
    }

    /// Nonspecific `FillPhysOut` 0x82d86820 inputs: jumped and its velocity.
    pub(crate) fn nonspecific_fill(
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
    ) -> Result<Option<V4>, GrindStateError> {
        Self::with_nonspecific(owner, board, skeleton, |s| {
            Ok(if need(s.jumped_this_frame, "Nonspecific jumped")? {
                Some(need(s.jump_vel, "Nonspecific jump velocity")?)
            } else {
                None
            })
        })
    }
}
