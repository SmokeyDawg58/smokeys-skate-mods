//! Retained CreateEntity/constructor prefix. The caller owns the pending slot;
//! this module owns neither an entity roster nor another physics backend.
use super::PedestrianConstruction;
use crate::{
    actor_face_playback::AnimationPoseRef,
    attribute_runtime::AttributeRuntimeVault,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::attrib_cache::LwAttribCache,
    living_world_pedestrian_memory::{PedestrianClass, PedestrianMemory, PedestrianStore},
    physical_player_assembly::PhysicalAssemblyWorld,
    world::{LivingWorldEntityIdAllocator, LivingWorldEntityType},
};
use std::sync::Arc;

/// Manager+1b5c4, populated from the existing AnimMan after its Init. These
/// are shared animation references, never substitute bind poses or hierarchies.
pub(crate) struct PedestrianAnimConstructInfo {
    pub(crate) bind_pose: Option<AnimationPoseRef>,
    pub(crate) hierarchy: Option<Arc<skate2_formats::andale::AndaleHierarchy>>,
    pub(crate) motion_graph_pool: u32,
}
impl PedestrianAnimConstructInfo {
    /// Manager ctor82722008..10 clears all three fields.
    pub(crate) const fn new() -> Self {
        Self { bind_pose: None, hierarchy: None, motion_graph_pool: 0 }
    }
    /// Manager.Init8271f83c/844 does not reset the motion-pool index.
    pub(crate) fn initialize(&mut self, bind_pose: AnimationPoseRef,
        hierarchy: Arc<skate2_formats::andale::AndaleHierarchy>) {
        self.bind_pose = Some(bind_pose);
        self.hierarchy = Some(hierarchy);
    }
    /// Called by Spawn AFTER successful CreateEntity and BEFORE InitModels,
    /// even if the later model lookup rejects the entity. Source826f55a8.
    pub(crate) fn increment_motion_graph_pool(&mut self) {
        self.motion_graph_pool = (self.motion_graph_pool.wrapping_sub(1) & 1) + 2;
    }
}

/// Constructs the existing owners in Pedestrian82e97748 order, retaining every
/// completed prefix on failure. This is deliberately not a publication token:
/// InitModels, placement, contained props and PostInitialize still follow.
///
/// Animation input remains borrowed from the real world. `animation_and_motion`
/// must construct animation and load the actual attribute-selected motion path
/// (including test-graph override), not clone the world into another backend.
/// Remaining constructor work uses the real Vault, global RNG and state.
/// creation_allows_hand_props is CreationInfo byte24 bit80. Factory tuning
/// comes from the live retained cvar references at each native read point.
pub(crate) fn construct_retained<A>(
    retained: &mut Option<PedestrianConstruction>,
    memory: &mut PedestrianMemory, ids: &mut LivingWorldEntityIdAllocator,
    vault: &Arc<AttributeRuntimeVault>, attrib_cache: Arc<LwAttribCache>,
    entity_key: u64, model_key: u64,
    world: &mut PhysicalAssemblyWorld, bind_pose: AnimationPoseRef,
    hierarchy: Arc<skate2_formats::andale::AndaleHierarchy>,
    animation_and_motion: A, cvars: &InputCvarReferences, creation_allows_hand_props: bool,
) -> Result<bool, String>
where
    A: FnOnce(&mut PedestrianConstruction, &mut PedestrianMemory,
        &mut PhysicalAssemblyWorld) -> Result<(), String>,
{
    if retained.is_some() { return Err("pedestrian factory has retained construction".into()); }
    let class = PedestrianClass::for_entity_key(entity_key);
    // Native RequestBlock0 precedes callback/Entity ID allocation. Exhaustion
    // returns null without advancing the sole global ID allocator.
    let Some(slot) = memory.request(PedestrianStore::Entity) else { return Ok(false); };
    let id = ids.allocate(LivingWorldEntityType::from_raw(2));
    *retained = Some(PedestrianConstruction::new(id, slot, attrib_cache));
    let entity = retained.as_mut().expect("factory prefix retained");
    entity.initialize_core(memory, vault, entity_key, model_key)?;
    //Pedestrian82e97df0 initializes its sole embedded perception before animdata.
    entity.perception.initialize(vault,entity_key)?;
    animation_and_motion(entity, memory, world)?;
    if entity.animation.is_none() {
        return Err("pedestrian animation callback did not construct its owner".into());
    }
    entity.initialize_adjust_owners(memory, bind_pose, hierarchy)?;
    entity.initialize_nav_rig(memory)?;
    entity.state.as_mut().ok_or("pedestrian AI state absent")?.reset();
    entity.reverse_attribute_category_lookup(vault)?;
    entity.initialize_hand_prop_selection(vault,
        cvars.try_pedestrian_allow_hand_props().map_err(|e| format!("pedestrian handprop cvar: {e:?}"))?,
        creation_allows_hand_props)?;
    entity.positioning_test_start_height = Some(cvars.try_pedestrian_ground_test_distance_up_default()
        .map_err(|e| format!("pedestrian ground-height cvar: {e:?}"))?);
    if class == PedestrianClass::SecurityGuard {
        entity.initialize_security_guard(memory)?;
    } else if class == PedestrianClass::BigBlack {
        entity.initialize_big_black(memory)?;
    }
    Ok(true)
}

