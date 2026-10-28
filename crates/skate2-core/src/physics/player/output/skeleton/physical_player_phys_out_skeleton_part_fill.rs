//! Fresh part queries and ordered Skeleton stores, native 0x82b58618..0x82b589b4.
use super::{
    super::{PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut},
    ConcretePhysOutError,
};
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};
use std::convert::Infallible;

pub(crate) trait SkeletonPartFillSource {
    type Error;
    fn volume_to_bone_translation(&mut self, part: usize) -> Result<[f32; 4], Self::Error>;
    fn part_transform(&mut self, part: usize) -> Result<SkeletonRootMatrix, Self::Error>;
    fn hips_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum SkeletonPartFillError<S> {
    Source(S),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
}
impl<S: std::fmt::Debug> std::fmt::Display for SkeletonPartFillError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(error) => write!(f, "Skeleton part Fill source: {error:?}"),
            Self::Access(error) => write!(f, "Skeleton part Fill access: {error:?}"),
            Self::Component(error) => write!(f, "Skeleton part Fill component: {error:?}"),
        }
    }
}

/// Three source VMADDFP operations; the local W is neither read nor substituted.
fn bone_position(local: [f32; 4], matrix: &SkeletonRootMatrix) -> SkeletonRootVector {
    let columns = matrix.columns();
    let x = columns[0].zip(columns[3], |basis, p| local[0].mul_add(basis, p));
    let y = columns[1].zip(x, |basis, previous| local[1].mul_add(basis, previous));
    columns[2].zip(y, |basis, previous| local[2].mul_add(basis, previous))
}

impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_skeleton_parts<S: SkeletonPartFillSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), SkeletonPartFillError<S::Error>> {
        use SkeletonPartFillError as E;
        // These macros preserve the differing guard/read order of each native store.
        macro_rules! store {
            ($guard:expr, $field:ident, $value:expr) => {{
                self.check_write_guard(&$guard).map_err(E::Access)?;
                self.require_complete_component(PhysOutComponent::Skeleton)
                    .map_err(E::Component)?;
                self.concrete
                    .skeleton
                    .as_mut()
                    .expect("guarded Skeleton")
                    .$field = $value;
            }};
        }
        macro_rules! bone {
            ($part:expr, $field:ident) => {{
                let local = source
                    .volume_to_bone_translation($part)
                    .map_err(E::Source)?;
                let matrix = source.part_transform($part).map_err(E::Source)?;
                let guard = self.sample_write_guard().map_err(E::Access)?;
                let position = bone_position(local, &matrix);
                store!(guard, $field, position);
            }};
        }
        macro_rules! column {
            ($part:expr, $column:expr, $field:ident) => {{
                let matrix = source.part_transform($part).map_err(E::Source)?;
                let guard = self.sample_write_guard().map_err(E::Access)?;
                store!(guard, $field, matrix.columns()[$column]);
            }};
        }
        bone!(19, skeleton_foot_right_bone_pos); //58684
        bone!(15, skeleton_foot_left_bone_pos); //586f0
        column!(19, 0, skeleton_foot_right_look); //58734
        column!(15, 0, skeleton_foot_left_look); //58778
        column!(1, 1, skeleton_head_look); //587b8
        column!(1, 3, skeleton_head_pos); //587f8
        column!(23, 0, skeleton_hips_look); //58838
        column!(23, 1, skeleton_hips_mat_y); //5887c
        column!(23, 2, skeleton_hips_mat_z); //588c0
        column!(23, 3, skeleton_hips_pos); //58904
        column!(15, 3, skeleton_foot_left_pos); //58948
        column!(19, 3, skeleton_foot_right_pos); //58984
        let guard = self.sample_write_guard().map_err(E::Access)?; //58988
        let velocity = source.hips_velocity().map_err(E::Source)?; //5898c
        store!(guard, skeleton_hips_vel, velocity); //589b4
        Ok(())
    }
}

#[cfg(test)]
#[path = "physical_player_phys_out_skeleton_part_fill_tests.rs"]
mod tests;
