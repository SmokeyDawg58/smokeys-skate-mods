//! `PhysState_Boneless` (602): Enter 0x82d89638, Update 0x82da0d20 with DoJump
//! 0x82d9ebe8 and CalcBonelessJump 0x82d978e0, and UpdatePostPhysics 0x82d896b8.
//! Its Exit and FillPhysOut vtable slots are the shared BLR 0x825187c0.
use super::ground_pre_solve::{GroundPreparationGrindPolicy, HostGroundPreparationNumeric};
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    debug_overlay::DebugOverlaySlot,
    dynamic_assembly_session::{
        skater_pool::SimulationSkaterAssemblyHandle as Handle,
        skeleton_normal::general_owner::SkeletonGeneralContinuation,
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{Boneless, JumpInfo, StateKind, StatePayload},
    },
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_trajectory_owners::{
        RetainedTrajectoryLaunchInfo, skeleton_launch_info::try_fill_launch_info,
    },
    physics::PointGraph,
    reckoning_numeric::{self as math, ReckoningNumeric},
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum BonelessError {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Service(&'static str, String),
}
impl std::fmt::Display for BonelessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "state602 Boneless: {self:?}")
    }
}
type E = BonelessError;
#[cfg(feature = "skate3-runtime")]
#[path = "skate3_boneless_binding.rs"]
mod source3;

fn service(stage: &'static str, error: impl std::fmt::Debug) -> E {
    E::Service(stage, format!("{error:?}"))
}
const ZERO: V = V::known([0.0; 4]);

