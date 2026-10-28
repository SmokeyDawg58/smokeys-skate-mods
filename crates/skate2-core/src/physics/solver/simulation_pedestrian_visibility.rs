//! Native geotest::LineOfSightIsClear82e4e578 over the same World line batches.
use crate::{physical_player_assembly::PhysicalAssemblyWorld,
    static_ground_pool::StaticGroundPool,reckoning_numeric::ReckoningNumeric,
    simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric,
    collision_line_collection::CollisionLineInput,skeleton_root_transforms::SkeletonRootVector};
impl PhysicalAssemblyWorld {
    pub(crate) fn try_pedestrian_line_of_sight<N:AssemblyGroundNumeric+ReckoningNumeric>(
        &mut self,ground:&StaticGroundPool,eye:[f32;4],target:[f32;4],numeric:&mut N
    )->Result<bool,String> where N::Error:std::fmt::Debug {
        // GetVisualIsland827b1b28 falls back to simulation when the separate
        // visual island is absent (the selected world's retained construction).
        // Island vtable820ffa10 slots68/72 both invoke827b2538 -> World BF7.
        self.try_pedestrian_visibility_line(ground,eye,target,"geotests::LineOfSightIsClear",numeric)
    }
    /// Shared BF_Default=7 query also used directly by transfer predicates.
    pub(crate) fn try_pedestrian_visibility_line<N:AssemblyGroundNumeric+ReckoningNumeric>(
        &mut self,ground:&StaticGroundPool,eye:[f32;4],target:[f32;4],label:&str,numeric:&mut N
    )->Result<bool,String> where N::Error:std::fmt::Debug {
        let queries=self.world_line_queries.as_mut().ok_or("perception World line queries absent")?;
        let dmos=self.dmos.as_ref().ok_or("perception World DMO pool absent")?;
        let vector=|v:[f32;4]|SkeletonRootVector{xyz:[v[0],v[1],v[2]],w:Some(v[3])};
        queries.try_line_test(ground,dmos,CollisionLineInput{start:vector(eye),end:vector(target),fatness:0.},
            label,numeric).map(|hit|hit.is_none())
            .map_err(|e|format!("perception line of sight: {e:?}"))
    }
}
