//! ROPA particle state and shipped vector solver graph, 8285ac38..8285be84.
//! Skinning and publication remain with the existing character owners. Positions
//! here are world-space; WriteDestination's inverse hips transform is a caller
//! operation. Host estimates follow the existing presentation numeric policy.
use skate2_formats::ropa::{RopaEffect, RopaError};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub(crate) struct RopaDriver {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RopaTuning {
    pub max_dt_ratio: f32,
    pub min_dt_integration: f32,
    pub fixed_dt: bool,
    pub gravity: f32,
    pub vertical_speed_dampening: f32,
    pub compression: f32,
    pub vert_distance_scale: f32,
    pub norm_distance_scale: f32,
    pub nb_iterations: i32,
    /// Retained native cvar; neither of the original driver solvers reads it.
    pub use_constraint_length_on_normals: bool,
    /// Original non-cvar globals used only with FixedDT disabled.
    pub max_dt: f32,
    pub ratio_max_iterations: i32,
}

impl RopaTuning {
    pub(crate) fn load(
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
    ) -> Result<Self, crate::input_cvar_runtime::InputCvarError> {
        Ok(Self {
            max_dt_ratio: cvars.try_ropa_maxdt_ratio()?,
            min_dt_integration: cvars.try_ropa_mindt_integration()?,
            fixed_dt: cvars.try_ropa_fixeddt()?,
            gravity: cvars.try_ropa_gravity()?,
            vertical_speed_dampening: cvars.try_ropa_vertical_speed_dampening()?,
            compression: cvars.try_ropa_compression()?,
            vert_distance_scale: cvars.try_ropa_vert_distance_scale()?,
            norm_distance_scale: cvars.try_ropa_norm_distance_scale()?,
            nb_iterations: cvars.try_ropa_nb_iterations()?,
            use_constraint_length_on_normals: cvars.try_ropa_use_constraint_length_on_normals()?,
            max_dt: 1.0,              //82ff0278
            ratio_max_iterations: 50, //82ff0274
        })
    }
}

#[derive(Debug)]
pub(crate) struct SkaterRopaSolver {
    effect: Arc<RopaEffect>,
    positions: Vec<[f32; 3]>,
    previous: Vec<[f32; 3]>,
    inverse_masses: Vec<f32>,
    vertex_particles: Vec<u16>,
    particle_vertices: Vec<u16>,
    dt_history: [f32; 30],
    dt_index: usize,
    last_dt: f32,
    last_full_dt: f32,
    reset_integrator: bool,
}

impl SkaterRopaSolver {
    /// ClothInit827d7d88: position.W is an integer particle number. Every
    /// particle must have a source vertex and the last duplicate wins.
    pub(crate) fn new(
        effect: Arc<RopaEffect>,
        vertex_particle_indices: &[u32],
    ) -> Result<Self, RopaError> {
        let mut particle_vertices = vec![u16::MAX; effect.particles.len()];
        let mut vertex_particles = Vec::with_capacity(vertex_particle_indices.len());
        for (vertex, &particle) in vertex_particle_indices.iter().enumerate() {
            let target = particle_vertices
                .get_mut(particle as usize)
                .ok_or(RopaError("ClothInit vertex particle outside effect"))?;
            *target = u16::try_from(vertex)
                .map_err(|_| RopaError("ClothInit vertex exceeds native mapping"))?;
            vertex_particles.push(particle as u16);
        }
        if particle_vertices.iter().any(|&vertex| vertex >= 0x8000) {
            return Err(RopaError("ClothInit particle has no native source vertex"));
        }
        let positions: Vec<_> = effect.particles.iter().map(|p| p.position).collect();
        let inverse_masses = effect.particles.iter().map(|p| p.inverse_mass).collect();
        Ok(Self {
            effect,
            previous: positions.clone(),
            positions,
            inverse_masses,
            vertex_particles,
            particle_vertices,
            dt_history: [0.0; 30],
            dt_index: 0,
            last_dt: 1.0,
            last_full_dt: 1.0 / 60.0,
            reset_integrator: true,
        })
    }
    pub(crate) fn vertex_particles(&self) -> &[u16] {
        &self.vertex_particles
    }
    pub(crate) fn particle_vertices(&self) -> &[u16] {
        &self.particle_vertices
    }
    pub(crate) fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }

    pub(crate) fn step(
        &mut self,
        drivers: &[RopaDriver],
        dt: f32,
        reset: bool,
        effect_scale: f32,
        tuning: &RopaTuning,
    ) -> Result<&[[f32; 3]], RopaError> {
        if drivers.len() != self.positions.len()
            || !dt.is_finite()
            || !effect_scale.is_finite()
            || drivers.iter().any(|v| {
                v.position
                    .iter()
                    .chain(v.normal.iter())
                    .any(|x| !x.is_finite())
            })
        {
            return Err(RopaError("invalid ROPA skinned driver input"));
        }
        // ClothUpdate827d24f8 resets the simulation directly, without calling
        // Effect::Update on this branch. Keep the averaging ring intact.
        if reset {
            for (i, driver) in drivers.iter().enumerate() {
                self.positions[i] = driver.position;
                self.previous[i] = driver.position;
            }
            self.reset_integrator = true;
            return Ok(&self.positions);
        }
        if dt < 1.0e-5 {
            return Ok(&self.positions);
        }
        if !tuning.gravity.is_finite()
            || !tuning.vertical_speed_dampening.is_finite()
            || !tuning.compression.is_finite()
            || !tuning.vert_distance_scale.is_finite()
            || !tuning.norm_distance_scale.is_finite()
            || tuning.vert_distance_scale <= 0.0
            || tuning.norm_distance_scale <= 0.0
        {
            return Err(RopaError("invalid ROPA tuning parameters"));
        }
        if tuning.fixed_dt {
            self.dt_history[self.dt_index] = dt;
            self.dt_index = (self.dt_index + 1) % 30;
            let average = self
                .dt_history
                .iter()
                .fold(0.0_f32, |sum, value| sum + *value)
                * (1.0 / 30.0);
            if self.reset_integrator {
                self.last_dt = average;
                self.reset_integrator = false;
            }
            self.integrate(average, tuning);
            self.last_dt = average;
            self.solve(drivers, effect_scale, tuning);
        } else if dt <= tuning.min_dt_integration {
            self.solve(drivers, effect_scale, tuning);
        } else {
            if !tuning.max_dt.is_finite() || tuning.max_dt <= 0.0 {
                return Err(RopaError("invalid ROPA maximum integration timestep"));
            }
            let divided = dt / tuning.max_dt;
            if divided >= (i32::MAX - 1) as f32 {
                return Err(RopaError("ROPA subdivision count overflow"));
            }
            let subdivisions = divided as i32 + 1;
            let sub_dt = dt / subdivisions as f32;
            for _ in 0..subdivisions {
                let (ratio, steps) = euler_ratio(
                    sub_dt / self.last_dt,
                    tuning.max_dt_ratio,
                    tuning.ratio_max_iterations,
                )?;
                let mut integrated_dt = self.last_dt;
                for _ in 0..steps {
                    integrated_dt *= ratio;
                    if self.reset_integrator {
                        self.last_dt = dt;
                        self.reset_integrator = false;
                    }
                    self.integrate(integrated_dt, tuning);
                    // Raw8285fc4c stores the *whole* input timestep here.
                    self.last_dt = dt;
                    self.solve(drivers, effect_scale, tuning);
                }
                self.last_full_dt = dt;
            }
        }
        if self.positions.iter().flatten().any(|v| !v.is_finite()) {
            return Err(RopaError("non-finite ROPA solver result"));
        }
        Ok(&self.positions)
    }

    fn integrate(&mut self, dt: f32, tuning: &RopaTuning) {
        let ratio = dt * reciprocal(self.last_dt);
        let gravity = -tuning.gravity * (dt * dt);
        for (i, particle) in self.effect.particles.iter().enumerate() {
            if !particle.movable {
                continue;
            }
            let current = self.positions[i];
            let previous = self.previous[i];
            self.previous[i] = current;
            // The shipped vector integrator does not read the scalar-only
            // vertical damping cvar. It fuses velocity into current+gravity.
            self.positions[i] = [
                (current[0] - previous[0]).mul_add(ratio, current[0]),
                (current[1] - previous[1]).mul_add(ratio, current[1] + gravity),
                (current[2] - previous[2]).mul_add(ratio, current[2]),
            ];
        }
    }

    fn solve(&mut self, drivers: &[RopaDriver], effect_scale: f32, tuning: &RopaTuning) {
        // SolveConstraints8285de68: all distance passes precede driver passes.
        for _ in 0..tuning.nb_iterations {
            for constraint in self.effect.distances.iter().flatten().flatten() {
                let a = constraint.particle as usize;
                let b = constraint.reference as usize;
                let delta = sub(self.positions[a], self.positions[b]);
                let squared =
                    delta[0].mul_add(delta[0], delta[1].mul_add(delta[1], delta[2] * delta[2]));
                let length = vector_sqrt(squared);
                let inverse_a = self.inverse_masses[a];
                let inverse_b = self.inverse_masses[b];
                // Both final position transposes inject v0=[1;4] as W.
                //8285b4a8/b508, stores b4c0..b534. W is inverse mass.
                self.inverse_masses[a] = 1.0;
                self.inverse_masses[b] = 1.0;
                if length <= 1.0e-5 {
                    continue;
                }
                let minimum = constraint.min - tuning.compression;
                let inverse_denominator = reciprocal(length * (inverse_b + inverse_a));
                let maximum_factor = (length - constraint.max) * inverse_denominator;
                let minimum_factor = (length - minimum) * inverse_denominator;
                let factor = if minimum >= length {
                    minimum_factor
                } else if length > constraint.max {
                    maximum_factor
                } else {
                    0.0
                };
                let b_correction = scale(delta, inverse_b * factor);
                let a_correction = scale(delta, inverse_a * factor);
                let movable_b = self.effect.particles[b].movable;
                if movable_b {
                    self.positions[b] = add(self.positions[b], b_correction);
                }
                // Preserve the vector implementation's shared lane correction:
                // v41=deltaX*inverseB*minimumFactor; v9=-v41 iff B static;
                // b4e0/b4e4/b4e8 subtract that same v9 from Z/Y/X.
                let reference_correction = if movable_b {
                    0.0
                } else {
                    -(delta[0] * (inverse_b * minimum_factor))
                };
                if self.effect.particles[a].movable {
                    self.positions[a] = std::array::from_fn(|axis| {
                        self.positions[a][axis] - (a_correction[axis] - reference_correction)
                    });
                }
            }
        }
        // Driver set mNbIterations stays constructor1, independent of distance
        // tuning. Both source solver paths choose min>0 => reversed normal.
        for (i, range) in self.effect.drivers.iter().enumerate() {
            let driver = drivers[i];
            if !self.effect.particles[i].movable {
                self.positions[i] = driver.position;
                continue;
            }
            let (normal, maximum) = if range.min > 0.0 {
                (scale(driver.normal, -1.0), range.min)
            } else {
                (driver.normal, range.max)
            };
            let mut delta = sub(driver.position, self.positions[i]);
            delta[1] *= reciprocal(tuning.vert_distance_scale);
            let along = scale(normal, vector_dot(delta, normal));
            let tangent = sub(delta, along);
            let scaled = mul_add(tangent, reciprocal(tuning.norm_distance_scale), along);
            let squared = vector_dot(scaled, scaled);
            let inverse_length = if squared == 0.0 {
                0.0
            } else {
                reciprocal_sqrt(squared)
            };
            let distance = squared * inverse_length;
            if distance > maximum {
                let correction = scale(scaled, (distance - maximum) * inverse_length);
                let along = scale(normal, vector_dot(correction, normal));
                let correction = mul_add(sub(correction, along), tuning.norm_distance_scale, along);
                self.positions[i] = [
                    self.positions[i][0] + correction[0],
                    correction[1].mul_add(tuning.vert_distance_scale, self.positions[i][1]),
                    self.positions[i][2] + correction[2],
                ];
            }
            let penetration = vector_dot(sub(driver.position, self.positions[i]), normal);
            if penetration > 0.0 {
                self.positions[i] = mul_add(normal, penetration, self.positions[i]);
            }
            self.positions[i] = mul_add(
                sub(self.positions[i], driver.position),
                effect_scale,
                driver.position,
            );
        }
    }
}