fn state<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &Handle,
    skeleton: &Handle,
    f: impl FnOnce(&mut Boneless) -> Result<T, E>,
) -> Result<T, E> {
    let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
    let Some(StatePayload::Boneless(s)) = states
        .states
        .get_mut(&StateKind::Boneless)
        .filter(|s| s.published)
        .and_then(|s| s.value.as_mut())
    else {
        return Err(E::Missing("retained Boneless"));
    };
    let binding = &s.base.physical;
    if !binding.board.same_as(board)
        || !binding.skeleton.same_as(skeleton)
        || !binding
            .owner
            .upgrade()
            .is_some_and(|v| Arc::ptr_eq(&v, owner))
    {
        return Err(E::ForeignOwner);
    }
    f(s)
}
fn processed(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<Arc<Mutex<SkeletonProcessedContact>>, E> {
    Ok(Arc::clone(
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .processed(),
    ))
}
/// 8c4 `mIsGenerallyPushingRightFoot` (0x04000000); its known bit is required.
fn pushing_right_foot(p: &Mutex<SkeletonProcessedContact>) -> Result<bool, E> {
    let (bits, known) = p.lock().map_err(|_| E::Poisoned)?.known_flags_8c4();
    if known & 0x0400_0000 == 0 {
        return Err(E::Missing("Processed 8c4 pushing-right-foot"));
    }
    Ok(bits & 0x0400_0000 != 0)
}
/// `lhz 8d0; clrlwi 31` reads the big-endian upper halfword: word bit
/// 0x10000 (`mStartFootJump`), not word bit 0.
fn start_foot_jump(p: &Mutex<SkeletonProcessedContact>) -> Result<bool, E> {
    Ok(p.lock()
        .map_err(|_| E::Poisoned)?
        .physics_state_fields()
        .flags_8d0
        .ok_or(E::Missing("Processed flags8d0"))?
        & 0x0001_0000
        != 0)
}
/// `GetAttributePointer`, zeroed `DefaultDataArea(0x50)` when absent, then the
/// eight-point graph with X at +0x10 and Y at +0x30.
fn graph(attributes: &AttributeRuntimeInstance, key: u64, input: f32) -> Result<f32, E> {
    let data = attributes
        .try_get_data(key, 0)
        .map_err(|e| service("Boneless graph attribute", e))?;
    let word = |offset: usize| -> Result<f32, E> {
        match &data {
            None => Ok(0.0),
            Some(d) => Ok(f32::from_be_bytes(
                d.bytes()
                    .get(offset..offset + 4)
                    .ok_or(E::Missing("Boneless graph data"))?
                    .try_into()
                    .expect("four graph bytes"),
            )),
        }
    };
    let mut x = [0.0; 8];
    let mut y = [0.0; 8];
    for (i, (x_point, y_point)) in x.iter_mut().zip(&mut y).enumerate() {
        *x_point = word(16 + i * 4)?;
        *y_point = word(48 + i * 4)?;
    }
    Ok(PointGraph::<8> { x, y }.evaluate(input))
}

/// CalcBonelessJump 0x82d978e0: zeroed result, then 8d0 `StartFootJump` and the
/// live maximum error. A set bit blends a prepared-velocity launch with the
/// COM's horizontal velocity by the up.y graph; when the blend's speed misses
/// the COM speed by more than the maximum, the COM velocity is used instead.
fn calc_boneless_jump<N: ReckoningNumeric>(
    p: &Mutex<SkeletonProcessedContact>,
    attributes: &AttributeRuntimeInstance,
    refs: &InputCvarReferences,
    n: &mut N,
) -> Result<JumpInfo, E>
where
    N::Error: std::fmt::Debug,
{
    let start = start_foot_jump(p)?;
    let maximum = refs
        .try_boneless_max_jump_error()
        .map_err(|e| service("kMaxJumpErrorBoneless", e))?;
    if !start {
        return Ok(JumpInfo {
            velocity: ZERO,
            com_height: 0.0,
            do_jump: false,
        });
    }
    let (up, prepared, com_velocity) = {
        let p = p.lock().map_err(|_| E::Poisoned)?;
        (
            p.prior_output()
                .reckoning_up()
                .ok_or(E::Missing("Processed up+220"))?
                .value,
            p.prepare_output()
                .velocity()
                .ok_or(E::Missing("Processed prepared velocity+2c0"))?
                .value,
            p.prior_output()
                .centre_of_mass_velocity()
                .ok_or(E::Missing("Processed COM velocity+260"))?
                .value,
        )
    };
    let numeric = |e| service("Boneless jump numeric", e);
    // 0x82d97990..a30: prepared velocity minus its up component, and that
    // horizontal length through the two-refinement estimate (zero at zero).
    let dot = n.dot_vector(prepared, up).map_err(numeric)?;
    let horizontal = prepared.zip(up.map(|u| u * dot), |a, b| a - b);
    let length = math::try_length(horizontal, n).map_err(numeric)?;
    let up_y = up.xyz[1];
    let a = graph(attributes, 0x6d78_1efa_f01e_707d, up_y)?;
    let b = graph(attributes, 0xc911_2ccd_0bcb_1850, length)?;
    let vertical = b * a;
    let c = graph(attributes, 0x88d0_cdef_a38a_36d6, length)?;
    // 0x82d97af4 recomputes the same dot with swapped operands.
    let dot = n.dot_vector(up, prepared).map_err(numeric)?;
    let forward = c * length;
    let horizontal = prepared.zip(up.map(|u| u * dot), |a, b| a - b);
    let direction = math::try_normalize_safe(horizontal, ZERO, n).map_err(numeric)?;
    let com_dot = n.dot_vector(up, com_velocity).map_err(numeric)?;
    let com_horizontal = com_velocity.zip(up.map(|u| u * com_dot), |a, b| a - b);
    // 0x82d97b9c also normalizes `com_horizontal` into a dead local; the
    // callee is pure, so omitting it changes no state.
    let from_prepared = direction.zip(up.map(|u| u * vertical), |d, u| d.mul_add(forward, u));
    let from_com = up.zip(com_horizontal, |u, h| vertical.mul_add(u, h));
    let g = graph(attributes, 0x9228_b7f1_8c22_3f9c, up_y)?;
    let remaining = 1.0 - g;
    let blended = from_com.map(|v| v * remaining);
    let velocity = from_prepared.zip(blended, |p, c| p.mul_add(g, c));
    let com_length = math::try_length(com_velocity, n).map_err(numeric)?;
    let jump_length = math::try_length(velocity, n).map_err(numeric)?;
    let difference = f32::from_bits((jump_length - com_length).to_bits() & 0x7fff_ffff);
    // 0x82d97d1c: all-lanes VCMPGT on splats, so unordered keeps the blend.
    let velocity = if difference > maximum {
        com_velocity
    } else {
        velocity
    };
    Ok(JumpInfo {
        velocity,
        com_height: 0.0,
        do_jump: true,
    })
}

impl PhysicalAssemblyWorld {
    /// Enter 0x82d89638: EnableBoardAnimation, the pushing-foot toe index (19
    /// right, else 15), its `GetPartTransform` translation, then the right-leg
    /// flag from a second 8c4 read.
    pub(crate) fn try_enter_pair_boneless<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        state(owner, board, skeleton, |_| Ok(()))?;
        #[cfg(feature = "skate3-runtime")]
        {
            state(owner,board,skeleton,|s|{s.source3=None;Ok(())})?;
            if owner.skate3_footplants.load(std::sync::atomic::Ordering::Relaxed) {
                return self.try_enter_source_boneless(owner,board,skeleton);
            }
        }
        self.try_enable_board_animation_for_hold(board, false, refs, numeric)
            .map_err(|e| service("EnableBoardAnimation", e))?;
        let p = processed(owner)?;
        let (index, part) = if pushing_right_foot(&p)? {
            (19, 19)
        } else {
            (15, 15)
        };
        state(owner, board, skeleton, |s| {
            s.landing_toe_index = Some(index);
            Ok(())
        })?;
        let transform = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.try_query_part_transform(part)
            })
            .map_err(|e| service("Skeleton pair", e))?
            .map_err(|e| service("GetPartTransform", e))?;
        let right = pushing_right_foot(&p)?;
        state(owner, board, skeleton, |s| {
            s.landing_toe_pos = Some(transform.columns()[3]);
            s.right_leg = Some(right);
            Ok(())
        })
    }

    /// Update 0x82da0d20: the fixed-bone Skeleton update at the stored toe, a
    /// three-frame foot collision disable and world IK target at blend 1, then
    /// DoJump only when 8d0 `StartFootJump` and live `kDoJump` are both set.
    #[expect(
        clippy::too_many_arguments,
        reason = "actual pair, ground and source services"
    )]
    pub(crate) fn try_update_pair_boneless(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostGroundPreparationNumeric,
        policy: GroundPreparationGrindPolicy,
    ) -> Result<(), E> {
        if owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .current
            != CurrentPhysicalState::Boneless
        {
            return Err(E::ForeignOwner);
        }
        let (index, position) = state(owner, board, skeleton, |s| {
            #[cfg(feature = "skate3-runtime")]
            if let Some(source)=s.source3.as_ref() {
                return Ok((i32::try_from(source.toe).map_err(|_|E::Missing("source boneless toe"))?,
                    V::known(source.anchor)));
            }
            Ok((
                s.landing_toe_index.ok_or(E::Missing("landing toe index"))?,
                s.landing_toe_pos
                    .ok_or(E::Missing("landing toe position"))?,
            ))
        })?;
        let part = usize::try_from(index).map_err(|_| E::Missing("entered landing toe"))?;
        let progress = self
            .try_update_pair_fixed_bone_skeleton(
                owner,
                board,
                skeleton,
                position,
                part,
                ground,
                refs,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
            )
            .map_err(|e| service("UpdateFixedWorldPositionOfBone", e))?;
        if progress.next != SkeletonGeneralContinuation::Complete {
            return Err(E::Missing("complete fixed-bone Skeleton"));
        }
        let right = state(owner, board, skeleton, |s| {
            #[cfg(feature = "skate3-runtime")]
            if let Some(source)=s.source3.as_ref() {return Ok(source.right);}
            s.right_leg.ok_or(E::Missing("right leg"))
        })?;
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.disable_foot_collisions(right, 3);
                normal.try_set_world_foot_ik_target(!right, position, 1.0)
            })
            .map_err(|e| service("Skeleton pair", e))?
            .map_err(|e| service("foot IK target", e))?;
        let p = processed(owner)?;
        #[cfg(feature = "skate3-runtime")]
        let source=state(owner,board,skeleton,|s|Ok(s.source3.is_some()))?;
        #[cfg(not(feature = "skate3-runtime"))]
        let source=false;
        if start_foot_jump(&p)?
            && (source || refs.try_boneless_do_jump().map_err(|e|service("kDoJump",e))?)
        {
            self.try_boneless_do_jump(owner, board, skeleton, &p, ground, refs, numeric, policy)?;
        }
        Ok(())
    }

    /// DoJump 0x82d9ebe8: CalcBonelessJump and GetLaunchInfo, then the jump
    /// velocity and both start overrides at the toe's height on the COM's up
    /// line plus `kTrajPosDelta`, before TrajectorySelector Launch and Update.
    #[expect(
        clippy::too_many_arguments,
        reason = "actual pair, ground and source services"
    )]
    fn try_boneless_do_jump(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        p: &Mutex<SkeletonProcessedContact>,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostGroundPreparationNumeric,
        policy: GroundPreparationGrindPolicy,
    ) -> Result<(), E> {
        #[cfg(feature = "skate3-runtime")]
        let source=source3::launch(owner,board,skeleton,p)?;
        #[cfg(not(feature = "skate3-runtime"))]
        let source:Option<(V,V)>=None;
        let (velocity,source_start,toe)=if let Some((velocity,start))=source {
            (velocity,Some(start),None)
        } else {
            let (attributes,toe)=state(owner,board,skeleton,|s|Ok((
                s.attribs.clone().ok_or(E::Missing("Boneless attributes"))?,
                s.landing_toe_pos.ok_or(E::Missing("landing toe position"))?)))?;
            (calc_boneless_jump(p,&attributes,refs,&mut numeric.reckoning)?.velocity,None,Some(toe))
        };
        let mut launch = RetainedTrajectoryLaunchInfo::constructed();
        {
            let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
            let trajectory = pool
                .try_with_ground_control_board(&self.backend, board, |r, _| {
                    r.wallride_trajectory.clone()
                })
                .map_err(|e| service("trajectory attributes", e))?;
            let constructor = &pool
                .skeleton_construction(skeleton)
                .map_err(|e| service("Skeleton", e))?
                .ok_or(E::Missing("Skeleton constructor"))?
                .constructor;
            try_fill_launch_info(constructor, owner, &trajectory, &mut launch)
                .map_err(|e| service("GetLaunchInfo", e))?;
        }
        launch.start_velocity = velocity;
        launch.player_jumped = true;
        let start=if let Some(start)=source_start {start} else {
        let delta = refs
            .try_boneless_traj_pos_delta()
            .map_err(|e| service("kTrajPosDelta", e))?;
        let (up, com) = {
            let p = p.lock().map_err(|_| E::Poisoned)?;
            (
                p.prior_output()
                    .reckoning_up()
                    .ok_or(E::Missing("Processed up+220"))?
                    .value,
                p.prior_output()
                    .centre_of_mass()
                    .ok_or(E::Missing("Processed COM+250"))?
                    .value,
            )
        };
        // 0x82d9ec84..ccc: (toe - COM) . up, then up FMA onto COM, then up
        // FMA with the same delta for the trajectory and board overrides.
        let height = numeric
            .reckoning
            .dot_vector(toe.ok_or(E::Missing("native boneless toe"))?.zip(com, |t, c| t - c), up)
            .map_err(|e| service("Boneless start numeric", e))?;
        let projected = up.zip(com, |u, c| u.mul_add(height, c));
        up.zip(projected, |u, q| u.mul_add(delta, q))
        };
        launch.trajectory_start_override = start;
        launch.board_position_override = start;
        launch.use_start_override = true;
        self.try_launch_pair_onboard(
            board,
            skeleton,
            &launch,
            ground,
            &mut numeric.reckoning,
            &mut numeric.sin_cos,
        )
        .map_err(|e| service("TrajectorySelector.Launch", e))?;
        let _ready = self
            .try_complete_pair_selector(
                board,
                skeleton,
                ground,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
                policy,
            )
            .map_err(|e| service("TrajectorySelector.Update", e))?;
        Ok(())
    }

    /// UpdatePostPhysics 0x82d896b8 tail-calls `Wipeout::CheckForPlantWipeout`.
    pub(crate) fn try_post_pair_boneless<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        state(owner, board, skeleton, |_| Ok(()))?;
        #[cfg(feature = "skate3-runtime")]
        if state(owner,board,skeleton,|s|Ok(s.source3.is_some()))? {
            // Latest source S3's postphysics dispatcher leaves Boneless empty;
            // its FootPlant601 collision check must not be assigned to602.
            return Ok(());
        }
        self.try_check_plant_wipeout(board, skeleton, owner, overlay, numeric)
            .map_err(|e| service("CheckForPlantWipeout", e))
    }
}
