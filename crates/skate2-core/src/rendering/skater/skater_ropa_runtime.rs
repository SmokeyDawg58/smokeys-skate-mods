//! Existing SkaterPresEntity CPU skin/cloth jobs, with host-owned decoded streams.
//! Native references: InitSkinParams827d2938, ClothInit827d7d88,
//! ClothUpdate827d24f8 and WriteDestination8285cae0.
use crate::{
    animation::AnimationMatrix,
    character::{CharacterScene, palette::CharacterPalette},
    input_cvar_runtime::InputCvarReferences,
    presentation::PresentationMesh,
    skater_ropa_solver::{RopaDriver, RopaTuning, SkaterRopaSolver},
};
use skate2_formats::render::{
    NativeMeshGeometry, NativeVertexAttributeValues as Values, VertexSemantic as Semantic,
};
use std::sync::Arc;
type Result<T> = std::result::Result<T, String>;

/// The restored time `ClothUpdate` 0x827d24f8 reads from the presentation's time
/// controller: the restored frame and the fraction toward it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClothClock {
    pub(crate) frame: u32,
    pub(crate) fraction: f32,
}

/// The clock `ClothUpdate` 0x827d24f8 picks at 0x827d2574.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ClothTime {
    /// The presentation's time controller.
    Game(ClothClock),
    /// In a character NIS (`SceneManager::IsInCharacterNIS` 0x82862db0), the scene's
    /// elapsed seconds (`ScenePlayer::GetTimeElapsed` 0x8286c9e0).
    Nis(f32),
}

impl ClothTime {
    /// `ClothUpdate` 0x827d2574..0x827d2684: the clock's state, frame and fraction.
    fn read(self) -> (ClothState, f32, f32) {
        match self {
            Self::Game(clock) => {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "the native converts the frame to float"
                )]
                let frame = clock.frame as f32;
                (ClothState::Game, frame, clock.fraction)
            }
            Self::Nis(elapsed) => {
                let (frame, fraction) = nis_frame(elapsed);
                (ClothState::Nis, frame, fraction)
            }
        }
    }
}

/// The position expansion of an entity without cloth effects, which no step reads.
const NO_EFFECT_OFFSET: [f32; 4] = [0., 0.8, 0., 2.];

/// `SkaterPresEntity::mLastClothState` (+0x280): the clock of the entity's last
/// restart, `LAST_CLOTH_STATE_GAME` (1) or `LAST_CLOTH_STATE_NIS` (2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClothState {
    Game,
    Nis,
}

