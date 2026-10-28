//! PostAdjust.Update82e7cb70; the six retained records feed the existing PRES replay.
use super::{PostAdjust, post_adjust_codec};
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, try_length, try_unsigned_angle, cross_full4},
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos}};
type Vector = [f32; 4];
type Matrix = [Vector; 4];
const RADIANS: f32 = f32::from_bits(0x3c8e_fa35);

impl PedestrianConstruction {
    /// LWPedestrianMan.UpdateAnimPostAdjust82e83708 gates. Skipped entities
    /// preserve their previous compressed records and procedural angle state.
    pub(crate) fn update_animation_post_adjust(&mut self, dt:f32,
        cvars:&crate::input_cvar_runtime::InputCvarReferences)->Result<(),String>{
        if !self.renderable { return Ok(()); }
        let animation=self.animation.as_ref().ok_or("post-adjust animation absent")?;
        let head=self.head_tracking.as_ref().ok_or("post-adjust head tracking absent")?;
        if !head.is_head_tracking(animation.ai_input.locomotion_turn_complete)? { return Ok(()); }
        let adjust=self.post_adjust.as_mut().ok_or("post-adjust owner absent")?;
        // The disabled cvar branch never reads either pose or target matrices.
        let enabled=cvars.try_pedestrian_enable_head_tracking().map_err(|e|format!("post-adjust cvar: {e:?}"))?;
        if !enabled { return adjust.disable(); }
        let pose=animation.sim_lod_pose.as_ref().ok_or("post-adjust SIM pose absent")?
            .pose.matrices().map_err(|e|format!("post-adjust SIM pose: {e:?}"))?
            .ok_or("post-adjust SIM pose before Sync")?;
        let bone=adjust.adjustments[0].ok_or("post-adjust head record unwritten")?.bone as i8;
        let pose=pose.get(usize::try_from(bone).map_err(|_|"post-adjust native negative head bone")?)
            .ok_or("post-adjust head outside SIM pose")?;
        let pose=std::array::from_fn(|i|std::array::from_fn(|j|pose[i*4+j]));
        adjust.update(dt,self.core.as_ref().ok_or("post-adjust core absent")?.target_transform,
            pose,head.resolved_target)
    }
}

