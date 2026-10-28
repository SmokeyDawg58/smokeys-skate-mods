//! Original admitted board and real solved Rob's Park contacts. The cvar is an
//! explicit initializer-default fixture; no PhysOut/Actor timing claim is made.
use super::PhysicalAssemblyWorld;
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    simulation_assembly_backend::constraint_session::physical_board_surface::PhysicalPlayerBoardSurfaceError,
    skateboard_contact_surface::{SkateboardContactSurfaceCvars, SkateboardContactSurfaceError},
};
use std::convert::Infallible;

#[derive(Default)]
struct Cvars {
    reads: usize,
}
impl SkateboardContactSurfaceCvars for Cvars {
    type Error = Infallible;
    fn forced_world_surface(&mut self) -> Result<u32, Self::Error> {
        self.reads += 1;
        Ok(0) // Original initializer 0x82f7b7cc; controlled diagnostic service.
    }
}

pub(crate) fn empty_completed(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
) {
    assert!(world.contact_spies().unwrap().unwrap().records().is_empty());
    assert!(!world.physical_contact_list_has_entry(board));
    assert_eq!(world.physical_contact_list_entry_count(), 0);
    let mut cvars = Cvars::default();
    world
        .try_update_board_contact_surface(board, &mut cvars)
        .unwrap();
    assert_eq!(cvars.reads, 1);
    assert!(world.physical_contact_list_has_entry(board));
    assert_eq!(world.physical_contact_list_entry_count(), 1);
    world
        .try_with_board_force_runtime(board, |runtime| {
            let state = runtime.contact_surface.lock().unwrap();
            assert_eq!(state.overall(), Some(1));
            assert!(state.parts().iter().all(|part| !part.in_contact));
        })
        .unwrap();
}

pub(crate) fn before_frame(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
) {
    let mut cvars = Cvars::default();
    assert!(matches!(
        world.try_update_board_contact_surface(board, &mut cvars),
        Err(PhysicalPlayerBoardSurfaceError::MissingFrame)
    ));
    assert_eq!(cvars.reads, 0);
}

pub(crate) fn completed(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) {
    let mut cvars = Cvars::default();
    assert!(matches!(
        world.try_update_board_contact_surface(skeleton, &mut cvars),
        Err(PhysicalPlayerBoardSurfaceError::Authority(_))
    ));
    assert_eq!(cvars.reads, 0);
    // This independent oracle reads actual immutable contact records and actual
    // shared Entity owners. It never constructs a replacement contact list.
    let contacts = world.try_get_physical_collisions(board).unwrap();
    let records: Vec<_> = contacts
        .iter()
        .map(|c| {
            (
                c.part(),
                c.normal()[1],
                c.surface(),
                c.this_body().clone(),
                c.other_body().cloned(),
            )
        })
        .collect();
    let mut winners: [Option<(f32, u32)>; 7] = [None; 7];
    for (part, y, surface, this, other) in records {
        if let Some(other) = other {
            if world
                .island_body_owner_for_contacts(&other)
                .unwrap()
                .same_owner(&world.island_body_owner_for_contacts(&this).unwrap())
            {
                continue;
            }
        }
        let winner = &mut winners[usize::try_from(part).unwrap()];
        if winner.is_none_or(|old| y > old.0) {
            *winner = Some((y, (surface >> 7) & 31));
        }
    }
    let expected = (1_u32..=9)
        .fold((1, 0), |best, kind| {
            let count = winners[..4]
                .iter()
                .filter(|w| w.is_some_and(|(_, value)| value == kind))
                .count();
            if count > best.1 { (kind, count) } else { best }
        })
        .0;
    let before = world.backend().rigid_body_snapshot();
    world
        .try_update_board_contact_surface(board, &mut cvars)
        .unwrap();
    assert_eq!(cvars.reads, 1);
    assert_eq!(world.backend().rigid_body_snapshot(), before);
    world
        .try_with_board_force_runtime(board, |runtime| {
            let state = runtime.contact_surface.lock().unwrap();
            assert_eq!(state.overall(), Some(expected));
            for (actual, expected) in state.parts().iter().zip(winners) {
                assert_eq!(actual.in_contact, expected.is_some());
                if let Some((normal, kind)) = expected {
                    let actual = actual.winner.unwrap();
                    assert_eq!(actual.normal_y.to_bits(), normal.to_bits());
                    assert_eq!(actual.physics_type, kind);
                }
            }
        })
        .unwrap();
    assert!(matches!(
        world.try_update_board_contact_surface(board, &mut cvars),
        Err(PhysicalPlayerBoardSurfaceError::Selection(
            SkateboardContactSurfaceError::AlreadyAttempted
        ))
    ));
    assert_eq!(cvars.reads, 1);
}
