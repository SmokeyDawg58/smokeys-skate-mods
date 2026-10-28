//! Original Vault + retained Reckoning -> SAME real paired Skeleton roots.
//! Invoked inside the existing actual 35-body/28-joint/49-drive solve corpus.
//! Current pose passes source mapping, IK targets, offboard activation and
//! retained offset decay before roots. Input remains controlled; other adjustments, IK solve,
//! complete `GroundSkateboard` and Actor scheduling remain unfinished.
//! The driver deliberately supplies grounded Reckoning even during the added
//! offboard animation case. It verifies component state flow through actual
//! physics frames, not the missing native offboard `GeneralUpdate` schedule.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    animation::AnimationMatrix,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_reckoning_owner::{
        PhysicalPlayerReckoningGroundSample, PhysicalPlayerReckoningOwner,
        PhysicalPlayerReckoningPhase,
    },
    reckoning::{Reckoning, ReckoningCvars, ReckoningDebug, ReckoningGroundInput},
    reckoning_attributes::{
        ReckoningAttributeRead, ReckoningAttributeScope, ReckoningVaultAttributes,
    },
    reckoning_body_spin::{BodySpinCvar, BodySpinCvars},
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, try_signed_angle},
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    skateboard_spawn_math::{
        HostSkateboardSpawnMathNumeric, skateboard_spawn_compose, skateboard_spawn_normalize,
        skateboard_spawn_rigid_inverse,
    },
    skeleton_root_transforms::{SkeletonRootInput, SkeletonRootMatrix, SkeletonRootVector},
};
use std::{
    convert::Infallible,
    ops::Deref,
    sync::{Arc, MutexGuard},
};

const J: [f32; 4] = [0.0, 1.0, 0.0, 0.0];
const K: [f32; 4] = [0.0, 0.0, 1.0, 0.0];
const IDENTITY: AnimationMatrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

struct Cvars;
impl BodySpinCvars for Cvars {
    type Pointer = BodySpinCvar;
    type Error = Infallible;
    fn resolve(&mut self, kind: BodySpinCvar) -> Result<Self::Pointer, Self::Error> {
        Ok(kind)
    }
    fn float(&mut self, kind: &Self::Pointer) -> Result<f32, Self::Error> {
        // Explicit current-cvar fixture, not frozen original Vault tuning.
        Ok(match kind {
            BodySpinCvar::DerivativeScalar => 1.0,
            BodySpinCvar::DerivativeThreshold => 0.0,
            BodySpinCvar::DerivativeBlend | BodySpinCvar::InputBlend => 0.5,
        })
    }
}
impl ReckoningCvars for Cvars {
    fn debug_enabled(&mut self, _: ReckoningDebug) -> Result<bool, Self::Error> {
        Ok(false)
    }
}

