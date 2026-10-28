//! Live refs for the LivingWorld DMO locator stream.
use super::{InputCvarBooleanTarget, InputCvarError, InputCvarReferences};

impl InputCvarReferences {
    /// `Island::Step_Collision1` 0x827b3a20 samples the linked native switch.
    pub(crate) fn try_run_skater_skater_collision_pipeline(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1258)
    }

    pub(crate) fn try_draw_pedestrian_ground_contacts(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1070)
    }
    pub(crate) fn try_run_pedestrian_ground_collision_pipeline(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1071)
    }
    pub(crate) fn try_run_skater_pedestrian_collision_pipeline(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1072)
    }
    pub(crate) fn try_run_dmo_pedestrian_collision_pipeline(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1073)
    }
    pub(crate) fn try_run_pedestrian_pedestrian_collision_pipeline(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1074)
    }
    pub(crate) fn try_run_pedestrian_vehicle_collision_pipeline(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1075)
    }
    pub(crate) fn try_draw_lw_position_test(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(930)
    }
    pub(crate) fn try_draw_inside_vehicle_test(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(931)
    }
    pub(crate) fn try_inside_vehicle_x(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(932)
    }
    pub(crate) fn try_inside_vehicle_y_size(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(933)
    }
    pub(crate) fn try_inside_vehicle_z(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(934)
    }
    pub(crate) fn try_inside_vehicle_y_offset(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(935)
    }
    pub(crate) fn try_vehicle_avoidance_disabled(&self) -> Result<[bool; 3], InputCvarError> {
        Ok([
            self.try_current_boolean(927)?,
            self.try_current_boolean(928)?,
            self.try_current_boolean(929)?,
        ])
    }
    pub(crate) fn try_enable_spawn_vehicles(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(926)
    }
    /// CensusMan ctor82720a0c and SpawnVehicle8271c358 sample independently.
    pub(crate) fn try_initial_spawn_vehicle_count(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(925)
    }
    /// LWVehicleMan.Init captures HiFi/LoFi/NoFi values before group Init.
    pub(crate) fn try_vehicle_execution_rates(&self) -> Result<[u32; 3], InputCvarError> {
        Ok([
            self.try_current_integer(922)? as u32,
            self.try_current_integer(923)? as u32,
            self.try_current_integer(924)? as u32,
        ])
    }
    pub(crate) fn try_capture_living_world_generated_graph(
        &self,
    ) -> Result<InputCvarBooleanTarget, InputCvarError> {
        self.try_query_target(921).map(InputCvarBooleanTarget)
    }
    /// LWBehaviourSink.InitContainedProps82e615a0, source default true.
    pub(crate) fn try_allow_contained_props(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(920)
    }
    /// Spawn82d28284 reads this only when LWCommonSpawnInfo.mbForceSpawn is clear.
    pub(crate) fn try_vehicle_spawn_dmo_check_radius(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(919)
    }
    /// Ref830525b4/init82f6a210, Step_Collision1 827b3a70.
    pub(crate) fn try_run_skater_vehicle_collision_pipeline(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(917)
    }
    /// Ref83052644/init82f6a350, Step_UpdatePrePhysics2 827b2a28.
    pub(crate) fn try_run_dmo_vehicle_collision_pipeline(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(918)
    }

    pub(crate) fn try_vehicle_collision_density(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(914)
    }
    pub(crate) fn try_living_world_clamp_velocity(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(915)
    }
    pub(crate) fn try_living_world_clamp_max_velocity(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(916)
    }
    pub(crate) fn try_vehicle_pool_update_in_parallel(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(913)
    }
    pub(crate) fn try_vehicle_brake_light_delay(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(912)
    }
    pub(crate) fn try_check_vehicles_in_parking_lane(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(911)
    }
    pub(crate) fn try_override_intersection_rules(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(910)
    }
    pub(crate) fn try_ignore_world_painter_entity_categories(
        &self,
    ) -> Result<bool, InputCvarError> {
        self.try_current_boolean(908)
    }
    pub(crate) fn try_ignore_fe_census_sliders(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(909)
    }
}
impl InputCvarReferences {
    /// Ref83059028, init82f73128/table82fd2a20; sampled per kind-31 asset by
    /// `Simulation::RegDMOArena` 0x828a0420.
    pub(crate) fn try_load_dmo_locator_from_stream(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(827)
    }
    /// Ref830a71ec, init82f8c008/table82fd5484; `AddStreamedLocatorData`
    /// 0x82d3d570.
    pub(crate) fn try_enable_dmo_locator_loading(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(828)
    }
    /// Ref830a71c8, init82f8c3a0/table82fd54b0; `CreateLocator` 0x82d3ad6c.
    pub(crate) fn try_enable_the_hack_to_remove_bad_locators(
        &self,
    ) -> Result<bool, InputCvarError> {
        self.try_current_boolean(829)
    }
    /// Ref830a73d0 +0xcd: `BuildSkaterTestCapsule`/`AABBox` and
    /// `DoesCapsuleAndAABBoxIntersectDynamicObject` assert the DebugOverlay.
    pub(crate) fn try_show_spawn_test_capsule(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(830)
    }
    /// Ref830a70d0 +0xd8 (0x827041fc, 0x827044c0).
    pub(crate) fn try_spawn_test_capsule_width(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(831)
    }
    /// Ref830a7210 +0xd8 (0x82704278).
    pub(crate) fn try_spawn_test_capsule_height(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(832)
    }
    /// Ref830a771c +0xd8 (0x82704254).
    pub(crate) fn try_spawn_test_capsule_feet_padding(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(833)
    }
    /// Ref830a781c +0xd8 (0x82704504).
    pub(crate) fn try_spawn_test_actor_aabb_head_offset(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(834)
    }
    /// Ref830a73f4 +0xd8 (0x82722a84, 0x82725994).
    pub(crate) fn try_spawn_test_capsule_min_distance(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(835)
    }
    /// Ref8304ea58, `CensusMan::EvaluateCensus`.
    pub(crate) fn try_evaluate_census_for_one_component_per_frame(
        &self,
    ) -> Result<bool, InputCvarError> {
        self.try_current_boolean(836)
    }
    /// Ref8304ea7c, `CensusMan::EvaluateCensus`.
    pub(crate) fn try_evaluate_cull_for_all_components_per_frame(
        &self,
    ) -> Result<bool, InputCvarError> {
        self.try_current_boolean(837)
    }
    /// Ref8304eb24 (signed `mValue`): `EvaluativeSpawnDynamicObjects` and
    /// `SpawnDynamicObject` 0x82726c08; `SetConfManVariables` also writes it.
    pub(crate) fn try_max_dynamic_objects(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(838)
    }
    /// Ref8304eb50, `CensusMan::EvaluateCensus`.
    pub(crate) fn try_enable_cull(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(839)
    }
    /// Ref8304eb74, `CensusMan::EvaluateCensus`.
    pub(crate) fn try_enable_spawn(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(840)
    }
    /// Ref8304ec04, `EvaluateDynamicObjectCensus` 0x82726930.
    pub(crate) fn try_enable_spawn_dynamic_objects(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(841)
    }
    /// Ref8304ecdc, `EvaluativeSpawnDynamicObjects` 0x8271c690.
    pub(crate) fn try_close_by_locator_search_radius(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(842)
    }
    /// Ref830a62fc, `ModelMan::AddDMOModel`.
    pub(crate) fn try_enable_dmo_model_loading(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(843)
    }
    /// Ref830a74c0, `LWDynamicObjectUtil::GetSim/PresModelIDFromInstanecID`.
    pub(crate) fn try_enable_dmo_bank_id_lookup(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(844)
    }
    /// Ref830e102c, `LoadMainDMOBank` 0x82e0e628 and `IsMainDMOBankLoaded`.
    pub(crate) fn try_enable_main_dmo_bank(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(845)
    }
    /// Ref830523d0, `Island::Step_UpdatePrePhysics2` 0x827b294c.
    pub(crate) fn try_dmo_ground_verbose(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(846)
    }
    /// Ref830523f4, `Island::Step_UpdatePrePhysics2` 0x827b2948.
    pub(crate) fn try_draw_dmo_ground_triangles(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(847)
    }
    /// Ref83052418, `Island::Step_UpdatePrePhysics2` 0x827b2950.
    pub(crate) fn try_draw_dmo_ground_contacts(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(848)
    }
    /// Ref8305243c, the DMO ground pipeline's `numTrianglesToCache` in
    /// `Island::InitWorld`.
    pub(crate) fn try_dmo_ground_triangles_to_allocate(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(849)
    }
    /// Ref83052524, `Island::Step_UpdatePrePhysics2` 0x827b2904.
    pub(crate) fn try_run_dmo_ground_collision_pipeline(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(850)
    }
    /// Ref83052590, `Island::Step_Collision1`.
    pub(crate) fn try_run_skater_dmo_collision_pipeline(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(851)
    }
    /// Ref830525fc, `Island::Step_UpdatePrePhysics2` 0x827b2958.
    pub(crate) fn try_run_dmo_dmo_collision_pipeline(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(852)
    }
    /// Ref830526f8, the DMO pool's `updateInParallel` in `Island::InitWorld`.
    pub(crate) fn try_dmo_pool_update_in_parallel(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(853)
    }
    /// Ref83052cdc, `DynamicPool::AddVolume` 0x827c86fc.
    pub(crate) fn try_ignore_volume_enable_flag(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(854)
    }
}

