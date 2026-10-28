//! EvalCurrAnimForSimLOD82e6d170 and UpdateSimLODGlobal82e6ab08.
//! Uses the retained Andale evaluator and actual ACS fetch/blend/matrix jobs.
use crate::{
    actor_animation_construction::{ActorAnimationConstruction, AnimationPointerState,
        channel_runtime::ChannelBaseReference,
        initialization::{bind_pose::{BindPoseCommand, OwnedAnimationPoseBuffer},
            body_dispatch::tree_commands::TreeCommands},
        hands_face::sqt_jobs::{QueuedSqtProgram, matrix::{MatrixProgram, ProgramOutput}}},
    actor_animation_tree::eval::{AnimationEvalCommands, AnimationEvalParams},
    actor_face_playback::FaceCommands,
    animation::{AnimationMatrix, compose_pose_matrix},
    input_cvar_runtime::InputCvarReferences,
    pedestrian_animation::sim_lod_pose::SimLodBoneGlobal,
};
use std::sync::{Arc, Mutex};
use crate::actor_animation_construction::initialization::bind_pose::sqt_output::OwnedAnimationSqtBuffer;

/// AnimMan+8/+12: one shared identity source and one shared extraction scratch.
/// Full allocation follows PRES count; simulation commands address its prefix.
pub(crate) struct FullSkeletonScratch {
    identity: OwnedAnimationSqtBuffer,
    extracted: OwnedAnimationSqtBuffer,
    sim_bones: usize,
}
impl FullSkeletonScratch {
    pub(crate) fn new(full_bones: usize, sim_bones: usize)
        -> Result<Self, crate::actor_face_playback::ActorFaceError> {
        use crate::actor_face_playback::ActorFaceError;
        if sim_bones > full_bones || sim_bones > 20 { return Err(ActorFaceError::UnsupportedPose); }
        let identity = OwnedAnimationSqtBuffer::allocate_unknown(full_bones);
        identity.begin_extract()?;
        identity.extract(&vec![skate2_formats::andale::AndaleSqt {
            scale: [1.; 4], rotation: [0.,0.,0.,1.], translation: [0.;4],
        }; full_bones])?;
        identity.complete_sync()?;
        Ok(Self { identity, extracted: OwnedAnimationSqtBuffer::allocate_unknown(full_bones), sim_bones })
    }
    fn views(&self) -> Result<(OwnedAnimationSqtBuffer, OwnedAnimationSqtBuffer), crate::actor_face_playback::ActorFaceError> {
        Ok((self.identity.try_subrange(0..self.sim_bones)?, self.extracted.try_subrange(0..self.sim_bones)?))
    }
}

/// AnimMan::Init827191f0..19370 initializes identity matrices, registers them
/// on the shared TrajPedestrianACS, and retains the returned InvBindPoseMatId.
/// Invoke once from that startup owner, not once per pedestrian construction.
pub(crate) fn register_sim_lod_identity(commands: &Arc<Mutex<FaceCommands>>)
    -> Result<u32, crate::actor_animation_construction::initialization::bind_pose::BindPoseError> {
    use crate::{actor_face_playback::ActorFaceError,
        actor_animation_construction::initialization::bind_pose::BindPoseError};
    let mut commands = commands.lock().map_err(|_| ActorFaceError::Lock)?;
    let bones = commands.database.lock().map_err(|_| ActorFaceError::Lock)?
        .parsed.as_ref().ok_or(ActorFaceError::MissingRecord)?.hierarchy.parents.len();
    if bones > 20 { return Err(BindPoseError::InvalidHierarchy); }
    let identity = [1.,0.,0.,0., 0.,1.,0.,0., 0.,0.,1.,0., 0.,0.,0.,1.];
    commands.register_fixed_matrix(&vec![identity; bones])
}

