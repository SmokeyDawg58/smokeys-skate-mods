//! FullHandFacePedestrianACS8280d1c8 / UpdateFullHandFacePedestrians82809738.
use std::{convert::Infallible,sync::{Arc,Mutex}};
use crate::{actor_face_playback::{ActorFaceError,AnimationCommandSetup,AnimationPoseRef,FaceDatabase,FacePlaybackData,
    presentation::{PresentationAnimation,PresentationAnimationError}},
    actor_animation_construction::{initialization::{bind_pose::{BindPoseCommand,OwnedAnimationPoseBuffer,
        sqt_output::OwnedAnimationSqtBuffer},body_dispatch::tree_commands::TreeCommands},
        hands_face::sqt_jobs::{QueuedSqtProgram,matrix::{MatrixProgram,ProgramOutput}}},
    actor_animation_tree::eval::AnimationEvalCommands,
    actor_graph_construction::ActorGraphIntents,
    simulation_animation_startup::{SimulationAnimationStartup,AnimationStartupSource},
    character::palette::CharacterPalette,
};

pub(crate) struct MarqueeStartup{
    pub(crate) database:Arc<Mutex<FaceDatabase>>,
    pub(crate) playback:Arc<Mutex<FacePlaybackData>>,
    bind:OwnedAnimationSqtBuffer,
    face:Arc<Mutex<FacePlaybackData>>,
    face_buffers:[OwnedAnimationSqtBuffer;3],
    body_offset:usize,body_count:usize,prop_offset:usize,prop_source:usize,
    face_offset:usize,face_count:usize,
}
pub(crate) struct MarqueeEntity{
    pub(crate) body:OwnedAnimationSqtBuffer,
    face_pose:AnimationPoseRef,
}
impl MarqueeStartup{
    pub(crate) fn initialize<S:AnimationStartupSource>(simulation:&mut SimulationAnimationStartup,
        presentation:&mut PresentationAnimation,source:&mut S,regular:&Arc<Mutex<FaceDatabase>>)
        ->Result<Self,super::PedestrianPlaybackError<S::Error>>{
        let database=simulation.load_database(source,"data\\anim/MarqueeSkeletonPres.abin")
            .map_err(PresentationAnimationError::Startup)?;
        let setup=AnimationCommandSetup{capacities:[6,0,5,1,2],averages:[3.,0.,2.5,0.5,1.],max_nblend:0,
            fixed_sqt:0,fixed_matrix:1,mode:1,fetch_mode:0,category:2,characters:3,
            truncation_offset:0.,fetch_time_error:0.,trajectory:true,codec:0};
        let playback=presentation.register_playback::<S,Infallible>(source,&database,
            b"FullHandFacePedestrianACS",b"FullHandFacePedestrianACS",&setup)?;
        let hierarchy=database.lock().map_err(|_|ActorFaceError::Lock)?.parsed.as_ref()
            .ok_or(ActorFaceError::MissingRecord)?.hierarchy.clone();
        let bind_pose=playback.lock().map_err(|_|ActorFaceError::Lock)?
            .get_pose(ActorGraphIntents::native_name(b"rig_Tpose"))?.ok_or(ActorFaceError::MissingRecord)?;
        let bind=OwnedAnimationSqtBuffer::allocate_unknown(hierarchy.parents.len());
        bind.begin_extract()?;bind.extract(&bind_pose.raw_sqt_for(&hierarchy)?)?;bind.complete_sync()?;
        let face=Arc::clone(presentation.face.as_ref().ok_or(PresentationAnimationError::Missing("SkaterFace playback"))?);
        let face_bones=face.lock().map_err(|_|ActorFaceError::Lock)?.database.lock()
            .map_err(|_|ActorFaceError::Lock)?.parsed.as_ref().ok_or(ActorFaceError::MissingRecord)?.hierarchy.parents.len();
        let regular=regular.lock().map_err(|_|ActorFaceError::Lock)?.parsed.as_ref()
            .ok_or(ActorFaceError::MissingRecord)?.hierarchy.clone();
        let bone=|hierarchy:&skate2_formats::andale::AndaleHierarchy,name:&str|
            hierarchy.bone_names.iter().position(|n|n.eq_ignore_ascii_case(name)).ok_or(ActorFaceError::PartMapping);
        let body_offset=bone(&hierarchy,"Trajectory")?;
        let body_count=1+bone(&regular,"LeftHandProp")?-bone(&regular,"Trajectory")?;
        let prop_offset=bone(&hierarchy,"Right_Prop")?;let prop_source=bone(&regular,"RightHandProp")?;
        let face_offset=bone(&hierarchy,"Face")?;let face_count=1+bone(&hierarchy,"UpperLip")?-face_offset;
        Ok(Self{database,playback,bind,face,face_buffers:std::array::from_fn(|_|OwnedAnimationSqtBuffer::allocate_unknown(face_bones)),
            body_offset,body_count,prop_offset,prop_source,face_offset,face_count})
    }
    pub(crate) fn entity(&self,schema:&str)->Result<MarqueeEntity,String>{
        // CharacterEntity+20 is mSchemaName, not the recipe filename.
        let face=self.face.lock().map_err(|_|"Marquee face playback lock")?;
        let face_pose=match face.get_pose(ActorGraphIntents::native_name(schema.as_bytes())).map_err(|e|format!("{e:?}"))? {
            Some(pose)=>pose,
            None=>face.get_pose(ActorGraphIntents::native_name(b"rig_Tpose")).map_err(|e|format!("{e:?}"))?
                .ok_or("Marquee model/default face bind pose absent")?,
        };
        let bones=self.database.lock().map_err(|_|"Marquee hierarchy lock")?.parsed.as_ref()
            .ok_or("Marquee hierarchy absent")?.hierarchy.parents.len();
        let body=OwnedAnimationSqtBuffer::allocate_unknown(bones);
        body.begin_extract().map_err(|e|format!("{e:?}"))?;
        body.extract(&vec![skate2_formats::andale::AndaleSqt{scale:[1.;4],rotation:[0.,0.,0.,1.],translation:[0.;4]};bones])
            .map_err(|e|format!("{e:?}"))?;body.complete_sync().map_err(|e|format!("{e:?}"))?;
        Ok(MarqueeEntity{body,face_pose})
    }
    pub(crate) fn queue(&self,index:usize,entity:&MarqueeEntity,adjust:&OwnedAnimationSqtBuffer,
        palette:&Arc<Mutex<CharacterPalette>>,pose:&OwnedAnimationPoseBuffer)->Result<(),String>{
        let face=self.face_buffers.get(index).ok_or("Marquee presentation capacity3")?;
        let face_hierarchy=self.face.lock().map_err(|_|"Marquee face lock")?.database.lock()
            .map_err(|_|"Marquee face hierarchy lock")?.hierarchy_snapshot().ok_or("Marquee face hierarchy absent")?;
        face.begin_extract().map_err(|e|format!("{e:?}"))?;
        face.extract(&entity.face_pose.raw_sqt_for(&face_hierarchy).map_err(|e|format!("{e:?}"))?).map_err(|e|format!("{e:?}"))?;
        face.complete_sync().map_err(|e|format!("{e:?}"))?;
        let commands=Arc::clone(&self.playback.lock().map_err(|_|"Marquee playback lock")?.commands);
        let owner=Arc::clone(commands.lock().map_err(|_|"Marquee ACS lock")?.buffered.as_ref().ok_or("Marquee buffered owner absent")?);
        {let mut owner=owner.lock().map_err(|_|"Marquee buffered lock")?;
            if owner.in_command_set!=1||owner.in_commands!=0{return Err("Marquee command boundary".into());}
            owner.in_commands+=1;owner.nblend.begin_commands();}
        let mut tree=TreeCommands::new_stream(Arc::clone(&owner)).map_err(|e|format!("Marquee stream: {e:?}"))?;tree.enable_reuse();
        tree.set_external_sqt(adjust).map_err(|e|format!("Marquee adjustments: {e:?}"))?;
        tree.set_external_sqt(&self.bind).map_err(|e|format!("Marquee bind: {e:?}"))?;
        tree.replace_external_sqt(face,self.face_offset,self.face_count).map_err(|e|format!("Marquee face replace: {e:?}"))?;
        tree.add_sqt(true).map_err(|e|format!("Marquee AddSQT: {e:?}"))?;
        tree.replace_external_sqt(&entity.body,self.body_offset,self.body_count).map_err(|e|format!("Marquee body replace: {e:?}"))?;
        let prop=entity.body.try_subrange(self.prop_source..self.prop_source+1).map_err(|e|format!("{e:?}"))?;
        tree.replace_external_sqt(&prop,self.prop_offset,1).map_err(|e|format!("Marquee prop replace: {e:?}"))?;
        let sqt=tree.output().map_err(|e|format!("Marquee stack: {e:?}"))?;
        let matrix=u16::try_from(index).map_err(|_|"Marquee matrix index")?;
        let hierarchy=self.database.lock().map_err(|_|"Marquee hierarchy lock")?.hierarchy_snapshot()
            .ok_or("Marquee hierarchy absent")?;
        let output=MatrixProgram::new(Arc::clone(&palette.lock().map_err(|_|"Marquee palette lock")?
            .skinning().hierarchy_inverse_bind_pose),pose).map_err(|e|format!("Marquee matrix destination: {e:?}"))?;
        let mut owner=owner.lock().map_err(|_|"Marquee retain lock")?;
        owner.commands.extend([BindPoseCommand::SetExternalMatrix{matrix},BindPoseCommand::PoseMatMult{sqt,matrix},BindPoseCommand::ExtractMatrix{matrix}]);
        owner.retain_sqt_program(QueuedSqtProgram{commands:tree,output:ProgramOutput::Matrix(output),hierarchy})
            .map_err(|e|format!("Marquee EndCommands: {e:?}"))
    }
}
