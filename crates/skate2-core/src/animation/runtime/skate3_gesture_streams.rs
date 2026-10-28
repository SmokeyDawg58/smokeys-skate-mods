//! Authored S3 gesture finger/expression cues on the retained S2 hands/face ACS.
//! Body tree sampling and native clip key/token arithmetic share the same time.
use crate::{actor_face_playback::{FaceCommands,blend_pose_stream::{self,BlendPoseBuffer,BlendPoseTokenPool,ClipStream}},
    actor_graph_construction::ActorGraphIntents};
use skate2_formats::andale::{AndaleDatabase,AndaleSqt};
use skate3_core::animation::playback_tree::PoseCommand;
use std::{collections::BTreeMap,sync::{Arc,Mutex,OnceLock}};

const NATIVE_HAND_FRAMES:usize=61;
struct HandPoses {names:Vec<String>,parents:Vec<i32>,frames:Vec<Vec<AndaleSqt>>}
pub(super) struct GestureStreams {
    clips:BTreeMap<String,Vec<ClipStream>>,
    lengths:BTreeMap<String,f32>,
    hands:Arc<[HandPoses;2]>,
}
impl GestureStreams {
    pub(super) fn load(source:&crate::skate3_native::skater_animation::AnimationSource)->Result<Self,String> {
        let mut clips=BTreeMap::new();
        let mut lengths=BTreeMap::new();
        for bank in &source.banks().banks {
            for record in bank.records() {
                if record.header.type_id!=2 || !record.header.name.contains("_GSTR_") {continue;}
                let end=record.header.offset+record.header.size;
                let mut streams=blend_pose_stream::decode_clip_streams(&bank.bytes()[record.header.payload_offset..end])
                    .map_err(|e|format!("{} gesture streams: {e}",record.header.name))?;
                for stream in &mut streams {
                    if [b"LeftHand".as_slice(),b"RightHand"].into_iter().any(|name|stream.name==ActorGraphIntents::native_name(name)) {
                        for (_,pose) in &mut stream.keys {
                            if *pose>=62 {return Err("S3 gesture hand pose exceeds the authored62-frame palette".into());}
                            // Existing S2 fixed poses0..60 keep their original identity.
                            // Appended S3 poses61..122 fit the native7-bit token lane.
                            *pose+=NATIVE_HAND_FRAMES as u16;
                        }
                    }
                }
                if let skate_data::abin::RecordData::Clip(clip)=&record.data {
                    // Native clip clock length is (frames-1)/fps.
                    lengths.insert(record.header.name.clone(),(f32::from_bits(clip.frame_count_bits)-1.)/f32::from_bits(clip.fps_bits));
                }
                clips.insert(record.header.name.clone(),streams);
            }
        }
        static HANDS:OnceLock<Result<Arc<[HandPoses;2]>,String>>=OnceLock::new();
        let hands=HANDS.get_or_init(|| {
            let root=skate_data::asset_io::mount(crate::skate3_native::physics::player::PLAYER_ASSETS).map_err(|e|e.to_string())?;
            let mut result=Vec::new();
            for name in ["SkaterLeftHand","SkaterRightHand"] {
                let bytes=skate_data::asset_io::read(&root.join(format!("private/stock/data/anim/{name}.abin"))).map_err(|e|e.to_string())?;
                let db=AndaleDatabase::parse(&bytes).map_err(|e|e.to_string())?;
                let clip=db.clips.iter().find(|c|c.name.eq_ignore_ascii_case("Poses")).ok_or("S3 hand Poses clip")?;
                if clip.frame_count!=62. {return Err("S3 hand palette must contain its authored62 frames".into());}
                let frames=(0..62).map(|frame|clip.sample_raw_frame_sqt(&db.hierarchy,frame).map_err(|e|e.to_string()))
                    .collect::<Result<Vec<_>,_>>()?;
                result.push(HandPoses {names:db.hierarchy.bone_names.clone(),parents:db.hierarchy.parents.clone(),frames});
            }
            Ok(Arc::new(result.try_into().map_err(|_|"S3 hand pair")?))
        }).as_ref().map_err(Clone::clone)?.clone();
        Ok(Self {clips,lengths,hands})
    }
    pub(super) fn register(&self,left:&Arc<Mutex<FaceCommands>>,right:&Arc<Mutex<FaceCommands>>)->Result<(),String> {
        for (commands,poses) in [left,right].into_iter().zip(self.hands.iter()) {
            let mut commands=commands.lock().map_err(|_|"gesture hand command lock")?;
            if commands.fixed_frames.len()==NATIVE_HAND_FRAMES+poses.frames.len() {continue;}
            if commands.fixed_frames.len()!=NATIVE_HAND_FRAMES || commands.max_frames!=NATIVE_HAND_FRAMES {
                return Err("Gesture hand registration requires the original61 fixed S2 poses".into());
            }
            let db=commands.database.lock().map_err(|_|"gesture hand database lock")?;
            let hierarchy=db.parsed.as_ref().ok_or("gesture hand hierarchy")?;
            if hierarchy.hierarchy.parents!=poses.parents || hierarchy.hierarchy.bone_names.len()!=poses.names.len()
                || !hierarchy.hierarchy.bone_names.iter().zip(&poses.names).all(|(a,b)|a.eq_ignore_ascii_case(b)) {
                return Err("S3 gesture fingers differ from the actual S2 character hand hierarchy".into());
            }
            drop(db);
            commands.fixed_frames.extend(poses.frames.iter().cloned());
            commands.max_frames=commands.fixed_frames.len();
            crate::diagnostics::test_log::event("skate3_gesture_hand_palette",format_args!("native_frames=61 appended_frames=62 same_s2_owner=true"));
        }
        Ok(())
    }
    pub(super) fn apply(&self,channels:&[(Vec<PoseCommand>,f32)],destination:&mut BlendPoseBuffer,pool:&mut BlendPoseTokenPool)->Result<(),String> {
        for (commands,weight) in channels {
            if *weight<=0. {continue;}
            let mut stack:Vec<BlendPoseBuffer>=Vec::new();
            for command in commands {
                match command {
                    PoseCommand::Clip {name,time,..}=>{
                        let streams=self.clips.get(name).ok_or_else(||format!("Missing original gesture clip stream {name}"))?;
                        let mut buffer=pool.buffer(1).map_err(|e|e.to_string())?;
                        // Every authored key has normalized8191 timing. The same
                        // stock clock length used by body playback is retained below.
                        let length=self.clip_length(name)?;
                        blend_pose_stream::eval_clip_streams(streams,*time,length,&mut buffer,pool).map_err(|e|e.to_string())?;
                        stack.push(buffer);
                    }
                    PoseCommand::Pose {..}=>stack.push(pool.buffer(1).map_err(|e|e.to_string())?),
                    PoseCommand::Add {..}=>{stack.pop().ok_or("Gesture stream Add underflow")?;},
                    PoseCommand::Mirror {..}=>stack.last_mut().ok_or("Gesture stream Mirror underflow")?.mirror_streams(),
                    PoseCommand::Blend {weight}=>{
                        let other=stack.pop().ok_or("Gesture stream Blend underflow")?;
                        let base=stack.last_mut().ok_or("Gesture stream Blend base")?;
                        pool.blend(base,&other,*weight).map_err(|e|e.to_string())?;
                    }
                    PoseCommand::WeightedBlend {weights}=>{
                        let start=stack.len().checked_sub(weights.len()).ok_or("Gesture stream weighted underflow")?;
                        let children=stack.split_off(start);
                        let mut combined=pool.buffer(1).map_err(|e|e.to_string())?;
                        for child in children {pool.append_streams(&mut combined,&child).map_err(|e|e.to_string())?;}
                        for weight in weights {for stream in 0..3 {pool.push_coefficient(&mut combined,stream,*weight).map_err(|e|e.to_string())?;}}
                        for stream in 0..3 {pool.push_blend(&mut combined,stream,weights.len()).map_err(|e|e.to_string())?;}
                        stack.push(combined);
                    }
                    PoseCommand::ChannelBlend {..}=>return Err("Nested gesture stream channel terminator".into()),
                }
            }
            if stack.len()!=1 {return Err(format!("Gesture stream stack retained{} results",stack.len()));}
            pool.blend(destination,&stack.pop().unwrap(),*weight).map_err(|e|e.to_string())?;
        }
        Ok(())
    }
    fn clip_length(&self,name:&str)->Result<f32,String> {
        self.lengths.get(name).copied().ok_or_else(||format!("Missing gesture length {name}"))
    }
}