impl PedestrianConstruction {
    /// Constructor82e9838c..85ec. The global cvar and creation flag precede
    /// the Bernoulli draw; successful gating consumes a discrete draw even for
    /// an empty list. Uses the same global generator and scalar choice kernel
    /// as native82704880, including its unordered comparison behavior.
    pub(crate) fn initialize_hand_prop_selection(
        &mut self, vault: &Arc<AttributeRuntimeVault>, allowed: bool, creation_allows: bool,
    ) -> Result<(), String> {
        use crate::native_skater_random::rw_math::{bernoulli_choice, global_next};
        const PROBABILITY: u64 = 0x3db0_19a0_8284_f45c;
        const LIST: u64 = 0x46b8_36ee_959c_0238;
        const ENTRY_TYPE: u64 = 0xe4aa_c2ea_bc05_3d0a;
        let attributes = &self.resources.as_ref().ok_or("pedestrian resources absent")?.entity;
        let mut selected = None;
        if allowed && creation_allows {
            let probability = attributes.try_get_data(PROBABILITY, 0).map_err(|e| e.to_string())?
                .map_or(Ok(0.), |data| data.bytes().get(..4)
                    .ok_or("pedestrian handprop probability truncated".to_string())
                    .map(|b| f32::from_be_bytes(b.try_into().expect("four bytes"))))?;
            if bernoulli_choice(probability, global_next().map_err(|e| format!("pedestrian RNG: {e:?}"))?) {
                let count = attributes.try_length(LIST).map_err(|e| e.to_string())?.min(32);
                let mut keys = Vec::with_capacity(count as usize);
                let mut probabilities = Vec::with_capacity(count as usize);
                for i in 0..count {
                    let key = attributes.try_array_field_data(LIST, i, ENTRY_TYPE, 32, 8..16)
                        .map_err(|e| e.to_string())?;
                    let probability = attributes.try_array_field_data(LIST, i, ENTRY_TYPE, 32, 24..28)
                        .map_err(|e| e.to_string())?;
                    keys.push(key.map_or(0, |d| u64::from_be_bytes(d.bytes().try_into().expect("key"))));
                    probabilities.push(probability.map_or(0., |d| f32::from_be_bytes(d.bytes().try_into().expect("float"))));
                }
                let word = global_next().map_err(|e| format!("pedestrian RNG: {e:?}"))?;
                if let Some(index) = crate::living_world_grind_services::census::targets::discrete_choice(&probabilities, word) {
                    selected = Some(keys[index]);
                }
            }
        }
        let state = self.state.as_mut().ok_or("pedestrian AI state absent")?;
        if let Some(key) = selected {
            self.hand_prop_attributes = Some(vault.instance(0xec8b_7565_46e2_26f6, key));
            state.flags[3] |= 2;
        } else {
            state.flags[3] &= !2;
        }
        Ok(())
    }

    /// DoReverseAttribCategoryLookup82e7f4e0: scan every sorted leaf and every
    /// member; later matches overwrite earlier ones. Raw cmpld82e7f5d4 compares
    /// the entire64-bit collection key (the decompiler misleadingly says LOWORD).
    pub(crate) fn reverse_attribute_category_lookup(
        &mut self, vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<(), String> {
        const CLASS: u64 = 0xadcb_4d6b_d3b4_f85e;
        const MEMBERS: u64 = 0xd5e1_267e_2d71_5124;
        let entity_key = self.core.as_ref().ok_or("pedestrian core absent")?.entity_key;
        for category in vault.leaf_collections_sorted(CLASS).map_err(|e| e.to_string())? {
            let attributes = vault.instance(CLASS, category);
            // Same original RefSpec Members array already read by the census.
            // Keep its typed storage checks; the generic POD reader rejects
            // this constructed type. Only the serialized collection key is used.
            let Some(members) = attributes.try_reference_members(MEMBERS, false)
                .map_err(|e|e.to_string())? else { continue; };
            for index in 0..members.len() {
                let data = members.get(index).expect("member within checked count");
                let key = u64::from_be_bytes(data[8..16].try_into().expect("checked RefSpec key"));
                if key == entity_key { self.entity_category_key = Some(category); }
            }
        }
        Ok(())
    }
}
