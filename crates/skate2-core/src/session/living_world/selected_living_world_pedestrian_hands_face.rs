//! Regular pedestrian token replay: native hand/face manager828115c0.
use std::{convert::Infallible,sync::{Arc,Mutex}};
use crate::{actor_face_playback::{ActorFaceError,AnimationCommandSetup,FaceCommands,FaceDatabase,
    presentation::{PresentationAnimation,PresentationAnimationError}},
    actor_animation_tree::{AnimationTreePool,AnimationNodeHandle,AnimationNodeKind},
    actor_animation_construction::{hands_face::{sqt_jobs::{QueuedSqtProgram,matrix::ProgramOutput}},
        initialization::{bind_pose::{BindPoseCommand,sqt_output::OwnedAnimationSqtBuffer},
            body_dispatch::tree_commands::TreeCommands}},
    actor_animation_tree::eval::AnimationEvalCommands,
    simulation_animation_startup::{SimulationAnimationStartup,AnimationStartupSource},
    actor_graph_construction::ActorGraphIntents,
};

pub(crate) struct RegularHandFace {
    pub(crate) hands:Mutex<PoseSystem>,
    pub(crate) face:Mutex<PoseSystem>,
}
pub(crate) struct PoseSystem {
    pub(crate) commands:Arc<Mutex<FaceCommands>>,
    pool:AnimationTreePool,
    clips:Vec<AnimationNodeHandle>,
    ranges:Vec<std::ops::Range<usize>>,
    hierarchy:Arc<skate2_formats::andale::AndaleHierarchy>,
}
impl RegularHandFace {
    /// InitHandAnimSys8280d5d0 / InitFaceAnimSys8280d8c0, descriptor82feff10.
    pub(crate) fn initialize<S:AnimationStartupSource>(simulation:&mut SimulationAnimationStartup,
        presentation:&mut PresentationAnimation,source:&mut S,full:&Arc<Mutex<FaceDatabase>>)
        ->Result<Self,super::PedestrianPlaybackError<S::Error>> {
        let left=simulation.load_database(source,"data\\anim/PedestrianLeftHandRegular.abin")
            .map_err(PresentationAnimationError::Startup)?;
        // Both paths in the original regular descriptor name this same file.
        let right=simulation.load_database(source,"data\\anim/PedestrianLeftHandRegular.abin")
            .map_err(PresentationAnimationError::Startup)?;
        let hands=PoseSystem::initialize(presentation,source,full,&[left,right],
            b"PedestrianHandACS4BoneHands6BoneFace",b"pose",&[b"LeftHand",b"RightHand"],20,0.5)?;
        let face=simulation.load_database(source,"data\\anim/PedestrianFaceRegular.abin")
            .map_err(PresentationAnimationError::Startup)?;
        let face=PoseSystem::initialize(presentation,source,full,&[face],
            b"PedestrianFaceACS4BoneHands6BoneFace",b"pose",&[b"Face"],12,0.)?;
        Ok(Self{hands:Mutex::new(hands),face:Mutex::new(face)})
    }
    /// Second descriptor82feff38; same manager/evaluator, full hierarchy ranges.
    pub(crate) fn initialize_marquee<S:AnimationStartupSource>(simulation:&mut SimulationAnimationStartup,
        presentation:&mut PresentationAnimation,source:&mut S,full:&Arc<Mutex<FaceDatabase>>)
        ->Result<Self,super::PedestrianPlaybackError<S::Error>>{
        let left=simulation.load_database(source,"data\\anim/SkaterLeftHand.abin").map_err(PresentationAnimationError::Startup)?;
        let right=simulation.load_database(source,"data\\anim/SkaterRightHand.abin").map_err(PresentationAnimationError::Startup)?;
        let hands=PoseSystem::initialize(presentation,source,full,&[left,right],
            b"PedestrianHandACS17BoneHands48BoneFace",b"poses",&[b"LeftHand",b"RightHand"],20,0.5)?;
        let face=simulation.load_database(source,"data\\anim/PedestrianFaceFull.abin").map_err(PresentationAnimationError::Startup)?;
        let face=PoseSystem::initialize(presentation,source,full,&[face],
            b"PedestrianFaceACS17BoneHands48BoneFace",b"SkaterRig_Hand_Ctrl",&[b"Face_Offset0"],12,0.)?;
        Ok(Self{hands:Mutex::new(hands),face:Mutex::new(face)})
    }
}
impl PoseSystem {
    fn initialize<S:AnimationStartupSource>(presentation:&mut PresentationAnimation,source:&mut S,
        full:&Arc<Mutex<FaceDatabase>>,databases:&[Arc<Mutex<FaceDatabase>>],name:&[u8],clip_name:&[u8],
        parts:&[&[u8]],characters:u32,truncation_offset:f32)
        ->Result<Self,super::PedestrianPlaybackError<S::Error>> {
        let setup=AnimationCommandSetup{capacities:[12,6,0,0,0],averages:[4.,2.,0.,0.,0.],
            max_nblend:0,fixed_sqt:0,fixed_matrix:0,mode:1,fetch_mode:0,category:2,
            characters,truncation_offset,fetch_time_error:0.,trajectory:false,codec:0};
        let playback=presentation.register_playback::<S,Infallible>(source,&databases[0],name,name,&setup)?;
        let commands=Arc::clone(&playback.lock().map_err(|_|ActorFaceError::Lock)?.commands);
        let hierarchy=databases[0].lock().map_err(|_|ActorFaceError::Lock)?.hierarchy_snapshot()
            .ok_or(ActorFaceError::MissingRecord)?;
        let body=full.lock().map_err(|_|ActorFaceError::Lock)?.parsed.as_ref()
            .ok_or(ActorFaceError::MissingRecord)?.hierarchy.clone();
        let mut pool=AnimationTreePool::new(16*64,16);
        let mut clips=Vec::new();let mut ranges=Vec::new();
        for (database,part) in databases.iter().zip(parts) {
            let playback=crate::actor_face_playback::FacePlaybackData{database:Arc::clone(database),
                databases:vec![Arc::clone(database)],commands:Arc::clone(&commands)};
            let clip=playback.try_construct_animation_tree(&mut pool,&ActorGraphIntents::native_name(clip_name),&[])
                .map_err(|_|PresentationAnimationError::Missing("pedestrian pose clip"))?
                .ok_or(PresentationAnimationError::Missing("pedestrian pose clip"))?;
            let name=ActorGraphIntents::native_name(part);
            let part=body.parts.iter().find(|p|p.packed_name==name).ok_or(ActorFaceError::PartMapping)?;
            let offset=usize::try_from(part.bone_offset).map_err(|_|ActorFaceError::PartMapping)?;
            ranges.push(offset..offset+hierarchy.parents.len());clips.push(clip);
        }
        Ok(Self{commands,pool,clips,ranges,hierarchy})
    }
    /// IsValidBlendPoseBuffer82805fc0. Both original limits are6 (82feff6c).
    pub(crate) fn valid(&self,index:usize,tokens:&[u32])->Result<bool,String>{
        let AnimationNodeKind::Clip(clip)=&self.pool.node(&self.clips[index]).map_err(|e|e.to_string())?.kind
            else{return Err("pedestrian pose source is not a clip".into());};
        if tokens.is_empty(){return Ok(false);}
        let mut depth=0;let mut fetches=0;let mut blends=0;
        for &token in tokens{
            if token&0xc000_0000==0{
                if ((token>>23)&127) as f32>=clip.frames{return Ok(false);}
                depth+=1;fetches+=1;
            }else{
                let coefficient=crate::actor_face_playback::blend_pose_stream::decode_coefficient(token);
                if coefficient>1.||coefficient<0.{return Ok(false);}
                if depth>=2{depth-=1;blends+=1;}
            }
        }
        Ok(fetches<=6&&blends<=6&&depth==1)
    }
    /// EvalBlendPose828078f0 intentionally treats every non-pose token as
    /// a pair blend when two values exist; it is not the Actor NBlend decoder.
    pub(crate) fn queue(&mut self,index:usize,tokens:&[u32],output:&OwnedAnimationSqtBuffer)
        ->Result<(),String>{
        let owner=Arc::clone(self.commands.lock().map_err(|_|"hand/face ACS lock")?
            .buffered.as_ref().ok_or("hand/face buffered ACS absent")?);
        let mut tree=TreeCommands::new_stream(Arc::clone(&owner)).map_err(|e|format!("hand/face stream: {e:?}"))?;tree.enable_reuse();
        {let mut owner=owner.lock().map_err(|_|"hand/face buffered lock")?;
            if owner.in_command_set==0||owner.in_commands!=0{return Err("hand/face command boundary".into());}
            owner.in_commands+=1;owner.nblend.begin_commands();}
        let mut depth=0;
        for &token in tokens{
            if token&0xc000_0000==0{
                let handle=&self.clips[index];
                let AnimationNodeKind::Clip(clip)=&self.pool.node(handle).map_err(|e|e.to_string())?.kind
                    else{return Err("hand/face pose clip absent".into());};
                // Clip.SetFrame825ecdb8 divides by N-1 before multiplying length.
                let ratio=((token>>23)&127) as f32/(clip.frames-1.);
                let time=self.pool.length(handle).map_err(|e|e.to_string())?*ratio;
                self.pool.set_time(handle,time).map_err(|e|e.to_string())?;
                let plan=self.pool.prepare_clip_fetch(handle,&self.commands,1).map_err(|e|e.to_string())?;
                tree.fetch(plan).map_err(|e|format!("hand/face fetch: {e:?}"))?;depth+=1;
            }else if depth>=2{
                tree.blend(crate::actor_face_playback::blend_pose_stream::decode_coefficient(token))
                    .map_err(|e|format!("hand/face blend: {e:?}"))?;depth-=1;
            }
        }
        if depth!=1{return Err("hand/face replay token stack".into());}
        let destination=output.try_subrange(self.ranges[index].clone()).map_err(|e|format!("{e:?}"))?;
        let sqt=tree.select_stream_extraction_root().map_err(|e|format!("hand/face root: {e:?}"))?;
        owner.lock().map_err(|_|"hand/face extract lock")?.commands.push(BindPoseCommand::ExtractSqt{sqt});
        destination.begin_shared_extract().map_err(|e|format!("{e:?}"))?;
        owner.lock().map_err(|_|"hand/face retain lock")?.retain_sqt_program(QueuedSqtProgram{
            commands:tree,output:ProgramOutput::Sqt(destination),hierarchy:self.hierarchy.clone()})
            .map_err(|e|format!("hand/face queue: {e:?}"))
    }
}