impl PostAdjust {
    ///82e7d4e0..d5d4 resets compressed output only, keeping yaw/pitch history.
    fn disable(&mut self)->Result<(),String>{
        let identity=post_adjust_codec::compress([0.,0.,0.,1.])?;
        for record in &mut self.adjustments {
            record.as_mut().ok_or("post-adjust bone record unwritten")?.axis_angle=identity;
        }
        Ok(())
    }
    fn layout_float(&self,offset:usize)->Result<f32,String>{
        let value=self.attributes.try_cached_layout_field(offset,4).map_err(|e|e.to_string())?
            .ok_or("post-adjust layout absent")?;
        Ok(f32::from_be_bytes(value.bytes().try_into().map_err(|_|"post-adjust layout float width")?))
    }
    fn attribute_float(&self,key:u64)->Result<f32,String>{
        Ok(f32::from_bits(self.attributes.try_scalar_word(key,0).map_err(|e|e.to_string())?.unwrap_or(0)))
    }
    fn update(&mut self,dt:f32,transform:Matrix,head_pose:Matrix,target:Vector)->Result<(),String>{
        let displacement=std::array::from_fn(|i|target[i]-transform[3][i]);
        let horizontal=flat(displacement);
        if try_length(horizontal,&mut HostReckoningNumeric).map_err(|e|format!("post-adjust length: {e:?}"))?>0.1 {
            let forward=flat(transform[2]);
            let base_cross=cross_full4(forward,horizontal);
            let base_angle=angle(horizontal,forward)?;
            let (s,c)=sin_cos(self.head_yaw)?;
            // Rotate the actual head Y basis by current global-Y yaw, then
            // by target orientation. Native intentionally uses pre-update yaw.
            let yaw=crate::physical_player_grind_air_adjust::update::axis_angle_rows([0.,1.,0.,0.],s,c);
            let facing=flat(direction(&transform,direction(&yaw,head_pose[1])));
            let cross=cross_full4(facing,horizontal);
            let mut remaining=angle(horizontal,facing)?;
            let mut step=remaining.min((self.layout_float(0)?*RADIANS)*dt)*sign(cross[1]);
            if base_angle>self.max_head_yaw.ok_or("post-adjust maximum head yaw unwritten")? {
                step=remaining.min((self.attribute_float(0xf238_88ec_33ea_efb9)?*RADIANS)*dt)*sign(base_cross[1]);
            }
            let reverse=step*self.head_yaw<0.;
            for slot in 0..6 {
                let index=if reverse {5-slot}else{slot};
                if remaining<=0. {break;}
                if reverse && step*self.bone_yaws[index]>=0. {continue;}
                let maximum=self.max_bone_yaws[index].ok_or("post-adjust bone maximum unwritten")?;
                self.bone_yaws[index]=(self.bone_yaws[index]+step).max(-maximum).min(maximum);
                remaining-=step;
                if maximum>self.bone_yaws[index].abs() {break;}
            }
        }
        // Pitch uses a local Z rotation and the actual pose basis; the source
        // subtracts the pose translation directly from world displacement.
        let (s,c)=sin_cos(self.head_pitch)?;
        let pitch=crate::physical_player_grind_air_adjust::update::axis_angle_rows([0.,0.,1.,0.],s,c);
        let pitched=direction(&transform,direction(&head_pose,pitch[1]));
        let current_pitch=angle(pitched,flat(pitched))?*(-sign(pitched[1]));
        let to_head=std::array::from_fn(|i|displacement[i]-head_pose[3][i]);
        let desired_pitch=angle(flat(to_head),to_head)?*(-sign(to_head[1]));
        let delta=desired_pitch-current_pitch;
        let step=delta.abs().min((self.layout_float(4)?*RADIANS)*dt)*sign(delta);
        let up=self.attribute_float(0x5925_36a4_6995_94d9)?;
        let down=self.layout_float(8)?;
        let maximum=self.max_head_pitch.ok_or("post-adjust maximum head pitch unwritten")?;
        self.head_pitch=(self.head_pitch+step).max((-1.*down)*maximum).min(maximum*up);
        self.head_yaw=0.;
        for index in 0..6 {
            let (sp,cp)=if index==0 {sin_cos(self.head_pitch*0.5)?}else{(0.,1.)};
            let (sy,cy)=sin_cos(self.bone_yaws[index]*0.5)?;
            // Quaternion product: Z pitch followed by the bone's X yaw.
            let pitch=if index==0 {[0.*sp,0.*sp,sp,cp]}else{[0.,0.,0.,1.]};
            let yaw=[sy,0.*sy,0.*sy,cy];
            let cross=cross_full4(pitch,yaw);
            let mut q=std::array::from_fn(|i|pitch[i].mul_add(yaw[3],yaw[i].mul_add(pitch[3],cross[i])));
            q[3]=pitch[3]*yaw[3]-HostReckoningNumeric.dot3(pitch,yaw)
                .map_err(|e|format!("post-adjust quaternion dot: {e:?}"))?;
            let compressed=post_adjust_codec::compress(q)?;
            self.adjustments[index].as_mut().ok_or("post-adjust bone record unwritten")?.axis_angle=compressed;
            self.head_yaw+=self.bone_yaws[index];
        }
        if self.head_yaw.is_nan() {return Err("post-adjust native head yaw validity assertion".into());}
        Ok(())
    }
}
fn flat(v:Vector)->Vector {[v[0],0.,v[2],v[0]]}
fn sign(value:f32)->f32 {if value>=0. {1.}else{-1.}}
fn direction(m:&[Vector],v:Vector)->Vector {
    std::array::from_fn(|i|m[2][i].mul_add(v[2],m[1][i].mul_add(v[1],m[0][i]*v[0])))
}
fn sin_cos(angle:f32)->Result<(f32,f32),String>{
    try_sin_cos(angle,&mut HostReckoningSinCosNumeric).map_err(|e|format!("post-adjust SinCos: {e:?}"))
}
fn angle(a:Vector,b:Vector)->Result<f32,String>{
    try_unsigned_angle(a,b,&mut HostReckoningNumeric).map_err(|e|format!("post-adjust angle: {e:?}"))
}