struct Part {
    index: usize,
    solver: SkaterRopaSolver,
    source: NativeMeshGeometry,
    matrix_offsets: Vec<u16>,
    driver_indices: Vec<u32>,
    meshes: Vec<PresentationMesh>,
    rows: Vec<Vec<[f32; 4]>>,
    drivers: Vec<RopaDriver>,
    /// The frame and fraction of this effect's last update (`+0x2a4`, `mPrevDelta`).
    last_clock: Option<(f32, f32)>,
    /// The hips-relative positions the last two steps left, before quantization.
    stepped: Option<(Vec<[f32; 3]>, Vec<[f32; 3]>)>,
    published: bool,
}
pub(crate) struct SkaterRopaRuntime {
    parts: Vec<Part>,
    offset: [f32; 4],
    /// The restored frame the last step reached.
    stepped_frame: Option<u32>,
    /// `mLastClothState` (+0x280), one for every effect; none before the first update.
    state: Option<ClothState>,
    /// The frames in wipe-out of the last restore's step (`mFramesInWipeOut`, +0x2c4).
    wipeout: u32,
}
impl SkaterRopaRuntime {
    pub(crate) fn new(
        scene: &Arc<CharacterScene>,
        palette: &CharacterPalette,
        vault: &Arc<crate::attribute_runtime::AttributeRuntimeVault>,
    ) -> Result<Self> {
        let mut parts = Vec::new();
        'parts: for (index, part) in palette.skinning().parts.iter().enumerate() {
            let lod = &scene.components[part.component_index].models[part.model_index].lods
                [part.lod_index];
            for binding in &lod.ropa_bindings {
                if parts.len() == 4 {
                    break 'parts;
                }
                let model = &lod.arena.models[binding.model_index];
                let source = lod.arena.meshes[binding.mesh_index].geometry.clone();
                // Original ClothInit false deletes only this effect. This is
                // limited to its mapping validation, never a solver fallback.
                let Ok(solver) = SkaterRopaSolver::new(
                    Arc::clone(&binding.effect),
                    &binding.vertex_particle_indices,
                ) else {
                    continue;
                };
                if source.positions.len() != solver.vertex_particles().len() {
                    return Err("cloth source mapping size".into());
                }
                let meshes: Vec<_> = model
                    .meshes
                    .iter()
                    .map(|m| lod.arena.meshes[m.mesh_index].clone())
                    .collect();
                let rows = meshes
                    .iter()
                    .map(|m| vec![[0.; 4]; m.skin_palette.len() * 3])
                    .collect();
                let drivers = vec![
                    RopaDriver {
                        position: [0.; 3],
                        normal: [0.; 3]
                    };
                    solver.positions().len()
                ];
                parts.push(Part {
                    index,
                    solver,
                    source,
                    matrix_offsets: binding.matrix_palette_offsets.clone(),
                    driver_indices: binding.driver_particle_indices.clone(),
                    meshes,
                    rows,
                    drivers,
                    last_clock: None,
                    stepped: None,
                    published: false,
                });
            }
        }
        let offset = if parts.is_empty() {
            NO_EFFECT_OFFSET
        } else {
            position_expansion(&vault.instance(0x38225df1cffaffaa, 0xd7edbd362d7d2152))?
        };
        Ok(Self {
            parts,
            offset,
            stepped_frame: None,
            state: None,
            wipeout: 0,
        })
    }
    /// An entity constructed with its cloth disabled: the `SkaterPresEntity` constructor
    /// skips `InitCloth` (0x827e5920..0x827e5944), so it has no effect to step.
    pub(crate) fn without_effects() -> Self {
        Self {
            parts: Vec::new(),
            offset: NO_EFFECT_OFFSET,
            stepped_frame: None,
            state: None,
            wipeout: 0,
        }
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
    pub(crate) fn view(
        &self,
        part: usize,
        mesh: usize,
    ) -> Option<(&PresentationMesh, &[[f32; 4]])> {
        let p = self.parts.iter().find(|p| p.index == part && p.published)?;
        Some((p.meshes.get(mesh)?, p.rows.get(mesh)?.as_slice()))
    }
    /// `ClothUpdate` 0x827d24f8 for a completed tick: one step, published whole.
    pub(crate) fn update(
        &mut self,
        palette: &CharacterPalette,
        hips: [f32; 16],
        inverse_hips: [f32; 16],
        world: [f32; 16],
        clock: ClothClock,
        wipeout: u32,
        cvars: &InputCvarReferences,
    ) -> Result<()> {
        self.step(palette, hips, inverse_hips, clock, wipeout, cvars)?;
        self.publish(palette, world, 1.)
    }

    /// Whether the cloth has yet to step to `frame`.
    pub(crate) fn needs_step(&self, frame: u32) -> bool {
        self.stepped_frame != Some(frame)
    }

    /// `ClothUpdate`'s step: the drivers skinned from `palette`, the solver stepped by
    /// the restored time since the last step, and the hips-relative positions it leaves.
    pub(crate) fn step(
        &mut self,
        palette: &CharacterPalette,
        hips: [f32; 16],
        inverse_hips: [f32; 16],
        clock: ClothClock,
        wipeout: u32,
        cvars: &InputCvarReferences,
    ) -> Result<()> {
        let tuning = RopaTuning::load(cvars).map_err(|e| e.to_string())?;
        let effect_scale = effect_scale(wipeout, cvars)?;
        self.wipeout = wipeout;
        #[expect(
            clippy::cast_precision_loss,
            reason = "the native converts the frame to float"
        )]
        let frame = clock.frame as f32;
        for part in &mut self.parts {
            skin_drivers(part, palette, hips, self.offset, None)?;
            // ClothUpdate 0x827d26a4..2720: the restored time since this effect's last
            // update steps it; fifteen frames or more either way, or the entity's last
            // restart by the NIS's clock, restart it.
            let dt = effect_time(part.last_clock, frame, clock.fraction);
            let reset = dt.is_none() || self.state != Some(ClothState::Game);
            if reset {
                self.state = Some(ClothState::Game);
            }
            part.solver
                .step(
                    &part.drivers,
                    dt.filter(|_| !reset).unwrap_or(0.),
                    reset,
                    effect_scale,
                    &tuning,
                )
                .map_err(|e| e.to_string())?;
            part.last_clock = Some((frame, clock.fraction));
            let positions = particle_positions(&part.solver, inverse_hips);
            part.stepped = Some(match part.stepped.take() {
                Some((_, last)) if !reset => (last, positions),
                _ => (positions.clone(), positions),
            });
        }
        self.stepped_frame = Some(clock.frame);
        Ok(())
    }

    /// `ClothUpdate` 0x827d24f8 for the entity a NIS borrows
    /// (`SceneCharacterManager::ControlCharacterRenderable` 0x82870fb0). No restore
    /// steps it: `SkaterPresEntity::Update` 0x827e8568 runs `StartJobs` 0x827dbb90
    /// itself each drawn frame, by `time`. `mFramesInWipeOut` keeps the last restore's
    /// frames (`DoSimRestore` 0x827e7294 is its only writer).
    pub(crate) fn step_borrowed(
        &mut self,
        palette: &CharacterPalette,
        hips: [f32; 16],
        inverse_hips: [f32; 16],
        time: ClothTime,
        cvars: &InputCvarReferences,
    ) -> Result<()> {
        let tuning = RopaTuning::load(cvars).map_err(|e| e.to_string())?;
        let effect_scale = effect_scale(self.wipeout, cvars)?;
        let clock = time.read();
        let offset = self.offset;
        for part in &mut self.parts {
            part.step_borrowed(
                &mut self.state,
                clock,
                |part| {
                    let mut skinned = Vec::with_capacity(part.source.positions.len());
                    skin_drivers(part, palette, hips, offset, Some(&mut skinned))?;
                    Ok(skinned)
                },
                inverse_hips,
                effect_scale,
                &tuning,
            )?;
        }
        self.stepped_frame = None;
        Ok(())
    }

    /// `ClothUpdate` 0x827d24f8 for a skater a replay-controlled scene restores:
    /// `DoSimRestore` 0x827e6438 sets `mWasRestoredThisFrame` and runs `StartJobs`
    /// 0x827dbb90 on the restored pose (0x827e7098), so each restore steps the cloth by
    /// the `SceneReplayTimeController`'s frame and fraction (`GetTickCount` 0x82872720) on
    /// the game clock, as [`Self::step_borrowed`] steps. `DoSimRestore` writes
    /// `mFramesInWipeOut` after the jobs start (0x827e7294): the step reads the last
    /// restore's frames and keeps `wipeout` for the next.
    pub(crate) fn step_restored(
        &mut self,
        palette: &CharacterPalette,
        hips: [f32; 16],
        inverse_hips: [f32; 16],
        clock: ClothClock,
        wipeout: u32,
        cvars: &InputCvarReferences,
    ) -> Result<()> {
        self.step_borrowed(palette, hips, inverse_hips, ClothTime::Game(clock), cvars)?;
        self.wipeout = wipeout;
        Ok(())
    }

    /// `WriteDestination` 0x8285cae0: the hips-relative positions `fraction` of the way
    /// from the step before the last to the last, normals and tangents skinned from
    /// `palette`, and `world` in every palette row.
    pub(crate) fn publish(
        &mut self,
        palette: &CharacterPalette,
        world: [f32; 16],
        fraction: f32,
    ) -> Result<()> {
        for part in &mut self.parts {
            part.published = false;
            let Some((before, last)) = &part.stepped else {
                continue;
            };
            let matrices = palette
                .cloth_matrices(part.index)
                .map_err(|e| e.to_string())?;
            let transforms = blends(&part.source, &part.matrix_offsets, matrices)?;
            let positions: Vec<[f32; 3]> = before
                .iter()
                .zip(last)
                .map(|(before, last)| {
                    std::array::from_fn(|i| {
                        let p = last[i].mul_add(fraction, before[i] * (1. - fraction));
                        short((p - self.offset[i]) / self.offset[3])
                    })
                })
                .collect();
            for (mesh_index, mesh) in part.meshes.iter_mut().enumerate() {
                if mesh.geometry.positions.len() != positions.len() {
                    return Err("cloth shared stream count mismatch".into());
                }
                mesh.geometry.positions.copy_from_slice(&positions);
                for destination in &mut mesh.geometry.attributes {
                    match destination.semantic {
                        Semantic::BlendWeight(0) => {
                            destination.values =
                                Values::UnsignedByte4(vec![[255, 0, 0, 0]; positions.len()])
                        }
                        Semantic::BlendIndices(0) => {
                            destination.values =
                                Values::UnsignedByte4(vec![[0; 4]; positions.len()])
                        }
                        Semantic::Normal(0)
                        | Semantic::Tangent(0)
                        | Semantic::Other { usage: 7, index: 0 } => {
                            let source = vectors(&part.source, destination.semantic)?;
                            let values = source
                                .iter()
                                .zip(&transforms)
                                .map(|(v, m)| {
                                    let v = transform(*m, *v, 0.);
                                    [hend(v[0], 1023.), hend(v[1], 1023.), hend(v[2], 511.)]
                                })
                                .collect();
                            destination.values = Values::Float3(values);
                        }
                        _ => {}
                    }
                }
                for rows in part.rows[mesh_index].chunks_exact_mut(3) {
                    for axis in 0..3 {
                        rows[axis] = [
                            world[axis],
                            world[4 + axis],
                            world[8 + axis],
                            world[12 + axis],
                        ];
                    }
                }
            }
            part.published = true;
        }
        Ok(())
    }
}

