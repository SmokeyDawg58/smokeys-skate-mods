//! Native intersection waiting targets: ctor82e6426c, Generate82e5a2b0,
//! Request82e4dfa8. Positions are accepted by the retained streamed NavMesh.
use super::{
    RainMan,
    network::{Intersection, RainNetworkError},
};
use crate::{
    living_world_mesh_store::MeshStore,
    reckoning_numeric::{HostReckoningNumeric, try_normalize_unsafe},
    skeleton_root_transforms::SkeletonRootVector,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct NavTarget {
    pub(crate) id: u32,
    /// The native empty constructor leaves these fields unwritten.
    pub(crate) position: Option<[f32; 4]>,
    pub(crate) entity_id: Option<u32>,
    pub(crate) occupied: bool,
}
pub(crate) struct NavTargetList {
    pub(crate) targets: [NavTarget; 16],
    pub(crate) count: usize,
    pub(crate) generated: bool,
}
impl NavTargetList {
    pub(crate) fn new() -> Self {
        Self {
            targets: std::array::from_fn(|id| NavTarget {
                id: id as u32,
                position: None,
                entity_id: None,
                occupied: true,
            }),
            count: 0,
            generated: false,
        }
    }
    pub(crate) fn request(&mut self) -> Option<&mut NavTarget> {
        let target = self.targets[..self.count]
            .iter_mut()
            .find(|t| !t.occupied)?;
        target.occupied = true;
        Some(target)
    }
    fn generate(
        &mut self,
        mesh: &MeshStore,
        root: [f32; 4],
        from: [f32; 4],
        to: [f32; 4],
    ) -> Result<(), RainNetworkError> {
        let root: [f32; 4] = std::array::from_fn(|i| (from[i] + to[i]).mul_add(0.5, root[i]));
        let from = from.map(|v| v * f32::from_bits(0x3fb33333));
        let to = to.map(|v| v * f32::from_bits(0x3fb33333));
        // Raw VMX82e5a3ac..428: from index increments after all four to offsets.
        for from_index in 0..4 {
            for to_index in 0..4 {
                let candidate = std::array::from_fn(|i| {
                    to[i].mul_add(to_index as f32, from[i].mul_add(from_index as f32, root[i]))
                });
                let cell = mesh
                    .try_get_cell(SkeletonRootVector::known(candidate))
                    .map_err(RainNetworkError::NavMesh)?;
                if cell.is_valid() && cell.score() == 0.0 {
                    let target = self
                        .targets
                        .get_mut(self.count)
                        .ok_or(RainNetworkError::NavTargetCapacity)?;
                    target.position = Some(candidate);
                    target.occupied = false;
                    self.count += 1;
                }
            }
        }
        self.generated = true;
        Ok(())
    }
}
impl Intersection {
    fn generate_nav_targets(&mut self, mesh: &MeshStore) -> Result<(), RainNetworkError> {
        if self.nav_targets_generated {
            return Ok(());
        }
        let corners = [
            self.record.vector(0)?,
            self.record.vector(16)?,
            self.record.vector(32)?,
            self.record.vector(48)?,
        ];
        let centre = std::array::from_fn(|i| {
            ((corners[0][i] + corners[1][i]) + corners[2][i] + corners[3][i]) * 0.25
        });
        if !mesh
            .try_get_cell(SkeletonRootVector::known(centre))
            .map_err(RainNetworkError::NavMesh)?
            .is_valid()
        {
            // Retry on a later getter after the missing tile streams in.
            return Ok(());
        }
        let mut numeric = HostReckoningNumeric;
        for i in 0..4 {
            let from: [f32; 4] =
                std::array::from_fn(|lane| corners[i][lane] - corners[(i + 1) & 3][lane]);
            let to: [f32; 4] =
                std::array::from_fn(|lane| corners[i][lane] - corners[(i + 3) & 3][lane]);
            // Source uses unsafe normalization, not a fabricated fallback axis.
            let from =
                try_normalize_unsafe(from, &mut numeric).map_err(RainNetworkError::Numeric)?;
            let to = try_normalize_unsafe(to, &mut numeric).map_err(RainNetworkError::Numeric)?;
            self.nav_targets[i].generate(mesh, corners[i], from, to)?;
        }
        self.nav_targets_generated = true;
        Ok(())
    }
}
impl RainMan {
    /// GetIntersection827155e0..15650 lazily generates targets on a found item.
    pub(crate) fn intersection(
        &mut self,
        id: u64,
        mesh: &MeshStore,
    ) -> Result<Option<&mut Intersection>, RainNetworkError> {
        let Some(intersection) = self.intersections.get_mut(&id) else {
            return Ok(None);
        };
        intersection.generate_nav_targets(mesh)?;
        Ok(Some(intersection))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn request_uses_first_free_generated_slot_and_does_not_invent_entity_identity() {
        let mut targets = NavTargetList::new();
        assert!(targets.request().is_none());
        targets.count = 3;
        targets.targets[1].occupied = false;
        targets.targets[2].occupied = false;
        let first = targets.request().unwrap();
        assert_eq!(first.id, 1);
        assert_eq!(first.entity_id, None);
        assert!(first.occupied);
        assert_eq!(targets.request().unwrap().id, 2);
        assert!(targets.request().is_none());
    }
    #[test]
    fn absent_navigation_does_not_accept_any_waiting_targets() {
        let mesh = MeshStore::construct();
        let mut targets = NavTargetList::new();
        targets
            .generate(&mesh, [0.; 4], [1., 0., 0., 0.], [0., 0., 1., 0.])
            .unwrap();
        assert!(targets.generated);
        assert_eq!(targets.count, 0);
        assert!(
            targets
                .targets
                .iter()
                .all(|target| target.position.is_none() && target.occupied)
        );
    }
}
