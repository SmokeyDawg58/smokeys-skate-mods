//! Ordinary-finite host reconstruction of `RigidBody::DynamicUpdate`.
//!
//! This module is private because it is an implementation seam for the body
//! pool, not a general public physics API. The operation graph and mutation
//! order come from `0x82ae4488..0x82ae4904`. Exact Xenon `vrsqrtefp128` seed
//! bits, `vmsum3fp128`/`vmsum4fp128` reduction rounding, denormal handling,
//! NaN payloads, infinities, and exceptional signed-zero behavior remain
//! outside this host model.

use crate::{
    simulation_rigid_body_pool::SimulationInertia, world::simulation_rotated_inverse_inertia_host,
};

pub(super) struct SimulationDynamicUpdateBody<'a> {
    pub quaternion: &'a mut [f32; 4],
    pub center_of_mass: &'a mut [f32; 3],
    pub velocity: &'a mut [f32; 3],
    pub omega: &'a mut [f32; 3],
    pub right: &'a mut [f32; 3],
    pub up: &'a mut [f32; 3],
    pub at: &'a mut [f32; 3],
    pub inverse_inertia_full: &'a mut [f32; 3],
    pub inverse_mass: f32,
    pub inverse_inertia_split: &'a mut [f32; 3],
    pub force: &'a mut [f32; 3],
    pub kinematic_value: &'a mut f32,
    pub torque: &'a mut [f32; 3],
    pub cooldown: &'a mut u32,
    pub solver_delta: &'a mut [[f32; 4]; 4],
}

#[derive(Clone, Copy)]
pub(super) struct SimulationDynamicUpdateConfiguration {
    pub timestep: f32,
    pub frequency: f32,
    pub gravity: [f32; 3],
    pub maximum_cooldown: u32,
    pub minimum_energy: f32,
}

/// Applies one source-ordered `DynamicUpdate` to a prevalidated Active body.
///
/// All arguments are infallible semantic storage. Callers must validate every
/// retained ring and every Active body's inertia before invoking this function;
/// there is deliberately no rollback or callback after the first COM store.
#[expect(
    clippy::needless_pass_by_value,
    reason = "consuming the bundle prevents reuse of its unique field borrows"
)]
pub(super) fn simulation_dynamic_update(
    body: SimulationDynamicUpdateBody<'_>,
    inertia: SimulationInertia,
    configuration: SimulationDynamicUpdateConfiguration,
) {
    let timestep = [configuration.timestep; 4];
    let force = xyz(*body.force);
    let velocity = xyz(*body.velocity);
    let torque = xyz(*body.torque);
    let omega = xyz(*body.omega);
    let center_of_mass = xyz(*body.center_of_mass);

    let pre_drag_velocity = vmadd(force, timestep, velocity);
    let center_with_direct_delta = vadd(center_of_mass, body.solver_delta[1]);
    let pre_drag_omega = vmadd(torque, timestep, omega);
    let linear_step = vmadd(pre_drag_velocity, timestep, body.solver_delta[0]);
    let angular_step = vmadd(pre_drag_omega, timestep, body.solver_delta[2]);

    // Native first observable body store: COM at 0x82ae4544.
    *body.center_of_mass = first3(vadd(center_with_direct_delta, linear_step));

    let angular_pose_step = vadd(angular_step, body.solver_delta[3]);
    let quaternion = quaternion_step(*body.quaternion, angular_pose_step);

    // Quaternion store at 0x82ae45c8.
    *body.quaternion = quaternion;

    let basis = basis_from_quaternion(quaternion);
    // Native basis store order is Ri, Up, At.
    *body.right = first3(basis[0]);
    *body.up = first3(basis[1]);
    *body.at = first3(basis[2]);

    let basis_matrix = [
        basis[0][0],
        basis[0][1],
        basis[0][2],
        0.0,
        basis[1][0],
        basis[1][1],
        basis[1][2],
        0.0,
        basis[2][0],
        basis[2][1],
        basis[2][2],
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ];
    let rotated = simulation_rotated_inverse_inertia_host(
        basis_matrix,
        inertia.inverse_tensor,
        body.inverse_mass,
    );
    // Native tensor store order is mIfull, then mIsplt. mInvm is retained.
    *body.inverse_inertia_full = rotated.full;
    *body.inverse_inertia_split = rotated.split;

    let angular_factor = positive_or_zero(configuration.frequency - inertia.angular_drag);
    let linear_factor = positive_or_zero(configuration.frequency - inertia.linear_drag);
    let angular_candidate = vmul(angular_step, [angular_factor; 4]);
    let linear_candidate = vmul(linear_step, [linear_factor; 4]);
    let (final_omega, angular_squared, _) = clamp_speed(angular_candidate, inertia.max_omega);
    let (final_velocity, linear_squared, _) = clamp_speed(linear_candidate, inertia.max_velocity);

    let angular_weight = inertia.spherical * body.inverse_mass;
    let kinematic_value = angular_weight.mul_add(angular_squared, linear_squared);
    if kinematic_value.partial_cmp(&configuration.minimum_energy) == Some(std::cmp::Ordering::Less)
    {
        if kinematic_value.partial_cmp(&*body.kinematic_value) != Some(std::cmp::Ordering::Greater)
        {
            *body.cooldown = (*body.cooldown).wrapping_add(1);
        }
        if *body.cooldown > configuration.maximum_cooldown {
            *body.cooldown = configuration.maximum_cooldown;
        }
    } else {
        // `fcmpu` unordered takes the same reset path as greater/equal.
        *body.cooldown = 0;
    }
    *body.kinematic_value = kinematic_value;

    // Native final dynamic-vector store order is velocity, then omega.
    *body.velocity = first3(final_velocity);
    *body.omega = first3(final_omega);

    // All four lanes are cleared in descending RF-vector address order.
    body.solver_delta[3] = [0.0; 4];
    body.solver_delta[2] = [0.0; 4];
    body.solver_delta[1] = [0.0; 4];
    body.solver_delta[0] = [0.0; 4];

    // Native reloads current Simulation gravity here and preserves adjacent W.
    *body.force = configuration.gravity;
    *body.torque = [0.0; 3];
}

