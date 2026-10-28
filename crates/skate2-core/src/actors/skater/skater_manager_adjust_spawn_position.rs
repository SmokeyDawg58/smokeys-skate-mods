//! `SkaterManager::AdjustSpawnPosition`825f1a20, shared by the host boot spawn
//! and `OnCommand` teleports. The result starts as a copy of the transform.
use crate::{
    collision_line_collection::CollisionLineInput,
    collision_line_numeric::HostCollisionLineNumeric,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::ReckoningHostNumericError,
    simulation_assembly_backend::constraint_session::{
        physical_world_line::PhysicalWorldLineError, world_services::PhysicalWorldServicesError,
    },
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};

#[derive(Debug)]
pub(crate) enum AdjustSpawnPositionError {
    MissingWorldServices,
    Network(PhysicalWorldServicesError),
    Cvar(InputCvarError),
    Line(PhysicalWorldLineError<ReckoningHostNumericError>),
    UnknownHitW,
}

/// Returns the replacement translation, or `None` when the copy is unchanged.
pub(crate) fn try_adjust_spawn_position(
    world: &mut PhysicalAssemblyWorld,
    ground: &StaticGroundPool,
    cvars: &InputCvarReferences,
    translation: [f32; 4],
) -> Result<Option<[f32; 4]>, AdjustSpawnPositionError> {
    use AdjustSpawnPositionError as E;
    let network_game = world
        .physical_world_services()
        .ok_or(E::MissingWorldServices)?
        .try_is_network_game()
        .map_err(E::Network)?;
    // 0x825F1AE8..0x825F1AFC: network games bypass the cvar; offline games
    // read its linked value.
    if !network_game && !cvars.try_enable_adjust_spawn_position().map_err(E::Cvar)? {
        return Ok(None);
    }
    // VMX128 0x825F1B1C..0x825F1B24 raises the full translation vector, then
    // subtracts 500 from that raised start.
    let start = std::array::from_fn(|lane| translation[lane] + [0.0, 0.75, 0.0, 0.0][lane]);
    let end = std::array::from_fn(|lane| start[lane] - [0.0, 500.0, 0.0, 0.0][lane]);
    let hit = world
        .try_world_line_test(
            ground,
            CollisionLineInput {
                start: SkeletonRootVector::known(start),
                end: SkeletonRootVector::known(end),
                fatness: 0.0,
            },
            "SkaterManager::AdjustSpawnPosition",
            &mut HostCollisionLineNumeric,
        )
        .map_err(E::Line)?;
    let Some(hit) = hit else {
        return Ok(None);
    };
    let position = hit.position.known_lanes().ok_or(E::UnknownHitW)?;
    // 0x825F1B8C..0x825F1B98 adds the offset to the complete VolumeLineResult
    // position and stores all four lanes.
    Ok(Some(std::array::from_fn(|lane| {
        position[lane] + [0.0, f32::from_bits(0x3e19_999a), 0.0, 0.0][lane]
    })))
}
