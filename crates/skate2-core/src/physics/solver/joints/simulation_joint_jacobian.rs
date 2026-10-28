//! Evidence-bounded value layout for `rw::physics::JointJacobian`.
//!
//! `JointJacobian::Build` (`0x82ae5918..0x82ae67a8`) calculates temporary
//! quaternion, limit, and basis values before clearing the three output cache
//! lines at `0x82ae6308..0x82ae631c`. This module freezes the exact
//! `0x180`-byte PDB layout, that all-zero construction boundary, and the pure
//! ordinary-finite per-record island Horus consumer. Private sibling stages now
//! compose the complete Build graph into an inert 96-word image. A private
//! sibling now installs checked tokens and stable island subsequences into one
//! unpublished owner. A private sibling now executes one checked selected
//! table through RF; the complete outer pipeline remains open.
//!
//! Native `m_ia`, `m_ib`, and `m_joint` are pointer-bearing words. Their host
//! counterparts are private tokens rather than native pointers, truncated host
//! pointers, stable public IDs, or mutation capabilities.

use crate::simulation_constraint_math::{
    horus_columns_from, horus_cross_add, horus_madd, horus_negative_columns_from, horus_nmsub,
    pack_xyz_word,
};
use std::num::NonZeroU32;

#[repr(transparent)]
#[derive(Clone, Copy, Default, Eq, PartialEq)]
struct SolverDeltaRowToken(u32);

#[repr(transparent)]
#[derive(Clone, Copy, Default, Eq, PartialEq)]
struct JointSlotToken(u32);

/// Exact host value layout corresponding to native `rw::physics::JointJacobian`.
///
/// Native reference: `JointJacobian::Build` at `0x82ae5918`; PDB size `0x180`.
/// The three private token words intentionally retain the native word widths
/// while enforcing safe-host ownership semantics.
#[repr(C, align(16))]
#[derive(Clone, Copy, PartialEq)]
pub struct SimulationJointJacobian {
    pub attachment_offset_a: [f32; 3],
    solver_delta_a: SolverDeltaRowToken,
    pub attachment_offset_b: [f32; 3],
    solver_delta_b: SolverDeltaRowToken,
    pub position_impulse: [f32; 3],
    pad_0: f32,
    pub angular_impulse: [f32; 3],
    pad_1: f32,
    pub linear_transpose_right: [f32; 3],
    /// Native PDB `m_nb` word at `+0x4c`.
    ///
    /// Build briefly stores `Joint::m_spy` here, but its later full-vector
    /// `m_LT_ri` store overwrites that scalar. The final word therefore carries
    /// the transpose vector's numeric W bits, not the Joint spy value.
    nb_word: u32,
    pub angular_transpose_right: [f32; 3],
    pad_2: f32,
    pub linear_transpose_up: [f32; 3],
    pub angular_max_x: f32,
    pub angular_transpose_up: [f32; 3],
    pub angular_max_y: f32,
    pub linear_transpose_at: [f32; 3],
    pub angular_min_x: f32,
    pub angular_transpose_at: [f32; 3],
    pub angular_min_y: f32,
    pub position_max: [f32; 3],
    pub angular_max_z: f32,
    pub position_min: [f32; 3],
    pub angular_min_z: f32,
    pub linear_right: [f32; 3],
    joint_slot: JointSlotToken,
    pub linear_up: [f32; 3],
    pad_3: f32,
    pub linear_at: [f32; 3],
    pad_4: f32,
    pub angular_right: [f32; 3],
    pad_5: f32,
    pub angular_up: [f32; 3],
    pad_6: f32,
    pub angular_at: [f32; 3],
    pad_7: f32,
    pub inverse_inertia_a_right: [f32; 3],
    pub inverse_mass_a: f32,
    pub inverse_inertia_a_up: [f32; 3],
    pad_8: f32,
    pub inverse_inertia_a_at: [f32; 3],
    pad_9: f32,
    pub inverse_inertia_b_right: [f32; 3],
    pub inverse_mass_b: f32,
    pub inverse_inertia_b_up: [f32; 3],
    pad_10: f32,
    pub inverse_inertia_b_at: [f32; 3],
    pad_11: f32,
}

