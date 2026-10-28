//! Pedestrian.UpdateTargetTransform82e7f660 (LWBasic receiver is complete+32).
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    physical_player_assembly::PhysicalAssemblyWorld,input_cvar_runtime::InputCvarReferences,
    reckoning_numeric::{HostReckoningNumeric,ReckoningNumeric,try_look_at_up_eye,
        try_normalize_unsafe,try_acos,cross_full4},
    reckoning_sin_cos::{HostReckoningSinCosNumeric,try_sin_cos},
    animation::{AnimationMatrix,compose_pose_matrix}};
type Matrix=[[f32;4];4];
fn flat(m:Matrix)->AnimationMatrix {std::array::from_fn(|i|m[i/4][i%4])}
fn matrix(m:AnimationMatrix)->Matrix {std::array::from_fn(|i|std::array::from_fn(|j|m[i*4+j]))}
fn valid(m:Matrix)->Result<(),String>{
    if m.iter().any(|axis|axis[..3].iter().any(|v|v.is_nan())){
        Err("pedestrian target transform native IsValid assertion".into())
    }else{Ok(())}
}

impl PedestrianConstruction {
    pub(crate) fn update_animation_target_transform(&mut self,world:&PhysicalAssemblyWorld,
        cvars:&InputCvarReferences,dt:f32,
        dmos:&mut crate::living_world_grind_services::dmo_objects::DmoObjectMan)->Result<(),String>{
        // Actual vslotb4 -> LWBasicEntity.IsRenderable82d1ceb8.
        if !self.renderable{return Ok(());}
        let initial=self.core.as_ref().ok_or("pedestrian target core absent")?.target_transform;
        let delta=self.animation.as_ref().ok_or("pedestrian target animation absent")?
            .sim_lod_pose.as_ref().ok_or("pedestrian target pose absent")?.root_delta()?;
        valid(matrix(delta))?;
        let mut target=matrix(compose_pose_matrix(flat(initial),delta));
        valid(target)?;
        let animated_y=target[3][1];
        let collision=self.collision.as_ref().ok_or("pedestrian target collision absent")?;
        let ground_offset=collision.colliding
            && self.collision_info.force==crate::pedestrian_graph::colliding::CollisionForce::Knockdown;
        let state=self.state.as_ref().ok_or("pedestrian target state absent")?;
        let sliding=self.adjust_world_collisions && state.flags[0]&2==0 && state.flags[2]&0x40==0;
        let spatial=self.spatial_adjust.as_mut().ok_or("pedestrian spatial adjustment absent")?;
        spatial.update(dt,&mut target,collision,world,ground_offset,sliding,self.nav_rig.as_ref(),
            &cvars.try_pedestrian_spatial_adjust_tuning().map_err(|e|format!("{e:?}"))?,
            &mut HostReckoningNumeric)?;
        valid(target)?;
        reorient_to_global_up(&mut target,spatial.up_vector)?;
        valid(target)?;
        let stage=self.intent_queue.as_ref().ok_or("pedestrian target intent queue absent")?.active_stage()?;
        if stage.rotate_to_target {
            rotate_to_target(&mut target,stage.target_direction,stage.max_rotate_angle_degrees,
                cvars.try_pedestrian_min_rotate_to_target_angle().map_err(|e|format!("{e:?}"))?)?;
        }
        if cvars.try_pedestrian_plugin_shift_with_dmo().map_err(|e|format!("{e:?}"))? {
            let state=self.state.as_mut().expect("checked state");
            if state.flags[3]&1!=0 {
                let shift=state.dmo_shift.ok_or("pedestrian DMO shift flag without producer")?;
                for i in 0..3 {target[3][i]+=shift[i];}
                state.dmo_shift=Some([0.;3]);
                state.flags[3]&=!1;
                if spatial.position_locked {spatial.locked_position=target[3];}
            }
        }
        // Optional height diagnosis only. Keep the source ground smoothing and
        // authored root motion intact; do not hide an unexplained lift by
        // snapping the render pose to the pavement.
        static TRACE_HEIGHT:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
        static HEIGHT_SAMPLES:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
        static HEIGHT_REPORTS:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
        if *TRACE_HEIGHT.get_or_init(||std::env::var_os("SKATE2_TRACE_PEDESTRIAN_HEIGHT").is_some()) {
            let sample=HEIGHT_SAMPLES.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
            let gap=target[3][1]-spatial.ground_position[1];
            if (sample%600==0 || gap.abs()>0.08 || delta[13].abs()>0.02)
                && HEIGHT_REPORTS.fetch_add(1,std::sync::atomic::Ordering::Relaxed)<8192 {
                std::eprintln!("pedestrian height id={} sample={} position={:?} prior_y={} root_dy={} animated_y={} ground={:?} gap={} valid={} jump={} pullback={} locked={} slide={} collision={}",
                    self.id,sample,target[3],initial[3][1],delta[13],animated_y,spatial.ground_position,gap,
                    spatial.has_valid_ground_position,spatial.jump.is_jumping,spatial.pulling_back,
                    spatial.position_locked,spatial.sliding_to_position,collision.colliding);
            }
        }
        self.core.as_mut().expect("checked core").set_target_transform::<std::convert::Infallible>(target)
            .map_err(|e|format!("pedestrian SetTargetTransform: {e:?}"))?;
        self.animation.as_mut().expect("checked animation").update_sim_lod_global(flat(target))?;
        // UpdateHandPropTargetTransform82e7b2b0 performs no work without BOTH
        // the retained object and HasHandProp bit. Never synthesize a grip pose.
        self.update_hand_prop_target(dmos,cvars)?;
        Ok(())
    }

