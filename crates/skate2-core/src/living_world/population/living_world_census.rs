//! Fields shared by ALL components of the original CensusMan. They were
//! previously retained under the selected DMO manager; there is one owner.
//! Constructor827208d0, Init82715908, Update82702780.
#[path = "living_world_census_pedestrian.rs"]
pub(crate) mod pedestrian;
#[path = "living_world_census_random.rs"]
pub(crate) mod random;
#[path = "living_world_census_targets.rs"]
pub(crate) mod targets;
#[path = "living_world_census_vehicle.rs"]
pub(crate) mod vehicle;

#[derive(Clone)]
pub(crate) struct CensusState {
    pub(crate) enable: u8,
    pub(crate) sampled_fe: bool,
    pub(crate) component_index: u32,
    pub(crate) pedestrian_slider: f32,
    pub(crate) vehicle_slider: f32,
    pub(crate) pedestrian_reserve: i32,
    pub(crate) reserve_frames: i32,
    /// Constructor82720a0c samples a linked cvar. None in constructor-only
    /// query fixtures, never a fabricated default population allowance.
    pub(crate) max_vehicles: Option<i32>,
    pub(crate) max_pedestrians: Option<i32>,
    /// Native CensusMan+0x68; populated by constructor from "pedestrians".
    pub(crate) pedestrian_range_key: Option<u64>,
    /// Native CensusMan+0x80: the constructor's "dynamicobjects", or the running challenge's
    /// `DMOCensusRange` (`SetDynamicObjectsCensusRangeKey` 0x8271c088) until it completes
    /// (`ResetDynamicObjectsCensusRangeKey` 0x8271c138).
    pub(crate) dynamic_objects_range_key: u64,
    /// Party Play and online games without pedestrians, a port change: the original populates
    /// them like every mode but free skate. None spawn and every cullable one is culled.
    pub(crate) pedestrians_banned: bool,
    /// Party Play and online games without traffic, a port change: the original populates them
    /// like every mode but free skate. None spawn and every cullable one is culled.
    pub(crate) vehicles_banned: bool,
}
impl CensusState {
    pub(crate) const fn new(spawn_enabled: bool) -> Self {
        Self {
            enable: 0x7e | if spawn_enabled { 0x80 } else { 0 },
            sampled_fe: false,
            component_index: 0,
            pedestrian_slider: 1.,
            vehicle_slider: 1.,
            pedestrian_reserve: 0,
            reserve_frames: 0,
            max_vehicles: None,
            max_pedestrians: None,
            pedestrian_range_key: None,
            dynamic_objects_range_key: crate::living_world_grind_services::dmo_objects::DYNAMIC_OBJECTS_KEY,
            pedestrians_banned: false,
            vehicles_banned: false,
        }
    }
    /// `ReserveForceSpawnSlots` 0x826f4528: the larger reservation, kept four updates.
    pub(crate) fn reserve_force_spawn_slots(&mut self, count: i32) {
        if count > self.pedestrian_reserve {
            self.pedestrian_reserve = count;
        }
        self.reserve_frames = 4;
    }
    /// Update prefix; the actual component cull queues are processed after this.
    /// ProfileFreeSkate uses the two supplied backend sliders, other modes1.
    pub(crate) fn update_settings(
        &mut self,
        profile_sliders: Option<[f32; 2]>,
        pedestrians_banned: bool,
        vehicles_banned: bool,
    ) {
        self.pedestrians_banned = pedestrians_banned;
        self.vehicles_banned = vehicles_banned;
        if self.reserve_frames > 0 {
            self.reserve_frames -= 1;
        } else {
            self.pedestrian_reserve = 0;
        }
        [self.pedestrian_slider, self.vehicle_slider] = profile_sliders.map_or([1.; 2], |values| {
            values.map(|value| {
                // fsel(-value,0,value), then fsel(1-value,value,1).
                let value = if -value >= 0. { 0. } else { value };
                if 1. - value >= 0. { value } else { 1. }
            })
        });
        self.sampled_fe = true;
    }
}
