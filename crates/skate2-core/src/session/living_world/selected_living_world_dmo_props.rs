//! DMO InitContainedProps82d391e0 over the existing PropMan and model groups.
use super::props::{PropRecipeFrame,PropSpawnOwners};
use crate::input_cvar_runtime::InputCvarReferences;

impl PropRecipeFrame<'_> {
    /// Spawn82d31d60: initialize children before attaching their parent to its
    /// EntityMan. The pending parent and shared global ID stream stay retained
    /// across failure; there is no delayed roster scan or second spawn owner.
    pub(crate) fn initialize_dmo_props(&mut self,
        object:&mut crate::living_world_grind_services::dmo_objects::DynamicObject,
        owners:&mut PropSpawnOwners<'_>,
        pedestrians:&mut crate::living_world_grind_services::entities::EntityMan<40,crate::living_world_pedestrian_entity::PedestrianConstruction>,
        cvars: &InputCvarReferences) -> Result<(), String> {
            if object.contained_props_initialized {return Err("DMO child initialization repeated".into());}
            let id=object.entity_id;
            let model = std::sync::Arc::clone(&object.model);
            let transform = std::array::from_fn(|axis|
                std::array::from_fn(|lane|object.transform[axis*4+lane]));
            for source in &model.waypoint_groups {
                let key = source.entity_key()?;
                let Some(child) = self.spawn_waypoint_prop_with_owners(
                    owners, key, transform, false, cvars)? else { continue; };
                {
                    let props = &mut *owners.props;
                    let prop = props.entities.get_mut(child).ok_or("DMO contained child absent")?;
                    prop.containing_entity = Some(id);
                    // Native QueryInterface is optional, although these authored
                    // hotpoint recipes normally implement IWaypointGroup.
                    if let Some(group) = prop.waypoint.as_mut() {
                        group.set_data(Some(source.clone()), &mut props.memory)?;
                    }
                }
                owners.props.release_dmo_waypoint(pedestrians,child)?;
                if object.contained_props.len() == 8 {
                    return Err("native DMO contained-prop capacity8 exceeded".into());
                }
                object.contained_props.push(child);
            }
            object.contained_props_initialized = true;
        Ok(())
    }
}