/// FillBlendPoseStream8280b410 copies authored tokens with strict capacity300.
/// Source submission order is right hand, left hand, face for each entity.
pub(crate) fn replay_streams(buffer:&crate::actor_face_playback::blend_pose_stream::BlendPoseBuffer,
    pool:&crate::actor_face_playback::blend_pose_stream::BlendPoseTokenPool,count:&mut usize)
    ->Result<[Vec<u32>;3],String>{
    let mut result=std::array::from_fn(|_|Vec::new());
    for (index,name) in [(1,b"RightHand".as_slice()),(0,b"LeftHand".as_slice()),(2,b"Face".as_slice())]{
        if let Some(stream)=buffer.stream_index(&ActorGraphIntents::native_name(name)){
            let tokens=pool.tokens(buffer,stream).map_err(|e|e.to_string())?;
            if !tokens.is_empty()&&*count+tokens.len()<300{
                result[index]=tokens.to_vec();*count+=tokens.len();
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_animation_construction::initialization::body_dispatch::tree_commands::execute_fetch;

    /// `UpdateHands` 0x8280e0a0 evaluates the right-hand clip on the one hand ACS, built on the
    /// left-hand poses hierarchy (`InitHandAnimSys` 0x8280d5d0). The marquee databases name
    /// their parts `LeftHand` and `RightHand`, so the right-hand fetch matches no part and
    /// `BatchFetch` 0x82d737d0 emits its null-source entry: the identity over the hierarchy.
    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn the_marquee_right_hand_pose_fetches_the_null_source_entry() {
        let (_manager, cvars) = crate::game_input_recognizer_corpus::original_input_cvars();
        let (mut simulation, mut source) =
            crate::simulation_animation_startup::corpus::load_original_startup_with_cvars_and_source(
                &cvars,
            );
        let mut presentation = PresentationAnimation::new(simulation.managers.clone());
        let full = simulation
            .load_database(&mut source, "data\\anim/MarqueeSkeletonPres.abin")
            .unwrap();
        let system =
            RegularHandFace::initialize_marquee(&mut simulation, &mut presentation, &mut source, &full)
                .unwrap();
        let hands = system.hands.lock().unwrap();
        let fetch = |index: usize| {
            hands
                .pool
                .prepare_clip_fetch(&hands.clips[index], &hands.commands, 1)
                .unwrap()
        };
        let (left, right) = (fetch(0), fetch(1));
        assert_eq!((left.parts.len(), left.extract_count()), (1, 1));
        assert_eq!((right.parts.len(), right.extract_count()), (0, 1));
        let pose = execute_fetch(&right).unwrap();
        assert_eq!(pose.len(), 17);
        assert!(pose.iter().all(|sqt| sqt.scale == [1.; 4]
            && sqt.rotation == [0., 0., 0., 1.]
            && sqt.translation == [0., 0., 0., 1.]));
    }
}
