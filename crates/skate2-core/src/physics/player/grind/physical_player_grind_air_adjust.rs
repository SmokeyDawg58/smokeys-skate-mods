//! Retained `GrindAirAdjust` ctor82dcf350 and `InitTargets`82dc9a70.
//! Prediction/activation are separate consumers; this constructor activates none.
#![expect(
    dead_code,
    reason = "retained GrindAirAdjust constructor payload awaits prediction and adjustment consumers"
)]
use crate::{
    attribute_runtime::{
        AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
        AttributeRuntimeVault,
    },
    physical_player_trajectory_owners::{RetainedGrindPrimitive, TrajectoryOwnerBindings},
    reckoning_sin_cos::ReckoningSinCosNumeric,
};
use std::sync::Arc;

#[path = "physical_player_grind_air_update.rs"]
pub(crate) mod update;

const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;
pub(crate) const GRIND_AIR_CLASS: u64 = 0x1cfb_42e0_240b_5b72;
const DECK_LENGTH: u64 = 0xc94d_6509_1dcb_828b;
const TRUCK_OFFSET: u64 = 0xe02d_cbd0_7830_d3a5;
const ZERO_FRAME: [[f32; 4]; 4] = [[0.0; 4]; 4];
const AFFINE: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0; 4],
];

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GrindAirTarget {
    pub(crate) angle: f32,
    pub(crate) angle_range: f32,
    pub(crate) max_distance: f32,
    pub(crate) angle_assist_factor: f32,
    pub(crate) contact: i32,
}

#[derive(Debug)]
pub(crate) enum GrindAirConstructError<N> {
    Attribute(AttributeRuntimeError),
    ValueSize(usize),
    FrameCount(i32),
    Numeric(N),
}
impl<N: std::fmt::Debug> std::fmt::Display for GrindAirConstructError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attribute(e) => write!(f, "grind-air attribute: {e}"),
            Self::ValueSize(n) => write!(f, "grind-air scalar has {n} bytes"),
            Self::FrameCount(n) => write!(
                f,
                "grind-air frame count {n} violates source2..12 assertion"
            ),
            Self::Numeric(e) => write!(f, "grind-air numeric: {e:?}"),
        }
    }
}

/// Globals are retained immutable instances. Every lookup remains independent;
/// this owner does not implement editor replacement of the global singleton.
struct GrindAirAttributes {
    grind: AttributeRuntimeInstance,
    deck: AttributeRuntimeInstance,
    trucks: AttributeRuntimeInstance,
}
impl GrindAirAttributes {
    fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            grind: vault.instance(GRIND_AIR_CLASS, DEFAULT),
            deck: vault.instance(0x9ed2_91a2_7445_3fc0, DEFAULT),
            trucks: vault.instance(0xaf78_1d7b_614d_ab50, DEFAULT),
        }
    }
}
fn pointer<N>(
    instance: &AttributeRuntimeInstance,
    key: u64,
) -> Result<Option<AttributeRuntimeData>, GrindAirConstructError<N>> {
    instance
        .try_get_data(key, 0)
        .map_err(GrindAirConstructError::Attribute)
}
fn word<N>(data: Option<&AttributeRuntimeData>) -> Result<u32, GrindAirConstructError<N>> {
    let Some(data) = data else {
        return Ok(0);
    };
    let bytes: [u8; 4] = data
        .bytes()
        .try_into()
        .map_err(|_| GrindAirConstructError::ValueSize(data.bytes().len()))?;
    Ok(u32::from_be_bytes(bytes))
}
fn scalar<N>(
    instance: &AttributeRuntimeInstance,
    key: u64,
) -> Result<f32, GrindAirConstructError<N>> {
    let data = pointer(instance, key)?;
    word(data.as_ref()).map(f32::from_bits)
}

