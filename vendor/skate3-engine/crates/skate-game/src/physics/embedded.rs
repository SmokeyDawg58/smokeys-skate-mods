//! Host boundary for the original production tick, with caller-owned scene data.
use super::{GamePhysics, PlayerControls, SkaterRuntime};
use crate::{camera::CameraRuntime, graph_runtime::StockGraphs, input::ControllerInput};
use skate_core::{
    input::xbox::XboxState,
    physics::{
        board::BodyId,
        skeleton_animation_record::{AnimationPartTransform, compose_affine},
    },
};
use skate_data::{
    GameAssets,
    skate_map::{Collision, Geometry, Material, Rail, SkateMap},
};
use std::path::Path;

#[derive(Clone, Copy, Debug, Default)]
pub struct RawController {
    pub buttons: u16,
    pub triggers: [u8; 2],
    pub left: [i16; 2],
    pub right: [i16; 2],
}

#[derive(Clone, Debug)]
pub struct SceneTriangle {
    pub points: [[f32; 3]; 3],
    pub surface: u32,
    /// RenderWare's packed audio (7), physics (5), and pattern (4) bits.
    pub packed_surface: u16,
    pub native_edges: Option<[u8; 3]>,
    pub native_features: Option<SceneTriangleFeatures>,
}

#[derive(Clone, Copy, Debug)]
pub struct SceneTriangleFeatures {
    pub flags: u32,
    pub edge_cosines: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct SceneRail {
    pub name: String,
    pub points: Vec<[f32; 3]>,
    pub closed: bool,
    pub native: Option<Vec<u8>>,
}

/// Native host spline data with occurrence-qualified header handles. The
/// source runtime computes its own contact chord and loose-octree traversal.
#[derive(Clone, Debug)]
pub struct HostGrindSegment {
    pub coefficients: [[f32; 4]; 4],
    pub minimum: [f32; 3],
    pub maximum: [f32; 3],
    pub owner: u64,
    pub spline_guids: [u64; 2],
    pub segment_index: u32,
}
#[derive(Clone, Debug)]
pub struct HostGrindAsset {
    pub segments: Vec<HostGrindSegment>,
}

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub spawn: [f32; 3],
    pub heading: f32,
    pub triangles: Vec<SceneTriangle>,
    pub rails: Vec<SceneRail>,
}

impl Scene {
    /// Already-decoded native RenderWare features are retained verbatim. An
    /// all-authored scene continues using the local engine's map compiler.
    fn world(
        &self,
        material: skate_core::physics::contact::RetailContactMaterial,
    ) -> Result<skate_core::physics::board_world::BoardWorld, String> {
        use skate_core::{
            math::Vector3,
            physics::{
                board_world::{
                    BoardWorld, WorldTriangle,
                    query_metadata::{Bounds, QueryMesh, QueryMetadata, QueryPool},
                },
                drive_frames::RetailAffineTransform,
            },
        };
        if self
            .triangles
            .iter()
            .all(|triangle| triangle.native_features.is_none())
        {
            return crate::skate_world::collision_world(&self.map()?, material);
        }
        let mut triangles = Vec::with_capacity(self.triangles.len());
        for source in &self.triangles {
            let feature = source
                .native_features
                .ok_or("Host collision must supply native features for every triangle")?;
            if feature.edge_cosines.iter().any(|v| !v.is_finite()) {
                return Err("Host collision contains a non-finite edge cosine".into());
            }
            let points = source.points.map(|[x, y, z]| Vector3::new(x, y, z));
            triangles.push(
                WorldTriangle::from_vertices(
                    points,
                    material,
                    source.surface,
                    feature.flags,
                    feature.edge_cosines,
                    0.,
                )
                .ok_or("Invalid host native triangle")?,
            );
        }
        let mut meshes = Vec::new();
        for start in (0..triangles.len()).step_by(64) {
            let end = (start + 64).min(triangles.len());
            let local_bounds = Bounds::from_points(
                triangles[start..end]
                    .iter()
                    .flat_map(|triangle| triangle.triangle.vertices),
            )
            .ok_or("Empty host collision bounds")?;
            meshes.push(QueryMesh {
                geometry: 0,
                rejection_flags: 0,
                triangle_range: start..end,
                local_to_world: RetailAffineTransform::IDENTITY,
                world_to_local: RetailAffineTransform::IDENTITY,
                local_bounds,
                matching_group: -1,
                pool: QueryPool::Ground,
            });
        }
        BoardWorld::with_query_metadata(
            triangles,
            QueryMetadata {
                packed_surfaces: self
                    .triangles
                    .iter()
                    .map(|triangle| triangle.packed_surface)
                    .collect(),
                meshes,
                static_edges: Vec::new(),
                island_flags: 0,
            },
        )
        .map_err(str::to_owned)
    }