impl SimulationJointJacobian {
    /// Reproduces the all-zero state after Build clears its output record.
    ///
    /// Native anchors: `0x82ae6308`, `0x82ae6314`, and `0x82ae631c`.
    /// Unlike Drive Build, Joint Build reaches this boundary after calculating
    /// most temporary values and immediately before its final stores.
    #[must_use]
    pub const fn zeroed() -> Self {
        Self {
            attachment_offset_a: [0.0; 3],
            solver_delta_a: SolverDeltaRowToken(0),
            attachment_offset_b: [0.0; 3],
            solver_delta_b: SolverDeltaRowToken(0),
            position_impulse: [0.0; 3],
            pad_0: 0.0,
            angular_impulse: [0.0; 3],
            pad_1: 0.0,
            linear_transpose_right: [0.0; 3],
            nb_word: 0,
            angular_transpose_right: [0.0; 3],
            pad_2: 0.0,
            linear_transpose_up: [0.0; 3],
            angular_max_x: 0.0,
            angular_transpose_up: [0.0; 3],
            angular_max_y: 0.0,
            linear_transpose_at: [0.0; 3],
            angular_min_x: 0.0,
            angular_transpose_at: [0.0; 3],
            angular_min_y: 0.0,
            position_max: [0.0; 3],
            angular_max_z: 0.0,
            position_min: [0.0; 3],
            angular_min_z: 0.0,
            linear_right: [0.0; 3],
            joint_slot: JointSlotToken(0),
            linear_up: [0.0; 3],
            pad_3: 0.0,
            linear_at: [0.0; 3],
            pad_4: 0.0,
            angular_right: [0.0; 3],
            pad_5: 0.0,
            angular_up: [0.0; 3],
            pad_6: 0.0,
            angular_at: [0.0; 3],
            pad_7: 0.0,
            inverse_inertia_a_right: [0.0; 3],
            inverse_mass_a: 0.0,
            inverse_inertia_a_up: [0.0; 3],
            pad_8: 0.0,
            inverse_inertia_a_at: [0.0; 3],
            pad_9: 0.0,
            inverse_inertia_b_right: [0.0; 3],
            inverse_mass_b: 0.0,
            inverse_inertia_b_up: [0.0; 3],
            pad_10: 0.0,
            inverse_inertia_b_at: [0.0; 3],
            pad_11: 0.0,
        }
    }

    /// Materializes one fully composed inert image with checked host bindings.
    ///
    /// Every non-binding word is copied bit-for-bit. The caller supplies three
    /// nonzero binding words. The private preparation sibling derives them by
    /// checked `slot + 1` preflight while retaining occurrence sidecars; this
    /// installer itself grants and validates no pool or RF authority.
    #[allow(
        clippy::too_many_lines,
        reason = "explicit 24-row mapping keeps private token ownership auditable"
    )]
    pub(crate) fn from_inert_rows_with_bindings(
        rows: &[[u32; 4]; 24],
        solver_delta_a: NonZeroU32,
        solver_delta_b: NonZeroU32,
        joint_slot: NonZeroU32,
    ) -> Self {
        let xyz = |row: [u32; 4]| {
            [
                f32::from_bits(row[0]),
                f32::from_bits(row[1]),
                f32::from_bits(row[2]),
            ]
        };
        Self {
            attachment_offset_a: xyz(rows[0]),
            solver_delta_a: SolverDeltaRowToken(solver_delta_a.get()),
            attachment_offset_b: xyz(rows[1]),
            solver_delta_b: SolverDeltaRowToken(solver_delta_b.get()),
            position_impulse: xyz(rows[2]),
            pad_0: f32::from_bits(rows[2][3]),
            angular_impulse: xyz(rows[3]),
            pad_1: f32::from_bits(rows[3][3]),
            linear_transpose_right: xyz(rows[4]),
            nb_word: rows[4][3],
            angular_transpose_right: xyz(rows[5]),
            pad_2: f32::from_bits(rows[5][3]),
            linear_transpose_up: xyz(rows[6]),
            angular_max_x: f32::from_bits(rows[6][3]),
            angular_transpose_up: xyz(rows[7]),
            angular_max_y: f32::from_bits(rows[7][3]),
            linear_transpose_at: xyz(rows[8]),
            angular_min_x: f32::from_bits(rows[8][3]),
            angular_transpose_at: xyz(rows[9]),
            angular_min_y: f32::from_bits(rows[9][3]),
            position_max: xyz(rows[10]),
            angular_max_z: f32::from_bits(rows[10][3]),
            position_min: xyz(rows[11]),
            angular_min_z: f32::from_bits(rows[11][3]),
            linear_right: xyz(rows[12]),
            joint_slot: JointSlotToken(joint_slot.get()),
            linear_up: xyz(rows[13]),
            pad_3: f32::from_bits(rows[13][3]),
            linear_at: xyz(rows[14]),
            pad_4: f32::from_bits(rows[14][3]),
            angular_right: xyz(rows[15]),
            pad_5: f32::from_bits(rows[15][3]),
            angular_up: xyz(rows[16]),
            pad_6: f32::from_bits(rows[16][3]),
            angular_at: xyz(rows[17]),
            pad_7: f32::from_bits(rows[17][3]),
            inverse_inertia_a_right: xyz(rows[18]),
            inverse_mass_a: f32::from_bits(rows[18][3]),
            inverse_inertia_a_up: xyz(rows[19]),
            pad_8: f32::from_bits(rows[19][3]),
            inverse_inertia_a_at: xyz(rows[20]),
            pad_9: f32::from_bits(rows[20][3]),
            inverse_inertia_b_right: xyz(rows[21]),
            inverse_mass_b: f32::from_bits(rows[21][3]),
            inverse_inertia_b_up: xyz(rows[22]),
            pad_10: f32::from_bits(rows[22][3]),
            inverse_inertia_b_at: xyz(rows[23]),
            pad_11: f32::from_bits(rows[23][3]),
        }
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "reserved for checked Joint RF revalidation")
    )]
    pub(crate) const fn binding_words(&self) -> [u32; 3] {
        [
            self.solver_delta_a.0,
            self.solver_delta_b.0,
            self.joint_slot.0,
        ]
    }

    #[cfg(test)]
    pub(crate) fn test_words(&self) -> [[u32; 4]; 24] {
        let row =
            |xyz: [f32; 3], word: u32| [xyz[0].to_bits(), xyz[1].to_bits(), xyz[2].to_bits(), word];
        [
            row(self.attachment_offset_a, self.solver_delta_a.0),
            row(self.attachment_offset_b, self.solver_delta_b.0),
            row(self.position_impulse, self.pad_0.to_bits()),
            row(self.angular_impulse, self.pad_1.to_bits()),
            row(self.linear_transpose_right, self.nb_word),
            row(self.angular_transpose_right, self.pad_2.to_bits()),
            row(self.linear_transpose_up, self.angular_max_x.to_bits()),
            row(self.angular_transpose_up, self.angular_max_y.to_bits()),
            row(self.linear_transpose_at, self.angular_min_x.to_bits()),
            row(self.angular_transpose_at, self.angular_min_y.to_bits()),
            row(self.position_max, self.angular_max_z.to_bits()),
            row(self.position_min, self.angular_min_z.to_bits()),
            row(self.linear_right, self.joint_slot.0),
            row(self.linear_up, self.pad_3.to_bits()),
            row(self.linear_at, self.pad_4.to_bits()),
            row(self.angular_right, self.pad_5.to_bits()),
            row(self.angular_up, self.pad_6.to_bits()),
            row(self.angular_at, self.pad_7.to_bits()),
            row(self.inverse_inertia_a_right, self.inverse_mass_a.to_bits()),
            row(self.inverse_inertia_a_up, self.pad_8.to_bits()),
            row(self.inverse_inertia_a_at, self.pad_9.to_bits()),
            row(self.inverse_inertia_b_right, self.inverse_mass_b.to_bits()),
            row(self.inverse_inertia_b_up, self.pad_10.to_bits()),
            row(self.inverse_inertia_b_at, self.pad_11.to_bits()),
        ]
    }

    /// Reports whether both private RF-row destinations have been installed.
    #[must_use]
    pub const fn has_solver_delta_bindings(&self) -> bool {
        self.solver_delta_a.0 != 0 && self.solver_delta_b.0 != 0
    }

    /// Reports whether the private source-Joint occurrence has been installed.
    #[must_use]
    pub const fn has_joint_binding(&self) -> bool {
        self.joint_slot.0 != 0
    }
}

