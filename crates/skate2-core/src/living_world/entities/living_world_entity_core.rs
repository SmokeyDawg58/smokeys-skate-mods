//! Shared LWEntityCoreData/LWCommonCoreData and transform operations.
//! Constructor82e4cb38/82e50f48; setters82e4cbf8/82e577a8.
use crate::{living_world_rain::geometry::Transform, reckoning_numeric::ReckoningNumeric};

#[path = "living_world_entity_distance.rs"]
pub(crate) mod distance;

const IDENTITY: Transform = [
    [1., 0., 0., 0.],
    [0., 1., 0., 0.],
    [0., 0., 1., 0.],
    [0.; 4],
];
pub(crate) struct CommonEntityCore {
    pub(crate) transform: Transform,
    pub(crate) target_transform: Transform,
    pub(crate) velocity: [f32; 4],
    pub(crate) position_xz: [f32; 2],
    pub(crate) player_distance: distance::EntityDistanceStack,
    pub(crate) entity_key: u64,
    pub(crate) entity_type: u32,
    pub(crate) model_key: u64,
    pub(crate) recipe_id: u64,
    pub(crate) colour_indices: [u8; 2],
    pub(crate) current_losim: u32,
    pub(crate) explicit_update_requested: bool,
    pub(crate) allow_culling: bool,
}
#[derive(Debug)]
pub(crate) enum CoreError<E> {
    Numeric(E),
    InvalidTransform,
}
#[derive(Debug)]
pub(crate) enum ModelColourError {
    Attribute(crate::attribute_runtime::AttributeRuntimeError),
    Random(crate::native_skater_random::rw_math::RwRandomLockError),
    EmptyArray(u64),
}
impl CommonEntityCore {
    /// LWBasicEntity.TestLOSim82e50180: shared full-XYZ distance and cached
    /// low/high radius comparisons, including equality at the high boundary.
    pub(crate) fn test_losim<N: ReckoningNumeric>(&self, center: [f32; 4],
        cache: &crate::living_world_grind_services::attrib_cache::LwAttribCache,
        numeric: &mut N) -> Result<u32, N::Error> {
        Self::test_losim_at(self.transform[3], center, cache, numeric)
    }
    pub(crate) fn test_losim_at<N: ReckoningNumeric>(position: [f32;4], center: [f32;4],
        cache: &crate::living_world_grind_services::attrib_cache::LwAttribCache,
        numeric: &mut N) -> Result<u32,N::Error> {
        let delta = std::array::from_fn(|i| center[i] - position[i]);
        let distance = numeric.dot3(delta, delta)?;
        Ok(if distance > cache.lo_losim_radius[0] * cache.lo_losim_radius[0] { 2 }
            else if cache.hi_losim_radius[0] * cache.hi_losim_radius[0] >= distance { 0 }
            else { 1 })
    }
    /// Shared InitModels colour selection: pedestrian82e785ac..644 and
    /// vehicle82d1d648..690 use identical keys, draws and byte stores.
    /// Draw even for a one-entry array; preserve the first store if second fails.
    pub(crate) fn initialize_model_colours(
        &mut self, model: &crate::attribute_runtime::AttributeRuntimeInstance,
    ) -> Result<(), ModelColourError> {
        for (index, key) in [0xdf76_d7d7_7385_7edb, 0x1202_6e2e_ed18_cc8d].into_iter().enumerate() {
            let count = model.try_length(key).map_err(ModelColourError::Attribute)?;
            let random = crate::native_skater_random::rw_math::global_next()
                .map_err(ModelColourError::Random)?;
            if count == 0 { return Err(ModelColourError::EmptyArray(key)); }
            self.colour_indices[index] = (random % count) as u8;
        }
        Ok(())
    }
    /// Selected constructor fields on a freshly requested MemStore block.
    /// The ctor does not initialize player-distance; MemStore.Init82a6de38
    /// and FreeBlock82a6df68 both zero the block, including reused slots.
    pub(crate) fn new() -> Self {
        Self {
            transform: IDENTITY,
            target_transform: IDENTITY,
            velocity: [0.; 4],
            position_xz: [0.; 2],
            player_distance: distance::EntityDistanceStack::from_cleared_memstore(),
            entity_key: 0,
            entity_type: 7,
            model_key: 0,
            recipe_id: 0,
            colour_indices: [0; 2],
            current_losim: 0,
            explicit_update_requested: false,
            allow_culling: true,
        }
    }
    pub(crate) fn set_target_transform<E>(&mut self, value: Transform) -> Result<(), CoreError<E>> {
        if !valid_transform(&value) {
            return Err(CoreError::InvalidTransform);
        }
        self.target_transform = value;
        Ok(())
    }
    pub(crate) fn set_transform<E>(&mut self, value: Transform) -> Result<(), CoreError<E>> {
        // Source checks the OLD transform (82e577d0), whereas the target setter
        // checks its argument. Self-comparison checks XYZ only, not finiteness.
        if !valid_transform(&self.transform) {
            return Err(CoreError::InvalidTransform);
        }
        self.transform = value;
        self.position_xz = [value[3][0], value[3][2]];
        Ok(())
    }
    pub(crate) fn update_transform<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        numeric: &mut N,
    ) -> Result<(), CoreError<N::Error>> {
        self.velocity = target_velocity(self.transform[3], self.target_transform[3], dt, numeric)
            .map_err(CoreError::Numeric)?;
        self.set_transform(self.target_transform)
    }
}
pub(crate) fn valid_transform(value: &Transform) -> bool {
    value
        .iter()
        .all(|row| row[..3].iter().all(|value| !value.is_nan()))
}
/// LWBasicEntity.UpdateTransform82e5b358; shared by DMOs and ambient entities.
/// Preserve both source Newton steps and all four velocity lanes.
pub(crate) fn target_velocity<N: ReckoningNumeric>(
    from: [f32; 4],
    to: [f32; 4],
    dt: f32,
    numeric: &mut N,
) -> Result<[f32; 4], N::Error> {
    let estimate = numeric.reciprocal_estimate(dt)?;
    let first = estimate.mul_add((-estimate).mul_add(dt, 1.), estimate);
    let inverse = first.mul_add((-first).mul_add(dt, 1.), first);
    Ok(std::array::from_fn(|lane| {
        inverse * (to[lane] - from[lane])
    }))
}
