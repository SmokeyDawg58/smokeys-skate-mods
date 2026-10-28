//! Original retail menu movies, rendered over the native game presentation.
use crate::{
    apt_scene,
    apt_vm::Value,
    frontend_runtime::Runtime,
    graphics_menu::Menu,
    scoring_hud::{ColorTransform, HudComposite, HudMaterial},
};
use bevy::{
    asset::RenderAssetUsages,
    camera::{RenderTarget, ScalingMode, visibility::RenderLayers},
    prelude::*,
    render::render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
    sprite_render::MeshMaterial2d,
    window::PrimaryWindow,
};
use std::{collections::BTreeMap, path::PathBuf};

struct Slot {
    entity: Entity,
    mesh: Handle<Mesh>,
    material: Handle<HudMaterial>,
}
#[derive(Resource)]
pub(crate) struct Frontend {
    root: PathBuf,
    screen: String,
    runtime: Runtime,
    dimmer: Runtime,
    dimmer_shapes: apt_scene::Shapes,
    shapes: apt_scene::Shapes,
    textures: BTreeMap<String, Handle<Image>>,
    slots: Vec<Slot>,
    target: Handle<Image>,
    composite: Handle<HudComposite>,
    node: Entity,
    rebind: bool,
    accumulator: f64,
    legacy: bool,
    failed: bool,
    warming_up: bool,
    warmup_seconds: f64,
}
#[derive(Component)]
struct FrontendCamera;
pub(crate) struct FrontendPlugin;
impl Plugin for FrontendPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostStartup,
            setup.after(crate::graphics_menu::PresentationSetup),
        )
        .add_systems(
            PreUpdate,
            input
                .before(crate::graphics_menu::MenuInput)
                .after(crate::customiser::navigation),
        )
        .add_systems(Update, (advance, render).chain());
    }
}
fn load(root: &std::path::Path, screen: &str) -> Result<(Runtime, apt_scene::Shapes), String> {
    let path = root
        .join("runtime")
        .join(format!("{}.json", screen.replace('/', "__")));
    let source: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?,
    )
    .map_err(|e| e.to_string())?;
    let mut runtime = Runtime::load(&source)?;
    if screen == "main/core_menu" {
        runtime.call("InitializeMenu", vec![])?;
        runtime.call("UpdateHelperText", vec![])?;
        let title = runtime.vm.get(runtime.bindings.movie.root, "mTitle");
        if let Value::Object(title) = title {
            if let (Value::Object(mode), Value::Object(column)) = (
                runtime.vm.get(title, "mMode"),
                runtime.vm.get(title, "mColumn"),
            ) {
                let mode_width = runtime.vm.get(mode, "_width").number();
                runtime
                    .vm
                    .set(column, "_x", Value::Number(mode_width + 8.))?;
            }
        }
        if let Value::Object(panel) = runtime
            .vm
            .get(runtime.bindings.movie.root, "mProgressionPanel")
        {
            runtime.vm.set(panel, "_visible", Value::Bool(false))?;
        }
    } else {
        let id = runtime.bindings.movie.root;
        runtime.bindings.movie.method(
            &mut runtime.vm,
            id,
            "gotoAndPlay",
            &[Value::Text("intro".into())],
        )?;
        runtime.drain()?;
        if screen == "bootflow/pressstart" {
            runtime.call("SetState", vec![Value::Number(1.)])?;
        }
        if screen == "options/options" {
            runtime.call("SetState", vec![Value::Number(1.)])?;
        }
        if screen == "popup/small_popup" {
            runtime.call("Refresh", vec![])?;
        }
    }
    let shapes = serde_json::from_value(source["shapes"].clone()).map_err(|e| e.to_string())?;
    Ok((runtime, shapes))
}
fn textures(fe: &mut Frontend, images: &mut Assets<Image>) -> Result<(), String> {
    let mut files = BTreeMap::new();
    for shape in fe.shapes.values().flatten() {
        files.insert(
            shape.texture.rgba.clone(),
            [shape.texture.width, shape.texture.height],
        );
    }
    for shape in fe.dimmer_shapes.values().flatten() {
        files.insert(
            shape.texture.rgba.clone(),
            [shape.texture.width, shape.texture.height],
        );
    }
    for font in fe.runtime.bindings.movie.text_assets.fonts.values() {
        files.insert(font.texture.clone(), font.size);
    }
    for texture in fe.runtime.bindings.buttons.values() {
        files.insert(texture.rgba.clone(), [texture.width, texture.height]);
    }
    for (file, [w, h]) in files {
        if fe.textures.contains_key(&file) {
            continue;
        }
        let bytes = std::fs::read(fe.root.join(&file)).map_err(|e| format!("UI {file}: {e}"))?;
        if bytes.len() != w as usize * h as usize * 4 {
            return Err(format!("Invalid UI texture: {file}"));
        }
        fe.textures.insert(
            file,
            images.add(Image::new(
                Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                bytes,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            )),
        );
    }
    Ok(())
}
fn setup(
    mut commands: Commands,
    config: Res<crate::config::Config>,
    mut menu: ResMut<Menu>,
    mut time: ResMut<Time<Virtual>>,
    cameras: Query<Entity, With<IsDefaultUiCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut composites: ResMut<Assets<HudComposite>>,
) {
    let root = std::env::var_os("SKATE_FRONTEND_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| config.asset_root.join("private/frontend"));
    if !root.join("runtime/bootflow__pressstart.json").is_file() {
        return;
    }
    let Ok(output) = cameras.single() else {
        return;
    };
    let screen =
        std::env::var("SKATE_FRONTEND_SCREEN").unwrap_or_else(|_| "bootflow/pressstart".into());
    let (runtime, shapes) = match load(&root, &screen) {
        Ok(v) => v,
        Err(e) => {
            error!("Retail front-end: {e}");
            return;
        }
    };
    let (dimmer, dimmer_shapes) = match load(&root, "main/dimmer") {
        Ok(v) => v,
        Err(e) => {
            error!("Retail dimmer: {e}");
            return;
        }
    };
    let target = images.add(Image::new_target_texture(
        window.physical_width().max(1),
        window.physical_height().max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    let composite = composites.add(HudComposite {
        image: target.clone(),
    });
    let node = commands
        .spawn((
            MaterialNode(composite.clone()),
            UiTargetCamera(output),
            GlobalZIndex(9),
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
        ))
        .id();
    let warming_up = screen != "bootflow/pressstart";
    let mut fe = Frontend {
        root,
        screen,
        runtime,
        dimmer,
        dimmer_shapes,
        shapes,
        textures: BTreeMap::new(),
        slots: Vec::new(),
        target: target.clone(),
        composite,
        node,
        rebind: false,
        accumulator: 0.,
        legacy: false,
        failed: false,
        warming_up,
        warmup_seconds: 0.,
    };
    if let Err(e) = textures(&mut fe, &mut images) {
        error!("Retail front-end: {e}");
        commands.entity(node).despawn();
        return;
    }
    commands.spawn((
        FrontendCamera,
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 1280.,
                min_height: 720.,
            },
            ..OrthographicProjection::default_2d()
        }),
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        RenderTarget::Image(target.into()),
        RenderLayers::layer(30),
        Msaa::Off,
    ));
    menu.retail = true;
    menu.open = !warming_up;
    if warming_up {
        time.unpause();
    } else {
        time.pause();
    }
    info!("Retail front-end loaded: {}", fe.screen);
    commands.insert_resource(fe);
}
impl Frontend {
    fn switch(&mut self, screen: &str, images: &mut Assets<Image>) -> Result<(), String> {
        let (runtime, shapes) = load(&self.root, screen)?;
        self.runtime = runtime;
        self.shapes = shapes;
        self.screen = screen.into();
        self.accumulator = 0.;
        textures(self, images)
    }
}
fn input(
    fe: Option<ResMut<Frontend>>,
    mut menu: ResMut<Menu>,
    mut time: ResMut<Time<Virtual>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut nav: ResMut<crate::customiser::Navigation>,
    mut images: ResMut<Assets<Image>>,
    mut config: ResMut<crate::config::Config>,
    mut physics: ResMut<crate::physics::GamePhysics>,
    real: Res<Time<Real>>,
    animation: Res<crate::animation::AnimationStatus>,
) {
    let Some(mut fe) = fe else {
        return;
    };
    if fe.failed {
        return;
    }
    if fe.warming_up {
        fe.warmup_seconds += real.delta_secs_f64();
        if fe.warmup_seconds >= 0.3 && animation.ready {
            if fe.screen == "bootflow/pressstart" {
                if let Err(e) = fe.switch("main/core_menu", &mut images) {
                    error!("Retail menu: {e}");
                    fe.failed = true;
                }
            }
            fe.warming_up = false;
            menu.open = true;
            time.pause();
        }
        return;
    }
    if std::env::var("SKATE_VERIFY_FRONTEND").as_deref() == Ok("1") {
        return;
    }
    if fe.legacy {
        if !menu.open {
            fe.legacy = false;
            menu.retail = true;
        }
        return;
    }
    let back = keys.just_pressed(KeyCode::Escape) || nav.pressed & 0x2000 != 0;
    let start = nav.pressed & 0x10 != 0;
    let accept = keys.just_pressed(KeyCode::Enter) || nav.pressed & 0x1000 != 0;
    let result = (|| -> Result<(), String> {
        if !menu.open {
            if back || start {
                fe.switch("main/core_menu", &mut images)?;
                menu.open = true;
                time.pause();
            }
            return Ok(());
        }
        if fe.screen == "bootflow/pressstart" {
            if accept || start {
                fe.warming_up = true;
                fe.warmup_seconds = 0.;
                menu.open = false;
                time.unpause();
            }
            return Ok(());
        }
        if fe.screen == "popup/small_popup" && accept {
            fe.switch("main/core_menu", &mut images)?;
            return Ok(());
        }
        if back || start {
            if fe.screen != "main/core_menu" {
                fe.switch("main/core_menu", &mut images)?;
            } else {
                menu.open = false;
                time.unpause();
            }
            return Ok(());
        }
        if keys.just_pressed(KeyCode::F10) {
            menu.retail = false;
            fe.legacy = true;
            menu.select_section(2);
            return Ok(());
        }
        let horizontal = if keys.just_pressed(KeyCode::ArrowLeft) || nav.pressed & 4 != 0 {
            -1
        } else if keys.just_pressed(KeyCode::ArrowRight) || nav.pressed & 8 != 0 {
            1
        } else {
            0
        };
        if fe.screen == "options/options" {
            let vertical = if keys.just_pressed(KeyCode::ArrowUp) || nav.pressed & 1 != 0 {
                -1
            } else if keys.just_pressed(KeyCode::ArrowDown) || nav.pressed & 2 != 0 {
                1
            } else {
                0
            };
            if vertical != 0 {
                fe.runtime.call(
                    if vertical < 0 {
                        "PrevValue"
                    } else {
                        "NextValue"
                    },
                    vec![],
                )?;
            }
            let selected = fe
                .runtime
                .vm
                .get(fe.runtime.controller, "mCurrentIndex")
                .number() as usize;
            if selected == 0 && (horizontal != 0 || accept) {
                let direction = if horizontal == 0 { 1 } else { horizontal };
                let modes = [
                    crate::difficulty::Difficulty::Easy,
                    crate::difficulty::Difficulty::Normal,
                    crate::difficulty::Difficulty::Hardcore,
                ];
                let index = modes
                    .iter()
                    .position(|v| *v == config.difficulty)
                    .unwrap_or(0) as i32;
                let value = modes[(index + direction).rem_euclid(3) as usize];
                value.save(&config.asset_root)?;
                physics.set_difficulty(value);
                config.difficulty = value;
                menu.set_retail_difficulty(value);
                fe.runtime.bindings.options[0].value = format!("#{}", value.label());
                fe.runtime.call("SetState", vec![Value::Number(50.)])?;
            } else if accept {
                menu.select_section(2);
                menu.retail = false;
                fe.legacy = true;
            }
            return Ok(());
        }
        if horizontal != 0 {
            fe.runtime.bindings.category =
                (fe.runtime.bindings.category as i32 + horizontal).rem_euclid(5) as usize;
            fe.runtime.bindings.selection = 0;
            fe.runtime.call("SetCategory", vec![])?;
            fe.runtime.call("UpdateHelperText", vec![])?;
        }
        if accept {
            if fe.runtime.bindings.category == 4 {
                fe.switch("options/options", &mut images)?;
                fe.runtime.bindings.options[0].value = format!("#{}", config.difficulty.label());
                fe.runtime.call("SetState", vec![Value::Number(50.)])?;
                return Ok(());
            }
            if fe.runtime.bindings.category == 3 {
                fe.switch("popup/small_popup", &mut images)?;
                return Ok(());
            }
            let section = [0, 3, 1, 4, 2][fe.runtime.bindings.category];
            menu.select_section(section);
            menu.retail = false;
            fe.legacy = true;
        }
        Ok(())
    })();
    if fe.legacy {
        keys.clear_just_pressed(KeyCode::Enter);
        keys.clear_just_pressed(KeyCode::Escape);
        nav.pressed = 0;
    }
    if let Err(e) = result {
        error!("Retail menu input: {e}");
        fe.failed = true;
        menu.retail = false;
        menu.open = true;
        time.pause();
    }
}
fn advance(
    fe: Option<ResMut<Frontend>>,
    mut menu: ResMut<Menu>,
    time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    let Some(mut fe) = fe else {
        return;
    };
    if fe.failed {
        if menu.retail {
            menu.retail = false;
            menu.open = true;
            virtual_time.pause();
        }
        return;
    }
    if (!menu.open && !fe.warming_up) || fe.legacy {
        return;
    }
    fe.accumulator = (fe.accumulator + time.delta_secs_f64()).min(0.1);
    while fe.accumulator >= 1. / 60. {
        fe.accumulator -= 1. / 60.;
        if let Err(e) = fe.runtime.update() {
            error!("Retail menu animation: {e}");
            fe.failed = true;
            break;
        }
        if let Err(e) = fe.dimmer.update() {
            error!("Retail menu dimmer: {e}");
            fe.failed = true;
            break;
        }
    }
}
fn render(
    mut commands: Commands,
    fe: Option<ResMut<Frontend>>,
    menu: Res<Menu>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut composites: ResMut<Assets<HudComposite>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<HudMaterial>>,
    mut cameras: Query<&mut Camera, With<FrontendCamera>>,
) {
    let Some(mut fe) = fe else {
        return;
    };
    let visible = (menu.open || fe.warming_up) && !fe.legacy && !fe.failed;
    for mut camera in &mut cameras {
        camera.clear_color = ClearColorConfig::Custom(if fe.screen == "bootflow/pressstart" {
            Color::BLACK
        } else {
            Color::NONE
        });
    }
    commands.entity(fe.node).insert(Node {
        display: if visible {
            Display::Flex
        } else {
            Display::None
        },
        position_type: PositionType::Absolute,
        width: percent(100),
        height: percent(100),
        ..default()
    });
    if fe.rebind {
        if let Some(m) = composites.get_mut(&fe.composite) {
            m.image = fe.target.clone();
        }
    }
    fe.rebind = crate::scoring_hud::resize_image(
        &mut images,
        &mut composites,
        &fe.target,
        &fe.composite,
        Extent3d {
            width: window.physical_width().max(1),
            height: window.physical_height().max(1),
            depth_or_array_layers: 1,
        },
    );
    let mut draws = Vec::new();
    if visible {
        let result = (|| -> Result<(), String> {
            if fe.screen != "bootflow/pressstart" {
                draws.extend(apt_scene::draw(
                    &fe.dimmer.bindings.movie,
                    &fe.dimmer.vm,
                    &fe.dimmer_shapes,
                )?);
            }
            draws.extend(apt_scene::draw(
                &fe.runtime.bindings.movie,
                &fe.runtime.vm,
                &fe.shapes,
            )?);
            Ok(())
        })();
        if let Err(e) = result {
            error!("Retail UI geometry: {e}");
            fe.failed = true;
            draws.clear();
        }
    }
    for (i, draw) in draws.iter().enumerate() {
        let Some(texture) = fe.textures.get(&draw.texture).cloned() else {
            continue;
        };
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            draw.vertices
                .iter()
                .map(|v| [v.position[0] - 640., 360. - v.position[1], 0.])
                .collect::<Vec<_>>(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_0,
            draw.vertices.iter().map(|v| v.uv).collect::<Vec<_>>(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![[0., 0., 1.]; draw.vertices.len()],
        );
        let material = HudMaterial {
            color: ColorTransform {
                multiply: draw.multiply.into(),
                add: draw.add.into(),
            },
            atlas: texture,
        };
        if i == fe.slots.len() {
            let mesh = meshes.add(mesh);
            let material = materials.add(material);
            let entity = commands
                .spawn((
                    Mesh2d(mesh.clone()),
                    MeshMaterial2d(material.clone()),
                    Transform::from_xyz(0., 0., i as f32 * 0.001),
                    RenderLayers::layer(30),
                ))
                .id();
            fe.slots.push(Slot {
                entity,
                mesh,
                material,
            });
        } else {
            let slot = &fe.slots[i];
            if let Some(m) = meshes.get_mut(&slot.mesh) {
                *m = mesh;
            }
            if let Some(m) = materials.get_mut(&slot.material) {
                *m = material;
            }
            commands.entity(slot.entity).insert(Visibility::Visible);
        }
    }
    for slot in &fe.slots[draws.len()..] {
        commands.entity(slot.entity).insert(Visibility::Hidden);
    }
}