/// EulerIntegrationOfDtRatio8285ab58, including its final (unrechecked) ratio
/// adjustment. A nonconvergent cvar combination is reported, not busy-looped.
fn euler_ratio(target: f32, maximum: f32, max_iterations: i32) -> Result<(f32, i32), RopaError> {
    if !target.is_finite() || target < 0.0 || !maximum.is_finite() || maximum <= 0.0 {
        return Err(RopaError("invalid ROPA timestep ratio"));
    }
    if maximum < 1.0 && target >= maximum / (1.0 - maximum) {
        return Err(RopaError("nonconvergent native ROPA timestep ratio"));
    }
    let mut steps = 1_i32;
    let mut product = 1.0_f32;
    let mut sum = 0.0_f32;
    loop {
        product *= maximum;
        let next = sum + product;
        if next > target {
            break;
        }
        if next == sum {
            return Err(RopaError("nonconvergent native ROPA timestep sum"));
        }
        sum = next;
        steps = steps
            .checked_add(1)
            .ok_or(RopaError("ROPA timestep count overflow"))?;
    }
    let mut ratio = maximum;
    let mut delta = maximum * 0.5;
    let mut iteration = 0;
    loop {
        iteration += 1;
        let mut sum = ratio;
        for _ in 1..steps {
            sum = (sum + 1.0) * ratio;
        }
        let error = target - sum;
        ratio += if error >= 0.0 { delta } else { -delta };
        delta *= 0.5;
        if iteration >= max_iterations || error.abs() <= 0.001 {
            break;
        }
    }
    Ok((ratio, steps))
}
fn vector_dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[2].mul_add(b[2], a[1].mul_add(b[1], a[0] * b[0]))
}
fn vector_sqrt(squared: f32) -> f32 {
    if squared == 0.0 {
        return 0.0;
    }
    squared * reciprocal_sqrt(squared)
}
// Same explicit host seed boundary as HostReckoningNumeric, but these vector
// graphs perform ONE Newton step (the scalar variants perform two).
fn reciprocal_sqrt(value: f32) -> f32 {
    let seed = value.sqrt().recip();
    let square = seed * seed;
    let residual = (-value).mul_add(square, 1.0);
    residual.mul_add(seed * 0.5, seed)
}
fn reciprocal(value: f32) -> f32 {
    let seed = value.recip();
    let residual = (-seed).mul_add(value, 1.0);
    seed.mul_add(residual, seed)
}
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn scale(a: [f32; 3], value: f32) -> [f32; 3] {
    a.map(|v| v * value)
}
fn mul_add(a: [f32; 3], value: f32, b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i].mul_add(value, b[i]))
}
