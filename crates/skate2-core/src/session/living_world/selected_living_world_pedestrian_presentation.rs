//! FullPedestrianACS command replay, on the existing presentation hierarchy.
use crate::{
    actor_animation_construction::{
        initialization::bind_pose::{OwnedAnimationPoseBuffer, sqt_output::OwnedAnimationSqtBuffer},
        hands_face::{runtime, sqt_jobs::{QueuedSqtProgram, matrix::{MatrixProgram, ProgramOutput}}},
    },
    actor_face_playback::{FaceCommands, FacePlaybackData},
    character::{CharacterHierarchy, CharacterScene, palette::CharacterPalette},
    physical_player_assembly::PhysicalAssemblyWorld,
    input_cvar_runtime::InputCvarReferences,
};
use std::sync::{Arc,Mutex};
use crate::pedestrian_adjust_owners::post_adjust_codec;

/// `livingworld_models` `Hide_Skateboard`, which `cPresPedestrian`'s constructor reads at 0x828124f0.
const HIDE_SKATEBOARD:u64=0x0626_fa10_792b_ba58;

/// cPresPedestrian owns full pose and post-adjust allocations; source828122b0.
/// Mesh remaps/inverse bind matrices stay in the shared CharacterPalette owner.
pub(crate) struct PedestrianPresentationPose {
    pub(crate) palette: Arc<Mutex<CharacterPalette>>,
    pub(crate) pose: OwnedAnimationPoseBuffer,
    pub(crate) post_adjust: OwnedAnimationSqtBuffer,
    marquee: Option<crate::selected_living_world_pedestrian_visual::marquee::MarqueeEntity>,
    commands: Arc<Mutex<FaceCommands>>,
    previous_post_adjust_bones:[i8;6],
}
impl PedestrianPresentationPose {
    fn new(playback:&Arc<Mutex<FacePlaybackData>>,scene:Arc<CharacterScene>,
        marquee:Option<&crate::selected_living_world_pedestrian_visual::marquee::MarqueeStartup>,hide_skateboard:bool)
        ->Result<Self,String>{
        let commands=Arc::clone(&playback.lock().map_err(|_|"pedestrian PRES playback lock")?.commands);
        let database=match marquee {Some(owner)=>Arc::clone(&owner.database),
            None=>Arc::clone(&commands.lock().map_err(|_|"pedestrian PRES commands lock")?.database)};
        let marquee=marquee.map(|owner|owner.entity(&scene.schema_name)).transpose()?;
        let hierarchy=database.lock().map_err(|_|"pedestrian PRES database lock")?.parsed.as_ref()
            .ok_or("pedestrian PRES database absent")?.hierarchy.clone();
        let palette=CharacterPalette::pedestrian(scene,&CharacterHierarchy{bone_names:hierarchy.bone_names.clone()},0,
            hide_skateboard).map_err(|e|e.to_string())?;
        let pose=OwnedAnimationPoseBuffer::allocate_unknown(database,hierarchy.parents.len())
            .map_err(|e|format!("pedestrian PRES pose allocation: {e:?}"))?;
        let post_adjust=OwnedAnimationSqtBuffer::allocate_unknown(hierarchy.parents.len());
        post_adjust.begin_extract().map_err(|e|format!("{e:?}"))?;
        // Constructor82812478..4c0, not a replacement animation pose.
        post_adjust.extract(&vec![skate2_formats::andale::AndaleSqt {
            scale:[1.;4],rotation:[0.,0.,0.,1.],translation:[0.;4],
        };hierarchy.parents.len()]).map_err(|e|format!("{e:?}"))?;
        post_adjust.complete_sync().map_err(|e|format!("{e:?}"))?;
        Ok(Self{palette:Arc::new(Mutex::new(palette)),pose,post_adjust,commands,marquee,
            previous_post_adjust_bones:[-1;6]})
    }