pub(crate) struct SimLodPose {
    /// Accepted SIM stream, retained before the shared scheduler consumes it.
    /// PRES replays it with its own hierarchy and callback allocations.
    pub(crate) recorded: Mutex<Option<TreeCommands>>,
    /// The actual callback2 destination, unreadable until extraction and Sync.
    pub(crate) pose: OwnedAnimationPoseBuffer,
    commands: Arc<Mutex<FaceCommands>>,
    inverse_bind: Arc<[AnimationMatrix]>,
    matrix_id: u32,
    lookup: [usize; 9],
}
impl SimLodPose {
    pub(crate) fn local_bone(&self, slot:usize)->Result<AnimationMatrix,String>{
        let index=*self.lookup.get(slot).ok_or("pedestrian SimLOD lookup slot")?;
        self.pose.matrices().map_err(|e|format!("pedestrian local bone: {e:?}"))?
            .ok_or("pedestrian local bone before Sync")?.get(index).copied()
            .ok_or("pedestrian SimLOD local bone absent".into())
    }
    /// InitSimLODBoneLookupIDs82e8d650 resolves the authored hierarchy. The
    /// callback2 returns the AnimMan's actual registered fixed-matrix ID.
    pub(crate) fn new<K, W>(core: &ActorAnimationConstruction<K, W>,
        matrix_id: u32) -> Result<Self, String> {
        let initialized = core.initialization().ok_or("pedestrian Initialize absent")?;
        let playback = initialized.playback.as_ref().ok_or("pedestrian playback absent")?
            .lock().map_err(|_| "pedestrian playback lock")?;
        let commands = Arc::clone(&playback.commands);
        drop(playback);
        let command_owner = commands.lock().map_err(|_| "pedestrian commands lock")?;
        let hierarchy = Arc::clone(&command_owner.database);
        let inverse_bind = command_owner.fixed_matrices.get(matrix_id as usize).cloned()
            .ok_or("pedestrian registered matrix ID absent")?;
        drop(command_owner);
        let hierarchy_data = hierarchy.lock().map_err(|_| "pedestrian hierarchy lock")?;
        let parsed = &hierarchy_data.parsed.as_ref().ok_or("pedestrian parsed hierarchy absent")?.hierarchy;
        if parsed.bone_names.len() > 20 {
            return Err("pedestrian SimLOD hierarchy exceeds native20bone buffer".into());
        }
        if inverse_bind.len() != parsed.bone_names.len() {
            return Err("pedestrian inverse-bind/hierarchy count mismatch".into());
        }
        let mut lookup = [0; 9];
        for (slot, name) in lookup.iter_mut().zip([
            "Head", "RightForeArm", "RightHand", "RightHandProp", "LeftForeArm",
            "LeftHand", "LeftHandProp", "Hips", "Spine1",
        ]) {
            // GetBone82590fc0 compares all five FastString words; decoding
            // intentionally canonicalizes case and is only for display.
            let packed = skate2_formats::andale::PackedFastString::<5>::encode(name)
                .map_err(|e| format!("pedestrian SimLOD bone name: {e:?}"))?;
            *slot = parsed.packed_bone_names.iter().position(|bone| *bone == packed)
                .ok_or_else(|| format!("pedestrian SimLOD bone assertion: {name}"))?;
        }
        let pose = OwnedAnimationPoseBuffer::allocate_unknown(Arc::clone(&hierarchy), parsed.bone_names.len())
            .map_err(|e| format!("pedestrian pose allocation: {e:?}"))?;
        drop(hierarchy_data);
        Ok(Self { pose, commands, inverse_bind, matrix_id, lookup, recorded: Mutex::new(None) })
    }

    /// Inside AnimMan::EvaluatePedCommandBuffer's existing BeginCommandSet.
    /// The shared submit_sqt_programs/sync_sqt_programs own execution and fence.
    /// Stream slots and reuse barriers remain on that same command-set owner.
    pub(crate) fn queue<K, W>(&self, core: &mut ActorAnimationConstruction<K, W>,
        update_old_time: bool, blend_frames: u32, blend_cull_threshold: f32,
        cvars: Option<InputCvarReferences>) -> Result<(), String> {
        self.queue_branch(core, update_old_time, blend_frames, blend_cull_threshold, cvars, None)
    }

