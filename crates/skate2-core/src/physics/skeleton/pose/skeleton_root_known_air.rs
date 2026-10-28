//! Retained `KnownAir` roots, `UpdateRootTransformsKnownAir` 0x82b3b1c8.
use super::{SkeletonRootMatrix, SkeletonRootTransforms, SkeletonRootVector};
use crate::skateboard_spawn_math::{SkateboardSpawnMathNumeric, skateboard_spawn_normalize};

impl SkeletonRootTransforms {
    pub(crate) fn update_known_air<M: SkateboardSpawnMathNumeric>(
        &mut self,
        reckoning: &SkeletonRootMatrix,
        centre: [f32; 4],
        target: SkeletonRootVector,
        ortho: &mut M,
    ) {
        if self.update_heading {
            self.animation_angle = reckoning.inverse().compose(&self.animation_to_world);
            self.animation_angle
                .set_column(3, SkeletonRootVector::known([0.0; 4]));
            self.update_heading = false;
        }
        self.animation_to_world = reckoning.compose(&self.animation_angle);
        // Rotate current AnimRecord COM without translation: X multiply, Y FMA,
        // Z FMA, then subtract from the supplied world trajectory position.
        let axes = self.animation_to_world.columns();
        let first = axes[0].map(|v| v * centre[0]);
        let second = axes[1].zip(first, |v, old| v.mul_add(centre[1], old));
        let rotated = axes[2].zip(second, |v, old| v.mul_add(centre[2], old));
        let position = target.zip(rotated, |a, b| a - b);
        self.animation_to_world.set_column(3, position);
        let unknown_basis_w = self.animation_to_world.columns()[..3]
            .iter()
            .any(|axis| axis.w.is_none());
        let normalized = skateboard_spawn_normalize(&self.animation_to_world.xyz_scratch(), ortho);
        self.animation_to_world = SkeletonRootMatrix::known(normalized);
        if unknown_basis_w {
            for index in 0..3 {
                let mut column = self.animation_to_world.columns()[index];
                column.w = None;
                self.animation_to_world.set_column(index, column);
            }
        }
        self.animation_to_world.set_column(3, position);
        self.world_to_animation = self.animation_to_world.inverse();
    }
}

#[cfg(test)]
#[path = "../movement/skeleton_known_air_tests.rs"]
mod tests;