impl Part {
    /// One effect of [`SkaterRopaRuntime::step_borrowed`]. A restart (0x827d2730..2810)
    /// takes the particles to the drivers the last skin job left, as this frame's job
    /// runs after it; `ROPA::Effect::Update` 0x8285ff38 does nothing under 1e-5 s. A
    /// frame without a step has no `WriteDestination` 0x8285cae0 and draws the skin
    /// job's vertices, which `skin` returns as it skins this frame's drivers.
    fn step_borrowed(
        &mut self,
        last_state: &mut Option<ClothState>,
        (state, frame, fraction): (ClothState, f32, f32),
        skin: impl FnOnce(&mut Self) -> Result<Vec<[f32; 3]>>,
        inverse_hips: [f32; 16],
        effect_scale: f32,
        tuning: &RopaTuning,
    ) -> Result<()> {
        let dt = effect_time(self.last_clock, frame, fraction);
        // 0x827d26a4..26dc: the clock's state is the entity's, so a change of clock
        // restarts only the first effect it reaches.
        let restart = dt.is_none() || *last_state != Some(state);
        if restart {
            *last_state = Some(state);
            self.solver
                .step(&self.drivers, 0., true, effect_scale, tuning)
                .map_err(|e| e.to_string())?;
        }
        let skinned = skin(self)?;
        let dt = dt.filter(|_| !restart).unwrap_or(0.);
        if !restart {
            self.solver
                .step(&self.drivers, dt, false, effect_scale, tuning)
                .map_err(|e| e.to_string())?;
        }
        self.last_clock = Some((frame, fraction));
        let positions = if restart || dt < 1.0e-5 {
            skinned
        } else {
            particle_positions(&self.solver, inverse_hips)
        };
        self.stepped = Some(match self.stepped.take() {
            Some((_, last)) if !restart => (last, positions),
            _ => (positions.clone(), positions),
        });
        Ok(())
    }
}

