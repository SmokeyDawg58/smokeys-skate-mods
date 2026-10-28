//! Evidence-bounded value layout for `rw::physics::ContactJacobian`.
//!
//! The indirect-island Contact consumer at `0x82ae2990..0x82ae2c48`
//! operates on a `0x100`-byte, sixteen-vector record. This module freezes that
//! PDB layout, pure ordinary-finite compiler, per-record Horus consumer, and a
//! private manager-owned, bounded, frame-local network Contact path through
//! sealed dense compilation, stable ten-island table preparation, and one
//! checked selected-island RF sweep. Native volatile-count publication,
//! non-network production, mixed Contact/Joint/Drive iteration, and the outer
//! Simulation pipeline remain open.
//!
//! Native `m_ia` and `m_ib` are RF-row addresses, while PDB `m_bodya` and
//! `m_bodyb` are integer binding words. Their host counterparts are private
//! inert tokens, not native pointers, truncated host pointers, stable public
//! IDs, or mutation capabilities.

use crate::simulation_constraint_math::{
    horus_columns_from, horus_cross_add, horus_madd, horus_negative_columns_from, horus_nmsub,
    pack_xyz_word,
};

#[repr(transparent)]
#[derive(Clone, Copy, Default, Eq, PartialEq)]
struct SolverDeltaRowToken(u32);

#[repr(transparent)]
#[derive(Clone, Copy, Default, Eq, PartialEq)]
struct ContactBodyToken(u32);

type ContactVector = [f32; 4];

/// One copied Contact input row whose W lane remains uninterpreted bits.
///
/// The word may be a float, raw state, or inert native binding value. This
/// private representation prevents compiler input from becoming authority.
#[derive(Clone, Copy, PartialEq)]
struct SimulationContactBuildWordVector {
    xyz: [f32; 3],
    word: u32,
}

impl SimulationContactBuildWordVector {
    #[cfg(test)]
    fn from_words(words: [u32; 4]) -> Self {
        Self {
            xyz: [
                f32::from_bits(words[0]),
                f32::from_bits(words[1]),
                f32::from_bits(words[2]),
            ],
            word: words[3],
        }
    }

    fn vector(self) -> ContactVector {
        pack_xyz_word(self.xyz, f32::from_bits(self.word))
    }
}

/// Complete copied `Contact` value consumed by one in-place compiler visit.
///
/// Native PDB declares rows six through fifteen as padding. Their concrete gameplay
/// producer fills the body-side values named here. This value is private,
/// inert, and neither a producer API nor a pool capability.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct SimulationContactBuildInput {
    position_a_body_a: SimulationContactBuildWordVector,
    position_b_body_b: SimulationContactBuildWordVector,
    contact_right_restitution: SimulationContactBuildWordVector,
    contact_up_static_friction: SimulationContactBuildWordVector,
    contact_at_dynamic_friction: SimulationContactBuildWordVector,
    contact_velocity_tag: SimulationContactBuildWordVector,
    side_0_com_rf_token: SimulationContactBuildWordVector,
    side_1_com_rf_token: SimulationContactBuildWordVector,
    side_0_inertia_full_inverse_mass: SimulationContactBuildWordVector,
    side_1_inertia_full_inverse_mass: SimulationContactBuildWordVector,
    side_0_inertia_split_state: SimulationContactBuildWordVector,
    side_1_inertia_split_state: SimulationContactBuildWordVector,
    side_0_force_kine: SimulationContactBuildWordVector,
    side_1_force_kine: SimulationContactBuildWordVector,
    side_0_torque_cool: SimulationContactBuildWordVector,
    side_1_torque_cool: SimulationContactBuildWordVector,
}

impl SimulationContactBuildInput {
    #[cfg(test)]
    pub(super) fn from_words(words: [[u32; 4]; 16]) -> Self {
        let row = |index: usize| SimulationContactBuildWordVector::from_words(words[index]);
        Self {
            position_a_body_a: row(0),
            position_b_body_b: row(1),
            contact_right_restitution: row(2),
            contact_up_static_friction: row(3),
            contact_at_dynamic_friction: row(4),
            contact_velocity_tag: row(5),
            side_0_com_rf_token: row(6),
            side_1_com_rf_token: row(7),
            side_0_inertia_full_inverse_mass: row(8),
            side_1_inertia_full_inverse_mass: row(9),
            side_0_inertia_split_state: row(10),
            side_1_inertia_split_state: row(11),
            side_0_force_kine: row(12),
            side_1_force_kine: row(13),
            side_0_torque_cool: row(14),
            side_1_torque_cool: row(15),
        }
    }

    #[cfg(test)]
    pub(super) fn test_words(self) -> [[u32; 4]; 16] {
        let row = |value: SimulationContactBuildWordVector| {
            [
                value.xyz[0].to_bits(),
                value.xyz[1].to_bits(),
                value.xyz[2].to_bits(),
                value.word,
            ]
        };
        [
            row(self.position_a_body_a),
            row(self.position_b_body_b),
            row(self.contact_right_restitution),
            row(self.contact_up_static_friction),
            row(self.contact_at_dynamic_friction),
            row(self.contact_velocity_tag),
            row(self.side_0_com_rf_token),
            row(self.side_1_com_rf_token),
            row(self.side_0_inertia_full_inverse_mass),
            row(self.side_1_inertia_full_inverse_mass),
            row(self.side_0_inertia_split_state),
            row(self.side_1_inertia_split_state),
            row(self.side_0_force_kine),
            row(self.side_1_force_kine),
            row(self.side_0_torque_cool),
            row(self.side_1_torque_cool),
        ]
    }
}

impl SimulationContactBuildInput {
    /// `rw::physics::Contact::IsValid` 0x827cb980: every point, velocity and
    /// frame lane and the three material words are ordered, restitution lies
    /// in [0, 1], `0 <= mud <= mus`, the frame's `GetOrthogonalError` is within
    /// `tolerance` and the two table body words differ.
    fn is_valid_contact(&self, tolerance: f32) -> bool {
        let ordered = |row: SimulationContactBuildWordVector| !row.xyz.iter().any(|v| v.is_nan());
        let restitution = f32::from_bits(self.contact_right_restitution.word);
        let dynamic_friction = f32::from_bits(self.contact_at_dynamic_friction.word);
        let static_friction = f32::from_bits(self.contact_up_static_friction.word);
        let rows = ordered(self.position_a_body_a)
            && ordered(self.position_b_body_b)
            && ordered(self.contact_velocity_tag)
            && ordered(self.contact_right_restitution)
            && ordered(self.contact_up_static_friction)
            && ordered(self.contact_at_dynamic_friction);
        let materials = !restitution.is_nan()
            && !dynamic_friction.is_nan()
            && !static_friction.is_nan()
            && restitution >= 0.0
            && restitution <= 1.0
            && dynamic_friction >= 0.0
            && static_friction >= dynamic_friction;
        let error = contact_frame_orthogonal_error([
            self.contact_right_restitution.xyz,
            self.contact_up_static_friction.xyz,
            self.contact_at_dynamic_friction.xyz,
        ]);
        let orthogonal = error <= tolerance && error >= -tolerance;
        rows && materials
            && orthogonal
            && self.position_a_body_a.word != self.position_b_body_b.word
    }
}

/// `GetOrthogonalError<float>` 0x827cb8a0 of `Mult<float>(m, transpose(m))`
/// 0x827cb678, in their exact fused grouping: the sum of the squared row
/// errors of `m * transpose(m) - I`.
fn contact_frame_orthogonal_error(m: [[f32; 3]; 3]) -> f32 {
    let r00 = m[0][0].mul_add(m[0][0], m[0][1].mul_add(m[0][1], m[0][2] * m[0][2]));
    let r01 = m[1][1].mul_add(m[0][1], m[0][2].mul_add(m[1][2], m[1][0] * m[0][0]));
    let r02 = m[2][1].mul_add(m[0][1], m[0][2].mul_add(m[2][2], m[2][0] * m[0][0]));
    let r10 = m[0][0].mul_add(m[1][0], m[0][2].mul_add(m[1][2], m[1][1] * m[0][1]));
    let r11 = m[1][2].mul_add(m[1][2], m[1][1].mul_add(m[1][1], m[1][0] * m[1][0]));
    let r12 = m[2][2].mul_add(m[1][2], m[1][1].mul_add(m[2][1], m[2][0] * m[1][0]));
    let r20 = m[2][0].mul_add(m[0][0], m[0][1].mul_add(m[2][1], m[0][2] * m[2][2]));
    let r21 = m[1][2].mul_add(m[2][2], m[1][1].mul_add(m[2][1], m[2][0] * m[1][0]));
    let r22 = m[2][2].mul_add(m[2][2], m[2][1].mul_add(m[2][1], m[2][0] * m[2][0]));
    let d00 = r00 - 1.0;
    let d11 = r11 - 1.0;
    let d22 = r22 - 1.0;
    let row0 = d00.mul_add(d00, r02.mul_add(r02, r01 * r01));
    let row1 = r10.mul_add(r10, r12.mul_add(r12, d11 * d11));
    let row2 = r20.mul_add(r20, d22.mul_add(d22, r21 * r21));
    row0.mul_add(row0, row2.mul_add(row2, row1 * row1))
}

/// Exact body rows copied by the network Contact producer after its geometry.
///
/// The value is inert. Its typed RF token is not a body handle, row index, or
/// mutation capability.
#[derive(Clone, Copy)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
struct SimulationContactCommonBodyInput {
    center_of_mass: [f32; 3],
    solver_delta: SolverDeltaRowToken,
    velocity: [f32; 3],
    omega: [f32; 3],
    inertia_full: [f32; 3],
    inverse_mass: f32,
    inertia_split: [f32; 3],
    state_word: u32,
    force: [f32; 3],
    kinematic_value: f32,
    torque: [f32; 3],
    cooldown_word: u32,
}

/// Selected-body values used to transform the queued Part-local point.
#[derive(Clone, Copy)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
struct SimulationNetworkContactSelectedBodyInput {
    common: SimulationContactCommonBodyInput,
    right: [f32; 3],
    up: [f32; 3],
    at: [f32; 3],
}

/// One fully resolved, inert input to the network Contact final-value graph.
#[derive(Clone, Copy)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
struct SimulationNetworkContactDrainInput {
    static_body: SimulationContactCommonBodyInput,
    selected_body: SimulationNetworkContactSelectedBodyInput,
    point: [f32; 4],
    normal: [f32; 4],
    displacement: f32,
    static_body_token: ContactBodyToken,
    selected_body_token: ContactBodyToken,
    static_friction: f32,
    dynamic_friction: f32,
    restitution: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
enum SimulationNetworkContactDrainEstimateSite {
    TangentReciprocalSqrt,
}

/// Numeric seam for the drain compositor's one unrefined Xenon estimate.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
trait SimulationNetworkContactDrainNumericBackend {
    type Error;

    fn reciprocal_sqrt_estimate(
        &mut self,
        site: SimulationNetworkContactDrainEstimateSite,
        operand: ContactVector,
    ) -> Result<ContactVector, Self::Error>;
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
fn contact_producer_madd(a: [f32; 3], addend: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    [
        a[0].mul_add(c[0], addend[0]),
        a[1].mul_add(c[1], addend[1]),
        a[2].mul_add(c[2], addend[2]),
    ]
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
fn contact_producer_nmsub(a: [f32; 3], addend: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    [
        (-a[0]).mul_add(c[0], addend[0]),
        (-a[1]).mul_add(c[1], addend[1]),
        (-a[2]).mul_add(c[2], addend[2]),
    ]
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
const fn contact_producer_yzx(value: [f32; 3]) -> [f32; 3] {
    [value[1], value[2], value[0]]
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
const fn contact_producer_zxy(value: [f32; 3]) -> [f32; 3] {
    [value[2], value[0], value[1]]
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
fn contact_producer_point_velocity(
    radius: [f32; 3],
    velocity: [f32; 3],
    omega: [f32; 3],
) -> [f32; 3] {
    contact_producer_madd(
        contact_producer_zxy(radius),
        contact_producer_nmsub(
            contact_producer_yzx(radius),
            velocity,
            contact_producer_zxy(omega),
        ),
        contact_producer_yzx(omega),
    )
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
fn contact_producer_cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    contact_producer_madd(
        contact_producer_yzx(a),
        contact_producer_nmsub(contact_producer_zxy(a), [0.0; 3], contact_producer_yzx(b)),
        contact_producer_zxy(b),
    )
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
fn contact_producer_binormal(normal: [f32; 3], tangent: [f32; 3]) -> [f32; 3] {
    let positive_product = contact_producer_madd(
        contact_producer_zxy(tangent),
        [0.0; 3],
        contact_producer_yzx(normal),
    );
    contact_producer_nmsub(
        contact_producer_yzx(tangent),
        positive_product,
        contact_producer_zxy(normal),
    )
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
const fn contact_producer_word_vector(
    xyz: [f32; 3],
    word: u32,
) -> SimulationContactBuildWordVector {
    SimulationContactBuildWordVector { xyz, word }
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
fn contact_producer_sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
fn contact_producer_scale(value: [f32; 3], scale: [f32; 4]) -> [f32; 3] {
    [
        value[0].mul_add(scale[0], 0.0),
        value[1].mul_add(scale[1], 0.0),
        value[2].mul_add(scale[2], 0.0),
    ]
}

/// Shared source-shaped tangent selection used by both proven Contact
/// producers. The squared magnitude retains the native X-then-Y-then-Z fused
/// accumulation, and equality with `f32::MIN_POSITIVE` selects the dynamic
/// tangent.
fn contact_producer_tangent_choice(
    normal: [f32; 3],
    relative_velocity: [f32; 3],
) -> ([f32; 3], ContactVector) {
    let nx = normal[0];
    let nz = normal[2];
    let choose_x = (-nx).mul_add(nx, 0.5) >= -0.0;
    let tangent_x = contact_producer_nmsub(normal, [1.0, 0.0, 0.0], [nx; 3]);
    let z_base = contact_producer_nmsub(normal, [0.0, 0.0, 1.0], [nz; 3]);
    let tangent_z = [
        f32::from_bits(z_base[0].to_bits() ^ 0x8000_0000),
        f32::from_bits(z_base[1].to_bits() ^ 0x8000_0000),
        f32::from_bits(z_base[2].to_bits() ^ 0x8000_0000),
    ];
    let (fallback_tangent, fallback_magnitude) = if choose_x {
        (tangent_x, tangent_x[0])
    } else {
        (tangent_z, z_base[2])
    };

    let tangent_raw = contact_producer_cross(relative_velocity, normal);
    let magnitude_x = tangent_raw[0].mul_add(tangent_raw[0], 0.0);
    let magnitude_y = tangent_raw[1].mul_add(tangent_raw[1], magnitude_x);
    let magnitude = tangent_raw[2].mul_add(tangent_raw[2], magnitude_y);
    let (tangent, estimate_operand) = if magnitude >= f32::MIN_POSITIVE {
        (tangent_raw, magnitude)
    } else {
        (fallback_tangent, fallback_magnitude)
    };
    (tangent, [estimate_operand; 4])
}

fn contact_producer_scaled_tangent_and_binormal(
    normal: [f32; 3],
    tangent: [f32; 3],
    reciprocal_sqrt: ContactVector,
) -> ([f32; 3], [f32; 3]) {
    (
        contact_producer_scale(tangent, reciprocal_sqrt),
        contact_producer_scale(contact_producer_binormal(normal, tangent), reciprocal_sqrt),
    )
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
fn contact_producer_common_rows(
    body: SimulationContactCommonBodyInput,
) -> [SimulationContactBuildWordVector; 5] {
    [
        contact_producer_word_vector(body.center_of_mass, body.solver_delta.0),
        contact_producer_word_vector(body.inertia_full, body.inverse_mass.to_bits()),
        contact_producer_word_vector(body.inertia_split, body.state_word),
        contact_producer_word_vector(body.force, body.kinematic_value.to_bits()),
        contact_producer_word_vector(body.torque, body.cooldown_word),
    ]
}

/// Composes the successful ordinary-finite network Contact final value.
///
/// This is the dependency-equivalent result of `0x827b3c04..0x827b3ec4`.
/// It does not model Part lifetime, queue/count/publication effects, native
/// store chronology, callback failure chronology, or exceptional VMX/FPSCR
/// behavior. Point/normal W words are retained by the queue but proven dead at
/// this final-value boundary. The backend supplies the one unrefined
/// `vrsqrtefp` result. Backend failure and its no-value return are safe-host
/// policy; native `vrsqrtefp` has no analogous failure path.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private one-record network Contact compositor")
)]
fn build_network_contact_input<B: SimulationNetworkContactDrainNumericBackend>(
    input: &SimulationNetworkContactDrainInput,
    backend: &mut B,
) -> Result<SimulationContactBuildInput, B::Error> {
    let point = [input.point[0], input.point[1], input.point[2]];
    let normal = [input.normal[0], input.normal[1], input.normal[2]];
    let selected = input.selected_body;
    let transform_x = contact_producer_madd(
        selected.right,
        selected.common.center_of_mass,
        [point[0]; 3],
    );
    let transform_y = contact_producer_madd(selected.up, transform_x, [point[1]; 3]);
    let position_a = contact_producer_madd(selected.at, transform_y, [point[2]; 3]);
    let position_b = contact_producer_madd(normal, position_a, [input.displacement; 3]);

    let radius_a = contact_producer_sub(position_a, input.static_body.center_of_mass);
    let radius_b = contact_producer_sub(position_b, selected.common.center_of_mass);
    let velocity_a = contact_producer_point_velocity(
        radius_a,
        input.static_body.velocity,
        input.static_body.omega,
    );
    let velocity_b =
        contact_producer_point_velocity(radius_b, selected.common.velocity, selected.common.omega);
    let relative_velocity = contact_producer_sub(velocity_b, velocity_a);

    let (tangent, estimate_operand) = contact_producer_tangent_choice(normal, relative_velocity);
    let reciprocal_sqrt = backend.reciprocal_sqrt_estimate(
        SimulationNetworkContactDrainEstimateSite::TangentReciprocalSqrt,
        estimate_operand,
    )?;
    let (tangent_scaled, binormal_scaled) =
        contact_producer_scaled_tangent_and_binormal(normal, tangent, reciprocal_sqrt);

    let restitution = if input.restitution <= 1.0 {
        input.restitution
    } else {
        1.0
    };
    let static_friction = if input.static_friction >= 0.0 {
        input.static_friction
    } else {
        0.0
    };
    let dynamic_friction = if input.dynamic_friction >= 0.0 {
        input.dynamic_friction
    } else {
        0.0
    };
    let static_rows = contact_producer_common_rows(input.static_body);
    let selected_rows = contact_producer_common_rows(selected.common);

    Ok(SimulationContactBuildInput {
        position_a_body_a: contact_producer_word_vector(position_a, input.static_body_token.0),
        position_b_body_b: contact_producer_word_vector(position_b, input.selected_body_token.0),
        contact_right_restitution: contact_producer_word_vector(normal, restitution.to_bits()),
        contact_up_static_friction: contact_producer_word_vector(
            tangent_scaled,
            static_friction.to_bits(),
        ),
        contact_at_dynamic_friction: contact_producer_word_vector(
            binormal_scaled,
            dynamic_friction.to_bits(),
        ),
        contact_velocity_tag: contact_producer_word_vector(relative_velocity, 0),
        side_0_com_rf_token: static_rows[0],
        side_1_com_rf_token: selected_rows[0],
        side_0_inertia_full_inverse_mass: static_rows[1],
        side_1_inertia_full_inverse_mass: selected_rows[1],
        side_0_inertia_split_state: static_rows[2],
        side_1_inertia_split_state: selected_rows[2],
        side_0_force_kine: static_rows[3],
        side_1_force_kine: selected_rows[3],
        side_0_torque_cool: static_rows[4],
        side_1_torque_cool: selected_rows[4],
    })
}

/// Exact three-scalar material input used by the Assembly/ground producer.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private Assembly/ground Contact compositor")
)]
struct SimulationAssemblyGroundMaterialInput {
    static_friction: f32,
    dynamic_friction: f32,
    restitution: f32,
}

/// One fully resolved world-point input to the Assembly/ground Contact tail.
///
/// Body A is the tested Assembly volume and body B is ground. Table bindings
/// in S0/S1 remain independent from the RF-row bindings carried by the body
/// records in S6/S7.
#[derive(Clone, Copy)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private Assembly/ground Contact compositor")
)]
struct SimulationAssemblyGroundContactInput {
    body_a: SimulationContactCommonBodyInput,
    body_b: SimulationContactCommonBodyInput,
    position_a: [f32; 3],
    position_b: [f32; 3],
    normal: [f32; 3],
    table_body_a: ContactBodyToken,
    table_body_b: ContactBodyToken,
    volume_material: SimulationAssemblyGroundMaterialInput,
    ground_material: SimulationAssemblyGroundMaterialInput,
    volume_surface_id: u32,
    triangle_user_tag: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private Assembly/ground Contact compositor")
)]
enum SimulationAssemblyGroundContactEstimateSite {
    TangentReciprocalSqrt,
}

/// Numeric seam for Assembly/ground's sole unrefined Xenon estimate.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private Assembly/ground Contact compositor")
)]
trait SimulationAssemblyGroundContactNumericBackend {
    type Error;

    fn reciprocal_sqrt_estimate(
        &mut self,
        site: SimulationAssemblyGroundContactEstimateSite,
        operand: ContactVector,
    ) -> Result<ContactVector, Self::Error>;
}

fn select_assembly_ground_material(
    volume: SimulationAssemblyGroundMaterialInput,
    ground: SimulationAssemblyGroundMaterialInput,
) -> SimulationAssemblyGroundMaterialInput {
    // `fcmpu` followed by bge/ble selects ground for equality and unordered
    // comparisons. Keep all three source comparisons independent.
    SimulationAssemblyGroundMaterialInput {
        restitution: if volume.restitution < ground.restitution {
            volume.restitution
        } else {
            ground.restitution
        },
        dynamic_friction: if volume.dynamic_friction > ground.dynamic_friction {
            volume.dynamic_friction
        } else {
            ground.dynamic_friction
        },
        static_friction: if volume.static_friction > ground.static_friction {
            volume.static_friction
        } else {
            ground.static_friction
        },
    }
}

/// Composes the successful final value at `0x827cd500..0x827cd778`.
///
/// Reservation, zeroing, final PPU-word overwrite chronology, live body
/// rereads, and append effects belong to the surrounding Run owner. This pure
/// value boundary preserves the final observable rows and exact ordinary-
/// finite fused grouping. A backend failure is test/host policy; production
/// must use an infallible estimate after native-order reservation.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private Assembly/ground Contact compositor")
)]
fn build_assembly_ground_contact_input<B: SimulationAssemblyGroundContactNumericBackend>(
    input: &SimulationAssemblyGroundContactInput,
    backend: &mut B,
) -> Result<SimulationContactBuildInput, B::Error> {
    let material = select_assembly_ground_material(input.volume_material, input.ground_material);
    build_assembly_ground_contact_input_with_material(input, material, backend)
}

/// Shared final rows after native Run has already selected material, before
/// its static-state gate, and subsequently reserved the Contact destination.
fn build_assembly_ground_contact_input_with_material<
    B: SimulationAssemblyGroundContactNumericBackend,
>(
    input: &SimulationAssemblyGroundContactInput,
    material: SimulationAssemblyGroundMaterialInput,
    backend: &mut B,
) -> Result<SimulationContactBuildInput, B::Error> {
    let radius_a = contact_producer_sub(input.position_a, input.body_a.center_of_mass);
    let radius_b = contact_producer_sub(input.position_b, input.body_b.center_of_mass);
    let velocity_a =
        contact_producer_point_velocity(radius_a, input.body_a.velocity, input.body_a.omega);
    let velocity_b =
        contact_producer_point_velocity(radius_b, input.body_b.velocity, input.body_b.omega);
    let relative_velocity = contact_producer_sub(velocity_b, velocity_a);

    let (tangent, estimate_operand) =
        contact_producer_tangent_choice(input.normal, relative_velocity);
    let reciprocal_sqrt = backend.reciprocal_sqrt_estimate(
        SimulationAssemblyGroundContactEstimateSite::TangentReciprocalSqrt,
        estimate_operand,
    )?;
    let (tangent, binormal) =
        contact_producer_scaled_tangent_and_binormal(input.normal, tangent, reciprocal_sqrt);

    let tag =
        ((input.volume_surface_id & 0xffff) << 16) | ((input.triangle_user_tag >> 16) & 0xffff);
    let rows_a = contact_producer_common_rows(input.body_a);
    let rows_b = contact_producer_common_rows(input.body_b);

    Ok(SimulationContactBuildInput {
        position_a_body_a: contact_producer_word_vector(input.position_a, input.table_body_a.0),
        position_b_body_b: contact_producer_word_vector(input.position_b, input.table_body_b.0),
        contact_right_restitution: contact_producer_word_vector(
            input.normal,
            material.restitution.to_bits(),
        ),
        contact_up_static_friction: contact_producer_word_vector(
            tangent,
            material.static_friction.to_bits(),
        ),
        contact_at_dynamic_friction: contact_producer_word_vector(
            binormal,
            material.dynamic_friction.to_bits(),
        ),
        contact_velocity_tag: contact_producer_word_vector(relative_velocity, tag),
        side_0_com_rf_token: rows_a[0],
        side_1_com_rf_token: rows_b[0],
        side_0_inertia_full_inverse_mass: rows_a[1],
        side_1_inertia_full_inverse_mass: rows_b[1],
        side_0_inertia_split_state: rows_a[2],
        side_1_inertia_split_state: rows_b[2],
        side_0_force_kine: rows_a[3],
        side_1_force_kine: rows_b[3],
        side_0_torque_cool: rows_a[4],
        side_1_torque_cool: rows_b[4],
    })
}

#[path = "simulation_contact_spy.rs"]
pub(crate) mod contact_spy;

#[path = "../../collision/simulation_ground_contact.rs"]
pub(crate) mod ground_preparation;

/// Shared sealed Contact ownership and solver preparation. The historic module
/// and exported type names retain network API compatibility; native ground
/// Contacts carry their own provenance and opposite CPU body-side assignment.
pub(crate) mod network_preparation {
    #![cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "private unpublished network Contact contribution foundation"
        )
    )]

    use super::{
        ContactBodyToken, SimulationContactBuildInput, SimulationContactCommonBodyInput,
        SimulationContactJacobian, SimulationContactNumericBackend,
        SimulationNetworkContactDrainInput, SimulationNetworkContactDrainNumericBackend,
        SimulationNetworkContactSelectedBodyInput, SolverDeltaRowToken,
        build_contact_jacobian_host, build_network_contact_input, solve_contact_horus_record_host,
    };
    use crate::{
        simulation_contact_network_queue::{
            SimulationNetworkContactQueue, SimulationNetworkContactQueueAuthority,
        },
        simulation_island_map::{SIMULATION_ISLAND_CAPACITY, SimulationRigidBodyIslandMapSnapshot},
        simulation_rigid_body_pool::{
            SimulationRigidBodyContactSolverDeltaStores, SimulationRigidBodyDynamicUpdatePhase,
            SimulationRigidBodyDynamicUpdatePhaseIdentity, SimulationRigidBodyFrameIdentity,
            SimulationRigidBodyHandle, SimulationRigidBodyIndexedSolverDeltaError,
            SimulationRigidBodyIndexedSolverDeltaPlan, SimulationRigidBodyNetworkContactSnapshot,
            SimulationRigidBodyObservation, SimulationRigidBodyPool,
            SimulationRigidBodyPoolAuthority, SimulationRigidBodyPoolError,
            SimulationRigidBodySolverDeltaPairRows,
        },
    };
    use std::{
        num::NonZeroU64,
        ops::Range,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_MANAGER_CONTACT_AUTHORITY: AtomicU64 = AtomicU64::new(1);

    const IDENTITY_AFFINE: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 0.0,
    ];

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) struct SimulationManagerContactAuthority(NonZeroU64);

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum SimulationNetworkContactPreparationBodyRole {
        Static,
        Selected,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) enum SimulationNetworkContactPreparedBindingKind {
        TableBodyA,
        TableBodyB,
        SolverSide0,
        SolverSide1,
    }

    #[derive(Debug, Eq, PartialEq)]
    enum SimulationNetworkContactPreparationError<E> {
        DestinationManagerMismatch,
        DestinationBodyPoolMismatch,
        DestinationFrameMismatch,
        DestinationPhaseMismatch,
        DestinationStaticBodyMismatch,
        DestinationCapacityExceeded {
            current: usize,
            incoming: usize,
            maximum: u32,
        },
        QueueBodyPoolMismatch,
        QueueFrameMismatch,
        EmptyQueue,
        StagingAllocationFailed {
            count: usize,
        },
        RecordAllocationFailed {
            count: usize,
        },
        Body {
            queue_ordinal: usize,
            role: SimulationNetworkContactPreparationBodyRole,
            source: SimulationRigidBodyPoolError,
        },
        BindingTokenOverflow {
            queue_ordinal: usize,
            kind: SimulationNetworkContactPreparedBindingKind,
            slot: u32,
        },
        Numeric {
            queue_ordinal: usize,
            source: E,
        },
    }

    /// Typed failure that returns the exact non-cloneable queue owner.
    ///
    /// Backend-owned trace effects are not rolled back. No record, table,
    /// count, RF row, or queue mutation is exposed by a returned failure. This
    /// guarantee covers returned typed errors only: a reservation or backend
    /// panic unwinds and drops the owned queue, and a backend trace prefix may
    /// remain. Panic recovery is deliberately unclaimed.
    struct SimulationNetworkContactPreparationFailure<E> {
        queue: SimulationNetworkContactQueue,
        error: SimulationNetworkContactPreparationError<E>,
    }

    #[derive(Clone, Copy)]
    struct SimulationPreparedNetworkContactSource {
        manager: SimulationManagerContactAuthority,
        queue: SimulationNetworkContactQueueAuthority,
        body_pool: SimulationRigidBodyPoolAuthority,
        frame: SimulationRigidBodyFrameIdentity,
        phase: SimulationRigidBodyDynamicUpdatePhaseIdentity,
        static_body: SimulationRigidBodyObservation,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct SimulationPreparedNetworkContactRoles {
        table_body_a: SimulationRigidBodyObservation,
        table_body_b: SimulationRigidBodyObservation,
        solver_side_0: SimulationRigidBodyObservation,
        solver_side_1: SimulationRigidBodyObservation,
    }

    struct SimulationStagedNetworkContact {
        queue_ordinal: usize,
        input: SimulationNetworkContactDrainInput,
        roles: SimulationPreparedNetworkContactRoles,
    }

    struct SimulationPreparedNetworkContactEntry {
        queue_ordinal: usize,
        selected_body: SimulationRigidBodyObservation,
        input: SimulationContactBuildInput,
        roles: SimulationPreparedNetworkContactRoles,
    }

    #[derive(Debug, Eq, PartialEq)]
    pub(crate) enum SimulationManagerContactDomainInitError {
        NativeContactByteSizeOverflow { maximum: u32 },
        AllocationFailed { maximum: u32 },
        AuthorityExhausted,
        BodyPool(SimulationRigidBodyPoolError),
        BodyPoolNotPristine,
        StaticBodyUnavailable,
        IslandState(crate::simulation_island_map::persistent::SimulationRigidBodyIslandStateError),
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum SimulationContactProducerSource {
        Network {
            queue: SimulationNetworkContactQueueAuthority,
            queue_ordinal: usize,
            selected_body: SimulationRigidBodyObservation,
        },
        AssemblyGround {
            source: super::ground_preparation::SimulationGroundContactSource,
            volume_body: SimulationRigidBodyObservation,
        },
        /// `SkaterDMOCollisionJob` 0x827cf140: A is the skater volume's body
        /// and B the DMO volume's own body (`RunIntersectionTests2`'s
        /// `GroundData`), never the manager static body.
        AssemblyBody {
            source: super::ground_preparation::SimulationGroundContactSource,
            volume_body: SimulationRigidBodyObservation,
            other_body: SimulationRigidBodyObservation,
        },
    }

    impl SimulationContactProducerSource {
        const fn expected_roles(
            self,
            static_body: SimulationRigidBodyObservation,
        ) -> SimulationPreparedNetworkContactRoles {
            let (body_a, body_b) = match self {
                Self::Network { selected_body, .. } => (static_body, selected_body),
                // CPU AssemblyGroundCollisionJob independently obtains the
                // table/live A occurrences from the volume body and B from
                // the manager static body (0x827cdb98..0x827cdf48).
                Self::AssemblyGround { volume_body, .. } => (volume_body, static_body),
                Self::AssemblyBody {
                    volume_body,
                    other_body,
                    ..
                } => (volume_body, other_body),
            };
            SimulationPreparedNetworkContactRoles {
                table_body_a: body_a,
                table_body_b: body_b,
                solver_side_0: body_a,
                solver_side_1: body_b,
            }
        }
    }

    struct SimulationOwnedContactEntry {
        dense_ordinal: u32,
        producer: SimulationContactProducerSource,
        input: SimulationContactBuildInput,
        roles: SimulationPreparedNetworkContactRoles,
    }

    /// Private manager-owned Contact backing and its retained static body.
    ///
    /// The vector is semantic persistent storage, not a native-layout or
    /// atomic-publication claim. `maximum` is the logical global Contact append
    /// bound; allocator over-capacity is irrelevant. The bound is independent
    /// of the 432-entry network producer queue.
    pub(crate) struct SimulationManagerContactDomain {
        authority: SimulationManagerContactAuthority,
        body_pool: SimulationRigidBodyPoolAuthority,
        static_body: SimulationRigidBodyHandle,
        maximum: u32,
        records: Vec<SimulationOwnedContactEntry>,
        last_started_frame: Option<SimulationRigidBodyFrameIdentity>,
        island_state: crate::simulation_island_map::persistent::SimulationRigidBodyIslandState,
        // Native manager +0x50, initialized at 0x82e0c324. Shared by all
        // assembly pools; dedicated static-body admission does not consume it.
        next_object_id: u32,
    }

    #[derive(Clone, Copy)]
    struct SimulationContactFrameSource {
        manager: SimulationManagerContactAuthority,
        body_pool: SimulationRigidBodyPoolAuthority,
        frame: SimulationRigidBodyFrameIdentity,
        phase: SimulationRigidBodyDynamicUpdatePhaseIdentity,
        static_body: SimulationRigidBodyObservation,
    }

    #[cfg(test)]
    mod assembly_id_tests {
        use super::*;

        #[test]
        fn manager_world_ids_start_zero_and_wrap_without_static_body_consumption() {
            let mut bodies = SimulationRigidBodyPool::try_new(1).unwrap();
            let mut manager = SimulationManagerContactDomain::try_new(&mut bodies, 0).unwrap();
            assert_eq!(bodies.counts().static_bodies, 1);
            assert_eq!(manager.next_object_id, 0);
            assert_eq!(
                manager
                    .try_allocate_assembly_world_index(bodies.authority())
                    .unwrap(),
                0
            );
            assert_eq!(
                manager
                    .try_allocate_assembly_world_index(bodies.authority())
                    .unwrap(),
                1
            );
            // Synthetic prior insertion history, never a production preseed.
            manager.next_object_id = u32::MAX;
            assert_eq!(
                manager
                    .try_allocate_assembly_world_index(bodies.authority())
                    .unwrap(),
                u32::MAX
            );
            assert_eq!(
                manager
                    .try_allocate_assembly_world_index(bodies.authority())
                    .unwrap(),
                0
            );
            assert_eq!(manager.next_object_id, 1);
        }

        #[test]
        fn foreign_body_domain_cannot_consume_manager_world_id() {
            let mut bodies = SimulationRigidBodyPool::try_new(1).unwrap();
            let mut manager = SimulationManagerContactDomain::try_new(&mut bodies, 0).unwrap();
            let foreign = SimulationRigidBodyPool::try_new(0).unwrap();
            assert!(matches!(manager.try_allocate_assembly_world_index(foreign.authority()),
                Err(crate::simulation_island_map::persistent::SimulationRigidBodyIslandStateError::ForeignPool)));
            assert_eq!(manager.next_object_id, 0);
            assert_eq!(
                manager
                    .try_allocate_assembly_world_index(bodies.authority())
                    .unwrap(),
                0
            );
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    pub(crate) enum SimulationContactFrameBeginError {
        BodyPoolMismatch,
        FrameAlreadyStarted,
        StaticBody(SimulationRigidBodyPoolError),
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) enum SimulationContactFrameSealError {
        Manager,
        BodyPool,
        Frame,
        Phase,
        StaticBody,
    }

    pub(super) struct SimulationContactFrameSealFailure<'manager> {
        pub(super) frame: SimulationOpenContactFrame<'manager>,
        pub(super) error: SimulationContactFrameSealError,
    }

    #[derive(Debug, Eq, PartialEq)]
    pub(crate) enum SimulationContactAppendError {
        DestinationManagerMismatch,
        DestinationBodyPoolMismatch,
        DestinationFrameMismatch,
        DestinationPhaseMismatch,
        DestinationStaticBodyMismatch,
        ContributionManagerMismatch,
        ContributionBodyPoolMismatch,
        ContributionFrameMismatch,
        ContributionPhaseMismatch,
        ContributionStaticBodyMismatch,
        CapacityExceeded {
            current: usize,
            incoming: usize,
            maximum: u32,
        },
        QueueOrdinalMismatch {
            contribution_ordinal: usize,
            actual: usize,
        },
        RoleMismatch {
            contribution_ordinal: usize,
            kind: SimulationNetworkContactPreparedBindingKind,
        },
        BindingTokenMismatch {
            contribution_ordinal: usize,
            kind: SimulationNetworkContactPreparedBindingKind,
            expected: u32,
            actual: u32,
        },
        BindingTokenOverflow {
            contribution_ordinal: usize,
            kind: SimulationNetworkContactPreparedBindingKind,
            slot: u32,
        },
        Body {
            contribution_ordinal: usize,
            kind: SimulationNetworkContactPreparedBindingKind,
            source: SimulationRigidBodyPoolError,
        },
    }

    struct SimulationContactAppendFailure {
        contribution: SimulationPreparedNetworkContactContribution,
        error: SimulationContactAppendError,
    }

    /// Append-capable, phase-bound view of the manager's persistent backing.
    #[must_use = "dropping an open Contact frame abandons unpublished records"]
    pub(super) struct SimulationOpenContactFrame<'manager> {
        domain: &'manager mut SimulationManagerContactDomain,
        source: SimulationContactFrameSource,
    }

    /// Immutable unpublished dense Contact values after producer completion.
    #[must_use = "dropping a sealed Contact frame abandons unpublished records"]
    pub(super) struct SimulationSealedContactFrame<'manager> {
        domain: &'manager SimulationManagerContactDomain,
        source: SimulationContactFrameSource,
    }

    #[derive(Clone, Copy)]
    pub(super) enum SimulationContactIslandMapSource<'map> {
        Null {
            frame: SimulationRigidBodyFrameIdentity,
        },
        Present(&'map SimulationRigidBodyIslandMapSnapshot),
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) enum SimulationContactTableEndpoint {
        BodyA,
        BodyB,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum SimulationContactIslandPreparationReadKind {
        DenseOrdinal,
        Role,
        BindingToken,
        IslandLabelA,
        IslandLabelB,
        Compile,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) struct SimulationContactIdentity {
        manager: SimulationManagerContactAuthority,
        frame: SimulationRigidBodyFrameIdentity,
        dense_ordinal: u32,
    }

    #[derive(Debug, Eq, PartialEq)]
    pub(crate) enum SimulationNetworkContactIslandPreparationError<E> {
        SourceManagerMismatch,
        SourceBodyPoolMismatch,
        SourceFrameMismatch,
        SourcePhaseMismatch,
        SourceStaticBodyMismatch,
        SourceStaticBody(SimulationRigidBodyPoolError),
        IslandMapBodyPoolMismatch,
        IslandMapFrameMismatch,
        DenseOrdinalMismatch {
            dense_index: usize,
            actual: u32,
        },
        DenseOrdinalOverflow {
            dense_index: usize,
        },
        RoleMismatch {
            dense_ordinal: u32,
            kind: SimulationNetworkContactPreparedBindingKind,
        },
        Body {
            dense_ordinal: u32,
            kind: SimulationNetworkContactPreparedBindingKind,
            source: SimulationRigidBodyPoolError,
        },
        BindingTokenOverflow {
            dense_ordinal: u32,
            kind: SimulationNetworkContactPreparedBindingKind,
            slot: u32,
        },
        BindingTokenMismatch {
            dense_ordinal: u32,
            kind: SimulationNetworkContactPreparedBindingKind,
            expected: u32,
            actual: u32,
        },
        MissingIslandLabel {
            dense_ordinal: u32,
            endpoint: SimulationContactTableEndpoint,
            body: SimulationRigidBodyObservation,
        },
        IslandLabelOccurrenceMismatch {
            dense_ordinal: u32,
            endpoint: SimulationContactTableEndpoint,
            expected_body: SimulationRigidBodyObservation,
            mapped_body: SimulationRigidBodyObservation,
        },
        InvalidSelectedIsland {
            dense_ordinal: u32,
            raw_body_a: i8,
            raw_body_b: i8,
            selected: i8,
        },
        IslandCountOverflow {
            island: u8,
            count: usize,
        },
        RecordAllocationFailed {
            count: usize,
        },
        IslandAllocationFailed {
            island: u8,
            count: usize,
        },
        Numeric {
            contact: SimulationContactIdentity,
            source: E,
        },
    }

    pub(super) struct SimulationNetworkContactIslandPreparationFailure<'manager, E> {
        pub(super) frame: SimulationSealedContactFrame<'manager>,
        pub(super) error: SimulationNetworkContactIslandPreparationError<E>,
    }

    pub(crate) struct SimulationPreparedNetworkContactJacobian {
        contact: SimulationContactIdentity,
        table_body_a: SimulationRigidBodyObservation,
        table_body_b: SimulationRigidBodyObservation,
        solver_side_0: SimulationRigidBodyObservation,
        solver_side_1: SimulationRigidBodyObservation,
        record: SimulationContactJacobian,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct SimulationPreparedNetworkContactIslandEntry {
        contact: SimulationContactIdentity,
        table_body_a: SimulationRigidBodyObservation,
        table_body_b: SimulationRigidBodyObservation,
    }

    /// Host-only compiled-batch issuance, not a native pointer or numeric token.
    /// Its retained identity binds a completed solver receipt to this exact
    /// non-cloneable batch, including the zero-iteration path.
    #[derive(Clone)]
    pub(crate) struct SimulationContactBatchIdentity(std::sync::Arc<()>);

    impl SimulationContactBatchIdentity {
        pub(crate) fn same_as(&self, other: &Self) -> bool {
            std::sync::Arc::ptr_eq(&self.0, &other.0)
        }
    }

    /// Unpublished Contact Jacobians and ten stable island subsequences.
    ///
    /// This combines native zero-separation table formation and later in-place
    /// `ContactBatchBuild` into one checked host publication. It owns separate
    /// Jacobian values and keeps the sealed manager input backing immutable.
    /// Both network and CPU Assembly/ground producers retain their own source
    /// identity and independent table/RF bindings. The historic type name is
    /// retained for existing network callers; it does not classify its content.
    #[must_use = "dropping prepared Contacts abandons them without RF effects"]
    pub(crate) struct SimulationPreparedNetworkContactIslandBatch<'manager> {
        identity: SimulationContactBatchIdentity,
        frame: SimulationSealedContactFrame<'manager>,
        records: Vec<SimulationPreparedNetworkContactJacobian>,
        islands: [Vec<SimulationPreparedNetworkContactIslandEntry>; SIMULATION_ISLAND_CAPACITY],
    }

    #[cfg(test)]
    pub(crate) struct PreparedContactMaterialObservation {
        pub(crate) bodies: [SimulationRigidBodyObservation; 2],
        pub(crate) friction_bits: [u32; 2],
        pub(crate) spy_word: u32,
        pub(crate) normal_impulse_bits: u32,
    }

    impl SimulationPreparedNetworkContactIslandBatch<'_> {
        #[cfg(test)]
        pub(crate) fn material_observations_for_test(
            &self,
        ) -> Vec<PreparedContactMaterialObservation> {
            self.records
                .iter()
                .map(|prepared| PreparedContactMaterialObservation {
                    bodies: [prepared.table_body_a, prepared.table_body_b],
                    spy_word: prepared.record.nb_word,
                    normal_impulse_bits: prepared.record.position_impulse[0].to_bits(),
                    friction_bits: [
                        prepared.record.static_friction.to_bits(),
                        prepared.record.dynamic_friction.to_bits(),
                    ],
                })
                .collect()
        }
        pub(crate) fn identity(&self) -> SimulationContactBatchIdentity {
            self.identity.clone()
        }

        pub(crate) fn record_count(&self) -> usize {
            self.records.len()
        }

        /// One-shot semantic XYZ `SpyContactJacobians` (0x82ae4a88), from the
        /// actual completed solver batch. Native destroys the Jacobian backing
        /// while compacting; consuming this owner prevents replay as Jacobians.
        /// Current spy flags are explicit caller sampling, not a cached default.
        /// Host allocation/identity errors do not undo completed body integration.
        /// A later read refusal drops prior derived output instead of exposing
        /// native partial in-place compaction; no successful frame is published.
        #[expect(
            clippy::needless_pass_by_value,
            reason = "consume the unique completed receipt to prevent export replay"
        )]
        pub(crate) fn try_export_contact_spies(
            self,
            completed: crate::simulation_horus_pipeline::SimulationCompletedHorusFrame<'_>,
            spy_flags: u32,
        ) -> Result<
            super::contact_spy::SimulationContactSpyFrame,
            super::contact_spy::SimulationContactSpyError,
        > {
            use super::contact_spy::{
                SimulationContactSpy, SimulationContactSpyError as Error, SimulationContactSpyFrame,
            };
            let source = self.frame.source;
            let pool = completed.body_pool();
            if source.manager != self.frame.domain.authority {
                return Err(Error::ManagerMismatch);
            }
            if source.body_pool != pool.authority() {
                return Err(Error::BodyPoolMismatch);
            }
            if pool.frame_identity() != Some(source.frame) {
                return Err(Error::FrameMismatch);
            }
            if !completed.matches_source(source.body_pool, source.frame, source.phase) {
                return Err(Error::PhaseMismatch);
            }
            if !completed.matches_contact_batch(&self.identity) {
                return Err(Error::BatchMismatch);
            }
            let mut output = Vec::new();
            // Native PopulateJobTree skips all jobs without Active bodies,
            // and schedules CONTACT_SPIES only when m_SpyFlag & 4 is set.
            if completed.had_active_bodies() && spy_flags & 4 != 0 {
                let frequency = pool.frequency();
                let frequency_squared = frequency * frequency;
                // Separate host storage instead of destructive CJ-stack aliasing.
                // Reservation is a disclosed pre-output host failure policy.
                output
                    .try_reserve_exact(self.records.len())
                    .map_err(|_| Error::Allocation {
                        count: self.records.len(),
                    })?;
                for prepared in self.records {
                    if !SimulationContactSpy::selects(&prepared.record) {
                        continue;
                    }
                    // Source 0x82ae4b40 reads B first, 0x82ae4b60 then A.
                    let center_b = pool
                        .try_contact_spy_center_of_mass(prepared.table_body_b)
                        .map_err(|source| Error::Body {
                            contact: prepared.contact,
                            side: SimulationContactTableEndpoint::BodyB,
                            source,
                        })?;
                    let center_a = pool
                        .try_contact_spy_center_of_mass(prepared.table_body_a)
                        .map_err(|source| Error::Body {
                            contact: prepared.contact,
                            side: SimulationContactTableEndpoint::BodyA,
                            source,
                        })?;
                    output.push(SimulationContactSpy::from_solved(
                        prepared.contact,
                        prepared.table_body_a,
                        prepared.table_body_b,
                        &prepared.record,
                        frequency_squared,
                        center_a,
                        center_b,
                    ));
                }
            }
            Ok(SimulationContactSpyFrame::new(
                source.manager,
                source.body_pool,
                source.frame,
                output,
            ))
        }
    }

    #[derive(Debug)]
    pub(crate) enum SimulationEmptyContactPreparationError {
        Begin(SimulationContactFrameBeginError),
        Seal(SimulationContactFrameSealError),
        Build(SimulationNetworkContactIslandPreparationError<std::convert::Infallible>),
    }

    impl std::fmt::Display for SimulationEmptyContactPreparationError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Begin(error) => write!(f, "empty Contact begin: {error:?}"),
                Self::Seal(error) => write!(f, "empty Contact seal: {error:?}"),
                Self::Build(error) => write!(f, "empty Contact build: {error:?}"),
            }
        }
    }

    #[derive(Debug)]
    pub(crate) enum SimulationGroundContactPreparationError {
        Frame(SimulationEmptyContactPreparationError),
        Append(SimulationContactAppendError),
    }

    impl std::fmt::Display for SimulationGroundContactPreparationError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Frame(error) => write!(f, "ground Contact frame: {error}"),
                Self::Append(error) => write!(f, "ground Contact append: {error:?}"),
            }
        }
    }

    impl SimulationManagerContactDomain {
        /// Serial pipeline prefix: DMO ground/body jobs, then skater ground/body
        /// jobs. Every record follows the same sealed compiler and solver.
        pub(crate) fn try_prepare_assembly_contact_islands_host<'manager>(
            &'manager mut self,
            dmo_ground: &[super::ground_preparation::SimulationGroundContactRecord],
            dmo_bodies: &[super::ground_preparation::SimulationGroundContactRecord],
            ground: &[super::ground_preparation::SimulationGroundContactRecord],
            bodies: &[super::ground_preparation::SimulationGroundContactRecord],
            phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
            map: &SimulationRigidBodyIslandMapSnapshot,
        ) -> Result<
            SimulationPreparedNetworkContactIslandBatch<'manager>,
            SimulationGroundContactPreparationError,
        > {
            use SimulationEmptyContactPreparationError as Frame;
            use SimulationGroundContactPreparationError as Error;
            let mut open = self
                .try_begin_contact_frame(phase)
                .map_err(|e| Error::Frame(Frame::Begin(e)))?;
            open.try_append_ground_contact_records(dmo_ground, phase)
                .map_err(Error::Append)?;
            open.try_append_body_contact_records(dmo_bodies, phase)
                .map_err(Error::Append)?;
            open.try_append_ground_contact_records(ground, phase)
                .map_err(Error::Append)?;
            open.try_append_body_contact_records(bodies, phase)
                .map_err(Error::Append)?;
            let sealed = open
                .try_seal(phase)
                .map_err(|f| Error::Frame(Frame::Seal(f.error)))?;
            try_prepare_network_contact_island_batch_with_backend(
                sealed,
                phase,
                SimulationContactIslandMapSource::Present(map),
                &mut super::HostContactNumericBackend,
            )
            .map_err(|f| Error::Frame(Frame::Build(f.error)))
        }

        /// Imports only sealed, occurrence/phase-qualified ground records into
        /// the existing manager Contact compiler. No alternate solver model.
        pub(crate) fn try_prepare_ground_contact_islands_host<'manager>(
            &'manager mut self,
            records: &[super::ground_preparation::SimulationGroundContactRecord],
            phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
            map: &SimulationRigidBodyIslandMapSnapshot,
        ) -> Result<
            SimulationPreparedNetworkContactIslandBatch<'manager>,
            SimulationGroundContactPreparationError,
        > {
            use SimulationEmptyContactPreparationError as Frame;
            use SimulationGroundContactPreparationError as Error;
            let mut open = self
                .try_begin_contact_frame(phase)
                .map_err(|e| Error::Frame(Frame::Begin(e)))?;
            open.try_append_ground_contact_records(records, phase)
                .map_err(Error::Append)?;
            let sealed = open
                .try_seal(phase)
                .map_err(|f| Error::Frame(Frame::Seal(f.error)))?;
            try_prepare_network_contact_island_batch_with_backend(
                sealed,
                phase,
                SimulationContactIslandMapSource::Present(map),
                &mut super::HostContactNumericBackend,
            )
            .map_err(|f| Error::Frame(Frame::Build(f.error)))
        }

        /// Deliberately contactless branch using real manager backing and
        /// phase sealing. This does NOT assert that world collision found none.
        pub(crate) fn try_prepare_empty_contact_islands_host<'manager>(
            &'manager mut self,
            phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
            map: &SimulationRigidBodyIslandMapSnapshot,
        ) -> Result<
            SimulationPreparedNetworkContactIslandBatch<'manager>,
            SimulationEmptyContactPreparationError,
        > {
            let open = self
                .try_begin_contact_frame(phase)
                .map_err(SimulationEmptyContactPreparationError::Begin)?;
            let sealed = open
                .try_seal(phase)
                .map_err(|failure| SimulationEmptyContactPreparationError::Seal(failure.error))?;
            try_prepare_network_contact_island_batch_with_backend(
                sealed,
                phase,
                SimulationContactIslandMapSource::Present(map),
                &mut super::HostContactNumericBackend,
            )
            .map_err(|failure| SimulationEmptyContactPreparationError::Build(failure.error))
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) enum SimulationPreparedNetworkContactIslandHorusError {
        IslandOutOfRange {
            island: usize,
        },
        SourceManagerMismatch,
        SourceBodyPoolMismatch,
        SourceFrameMismatch,
        SourcePhaseMismatch,
        SourceStaticBodyMismatch,
        SourceStaticBody(SimulationRigidBodyPoolError),
        TableEntryMismatch {
            island_ordinal: usize,
            contact: SimulationContactIdentity,
        },
        SourceRecordMismatch {
            island_ordinal: usize,
            contact: SimulationContactIdentity,
        },
        Body {
            island_ordinal: usize,
            contact: SimulationContactIdentity,
            kind: SimulationNetworkContactPreparedBindingKind,
            source: SimulationRigidBodyPoolError,
        },
        BindingTokenOverflow {
            island_ordinal: usize,
            contact: SimulationContactIdentity,
            kind: SimulationNetworkContactPreparedBindingKind,
            slot: u32,
        },
        BindingTokenMismatch {
            island_ordinal: usize,
            contact: SimulationContactIdentity,
            kind: SimulationNetworkContactPreparedBindingKind,
            expected: u32,
            actual: u32,
        },
        BodySequence(SimulationRigidBodyIndexedSolverDeltaError),
    }

    /// Typed pre-effect refusal while flattening every scheduled Contact island.
    ///
    /// Per-island semantic validation retains the exact island attribution.
    /// The final RF plan is one global allocation/body-sequence boundary, so
    /// failures there deliberately do not invent an island attribution.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) enum SimulationPreparedNetworkContactHorusSerialPlanError {
        IslandCountOutOfRange {
            scheduled_islands: usize,
        },
        Island {
            island: usize,
            source: SimulationPreparedNetworkContactIslandHorusError,
        },
        EntryCountOverflow {
            island: usize,
            accumulated: usize,
            incoming: usize,
        },
        BodySequence(SimulationRigidBodyIndexedSolverDeltaError),
    }

    /// One flattened Contact RF plan plus exact ranges for all ten table slots.
    ///
    /// Only `0..scheduled_islands` ranges are populated. Their ascending,
    /// nonoverlapping layout is deterministic host serialization policy; each
    /// range itself retains the native indirect-table order and repetitions.
    pub(crate) struct SimulationPreparedNetworkContactHorusSerialPlan<'batch> {
        rf: SimulationRigidBodyIndexedSolverDeltaPlan<
            'batch,
            SimulationPreparedNetworkContactJacobian,
        >,
        ranges: [Range<usize>; SIMULATION_ISLAND_CAPACITY],
        scheduled_islands: usize,
    }

    /// One unpublished, phase-bound, non-cloneable network contribution.
    ///
    /// Successful construction consumes the source queue. Dropping this value
    /// abandons its records without publishing a global Contact count, table,
    /// compiled Jacobian, or RF effect. Its static occurrence and global
    /// capacity come only from the private manager Contact domain.
    #[must_use = "dropping the contribution abandons unpublished network Contacts"]
    struct SimulationPreparedNetworkContactContribution {
        source: SimulationPreparedNetworkContactSource,
        entries: Vec<SimulationPreparedNetworkContactEntry>,
    }

    fn fresh_manager_contact_authority()
    -> Result<SimulationManagerContactAuthority, SimulationManagerContactDomainInitError> {
        let raw = NEXT_MANAGER_CONTACT_AUTHORITY
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| SimulationManagerContactDomainInitError::AuthorityExhausted)?;
        NonZeroU64::new(raw)
            .map(SimulationManagerContactAuthority)
            .ok_or(SimulationManagerContactDomainInitError::AuthorityExhausted)
    }

    impl SimulationManagerContactDomain {
        pub(crate) fn try_new(
            body_pool: &mut SimulationRigidBodyPool,
            maximum: u32,
        ) -> Result<Self, SimulationManagerContactDomainInitError> {
            Self::try_new_with_reservation(body_pool, maximum, |records, count| {
                records.try_reserve_exact(count).map_err(|_| ())
            })
        }

        fn try_new_with_reservation(
            body_pool: &mut SimulationRigidBodyPool,
            maximum: u32,
            reserve: impl FnOnce(&mut Vec<SimulationOwnedContactEntry>, usize) -> Result<(), ()>,
        ) -> Result<Self, SimulationManagerContactDomainInitError> {
            maximum.checked_mul(0x100).ok_or(
                SimulationManagerContactDomainInitError::NativeContactByteSizeOverflow { maximum },
            )?;
            let maximum_usize = maximum as usize;
            if !body_pool
                .try_is_manager_construction_pristine()
                .map_err(SimulationManagerContactDomainInitError::BodyPool)?
            {
                return Err(SimulationManagerContactDomainInitError::BodyPoolNotPristine);
            }

            let mut records = Vec::new();
            if reserve(&mut records, maximum_usize).is_err() || records.capacity() < maximum_usize {
                return Err(SimulationManagerContactDomainInitError::AllocationFailed { maximum });
            }
            let authority = fresh_manager_contact_authority()?;
            let island_state =
                crate::simulation_island_map::persistent::SimulationRigidBodyIslandState::try_new(
                    body_pool,
                )
                .map_err(SimulationManagerContactDomainInitError::IslandState)?;
            let static_body = body_pool
                // Native passes its static-inertia address, but Static Add does
                // not retain it; `None` reconstructs the resulting body state.
                .try_add_rigid_body(IDENTITY_AFFINE, None, 1)
                .map_err(SimulationManagerContactDomainInitError::BodyPool)?
                .ok_or(SimulationManagerContactDomainInitError::StaticBodyUnavailable)?;

            Ok(Self {
                authority,
                body_pool: body_pool.authority(),
                static_body,
                maximum,
                records,
                last_started_frame: None,
                island_state,
                next_object_id: 0,
            })
        }

        pub(crate) const fn assembly_pool_authority(&self) -> SimulationManagerContactAuthority {
            self.authority
        }

        pub(crate) const fn assembly_body_pool_authority(
            &self,
        ) -> SimulationRigidBodyPoolAuthority {
            self.body_pool
        }

        /// `AssemblyPool::Insert` 0x82e0dbd0..dc: consume old u32 BEFORE Add.
        /// This is a native wrapping value, not a host lifetime capability.
        pub(crate) fn try_allocate_assembly_world_index(
            &mut self,
            body_pool: SimulationRigidBodyPoolAuthority,
        ) -> Result<
            u32,
            crate::simulation_island_map::persistent::SimulationRigidBodyIslandStateError,
        > {
            if body_pool != self.body_pool {
                return Err(crate::simulation_island_map::persistent::SimulationRigidBodyIslandStateError::ForeignPool);
            }
            let old = self.next_object_id;
            self.next_object_id = old.wrapping_add(1);
            Ok(old)
        }

        pub(crate) fn try_exclude_hook_assemblies<M, C>(
            &mut self,
            body_pool: &SimulationRigidBodyPool,
            hooks: &crate::dynamic_assembly_session::hook_pool::SimulationHookAssemblyPool<M, C>,
        ) -> Result<(), crate::dynamic_assembly_session::hook_pool::SimulationHookPoolError>
        {
            hooks.try_exclude_island_labels(self.authority, body_pool, &mut self.island_state)
        }

        pub(crate) fn static_body_observation(&self) -> SimulationRigidBodyObservation {
            self.static_body.observation()
        }

        /// Source `StartFrame`'s label stores only; timestep/counters and later
        /// offline assembly-pair partitioning are separate manager operations.
        pub(crate) fn try_start_island_frame_labels(
            &mut self,
            body_pool: &SimulationRigidBodyPool,
        ) -> Result<(), crate::simulation_island_map::persistent::SimulationRigidBodyIslandStateError>
        {
            self.island_state
                .try_start_frame_labels(body_pool, self.static_body.observation())
        }

        pub(crate) fn try_island_map_snapshot(
            &self,
            body_pool: &SimulationRigidBodyPool,
        ) -> Result<
            SimulationRigidBodyIslandMapSnapshot,
            crate::simulation_island_map::persistent::SimulationRigidBodyIslandStateError,
        > {
            self.island_state.try_snapshot(body_pool)
        }

        pub(crate) fn try_partition_assembly_pairs(
            &mut self,
            body_pool: &SimulationRigidBodyPool,
            pairs: &crate::simulation_island_map::assembly_pairs::SimulationAssemblyPairFrame<'_>,
        ) -> Result<(), crate::simulation_island_map::persistent::SimulationRigidBodyIslandStateError>
        {
            self.island_state.try_partition_assemblies(body_pool, pairs)
        }

        pub(crate) const fn maximum(&self) -> u32 {
            self.maximum
        }

        pub(super) fn try_begin_contact_frame<'manager>(
            &'manager mut self,
            phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
        ) -> Result<SimulationOpenContactFrame<'manager>, SimulationContactFrameBeginError>
        {
            if self.body_pool != phase.body_pool_authority() {
                return Err(SimulationContactFrameBeginError::BodyPoolMismatch);
            }
            let frame = phase.frame_identity();
            if self.last_started_frame == Some(frame) {
                return Err(SimulationContactFrameBeginError::FrameAlreadyStarted);
            }
            let static_body = self.static_body.observation();
            phase
                .try_validate_contact_observation(static_body)
                .map_err(SimulationContactFrameBeginError::StaticBody)?;

            self.records.clear();
            self.last_started_frame = Some(frame);
            let source = SimulationContactFrameSource {
                manager: self.authority,
                body_pool: self.body_pool,
                frame,
                phase: phase.phase_identity(),
                static_body,
            };
            Ok(SimulationOpenContactFrame {
                domain: self,
                source,
            })
        }
    }

    impl<'manager> SimulationOpenContactFrame<'manager> {
        pub(super) fn len(&self) -> usize {
            self.domain.records.len()
        }

        pub(super) const fn maximum(&self) -> u32 {
            self.domain.maximum
        }

        pub(super) const fn static_body_observation(&self) -> SimulationRigidBodyObservation {
            self.source.static_body
        }

        pub(super) fn try_seal(
            self,
            phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
        ) -> Result<
            SimulationSealedContactFrame<'manager>,
            SimulationContactFrameSealFailure<'manager>,
        > {
            let error = if self.source.manager != self.domain.authority {
                Some(SimulationContactFrameSealError::Manager)
            } else if self.source.body_pool != self.domain.body_pool
                || self.source.body_pool != phase.body_pool_authority()
            {
                Some(SimulationContactFrameSealError::BodyPool)
            } else if self.domain.last_started_frame != Some(self.source.frame)
                || self.source.frame != phase.frame_identity()
            {
                Some(SimulationContactFrameSealError::Frame)
            } else if self.source.phase != phase.phase_identity() {
                Some(SimulationContactFrameSealError::Phase)
            } else if self.source.static_body != self.domain.static_body.observation() {
                Some(SimulationContactFrameSealError::StaticBody)
            } else {
                None
            };
            if let Some(error) = error {
                return Err(SimulationContactFrameSealFailure { frame: self, error });
            }
            Ok(SimulationSealedContactFrame {
                domain: self.domain,
                source: self.source,
            })
        }
    }

    impl SimulationContactIslandMapSource<'_> {
        const fn frame(self) -> SimulationRigidBodyFrameIdentity {
            match self {
                Self::Null { frame } => frame,
                Self::Present(map) => map.frame_identity(),
            }
        }

        const fn body_pool(self) -> Option<SimulationRigidBodyPoolAuthority> {
            match self {
                Self::Null { .. } => None,
                Self::Present(map) => Some(map.body_pool_authority()),
            }
        }
    }

    fn try_contact_island_label<E>(
        source: SimulationContactIslandMapSource<'_>,
        dense_ordinal: u32,
        endpoint: SimulationContactTableEndpoint,
        body: SimulationRigidBodyObservation,
    ) -> Result<i8, SimulationNetworkContactIslandPreparationError<E>> {
        let SimulationContactIslandMapSource::Present(map) = source else {
            return Ok(0);
        };
        let Some((mapped_body, raw_label)) = map.assignment(body) else {
            return Err(
                SimulationNetworkContactIslandPreparationError::MissingIslandLabel {
                    dense_ordinal,
                    endpoint,
                    body,
                },
            );
        };
        if mapped_body != body {
            return Err(
                SimulationNetworkContactIslandPreparationError::IslandLabelOccurrenceMismatch {
                    dense_ordinal,
                    endpoint,
                    expected_body: body,
                    mapped_body,
                },
            );
        }
        Ok(raw_label)
    }

    fn try_selected_contact_island<E>(
        dense_ordinal: u32,
        raw_body_a: i8,
        raw_body_b: i8,
    ) -> Result<usize, SimulationNetworkContactIslandPreparationError<E>> {
        let selected = if raw_body_a == -1 {
            raw_body_b
        } else {
            raw_body_a
        };
        let Ok(island) = usize::try_from(selected) else {
            return Err(
                SimulationNetworkContactIslandPreparationError::InvalidSelectedIsland {
                    dense_ordinal,
                    raw_body_a,
                    raw_body_b,
                    selected,
                },
            );
        };
        if island >= SIMULATION_ISLAND_CAPACITY {
            return Err(
                SimulationNetworkContactIslandPreparationError::InvalidSelectedIsland {
                    dense_ordinal,
                    raw_body_a,
                    raw_body_b,
                    selected,
                },
            );
        }
        Ok(island)
    }

    fn contact_identity(
        source: SimulationContactFrameSource,
        dense_ordinal: u32,
    ) -> SimulationContactIdentity {
        SimulationContactIdentity {
            manager: source.manager,
            frame: source.frame,
            dense_ordinal,
        }
    }

    pub(super) fn try_prepare_network_contact_island_batch_with_backend<'manager, B>(
        frame: SimulationSealedContactFrame<'manager>,
        phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
        island_map: SimulationContactIslandMapSource<'_>,
        backend: &mut B,
    ) -> Result<
        SimulationPreparedNetworkContactIslandBatch<'manager>,
        SimulationNetworkContactIslandPreparationFailure<'manager, B::Error>,
    >
    where
        B: SimulationContactNumericBackend,
    {
        try_prepare_network_contact_island_batch_with_backend_reservation_and_observer(
            frame,
            phase,
            island_map,
            backend,
            |records, count| records.try_reserve_exact(count).map_err(|_| ()),
            |_, entries, count| entries.try_reserve_exact(count).map_err(|_| ()),
            |_, _| {},
        )
    }

    #[allow(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "one whole-frame preflight and publication boundary preserves failure atomicity"
    )]
    fn try_prepare_network_contact_island_batch_with_backend_reservation_and_observer<'manager, B>(
        frame: SimulationSealedContactFrame<'manager>,
        phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
        island_map: SimulationContactIslandMapSource<'_>,
        backend: &mut B,
        reserve_records: impl FnOnce(
            &mut Vec<SimulationPreparedNetworkContactJacobian>,
            usize,
        ) -> Result<(), ()>,
        mut reserve_island: impl FnMut(
            usize,
            &mut Vec<SimulationPreparedNetworkContactIslandEntry>,
            usize,
        ) -> Result<(), ()>,
        mut observe: impl FnMut(SimulationContactIslandPreparationReadKind, u32),
    ) -> Result<
        SimulationPreparedNetworkContactIslandBatch<'manager>,
        SimulationNetworkContactIslandPreparationFailure<'manager, B::Error>,
    >
    where
        B: SimulationContactNumericBackend,
    {
        let preparation = (|| {
            if frame.source.manager != frame.domain.authority {
                return Err(SimulationNetworkContactIslandPreparationError::SourceManagerMismatch);
            }
            if frame.source.body_pool != frame.domain.body_pool
                || frame.source.body_pool != phase.body_pool_authority()
            {
                return Err(SimulationNetworkContactIslandPreparationError::SourceBodyPoolMismatch);
            }
            if frame.domain.last_started_frame != Some(frame.source.frame)
                || frame.source.frame != phase.frame_identity()
            {
                return Err(SimulationNetworkContactIslandPreparationError::SourceFrameMismatch);
            }
            if frame.source.phase != phase.phase_identity() {
                return Err(SimulationNetworkContactIslandPreparationError::SourcePhaseMismatch);
            }
            if frame.source.static_body != frame.domain.static_body.observation() {
                return Err(
                    SimulationNetworkContactIslandPreparationError::SourceStaticBodyMismatch,
                );
            }
            phase
                .try_validate_contact_observation(frame.source.static_body)
                .map_err(SimulationNetworkContactIslandPreparationError::SourceStaticBody)?;
            if island_map
                .body_pool()
                .is_some_and(|authority| authority != frame.source.body_pool)
            {
                return Err(
                    SimulationNetworkContactIslandPreparationError::IslandMapBodyPoolMismatch,
                );
            }
            if island_map.frame() != frame.source.frame {
                return Err(SimulationNetworkContactIslandPreparationError::IslandMapFrameMismatch);
            }

            let mut island_counts = [0_usize; SIMULATION_ISLAND_CAPACITY];
            for (dense_index, owned) in frame.domain.records.iter().enumerate() {
                let dense_ordinal = u32::try_from(dense_index).map_err(|_| {
                    SimulationNetworkContactIslandPreparationError::DenseOrdinalOverflow {
                        dense_index,
                    }
                })?;
                observe(
                    SimulationContactIslandPreparationReadKind::DenseOrdinal,
                    dense_ordinal,
                );
                if owned.dense_ordinal != dense_ordinal {
                    return Err(
                        SimulationNetworkContactIslandPreparationError::DenseOrdinalMismatch {
                            dense_index,
                            actual: owned.dense_ordinal,
                        },
                    );
                }
                observe(
                    SimulationContactIslandPreparationReadKind::Role,
                    dense_ordinal,
                );
                let expected_roles = owned.producer.expected_roles(frame.source.static_body);
                for (kind, actual, expected) in [
                    (
                        SimulationNetworkContactPreparedBindingKind::TableBodyA,
                        owned.roles.table_body_a,
                        expected_roles.table_body_a,
                    ),
                    (
                        SimulationNetworkContactPreparedBindingKind::TableBodyB,
                        owned.roles.table_body_b,
                        expected_roles.table_body_b,
                    ),
                    (
                        SimulationNetworkContactPreparedBindingKind::SolverSide0,
                        owned.roles.solver_side_0,
                        expected_roles.solver_side_0,
                    ),
                    (
                        SimulationNetworkContactPreparedBindingKind::SolverSide1,
                        owned.roles.solver_side_1,
                        expected_roles.solver_side_1,
                    ),
                ] {
                    if actual != expected {
                        return Err(
                            SimulationNetworkContactIslandPreparationError::RoleMismatch {
                                dense_ordinal,
                                kind,
                            },
                        );
                    }
                    phase
                        .try_validate_contact_observation(actual)
                        .map_err(
                            |source| SimulationNetworkContactIslandPreparationError::Body {
                                dense_ordinal,
                                kind,
                                source,
                            },
                        )?;
                }

                observe(
                    SimulationContactIslandPreparationReadKind::BindingToken,
                    dense_ordinal,
                );
                for (kind, observation, actual) in [
                    (
                        SimulationNetworkContactPreparedBindingKind::TableBodyA,
                        owned.roles.table_body_a,
                        owned.input.position_a_body_a.word,
                    ),
                    (
                        SimulationNetworkContactPreparedBindingKind::TableBodyB,
                        owned.roles.table_body_b,
                        owned.input.position_b_body_b.word,
                    ),
                    (
                        SimulationNetworkContactPreparedBindingKind::SolverSide0,
                        owned.roles.solver_side_0,
                        owned.input.side_0_com_rf_token.word,
                    ),
                    (
                        SimulationNetworkContactPreparedBindingKind::SolverSide1,
                        owned.roles.solver_side_1,
                        owned.input.side_1_com_rf_token.word,
                    ),
                ] {
                    let slot = observation.identity().id().index();
                    let expected = slot.checked_add(1).ok_or(
                        SimulationNetworkContactIslandPreparationError::BindingTokenOverflow {
                            dense_ordinal,
                            kind,
                            slot,
                        },
                    )?;
                    if actual != expected {
                        return Err(
                            SimulationNetworkContactIslandPreparationError::BindingTokenMismatch {
                                dense_ordinal,
                                kind,
                                expected,
                                actual,
                            },
                        );
                    }
                }

                observe(
                    SimulationContactIslandPreparationReadKind::IslandLabelA,
                    dense_ordinal,
                );
                let raw_body_a = try_contact_island_label(
                    island_map,
                    dense_ordinal,
                    SimulationContactTableEndpoint::BodyA,
                    owned.roles.table_body_a,
                )?;
                observe(
                    SimulationContactIslandPreparationReadKind::IslandLabelB,
                    dense_ordinal,
                );
                let raw_body_b = try_contact_island_label(
                    island_map,
                    dense_ordinal,
                    SimulationContactTableEndpoint::BodyB,
                    owned.roles.table_body_b,
                )?;
                let island = try_selected_contact_island(dense_ordinal, raw_body_a, raw_body_b)?;
                island_counts[island] = island_counts[island].checked_add(1).ok_or(
                    SimulationNetworkContactIslandPreparationError::IslandCountOverflow {
                        island: u8::try_from(island).expect("validated island is in 0..10"),
                        count: island_counts[island],
                    },
                )?;
            }

            let count = frame.domain.records.len();
            let mut records = Vec::new();
            if reserve_records(&mut records, count).is_err() || records.capacity() < count {
                return Err(
                    SimulationNetworkContactIslandPreparationError::RecordAllocationFailed {
                        count,
                    },
                );
            }
            let mut islands: [Vec<SimulationPreparedNetworkContactIslandEntry>;
                SIMULATION_ISLAND_CAPACITY] = std::array::from_fn(|_| Vec::new());
            for (island, (&required, entries)) in
                island_counts.iter().zip(islands.iter_mut()).enumerate()
            {
                if required == 0 {
                    continue;
                }
                if reserve_island(island, entries, required).is_err()
                    || entries.capacity() < required
                {
                    return Err(
                        SimulationNetworkContactIslandPreparationError::IslandAllocationFailed {
                            island: u8::try_from(island)
                                .expect("physical island enumeration is in 0..10"),
                            count: required,
                        },
                    );
                }
            }

            for owned in &frame.domain.records {
                let dense_ordinal = owned.dense_ordinal;
                let raw_body_a = try_contact_island_label(
                    island_map,
                    dense_ordinal,
                    SimulationContactTableEndpoint::BodyA,
                    owned.roles.table_body_a,
                )?;
                let raw_body_b = try_contact_island_label(
                    island_map,
                    dense_ordinal,
                    SimulationContactTableEndpoint::BodyB,
                    owned.roles.table_body_b,
                )?;
                let island = try_selected_contact_island(dense_ordinal, raw_body_a, raw_body_b)?;
                islands[island].push(SimulationPreparedNetworkContactIslandEntry {
                    contact: contact_identity(frame.source, dense_ordinal),
                    table_body_a: owned.roles.table_body_a,
                    table_body_b: owned.roles.table_body_b,
                });
            }

            let timestep = phase.timestep();
            for owned in &frame.domain.records {
                observe(
                    SimulationContactIslandPreparationReadKind::Compile,
                    owned.dense_ordinal,
                );
                let contact = contact_identity(frame.source, owned.dense_ordinal);
                let record = build_contact_jacobian_host(owned.input, timestep, backend).map_err(
                    |source| SimulationNetworkContactIslandPreparationError::Numeric {
                        contact,
                        source,
                    },
                )?;
                records.push(SimulationPreparedNetworkContactJacobian {
                    contact,
                    table_body_a: owned.roles.table_body_a,
                    table_body_b: owned.roles.table_body_b,
                    solver_side_0: owned.roles.solver_side_0,
                    solver_side_1: owned.roles.solver_side_1,
                    record,
                });
            }
            Ok((records, islands))
        })();

        match preparation {
            Ok((records, islands)) => Ok(SimulationPreparedNetworkContactIslandBatch {
                identity: SimulationContactBatchIdentity(std::sync::Arc::new(())),
                frame,
                records,
                islands,
            }),
            Err(error) => Err(SimulationNetworkContactIslandPreparationFailure { frame, error }),
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one linear preflight keeps every checked refusal before the first Contact/RF effect"
    )]
    fn validate_prepared_network_contact_horus_island(
        batch: &SimulationPreparedNetworkContactIslandBatch<'_>,
        island: usize,
        phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    ) -> Result<(), SimulationPreparedNetworkContactIslandHorusError> {
        if island >= SIMULATION_ISLAND_CAPACITY {
            return Err(
                SimulationPreparedNetworkContactIslandHorusError::IslandOutOfRange { island },
            );
        }
        let source = batch.frame.source;
        let domain = batch.frame.domain;
        if source.manager != domain.authority {
            return Err(SimulationPreparedNetworkContactIslandHorusError::SourceManagerMismatch);
        }
        if source.body_pool != domain.body_pool || source.body_pool != phase.body_pool_authority() {
            return Err(SimulationPreparedNetworkContactIslandHorusError::SourceBodyPoolMismatch);
        }
        if domain.last_started_frame != Some(source.frame) || source.frame != phase.frame_identity()
        {
            return Err(SimulationPreparedNetworkContactIslandHorusError::SourceFrameMismatch);
        }
        if source.phase != phase.phase_identity() {
            return Err(SimulationPreparedNetworkContactIslandHorusError::SourcePhaseMismatch);
        }
        if source.static_body != domain.static_body.observation() {
            return Err(SimulationPreparedNetworkContactIslandHorusError::SourceStaticBodyMismatch);
        }
        phase
            .try_validate_contact_observation(source.static_body)
            .map_err(SimulationPreparedNetworkContactIslandHorusError::SourceStaticBody)?;

        let entries = &batch.islands[island];
        for (island_ordinal, entry) in entries.iter().enumerate() {
            let dense_index = entry.contact.dense_ordinal as usize;
            let Some(prepared) = batch.records.get(dense_index) else {
                return Err(
                    SimulationPreparedNetworkContactIslandHorusError::TableEntryMismatch {
                        island_ordinal,
                        contact: entry.contact,
                    },
                );
            };
            if prepared.contact != entry.contact
                || prepared.table_body_a != entry.table_body_a
                || prepared.table_body_b != entry.table_body_b
            {
                return Err(
                    SimulationPreparedNetworkContactIslandHorusError::TableEntryMismatch {
                        island_ordinal,
                        contact: entry.contact,
                    },
                );
            }
            let expected_contact = contact_identity(source, entry.contact.dense_ordinal);
            let Some(owned) = domain.records.get(dense_index) else {
                return Err(
                    SimulationPreparedNetworkContactIslandHorusError::SourceRecordMismatch {
                        island_ordinal,
                        contact: entry.contact,
                    },
                );
            };
            if entry.contact != expected_contact
                || owned.dense_ordinal != entry.contact.dense_ordinal
                || prepared.table_body_a != owned.roles.table_body_a
                || prepared.table_body_b != owned.roles.table_body_b
                || prepared.solver_side_0 != owned.roles.solver_side_0
                || prepared.solver_side_1 != owned.roles.solver_side_1
                || owned.roles != owned.producer.expected_roles(source.static_body)
            {
                return Err(
                    SimulationPreparedNetworkContactIslandHorusError::SourceRecordMismatch {
                        island_ordinal,
                        contact: entry.contact,
                    },
                );
            }

            for (kind, observation, actual) in [
                (
                    SimulationNetworkContactPreparedBindingKind::TableBodyA,
                    prepared.table_body_a,
                    prepared.record.body_a.0,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::TableBodyB,
                    prepared.table_body_b,
                    prepared.record.body_b.0,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::SolverSide0,
                    prepared.solver_side_0,
                    prepared.record.solver_delta_a.0,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::SolverSide1,
                    prepared.solver_side_1,
                    prepared.record.solver_delta_b.0,
                ),
            ] {
                phase
                    .try_validate_contact_observation(observation)
                    .map_err(
                        |source| SimulationPreparedNetworkContactIslandHorusError::Body {
                            island_ordinal,
                            contact: entry.contact,
                            kind,
                            source,
                        },
                    )?;
                let slot = observation.identity().id().index();
                let expected = checked_binding_word(slot, kind).map_err(|(_, slot)| {
                    SimulationPreparedNetworkContactIslandHorusError::BindingTokenOverflow {
                        island_ordinal,
                        contact: entry.contact,
                        kind,
                        slot,
                    }
                })?;
                if actual != expected {
                    return Err(
                        SimulationPreparedNetworkContactIslandHorusError::BindingTokenMismatch {
                            island_ordinal,
                            contact: entry.contact,
                            kind,
                            expected,
                            actual,
                        },
                    );
                }
            }
        }

        Ok(())
    }

    /// Prevalidates and preallocates one network Contact-table subsequence.
    ///
    /// This is one ordinary-finite selected-island Contact sweep, not a complete
    /// Horus iteration. Every batch, phase, occurrence, table, role, token, and
    /// selector check precedes the first Contact-record or RF effect. Execution
    /// is then infallible except for panic; an earlier/current effect prefix is
    /// deliberately not rolled back.
    pub(crate) fn try_prepare_network_contact_horus_island_sweep<'batch>(
        batch: &'batch mut SimulationPreparedNetworkContactIslandBatch<'_>,
        island: usize,
        phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    ) -> Result<
        SimulationRigidBodyIndexedSolverDeltaPlan<'batch, SimulationPreparedNetworkContactJacobian>,
        SimulationPreparedNetworkContactIslandHorusError,
    > {
        validate_prepared_network_contact_horus_island(batch, island, phase)?;

        let SimulationPreparedNetworkContactIslandBatch {
            frame,
            records,
            islands,
            ..
        } = batch;
        let entries = &islands[island];
        phase
            .try_prepare_indexed_solver_delta_plan(
                records,
                entries.iter().map(|entry| {
                    let dense_index = entry.contact.dense_ordinal as usize;
                    let owned = &frame.domain.records[dense_index];
                    (
                        dense_index,
                        owned.roles.solver_side_0,
                        owned.roles.solver_side_1,
                    )
                }),
            )
            .map_err(SimulationPreparedNetworkContactIslandHorusError::BodySequence)
    }

    /// Prevalidates all scheduled Contact islands into one reusable RF plan.
    ///
    /// Semantic checks visit islands in ascending host-policy order and retain
    /// the failing island. Only after every island passes does the phase reserve
    /// and resolve one flattened body sequence. No record or RF effect occurs.
    pub(crate) fn try_prepare_network_contact_horus_serial_plan<'batch>(
        batch: &'batch mut SimulationPreparedNetworkContactIslandBatch<'_>,
        scheduled_islands: usize,
        phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    ) -> Result<
        SimulationPreparedNetworkContactHorusSerialPlan<'batch>,
        SimulationPreparedNetworkContactHorusSerialPlanError,
    > {
        if scheduled_islands > SIMULATION_ISLAND_CAPACITY {
            return Err(
                SimulationPreparedNetworkContactHorusSerialPlanError::IslandCountOutOfRange {
                    scheduled_islands,
                },
            );
        }
        let mut ranges = std::array::from_fn(|_| 0..0);
        let mut entry_count = 0_usize;
        for (island, range) in ranges.iter_mut().enumerate().take(scheduled_islands) {
            validate_prepared_network_contact_horus_island(batch, island, phase).map_err(
                |source| SimulationPreparedNetworkContactHorusSerialPlanError::Island {
                    island,
                    source,
                },
            )?;
            let incoming = batch.islands[island].len();
            let start = entry_count;
            entry_count = entry_count.checked_add(incoming).ok_or(
                SimulationPreparedNetworkContactHorusSerialPlanError::EntryCountOverflow {
                    island,
                    accumulated: start,
                    incoming,
                },
            )?;
            *range = start..entry_count;
        }

        let SimulationPreparedNetworkContactIslandBatch {
            frame,
            records,
            islands,
            ..
        } = batch;
        let sequence = islands[..scheduled_islands]
            .iter()
            .flat_map(|entries| entries.iter())
            .map(|entry| {
                let dense_index = entry.contact.dense_ordinal as usize;
                let owned = &frame.domain.records[dense_index];
                (
                    dense_index,
                    owned.roles.solver_side_0,
                    owned.roles.solver_side_1,
                )
            });
        let rf = phase
            .try_prepare_indexed_solver_delta_plan_counted(records, entry_count, sequence)
            .map_err(SimulationPreparedNetworkContactHorusSerialPlanError::BodySequence)?;
        Ok(SimulationPreparedNetworkContactHorusSerialPlan {
            rf,
            ranges,
            scheduled_islands,
        })
    }

    /// Applies one already-prevalidated network Contact island sweep.
    pub(crate) fn apply_prepared_network_contact_horus_island_sweep(
        plan: &mut SimulationRigidBodyIndexedSolverDeltaPlan<
            '_,
            SimulationPreparedNetworkContactJacobian,
        >,
        phase: &mut SimulationRigidBodyDynamicUpdatePhase<'_>,
    ) {
        phase.apply_indexed_contact_solver_delta_plan(plan, |prepared, rows| {
            solve_prepared_network_contact_horus_record(prepared, rows)
        });
    }

    /// Applies one scheduled island range from a fully prevalidated serial plan.
    pub(crate) fn apply_prepared_network_contact_horus_serial_island(
        plan: &mut SimulationPreparedNetworkContactHorusSerialPlan<'_>,
        island: usize,
        phase: &mut SimulationRigidBodyDynamicUpdatePhase<'_>,
    ) {
        assert!(
            island < plan.scheduled_islands,
            "Contact serial-plan island must have been scheduled"
        );
        let range = plan.ranges[island].clone();
        phase.apply_indexed_contact_solver_delta_plan_range(
            &mut plan.rf,
            range,
            solve_prepared_network_contact_horus_record,
        );
    }

    /// Solves one prevalidated Contact record and stages its exact eight stores.
    pub(crate) fn solve_prepared_network_contact_horus_record(
        prepared: &mut SimulationPreparedNetworkContactJacobian,
        rows: SimulationRigidBodySolverDeltaPairRows,
    ) -> SimulationRigidBodyContactSolverDeltaStores {
        let (solver_delta_a, solver_delta_b) = rows.into_rows();
        let solved =
            solve_contact_horus_record_host(&mut prepared.record, solver_delta_a, solver_delta_b);
        SimulationRigidBodyContactSolverDeltaStores::new(
            solved.solver_delta_a[3],
            solved.solver_delta_b[3],
            solved.solver_delta_a[1],
            solved.solver_delta_b[1],
            solved.solver_delta_a[2],
            solved.solver_delta_b[2],
            solved.solver_delta_a[0],
            solved.solver_delta_b[0],
        )
    }

    /// Executes one checked standalone network Contact island sweep.
    pub(crate) fn try_solve_prepared_network_contact_horus_island_sweep(
        batch: &mut SimulationPreparedNetworkContactIslandBatch<'_>,
        island: usize,
        phase: &mut SimulationRigidBodyDynamicUpdatePhase<'_>,
    ) -> Result<(), SimulationPreparedNetworkContactIslandHorusError> {
        let mut plan = try_prepare_network_contact_horus_island_sweep(batch, island, phase)?;
        apply_prepared_network_contact_horus_island_sweep(&mut plan, phase);
        Ok(())
    }

    fn network_contact_common_body(
        body: SimulationRigidBodyNetworkContactSnapshot,
        solver_delta: SolverDeltaRowToken,
    ) -> SimulationContactCommonBodyInput {
        SimulationContactCommonBodyInput {
            center_of_mass: body.center_of_mass,
            solver_delta,
            velocity: body.velocity,
            omega: body.omega,
            inertia_full: body.inverse_inertia_full,
            inverse_mass: body.inverse_mass,
            inertia_split: body.inverse_inertia_split,
            state_word: body.state,
            force: body.force,
            kinematic_value: body.kinematic_value,
            torque: body.torque,
            cooldown_word: body.cooldown,
        }
    }

    fn checked_binding_word(
        slot: u32,
        kind: SimulationNetworkContactPreparedBindingKind,
    ) -> Result<u32, (SimulationNetworkContactPreparedBindingKind, u32)> {
        slot.checked_add(1).ok_or((kind, slot))
    }

    fn try_stage_network_contact(
        queue_ordinal: usize,
        static_body: SimulationRigidBodyNetworkContactSnapshot,
        selected_body: SimulationRigidBodyNetworkContactSnapshot,
        request: crate::simulation_contact_network_queue::SimulationResolvedNetworkContactRequest,
    ) -> Result<SimulationStagedNetworkContact, (SimulationNetworkContactPreparedBindingKind, u32)>
    {
        let static_slot = static_body.observation.identity().id().index();
        let selected_slot = selected_body.observation.identity().id().index();
        let table_body_a = checked_binding_word(
            static_slot,
            SimulationNetworkContactPreparedBindingKind::TableBodyA,
        )?;
        let table_body_b = checked_binding_word(
            selected_slot,
            SimulationNetworkContactPreparedBindingKind::TableBodyB,
        )?;
        let solver_side_0 = checked_binding_word(
            static_slot,
            SimulationNetworkContactPreparedBindingKind::SolverSide0,
        )?;
        let solver_side_1 = checked_binding_word(
            selected_slot,
            SimulationNetworkContactPreparedBindingKind::SolverSide1,
        )?;
        let roles = SimulationPreparedNetworkContactRoles {
            table_body_a: static_body.observation,
            table_body_b: selected_body.observation,
            solver_side_0: static_body.observation,
            solver_side_1: selected_body.observation,
        };
        Ok(SimulationStagedNetworkContact {
            queue_ordinal,
            input: SimulationNetworkContactDrainInput {
                static_body: network_contact_common_body(
                    static_body,
                    SolverDeltaRowToken(solver_side_0),
                ),
                selected_body: SimulationNetworkContactSelectedBodyInput {
                    common: network_contact_common_body(
                        selected_body,
                        SolverDeltaRowToken(solver_side_1),
                    ),
                    right: selected_body.right,
                    up: selected_body.up,
                    at: selected_body.at,
                },
                point: request.point,
                normal: request.normal,
                displacement: request.displacement,
                static_body_token: ContactBodyToken(table_body_a),
                selected_body_token: ContactBodyToken(table_body_b),
                static_friction: request.material.static_friction,
                dynamic_friction: request.material.dynamic_friction,
                restitution: request.material.restitution,
            },
            roles,
        })
    }

    /// Prepares one whole queue without publishing records or mutating bodies.
    ///
    /// Every returned typed failure owns the exact unchanged queue. A panic in
    /// allocation or the injected numeric backend instead unwinds through and
    /// drops that queue; any backend trace prefix is not rolled back. This
    /// boundary makes no panic-recovery guarantee.
    fn try_prepare_network_contact_contribution_with_backend<B>(
        queue: SimulationNetworkContactQueue,
        destination: &SimulationOpenContactFrame<'_>,
        phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
        backend: &mut B,
    ) -> Result<
        SimulationPreparedNetworkContactContribution,
        SimulationNetworkContactPreparationFailure<B::Error>,
    >
    where
        B: SimulationNetworkContactDrainNumericBackend,
    {
        try_prepare_network_contact_contribution_with_backend_and_reservation(
            queue,
            destination,
            phase,
            backend,
            |staged, count| staged.try_reserve_exact(count).map_err(|_| ()),
            |entries, count| entries.try_reserve_exact(count).map_err(|_| ()),
        )
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one linear whole-queue preflight keeps typed failures before backend effects"
    )]
    fn try_prepare_network_contact_contribution_with_backend_and_reservation<B>(
        queue: SimulationNetworkContactQueue,
        destination: &SimulationOpenContactFrame<'_>,
        phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
        backend: &mut B,
        reserve_staging: impl FnOnce(&mut Vec<SimulationStagedNetworkContact>, usize) -> Result<(), ()>,
        reserve_records: impl FnOnce(
            &mut Vec<SimulationPreparedNetworkContactEntry>,
            usize,
        ) -> Result<(), ()>,
    ) -> Result<
        SimulationPreparedNetworkContactContribution,
        SimulationNetworkContactPreparationFailure<B::Error>,
    >
    where
        B: SimulationNetworkContactDrainNumericBackend,
    {
        if destination.source.manager != destination.domain.authority {
            return Err(SimulationNetworkContactPreparationFailure {
                queue,
                error: SimulationNetworkContactPreparationError::DestinationManagerMismatch,
            });
        }
        if destination.source.body_pool != destination.domain.body_pool
            || destination.source.body_pool != phase.body_pool_authority()
        {
            return Err(SimulationNetworkContactPreparationFailure {
                queue,
                error: SimulationNetworkContactPreparationError::DestinationBodyPoolMismatch,
            });
        }
        if destination.domain.last_started_frame != Some(destination.source.frame)
            || destination.source.frame != phase.frame_identity()
        {
            return Err(SimulationNetworkContactPreparationFailure {
                queue,
                error: SimulationNetworkContactPreparationError::DestinationFrameMismatch,
            });
        }
        if destination.source.phase != phase.phase_identity() {
            return Err(SimulationNetworkContactPreparationFailure {
                queue,
                error: SimulationNetworkContactPreparationError::DestinationPhaseMismatch,
            });
        }
        if destination.source.static_body != destination.domain.static_body.observation() {
            return Err(SimulationNetworkContactPreparationFailure {
                queue,
                error: SimulationNetworkContactPreparationError::DestinationStaticBodyMismatch,
            });
        }
        if queue.body_pool_authority() != phase.body_pool_authority() {
            return Err(SimulationNetworkContactPreparationFailure {
                queue,
                error: SimulationNetworkContactPreparationError::QueueBodyPoolMismatch,
            });
        }
        if queue.frame_identity() != phase.frame_identity() {
            return Err(SimulationNetworkContactPreparationFailure {
                queue,
                error: SimulationNetworkContactPreparationError::QueueFrameMismatch,
            });
        }
        if queue.is_empty() {
            return Err(SimulationNetworkContactPreparationFailure {
                queue,
                error: SimulationNetworkContactPreparationError::EmptyQueue,
            });
        }

        let count = queue.len();
        let current = destination.domain.records.len();
        let maximum = destination.domain.maximum;
        if count > (maximum as usize).saturating_sub(current) {
            return Err(SimulationNetworkContactPreparationFailure {
                queue,
                error: SimulationNetworkContactPreparationError::DestinationCapacityExceeded {
                    current,
                    incoming: count,
                    maximum,
                },
            });
        }
        let mut staged = Vec::new();
        if reserve_staging(&mut staged, count).is_err() || staged.capacity() < count {
            return Err(SimulationNetworkContactPreparationFailure {
                queue,
                error: SimulationNetworkContactPreparationError::StagingAllocationFailed { count },
            });
        }
        let mut entries = Vec::new();
        if reserve_records(&mut entries, count).is_err() || entries.capacity() < count {
            return Err(SimulationNetworkContactPreparationFailure {
                queue,
                error: SimulationNetworkContactPreparationError::RecordAllocationFailed { count },
            });
        }

        let stage_result = (|| {
            for (queue_ordinal, request) in queue.entries().iter().copied().enumerate() {
                let static_snapshot = phase
                    .try_network_contact_snapshot(destination.source.static_body)
                    .map_err(|source| SimulationNetworkContactPreparationError::Body {
                        queue_ordinal,
                        role: SimulationNetworkContactPreparationBodyRole::Static,
                        source,
                    })?;
                let selected_snapshot =
                    phase
                        .try_network_contact_snapshot(request.selected_body)
                        .map_err(|source| SimulationNetworkContactPreparationError::Body {
                            queue_ordinal,
                            role: SimulationNetworkContactPreparationBodyRole::Selected,
                            source,
                        })?;
                staged.push(
                    try_stage_network_contact(
                        queue_ordinal,
                        static_snapshot,
                        selected_snapshot,
                        request,
                    )
                    .map_err(|(kind, slot)| {
                        SimulationNetworkContactPreparationError::BindingTokenOverflow {
                            queue_ordinal,
                            kind,
                            slot,
                        }
                    })?,
                );
            }
            Ok::<(), SimulationNetworkContactPreparationError<B::Error>>(())
        })();
        if let Err(error) = stage_result {
            return Err(SimulationNetworkContactPreparationFailure { queue, error });
        }

        for staged_entry in staged {
            let input = match build_network_contact_input(&staged_entry.input, backend) {
                Ok(input) => input,
                Err(source) => {
                    return Err(SimulationNetworkContactPreparationFailure {
                        queue,
                        error: SimulationNetworkContactPreparationError::Numeric {
                            queue_ordinal: staged_entry.queue_ordinal,
                            source,
                        },
                    });
                }
            };
            entries.push(SimulationPreparedNetworkContactEntry {
                queue_ordinal: staged_entry.queue_ordinal,
                selected_body: staged_entry.roles.table_body_b,
                input,
                roles: staged_entry.roles,
            });
        }

        Ok(SimulationPreparedNetworkContactContribution {
            source: SimulationPreparedNetworkContactSource {
                manager: destination.source.manager,
                queue: queue.authority(),
                body_pool: phase.body_pool_authority(),
                frame: phase.frame_identity(),
                phase: phase.phase_identity(),
                static_body: destination.source.static_body,
            },
            entries,
        })
    }

    fn try_validate_contact_destination(
        destination: &SimulationOpenContactFrame<'_>,
        phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    ) -> Result<(), SimulationContactAppendError> {
        if destination.source.manager != destination.domain.authority {
            return Err(SimulationContactAppendError::DestinationManagerMismatch);
        }
        if destination.source.body_pool != destination.domain.body_pool
            || destination.source.body_pool != phase.body_pool_authority()
        {
            return Err(SimulationContactAppendError::DestinationBodyPoolMismatch);
        }
        if destination.domain.last_started_frame != Some(destination.source.frame)
            || destination.source.frame != phase.frame_identity()
        {
            return Err(SimulationContactAppendError::DestinationFrameMismatch);
        }
        if destination.source.phase != phase.phase_identity() {
            return Err(SimulationContactAppendError::DestinationPhaseMismatch);
        }
        if destination.source.static_body != destination.domain.static_body.observation() {
            return Err(SimulationContactAppendError::DestinationStaticBodyMismatch);
        }
        Ok(())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one linear whole-contribution preflight precedes every destination write"
    )]
    fn try_validate_contact_contribution(
        destination: &SimulationOpenContactFrame<'_>,
        contribution: &SimulationPreparedNetworkContactContribution,
        phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    ) -> Result<(), SimulationContactAppendError> {
        try_validate_contact_destination(destination, phase)?;
        if contribution.source.manager != destination.source.manager {
            return Err(SimulationContactAppendError::ContributionManagerMismatch);
        }
        if contribution.source.body_pool != destination.source.body_pool {
            return Err(SimulationContactAppendError::ContributionBodyPoolMismatch);
        }
        if contribution.source.frame != destination.source.frame {
            return Err(SimulationContactAppendError::ContributionFrameMismatch);
        }
        if contribution.source.phase != destination.source.phase {
            return Err(SimulationContactAppendError::ContributionPhaseMismatch);
        }
        if contribution.source.static_body != destination.source.static_body {
            return Err(SimulationContactAppendError::ContributionStaticBodyMismatch);
        }

        let current = destination.domain.records.len();
        let incoming = contribution.entries.len();
        let maximum = destination.domain.maximum;
        if incoming > (maximum as usize).saturating_sub(current) {
            return Err(SimulationContactAppendError::CapacityExceeded {
                current,
                incoming,
                maximum,
            });
        }

        for (contribution_ordinal, entry) in contribution.entries.iter().enumerate() {
            if entry.queue_ordinal != contribution_ordinal {
                return Err(SimulationContactAppendError::QueueOrdinalMismatch {
                    contribution_ordinal,
                    actual: entry.queue_ordinal,
                });
            }
            for (kind, actual, expected) in [
                (
                    SimulationNetworkContactPreparedBindingKind::TableBodyA,
                    entry.roles.table_body_a,
                    contribution.source.static_body,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::SolverSide0,
                    entry.roles.solver_side_0,
                    contribution.source.static_body,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::TableBodyB,
                    entry.roles.table_body_b,
                    entry.selected_body,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::SolverSide1,
                    entry.roles.solver_side_1,
                    entry.selected_body,
                ),
            ] {
                if actual != expected {
                    return Err(SimulationContactAppendError::RoleMismatch {
                        contribution_ordinal,
                        kind,
                    });
                }
            }

            for (kind, observation) in [
                (
                    SimulationNetworkContactPreparedBindingKind::TableBodyA,
                    entry.roles.table_body_a,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::TableBodyB,
                    entry.roles.table_body_b,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::SolverSide0,
                    entry.roles.solver_side_0,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::SolverSide1,
                    entry.roles.solver_side_1,
                ),
            ] {
                phase
                    .try_validate_contact_observation(observation)
                    .map_err(|source| SimulationContactAppendError::Body {
                        contribution_ordinal,
                        kind,
                        source,
                    })?;
            }

            for (kind, observation, actual) in [
                (
                    SimulationNetworkContactPreparedBindingKind::TableBodyA,
                    entry.roles.table_body_a,
                    entry.input.position_a_body_a.word,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::TableBodyB,
                    entry.roles.table_body_b,
                    entry.input.position_b_body_b.word,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::SolverSide0,
                    entry.roles.solver_side_0,
                    entry.input.side_0_com_rf_token.word,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::SolverSide1,
                    entry.roles.solver_side_1,
                    entry.input.side_1_com_rf_token.word,
                ),
            ] {
                let slot = observation.identity().id().index();
                let expected = slot.checked_add(1).ok_or(
                    SimulationContactAppendError::BindingTokenOverflow {
                        contribution_ordinal,
                        kind,
                        slot,
                    },
                )?;
                if actual != expected {
                    return Err(SimulationContactAppendError::BindingTokenMismatch {
                        contribution_ordinal,
                        kind,
                        expected,
                        actual,
                    });
                }
            }
        }
        Ok(())
    }

    impl SimulationOpenContactFrame<'_> {
        /// Imports the already accepted dense prefix produced by native-order
        /// CPU ground append. This is not another capacity-clamping stage:
        /// disagreement with the destination bound refuses the whole import.
        /// All occurrences, phase provenance and four independent binding words
        /// are checked before the first persistent write. The manager reserved
        /// its entire logical capacity at construction, so import cannot allocate.
        #[allow(
            clippy::too_many_lines,
            reason = "whole-prefix preflight must precede every destination write"
        )]
        pub(super) fn try_append_ground_contact_records(
            &mut self,
            records: &[super::ground_preparation::SimulationGroundContactRecord],
            phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
        ) -> Result<(), SimulationContactAppendError> {
            try_validate_contact_destination(self, phase)?;
            let current = self.domain.records.len();
            let incoming = records.len();
            let maximum = self.domain.maximum;
            if incoming > (maximum as usize).saturating_sub(current) {
                return Err(SimulationContactAppendError::CapacityExceeded {
                    current,
                    incoming,
                    maximum,
                });
            }
            for (contribution_ordinal, entry) in records.iter().enumerate() {
                if entry.pool != self.source.body_pool {
                    return Err(SimulationContactAppendError::ContributionBodyPoolMismatch);
                }
                if entry.frame != self.source.frame {
                    return Err(SimulationContactAppendError::ContributionFrameMismatch);
                }
                if entry.phase != self.source.phase {
                    return Err(SimulationContactAppendError::ContributionPhaseMismatch);
                }
                // CPU job obtains table/live A from the same volume occurrence
                // and both B roles from its manager static body. Retain each
                // separately; this does not infer generic producer equivalence.
                for (kind, actual, expected, word) in [
                    (
                        SimulationNetworkContactPreparedBindingKind::TableBodyA,
                        entry.table_a,
                        entry.rf_a,
                        entry.input.position_a_body_a.word,
                    ),
                    (
                        SimulationNetworkContactPreparedBindingKind::TableBodyB,
                        entry.table_b,
                        self.source.static_body,
                        entry.input.position_b_body_b.word,
                    ),
                    (
                        SimulationNetworkContactPreparedBindingKind::SolverSide0,
                        entry.rf_a,
                        entry.table_a,
                        entry.input.side_0_com_rf_token.word,
                    ),
                    (
                        SimulationNetworkContactPreparedBindingKind::SolverSide1,
                        entry.rf_b,
                        self.source.static_body,
                        entry.input.side_1_com_rf_token.word,
                    ),
                ] {
                    if actual != expected {
                        return Err(SimulationContactAppendError::RoleMismatch {
                            contribution_ordinal,
                            kind,
                        });
                    }
                    phase
                        .try_validate_contact_observation(actual)
                        .map_err(|source| SimulationContactAppendError::Body {
                            contribution_ordinal,
                            kind,
                            source,
                        })?;
                    let slot = actual.identity().id().index();
                    let expected = slot.checked_add(1).ok_or(
                        SimulationContactAppendError::BindingTokenOverflow {
                            contribution_ordinal,
                            kind,
                            slot,
                        },
                    )?;
                    if word != expected {
                        return Err(SimulationContactAppendError::BindingTokenMismatch {
                            contribution_ordinal,
                            kind,
                            expected,
                            actual: word,
                        });
                    }
                }
            }
            let mut dense_ordinal =
                u32::try_from(current).expect("preflight bounds dense base by u32 maximum");
            for entry in records {
                self.domain.records.push(SimulationOwnedContactEntry {
                    dense_ordinal,
                    producer: SimulationContactProducerSource::AssemblyGround {
                        source: entry.source,
                        volume_body: entry.table_a,
                    },
                    input: entry.input,
                    roles: SimulationPreparedNetworkContactRoles {
                        table_body_a: entry.table_a,
                        table_body_b: entry.table_b,
                        solver_side_0: entry.rf_a,
                        solver_side_1: entry.rf_b,
                    },
                });
                dense_ordinal = dense_ordinal.wrapping_add(1);
            }
            Ok(())
        }

        /// Two-body CPU pipeline records: the same whole-prefix preflight,
        /// with B bound to each record's actual assembly body.
        pub(super) fn try_append_body_contact_records(
            &mut self,
            records: &[super::ground_preparation::SimulationGroundContactRecord],
            phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
        ) -> Result<(), SimulationContactAppendError> {
            try_validate_contact_destination(self, phase)?;
            let current = self.domain.records.len();
            let incoming = records.len();
            let maximum = self.domain.maximum;
            if incoming > (maximum as usize).saturating_sub(current) {
                return Err(SimulationContactAppendError::CapacityExceeded {
                    current,
                    incoming,
                    maximum,
                });
            }
            for (contribution_ordinal, entry) in records.iter().enumerate() {
                if entry.pool != self.source.body_pool {
                    return Err(SimulationContactAppendError::ContributionBodyPoolMismatch);
                }
                if entry.frame != self.source.frame {
                    return Err(SimulationContactAppendError::ContributionFrameMismatch);
                }
                if entry.phase != self.source.phase {
                    return Err(SimulationContactAppendError::ContributionPhaseMismatch);
                }
                for (kind, actual, expected, word) in [
                    (
                        SimulationNetworkContactPreparedBindingKind::TableBodyA,
                        entry.table_a,
                        entry.rf_a,
                        entry.input.position_a_body_a.word,
                    ),
                    (
                        SimulationNetworkContactPreparedBindingKind::TableBodyB,
                        entry.table_b,
                        entry.rf_b,
                        entry.input.position_b_body_b.word,
                    ),
                    (
                        SimulationNetworkContactPreparedBindingKind::SolverSide0,
                        entry.rf_a,
                        entry.table_a,
                        entry.input.side_0_com_rf_token.word,
                    ),
                    (
                        SimulationNetworkContactPreparedBindingKind::SolverSide1,
                        entry.rf_b,
                        entry.table_b,
                        entry.input.side_1_com_rf_token.word,
                    ),
                ] {
                    if actual != expected {
                        return Err(SimulationContactAppendError::RoleMismatch {
                            contribution_ordinal,
                            kind,
                        });
                    }
                    phase
                        .try_validate_contact_observation(actual)
                        .map_err(|source| SimulationContactAppendError::Body {
                            contribution_ordinal,
                            kind,
                            source,
                        })?;
                    let slot = actual.identity().id().index();
                    let expected = slot.checked_add(1).ok_or(
                        SimulationContactAppendError::BindingTokenOverflow {
                            contribution_ordinal,
                            kind,
                            slot,
                        },
                    )?;
                    if word != expected {
                        return Err(SimulationContactAppendError::BindingTokenMismatch {
                            contribution_ordinal,
                            kind,
                            expected,
                            actual: word,
                        });
                    }
                }
            }
            let mut dense_ordinal =
                u32::try_from(current).expect("preflight bounds dense base by u32 maximum");
            for entry in records {
                self.domain.records.push(SimulationOwnedContactEntry {
                    dense_ordinal,
                    producer: SimulationContactProducerSource::AssemblyBody {
                        source: entry.source,
                        volume_body: entry.table_a,
                        other_body: entry.table_b,
                    },
                    input: entry.input,
                    roles: SimulationPreparedNetworkContactRoles {
                        table_body_a: entry.table_a,
                        table_body_b: entry.table_b,
                        solver_side_0: entry.rf_a,
                        solver_side_1: entry.rf_b,
                    },
                });
                dense_ordinal = dense_ordinal.wrapping_add(1);
            }
            Ok(())
        }

        /// Appends one fully preflighted contribution in explicit host call order.
        ///
        /// Every typed refusal returns the exact contribution and leaves the
        /// persistent destination unchanged. No allocation occurs after the
        /// preflight because the domain reserved its global maximum at creation.
        fn try_append_network_contact_contribution(
            &mut self,
            contribution: SimulationPreparedNetworkContactContribution,
            phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
        ) -> Result<(), SimulationContactAppendFailure> {
            if let Err(error) = try_validate_contact_contribution(self, &contribution, phase) {
                return Err(SimulationContactAppendFailure {
                    contribution,
                    error,
                });
            }

            let base = self.domain.records.len();
            let mut dense_ordinal =
                u32::try_from(base).expect("preflight bounds the dense base by the u32 maximum");
            let queue = contribution.source.queue;
            for entry in contribution.entries {
                self.domain.records.push(SimulationOwnedContactEntry {
                    dense_ordinal,
                    producer: SimulationContactProducerSource::Network {
                        queue,
                        queue_ordinal: entry.queue_ordinal,
                        selected_body: entry.selected_body,
                    },
                    input: entry.input,
                    roles: entry.roles,
                });
                dense_ordinal = dense_ordinal.wrapping_add(1);
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::{
            simulation_contact_network_queue::{
                SimulationNetworkContactMaterial, SimulationResolvedNetworkContactRequest,
            },
            simulation_drive_jacobian::{
                SimulationPreparedDriveIslandHorusError, prepare_drive_horus_island_batch_for_test,
                prepared_drive_horus_batch_words_for_test,
                try_solve_prepared_drive_horus_island_sweep,
            },
            simulation_drive_pool::{
                SimulationDriveDynamics, SimulationDriveDynamicsParams, SimulationDriveFrames,
                SimulationDrivePool,
            },
            simulation_horus_pipeline::{
                SimulationPreparedHorusIslandError, SimulationPreparedHorusSerialFrameError,
                try_solve_prepared_horus_island,
                try_solve_prepared_horus_islands_serial_and_integrate_ordinary_finite,
            },
            simulation_joint_pool::{
                SimulationJointFrames, SimulationJointLimits, SimulationJointPool,
            },
            simulation_joint_preparation::{
                prepare_joint_horus_island_batch_for_test,
                prepared_joint_horus_batch_words_for_test,
                try_solve_prepared_joint_horus_island_sweep,
            },
            simulation_rigid_body_pool::{
                SimulationInertia, SimulationRigidBodyHandle, SimulationRigidBodyList,
                SimulationRigidBodyPool,
            },
        };
        use std::sync::{Arc, RwLock};

        struct TraceBackend {
            calls: Vec<[u32; 4]>,
            fail_on: Option<usize>,
        }

        impl TraceBackend {
            fn passing() -> Self {
                Self {
                    calls: Vec::new(),
                    fail_on: None,
                }
            }
        }

        impl SimulationNetworkContactDrainNumericBackend for TraceBackend {
            type Error = &'static str;

            fn reciprocal_sqrt_estimate(
                &mut self,
                _site: super::super::SimulationNetworkContactDrainEstimateSite,
                operand: [f32; 4],
            ) -> Result<[f32; 4], Self::Error> {
                let ordinal = self.calls.len();
                self.calls.push(operand.map(f32::to_bits));
                if self.fail_on == Some(ordinal) {
                    Err("injected network preparation estimate failure")
                } else {
                    Ok([1.0; 4])
                }
            }
        }

        struct ContactCompileTraceBackend {
            calls: Vec<[u32; 4]>,
            fail_on: Option<usize>,
        }

        impl ContactCompileTraceBackend {
            fn passing() -> Self {
                Self {
                    calls: Vec::new(),
                    fail_on: None,
                }
            }
        }

        impl SimulationContactNumericBackend for ContactCompileTraceBackend {
            type Error = &'static str;

            fn reciprocal_estimate(
                &mut self,
                _site: super::super::SimulationContactReciprocalSite,
                denominator: [f32; 4],
            ) -> Result<[f32; 4], Self::Error> {
                let ordinal = self.calls.len();
                self.calls.push(denominator.map(f32::to_bits));
                if self.fail_on == Some(ordinal) {
                    Err("injected Contact compiler estimate failure")
                } else {
                    Ok([1.0; 4])
                }
            }
        }

        fn matrix_at(position: [f32; 3]) -> [f32; 16] {
            [
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
                position[0],
                position[1],
                position[2],
                1.0,
            ]
        }

        fn inertia() -> Arc<SimulationInertia> {
            Arc::new(SimulationInertia {
                inverse_tensor: [2.0, 3.0, 4.0],
                inverse_mass: 0.5,
                spherical: 1.0,
                max_velocity: f32::MAX,
                max_omega: f32::MAX,
                linear_drag: 0.0,
                angular_drag: 0.0,
            })
        }

        fn add_body(
            pool: &mut SimulationRigidBodyPool,
            position: [f32; 3],
            inertia: Option<&Arc<SimulationInertia>>,
            state: u32,
        ) -> SimulationRigidBodyHandle {
            pool.try_add_rigid_body(matrix_at(position), inertia, state)
                .expect("body add")
                .expect("body capacity")
        }

        fn request(
            selected_body: SimulationRigidBodyObservation,
            point_x: f32,
            material_base: f32,
        ) -> SimulationResolvedNetworkContactRequest {
            SimulationResolvedNetworkContactRequest {
                selected_body,
                point: [point_x, 0.0, 0.0, f32::from_bits(0x7fc0_0041)],
                normal: [0.0, 0.0, 1.0, f32::from_bits(0xffc0_0042)],
                displacement: 0.0,
                material: SimulationNetworkContactMaterial {
                    static_friction: material_base,
                    dynamic_friction: material_base + 0.125,
                    restitution: material_base + 0.25,
                },
            }
        }

        #[derive(Clone, Copy)]
        enum SpyReceiptFixture {
            Matching,
            WrongPhase,
            WrongBatch,
        }

        struct CompletedSpyFixture {
            pool: SimulationRigidBodyPool,
            manager: SimulationManagerContactDomain,
            bodies: [SimulationRigidBodyHandle; 4],
            result: Result<
                super::super::contact_spy::SimulationContactSpyFrame,
                super::super::contact_spy::SimulationContactSpyError,
            >,
        }

        /// Controlled real-pool/RF inputs, not captured game tuning or a native
        /// Island-registration implementation. Every exported record goes through
        /// the actual queue, compiler, Horus pass and `DynamicUpdate` consumer.
        #[allow(
            clippy::too_many_lines,
            reason = "one retained fixture proves the complete contact-to-spy ownership path"
        )]
        fn completed_spy_fixture(flags: u32, receipt: SpyReceiptFixture) -> CompletedSpyFixture {
            use crate::simulation_horus_pipeline::try_complete_prepared_horus_islands_serial;

            let mut pool = SimulationRigidBodyPool::try_new(5).expect("pool");
            let mut manager =
                SimulationManagerContactDomain::try_new(&mut pool, 4).expect("manager");
            let retained = inertia();
            let bodies = [0.0, 2.0, 4.0, 6.0]
                .map(|x| add_body(&mut pool, [x, 0.0, 0.0], Some(&retained), 4));
            for index in [0, 1, 3] {
                pool.try_register_contact_spy(bodies[index])
                    .expect("register");
            }
            let joint_pool = SimulationJointPool::try_new(0).expect("joints");
            let drive_pool = SimulationDrivePool::try_new(0, &pool).expect("drives");
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            pool.set_gravity([0.0; 3]);
            for index in [0, 2, 3] {
                pool.try_with_solver_delta_row_mut(bodies[index].observation(), |rows| {
                    rows[0] = [0.125, 0.0, 1.0, 0.0];
                })
                .expect("controlled incoming RF");
            }
            let old_phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("prior phase")
                .phase_identity();
            let map = SimulationRigidBodyIslandMapSnapshot::try_new(
                &pool,
                pool.frame_identity().expect("frame"),
                std::iter::once((manager.static_body_observation(), -1))
                    .chain(bodies.iter().map(|body| (body.observation(), 0))),
            )
            .expect("map");
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for body in bodies {
                queue
                    .try_enqueue(&pool, request(body.observation(), 0.0, 0.5))
                    .expect("request");
            }
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            assert!(old_phase != phase.phase_identity());
            let mut destination = manager.try_begin_contact_frame(&phase).expect("frame");
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut TraceBackend::passing(),
            )
            .unwrap_or_else(|_| panic!("real network preparation"));
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("append"));
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|_| panic!("seal"));
            let mut batch = try_prepare_network_contact_island_batch_with_backend(
                sealed,
                &phase,
                SimulationContactIslandMapSource::Present(&map),
                &mut super::super::HostContactNumericBackend,
            )
            .unwrap_or_else(|_| panic!("actual Contact compiler"));
            let old_centers: Vec<_> = batch
                .records
                .iter()
                .map(|entry| {
                    (
                        phase
                            .try_network_contact_snapshot(entry.table_body_a)
                            .expect("A")
                            .center_of_mass,
                        phase
                            .try_network_contact_snapshot(entry.table_body_b)
                            .expect("B")
                            .center_of_mass,
                    )
                })
                .collect();
            let mut joints = prepare_joint_horus_island_batch_for_test(&joint_pool, &phase, &map);
            let mut drives = prepare_drive_horus_island_batch_for_test(&drive_pool, &phase, &map);
            let completed = try_complete_prepared_horus_islands_serial(
                &mut batch,
                &mut joints,
                &mut drives,
                1,
                2,
                &drive_pool,
                phase,
            )
            .expect("actual two Horus passes and integration");
            assert_eq!(batch.record_count(), 4);
            for index in [0, 2, 3] {
                assert!(batch.records[index].record.position_impulse[0] > 0.0);
            }
            assert_eq!(batch.records[1].record.position_impulse[0].to_bits(), 0);
            assert_eq!(batch.records[2].record.nb_word & 8, 0);

            // Independent expected XYZ evaluation from the actual solved record;
            // do not use the exporter kernel as its own expected-value function.
            let frequency = completed.body_pool().frequency();
            let frequency_squared = frequency * frequency;
            let mut expected = Vec::new();
            for index in [0, 3] {
                let entry = &batch.records[index];
                let solved = &entry.record;
                let center_b = completed
                    .body_pool()
                    .try_contact_spy_center_of_mass(entry.table_body_b)
                    .expect("live B");
                let center_a = completed
                    .body_pool()
                    .try_contact_spy_center_of_mass(entry.table_body_a)
                    .expect("live A");
                assert_ne!(
                    center_b.map(f32::to_bits),
                    old_centers[index].1.map(f32::to_bits)
                );
                let position = |a: [f32; 3], b: [f32; 3]| -> [u32; 3] {
                    std::array::from_fn(|lane| {
                        let weighted_b =
                            (b[lane] + solved.attachment_offset_b[lane]) * solved.inverse_mass_b;
                        let weighted_sum = (a[lane] + solved.attachment_offset_a[lane])
                            .mul_add(solved.inverse_mass_a, weighted_b);
                        (weighted_sum * (1.0 / (solved.inverse_mass_b + solved.inverse_mass_a)))
                            .to_bits()
                    })
                };
                let position_words = position(center_a, center_b);
                assert_ne!(
                    position_words,
                    position(old_centers[index].0, old_centers[index].1)
                );
                let frame = [
                    solved.contact_right.map(f32::to_bits),
                    solved.contact_up.map(f32::to_bits),
                    solved.contact_at.map(f32::to_bits),
                    position_words,
                ];
                let normal = solved.contact_right.map(|axis| {
                    ((axis * solved.position_impulse[0]) * frequency_squared).to_bits()
                });
                let friction: [u32; 3] = std::array::from_fn(|lane| {
                    let tangent_z = solved.contact_at[lane] * solved.position_impulse[2];
                    (solved.contact_up[lane].mul_add(solved.position_impulse[1], tangent_z)
                        * frequency_squared)
                        .to_bits()
                });
                expected.push((
                    entry.contact,
                    entry.table_body_a,
                    entry.table_body_b,
                    frame,
                    normal,
                    friction,
                    solved.tag_word,
                ));
            }
            // Only refusal fixtures alter private provenance. No Jacobian or
            // solver output is fabricated to produce the positive path.
            match receipt {
                SpyReceiptFixture::Matching => {}
                SpyReceiptFixture::WrongPhase => {
                    batch.frame.source.phase = old_phase;
                }
                SpyReceiptFixture::WrongBatch => {
                    batch.identity = SimulationContactBatchIdentity(Arc::new(()));
                }
            }
            let result = batch.try_export_contact_spies(completed, flags);
            if let Ok(frame) = &result {
                frame
                    .try_validate_current(&manager, &pool)
                    .expect("current publication");
                if flags & 4 != 0 {
                    assert_eq!(frame.records().len(), expected.len());
                    for (record, (contact, a, b, axes, normal, friction, tag)) in
                        frame.records().iter().zip(expected)
                    {
                        assert_eq!(record.contact(), contact);
                        assert_eq!(record.body_a(), a);
                        assert_eq!(record.body_b(), b);
                        assert_eq!(record.frame().map(|axis| axis.map(f32::to_bits)), axes);
                        assert_eq!(record.normal_force().map(f32::to_bits), normal);
                        assert_eq!(record.friction_force().map(f32::to_bits), friction);
                        assert_eq!(record.tag(), tag);
                    }
                } else {
                    assert!(frame.records().is_empty());
                }
            }
            CompletedSpyFixture {
                pool,
                manager,
                bodies,
                result,
            }
        }

        #[test]
        fn contact_spy_real_horus_compacts_selected_records_and_reads_integrated_com() {
            let fixture = completed_spy_fixture(4, SpyReceiptFixture::Matching);
            let frame = fixture.result.expect("export");
            assert_eq!(frame.records().len(), 2);
            assert_eq!(frame.records()[0].body_b(), fixture.bodies[0].observation());
            assert_eq!(frame.records()[1].body_b(), fixture.bodies[3].observation());
            for record in frame.records() {
                assert_eq!(record.body_a(), fixture.manager.static_body_observation());
            }
        }

        #[test]
        fn contact_spy_feedback_freeze_preserves_positive_records_and_endpoint_validation() {
            let CompletedSpyFixture {
                mut pool,
                manager,
                bodies,
                result,
            } = completed_spy_fixture(4, SpyReceiptFixture::Matching);
            let frame = result.expect("actual solved export");
            assert_eq!(frame.records().len(), 2);
            let retained = frame.records().to_vec();
            let before = pool.snapshot();
            pool.try_reset_static_reaction_forces().unwrap();
            // Controlled threshold selects all real Active endpoints; this is
            // not a trace of the shipped manager threshold or sleeping motion.
            assert_eq!(pool.try_freeze_if_necessary(0).unwrap(), bodies.len());
            assert!(pool.snapshot().active_order.is_empty());
            for body in bodies {
                let index = body.id().index() as usize;
                assert_eq!(
                    pool.snapshot().slots[index].state,
                    (before.slots[index].state & 8) | 2
                );
            }
            frame.try_validate_current(&manager, &pool).unwrap();
            assert_eq!(frame.records(), retained);
        }

        #[test]
        fn contact_spy_disabled_flag_skips_positive_registered_solved_contacts() {
            for flags in [0, 1, 2, 8] {
                let fixture = completed_spy_fixture(flags, SpyReceiptFixture::Matching);
                assert!(
                    fixture
                        .result
                        .expect("disabled export")
                        .records()
                        .is_empty()
                );
            }
        }

        #[test]
        fn contact_spy_completed_receipt_rejects_different_phase_or_batch() {
            use super::super::contact_spy::SimulationContactSpyError;
            for (mode, expected) in [
                (
                    SpyReceiptFixture::WrongPhase,
                    SimulationContactSpyError::PhaseMismatch,
                ),
                (
                    SpyReceiptFixture::WrongBatch,
                    SimulationContactSpyError::BatchMismatch,
                ),
            ] {
                assert_eq!(
                    completed_spy_fixture(4, mode)
                        .result
                        .expect_err("provenance refusal"),
                    expected
                );
            }
        }

        #[test]
        fn contact_spy_publication_rejects_stale_frame_foreign_manager_and_reused_body() {
            use super::super::contact_spy::SimulationContactSpyError;
            let CompletedSpyFixture {
                mut pool,
                manager,
                bodies,
                result,
            } = completed_spy_fixture(4, SpyReceiptFixture::Matching);
            let frame = result.expect("export");
            let mut foreign = SimulationRigidBodyPool::try_new(1).expect("foreign pool");
            let foreign_manager =
                SimulationManagerContactDomain::try_new(&mut foreign, 0).expect("foreign manager");
            assert_eq!(
                frame.try_validate_current(&foreign_manager, &pool),
                Err(SimulationContactSpyError::ManagerMismatch)
            );
            assert_eq!(
                frame.try_validate_current(&manager, &foreign),
                Err(SimulationContactSpyError::BodyPoolMismatch)
            );
            pool.try_remove_rigid_body(bodies[0])
                .expect("remove actual occurrence");
            let replacement = add_body(&mut pool, [0.0; 3], Some(&inertia()), 4);
            assert_eq!(
                replacement.observation().identity().id(),
                bodies[0].observation().identity().id()
            );
            assert_ne!(replacement.observation(), bodies[0].observation());
            assert!(matches!(
                frame.try_validate_current(&manager, &pool),
                Err(SimulationContactSpyError::Body {
                    side: SimulationContactTableEndpoint::BodyB,
                    ..
                })
            ));
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            assert_eq!(
                frame.try_validate_current(&manager, &pool),
                Err(SimulationContactSpyError::FrameMismatch)
            );
        }

        #[test]
        fn contact_spy_empty_fast_paths_still_require_the_exact_completed_batch() {
            use super::super::contact_spy::SimulationContactSpyError;
            use crate::simulation_horus_pipeline::try_complete_prepared_horus_islands_serial;

            for active in [false, true] {
                for corrupt_identity in [false, true] {
                    let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
                    let mut manager =
                        SimulationManagerContactDomain::try_new(&mut pool, 0).expect("manager");
                    let mut labels = vec![(manager.static_body_observation(), -1)];
                    if active {
                        let body = add_body(&mut pool, [0.0; 3], Some(&inertia()), 4);
                        labels.push((body.observation(), 0));
                    }
                    let joint_pool = SimulationJointPool::try_new(0).expect("joints");
                    let drive_pool = SimulationDrivePool::try_new(0, &pool).expect("drives");
                    pool.begin_dynamic_update_frame(1.0 / 60.0);
                    let map = SimulationRigidBodyIslandMapSnapshot::try_new(
                        &pool,
                        pool.frame_identity().expect("frame"),
                        labels,
                    )
                    .expect("map");
                    let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
                    let mut batch = manager
                        .try_prepare_empty_contact_islands_host(&phase, &map)
                        .expect("empty real backing");
                    let mut joints =
                        prepare_joint_horus_island_batch_for_test(&joint_pool, &phase, &map);
                    let mut drives =
                        prepare_drive_horus_island_batch_for_test(&drive_pool, &phase, &map);
                    // No Active body skips scheduling even with a nonzero pass
                    // count; an Active body uses the distinct zero-pass branch.
                    let completed = try_complete_prepared_horus_islands_serial(
                        &mut batch,
                        &mut joints,
                        &mut drives,
                        1,
                        if active { 0 } else { 3 },
                        &drive_pool,
                        phase,
                    )
                    .expect("fast completion");
                    assert_eq!(completed.had_active_bodies(), active);
                    if corrupt_identity {
                        batch.identity = SimulationContactBatchIdentity(Arc::new(()));
                    }
                    let result = batch.try_export_contact_spies(completed, 4);
                    if corrupt_identity {
                        assert_eq!(
                            result.expect_err("identity before empty/Active gate"),
                            SimulationContactSpyError::BatchMismatch
                        );
                    } else {
                        let frame = result.expect("empty frame");
                        assert!(frame.records().is_empty());
                        frame
                            .try_validate_current(&manager, &pool)
                            .expect("actual frame provenance");
                    }
                }
            }
        }

        #[test]
        fn host_contact_reciprocal_policy_is_unmodified_host_division() {
            let actual = super::super::HostContactNumericBackend
                .reciprocal_estimate(
                    super::super::SimulationContactReciprocalSite::EffectiveMassFourLane,
                    [2.0, -4.0, 0.0, -0.0],
                )
                .expect("infallible host reciprocal");
            assert_eq!(
                actual.map(f32::to_bits),
                [0.5, -0.25, f32::INFINITY, f32::NEG_INFINITY].map(f32::to_bits)
            );
        }

        fn ground_source_fixture() -> super::super::ground_preparation::SimulationGroundContactSource
        {
            use crate::simulation_assembly_ground_triangle_topology::build_old_assembly_unit_gp;
            use skate2_formats::collision::{
                ClusterVertex, ClusteredMesh, MeshCluster, MeshUnit, UnitType, VertexCompression,
            };
            let mesh = ClusteredMesh {
                bounding_box_min: [0.0; 4],
                bounding_box_max: [1.0; 4],
                aggregate_vtable_offset: 0,
                tag_bit_count: 0,
                volume_count: 1,
                aggregate_padding: 0,
                kd_tree_offset: 0,
                cluster_data_offset: 0,
                vertex_compression_granularity: 1.0,
                cluster_flags: 0,
                group_id_size: 2,
                surface_id_size: 2,
                cluster_count: 1,
                maximum_cluster_count: 1,
                unit_count: 1,
                maximum_unit_count: 1,
                allocation_size: 0,
                default_group_id: 0,
                default_surface_id: 0,
                default_edge_angle: 0,
            };
            let cluster = MeshCluster {
                unit_count: 1,
                unit_data_size: 0,
                unit_data_start: 0,
                normal_start: 0,
                total_size: 0,
                vertex_count: 3,
                normal_count: 0,
                compression: VertexCompression::Uncompressed,
                vertices: [
                    [0.0, 0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                ]
                .map(ClusterVertex::Uncompressed)
                .to_vec(),
                normals: Vec::new(),
                unit_data: Vec::new(),
            };
            let unit = MeshUnit {
                stream_offset: 0x34,
                unit_type: UnitType::Triangle,
                flags: 1,
                vertex_indices: vec![0, 1, 2],
                edge_data: Vec::new(),
                group_id: Some(0x12),
                surface_id: Some(0x34),
            };
            let batch =
                build_old_assembly_unit_gp(&mesh, 0, &cluster, &unit, matrix_at([0.0; 3]), 0)
                    .expect("source triangle");
            super::super::ground_preparation::SimulationGroundContactSource {
                assembly_ordinal: 7,
                volume_ordinal: 2,
                target: batch.iter().next().expect("triangle").source().into(),
                point_ordinal: 0,
            }
        }

        fn ground_record_fixture(
            phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
            volume: SimulationRigidBodyObservation,
            ground: SimulationRigidBodyObservation,
        ) -> super::super::ground_preparation::SimulationGroundContactRecord {
            use super::super::{
                SimulationAssemblyGroundContactEstimateSite, SimulationAssemblyGroundContactInput,
                SimulationAssemblyGroundContactNumericBackend,
                SimulationAssemblyGroundMaterialInput, build_assembly_ground_contact_input,
            };
            struct HostGroundNumeric;
            impl SimulationAssemblyGroundContactNumericBackend for HostGroundNumeric {
                type Error = std::convert::Infallible;
                fn reciprocal_sqrt_estimate(
                    &mut self,
                    _: SimulationAssemblyGroundContactEstimateSite,
                    value: [f32; 4],
                ) -> Result<[f32; 4], Self::Error> {
                    Ok(value.map(|lane| lane.sqrt().recip()))
                }
            }
            let volume_word = volume.identity().id().index() + 1;
            let ground_word = ground.identity().id().index() + 1;
            let material = SimulationAssemblyGroundMaterialInput {
                static_friction: 0.5,
                dynamic_friction: 0.25,
                restitution: 0.0,
            };
            let input = build_assembly_ground_contact_input(
                &SimulationAssemblyGroundContactInput {
                    body_a: network_contact_common_body(
                        phase
                            .try_network_contact_snapshot(volume)
                            .expect("volume snapshot"),
                        SolverDeltaRowToken(volume_word),
                    ),
                    body_b: network_contact_common_body(
                        phase
                            .try_network_contact_snapshot(ground)
                            .expect("ground snapshot"),
                        SolverDeltaRowToken(ground_word),
                    ),
                    position_a: [0.0, 0.0, -0.25],
                    position_b: [0.0; 3],
                    normal: [0.0, 0.0, 1.0],
                    table_body_a: ContactBodyToken(volume_word),
                    table_body_b: ContactBodyToken(ground_word),
                    volume_material: material,
                    ground_material: material,
                    volume_surface_id: 0x56,
                    triangle_user_tag: 0x0034_0012,
                },
                &mut HostGroundNumeric,
            )
            .expect("ground compositor");
            super::super::ground_preparation::SimulationGroundContactRecord {
                input,
                table_a: volume,
                table_b: ground,
                rf_a: volume,
                rf_b: ground,
                source: ground_source_fixture(),
                frame: phase.frame_identity(),
                phase: phase.phase_identity(),
                pool: phase.body_pool_authority(),
            }
        }

        #[test]
        #[allow(
            clippy::too_many_lines,
            reason = "one ground record follows the shared C/J/D consumer with independent RF oracle"
        )]
        fn ground_contact_import_preserves_cpu_sides_and_reaches_existing_horus() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 1).expect("domain");
            let volume = add_body(&mut pool, [0.0; 3], Some(&inertia()), 4);
            let ground = contacts.static_body_observation();
            let joint_pool = SimulationJointPool::try_new(0).expect("joint pool");
            let drive_pool = SimulationDrivePool::try_new(0, &pool).expect("drive pool");
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let map = SimulationRigidBodyIslandMapSnapshot::try_new(
                &pool,
                pool.frame_identity().expect("frame"),
                [(volume.observation(), 2), (ground, -1)],
            )
            .expect("island map");
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let record = ground_record_fixture(&phase, volume.observation(), ground);
            let mut destination = contacts.try_begin_contact_frame(&phase).expect("frame");
            let backing = destination.domain.records.as_ptr();
            destination
                .try_append_ground_contact_records(&[record], &phase)
                .expect("ground import");
            assert_eq!(destination.domain.records.as_ptr(), backing);
            assert_eq!(
                destination.domain.records[0].producer,
                SimulationContactProducerSource::AssemblyGround {
                    source: record.source,
                    volume_body: volume.observation(),
                }
            );
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|_| panic!("seal"));
            let mut batch = try_prepare_network_contact_island_batch_with_backend(
                sealed,
                &phase,
                SimulationContactIslandMapSource::Present(&map),
                &mut ContactCompileTraceBackend::passing(),
            )
            .unwrap_or_else(|_| panic!("compile ground"));
            assert_eq!(batch.islands[2].len(), 1);
            assert_eq!(batch.records[0].table_body_a, volume.observation());
            assert_eq!(batch.records[0].table_body_b, ground);
            assert_eq!(batch.records[0].solver_side_0, volume.observation());
            assert_eq!(batch.records[0].solver_side_1, ground);
            let mut expected_record = batch.records[0].record;
            let expected =
                solve_contact_horus_record_host(&mut expected_record, [[0.0; 4]; 4], [[0.0; 4]; 4]);
            let mut joint_batch =
                prepare_joint_horus_island_batch_for_test(&joint_pool, &phase, &map);
            let mut drive_batch =
                prepare_drive_horus_island_batch_for_test(&drive_pool, &phase, &map);
            try_solve_prepared_horus_island(
                &mut batch,
                &mut joint_batch,
                &mut drive_batch,
                2,
                1,
                &drive_pool,
                &mut phase,
            )
            .expect("ground C/J/D pass");
            assert_eq!(
                contact_record_words(&batch.records[0].record),
                contact_record_words(&expected_record)
            );
            // A prepared RF token is not authorized merely because its table
            // endpoint is valid; corruption must refuse before further effects.
            batch.records[0].record.solver_delta_a =
                SolverDeltaRowToken(ground.identity().id().index() + 1);
            assert!(matches!(
                try_solve_prepared_network_contact_horus_island_sweep(&mut batch, 2, &mut phase),
                Err(
                    SimulationPreparedNetworkContactIslandHorusError::BindingTokenMismatch {
                        kind: SimulationNetworkContactPreparedBindingKind::SolverSide0,
                        ..
                    }
                )
            ));
            drop(phase);
            let actual_a = pool
                .try_with_solver_delta_row(volume.observation(), |rows| *rows)
                .expect("volume RF");
            let actual_b = pool
                .try_with_solver_delta_row(ground, |rows| *rows)
                .expect("ground RF");
            assert_eq!(
                solver_delta_words(actual_a),
                solver_delta_words(expected.solver_delta_a)
            );
            assert_eq!(
                solver_delta_words(actual_b),
                solver_delta_words(expected.solver_delta_b)
            );
        }

        #[test]
        fn ground_contact_import_checks_whole_prefix_roles_tokens_and_capacity() {
            let mut pool = SimulationRigidBodyPool::try_new(3).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 3).expect("domain");
            assert_eq!(contacts.maximum(), 3);
            let volume = add_body(&mut pool, [0.0; 3], Some(&inertia()), 4);
            let other = add_body(&mut pool, [1.0; 3], Some(&inertia()), 4);
            let ground = contacts.static_body_observation();
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let record = ground_record_fixture(&phase, volume.observation(), ground);
            let mut destination = contacts.try_begin_contact_frame(&phase).expect("frame");
            assert_eq!(destination.maximum(), 3);
            assert_eq!(destination.static_body_observation(), ground);
            destination
                .try_append_ground_contact_records(&[record], &phase)
                .expect("first prefix");
            for kind in [
                SimulationNetworkContactPreparedBindingKind::TableBodyA,
                SimulationNetworkContactPreparedBindingKind::TableBodyB,
                SimulationNetworkContactPreparedBindingKind::SolverSide0,
                SimulationNetworkContactPreparedBindingKind::SolverSide1,
            ] {
                let mut bad = record;
                match kind {
                    SimulationNetworkContactPreparedBindingKind::TableBodyA => {
                        bad.table_a = other.observation();
                    }
                    SimulationNetworkContactPreparedBindingKind::TableBodyB => {
                        bad.table_b = other.observation();
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide0 => {
                        bad.rf_a = other.observation();
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide1 => {
                        bad.rf_b = other.observation();
                    }
                }
                assert!(matches!(
                    destination.try_append_ground_contact_records(&[record, bad], &phase),
                    Err(SimulationContactAppendError::RoleMismatch {
                        contribution_ordinal: 1,
                        ..
                    })
                ));
                assert_eq!(destination.len(), 1);
                let mut bad = record;
                match kind {
                    SimulationNetworkContactPreparedBindingKind::TableBodyA => {
                        bad.input.position_a_body_a.word = 0;
                    }
                    SimulationNetworkContactPreparedBindingKind::TableBodyB => {
                        bad.input.position_b_body_b.word = 0;
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide0 => {
                        bad.input.side_0_com_rf_token.word = 0;
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide1 => {
                        bad.input.side_1_com_rf_token.word = 0;
                    }
                }
                assert!(
                    matches!(destination.try_append_ground_contact_records(&[record, bad], &phase),
                    Err(SimulationContactAppendError::BindingTokenMismatch { contribution_ordinal: 1, kind: actual, .. }) if actual == kind)
                );
                assert_eq!(destination.len(), 1);
            }
            assert_eq!(
                destination.try_append_ground_contact_records(&[record; 3], &phase),
                Err(SimulationContactAppendError::CapacityExceeded {
                    current: 1,
                    incoming: 3,
                    maximum: 3
                })
            );
            assert_eq!(destination.len(), 1);
            destination
                .try_append_ground_contact_records(&[record; 2], &phase)
                .expect("exact accepted prefix");
            assert_eq!(destination.len(), 3);
        }

        #[test]
        fn ground_contact_numeric_rows_cannot_migrate_between_phases_frames_or_pools() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 1).expect("domain");
            let volume = add_body(&mut pool, [0.0; 3], Some(&inertia()), 4);
            let ground = contacts.static_body_observation();
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let first_phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("first phase");
            let record = ground_record_fixture(&first_phase, volume.observation(), ground);
            drop(first_phase);
            let replacement_phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("replacement phase");
            let mut destination = contacts
                .try_begin_contact_frame(&replacement_phase)
                .expect("frame");
            assert_eq!(
                destination.try_append_ground_contact_records(&[record], &replacement_phase),
                Err(SimulationContactAppendError::ContributionPhaseMismatch)
            );
            assert_eq!(destination.len(), 0);
            drop(destination);
            drop(replacement_phase);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let later_phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("later phase");
            let mut destination = contacts
                .try_begin_contact_frame(&later_phase)
                .expect("later frame");
            assert_eq!(
                destination.try_append_ground_contact_records(&[record], &later_phase),
                Err(SimulationContactAppendError::ContributionFrameMismatch)
            );
            let mut foreign_pool = SimulationRigidBodyPool::try_new(0).expect("foreign pool");
            foreign_pool.begin_dynamic_update_frame(1.0 / 60.0);
            let foreign_phase = foreign_pool
                .try_prepare_dynamic_update_phase()
                .expect("foreign phase");
            let mut foreign_record = record;
            foreign_record.pool = foreign_phase.body_pool_authority();
            assert_eq!(
                destination.try_append_ground_contact_records(&[foreign_record], &later_phase),
                Err(SimulationContactAppendError::ContributionBodyPoolMismatch)
            );
            assert_eq!(destination.len(), 0);
        }

        fn words3(value: [f32; 3], word: u32) -> [u32; 4] {
            [
                value[0].to_bits(),
                value[1].to_bits(),
                value[2].to_bits(),
                word,
            ]
        }

        fn contact_record_words(value: &SimulationContactJacobian) -> [[u32; 4]; 16] {
            [
                words3(value.attachment_offset_a, value.solver_delta_a.0),
                words3(value.attachment_offset_b, value.solver_delta_b.0),
                words3(value.contact_transpose_right, value.nb_word),
                words3(value.contact_transpose_up, value.static_friction.to_bits()),
                words3(value.contact_transpose_at, value.dynamic_friction.to_bits()),
                words3(value.position_impulse, value.positional_impulse.to_bits()),
                words3(value.position_limit, value.distance.to_bits()),
                words3(value.contact_right, value.body_a.0),
                words3(value.irac_right, value.inverse_mass_a.to_bits()),
                words3(value.irbc_right, value.inverse_mass_b.to_bits()),
                words3(value.contact_up, value.body_b.0),
                words3(value.irac_up, value.pad_0.to_bits()),
                words3(value.irbc_up, value.pad_1.to_bits()),
                words3(value.contact_at, value.tag_word),
                words3(value.irac_at, value.pad_2.to_bits()),
                words3(value.irbc_at, value.pad_3.to_bits()),
            ]
        }

        fn solver_delta_words(value: [[f32; 4]; 4]) -> [[u32; 4]; 4] {
            value.map(|row| row.map(f32::to_bits))
        }

        #[derive(Debug, Eq, PartialEq)]
        struct CombinedHorusBodyEffect {
            quaternion: [u32; 4],
            center_of_mass: [u32; 3],
            velocity: [u32; 3],
            omega: [u32; 3],
            right: [u32; 3],
            up: [u32; 3],
            at: [u32; 3],
            inverse_inertia_full: [u32; 3],
            inverse_mass: u32,
            inverse_inertia_split: [u32; 3],
            state: u32,
            force: [u32; 3],
            kinematic_value: u32,
            torque: [u32; 3],
            cooldown: u32,
        }

        fn combined_horus_body_effects(
            pool: &SimulationRigidBodyPool,
        ) -> Vec<CombinedHorusBodyEffect> {
            pool.snapshot()
                .slots
                .into_iter()
                .map(|slot| CombinedHorusBodyEffect {
                    quaternion: slot.quaternion.map(f32::to_bits),
                    center_of_mass: slot.center_of_mass.map(f32::to_bits),
                    velocity: slot.velocity.map(f32::to_bits),
                    omega: slot.omega.map(f32::to_bits),
                    right: slot.right.map(f32::to_bits),
                    up: slot.up.map(f32::to_bits),
                    at: slot.at.map(f32::to_bits),
                    inverse_inertia_full: slot.inverse_inertia_full.map(f32::to_bits),
                    inverse_mass: slot.inverse_mass.to_bits(),
                    inverse_inertia_split: slot.inverse_inertia_split.map(f32::to_bits),
                    state: slot.state,
                    force: slot.force.map(f32::to_bits),
                    kinematic_value: slot.kinematic_value.to_bits(),
                    torque: slot.torque.map(f32::to_bits),
                    cooldown: slot.cooldown,
                })
                .collect()
        }

        #[derive(Clone, Copy)]
        enum CombinedHorusRun {
            Outer {
                island: usize,
                iterations: u32,
                foreign_drive_pool: bool,
            },
            Manual {
                island: usize,
                iterations: u32,
            },
            GroupedMutant {
                island: usize,
                iterations: u32,
            },
            Serial {
                island_count: usize,
                iterations: u32,
                foreign_drive_pool: bool,
                corrupt_contact_island_7: bool,
                empty_active: bool,
            },
            ManualSerial {
                island_count: usize,
                iterations: u32,
            },
            IntegrateOnly,
        }

        #[derive(Clone, Debug, Eq, PartialEq)]
        enum CombinedHorusError {
            Island(SimulationPreparedHorusIslandError),
            Serial(SimulationPreparedHorusSerialFrameError),
        }

        #[derive(Debug, Eq, PartialEq)]
        struct CombinedHorusEffect {
            contact: Vec<[[u32; 4]; 16]>,
            joint: Vec<[[u32; 4]; 24]>,
            drive: Vec<[[u32; 4]; 24]>,
            rf: Vec<[[u32; 4]; 4]>,
            bodies: Vec<CombinedHorusBodyEffect>,
        }

        #[derive(Debug, Eq, PartialEq)]
        struct CombinedHorusOutcome {
            result: Result<(), CombinedHorusError>,
            before: CombinedHorusEffect,
            after: CombinedHorusEffect,
        }

        #[allow(
            clippy::too_many_lines,
            reason = "one real three-family fixture keeps the outer/manual differential auditable"
        )]
        fn run_combined_horus_fixture(run: CombinedHorusRun) -> CombinedHorusOutcome {
            let mut pool = SimulationRigidBodyPool::try_new(5).expect("body pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2).expect("Contact domain");
            let retained_inertia = inertia();
            let body_state = match run {
                CombinedHorusRun::Serial {
                    empty_active: true, ..
                } => 2,
                _ => 4,
            };
            let island_2_a = add_body(
                &mut pool,
                [1.0, 0.25, -0.5],
                Some(&retained_inertia),
                body_state,
            );
            let island_2_b = add_body(
                &mut pool,
                [1.5, -0.25, 0.75],
                Some(&retained_inertia),
                body_state,
            );
            let island_7_a = add_body(
                &mut pool,
                [3.0, 0.5, -1.0],
                Some(&retained_inertia),
                body_state,
            );
            let island_7_b = add_body(
                &mut pool,
                [3.5, -0.5, 1.25],
                Some(&retained_inertia),
                body_state,
            );
            let static_body = contacts.static_body;
            let bodies = [static_body, island_2_a, island_2_b, island_7_a, island_7_b];

            let joint_frames = Arc::new(SimulationJointFrames {
                quaternion_a: [0.17, -0.23, 0.31, 0.89],
                position_a: [0.41, -0.27, 0.13, 9.0],
                quaternion_b: [-0.19, 0.29, 0.11, 0.91],
                position_b: [-0.37, 0.22, 0.46, -8.0],
                quaternion_l: [0.09, -0.14, 0.21, 0.96],
            });
            let joint_limits = Arc::new(SimulationJointLimits {
                p_prism: [-0.7, -0.5, -0.9, 13.0],
                v_prism: [0.8, 0.65, 1.1, -14.0],
                v_twist: 0.15,
                v_swing: -0.07,
                swing_a: 0.72,
                twist_a: 0.63,
                swing_c: 0.37,
                twist_c: -0.24,
                raw_swing_type: 1,
                raw_twist_type: 1,
            });
            let mut joint_pool = SimulationJointPool::try_new(2).expect("Joint pool");
            for (parent, child) in [(island_2_b, island_2_a), (island_7_b, island_7_a)] {
                joint_pool
                    .try_add_joint(parent, child, &joint_frames, &joint_limits)
                    .expect("Joint add")
                    .expect("Joint capacity");
            }

            let drive_frames = Arc::new(RwLock::new(SimulationDriveFrames {
                quaternion_a: [0.125, -0.25, 0.0625, 0.875],
                position_a: [0.5, -0.25, 0.75, 11.0],
                quaternion_b: [-0.0625, 0.1875, -0.125, 0.9375],
                position_b: [-0.375, 0.625, 0.25, -13.0],
            }));
            let drive_dynamics = Arc::new(RwLock::new(SimulationDriveDynamics {
                linear: SimulationDriveDynamicsParams {
                    spring: 2.0,
                    damping: 0.5,
                    strength: 3.0,
                    raw_type: 2,
                },
                angular: SimulationDriveDynamicsParams {
                    spring: 0.25,
                    damping: 0.75,
                    strength: 1.5,
                    raw_type: 1,
                },
            }));
            let mut drive_pool = SimulationDrivePool::try_new(2, &pool).expect("Drive pool");
            for (parent, child) in [(island_2_b, island_2_a), (island_7_b, island_7_a)] {
                drive_pool
                    .try_add_drive(&pool, parent, child, &drive_frames, &drive_dynamics)
                    .expect("Drive add")
                    .expect("Drive capacity");
            }
            let foreign_drive_pool =
                SimulationDrivePool::try_new(0, &pool).expect("foreign Drive pool");

            pool.begin_dynamic_update_frame(1.0 / 60.0);
            for (body_base, body) in [0.0_f32, 10.0, 20.0, 30.0, 40.0].into_iter().zip(&bodies) {
                pool.try_with_solver_delta_row_mut(body.observation(), |rows| {
                    for (row_offset, row) in [0.0_f32, 1.0, 2.0, 3.0].into_iter().zip(rows) {
                        let base = body_base + row_offset;
                        *row = [base + 0.1, base + 0.2, base + 0.3, base + 0.4];
                    }
                })
                .expect("seed RF");
            }
            let initial_rf = bodies
                .iter()
                .map(|body| {
                    pool.try_with_solver_delta_row(body.observation(), |rows| {
                        solver_delta_words(*rows)
                    })
                    .expect("initial RF")
                })
                .collect();
            let initial_bodies = combined_horus_body_effects(&pool);
            let frame = pool.frame_identity().expect("frame");
            let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
                &pool,
                frame,
                [
                    (static_body.observation(), -1),
                    (island_2_a.observation(), 2),
                    (island_2_b.observation(), 2),
                    (island_7_a.observation(), 7),
                    (island_7_b.observation(), 7),
                ],
            )
            .expect("island map");
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for (selected, point_x) in [(island_2_a, 0.25), (island_7_a, 0.75)] {
                queue
                    .try_enqueue(&pool, request(selected.observation(), point_x, 0.125))
                    .expect("Contact request");
            }

            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut drain_backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut drain_backend,
            )
            .unwrap_or_else(|_| panic!("Contact contribution"));
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("Contact append"));
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("Contact seal: {:?}", failure.error));
            let mut contact_backend = ContactCompileTraceBackend::passing();
            let mut contact_batch = try_prepare_network_contact_island_batch_with_backend(
                sealed,
                &phase,
                SimulationContactIslandMapSource::Present(&island_map),
                &mut contact_backend,
            )
            .unwrap_or_else(|_| panic!("Contact island batch"));
            let mut joint_batch =
                prepare_joint_horus_island_batch_for_test(&joint_pool, &phase, &island_map);
            let mut drive_batch =
                prepare_drive_horus_island_batch_for_test(&drive_pool, &phase, &island_map);
            let before = CombinedHorusEffect {
                contact: contact_batch
                    .records
                    .iter()
                    .map(|prepared| contact_record_words(&prepared.record))
                    .collect(),
                joint: prepared_joint_horus_batch_words_for_test(&joint_batch),
                drive: prepared_drive_horus_batch_words_for_test(&drive_batch),
                rf: initial_rf,
                bodies: initial_bodies,
            };
            let mut phase = Some(phase);

            let result = match run {
                CombinedHorusRun::Outer {
                    island,
                    iterations,
                    foreign_drive_pool: use_foreign,
                } => try_solve_prepared_horus_island(
                    &mut contact_batch,
                    &mut joint_batch,
                    &mut drive_batch,
                    island,
                    iterations,
                    if use_foreign {
                        &foreign_drive_pool
                    } else {
                        &drive_pool
                    },
                    phase.as_mut().expect("unconsumed phase"),
                )
                .map_err(CombinedHorusError::Island),
                CombinedHorusRun::Manual { island, iterations } => {
                    for _ in 0..iterations {
                        try_solve_prepared_network_contact_horus_island_sweep(
                            &mut contact_batch,
                            island,
                            phase.as_mut().expect("unconsumed phase"),
                        )
                        .expect("manual Contact sweep");
                        try_solve_prepared_joint_horus_island_sweep(
                            &mut joint_batch,
                            island,
                            phase.as_mut().expect("unconsumed phase"),
                        )
                        .expect("manual Joint sweep");
                        try_solve_prepared_drive_horus_island_sweep(
                            &mut drive_batch,
                            island,
                            &drive_pool,
                            phase.as_mut().expect("unconsumed phase"),
                        )
                        .expect("manual Drive sweep");
                    }
                    Ok(())
                }
                CombinedHorusRun::GroupedMutant { island, iterations } => {
                    for _ in 0..iterations {
                        try_solve_prepared_network_contact_horus_island_sweep(
                            &mut contact_batch,
                            island,
                            phase.as_mut().expect("unconsumed phase"),
                        )
                        .expect("grouped Contact sweep");
                    }
                    for _ in 0..iterations {
                        try_solve_prepared_joint_horus_island_sweep(
                            &mut joint_batch,
                            island,
                            phase.as_mut().expect("unconsumed phase"),
                        )
                        .expect("grouped Joint sweep");
                    }
                    for _ in 0..iterations {
                        try_solve_prepared_drive_horus_island_sweep(
                            &mut drive_batch,
                            island,
                            &drive_pool,
                            phase.as_mut().expect("unconsumed phase"),
                        )
                        .expect("grouped Drive sweep");
                    }
                    Ok(())
                }
                CombinedHorusRun::Serial {
                    island_count,
                    iterations,
                    foreign_drive_pool: use_foreign,
                    corrupt_contact_island_7,
                    empty_active: _,
                } => {
                    if corrupt_contact_island_7 {
                        contact_batch.islands[7][0].table_body_b = static_body.observation();
                    }
                    try_solve_prepared_horus_islands_serial_and_integrate_ordinary_finite(
                        &mut contact_batch,
                        &mut joint_batch,
                        &mut drive_batch,
                        island_count,
                        iterations,
                        if use_foreign {
                            &foreign_drive_pool
                        } else {
                            &drive_pool
                        },
                        phase.take().expect("unconsumed phase"),
                    )
                    .map_err(CombinedHorusError::Serial)
                }
                CombinedHorusRun::ManualSerial {
                    island_count,
                    iterations,
                } => {
                    for island in 0..island_count {
                        for _ in 0..iterations {
                            try_solve_prepared_network_contact_horus_island_sweep(
                                &mut contact_batch,
                                island,
                                phase.as_mut().expect("unconsumed phase"),
                            )
                            .expect("manual serial Contact sweep");
                            try_solve_prepared_joint_horus_island_sweep(
                                &mut joint_batch,
                                island,
                                phase.as_mut().expect("unconsumed phase"),
                            )
                            .expect("manual serial Joint sweep");
                            try_solve_prepared_drive_horus_island_sweep(
                                &mut drive_batch,
                                island,
                                &drive_pool,
                                phase.as_mut().expect("unconsumed phase"),
                            )
                            .expect("manual serial Drive sweep");
                        }
                    }
                    phase
                        .take()
                        .expect("unconsumed phase")
                        .integrate_ordinary_finite();
                    Ok(())
                }
                CombinedHorusRun::IntegrateOnly => {
                    phase
                        .take()
                        .expect("unconsumed phase")
                        .integrate_ordinary_finite();
                    Ok(())
                }
            };
            drop(phase);
            let after = CombinedHorusEffect {
                contact: contact_batch
                    .records
                    .iter()
                    .map(|prepared| contact_record_words(&prepared.record))
                    .collect(),
                joint: prepared_joint_horus_batch_words_for_test(&joint_batch),
                drive: prepared_drive_horus_batch_words_for_test(&drive_batch),
                rf: bodies
                    .iter()
                    .map(|body| {
                        pool.try_with_solver_delta_row(body.observation(), |rows| {
                            solver_delta_words(*rows)
                        })
                        .expect("final RF")
                    })
                    .collect(),
                bodies: combined_horus_body_effects(&pool),
            };
            CombinedHorusOutcome {
                result,
                before,
                after,
            }
        }

        #[test]
        fn combined_horus_outer_loop_matches_literal_cjd_and_rejects_grouped_order() {
            let outer = run_combined_horus_fixture(CombinedHorusRun::Outer {
                island: 2,
                iterations: 2,
                foreign_drive_pool: false,
            });
            let literal = run_combined_horus_fixture(CombinedHorusRun::Manual {
                island: 2,
                iterations: 2,
            });
            let grouped = run_combined_horus_fixture(CombinedHorusRun::GroupedMutant {
                island: 2,
                iterations: 2,
            });

            assert_eq!(outer.result, Ok(()));
            assert_eq!(literal.result, Ok(()));
            assert_eq!(grouped.result, Ok(()));
            assert_eq!(outer.before, literal.before);
            assert_eq!(outer.before, grouped.before);
            assert_eq!(outer.after, literal.after);
            assert_ne!(outer.after, grouped.after);

            assert_ne!(outer.after.contact[0], outer.before.contact[0]);
            assert_ne!(outer.after.joint[0], outer.before.joint[0]);
            assert_ne!(outer.after.drive[0], outer.before.drive[0]);
            assert!(
                outer.after.rf[..3] != outer.before.rf[..3],
                "the selected island must have a visible RF effect"
            );

            assert_eq!(outer.after.contact[1], outer.before.contact[1]);
            assert_eq!(outer.after.joint[1], outer.before.joint[1]);
            assert_eq!(outer.after.drive[1], outer.before.drive[1]);
            assert_eq!(outer.after.rf[3], outer.before.rf[3]);
            assert_eq!(outer.after.rf[4], outer.before.rf[4]);
        }

        #[test]
        fn combined_horus_zero_gate_and_late_drive_refusal_are_pre_effect() {
            let out_of_range = run_combined_horus_fixture(CombinedHorusRun::Outer {
                island: SIMULATION_ISLAND_CAPACITY,
                iterations: 0,
                foreign_drive_pool: false,
            });
            assert_eq!(
                out_of_range.result,
                Err(CombinedHorusError::Island(
                    SimulationPreparedHorusIslandError::IslandOutOfRange {
                        island: SIMULATION_ISLAND_CAPACITY,
                    }
                ))
            );
            assert_eq!(out_of_range.after, out_of_range.before);

            let zero = run_combined_horus_fixture(CombinedHorusRun::Outer {
                island: 2,
                iterations: 0,
                foreign_drive_pool: true,
            });
            assert_eq!(zero.result, Ok(()));
            assert_eq!(zero.after, zero.before);

            let late_drive = run_combined_horus_fixture(CombinedHorusRun::Outer {
                island: 2,
                iterations: 1,
                foreign_drive_pool: true,
            });
            assert_eq!(
                late_drive.result,
                Err(CombinedHorusError::Island(
                    SimulationPreparedHorusIslandError::Drive(
                        SimulationPreparedDriveIslandHorusError::DrivePoolSourceMismatch,
                    )
                ))
            );
            assert_eq!(late_drive.after, late_drive.before);
        }

        #[test]
        fn combined_horus_serial_owner_matches_literal_ascending_islands_and_integrates_once() {
            let owner = run_combined_horus_fixture(CombinedHorusRun::Serial {
                island_count: 8,
                iterations: 2,
                foreign_drive_pool: false,
                corrupt_contact_island_7: false,
                empty_active: false,
            });
            let literal = run_combined_horus_fixture(CombinedHorusRun::ManualSerial {
                island_count: 8,
                iterations: 2,
            });

            assert_eq!(owner.result, Ok(()));
            assert_eq!(literal.result, Ok(()));
            assert_eq!(owner.before, literal.before);
            assert_eq!(owner.after, literal.after);
            assert_ne!(owner.after.contact, owner.before.contact);
            assert_ne!(owner.after.joint, owner.before.joint);
            assert_ne!(owner.after.drive, owner.before.drive);
            assert_ne!(owner.after.rf, owner.before.rf);
            assert_ne!(owner.after.bodies, owner.before.bodies);
            assert_eq!(owner.after.rf[0], literal.after.rf[0]);
            assert!(
                owner.after.rf[1..]
                    .iter()
                    .all(|rows| *rows == [[0_u32; 4]; 4]),
                "DynamicUpdate must clear every Active RF row after all islands"
            );
        }

        #[test]
        fn combined_horus_serial_owner_rejects_later_island_before_every_effect() {
            let refused = run_combined_horus_fixture(CombinedHorusRun::Serial {
                island_count: 8,
                iterations: 1,
                foreign_drive_pool: false,
                corrupt_contact_island_7: true,
                empty_active: false,
            });

            assert!(matches!(
                refused.result,
                Err(CombinedHorusError::Serial(
                    SimulationPreparedHorusSerialFrameError::Contact(
                        SimulationPreparedNetworkContactHorusSerialPlanError::Island {
                            island: 7,
                            source:
                                SimulationPreparedNetworkContactIslandHorusError::TableEntryMismatch {
                                    island_ordinal: 0,
                                    ..
                                },
                        }
                    )
                ))
            ));
            assert_eq!(refused.after.contact, refused.before.contact);
            assert_eq!(refused.after.joint, refused.before.joint);
            assert_eq!(refused.after.drive, refused.before.drive);
            assert_eq!(refused.after.rf, refused.before.rf);
            assert_eq!(refused.after.bodies, refused.before.bodies);
            assert!(
                refused.before.rf[1..]
                    .iter()
                    .any(|rows| *rows != [[0_u32; 4]; 4]),
                "the failure oracle must retain observably seeded Active RF"
            );
        }

        #[test]
        fn combined_horus_serial_zero_iterations_skips_poisoned_drive_and_integrates_once() {
            let owner = run_combined_horus_fixture(CombinedHorusRun::Serial {
                island_count: 8,
                iterations: 0,
                foreign_drive_pool: true,
                corrupt_contact_island_7: false,
                empty_active: false,
            });
            let integrate_only = run_combined_horus_fixture(CombinedHorusRun::IntegrateOnly);

            assert_eq!(owner.result, Ok(()));
            assert_eq!(integrate_only.result, Ok(()));
            assert_eq!(owner.before, integrate_only.before);
            assert_eq!(owner.after, integrate_only.after);
            assert_eq!(owner.after.contact, owner.before.contact);
            assert_eq!(owner.after.joint, owner.before.joint);
            assert_eq!(owner.after.drive, owner.before.drive);
            assert_ne!(owner.after.bodies, owner.before.bodies);
            assert!(
                owner.after.rf[1..]
                    .iter()
                    .all(|rows| *rows == [[0_u32; 4]; 4]),
                "zero solver passes must still end in one DynamicUpdate RF clear"
            );
        }

        #[test]
        fn combined_horus_serial_active_route_validates_host_island_count_before_effects() {
            for island_count in [0, SIMULATION_ISLAND_CAPACITY + 1] {
                let refused = run_combined_horus_fixture(CombinedHorusRun::Serial {
                    island_count,
                    iterations: 1,
                    foreign_drive_pool: false,
                    corrupt_contact_island_7: false,
                    empty_active: false,
                });
                assert_eq!(
                    refused.result,
                    Err(CombinedHorusError::Serial(
                        SimulationPreparedHorusSerialFrameError::IslandCountOutOfRange {
                            island_count,
                        },
                    ))
                );
                assert_eq!(refused.after, refused.before);
            }

            for island_count in [1, SIMULATION_ISLAND_CAPACITY] {
                let accepted = run_combined_horus_fixture(CombinedHorusRun::Serial {
                    island_count,
                    iterations: 1,
                    foreign_drive_pool: false,
                    corrupt_contact_island_7: false,
                    empty_active: false,
                });
                assert_eq!(accepted.result, Ok(()));
                assert_ne!(accepted.after.bodies, accepted.before.bodies);
            }
        }

        #[test]
        fn combined_horus_serial_empty_active_bypasses_invalid_count_and_all_work() {
            let bypassed = run_combined_horus_fixture(CombinedHorusRun::Serial {
                island_count: 0,
                iterations: 1,
                foreign_drive_pool: true,
                corrupt_contact_island_7: false,
                empty_active: true,
            });

            assert_eq!(bypassed.result, Ok(()));
            assert_eq!(bypassed.after, bypassed.before);
            assert!(!bypassed.before.contact.is_empty());
        }

        fn request_words(value: SimulationResolvedNetworkContactRequest) -> [u32; 12] {
            [
                value.point[0].to_bits(),
                value.point[1].to_bits(),
                value.point[2].to_bits(),
                value.point[3].to_bits(),
                value.normal[0].to_bits(),
                value.normal[1].to_bits(),
                value.normal[2].to_bits(),
                value.normal[3].to_bits(),
                value.displacement.to_bits(),
                value.material.static_friction.to_bits(),
                value.material.dynamic_friction.to_bits(),
                value.material.restitution.to_bits(),
            ]
        }

        fn assert_queue_entries_exact(
            queue: &SimulationNetworkContactQueue,
            expected: &[SimulationResolvedNetworkContactRequest],
        ) {
            assert_eq!(queue.len(), expected.len());
            for (actual, expected) in queue
                .entries()
                .iter()
                .copied()
                .zip(expected.iter().copied())
            {
                assert_eq!(actual.selected_body, expected.selected_body);
                assert_eq!(request_words(actual), request_words(expected));
            }
        }

        #[test]
        #[allow(
            clippy::too_many_lines,
            reason = "one fixture checks constructor success and every pre-mutation refusal"
        )]
        fn manager_domain_constructor_is_atomic_and_mints_exact_static_body() {
            let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
            let before = pool.snapshot();
            let contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 500).expect("Contact domain");
            assert_eq!(contacts.body_pool, before.authority);
            assert_eq!(contacts.maximum, 500);
            assert!(contacts.records.is_empty());
            assert!(contacts.records.capacity() >= 500);
            assert_eq!(contacts.static_body.id().index(), 0);

            let snapshot = pool.snapshot();
            assert_eq!(snapshot.counts.free, 0);
            assert_eq!(snapshot.counts.static_bodies, 1);
            assert_eq!(snapshot.static_order, vec![contacts.static_body.id()]);
            let body = &snapshot.slots[0];
            assert_eq!(body.occurrence, Some(1));
            assert_eq!(body.membership, SimulationRigidBodyList::Static);
            assert_eq!(body.state, 1);
            assert_eq!(body.center_of_mass.map(f32::to_bits), [0; 3]);
            assert_eq!(body.right.map(f32::to_bits), [1.0_f32.to_bits(), 0, 0]);
            assert_eq!(body.up.map(f32::to_bits), [0, 1.0_f32.to_bits(), 0]);
            assert_eq!(body.at.map(f32::to_bits), [0, 0, 1.0_f32.to_bits()]);
            assert!(body.inertia.is_none());
            assert_eq!(body.inverse_inertia_full.map(f32::to_bits), [0; 3]);
            assert_eq!(body.inverse_inertia_split.map(f32::to_bits), [0; 3]);
            assert_eq!(body.inverse_mass.to_bits(), 0);

            let mut allocation_pool = SimulationRigidBodyPool::try_new(1).expect("pool");
            let before = allocation_pool.snapshot();
            let error = SimulationManagerContactDomain::try_new_with_reservation(
                &mut allocation_pool,
                1,
                |_, _| Err(()),
            )
            .err()
            .expect("allocation refusal");
            assert_eq!(
                error,
                SimulationManagerContactDomainInitError::AllocationFailed { maximum: 1 }
            );
            assert_eq!(allocation_pool.snapshot(), before);

            let error = SimulationManagerContactDomain::try_new_with_reservation(
                &mut allocation_pool,
                1,
                |_, _| Ok(()),
            )
            .err()
            .expect("dishonest reservation refusal");
            assert_eq!(
                error,
                SimulationManagerContactDomainInitError::AllocationFailed { maximum: 1 }
            );
            assert_eq!(allocation_pool.snapshot(), before);

            let mut occupied_pool = SimulationRigidBodyPool::try_new(1).expect("pool");
            let _body = add_body(&mut occupied_pool, [0.0; 3], None, 1);
            let occupied = occupied_pool.snapshot();
            let error = SimulationManagerContactDomain::try_new_with_reservation(
                &mut occupied_pool,
                1,
                |_, _| panic!("non-pristine pool must refuse before reservation"),
            )
            .err()
            .expect("non-pristine refusal");
            assert_eq!(
                error,
                SimulationManagerContactDomainInitError::BodyPoolNotPristine
            );
            assert_eq!(occupied_pool.snapshot(), occupied);

            let mut phased_pool = SimulationRigidBodyPool::try_new(1).expect("pool");
            phased_pool.begin_dynamic_update_frame(1.0 / 60.0);
            let phased = phased_pool.snapshot();
            let error = SimulationManagerContactDomain::try_new_with_reservation(
                &mut phased_pool,
                1,
                |_, _| panic!("already-phased pool must refuse before reservation"),
            )
            .err()
            .expect("already-phased refusal");
            assert_eq!(
                error,
                SimulationManagerContactDomainInitError::BodyPoolNotPristine
            );
            assert_eq!(phased_pool.snapshot(), phased);

            let mut empty_pool = SimulationRigidBodyPool::try_new(0).expect("pool");
            let empty = empty_pool.snapshot();
            let error = SimulationManagerContactDomain::try_new(&mut empty_pool, 1)
                .err()
                .expect("static capacity refusal");
            assert_eq!(
                error,
                SimulationManagerContactDomainInitError::StaticBodyUnavailable
            );
            assert_eq!(empty_pool.snapshot(), empty);

            let mut overflow_pool = SimulationRigidBodyPool::try_new(1).expect("pool");
            let overflow = overflow_pool.snapshot();
            let error = SimulationManagerContactDomain::try_new_with_reservation(
                &mut overflow_pool,
                0x0100_0000,
                |_, _| panic!("native byte overflow must refuse before reservation"),
            )
            .err()
            .expect("native-width overflow refusal");
            assert_eq!(
                error,
                SimulationManagerContactDomainInitError::NativeContactByteSizeOverflow {
                    maximum: 0x0100_0000,
                }
            );
            assert_eq!(overflow_pool.snapshot(), overflow);
        }

        #[test]
        #[allow(
            clippy::too_many_lines,
            reason = "one fixture checks every copied body row and all four binding roles"
        )]
        fn successful_contribution_freezes_rows_roles_order_and_source_without_effects() {
            let mut pool = SimulationRigidBodyPool::try_new(3).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2040).expect("Contact domain");
            pool.set_gravity([7.0, 8.0, 9.0]);
            pool.set_cooldown(55);
            let body_inertia = inertia();
            let static_body = contacts.static_body;
            let selected_a = add_body(&mut pool, [1.0, 2.0, 3.0], Some(&body_inertia), 4);
            let selected_b = add_body(&mut pool, [4.0, 5.0, 6.0], Some(&body_inertia), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let before = pool.snapshot();
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            queue
                .try_enqueue(&pool, request(selected_a.observation(), 0.25, 0.125))
                .expect("first request");
            queue
                .try_enqueue(&pool, request(selected_b.observation(), -0.5, 0.25))
                .expect("second request");
            let queue_authority = queue.authority();

            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut backend,
            )
            .unwrap_or_else(|_| panic!("valid contribution"));

            assert_eq!(contribution.source.queue, queue_authority);
            assert_eq!(contribution.source.manager, destination.source.manager);
            assert_eq!(contribution.source.body_pool, phase.body_pool_authority());
            assert_eq!(contribution.source.frame, phase.frame_identity());
            assert!(contribution.source.phase == phase.phase_identity());
            assert_eq!(contribution.source.static_body, static_body.observation());
            assert_eq!(contribution.entries.len(), 2);
            assert_eq!(backend.calls.len(), 2);

            let static_slot = &before.slots[static_body.id().index() as usize];
            for (ordinal, (entry, (selected, point_x, material_base))) in contribution
                .entries
                .iter()
                .zip([
                    (selected_a, 0.25_f32, 0.125_f32),
                    (selected_b, -0.5_f32, 0.25_f32),
                ])
                .enumerate()
            {
                let selected_slot = &before.slots[selected.id().index() as usize];
                let static_token = static_body.id().index() + 1;
                let selected_token = selected.id().index() + 1;
                assert_eq!(entry.queue_ordinal, ordinal);
                assert_eq!(
                    entry.roles,
                    SimulationPreparedNetworkContactRoles {
                        table_body_a: static_body.observation(),
                        table_body_b: selected.observation(),
                        solver_side_0: static_body.observation(),
                        solver_side_1: selected.observation(),
                    }
                );
                let rows = entry.input.test_words();
                assert_eq!(
                    rows[0],
                    words3(
                        [
                            selected_slot.center_of_mass[0] + point_x,
                            selected_slot.center_of_mass[1],
                            selected_slot.center_of_mass[2],
                        ],
                        static_token,
                    )
                );
                let mut expected_position_b = rows[0];
                expected_position_b[3] = selected_token;
                assert_eq!(rows[1], expected_position_b);
                assert_eq!(rows[2][3], (material_base + 0.25).to_bits());
                assert_eq!(rows[3][3], material_base.to_bits());
                assert_eq!(rows[4][3], (material_base + 0.125).to_bits());
                assert_eq!(rows[6], words3(static_slot.center_of_mass, static_token));
                assert_eq!(
                    rows[7],
                    words3(selected_slot.center_of_mass, selected_token)
                );
                assert_eq!(
                    rows[8],
                    words3(
                        static_slot.inverse_inertia_full,
                        static_slot.inverse_mass.to_bits(),
                    )
                );
                assert_eq!(
                    rows[9],
                    words3(
                        selected_slot.inverse_inertia_full,
                        selected_slot.inverse_mass.to_bits(),
                    )
                );
                assert_eq!(
                    rows[10],
                    words3(static_slot.inverse_inertia_split, static_slot.state)
                );
                assert_eq!(
                    rows[11],
                    words3(selected_slot.inverse_inertia_split, selected_slot.state)
                );
                assert_eq!(
                    rows[12],
                    words3(static_slot.force, static_slot.kinematic_value.to_bits())
                );
                assert_eq!(
                    rows[13],
                    words3(selected_slot.force, selected_slot.kinematic_value.to_bits(),)
                );
                assert_eq!(rows[14], words3(static_slot.torque, static_slot.cooldown));
                assert_eq!(
                    rows[15],
                    words3(selected_slot.torque, selected_slot.cooldown)
                );
            }

            drop(contribution);
            drop(destination);
            drop(phase);
            assert_eq!(pool.snapshot().slots, before.slots);
        }

        #[test]
        fn preparation_samples_late_body_state_but_keeps_enqueue_time_contact_values() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2040).expect("Contact domain");
            pool.set_gravity([0.0; 3]);
            let body_inertia = inertia();
            let static_body = contacts.static_body;
            let selected = add_body(&mut pool, [1.0, 2.0, 3.0], Some(&body_inertia), 4);
            pool.begin_dynamic_update_frame(0.5);

            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            let mut caller_request = request(selected.observation(), 0.25, 0.5);
            queue
                .try_enqueue(&pool, caller_request)
                .expect("enqueue-time copy");
            caller_request.point = [9.0; 4];
            caller_request.normal = [1.0, 0.0, 0.0, 0.0];
            caller_request.material = SimulationNetworkContactMaterial {
                static_friction: 9.0,
                dynamic_friction: 10.0,
                restitution: 11.0,
            };

            let selected_index = selected.id().index() as usize;
            let before_center = pool.snapshot().slots[selected_index].center_of_mass;
            pool.try_with_solver_delta_row_mut(selected.observation(), |row| {
                row[0] = [2.0, 0.0, 0.0, 0.0];
            })
            .expect("same-occurrence RF mutation");
            pool.try_prepare_dynamic_update_phase()
                .expect("integration phase")
                .integrate_ordinary_finite();
            let after = pool.snapshot().slots[selected_index].clone();
            assert_ne!(
                after.center_of_mass.map(f32::to_bits),
                before_center.map(f32::to_bits)
            );

            let phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("preparation phase");
            let destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut backend,
            )
            .unwrap_or_else(|_| panic!("late-state contribution"));
            let rows = contribution.entries[0].input.test_words();
            let static_token = static_body.id().index() + 1;
            let selected_token = selected.id().index() + 1;

            assert_eq!(
                rows[0],
                words3(
                    [
                        after.center_of_mass[0] + 0.25,
                        after.center_of_mass[1],
                        after.center_of_mass[2],
                    ],
                    static_token,
                )
            );
            assert_eq!(rows[7], words3(after.center_of_mass, selected_token));
            assert_eq!(rows[2], words3([0.0, 0.0, 1.0], 0.75_f32.to_bits()));
            assert_eq!(rows[3][3], 0.5_f32.to_bits());
            assert_eq!(rows[4][3], 0.625_f32.to_bits());
            assert_eq!(caller_request.point[0].to_bits(), 9.0_f32.to_bits());
            assert_eq!(backend.calls.len(), 1);
        }

        #[test]
        fn empty_queue_and_pool_or_frame_mismatches_return_exact_queue_before_backend() {
            let mut backend = TraceBackend::passing();

            {
                let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
                let mut contacts = SimulationManagerContactDomain::try_new(&mut pool, 2040)
                    .expect("Contact domain");
                pool.begin_dynamic_update_frame(1.0 / 60.0);
                let queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
                let authority = queue.authority();
                let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
                let destination = contacts
                    .try_begin_contact_frame(&phase)
                    .expect("Contact frame");
                let failure = try_prepare_network_contact_contribution_with_backend(
                    queue,
                    &destination,
                    &phase,
                    &mut backend,
                )
                .err()
                .expect("empty refusal");
                assert_eq!(
                    failure.error,
                    SimulationNetworkContactPreparationError::EmptyQueue
                );
                assert_eq!(failure.queue.authority(), authority);
                assert!(failure.queue.is_empty());
                assert!(backend.calls.is_empty());
            }

            {
                let mut owner = SimulationRigidBodyPool::try_new(1).expect("owner");
                owner.begin_dynamic_update_frame(1.0 / 60.0);
                let queue = SimulationNetworkContactQueue::try_new(&owner).expect("queue");
                let authority = queue.authority();
                let mut foreign = SimulationRigidBodyPool::try_new(1).expect("foreign");
                let mut contacts = SimulationManagerContactDomain::try_new(&mut foreign, 2040)
                    .expect("Contact domain");
                foreign.begin_dynamic_update_frame(1.0 / 60.0);
                let phase = foreign.try_prepare_dynamic_update_phase().expect("phase");
                let destination = contacts
                    .try_begin_contact_frame(&phase)
                    .expect("Contact frame");
                let failure = try_prepare_network_contact_contribution_with_backend(
                    queue,
                    &destination,
                    &phase,
                    &mut backend,
                )
                .err()
                .expect("pool refusal");
                assert_eq!(
                    failure.error,
                    SimulationNetworkContactPreparationError::QueueBodyPoolMismatch
                );
                assert_eq!(failure.queue.authority(), authority);
                assert!(backend.calls.is_empty());
            }

            {
                let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
                let mut contacts = SimulationManagerContactDomain::try_new(&mut pool, 2040)
                    .expect("Contact domain");
                pool.begin_dynamic_update_frame(1.0 / 60.0);
                let queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
                let authority = queue.authority();
                pool.begin_dynamic_update_frame(1.0 / 120.0);
                let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
                let destination = contacts
                    .try_begin_contact_frame(&phase)
                    .expect("Contact frame");
                let failure = try_prepare_network_contact_contribution_with_backend(
                    queue,
                    &destination,
                    &phase,
                    &mut backend,
                )
                .err()
                .expect("frame refusal");
                assert_eq!(
                    failure.error,
                    SimulationNetworkContactPreparationError::QueueFrameMismatch
                );
                assert_eq!(failure.queue.authority(), authority);
                assert!(backend.calls.is_empty());
            }
        }

        #[test]
        fn later_stale_selected_body_is_preflighted_before_any_backend_call() {
            let mut pool = SimulationRigidBodyPool::try_new(3).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2040).expect("Contact domain");
            let body_inertia = inertia();
            let selected_a = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&body_inertia), 4);
            let selected_b = add_body(&mut pool, [2.0, 0.0, 0.0], Some(&body_inertia), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            queue
                .try_enqueue(&pool, request(selected_a.observation(), 0.0, 0.1))
                .expect("first request");
            queue
                .try_enqueue(&pool, request(selected_b.observation(), 0.0, 0.2))
                .expect("second request");
            let authority = queue.authority();
            let queued = queue.entries().to_vec();
            pool.try_remove_rigid_body(selected_b)
                .expect("remove selected B");
            let replacement = add_body(&mut pool, [3.0, 0.0, 0.0], Some(&body_inertia), 4);
            assert_eq!(replacement.id(), selected_b.id());

            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut backend = TraceBackend::passing();
            let failure = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut backend,
            )
            .err()
            .expect("stale selected refusal");
            assert_eq!(
                failure.error,
                SimulationNetworkContactPreparationError::Body {
                    queue_ordinal: 1,
                    role: SimulationNetworkContactPreparationBodyRole::Selected,
                    source: SimulationRigidBodyPoolError::StaleHandle {
                        slot: selected_b.id().index(),
                    },
                }
            );
            assert_eq!(failure.queue.authority(), authority);
            assert_queue_entries_exact(&failure.queue, &queued);
            assert!(backend.calls.is_empty());
        }

        #[test]
        fn free_selected_role_is_preflighted_before_backend() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2040).expect("Contact domain");
            let body_inertia = inertia();
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&body_inertia), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            queue
                .try_enqueue(&pool, request(selected.observation(), 0.0, 0.1))
                .expect("request");
            let mut backend = TraceBackend::passing();
            pool.try_remove_rigid_body(selected)
                .expect("remove selected");
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let failure = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut backend,
            )
            .err()
            .expect("free selected refusal");
            assert_eq!(
                failure.error,
                SimulationNetworkContactPreparationError::Body {
                    queue_ordinal: 0,
                    role: SimulationNetworkContactPreparationBodyRole::Selected,
                    source: SimulationRigidBodyPoolError::BodyIsFree {
                        slot: selected.id().index(),
                    },
                }
            );
            assert_eq!(failure.queue.len(), 1);
            assert!(backend.calls.is_empty());
        }

        #[test]
        fn allocation_refusals_are_zero_backend_and_retryable_with_the_same_queue() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2040).expect("Contact domain");
            let body_inertia = inertia();
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&body_inertia), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            queue
                .try_enqueue(&pool, request(selected.observation(), 0.0, 0.1))
                .expect("request");
            let authority = queue.authority();
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut backend = TraceBackend::passing();

            let failure = try_prepare_network_contact_contribution_with_backend_and_reservation(
                queue,
                &destination,
                &phase,
                &mut backend,
                |_, _| Err(()),
                |_, _| panic!("record reservation must be suppressed"),
            )
            .err()
            .expect("staging allocation refusal");
            assert_eq!(
                failure.error,
                SimulationNetworkContactPreparationError::StagingAllocationFailed { count: 1 }
            );
            assert_eq!(failure.queue.authority(), authority);
            assert!(backend.calls.is_empty());

            let failure = try_prepare_network_contact_contribution_with_backend_and_reservation(
                failure.queue,
                &destination,
                &phase,
                &mut backend,
                |_, _| Ok(()),
                |_, _| panic!("record reservation must be suppressed"),
            )
            .err()
            .expect("dishonest staging reservation");
            assert_eq!(
                failure.error,
                SimulationNetworkContactPreparationError::StagingAllocationFailed { count: 1 }
            );
            assert_eq!(failure.queue.authority(), authority);
            assert!(backend.calls.is_empty());

            let failure = try_prepare_network_contact_contribution_with_backend_and_reservation(
                failure.queue,
                &destination,
                &phase,
                &mut backend,
                |staged, count| staged.try_reserve_exact(count).map_err(|_| ()),
                |_, _| Ok(()),
            )
            .err()
            .expect("dishonest record reservation");
            assert_eq!(
                failure.error,
                SimulationNetworkContactPreparationError::RecordAllocationFailed { count: 1 }
            );
            assert_eq!(failure.queue.authority(), authority);
            assert!(backend.calls.is_empty());

            let prepared = try_prepare_network_contact_contribution_with_backend(
                failure.queue,
                &destination,
                &phase,
                &mut backend,
            )
            .unwrap_or_else(|_| panic!("retry succeeds"));
            assert_eq!(prepared.entries.len(), 1);
            assert_eq!(backend.calls.len(), 1);
        }

        #[test]
        fn later_numeric_failure_returns_queue_and_retry_restarts_at_ordinal_zero() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2040).expect("Contact domain");
            let body_inertia = inertia();
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&body_inertia), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            queue
                .try_enqueue(&pool, request(selected.observation(), 0.0, 0.1))
                .expect("first request");
            queue
                .try_enqueue(&pool, request(selected.observation(), 0.25, 0.2))
                .expect("second request");
            let authority = queue.authority();
            let queued = queue.entries().to_vec();
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut first_backend = TraceBackend {
                calls: Vec::new(),
                fail_on: Some(0),
            };
            let failure = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut first_backend,
            )
            .err()
            .expect("first numeric refusal");
            assert_eq!(
                failure.error,
                SimulationNetworkContactPreparationError::Numeric {
                    queue_ordinal: 0,
                    source: "injected network preparation estimate failure",
                }
            );
            assert_eq!(failure.queue.authority(), authority);
            assert_queue_entries_exact(&failure.queue, &queued);
            assert_eq!(first_backend.calls.len(), 1);

            let mut later_backend = TraceBackend {
                calls: Vec::new(),
                fail_on: Some(1),
            };
            let failure = try_prepare_network_contact_contribution_with_backend(
                failure.queue,
                &destination,
                &phase,
                &mut later_backend,
            )
            .err()
            .expect("later numeric refusal");
            assert_eq!(
                failure.error,
                SimulationNetworkContactPreparationError::Numeric {
                    queue_ordinal: 1,
                    source: "injected network preparation estimate failure",
                }
            );
            assert_eq!(failure.queue.authority(), authority);
            assert_queue_entries_exact(&failure.queue, &queued);
            assert_eq!(later_backend.calls.len(), 2);

            let mut retry_backend = TraceBackend::passing();
            let prepared = try_prepare_network_contact_contribution_with_backend(
                failure.queue,
                &destination,
                &phase,
                &mut retry_backend,
            )
            .unwrap_or_else(|_| panic!("numeric retry"));
            assert_eq!(prepared.entries.len(), 2);
            assert_eq!(retry_backend.calls.len(), 2);
            assert_eq!(prepared.entries[0].queue_ordinal, 0);
            assert_eq!(prepared.entries[1].queue_ordinal, 1);
        }

        #[test]
        fn append_is_dense_in_call_order_and_frame_reset_reuses_backing() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 3).expect("Contact domain");
            let static_body = contacts.static_body;
            let selected = add_body(&mut pool, [1.0, 2.0, 3.0], Some(&inertia()), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut first_queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for point_x in [0.25, 0.5] {
                first_queue
                    .try_enqueue(&pool, request(selected.observation(), point_x, 0.125))
                    .expect("first producer request");
            }
            let first_authority = first_queue.authority();
            let mut second_queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            second_queue
                .try_enqueue(&pool, request(selected.observation(), 0.75, 0.25))
                .expect("second producer request");
            let second_authority = second_queue.authority();

            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let backing = destination.domain.records.as_ptr();
            let capacity = destination.domain.records.capacity();
            let mut backend = TraceBackend::passing();
            let first = try_prepare_network_contact_contribution_with_backend(
                first_queue,
                &destination,
                &phase,
                &mut backend,
            )
            .unwrap_or_else(|_| panic!("first contribution"));
            destination
                .try_append_network_contact_contribution(first, &phase)
                .unwrap_or_else(|_| panic!("first append"));
            let second = try_prepare_network_contact_contribution_with_backend(
                second_queue,
                &destination,
                &phase,
                &mut backend,
            )
            .unwrap_or_else(|_| panic!("second contribution"));
            destination
                .try_append_network_contact_contribution(second, &phase)
                .unwrap_or_else(|_| panic!("second append"));

            assert_eq!(destination.domain.records.len(), 3);
            assert_eq!(destination.domain.records.as_ptr(), backing);
            assert_eq!(destination.domain.records.capacity(), capacity);
            for (ordinal, record) in destination.domain.records.iter().enumerate() {
                assert_eq!(
                    record.dense_ordinal,
                    u32::try_from(ordinal).expect("three records fit u32")
                );
                assert_eq!(
                    record.producer,
                    SimulationContactProducerSource::Network {
                        queue: [first_authority, first_authority, second_authority][ordinal],
                        queue_ordinal: [0, 1, 0][ordinal],
                        selected_body: selected.observation(),
                    }
                );
                assert_eq!(record.roles.table_body_a, static_body.observation());
                assert_eq!(record.roles.table_body_b, selected.observation());
                let rows = record.input.test_words();
                assert_eq!(rows[0][3], static_body.id().index() + 1);
                assert_eq!(rows[1][3], selected.id().index() + 1);
            }

            let source = destination.source;
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            assert_eq!(sealed.source.manager, source.manager);
            assert_eq!(sealed.source.frame, source.frame);
            assert_eq!(sealed.domain.records.len(), 3);
            drop(sealed);
            let error = contacts
                .try_begin_contact_frame(&phase)
                .err()
                .expect("same-frame reopen refusal");
            assert_eq!(error, SimulationContactFrameBeginError::FrameAlreadyStarted);
            assert_eq!(contacts.records.len(), 3);
            drop(phase);

            pool.begin_dynamic_update_frame(1.0 / 120.0);
            let phase = pool.try_prepare_dynamic_update_phase().expect("next phase");
            let destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("next Contact frame");
            assert!(destination.domain.records.is_empty());
            assert_eq!(destination.domain.records.as_ptr(), backing);
            assert_eq!(destination.domain.records.capacity(), capacity);
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("empty seal failed: {:?}", failure.error));
            assert!(sealed.domain.records.is_empty());
        }

        #[test]
        fn two_full_432_queues_distinguish_local_from_global_500_capacity() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 500).expect("Contact domain");
            let body_inertia = inertia();
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&body_inertia), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            let mut second_queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for ordinal in 0..432 {
                let request = request(
                    selected.observation(),
                    f32::from(u16::try_from(ordinal).expect("queue ordinal fits u16")),
                    0.125,
                );
                queue
                    .try_enqueue(&pool, request)
                    .expect("within first native queue capacity");
                second_queue
                    .try_enqueue(&pool, request)
                    .expect("within second native queue capacity");
            }
            let second_authority = second_queue.authority();
            let second_entries = second_queue.entries().to_vec();
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut backend = TraceBackend::passing();
            let prepared = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut backend,
            )
            .unwrap_or_else(|_| panic!("full local contribution"));
            assert_eq!(prepared.entries.len(), 432);
            assert_eq!(backend.calls.len(), 432);
            assert_eq!(prepared.entries[0].queue_ordinal, 0);
            assert_eq!(prepared.entries[431].queue_ordinal, 431);
            destination
                .try_append_network_contact_contribution(prepared, &phase)
                .unwrap_or_else(|_| panic!("first global append"));
            assert_eq!(destination.domain.records.len(), 432);

            let mut second_backend = TraceBackend::passing();
            let failure = try_prepare_network_contact_contribution_with_backend_and_reservation(
                second_queue,
                &destination,
                &phase,
                &mut second_backend,
                |_, _| panic!("global capacity must precede staging reservation"),
                |_, _| panic!("global capacity must precede record reservation"),
            )
            .err()
            .expect("global capacity refusal");
            assert_eq!(
                failure.error,
                SimulationNetworkContactPreparationError::DestinationCapacityExceeded {
                    current: 432,
                    incoming: 432,
                    maximum: 500,
                }
            );
            assert_eq!(failure.queue.authority(), second_authority);
            assert_queue_entries_exact(&failure.queue, &second_entries);
            assert!(second_backend.calls.is_empty());
            assert_eq!(destination.domain.records.len(), 432);
        }

        #[test]
        fn append_preflights_the_whole_contribution_before_first_write() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 3).expect("Contact domain");
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&inertia()), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for point_x in [0.25, 0.5] {
                queue
                    .try_enqueue(&pool, request(selected.observation(), point_x, 0.125))
                    .expect("request");
            }
            let queue_authority = queue.authority();
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let backing = destination.domain.records.as_ptr();
            let capacity = destination.domain.records.capacity();
            let mut backend = TraceBackend::passing();
            let mut contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));
            let first_words = contribution.entries[0].input.test_words();
            let second_words = contribution.entries[1].input.test_words();
            let source = contribution.source;
            contribution.entries[1].queue_ordinal = 0;

            let failure = destination
                .try_append_network_contact_contribution(contribution, &phase)
                .expect_err("ordinal refusal");
            assert_eq!(
                failure.error,
                SimulationContactAppendError::QueueOrdinalMismatch {
                    contribution_ordinal: 1,
                    actual: 0,
                }
            );
            assert!(destination.domain.records.is_empty());
            assert_eq!(destination.domain.records.as_ptr(), backing);
            assert_eq!(destination.domain.records.capacity(), capacity);
            assert_eq!(failure.contribution.source.manager, source.manager);
            assert_eq!(failure.contribution.source.queue, queue_authority);
            assert_eq!(failure.contribution.entries.len(), 2);
            assert_eq!(
                failure.contribution.entries[0].input.test_words(),
                first_words
            );
            assert_eq!(
                failure.contribution.entries[1].input.test_words(),
                second_words
            );

            let mut contribution = failure.contribution;
            contribution.entries[1].queue_ordinal = 1;
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("repaired append"));
            assert_eq!(destination.domain.records.len(), 2);
            assert_eq!(
                destination.domain.records[0].input.test_words(),
                first_words
            );
            assert_eq!(
                destination.domain.records[1].input.test_words(),
                second_words
            );
        }

        #[test]
        #[allow(
            clippy::too_many_lines,
            reason = "one table-driven fixture independently corrupts four roles and four words"
        )]
        fn append_checks_each_role_and_embedded_one_based_word() {
            let mut pool = SimulationRigidBodyPool::try_new(3).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 1).expect("Contact domain");
            let body_inertia = inertia();
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&body_inertia), 4);
            let other = add_body(&mut pool, [2.0, 0.0, 0.0], Some(&body_inertia), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            queue
                .try_enqueue(&pool, request(selected.observation(), 0.25, 0.125))
                .expect("request");
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut backend = TraceBackend::passing();
            let mut contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));

            for kind in [
                SimulationNetworkContactPreparedBindingKind::TableBodyA,
                SimulationNetworkContactPreparedBindingKind::TableBodyB,
                SimulationNetworkContactPreparedBindingKind::SolverSide0,
                SimulationNetworkContactPreparedBindingKind::SolverSide1,
            ] {
                let original = match kind {
                    SimulationNetworkContactPreparedBindingKind::TableBodyA => {
                        &mut contribution.entries[0].roles.table_body_a
                    }
                    SimulationNetworkContactPreparedBindingKind::TableBodyB => {
                        &mut contribution.entries[0].roles.table_body_b
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide0 => {
                        &mut contribution.entries[0].roles.solver_side_0
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide1 => {
                        &mut contribution.entries[0].roles.solver_side_1
                    }
                };
                let saved = *original;
                *original = other.observation();
                let failure = destination
                    .try_append_network_contact_contribution(contribution, &phase)
                    .expect_err("role refusal");
                assert_eq!(
                    failure.error,
                    SimulationContactAppendError::RoleMismatch {
                        contribution_ordinal: 0,
                        kind,
                    }
                );
                assert!(destination.domain.records.is_empty());
                contribution = failure.contribution;
                match kind {
                    SimulationNetworkContactPreparedBindingKind::TableBodyA => {
                        contribution.entries[0].roles.table_body_a = saved;
                    }
                    SimulationNetworkContactPreparedBindingKind::TableBodyB => {
                        contribution.entries[0].roles.table_body_b = saved;
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide0 => {
                        contribution.entries[0].roles.solver_side_0 = saved;
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide1 => {
                        contribution.entries[0].roles.solver_side_1 = saved;
                    }
                }
            }

            for kind in [
                SimulationNetworkContactPreparedBindingKind::TableBodyA,
                SimulationNetworkContactPreparedBindingKind::TableBodyB,
                SimulationNetworkContactPreparedBindingKind::SolverSide0,
                SimulationNetworkContactPreparedBindingKind::SolverSide1,
            ] {
                let word = match kind {
                    SimulationNetworkContactPreparedBindingKind::TableBodyA => {
                        &mut contribution.entries[0].input.position_a_body_a.word
                    }
                    SimulationNetworkContactPreparedBindingKind::TableBodyB => {
                        &mut contribution.entries[0].input.position_b_body_b.word
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide0 => {
                        &mut contribution.entries[0].input.side_0_com_rf_token.word
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide1 => {
                        &mut contribution.entries[0].input.side_1_com_rf_token.word
                    }
                };
                let expected = *word;
                *word ^= 0x8000_0000;
                let actual = *word;
                let failure = destination
                    .try_append_network_contact_contribution(contribution, &phase)
                    .expect_err("binding-word refusal");
                assert_eq!(
                    failure.error,
                    SimulationContactAppendError::BindingTokenMismatch {
                        contribution_ordinal: 0,
                        kind,
                        expected,
                        actual,
                    }
                );
                assert!(destination.domain.records.is_empty());
                contribution = failure.contribution;
                match kind {
                    SimulationNetworkContactPreparedBindingKind::TableBodyA => {
                        contribution.entries[0].input.position_a_body_a.word = expected;
                    }
                    SimulationNetworkContactPreparedBindingKind::TableBodyB => {
                        contribution.entries[0].input.position_b_body_b.word = expected;
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide0 => {
                        contribution.entries[0].input.side_0_com_rf_token.word = expected;
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide1 => {
                        contribution.entries[0].input.side_1_com_rf_token.word = expected;
                    }
                }
            }

            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("fully repaired append"));
            assert_eq!(destination.domain.records.len(), 1);
        }

        #[test]
        fn append_rechecks_global_capacity_and_returns_exact_late_contribution() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 3).expect("Contact domain");
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&inertia()), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut first_queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            let mut second_queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for point_x in [0.25, 0.5] {
                let request = request(selected.observation(), point_x, 0.125);
                first_queue
                    .try_enqueue(&pool, request)
                    .expect("first request");
                second_queue
                    .try_enqueue(&pool, request)
                    .expect("second request");
            }
            let second_authority = second_queue.authority();
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut backend = TraceBackend::passing();
            let first = try_prepare_network_contact_contribution_with_backend(
                first_queue,
                &destination,
                &phase,
                &mut backend,
            )
            .unwrap_or_else(|_| panic!("first preparation"));
            let second = try_prepare_network_contact_contribution_with_backend(
                second_queue,
                &destination,
                &phase,
                &mut backend,
            )
            .unwrap_or_else(|_| panic!("second preparation"));
            let second_words: Vec<_> = second
                .entries
                .iter()
                .map(|entry| entry.input.test_words())
                .collect();
            destination
                .try_append_network_contact_contribution(first, &phase)
                .unwrap_or_else(|_| panic!("first append"));
            let prefix_words: Vec<_> = destination
                .domain
                .records
                .iter()
                .map(|record| record.input.test_words())
                .collect();
            let backing = destination.domain.records.as_ptr();
            let failure = destination
                .try_append_network_contact_contribution(second, &phase)
                .expect_err("append capacity refusal");
            assert_eq!(
                failure.error,
                SimulationContactAppendError::CapacityExceeded {
                    current: 2,
                    incoming: 2,
                    maximum: 3,
                }
            );
            assert_eq!(failure.contribution.source.queue, second_authority);
            assert_eq!(
                failure
                    .contribution
                    .entries
                    .iter()
                    .map(|entry| entry.input.test_words())
                    .collect::<Vec<_>>(),
                second_words
            );
            assert_eq!(destination.domain.records.len(), 2);
            assert_eq!(destination.domain.records.as_ptr(), backing);
            assert_eq!(
                destination
                    .domain
                    .records
                    .iter()
                    .map(|record| record.input.test_words())
                    .collect::<Vec<_>>(),
                prefix_words
            );
        }

        #[test]
        fn append_revalidates_occurrence_without_resampling_numeric_rows() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 1).expect("Contact domain");
            let body_inertia = inertia();
            let stale = add_body(&mut pool, [9.0, 0.0, 0.0], Some(&body_inertia), 4);
            let stale_observation = stale.observation();
            pool.try_remove_rigid_body(stale)
                .expect("remove old occurrence");
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&body_inertia), 4);
            assert_eq!(selected.id(), stale.id());
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            queue
                .try_enqueue(&pool, request(selected.observation(), 0.25, 0.125))
                .expect("request");
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut backend = TraceBackend::passing();
            let mut contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));
            let words = contribution.entries[0].input.test_words();
            contribution.entries[0].selected_body = stale_observation;
            contribution.entries[0].roles.table_body_b = stale_observation;
            contribution.entries[0].roles.solver_side_1 = stale_observation;

            let failure = destination
                .try_append_network_contact_contribution(contribution, &phase)
                .expect_err("stale occurrence refusal");
            assert_eq!(
                failure.error,
                SimulationContactAppendError::Body {
                    contribution_ordinal: 0,
                    kind: SimulationNetworkContactPreparedBindingKind::TableBodyB,
                    source: SimulationRigidBodyPoolError::StaleHandle {
                        slot: selected.id().index(),
                    },
                }
            );
            assert!(destination.domain.records.is_empty());
            assert_eq!(failure.contribution.entries[0].input.test_words(), words);

            let mut contribution = failure.contribution;
            contribution.entries[0].selected_body = selected.observation();
            contribution.entries[0].roles.table_body_b = selected.observation();
            contribution.entries[0].roles.solver_side_1 = selected.observation();
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("repaired occurrence append"));
            assert_eq!(destination.domain.records[0].input.test_words(), words);
        }

        #[test]
        fn prepared_contribution_is_bound_to_its_exact_manager_and_frame() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 1).expect("Contact domain");
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&inertia()), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            queue
                .try_enqueue(&pool, request(selected.observation(), 0.25, 0.125))
                .expect("request");
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));
            let source = contribution.source;
            let words = contribution.entries[0].input.test_words();
            drop(destination);
            drop(phase);

            pool.begin_dynamic_update_frame(1.0 / 120.0);
            let phase = pool.try_prepare_dynamic_update_phase().expect("next phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("next Contact frame");
            let failure = destination
                .try_append_network_contact_contribution(contribution, &phase)
                .expect_err("old-frame contribution refusal");
            assert_eq!(
                failure.error,
                SimulationContactAppendError::ContributionFrameMismatch
            );
            assert_eq!(failure.contribution.source.manager, source.manager);
            assert_eq!(failure.contribution.source.frame, source.frame);
            assert_eq!(failure.contribution.entries[0].input.test_words(), words);
            assert!(destination.domain.records.is_empty());
            let contribution = failure.contribution;
            drop(destination);
            drop(phase);

            let mut foreign_pool = SimulationRigidBodyPool::try_new(1).expect("foreign pool");
            let mut foreign_contacts =
                SimulationManagerContactDomain::try_new(&mut foreign_pool, 1)
                    .expect("foreign Contact domain");
            foreign_pool.begin_dynamic_update_frame(1.0 / 60.0);
            let foreign_phase = foreign_pool
                .try_prepare_dynamic_update_phase()
                .expect("foreign phase");
            let mut foreign_destination = foreign_contacts
                .try_begin_contact_frame(&foreign_phase)
                .expect("foreign Contact frame");
            let failure = foreign_destination
                .try_append_network_contact_contribution(contribution, &foreign_phase)
                .expect_err("foreign manager refusal");
            assert_eq!(
                failure.error,
                SimulationContactAppendError::ContributionManagerMismatch
            );
            assert_eq!(failure.contribution.source.manager, source.manager);
            assert_eq!(failure.contribution.entries[0].input.test_words(), words);
            assert!(foreign_destination.domain.records.is_empty());
        }

        #[test]
        fn frame_begin_and_seal_classify_foreign_replacement_and_stale_phases() {
            let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 1).expect("Contact domain");
            let mut foreign_pool = SimulationRigidBodyPool::try_new(1).expect("foreign pool");
            let _foreign_contacts = SimulationManagerContactDomain::try_new(&mut foreign_pool, 1)
                .expect("foreign Contact domain");
            foreign_pool.begin_dynamic_update_frame(1.0 / 60.0);
            let foreign_phase = foreign_pool
                .try_prepare_dynamic_update_phase()
                .expect("foreign phase");
            let error = contacts
                .try_begin_contact_frame(&foreign_phase)
                .err()
                .expect("foreign phase refusal");
            assert_eq!(error, SimulationContactFrameBeginError::BodyPoolMismatch);
            assert_eq!(contacts.last_started_frame, None);
            assert!(contacts.records.is_empty());
            drop(foreign_phase);

            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            drop(phase);
            let replacement_phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("same-frame replacement phase");
            let failure = destination
                .try_seal(&replacement_phase)
                .err()
                .expect("replacement-phase seal refusal");
            assert_eq!(failure.error, SimulationContactFrameSealError::Phase);
            assert!(failure.frame.domain.records.is_empty());
            drop(failure.frame);
            drop(replacement_phase);

            pool.begin_dynamic_update_frame(1.0 / 120.0);
            let next_phase = pool.try_prepare_dynamic_update_phase().expect("next phase");
            let destination = contacts
                .try_begin_contact_frame(&next_phase)
                .expect("next Contact frame");
            drop(next_phase);
            pool.begin_dynamic_update_frame(1.0 / 30.0);
            let current_phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("current phase");
            let failure = destination
                .try_seal(&current_phase)
                .err()
                .expect("stale seal refusal");
            assert_eq!(failure.error, SimulationContactFrameSealError::Frame);
            assert!(failure.frame.domain.records.is_empty());
            drop(failure.frame);
            let destination = contacts
                .try_begin_contact_frame(&current_phase)
                .expect("current Contact frame");
            let sealed = destination
                .try_seal(&current_phase)
                .unwrap_or_else(|failure| panic!("current seal failed: {:?}", failure.error));
            assert!(sealed.domain.records.is_empty());
        }

        #[test]
        fn preparation_append_and_seal_all_reject_a_same_frame_replacement_phase() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2).expect("Contact domain");
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&inertia()), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut first_queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            first_queue
                .try_enqueue(&pool, request(selected.observation(), 0.25, 0.125))
                .expect("first request");
            let mut second_queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            second_queue
                .try_enqueue(&pool, request(selected.observation(), 0.5, 0.25))
                .expect("second request");
            let second_authority = second_queue.authority();
            let second_entries = second_queue.entries().to_vec();

            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut first_backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                first_queue,
                &destination,
                &phase,
                &mut first_backend,
            )
            .unwrap_or_else(|_| panic!("first contribution"));
            let contribution_source = contribution.source;
            let contribution_words = contribution.entries[0].input.test_words();
            drop(phase);

            let replacement_phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("same-frame replacement phase");
            let mut second_backend = TraceBackend::passing();
            let preparation_failure =
                try_prepare_network_contact_contribution_with_backend_and_reservation(
                    second_queue,
                    &destination,
                    &replacement_phase,
                    &mut second_backend,
                    |_, _| panic!("phase mismatch must precede staging reservation"),
                    |_, _| panic!("phase mismatch must precede record reservation"),
                )
                .err()
                .expect("preparation phase refusal");
            assert_eq!(
                preparation_failure.error,
                SimulationNetworkContactPreparationError::DestinationPhaseMismatch
            );
            assert_eq!(preparation_failure.queue.authority(), second_authority);
            assert_queue_entries_exact(&preparation_failure.queue, &second_entries);
            assert!(second_backend.calls.is_empty());
            assert!(destination.domain.records.is_empty());

            let append_failure = destination
                .try_append_network_contact_contribution(contribution, &replacement_phase)
                .expect_err("append phase refusal");
            assert_eq!(
                append_failure.error,
                SimulationContactAppendError::DestinationPhaseMismatch
            );
            assert_eq!(
                append_failure.contribution.source.manager,
                contribution_source.manager
            );
            assert!(append_failure.contribution.source.phase == contribution_source.phase);
            assert_eq!(
                append_failure.contribution.entries[0].input.test_words(),
                contribution_words
            );
            assert!(destination.domain.records.is_empty());
            drop(append_failure.contribution);

            let seal_failure = destination
                .try_seal(&replacement_phase)
                .err()
                .expect("seal phase refusal");
            assert_eq!(seal_failure.error, SimulationContactFrameSealError::Phase);
            assert!(seal_failure.frame.domain.records.is_empty());
        }

        #[test]
        fn contact_island_selector_is_a_preferred_and_checks_only_the_winner() {
            assert_eq!(try_selected_contact_island::<()>(0, 3, 7), Ok(3));
            assert_eq!(try_selected_contact_island::<()>(1, -1, 7), Ok(7));
            assert_eq!(try_selected_contact_island::<()>(2, 0, -2), Ok(0));
            for (dense_ordinal, raw_body_a, raw_body_b, selected) in
                [(3, -2, 0, -2), (4, 10, 0, 10), (5, -1, -1, -1)]
            {
                assert_eq!(
                    try_selected_contact_island::<()>(dense_ordinal, raw_body_a, raw_body_b,),
                    Err(
                        SimulationNetworkContactIslandPreparationError::InvalidSelectedIsland {
                            dense_ordinal,
                            raw_body_a,
                            raw_body_b,
                            selected,
                        }
                    )
                );
            }
        }

        #[test]
        #[allow(
            clippy::too_many_lines,
            reason = "one integrated fixture checks dense compilation and all ten stable tables"
        )]
        fn sealed_contacts_compile_in_dense_order_and_form_stable_island_subsequences() {
            let mut pool = SimulationRigidBodyPool::try_new(4).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 3).expect("Contact domain");
            let body_inertia = inertia();
            let selected_a = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&body_inertia), 4);
            let selected_b = add_body(&mut pool, [2.0, 0.0, 0.0], Some(&body_inertia), 4);
            let selected_c = add_body(&mut pool, [3.0, 0.0, 0.0], Some(&body_inertia), 4);
            let static_body = contacts.static_body;
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for (selected, point_x) in [(selected_a, 0.25), (selected_b, 0.5), (selected_c, 0.75)] {
                queue
                    .try_enqueue(&pool, request(selected.observation(), point_x, 0.125))
                    .expect("request");
            }
            let frame_identity = pool.frame_identity().expect("started frame identity");
            let map = SimulationRigidBodyIslandMapSnapshot::try_new(
                &pool,
                frame_identity,
                [
                    (static_body.observation(), -1),
                    (selected_a.observation(), 2),
                    (selected_b.observation(), 0),
                    (selected_c.observation(), 2),
                ],
            )
            .expect("island map");
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut drain_backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut drain_backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("append"));
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            let source = sealed.source;
            let inputs: Vec<_> = sealed
                .domain
                .records
                .iter()
                .map(|record| record.input)
                .collect();
            let mut compile_backend = ContactCompileTraceBackend::passing();
            let batch = try_prepare_network_contact_island_batch_with_backend(
                sealed,
                &phase,
                SimulationContactIslandMapSource::Present(&map),
                &mut compile_backend,
            )
            .unwrap_or_else(|_| panic!("Contact island batch"));

            assert_eq!(batch.records.len(), 3);
            assert_eq!(compile_backend.calls.len(), 3);
            assert_eq!(
                batch.islands[0]
                    .iter()
                    .map(|entry| entry.contact.dense_ordinal)
                    .collect::<Vec<_>>(),
                vec![1]
            );
            assert_eq!(
                batch.islands[2]
                    .iter()
                    .map(|entry| entry.contact.dense_ordinal)
                    .collect::<Vec<_>>(),
                vec![0, 2]
            );
            for (island, entries) in batch.islands.iter().enumerate() {
                if island != 0 && island != 2 {
                    assert!(entries.is_empty());
                }
            }
            for (dense_ordinal, prepared) in batch.records.iter().enumerate() {
                assert_eq!(
                    prepared.contact,
                    SimulationContactIdentity {
                        manager: source.manager,
                        frame: source.frame,
                        dense_ordinal: u32::try_from(dense_ordinal)
                            .expect("three Contacts fit u32"),
                    }
                );
                assert_eq!(prepared.table_body_a, static_body.observation());
                assert_eq!(prepared.solver_side_0, static_body.observation());
                assert_eq!(
                    prepared.table_body_b,
                    [selected_a, selected_b, selected_c][dense_ordinal].observation()
                );
                assert_eq!(prepared.solver_side_1, prepared.table_body_b);
                let mut direct_backend = ContactCompileTraceBackend::passing();
                let expected = build_contact_jacobian_host(
                    inputs[dense_ordinal],
                    phase.timestep(),
                    &mut direct_backend,
                )
                .expect("direct Contact Build");
                assert!(prepared.record == expected);
                assert_eq!(
                    compile_backend.calls[dense_ordinal],
                    direct_backend.calls[0]
                );
            }
            for entry in batch.islands.iter().flatten() {
                let record = &batch.records[entry.contact.dense_ordinal as usize];
                assert_eq!(entry.contact, record.contact);
                assert_eq!(entry.table_body_a, record.table_body_a);
                assert_eq!(entry.table_body_b, record.table_body_b);
            }
            assert_eq!(batch.frame.source.manager, source.manager);
            assert_eq!(batch.frame.source.frame, source.frame);
        }

        #[test]
        #[allow(
            clippy::too_many_lines,
            reason = "the integrated oracle keeps two full live Contact sweeps auditable"
        )]
        fn checked_contact_island_sweep_matches_pure_record_and_all_rf_rows() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 1).expect("Contact domain");
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&inertia()), 4);
            let static_body = contacts.static_body;
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let initial_a = [
                [0.10, -0.20, 0.30, 0.40],
                [-0.50, 0.60, -0.70, 0.80],
                [0.90, -1.00, 1.10, -1.20],
                [-1.30, 1.40, -1.50, 1.60],
            ];
            let initial_b = [
                [-0.15, 0.25, -0.35, 0.45],
                [0.55, -0.65, 0.75, -0.85],
                [-0.95, 1.05, -1.15, 1.25],
                [1.35, -1.45, 1.55, -1.65],
            ];
            pool.try_with_solver_delta_row_mut(static_body.observation(), |rows| {
                *rows = initial_a;
            })
            .expect("seed static RF");
            pool.try_with_solver_delta_row_mut(selected.observation(), |rows| {
                *rows = initial_b;
            })
            .expect("seed selected RF");
            let frame_identity = pool.frame_identity().expect("frame");
            let map = SimulationRigidBodyIslandMapSnapshot::try_new(
                &pool,
                frame_identity,
                [(static_body.observation(), -1), (selected.observation(), 2)],
            )
            .expect("island map");
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            queue
                .try_enqueue(&pool, request(selected.observation(), 0.25, 0.125))
                .expect("request");
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut drain_backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut drain_backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("append"));
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            let mut compile_backend = ContactCompileTraceBackend::passing();
            let mut batch = try_prepare_network_contact_island_batch_with_backend(
                sealed,
                &phase,
                SimulationContactIslandMapSource::Present(&map),
                &mut compile_backend,
            )
            .unwrap_or_else(|_| panic!("Contact island batch"));

            let original_words = contact_record_words(&batch.records[0].record);
            try_solve_prepared_network_contact_horus_island_sweep(&mut batch, 1, &mut phase)
                .expect("empty island sweep");
            assert_eq!(
                contact_record_words(&batch.records[0].record),
                original_words
            );

            let mut expected_record = batch.records[0].record;
            let first = solve_contact_horus_record_host(&mut expected_record, initial_a, initial_b);
            try_solve_prepared_network_contact_horus_island_sweep(&mut batch, 2, &mut phase)
                .expect("first Contact sweep");
            assert_eq!(
                contact_record_words(&batch.records[0].record),
                contact_record_words(&expected_record)
            );

            let second = solve_contact_horus_record_host(
                &mut expected_record,
                first.solver_delta_a,
                first.solver_delta_b,
            );
            try_solve_prepared_network_contact_horus_island_sweep(&mut batch, 2, &mut phase)
                .expect("second Contact sweep");
            assert_eq!(
                contact_record_words(&batch.records[0].record),
                contact_record_words(&expected_record)
            );
            drop(phase);
            let actual_a = pool
                .try_with_solver_delta_row(static_body.observation(), |rows| *rows)
                .expect("read static RF");
            let actual_b = pool
                .try_with_solver_delta_row(selected.observation(), |rows| *rows)
                .expect("read selected RF");
            assert_eq!(
                solver_delta_words(actual_a),
                solver_delta_words(second.solver_delta_a)
            );
            assert_eq!(
                solver_delta_words(actual_b),
                solver_delta_words(second.solver_delta_b)
            );
        }

        #[test]
        #[allow(
            clippy::too_many_lines,
            reason = "one inert record distinguishes range, foreign pool, replacement phase, and later frame"
        )]
        fn contact_sweep_rejects_range_and_replacement_phase_without_effects() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 1).expect("Contact domain");
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&inertia()), 4);
            let static_body = contacts.static_body;
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let initial_a = [[0.25; 4], [0.5; 4], [0.75; 4], [1.0; 4]];
            let initial_b = [[-0.25; 4], [-0.5; 4], [-0.75; 4], [-1.0; 4]];
            pool.try_with_solver_delta_row_mut(static_body.observation(), |rows| {
                *rows = initial_a;
            })
            .expect("seed static RF");
            pool.try_with_solver_delta_row_mut(selected.observation(), |rows| {
                *rows = initial_b;
            })
            .expect("seed selected RF");
            let frame_identity = pool.frame_identity().expect("frame");
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            queue
                .try_enqueue(&pool, request(selected.observation(), 0.25, 0.125))
                .expect("request");
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut drain_backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut drain_backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("append"));
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            let mut compile_backend = ContactCompileTraceBackend::passing();
            let mut batch = try_prepare_network_contact_island_batch_with_backend(
                sealed,
                &phase,
                SimulationContactIslandMapSource::Null {
                    frame: frame_identity,
                },
                &mut compile_backend,
            )
            .unwrap_or_else(|_| panic!("Contact island batch"));
            let record_words = contact_record_words(&batch.records[0].record);

            assert_eq!(
                try_solve_prepared_network_contact_horus_island_sweep(
                    &mut batch,
                    SIMULATION_ISLAND_CAPACITY,
                    &mut phase,
                ),
                Err(
                    SimulationPreparedNetworkContactIslandHorusError::IslandOutOfRange {
                        island: SIMULATION_ISLAND_CAPACITY,
                    }
                )
            );
            assert_eq!(contact_record_words(&batch.records[0].record), record_words);
            drop(phase);

            let mut foreign_pool = SimulationRigidBodyPool::try_new(0).expect("foreign pool");
            foreign_pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut foreign_phase = foreign_pool
                .try_prepare_dynamic_update_phase()
                .expect("foreign phase");
            assert_eq!(
                try_solve_prepared_network_contact_horus_island_sweep(
                    &mut batch,
                    0,
                    &mut foreign_phase,
                ),
                Err(SimulationPreparedNetworkContactIslandHorusError::SourceBodyPoolMismatch)
            );
            assert_eq!(contact_record_words(&batch.records[0].record), record_words);
            drop(foreign_phase);

            let mut replacement_phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("replacement phase");
            assert_eq!(
                try_solve_prepared_network_contact_horus_island_sweep(
                    &mut batch,
                    0,
                    &mut replacement_phase,
                ),
                Err(SimulationPreparedNetworkContactIslandHorusError::SourcePhaseMismatch)
            );
            assert_eq!(contact_record_words(&batch.records[0].record), record_words);
            drop(replacement_phase);
            assert_eq!(
                solver_delta_words(
                    pool.try_with_solver_delta_row(static_body.observation(), |rows| *rows)
                        .expect("read static RF")
                ),
                solver_delta_words(initial_a)
            );
            assert_eq!(
                solver_delta_words(
                    pool.try_with_solver_delta_row(selected.observation(), |rows| *rows)
                        .expect("read selected RF")
                ),
                solver_delta_words(initial_b)
            );

            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut later_frame_phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("later-frame phase");
            assert_eq!(
                try_solve_prepared_network_contact_horus_island_sweep(
                    &mut batch,
                    0,
                    &mut later_frame_phase,
                ),
                Err(SimulationPreparedNetworkContactIslandHorusError::SourceFrameMismatch)
            );
            assert_eq!(contact_record_words(&batch.records[0].record), record_words);
            drop(later_frame_phase);
            drop(batch);
            assert_eq!(
                solver_delta_words(
                    pool.try_with_solver_delta_row(static_body.observation(), |rows| *rows)
                        .expect("read static RF after drop")
                ),
                solver_delta_words(initial_a)
            );
            assert_eq!(
                solver_delta_words(
                    pool.try_with_solver_delta_row(selected.observation(), |rows| *rows)
                        .expect("read selected RF after drop")
                ),
                solver_delta_words(initial_b)
            );
        }

        #[test]
        #[allow(
            clippy::too_many_lines,
            reason = "one live batch is repaired after each pre-effect source, table, role, and token refusal"
        )]
        fn contact_sweep_preflights_sources_tables_roles_and_tokens_before_effects() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 1).expect("Contact domain");
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&inertia()), 4);
            let static_body = contacts.static_body;
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let initial_a = [[0.1; 4], [0.2; 4], [0.3; 4], [0.4; 4]];
            let initial_b = [[-0.1; 4], [-0.2; 4], [-0.3; 4], [-0.4; 4]];
            pool.try_with_solver_delta_row_mut(static_body.observation(), |rows| {
                *rows = initial_a;
            })
            .expect("seed static RF");
            pool.try_with_solver_delta_row_mut(selected.observation(), |rows| {
                *rows = initial_b;
            })
            .expect("seed selected RF");
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            queue
                .try_enqueue(&pool, request(selected.observation(), 0.25, 0.125))
                .expect("request");
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut drain_backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut drain_backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("append"));
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            let mut compile_backend = ContactCompileTraceBackend::passing();
            let mut batch = try_prepare_network_contact_island_batch_with_backend(
                sealed,
                &phase,
                SimulationContactIslandMapSource::Null {
                    frame: phase.frame_identity(),
                },
                &mut compile_backend,
            )
            .unwrap_or_else(|_| panic!("Contact island batch"));
            let contact = batch.records[0].contact;
            let original_record_words = contact_record_words(&batch.records[0].record);

            let original_source = batch.frame.source;
            batch.frame.source.manager = SimulationManagerContactAuthority(
                NonZeroU64::new(u64::MAX).expect("nonzero manager authority"),
            );
            assert_eq!(
                try_solve_prepared_network_contact_horus_island_sweep(&mut batch, 0, &mut phase,),
                Err(SimulationPreparedNetworkContactIslandHorusError::SourceManagerMismatch)
            );
            batch.frame.source = original_source;

            batch.frame.source.static_body = selected.observation();
            assert_eq!(
                try_solve_prepared_network_contact_horus_island_sweep(&mut batch, 0, &mut phase,),
                Err(SimulationPreparedNetworkContactIslandHorusError::SourceStaticBodyMismatch)
            );
            batch.frame.source = original_source;

            let original_entry = batch.islands[0][0];
            batch.islands[0][0].contact.dense_ordinal = 7;
            let corrupt_contact = batch.islands[0][0].contact;
            assert_eq!(
                try_solve_prepared_network_contact_horus_island_sweep(&mut batch, 0, &mut phase,),
                Err(
                    SimulationPreparedNetworkContactIslandHorusError::TableEntryMismatch {
                        island_ordinal: 0,
                        contact: corrupt_contact,
                    }
                )
            );
            batch.islands[0][0] = original_entry;

            batch.islands[0][0].table_body_a = selected.observation();
            assert_eq!(
                try_solve_prepared_network_contact_horus_island_sweep(&mut batch, 0, &mut phase,),
                Err(
                    SimulationPreparedNetworkContactIslandHorusError::TableEntryMismatch {
                        island_ordinal: 0,
                        contact,
                    }
                )
            );
            batch.islands[0][0] = original_entry;

            for kind in [
                SimulationNetworkContactPreparedBindingKind::TableBodyA,
                SimulationNetworkContactPreparedBindingKind::TableBodyB,
                SimulationNetworkContactPreparedBindingKind::SolverSide0,
                SimulationNetworkContactPreparedBindingKind::SolverSide1,
            ] {
                let original_prepared = (
                    batch.records[0].table_body_a,
                    batch.records[0].table_body_b,
                    batch.records[0].solver_side_0,
                    batch.records[0].solver_side_1,
                );
                match kind {
                    SimulationNetworkContactPreparedBindingKind::TableBodyA => {
                        batch.records[0].table_body_a = selected.observation();
                        batch.islands[0][0].table_body_a = selected.observation();
                    }
                    SimulationNetworkContactPreparedBindingKind::TableBodyB => {
                        batch.records[0].table_body_b = static_body.observation();
                        batch.islands[0][0].table_body_b = static_body.observation();
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide0 => {
                        batch.records[0].solver_side_0 = selected.observation();
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide1 => {
                        batch.records[0].solver_side_1 = static_body.observation();
                    }
                }
                assert_eq!(
                    try_solve_prepared_network_contact_horus_island_sweep(
                        &mut batch, 0, &mut phase,
                    ),
                    Err(
                        SimulationPreparedNetworkContactIslandHorusError::SourceRecordMismatch {
                            island_ordinal: 0,
                            contact,
                        }
                    ),
                    "role {kind:?}"
                );
                (
                    batch.records[0].table_body_a,
                    batch.records[0].table_body_b,
                    batch.records[0].solver_side_0,
                    batch.records[0].solver_side_1,
                ) = original_prepared;
                batch.islands[0][0] = original_entry;
            }

            for (kind, expected) in [
                (
                    SimulationNetworkContactPreparedBindingKind::TableBodyA,
                    static_body.id().index() + 1,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::TableBodyB,
                    selected.id().index() + 1,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::SolverSide0,
                    static_body.id().index() + 1,
                ),
                (
                    SimulationNetworkContactPreparedBindingKind::SolverSide1,
                    selected.id().index() + 1,
                ),
            ] {
                let actual = match kind {
                    SimulationNetworkContactPreparedBindingKind::TableBodyA => {
                        let actual = batch.records[0].record.body_a.0;
                        batch.records[0].record.body_a.0 = 0;
                        actual
                    }
                    SimulationNetworkContactPreparedBindingKind::TableBodyB => {
                        let actual = batch.records[0].record.body_b.0;
                        batch.records[0].record.body_b.0 = 0;
                        actual
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide0 => {
                        let actual = batch.records[0].record.solver_delta_a.0;
                        batch.records[0].record.solver_delta_a.0 = 0;
                        actual
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide1 => {
                        let actual = batch.records[0].record.solver_delta_b.0;
                        batch.records[0].record.solver_delta_b.0 = 0;
                        actual
                    }
                };
                assert_eq!(actual, expected);
                assert_eq!(
                    try_solve_prepared_network_contact_horus_island_sweep(
                        &mut batch, 0, &mut phase,
                    ),
                    Err(
                        SimulationPreparedNetworkContactIslandHorusError::BindingTokenMismatch {
                            island_ordinal: 0,
                            contact,
                            kind,
                            expected,
                            actual: 0,
                        }
                    )
                );
                match kind {
                    SimulationNetworkContactPreparedBindingKind::TableBodyA => {
                        batch.records[0].record.body_a.0 = actual;
                    }
                    SimulationNetworkContactPreparedBindingKind::TableBodyB => {
                        batch.records[0].record.body_b.0 = actual;
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide0 => {
                        batch.records[0].record.solver_delta_a.0 = actual;
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide1 => {
                        batch.records[0].record.solver_delta_b.0 = actual;
                    }
                }
            }
            assert_eq!(
                contact_record_words(&batch.records[0].record),
                original_record_words
            );

            let mut expected_record = batch.records[0].record;
            let expected =
                solve_contact_horus_record_host(&mut expected_record, initial_a, initial_b);
            try_solve_prepared_network_contact_horus_island_sweep(&mut batch, 0, &mut phase)
                .expect("repaired Contact sweep");
            assert_eq!(
                contact_record_words(&batch.records[0].record),
                contact_record_words(&expected_record)
            );
            drop(phase);
            assert_eq!(
                solver_delta_words(
                    pool.try_with_solver_delta_row(static_body.observation(), |rows| *rows)
                        .expect("read static RF")
                ),
                solver_delta_words(expected.solver_delta_a)
            );
            assert_eq!(
                solver_delta_words(
                    pool.try_with_solver_delta_row(selected.observation(), |rows| *rows)
                        .expect("read selected RF")
                ),
                solver_delta_words(expected.solver_delta_b)
            );
        }

        #[test]
        #[allow(
            clippy::too_many_lines,
            reason = "same-slot reuse after preparation isolates the later occurrence preflight"
        )]
        fn later_stale_contact_role_is_rejected_before_first_record_or_rf_effect() {
            let mut pool = SimulationRigidBodyPool::try_new(3).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2).expect("Contact domain");
            let body_inertia = inertia();
            let first_body = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&body_inertia), 4);
            let stale_body = add_body(&mut pool, [2.0, 0.0, 0.0], Some(&body_inertia), 4);
            let static_body = contacts.static_body;
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for (body, point_x) in [(first_body, 0.25), (stale_body, 0.5)] {
                queue
                    .try_enqueue(&pool, request(body.observation(), point_x, 0.125))
                    .expect("request");
            }
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut drain_backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut drain_backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("append"));
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            let mut compile_backend = ContactCompileTraceBackend::passing();
            let mut batch = try_prepare_network_contact_island_batch_with_backend(
                sealed,
                &phase,
                SimulationContactIslandMapSource::Null {
                    frame: phase.frame_identity(),
                },
                &mut compile_backend,
            )
            .unwrap_or_else(|_| panic!("Contact island batch"));
            let record_words: Vec<_> = batch
                .records
                .iter()
                .map(|record| contact_record_words(&record.record))
                .collect();
            let failed_contact = batch.records[1].contact;
            drop(phase);

            pool.try_remove_rigid_body(stale_body)
                .expect("remove selected occurrence");
            let replacement = add_body(&mut pool, [3.0, 0.0, 0.0], Some(&body_inertia), 4);
            assert_eq!(replacement.id(), stale_body.id());
            let static_rows = [[0.1; 4], [0.2; 4], [0.3; 4], [0.4; 4]];
            let first_rows = [[1.1; 4], [1.2; 4], [1.3; 4], [1.4; 4]];
            let replacement_rows = [[2.1; 4], [2.2; 4], [2.3; 4], [2.4; 4]];
            for (body, rows) in [
                (static_body, static_rows),
                (first_body, first_rows),
                (replacement, replacement_rows),
            ] {
                pool.try_with_solver_delta_row_mut(body.observation(), |actual| *actual = rows)
                    .expect("seed replacement-phase RF");
            }
            let mut replacement_phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("replacement phase");
            // Test-only isolation: bypass the independent replay seal so the
            // stale second body occurrence is the first failing invariant.
            batch.frame.source.phase = replacement_phase.phase_identity();
            let failure = try_solve_prepared_network_contact_horus_island_sweep(
                &mut batch,
                0,
                &mut replacement_phase,
            )
            .expect_err("later stale occurrence refusal");
            assert!(matches!(
                failure,
                SimulationPreparedNetworkContactIslandHorusError::Body {
                    island_ordinal: 1,
                    contact,
                    kind: SimulationNetworkContactPreparedBindingKind::TableBodyB,
                    source: SimulationRigidBodyPoolError::StaleHandle { slot, .. },
                } if contact == failed_contact && slot == replacement.id().index()
            ));
            assert_eq!(
                batch
                    .records
                    .iter()
                    .map(|record| contact_record_words(&record.record))
                    .collect::<Vec<_>>(),
                record_words
            );
            drop(replacement_phase);
            for (body, expected) in [
                (static_body, static_rows),
                (first_body, first_rows),
                (replacement, replacement_rows),
            ] {
                let actual = pool
                    .try_with_solver_delta_row(body.observation(), |rows| *rows)
                    .expect("read unchanged RF");
                assert_eq!(solver_delta_words(actual), solver_delta_words(expected));
            }
        }

        #[test]
        #[allow(
            clippy::too_many_lines,
            reason = "the literal repeated-table oracle tracks one shared and two distinct RF blocks"
        )]
        fn contact_sweep_preserves_nonmonotonic_repeated_table_order() {
            let mut pool = SimulationRigidBodyPool::try_new(3).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2).expect("Contact domain");
            let body_inertia = inertia();
            let selected_0 = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&body_inertia), 4);
            let selected_1 = add_body(&mut pool, [2.0, 0.0, 0.0], Some(&body_inertia), 4);
            let static_body = contacts.static_body;
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let initial_a = [[0.1; 4], [0.2; 4], [0.3; 4], [0.4; 4]];
            let initial_b0 = [[1.1; 4], [1.2; 4], [1.3; 4], [1.4; 4]];
            let initial_b1 = [[2.1; 4], [2.2; 4], [2.3; 4], [2.4; 4]];
            for (body, rows) in [
                (static_body, initial_a),
                (selected_0, initial_b0),
                (selected_1, initial_b1),
            ] {
                pool.try_with_solver_delta_row_mut(body.observation(), |actual| *actual = rows)
                    .expect("seed RF");
            }
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for (body, point_x) in [(selected_0, 0.25), (selected_1, 0.5)] {
                queue
                    .try_enqueue(&pool, request(body.observation(), point_x, 0.125))
                    .expect("request");
            }
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut drain_backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut drain_backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("append"));
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            let mut compile_backend = ContactCompileTraceBackend::passing();
            let mut batch = try_prepare_network_contact_island_batch_with_backend(
                sealed,
                &phase,
                SimulationContactIslandMapSource::Null {
                    frame: phase.frame_identity(),
                },
                &mut compile_backend,
            )
            .unwrap_or_else(|_| panic!("Contact island batch"));
            let first_entry = batch.islands[0][0];
            let second_entry = batch.islands[0][1];
            batch.islands[0] = vec![second_entry, first_entry, second_entry];

            let mut expected_records = [batch.records[0].record, batch.records[1].record];
            let mut expected_a = initial_a;
            let mut expected_b0 = initial_b0;
            let mut expected_b1 = initial_b1;
            for dense_index in [1_usize, 0, 1] {
                let selected_rows = if dense_index == 0 {
                    expected_b0
                } else {
                    expected_b1
                };
                let solved = solve_contact_horus_record_host(
                    &mut expected_records[dense_index],
                    expected_a,
                    selected_rows,
                );
                expected_a = solved.solver_delta_a;
                if dense_index == 0 {
                    expected_b0 = solved.solver_delta_b;
                } else {
                    expected_b1 = solved.solver_delta_b;
                }
            }

            try_solve_prepared_network_contact_horus_island_sweep(&mut batch, 0, &mut phase)
                .expect("repeated-table Contact sweep");
            for (actual, expected) in batch.records.iter().zip(&expected_records) {
                assert_eq!(
                    contact_record_words(&actual.record),
                    contact_record_words(expected)
                );
            }
            drop(phase);
            for (body, expected) in [
                (static_body, expected_a),
                (selected_0, expected_b0),
                (selected_1, expected_b1),
            ] {
                let actual = pool
                    .try_with_solver_delta_row(body.observation(), |rows| *rows)
                    .expect("read RF");
                assert_eq!(solver_delta_words(actual), solver_delta_words(expected));
            }
        }

        #[test]
        fn null_island_map_routes_every_sealed_contact_to_zero() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2).expect("Contact domain");
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&inertia()), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for point_x in [0.25, 0.5] {
                queue
                    .try_enqueue(&pool, request(selected.observation(), point_x, 0.125))
                    .expect("request");
            }
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut drain_backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut drain_backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("append"));
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            let mut compile_backend = ContactCompileTraceBackend::passing();
            let batch = try_prepare_network_contact_island_batch_with_backend(
                sealed,
                &phase,
                SimulationContactIslandMapSource::Null {
                    frame: phase.frame_identity(),
                },
                &mut compile_backend,
            )
            .unwrap_or_else(|_| panic!("Contact island batch"));
            assert_eq!(batch.records.len(), 2);
            assert_eq!(batch.islands[0].len(), 2);
            assert_eq!(batch.islands[0][0].contact.dense_ordinal, 0);
            assert_eq!(batch.islands[0][1].contact.dense_ordinal, 1);
            assert!(batch.islands[1..].iter().all(Vec::is_empty));
        }

        #[test]
        fn present_map_body_a_wins_without_a_contact_state_gate() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2).expect("Contact domain");
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&inertia()), 1);
            let static_body = contacts.static_body;
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let frame_identity = pool.frame_identity().expect("frame");
            let map = SimulationRigidBodyIslandMapSnapshot::try_new(
                &pool,
                frame_identity,
                [(static_body.observation(), 4), (selected.observation(), -2)],
            )
            .expect("island map");
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for point_x in [0.25, 0.5] {
                queue
                    .try_enqueue(&pool, request(selected.observation(), point_x, 0.125))
                    .expect("request");
            }
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut drain_backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut drain_backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("append"));
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            let mut compile_backend = ContactCompileTraceBackend::passing();
            let batch = try_prepare_network_contact_island_batch_with_backend(
                sealed,
                &phase,
                SimulationContactIslandMapSource::Present(&map),
                &mut compile_backend,
            )
            .unwrap_or_else(|_| panic!("Contact island batch"));

            assert_eq!(batch.records.len(), 2);
            assert_eq!(batch.islands[4].len(), 2);
            assert_eq!(batch.islands[4][0].contact.dense_ordinal, 0);
            assert_eq!(batch.islands[4][1].contact.dense_ordinal, 1);
            assert!(
                batch
                    .islands
                    .iter()
                    .enumerate()
                    .all(|(island, entries)| island == 4 || entries.is_empty())
            );
            assert_eq!(compile_backend.calls.len(), 2);
        }

        #[test]
        fn empty_seal_remains_source_bound_without_numeric_work() {
            let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 0).expect("Contact domain");
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let old_frame = pool.frame_identity().expect("old frame");
            let old_map = SimulationRigidBodyIslandMapSnapshot::try_new(&pool, old_frame, [])
                .expect("old map");
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let frame_identity = phase.frame_identity();
            let destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut frame = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            let source = frame.source;

            let mut foreign_pool = SimulationRigidBodyPool::try_new(0).expect("foreign pool");
            foreign_pool.begin_dynamic_update_frame(1.0 / 60.0);
            foreign_pool.begin_dynamic_update_frame(1.0 / 60.0);
            let foreign_frame = foreign_pool.frame_identity().expect("foreign frame");
            let foreign_map =
                SimulationRigidBodyIslandMapSnapshot::try_new(&foreign_pool, foreign_frame, [])
                    .expect("foreign map");
            let mut backend = ContactCompileTraceBackend::passing();

            for (map, expected) in [
                (
                    SimulationContactIslandMapSource::Present(&foreign_map),
                    SimulationNetworkContactIslandPreparationError::IslandMapBodyPoolMismatch,
                ),
                (
                    SimulationContactIslandMapSource::Present(&old_map),
                    SimulationNetworkContactIslandPreparationError::IslandMapFrameMismatch,
                ),
                (
                    SimulationContactIslandMapSource::Null { frame: old_frame },
                    SimulationNetworkContactIslandPreparationError::IslandMapFrameMismatch,
                ),
            ] {
                let failure =
                    try_prepare_network_contact_island_batch_with_backend_reservation_and_observer(
                        frame,
                        &phase,
                        map,
                        &mut backend,
                        |_, _| panic!("source refusal must precede record reservation"),
                        |_, _, _| panic!("source refusal must precede island reservation"),
                        |_, _| panic!("empty source refusal must precede record reads"),
                    )
                    .err()
                    .expect("source refusal");
                assert_eq!(failure.error, expected);
                assert_eq!(failure.frame.source.manager, source.manager);
                assert_eq!(failure.frame.source.frame, source.frame);
                assert!(failure.frame.domain.records.is_empty());
                assert!(backend.calls.is_empty());
                frame = failure.frame;
            }

            let mut record_reservations = Vec::new();
            let mut island_reservations = Vec::new();
            let mut reads = Vec::new();
            let mut batch =
                try_prepare_network_contact_island_batch_with_backend_reservation_and_observer(
                    frame,
                    &phase,
                    SimulationContactIslandMapSource::Null {
                        frame: frame_identity,
                    },
                    &mut backend,
                    |_, count| {
                        record_reservations.push(count);
                        Ok(())
                    },
                    |island, _, count| {
                        island_reservations.push((island, count));
                        Ok(())
                    },
                    |kind, ordinal| reads.push((kind, ordinal)),
                )
                .unwrap_or_else(|_| panic!("empty source-bound batch"));
            assert_eq!(batch.frame.source.manager, source.manager);
            assert_eq!(batch.frame.source.frame, source.frame);
            assert!(batch.records.is_empty());
            assert!(batch.islands.iter().all(Vec::is_empty));
            assert_eq!(record_reservations, [0]);
            assert!(island_reservations.is_empty());
            assert!(reads.is_empty());
            assert!(backend.calls.is_empty());
            try_solve_prepared_network_contact_horus_island_sweep(&mut batch, 9, &mut phase)
                .expect("empty selected-island sweep");
            assert!(batch.records.is_empty());
            assert!(batch.islands.iter().all(Vec::is_empty));
        }

        #[test]
        fn empty_seal_rejects_a_same_frame_replacement_phase_before_work() {
            let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 0).expect("Contact domain");
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            let source = sealed.source;
            drop(phase);
            let replacement_phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("replacement phase");
            let mut backend = ContactCompileTraceBackend::passing();
            let failure =
                try_prepare_network_contact_island_batch_with_backend_reservation_and_observer(
                    sealed,
                    &replacement_phase,
                    SimulationContactIslandMapSource::Null {
                        frame: replacement_phase.frame_identity(),
                    },
                    &mut backend,
                    |_, _| panic!("phase refusal must precede record reservation"),
                    |_, _, _| panic!("phase refusal must precede island reservation"),
                    |_, _| panic!("phase refusal must precede record reads"),
                )
                .err()
                .expect("replacement phase refusal");
            assert_eq!(
                failure.error,
                SimulationNetworkContactIslandPreparationError::SourcePhaseMismatch
            );
            assert_eq!(failure.frame.source.manager, source.manager);
            assert!(failure.frame.source.phase == source.phase);
            assert!(failure.frame.domain.records.is_empty());
            assert!(backend.calls.is_empty());
        }

        #[test]
        fn same_slot_stale_island_label_is_rejected_before_allocation_or_compile() {
            let mut pool = SimulationRigidBodyPool::try_new(3).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2).expect("Contact domain");
            let static_body = contacts.static_body;
            let body_inertia = inertia();
            let first_body = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&body_inertia), 4);
            let stale = add_body(&mut pool, [9.0, 0.0, 0.0], Some(&body_inertia), 4);
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let frame_identity = pool.frame_identity().expect("frame");
            let stale_map = SimulationRigidBodyIslandMapSnapshot::try_new(
                &pool,
                frame_identity,
                [
                    (static_body.observation(), -1),
                    (first_body.observation(), 0),
                    (stale.observation(), 3),
                ],
            )
            .expect("stale map");
            pool.try_remove_rigid_body(stale)
                .expect("remove stale body");
            let replacement = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&body_inertia), 4);
            assert_eq!(replacement.id(), stale.id());

            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for (body, point_x) in [(first_body, 0.25), (replacement, 0.5)] {
                queue
                    .try_enqueue(&pool, request(body.observation(), point_x, 0.125))
                    .expect("request");
            }
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut drain_backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut drain_backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("append"));
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            let mut compile_backend = ContactCompileTraceBackend::passing();
            let mut reads = Vec::new();
            let failure =
                try_prepare_network_contact_island_batch_with_backend_reservation_and_observer(
                    sealed,
                    &phase,
                    SimulationContactIslandMapSource::Present(&stale_map),
                    &mut compile_backend,
                    |_, _| panic!("stale occurrence must precede record reservation"),
                    |_, _, _| panic!("stale occurrence must precede island reservation"),
                    |kind, ordinal| reads.push((kind, ordinal)),
                )
                .err()
                .expect("stale label occurrence refusal");
            assert_eq!(
                failure.error,
                SimulationNetworkContactIslandPreparationError::IslandLabelOccurrenceMismatch {
                    dense_ordinal: 1,
                    endpoint: SimulationContactTableEndpoint::BodyB,
                    expected_body: replacement.observation(),
                    mapped_body: stale.observation(),
                }
            );
            assert_eq!(
                reads,
                [
                    (SimulationContactIslandPreparationReadKind::DenseOrdinal, 0),
                    (SimulationContactIslandPreparationReadKind::Role, 0),
                    (SimulationContactIslandPreparationReadKind::BindingToken, 0,),
                    (SimulationContactIslandPreparationReadKind::IslandLabelA, 0),
                    (SimulationContactIslandPreparationReadKind::IslandLabelB, 0),
                    (SimulationContactIslandPreparationReadKind::DenseOrdinal, 1),
                    (SimulationContactIslandPreparationReadKind::Role, 1),
                    (SimulationContactIslandPreparationReadKind::BindingToken, 1,),
                    (SimulationContactIslandPreparationReadKind::IslandLabelA, 1),
                    (SimulationContactIslandPreparationReadKind::IslandLabelB, 1),
                ]
            );
            assert!(compile_backend.calls.is_empty());
            assert_eq!(failure.frame.domain.records.len(), 2);
        }

        #[test]
        fn every_stored_contact_binding_token_is_rechecked_before_allocation_or_compile() {
            for kind in [
                SimulationNetworkContactPreparedBindingKind::TableBodyA,
                SimulationNetworkContactPreparedBindingKind::TableBodyB,
                SimulationNetworkContactPreparedBindingKind::SolverSide0,
                SimulationNetworkContactPreparedBindingKind::SolverSide1,
            ] {
                let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
                let mut contacts =
                    SimulationManagerContactDomain::try_new(&mut pool, 1).expect("Contact domain");
                let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&inertia()), 4);
                pool.begin_dynamic_update_frame(1.0 / 60.0);
                let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
                queue
                    .try_enqueue(&pool, request(selected.observation(), 0.25, 0.125))
                    .expect("request");
                let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
                let mut destination = contacts
                    .try_begin_contact_frame(&phase)
                    .expect("Contact frame");
                let mut drain_backend = TraceBackend::passing();
                let contribution = try_prepare_network_contact_contribution_with_backend(
                    queue,
                    &destination,
                    &phase,
                    &mut drain_backend,
                )
                .unwrap_or_else(|_| panic!("contribution"));
                destination
                    .try_append_network_contact_contribution(contribution, &phase)
                    .unwrap_or_else(|_| panic!("append"));
                let record = &mut destination.domain.records[0];
                let observation = match kind {
                    SimulationNetworkContactPreparedBindingKind::TableBodyA => {
                        record.input.position_a_body_a.word = 0;
                        record.roles.table_body_a
                    }
                    SimulationNetworkContactPreparedBindingKind::TableBodyB => {
                        record.input.position_b_body_b.word = 0;
                        record.roles.table_body_b
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide0 => {
                        record.input.side_0_com_rf_token.word = 0;
                        record.roles.solver_side_0
                    }
                    SimulationNetworkContactPreparedBindingKind::SolverSide1 => {
                        record.input.side_1_com_rf_token.word = 0;
                        record.roles.solver_side_1
                    }
                };
                let expected = observation
                    .identity()
                    .id()
                    .index()
                    .checked_add(1)
                    .expect("two-slot fixture");
                let sealed = destination
                    .try_seal(&phase)
                    .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
                let mut compile_backend = ContactCompileTraceBackend::passing();
                let mut reads = Vec::new();
                let failure =
                    try_prepare_network_contact_island_batch_with_backend_reservation_and_observer(
                        sealed,
                        &phase,
                        SimulationContactIslandMapSource::Null {
                            frame: phase.frame_identity(),
                        },
                        &mut compile_backend,
                        |_, _| panic!("token refusal must precede record reservation"),
                        |_, _, _| panic!("token refusal must precede island reservation"),
                        |read, ordinal| reads.push((read, ordinal)),
                    )
                    .err()
                    .expect("stored token refusal");
                assert_eq!(
                    failure.error,
                    SimulationNetworkContactIslandPreparationError::BindingTokenMismatch {
                        dense_ordinal: 0,
                        kind,
                        expected,
                        actual: 0,
                    }
                );
                assert_eq!(
                    reads,
                    [
                        (SimulationContactIslandPreparationReadKind::DenseOrdinal, 0),
                        (SimulationContactIslandPreparationReadKind::Role, 0),
                        (SimulationContactIslandPreparationReadKind::BindingToken, 0,),
                    ]
                );
                assert!(compile_backend.calls.is_empty());
                assert_eq!(failure.frame.domain.records.len(), 1);
            }
        }

        #[test]
        #[allow(
            clippy::too_many_lines,
            reason = "one returned seal is retried through every allocation and label boundary"
        )]
        fn island_preparation_refusals_return_exact_seal_before_publication() {
            let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2).expect("Contact domain");
            let selected = add_body(&mut pool, [1.0, 0.0, 0.0], Some(&inertia()), 4);
            let static_body = contacts.static_body;
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let frame_identity = pool.frame_identity().expect("started frame identity");
            let missing_a = SimulationRigidBodyIslandMapSnapshot::try_new(
                &pool,
                frame_identity,
                [(selected.observation(), 0)],
            )
            .expect("missing-A map");
            let missing_b = SimulationRigidBodyIslandMapSnapshot::try_new(
                &pool,
                frame_identity,
                [(static_body.observation(), -1)],
            )
            .expect("missing-B map");
            let invalid = SimulationRigidBodyIslandMapSnapshot::try_new(
                &pool,
                frame_identity,
                [
                    (static_body.observation(), -1),
                    (selected.observation(), -2),
                ],
            )
            .expect("invalid map");
            let invalid_ten = SimulationRigidBodyIslandMapSnapshot::try_new(
                &pool,
                frame_identity,
                [(static_body.observation(), 10), (selected.observation(), 0)],
            )
            .expect("label-ten map");
            let both_minus_one = SimulationRigidBodyIslandMapSnapshot::try_new(
                &pool,
                frame_identity,
                [
                    (static_body.observation(), -1),
                    (selected.observation(), -1),
                ],
            )
            .expect("both-minus-one map");
            let valid = SimulationRigidBodyIslandMapSnapshot::try_new(
                &pool,
                frame_identity,
                [(static_body.observation(), -1), (selected.observation(), 2)],
            )
            .expect("valid map");
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for point_x in [0.25, 0.5] {
                queue
                    .try_enqueue(&pool, request(selected.observation(), point_x, 0.125))
                    .expect("request");
            }
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut drain_backend = TraceBackend::passing();
            let contribution = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut drain_backend,
            )
            .unwrap_or_else(|_| panic!("contribution"));
            destination
                .try_append_network_contact_contribution(contribution, &phase)
                .unwrap_or_else(|_| panic!("append"));
            let mut frame = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            let source = frame.source;
            let input_words: Vec<_> = frame
                .domain
                .records
                .iter()
                .map(|record| record.input.test_words())
                .collect();
            let mut compile_backend = ContactCompileTraceBackend::passing();

            let failure =
                try_prepare_network_contact_island_batch_with_backend_reservation_and_observer(
                    frame,
                    &phase,
                    SimulationContactIslandMapSource::Present(&missing_a),
                    &mut compile_backend,
                    |_, _| panic!("label refusal must precede record reservation"),
                    |_, _, _| panic!("label refusal must precede island reservation"),
                    |_, _| {},
                )
                .err()
                .expect("missing A refusal");
            assert_eq!(
                failure.error,
                SimulationNetworkContactIslandPreparationError::MissingIslandLabel {
                    dense_ordinal: 0,
                    endpoint: SimulationContactTableEndpoint::BodyA,
                    body: static_body.observation(),
                }
            );
            assert!(compile_backend.calls.is_empty());
            frame = failure.frame;

            let failure =
                try_prepare_network_contact_island_batch_with_backend_reservation_and_observer(
                    frame,
                    &phase,
                    SimulationContactIslandMapSource::Present(&missing_b),
                    &mut compile_backend,
                    |_, _| panic!("label refusal must precede record reservation"),
                    |_, _, _| panic!("label refusal must precede island reservation"),
                    |_, _| {},
                )
                .err()
                .expect("missing B refusal");
            assert_eq!(
                failure.error,
                SimulationNetworkContactIslandPreparationError::MissingIslandLabel {
                    dense_ordinal: 0,
                    endpoint: SimulationContactTableEndpoint::BodyB,
                    body: selected.observation(),
                }
            );
            assert!(compile_backend.calls.is_empty());
            frame = failure.frame;

            for (map, raw_body_a, raw_body_b, selected_label) in [
                (&invalid, -1, -2, -2),
                (&invalid_ten, 10, 0, 10),
                (&both_minus_one, -1, -1, -1),
            ] {
                let failure =
                    try_prepare_network_contact_island_batch_with_backend_reservation_and_observer(
                        frame,
                        &phase,
                        SimulationContactIslandMapSource::Present(map),
                        &mut compile_backend,
                        |_, _| panic!("label refusal must precede record reservation"),
                        |_, _, _| panic!("label refusal must precede island reservation"),
                        |_, _| {},
                    )
                    .err()
                    .expect("invalid selected island refusal");
                assert_eq!(
                    failure.error,
                    SimulationNetworkContactIslandPreparationError::InvalidSelectedIsland {
                        dense_ordinal: 0,
                        raw_body_a,
                        raw_body_b,
                        selected: selected_label,
                    }
                );
                assert!(compile_backend.calls.is_empty());
                frame = failure.frame;
            }

            for dishonest in [false, true] {
                let failure =
                    try_prepare_network_contact_island_batch_with_backend_reservation_and_observer(
                        frame,
                        &phase,
                        SimulationContactIslandMapSource::Present(&valid),
                        &mut compile_backend,
                        move |records, count| {
                            if dishonest {
                                Ok(())
                            } else {
                                let _ = (records, count);
                                Err(())
                            }
                        },
                        |_, _, _| panic!("record refusal must precede island reservation"),
                        |_, _| {},
                    )
                    .err()
                    .expect("record allocation refusal");
                assert_eq!(
                    failure.error,
                    SimulationNetworkContactIslandPreparationError::RecordAllocationFailed {
                        count: 2,
                    }
                );
                assert!(compile_backend.calls.is_empty());
                frame = failure.frame;
            }

            for dishonest in [false, true] {
                let failure =
                    try_prepare_network_contact_island_batch_with_backend_reservation_and_observer(
                        frame,
                        &phase,
                        SimulationContactIslandMapSource::Present(&valid),
                        &mut compile_backend,
                        |records, count| records.try_reserve_exact(count).map_err(|_| ()),
                        move |_, entries, count| {
                            if dishonest {
                                Ok(())
                            } else {
                                let _ = (entries, count);
                                Err(())
                            }
                        },
                        |_, _| {},
                    )
                    .err()
                    .expect("island allocation refusal");
                assert_eq!(
                    failure.error,
                    SimulationNetworkContactIslandPreparationError::IslandAllocationFailed {
                        island: 2,
                        count: 2,
                    }
                );
                assert!(compile_backend.calls.is_empty());
                frame = failure.frame;
            }

            let mut failing_backend = ContactCompileTraceBackend {
                calls: Vec::new(),
                fail_on: Some(1),
            };
            let failure = try_prepare_network_contact_island_batch_with_backend(
                frame,
                &phase,
                SimulationContactIslandMapSource::Present(&valid),
                &mut failing_backend,
            )
            .err()
            .expect("later numeric refusal");
            assert_eq!(
                failure.error,
                SimulationNetworkContactIslandPreparationError::Numeric {
                    contact: SimulationContactIdentity {
                        manager: source.manager,
                        frame: source.frame,
                        dense_ordinal: 1,
                    },
                    source: "injected Contact compiler estimate failure",
                }
            );
            assert_eq!(failing_backend.calls.len(), 2);
            assert_eq!(
                failure
                    .frame
                    .domain
                    .records
                    .iter()
                    .map(|record| record.input.test_words())
                    .collect::<Vec<_>>(),
                input_words
            );

            let mut retry_backend = ContactCompileTraceBackend::passing();
            let batch = try_prepare_network_contact_island_batch_with_backend(
                failure.frame,
                &phase,
                SimulationContactIslandMapSource::Present(&valid),
                &mut retry_backend,
            )
            .unwrap_or_else(|_| panic!("retry succeeds"));
            assert_eq!(batch.records.len(), 2);
            assert_eq!(batch.islands[2].len(), 2);
            assert_eq!(retry_backend.calls.len(), 2);
        }

        #[test]
        #[allow(
            clippy::too_many_lines,
            reason = "one self-pair fixture follows preparation through both live solver visits"
        )]
        fn repeated_self_pair_keeps_all_four_roles_and_b_wins_each_live_row() {
            let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
            let mut contacts =
                SimulationManagerContactDomain::try_new(&mut pool, 2040).expect("Contact domain");
            let body = contacts.static_body;
            pool.begin_dynamic_update_frame(1.0 / 60.0);
            let initial = [
                [0.125, -0.25, 0.375, -0.5],
                [0.625, -0.75, 0.875, -1.0],
                [1.125, -1.25, 1.375, -1.5],
                [1.625, -1.75, 1.875, -2.0],
            ];
            pool.try_with_solver_delta_row_mut(body.observation(), |rows| *rows = initial)
                .expect("seed self-pair RF");
            let mut queue = SimulationNetworkContactQueue::try_new(&pool).expect("queue");
            for point_x in [0.25, -0.5] {
                queue
                    .try_enqueue(&pool, request(body.observation(), point_x, 0.125))
                    .expect("self-pair request");
            }
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut destination = contacts
                .try_begin_contact_frame(&phase)
                .expect("Contact frame");
            let mut backend = TraceBackend::passing();
            let prepared = try_prepare_network_contact_contribution_with_backend(
                queue,
                &destination,
                &phase,
                &mut backend,
            )
            .unwrap_or_else(|_| panic!("self-pair contribution"));
            let roles = SimulationPreparedNetworkContactRoles {
                table_body_a: body.observation(),
                table_body_b: body.observation(),
                solver_side_0: body.observation(),
                solver_side_1: body.observation(),
            };
            assert_eq!(prepared.entries[0].roles, roles);
            assert_eq!(prepared.entries[1].roles, roles);
            assert_eq!(prepared.entries[0].queue_ordinal, 0);
            assert_eq!(prepared.entries[1].queue_ordinal, 1);
            for entry in &prepared.entries {
                let rows = entry.input.test_words();
                assert_eq!(rows[0][3], 1);
                assert_eq!(rows[1][3], 1);
                assert_eq!(rows[6][3], 1);
                assert_eq!(rows[7][3], 1);
            }
            destination
                .try_append_network_contact_contribution(prepared, &phase)
                .unwrap_or_else(|_| panic!("append self pairs"));
            let sealed = destination
                .try_seal(&phase)
                .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error));
            let mut compile_backend = ContactCompileTraceBackend::passing();
            let mut batch = try_prepare_network_contact_island_batch_with_backend(
                sealed,
                &phase,
                SimulationContactIslandMapSource::Null {
                    frame: phase.frame_identity(),
                },
                &mut compile_backend,
            )
            .unwrap_or_else(|_| panic!("self-pair island batch"));
            let mut expected_0 = batch.records[0].record;
            let first = solve_contact_horus_record_host(&mut expected_0, initial, initial);
            let after_first_b_wins = first.solver_delta_b;
            let mut expected_1 = batch.records[1].record;
            let second = solve_contact_horus_record_host(
                &mut expected_1,
                after_first_b_wins,
                after_first_b_wins,
            );

            try_solve_prepared_network_contact_horus_island_sweep(&mut batch, 0, &mut phase)
                .expect("self-pair Contact sweep");
            assert_eq!(
                contact_record_words(&batch.records[0].record),
                contact_record_words(&expected_0)
            );
            assert_eq!(
                contact_record_words(&batch.records[1].record),
                contact_record_words(&expected_1)
            );
            drop(phase);
            let actual = pool
                .try_with_solver_delta_row(body.observation(), |rows| *rows)
                .expect("read self-pair RF");
            assert_eq!(
                solver_delta_words(actual),
                solver_delta_words(second.solver_delta_b)
            );
        }

        #[test]
        fn binding_word_helper_is_nonzero_checked_for_every_role() {
            for kind in [
                SimulationNetworkContactPreparedBindingKind::TableBodyA,
                SimulationNetworkContactPreparedBindingKind::TableBodyB,
                SimulationNetworkContactPreparedBindingKind::SolverSide0,
                SimulationNetworkContactPreparedBindingKind::SolverSide1,
            ] {
                assert_eq!(checked_binding_word(0, kind), Ok(1));
                assert_eq!(checked_binding_word(u32::MAX - 1, kind), Ok(u32::MAX));
                assert_eq!(checked_binding_word(u32::MAX, kind), Err((kind, u32::MAX)));
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimulationContactReciprocalSite {
    EffectiveMassFourLane,
}

/// Private numeric seam for Contact Build's one unrefined Xenon estimate.
trait SimulationContactNumericBackend {
    type Error;

    fn reciprocal_estimate(
        &mut self,
        site: SimulationContactReciprocalSite,
        denominator: ContactVector,
    ) -> Result<ContactVector, Self::Error>;
}

/// Executable host policy for Contact Build's one reciprocal estimate at
/// `0x82ae1e30`: ordinary floating-point reciprocal, not a Xenon `vrefp` oracle.
/// No clamping, zero substitution, or refinement is added. The recovered
/// compiler's fused grouping and estimate call site remain unchanged.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "ordinary-finite Contact compiler backend awaiting collision-frame scheduling"
    )
)]
struct HostContactNumericBackend;

impl SimulationContactNumericBackend for HostContactNumericBackend {
    type Error = std::convert::Infallible;

    fn reciprocal_estimate(
        &mut self,
        _site: SimulationContactReciprocalSite,
        denominator: ContactVector,
    ) -> Result<ContactVector, Self::Error> {
        Ok(denominator.map(f32::recip))
    }
}

#[cfg(test)]
const fn contact_build_native_store_schedule() -> [(u32, usize); 16] {
    [
        (0x82ae_1ae0, 7),
        (0x82ae_1ae8, 0),
        (0x82ae_1aec, 10),
        (0x82ae_1af4, 13),
        (0x82ae_1c30, 1),
        (0x82ae_1d4c, 11),
        (0x82ae_1d64, 8),
        (0x82ae_1d8c, 9),
        (0x82ae_1d90, 14),
        (0x82ae_1d94, 12),
        (0x82ae_1dbc, 15),
        (0x82ae_1df0, 5),
        (0x82ae_1e74, 4),
        (0x82ae_1e78, 2),
        (0x82ae_1e7c, 3),
        (0x82ae_1e98, 6),
    ]
}

/// Exact host value layout corresponding to native `rw::physics::ContactJacobian`.
///
/// Native consumer: `0x82ae2990..0x82ae2c48`; PDB size: `0x100` bytes.
/// Pointer/index-bearing words remain private inert tokens. Other private fourth
/// words preserve packed numeric bits without giving them an unproven public
/// semantic or ownership role.
#[repr(C, align(16))]
#[derive(Clone, Copy, PartialEq)]
pub struct SimulationContactJacobian {
    pub attachment_offset_a: [f32; 3],
    solver_delta_a: SolverDeltaRowToken,
    pub attachment_offset_b: [f32; 3],
    solver_delta_b: SolverDeltaRowToken,
    pub contact_transpose_right: [f32; 3],
    nb_word: u32,
    pub contact_transpose_up: [f32; 3],
    pub static_friction: f32,
    pub contact_transpose_at: [f32; 3],
    pub dynamic_friction: f32,
    pub position_impulse: [f32; 3],
    pub positional_impulse: f32,
    pub position_limit: [f32; 3],
    pub distance: f32,
    pub contact_right: [f32; 3],
    body_a: ContactBodyToken,
    /// Native PDB `m_IRAC_Ri`; its acronym is kept unexpanded.
    pub irac_right: [f32; 3],
    pub inverse_mass_a: f32,
    /// Native PDB `m_IRBC_Ri`; its acronym is kept unexpanded.
    pub irbc_right: [f32; 3],
    pub inverse_mass_b: f32,
    pub contact_up: [f32; 3],
    body_b: ContactBodyToken,
    /// Native PDB `m_IRAC_Up`; its acronym is kept unexpanded.
    pub irac_up: [f32; 3],
    pad_0: f32,
    /// Native PDB `m_IRBC_Up`; its acronym is kept unexpanded.
    pub irbc_up: [f32; 3],
    pad_1: f32,
    pub contact_at: [f32; 3],
    tag_word: u32,
    /// Native PDB `m_IRAC_At`; its acronym is kept unexpanded.
    pub irac_at: [f32; 3],
    pad_2: f32,
    /// Native PDB `m_IRBC_At`; its acronym is kept unexpanded.
    pub irbc_at: [f32; 3],
    pad_3: f32,
}

impl SimulationContactJacobian {
    /// Constructs the inert all-zero host value.
    ///
    /// This is a safe-host construction boundary, not a claim that native
    /// Contact Build publishes an observable all-zero record at one matching
    /// instruction boundary.
    #[must_use]
    pub const fn zeroed() -> Self {
        Self {
            attachment_offset_a: [0.0; 3],
            solver_delta_a: SolverDeltaRowToken(0),
            attachment_offset_b: [0.0; 3],
            solver_delta_b: SolverDeltaRowToken(0),
            contact_transpose_right: [0.0; 3],
            nb_word: 0,
            contact_transpose_up: [0.0; 3],
            static_friction: 0.0,
            contact_transpose_at: [0.0; 3],
            dynamic_friction: 0.0,
            position_impulse: [0.0; 3],
            positional_impulse: 0.0,
            position_limit: [0.0; 3],
            distance: 0.0,
            contact_right: [0.0; 3],
            body_a: ContactBodyToken(0),
            irac_right: [0.0; 3],
            inverse_mass_a: 0.0,
            irbc_right: [0.0; 3],
            inverse_mass_b: 0.0,
            contact_up: [0.0; 3],
            body_b: ContactBodyToken(0),
            irac_up: [0.0; 3],
            pad_0: 0.0,
            irbc_up: [0.0; 3],
            pad_1: 0.0,
            contact_at: [0.0; 3],
            tag_word: 0,
            irac_at: [0.0; 3],
            pad_2: 0.0,
            irbc_at: [0.0; 3],
            pad_3: 0.0,
        }
    }

    /// Reports whether both private RF-row tokens have been installed.
    #[must_use]
    pub const fn has_solver_delta_bindings(&self) -> bool {
        self.solver_delta_a.0 != 0 && self.solver_delta_b.0 != 0
    }
}

impl Default for SimulationContactJacobian {
    fn default() -> Self {
        Self::zeroed()
    }
}

/// Compiles one copied native Contact record into a completed Jacobian value.
///
/// Native range: `Simulation::ContactBatchBuild` at
/// `0x82ae1958..0x82ae1ea4`. All sixteen source vectors are already copied, so
/// the in-place native overwrite chronology cannot destroy later inputs. The
/// backend observes exactly one four-lane unrefined reciprocal-estimate site.
/// A backend failure returns no record, but any backend-owned trace/callback
/// effects already performed are intentionally not rolled back.
///
/// This function owns no Contact producer, pool occurrence, RF row, island
/// table, or publication authority. Ordinary-finite fused order and packed W
/// lanes are in scope. Xenon estimate bits, compare/select behavior for NaNs
/// and signed zero, denormals, overflow, FPSCR, and exceptions remain outside
/// the claim.
#[allow(
    clippy::similar_names,
    clippy::too_many_lines,
    reason = "source-shaped side and coefficient names preserve native roles"
)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for the checked Contact compiler batch")
)]
fn build_contact_jacobian_host<B: SimulationContactNumericBackend>(
    input: SimulationContactBuildInput,
    time_step: f32,
    backend: &mut B,
) -> Result<SimulationContactJacobian, B::Error> {
    let position_a = input.position_a_body_a.vector();
    let position_b = input.position_b_body_b.vector();
    let right = input.contact_right_restitution.vector();
    let up = input.contact_up_static_friction.vector();
    let at = input.contact_at_dynamic_friction.vector();
    let contact_velocity = input.contact_velocity_tag.vector();
    let center_0 = input.side_0_com_rf_token.vector();
    let center_1 = input.side_1_com_rf_token.vector();
    let inertia_full_0_raw = input.side_0_inertia_full_inverse_mass.vector();
    let inertia_full_1_raw = input.side_1_inertia_full_inverse_mass.vector();
    let inertia_split_0_raw = input.side_0_inertia_split_state.vector();
    let inertia_split_1_raw = input.side_1_inertia_split_state.vector();
    let force_0 = input.side_0_force_kine.vector();
    let force_1 = input.side_1_force_kine.vector();
    let torque_0 = input.side_0_torque_cool.vector();
    let torque_1 = input.side_1_torque_cool.vector();

    let zero = [0.0; 4];
    let step = [time_step; 4];
    let step_squared = contact_build_mul(step, step);
    let radius_0_raw = vector_sub(position_a, center_0);
    let radius_1_raw = vector_sub(position_b, center_1);
    let radius_0 = contact_build_replace_word(radius_0_raw, input.side_0_com_rf_token.word);
    let radius_1 = contact_build_replace_word(radius_1_raw, input.side_1_com_rf_token.word);

    let state_0 = input.side_0_inertia_split_state.word;
    let state_1 = input.side_1_inertia_split_state.word;
    let active_0 = state_0 & 4 == 4;
    let active_1 = state_1 & 4 == 4;
    let inertia_full_0 = contact_build_gate(inertia_full_0_raw, active_0);
    let inertia_split_0 = contact_build_gate(inertia_split_0_raw, active_0);
    let inertia_full_1 = contact_build_gate(inertia_full_1_raw, active_1);
    let inertia_split_1 = contact_build_gate(inertia_split_1_raw, active_1);

    let e_right_0 = contact_build_cross(radius_0_raw, right);
    let e_up_0 = contact_build_cross(radius_0_raw, up);
    let e_at_0 = contact_build_cross(radius_0_raw, at);
    let e_right_1 = contact_build_cross(radius_1_raw, right);
    let e_up_1 = contact_build_cross(radius_1_raw, up);
    let e_at_1 = contact_build_cross(radius_1_raw, at);

    let irac_right = contact_build_tensor(inertia_full_0, inertia_split_0, e_right_0);
    let irac_up = contact_build_tensor(inertia_full_0, inertia_split_0, e_up_0);
    let irac_at = contact_build_tensor(inertia_full_0, inertia_split_0, e_at_0);
    let irbc_right = contact_build_tensor(inertia_full_1, inertia_split_1, e_right_1);
    let irbc_up = contact_build_tensor(inertia_full_1, inertia_split_1, e_up_1);
    let irbc_at = contact_build_tensor(inertia_full_1, inertia_split_1, e_at_1);

    // Both source-shaped point expressions execute before their independent
    // bit-four selections.
    let point_0 = contact_build_madd(
        contact_build_zxy(radius_0_raw),
        contact_build_nmsub(
            contact_build_yzx(radius_0_raw),
            force_0,
            contact_build_zxy(torque_0),
        ),
        contact_build_yzx(torque_0),
    );
    let point_1 = contact_build_madd(
        contact_build_zxy(radius_1_raw),
        contact_build_nmsub(
            contact_build_yzx(radius_1_raw),
            force_1,
            contact_build_zxy(torque_1),
        ),
        contact_build_yzx(torque_1),
    );
    let relative_point = vector_sub(
        contact_build_gate(point_1, active_1),
        contact_build_gate(point_0, active_0),
    );
    let separation = vector_sub(position_b, position_a);
    let relative_h2 = contact_build_madd(relative_point, zero, step_squared);
    let contact_h = contact_build_madd(contact_velocity, zero, step);
    let predicted = vector_add(contact_h, vector_add(separation, relative_h2));
    let restitution = contact_build_nmsub(contact_h, zero, [right[3]; 4]);

    let product_right =
        contact_build_component_product(e_right_0, irac_right, e_right_1, irbc_right);
    let product_up = contact_build_component_product(e_up_0, irac_up, e_up_1, irbc_up);
    let product_at = contact_build_component_product(e_at_0, irac_at, e_at_1, irbc_at);
    let denominator_x = [
        product_right[0],
        product_up[0],
        product_at[0],
        product_at[1],
    ];
    let denominator_y = [
        product_right[1],
        product_up[1],
        product_at[1],
        product_at[0],
    ];
    let denominator_z = [
        product_right[2],
        product_up[2],
        product_at[2],
        product_at[0],
    ];
    // Exact raw grouping: (z + (x + im0)) + (y + im1).
    let denominator = vector_add(
        vector_add(
            denominator_z,
            vector_add(denominator_x, [inertia_full_0[3]; 4]),
        ),
        vector_add(denominator_y, [inertia_full_1[3]; 4]),
    );
    let reciprocal = backend.reciprocal_estimate(
        SimulationContactReciprocalSite::EffectiveMassFourLane,
        denominator,
    )?;

    let transposed_right = contact_build_madd([right[0], up[0], at[0], at[1]], zero, reciprocal);
    let transposed_up = contact_build_madd([right[1], up[1], at[1], at[0]], zero, reciprocal);
    let transposed_at = contact_build_madd([right[2], up[2], at[2], at[0]], zero, reciprocal);
    let distance_scaled = contact_build_madd(
        contact_build_axis_dot_packet(right, up, at, separation),
        zero,
        reciprocal,
    );
    let restitution_scaled = contact_build_madd(
        contact_build_axis_dot_packet(right, up, at, restitution),
        zero,
        reciprocal,
    );
    let predicted_scaled = contact_build_madd(
        contact_build_axis_dot_packet(right, up, at, predicted),
        zero,
        reciprocal,
    );
    let limit_candidate =
        contact_build_normal_limit_candidate(distance_scaled, restitution_scaled, predicted_scaled);
    let position_limit = contact_build_replace_word(
        vector_sub(predicted_scaled, limit_candidate),
        distance_scaled[0].to_bits(),
    );

    // Native producer scratch side 0/1 feeds these PDB A/B numeric slots, but
    // Contact Build never proves either scratch side equals the metadata body
    // token with the matching letter. Keep numeric scratch and binding-token
    // provenance independent until every producer family is proven.
    Ok(SimulationContactJacobian {
        attachment_offset_a: first3(radius_0),
        solver_delta_a: SolverDeltaRowToken(radius_0[3].to_bits()),
        attachment_offset_b: first3(radius_1),
        solver_delta_b: SolverDeltaRowToken(radius_1[3].to_bits()),
        contact_transpose_right: first3(transposed_right),
        nb_word: (state_0 | state_1) & 8,
        contact_transpose_up: first3(transposed_up),
        static_friction: f32::from_bits(input.contact_up_static_friction.word),
        contact_transpose_at: first3(transposed_at),
        dynamic_friction: f32::from_bits(input.contact_at_dynamic_friction.word),
        position_impulse: [0.0; 3],
        positional_impulse: 0.0,
        position_limit: first3(position_limit),
        distance: position_limit[3],
        contact_right: first3(right),
        body_a: ContactBodyToken(input.position_a_body_a.word),
        irac_right: first3(irac_right),
        inverse_mass_a: inertia_full_0[3],
        irbc_right: first3(irbc_right),
        inverse_mass_b: inertia_full_1[3],
        contact_up: first3(up),
        body_b: ContactBodyToken(input.position_b_body_b.word),
        irac_up: first3(irac_up),
        pad_0: irac_up[3],
        irbc_up: first3(irbc_up),
        pad_1: irbc_up[3],
        contact_at: first3(at),
        tag_word: input.contact_velocity_tag.word,
        irac_at: first3(irac_at),
        pad_2: irac_at[3],
        irbc_at: first3(irbc_at),
        pad_3: irbc_at[3],
    })
}

fn contact_build_mul(left: ContactVector, right: ContactVector) -> ContactVector {
    std::array::from_fn(|lane| left[lane] * right[lane])
}

/// Packet notation `M(a,b,c) = a*c+b`.
fn contact_build_madd(
    multiplier: ContactVector,
    addend: ContactVector,
    multiplicand: ContactVector,
) -> ContactVector {
    horus_madd(multiplier, multiplicand, addend)
}

/// Packet notation `N(a,b,c) = b-a*c`.
fn contact_build_nmsub(
    multiplier: ContactVector,
    addend: ContactVector,
    multiplicand: ContactVector,
) -> ContactVector {
    horus_nmsub(multiplier, multiplicand, addend)
}

fn contact_build_yzx(value: ContactVector) -> ContactVector {
    [value[1], value[2], value[0], value[3]]
}

fn contact_build_zxy(value: ContactVector) -> ContactVector {
    [value[2], value[0], value[1], value[3]]
}

fn contact_build_replace_word(mut value: ContactVector, word: u32) -> ContactVector {
    value[3] = f32::from_bits(word);
    value
}

fn contact_build_gate(value: ContactVector, enabled: bool) -> ContactVector {
    if enabled { value } else { [0.0; 4] }
}

fn contact_build_cross(left: ContactVector, right: ContactVector) -> ContactVector {
    let first = contact_build_madd(contact_build_yzx(left), [0.0; 4], contact_build_zxy(right));
    contact_build_nmsub(contact_build_zxy(left), first, contact_build_yzx(right))
}

fn contact_build_tensor(
    full: ContactVector,
    split: ContactVector,
    value: ContactVector,
) -> ContactVector {
    let packed_y = [full[1], split[1], split[2], full[3]];
    let packed_z = [full[2], split[2], split[0], full[3]];
    let first = contact_build_madd(full, [0.0; 4], [value[0]; 4]);
    let second = contact_build_madd(packed_y, first, [value[1]; 4]);
    contact_build_madd(packed_z, second, [value[2]; 4])
}

fn contact_build_component_product(
    e_0: ContactVector,
    inertia_0: ContactVector,
    e_1: ContactVector,
    inertia_1: ContactVector,
) -> ContactVector {
    let side_0 = contact_build_madd(e_0, [0.0; 4], inertia_0);
    contact_build_madd(e_1, side_0, inertia_1)
}

fn contact_build_axis_dot_packet(
    right: ContactVector,
    up: ContactVector,
    at: ContactVector,
    value: ContactVector,
) -> ContactVector {
    // The dead residue lane follows raw AX/AY/AZ W = At.y/At.x/At.x.
    let axis_x = [right[0], up[0], at[0], at[1]];
    let axis_y = [right[1], up[1], at[1], at[0]];
    let axis_z = [right[2], up[2], at[2], at[0]];
    let first = contact_build_madd(axis_x, [0.0; 4], [value[0]; 4]);
    let second = contact_build_madd(axis_y, first, [value[1]; 4]);
    contact_build_madd(axis_z, second, [value[2]; 4])
}

fn contact_build_normal_limit_candidate(
    distance_scaled: ContactVector,
    restitution_scaled: ContactVector,
    predicted_scaled: ContactVector,
) -> ContactVector {
    let summed = vector_add(distance_scaled, restitution_scaled);
    let mut candidate = [0.0; 4];
    for lane in 0..4 {
        candidate[lane] = if restitution_scaled[lane] >= 0.0 {
            0.0
        } else {
            summed[lane]
        };
        candidate[lane] = if distance_scaled[lane] >= 0.0 {
            restitution_scaled[lane]
        } else {
            candidate[lane]
        };
        candidate[lane] = if 0.0 >= predicted_scaled[lane] {
            0.0
        } else {
            candidate[lane]
        };
    }
    [candidate[0], 0.0, 0.0, 0.0]
}

/// Staged value result of one indirect-island Horus Contact solve.
///
/// Complete copied blocks preserve native A-then-B self-pair resolution and
/// let the checked executor retain sequential shared-body visibility
/// without ever forming two mutable references to one body slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationContactHorusRows {
    pub solver_delta_a: [[f32; 4]; 4],
    pub solver_delta_b: [[f32; 4]; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimulationContactHorusStore {
    Accumulator,
    A3,
    B3,
    A1,
    B1,
    A2,
    B2,
    A0,
    B0,
}

/// Applies one indirect-island Horus Contact consumer to copied RF inputs.
///
/// Native per-record anchors are `0x82ae2990..0x82ae2c48`. The helper retains
/// separate positional and velocity point channels, the nonconventional
/// static-test/dynamic-fallback friction graph, full four-lane arithmetic,
/// Right/Up/At fused association, the accumulator store, and complete staged
/// A/B RF blocks. The native observable store sequence is accumulator, A3,
/// B3, A1, B1, A2, B2, A0, B0.
///
/// This is an ordinary-finite host model. Xenon compare/select behavior for
/// NaNs and signed zero, denormal handling, overflow, FPSCR effects, VMX
/// exceptional fused behavior, and native pointer-derived W parity remain
/// outside the claim. Private inert token words still participate in packed W
/// arithmetic exactly as stored in this host value.
pub(crate) fn solve_contact_horus_record_host(
    jacobian: &mut SimulationContactJacobian,
    solver_delta_a: [[f32; 4]; 4],
    solver_delta_b: [[f32; 4]; 4],
) -> SimulationContactHorusRows {
    solve_contact_horus_record_with_observer(
        jacobian,
        solver_delta_a,
        solver_delta_b,
        |_, _, _, _| {},
    )
}

#[allow(
    clippy::similar_names,
    clippy::too_many_lines,
    reason = "source-shaped A/B names and uninterrupted native operation order are intentional"
)]
fn solve_contact_horus_record_with_observer(
    jacobian: &mut SimulationContactJacobian,
    mut solver_delta_a: [[f32; 4]; 4],
    mut solver_delta_b: [[f32; 4]; 4],
    mut observe_store: impl FnMut(
        SimulationContactHorusStore,
        &SimulationContactJacobian,
        &[[f32; 4]; 4],
        &[[f32; 4]; 4],
    ),
) -> SimulationContactHorusRows {
    let zero = [0.0; 4];
    let attachment_a = pack_xyz_word(
        jacobian.attachment_offset_a,
        f32::from_bits(jacobian.solver_delta_a.0),
    );
    let attachment_b = pack_xyz_word(
        jacobian.attachment_offset_b,
        f32::from_bits(jacobian.solver_delta_b.0),
    );

    let point_a_positional = horus_cross_add(solver_delta_a[3], attachment_a, solver_delta_a[1]);
    let point_b_positional = horus_cross_add(solver_delta_b[3], attachment_b, solver_delta_b[1]);
    let point_a_velocity = horus_cross_add(solver_delta_a[2], attachment_a, solver_delta_a[0]);
    let point_b_velocity = horus_cross_add(solver_delta_b[2], attachment_b, solver_delta_b[0]);
    let position_relative = vector_sub(point_b_positional, point_a_positional);
    let velocity_relative = vector_sub(point_b_velocity, point_a_velocity);
    let total_relative = vector_add(velocity_relative, position_relative);

    let old_impulse = pack_xyz_word(jacobian.position_impulse, jacobian.positional_impulse);
    let position_limit = pack_xyz_word(jacobian.position_limit, jacobian.distance);
    let friction_base = [0.0, old_impulse[0], old_impulse[0], 0.0];
    let friction_edge = [f32::MAX, 0.0, 0.0, f32::MAX];
    let static_friction = [jacobian.static_friction; 4];
    let dynamic_friction = [jacobian.dynamic_friction; 4];
    let upper_dynamic = horus_madd(dynamic_friction, friction_base, friction_edge);
    let lower_static = horus_nmsub(static_friction, friction_base, zero);
    let upper_static = horus_madd(static_friction, friction_base, friction_edge);
    let lower_dynamic = horus_nmsub(dynamic_friction, friction_base, zero);

    let contact_transpose = [
        pack_xyz_word(
            jacobian.contact_transpose_right,
            jacobian.contact_transpose_right[0],
        ),
        pack_xyz_word(
            jacobian.contact_transpose_up,
            jacobian.contact_transpose_up[0],
        ),
        pack_xyz_word(
            jacobian.contact_transpose_at,
            jacobian.contact_transpose_at[0],
        ),
    ];
    let projection_scalars = [
        [
            total_relative[0],
            total_relative[0],
            total_relative[0],
            position_relative[0],
        ],
        [
            total_relative[1],
            total_relative[1],
            total_relative[1],
            position_relative[1],
        ],
        [
            total_relative[2],
            total_relative[2],
            total_relative[2],
            position_relative[2],
        ],
    ];
    let projection_base = vector_add(old_impulse, position_limit);
    let projected_right = horus_madd(contact_transpose[0], projection_scalars[0], projection_base);
    let projected_up = horus_madd(contact_transpose[1], projection_scalars[1], projected_right);
    let candidate = horus_madd(contact_transpose[2], projection_scalars[2], projected_up);

    let after_lower = vector_select_ge(candidate, lower_static, candidate, lower_dynamic);
    let new_impulse = vector_select_ge(upper_static, candidate, after_lower, upper_dynamic);
    let impulse_change = vector_sub(new_impulse, old_impulse);
    jacobian.position_impulse = first3(new_impulse);
    jacobian.positional_impulse = new_impulse[3];
    observe_store(
        SimulationContactHorusStore::Accumulator,
        jacobian,
        &solver_delta_a,
        &solver_delta_b,
    );

    let contact_axes = [
        pack_xyz_word(jacobian.contact_right, f32::from_bits(jacobian.body_a.0)),
        pack_xyz_word(jacobian.contact_up, f32::from_bits(jacobian.body_b.0)),
        pack_xyz_word(jacobian.contact_at, f32::from_bits(jacobian.tag_word)),
    ];
    let inverse_inertia_a = [
        pack_xyz_word(jacobian.irac_right, jacobian.inverse_mass_a),
        pack_xyz_word(jacobian.irac_up, jacobian.pad_0),
        pack_xyz_word(jacobian.irac_at, jacobian.pad_2),
    ];
    let inverse_inertia_b = [
        pack_xyz_word(jacobian.irbc_right, jacobian.inverse_mass_b),
        pack_xyz_word(jacobian.irbc_up, jacobian.pad_1),
        pack_xyz_word(jacobian.irbc_at, jacobian.pad_3),
    ];
    let positional_linear = horus_madd(contact_axes[0], [impulse_change[3]; 4], zero);
    let linear = horus_columns_from(contact_axes, impulse_change, zero);

    let new_a_3 = horus_madd(
        inverse_inertia_a[0],
        [impulse_change[3]; 4],
        solver_delta_a[3],
    );
    let new_b_3 = horus_nmsub(
        inverse_inertia_b[0],
        [impulse_change[3]; 4],
        solver_delta_b[3],
    );
    let new_a_1 = horus_madd(
        positional_linear,
        [jacobian.inverse_mass_a; 4],
        solver_delta_a[1],
    );
    let new_b_1 = horus_nmsub(
        positional_linear,
        [jacobian.inverse_mass_b; 4],
        solver_delta_b[1],
    );
    let new_a_2 = horus_columns_from(inverse_inertia_a, impulse_change, solver_delta_a[2]);
    let new_b_2 = horus_negative_columns_from(inverse_inertia_b, impulse_change, solver_delta_b[2]);
    let new_a_0 = horus_madd(linear, [jacobian.inverse_mass_a; 4], solver_delta_a[0]);
    let new_b_0 = horus_nmsub(linear, [jacobian.inverse_mass_b; 4], solver_delta_b[0]);

    solver_delta_a[3] = new_a_3;
    observe_store(
        SimulationContactHorusStore::A3,
        jacobian,
        &solver_delta_a,
        &solver_delta_b,
    );
    solver_delta_b[3] = new_b_3;
    observe_store(
        SimulationContactHorusStore::B3,
        jacobian,
        &solver_delta_a,
        &solver_delta_b,
    );
    solver_delta_a[1] = new_a_1;
    observe_store(
        SimulationContactHorusStore::A1,
        jacobian,
        &solver_delta_a,
        &solver_delta_b,
    );
    solver_delta_b[1] = new_b_1;
    observe_store(
        SimulationContactHorusStore::B1,
        jacobian,
        &solver_delta_a,
        &solver_delta_b,
    );
    solver_delta_a[2] = new_a_2;
    observe_store(
        SimulationContactHorusStore::A2,
        jacobian,
        &solver_delta_a,
        &solver_delta_b,
    );
    solver_delta_b[2] = new_b_2;
    observe_store(
        SimulationContactHorusStore::B2,
        jacobian,
        &solver_delta_a,
        &solver_delta_b,
    );
    solver_delta_a[0] = new_a_0;
    observe_store(
        SimulationContactHorusStore::A0,
        jacobian,
        &solver_delta_a,
        &solver_delta_b,
    );
    solver_delta_b[0] = new_b_0;
    observe_store(
        SimulationContactHorusStore::B0,
        jacobian,
        &solver_delta_a,
        &solver_delta_b,
    );

    SimulationContactHorusRows {
        solver_delta_a,
        solver_delta_b,
    }
}

fn first3(value: [f32; 4]) -> [f32; 3] {
    [value[0], value[1], value[2]]
}

fn vector_add(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane] + right[lane])
}

fn vector_sub(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane] - right[lane])
}

fn vector_select_ge(
    compare_left: [f32; 4],
    compare_right: [f32; 4],
    if_true: [f32; 4],
    if_false: [f32; 4],
) -> [f32; 4] {
    std::array::from_fn(|lane| {
        if compare_left[lane] >= compare_right[lane] {
            if_true[lane]
        } else {
            if_false[lane]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{
        ContactBodyToken, SimulationAssemblyGroundContactEstimateSite,
        SimulationAssemblyGroundContactInput, SimulationAssemblyGroundContactNumericBackend,
        SimulationAssemblyGroundMaterialInput, SimulationContactBuildInput,
        SimulationContactCommonBodyInput, SimulationContactHorusStore, SimulationContactJacobian,
        SimulationContactNumericBackend, SimulationContactReciprocalSite,
        SimulationNetworkContactDrainEstimateSite, SimulationNetworkContactDrainInput,
        SimulationNetworkContactDrainNumericBackend, SimulationNetworkContactSelectedBodyInput,
        SolverDeltaRowToken, build_assembly_ground_contact_input, build_contact_jacobian_host,
        build_network_contact_input, contact_build_native_store_schedule,
        select_assembly_ground_material, solve_contact_horus_record_host,
        solve_contact_horus_record_with_observer,
    };
    use std::mem::{align_of, offset_of, size_of};

    const RECORD_WORDS: [[u32; 4]; 16] = [
        [0x3e80_0000, 0xbecc_cccd, 0x3f19_999a, 0x3cfd_f3b6],
        [0xbe99_999a, 0x3ee6_6666, 0xbe4c_cccd, 0xbd23_d70a],
        [0x3f4c_cccd, 0xbe80_0000, 0x3e19_999a, 0x4000_0000],
        [0xbdcc_cccd, 0x3f33_3333, 0xbeb3_3333, 0x3f19_999a],
        [0x3e4c_cccd, 0x3e99_999a, 0xbf66_6666, 0x3ecc_cccd],
        [0x3f40_0000, 0x3dcc_cccd, 0xbe4c_cccd, 0x3d4c_cccd],
        [0x3ca3_d70a, 0xbcf5_c28f, 0x3d23_d70a, 0xbc23_d70a],
        [0x3f66_6666, 0xbe4c_cccd, 0x3dcc_cccd, 0x3c75_c28f],
        [0x3f19_999a, 0x3d4c_cccd, 0xbda3_d70a, 0x3f00_0000],
        [0x3ecc_cccd, 0xbcf5_c28f, 0x3d8f_5c29, 0x3e80_0000],
        [0x3dcc_cccd, 0x3f4c_cccd, 0xbe19_999a, 0x3c8b_4396],
        [0x3ca3_d70a, 0x3f33_3333, 0x3d23_d70a, 0xbcac_0831],
        [0xbc23_d70a, 0x3f00_0000, 0x3d75_c28f, 0x3ced_9168],
        [0xbd4c_cccd, 0x3df5_c28f, 0x3f73_3333, 0x3cbc_6a7f],
        [0x3cf5_c28f, 0xbca3_d70a, 0x3f4c_cccd, 0xbcdd_2f1b],
        [0x3d23_d70a, 0x3c23_d70a, 0x3f26_6666, 0x3d07_2b02],
    ];

    const BUILD_INPUT_WORDS: [[u32; 4]; 16] = [
        [0x3f3a_e148, 0xbed1_eb85, 0x3fa2_8f5c, 0x1111_1111],
        [0xbf05_1eb8, 0x3f70_a3d7, 0xbf8e_147b, 0x2222_2222],
        [0x3eb8_51ec, 0x3ef5_c28f, 0x3f4c_cccd, 0x3ebd_70a4],
        [0xbf4c_cccd, 0x3f19_999a, 0x0000_0000, 0x3f1c_28f6],
        [0x3ef5_c28f, 0x3f23_d70a, 0xbf19_999a, 0x3e94_7ae1],
        [0x3e2e_147b, 0xbdb8_51ec, 0x3e6b_851f, 0x3333_3333],
        [0xbf99_999a, 0x3eb3_3333, 0x3ed1_eb85, 0x4444_4444],
        [0x3f1e_b852, 0xbf45_1eb8, 0x3faa_3d71, 0x4555_5555],
        [0x3fb3_3333, 0x3e19_999a, 0xbda3_d70a, 0x3f33_3333],
        [0x3f66_6666, 0xbdf5_c28f, 0x3e2e_147b, 0x3fa6_6666],
        [0x3de1_47ae, 0x3f8c_cccd, 0x3d75_c28f, 0x0000_0004],
        [0xbd8f_5c29, 0x3f99_999a, 0x3db8_51ec, 0x0000_0004],
        [0x3e57_0a3d, 0xbe23_d70a, 0x3df5_c28f, 0x3ed7_0a3d],
        [0xbe0f_5c29, 0x3e80_0000, 0xbd75_c28f, 0xbe9e_b852],
        [0xbe2e_147b, 0x3da3_d70a, 0x3e6b_851f, 0x3e42_8f5c],
        [0x3e42_8f5c, 0xbe57_0a3d, 0x3dcc_cccd, 0xbe8a_3d71],
    ];

    const EXPECTED_BUILD_RECORD: [[u32; 4]; 16] = [
        [0x3ff7_0a3e, 0xbf42_8f5c, 0x3f5c_28f6, 0x4444_4444],
        [0xbf91_eb85, 0x3fda_e148, 0xc01c_28f6, 0x4555_5555],
        [0x3d0e_39d5, 0xbdae_5bbe, 0x3d44_ef3d, 0x0000_0000],
        [0x3d3d_a271, 0x3d82_c4cf, 0x3d83_4a29, 0x3f1c_28f6],
        [0x3d9e_075f, 0x0000_0000, 0xbd76_2b0d, 0x3e94_7ae1],
        [0x0000_0000, 0x0000_0000, 0x0000_0000, 0x0000_0000],
        [0xbe28_2ae0, 0x3e44_e831, 0x3e2d_5ac1, 0xbe28_7f8a],
        [0x3eb8_51ec, 0x3ef5_c28f, 0x3f4c_cccd, 0x1111_1111],
        [0xbfda_ea74, 0xbfb8_2fd8, 0x3e0e_f34d, 0x3f33_3333],
        [0x4005_593f, 0xbebc_f24a, 0x3f04_1e15, 0x3fa6_6666],
        [0xbf4c_cccd, 0x3f19_999a, 0x0000_0000, 0x2222_2222],
        [0xbf5e_9e1c, 0xbf4d_1b72, 0x3d77_cedd, 0xbeea_64c1],
        [0x3f99_8dcd, 0x400e_9c24, 0x3ec0_dc34, 0x40aa_8f5d],
        [0x3ef5_c28f, 0x3f23_d70a, 0xbf19_999a, 0x3333_3333],
        [0xbcc9_080c, 0x3fe7_a4e8, 0x3e8e_3bcd, 0x4009_d29e],
        [0x3ee1_d756, 0xc01b_85e8, 0x3d05_9425, 0xc06e_c8b3],
    ];

    const BUILD_DENOMINATOR_WORDS: [u32; 4] = [0x4125_e23a, 0x4116_590b, 0x411f_bc16, 0x411d_3fc5];
    const BUILD_RECIPROCAL_WORDS: [u32; 4] = [0x3dc5_8936, 0x3dd9_f2ad, 0x3dcd_23e0, 0x3dd0_61fc];

    #[derive(Default)]
    struct ContactBuildTraceBackend {
        calls: Vec<(SimulationContactReciprocalSite, [u32; 4])>,
        fail: bool,
    }

    impl SimulationContactNumericBackend for ContactBuildTraceBackend {
        type Error = &'static str;

        fn reciprocal_estimate(
            &mut self,
            site: SimulationContactReciprocalSite,
            denominator: [f32; 4],
        ) -> Result<[f32; 4], Self::Error> {
            self.calls.push((site, denominator.map(f32::to_bits)));
            if self.fail {
                Err("injected reciprocal failure")
            } else {
                Ok(BUILD_RECIPROCAL_WORDS.map(f32::from_bits))
            }
        }
    }

    struct ContactDrainTraceBackend {
        returned: [f32; 4],
        calls: Vec<(SimulationNetworkContactDrainEstimateSite, [u32; 4])>,
        fail: bool,
    }

    impl ContactDrainTraceBackend {
        fn ones() -> Self {
            Self {
                returned: [1.0; 4],
                calls: Vec::new(),
                fail: false,
            }
        }
    }

    impl SimulationNetworkContactDrainNumericBackend for ContactDrainTraceBackend {
        type Error = &'static str;

        fn reciprocal_sqrt_estimate(
            &mut self,
            site: SimulationNetworkContactDrainEstimateSite,
            operand: [f32; 4],
        ) -> Result<[f32; 4], Self::Error> {
            self.calls.push((site, operand.map(f32::to_bits)));
            if self.fail {
                Err("injected drain estimate failure")
            } else {
                Ok(self.returned)
            }
        }
    }

    struct AssemblyGroundContactTraceBackend {
        returned: [f32; 4],
        calls: Vec<(SimulationAssemblyGroundContactEstimateSite, [u32; 4])>,
    }

    impl AssemblyGroundContactTraceBackend {
        fn ones() -> Self {
            Self {
                returned: [1.0; 4],
                calls: Vec::new(),
            }
        }
    }

    impl SimulationAssemblyGroundContactNumericBackend for AssemblyGroundContactTraceBackend {
        type Error = core::convert::Infallible;

        fn reciprocal_sqrt_estimate(
            &mut self,
            site: SimulationAssemblyGroundContactEstimateSite,
            operand: [f32; 4],
        ) -> Result<[f32; 4], Self::Error> {
            self.calls.push((site, operand.map(f32::to_bits)));
            Ok(self.returned)
        }
    }

    fn zero_contact_body() -> SimulationContactCommonBodyInput {
        SimulationContactCommonBodyInput {
            center_of_mass: [0.0; 3],
            solver_delta: SolverDeltaRowToken(0),
            velocity: [0.0; 3],
            omega: [0.0; 3],
            inertia_full: [0.0; 3],
            inverse_mass: 0.0,
            inertia_split: [0.0; 3],
            state_word: 0,
            force: [0.0; 3],
            kinematic_value: 0.0,
            torque: [0.0; 3],
            cooldown_word: 0,
        }
    }

    fn zero_selected_contact_body() -> SimulationNetworkContactSelectedBodyInput {
        SimulationNetworkContactSelectedBodyInput {
            common: zero_contact_body(),
            right: [0.0; 3],
            up: [0.0; 3],
            at: [0.0; 3],
        }
    }

    fn contact_drain_input() -> SimulationNetworkContactDrainInput {
        SimulationNetworkContactDrainInput {
            static_body: zero_contact_body(),
            selected_body: zero_selected_contact_body(),
            point: [0.0; 4],
            normal: [0.0, 0.0, 1.0, 0.0],
            displacement: 0.0,
            static_body_token: ContactBodyToken(0x1111_1111),
            selected_body_token: ContactBodyToken(0x2222_2222),
            static_friction: 0.6,
            dynamic_friction: 0.4,
            restitution: 0.2,
        }
    }

    fn assembly_ground_contact_input() -> SimulationAssemblyGroundContactInput {
        SimulationAssemblyGroundContactInput {
            body_a: zero_contact_body(),
            body_b: zero_contact_body(),
            position_a: [0.0; 3],
            position_b: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            table_body_a: ContactBodyToken(0x0102_0304),
            table_body_b: ContactBodyToken(0x1112_1314),
            volume_material: SimulationAssemblyGroundMaterialInput {
                static_friction: 0.75,
                dynamic_friction: 0.25,
                restitution: 0.25,
            },
            ground_material: SimulationAssemblyGroundMaterialInput {
                static_friction: 0.5,
                dynamic_friction: 0.5,
                restitution: 0.5,
            },
            volume_surface_id: 0x89ab_cdef,
            triangle_user_tag: 0x1234_5678,
        }
    }

    const A_WORDS: [[u32; 4]; 4] = [
        [0x3e01_2345, 0xbe23_4567, 0x3e89_abcd, 0x3c91_9293],
        [0x3f01_2345, 0x3f12_3456, 0x3f23_4567, 0x3f34_5678],
        [0xbe9a_bcde, 0x3eab_cdef, 0xbf12_3456, 0xbc99_9a9b],
        [0xbf45_6789, 0xbf56_789a, 0xbf67_89ab, 0xbf78_9abc],
    ];

    const B_WORDS: [[u32; 4]; 4] = [
        [0xbe31_2345, 0x3e42_3456, 0xbe53_4567, 0x3ca1_a2a3],
        [0x3d01_2345, 0x3d12_3456, 0x3d23_4567, 0x3d34_5678],
        [0x3eb1_2345, 0xbec2_3456, 0x3ed3_4567, 0xbca9_aaab],
        [0xbd45_6789, 0xbd56_789a, 0xbd67_89ab, 0xbd78_9abc],
    ];

    const EXPECTED_A: [[u32; 4]; 4] = [
        [0xbb89_3d80, 0xbe89_1173, 0x3f06_88a0, 0x3c97_788c],
        [0x3f15_9e12, 0x3f0d_a745, 0x3f25_8bf0, 0x3f34_add9],
        [0xbed9_eb0a, 0x3d0e_15fe, 0xbe2d_cc42, 0xbe07_b262],
        [0xbf2a_1923, 0xbf54_3211, 0xbf6b_2db9, 0xbf61_d967],
    ];

    const EXPECTED_B: [[u32; 4]; 4] = [
        [0xbddc_d959, 0x3e79_a316, 0xbeab_556e, 0x3c9e_afa7],
        [0xbc0a_cc7c, 0x3d36_9cde, 0x3d11_1123, 0x3d31_9b6e],
        [0x3ed1_5bfd, 0xbe43_67c1, 0x3e01_e584, 0x3ceb_49c8],
        [0xbdf4_55e5, 0xbd40_a048, 0xbd8d_4135, 0xbdd7_52b2],
    ];

    fn values<const N: usize>(words: [[u32; 4]; N]) -> [[f32; 4]; N] {
        words.map(|row| row.map(f32::from_bits))
    }

    fn word3(value: [f32; 3], word: u32) -> [u32; 4] {
        [
            value[0].to_bits(),
            value[1].to_bits(),
            value[2].to_bits(),
            word,
        ]
    }

    fn rf_bits(value: [[f32; 4]; 4]) -> [[u32; 4]; 4] {
        value.map(|row| row.map(f32::to_bits))
    }

    fn record_bits(value: &SimulationContactJacobian) -> [[u32; 4]; 16] {
        [
            word3(value.attachment_offset_a, value.solver_delta_a.0),
            word3(value.attachment_offset_b, value.solver_delta_b.0),
            word3(value.contact_transpose_right, value.nb_word),
            word3(value.contact_transpose_up, value.static_friction.to_bits()),
            word3(value.contact_transpose_at, value.dynamic_friction.to_bits()),
            word3(value.position_impulse, value.positional_impulse.to_bits()),
            word3(value.position_limit, value.distance.to_bits()),
            word3(value.contact_right, value.body_a.0),
            word3(value.irac_right, value.inverse_mass_a.to_bits()),
            word3(value.irbc_right, value.inverse_mass_b.to_bits()),
            word3(value.contact_up, value.body_b.0),
            word3(value.irac_up, value.pad_0.to_bits()),
            word3(value.irbc_up, value.pad_1.to_bits()),
            word3(value.contact_at, value.tag_word),
            word3(value.irac_at, value.pad_2.to_bits()),
            word3(value.irbc_at, value.pad_3.to_bits()),
        ]
    }

    fn record_from_words(rows: [[u32; 4]; 16]) -> SimulationContactJacobian {
        let rows = values(rows);
        SimulationContactJacobian {
            attachment_offset_a: rows[0][0..3].try_into().expect("three words"),
            solver_delta_a: SolverDeltaRowToken(rows[0][3].to_bits()),
            attachment_offset_b: rows[1][0..3].try_into().expect("three words"),
            solver_delta_b: SolverDeltaRowToken(rows[1][3].to_bits()),
            contact_transpose_right: rows[2][0..3].try_into().expect("three words"),
            nb_word: rows[2][3].to_bits(),
            contact_transpose_up: rows[3][0..3].try_into().expect("three words"),
            static_friction: rows[3][3],
            contact_transpose_at: rows[4][0..3].try_into().expect("three words"),
            dynamic_friction: rows[4][3],
            position_impulse: rows[5][0..3].try_into().expect("three words"),
            positional_impulse: rows[5][3],
            position_limit: rows[6][0..3].try_into().expect("three words"),
            distance: rows[6][3],
            contact_right: rows[7][0..3].try_into().expect("three words"),
            body_a: ContactBodyToken(rows[7][3].to_bits()),
            irac_right: rows[8][0..3].try_into().expect("three words"),
            inverse_mass_a: rows[8][3],
            irbc_right: rows[9][0..3].try_into().expect("three words"),
            inverse_mass_b: rows[9][3],
            contact_up: rows[10][0..3].try_into().expect("three words"),
            body_b: ContactBodyToken(rows[10][3].to_bits()),
            irac_up: rows[11][0..3].try_into().expect("three words"),
            pad_0: rows[11][3],
            irbc_up: rows[12][0..3].try_into().expect("three words"),
            pad_1: rows[12][3],
            contact_at: rows[13][0..3].try_into().expect("three words"),
            tag_word: rows[13][3].to_bits(),
            irac_at: rows[14][0..3].try_into().expect("three words"),
            pad_2: rows[14][3],
            irbc_at: rows[15][0..3].try_into().expect("three words"),
            pad_3: rows[15][3],
        }
    }

    fn record_fnv64(rows: [[u32; 4]; 16]) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for word in rows.into_iter().flatten() {
            for byte in word.to_be_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        hash
    }

    #[test]
    fn assembly_ground_contact_all_rows_and_four_roles_match_literal_packet() {
        let mut input = assembly_ground_contact_input();
        input.body_a = SimulationContactCommonBodyInput {
            center_of_mass: [1.0, 2.0, 3.0],
            solver_delta: SolverDeltaRowToken(0x2122_2324),
            velocity: [0.0; 3],
            omega: [0.0; 3],
            inertia_full: [4.0, 5.0, 6.0],
            inverse_mass: 7.0,
            inertia_split: [8.0, 9.0, 10.0],
            state_word: 0x4142_4344,
            force: [12.0, 13.0, 14.0],
            kinematic_value: 15.0,
            torque: [16.0, 17.0, 18.0],
            cooldown_word: 0x5152_5354,
        };
        input.body_b = SimulationContactCommonBodyInput {
            center_of_mass: [21.0, 22.0, 23.0],
            solver_delta: SolverDeltaRowToken(0x3132_3334),
            velocity: [0.0; 3],
            omega: [0.0; 3],
            inertia_full: [24.0, 25.0, 26.0],
            inverse_mass: 27.0,
            inertia_split: [28.0, 29.0, 30.0],
            state_word: 0x6162_6364,
            force: [32.0, 33.0, 34.0],
            kinematic_value: 35.0,
            torque: [36.0, 37.0, 38.0],
            cooldown_word: 0x7172_7374,
        };
        input.position_a = input.body_a.center_of_mass;
        input.position_b = input.body_b.center_of_mass;
        let mut backend = AssemblyGroundContactTraceBackend::ones();

        let rows = build_assembly_ground_contact_input(&input, &mut backend)
            .expect("infallible ground estimate")
            .test_words();
        let f = f32::to_bits;
        assert_eq!(
            rows,
            [
                [f(1.0), f(2.0), f(3.0), 0x0102_0304],
                [f(21.0), f(22.0), f(23.0), 0x1112_1314],
                [0, 0, f(1.0), f(0.25)],
                [f(1.0), 0, 0, f(0.75)],
                [0, f(1.0), 0, f(0.5)],
                [0, 0, 0, 0xcdef_1234],
                [f(1.0), f(2.0), f(3.0), 0x2122_2324],
                [f(21.0), f(22.0), f(23.0), 0x3132_3334],
                [f(4.0), f(5.0), f(6.0), f(7.0)],
                [f(24.0), f(25.0), f(26.0), f(27.0)],
                [f(8.0), f(9.0), f(10.0), 0x4142_4344],
                [f(28.0), f(29.0), f(30.0), 0x6162_6364],
                [f(12.0), f(13.0), f(14.0), f(15.0)],
                [f(32.0), f(33.0), f(34.0), f(35.0)],
                [f(16.0), f(17.0), f(18.0), 0x5152_5354],
                [f(36.0), f(37.0), f(38.0), 0x7172_7374],
            ]
        );
        assert_eq!(
            backend.calls,
            vec![(
                SimulationAssemblyGroundContactEstimateSite::TangentReciprocalSqrt,
                [f(1.0); 4],
            )]
        );
    }

    #[test]
    fn assembly_ground_material_selects_ground_on_ties_and_unordered_without_clamps() {
        let material = |static_friction, dynamic_friction, restitution| {
            SimulationAssemblyGroundMaterialInput {
                static_friction,
                dynamic_friction,
                restitution,
            }
        };
        let bits = |value: SimulationAssemblyGroundMaterialInput| {
            [
                value.static_friction.to_bits(),
                value.dynamic_friction.to_bits(),
                value.restitution.to_bits(),
            ]
        };

        assert_eq!(
            bits(select_assembly_ground_material(
                material(0.0, 0.0, 0.0),
                material(-0.0, -0.0, -0.0),
            )),
            [0x8000_0000; 3]
        );
        let volume_nan = f32::from_bits(0x7fc1_2345);
        assert_eq!(
            bits(select_assembly_ground_material(
                material(volume_nan, volume_nan, volume_nan),
                material(1.25, 2.5, 3.5),
            )),
            [1.25_f32.to_bits(), 2.5_f32.to_bits(), 3.5_f32.to_bits()]
        );
        let ground_nan = f32::from_bits(0x7fc5_4321);
        assert_eq!(
            bits(select_assembly_ground_material(
                material(1.25, 2.5, 3.5),
                material(ground_nan, ground_nan, ground_nan),
            )),
            [0x7fc5_4321; 3]
        );
        assert_eq!(
            bits(select_assembly_ground_material(
                material(-2.0, -4.0, 2.5),
                material(-4.0, -2.0, 3.0),
            )),
            [
                (-2.0_f32).to_bits(),
                (-2.0_f32).to_bits(),
                2.5_f32.to_bits()
            ]
        );

        let mut input = assembly_ground_contact_input();
        input.volume_material = material(-2.0, -4.0, 2.5);
        input.ground_material = material(-4.0, -2.0, 3.0);
        let mut backend = AssemblyGroundContactTraceBackend::ones();
        let rows = build_assembly_ground_contact_input(&input, &mut backend)
            .expect("infallible ground estimate")
            .test_words();
        assert_eq!(rows[2][3], 2.5_f32.to_bits());
        assert_eq!(rows[3][3], (-2.0_f32).to_bits());
        assert_eq!(rows[4][3], (-2.0_f32).to_bits());
    }

    #[test]
    fn assembly_ground_x_z_fallback_boundary_uses_exact_fused_cutoff() {
        for (nx_bits, expected_tangent, expected_operand) in [
            (0x3f35_04f3, [0x3f00_0000, 0, 0], 0x3f00_0000),
            (0x3f35_04f4, [0, 0, 0xbf80_0000], 0x3f80_0000),
        ] {
            let mut input = assembly_ground_contact_input();
            input.normal = [f32::from_bits(nx_bits), 0.0, 0.0];
            let mut backend = AssemblyGroundContactTraceBackend::ones();
            let rows = build_assembly_ground_contact_input(&input, &mut backend)
                .expect("infallible ground estimate")
                .test_words();
            assert_eq!(rows[3][..3], expected_tangent);
            assert_eq!(backend.calls[0].1, [expected_operand; 4]);
        }
    }

    #[test]
    fn assembly_ground_point_velocity_uses_numeric_bodies_and_b_minus_a() {
        let mut input = assembly_ground_contact_input();
        input.body_a.center_of_mass = [1.0, 2.0, 3.0];
        input.position_a = [3.0, 2.0, 3.0];
        input.body_a.velocity = [10.0, 20.0, 30.0];
        input.body_a.omega = [0.0, 0.0, 3.0];
        input.body_b.center_of_mass = [-1.0, -2.0, -3.0];
        input.position_b = [-1.0, 1.0, -3.0];
        input.body_b.velocity = [-1.0, -2.0, -3.0];
        input.body_b.omega = [4.0, 0.0, 0.0];
        let mut backend = AssemblyGroundContactTraceBackend::ones();

        let rows = build_assembly_ground_contact_input(&input, &mut backend)
            .expect("infallible ground estimate")
            .test_words();

        assert_eq!(
            rows[5],
            [
                (-11.0_f32).to_bits(),
                (-28.0_f32).to_bits(),
                (-21.0_f32).to_bits(),
                0xcdef_1234,
            ]
        );
    }

    #[test]
    fn assembly_ground_one_estimate_drives_both_axes_with_exact_operand_lanes() {
        let mut input = assembly_ground_contact_input();
        input.body_b.velocity = [1.0, 2.0, 3.0];
        input.normal = [0.3, 0.5, 0.7];
        let mut backend = AssemblyGroundContactTraceBackend {
            returned: [2.0, 3.0, 5.0, 7.0],
            calls: Vec::new(),
        };

        let rows = build_assembly_ground_contact_input(&input, &mut backend)
            .expect("infallible ground estimate")
            .test_words();

        assert_eq!(
            backend.calls,
            vec![(
                SimulationAssemblyGroundContactEstimateSite::TangentReciprocalSqrt,
                [0x3d75_c297; 4],
            )]
        );
        assert_eq!(
            rows[3],
            [0xbe4c_ccd0, 0x3f19_999c, 0xbf00_0002, 0x3f40_0000]
        );
        assert_eq!(
            rows[4],
            [0xbec2_8f5f, 0xbdf5_c292, 0x3f0c_cccf, 0x3f00_0000]
        );
    }

    #[test]
    fn network_drain_transform_and_point_velocity_fmas_match_oracle() {
        let mut transform = contact_drain_input();
        transform.selected_body.common.center_of_mass[0] = -1.0;
        transform.selected_body.right[0] = f32::from_bits(0x3f80_0001);
        transform.point[0] = f32::from_bits(0x3f7f_fffe);
        let mut backend = ContactDrainTraceBackend::ones();
        let rows = build_network_contact_input(&transform, &mut backend)
            .expect("transform fixture")
            .test_words();
        assert_eq!(rows[0], [0xa880_0000, 0, 0, 0x1111_1111]);
        assert_eq!(rows[1], [0xa880_0000, 0, 0, 0x2222_2222]);
        assert_eq!(backend.calls.len(), 1);

        let mut point_velocity = contact_drain_input();
        point_velocity.selected_body.right = [1.0, 0.0, 0.0];
        point_velocity.selected_body.up = [0.0, 1.0, 0.0];
        point_velocity.selected_body.at = [0.0, 0.0, 1.0];
        point_velocity.point[1] = f32::from_bits(0x3f80_0001);
        point_velocity.static_body.velocity = [1.0, 0.0, 0.0];
        point_velocity.static_body.omega[2] = f32::from_bits(0x3f7f_fffe);
        let mut backend = ContactDrainTraceBackend::ones();
        let rows = build_network_contact_input(&point_velocity, &mut backend)
            .expect("point-velocity fixture")
            .test_words();
        assert_eq!(rows[5], [0xa880_0000, 0, 0, 0]);
        assert_eq!(
            backend.calls,
            vec![(
                SimulationNetworkContactDrainEstimateSite::TangentReciprocalSqrt,
                [0x1180_0000; 4],
            )]
        );
    }

    #[test]
    fn network_drain_tangent_routes_and_threshold_match_oracle() {
        let mut dynamic = contact_drain_input();
        dynamic.selected_body.common.velocity = [1.0, 0.0, 0.0];
        let mut backend = ContactDrainTraceBackend::ones();
        let rows = build_network_contact_input(&dynamic, &mut backend)
            .expect("dynamic tangent")
            .test_words();
        assert_eq!(rows[3][0..3], [0, 0xbf80_0000, 0]);
        assert_eq!(rows[4][0..3], [0x3f80_0000, 0, 0]);
        assert_eq!(rows[5], [0x3f80_0000, 0, 0, 0]);
        assert_eq!(backend.calls[0].1, [0x3f80_0000; 4]);

        for (normal, expected_tangent, expected_binormal) in [
            (
                [0.0, 1.0, 0.0, 0.0],
                [0x3f80_0000, 0, 0],
                [0, 0, 0xbf80_0000],
            ),
            (
                [1.0, 0.0, 0.0, 0.0],
                [0, 0, 0xbf80_0000],
                [0, 0x3f80_0000, 0],
            ),
        ] {
            let mut input = contact_drain_input();
            input.normal = normal;
            let mut backend = ContactDrainTraceBackend::ones();
            let rows = build_network_contact_input(&input, &mut backend)
                .expect("fallback")
                .test_words();
            assert_eq!(rows[3][0..3], expected_tangent);
            assert_eq!(rows[4][0..3], expected_binormal);
            assert_eq!(backend.calls[0].1, [0x3f80_0000; 4]);
        }

        for (normal, expected_tangent) in [
            ([0.25, 2.0, 0.0, 0.0], [0x3f70_0000, 0xbf00_0000, 0]),
            (
                [1.0, 2.0, 0.25, 0.0],
                [0x3e80_0000, 0x3f00_0000, 0xbf70_0000],
            ),
        ] {
            let mut input = contact_drain_input();
            input.normal = normal;
            let mut backend = ContactDrainTraceBackend::ones();
            let rows = build_network_contact_input(&input, &mut backend)
                .expect("shortcut magnitude")
                .test_words();
            assert_eq!(rows[3][0..3], expected_tangent);
            assert_eq!(backend.calls[0].1, [0x3f70_0000; 4]);
        }

        let mut threshold = contact_drain_input();
        threshold.selected_body.common.velocity[0] = 2.0_f32.powi(-63);
        let mut backend = ContactDrainTraceBackend::ones();
        let rows = build_network_contact_input(&threshold, &mut backend)
            .expect("minimum-normal equality")
            .test_words();
        assert_eq!(backend.calls[0].1, [0x0080_0000; 4]);
        assert_eq!(rows[3][1], (-2.0_f32.powi(-63)).to_bits());
    }

    #[test]
    fn network_drain_one_estimate_drives_both_axes_and_failure_returns_no_value() {
        let mut input = contact_drain_input();
        input.selected_body.common.velocity = [1.0, 2.0, 3.0];
        input.normal = [0.3, 0.5, 0.7, f32::from_bits(0x7fc0_1234)];
        let mut backend = ContactDrainTraceBackend {
            returned: [2.0, 3.0, 5.0, 7.0],
            calls: Vec::new(),
            fail: false,
        };
        let rows = build_network_contact_input(&input, &mut backend)
            .expect("lane-distinct estimate")
            .test_words();
        assert_eq!(
            backend.calls,
            vec![(
                SimulationNetworkContactDrainEstimateSite::TangentReciprocalSqrt,
                [0x3d75_c297; 4],
            )]
        );
        assert_eq!(
            rows[3],
            [0xbe4c_ccd0, 0x3f19_999c, 0xbf00_0002, 0x3f19_999a]
        );
        assert_eq!(
            rows[4],
            [0xbec2_8f5f, 0xbdf5_c292, 0x3f0c_cccf, 0x3ecc_cccd]
        );

        let mut backend = ContactDrainTraceBackend {
            returned: [1.0; 4],
            calls: Vec::new(),
            fail: true,
        };
        assert!(matches!(
            build_network_contact_input(&input, &mut backend),
            Err("injected drain estimate failure")
        ));
        assert_eq!(backend.calls.len(), 1);

        let mut binormal_association = contact_drain_input();
        binormal_association.normal = [
            f32::from_bits(0x3fe9_7d53),
            f32::from_bits(0x3fe5_49a5),
            f32::from_bits(0xbfe3_0bb3),
            0.0,
        ];
        binormal_association.selected_body.common.velocity = [
            f32::from_bits(0xc084_d749),
            f32::from_bits(0x4056_b823),
            f32::from_bits(0x4017_0553),
        ];
        let mut backend = ContactDrainTraceBackend::ones();
        let rows = build_network_contact_input(&binormal_association, &mut backend)
            .expect("binormal fused association")
            .test_words();
        assert_eq!(rows[3][0..3], [0xc122_d926, 0xc043_c852, 0xc158_e626]);
        assert_eq!(rows[4][0..3], [0xc1ed_ad2a, 0x422b_20d3, 0x414a_6d9a]);
    }

    #[test]
    fn network_drain_w_sources_scratch_order_and_material_routes_match_oracle() {
        let mut input = contact_drain_input();
        input.static_body.center_of_mass = [1.0, 2.0, 3.0];
        input.static_body.solver_delta = SolverDeltaRowToken(0x0102_0304);
        input.selected_body.common.center_of_mass = [4.0, 5.0, 6.0];
        input.selected_body.common.solver_delta = SolverDeltaRowToken(0x1112_1314);
        input.static_body.inertia_full = [7.0, 8.0, 9.0];
        input.static_body.inverse_mass = f32::from_bits(0x2122_2324);
        input.selected_body.common.inertia_full = [10.0, 11.0, 12.0];
        input.selected_body.common.inverse_mass = f32::from_bits(0x3132_3334);
        input.static_body.inertia_split = [13.0, 14.0, 15.0];
        input.static_body.state_word = 0x4142_4344;
        input.selected_body.common.inertia_split = [16.0, 17.0, 18.0];
        input.selected_body.common.state_word = 0x5152_5354;
        input.static_body.force = [19.0, 20.0, 21.0];
        input.static_body.kinematic_value = f32::from_bits(0x6162_6364);
        input.selected_body.common.force = [22.0, 23.0, 24.0];
        input.selected_body.common.kinematic_value = f32::from_bits(0x7172_7374);
        input.static_body.torque = [25.0, 26.0, 27.0];
        input.static_body.cooldown_word = 0x0123_4567;
        input.selected_body.common.torque = [28.0, 29.0, 30.0];
        input.selected_body.common.cooldown_word = 0x89ab_cdef;
        input.static_friction = -3.0;
        input.dynamic_friction = 2.5;
        input.restitution = -4.0;
        let mut backend = ContactDrainTraceBackend::ones();
        let rows = build_network_contact_input(&input, &mut backend)
            .expect("W sources")
            .test_words();
        let expected = [
            [0x4080_0000, 0x40a0_0000, 0x40c0_0000, 0x1111_1111],
            [0x4080_0000, 0x40a0_0000, 0x40c0_0000, 0x2222_2222],
            [0, 0, 0x3f80_0000, 0xc080_0000],
            [0x3f80_0000, 0, 0, 0],
            [0, 0x3f80_0000, 0, 0x4020_0000],
            [0, 0, 0, 0],
            [0x3f80_0000, 0x4000_0000, 0x4040_0000, 0x0102_0304],
            [0x4080_0000, 0x40a0_0000, 0x40c0_0000, 0x1112_1314],
            [0x40e0_0000, 0x4100_0000, 0x4110_0000, 0x2122_2324],
            [0x4120_0000, 0x4130_0000, 0x4140_0000, 0x3132_3334],
            [0x4150_0000, 0x4160_0000, 0x4170_0000, 0x4142_4344],
            [0x4180_0000, 0x4188_0000, 0x4190_0000, 0x5152_5354],
            [0x4198_0000, 0x41a0_0000, 0x41a8_0000, 0x6162_6364],
            [0x41b0_0000, 0x41b8_0000, 0x41c0_0000, 0x7172_7374],
            [0x41c8_0000, 0x41d0_0000, 0x41d8_0000, 0x0123_4567],
            [0x41e0_0000, 0x41e8_0000, 0x41f0_0000, 0x89ab_cdef],
        ];
        assert_eq!(rows, expected);

        input.static_friction = 2.0;
        input.dynamic_friction = -3.0;
        input.restitution = 1.0;
        let mut backend = ContactDrainTraceBackend::ones();
        let rows = build_network_contact_input(&input, &mut backend)
            .expect("material boundaries")
            .test_words();
        assert_eq!(rows[2][3], 0x3f80_0000);
        assert_eq!(rows[3][3], 0x4000_0000);
        assert_eq!(rows[4][3], 0);
    }

    #[test]
    fn contact_layout_is_exactly_sixteen_packed_vectors() {
        assert_eq!(size_of::<SimulationContactJacobian>(), 0x100);
        assert_eq!(align_of::<SimulationContactJacobian>(), 0x10);
        assert_eq!(
            offset_of!(SimulationContactJacobian, attachment_offset_a),
            0x00
        );
        assert_eq!(
            offset_of!(SimulationContactJacobian, attachment_offset_b),
            0x10
        );
        assert_eq!(
            offset_of!(SimulationContactJacobian, contact_transpose_right),
            0x20
        );
        assert_eq!(
            offset_of!(SimulationContactJacobian, contact_transpose_up),
            0x30
        );
        assert_eq!(
            offset_of!(SimulationContactJacobian, contact_transpose_at),
            0x40
        );
        assert_eq!(
            offset_of!(SimulationContactJacobian, position_impulse),
            0x50
        );
        assert_eq!(offset_of!(SimulationContactJacobian, position_limit), 0x60);
        assert_eq!(offset_of!(SimulationContactJacobian, contact_right), 0x70);
        assert_eq!(offset_of!(SimulationContactJacobian, irac_right), 0x80);
        assert_eq!(offset_of!(SimulationContactJacobian, irbc_right), 0x90);
        assert_eq!(offset_of!(SimulationContactJacobian, contact_up), 0xa0);
        assert_eq!(offset_of!(SimulationContactJacobian, irac_up), 0xb0);
        assert_eq!(offset_of!(SimulationContactJacobian, irbc_up), 0xc0);
        assert_eq!(offset_of!(SimulationContactJacobian, contact_at), 0xd0);
        assert_eq!(offset_of!(SimulationContactJacobian, irac_at), 0xe0);
        assert_eq!(offset_of!(SimulationContactJacobian, irbc_at), 0xf0);
    }

    #[test]
    fn zeroed_value_preserves_every_word_and_has_no_row_authority() {
        let value = SimulationContactJacobian::zeroed();
        assert_eq!(record_bits(&value), [[0; 4]; 16]);
        assert!(!value.has_solver_delta_bindings());
    }

    #[test]
    fn build_literal_record_and_single_backend_operand_match_frozen_oracle() {
        let mut backend = ContactBuildTraceBackend::default();
        let record = build_contact_jacobian_host(
            SimulationContactBuildInput::from_words(BUILD_INPUT_WORDS),
            1.0 / 60.0,
            &mut backend,
        )
        .expect("fixture backend succeeds");

        assert_eq!(record_bits(&record), EXPECTED_BUILD_RECORD);
        assert_eq!(
            backend.calls,
            vec![(
                SimulationContactReciprocalSite::EffectiveMassFourLane,
                BUILD_DENOMINATOR_WORDS,
            )]
        );
    }

    #[test]
    fn build_state_gates_are_independent_and_nb_uses_raw_bit_eight() {
        let cases = [
            (0x0000_0000, 0x0000_0004, 0x56dd_3c45_b4ec_f316_u64),
            (0x0000_0004, 0x0000_0000, 0xd727_3de0_229a_1958),
            (0x0000_0000, 0x0000_0000, 0xc03e_5080_1389_311f),
            (0x0000_0008, 0x0000_0008, 0x38f3_660c_040f_c1d7),
            (0x0000_0005, 0x8000_0004, 0x7e9f_1c50_02e7_2803),
        ];

        for (state_0, state_1, expected_hash) in cases {
            let mut words = BUILD_INPUT_WORDS;
            words[10][3] = state_0;
            words[11][3] = state_1;
            let mut backend = ContactBuildTraceBackend::default();
            let record = build_contact_jacobian_host(
                SimulationContactBuildInput::from_words(words),
                1.0 / 60.0,
                &mut backend,
            )
            .expect("fixture backend succeeds");
            let actual = record_bits(&record);
            assert_eq!(
                record_fnv64(actual),
                expected_hash,
                "state pair {state_0:#010x}/{state_1:#010x}"
            );
            assert_eq!(actual[2][3], (state_0 | state_1) & 8);
        }
    }

    #[test]
    fn build_numeric_failure_returns_no_record_after_exactly_one_observation() {
        let mut backend = ContactBuildTraceBackend {
            fail: true,
            ..ContactBuildTraceBackend::default()
        };
        let result = build_contact_jacobian_host(
            SimulationContactBuildInput::from_words(BUILD_INPUT_WORDS),
            1.0 / 60.0,
            &mut backend,
        );

        assert!(matches!(result, Err("injected reciprocal failure")));
        assert_eq!(
            backend.calls,
            vec![(
                SimulationContactReciprocalSite::EffectiveMassFourLane,
                BUILD_DENOMINATOR_WORDS,
            )]
        );
    }

    #[test]
    fn documented_native_build_store_schedule_replays_all_sixteen_rows() {
        let expected_schedule = [
            (0x82ae_1ae0, 7),
            (0x82ae_1ae8, 0),
            (0x82ae_1aec, 10),
            (0x82ae_1af4, 13),
            (0x82ae_1c30, 1),
            (0x82ae_1d4c, 11),
            (0x82ae_1d64, 8),
            (0x82ae_1d8c, 9),
            (0x82ae_1d90, 14),
            (0x82ae_1d94, 12),
            (0x82ae_1dbc, 15),
            (0x82ae_1df0, 5),
            (0x82ae_1e74, 4),
            (0x82ae_1e78, 2),
            (0x82ae_1e7c, 3),
            (0x82ae_1e98, 6),
        ];
        let schedule = contact_build_native_store_schedule();
        assert_eq!(schedule, expected_schedule);
        let mut replayed = [[0_u32; 4]; 16];
        let mut seen = [false; 16];
        for &(pc, row) in &schedule {
            assert_eq!(pc & 3, 0, "native vector-store PC remains aligned");
            assert!(!seen[row], "row {row} stored more than once");
            seen[row] = true;
            replayed[row] = EXPECTED_BUILD_RECORD[row];
        }

        assert!(seen.into_iter().all(|value| value));
        assert_eq!(replayed, EXPECTED_BUILD_RECORD);
    }

    #[test]
    fn baseline_matches_the_standalone_full_record_and_rf_oracle() {
        let mut record = record_from_words(RECORD_WORDS);
        let result = solve_contact_horus_record_host(&mut record, values(A_WORDS), values(B_WORDS));
        let mut expected_record = RECORD_WORDS;
        expected_record[5] = [0x3f08_5cf0, 0xbe99_999a, 0x3e99_999a, 0x3e69_3ddb];

        assert_eq!(record_bits(&record), expected_record);
        assert_eq!(rf_bits(result.solver_delta_a), EXPECTED_A);
        assert_eq!(rf_bits(result.solver_delta_b), EXPECTED_B);
    }

    #[test]
    fn observer_freezes_accumulator_then_a_b_row_store_order() {
        let original_a = values(A_WORDS);
        let original_b = values(B_WORDS);
        let mut record = record_from_words(RECORD_WORDS);
        let mut observations = Vec::new();
        let result = solve_contact_horus_record_with_observer(
            &mut record,
            original_a,
            original_b,
            |store, record, a, b| {
                observations.push((store, record_bits(record), rf_bits(*a), rf_bits(*b)));
            },
        );

        let stores = observations
            .iter()
            .map(|observation| observation.0)
            .collect::<Vec<_>>();
        assert_eq!(
            stores,
            [
                SimulationContactHorusStore::Accumulator,
                SimulationContactHorusStore::A3,
                SimulationContactHorusStore::B3,
                SimulationContactHorusStore::A1,
                SimulationContactHorusStore::B1,
                SimulationContactHorusStore::A2,
                SimulationContactHorusStore::B2,
                SimulationContactHorusStore::A0,
                SimulationContactHorusStore::B0,
            ]
        );

        let mut expected_record = RECORD_WORDS;
        expected_record[5] = [0x3f08_5cf0, 0xbe99_999a, 0x3e99_999a, 0x3e69_3ddb];
        let row_stores = [
            (3, true),
            (3, false),
            (1, true),
            (1, false),
            (2, true),
            (2, false),
            (0, true),
            (0, false),
        ];
        let mut expected_a = A_WORDS;
        let mut expected_b = B_WORDS;
        for (index, observation) in observations.iter().enumerate() {
            assert_eq!(observation.1, expected_record);
            if let Some(&(row, side_a)) = index.checked_sub(1).and_then(|slot| row_stores.get(slot))
            {
                if side_a {
                    expected_a[row] = EXPECTED_A[row];
                } else {
                    expected_b[row] = EXPECTED_B[row];
                }
            }
            assert_eq!(observation.2, expected_a);
            assert_eq!(observation.3, expected_b);
        }
        assert_eq!(rf_bits(result.solver_delta_a), EXPECTED_A);
        assert_eq!(rf_bits(result.solver_delta_b), EXPECTED_B);
    }

    #[test]
    fn self_pair_stages_one_snapshot_and_native_a_then_b_commit_makes_b_win() {
        let original = values(A_WORDS);
        let mut record = record_from_words(RECORD_WORDS);
        let staged = solve_contact_horus_record_host(&mut record, original, original);
        let expected_staged_a = [
            [0x3f18_0b37, 0xbecd_1ea6, 0x3f17_8bee, 0x3ce9_21fc],
            [0x3f6c_4322, 0x3ef4_cc49, 0x3f2f_2c80, 0x3f36_1f88],
            [0x3ebe_6428, 0x3dcf_2563, 0xbe8d_5717, 0x3f08_3467],
            [0xbe5a_4983, 0xbf4a_9181, 0xbf7a_9506, 0xbf01_93c6],
        ];
        let expected_staged_b = [
            [0xbddc_c30c, 0xbd1f_25d4, 0x3ddb_d718, 0x3c4b_95bd],
            [0x3e97_26ac, 0x3f1e_1b6f, 0x3f1d_51db, 0x3f33_71f0],
            [0xbf45_5c78, 0x3f10_5c60, 0xbf73_2f14, 0xbe9a_6b62],
            [0xbf92_5027, 0xbf4f_5458, 0xbf78_339a, 0xbf9a_0f1c],
        ];
        assert_eq!(rf_bits(staged.solver_delta_a), expected_staged_a);
        assert_eq!(rf_bits(staged.solver_delta_b), expected_staged_b);

        let mut committed = original;
        for row in [3, 1, 2, 0] {
            committed[row] = staged.solver_delta_a[row];
            committed[row] = staged.solver_delta_b[row];
        }
        assert_eq!(rf_bits(committed), expected_staged_b);
    }
}
