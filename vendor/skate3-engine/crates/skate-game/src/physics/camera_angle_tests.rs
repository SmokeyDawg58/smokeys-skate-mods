//! Camera Angle (retail Game Settings > Control Settings) against the real stock camera graph.
use super::super::{GamePhysics, PlayerControls, SkaterRuntime, frame};

/// Retail Camera Angle: graph type 0 routes the stock graph to `cameragraph_low.xml`, type 1 to
/// `cameragraph_high.xml`. The same standing skater gets `bl_chase` (low, close) vs
/// `bl_high_chase` (high, farther), and switching back and forth needs no reload.
#[test]
#[ignore = "requires private stock graphs and animation assets"]
fn camera_angle_switches_between_the_stock_low_and_high_graphs() {
    let root = std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT").unwrap());
    let assets = skate_data::GameAssets::load(&root).unwrap();
    let graphs = crate::graph_runtime::StockGraphs::load(&root, &assets).unwrap();
    let mut physics =
        GamePhysics::load_with_difficulty(&root, None, crate::difficulty::Difficulty::Normal)
            .unwrap();
    let mut skater = SkaterRuntime::load(&root, &graphs, &physics, "normal").unwrap();
    let mut camera = crate::camera::CameraRuntime::load(&root).unwrap();
    assert_eq!(
        camera.camera_type(),
        crate::camera::CameraAngle::High.graph_type()
    );
    let mut input = crate::input::ControllerInput::default();
    let mut controls = PlayerControls::load(&root).unwrap();
    let mut run = |camera: &mut crate::camera::CameraRuntime, ticks: usize| -> (String, f32) {
        let mut height = 0.0;
        for tick in 0..ticks {
            input.sample_raw_for_test(skate_core::input::xbox::XboxState {
                buttons: 0,
                triggers: [0; 2],
                left: [0; 2],
                right: [0; 2],
            });
            let mut actions = input.player_actions();
            controls.update(
                &mut actions,
                physics.settings.step.simulation.time_step,
                physics.settings.input_magnitude_threshold,
                skater.player_input.physical.scoring.capabilities_204,
            );
            controls.publish_gestures(
                physics.animation_profile.physics_mode,
                skater.player_input.physical.state.state_16,
            );
            frame::advance(
                &mut physics,
                &mut skater,
                &mut controls,
                &graphs,
                &mut actions,
                true,
                camera,
            )
            .unwrap();
            if tick + 30 >= ticks {
                height += camera.frame.unwrap().position[1] / 30.0;
            }
        }
        (camera.selected_shot().to_owned(), height)
    };
    let (high_shot, high) = run(&mut camera, 180);
    assert_eq!(high_shot, "bl_high_chase");
    camera.set_camera_type(crate::camera::CameraAngle::Low.graph_type());
    let (low_shot, low) = run(&mut camera, 180);
    assert_eq!(low_shot, "bl_chase");
    assert!(
        low + 0.3 < high,
        "the low camera must sit lower: low={low} high={high}"
    );
    // A mod tuning reloads the current shot tree without disturbing the graph.
    let tuning = skate_mods::presentation::CameraShotTuning {
        position_distance: Some(2.0),
        ..Default::default()
    };
    camera.set_shot_tunings(2, [("low_ollie", &tuning)].into_iter());
    let (still_low, _) = run(&mut camera, 10);
    assert_eq!(still_low, "bl_chase");
    camera.set_camera_type(crate::camera::CameraAngle::High.graph_type());
    // The high shot's elevation smoothing is slow: about 10 s to settle back from the low camera.
    let (back, back_height) = run(&mut camera, 600);
    assert_eq!(back, "bl_high_chase");
    assert!(
        (back_height - high).abs() < 0.2,
        "high camera returns: {back_height} vs {high}"
    );
    eprintln!("standing camera height: high={high} low={low} back={back_height}");
}