/// Sim allocation82e03fb0 establishes zero residual storage before ctor. Later
/// Start may copy unknown caller input, which must replace these values as such.
#[expect(
    clippy::struct_excessive_bools,
    reason = "distinct source input, activation and selection flags"
)]
pub(crate) struct GrindAirAdjustOwner {
    pub(crate) bindings: TrajectoryOwnerBindings,
    pub(crate) targets: [GrindAirTarget; 6],
    pub(crate) input_primitive: RetainedGrindPrimitive,
    pub(crate) input_geometry_type: i32,
    pub(crate) input_is_garbage: bool,
    pub(crate) input_directions: [[f32; 4]; 4],
    pub(crate) input_target_orientations: [[[f32; 4]; 4]; 6],
    pub(crate) started: bool,
    pub(crate) activated: bool,
    pub(crate) adjusting: bool,
    pub(crate) predicted_decks: [[[f32; 4]; 4]; 12],
    pub(crate) contact_offsets: [[f32; 4]; 5],
    pub(crate) predicted_contacts: [[[f32; 4]; 5]; 12],
    pub(crate) previous_com_to_deck: [f32; 4],
    pub(crate) contacts: [i32; 9],
    pub(crate) types: [i32; 9],
    pub(crate) plane_intersections: [[f32; 4]; 5],
    pub(crate) nearest_points: [[f32; 4]; 5],
    pub(crate) world_offsets_to_hit: [[f32; 4]; 5],
    pub(crate) frames_to_plane: [f32; 5],
    pub(crate) distances_to_grind: [f32; 5],
    pub(crate) deck_angles: [f32; 12],
    pub(crate) target_orientations: [[[f32; 4]; 4]; 9],
    pub(crate) grind_transform: [[f32; 4]; 4],
    pub(crate) grind_velocity_transform: [[f32; 4]; 4],
    pub(crate) position_adjust_per_frame: [f32; 4],
    pub(crate) total_position_adjust: [f32; 4],
    pub(crate) orientation_adjust_per_frame: [f32; 4],
    pub(crate) total_orientation_adjust: [f32; 4],
    pub(crate) projected_deck_z: [f32; 4],
    pub(crate) chosen_contact_valid: bool,
    pub(crate) chosen_contact: i32,
    pub(crate) chosen_type: i32,
    pub(crate) last_chosen_type: i32,
    pub(crate) chosen_angle: f32,
    pub(crate) x_angle_distance: f32,
    pub(crate) y_angle_distance: f32,
    pub(crate) closest_frame: f32,
    pub(crate) num_frames: i32,
}
impl GrindAirAdjustOwner {
    /// Retain this prefix in the outer owner BEFORE invoking fallible `InitTargets`.
    pub(crate) fn constructor_prefix(bindings: TrajectoryOwnerBindings) -> Self {
        Self {
            bindings,
            targets: [GrindAirTarget::default(); 6],
            input_primitive: RetainedGrindPrimitive::in_sim_allocation(),
            input_geometry_type: 0,
            input_is_garbage: false,
            input_directions: [[0.0; 4]; 4],
            input_target_orientations: [ZERO_FRAME; 6],
            started: false,
            activated: false,
            adjusting: false,
            predicted_decks: [ZERO_FRAME; 12],
            contact_offsets: [[0.0; 4]; 5],
            predicted_contacts: [[[0.0; 4]; 5]; 12],
            previous_com_to_deck: [0.0; 4],
            contacts: [0; 9],
            types: [0; 9],
            plane_intersections: [[0.0; 4]; 5],
            nearest_points: [[0.0; 4]; 5],
            world_offsets_to_hit: [[0.0; 4]; 5],
            frames_to_plane: [0.0; 5],
            distances_to_grind: [0.0; 5],
            deck_angles: [0.0; 12],
            target_orientations: [ZERO_FRAME; 9],
            grind_transform: AFFINE,
            grind_velocity_transform: AFFINE,
            position_adjust_per_frame: [0.0; 4],
            total_position_adjust: [0.0; 4],
            orientation_adjust_per_frame: [0.0; 4],
            total_orientation_adjust: [0.0; 4],
            projected_deck_z: [0.0; 4],
            chosen_contact_valid: false,
            chosen_contact: 0,
            chosen_type: 0,
            last_chosen_type: 0,
            chosen_angle: 0.0,
            x_angle_distance: 0.0,
            y_angle_distance: 0.0,
            closest_frame: 0.0,
            num_frames: 0,
        }
    }

