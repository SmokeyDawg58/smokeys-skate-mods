use crate::{
    apt_movie::Movie,
    apt_vm::{Value, Vm},
};

fn movie() -> Movie {
    Movie::load(&serde_json::json!({
        "characters":[
            {"id":0,"type_name":"animation","frames":[
                {"controls":[{"type_name":"place_object2","depth":1,"flags":166,"character_id":1,"matrix":[1,0,0,1,10,20],"name":"authored","actions_offset":12,"clip_actions":[{"events":1,"key":0,"action":1},{"events":2,"key":0,"action":2}]}]},
                {"controls":[{"type_name":"place_object2","depth":1,"flags":5,"matrix":[1,0,0,1,40,20]}]}
            ]},
            {"id":1,"type_name":"shape","bounds":[0,0,10,20]}
        ],"actions":{},"fonts":{},"font_mappings":{},"language":{},"exports":{"0":{"Child":1}}
    })).unwrap()
}

#[test]
fn authored_timeline_moves_keep_dynamically_attached_controls() {
    let mut movie = movie();
    let mut vm = Vm::new();
    movie.initialize(&mut vm).unwrap();
    let root = movie.root;
    let child = movie.attach(&mut vm, root, "Child", "created", 0).unwrap();
    movie.seek(&mut vm, root, 1, 0).unwrap();
    assert_eq!(vm.get(root, "created"), Value::Object(child));
    assert_eq!(movie.instances[&root].children[&16384], child);
    let Value::Object(authored) = vm.get(root, "authored") else {
        panic!()
    };
    assert_eq!(vm.get(authored, "_x"), Value::Number(40.));
    movie.remove_clip(&mut vm, child);
    assert_eq!(vm.get(root, "created"), Value::Undefined);
    assert!(!movie.instances.contains_key(&child));
}

#[test]
fn load_events_fire_once_and_enter_frame_events_repeat() {
    let mut movie = movie();
    let mut vm = Vm::new();
    movie.initialize(&mut vm).unwrap();
    assert_eq!(
        movie
            .pending
            .iter()
            .filter(|(_, action)| *action == 1)
            .count(),
        1
    );
    movie.pending.clear();
    movie.advance(&mut vm).unwrap();
    assert_eq!(
        movie
            .pending
            .iter()
            .filter(|(_, action)| *action == 1)
            .count(),
        0
    );
    assert_eq!(
        movie
            .pending
            .iter()
            .filter(|(_, action)| *action == 2)
            .count(),
        1
    );
}

#[test]
fn child_scaling_updates_parent_layout_dimensions() {
    let mut movie = movie();
    let mut vm = Vm::new();
    movie.initialize(&mut vm).unwrap();
    let Value::Object(child) = vm.get(movie.root, "authored") else {
        panic!()
    };
    vm.set(child, "_xscale", Value::Number(250.)).unwrap();
    movie.refresh_dimensions(&mut vm, movie.root).unwrap();
    assert_eq!(vm.get(movie.root, "_width"), Value::Number(25.));
}