/// `BeginSkinJob` 0x827d7c48's skin job for one effect: each vertex skinned from
/// `palette`, and its particle's driver in the hips' space. The skinned vertices go
/// to `skinned` when it is given.
fn skin_drivers(
    part: &mut Part,
    palette: &CharacterPalette,
    hips: [f32; 16],
    offset: [f32; 4],
    mut skinned: Option<&mut Vec<[f32; 3]>>,
) -> Result<()> {
    let matrices = palette
        .cloth_matrices(part.index)
        .map_err(|e| e.to_string())?;
    let blends = blends(&part.source, &part.matrix_offsets, matrices)?;
    let normal = vectors(&part.source, Semantic::Normal(0))?;
    for (vertex, (position, blend)) in part.source.positions.iter().zip(&blends).enumerate() {
        // Original char_material_model position expansion and inverse
        // destination packing; shader OffsetPos uses this same space.
        let position = std::array::from_fn(|i| position[i].mul_add(offset[3], offset[i]));
        let p = transform(*blend, position, 1.);
        if let Some(skinned) = skinned.as_deref_mut() {
            skinned.push(p);
        }
        let particle = *part
            .driver_indices
            .get(vertex)
            .ok_or("cloth driver mapping absent")? as usize;
        *part
            .drivers
            .get_mut(particle)
            .ok_or("cloth driver particle outside effect")? = RopaDriver {
            position: transform(hips, p, 1.),
            normal: transform(hips, transform(*blend, normal[vertex], 0.), 0.),
        };
    }
    Ok(())
}