    /// UpdateFromReplayData82809d98 clears previous rotations then decodes
    /// current six records, including the signed-byte strictly-positive gate.
    fn update_post_adjust(&mut self,adjust:&crate::pedestrian_adjust_owners::PostAdjust)
        ->Result<(),String>{
        let (_,mut values)=self.post_adjust.read().map_err(|e|format!("post-adjust read: {e:?}"))?;
        for bone in self.previous_post_adjust_bones{
            if bone>0{values.get_mut(bone as usize).ok_or("previous post-adjust bone outside hierarchy")?
                .rotation=[0.,0.,0.,1.];}
        }
        for (index,record) in adjust.adjustments.iter().enumerate(){
            let record=record.ok_or("post-adjust replay record unwritten")?;
            let bone=record.bone as i8;
            if bone>0{values.get_mut(bone as usize).ok_or("post-adjust bone outside hierarchy")?
                .rotation=post_adjust_codec::decompress(record.axis_angle)?;}
            self.previous_post_adjust_bones[index]=bone;
        }
        self.post_adjust.begin_extract().map_err(|e|format!("post-adjust begin: {e:?}"))?;
        self.post_adjust.extract(&values).map_err(|e|format!("post-adjust write: {e:?}"))?;
        self.post_adjust.complete_sync().map_err(|e|format!("post-adjust publish: {e:?}"))
    }

    fn queue(&self,recorded:&crate::actor_animation_construction::initialization::body_dispatch::tree_commands::TreeCommands,
        inverse_bind:Arc<[crate::animation::AnimationMatrix]>)
        ->Result<(),String>{
        let hierarchy=self.commands.lock().map_err(|_|"pedestrian PRES command lock")?.database.lock()
            .map_err(|_|"pedestrian PRES hierarchy lock")?.hierarchy_snapshot()
            .ok_or("pedestrian PRES hierarchy absent")?;
        let adjust=self.post_adjust.try_subrange(0..hierarchy.parents.len()).map_err(|e|format!("{e:?}"))?;
        let tree=recorded.replay_pedestrian(&self.commands,&adjust)
            .map_err(|e|format!("pedestrian LoadCmdBufferInfo: {e:?}"))?;
        let mut matrix=MatrixProgram::new_fixed(inverse_bind,&self.pose)
            .map_err(|e|format!("pedestrian PRES matrix destination: {e:?}"))?;
        if let Some(full)=&self.marquee {
            let output=full.body.try_subrange(0..hierarchy.parents.len()).map_err(|e|format!("{e:?}"))?;
            output.begin_shared_extract().map_err(|e|format!("{e:?}"))?;
            matrix.sqt_extract=Some(output);
        }
        let owner=Arc::clone(&tree.owner);
        let mut owner=owner.lock().map_err(|_|"pedestrian PRES buffered lock")?;
        if owner.in_command_set!=1||owner.in_commands!=0{return Err("pedestrian PRES replay command state".into());}
        owner.in_commands+=1;
        owner.retain_sqt_program(QueuedSqtProgram{commands:tree,output:ProgramOutput::Matrix(matrix),hierarchy})
            .map_err(|e|format!("pedestrian PRES EndCommands: {e:?}"))
    }
}

