//! DMO contained-waypoint activation over the one Prop and pedestrian rosters.
use super::{SelectedRecipeFrame, props::PropRecipeFrame};
use crate::{living_world_grind_services::LivingWorldGrindServices,
    input_cvar_runtime::InputCvarReferences, native_skater_random::NativeSkaterRandom,
    physical_player_assembly::PhysicalAssemblyWorld, world_painter_layer::WorldPainterLayerMan,
    living_world_grind_services::waypoints::WaypointRef, skeleton_root_transforms::SkeletonRootVector};

impl crate::living_world_grind_services::waypoints::PropMan {
    /// DynamicObject.UpdateContainedPropTransforms82d2e510 uses immutable
    /// authored local coordinates each time; never accumulates world transforms.
    pub(crate) fn update_dmo_contained_prop_transforms(&mut self,
        children:&[u32],transform:[[f32;4];4])->Result<(),String>{
        if children.len()>8{return Err("native DMO contained Prop capacity8 exceeded".into());}
        let props=self;
        // First the complete LWBehaviourSink base pass, then waypoint updates.
        props.update_contained_prop_transforms(children,transform)?;
        let position=|value:[f32;4]|std::array::from_fn(|i|transform[2][i].mul_add(value[2],
            transform[1][i].mul_add(value[1],transform[0][i].mul_add(value[0],transform[3][i]))));
        let direction=|value:[f32;4]|std::array::from_fn(|i|transform[2][i].mul_add(value[2],
            transform[1][i].mul_add(value[1],transform[0][i]*value[0])));
        for child in children {
            let prop=props.entities.get_mut(*child).ok_or("contained transform Prop absent")?;
            let Some(group)=prop.waypoint.as_mut() else{continue;};
            let Some(source)=group.source.as_ref() else{continue;};
            for index in 0..source.point_count()? {
                let (id,local_position,local_orientation)=source.point_with_id(index)?;
                if let Some(point)=group.points.get_mut(id as usize){
                    point.location=position(local_position);
                    point.orientation=direction(local_orientation);
                }
            }
            group.centroid=position(source.centroid()?);
            let centred=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],group.centroid];
            prop.core.as_mut().ok_or("contained transform Prop core absent")?.set_transform::<String>(centred)
                .map_err(|e|format!("contained centroid transform: {e:?}"))?;
        }
        Ok(())
    }
}

impl PropRecipeFrame<'_> {
    /// Nonzero-operation specialization of DeactivateContainedProps82d2e430.
    pub(crate) fn release_dmo_contained_props(&self,living:&mut LivingWorldGrindServices,
        id:u32)->Result<(),String>{
        let object=living.dmo_objects.as_deref().and_then(|objects|objects.object(id))
            .ok_or("contained release DMO absent")?;
        if object.flags&0x80==0{return Ok(());}
        let children=object.contained_props.clone();
        if children.len()>8{return Err("native DMO contained Prop capacity8 exceeded".into());}
        for child in children {self.release_dmo_waypoint(living,child)?;}
        living.dmo_objects.as_deref_mut().and_then(|objects|objects.object_mut(id))
            .ok_or("contained release DMO disappeared")?.flags&=!0x80;
        Ok(())
    }

    /// WaypointGroup.Deactivate82e65718, nonzero operation. The plugin removal
    /// is requested for its next state update, not executed during this call.
    pub(crate) fn release_dmo_waypoint(&self,living:&mut LivingWorldGrindServices,
        prop:u32)->Result<(),String>{
        let props=living.props.as_deref_mut().ok_or("contained PropMan absent")?;
        props.release_dmo_waypoint(living.pedestrian_entities.as_deref_mut().ok_or("waypoint pedestrian roster absent")?,prop)
    }
}
impl crate::living_world_grind_services::waypoints::PropMan {
    pub(crate) fn release_dmo_waypoint(&mut self,
        pedestrians:&mut crate::living_world_grind_services::entities::EntityMan<40,crate::living_world_pedestrian_entity::PedestrianConstruction>,
        prop:u32)->Result<(),String>{
        let props=self;
        let owner=props.entities.get_mut(prop).ok_or("contained Prop absent")?;
        owner.flags&=!0x80;
        let Some(group)=owner.waypoint.as_ref() else{return Ok(());};
        let count=group.points.len();
        for index in 0..count {
            let reference=WaypointRef{prop,index:index as u32};
            let Some(locker)=props.point(reference)?.locking_entity else{continue;};
            let entity=pedestrians.get_mut(locker).ok_or("native locked waypoint pedestrian assertion")?;
            entity.core.as_mut().ok_or("waypoint pedestrian core absent")?.allow_culling=true;
            entity.plugin_removal_requested=true;
            entity.clear_current_waypoint(props)?;
            // Native clears the visited point even if the pedestrian's own
            // current-waypoint pointer referred to another reservation.
            props.point_mut(reference)?.unlock();
        }
        Ok(())
    }
}