/// `PreClothUpdate` 0x827d2318's `mEffectScale`: one, less the wipe-out's share of
/// `cv_NbWipeOutFrames` during one.
fn effect_scale(wipeout: u32, cvars: &InputCvarReferences) -> Result<f32> {
    let wipeout_max = cvars
        .try_ropa_nb_wipe_out_frames()
        .map_err(|e| e.to_string())?;
    let wipeout = wipeout.min(wipeout_max.max(0) as u32);
    Ok(if wipeout == 0 {
        1.
    } else {
        1. - wipeout as f32 / wipeout_max as f32
    })
}

/// `ClothUpdate` 0x827d2688..0x827d2720: the time since an effect's last update, in
/// frames plus the change of fraction, a sixtieth of a second a frame, by its magnitude
/// (`fsel`); none before a first update or fifteen frames or more away either way.
fn effect_time(last_clock: Option<(f32, f32)>, frame: f32, fraction: f32) -> Option<f32> {
    last_clock
        .map(|(last_frame, last_fraction)| (frame - last_frame, last_fraction))
        .filter(|(frames, _)| *frames < 15. && *frames > -15.)
        .map(|(frames, last_fraction)| {
            (((fraction - last_fraction) + frames) * f32::from_bits(0x3c88_8889)).abs()
        })
}

/// `ClothUpdate` 0x827d25d8..0x827d2630: the NIS's elapsed seconds in sixtieths, its
/// frame floored (`vrfim`) through an integer (`fctiwz`, `fcfid`) and its fraction the
/// rest.
fn nis_frame(elapsed: f32) -> (f32, f32) {
    let sixtieths = elapsed * 60.;
    #[expect(clippy::cast_possible_truncation, reason = "fctiwz")]
    let whole = sixtieths.floor() as i32;
    #[expect(clippy::cast_precision_loss, reason = "fcfid, then frsp")]
    let frame = whole as f32;
    (frame, sixtieths - frame)
}

/// `WriteDestination` 0x8285cae0's positions before their packing: each vertex's
/// particle through the inverse of the hips' transform.
fn particle_positions(solver: &SkaterRopaSolver, inverse_hips: [f32; 16]) -> Vec<[f32; 3]> {
    solver
        .vertex_particles()
        .iter()
        .map(|&particle| transform(inverse_hips, solver.positions()[usize::from(particle)], 1.))
        .collect()
}