impl super::super::SelectedRecipeFrame<'_>{
    /// UpdateAnimationPoses82809610 / SyncAnimationPoses8280a640 followed by
    /// cLivingWorldSkinnedPresEntity.Update828121d8. No graph reevaluation.
    pub(crate) fn present_pedestrian_animation(&self,world:&PhysicalAssemblyWorld,cvars:&InputCvarReferences,
        tokens:&crate::actor_face_playback::blend_pose_stream::BlendPoseTokenPool)
        ->Result<(),String>{
        let mut living=world.grind_services.living_world().lock().map_err(|_|"pedestrian PRES roster lock")?;
        let roster=living.pedestrian_entities.as_deref_mut().ok_or("pedestrian PRES roster absent")?;
        let ids=roster.ids();
        if ids.is_empty(){return Ok(());}
        let playback=self.pedestrian_animation.startup.presentation.as_ref().ok_or("pedestrian PRES startup absent")?;
        let commands=Arc::clone(&playback.lock().map_err(|_|"pedestrian PRES playback lock")?.commands);
        let mut source=crate::actor_construction::body_session::ActorBodyAnimationServices::try_from_startup(
            self.pedestrian_animation.shared,cvars).map_err(|e|e.to_string())?;
        let mut replay=Vec::new();let mut token_count=0;
        for &id in &ids{
            let entity=roster.get_mut(id).ok_or("pedestrian PRES member absent")?;
            let recipe=entity.core.as_ref().ok_or("pedestrian PRES core absent")?.recipe_id;
            let animation=entity.animation.as_mut().ok_or("pedestrian PRES animation absent")?;
            if animation.presentation.is_none(){
                let assets=self.runtime.presentation.get(recipe).ok_or("pedestrian PRES finalized recipe absent")?;
                let hide_skateboard=entity.resources.as_ref().ok_or("pedestrian PRES resources absent")?.model
                    .try_get_data(HIDE_SKATEBOARD,0).map_err(|e|e.to_string())?
                    .is_some_and(|data|data.bytes().first().is_some_and(|byte|*byte!=0));
                animation.presentation=Some(PedestrianPresentationPose::new(playback,
                    Arc::clone(assets.finalized.scene().map_err(|e|e.to_string())?),
                    if animation.model_category==0{None}else{Some(self.pedestrian_animation.startup.marquee.as_ref()
                        .ok_or("Marquee startup absent")?)},hide_skateboard)?);
                if animation.model_category==0 {
                    let mut registered=self.pedestrian_animation.startup.regular_inverse_bind.lock().map_err(|_|"regular inverse bind lock")?;
                    if registered.is_none(){
                        let matrices=Arc::clone(&animation.presentation.as_ref().expect("initialized")
                            .palette.lock().map_err(|_|"regular palette lock")?.skinning().hierarchy_inverse_bind_pose);
                        commands.lock().map_err(|_|"regular ACS lock")?.register_fixed_matrix(&matrices)
                            .map_err(|e|format!("regular inverse-bind registration: {e:?}"))?;
                        *registered=Some(matrices);
                    }
                }
            }
            animation.presentation.as_mut().expect("initialized").update_post_adjust(
                entity.post_adjust.as_ref().ok_or("pedestrian post-adjust owner absent")?)?;
            if animation.frame.valid_blend_pose_buffer {
                let streams=crate::selected_living_world_pedestrian_visual::hands_face::replay_streams(
                    animation.blend_pose_buffer.as_ref().ok_or("pedestrian valid pose buffer absent")?,tokens,&mut token_count)?;
                replay.push((id,streams));
            }
        }
        // HandFaceManager828115c0 visits the presentation map in ID order.
        replay.sort_by_key(|(id,_)|*id);
        let hand_face=self.pedestrian_animation.startup.hands_face.as_ref().ok_or("pedestrian hand/face startup absent")?;
        let full_hand_face=self.pedestrian_animation.startup.marquee_hands_face.as_ref().ok_or("Marquee hand/face startup absent")?;
        let mut pending=Vec::new();
        for (owner,enabled,is_face,full) in [(&hand_face.hands,cvars.try_enable_pedestrian_hand_anim().map_err(|e|format!("{e:?}"))?,false,false),
            (&full_hand_face.hands,cvars.try_enable_marquee_hand_anim().map_err(|e|format!("{e:?}"))?,false,true),
            (&hand_face.face,cvars.try_enable_pedestrian_face_anim().map_err(|e|format!("{e:?}"))?,true,false),
            (&full_hand_face.face,cvars.try_enable_marquee_face_anim().map_err(|e|format!("{e:?}"))?,true,true)] {
            if !enabled{continue;}
            let mut owner=owner.lock().map_err(|_|"pedestrian hand/face owner lock")?;
            let mut selected=Vec::new();
            for (id,streams) in &replay{
                let animation=roster.get(*id).and_then(|e|e.animation.as_ref()).ok_or("hand/face entity absent")?;
                if (animation.model_category!=0)!=full{continue;}
                let valid=if is_face {[owner.valid(0,&streams[2])?,false]}
                    else{[owner.valid(0,&streams[0])?,owner.valid(1,&streams[1])?]};
                if valid[0]||valid[1]{selected.push((*id,streams,valid));}
                if selected.len()==10{break;}
            }
            if selected.is_empty(){continue;}
            let commands=Arc::clone(&owner.commands);
            runtime::begin_command_set(&commands).map_err(|e|format!("pedestrian hand/face Begin: {e:?}"))?;
            for (id,streams,valid) in selected{
                let output=&roster.get(id).and_then(|e|e.animation.as_ref()).and_then(|a|a.presentation.as_ref())
                    .ok_or("hand/face presentation absent")?.post_adjust;
                if valid[0]{owner.queue(0,&streams[if is_face{2}else{0}],output)?;}
                if valid[1]{owner.queue(1,&streams[1],output)?;}
            }
            runtime::end_command_set(&commands,&mut source).map_err(|e|format!("pedestrian hand/face End: {e:?}"))?;
            pending.push(commands);
        }
        for commands in pending{
            runtime::sync_command_set(&commands,&mut source).map_err(|e|format!("pedestrian hand/face Sync: {e:?}"))?;
        }
        // The selected runtime has no speech playback backend. Expression
        // tokens execute; native playing-channel Jaw overlays are not produced.
        runtime::begin_command_set(&commands).map_err(|e|format!("pedestrian PRES Begin: {e:?}"))?;
        let inverse_bind=self.pedestrian_animation.startup.regular_inverse_bind.lock().map_err(|_|"regular inverse bind lock")?
            .clone().or_else(||self.pedestrian_animation.startup.default_inverse_bind.clone()).ok_or("regular inverse bind absent")?;
        for &id in &ids{
            let animation=roster.get(id).and_then(|e|e.animation.as_ref()).ok_or("pedestrian PRES animation absent")?;
            let recorded=animation.sim_lod_pose.as_ref().ok_or("pedestrian SIM pose absent")?.recorded.lock()
                .map_err(|_|"pedestrian recorded stream lock")?;
            animation.presentation.as_ref().expect("initialized").queue(recorded.as_ref()
                .ok_or("pedestrian recorded animation stream absent")?,Arc::clone(&inverse_bind))?;
        }
        runtime::end_command_set(&commands,&mut source).map_err(|e|format!("pedestrian PRES End: {e:?}"))?;
        runtime::sync_command_set(&commands,&mut source).map_err(|e|format!("pedestrian PRES Sync: {e:?}"))?;
        let mut full_ids=ids.iter().copied().filter(|id|roster.get(*id).and_then(|e|e.animation.as_ref())
            .is_some_and(|a|a.model_category!=0)).collect::<Vec<_>>();
        full_ids.sort_unstable();
        if !full_ids.is_empty(){
            let full=self.pedestrian_animation.startup.marquee.as_ref().ok_or("Marquee startup absent")?;
            let commands=Arc::clone(&full.playback.lock().map_err(|_|"Marquee playback lock")?.commands);
            runtime::begin_command_set(&commands).map_err(|e|format!("Marquee Begin: {e:?}"))?;
            for (index,id) in full_ids.into_iter().enumerate(){
                let pres=roster.get(id).and_then(|e|e.animation.as_ref()).and_then(|a|a.presentation.as_ref()).expect("initialized");
                full.queue(index,pres.marquee.as_ref().ok_or("Marquee entity absent")?,&pres.post_adjust,&pres.palette,&pres.pose)?;
            }
            runtime::end_command_set(&commands,&mut source).map_err(|e|format!("Marquee End: {e:?}"))?;
            runtime::sync_command_set(&commands,&mut source).map_err(|e|format!("Marquee Sync: {e:?}"))?;
        }
        for id in ids{
            let entity=roster.get_mut(id).expect("captured PRES roster");
            let core=entity.core.as_ref().ok_or("pedestrian PRES core absent")?;
            let mut transform=std::array::from_fn(|i|core.transform[i/4][i%4]);transform[15]=1.;
            let presentation=entity.animation.as_mut().and_then(|a|a.presentation.as_mut()).expect("initialized");
            let pose=presentation.pose.matrices().map_err(|e|format!("pedestrian PRES pose: {e:?}"))?
                .ok_or("pedestrian PRES extraction not synchronized")?;
            let mut palette=presentation.palette.lock().map_err(|_|"pedestrian PRES palette lock")?;
            palette.update_bone_transforms(&pose).map_err(|e|e.to_string())?;
            palette.pack_for_upload(transform).map_err(|e|e.to_string())?;
        }
        Ok(())
    }
}