pub(crate) struct RootCorpus {
    owner: Arc<PhysicalPlayerAssemblyOwner>,
    phase: Option<PhysicalPlayerReckoningPhase>,
    previous_deck: Option<AnimationMatrix>,
    previous_up: Option<[u32; 3]>,
    updates: u8,
    current_animation: super::current_animation_corpus::CurrentAnimationCorpus,
}
/// Locks the actual retained owner for the entire validated read lifetime.
pub(crate) struct RootReckoningRead<'a> {
    owner: MutexGuard<'a, PhysicalPlayerReckoningOwner>,
    phase: PhysicalPlayerReckoningPhase,
}
impl<'a> RootReckoningRead<'a> {
    fn new(owner: &'a PhysicalPlayerAssemblyOwner, phase: &PhysicalPlayerReckoningPhase) -> Self {
        let owner = owner.reckoning().lock().unwrap();
        owner.try_completed(phase).unwrap();
        Self {
            owner,
            phase: phase.clone(),
        }
    }
}
impl Deref for RootReckoningRead<'_> {
    type Target = Reckoning<ReckoningVaultAttributes>;
    fn deref(&self) -> &Self::Target {
        self.owner.try_completed(&self.phase).unwrap()
    }
}
impl RootCorpus {
    pub(super) fn reckoning(&self) -> RootReckoningRead<'_> {
        RootReckoningRead::new(&self.owner, self.phase())
    }
    pub(super) fn phase(&self) -> &PhysicalPlayerReckoningPhase {
        self.phase
            .as_ref()
            .expect("controlled Reckoning phase completed")
    }
    pub(super) fn ground_normal(&self) -> SkeletonRootVector {
        self.reckoning().ground_normal()
    }
    pub(crate) fn new(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Self {
        let mut attributes = ReckoningVaultAttributes::bind(&owner.inputs().vault);
        let layout = attributes
            .capture_layout(ReckoningAttributeScope::Own)
            .unwrap();
        // Independent original BIN9e110+4c0 oracle already fingerprinted by the
        // Reckoning corpus. A missing selected class/default-zero cannot pass.
        let original_control: [u32; 4] = std::array::from_fn(|lane| {
            attributes
                .layout_float(&layout, 0x4c0 + lane * 4)
                .unwrap()
                .to_bits()
        });
        assert_eq!(
            original_control,
            [0x3e4c_cccd, 0x3e4c_cccd, 0x3d4c_cccd, 0x3dcc_cccd]
        );
        Self {
            current_animation: super::current_animation_corpus::CurrentAnimationCorpus::new(owner),
            owner: Arc::clone(owner),
            phase: None,
            previous_deck: None,
            previous_up: None,
            updates: 0,
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "connected live pair/source arithmetic and no-mutation proof"
    )]
    pub(crate) fn check(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        current_pose: &[AnimationMatrix],
    ) {
        let board_owner = world.physical_body_member(board).unwrap().owner();
        let skeleton_owner = world.physical_body_member(skeleton).unwrap().owner();
        assert!(Arc::ptr_eq(board_owner, &self.owner));
        assert!(Arc::ptr_eq(board_owner, skeleton_owner));
        assert_eq!(self.owner.manager(), skeleton_owner.manager());
        let deck = world
            .board(board)
            .unwrap()
            .try_live_part_transform_host(world.backend(), 6)
            .unwrap();
        // The host affine projection is only an XYZ geometry oracle. The
        // original attached Part also carries packed words in its W lanes.
        let typed_deck = world
            .board(board)
            .unwrap()
            .try_live_part_transform_typed(world.backend(), 6)
            .unwrap();
        assert!(typed_deck.columns().iter().all(|column| column.w.is_none()));
        assert_xyz(&typed_deck, deck);
        let occurrence = world
            .board(board)
            .unwrap()
            .try_part_contact_spy_observation(world.backend(), 6)
            .unwrap();
        let velocity = world
            .backend()
            .collision_body_pool()
            .try_contact_spy_linear_velocity(occurrence)
            .unwrap();
        assert!(
            velocity.iter().any(|lane| *lane != 0.0),
            "real solved deck must move, not a zero-velocity oracle"
        );
        if let Some(previous) = self.previous_deck {
            assert_ne!(
                deck.map(f32::to_bits),
                previous.map(f32::to_bits),
                "second real solve must change the sampled deck"
            );
        }
        let old_root_position = world
            .skeleton(skeleton)
            .unwrap()
            .root_transforms()
            .board()
            .columns()[3];
        if let Some(previous) = self.previous_deck {
            assert_eq!(
                old_root_position.xyz.map(f32::to_bits),
                [previous[12], previous[13], previous[14]].map(f32::to_bits)
            );
        } else {
            assert_eq!(old_root_position.xyz.map(f32::to_bits), [0; 3]);
        }
        let time_delta = f32::from_bits(0x3d08_8889);
        let input = self
            .current_animation
            .check(world, skeleton, board, current_pose, time_delta);
        let skate_animation = input.skate_animation;
        let ground_input = ReckoningGroundInput {
            com_to_deck: J,
            deck_y: J,
            deck_z: K,
            effective_z: K,
            body_spin: 0.5,
            deck_speed: 3.0,
            absolute_body_speed: 3.0,
            balance: 0.0,
            time_delta,
            wheels_in_contact: 4,
            flags_8c4: 0,
            flags_8c8: 0,
        };
        self.phase = Some({
            let mut owner = self.owner.reckoning().lock().unwrap();
            let processed = Arc::clone(owner.processed());
            owner
                .try_update_grounded(
                    &processed,
                    |_| {
                        Ok::<_, Infallible>(PhysicalPlayerReckoningGroundSample {
                            input: ground_input,
                            ground: [0.6, 0.8, 0.0, 0.0],
                            dynamic: J,
                        })
                    },
                    K,
                    &mut Cvars,
                    &mut HostReckoningNumeric,
                )
                .unwrap()
        });
        // Controlled sample above uses the actual processed occurrence; its
        // values remain diagnostics, not a claim of missing input producers.
        let retained_owner = Arc::clone(&self.owner);
        let reckoning = RootReckoningRead::new(&retained_owner, self.phase());
        assert_ne!(
            reckoning.transform().xyz_scratch().map(f32::to_bits),
            IDENTITY.map(f32::to_bits)
        );
        if let Some(previous) = self.previous_up {
            assert_ne!(
                reckoning.up().xyz.map(f32::to_bits),
                previous,
                "same original-tuned filters advance, not reconstructed each frame"
            );
        }
        let expected_prediction: [f32; 3] =
            std::array::from_fn(|axis| velocity[axis].mul_add(time_delta, deck[12 + axis]));
        assert_ne!(
            expected_prediction.map(f32::to_bits),
            [deck[12], deck[13], deck[14]].map(f32::to_bits)
        );
        // This oracle checks XYZ only; scratch W never enters a publication.
        let expected = expected_matrices(
            &input,
            &reckoning.transform().xyz_scratch(),
            expected_prediction,
        );

        // Both actual registrations exist, but a board in the skeleton slot
        // still cannot run source reads or change the real Skeleton root state.
        let before = *world
            .skeleton(skeleton)
            .unwrap()
            .root_transforms()
            .animation_to_world();
        let mut forbidden = FailNumeric { calls: 0 };
        assert!(
            world
                .try_update_skeleton_roots_from_controlled_ground_input(
                    board,
                    board,
                    &input,
                    &reckoning,
                    &mut forbidden,
                    &mut HostReckoningSinCosNumeric,
                    &mut HostSkateboardSpawnMathNumeric
                )
                .is_err()
        );
        assert_eq!(forbidden.calls, 0);
        assert_eq!(
            *world
                .skeleton(skeleton)
                .unwrap()
                .root_transforms()
                .animation_to_world(),
            before
        );

        world
            .try_update_skeleton_roots_from_current_animation(
                skeleton,
                board,
                time_delta,
                &reckoning,
                &mut HostReckoningNumeric,
                &mut HostReckoningSinCosNumeric,
                &mut HostSkateboardSpawnMathNumeric,
            )
            .unwrap();
        let roots = world.skeleton(skeleton).unwrap().root_transforms();
        assert_eq!(roots.board(), &typed_deck);
        assert_eq!(roots.last_board_position(), old_root_position);
        assert_eq!(
            roots.predicted_board_position().xyz.map(f32::to_bits),
            expected_prediction.map(f32::to_bits)
        );
        assert!(roots.predicted_board_position().w.is_none());
        assert_xyz(roots.animation_to_board(), expected[0]);
        assert_xyz(roots.animation_to_world(), expected[1]);
        assert_xyz(roots.world_to_animation(), expected[2]);
        assert!(roots.animation_to_world().columns()[3].w.is_none());
        assert!(
            roots
                .world_to_animation()
                .columns()
                .iter()
                .all(|column| column.w.is_some())
        );

        // A late numeric refusal occurs AFTER the actual deck/prediction
        // stores, and must not reset/roll back the same owner. It does not
        // consume a fabricated once-per-solve token: source scheduling is open.
        if self.updates == 1 {
            let before = *roots.animation_to_world();
            let failed_input = SkeletonRootInput {
                time_delta: time_delta * 2.0,
                skate_animation,
            };
            let mut numeric = FailNumeric { calls: 0 };
            assert!(
                world
                    .try_update_skeleton_roots_from_controlled_ground_input(
                        skeleton,
                        board,
                        &failed_input,
                        &reckoning,
                        &mut numeric,
                        &mut HostReckoningSinCosNumeric,
                        &mut HostSkateboardSpawnMathNumeric
                    )
                    .is_err()
            );
            assert_eq!(numeric.calls, 1);
            let roots = world.skeleton(skeleton).unwrap().root_transforms();
            let failed_prediction: [u32; 3] = std::array::from_fn(|axis| {
                velocity[axis]
                    .mul_add(failed_input.time_delta, deck[12 + axis])
                    .to_bits()
            });
            assert_eq!(
                roots.predicted_board_position().xyz.map(f32::to_bits),
                failed_prediction
            );
            assert_ne!(failed_prediction, expected_prediction.map(f32::to_bits));
            assert_eq!(*roots.animation_to_world(), before);
        }
        assert_eq!(
            world
                .board(board)
                .unwrap()
                .try_part_contact_spy_observation(world.backend(), 6)
                .unwrap(),
            occurrence
        );
        assert_eq!(
            world
                .board(board)
                .unwrap()
                .try_live_part_transform_host(world.backend(), 6)
                .unwrap()
                .map(f32::to_bits),
            deck.map(f32::to_bits)
        );
        assert_eq!(
            world
                .backend()
                .collision_body_pool()
                .try_contact_spy_linear_velocity(occurrence)
                .unwrap()
                .map(f32::to_bits),
            velocity.map(f32::to_bits)
        );
        self.previous_deck = Some(deck);
        self.previous_up = Some(reckoning.up().xyz.map(f32::to_bits));
        self.updates += 1;
        if self.updates == 4 {
            println!(
                "original Vault + current mapped pose -> IK targets/offboard activation/retained offset decay/animation COM -> SAME Skeleton roots + retained Reckoning and actual moving Part6 over four solved frames; prediction/transform XYZ, unknown packed W and failure prefixes verified; controlled input, not full GeneralUpdate or native tick parity"
            );
        }
    }
}