/// `DoSkinMesh_vector` 0x82841760: each vertex's four weighted palette matrices, blended.
fn blends(
    source: &NativeMeshGeometry,
    matrix_offsets: &[u16],
    matrices: &[AnimationMatrix],
) -> Result<Vec<[f32; 16]>> {
    let weights = bytes(source, Semantic::BlendWeight(0))?;
    let indices = bytes(source, Semantic::BlendIndices(0))?;
    let mut blends = Vec::with_capacity(source.positions.len());
    for vertex in 0..source.positions.len() {
        let offset = *matrix_offsets
            .get(vertex)
            .ok_or("cloth matrix offset absent")? as usize;
        let mut blend = [0.; 16];
        // DoSkinMesh_vector82841760..82841880 duplicates each byte
        // across a word, shifts right1, then VCFUX scale31. Keep that
        // conversion before the four-weight normalization.
        let weight = weights[vertex]
            .map(|v| ((u32::from(v) * 0x0101_0101) >> 1) as f32 * (1.0 / 2147483648.0));
        let sum = (weight[0] + weight[1]) + (weight[2] + weight[3]);
        if sum == 0. {
            return Err("cloth source has zero skin weight sum".into());
        }
        let inverse = sum.recip();
        let inverse = inverse.mul_add(-sum.mul_add(inverse, -1.), inverse);
        for influence in 0..4 {
            let matrix = matrices
                .get(offset + usize::from(indices[vertex][influence]))
                .ok_or("cloth matrix index absent")?;
            let weight = weight[influence] * inverse;
            for i in 0..16 {
                blend[i] = if influence == 0 {
                    matrix[i] * weight
                } else {
                    matrix[i].mul_add(weight, blend[i])
                };
            }
        }
        blends.push(blend);
    }
    Ok(blends)
}