/// Renderer borrows the one completed palette; it never evaluates animation.
#[derive(Clone)]
pub struct SelectedPedestrianVisual {
    pub spawn_opacity:f32,
    pub entity_id:u32,
    pub recipe_id:u64,
    /// `cLivingWorldPresEntity::mModelAttribs`' `livingworld_models` collection.
    pub model_key:u64,
    pub palette:Arc<Mutex<CharacterPalette>>,
    pub colours:[[f32;4];2],
    pub irradiance:[[f32;4];9],
    pub position:[f32;3],
    pub material_lod_distance:f32,
    pub(crate) geometry_lod_squared:[f32;3],
    pub(crate) hierarchy:Arc<CharacterHierarchy>,
    /// The synchronized hierarchy pose this tick published, which a geometry LOD switch
    /// packs again (`SwitchLOD` 0x82806528). Frames between ticks switch from it too, while
    /// the SIM thread evaluates the next tick's pose.
    pub(crate) pose:Option<Arc<[crate::animation::AnimationMatrix]>>,
    pub(crate) transform:crate::animation::AnimationMatrix,
    /// The LOD0 RModel union, min then max.
    pub(crate) local_bounds:[[f32;3];2],
    pub bounds_center:[f32;3],
    /// SceneRenderView.Cull828501c0 translates the LOD0 local union, without rotation.
    pub world_bounds:[[f32;3];2],
    pub opacity_parameters:crate::LivingWorldOpacityParameters,
    pub opacity_settings:crate::LivingWorldOpacitySettings,
    pub simulation_center_velocity:[f32;4],
}
/// Where a frame draws a pedestrian: its local-to-world and what
/// SetLocalToWorldMatrix827efd08 derives from it.
#[derive(Clone, Copy, Debug)]
pub struct PedestrianPlacement {
    pub transform:crate::animation::AnimationMatrix,
    pub position:[f32;3],
    pub bounds_center:[f32;3],
    pub world_bounds:[[f32;3];2],
}
impl PedestrianPlacement {
    /// CalculateBoundsInfo827f2708: the LOD0 union midpoint transformed in X/Y/Z/W
    /// order; Cull's bounds translate the union without rotation.
    fn new(transform:crate::animation::AnimationMatrix,local_bounds:[[f32;3];2])->Self{
        let [minimum,maximum]=local_bounds;
        let midpoint:[f32;3]=std::array::from_fn(|i|(minimum[i]+maximum[i])*0.5);
        let bounds_center=std::array::from_fn(|i|{
            let x=transform[i]*midpoint[0];
            let y=transform[4+i].mul_add(midpoint[1],x);
            let z=transform[8+i].mul_add(midpoint[2],y);
            transform[12+i].mul_add(1.,z)
        });
        Self{transform,position:std::array::from_fn(|i|transform[12+i]),bounds_center,
            world_bounds:[std::array::from_fn(|i|minimum[i]+transform[12+i]),
                std::array::from_fn(|i|maximum[i]+transform[12+i])]}
    }
}
impl SelectedPedestrianVisual {
    /// Each frame owns its immutable synchronized pose. Only one playback
    /// palette per living pedestrian is needed; copying all of its per-mesh
    /// matrices, packed matrices and GPU rows every tick multiplies the archive.
    /// Its identity still distinguishes a despawn/respawn of the same entity ID.
    pub(crate) fn freeze_replay_pose(&mut self,previous:Option<&Self>)->Result<(),String> {
        let identity=self.palette.lock().map_err(|_|"replay pedestrian palette lock")?.identity();
        if let Some(previous)=previous
            && previous.palette.lock().map_err(|_|"replay previous palette lock")?.identity()==identity {
            self.palette=Arc::clone(&previous.palette);
        } else {
            let palette=self.palette.lock().map_err(|_|"replay pedestrian palette lock")?.clone();
            self.palette=Arc::new(Mutex::new(palette));
        }
        Ok(())
    }