impl Default for SimulationJointJacobian {
    fn default() -> Self {
        Self::zeroed()
    }
}

/// Staged value result of one island Horus Joint-record solve.
///
/// Only rows zero and two are changed. Keeping complete copied blocks lets a
/// later pool transaction preserve self-pair B-wins behavior and then-live
/// shared-body history without forming aliased mutable references.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private checked Joint island sweep")
)]
pub(crate) struct SimulationJointHorusRows {
    pub solver_delta_a: [[f32; 4]; 4],
    pub solver_delta_b: [[f32; 4]; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for Joint accumulator observation")
)]
enum SimulationJointHorusAccumulator {
    Angular,
    Position,
}

/// Applies one island-indirect Horus Joint consumer to copied RF inputs.
///
/// Native per-record anchors are `0x82ae2c68..0x82ae2ee4`. Angular projection
/// uses `B2-A2`, while position projection uses the separately calculated
/// `pointB-pointA`; sharing those inputs is not source-equivalent. The helper
/// preserves full four-lane arithmetic, Right/Up/At FMA order, Joint's
/// two-input dead-zone graph, Angular-then-Position record stores, and staged
/// A0/B0/A2/B2 results.
///
/// This is an ordinary-finite host model. Xenon min/max NaN and signed-zero
/// selection, denormal/flush behavior, overflow, exceptional fused behavior,
/// and native pointer-derived W parity remain outside the claim. Private host
/// token words still participate in the packed W computation.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private checked Joint island sweep")
)]
pub(crate) fn solve_joint_horus_record_host(
    jacobian: &mut SimulationJointJacobian,
    solver_delta_a: [[f32; 4]; 4],
    solver_delta_b: [[f32; 4]; 4],
) -> SimulationJointHorusRows {
    solve_joint_horus_record_with_observer(jacobian, solver_delta_a, solver_delta_b, |_, _| {})
}

