//! Actual same-board Part velocity removal for Ground entry82d8d090.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, board_entry::SkateboardEntrySessionError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate},
    skeleton_root_transforms::SkeletonRootVector,
};

#[derive(Debug)]
pub(crate) enum ManualVelocityError<N> {
    Entry(SkateboardEntrySessionError),
    Numeric(N),
    NormalNotUnit { length_bits: u32 },
    InvalidPart(usize),
}
impl<N: std::fmt::Debug> std::fmt::Display for ManualVelocityError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Entry(error) => write!(f, "manual velocity owner: {error}"),
            Self::Numeric(error) => write!(f, "manual velocity numeric: {error:?}"),
            Self::NormalNotUnit { length_bits } => write!(
                f,
                "manual normal source assertion: length bits {length_bits:08x}"
            ),
            Self::InvalidPart(part) => write!(f, "manual velocity source Part assertion: {part}"),
        }
    }
}

/// `RemovePartVelAlongNormalVector` 82b266c0 asserts 0.99 < length < 1.01.
/// It calculates one dot3 and conditionally two fresh rsqrt estimates; it does
/// not normalize the supplied normal. A failed source assertion stops stores.
fn require_unit_normal<N: ReckoningNumeric>(
    normal: [f32; 3],
    numeric: &mut N,
) -> Result<(), ManualVelocityError<N::Error>> {
    let lanes = [normal[0], normal[1], normal[2], 0.0];
    let squared = numeric
        .dot3(lanes, lanes)
        .map_err(ManualVelocityError::Numeric)?;
    let first = numeric
        .reciprocal_sqrt_estimate(squared)
        .map_err(ManualVelocityError::Numeric)?;
    let first = length_from_squared_and_estimate(squared, first);
    if first.partial_cmp(&0.99) != Some(std::cmp::Ordering::Greater) {
        return Err(ManualVelocityError::NormalNotUnit {
            length_bits: first.to_bits(),
        });
    }
    let second = numeric
        .reciprocal_sqrt_estimate(squared)
        .map_err(ManualVelocityError::Numeric)?;
    let second = length_from_squared_and_estimate(squared, second);
    if second.partial_cmp(&1.01) != Some(std::cmp::Ordering::Less) {
        return Err(ManualVelocityError::NormalNotUnit {
            length_bits: second.to_bits(),
        });
    }
    Ok(())
}

impl PhysicalAssemblyWorld {
    /// Source82b266c0 and `SetVelocityOfPart` 82b24a00, using current native
    /// Part observations twice, and preserving packed W via the existing setter.
    pub(crate) fn try_remove_board_part_velocity_along_normal<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        part: usize,
        normal: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), ManualVelocityError<N::Error>> {
        use ManualVelocityError as E;
        use SkateboardEntrySessionError as B;
        require_unit_normal(normal.xyz, numeric)?;
        let body = self
            .board(board)
            .map_err(|error| E::Entry(B::Pool(error)))?;
        if part >= 8 {
            return Err(E::InvalidPart(part));
        }
        let observed = body
            .try_part_contact_spy_observation(&self.backend, part)
            .map_err(|error| E::Entry(B::Part(error)))?;
        let velocity = self
            .backend
            .collision_body_pool()
            .try_contact_spy_linear_velocity(observed)
            .map_err(|error| E::Entry(B::Body(error)))?;
        let dot = numeric
            .dot3(
                [normal.xyz[0], normal.xyz[1], normal.xyz[2], 0.0],
                [velocity[0], velocity[1], velocity[2], 0.0],
            )
            .map_err(E::Numeric)?;
        let projected = normal.xyz.map(|lane| lane * dot);
        let result = [
            velocity[0] - projected[0],
            velocity[1] - projected[1],
            velocity[2] - projected[2],
        ];
        // Source recaptures its actual board body/Part inside SetVelocityOfPart.
        let observed = self
            .board(board)
            .map_err(|error| E::Entry(B::Pool(error)))?
            .try_part_contact_spy_observation(&self.backend, part)
            .map_err(|error| E::Entry(B::Part(error)))?;
        self.backend
            .try_set_part_linear_velocity(observed, result)
            .map_err(|error| E::Entry(B::Body(error)))
    }
}

/// Called by the original-data startup corpus on its actual retained pair.
/// Controlled velocity/normal exercise this operation without claiming a
/// completed animation/contact producer supplied the Manual branch inputs.
#[cfg(test)]
pub(crate) fn check_original_part_velocity(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
) {
    let deck = world
        .board(board)
        .unwrap()
        .try_part_contact_spy_observation(&world.backend, 6)
        .unwrap();
    let original = world
        .backend
        .collision_body_pool()
        .try_contact_spy_linear_velocity(deck)
        .unwrap();
    world
        .backend
        .try_set_part_linear_velocity(deck, [3.0, -4.0, 5.0])
        .unwrap();
    let before = world.backend.rigid_body_snapshot();
    let packed = world
        .backend
        .collision_body_pool()
        .try_packed_word_topology_snapshot(deck)
        .unwrap();
    let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
    // Source assertion must stop before touching any body, including deck.
    assert!(matches!(
        world.try_remove_board_part_velocity_along_normal(
            board,
            6,
            SkeletonRootVector::known([0.0; 4]),
            &mut numeric
        ),
        Err(ManualVelocityError::NormalNotUnit { .. })
    ));
    assert_eq!(world.backend.rigid_body_snapshot(), before);
    world
        .try_remove_board_part_velocity_along_normal(
            board,
            6,
            SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
            &mut numeric,
        )
        .unwrap();
    let mut expected = before;
    expected
        .slots
        .iter_mut()
        .find(|slot| slot.id == deck.identity().id())
        .unwrap()
        .velocity = [3.0, 0.0, 5.0];
    // Entire actual pool equality proves every other Part and packed neighbor
    // word, plus force/torque/omega/pose/time/counts/topology, stayed untouched.
    assert_eq!(world.backend.rigid_body_snapshot(), expected);
    assert_eq!(
        world
            .backend
            .collision_body_pool()
            .try_packed_word_topology_snapshot(deck)
            .unwrap(),
        packed
    );
    world
        .backend
        .try_set_part_linear_velocity(deck, original)
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_normal_assertion_rejects_zero_nan_and_nonunit_instead_of_normalizing() {
        let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
        for normal in [[0.0; 3], [f32::NAN, 0.0, 0.0], [2.0, 0.0, 0.0]] {
            assert!(matches!(
                require_unit_normal(normal, &mut numeric),
                Err(ManualVelocityError::NormalNotUnit { .. })
            ));
        }
        require_unit_normal([0.0, 1.0, -0.0], &mut numeric).unwrap();
    }
}
