//! Controlled ordinary-state continuation on the same original paired owners
//! and Rob's Park terrain. Physical toolkit reads and raw hierarchy attribute
//! lookup produce Contact inputs; this is not native Actor scheduling.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    dynamic_assembly_session::{
        skater_pool::SimulationSkaterAssemblyHandle,
        skeleton_normal::general_owner::{
            SkeletonGeneralContinuation, SkeletonGeneralCvars, SkeletonGeneralInput,
            SkeletonGroundedPassedOutput,
        },
    },
    reckoning::Reckoning,
    reckoning_attributes::ReckoningVaultAttributes,
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skateboard_assembly::{SkateboardAssemblySession, SkateboardSimulationBackend},
    skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_drives::update::SkeletonDrivesUpdateCvars,
    skeleton_ik::SkeletonIkSuffixServices,
    skeleton_ik_prephysics::SkeletonIkPrePhysicsServices,
    skeleton_ik_state::{
        contact::{
            SkeletonIkContactLineQuery, SkeletonIkContactLineResult, SkeletonIkContactProcessed,
            SkeletonIkContactServices,
        },
        drive_frames::SkeletonIkDriveFramesServices,
    },
    skeleton_processed_contact::{SkeletonProcessedContact, SkeletonProcessedContactBoard},
    skeleton_root_transforms::SkeletonRootMatrix,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

struct ActualBoard<'a> {
    board: &'a SkateboardAssemblySession,
    backend: &'a SkateboardSimulationBackend,
    reads: Vec<&'static str>,
}

/// Explicit prior-motion/flip fixture. Toolkit directions read the real Part 6
/// prefix; this does not replace the still-required native `PhysOut` publisher.
#[derive(Default)]
struct ToolkitMotion {
    reads: Vec<&'static str>,
}

struct ToolkitGround(crate::skeleton_root_transforms::SkeletonRootVector);
impl crate::skeleton_processed_contact::toolkit::SkeletonProcessedToolkitJumpInputRead
    for ToolkitGround
{
    type Error = std::convert::Infallible;
    fn ground_normal(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, Self::Error> {
        Ok(self.0)
    }
}
struct ToolkitJumpDefaults;
impl crate::skeleton_processed_contact::toolkit::SkeletonProcessedToolkitJumpCvars
    for ToolkitJumpDefaults
{
    type FloatTarget = crate::skeleton_processed_contact::toolkit::ToolkitJumpFloat;
    type BoolTarget = ();
    type Error = std::convert::Infallible;
    fn capture_float(
        &mut self,
        field: Self::FloatTarget,
    ) -> Result<Self::FloatTarget, Self::Error> {
        Ok(field)
    }
    fn float(&mut self, field: &Self::FloatTarget) -> Result<f32, Self::Error> {
        use crate::skeleton_processed_contact::toolkit::ToolkitJumpFloat;
        // Source cvar initializer words; this is a diagnostic read service.
        Ok(f32::from_bits(match field {
            ToolkitJumpFloat::MaxCosAngleDelta => 0x3f6f_5c29,
            ToolkitJumpFloat::ProportionalBlend => 0x3dcc_cccd,
        }))
    }
    fn capture_draw_jump(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn draw_jump(&mut self, (): &()) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("original draw-jump initializer is false")
    }
}
impl crate::skeleton_processed_contact::toolkit::SkeletonProcessedToolkitInputRead
    for ToolkitMotion
{
    type Error = std::convert::Infallible;
    fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
        self.reads.push("flags");
        Ok(0)
    }
    fn z_speed(&mut self) -> Result<f32, Self::Error> {
        self.reads.push("prior zSpeed");
        Ok(3.0)
    }
}
impl SkeletonProcessedContactBoard for ActualBoard<'_> {
    type Error = String;
    fn mass(&mut self) -> Result<f32, Self::Error> {
        self.reads.push("mass");
        crate::skateboard_body_mass::try_skateboard_body_mass(self.board, self.backend)
            .map_err(|error| format!("actual board mass: {error:?}"))
    }
    fn deck_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.reads.push("Part6");
        self.board
            .try_live_part_transform_host(self.backend, 6)
            .map(SkeletonRootMatrix::known)
            .map_err(|error| error.to_string())
    }
    fn flags_8c4(&self) -> u32 {
        assert_eq!(self.reads, ["mass", "Part6", "Part6"]);
        0 // Explicit controlled processed BoardFlipped=false input.
    }
}