#[allow(
    clippy::similar_names,
    clippy::too_many_lines,
    reason = "source-shaped A/B names and uninterrupted native operation order are intentional"
)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private checked Joint island sweep")
)]
fn solve_joint_horus_record_with_observer(
    jacobian: &mut SimulationJointJacobian,
    mut solver_delta_a: [[f32; 4]; 4],
    mut solver_delta_b: [[f32; 4]; 4],
    mut observe_accumulator_store: impl FnMut(SimulationJointHorusAccumulator, &SimulationJointJacobian),
) -> SimulationJointHorusRows {
    let zero = [0.0; 4];
    let attachment_a = pack_xyz_word(
        jacobian.attachment_offset_a,
        f32::from_bits(jacobian.solver_delta_a.0),
    );
    let attachment_b = pack_xyz_word(
        jacobian.attachment_offset_b,
        f32::from_bits(jacobian.solver_delta_b.0),
    );
    let old_position_impulse = pack_xyz_word(jacobian.position_impulse, jacobian.pad_0);
    let old_angular_impulse = pack_xyz_word(jacobian.angular_impulse, jacobian.pad_1);

    let angular_relative = vector_sub(solver_delta_b[2], solver_delta_a[2]);
    let point_a = horus_cross_add(solver_delta_a[2], attachment_a, solver_delta_a[0]);
    let point_b = horus_cross_add(solver_delta_b[2], attachment_b, solver_delta_b[0]);

    let angular_transpose = [
        pack_xyz_word(jacobian.angular_transpose_right, jacobian.pad_2),
        pack_xyz_word(jacobian.angular_transpose_up, jacobian.angular_max_y),
        pack_xyz_word(jacobian.angular_transpose_at, jacobian.angular_min_y),
    ];
    let angular_projection =
        horus_columns_from(angular_transpose, angular_relative, old_angular_impulse);
    let point_relative = vector_sub(point_b, point_a);
    let linear_transpose = [
        pack_xyz_word(
            jacobian.linear_transpose_right,
            f32::from_bits(jacobian.nb_word),
        ),
        pack_xyz_word(jacobian.linear_transpose_up, jacobian.angular_max_x),
        pack_xyz_word(jacobian.linear_transpose_at, jacobian.angular_min_x),
    ];
    let position_projection =
        horus_columns_from(linear_transpose, point_relative, old_position_impulse);

    let angular_minimum = [
        jacobian.angular_min_x,
        jacobian.angular_min_y,
        jacobian.angular_min_z,
        jacobian.angular_min_z,
    ];
    let angular_maximum = [
        jacobian.angular_max_x,
        jacobian.angular_max_y,
        jacobian.angular_max_z,
        jacobian.angular_max_z,
    ];
    let angular_plus_min = vector_add(angular_projection, angular_minimum);
    let angular_plus_max = vector_add(angular_projection, angular_maximum);
    let angular_negative = vector_min(angular_plus_min, zero);
    let angular_positive = vector_max(angular_plus_max, zero);
    let new_angular_impulse = vector_add(angular_positive, angular_negative);

    let position_minimum = pack_xyz_word(jacobian.position_min, jacobian.angular_min_z);
    let position_maximum = pack_xyz_word(jacobian.position_max, jacobian.angular_max_z);
    let position_plus_min = vector_add(position_projection, position_minimum);
    let position_plus_max = vector_add(position_projection, position_maximum);
    let angular_change = vector_sub(new_angular_impulse, old_angular_impulse);
    let position_negative = vector_min(position_plus_min, zero);
    let position_positive = vector_max(position_plus_max, zero);
    let new_position_impulse = vector_add(position_positive, position_negative);

    jacobian.angular_impulse = first3(new_angular_impulse);
    jacobian.pad_1 = new_angular_impulse[3];
    observe_accumulator_store(SimulationJointHorusAccumulator::Angular, jacobian);
    jacobian.position_impulse = first3(new_position_impulse);
    jacobian.pad_0 = new_position_impulse[3];
    observe_accumulator_store(SimulationJointHorusAccumulator::Position, jacobian);
    let position_change = vector_sub(new_position_impulse, old_position_impulse);

    let linear_axes = [
        pack_xyz_word(jacobian.linear_right, f32::from_bits(jacobian.joint_slot.0)),
        pack_xyz_word(jacobian.linear_up, jacobian.pad_3),
        pack_xyz_word(jacobian.linear_at, jacobian.pad_4),
    ];
    let angular_axes = [
        pack_xyz_word(jacobian.angular_right, jacobian.pad_5),
        pack_xyz_word(jacobian.angular_up, jacobian.pad_6),
        pack_xyz_word(jacobian.angular_at, jacobian.pad_7),
    ];
    let linear_response = horus_columns_from(linear_axes, position_change, zero);
    let angular_response = horus_columns_from(angular_axes, angular_change, zero);
    let response_a = horus_cross_add(attachment_a, linear_response, angular_response);
    let response_b = horus_cross_add(attachment_b, linear_response, angular_response);

    let new_a_0 = horus_madd(
        linear_response,
        [jacobian.inverse_mass_a; 4],
        solver_delta_a[0],
    );
    let new_b_0 = horus_nmsub(
        linear_response,
        [jacobian.inverse_mass_b; 4],
        solver_delta_b[0],
    );
    let inverse_inertia_a = [
        pack_xyz_word(jacobian.inverse_inertia_a_right, jacobian.inverse_mass_a),
        pack_xyz_word(jacobian.inverse_inertia_a_up, jacobian.pad_8),
        pack_xyz_word(jacobian.inverse_inertia_a_at, jacobian.pad_9),
    ];
    let inverse_inertia_b = [
        pack_xyz_word(jacobian.inverse_inertia_b_right, jacobian.inverse_mass_b),
        pack_xyz_word(jacobian.inverse_inertia_b_up, jacobian.pad_10),
        pack_xyz_word(jacobian.inverse_inertia_b_at, jacobian.pad_11),
    ];
    let new_a_2 = horus_columns_from(inverse_inertia_a, response_a, solver_delta_a[2]);
    let new_b_2 = horus_negative_columns_from(inverse_inertia_b, response_b, solver_delta_b[2]);

    solver_delta_a[0] = new_a_0;
    solver_delta_b[0] = new_b_0;
    solver_delta_a[2] = new_a_2;
    solver_delta_b[2] = new_b_2;
    SimulationJointHorusRows {
        solver_delta_a,
        solver_delta_b,
    }
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private checked Joint island sweep")
)]
fn first3(value: [f32; 4]) -> [f32; 3] {
    [value[0], value[1], value[2]]
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private checked Joint island sweep")
)]
fn vector_add(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane] + right[lane])
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private checked Joint island sweep")
)]
fn vector_sub(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane] - right[lane])
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private checked Joint island sweep")
)]
fn vector_min(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane].min(right[lane]))
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private checked Joint island sweep")
)]
fn vector_max(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane].max(right[lane]))
}