/// Independent source composition oracle, using already audited scalar/trig
/// primitives rather than the root-state operation or its retained outputs.
/// Ordinary host reductions/seeds; not a captured Xenon tick trace.
fn expected_matrices(
    input: &SkeletonRootInput,
    reckoning: &AnimationMatrix,
    prediction: [f32; 3],
) -> [AnimationMatrix; 3] {
    let current = input.skate_animation;
    let angle = try_signed_angle(
        [current[8], 0.0, current[10], current[11]],
        K,
        J,
        &mut HostReckoningNumeric,
    )
    .unwrap();
    let (sin, cos) = try_sin_cos(angle, &mut HostReckoningSinCosNumeric).unwrap();
    let yaw = [
        cos,
        0.0,
        f32::from_bits(sin.to_bits() ^ 0x8000_0000),
        cos,
        0.0,
        1.0,
        0.0,
        0.0,
        sin,
        0.0,
        cos,
        sin,
        0.0,
        0.0,
        0.0,
        0.0,
    ];
    let mut negative = IDENTITY;
    for lane in 0..4 {
        negative[12 + lane] = f32::from_bits(current[12 + lane].to_bits() ^ 0x8000_0000);
    }
    let animation_to_board =
        skateboard_spawn_compose(reckoning, &skateboard_spawn_compose(&yaw, &negative));
    let mut translation = IDENTITY;
    translation[12..15].copy_from_slice(&prediction);
    let animation_to_world = skateboard_spawn_normalize(
        &skateboard_spawn_compose(&translation, &animation_to_board),
        &mut HostSkateboardSpawnMathNumeric,
    );
    [
        animation_to_board,
        animation_to_world,
        skateboard_spawn_rigid_inverse(&animation_to_world),
    ]
}
fn assert_xyz(actual: &SkeletonRootMatrix, expected: AnimationMatrix) {
    for (column, actual) in actual.columns().iter().enumerate() {
        assert_eq!(
            actual.xyz.map(f32::to_bits),
            std::array::from_fn::<_, 3, _>(|lane| expected[column * 4 + lane].to_bits())
        );
    }
}

struct FailNumeric {
    calls: usize,
}
impl ReckoningNumeric for FailNumeric {
    type Error = &'static str;
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        self.calls += 1;
        Err("controlled late root numeric refusal")
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("after first dot refusal")
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("after first dot refusal")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("after first dot refusal")
    }
}