    /// Restore into the playback owner, never the live animation owner.
    pub(crate) fn restore_replay_pose(&self)->Result<(),String> {
        let mut palette=self.palette.lock().map_err(|_|"replay pedestrian restore lock")?;
        let matrices=self.pose.as_deref().ok_or("replay pedestrian pose absent")?;
        palette.update_bone_transforms(matrices).map_err(|e|e.to_string())?;
        palette.pack_for_upload(self.transform).map_err(|e|e.to_string())
    }

    /// The tick's placement.
    #[must_use]
    pub fn placement(&self)->PedestrianPlacement{
        PedestrianPlacement{transform:self.transform,position:self.position,
            bounds_center:self.bounds_center,world_bounds:self.world_bounds}
    }
    /// The placement at `transform`, a local-to-world the presentation's restore blended.
    #[must_use]
    pub fn placement_at(&self,transform:crate::animation::AnimationMatrix)->PedestrianPlacement{
        PedestrianPlacement::new(transform,self.local_bounds)
    }
    /// Publish the synchronized pose again at the placement's local-to-world.
    pub fn publish_at(&self,placement:&PedestrianPlacement)->Result<(),String>{
        self.palette.lock().map_err(|_|"pedestrian placement palette lock")?
            .repack_for_upload(placement.transform).map_err(|e|e.to_string())
    }
    /// Called once the render-view camera is available. Reuses the actual
    /// synchronized PRES extraction; camera movement never reevaluates a tree.
    pub fn select_geometry_lod(&self,camera:[f32;3],placement:&PedestrianPlacement)->Result<u8,String>{
        let mut palette=self.palette.lock().map_err(|_|"pedestrian LOD palette lock")?;
        let position=placement.position;
        let lod=palette.scene().distance_lod(
            [position[0],position[1],position[2],1.],
            [camera[0],camera[1],camera[2],1.],self.geometry_lod_squared).map_err(|e|e.to_string())?;
        if palette.skinning().lod_level!=lod {
            let matrices=self.pose.as_deref().ok_or("pedestrian LOD pose before Sync")?;
            palette.select_pedestrian_lod(&self.hierarchy,lod).map_err(|e|e.to_string())?;
            palette.update_bone_transforms(matrices).map_err(|e|e.to_string())?;
            palette.pack_for_upload(placement.transform).map_err(|e|e.to_string())?;
        }
        Ok(lod)
    }
    /// cLivingWorldPresEntity::Update82811c40..1d7c. F32 division precedes
    /// the two double-precision operations and FSEL bounds, then FRSP.
    pub fn material_lod(&self,camera:[f32;3],position:[f32;3])->Result<f32,String>{
        let delta=[position[0]-camera[0],position[1]-camera[1],position[2]-camera[2],0.];
        let distance=crate::reckoning_numeric::try_length(delta,&mut crate::reckoning_numeric::HostReckoningNumeric)
            .map_err(|e|format!("pedestrian material LOD length: {e:?}"))?;
        let value=(f64::from(distance/self.material_lod_distance)-f64::from_bits(0x3fe6_6666_6666_6666))
            *f64::from_bits(0x400a_aaaa_aaaa_aaaa);
        let positive=if -value>=0. {0.}else{value};
        Ok((if 1.-positive>=0. {positive}else{1.}) as f32)
    }
}