    pub(crate) fn queue_full<K, W>(&self, core: &mut ActorAnimationConstruction<K, W>,
        update_old_time: bool, blend_cull_threshold: f32, cvars: Option<InputCvarReferences>,
        scratch: &FullSkeletonScratch) -> Result<(), String> {
        // SetUpSimLODAnimation82e6eac8 passes FETCHBLEND (1) here too.
        self.queue_branch(core, update_old_time, 1, blend_cull_threshold, cvars, Some(scratch))
    }

    fn queue_branch<K, W>(&self, core: &mut ActorAnimationConstruction<K, W>,
        update_old_time: bool, blend_frames: u32, blend_cull_threshold: f32,
        cvars: Option<InputCvarReferences>, full: Option<&FullSkeletonScratch>) -> Result<(), String> {
        let initialized = core.initialization().ok_or("pedestrian Initialize absent")?;
        let playback = initialized.playback.as_ref().ok_or("pedestrian playback absent")?
            .lock().map_err(|_| "pedestrian playback lock")?;
        if !Arc::ptr_eq(&playback.commands, &self.commands) {
            return Err("pedestrian pose playback owner mismatch".into());
        }
        drop(playback);
        let reference = if let Some(root) = initialized.channels.as_ref().and_then(|channels| channels.root) {
            Some(ChannelBaseReference::Channel(root))
        } else {
            let base = core.base().ok_or("pedestrian Animatable absent")?;
            if base.base_animation == AnimationPointerState::Null { None } else {
                Some(ChannelBaseReference::Body(base.body_root.clone().ok_or("pedestrian body root absent")?))
            }
        };
        let bind_pose = initialized.bind_pose.clone().ok_or("pedestrian authored bind pose absent")?;
        let commands = self.commands.lock().map_err(|_| "pedestrian commands lock")?;
        if commands.setup.as_ref().ok_or("pedestrian ACS setup absent")?.codec != 1 {
            return Err("pedestrian ACS requires native VBR fetch engine".into());
        }
        let owner = commands.buffered.clone().ok_or("pedestrian buffered ACS absent")?;
        let hierarchy = commands.database.lock().map_err(|_| "pedestrian commands hierarchy lock")?
            .hierarchy_snapshot().ok_or("pedestrian commands hierarchy absent")?;
        drop(commands);
        let matrix = {
            let mut buffered = owner.lock().map_err(|_| "pedestrian ACS lock")?;
            let matrix = buffered.sqt_programs.iter()
                .filter(|program| matches!(program.output, ProgramOutput::Matrix(_))).count();
            if !buffered.constructed || buffered.in_command_set == 0 || buffered.in_commands != 0
                || matrix >= buffered.max_dynamic_matrices || matrix >= buffered.max_pose_operations {
                return Err("pedestrian ACS command-set state/capacity assertion".into());
            }
            buffered.in_commands += 1;
            buffered.nblend.begin_commands();
            u16::try_from(matrix).map_err(|_| "pedestrian matrix slot overflow")?
        };
        let params = AnimationEvalParams { commands: Arc::clone(&self.commands), update_old_time,
            blend_frames, blend_cull_threshold, blend_space_cvars: cvars };
        let mut tree = TreeCommands::new_stream(Arc::clone(&owner))
            .map_err(|e| format!("pedestrian stream allocation: {e:?}"))?;
        tree.fetch_code = crate::animation_job_owners::AnimationJobCode::FetchVbr;
        tree.enable_reuse();
        let full_views = full.map(FullSkeletonScratch::views).transpose()
            .map_err(|e| format!("pedestrian full scratch: {e:?}"))?;
        if let Some((identity, _)) = &full_views {
            tree.set_external_sqt(identity).map_err(|e| format!("pedestrian SetSQT callback3: {e:?}"))?;
        }
        if let Some(reference) = reference {
            core.eval_reference(&reference, &params, true, &mut tree)
                .map_err(|e| format!("pedestrian pose Eval: {e}"))?;
        } else {
            // This is the native null-current-animation branch, using the
            // actual authored bind node, never an invented fallback pose.
            tree.fetch_pose(&bind_pose).map_err(|e| format!("pedestrian bind Eval: {e:?}"))?;
        }
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        static STREAMS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_PEDESTRIANS").is_some())
            && STREAMS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 300 == 0
        {
            for (_, fetch) in &tree.fetches {
                if let crate::actor_animation_construction::initialization::body_dispatch::tree_commands::Fetch::Clip(plan) = fetch {
                    std::eprintln!("pedestrian pose fetch guid={} interpolated={} parts={:?}",
                        plan.guid, plan.timing.blend_frames,
                        plan.parts.iter().map(|part|part.current).collect::<Vec<_>>());
                }
            }
        }
        if let Some((_, extracted)) = &full_views {
            tree.add_sqt(true).map_err(|e| format!("pedestrian AddSQT: {e:?}"))?;
            // Raw82e6d2f0..300: r5=true, r6=root0. Float occupies r4's slot.
            tree.uniform_scale(1.0, true, 0).map_err(|e| format!("pedestrian UniformScale: {e:?}"))?;
            extracted.begin_shared_extract().map_err(|e| format!("pedestrian ExtractSQT callback2: {e:?}"))?;
        }
        let sqt = tree.output().map_err(|e| format!("pedestrian pose stack: {e:?}"))?;
        let mut buffered = owner.lock().map_err(|_| "pedestrian ACS lock")?;
        if full_views.is_some() { buffered.commands.push(BindPoseCommand::ExtractSqt { sqt }); }
        buffered.commands.push(BindPoseCommand::SetFixedMatrix { id: self.matrix_id });
        buffered.commands.push(BindPoseCommand::PoseMatMult { sqt, matrix });
        buffered.commands.push(BindPoseCommand::ExtractMatrix { matrix });
        let mut output = MatrixProgram::new_fixed(Arc::clone(&self.inverse_bind), &self.pose)
            .map_err(|e| format!("pedestrian matrix destination: {e:?}"))?;
        output.sqt_extract = full_views.map(|(_, extracted)| extracted);
        *self.recorded.lock().map_err(|_| "pedestrian recorded stream lock")? = Some(tree.clone());
        buffered.retain_sqt_program(QueuedSqtProgram { commands: tree,
            output: ProgramOutput::Matrix(output), hierarchy })
            .map_err(|e| format!("pedestrian EndCommands: {e:?}"))
    }

