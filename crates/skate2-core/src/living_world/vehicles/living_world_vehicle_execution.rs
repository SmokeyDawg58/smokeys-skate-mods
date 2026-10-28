//! Vehicle specialization of the shared native LWExecutionMan owner.
use super::Vehicle;
use crate::{
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::{entities::EntityMan, entity_core::CommonEntityCore,
        execution::{ExecutionMan, ExecutionEntity}},
    reckoning_numeric::ReckoningNumeric,
};
pub(crate) use crate::living_world_grind_services::execution::{
    ExecutionError as VehicleExecutionError, ExecutionTarget as VehicleExecutionTarget,
    ExecutionTaskError as VehicleExecutionTaskError,
};
pub(crate) type VehicleExecutionMan = ExecutionMan<24, Vehicle>;
impl ExecutionEntity for Vehicle {
    fn execution_core(&self) -> Option<&CommonEntityCore> { Some(&self.core) }
    fn execution_core_mut(&mut self) -> Option<&mut CommonEntityCore> { Some(&mut self.core) }
}
impl ExecutionMan<24, Vehicle> {
    pub(crate) fn initialize(&mut self, cvars: &InputCvarReferences) -> Result<(), VehicleExecutionError> {
        self.initialize_with(|| cvars.try_vehicle_execution_rates())
    }
    /// Task_UpdatePositionalData82d1afa0. UpdatePreFrame82d28488 supplies
    /// local actor position, or the source zero vector when GetLocal is null.
    /// Invoke AFTER collect_preframe and BEFORE all state-graph tasks.
    pub(crate) fn update_positional_data<N: ReckoningNumeric>(
        vehicles: &mut EntityMan<24, Vehicle>,
        targets: &[VehicleExecutionTarget],
        local_actor_position: Option<[f32; 4]>,
        numeric: &mut N,
    ) -> Result<(), VehicleExecutionTaskError<N::Error>> {
        use VehicleExecutionTaskError as E;
        let player = local_actor_position.unwrap_or([0.; 4]);
        for target in targets {
            let vehicle = vehicles.get_mut(target.entity).ok_or(E::Execution(
                VehicleExecutionError::MissingEntity(target.entity),
            ))?;
            vehicle
                .update_positional_data(player, numeric)
                .map_err(E::Numeric)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living_world_grind_services::execution::{ExecutionGroup, sort_bucket_proxies};
    #[test]
    fn preframe_sorts_actual_distance_and_preserves_explicit_flag_rules() {
        use super::super::factory::tests::vehicle_fixture;
        use crate::reckoning_numeric::HostReckoningNumeric;
        let mut entities = EntityMan::<24, Vehicle>::try_construct().unwrap();
        entities.try_initialize().unwrap();
        let mut execution = VehicleExecutionMan::new();
        execution.groups.push(ExecutionGroup::initialize(30).unwrap());
        for (id, x) in [(1, -2.), (2, 2.), (3, 100.), (4, 1.)] {
            let mut vehicle = vehicle_fixture();
            vehicle.id = id;
            vehicle
                .core
                .set_transform::<std::convert::Infallible>([
                    [1., 0., 0., 0.],
                    [0., 1., 0., 0.],
                    [0., 0., 1., 0.],
                    [x, 0., 0., 0.],
                ])
                .unwrap();
            vehicle.core.explicit_update_requested = id == 1 || id == 2;
            entities.add(id, vehicle).unwrap();
            execution.add(0, id).unwrap();
        }
        let mut targets = Vec::new();
        execution
            .collect_preframe(
                &mut entities,
                [0.; 4],
                &mut targets,
                &mut HostReckoningNumeric,
            )
            .unwrap();
        assert_eq!(
            targets.iter().map(|t| t.entity).collect::<Vec<_>>(),
            [4, 2, 1]
        );
        assert!(!entities.get(2).unwrap().core.explicit_update_requested);
        assert!(entities.get(1).unwrap().core.explicit_update_requested);
        VehicleExecutionMan::update_positional_data(
            &mut entities,
            &targets,
            Some([10., 0., 0., 0.]),
            &mut HostReckoningNumeric,
        )
        .unwrap();
        assert_eq!(entities.get(4).unwrap().core.player_distance.magnitude, 9.);
        assert_eq!(entities.get(2).unwrap().core.player_distance.magnitude, 8.);
        assert_eq!(entities.get(1).unwrap().core.player_distance.magnitude, 12.);
        assert_eq!(entities.get(3).unwrap().core.player_distance.magnitude, 0.);
    }
    // UNRUN: reference admission ties depend on retained sort order.
    #[test]
    fn admission_keeps_equal_count_order_and_skips_current_bucket() {
        let mut group = ExecutionGroup::initialize(30).unwrap();
        for id in 1..=4 {
            group.add(id).unwrap();
        }
        assert_eq!(group.buckets[0].entities, [1, 4]);
        assert_eq!(group.buckets[1].entities, [2, 3]);
        group.advance();
        group.remove(1);
        group.remove(4);
        group.add(5).unwrap();
        assert!(group.buckets[0].entities.is_empty());
        assert_eq!(group.buckets[1].entities, [2, 3, 5]);
        group.advance();
        group.add(6).unwrap();
        assert_eq!(group.buckets[0].entities, [6]);
    }
    #[test]
    fn quick_partition_preserves_native_equal_key_permutation_above_cutoff() {
        let mut order: Vec<_> = (0..60).collect();
        sort_bucket_proxies(&mut order, &[0; 60]);
        // 60-way partition reverses all; each 30-way half reverses again.
        assert_eq!(order, (30..60).chain(0..30).collect::<Vec<_>>());
    }
    #[test]
    fn source_rates_create_two_six_twenty_buckets_and_exact_dt() {
        use crate::input_cvar_runtime::InputCvarManager;
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        let mut execution = VehicleExecutionMan::new();
        execution.initialize(&refs).unwrap();
        for (group, (count, dt)) in
            execution
                .groups
                .iter()
                .zip([(2, 1.0_f32 / 30.0), (6, 0.1), (20, 1.0_f32 / 3.0)])
        {
            assert_eq!(group.buckets.len(), count);
            assert_eq!(group.dt.to_bits(), dt.to_bits());
        }
        assert!(matches!(
            ExecutionGroup::initialize(0),
            Err(VehicleExecutionError::InvalidFrameRate(0))
        ));
        assert!(matches!(
            ExecutionGroup::initialize(60),
            Err(VehicleExecutionError::InvalidFrameRate(60))
        ));
    }
}