/// Source initializer defaults, independently checked against debug XEX.
/// These are explicit diagnostic reads, not a replacement runtime cvar owner.
#[derive(Default)]
struct Cvars {
    events: Vec<&'static str>,
    damping_reads: usize,
}
impl SkeletonDrivesUpdateCvars for Cvars {
    type Error = &'static str;
    fn offboard_drive_scalar(&mut self) -> Result<f32, Self::Error> {
        Ok(2.0)
    }
    fn soft_damping(&mut self) -> Result<f32, Self::Error> {
        self.events.push("drives");
        self.damping_reads += 1;
        Ok(200.0)
    }
}
impl SkeletonGeneralCvars for Cvars {
    fn ik_draw_feet_on_deck(&mut self) -> Result<u8, &'static str> {
        self.events.push("initial");
        Ok(0)
    }
    fn ik_require_debug_overlay(&mut self) -> Result<(), &'static str> {
        panic!("draw disabled")
    }
    fn ik_maximum_external_normal_angle(&mut self) -> Result<f32, &'static str> {
        panic!("no external targets")
    }
    fn ik_target_normal_blend_speed(&mut self) -> Result<f32, &'static str> {
        panic!("no external targets")
    }
    fn ik_maximum_local_delta(&mut self) -> Result<f32, &'static str> {
        panic!("no external targets")
    }
    fn ik_maximum_world_offset(&mut self) -> Result<f32, &'static str> {
        panic!("no external targets")
    }
}
impl SkeletonIkContactServices for Cvars {
    type Error = &'static str;
    fn draw_ik_contact_debug(&mut self) -> Result<bool, Self::Error> {
        self.events.push("contact");
        Ok(false)
    }
    fn max_ik_contact_penetration_offset(&mut self) -> Result<f32, Self::Error> {
        Ok(0.3)
    }
    fn sideways_offset(&mut self) -> Result<f32, Self::Error> {
        self.events.push("line");
        Ok(f32::from_bits(0x3e8f_5c29))
    }
    fn sideways_offset_draw(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn max_ik_contact_offset(&mut self) -> Result<f32, Self::Error> {
        Ok(0.3)
    }
    fn toe_contact_threshold(&mut self) -> Result<f32, Self::Error> {
        Ok(f32::from_bits(0x3f33_3333))
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("draw disabled")
    }
    fn try_line_query(
        &mut self,
        _: &SkeletonIkContactLineQuery,
    ) -> Result<SkeletonIkContactLineResult, Self::Error> {
        panic!("actual selected World owns the line query")
    }
}
impl SkeletonIkPrePhysicsServices for Cvars {
    type Error = &'static str;
    fn draw_pre_feet_ik(&mut self) -> Result<bool, Self::Error> {
        self.events.push("prephysics");
        Ok(false)
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("draw disabled")
    }
}
impl SkeletonIkDriveFramesServices for Cvars {
    fn enable_prephysics(&mut self) -> Result<bool, &'static str> {
        self.events.push("prephysics-enable");
        Ok(true)
    }
    fn maximum_leg_length(&mut self) -> Result<f32, &'static str> {
        Ok(f32::from_bits(0x3f4c_cccd))
    }
    fn maximum_arm_length(&mut self) -> Result<f32, &'static str> {
        Ok(1.0)
    }
}
impl SkeletonIkSuffixServices for Cvars {
    fn enable_contact_offset(&mut self) -> Result<bool, &'static str> {
        self.events.push("contact-enable");
        Ok(true)
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "same-owner original corpus stage and retained consumer checks"
)]
pub(super) fn check(
    world: &mut PhysicalAssemblyWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
    board: &SimulationSkaterAssemblyHandle,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    reckoning: &Reckoning<ReckoningVaultAttributes>,
    ground: &StaticGroundPool,
) {
    // Reuse the SAME world query owner constructed by the enclosing collision
    // corpus before skeleton trajectories. Its line capacity remains five.
    let mut processed = SkeletonProcessedContact::new();
    let normal = world.skeleton(skeleton).unwrap();
    let data = normal.current_animation().ik().data();
    assert!(Arc::ptr_eq(data.hierarchy(), &owner.inputs().hierarchy));
    let bone = normal.current_animation().ik().bone_ids()[0].unwrap();
    let packed = data.hierarchy().packed_bone_names[bone];
    let mut raw = [0_u8; 0x40];
    for (chunk, word) in raw[..20].chunks_exact_mut(4).zip(packed.words) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    raw[0x34] = 4;
    raw[0x35] = 3;
    let mut source_board = ActualBoard {
        board: world.board(board).unwrap(),
        backend: world.backend(),
        reads: vec![],
    };
    processed
        .try_sample_toolkit_prefix(&mut source_board)
        .unwrap();
    let mut toolkit_motion = ToolkitMotion::default();
    let bodies_before_toolkit = world.backend().rigid_body_snapshot();
    processed
        .try_continue_toolkit_directions(&mut toolkit_motion, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(
        toolkit_motion.reads,
        ["flags", "prior zSpeed", "prior zSpeed"]
    );
    assert_eq!(world.backend().rigid_body_snapshot(), bodies_before_toolkit);
    assert_eq!(source_board.reads, ["mass", "Part6", "Part6"]);
    let toolkit = processed.toolkit_state();
    assert!(toolkit.directions_complete());
    let deck = processed.deck_transform().unwrap().columns();
    assert_eq!(toolkit.body_x(), Some(deck[0]));
    assert_eq!(toolkit.body_y(), Some(deck[1]));
    assert_eq!(toolkit.body_z(), Some(deck[2]));
    assert_eq!(
        toolkit.abs_body_speed().unwrap().to_bits(),
        3.0_f32.to_bits()
    );
    let direction = toolkit.body_z_direction().unwrap();
    assert_eq!(direction.xyz[1].to_bits(), 0);
    assert_eq!(toolkit.z_axis_velocity_dir(), Some(deck[2]));
    assert_eq!(
        toolkit.z_axis_velocity().unwrap().xyz.map(f32::to_bits),
        deck[2].xyz.map(|value| (value * 3.0).to_bits())
    );
    assert!(toolkit.inverse_effective().is_some());
    // Native Update1 captures the physical toolkit prefix before ProcessData
    // dispatches these animation records into the same processed input.
    assert!(processed.apply_raw_attribute(data, &raw));
    let ready = processed.ready(0).unwrap();
    assert_eq!(ready.contact_bone_id(), Some(bone));
    assert_eq!(source_board.reads, ["mass", "Part6", "Part6"]);
    assert!(processed.physical_mass().unwrap() > 0.0);
    assert_eq!(
        processed.deck_transform(),
        Some(ready.effective_transform())
    );
    // Actual retained board normal; ground is explicitly supplied from the
    // original-data Reckoning fixture, not claimed to be a PhysOut copy.
    world
        .try_with_board_force_runtime(board, |runtime| {
            assert_eq!(
                runtime.jump_normal().xyz.map(f32::to_bits),
                [0, 0x3f80_0000, 0]
            );
            processed
                .try_continue_toolkit_jump_normal(
                    &mut ToolkitGround(reckoning.ground_normal()),
                    runtime,
                    &mut ToolkitJumpDefaults,
                    &mut HostReckoningNumeric,
                )
                .unwrap();
            assert!(processed.toolkit_state().jump_tail_complete());
            assert_eq!(
                processed.toolkit_state().processed_jump_normal(),
                Some(runtime.jump_normal())
            );
        })
        .unwrap();
    assert_eq!(world.backend().rigid_body_snapshot(), bodies_before_toolkit);
    let normal = world.skeleton(skeleton).unwrap();
    let raw_before = normal
        .animation_targets()
        .animation_rows()
        .map(|row| row.unwrap().map(f32::to_bits));
    let poses_before = normal
        .current_animation()
        .record()
        .poses()
        .map(|row| row.map(f32::to_bits));
    let mapped_before = normal.animation_targets().completed_volumes().unwrap();
    let handles: Vec<_> = normal
        .drive_owner()
        .unwrap()
        .prepared()
        .eligible_bone_drives()
        .map(|drive| {
            (
                drive.part,
                drive.kind,
                Arc::clone(drive.frames),
                Arc::clone(drive.dynamics),
            )
        })
        .collect();
    let before = world.backend().rigid_body_snapshot();
    let mut cvars = Cvars::default();
    let input = SkeletonGeneralInput {
        time_delta: 0.125,
        gravity: 73.25,
        current_physics_state: 1,
        flags_8c8: 0,
        flags_8d0: 0,
    };
    let mut output = SkeletonGroundedPassedOutput {
        orientation: SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM),
        flags_8c4: 0,
    };
    let result = world.try_update_skeleton_grounded_complete(
        skeleton,
        board,
        ground,
        0.0,
        &input,
        &mut output,
        &ready,
        reckoning,
        &mut HostReckoningNumeric,
        &mut HostReckoningSinCosNumeric,
        &mut HostSkateboardSpawnMathNumeric,
        &mut cvars,
    );
    if let Err(error) = &result {
        let runtime = world
            .skeleton(skeleton)
            .unwrap()
            .current_animation()
            .ik_runtime()
            .unwrap();
        eprintln!(
            "ordinary IK refusal: {error:?}; events={:?}; contact_states={:?}; offsets={:?}; modes={:?}; animation_to_world={:?}; world_to_animation={:?}; part_world={:?}; mapped_before={:?}",
            cvars.events,
            runtime.state().contact_states(),
            runtime.state().contact_offsets(),
            runtime.state().modes(),
            runtime.data_in().animation_to_world,
            runtime.data_in().world_to_animation,
            runtime.state().part_to_world(),
            mapped_before
        );
    }
    let progress = result.expect("ordinary source-default IK must complete without bypass");
    assert_eq!(progress.next, SkeletonGeneralContinuation::Complete);
    assert!(!progress.reset_performed);
    let normal = world.skeleton(skeleton).unwrap();
    let line = world
        .last_world_line_input_for_test()
        .expect("actual Contact line query reached World");
    let mut reference_hits = 0;
    let mut extended_hits = Vec::new();
    let vertical_start = [
        line.start.xyz[0],
        line.start.xyz[1] + 100.0,
        line.start.xyz[2],
    ];
    let vertical_end = [
        line.start.xyz[0],
        line.start.xyz[1] - 100.0,
        line.start.xyz[2],
    ];
    for (index, published) in ground.assembly_ground_meshes().enumerate() {
        let mesh = published.mesh;
        let hits = mesh
            .clustered_mesh
            .thin_segment_intersections(&mesh.object_bytes, line.start.xyz, line.end.xyz)
            .unwrap();
        reference_hits += hits.len();
        for hit in mesh
            .clustered_mesh
            .thin_segment_intersections(&mesh.object_bytes, vertical_start, vertical_end)
            .unwrap()
        {
            extended_hits.push((index, hit.intersection.position));
        }
        eprintln!(
            "actual IK line terrain mesh {index} bounds={:?}, independent finite segment hits={}",
            published.kd_bounds,
            hits.len()
        );
    }
    eprintln!(
        "actual IK Contact line start={:?}, end={:?}, fatness={:?}; original geometry hits={reference_hits}; extended vertical probes={extended_hits:?}; effective={:?}; post-drive target={:?}",
        line.start,
        line.end,
        line.fatness,
        ready.effective_transform(),
        normal
            .current_animation()
            .ik_runtime()
            .unwrap()
            .state()
            .part_to_world()[0]
    );
    assert_eq!(
        reference_hits, 0,
        "this retained original-pose finite segment must independently miss the terrain; investigate any geometry disagreement"
    );
    assert_eq!(
        normal
            .animation_targets()
            .animation_rows()
            .map(|row| row.unwrap().map(f32::to_bits)),
        raw_before
    );
    assert_eq!(
        normal
            .current_animation()
            .record()
            .poses()
            .map(|row| row.map(f32::to_bits)),
        poses_before
    );
    assert_eq!(
        normal
            .current_animation()
            .ik_runtime()
            .unwrap()
            .state()
            .contact_states()[0],
        2,
        "actual World miss agrees with independent original finite-segment geometry"
    );
    assert!(cvars.events.contains(&"line"));
    assert!(cvars.events.contains(&"prephysics"));
    let ordered = [
        "initial",
        "contact-enable",
        "line",
        "prephysics-enable",
        "prephysics",
        "drives",
    ]
    .map(|event| {
        cvars
            .events
            .iter()
            .position(|value| *value == event)
            .unwrap()
    });
    assert!(
        ordered.windows(2).all(|pair| pair[0] < pair[1]),
        "source stage order: {:?}",
        cvars.events
    );
    assert_eq!(cvars.damping_reads, handles.len());
    assert_ne!(
        normal
            .animation_targets()
            .completed_volumes()
            .unwrap()
            .map(|row| row.map(f32::to_bits)),
        mapped_before.map(|row| row.map(f32::to_bits))
    );
    let payloads = world.backend().active_drive_payloads_for_test().unwrap();
    for (ordinal, (part, kind, frames, dynamics)) in handles.iter().enumerate() {
        let bone = normal.drive_owner().unwrap().prepared().bone(*part);
        assert!(Arc::ptr_eq(frames, bone.frames(*kind)));
        assert!(Arc::ptr_eq(dynamics, bone.dynamics(*kind)));
        assert_eq!(
            payloads[ordinal + 5],
            (*frames.read().unwrap(), *dynamics.read().unwrap())
        );
    }
    let t = f32::from_bits(0x3c88_8889);
    let displacement = ((-input.gravity * 0.5) * t) * t;
    let seed = 1.0 / t;
    let first = seed.mul_add((-seed).mul_add(t, 1.0), seed);
    let reciprocal = first.mul_add((-first).mul_add(t, 1.0), first);
    let mut expected = before;
    for part in 0..24 {
        let observation = normal
            .session()
            .try_part_contact_spy_observation(world.backend(), part)
            .unwrap();
        let index = usize::try_from(observation.identity().id().index()).unwrap();
        let body = &mut expected.slots[index];
        for (force, delta) in body
            .force
            .iter_mut()
            .zip([0.0, displacement, 0.0].map(|value| (reciprocal * value) * body.inverse_mass))
        {
            *force += delta;
        }
        body.cooldown = 0;
    }
    assert_eq!(world.backend().rigid_body_snapshot(), expected);
    let next = normal
        .general_update()
        .next_trajectory_transform()
        .known_lanes()
        .unwrap();
    assert_eq!(
        next.map(f32::to_bits),
        FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
    );
    println!(
        "original ordinary Skeleton IK Contact→DriveFrames→retained drives→24 forces complete; independently verified actual World finite-segment miss, original Vault attrs, controlled processed/cvar schedule"
    );
}