    /// SpawnAndSetHandProp uses the identical native hand/grip reorientation.
    pub(crate) fn hand_prop_spawn_transform(&self,grip:AnimationMatrix)->Result<AnimationMatrix,String>{
        if !self.renderable{return Err("hand prop spawn renderability assertion".into());}
        let hand=self.animation.as_ref().ok_or("hand prop animation absent")?
            .sim_lod_bone_global.as_ref().ok_or("hand prop global pose absent")?.matrices[3];
        valid(hand)?;
        let target=reorient_hand(hand,matrix(grip),self.hand_prop_attributes.as_ref())?;
        valid(target)?;
        Ok(flat(target))
    }

    /// UpdateHandPropTargetTransform82e7b2b0; actual right-hand bone and DMO.
    fn update_hand_prop_target(&mut self,dmos:&mut crate::living_world_grind_services::dmo_objects::DmoObjectMan,
        cvars:&InputCvarReferences)->Result<(),String>{
        let Some(id)=self.held_hand_prop else{return Ok(());};
        if self.state.as_ref().ok_or("hand-prop state absent")?.flags[2]&2==0{return Ok(());}
        if !self.renderable{return Err("held hand-prop renderability assertion".into());}
        let hand=self.animation.as_ref().ok_or("hand-prop animation absent")?
            .sim_lod_bone_global.as_ref().ok_or("hand-prop global pose absent")?.matrices[3];
        let object=dmos.object_mut(id).ok_or("held hand-prop DMO absent")?;
        let grip=object.model.hand_hotpoint_transform.ok_or("held hand-prop authored grip absent")?;
        let mut target=reorient_hand(hand,matrix(grip),self.hand_prop_attributes.as_ref())?;
        if cvars.try_pedestrian_hand_prop_position_prediction().map_err(|e|format!("{e:?}"))? {
            if self.last_frame_hand_position[..3].iter().any(|v|*v!=0.) {
                for i in 0..4 {target[3][i]+=hand[3][i]-self.last_frame_hand_position[i];}
            }
            self.last_frame_hand_position=hand[3];
        }
        // IDynamicObject adjusted receiver at+8c, actual transformtype110c.
        if !object.is_set_as_hand_prop(){return Err("SetHandPropTargetTransform type assertion".into());}
        valid(target)?;
        object.target_transform=flat(target);
        Ok(())
    }
}

