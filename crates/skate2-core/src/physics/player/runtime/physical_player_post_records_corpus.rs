//! Actual paired-owner solve -> controlled current animation/grounded Reckoning
//! -> collision/volume states -> retained post-physics records. This is component
//! scheduling, not the missing full native `GeneralUpdate` state schedule.
//! Error/COM expectations use independently written source-form arithmetic;
//! hooks use an f64 geometric oracle, not the production displacement helper.
//! Geometry tolerances cover f32 reduction/estimates and source trig polynomials;
//! frozen unit graphs establish ordering, ties and exact retained failure stores.

use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner, roots_corpus::RootCorpus};
use crate::{
    animation::AnimationMatrix,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    skeleton_collision_data::SkeletonCollisionData,
    skeleton_physics_history::lane_knowledge::RecordVectorField,
    skeleton_post_physics_records::SkeletonPostPhysicsRecordServices,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{convert::Infallible, sync::Arc};

const HOOK_PARTS: [usize; 8] = [23, 17, 18, 16, 21, 22, 20, 1];

pub(super) struct PostRecordsCorpus {
    roots: RootCorpus,
    previous_com: [f32; 4],
    updates: usize,
}
impl PostRecordsCorpus {
    pub(super) fn reckoning(&self) -> super::roots_corpus::RootReckoningRead<'_> {
        self.roots.reckoning()
    }
    pub(super) fn reckoning_phase(
        &self,
    ) -> &crate::physical_player_reckoning_owner::PhysicalPlayerReckoningPhase {
        self.roots.phase()
    }
    pub(super) fn new(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Self {
        Self {
            roots: RootCorpus::new(owner),
            previous_com: [0.0; 4],
            updates: 0,
        }
    }

    pub(super) fn prepare(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        pose: &[AnimationMatrix],
    ) {
        // Invoked after the actual solve. The existing driver deliberately uses
        // controlled grounded Reckoning and current original-pose adjustments.
        self.roots.check(world, skeleton, board, pose);
        assert!(
            world
                .skeleton(skeleton)
                .unwrap()
                .current_animation()
                .root_input(0.0)
                .is_some()
        );
    }

    pub(super) fn services(&self) -> PostRecordServices<'_> {
        PostRecordServices {
            roots: &self.roots,
            calls: Vec::new(),
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "independent connected errors, COM, maximum and geometric hook assertions"
    )]
    pub(super) fn check(
        &mut self,
        world: &PhysicalAssemblyWorld,
        skeleton: &SimulationSkaterAssemblyHandle,
        flags_8c8: u32,
        dt: f32,
    ) {
        let owner = world.skeleton(skeleton).unwrap();
        let actual = owner.post_physics_records();
        assert!(
            !actual.proxy_enabled(),
            "actual ctor/Init false, no invented proxy mode"
        );
        let physics = owner.physics_history().record();
        let volumes = owner.animation_targets().completed_volumes().unwrap();
        let columns = owner.root_transforms().animation_to_world().columns();
        let proxy_target = owner
            .state()
            .proxy_volume_target
            .expect("actual Set established proxy target");
        let proxy = physics.vector_typed(RecordVectorField::ProxyVolumePosition);
        let mut expected_errors = [SkeletonRootVector {
            xyz: [0.0; 3],
            w: Some(0.0),
        }; 24];
        expected_errors[0] = SkeletonRootVector {
            xyz: std::array::from_fn(|lane| proxy.xyz[lane] - proxy_target.xyz[lane]),
            w: proxy.w.zip(proxy_target.w).map(|(p, target)| p - target),
        };
        for part in 1..24 {
            let position = &volumes[part][12..16];
            let physical = physics.transform_typed(part).unwrap().columns()[3];
            for (lane, physical) in physical.xyz.iter().enumerate() {
                let x = position[0].mul_add(columns[0].xyz[lane], columns[3].xyz[lane]);
                let y = position[1].mul_add(columns[1].xyz[lane], x);
                let predicted = position[2].mul_add(columns[2].xyz[lane], y);
                expected_errors[part].xyz[lane] = physical - predicted;
            }
            expected_errors[part].w = match columns.map(|column| column.w) {
                [Some(x), Some(y), Some(z), Some(p)] => {
                    let first = position[0].mul_add(x, p);
                    let second = position[1].mul_add(y, first);
                    physical.w.map(|w| w - position[2].mul_add(z, second))
                }
                _ => None,
            };
        }
        for (part, expected) in expected_errors.iter().enumerate() {
            assert_eq!(
                actual.errors()[part].xyz.map(f32::to_bits),
                expected.xyz.map(f32::to_bits),
                "Part{part} physical minus adjusted world error"
            );
            assert_eq!(
                actual.errors()[part].w.map(f32::to_bits),
                expected.w.map(f32::to_bits),
                "Part{part} source-known W mask and value"
            );
        }
        let current = physics.centre_of_mass();
        assert_eq!(
            physics.vector_typed(RecordVectorField::CentreOfMass).w,
            None
        );
        assert_eq!(actual.centre_of_mass_typed().w, None);
        assert_eq!(actual.centre_of_mass_velocity_typed().w, None);
        let seed = 1.0 / dt;
        let residual0 = (-seed).mul_add(dt, 1.0);
        let refined0 = seed.mul_add(residual0, seed);
        let residual1 = (-refined0).mul_add(dt, 1.0);
        let reciprocal = refined0.mul_add(residual1, refined0);
        let velocity: [f32; 4] =
            std::array::from_fn(|lane| (current[lane] - self.previous_com[lane]) * reciprocal);
        assert_eq!(
            actual.centre_of_mass().map(f32::to_bits),
            current.map(f32::to_bits)
        );
        assert_eq!(
            actual.centre_of_mass_velocity().map(f32::to_bits),
            velocity.map(f32::to_bits)
        );
        let previous = if flags_8c8 & 0x1000 == 0 {
            self.previous_com
        } else {
            current
        };
        assert_eq!(
            actual.previous_centre_of_mass().map(f32::to_bits),
            previous.map(f32::to_bits)
        );
        assert!(
            velocity[..3].iter().any(|lane| *lane != 0.0),
            "actual solved COM changes on retained frames"
        );

        let maximum = HOOK_PARTS
            .into_iter()
            .map(|part| length(expected_errors[part].xyz.map(f64::from)))
            .fold(0.0_f64, f64::max);
        close(
            f64::from(actual.max_displacement().unwrap()),
            maximum,
            0.00001,
            "maximum displacement",
        );
        let ground = self.roots.ground_normal();
        let candidates = HOOK_PARTS.map(|part| {
            geometric_displacement(expected_errors[part], ground, &owner.state().collision)
        });
        let maximum_adjusted = candidates
            .iter()
            .map(|candidate| length(candidate.0))
            .fold(0.0_f64, f64::max);
        let hook = actual.hook_to_hips();
        // Geometrically indistinguishable near-ties may choose different rows
        // after native f32 rounding. Exact Part23 tie/order is unit-tested.
        assert!(
            candidates.iter().any(|(xyz, w)| {
                let near_maximum =
                    (length(*xyz) - maximum_adjusted).abs() <= 0.002 * (1.0 + maximum_adjusted);
                near_maximum
                    && xyz
                        .iter()
                        .zip(hook.xyz)
                        .all(|(expected, actual)| within(f64::from(actual), *expected, 0.002))
                    && match (hook.w, w) {
                        (Some(actual), Some(expected)) => {
                            within(f64::from(actual), *expected, 0.002)
                        }
                        (None, None) => true,
                        _ => false,
                    }
            }),
            "retained hook {hook:?} disagrees with f64 geometric candidates {candidates:?}"
        );
        self.previous_com = current;
        self.updates += 1;
        assert!(
            self.updates <= 2,
            "bounded two-frame collision/records fixture"
        );
    }
}

