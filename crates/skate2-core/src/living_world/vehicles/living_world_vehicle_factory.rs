//! Vehicle factory82d25670/82e66bb8 and memory manager82d15e38.
//! Uses the existing MemStore slot allocator; the EntityMan remains the sole
//! active roster. These slots also cover entities not yet attached or culled.
use super::Vehicle;
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    living_world_grind_services::attrib_cache::LwAttribCache,
    world::{LivingWorldEntityIdAllocator, LivingWorldEntityType, LivingWorldMemStoreSlots},
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub(crate) enum VehicleCreateCallback {
    Regular,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        attribute_runtime::tests::graph, living_world_grind_services::dmo_objects::ENTITIES_CLASS,
    };

    pub(crate) fn vehicle_fixture() -> Vehicle {
        let vault = Arc::new(graph(ENTITIES_CLASS, vec![]));
        let cache = Arc::new(LwAttribCache {
            hi_lod_radius: [0.; 4],
            lo_lod_radius: [0.; 4],
            hi_losim_radius: [0.; 4],
            lo_losim_radius: [0.; 4],
            model_key: 55,
        });
        VehicleFactory::initialized()
            .create(
                &mut LivingWorldEntityIdAllocator::default(),
                11,
                55,
                cache,
                &vault,
            )
            .unwrap()
            .unwrap()
    }

    // UNRUN: exercises real constructor, shared IDs/cache, and allocation order.
    #[test]
    fn capacity_does_not_consume_ids_and_collision_store_is_separate() {
        let vault = Arc::new(graph(ENTITIES_CLASS, vec![]));
        let cache = Arc::new(LwAttribCache {
            hi_lod_radius: [0.; 4],
            lo_lod_radius: [0.; 4],
            hi_losim_radius: [0.; 4],
            lo_losim_radius: [0.; 4],
            model_key: 55,
        });
        let mut factory = VehicleFactory::initialized();
        let mut ids = LivingWorldEntityIdAllocator::default();
        let mut vehicles = Vec::new();
        for i in 0..24 {
            let vehicle = factory
                .create(&mut ids, 11, 55, Arc::clone(&cache), &vault)
                .unwrap()
                .unwrap();
            assert_eq!(vehicle.id, 0x0300_0001 + i);
            assert_eq!(vehicle.allocation.entity, i);
            assert_eq!(vehicle.allocation.core, i);
            assert!(vehicle.allocation.collision.is_none());
            assert!(Arc::ptr_eq(&vehicle.attrib_cache, &cache));
            vehicles.push(vehicle);
        }
        assert_eq!(factory.collisions.available(), 24);
        assert!(
            factory
                .create(&mut ids, 11, 55, Arc::clone(&cache), &vault)
                .unwrap()
                .is_none()
        );
        assert_eq!(ids.next_counter(), 25);
        // InitModels can be ordered differently from object construction.
        factory
            .allocate_collision(&mut vehicles[7].allocation)
            .unwrap();
        factory
            .allocate_collision(&mut vehicles[2].allocation)
            .unwrap();
        assert_eq!(vehicles[7].allocation.collision, Some(0));
        assert_eq!(vehicles[2].allocation.collision, Some(1));
        assert!(matches!(
            factory.free_core(&vehicles[7].allocation),
            Err(VehicleFactoryError::CollisionStillAllocated)
        ));
        let mut retired = vehicles.remove(7);
        factory.free_collision(&mut retired.allocation).unwrap();
        factory.free_core(&retired.allocation).unwrap();
        factory.free_entity_slot(&retired.allocation).unwrap();
        drop(retired);
        let mut replacement = factory
            .create(&mut ids, 11, 55, cache, &vault)
            .unwrap()
            .unwrap();
        assert_eq!(replacement.id, 0x0300_0019);
        assert_eq!(replacement.allocation.entity, 7);
        assert_eq!(replacement.allocation.core, 7);
        factory
            .allocate_collision(&mut replacement.allocation)
            .unwrap();
        assert_eq!(replacement.allocation.collision, Some(0));
    }

    #[test]
    fn null_callback_keeps_requested_block_without_consuming_an_id() {
        let vault = Arc::new(graph(ENTITIES_CLASS, vec![]));
        let cache = Arc::new(LwAttribCache {
            hi_lod_radius: [0.; 4],
            lo_lod_radius: [0.; 4],
            hi_losim_radius: [0.; 4],
            lo_losim_radius: [0.; 4],
            model_key: 55,
        });
        let mut factory = VehicleFactory::initialized();
        let mut ids = LivingWorldEntityIdAllocator::default();
        factory.set_callback(0x0000_0001_0000_000b, None);
        assert!(
            factory
                .create(
                    &mut ids,
                    0x0000_0001_0000_000b,
                    55,
                    Arc::clone(&cache),
                    &vault
                )
                .unwrap()
                .is_none()
        );
        assert_eq!(ids.next_counter(), 1);
        assert_eq!(factory.entities.available(), 23);
        let vehicle = factory
            .create(&mut ids, 11, 55, cache, &vault)
            .unwrap()
            .unwrap();
        assert_eq!(vehicle.allocation.entity, 1);
        assert_eq!(vehicle.allocation.core, 0);
        assert_eq!(vehicle.id, 0x0300_0001);
    }
}