/// ReorientHandTransformToHandPropBase82e7af78: the hand bone, then the authored
/// Euler/translation offset, then the inverse grip. The result keeps the hand's
/// axes (a rotation); it stores no axis exchange.
fn reorient_hand(hand:Matrix,grip:Matrix,attributes:Option<&crate::attribute_runtime::AttributeRuntimeInstance>)
    ->Result<Matrix,String>{
    let mut offset=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.;4]];
    if let Some(attributes)=attributes {
        let bytes=attributes.try_cached_layout_field(0,32).map_err(|e|e.to_string())?
            .ok_or("hand-prop authored offset layout absent")?;
        let read=|i:usize|f32::from_bits(u32::from_be_bytes(bytes.bytes()[i..i+4].try_into().expect("bounded layout")));
        let radians=f32::from_bits(0x3c8e_fa35);
        let(sx,cx)=try_sin_cos(read(0)*radians,&mut HostReckoningSinCosNumeric).map_err(|e|format!("{e:?}"))?;
        let(sy,cy)=try_sin_cos(read(4)*radians,&mut HostReckoningSinCosNumeric).map_err(|e|format!("{e:?}"))?;
        let(sz,cz)=try_sin_cos(read(8)*radians,&mut HostReckoningSinCosNumeric).map_err(|e|format!("{e:?}"))?;
        let xx=cy*cz;let yx=sy*(sx*cz)-cx*sz;let zx=sy.mul_add(cx*cz,sx*sz);
        offset=[[xx,cy*sz,-sy,xx],[yx,sy.mul_add(sx*sz,cx*cz),cy*sx,yx],
            [zx,sy*(cx*sz)-sx*cz,cy*cx,zx],std::array::from_fn(|i|read(16+i*4))];
    }
    let mut inverse:Matrix=std::array::from_fn(|i|std::array::from_fn(|j|
        if i<3&&j<3 {grip[j][i]}else{0.}));
    for i in 0..4 {
        let z=(-grip[3][2])*inverse[2][i];
        let yz=(-grip[3][1]).mul_add(inverse[1][i],z);
        inverse[3][i]=(-grip[3][0]).mul_add(inverse[0][i],yz);
    }
    let local=compose_pose_matrix(flat(offset),flat(inverse));
    Ok(matrix(compose_pose_matrix(flat(hand),local)))
}

/// ReorientToGlobalUp82e4e5c8: LookAt(position+up, oldForward, position),
/// negate resulting X and swap Y/Z. Translation remains the original value.
fn reorient_to_global_up(target:&mut Matrix,up:[f32;4])->Result<(),String>{
    let look=std::array::from_fn(|i|target[3][i]+up[i]);
    let oriented=try_look_at_up_eye(look,target[2],target[3],&mut HostReckoningNumeric)
        .map_err(|e|format!("pedestrian global up: {e:?}"))?;
    valid(oriented)?;
    target[0]=oriented[0].map(|v|-v);
    target[1]=oriented[2];
    target[2]=oriented[1];
    Ok(())
}

/// RotateToTarget82e56640. Shared rsqrt/ACos/SinCos and axis matrix services
/// preserve the original two normalizations and signed angular threshold.
fn rotate_to_target(current:&mut Matrix,target:[f32;4],max_degrees:f32,min_degrees:f32)
    ->Result<(),String>{
    let mut numeric=HostReckoningNumeric;
    let squared=numeric.dot3(target,target).map_err(|e|format!("{e:?}"))?;
    let epsilon=f32::from_bits(0x3586_37bd);
    if 0.>=squared-epsilon*epsilon{return Ok(());}
    let target=try_normalize_unsafe(target,&mut numeric).map_err(|e|format!("{e:?}"))?;
    let up=current[1];
    let dot=numeric.dot3(target,up).map_err(|e|format!("{e:?}"))?;
    let projected=std::array::from_fn::<_,4,_>(|i|target[i]-up[i]*dot);
    let direction=try_normalize_unsafe(projected,&mut numeric).map_err(|e|format!("{e:?}"))?;
    let difference=std::array::from_fn(|i|current[2][i]-direction[i]);
    let difference=numeric.dot3(difference,difference).map_err(|e|format!("{e:?}"))?;
    let epsilon=f32::from_bits(0x3a83_126f);
    if 0.>=difference-epsilon*epsilon{return Ok(());}
    let dot=numeric.dot3(current[2],direction).map_err(|e|format!("{e:?}"))?;
    let lower=if dot< -1. {-1.}else{dot};
    let bounded=if lower>1. {1.}else{lower};
    let radians_per_degree=f32::from_bits(0x3c8e_fa35);
    let angle=try_acos(bounded,&mut numeric).map_err(|e|format!("{e:?}"))?;
    let max=max_degrees*radians_per_degree;
    let mut angle=if angle<max {angle}else{max};
    if cross_full4(current[2],direction)[1]<0. {angle *= -1.;}
    if angle.abs()>min_degrees*radians_per_degree {
        let(sine,cosine)=try_sin_cos(angle,&mut HostReckoningSinCosNumeric)
            .map_err(|e|format!("{e:?}"))?;
        let rotation=crate::skeleton_ik_state::external::normal_rotation([up[0],up[1],up[2]],sine,cosine);
        let columns=rotation.columns();
        let rotation=std::array::from_fn(|i|std::array::from_fn(|j|
            if j<3 {columns[i].xyz[j]} else {columns[i].w.expect("known axis rotation")}));
        *current=matrix(compose_pose_matrix(flat(*current),flat(rotation)));
    }
    valid(*current)
}