fn xyz(value: [f32; 3]) -> [f32; 4] {
    [value[0], value[1], value[2], 0.0]
}

fn first3(value: [f32; 4]) -> [f32; 3] {
    [value[0], value[1], value[2]]
}

fn vadd(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane] + right[lane])
}

fn vsub(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane] - right[lane])
}

fn vmul(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane] * right[lane])
}

fn vmadd(multiplier: [f32; 4], multiplicand: [f32; 4], addend: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| multiplier[lane].mul_add(multiplicand[lane], addend[lane]))
}

fn vnmsub(multiplier: [f32; 4], multiplicand: [f32; 4], addend: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| (-multiplier[lane]).mul_add(multiplicand[lane], addend[lane]))
}

fn yzxw(value: [f32; 4]) -> [f32; 4] {
    [value[1], value[2], value[0], value[3]]
}

fn yzxx(value: [f32; 4]) -> [f32; 4] {
    [value[1], value[2], value[0], value[0]]
}

fn zxyx(value: [f32; 4]) -> [f32; 4] {
    [value[2], value[0], value[1], value[0]]
}

fn dot3(left: [f32; 4], right: [f32; 4]) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn dot4(value: [f32; 4]) -> f32 {
    value[0].mul_add(
        value[0],
        value[1].mul_add(value[1], value[2].mul_add(value[2], value[3] * value[3])),
    )
}

fn refined_reciprocal_square_root(value: f32) -> f32 {
    // `sqrt().recip()` is the explicit safe-host seed policy. Xenon uses
    // `vrsqrtefp128`; both following fused Newton rounds are source-shaped.
    let estimate0 = value.sqrt().recip();
    let square0 = estimate0 * estimate0;
    let half0 = estimate0 * 0.5;
    let residual0 = (-value).mul_add(square0, 1.0);
    let estimate1 = half0.mul_add(residual0, estimate0);
    let square1 = estimate1 * estimate1;
    let half1 = estimate1 * 0.5;
    let residual1 = (-value).mul_add(square1, 1.0);
    half1.mul_add(residual1, estimate1)
}

fn quaternion_step(quaternion: [f32; 4], angular_pose_step: [f32; 4]) -> [f32; 4] {
    let products = vmul(angular_pose_step, yzxw(quaternion));
    let rotated_cross = vnmsub(yzxw(angular_pose_step), quaternion, products);
    let cross = yzxw(rotated_cross);
    let quaternion_w = [quaternion[3]; 4];
    let mut increment = vmul(vmadd(angular_pose_step, quaternion_w, cross), [0.5; 4]);
    increment[3] = dot3(angular_pose_step, quaternion) * -0.5;
    let candidate = vadd(quaternion, increment);
    vmul(
        candidate,
        [refined_reciprocal_square_root(dot4(candidate)); 4],
    )
}