impl PropRecipeFrame<'_> {
    /// DynamicObject.ActivateContainedProps82d2e160. Each child is considered
    /// in its retained capacity-eight vector order, even if already active.
    pub(crate) fn activate_dmo_contained_props(&self,living:&mut LivingWorldGrindServices,
        id:u32,cvars:&InputCvarReferences)->Result<(),String>{
        let children=living.dmo_objects.as_deref().and_then(|objects|objects.object(id))
            .ok_or("contained activation DMO absent")?.contained_props.clone();
        if children.len()>8{return Err("native DMO contained Prop capacity8 exceeded".into());}
        for child in children {
            let prop=living.props.as_deref().and_then(|props|props.entities.get(child))
                .ok_or("contained activation Prop absent")?;
            let probe=if prop.flags&0x20!=0 {
                if let Some(group)=prop.waypoint.as_ref(){
                    let point=group.points.first().ok_or("native activation waypoint0 assertion")?;
                    let offset=prop.test_nav_mesh_offset.ok_or("contained waypoint offset absent")?;
                    // vmulfp followed by vaddfp; preserve separate rounding.
                    Some(std::array::from_fn::<_,4,_>(|i|point.location[i]+point.orientation[i]*offset))
                }else{None}
            }else{None};
            let rejected=if let Some(position)=probe {
                let tolerance=cvars.try_contained_waypoint_y_tolerance().map_err(|e|format!("contained waypoint tuning: {e:?}"))?;
                let mesh=living.try_mesh_store().map_err(|e|format!("contained waypoint mesh: {e:?}"))?;
                let cell=mesh.try_get_cell_with_tolerance(SkeletonRootVector::known(position),tolerance)
                    .map_err(|e|format!("contained waypoint query: {e:?}"))?;
                // EPSILON_160 at82327080. NaN follows native ordered compare.
                !cell.is_valid() || cell.score_with_threshold(0.).abs()>f32::from_bits(0x3400_0000)
            }else{false};
            if rejected {self.release_dmo_waypoint(living,child)?;}
            else {living.props.as_deref_mut().and_then(|props|props.entities.get_mut(child))
                .ok_or("contained activation Prop disappeared")?.flags|=0x80;}
        }
        living.dmo_objects.as_deref_mut().and_then(|objects|objects.object_mut(id))
            .ok_or("contained activation DMO disappeared")?.flags|=0x80;
        Ok(())
    }
}

impl SelectedRecipeFrame<'_> {
    /// WaypointGroup.Deactivate82e65718 operation zero uses the existing
    /// explicit census cull (including plugin/hand-prop/chase cleanup).
    pub(crate) fn deactivate_dmo_waypoint(&self,living:&mut LivingWorldGrindServices,
        world:&mut PhysicalAssemblyWorld,prop:u32,op:u32,cvars:&InputCvarReferences,
        painter:&WorldPainterLayerMan,random:&mut NativeSkaterRandom)->Result<(),String>{
        if op!=0{return self.prop_recipe_frame().release_dmo_waypoint(living,prop);}
        let owner=living.props.as_deref_mut().and_then(|props|props.entities.get_mut(prop))
            .ok_or("contained deactivation Prop absent")?;
        owner.flags&=!0x80;
        let Some(group)=owner.waypoint.as_ref() else{return Ok(());};
        let count=group.points.len();
        for index in 0..count {
            let reference=WaypointRef{prop,index:index as u32};
            let Some(locker)=living.props.as_deref().ok_or("PropMan absent")?.point(reference)?.locking_entity else{continue;};
            living.pedestrian_entities.as_deref_mut().and_then(|p|p.get_mut(locker))
                .ok_or("native locked waypoint pedestrian assertion")?.core.as_mut()
                .ok_or("waypoint pedestrian core absent")?.allow_culling=true;
            self.request_pedestrian_cull(living,world,locker,cvars,painter,random)?;
            living.props.as_deref_mut().ok_or("PropMan absent")?.point_mut(reference)?.unlock();
        }
        Ok(())
    }

    /// DynamicObject.DeactivateContainedProps82d2e430 gates the entire visit
    /// on flags+4368 bit80, then clears it only after all child callbacks.
    pub(crate) fn deactivate_dmo_contained_props(&self,living:&mut LivingWorldGrindServices,
        world:&mut PhysicalAssemblyWorld,id:u32,op:u32,cvars:&InputCvarReferences,
        painter:&WorldPainterLayerMan,random:&mut NativeSkaterRandom)->Result<(),String>{
        if op!=0{return self.prop_recipe_frame().release_dmo_contained_props(living,id);}
        let object=living.dmo_objects.as_deref().and_then(|objects|objects.object(id))
            .ok_or("contained deactivation DMO absent")?;
        if object.flags&0x80==0{return Ok(());}
        let children=object.contained_props.clone();
        if children.len()>8{return Err("native DMO contained Prop capacity8 exceeded".into());}
        for child in children {self.deactivate_dmo_waypoint(living,world,child,op,cvars,painter,random)?;}
        living.dmo_objects.as_deref_mut().and_then(|objects|objects.object_mut(id))
            .ok_or("contained deactivation DMO disappeared")?.flags&=!0x80;
        Ok(())
    }
}
