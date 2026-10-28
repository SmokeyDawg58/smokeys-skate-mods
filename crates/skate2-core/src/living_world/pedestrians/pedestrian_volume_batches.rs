//! Volume portion of LWPedestrianExecutionMan's retained query buffers.
use crate::{
    skeleton_root_transforms::SkeletonRootMatrix,
    collision_volume_line::{VolumeData, VolumeLineBatch},
    living_world_pedestrian_entity::PedestrianConstruction,
    pedestrian_navigation::rig::NavRigPhysicsRequest,
    reckoning_numeric::HostReckoningNumeric,
};

struct VolumeBuffer {
    batch: VolumeLineBatch,
    requests: Vec<NavRigPhysicsRequest>,
    accepting: bool,
}
pub(crate) struct PedestrianVolumeBatches {
    buffers: [VolumeBuffer; 2],
    send: usize,
}
impl PedestrianVolumeBatches {
    pub(crate) fn retire_rig(&mut self, id: u32) {
        for buffer in &mut self.buffers {
            for request in &mut buffer.requests {
                if request.pedestrian_id == id { request.result.submitted = false; }
            }
        }
    }
    /// ctor82e8a6d4/82e8a720: capacity80, nullMask0 on both sides.
    /// DoubleBuffer ctor82e7c1e0 initially sends B and receives A.
    pub(crate) fn new() -> Self {
        Self { buffers: std::array::from_fn(|_| VolumeBuffer {
            batch: VolumeLineBatch::new(80, 0), requests: Vec::with_capacity(80), accepting: false,
        }), send: 1 }
    }
    ///82e8693c..40. Caller visits ped, vehicle, DMO, skater, world-avoid
    /// volumes in that order before submitting this frame's entity tests.
    /// Disabled kPhysicsDoVolumeTests skips this entire creation flow.
    pub(crate) fn prepare_volumes(&mut self) {
        let buffer = &mut self.buffers[self.send];
        buffer.requests.clear();
        buffer.batch.clear_volumes();
        buffer.accepting = false;
    }
    /// Native begins line tests only after all volume source passes.
    pub(crate) fn begin(&mut self) {
        let buffer = &mut self.buffers[self.send];
        buffer.batch.begin();
        buffer.accepting = true;
    }
    pub(crate) fn add_volume(&mut self, volume: VolumeData) -> bool {
        self.buffers[self.send].batch.add_volume(volume)
    }
    pub(crate) fn add_vehicles(&mut self,
        world: &crate::living_world_grind_services::LivingWorldGrindServices,
    ) -> Result<(), String> {
        world.add_vehicle_avoidance_volumes(&mut self.buffers[self.send].batch)
            .map_err(|e| format!("pedestrian vehicle volumes: {e:?}"))
    }
    pub(crate) fn add_dmos(&mut self,
        objects: &crate::living_world_grind_services::dmo_objects::DmoObjectMan,
        vault: &std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        owner_mask: impl FnMut(u32) -> u32,
    ) -> Result<(), String> {
        objects.add_avoidance_volumes(&mut self.buffers[self.send].batch, vault, owner_mask)
            .map_err(|e| format!("pedestrian DMO volumes: {e:?}"))
    }
    /// LWPedestrianMan::AddEntityVolumesToBatch82e84f20. Wipeout comes from
    /// ITargetable::IsWipedOut, not the AI graph's state name.
    pub(crate) fn add_pedestrian(&mut self, entity: &PedestrianConstruction,
        wiped_out: bool, owner_mask: u32, velocity_scalar: f32, max_velocity_shift: f32,
    ) -> Result<bool, String> {
        let core = entity.core.as_ref().ok_or("pedestrian volume missing core")?;
        let mut rows = core.transform;
        let mut dimensions = entity.model.as_ref()
            .ok_or("pedestrian volume model absent")?.dimensions;
        dimensions[1] *= 0.5;
        if !wiped_out {
            dimensions[0] *= 0.6;
            dimensions[2] *= 0.6;
            let horizontal = [core.velocity[0], 0., core.velocity[2], 0.];
            let speed = crate::reckoning_numeric::try_length(horizontal, &mut HostReckoningNumeric)
                .map_err(|e| format!("pedestrian avoidance speed: {e:?}"))?;
            let requested = speed * velocity_scalar;
            let shift = crate::world_stream_adapter::vmx_min_ordinary(requested, max_velocity_shift) * 0.5;
            dimensions[2] += shift;
            rows[3][1] += dimensions[1];
            rows[3] = std::array::from_fn(|i| rows[2][i].mul_add(shift, rows[3][i]));
        }
        // Wiped-out branch82e8522c computes the lifted centre into a temporary
        // at sp+120, whereas AddVolume uses transform at sp+d0 (translation
        // sp+100). It does not publish that temporary. Preserve the binary.
        Ok(self.add_volume(VolumeData {
            transform: std::array::from_fn(|i| rows[i / 4][i % 4]),
            dimensions, mask: owner_mask,
        }))
    }
    ///82e869ec..86a84: actor root plus world Y0.9; half extents(.5,1,.5).
    /// owner_mask is the shared entity/actor identity token, not pedestrian ID.
    pub(crate) fn add_skater(&mut self, root: SkeletonRootMatrix, owner_mask: u32) -> bool {
        // Actor.GetRootTransform825ed2c0 supplies the same PhysOut root. Its
        // SIMD W lanes may be unknown; box intersection/avoidance distances
        // consume XYZ only (82ad72d4..7378, 827c05d8). Project only this query
        // snapshot, never manufacture known lanes in retained physics output.
        let mut root = root.xyz_scratch();
        root[13] += 0.9;
        self.add_volume(VolumeData { transform: root, dimensions: [0.5,1.,0.5,0.], mask: owner_mask })
    }
    ///82e86b54..c20: one entity appends requests, then submits them in order.
    /// Keep ownership separate: native mPedestrianId is GetID(), whereas
    /// mOwnerId is the entity pointer used by volume-mask exclusion.
    pub(crate) fn add_entity(&mut self, entity: &PedestrianConstruction,
        enabled: bool, owner_mask: u32, ignore_mask: u32,
        fatness: f32, y_clearance: f32, start_offset: f32,
    ) -> Result<bool, String> {
        let buffer = &mut self.buffers[self.send];
        if !buffer.accepting { return Ok(false); }
        let begin = buffer.requests.len();
        entity.add_volume_probes(enabled, ignore_mask, fatness, y_clearance, start_offset,
            &mut buffer.requests, 80, &mut HostReckoningNumeric)?;
        for request in &mut buffer.requests[begin..] {
            if !buffer.accepting { break; }
            let accepted = buffer.batch.add_test(request.start, request.result.end,
                request.fatness, owner_mask, request.ignore_volume_id);
            request.result.submitted = accepted;
            buffer.accepting = accepted;
        }
        Ok(buffer.accepting)
    }
    pub(crate) fn end(&mut self) -> Result<(), String> {
        self.buffers[self.send].batch.end(&mut HostReckoningNumeric)
            .map_err(|e| format!("pedestrian volume batch: {e:?}"))
    }
    /// UpdateCurrentFrame82e96ef8 swaps before ProcessVolume82e72940.
    /// Host End completes synchronously; no independent async owner is invented.
    /// ReceiveVolume82e72a60 consumes these requests through the existing rig.
    pub(crate) fn swap_buffers(&mut self) {
        self.send = 1 - self.send;
    }
    pub(crate) fn process_received(&mut self) -> Result<&[NavRigPhysicsRequest], String> {
        let receive = &mut self.buffers[1 - self.send];
        for (index, request) in receive.requests.iter_mut().enumerate() {
            if let Some(hit) = receive.batch.result(index) {
                // Same copy/length graph as ProcessGround; misses retain the
                // original endpoint/distance and still have submitted=true.
                request.apply_ground_hit(hit.position, hit.normal, hit.surface_id,
                    hit.owner, &mut HostReckoningNumeric)
                    .map_err(|e| format!("pedestrian volume hit distance: {e:?}"))?;
            }
        }
        Ok(&receive.requests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton_root_transforms::SkeletonRootVector;

    #[test]
    fn actor_avoidance_hit_does_not_require_physics_w_lanes() {
        let mut parameters = Vec::new();
        for w in [None, Some(0.), Some(f32::NAN)] {
            let xyz = [[0.,0.,-1.], [0.,1.,0.], [1.,0.,0.], [10.,2.,20.]];
            let root = SkeletonRootMatrix::from_columns(xyz.map(|xyz| SkeletonRootVector { xyz, w }));
            let mut batch = PedestrianVolumeBatches::new();
            batch.prepare_volumes();
            assert!(batch.add_skater(root, 7));
            let volume = &batch.buffers[batch.send].batch.volumes[0];
            assert_eq!(&volume.transform[12..15], &[10., 2.9, 20.]);
            let hit = crate::collision_box_line::intersect(volume,
                [8.,2.9,20.,0.], [12.,2.9,20.,0.], 0., &mut HostReckoningNumeric)
                .unwrap().expect("line crosses the skater avoidance box");
            parameters.push(hit.parameter);
            assert_eq!(root.columns()[3].w.is_none(), w.is_none());
        }
        assert_eq!(parameters, vec![0.375; 3]);
    }
}
