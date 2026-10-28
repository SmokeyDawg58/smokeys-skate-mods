//! MoodMan82ce8728/Init82ce9638. Authored category and interest owners.
use crate::attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault};
use std::{collections::BTreeMap, sync::Arc};
use skate2_formats::attrib::string_hash64;

const CATEGORIES: [&str; 18] = [
    "genericbad", "litter", "skaterslam", "skateravoid", "skaterskitch",
    "skaternoise", "skatertrick", "skaterdamage", "presence",
    "inpatrolbufferwhileownerisreturning", "collision", "nearbycollision",
    "warningcomplete", "aggressivechasecomplete", "nearbychase",
    "chasedpersondetectibilitytest", "greeted", "beingchased",
];
pub(crate) const CATEGORY_CLASS: u64 = 0xd55e_a4ec_5987_f507;
const ENTITY_CLASS: u64 = 0x60d7_03c7_7cab_1631;
pub(crate) const MOOD_CLASS: u64 = 0xd353_ca7c_1f86_61ea;
pub(crate) const RESULT_CLASS: u64 = 0xb58b_16a4_717a_fe99;
pub(crate) const RESULT_LIST: u64 = 0x4694_b003_9eaa_cdb8;
pub(crate) const RESULT_ENTRY_TYPE: u64 = 0x37ee_ae39_0851_cff9;

pub(crate) struct MoodMan {
    categories: [Option<AttributeRuntimeInstance>; 18],
    event_map: BTreeMap<(u64, u64), u32>,
    reactions_allowed: bool,
}
impl MoodMan {
    pub(crate) fn new() -> Self {
        Self { categories: std::array::from_fn(|_| None), event_map: BTreeMap::new(),
            reactions_allowed: true }
    }
    /// Category prefix of Init. Keep successful bindings if a later assertion
    /// fails; registration with the existing broadcaster follows this prefix.
    pub(crate) fn initialize_categories(&mut self, vault: &Arc<AttributeRuntimeVault>) -> Result<(), String> {
        for (index, name) in CATEGORIES.iter().enumerate() {
            let instance = vault.instance(CATEGORY_CLASS, string_hash64(name));
            let missing = instance.is_null();
            self.categories[index] = Some(instance);
            if missing { return Err(format!("MoodMan category {name} missing (82ce96bc)")); }
        }
        Ok(())
    }
    /// Init suffix after the native Mood message subscription. This reuses the
    /// complete collection graph and typed array storage in the original Vault.
    pub(crate) fn initialize_event_map(&mut self, vault: &Arc<AttributeRuntimeVault>) -> Result<(), String> {
        for witness in vault.descendants_sorted(ENTITY_CLASS, string_hash64("regular")).map_err(|e| e.to_string())? {
            let entity = vault.instance(ENTITY_CLASS, witness);
            let collection = entity.try_ref_spec_collection(0x712e_5451_3736_63be).map_err(|e| e.to_string())?.unwrap_or(0);
            let mood = vault.instance(MOOD_CLASS, collection);
            let count = mood.try_length(RESULT_LIST).map_err(|e| e.to_string())?;
            for index in 0..count {
                let data = mood.try_array_field_data(RESULT_LIST, index, RESULT_ENTRY_TYPE, 32, 8..16)
                    .map_err(|e| e.to_string())?;
                let result_key = data.as_ref().map(|d| u64::from_be_bytes(d.bytes().try_into().expect("RefSpec collection"))).unwrap_or(0);
                let result = vault.instance(RESULT_CLASS, result_key);
                let enabled = layout(&result, 97, 1)?[0] != 0;
                let perpetrator = if enabled { u64::from_be_bytes(layout(&result, 32, 8)?.try_into().expect("RefSpec collection")) } else { 0 };
                let category = u64::from_be_bytes(layout(&result, 56, 8)?.try_into().expect("RefSpec collection"));
                let bits = self.event_map.entry((witness, perpetrator)).or_default();
                // Raw82ce9990 compares the collection low word, as native does.
                if let Some(event) = CATEGORIES.iter().position(|name| string_hash64(name) as u32 == category as u32) {
                    *bits |= 1 << event;
                }
            }
        }
        Ok(())
    }
    pub(crate) fn reactions_allowed(&self) -> bool { self.reactions_allowed }
    pub(crate) fn set_reactions_allowed(&mut self, allowed: bool) { self.reactions_allowed = allowed; }
    /// Reset82ce6c60 resets the permission bit only, retaining authored data.
    pub(crate) fn reset(&mut self) { self.reactions_allowed = true; }
    pub(crate) fn category_key(&self,event:i32)->Result<u64,String>{
        CATEGORIES.get(event as usize).map(|name|string_hash64(name)).ok_or_else(||format!("invalid mood category {event}"))
    }
    pub(crate) fn category(&self, event: i32) -> Result<&AttributeRuntimeInstance, String> {
        self.categories.get(event as usize).and_then(Option::as_ref)
            .ok_or_else(|| format!("MoodMan category {event} not initialized"))
    }
    pub(crate) fn has_event(&self, witness: u64, perpetrator: u64, event: i32) -> bool {
        if !(0..18).contains(&event) { return false; }
        let bit = 1u32 << event;
        self.event_map.get(&(witness, perpetrator)).is_some_and(|v| v & bit != 0)
            || self.event_map.get(&(witness, 0)).is_some_and(|v| v & bit != 0)
    }
    pub(crate) fn expire_time(&self, event: i32) -> Result<f32, String> {
        self.scalar(event, 0xa379_0f7a_17dd_95db)
    }
    pub(crate) fn priority(&self, event: i32) -> Result<u32, String> {
        Ok(self.category(event)?.try_scalar_word(0x1a3f_b63d_fdae_0c4f, 0)
            .map_err(|e| e.to_string())?.unwrap_or(0))
    }
    /// EntityCaresAboutEvent82ce8258: map lookup precedes all prerequisite
    /// attributes and world queries. Only matching authored predicates run.
    pub(crate) fn entity_cares_about_event(&self, vault: &Arc<AttributeRuntimeVault>,
        witness: u64, perpetrator: u64, event: i32,
        mut evaluate: impl FnMut(i32, i32, f32) -> Result<bool, String>,
    ) -> Result<bool, String> {
        if !self.has_event(witness, perpetrator, event) { return Ok(false); }
        let entity = vault.instance(ENTITY_CLASS, witness);
        let mood = entity.try_ref_spec_instance(0x712e_5451_3736_63be)
            .map_err(|e| e.to_string())?;
        let Some(mood) = mood else { return Ok(true); };
        const KEY: u64 = 0xfa4c_8c3b_fcc7_22bc;
        const TYPE: u64 = 0xa0a8_bf1b_a3b3_6a68;
        let category = string_hash64(CATEGORIES[event as usize]);
        for index in 0..mood.try_length(KEY).map_err(|e| e.to_string())? {
            let key = mood.try_array_field_data(KEY, index, TYPE, 40, 8..16)
                .map_err(|e| e.to_string())?;
            let key = key.as_ref().map(|d|u64::from_be_bytes(d.bytes().try_into().expect("category key"))).unwrap_or(0);
            if key as u32 != category as u32 { continue; }
            let fields = mood.try_array_field_data(KEY, index, TYPE, 40, 24..36)
                .map_err(|e| e.to_string())?;
            let zero = [0u8; 12];
            let fields = fields.as_ref().map(|d|d.bytes()).unwrap_or(&zero);
            let word = |offset| u32::from_be_bytes(fields[offset..offset+4].try_into().expect("prerequisite word"));
            if !evaluate(word(0) as i32, word(4) as i32, f32::from_bits(word(8)))? { return Ok(false); }
        }
        Ok(true)
    }
    pub(crate) fn update_rate(&self, event: i32) -> Result<f32, String> {
        self.scalar(event, 0xb1dd_83e0_617a_f652)
    }
    pub(crate) fn visibility_distance(&self, event: i32) -> Result<f32, String> {
        Ok(f32::from_be_bytes(layout(self.category(event)?, 0, 4)?.try_into().expect("float")))
    }
    pub(crate) fn audibility_distance(&self, event: i32) -> Result<f32, String> {
        Ok(f32::from_be_bytes(layout(self.category(event)?, 4, 4)?.try_into().expect("float")))
    }
    pub(crate) fn continuous(&self, event: i32) -> Result<bool, String> {
        Ok(layout(self.category(event)?, 8, 1)?[0] != 0)
    }
    /// IsMoodEventLocal82ce6f30; raw PPC forms the full 64-bit key.
    pub(crate) fn is_local(&self, event: i32) -> Result<bool, String> {
        Ok(self.category(event)?.try_get_data(0xe887_4b03_b58f_0c67, 0)
            .map_err(|e|e.to_string())?.is_some_and(|d|d.bytes().first().is_some_and(|v|*v!=0)))
    }
    pub(crate) fn broadcast_range(&self, event: i32) -> Result<f32, String> {
        self.scalar(event, 0x1825_0d95_84fc_82c0)
    }
    fn scalar(&self, event: i32, key: u64) -> Result<f32, String> {
        Ok(f32::from_bits(self.category(event)?.try_scalar_word(key, 0)
            .map_err(|e| e.to_string())?.unwrap_or(0)))
    }
}

