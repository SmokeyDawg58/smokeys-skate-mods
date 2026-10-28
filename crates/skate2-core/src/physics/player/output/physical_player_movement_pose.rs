//! Movement animation publication into the existing Skate 2 Actor pose owner.
//! The normal Skate 2 writer still merges its hand/face buffers, records replay
//! data, runs character corrections and skins the selected Skate 2 recipe.
use super::{AnimOutPhysInOwner, Matrix, PoseBuffer, SkeletonNormalAssembly};

struct MovementRig<'a> {
    names: &'a [String],
    parents: &'a [i32],
    trajectory: Option<usize>,
    authored: &'a [Matrix],
    movement_names: &'a [String],
    movement_world: &'a [Matrix],
    world_to_animation: Matrix,
}

impl MovementRig<'_> {
    fn resolve(
        &self,
        bone: usize,
        globals: &mut [Option<Matrix>],
        visiting: &mut [bool],
    ) -> Result<Matrix, String> {
        if let Some(matrix) = globals[bone] {
            return Ok(matrix);
        }
        if visiting[bone] {
            return Err("Cyclic Skate 2 animation hierarchy".into());
        }
        visiting[bone] = true;
        let matrix = if let Some(source) = self
            .movement_names
            .iter()
            .position(|name| name.eq_ignore_ascii_case(&self.names[bone]))
        {
            self.world_to_animation
                .compose(&self.movement_world[source])
        } else if self.parents[bone] < 0
            || usize::try_from(self.parents[bone])
                .ok()
                .is_some_and(|parent| Some(parent) == self.trajectory)
        {
            self.authored[bone]
        } else {
            let parent = usize::try_from(self.parents[bone])
                .map_err(|_| "Invalid Skate 2 animation parent")?;
            if parent >= globals.len() {
                return Err("Skate 2 animation parent out of range".into());
            }
            self.resolve(parent, globals, visiting)?
                .compose(&self.authored[bone])
        };
        visiting[bone] = false;
        globals[bone] = Some(matrix);
        Ok(matrix)
    }
}

impl SkeletonNormalAssembly {
    /// Ported movement and completed physics enter the SAME Actor-owned
    /// AnimOutPhysIn allocation that the original presentation writer reads.
    /// A character-specific joint absent from the movement rig retains its
    /// actual animated local pose under its posed parent, never a bind pose.
    pub(crate) fn try_publish_mode_movement_pose(
        &self,
        pose: &mut AnimOutPhysInOwner,
        movement_names: &[String],
        movement_world: &[[f32; 16]],
        root: &Matrix,
    ) -> Result<(), String> {
        let data = &self.state.binding.body().data;
        let hierarchy = data.hierarchy();
        if movement_names.len() != movement_world.len() {
            return Err("Movement pose names and matrices differ in size".into());
        }
        if movement_world.iter().flatten().any(|v| !v.is_finite()) || root.has_nan_xyz() {
            return Err("Non-finite movement pose".into());
        }
        for &bone in data.bone_ids() {
            if !movement_names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&hierarchy.bone_names[bone]))
            {
                return Err(format!(
                    "Movement rig lacks physical bone {}",
                    hierarchy.bone_names[bone]
                ));
            }
        }
        let count = hierarchy.bone_names.len();
        let authored = (0..count)
            .map(|bone| pose.pose_matrix_typed(PoseBuffer::Local, bone))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        let source: Vec<_> = movement_world.iter().copied().map(Matrix::known).collect();
        let rig = MovementRig {
            names: &hierarchy.bone_names,
            parents: &hierarchy.parents,
            trajectory: hierarchy
                .has_trajectory
                .then(|| {
                    hierarchy
                        .bone_names
                        .iter()
                        .position(|name| name.eq_ignore_ascii_case("trajectory"))
                })
                .flatten(),
            authored: &authored,
            movement_names,
            movement_world: &source,
            world_to_animation: root.inverse(),
        };
        let mut globals = vec![None; count];
        let mut visiting = vec![false; count];
        for bone in 0..count {
            rig.resolve(bone, &mut globals, &mut visiting)?;
        }
        for bone in 0..count {
            let global = globals[bone].expect("resolved movement joint");
            pose.set_pose_matrix_typed(PoseBuffer::Global, bone, &global)
                .map_err(|error| error.to_string())?;
            let parent = hierarchy.parents[bone];
            let local = if parent < 0
                || usize::try_from(parent)
                    .ok()
                    .is_some_and(|parent| Some(parent) == rig.trajectory)
            {
                global
            } else {
                let parent = usize::try_from(parent).map_err(|_| "Invalid movement parent")?;
                globals
                    .get(parent)
                    .and_then(|value| *value)
                    .ok_or("Unresolved movement parent")?
                    .inverse()
                    .compose(&global)
            };
            pose.set_pose_matrix_typed(PoseBuffer::Local, bone, &local)
                .map_err(|error| error.to_string())?;
        }
        // The native writer's physical roots use the physical-parent table,
        // which is distinct from the full authored animation hierarchy.
        self.try_publish_physical_local_pose(pose)
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn translated(x: f32, y: f32) -> Matrix {
        Matrix::known([1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., x, y, 0., 1.])
    }
    #[test]
    fn movement_keeps_character_specific_animated_children_under_the_moving_parent() {
        let names = vec!["Hips".into(), "Head".into(), "Eye".into()];
        let source_names = vec!["HEAD".into(), "HIPS".into()];
        let authored = [translated(0., 1.), translated(0., 1.), translated(0.3, 0.2)];
        let source = [translated(10., 4.), translated(10., 3.)];
        let rig = MovementRig {
            names: &names,
            parents: &[-1, 0, 1],
            trajectory: None,
            authored: &authored,
            movement_names: &source_names,
            movement_world: &source,
            world_to_animation: translated(10., 2.).inverse(),
        };
        let mut globals = vec![None; 3];
        rig.resolve(2, &mut globals, &mut [false; 3]).unwrap();
        let eye = globals[2].unwrap().columns()[3].xyz;
        assert!((eye[0] - 0.3).abs() < 0.00001);
        assert!((eye[1] - 2.2).abs() < 0.00001);
    }
}