/// The collision allocation happens later, in InitModels, not in the ctor.
#[derive(Debug)]
pub(crate) struct VehicleAllocation {
    entity: u32,
    core: u32,
    pub(super) collision: Option<u32>,
}

pub(crate) struct VehicleFactory {
    entities: LivingWorldMemStoreSlots<24>,
    cores: LivingWorldMemStoreSlots<24>,
    collisions: LivingWorldMemStoreSlots<24>,
    default_callback: Option<VehicleCreateCallback>,
    callbacks: BTreeMap<u64, Option<VehicleCreateCallback>>,
}
#[derive(Debug)]
pub(crate) enum VehicleFactoryError {
    Attribute(AttributeRuntimeError),
    CoreStoreExhausted,
    CollisionStoreExhausted,
    CollisionAlreadyAllocated,
    CollisionStillAllocated,
    InvalidFree,
}

impl VehicleFactory {
    /// CreateEntityFactory82d25670 installs CreateRegularVehicle; Init's
    /// virtual InitCallbackMap resolves to the map-clear body82d24cd8.
    /// MemoryMan.Init82d15e38 initializes 0x1130/0x120/0x2c-byte stores,
    /// all with 24 blocks. Host payloads are typed, not guest byte buffers.
    pub(crate) fn initialized() -> Self {
        Self {
            entities: LivingWorldMemStoreSlots::new(),
            cores: LivingWorldMemStoreSlots::new(),
            collisions: LivingWorldMemStoreSlots::new(),
            default_callback: Some(VehicleCreateCallback::Regular),
            callbacks: BTreeMap::new(),
        }
    }

    /// GetCallback82e66c28: a present null override suppresses the default.
    fn callback(&self, key: u64) -> Option<VehicleCreateCallback> {
        self.callbacks
            .get(&key)
            .copied()
            .unwrap_or(self.default_callback)
    }

    /// AddCallback82d26eb0 overwrites operator[]; keys retain all 64 bits.
    pub(crate) fn set_callback(&mut self, key: u64, callback: Option<VehicleCreateCallback>) {
        self.callbacks.insert(key, callback);
    }

    /// Caller obtains the shared census cache BEFORE this factory call.
    /// The source does not free a requested block when GetCallback is null.
    /// Checked constructor failures likewise retain the allocation prefix.
    pub(crate) fn create(
        &mut self,
        ids: &mut LivingWorldEntityIdAllocator,
        entity_key: u64,
        model_key: u64,
        cache: Arc<LwAttribCache>,
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Option<Vehicle>, VehicleFactoryError> {
        let Some(entity) = self.entities.allocate() else {
            return Ok(None);
        };
        let Some(callback) = self.callback(entity_key) else {
            return Ok(None);
        };
        match callback {
            VehicleCreateCallback::Regular => {
                // LWBasicEntity82e61400 consumes ID before Vehicle's core
                // RequestBlock82d1cd94. An exhausted object store consumes none.
                let id = ids.allocate(LivingWorldEntityType::from_raw(3));
                let core = self
                    .cores
                    .allocate()
                    .ok_or(VehicleFactoryError::CoreStoreExhausted)?;
                Vehicle::construct_fields(
                    id,
                    VehicleAllocation {
                        entity,
                        core,
                        collision: None,
                    },
                    entity_key,
                    model_key,
                    cache,
                    vault,
                )
                .map(Some)
                .map_err(VehicleFactoryError::Attribute)
            }
        }
    }

    /// CreateLivingWorldCollision82d1d290 asserts on store exhaustion.
    pub(super) fn allocate_collision(
        &mut self,
        allocation: &mut VehicleAllocation,
    ) -> Result<(), VehicleFactoryError> {
        if allocation.collision.is_some() {
            return Err(VehicleFactoryError::CollisionAlreadyAllocated);
        }
        allocation.collision = Some(
            self.collisions
                .allocate()
                .ok_or(VehicleFactoryError::CollisionStoreExhausted)?,
        );
        Ok(())
    }

    /// DestroyLivingWorldCollision82d1d310: controller destructor THEN free.
    pub(super) fn free_collision(
        &mut self,
        allocation: &mut VehicleAllocation,
    ) -> Result<(), VehicleFactoryError> {
        if let Some(slot) = allocation.collision {
            if !self.collisions.free(slot) {
                return Err(VehicleFactoryError::InvalidFree);
            }
            allocation.collision = None;
        }
        Ok(())
    }

    /// Vehicle destructor82d2522c frees core before base destructors; factory
    /// DestroyEntity82d24b90 frees the object AFTER the complete destructor.
    pub(super) fn free_core(
        &mut self,
        allocation: &VehicleAllocation,
    ) -> Result<(), VehicleFactoryError> {
        if allocation.collision.is_some() {
            return Err(VehicleFactoryError::CollisionStillAllocated);
        }
        self.cores
            .free(allocation.core)
            .then_some(())
            .ok_or(VehicleFactoryError::InvalidFree)
    }
    pub(super) fn free_entity_slot(
        &mut self,
        allocation: &VehicleAllocation,
    ) -> Result<(), VehicleFactoryError> {
        self.entities
            .free(allocation.entity)
            .then_some(())
            .ok_or(VehicleFactoryError::InvalidFree)
    }
}