impl crate::living_world_grind_services::LivingWorldGrindServices {
    /// ReceiveAmbientMoodEventMsg82ce79d8. Raw82ce7af8 supplies capacity40;
    /// its loop is omitted by the decompiler. Preserve the existing KD order.
    pub(crate) fn ambient_mood_recipients(&self, event: &crate::hermes_game_sim_queue::MoodEvent)
        -> Result<Vec<u32>, String> {
        let mood=self.mood.as_ref().ok_or("ambient MoodMan absent")?;
        if mood.category(event.event_type)?.is_null() { return Ok(Vec::new()); }
        if mood.is_local(event.event_type)? {
            // Pedestrian is the only ILWMoodable implementation among the
            // four concrete Targetable types; guards share the same interface.
            if event.victim.kind==1 && event.victim.id>=0 && self.pedestrian_entities.as_ref()
                .is_some_and(|r|r.get(event.victim.id as u32).is_some()) {
                return Ok(vec![event.victim.id as u32]);
            }
            return Ok(Vec::new());
        }
        let range=mood.broadcast_range(event.event_type)?;
        let range=if range==0. {1.0e10} else {range};
        let mut count=0; let mut ids=[0;40];let mut distances=[0.;40];
        self.try_get_pedestrians_within_range([f32::from_bits(event.position[0]),f32::from_bits(event.position[2])],
            range,40,&mut count,&mut ids,&mut distances).map_err(|e|format!("ambient mood range: {e:?}"))?;
        Ok(ids[..count as usize].to_vec())
    }
}

pub(crate) fn layout(instance: &AttributeRuntimeInstance, offset: usize, size: usize) -> Result<Vec<u8>, String> {
    Ok(instance.try_cached_layout_field(offset, size).map_err(|e| e.to_string())?
        .map(|data| data.bytes().to_vec()).unwrap_or_else(|| vec![0; size]))
}