#[cfg(test)]
mod tests {
    use super::{
        JointSlotToken, SimulationJointHorusAccumulator, SimulationJointJacobian,
        SolverDeltaRowToken, solve_joint_horus_record_host, solve_joint_horus_record_with_observer,
    };
    use std::{
        mem::{align_of, offset_of, size_of},
        num::NonZeroU32,
    };

    fn record_bits(value: &SimulationJointJacobian) -> [[u32; 4]; 24] {
        value.test_words()
    }

    fn values(words: [u32; 4]) -> [f32; 4] {
        words.map(f32::from_bits)
    }

    fn bits(value: [f32; 4]) -> [u32; 4] {
        value.map(f32::to_bits)
    }

    fn xyz(words: [u32; 4]) -> [f32; 3] {
        [
            f32::from_bits(words[0]),
            f32::from_bits(words[1]),
            f32::from_bits(words[2]),
        ]
    }

    fn fixture_record() -> SimulationJointJacobian {
        let rows = [
            [0x3e23_4567, 0xbe87_6543, 0x3f01_2345, 0x3c01_0203],
            [0xbe45_6789, 0x3e76_5432, 0xbecd_ef01, 0xbc11_1213],
            [0x3de1_2345, 0xbda2_3456, 0x3e31_2345, 0x3c21_2223],
            [0xbe11_2345, 0x3dd2_3456, 0xbdb1_2345, 0xbc29_2a2b],
            [0x3f01_2345, 0xbe89_abcd, 0x3e76_5432, 0x3c31_3233],
            [0xbe23_4567, 0x3edc_ba98, 0x3f12_3456, 0xbc39_3a3b],
            [0x3e9a_bcde, 0x3f23_4567, 0xbefe_dcba, 0x3e99_999a],
            [0x3f34_5678, 0xbe76_5432, 0x3eab_cdef, 0x3e4c_cccd],
            [0xbef0_1234, 0x3e13_579b, 0x3f45_6789, 0xbe4c_cccd],
            [0x3e24_68ac, 0xbf01_2345, 0x3e89_abcd, 0xbe99_999a],
            [0x3ecc_cccd, 0x3e99_999a, 0x3f00_0000, 0x3ecc_cccd],
            [0xbe4c_cccd, 0xbecc_cccd, 0xbe99_999a, 0xbe80_0000],
            [0x3f01_2345, 0x3e23_4567, 0xbe45_6789, 0x3c41_4243],
            [0xbe56_789a, 0x3f12_3456, 0x3e76_5432, 0xbc49_4a4b],
            [0x3e89_abcd, 0xbecd_ef01, 0x3f23_4567, 0x3c51_5253],
            [0xbe9a_bcde, 0x3eab_cdef, 0x3f34_5678, 0xbc59_5a5b],
            [0x3f45_6789, 0xbe01_2345, 0x3e13_579b, 0x3c61_6263],
            [0x3e24_68ac, 0x3f56_789a, 0xbefe_dcba, 0xbc69_6a6b],
            [0x3f00_0000, 0x3dcc_cccd, 0xbd4c_cccd, 0x3ecc_cccd],
            [0x3d4c_cccd, 0x3f19_999a, 0x3e19_999a, 0x3c71_7273],
            [0xbdcc_cccd, 0x3e4c_cccd, 0x3f33_3333, 0xbc79_7a7b],
            [0x3f26_6666, 0xbe19_999a, 0x3dcc_cccd, 0x3f19_999a],
            [0x3e33_3333, 0x3f0c_cccd, 0xbdcc_cccd, 0x3c81_8283],
            [0xbe4c_cccd, 0x3e99_999a, 0x3f40_0000, 0xbc89_8a8b],
        ];
        SimulationJointJacobian {
            attachment_offset_a: xyz(rows[0]),
            solver_delta_a: SolverDeltaRowToken(rows[0][3]),
            attachment_offset_b: xyz(rows[1]),
            solver_delta_b: SolverDeltaRowToken(rows[1][3]),
            position_impulse: xyz(rows[2]),
            pad_0: f32::from_bits(rows[2][3]),
            angular_impulse: xyz(rows[3]),
            pad_1: f32::from_bits(rows[3][3]),
            linear_transpose_right: xyz(rows[4]),
            nb_word: rows[4][3],
            angular_transpose_right: xyz(rows[5]),
            pad_2: f32::from_bits(rows[5][3]),
            linear_transpose_up: xyz(rows[6]),
            angular_max_x: f32::from_bits(rows[6][3]),
            angular_transpose_up: xyz(rows[7]),
            angular_max_y: f32::from_bits(rows[7][3]),
            linear_transpose_at: xyz(rows[8]),
            angular_min_x: f32::from_bits(rows[8][3]),
            angular_transpose_at: xyz(rows[9]),
            angular_min_y: f32::from_bits(rows[9][3]),
            position_max: xyz(rows[10]),
            angular_max_z: f32::from_bits(rows[10][3]),
            position_min: xyz(rows[11]),
            angular_min_z: f32::from_bits(rows[11][3]),
            linear_right: xyz(rows[12]),
            joint_slot: JointSlotToken(rows[12][3]),
            linear_up: xyz(rows[13]),
            pad_3: f32::from_bits(rows[13][3]),
            linear_at: xyz(rows[14]),
            pad_4: f32::from_bits(rows[14][3]),
            angular_right: xyz(rows[15]),
            pad_5: f32::from_bits(rows[15][3]),
            angular_up: xyz(rows[16]),
            pad_6: f32::from_bits(rows[16][3]),
            angular_at: xyz(rows[17]),
            pad_7: f32::from_bits(rows[17][3]),
            inverse_inertia_a_right: xyz(rows[18]),
            inverse_mass_a: f32::from_bits(rows[18][3]),
            inverse_inertia_a_up: xyz(rows[19]),
            pad_8: f32::from_bits(rows[19][3]),
            inverse_inertia_a_at: xyz(rows[20]),
            pad_9: f32::from_bits(rows[20][3]),
            inverse_inertia_b_right: xyz(rows[21]),
            inverse_mass_b: f32::from_bits(rows[21][3]),
            inverse_inertia_b_up: xyz(rows[22]),
            pad_10: f32::from_bits(rows[22][3]),
            inverse_inertia_b_at: xyz(rows[23]),
            pad_11: f32::from_bits(rows[23][3]),
        }
    }

