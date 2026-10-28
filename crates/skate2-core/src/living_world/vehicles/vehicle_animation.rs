//! UpdateAnimationPoses82809610 over cPresVehicle's procedural wheel rotations,
//! which UpdateFromReplayData82807e88 restores (`presentation_restore::living_world`).
//! Reuses the existing Andale SQT arithmetic, hierarchy and palette packing.
use crate::{
    actor_animation_construction::initialization::body_dispatch::tree_commands::numeric,
    animation::{AnimationSkeleton, LocalPose, compose_pose_matrix},
    character::{CharacterHierarchy, CharacterScene, palette::CharacterPalette},
    presentation_restore::living_world::VehicleWheels,
};
use skate2_formats::andale::{AndaleDatabase, AndaleSqt};
use std::sync::Arc;

pub struct VehicleAnimation {
    skeleton: AnimationSkeleton,
    bind: Vec<AndaleSqt>,
    ordered: [usize; 8],
    pub palette: CharacterPalette,
}
impl VehicleAnimation {
    pub fn new(
        db: &AndaleDatabase,
        name: &str,
        scene: Arc<CharacterScene>,
        lod: u8,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let record = db
            .records
            .iter()
            .find(|r| {
                r.header.data_type == 3
                    && r.header
                        .name
                        .decode()
                        .is_ok_and(|n| n.eq_ignore_ascii_case(name))
            })
            .ok_or_else(|| format!("VehiclesDB pose {name:?} absent"))?;
        let pose = db
            .poses
            .iter()
            .find(|p| p.record_offset == record.offset)
            .ok_or("VehiclesDB pose record absent")?;
        let bind = pose.sample_raw_sqt(&db.hierarchy)?;
        let mut ordered = [0; 8];
        for (slot, name) in ordered.iter_mut().zip([
            "Vehicle_Root",
            "Chassis",
            "LeftFront_wheel",
            "RightFront_wheel",
            "LeftRear_wheel_1",
            "RightRear_wheel_1",
            "LeftRear_wheel_2",
            "RightRear_wheel_2",
        ]) {
            *slot = db
                .hierarchy
                .bone_names
                .iter()
                .position(|b| b.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("vehicle required bone {name:?} absent"))?;
        }
        let mut skeleton = AnimationSkeleton::from_andale(&db.hierarchy)?;
        // InitVehicleAnimSys82810cc4 uses NewACS824d62c0. Its Skeleton
        // constructor passes trajNode=0 unconditionally (li r5,0 at824d6340),
        // independently of FetchSys::mHasTrajectoryBone or the bone's name.
        // PoseWithMatMult82a23944..54 copies children of this node as roots.
        // Keep the authored Vehicle_Root scale; it must NOT scale Chassis.
        skeleton.trajectory_bone = Some(0);
        Ok(Self {
            skeleton,
            bind,
            ordered,
            palette: CharacterPalette::with_default_bone(
                scene,
                &CharacterHierarchy {
                    bone_names: db.hierarchy.bone_names.clone(),
                },
                lod,
                "Chassis",
            )?,
        })
    }
    /// The front wheel bones take `wheels.front` and the rear ones `wheels.spin`, over
    /// the bind pose, packed at `world`.
    pub fn update(
        &mut self,
        wheels: &VehicleWheels,
        world: [f32; 16],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut procedural = vec![
            AndaleSqt {
                scale: [1.; 4],
                rotation: [0., 0., 0., 1.],
                translation: [0.; 4]
            };
            self.bind.len()
        ];
        for &bone in &self.ordered[2..4] {
            procedural[bone].rotation = wheels.front;
        }
        for &bone in &self.ordered[4..8] {
            procedural[bone].rotation = wheels.spin;
        }
        // SetSQT(procedural), SetSQT(bind), AddSQT(true,true): same operand order.
        let pose = LocalPose {
            bones: procedural
                .into_iter()
                .zip(&self.bind)
                .map(|(p, &b)| numeric::add_sqt(p, b, true).into())
                .collect(),
        };
        let global = self.skeleton.global_matrices(&pose)?;
        let skin = global
            .into_iter()
            .zip(self.palette.skinning().hierarchy_inverse_bind_pose.iter())
            .map(|(g, &ibp)| compose_pose_matrix(g, ibp))
            .collect::<Vec<_>>();
        self.palette.update_bone_transforms(&skin)?;
        self.palette.pack_for_upload(world)?;
        Ok(())
    }
}