    /// Called after SetTargetTransform, before UpdateHandPropTargetTransform
    /// (UpdateTargetTransform82e7fc88..ca0). Requires the actual completed pose.
    pub(crate) fn global_bones(&self, target: AnimationMatrix) -> Result<SimLodBoneGlobal, String> {
        let pose = self.pose.matrices().map_err(|e| format!("pedestrian pose read: {e:?}"))?
            .ok_or("pedestrian pose has not completed extraction and Sync")?;
        let mut matrices = [[[0.; 4]; 4]; 9];
        for (destination, index) in matrices.iter_mut().zip(self.lookup) {
            let local = *pose.get(index).ok_or("pedestrian SimLOD bone lookup assertion")?;
            let global = compose_pose_matrix(target, local);
            *destination = std::array::from_fn(|column| std::array::from_fn(|row| global[column * 4 + row]));
        }
        Ok(SimLodBoneGlobal { matrices })
    }

    /// UpdateTargetTransform82e7f6cc reads SimLODPoseBuffer[0] (+5f90),
    /// after the same ACS fence as the nine global-bone consumers.
    pub(crate) fn root_delta(&self) -> Result<AnimationMatrix,String> {
        self.pose.matrices().map_err(|e|format!("pedestrian root pose: {e:?}"))?
            .ok_or("pedestrian root pose has not completed Sync")?.first().copied()
            .ok_or("pedestrian root pose absent".into())
    }
}