    fn fixture_solver_delta_a() -> [[f32; 4]; 4] {
        [
            values([0x3e01_2345, 0xbe23_4567, 0x3e89_abcd, 0x3c91_9293]),
            values([0x3f01_2345, 0x3f12_3456, 0x3f23_4567, 0x3f34_5678]),
            values([0xbe9a_bcde, 0x3eab_cdef, 0xbf12_3456, 0xbc99_9a9b]),
            values([0xbf45_6789, 0xbf56_789a, 0xbf67_89ab, 0xbf78_9abc]),
        ]
    }

    fn fixture_solver_delta_b() -> [[f32; 4]; 4] {
        [
            values([0xbe31_2345, 0x3e42_3456, 0xbe53_4567, 0x3ca1_a2a3]),
            values([0x3d01_2345, 0x3d12_3456, 0x3d23_4567, 0x3d34_5678]),
            values([0x3eb1_2345, 0xbec2_3456, 0x3ed3_4567, 0xbca9_aaab]),
            values([0xbd45_6789, 0xbd56_789a, 0xbd67_89ab, 0xbd78_9abc]),
        ]
    }

    #[test]
    fn layout_matches_pdb_offsets() {
        assert_eq!(size_of::<SimulationJointJacobian>(), 0x180);
        assert_eq!(align_of::<SimulationJointJacobian>(), 0x10);
        assert_eq!(
            [
                offset_of!(SimulationJointJacobian, attachment_offset_a),
                offset_of!(SimulationJointJacobian, solver_delta_a),
                offset_of!(SimulationJointJacobian, attachment_offset_b),
                offset_of!(SimulationJointJacobian, solver_delta_b),
                offset_of!(SimulationJointJacobian, position_impulse),
                offset_of!(SimulationJointJacobian, pad_0),
                offset_of!(SimulationJointJacobian, angular_impulse),
                offset_of!(SimulationJointJacobian, pad_1),
                offset_of!(SimulationJointJacobian, linear_transpose_right),
                offset_of!(SimulationJointJacobian, nb_word),
                offset_of!(SimulationJointJacobian, angular_transpose_right),
                offset_of!(SimulationJointJacobian, pad_2),
                offset_of!(SimulationJointJacobian, linear_transpose_up),
                offset_of!(SimulationJointJacobian, angular_max_x),
                offset_of!(SimulationJointJacobian, angular_transpose_up),
                offset_of!(SimulationJointJacobian, angular_max_y),
                offset_of!(SimulationJointJacobian, linear_transpose_at),
                offset_of!(SimulationJointJacobian, angular_min_x),
                offset_of!(SimulationJointJacobian, angular_transpose_at),
                offset_of!(SimulationJointJacobian, angular_min_y),
                offset_of!(SimulationJointJacobian, position_max),
                offset_of!(SimulationJointJacobian, angular_max_z),
                offset_of!(SimulationJointJacobian, position_min),
                offset_of!(SimulationJointJacobian, angular_min_z),
                offset_of!(SimulationJointJacobian, linear_right),
                offset_of!(SimulationJointJacobian, joint_slot),
                offset_of!(SimulationJointJacobian, linear_up),
                offset_of!(SimulationJointJacobian, pad_3),
                offset_of!(SimulationJointJacobian, linear_at),
                offset_of!(SimulationJointJacobian, pad_4),
                offset_of!(SimulationJointJacobian, angular_right),
                offset_of!(SimulationJointJacobian, pad_5),
                offset_of!(SimulationJointJacobian, angular_up),
                offset_of!(SimulationJointJacobian, pad_6),
                offset_of!(SimulationJointJacobian, angular_at),
                offset_of!(SimulationJointJacobian, pad_7),
                offset_of!(SimulationJointJacobian, inverse_inertia_a_right),
                offset_of!(SimulationJointJacobian, inverse_mass_a),
                offset_of!(SimulationJointJacobian, inverse_inertia_a_up),
                offset_of!(SimulationJointJacobian, pad_8),
                offset_of!(SimulationJointJacobian, inverse_inertia_a_at),
                offset_of!(SimulationJointJacobian, pad_9),
                offset_of!(SimulationJointJacobian, inverse_inertia_b_right),
                offset_of!(SimulationJointJacobian, inverse_mass_b),
                offset_of!(SimulationJointJacobian, inverse_inertia_b_up),
                offset_of!(SimulationJointJacobian, pad_10),
                offset_of!(SimulationJointJacobian, inverse_inertia_b_at),
                offset_of!(SimulationJointJacobian, pad_11),
            ],
            [
                0x00, 0x0c, 0x10, 0x1c, 0x20, 0x2c, 0x30, 0x3c, 0x40, 0x4c, 0x50, 0x5c, 0x60, 0x6c,
                0x70, 0x7c, 0x80, 0x8c, 0x90, 0x9c, 0xa0, 0xac, 0xb0, 0xbc, 0xc0, 0xcc, 0xd0, 0xdc,
                0xe0, 0xec, 0xf0, 0xfc, 0x100, 0x10c, 0x110, 0x11c, 0x120, 0x12c, 0x130, 0x13c,
                0x140, 0x14c, 0x150, 0x15c, 0x160, 0x16c, 0x170, 0x17c,
            ]
        );
    }

