//! tAnimData+6490 SimLODBoneGlobal[9]. Native MemStore zeroes this allocation
//! before construction; UpdateSimLODGlobal replaces it after animation sync.
//! Initial zero matrices are pool contents, not an identity or bind pose.
pub(crate) struct SimLodBoneGlobal {
    pub(crate) matrices: [[[f32; 4]; 4]; 9],
}
impl SimLodBoneGlobal {
    /// CollectNearbyPedestriansForScoring827ac978..ca0c. Preserve each VMX
    /// multiply, fused torso add and separate legs subtract (including W).
    pub(crate) fn scoring_endpoints(&self) -> ([f32; 4], [f32; 4]) {
        let head = self.matrices[0][3];
        let hips = self.matrices[7][3];
        let hip_axis = self.matrices[7][0];
        // 6650 is bone7's first axis, not the second/Y axis: 6490+7*40.
        let torso = std::array::from_fn(|i| head[i].mul_add(0.5, hips[i] * 0.5));
        let legs = std::array::from_fn(|i| hips[i] - hip_axis[i] * 0.5);
        (torso, legs)
    }
}