#[cfg(test)]
mod tests {
    use crate::input_cvar_runtime::{InputCvarManager, InputCvarReferences};
    #[test]
    fn original_defaults_link_and_deserialize() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        assert!(refs.try_load_dmo_locator_from_stream().unwrap());
        assert!(refs.try_enable_dmo_locator_loading().unwrap());
        assert!(refs.try_enable_the_hack_to_remove_bad_locators().unwrap());
        manager
            .try_deserialize_startup(Some(
                b"[Config]\r\nkLoadDmoLocatorFromStream=0\r\nkEnableDMOLocatorLoading=0\r\nkEnableTheHackToRemoveBadLocators=0\r\n",
            ))
            .unwrap();
        assert!(!refs.try_load_dmo_locator_from_stream().unwrap());
        assert!(!refs.try_enable_dmo_locator_loading().unwrap());
        assert!(!refs.try_enable_the_hack_to_remove_bad_locators().unwrap());
    }

    #[test]
    fn spawn_test_and_census_defaults_match_the_initializers() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        assert!(!refs.try_show_spawn_test_capsule().unwrap());
        assert_eq!(
            refs.try_spawn_test_capsule_width().unwrap().to_bits(),
            0x3f33_3333
        );
        assert_eq!(
            refs.try_spawn_test_capsule_height().unwrap().to_bits(),
            0x4033_3333
        );
        assert_eq!(
            refs.try_spawn_test_capsule_feet_padding()
                .unwrap()
                .to_bits(),
            0
        );
        assert_eq!(
            refs.try_spawn_test_actor_aabb_head_offset()
                .unwrap()
                .to_bits(),
            0x3e4c_cccd
        );
        assert_eq!(refs.try_spawn_test_capsule_min_distance().unwrap(), 8.0);
        assert!(
            refs.try_evaluate_census_for_one_component_per_frame()
                .unwrap()
        );
        assert!(
            !refs
                .try_evaluate_cull_for_all_components_per_frame()
                .unwrap()
        );
        assert_eq!(refs.try_max_dynamic_objects().unwrap(), 49);
        assert!(refs.try_enable_cull().unwrap());
        assert!(refs.try_enable_spawn().unwrap());
        assert!(refs.try_enable_spawn_dynamic_objects().unwrap());
        assert_eq!(refs.try_close_by_locator_search_radius().unwrap(), 20.0);
        assert!(refs.try_enable_dmo_model_loading().unwrap());
        assert!(refs.try_enable_dmo_bank_id_lookup().unwrap());
        assert!(refs.try_enable_main_dmo_bank().unwrap());
    }

    #[test]
    fn dmo_physics_defaults_match_the_initializers() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        assert!(!refs.try_dmo_ground_verbose().unwrap());
        assert!(!refs.try_draw_dmo_ground_triangles().unwrap());
        assert!(!refs.try_draw_dmo_ground_contacts().unwrap());
        assert_eq!(refs.try_dmo_ground_triangles_to_allocate().unwrap(), 0);
        assert!(refs.try_run_dmo_ground_collision_pipeline().unwrap());
        assert!(refs.try_run_skater_dmo_collision_pipeline().unwrap());
        assert!(refs.try_run_dmo_dmo_collision_pipeline().unwrap());
        assert!(!refs.try_dmo_pool_update_in_parallel().unwrap());
        assert!(refs.try_ignore_volume_enable_flag().unwrap());
    }
}