/// BindConstants8280b5d0 copies the same SHLightingMan default probe as
/// cPresVehicle; no spatial interpolation is performed. Update82811bd8 fetches
/// the LOD distance array's element0 live (a null lookup is DefaultDataArea(4));
/// GetLODDistances fetches and squares all three.
fn living_world_lighting(vault:&Arc<crate::attribute_runtime::AttributeRuntimeVault>)
    ->Result<([[f32;4];9],[f32;3]),String>{
    let irradiance=crate::selected_living_world_vehicle_visual::vehicle_default_irradiance(vault)
        .map_err(|e|e.to_string())?;
    let rendering=vault.instance(skate2_formats::attrib::string_hash64("rendering"),
        0x5141_6546_b54f_421d);
    let mut distances=[0.;3];
    for (index,value) in distances.iter_mut().enumerate(){
        if let Some(data)=rendering.try_plain_array_data(0xf412_fc7b_191f_16c8,index as u32,
            0xe222_28fb_b3c2_09d8,4).map_err(|e|e.to_string())? {
            *value=f32::from_be_bytes(data.bytes().try_into().map_err(|_|"pedestrian LOD float width")?);
        }
    }
    Ok((irradiance,distances))
}

/// A crowd target's cNoSimPresPedestrian as a pedestrian visual: at the origin,
/// its mSpawnTransparency starting at zero (82812830), its geometry LOD 0.
#[expect(clippy::too_many_arguments,reason="one no-sim pedestrian's presentation inputs")]
pub(crate) fn no_sim_visual(id:u32,recipe_id:u64,model_key:u64,palette:&Arc<Mutex<CharacterPalette>>,
    colours:[[f32;4];2],hierarchy:&Arc<CharacterHierarchy>,
    pose:&crate::actor_animation_construction::initialization::bind_pose::OwnedAnimationPoseBuffer,
    vault:&Arc<crate::attribute_runtime::AttributeRuntimeVault>,cvars:&InputCvarReferences,
    actor:&crate::actor_construction::ActorConstruction)->Result<SelectedPedestrianVisual,String>{
    let (irradiance,distances)=living_world_lighting(vault)?;
    let (_,velocity)=crate::simulation_assembly_backend::constraint_session::dmo_living_world::local_skater_census_centre(actor)
        .map_err(|e|e.to_string())?;
    let bounds=palette.lock().map_err(|_|"crowd bounds palette lock")?
        .scene().level_bounds(0).map_err(|e|e.to_string())?.ok_or("crowd pedestrian original bounds absent")?;
    let local_bounds=[std::array::from_fn(|i|bounds.minimum[i]),std::array::from_fn(|i|bounds.maximum[i])];
    let transform=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.];
    let placement=PedestrianPlacement::new(transform,local_bounds);
    let model=vault.instance(0x70f2_41a7_4e74_0a66,model_key);
    Ok(SelectedPedestrianVisual{entity_id:id,recipe_id,model_key,spawn_opacity:0.,
        palette:Arc::clone(palette),colours,irradiance,material_lod_distance:distances[0],
        geometry_lod_squared:distances.map(|value|value*value),hierarchy:Arc::clone(hierarchy),
        pose:pose.matrices().map_err(|e|format!("crowd pose: {e:?}"))?.map(Arc::from),transform,
        local_bounds,bounds_center:placement.bounds_center,simulation_center_velocity:velocity,
        world_bounds:placement.world_bounds,
        opacity_parameters:crate::LivingWorldOpacityParameters::try_read(&model).map_err(|e|e.to_string())?,
        opacity_settings:cvars.try_living_world_opacity_settings().map_err(|e|e.to_string())?,
        position:placement.position})
}