pub(super) struct PostRecordServices<'a> {
    roots: &'a RootCorpus,
    calls: Vec<&'static str>,
}
impl PostRecordServices<'_> {
    pub(super) fn assert_normal_calls(&self) {
        assert_eq!(self.calls, ["draw", "ground", "draw"]);
    }
}
impl SkeletonPostPhysicsRecordServices for PostRecordServices<'_> {
    type Error = Infallible;
    fn ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("ground");
        Ok(self.roots.ground_normal())
    }
    fn use_normals_for_proxy_offset(&mut self) -> Result<u8, Self::Error> {
        panic!("actual constructor/Init proxy mode is false")
    }
    fn scale_hook_to_hips_y(&mut self) -> Result<f32, Self::Error> {
        panic!("normal hook does not fetch proxy Y scale")
    }
    fn draw_hook_to_hips(&mut self) -> Result<u8, Self::Error> {
        self.calls.push("draw");
        Ok(0)
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
fn length(value: [f64; 3]) -> f64 {
    dot(value, value).sqrt()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn project(value: [f64; 3], normal: [f64; 3]) -> [f64; 3] {
    let distance = dot(value, normal);
    std::array::from_fn(|lane| value[lane] - normal[lane] * distance)
}

/// Independent geometry: ground-plane projection, signed angular envelope of
/// eligible normals, then Rodrigues rotation around the actual ground axis.
fn geometric_displacement(
    displacement: SkeletonRootVector,
    ground: SkeletonRootVector,
    collision: &SkeletonCollisionData,
) -> ([f64; 3], Option<f64>) {
    let normal = [
        f64::from(ground.xyz[0]),
        f64::from(ground.xyz[1]),
        f64::from(ground.xyz[2]),
    ];
    let original = displacement.xyz.map(f64::from);
    let projected = project(original, normal);
    if length(projected) <= f64::from(f32::from_bits(0x3586_37bd)) {
        return (
            projected,
            displacement
                .w
                .zip(ground.w)
                .map(|(w, g)| f64::from(w) - f64::from(g) * dot(original, normal)),
        );
    }
    let mut minimum = std::f64::consts::FRAC_PI_2;
    let mut maximum = -std::f64::consts::FRAC_PI_2;
    let mut found = false;
    for row in collision
        .normal_info
        .iter()
        .take(usize::try_from(collision.num_normals).unwrap())
    {
        let contact = project(row.normal.xyz().map(f64::from), normal);
        if length(contact) <= f64::from(f32::from_bits(0x3586_37bd)) {
            continue;
        }
        let angle = dot(cross(projected, contact), normal).atan2(dot(projected, contact));
        if angle > -std::f64::consts::FRAC_PI_2 && angle < std::f64::consts::FRAC_PI_2 {
            minimum = minimum.min(angle);
            maximum = maximum.max(angle);
            found = true;
        }
    }
    let middle = minimum.midpoint(maximum);
    let factor = if found { middle.cos() } else { 0.0 };
    let perpendicular = cross(normal, projected);
    let parallel = dot(normal, projected);
    let rotated: [f64; 3] = std::array::from_fn(|lane| {
        (projected[lane] * middle.cos()
            + perpendicular[lane] * middle.sin()
            + normal[lane] * parallel * (1.0 - middle.cos()))
            * factor
    });
    // Raw rotation explicitly constructs basis W from basis X; full result W
    // therefore follows rotated X. This is not an assumed homogeneous zero.
    (rotated, Some(rotated[0]))
}
fn within(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance * (1.0 + expected.abs())
}
fn close(actual: f64, expected: f64, tolerance: f64, label: &str) {
    assert!(
        within(actual, expected, tolerance),
        "{label}: {actual} vs {expected}"
    );
}
