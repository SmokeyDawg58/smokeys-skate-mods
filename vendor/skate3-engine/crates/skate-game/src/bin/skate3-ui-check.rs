//! Headless execution check against locally extracted retail UI assets.
#[path = "../apt_display.rs"]
mod apt_display;
#[path = "../apt_movie.rs"]
mod apt_movie;
#[path = "../apt_scene.rs"]
mod apt_scene;
#[path = "../apt_text.rs"]
mod apt_text;
#[path = "../apt_vm.rs"]
mod apt_vm;
#[path = "../frontend_runtime.rs"]
mod frontend_runtime;
#[cfg(test)]
#[path = "../frontend_tests.rs"]
mod frontend_tests;
#[cfg(test)]
#[path = "../hud_runtime.rs"]
mod hud_runtime;

fn main() {
    if let Err(error) = check() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn check() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Usage: skate3-ui-check SCREEN.json [STATE]")?;
    let source = serde_json::from_slice(&std::fs::read(&path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut runtime = frontend_runtime::Runtime::load(&source)?;
    println!("Loaded {path}; controller {}", runtime.controller);
    if let Some(state) = std::env::args().nth(2) {
        if let Ok(state) = state.parse::<f64>() {
            runtime.call("SetState", vec![apt_vm::Value::Number(state)])?;
        } else {
            runtime.call(&state, Vec::new())?;
        }
    }
    let root = runtime.bindings.movie.root;
    if std::env::args().nth(2).as_deref() != Some("InitializeMenu") {
        runtime.bindings.movie.method(
            &mut runtime.vm,
            root,
            "gotoAndPlay",
            &[apt_vm::Value::Text("intro".into())],
        )?;
    }
    let shapes = serde_json::from_value(source["shapes"].clone()).map_err(|e| e.to_string())?;
    for frame in 0..180 {
        runtime
            .update()
            .map_err(|e| format!("Frame {frame}: {e}"))?;
        let draws = apt_scene::draw(&runtime.bindings.movie, &runtime.vm, &shapes)
            .map_err(|e| format!("Frame {frame}: {e}"))?;
        if frame % 60 == 0 {
            println!(
                "frame {frame}: {} draws, {} clips",
                draws.len(),
                runtime.bindings.movie.instances.len()
            );
        }
    }
    if std::env::var_os("SKATE_APT_TRACE").is_some() {
        for trace in &runtime.bindings.trace {
            println!("trace: {trace}");
        }
    }
    if std::env::var_os("SKATE_APT_DUMP").is_some() {
        fn tree(r: &frontend_runtime::Runtime, id: usize, depth: usize) {
            if depth > 3 {
                return;
            }
            let i = &r.bindings.movie.instances[&id];
            println!(
                "{}id={id} char={} name={} frame={} play={} x={} y={} sx={} sy={} alpha={} visible={} placement={:?}",
                " ".repeat(depth),
                i.character,
                r.vm.get(id, "_name").text(),
                i.frame,
                i.playing,
                r.vm.get(id, "_x").text(),
                r.vm.get(id, "_y").text(),
                r.vm.get(id, "_xscale").text(),
                r.vm.get(id, "_yscale").text(),
                r.vm.get(id, "_alpha").text(),
                r.vm.get(id, "_visible").text(),
                i.placement.as_ref().map(|p| p.color)
            );
            for child in i.children.values() {
                tree(r, *child, depth + 1);
            }
        }
        tree(&runtime, root, 0);
    }
    if std::env::var_os("SKATE_APT_DUMP_TEXT").is_some() {
        for (&id, instance) in &runtime.bindings.movie.instances {
            let text = runtime.vm.get(id, "_displayText").text();
            if text.is_empty() || text == "undefined" {
                continue;
            }
            let parent = runtime.vm.get(id, "_parent").text();
            println!(
                "text id={id} parent={parent} name={} text={text:?} x={} y={} visible={} char={}",
                runtime.vm.get(id, "_name").text(),
                runtime.vm.get(id, "_x").text(),
                runtime.vm.get(id, "_y").text(),
                runtime.vm.get(id, "_visible").text(),
                instance.character
            );
            if text == "Main" || text == "Free Play" {
                let mut child = id;
                for _ in 0..7 {
                    let apt_vm::Value::Object(parent) = runtime.vm.get(child, "_parent") else {
                        break;
                    };
                    println!(
                        "  ancestor id={parent} name={} char={} x={} y={} width={} sx={} visible={}",
                        runtime.vm.get(parent, "_name").text(),
                        runtime.bindings.movie.instances[&parent].character,
                        runtime.vm.get(parent, "_x").text(),
                        runtime.vm.get(parent, "_y").text(),
                        runtime.vm.get(parent, "_width").text(),
                        runtime.vm.get(parent, "_xscale").text(),
                        runtime.vm.get(parent, "_visible").text()
                    );
                    child = parent;
                }
            }
        }
    }
    Ok(())
}