    #[test]
    fn zeroed_record_has_no_public_or_private_bindings() {
        let record = SimulationJointJacobian::zeroed();
        assert!(record == SimulationJointJacobian::default());
        assert!(!record.has_solver_delta_bindings());
        assert!(!record.has_joint_binding());
        assert_eq!(record_bits(&record), [[0; 4]; 24]);
    }

    #[test]
    fn inert_row_installer_preserves_every_word_and_owns_three_checked_tokens() {
        let mut rows = std::array::from_fn(|row| {
            let row = u32::try_from(row).expect("24 rows fit u32");
            std::array::from_fn(|lane| {
                0x3c00_0000_u32 + (row * 4) + u32::try_from(lane).expect("four lanes fit u32")
            })
        });
        rows[0][3] = 0xaaaa_aaaa;
        rows[1][3] = 0xbbbb_bbbb;
        rows[12][3] = 0xcccc_cccc;
        let record = SimulationJointJacobian::from_inert_rows_with_bindings(
            &rows,
            NonZeroU32::new(1).expect("nonzero"),
            NonZeroU32::new(7).expect("nonzero"),
            NonZeroU32::new(11).expect("nonzero"),
        );
        rows[0][3] = 1;
        rows[1][3] = 7;
        rows[12][3] = 11;
        assert_eq!(record_bits(&record), rows);
        assert_eq!(record.binding_words(), [1, 7, 11]);
    }