pub(crate) fn pedestrian_visuals(world:&PhysicalAssemblyWorld,
    actor:&crate::actor_construction::ActorConstruction,
    vault:&Arc<crate::attribute_runtime::AttributeRuntimeVault>,
    cvars:&InputCvarReferences)->Result<Vec<SelectedPedestrianVisual>,String>{
    let living=world.grind_services.living_world().lock().map_err(|_|"pedestrian visual roster lock")?;
    let Some(roster)=living.pedestrian_entities.as_deref() else{return Ok(Vec::new())};
    let mut visuals=Vec::new();
    let mut lighting=None;
    let mut center_velocity=None;
    for id in roster.ids(){
        let entity=roster.get(id).ok_or("pedestrian visual member absent")?;
        if !entity.renderable{continue;}
        let core=entity.core.as_ref().ok_or("pedestrian visual core absent")?;
        let Some(presentation)=entity.animation.as_ref().and_then(|a|a.presentation.as_ref()) else {continue};
        let model=&entity.resources.as_ref().ok_or("pedestrian visual resources absent")?.model;
        let mut colours=[[0.;4];2];
        for (i,key) in [0xdf76_d7d7_7385_7edb,0x1202_6e2e_ed18_cc8d].into_iter().enumerate(){
            if let Some(data)=model.try_vector_array_data(key,u32::from(core.colour_indices[i]),4)
                .map_err(|e|e.to_string())? {
                if data.bytes().len()<16{return Err("pedestrian colour float4 truncated".into());}
                for (lane,word) in colours[i].iter_mut().zip(data.bytes().chunks_exact(4)){
                    *lane=f32::from_be_bytes(word.try_into().expect("four-byte lane"));
                }
            }
        }
        let (irradiance,distances)=match lighting {
            Some(values)=>values,
            None=>{let values=living_world_lighting(vault)?;lighting=Some(values);values}
        };
        let velocity=match center_velocity {
            Some(value)=>value,
            None=>{
                let (_,value)=crate::simulation_assembly_backend::constraint_session::dmo_living_world::local_skater_census_centre(actor)
                    .map_err(|e|e.to_string())?;
                center_velocity=Some(value);value
            }
        };
        let bounds=presentation.palette.lock().map_err(|_|"pedestrian bounds palette lock")?
            .scene().level_bounds(0).map_err(|e|e.to_string())?.ok_or("pedestrian original bounds absent")?;
        let local_bounds=[std::array::from_fn(|i|bounds.minimum[i]),std::array::from_fn(|i|bounds.maximum[i])];
        let hierarchy=presentation.pose.hierarchy.as_ref()
            .ok_or("pedestrian LOD hierarchy absent")?.lock().map_err(|_|"pedestrian LOD hierarchy lock")?
            .character_hierarchy_snapshot().ok_or("pedestrian LOD hierarchy not parsed")?;
        let mut transform=std::array::from_fn(|i|core.transform[i/4][i%4]);transform[15]=1.;
        let placement=PedestrianPlacement::new(transform,local_bounds);
        visuals.push(SelectedPedestrianVisual{entity_id:id,recipe_id:core.recipe_id,model_key:core.model_key,
            spawn_opacity:if presentation.marquee.is_some(){0.}else{1.},
            palette:Arc::clone(&presentation.palette),colours,irradiance,material_lod_distance:distances[0],
            geometry_lod_squared:distances.map(|value|value*value),hierarchy,
            pose:presentation.pose.matrices().map_err(|e|format!("pedestrian LOD pose: {e:?}"))?.map(Arc::from),transform,
            local_bounds,bounds_center:placement.bounds_center,simulation_center_velocity:velocity,
            world_bounds:placement.world_bounds,
            opacity_parameters:crate::LivingWorldOpacityParameters::try_read(model).map_err(|e|e.to_string())?,
            opacity_settings:cvars.try_living_world_opacity_settings().map_err(|e|e.to_string())?,
            position:placement.position});
    }
    Ok(visuals)
}