fn basis_from_quaternion(quaternion: [f32; 4]) -> [[f32; 4]; 3] {
    let scaled = vmul(quaternion, [f32::from_bits(0x3fb5_04f3); 4]);
    let negative_squares = vnmsub(scaled, scaled, [0.5; 4]);
    // `vpermwi128 0x60` is YZXX. The fourth lane is later overwritten in the
    // semantic basis stores, but retaining the decoded permutation avoids a
    // false full-vector claim.
    let products = vmul(scaled, yzxx(scaled));
    let w_products = vmul([scaled[3]; 4], zxyx(scaled));
    let sum = vadd(products, w_products);
    let difference = vsub(products, w_products);
    let diagonal = vadd(negative_squares, yzxx(negative_squares));
    [
        [diagonal[1], sum[0], difference[2], 0.0],
        [difference[0], diagonal[2], sum[1], 0.0],
        [sum[2], difference[1], diagonal[0], 0.0],
    ]
}

fn positive_or_zero(value: f32) -> f32 {
    if value > 0.0 { value } else { 0.0 }
}

fn speed_exceeds_limit(squared: f32, maximum_squared: f32) -> bool {
    squared > maximum_squared
}

fn clamp_speed(mut value: [f32; 4], maximum: f32) -> ([f32; 4], f32, bool) {
    let mut squared = dot3(value, value);
    let maximum_squared = maximum * maximum;
    if speed_exceeds_limit(squared, maximum_squared) {
        let ratio = maximum_squared / squared;
        // Native evaluates the estimate and both Newton rounds before the
        // scalar zero select. The host deliberately preserves that dataflow;
        // FPSCR side effects and exceptional estimate bits remain out of scope.
        let candidate_scale = ratio * refined_reciprocal_square_root(ratio);
        let scale = if ratio == 0.0 { 0.0 } else { candidate_scale };
        value = vmul(value, [scale; 4]);
        squared = maximum_squared;
        return (value, squared, true);
    }
    (value, squared, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct Fixture {
        quaternion: [f32; 4],
        center_of_mass: [f32; 3],
        velocity: [f32; 3],
        omega: [f32; 3],
        right: [f32; 3],
        up: [f32; 3],
        at: [f32; 3],
        inverse_inertia_full: [f32; 3],
        inverse_mass: f32,
        inverse_inertia_split: [f32; 3],
        force: [f32; 3],
        kinematic_value: f32,
        torque: [f32; 3],
        cooldown: u32,
        solver_delta: [[f32; 4]; 4],
        inertia: SimulationInertia,
        configuration: SimulationDynamicUpdateConfiguration,
    }

    impl Fixture {
        fn run(&mut self) {
            simulation_dynamic_update(
                SimulationDynamicUpdateBody {
                    quaternion: &mut self.quaternion,
                    center_of_mass: &mut self.center_of_mass,
                    velocity: &mut self.velocity,
                    omega: &mut self.omega,
                    right: &mut self.right,
                    up: &mut self.up,
                    at: &mut self.at,
                    inverse_inertia_full: &mut self.inverse_inertia_full,
                    inverse_mass: self.inverse_mass,
                    inverse_inertia_split: &mut self.inverse_inertia_split,
                    force: &mut self.force,
                    kinematic_value: &mut self.kinematic_value,
                    torque: &mut self.torque,
                    cooldown: &mut self.cooldown,
                    solver_delta: &mut self.solver_delta,
                },
                self.inertia,
                self.configuration,
            );
        }
    }

    fn default_fixture() -> Fixture {
        let timestep = f32::from_bits(0x3c88_8889);
        Fixture {
            quaternion: [0.0, 0.0, 0.0, 1.0],
            center_of_mass: [0.0; 3],
            velocity: [0.0; 3],
            omega: [0.0; 3],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            at: [0.0, 0.0, 1.0],
            inverse_inertia_full: [1.0, 0.0, 0.0],
            inverse_mass: 1.0,
            inverse_inertia_split: [1.0, 1.0, 0.0],
            force: [0.0, -600.0, 0.0],
            kinematic_value: f32::MAX,
            torque: [0.0; 3],
            cooldown: 0,
            solver_delta: [[0.0; 4]; 4],
            inertia: SimulationInertia::native_default(),
            configuration: SimulationDynamicUpdateConfiguration {
                timestep,
                frequency: 1.0 / timestep,
                gravity: [0.0, -600.0, 0.0],
                maximum_cooldown: 30,
                minimum_energy: 1.0,
            },
        }
    }

    fn bits3(value: [f32; 3]) -> [u32; 3] {
        value.map(f32::to_bits)
    }

    fn bits4(value: [f32; 4]) -> [u32; 4] {
        value.map(f32::to_bits)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one compact assertion freezes every observable oracle field"
    )]
    fn assert_common(
        fixture: &Fixture,
        quaternion: [u32; 4],
        center_of_mass: [u32; 3],
        right: [u32; 3],
        up: [u32; 3],
        at: [u32; 3],
        full: [u32; 3],
        split: [u32; 3],
        velocity: [u32; 3],
        omega: [u32; 3],
        kinematic_value: u32,
        cooldown: u32,
    ) {
        assert_eq!(bits4(fixture.quaternion), quaternion);
        assert_eq!(bits3(fixture.center_of_mass), center_of_mass);
        assert_eq!(bits3(fixture.right), right);
        assert_eq!(bits3(fixture.up), up);
        assert_eq!(bits3(fixture.at), at);
        assert_eq!(bits3(fixture.inverse_inertia_full), full);
        assert_eq!(bits3(fixture.inverse_inertia_split), split);
        assert_eq!(bits3(fixture.velocity), velocity);
        assert_eq!(bits3(fixture.omega), omega);
        assert_eq!(fixture.kinematic_value.to_bits(), kinematic_value);
        assert_eq!(fixture.cooldown, cooldown);
        assert_eq!(bits3(fixture.force), [0, 0xc416_0000, 0]);
        assert_eq!(bits3(fixture.torque), [0; 3]);
        assert_eq!(fixture.solver_delta, [[0.0; 4]; 4]);
    }

    #[test]
    fn default_fall_matches_full_ordinary_finite_oracle() {
        let mut fixture = default_fixture();
        fixture.run();
        assert_common(
            &fixture,
            [0, 0, 0, 0x3f80_0000],
            [0, 0xbe2a_aaac, 0],
            [0x3f80_0000, 0, 0],
            [0, 0x3f80_0000, 0],
            [0, 0, 0x3f80_0000],
            [0x3f80_0000, 0, 0],
            [0x3f80_0000, 0x3f80_0000, 0],
            [0, 0xc120_0001, 0],
            [0; 3],
            0x42c8_0003,
            0,
        );
    }

    #[test]
    fn mixed_deltas_and_both_clamps_match_full_ordinary_finite_oracle() {
        let mut fixture = default_fixture();
        fixture.quaternion = [0.182_574_18, -0.365_148_37, 0.547_722_6, 0.730_296_73];
        fixture.center_of_mass = [1.25, -2.5, 3.75];
        fixture.velocity = [3.25, -4.5, 5.75];
        fixture.omega = [-2.0, 1.5, 0.75];
        fixture.force = [10.0, -40.0, 7.0];
        fixture.torque = [5.0, -3.0, 9.0];
        fixture.solver_delta = [
            [0.125, -0.25, 0.5, 99.0],
            [-7.0, 8.0, -9.0, 101.0],
            [0.05, -0.025, 0.0125, 77.0],
            [-0.02, 0.03, -0.04, 55.0],
        ];
        fixture.inertia = SimulationInertia {
            inverse_tensor: [0.7, 1.1, 1.6],
            inverse_mass: 0.25,
            spherical: 2.75,
            max_velocity: 4.0,
            max_omega: 1.25,
            linear_drag: 2.5,
            angular_drag: 1.75,
        };
        fixture.inverse_mass = 0.25;
        fixture.configuration.minimum_energy = 999.0;
        fixture.kinematic_value = 18.0;
        fixture.cooldown = 29;
        fixture.run();
        assert_common(
            &fixture,
            [0x3e40_0c0a, 0xbeb5_caad, 0x3f0a_70b7, 0x3f3d_3793],
            [0xc0b2_2d83, 0x40a5_3e94, 0xc094_df01],
            [0x3e26_e1da, 0x3f2a_8e64, 0x3f3a_4b9e],
            [0xbf6e_beac, 0x3eb0_84a1, 0xbdda_b00c],
            [0xbea4_e124, 0xbf29_4896, 0x3f2d_71a2],
            [0x3f92_13c1, 0x3d81_1c86, 0xbe20_4918],
            [0x3f8f_104e, 0x3f92_0f25, 0xbed5_fc46],
            [0x3f83_4b67, 0xbff2_8b4a, 0x4057_af10],
            [0x3f2f_92a0, 0xbd01_a749, 0x3f85_b496],
            0x4188_9800,
            30,
        );
    }

    #[test]
    fn nonpositive_drag_factors_match_full_ordinary_finite_oracle() {
        let mut fixture = default_fixture();
        fixture.velocity = [1.0, 2.0, 3.0];
        fixture.omega = [4.0, 5.0, 6.0];
        fixture.force = [0.0; 3];
        fixture.configuration.frequency = 10.0;
        fixture.inertia.linear_drag = 10.0;
        fixture.inertia.angular_drag = 11.0;
        fixture.kinematic_value = 0.0;
        fixture.cooldown = u32::MAX;
        fixture.run();
        assert_common(
            &fixture,
            [0x3d08_2b75, 0x3d2a_3652, 0x3d4c_4130, 0x3f7f_517a],
            [0x3c88_8889, 0x3d08_8889, 0x3d4c_ccce],
            [0x3f7d_d7b8, 0x3dd1_5e8d, 0xbda2_f7f3],
            [0xbdc6_0d55, 0x3f7e_2934, 0x3d90_4b8a],
            [0x3db0_8c9d, 0xbd7e_a36c, 0x3f7e_8ccb],
            [0x3f80_0000, 0xb0f2_524f, 0xb129_4702],
            [0x3f80_0000, 0x3f80_0000, 0xb0e5_4d48],
            [0; 3],
            [0; 3],
            0,
            0,
        );
    }

    #[test]
    fn speed_limit_comparison_is_strict_and_clamps_one_ulp_over() {
        assert!(!speed_exceeds_limit(
            f32::from_bits(0x4080_0000),
            f32::from_bits(0x4080_0000)
        ));
        assert!(speed_exceeds_limit(
            f32::from_bits(0x4080_0001),
            f32::from_bits(0x4080_0000)
        ));
        assert!(!speed_exceeds_limit(
            f32::from_bits(0x407f_ffff),
            f32::from_bits(0x4080_0000)
        ));
        assert!(!speed_exceeds_limit(f32::NAN, 1.0));
        assert!(!speed_exceeds_limit(1.0, f32::NAN));

        let equality = [3.0, 4.0, 0.0, 7.0];
        let (equal_value, equal_squared, equal_clamped) = clamp_speed(equality, 5.0);
        assert_eq!(equal_value.map(f32::to_bits), equality.map(f32::to_bits));
        assert_eq!(equal_squared.to_bits(), 25.0_f32.to_bits());
        assert!(!equal_clamped);

        let one_ulp_over = [f32::from_bits(0x3f80_0001), 0.0, 0.0, 7.0];
        let (over_value, over_squared, over_clamped) = clamp_speed(one_ulp_over, 1.0);
        assert!(over_clamped);
        assert_eq!(over_squared.to_bits(), 1.0_f32.to_bits());
        assert_eq!(over_value[0].to_bits(), 1.0_f32.to_bits());
    }

    #[test]
    fn basis_permutation_freezes_the_discarded_yzxx_lane() {
        assert_eq!(
            bits4(yzxx([1.0, 2.0, 3.0, 4.0])),
            bits4([2.0, 3.0, 1.0, 1.0])
        );
    }

    #[test]
    fn unordered_energy_comparisons_take_both_proven_cooldown_routes() {
        let mut unordered_minimum = default_fixture();
        unordered_minimum.configuration.minimum_energy = f32::from_bits(0x7fc1_2345);
        unordered_minimum.cooldown = 17;
        unordered_minimum.run();
        assert!(unordered_minimum.kinematic_value.is_finite());
        assert_eq!(unordered_minimum.cooldown, 0);

        let mut unordered_previous = default_fixture();
        unordered_previous.force = [0.0; 3];
        unordered_previous.inertia.linear_drag = unordered_previous.configuration.frequency;
        unordered_previous.inertia.angular_drag = unordered_previous.configuration.frequency;
        unordered_previous.kinematic_value = f32::from_bits(0x7fc5_4321);
        unordered_previous.cooldown = 7;
        unordered_previous.run();
        assert_eq!(unordered_previous.kinematic_value.to_bits(), 0);
        assert_eq!(unordered_previous.cooldown, 8);
    }
}
