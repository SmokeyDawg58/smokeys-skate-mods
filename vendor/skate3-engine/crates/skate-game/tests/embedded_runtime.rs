//! Headless integration tests. These call the library and never initialize App.
use skate_game::{EmbeddedRuntime, RawController, Scene, SceneTriangle};

#[cfg(feature = "embedded-assets")]
#[test]
fn exe_embedded_assets_run_native_physics_without_an_asset_directory() {
    static PACK: &[u8] = include_bytes!(env!("SKATE3_EMBEDDED_PACK"));
    let before = skate_data::asset_io::disk_read_count();
    let mut runtime = EmbeddedRuntime::load_bundled(PACK, &floor(), "normal").unwrap();
    for _ in 0..60 {
        runtime.advance(RawController::default()).unwrap();
    }
    let before_toggle = runtime.output().tick;
    runtime
        .advance(RawController {
            buttons: 1 << 15,
            ..Default::default()
        })
        .unwrap();
    let mut offboard = false;
    for _ in 0..90 {
        offboard |= runtime.advance(RawController::default()).unwrap().offboard;
    }
    assert!(offboard, "bundled runtime did not dismount");
    assert!(runtime.output().tick > before_toggle);
    runtime.set_easy_motorized().unwrap();
    runtime.advance(RawController::default()).unwrap();
    assert_eq!(
        skate_data::asset_io::disk_read_count(),
        before,
        "the embedded runtime attempted a filesystem asset read"
    );
    assert!(
        !std::path::Path::new(skate_data::asset_io::ROOT).exists(),
        "no extracted asset directory should exist"
    );
}

fn floor() -> Scene {
    Scene {
        triangles: vec![
            SceneTriangle {
                points: [[-100., 0., -100.], [-100., 0., 100.], [100., 0., -100.]],
                surface: 0,
                packed_surface: 0,
                native_edges: None,
                native_features: None,
            },
            SceneTriangle {
                points: [[100., 0., -100.], [-100., 0., 100.], [100., 0., 100.]],
                surface: 0,
                packed_surface: 0,
                native_edges: None,
                native_features: None,
            },
        ],
        ..Default::default()
    }
}

#[test]
fn missing_host_geometry_is_rejected_before_loading_assets() {
    assert!(
        EmbeddedRuntime::load(
            std::path::Path::new("missing-assets"),
            &Scene::default(),
            "normal"
        )
        .is_err()
    );
}

#[test]
#[ignore = "requires prepared local Skate 3 assets through SKATE3_ASSET_ROOT"]
fn native_ticks_profiles_and_host_reactions_work_without_an_application() {
    use skate_game::{SolidBody, SolidCollider, rapier3d::prelude::*};
    let root = std::env::var_os("SKATE3_ASSET_ROOT").expect("SKATE3_ASSET_ROOT");
    let mut runtime =
        EmbeddedRuntime::load(std::path::Path::new(&root), &floor(), "normal").unwrap();
    for _ in 0..30 {
        let tick = runtime.output().tick;
        let output = runtime.advance(RawController::default()).unwrap();
        assert_eq!(output.tick, tick + 1);
        assert!(
            output
                .board_transform
                .iter()
                .flatten()
                .all(|lane| lane.is_finite())
        );
        assert!(
            output
                .world_bones
                .iter()
                .flatten()
                .flatten()
                .all(|lane| lane.is_finite())
        );
        assert_eq!(output.world_bones.len(), output.bone_names.len());
    }
    for profile in ["easy", "normal", "hardcore", "motorized"] {
        let before = runtime.output();
        runtime.set_profile(profile).unwrap();
        assert_eq!(runtime.output().tick, before.tick);
        assert_eq!(runtime.output().board_transform, before.board_transform);
        assert_eq!(runtime.output().board_velocity, before.board_velocity);
        runtime.advance(RawController::default()).unwrap();
    }
    let before = runtime.output();
    runtime.set_easy_motorized().unwrap();
    assert_eq!(runtime.output().tick, before.tick);
    assert_eq!(runtime.output().board_transform, before.board_transform);

    let deck = runtime.output().board_transform[3];
    let solid = || {
        let position = Vector::new(deck[0], deck[1] - 0.02, deck[2]);
        let pose = Pose::from_parts(position, Rotation::IDENTITY);
        SolidBody {
            id: 7,
            pose,
            center_of_mass: position,
            inertia_rotation: Rotation::IDENTITY,
            inverse_mass: 0.1,
            inverse_inertia: Vector::ONE,
            linvel: Vector::ZERO,
            angvel: Vector::ZERO,
            contact_group: 8,
            colliders: vec![SolidCollider {
                shape: SharedShape::cuboid(1., 1., 1.),
                pose,
                friction: 0.5,
            }],
        }
    };
    runtime.set_moving_bodies(vec![solid()]).unwrap();
    assert!(runtime.set_moving_bodies(vec![solid(), solid()]).is_err());
    runtime.advance(RawController::default()).unwrap();
    let reactions = runtime.take_body_reactions();
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0].id, 7);
    assert!(
        reactions[0]
            .linear_velocity_delta
            .iter()
            .all(|v| v.is_finite())
    );
    assert!(
        reactions[0].linear_velocity_delta.iter().any(|v| *v != 0.),
        "the host must receive an actual collision reaction: {reactions:?}"
    );
    assert!(runtime.take_body_reactions().is_empty());
}

#[test]
#[ignore = "requires prepared local Skate 3 assets through SKATE3_ASSET_ROOT"]
fn native_offboard_transition_uses_real_controller_input() {
    let root = std::env::var_os("SKATE3_ASSET_ROOT").unwrap();
    let mut runtime =
        EmbeddedRuntime::load(std::path::Path::new(&root), &floor(), "normal").unwrap();
    for _ in 0..60 {
        runtime.advance(RawController::default()).unwrap();
    }
    let start = runtime.output();
    assert!(!start.wiping_out, "initial state {}", start.state);
    // Real Xbox Y goes through source polling conversion, action maps, derived
    // input, graph transitions, native physical state and animation outputs.
    runtime
        .advance(RawController {
            buttons: 1 << 15,
            ..Default::default()
        })
        .unwrap();
    let mut saw_offboard = false;
    for _ in 0..90 {
        let output = runtime.advance(RawController::default()).unwrap();
        saw_offboard |= output.offboard;
    }
    assert!(
        saw_offboard,
        "native Y transition never reached offboard; last state {}",
        runtime.output().state
    );
}