    fn map(&self) -> Result<SkateMap, String> {
        if self.triangles.is_empty() {
            return Err("Embedded Skate 3 requires the host's real collision scene".into());
        }
        let mut surface_ids = std::collections::BTreeMap::new();
        let mut materials = Vec::new();
        let mut collision = Vec::with_capacity(self.triangles.len());
        for triangle in &self.triangles {
            let material = *surface_ids
                .entry(triangle.packed_surface)
                .or_insert_with(|| {
                    let packed = u32::from(triangle.packed_surface);
                    // This map is collision-only. The source collision adapter reads
                    // these three surface fields; contact coefficients still come
                    // from the original Skate 3 settings, never these visual fields.
                    materials.push(Material {
                        name: format!("host-surface-{packed:04x}"),
                        flags: 0,
                        friction: 0.,
                        restitution: 0.,
                        color: [0.; 3],
                        roughness: 0.,
                        emissive: 0.,
                        textures: [0; 5],
                        indirect_strength: 0.,
                        alpha_mode: 0,
                        alpha_cutoff: 0.,
                        audio: packed & 0x7f,
                        physics: (packed >> 7) & 0x1f,
                        pattern: packed >> 12,
                        depth_layer: None,
                        retail_definition: None,
                    });
                    materials.len() as u32
                });
            collision.push(Collision {
                points: triangle.points,
                surface: triangle.surface,
                material,
                native_edges: triangle.native_edges,
            });
        }
        Ok(SkateMap {
            version: 15,
            name: "Skate 2 resident world".into(),
            spawn: self.spawn,
            heading: self.heading,
            environment: Vec::new(),
            materials,
            textures: Vec::new(),
            geometry: Geometry {
                vertices: Vec::new(),
                indices: Vec::new(),
                collision,
            },
            rails: self
                .rails
                .iter()
                .map(|rail| Rail {
                    name: rail.name.clone(),
                    points: rail.points.clone(),
                    closed: rail.closed,
                    native: rail.native.clone(),
                })
                .collect(),
            doors: Vec::new(),
            lights: Vec::new(),
            routes: Vec::new(),
            extensions: Vec::new(),
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BodyOutput {
    pub transform: AnimationPartTransform,
    pub centre_of_mass: [f32; 3],
    pub velocity: [f32; 3],
    pub omega: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct SkeletonPartOutput {
    /// Original physics-to-animation name binding, not a host part index.
    pub bone_name: String,
    pub body: BodyOutput,
    pub bone_transform: AnimationPartTransform,
}

#[derive(Clone, Debug)]
pub struct FrameOutput {
    pub tick: u64,
    pub physical: skate_core::player::input_phase::PhysicalPlayerInput,
    /// Completed world-space publication, distinct from the animation record's
    /// local-space centre of mass.
    pub centre_of_mass: [f32; 4],
    pub damped_centre_of_mass: skate_core::physics::centre_of_mass_filter::CentreOfMassOutput,
    pub state: u32,
    pub category: u32,
    pub offboard: bool,
    pub wiping_out: bool,
    pub board_transform: AnimationPartTransform,
    pub board_velocity: [f32; 3],
    pub board_parts: [BodyOutput; 7],
    pub skeleton_parts: Vec<SkeletonPartOutput>,
    pub root_velocity: [f32; 3],
    pub root_omega: [f32; 3],
    pub camera: Option<skate_core::camera::CameraFrame>,
    pub root_transform: AnimationPartTransform,
    pub bone_names: Vec<String>,
    pub world_bones: Vec<AnimationPartTransform>,
}

/// Corrections produced by the source solver for a host-owned moving body.
#[derive(Clone, Copy, Debug)]
pub struct BodyReaction {
    pub id: u64,
    pub linear_velocity_delta: [f32; 3],
    pub angular_velocity_delta: [f32; 3],
    pub position_delta: [f32; 3],
    pub orientation_delta: [f32; 3],
}

/// No Bevy application, render schedule, OS controller polling or updater is run.
pub struct EmbeddedRuntime {
    root: std::path::PathBuf,
    graphs: StockGraphs,
    physics: GamePhysics,
    skater: SkaterRuntime,
    controls: PlayerControls,
    input: ControllerInput,
    camera: CameraRuntime,
    hips_body: usize,
}

impl EmbeddedRuntime {
    /// Native assets remain in the executable. No extraction, external asset
    /// folder, installation lookup or filesystem fallback is used.
    pub fn load_bundled(
        bytes: &'static [u8],
        scene: &Scene,
        profile: &str,
    ) -> Result<Self, String> {
        let root = skate_data::asset_io::mount(bytes).map_err(|e| e.to_string())?;
        Self::load(root, scene, profile)
    }
    pub fn load(root: &Path, scene: &Scene, profile: &str) -> Result<Self, String> {
        let difficulty = crate::difficulty::Difficulty::parse(profile)?;
        let map = scene.map()?;
        let assets = GameAssets::load_physics(root).map_err(|error| error.to_string())?;
        let graphs = StockGraphs::load(root, &assets)?;
        let mut physics = GamePhysics::load_with_difficulty(root, Some(&map), difficulty)?;
        if scene
            .triangles
            .iter()
            .any(|triangle| triangle.native_features.is_some())
        {
            physics.world = scene.world(physics.settings.floor_material)?;
        }
        let skater = SkaterRuntime::load(root, &graphs, &physics, difficulty.profile_key())?;
        let controls = PlayerControls::load(root)?;
        let camera = CameraRuntime::load(root)?;
        let hips_bone = skater
            .animation
            .evaluator
            .frames
            .bone_names
            .iter()
            .position(|name| name.eq_ignore_ascii_case("Hips"))
            .ok_or("Native rig lacks Hips")?;
        let hips_body = skater
            .animated_skeleton
            .bone_indices
            .iter()
            .position(|&bone| bone == hips_bone)
            .ok_or("Native physical rig lacks Hips body")?;

        Ok(Self {
            root: root.to_owned(),
            graphs,
            physics,
            skater,
            controls,
            input: ControllerInput::default(),
            camera,
            hips_body,
        })
    }

    pub fn period(&self) -> std::time::Duration {
        self.physics.period()
    }

    /// Replace the current tick's host-owned NPC/car/prop snapshots. A body has
    /// exactly one identity and one reaction recipient; remote-player prediction
    /// and multiplayer clock rules are not enabled by this host boundary.
    pub fn set_moving_bodies(
        &mut self,
        bodies: Vec<skate_dynamics::SolidBody>,
    ) -> Result<(), String> {
        let mut ids = std::collections::BTreeSet::new();
        for body in &bodies {
            if body.id == 0
                || !ids.insert(body.id)
                || !body.inverse_mass.is_finite()
                || body.inverse_mass < 0.
                || !body.center_of_mass.is_finite()
                || !body.linvel.is_finite()
                || !body.angvel.is_finite()
                || !body.inertia_rotation.is_finite()
                || !body.inverse_inertia.is_finite()
            {
                return Err(
                    "Moving bodies require unique nonzero IDs and finite physical state".into(),
                );
            }
        }
        let mut proxies = super::network::Proxies::default();
        // The same snapshots must answer the source engine's wheel, biped and
        // trajectory queries as well as its solid solver contacts.
        self.physics.set_external_queries(Some(std::sync::Arc::new(
            crate::modding::bridge::MovingQueries(bodies.clone()),
        )));
        for body in bodies {
            proxies.append_solid(body, &self.physics, &self.skater, true);
        }
        self.physics.network_proxies = proxies;
        Ok(())
    }

    pub fn set_external_queries(
        &mut self,
        queries: std::sync::Arc<dyn skate_core::physics::board_world::ExternalQueries>,
    ) {
        self.physics.set_external_queries(Some(queries));
    }

    /// Bind actual authored grab records to their actual collision meshes.
    /// Identities are supplied by the host's object registry, never inferred.
    pub fn set_grab_objects(
        &mut self,
        objects: Vec<skate_core::player::offboard::grab_scene::Object>,
        mesh_assemblies: Vec<(u32, u32)>,
    ) -> Result<(), String> {
        let bindings = mesh_assemblies
            .into_iter()
            .map(|(mesh, assembly)| super::offboard::grab_scene::MeshAssembly { mesh, assembly })
            .collect();
        let registry =
            super::offboard::grab_scene::Registry::new(&self.physics.world, objects, bindings)?;
        self.physics.offboard_grab_scene = registry;
        Ok(())
    }

    /// Drain once after the tick, then apply each correction to the matching
    /// Skate 2 world body before its next publication.
    pub fn take_body_reactions(&mut self) -> Vec<BodyReaction> {
        std::mem::take(&mut self.physics.network_proxies.dynamics_deltas)
            .into_iter()
            .map(
                |(
                    id,
                    linear_velocity_delta,
                    angular_velocity_delta,
                    position_delta,
                    orientation_delta,
                )| BodyReaction {
                    id,
                    linear_velocity_delta,
                    angular_velocity_delta,
                    position_delta,
                    orientation_delta,
                },
            )
            .collect()
    }

    pub fn set_profile(&mut self, profile: &str) -> Result<(), String> {
        let difficulty = crate::difficulty::Difficulty::parse(profile)?;
        self.physics.set_difficulty(difficulty);
        Ok(())
    }

    /// Prepare the complete difficulty table before replacing the active slot.
    pub fn set_easy_motorized(&mut self) -> Result<(), String> {
        self.set_motorized_profile("easy")
    }
    pub fn set_motorized_profile(&mut self, base: &str) -> Result<(), String> {
        let _ = crate::difficulty::Difficulty::parse(base)?;
        let mut data = skate_data::collections::Collections::load(&self.root)?;
        let target = data
            .field("physics_mode", "motorized", "MotorTopSpeed")?
            .clone();
        let enabled = skate_data::collections::Field {
            type_name: "EA::Reflection::Bool".into(),
            data: "01".into(),
        };
        data.override_profile(
            "physics_mode",
            "test",
            base,
            [
                ("MotorEnabled".into(), enabled),
                ("MotorTopSpeed".into(), target),
            ]
            .into_iter()
            .collect(),
        )?;
        self.skater.reload_difficulty(&data)?;
        self.physics
            .set_difficulty(crate::difficulty::Difficulty::Custom);
        Ok(())
    }

    pub fn replace_scene(&mut self, scene: &Scene) -> Result<(), String> {
        let map = scene.map()?;
        let world = scene.world(self.physics.settings.floor_material)?;
        let grind = crate::grind_world::StaticProvider::new(Some(&map))?;
        let registry = super::offboard::grab_scene::Registry::new(&world, Vec::new(), Vec::new())?;
        self.physics.world = world;
        self.physics.grind_world = std::sync::Arc::new(grind);
        self.skater
            .trajectory
            .bind_grind_world(std::sync::Arc::clone(&self.physics.grind_world));
        self.physics.offboard_grab_scene = registry;
        Ok(())
    }

    pub fn replace_collision_scene(&mut self, scene: &Scene) -> Result<(), String> {
        let world = scene.world(self.physics.settings.floor_material)?;
        let registry = super::offboard::grab_scene::Registry::new(&world, Vec::new(), Vec::new())?;
        self.physics.world = world;
        self.physics.offboard_grab_scene = registry;
        self.skater.offboard_grab.invalidate();
        Ok(())
    }
    pub fn set_host_grinds(&mut self, assets: &[HostGrindAsset]) -> Result<(), String> {
        let provider = std::sync::Arc::new(crate::grind_world::StaticProvider::from_host(assets)?);
        self.skater
            .trajectory
            .bind_grind_world(std::sync::Arc::clone(&provider));
        self.physics.grind_world = provider;
        Ok(())
    }
    pub fn grind_primitive_count(&self) -> usize {
        self.physics.grind_world.primitives().len()
    }

    pub fn travel(
        &mut self,
        transform: AnimationPartTransform,
        velocity: [f32; 3],
    ) -> Result<(), String> {
        self.skater.travel(transform, Some(velocity))
    }
    pub fn host_teleport(
        &mut self,
        transform: AnimationPartTransform,
        on_board: bool,
    ) -> Result<(), String> {
        self.skater
            .player_input
            .request_teleport_ex(transform, None)?;
        self.skater
            .teleport_state
            .request_manual(transform, on_board);
        Ok(())
    }
    pub fn teleport_pending(&self) -> bool {
        self.skater.player_input.pending_teleport().is_some()
    }

    /// Collect every host sample even when no native fixed tick is due yet.
    pub fn collect_controller(&mut self, raw: Option<RawController>) {
        crate::input::collect_embedded(
            &mut self.input,
            raw.map(|raw| XboxState {
                buttons: raw.buttons,
                triggers: raw.triggers,
                left: raw.left,
                right: raw.right,
            }),
        );
    }

    pub fn advance(&mut self, raw: RawController) -> Result<FrameOutput, String> {
        self.collect_controller(Some(raw));
        self.advance_collected()
    }

    pub fn advance_collected(&mut self) -> Result<FrameOutput, String> {
        let tick = crate::input::publish_embedded(&mut self.input);
        let mut actions = tick.actions();
        self.controls.update_for_physics(
            &mut actions,
            &self.physics,
            &self.skater,
            &self.camera,
        )?;
        self.controls.publish_gestures(
            self.physics.difficulty_index(),
            self.skater.player_input.physical.state.state_16,
        );
        super::frame::advance(
            &mut self.physics,
            &mut self.skater,
            &mut self.controls,
            &self.graphs,
            &mut actions,
            tick.controller_available(),
            &mut self.camera,
        )?;
        Ok(self.output())
    }

    pub fn output(&self) -> FrameOutput {
        let deck = self.physics.board.part_transforms()[BodyId::Deck.index()];
        let board_transform = [
            [
                deck.basis.columns[0][0],
                deck.basis.columns[0][1],
                deck.basis.columns[0][2],
                0.,
            ],
            [
                deck.basis.columns[1][0],
                deck.basis.columns[1][1],
                deck.basis.columns[1][2],
                0.,
            ],
            [
                deck.basis.columns[2][0],
                deck.basis.columns[2][1],
                deck.basis.columns[2][2],
                0.,
            ],
            [
                deck.translation.x,
                deck.translation.y,
                deck.translation.z,
                1.,
            ],
        ];
        let velocity = self.physics.board.bodies()[BodyId::Deck.index()]
            .rates
            .linear_velocity;
        let root = self.skater.animated_skeleton.roots.animation_to_world;
        let physical = &self.skater.player_input.physical;
        let skeleton_transforms = self.skater.skeleton.part_transforms();
        FrameOutput {
            tick: self.physics.ticks,
            physical: *physical,
            centre_of_mass: physical.reckoning.vector_64.map(f32::from_bits),
            damped_centre_of_mass: self.skater.centre_of_mass_output,
            state: physical.state.state_16,
            category: physical.state.category_12,
            offboard: physical.state.category_12 == 500,
            wiping_out: self.skater.skeleton_collision.is_ragdoll,
            board_transform,
            board_velocity: [velocity.x, velocity.y, velocity.z],
            board_parts: std::array::from_fn(|i| {
                let part = self.physics.board.part_transforms()[i];
                let rates = &self.physics.board.bodies()[i].rates;
                BodyOutput {
                    transform: [
                        [
                            part.basis.columns[0][0],
                            part.basis.columns[0][1],
                            part.basis.columns[0][2],
                            0.,
                        ],
                        [
                            part.basis.columns[1][0],
                            part.basis.columns[1][1],
                            part.basis.columns[1][2],
                            0.,
                        ],
                        [
                            part.basis.columns[2][0],
                            part.basis.columns[2][1],
                            part.basis.columns[2][2],
                            0.,
                        ],
                        [
                            part.translation.x,
                            part.translation.y,
                            part.translation.z,
                            1.,
                        ],
                    ],
                    centre_of_mass: [rates.position.x, rates.position.y, rates.position.z],
                    velocity: [
                        rates.linear_velocity.x,
                        rates.linear_velocity.y,
                        rates.linear_velocity.z,
                    ],
                    omega: [
                        rates.angular_velocity.x,
                        rates.angular_velocity.y,
                        rates.angular_velocity.z,
                    ],
                }
            }),
            skeleton_parts: self
                .skater
                .animated_skeleton
                .bone_indices
                .iter()
                .enumerate()
                .map(|(part, &bone)| {
                    let transform = skeleton_transforms[part];
                    let rates = &self.skater.skeleton.bodies()[part].rates;
                    SkeletonPartOutput {
                        bone_name: self.skater.animation.evaluator.frames.bone_names[bone].clone(),
                        body: BodyOutput {
                            transform,
                            centre_of_mass: [rates.position.x, rates.position.y, rates.position.z],
                            velocity: [
                                rates.linear_velocity.x,
                                rates.linear_velocity.y,
                                rates.linear_velocity.z,
                            ],
                            omega: [
                                rates.angular_velocity.x,
                                rates.angular_velocity.y,
                                rates.angular_velocity.z,
                            ],
                        },
                        bone_transform: compose_affine(
                            &transform,
                            &skate_core::physics::skeleton_root::inverse_rigid(
                                &self.skater.animated_skeleton.physics_frames[part],
                            ),
                        ),
                    }
                })
                .collect(),
            root_velocity: {
                let v = self.skater.skeleton.bodies()[self.hips_body]
                    .rates
                    .linear_velocity;
                [v.x, v.y, v.z]
            },
            root_omega: {
                let v = self.skater.skeleton.bodies()[self.hips_body]
                    .rates
                    .angular_velocity;
                [v.x, v.y, v.z]
            },
            camera: self.camera.frame,

            root_transform: root,
            bone_names: self.skater.animation.evaluator.frames.bone_names.clone(),
            world_bones: self
                .skater
                .render_pose
                .iter()
                .map(|bone| compose_affine(&root, bone))
                .collect(),
        }
    }
}
