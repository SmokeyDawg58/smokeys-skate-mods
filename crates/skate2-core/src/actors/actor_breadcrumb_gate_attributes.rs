//! Live `skater_breadcrumbs.default` scalar reads for respawn gates.
//! The generated accessors and direct GetAttributePointer sites all use index 0;
//! a null pointer falls through to the native four-byte zero DefaultDataArea.
use super::ActorBreadCrumbManager;
use crate::attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BreadCrumbFloatAttribute {
    /// Direct lookup in `Implementation::Update` 0x82ce232c; squared at 2354.
    UpdateMinimumDistance,
    /// Direct lookup at `_TestForCollisionConstraints` 0x82ce1ab4.
    CollisionNormalMinY,
    /// Direct lookup at `_TestForCollisionConstraints` 0x82ce1b28.
    CollisionMaxVerticalDistance,
    CapsuleHeightForCollisionTest,
    CapsuleRadiusForCollisionTest,
    MaxNavMeshScoreForBreadCrumb,
    MinDistToPedestriansForRespawn,
    MinDistToVehicleForRespawn,
}

impl BreadCrumbFloatAttribute {
    pub(crate) fn try_read(
        self,
        attributes: &AttributeRuntimeInstance,
    ) -> Result<f32, AttributeRuntimeError> {
        let word = attributes.try_scalar_word(self.key(), 0)?.unwrap_or(0);
        Ok(f32::from_bits(word))
    }

    pub(crate) const fn key(self) -> u64 {
        match self {
            Self::UpdateMinimumDistance => 0x6884_00d1_3771_e625,
            Self::CollisionNormalMinY => 0xadd0_32ca_cf6a_1c15,
            Self::CollisionMaxVerticalDistance => 0x8abe_098d_3806_d273,
            Self::CapsuleHeightForCollisionTest => 0xc052_6c88_3af0_ecca,
            Self::CapsuleRadiusForCollisionTest => 0xceb0_92e4_18a5_b001,
            Self::MaxNavMeshScoreForBreadCrumb => 0x2d56_7511_a6f1_90eb,
            Self::MinDistToPedestriansForRespawn => 0xe64c_980e_e211_4070,
            Self::MinDistToVehicleForRespawn => 0x59e1_f2b3_cdc4_e658,
        }
    }
}

/// `_IsSurfaceOK` 0x82ce12d0 accepts a ushort enum, not a packed hit ID.
pub(crate) fn breadcrumb_surface_ok(surface: u16) -> bool {
    !matches!(surface, 5 | 6 | 8 | 9)
}

impl ActorBreadCrumbManager {
    /// Generated getter82ce1288 and direct offboard lookup82ce195c use
    /// the same signed word at index zero; null uses DefaultDataArea(4).
    pub(crate) fn try_min_ticks_in_state(&self) -> Result<i32, AttributeRuntimeError> {
        Ok(self
            .attributes
            .try_scalar_word(0x10b7_c3a9_cc8d_3721, 0)?
            .unwrap_or(0) as i32)
    }

    /// Samples the retained native instance on every reached gate read.
    pub(crate) fn try_float_attribute(
        &self,
        attribute: BreadCrumbFloatAttribute,
    ) -> Result<f32, AttributeRuntimeError> {
        attribute.try_read(&self.attributes)
    }

    /// `_TestForCollisionConstraints` 0x82ce1af8..1b08 rejects when the
    /// minimum Y normal is greater than the hit normal's Y lane.
    pub(crate) fn try_reject_collision_normal(
        &self,
        normal_y: f32,
    ) -> Result<bool, AttributeRuntimeError> {
        Ok(self.try_float_attribute(BreadCrumbFloatAttribute::CollisionNormalMinY)? > normal_y)
    }

    /// `_TestForCollisionConstraints` 0x82ce1b5c..1b70 compares the downward
    /// start-to-hit Y distance strictly against the live maximum.
    pub(crate) fn try_reject_collision_distance(
        &self,
        start_y: f32,
        hit_y: f32,
    ) -> Result<bool, AttributeRuntimeError> {
        Ok(start_y - hit_y
            > self.try_float_attribute(BreadCrumbFloatAttribute::CollisionMaxVerticalDistance)?)
    }

    /// `_TestForNavMeshConstraints` 0x82ce1574..1580, reached only after a
    /// valid cell. The PPC `fcmpu`/`bge` path also rejects unordered values.
    pub(crate) fn try_accept_navmesh_cell_score(
        &self,
        cell_score: f32,
    ) -> Result<bool, AttributeRuntimeError> {
        Ok(cell_score
            < self.try_float_attribute(BreadCrumbFloatAttribute::MaxNavMeshScoreForBreadCrumb)?)
    }
}

#[cfg(test)]
mod tests {
    use super::BreadCrumbFloatAttribute as A;
    use crate::{
        actor_construction::breadcrumbs::CLASS,
        attribute_runtime_corpus::mounted_physics_attributes_with_classes,
    };

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn mounted_breadcrumb_class_binds_original_gate_nodes() {
        let vault = mounted_physics_attributes_with_classes(&[CLASS]);
        let instance = vault.instance(CLASS, 0xd7ed_bd36_2d7d_2152);
        let expected = [
            (A::CollisionNormalMinY, 0x3f40_0000),
            (A::CollisionMaxVerticalDistance, 0x3f80_0000),
            (A::CapsuleHeightForCollisionTest, 0x3ecc_cccd),
            (A::CapsuleRadiusForCollisionTest, 0x3f33_3333),
            (A::MaxNavMeshScoreForBreadCrumb, 0x447a_0000),
            (A::MinDistToPedestriansForRespawn, 0x3f80_0000),
            (A::MinDistToVehicleForRespawn, 0x4000_0000),
        ];
        for (attribute, word) in expected {
            assert_eq!(
                instance.try_scalar_word(attribute.key(), 0).unwrap(),
                Some(word)
            );
        }
        // Update 0x82ce2320 inserts r8 into the HIGH half of r4. Check that
        // this corrected direct key binds a shipped scalar on the same class.
        assert!(
            instance
                .try_scalar_word(A::UpdateMinimumDistance.key(), 0)
                .unwrap()
                .is_some()
        );
    }
}
