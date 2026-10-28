//! Same moving original pair, explicit selected Update1 schedule.
use super::{PhysicalAssemblyWorld, jump_corpus::record, phys_out_corpus::Gaps};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle, input_pad::InputPad,
    physical_player_phys_out::PhysOutOrigin, reckoning_numeric::HostReckoningNumeric,
    simulation_assembly_backend::constraint_session::physical_jump_events::PhysicalPlayerJumpInputBinding,
};
use std::sync::Arc;
pub(crate) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) {
    let owner = Arc::clone(world.physical_body_member(board).unwrap().owner());
    let processed = Arc::clone(owner.reckoning().lock().unwrap().processed());
    // Earlier selected Reset then fresh source prior copies: do not consume
    // invalidated old prior values as if a complete new Update1 had occurred.
    processed.lock().unwrap().reset();
    world
        .try_with_pair_phys_out(board, skeleton, |output| {
            processed
                .lock()
                .unwrap()
                .try_copy_prior_output(output, &mut Gaps::default())
                .unwrap();
        })
        .unwrap();
    let (velocity, surface) = {
        let state = processed.lock().unwrap();
        (
            state.prior_output().velocity().unwrap(),
            state.prior_output().surface_velocity().unwrap(),
        )
    };
    assert_eq!(velocity.origin, PhysOutOrigin::Publication);
    assert_eq!(surface.origin, PhysOutOrigin::Publication);
    assert_eq!(surface.value.w, None);
    let old = owner.prepare_velocity().lock().unwrap().velocity().unwrap();
    let mut ai = PhysicalPlayerJumpInputBinding::<InputPad>::AiNull;
    world
        .try_process_pair_selected_jump_contact_events(
            board,
            skeleton,
            &[record("Spin", 1.0)],
            &mut ai,
        )
        .unwrap();
    // Independent f64 direction/increment oracle; source implementation keeps its
    // own f32 estimate/refinement/FMA ordering, so this comparison is tolerance-based.
    let squared: f64 = surface
        .value
        .xyz
        .iter()
        .map(|v| f64::from(*v).powi(2))
        .sum();
    let length = squared.sqrt();
    assert!(length > 0.0);
    let direction = surface.value.xyz.map(|v| f64::from(v) / length);
    let acceleration = -direction[1] * f64::from(f32::from_bits(0x411c_cccd));
    let step = f64::from(f32::from_bits(0x3c88_8889));
    for iteration in 0..2 {
        world
            .try_update_pair_prepare_velocity(board, skeleton, &mut HostReckoningNumeric)
            .unwrap();
        let state = processed.lock().unwrap();
        let result = state.prepare_output().velocity().unwrap();
        assert!(state.prepare_output().complete());
        assert_eq!(result.origin, PhysOutOrigin::Publication);
        assert_eq!(result.value.w, None);
        for (lane, old_lane) in old.xyz.into_iter().enumerate() {
            let expected = f64::from(old_lane)
                + f64::from(iteration + 1) * direction[lane] * acceleration * step;
            assert!((f64::from(result.value.xyz[lane]) - expected).abs() < 0.000_001);
        }
        let count = state.prepare_output().count().unwrap();
        assert_eq!(count.origin, PhysOutOrigin::Publication);
        assert_eq!(count.value, u32::try_from(iteration).unwrap());
    }
    world
        .try_process_pair_selected_jump_contact_events(
            board,
            skeleton,
            &[record("PrepareJump", 0.0)],
            &mut ai,
        )
        .unwrap();
    world
        .try_update_pair_prepare_velocity(board, skeleton, &mut HostReckoningNumeric)
        .unwrap();
    let state = processed.lock().unwrap();
    let result = state.prepare_output().velocity().unwrap();
    assert_eq!(
        result.value.xyz.map(f32::to_bits),
        velocity.value.xyz.map(f32::to_bits)
    );
    assert_eq!(result.value.w, velocity.value.w);
    assert_eq!(state.prepare_output().count().unwrap().value, 2);
    assert_eq!(
        owner.prepare_velocity().lock().unwrap().velocity(),
        Some(result.value)
    );
    println!(
        "SAME actual moving prior surface -> retained HiLOD prepared history twice with independent f64 oracle; actual Spin event counts0/1, PrepareJump copies actual prior velocity exactly with unknown W/count2; controlled selected Update1 scheduling, no Actor tick claim"
    );
}