    #[test]
    fn island_horus_record_matches_independent_full_word_oracle() {
        let mut record = fixture_record();
        let mut expected_record = record_bits(&record);
        expected_record[2] = [0x3f36_50e5, 0x3e3d_99ae, 0xbf2f_534e, 0x3f11_a6f7];
        expected_record[3] = [0xbf4a_6ca7, 0x3c7f_9170, 0x3f35_55e4, 0xbf34_ac0d];
        let rows = solve_joint_horus_record_host(
            &mut record,
            fixture_solver_delta_a(),
            fixture_solver_delta_b(),
        );

        assert_eq!(record_bits(&record), expected_record);
        assert_eq!(
            rows.solver_delta_a.map(bits),
            [
                [0x3e08_69ca, 0x3d9e_754c, 0x3ced_e38e, 0x3c74_96b8],
                [0x3f01_2345, 0x3f12_3456, 0x3f23_4567, 0x3f34_5678],
                [0xbe0e_7a46, 0x3f08_3bb1, 0xbf83_ae9a, 0x3d43_f804],
                [0xbf45_6789, 0xbf56_789a, 0xbf67_89ab, 0xbf78_9abc],
            ]
        );
        assert_eq!(
            rows.solver_delta_b.map(bits),
            [
                [0xbe3c_0d0d, 0xbe29_8bbe, 0x3e1d_2356, 0x3cc4_8d76],
                [0x3d01_2345, 0x3d12_3456, 0x3d23_4567, 0x3d34_5678],
                [0xbe0c_c77b, 0xbe5d_ccad, 0x3f93_13b3, 0xbe81_768a],
                [0xbd45_6789, 0xbd56_789a, 0xbd67_89ab, 0xbd78_9abc],
            ]
        );
    }

    #[test]
    fn island_horus_stores_full_angular_then_position_accumulators() {
        let mut record = fixture_record();
        let old_position = record_bits(&record)[2];
        let mut stores = Vec::new();
        let rows = solve_joint_horus_record_with_observer(
            &mut record,
            fixture_solver_delta_a(),
            fixture_solver_delta_b(),
            |accumulator, record| {
                let words = record_bits(record);
                stores.push((accumulator, words[2], words[3]));
            },
        );

        assert_eq!(stores.len(), 2);
        assert_eq!(stores[0].0, SimulationJointHorusAccumulator::Angular);
        assert_eq!(stores[0].1, old_position);
        assert_eq!(
            stores[0].2,
            [0xbf4a_6ca7, 0x3c7f_9170, 0x3f35_55e4, 0xbf34_ac0d]
        );
        assert_eq!(stores[1].0, SimulationJointHorusAccumulator::Position);
        assert_eq!(
            stores[1].1,
            [0x3f36_50e5, 0x3e3d_99ae, 0xbf2f_534e, 0x3f11_a6f7]
        );
        assert_eq!(stores[1].2, stores[0].2);
        assert_eq!(
            rows.solver_delta_a[1].map(f32::to_bits),
            fixture_solver_delta_a()[1].map(f32::to_bits)
        );
        assert_eq!(
            rows.solver_delta_b[3].map(f32::to_bits),
            fixture_solver_delta_b()[3].map(f32::to_bits)
        );
    }

    #[test]
    fn island_horus_self_pair_stages_native_b_wins_commit_values() {
        let mut record = fixture_record();
        record.solver_delta_b = record.solver_delta_a;
        let initial = fixture_solver_delta_a();
        let rows = solve_joint_horus_record_host(&mut record, initial, initial);

        assert_eq!(
            bits(rows.solver_delta_a[0]),
            [0x3e7d_4bfe, 0xbe88_1c46, 0x3ea3_5093, 0x3cb7_9a25]
        );
        assert_eq!(
            bits(rows.solver_delta_b[0]),
            [0xbd64_6743, 0x391d_474d, 0x3e46_6948, 0x3c31_0e6f]
        );
        assert_eq!(
            bits(rows.solver_delta_a[2]),
            [0xbe69_de84, 0x3ee4_b021, 0xbf11_51cd, 0x3d0d_49b3]
        );
        assert_eq!(
            bits(rows.solver_delta_b[2]),
            [0xbe8c_925d, 0x3ec5_097a, 0xbf02_9595, 0x3c44_cb4d]
        );

        let mut final_rf = initial;
        final_rf[0] = rows.solver_delta_a[0];
        final_rf[0] = rows.solver_delta_b[0];
        final_rf[2] = rows.solver_delta_a[2];
        final_rf[2] = rows.solver_delta_b[2];
        assert_eq!(
            final_rf.map(bits),
            [
                [0xbd64_6743, 0x391d_474d, 0x3e46_6948, 0x3c31_0e6f],
                [0x3f01_2345, 0x3f12_3456, 0x3f23_4567, 0x3f34_5678],
                [0xbe8c_925d, 0x3ec5_097a, 0xbf02_9595, 0x3c44_cb4d],
                [0xbf45_6789, 0xbf56_789a, 0xbf67_89ab, 0xbf78_9abc],
            ]
        );
    }
}