/// InitSkinParams827d2f5c..827d3038 reads g_charattributes[1] from
/// material_charattributes' cached 64-byte layout, not GetAttributePointer.
/// Its float4 array starts at +16, so element 1 is at +32. GetLength829766b8
/// reads the count at +2. The generated constructor8272bb48 uses a zeroed
/// DefaultDataArea(64) for a missing layout; a short array returns zero too.
fn position_expansion(
    attributes: &crate::attribute_runtime::AttributeRuntimeInstance,
) -> Result<[f32; 4]> {
    let Some(layout) = attributes.try_cached_layout(64).map_err(|e| e.to_string())? else {
        return Ok([0.; 4]);
    };
    let bytes = layout.bytes();
    let count = u16::from_be_bytes(bytes[2..4].try_into().expect("layout count"));
    if count <= 1 {
        return Ok([0.; 4]);
    }
    Ok(std::array::from_fn(|lane| {
        let offset = 32 + lane * 4;
        f32::from_be_bytes(bytes[offset..offset + 4].try_into().expect("layout float4"))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute_runtime::tests::{collection, graph, set_dependency_bytes};

    #[test]
    fn cloth_position_expansion_uses_cached_layout_count_and_second_element() {
        for count in [0u16, 1, 3] {
            let mut vault = graph(1, vec![]);
            collection(&mut vault, 1, 10, 0, Some(0), vec![]);
            let mut bytes = vec![0; 64];
            bytes[..2].copy_from_slice(&3u16.to_be_bytes()); // capacity is not count
            bytes[2..4].copy_from_slice(&count.to_be_bytes());
            bytes[4..6].copy_from_slice(&16u16.to_be_bytes());
            bytes[6..8].copy_from_slice(&0x8000u16.to_be_bytes());
            for (index, value) in [0f32, 0.8, 0., 2.].into_iter().enumerate() {
                bytes[32 + index * 4..36 + index * 4].copy_from_slice(&value.to_be_bytes());
            }
            set_dependency_bytes(&mut vault, 1, bytes);
            let vault = Arc::new(vault);
            assert_eq!(position_expansion(&vault.instance(1, 10)).unwrap(),
                if count > 1 { [0., 0.8, 0., 2.] } else { [0.; 4] });
            assert_eq!(position_expansion(&vault.instance(1, 99)).unwrap(), [0.; 4]);
        }
    }

    const IDENTITY: [f32; 16] = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ];
    const PREVIOUS: [[f32; 3]; 2] = [[1., 2., 3.], [4., 5., 6.]];
    const CURRENT: [[f32; 3]; 2] = [[7., 8., 9.], [10., 11., 12.]];

    /// A two-particle effect over two vertices, whose drivers hold `PREVIOUS`.
    fn effect_part() -> Part {
        use skate2_formats::ropa::{RopaDriverRange, RopaEffect, RopaParticle};
        let positions = [[9.; 3], [8.; 3]];
        let effect = Arc::new(RopaEffect {
            particles: positions
                .iter()
                .map(|&position| RopaParticle {
                    position,
                    inverse_mass: 1.,
                    movable: true,
                })
                .collect(),
            drivers: vec![RopaDriverRange { min: 0., max: 0. }; 2],
            distances: Vec::new(),
        });
        Part {
            index: 0,
            solver: SkaterRopaSolver::new(effect, &[0, 1]).unwrap(),
            source: NativeMeshGeometry {
                positions: positions.to_vec(),
                attributes: Vec::new(),
                indices: Vec::new(),
                draws: Vec::new(),
                position_format: 0,
            },
            matrix_offsets: Vec::new(),
            driver_indices: vec![0, 1],
            meshes: Vec::new(),
            rows: Vec::new(),
            drivers: PREVIOUS
                .iter()
                .map(|&position| RopaDriver {
                    position,
                    normal: [0., 1., 0.],
                })
                .collect(),
            last_clock: None,
            stepped: None,
            published: false,
        }
    }

    /// Integration off (`dt <= MinDTIntegration`); a zero effect scale then leaves a
    /// stepped particle on its driver.
    fn solve_only() -> RopaTuning {
        RopaTuning {
            max_dt_ratio: 0.5,
            min_dt_integration: 1.,
            fixed_dt: false,
            gravity: 9.8,
            vertical_speed_dampening: 0.,
            compression: 0.,
            vert_distance_scale: 1.,
            norm_distance_scale: 1.,
            nb_iterations: 1,
            use_constraint_length_on_normals: false,
            max_dt: 1.,
            ratio_max_iterations: 50,
        }
    }

    /// One borrowed update that skins `CURRENT` and the skinned vertices `skinned`.
    fn step(
        part: &mut Part,
        state: &mut Option<ClothState>,
        clock: (ClothState, f32, f32),
        skinned: [f32; 3],
    ) {
        part.step_borrowed(
            state,
            clock,
            |part| {
                for (driver, position) in part.drivers.iter_mut().zip(CURRENT) {
                    driver.position = position;
                }
                Ok(vec![skinned; 2])
            },
            IDENTITY,
            0.,
            &solve_only(),
        )
        .unwrap();
    }

    #[test]
    fn a_change_of_clock_restarts_on_the_last_jobs_drivers_and_draws_the_skin() {
        let mut part = effect_part();
        part.last_clock = Some((2., 0.5));
        let mut state = Some(ClothState::Game);
        step(&mut part, &mut state, (ClothState::Nis, 3., 0.5), [0.5; 3]);
        assert_eq!(state, Some(ClothState::Nis));
        assert_eq!(part.solver.positions(), PREVIOUS);
        assert_eq!(part.stepped, Some((vec![[0.5; 3]; 2], vec![[0.5; 3]; 2])));
        assert_eq!(part.last_clock, Some((3., 0.5)));
        // The same moment again steps by nothing: the particles stay and the skin
        // job's vertices are drawn.
        step(&mut part, &mut state, (ClothState::Nis, 3., 0.5), [0.25; 3]);
        assert_eq!(part.solver.positions(), PREVIOUS);
        assert_eq!(part.stepped, Some((vec![[0.5; 3]; 2], vec![[0.25; 3]; 2])));
        // A frame later the particles solve against this frame's drivers.
        step(
            &mut part,
            &mut state,
            (ClothState::Nis, 4., 0.5),
            [0.125; 3],
        );
        assert_eq!(part.solver.positions(), CURRENT);
        assert_eq!(part.stepped, Some((vec![[0.25; 3]; 2], CURRENT.to_vec())));
    }

    #[test]
    fn a_change_of_clock_restarts_only_the_first_effect() {
        let (mut first, mut second) = (effect_part(), effect_part());
        first.last_clock = Some((10., 0.));
        second.last_clock = Some((10., 0.));
        let initial = second.solver.positions().to_vec();
        let mut state = Some(ClothState::Game);
        step(&mut first, &mut state, (ClothState::Nis, 10., 0.), [0.5; 3]);
        step(
            &mut second,
            &mut state,
            (ClothState::Nis, 10., 0.),
            [0.5; 3],
        );
        assert_eq!(first.solver.positions(), PREVIOUS);
        assert_eq!(second.solver.positions(), initial);
        // Fifteen frames away restarts it on its own.
        step(
            &mut second,
            &mut state,
            (ClothState::Nis, 25., 0.),
            [0.5; 3],
        );
        assert_eq!(second.solver.positions(), CURRENT);
    }

    #[test]
    fn the_nis_clock_floors_the_elapsed_sixtieths_through_an_integer() {
        assert_eq!(nis_frame(0.5), (30., 0.));
        assert_eq!(nis_frame(0.008), (0., 0.008_f32 * 60.));
        // vrfim floors: a time before the scene's start is in the frame below.
        assert_eq!(nis_frame(-0.01), (-1., -0.01_f32 * 60. + 1.));
        assert_eq!(ClothTime::Nis(0.25).read(), (ClothState::Nis, 15., 0.));
        let game = ClothTime::Game(ClothClock {
            frame: 7,
            fraction: 0.25,
        });
        assert_eq!(game.read(), (ClothState::Game, 7., 0.25));
    }

    #[test]
    fn an_effects_time_is_a_sixtieth_a_frame_within_fifteen_frames() {
        let sixtieth = f32::from_bits(0x3c88_8889);
        assert_eq!(effect_time(None, 3., 0.), None);
        assert_eq!(effect_time(Some((0., 0.)), 15., 0.), None);
        assert_eq!(effect_time(Some((15., 0.)), 0., 0.), None);
        assert_eq!(
            effect_time(Some((2., 0.75)), 1., 0.5),
            Some((((0.5_f32 - 0.75) - 1.) * sixtieth).abs())
        );
        assert_eq!(effect_time(Some((0., 0.)), 14., 0.), Some(14. * sixtieth));
    }
}
fn bytes(g: &NativeMeshGeometry, s: Semantic) -> Result<&[[u8; 4]]> {
    match &g
        .attributes
        .iter()
        .find(|a| a.semantic == s)
        .ok_or("cloth byte attribute absent")?
        .values
    {
        Values::UnsignedByte4(v) if v.len() == g.positions.len() => Ok(v),
        _ => Err("cloth byte attribute format".into()),
    }
}
fn vectors(g: &NativeMeshGeometry, s: Semantic) -> Result<&[[f32; 3]]> {
    match &g
        .attributes
        .iter()
        .find(|a| a.semantic == s)
        .ok_or("cloth vector attribute absent")?
        .values
    {
        Values::Float3(v) if v.len() == g.positions.len() => Ok(v),
        _ => Err("cloth vector attribute format".into()),
    }
}
fn transform(m: [f32; 16], v: [f32; 3], w: f32) -> [f32; 3] {
    std::array::from_fn(|i| {
        m[12 + i].mul_add(
            w,
            m[8 + i].mul_add(v[2], m[4 + i].mul_add(v[1], m[i] * v[0])),
        )
    })
}
fn short(v: f32) -> f32 {
    // vec_put_short4_clamped8285a060: clamp, VCFPSXWS31 (signed32
    // saturation/truncation), shift16, then retain the low halfword. Negative
    // fractions therefore floor at the halfword boundary, not truncate.
    let fixed = (v.clamp(-1., 1.) * 2147483648.0) as i32;
    ((fixed >> 16) as f32 / 32767.).max(-1.)
}
fn hend(v: f32, scale: f32) -> f32 {
    // DoSkinMesh_vector packs signed32 truncations with 11/11/10-bit masks;
    // there is no extra per-component saturation before those masks.
    let bits = if scale == 511. { 10 } else { 11 };
    let mask = (1_i32 << bits) - 1;
    let raw = ((v * scale) as i32) & mask;
    let signed = (raw << (32 - bits)) >> (32 - bits);
    (signed as f32 / scale).max(-1.)
}
