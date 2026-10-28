//! Actual board effects needed by `Ground.Exit` and `WipeoutGround.Enter`.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle,
        board_wipeout::{BoardWipeoutMaterialOperation, PoolBoardWipeoutError},
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalAssemblyWorld,
    simulation_assembly_backend::constraint_session::physical_island::SimulationPhysicalBodyError,
};

#[derive(Debug)]
pub(crate) enum PhysicalBoardWipeoutError {
    Owner(SimulationPhysicalBodyError),
    Pool(PoolBoardWipeoutError),
}
impl std::fmt::Display for PhysicalBoardWipeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(e) => write!(f, "wipeout physical board: {e}"),
            Self::Pool(e) => write!(f, "{e}"),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_set_board_free(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
    ) -> Result<(), PhysicalBoardWipeoutError> {
        self.try_board_wipeout_material_operation(
            board,
            BoardWipeoutMaterialOperation::SetFree(refs),
        )
    }

    pub(crate) fn try_set_board_wipeout(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PhysicalBoardWipeoutError> {
        self.try_board_wipeout_material_operation(board, BoardWipeoutMaterialOperation::SetWipeout)
    }
    pub(crate) fn try_restore_board_wheel_material(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PhysicalBoardWipeoutError> {
        self.try_board_wipeout_material_operation(
            board,
            BoardWipeoutMaterialOperation::RestoreWheels,
        )
    }
    /// Grind materials of `GrindMaterialsManager::Update` 0x82dd3178.
    pub(crate) fn try_set_board_grind_materials(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        grind: bool,
    ) -> Result<(), PhysicalBoardWipeoutError> {
        self.try_board_wipeout_material_operation(board, BoardWipeoutMaterialOperation::Grind(grind))
    }
    fn try_board_wipeout_material_operation(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        operation: BoardWipeoutMaterialOperation<'_>,
    ) -> Result<(), PhysicalBoardWipeoutError> {
        self.physical_body_owner_for_contacts(board)
            .map_err(PhysicalBoardWipeoutError::Owner)?;
        self.skaters
            .as_mut()
            .expect("registered physical owner retains pool")
            .try_board_wipeout_material_operation(&mut self.backend, board, operation)
            .map_err(PhysicalBoardWipeoutError::Pool)
    }
}

#[cfg(test)]
pub(crate) fn check_original(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
) {
    use crate::input_cvar_runtime::InputCvarManager;
    use std::sync::Arc;
    let construction = world.board_construction(board).unwrap().unwrap();
    let wipeout = construction.wipeout_material_owner_for_test();
    let wheel = construction.wheel_material_owner_for_test();
    let collision = Arc::clone(world.board(board).unwrap().template().collision());
    let deck = world
        .board(board)
        .unwrap()
        .try_part_contact_spy_observation(world.backend(), 6)
        .unwrap();
    let inertia = world.backend().try_capture_part_inertia(deck).unwrap();
    let before_inertia = inertia.try_read().unwrap();
    let revision = collision.volume_flags.read().unwrap().revision();
    world.try_set_board_wipeout(board).unwrap();
    for part in 0..7 {
        assert!(std::ptr::eq(
            world
                .board(board)
                .unwrap()
                .current_part_material(part)
                .unwrap(),
            Arc::as_ref(&wipeout)
        ));
    }
    let mut after_inertia = inertia.try_read().unwrap();
    assert_eq!(after_inertia.angular_drag.to_bits(), 0.0_f32.to_bits());
    after_inertia.angular_drag = before_inertia.angular_drag;
    assert_eq!(after_inertia, before_inertia);
    assert!(
        world
            .board_construction(board)
            .unwrap()
            .unwrap()
            .post_fields()
            .unwrap()
            .free_wheel_enabled
    );
    let flags = collision.volume_flags.read().unwrap();
    assert_eq!(
        flags.revision() - revision,
        3 + u64::try_from(flags.deck_count()).unwrap()
    );
    for emission in 0..6 + flags.deck_count() {
        assert_ne!(flags.emission_flags(emission).unwrap() & 1, 0);
    }
    drop(flags);
    world.try_restore_board_wheel_material(board).unwrap();
    for part in 0..7 {
        let expected = if part < 4 { &wheel } else { &wipeout };
        assert!(std::ptr::eq(
            world
                .board(board)
                .unwrap()
                .current_part_material(part)
                .unwrap(),
            Arc::as_ref(expected)
        ));
    }
    assert!(
        world
            .board_construction(board)
            .unwrap()
            .unwrap()
            .post_fields()
            .unwrap()
            .free_wheel_enabled
    );
    assert_eq!(
        inertia.try_read().unwrap().angular_drag.to_bits(),
        0.0_f32.to_bits()
    );
    let refs = InputCvarReferences::new_static_initialized();
    let mut cvars = InputCvarManager::new();
    cvars.try_link(&refs).unwrap();
    assert_eq!(refs.try_free_skateboard_angular_damping().unwrap(), 0.25);
    let free_revision = collision.volume_flags.read().unwrap().revision();
    world.try_set_board_free(board, &refs).unwrap();
    assert_eq!(
        inertia.try_read().unwrap().angular_drag.to_bits(),
        (0.25_f32 * f32::from_bits(0x426f_ffff)).to_bits()
    );
    for part in 0..7 {
        assert!(std::ptr::eq(
            world
                .board(board)
                .unwrap()
                .current_part_material(part)
                .unwrap(),
            Arc::as_ref(&wipeout)
        ));
    }
    let flags = collision.volume_flags.read().unwrap();
    assert_eq!(
        flags.revision() - free_revision,
        3 + u64::try_from(flags.deck_count()).unwrap()
    );
    drop(flags);
    cvars
        .try_deserialize_startup(Some(b"[Config]\r\nkFreeSkateboardAngularDamping=0.5\r\n"))
        .unwrap();
    world.try_set_board_free(board, &refs).unwrap();
    assert_eq!(
        inertia.try_read().unwrap().angular_drag.to_bits(),
        (0.5_f32 * f32::from_bits(0x426f_ffff)).to_bits()
    );
    assert!(Arc::ptr_eq(
        world.board(board).unwrap().template().collision(),
        &collision
    ));
}