    pub(crate) fn try_init_targets<R: ReckoningSinCosNumeric>(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        rounding: &mut R,
    ) -> Result<(), GrindAirConstructError<R::Error>> {
        let a = GrindAirAttributes::bind(vault);
        let count = pointer(&a.grind, 0x9759_3d0d_4f34_2621)?;
        self.num_frames = i32::from_ne_bytes(word(count.as_ref())?.to_ne_bytes());
        if !(2..=12).contains(&self.num_frames) {
            return Err(GrindAirConstructError::FrameCount(self.num_frames));
        }
        let half_length = scalar(&a.deck, DECK_LENGTH)? * 0.5;
        let end_scale = scalar(&a.grind, 0xb2b1_3ea9_e606_a530)?;
        let end_length = scalar(&a.deck, 0x49f4_29ae_2e66_f063)? * end_scale;
        let angle = scalar(&a.deck, 0x00e9_49f6_d567_bb10)?;
        // Raw9ce8 stores converted angle,9cec reloads it,9cfc supplies v1.
        // The old decompiler-only packet incorrectly called this a literal.
        let radians = angle * f32::from_bits(0x3c8e_fa35);
        let sine = crate::skeleton_normal_displacement::try_sine(radians, rounding)
            .map_err(GrindAirConstructError::Numeric)?;
        let end_height = sine * end_length;
        let length_pointer = pointer(&a.deck, DECK_LENGTH)?;
        let truck_pointer = pointer(&a.trucks, TRUCK_OFFSET)?;
        let truck_z = f32::from_bits(word(length_pointer.as_ref())?)
            .mul_add(0.5, f32::from_bits(word(truck_pointer.as_ref())?));
        let truck_y = scalar(&a.trucks, 0x2be0_8dd7_a920_7622)?;
        let end_z = end_length + half_length;
        // Original source store order nose,tail,deck,frontTruck,backTruck.
        self.contact_offsets[1] = [0.0, end_height, end_z, 0.0];
        self.contact_offsets[4] = [0.0, end_height, -end_z, 0.0];
        self.contact_offsets[0] = [0.0; 4];
        self.contact_offsets[2] = [0.0, truck_y, truck_z, 0.0];
        self.contact_offsets[3] = [0.0, truck_y, -truck_z, 0.0];
        self.targets[0].contact = 1;
        self.targets[2].angle = 90.0;
        self.targets[2].contact = 0;
        self.targets[0].angle = 90.0;
        self.targets[1].contact = 4;
        self.targets[1].angle = 90.0;
        self.targets[3].contact = 2;
        self.targets[3].angle = 0.0;
        self.targets[4].contact = 2;
        self.targets[4].angle = 0.0;
        self.targets[5].contact = 3;
        self.targets[5].angle = 0.0;
        for (target, key) in [
            (2, 0xff18_6b4d_8af4_aba6),
            (0, 0x9ff1_7e8b_68e5_955e),
            (1, 0x9ff1_7e8b_68e5_955e),
            (3, 0xeeb4_35be_3ced_c175),
            (4, 0x5eaa_1cc2_8ee5_a6f7),
            (5, 0x5eaa_1cc2_8ee5_a6f7),
        ] {
            self.targets[target].angle_range = scalar(&a.grind, key)?;
        }
        let length_pointer = pointer(&a.deck, DECK_LENGTH)?;
        let truck_pointer = pointer(&a.trucks, TRUCK_OFFSET)?;
        let distance_base = f32::from_bits(word(length_pointer.as_ref())?)
            .mul_add(0.5, f32::from_bits(word(truck_pointer.as_ref())?));
        // Multiplicand order differs by target in source; retain it for NaNs.
        for (target, key, attribute_first) in [
            (2, 0xb9f6_b1cf_478e_0694, true),
            (0, 0x311e_6e5f_4577_7527, false),
            (1, 0x311e_6e5f_4577_7527, true),
            (3, 0x444c_bfc0_6344_8664, false),
            (4, 0xf3e7_ee4d_c044_bd5e, true),
            (5, 0xf3e7_ee4d_c044_bd5e, false),
        ] {
            let value = scalar(&a.grind, key)?;
            #[expect(
                clippy::if_same_then_else,
                reason = "preserve source floating-point operand order, including NaN inputs"
            )]
            let distance = if attribute_first {
                value * distance_base
            } else {
                distance_base * value
            };
            self.targets[target].max_distance = distance;
        }
        for (target, key) in [
            (2, 0xb851_a40a_680c_ee25),
            (0, 0x6a96_f60d_9b1b_26ee),
            (1, 0x6a96_f60d_9b1b_26ee),
            (3, 0x9947_5fdf_5292_ca3d),
            (4, 0x8790_108f_0510_dbc6),
            (5, 0x8790_108f_0510_dbc6),
        ] {
            self.targets[target].angle_assist_factor = scalar(&a.grind, key)?;
        }
        Ok(())
    }
}