#[test]
#[ignore = "requires locally extracted retail screens; set SKATE_FRONTEND_ROOT"]
fn owned_screens_run_without_missing_bindings_or_nonfinite_geometry() {
    let root = std::path::PathBuf::from(std::env::var_os("SKATE_FRONTEND_ROOT").unwrap());
    for (screen, method, args) in [
        ("bootflow__pressstart", "SetState", vec![Value::Number(1.)]),
        ("main__core_menu", "InitializeMenu", vec![]),
        ("options__options", "SetState", vec![Value::Number(1.)]),
        ("popup__small_popup", "Refresh", vec![]),
        (
            "main__dimmer",
            "gotoAndPlay",
            vec![Value::Text("intro".into())],
        ),
    ] {
        let source: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("runtime").join(format!("{screen}.json"))).unwrap(),
        )
        .unwrap();
        let mut runtime = crate::frontend_runtime::Runtime::load(&source).unwrap();
        if screen == "options__options" || screen == "popup__small_popup" {
            let id = runtime.bindings.movie.root;
            runtime
                .bindings
                .movie
                .method(
                    &mut runtime.vm,
                    id,
                    "gotoAndPlay",
                    &[Value::Text("intro".into())],
                )
                .unwrap();
        }
        runtime.call(method, args).unwrap();
        if screen == "main__core_menu" {
            runtime.call("UpdateHelperText", vec![]).unwrap();
        }
        let shapes = serde_json::from_value(source["shapes"].clone()).unwrap();
        for _ in 0..180 {
            runtime.update().unwrap();
        }
        let draws = crate::apt_scene::draw(&runtime.bindings.movie, &runtime.vm, &shapes).unwrap();
        assert!(!draws.is_empty(), "{screen} rendered nothing");
        assert!(
            draws
                .iter()
                .flat_map(|d| d.vertices.iter())
                .all(|v| v.position.iter().all(|x| x.is_finite())),
            "{screen}"
        );
        if screen == "bootflow__pressstart" {
            let icon = &runtime.bindings.buttons["Start"].rgba;
            assert!(
                draws.iter().any(|draw| &draw.texture == icon),
                "Original Start icon was not rendered"
            );
        }
        if screen == "options__options" {
            let before = runtime.vm.get(runtime.controller, "mCurrentIndex");
            runtime.call("NextValue", vec![]).unwrap();
            assert_ne!(runtime.vm.get(runtime.controller, "mCurrentIndex"), before);
            assert!(
                !runtime.bindings.movie.external_images.is_empty(),
                "Retail controller buttons absent"
            );
            runtime.call("PrevValue", vec![]).unwrap();
            assert_eq!(runtime.vm.get(runtime.controller, "mCurrentIndex"), before);
        }
        if screen == "main__core_menu" {
            for category in 0..5 {
                runtime.bindings.category = category;
                runtime.call("SetCategory", vec![]).unwrap();
                runtime.call("UpdateHelperText", vec![]).unwrap();
                for _ in 0..20 {
                    runtime.update().unwrap();
                }
                crate::apt_scene::draw(&runtime.bindings.movie, &runtime.vm, &shapes).unwrap();
            }
        }
    }
}

#[test]
#[ignore = "requires installed original HUD; set SKATE3_ASSET_ROOT"]
fn original_scoring_hud_survives_shared_movie_runtime_changes() {
    use crate::hud_runtime::{Input, Runtime};
    let root = std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT").unwrap());
    let source: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("private/hud/runtime/trickdisplay.json")).unwrap(),
    )
    .unwrap();
    let mut input = Input {
        sequence_score: 0,
        line_score: 0,
        sequence_timer: 0,
        line_time: 0.,
        line_capacity: 1000.,
        multiplier: 1.,
        clean: false,
        sketchy: false,
        stance: [false; 4],
        trick_name: "Ollie".into(),
        trick_metrics: std::array::from_fn(|_| Value::Number(0.)),
        context_tricks: vec![],
    };
    let mut runtime = Runtime::load(&source, input.clone()).unwrap();
    let shapes = serde_json::from_value(source["shapes"].clone()).unwrap();
    for frame in 0..720 {
        let landed = frame % 60 == 0;
        if landed {
            input.sequence_score += 100;
            input.line_score += 100;
            input.sequence_timer += 1;
            input.multiplier = (1 + (frame / 180).min(2)) as f32;
            input.line_time = 8.;
        } else {
            input.line_time = (input.line_time - 1. / 60.).max(0.);
        }
        runtime.update(input.clone(), landed, false, false).unwrap();
        let draws = crate::apt_scene::draw(&runtime.bindings.movie, &runtime.vm, &shapes).unwrap();
        assert!(!draws.is_empty());
        assert!(
            draws
                .iter()
                .flat_map(|d| d.vertices.iter())
                .all(|v| v.position.iter().all(|x| x.is_finite()))
        );
    }
    assert!(
        runtime
            .vm
            .objects
            .iter()
            .any(|o| o.fields.get("_displayText") == Some(&Value::Text("1,200".into()))),
        "HUD score did not update"
    );
}
