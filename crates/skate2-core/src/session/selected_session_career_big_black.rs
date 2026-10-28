//! Manager.SpawnBigBlackMsgHandler82719598 and UnspawnBigBlackMsgHandler8271f618.
use super::Owners;
use crate::{ai::vmx::{self,add,sub,scale,dot3},living_world_chase::TargetableId,
    collision_line_collection::CollisionLineInput,skeleton_root_transforms::SkeletonRootVector,
    reckoning_numeric::{HostReckoningNumeric,ReckoningVector},
    collision_line_numeric::HostCollisionLineNumeric,
    actor_construction::breadcrumbs::BreadCrumbFloatAttribute};
impl Owners<'_, '_>{
    pub(super) fn big_black(&mut self,spawn:bool)->Result<(),String>{
        let owner=self.living();
        let current=owner.lock().map_err(|_|"BigBlack Manager lock")?.big_black_entity;
        if !spawn{
            if let Some(id)=current{
                self.request_explicit_cull(id)?;
                owner.lock().map_err(|_|"BigBlack Manager lock")?.big_black_entity=None;
            }
            return Ok(());
        }
        if current.is_some(){return Err("native SpawnBigBlack existing protector assertion".into());}
        let Some(actor)=self.actor else{return Ok(());};
        let local=actor.prefix().map_err(|e|e.to_string())?.fields().ok_or("BigBlack local Actor fields absent")?.player_index as i32;
        let physical=actor.cached_physical_allocation().map_err(|e|format!("BigBlack spawn Actor: {e:?}"))?;
        let (transform,offboard)={
            let output=physical.owner().phys_out().lock().map_err(|_|"BigBlack spawn PhysOut lock")?;
            let matrix=output.try_skeleton_output().map_err(|e|format!("BigBlack spawn transform: {e:?}"))?.skeleton_root_matrix;
            (matrix.columns().map(|v|v.operand()),output.try_state_output().map_err(|e|format!("BigBlack spawn state: {e:?}"))?.is_offboard)
        };
        let position=transform[3];
        let(z_offset,x_offset,min_crumb,max_crumb,directions,half_height,horizontal_height,buffer)=self.cvars.try_big_black_spawn()
            .map_err(|e|format!("BigBlack spawn tuning: {e:?}"))?;
        let mut destination=None;
        if let Some((crumbs,attributes))=actor.try_service_breadcrumbs()?{
            for crumb in crumbs{
                let candidate=crumb.transform.columns()[3];let mut delta=sub(candidate.operand(),position);delta[1]=0.;delta[3]=0.;
                let squared=dot3(delta,delta);
                if squared-min_crumb*min_crumb>0. && squared-max_crumb*max_crumb<0.
                    && self.world.try_offline_respawn_lw_constraints(&owner,self.ground,candidate,2.,1.,self.cvars,||{
                        let height=BreadCrumbFloatAttribute::CapsuleHeightForCollisionTest.try_read(&attributes)?;
                        let radius=BreadCrumbFloatAttribute::CapsuleRadiusForCollisionTest.try_read(&attributes)?;
                        let start=candidate.zip(SkeletonRootVector::known([0.,radius,0.,0.]),|a,b|a+b);
                        Ok(CollisionLineInput{start,end:start.zip(SkeletonRootVector::known([0.,height,0.,0.]),|a,b|a+b),fatness:radius})
                    }).map_err(|e|e.to_string())?{
                    destination=Some(candidate.operand());break;
                }
            }
        }
        if destination.is_none(){
            let up=[0.,1.,0.,0.];
            // Original compares each XYZ axis to +J with epsilon 0x37800000.
            let parallel=|axis:[f32;4]|(0..3).all(|i|(axis[i]-up[i]).abs()<=f32::from_bits(0x3780_0000));
            let mut z=if parallel(transform[2]){scale(transform[1],-1.)}else{transform[2]};
            let mut x=if parallel(transform[0]){transform[1]}else{transform[0]};
            z[1]=0.;z[3]=0.;x[1]=0.;x[3]=0.;
            let mut offset=scale(scale(z,vmx::rsqrt(dot3(z,z))),z_offset);
            if !offboard{offset=vmx::madd(scale(x,vmx::rsqrt(dot3(x,x))),[x_offset;4],offset);}
            let angle=6.2831855/(directions as u32 as f32);
            for index in 0..directions as u32{
                let rotated=crate::physical_player_grind_manager::math::rotate_vector(up,index as f32*angle,offset,
                    &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric).map_err(|e|format!("BigBlack spawn rotation: {e:?}"))?;
                let candidate=add(position,rotated);
                let vertical_start=vmx::madd(up,[half_height;4],candidate);
                let vertical_end=sub(candidate,scale(up,half_height));
                let horizontal_start=vmx::madd(up,[horizontal_height;4],position);
                let mut horizontal_end=vmx::madd(up,[horizontal_height;4],candidate);
                let delta=sub(horizontal_end,horizontal_start);let direction=scale(delta,vmx::rsqrt(dot3(delta,delta)));
                horizontal_end=vmx::madd(direction,[buffer;4],horizontal_end);
                let vertical=self.big_black_line(vertical_start,vertical_end,"LivingWorldManager::BigBlack Spawning Test Vertical")?;
                let horizontal=self.big_black_line(horizontal_start,horizontal_end,"LivingWorldManager::BigBlack Spawning Test Horizontal")?;
                if horizontal.is_none() && let Some((hit,surface))=vertical && surface&0xf80!=0x300{
                    destination=Some(hit);break;
                }
            }
            if destination.is_none(){destination=Some(add(position,offset));}
        }
        let destination=destination.ok_or("BigBlack destination absent")?;
        let mut direction=sub(position,destination);direction[1]=0.;direction[3]=0.;
        let matrix=crate::reckoning_numeric::try_look_at_up_eye(add(destination,direction),[0.,1.,0.,0.],destination,&mut HostReckoningNumeric)
            .map_err(|e|format!("BigBlack spawn look-at: {e:?}"))?;
        // PPC r6..r10 and stack byte+87: initial0,cull0,force1,world1,ignoreMesh0,handProp0.
        let spawned=self.frame.spawn_census_pedestrian(self.world,self.cvars,self.ground,
            skate2_formats::attrib::string_hash64("big_black_regular01"),matrix,false,false,true,true,false,false)?;
        let mut living=owner.lock().map_err(|_|"BigBlack Manager lock")?;
        living.big_black_entity=spawned;
        if let Some(id)=spawned{
            living.pedestrian_entities.as_deref_mut().and_then(|p|p.get_mut(id))
                .and_then(|p|p.big_black.as_mut()).ok_or("spawned BigBlack interface absent")?.assigned=TargetableId{kind:0,id:local};
        }
        Ok(())
    }
    fn big_black_line(&mut self,start:[f32;4],end:[f32;4],label:&str)->Result<Option<([f32;4],u16)>,String>{
        let(queries,dmos)=self.world.world_line_owners();
        let result=queries.ok_or("BigBlack world line queries absent")?.try_line_test(self.ground,dmos.ok_or("BigBlack DMO pool absent")?,
            CollisionLineInput{start:SkeletonRootVector::known(start),end:SkeletonRootVector::known(end),fatness:0.},label,
            &mut HostCollisionLineNumeric).map_err(|e|format!("BigBlack spawn line: {e:?}"))?;
        Ok(result.map(|hit|(hit.position.operand(),hit.surface_id)))
    }
}
