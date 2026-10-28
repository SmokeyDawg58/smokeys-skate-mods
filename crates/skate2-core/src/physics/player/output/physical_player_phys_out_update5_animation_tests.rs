use super::*;
use crate::{
    physical_player_phys_out::construct::PhysOutAllocation,
    skeleton_root_transforms::SkeletonRootVector,
};
#[derive(Default)]
struct Source {
    calls: usize,
}
impl Update5AnimationSource for Source {
    type Error = ();
    fn capture_carving_input(&mut self) -> Result<(), ()> {
        self.calls += 1;
        Ok(())
    }
    fn scalar(&mut self, f: AnimationScalar) -> Result<f32, ()> {
        self.calls += 1;
        Ok(match f {
            AnimationScalar::Turn => -0.0,
            AnimationScalar::Spin => 3.0,
            AnimationScalar::TimeSinceInput => 7.0,
        })
    }
    fn flag(&mut self, _: AnimationFlag) -> Result<bool, ()> {
        self.calls += 1;
        Ok(true)
    }
}
#[derive(Default)]
struct Numeric {
    args: Option<([f32; 3], [f32; 3])>,
    fail: bool,
}
impl SkeletonCollisionNumeric for Numeric {
    type Error = ();
    fn dot3(&mut self, a: [f32; 3], b: [f32; 3]) -> Result<f32, ()> {
        self.args = Some((a, b));
        if self.fail {
            Err(())
        } else {
            Ok((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
        }
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, ()> {
        panic!("no reciprocal")
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, ()> {
        panic!("no reciprocal sqrt")
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out.concrete
        .skateboard_reckoning
        .as_mut()
        .unwrap()
        .transform
        .set_column(
            3,
            SkeletonRootVector {
                xyz: [1.0, 2.0, 3.0],
                w: None,
            },
        );
    // Effective transform stays at its reset origin, distinguishing the source.
    let guard = out.sample_write_guard().unwrap();
    out.publish_system_vector(
        &guard,
        PhysOutSystemVector::CentreOfMass,
        SkeletonRootVector {
            xyz: [5.0, 7.0, 9.0],
            w: None,
        },
    )
    .unwrap();
    out.publish_system_vector(
        &guard,
        PhysOutSystemVector::Up,
        SkeletonRootVector {
            xyz: [2.0, 3.0, 4.0],
            w: None,
        },
    )
    .unwrap();
    out
}
#[test]
fn com_uses_actual_transform_position_and_system_up_ignoring_unknown_w() {
    let mut out = output();
    let mut numeric = Numeric::default();
    out.try_publish_update5_animation(&mut Source::default(), &mut numeric)
        .unwrap();
    assert_eq!(numeric.args, Some(([4.0, 5.0, 6.0], [2.0, 3.0, 4.0])));
    let anim = out.concrete.animation.as_ref().unwrap();
    assert_eq!(anim.centre_of_mass_to_deck.to_bits(), 47.0_f32.to_bits());
    assert_eq!(anim.input_turn.to_bits(), 0x8000_0000);
    assert_eq!(anim.input_kickturn.to_bits(), 3.0_f32.to_bits());
    assert_eq!(anim.time_since_last_input.to_bits(), 7.0_f32.to_bits());
    assert!(
        anim.is_carving
            && anim.is_mirrored
            && anim.is_riding_switch
            && anim.is_riding_fakie
            && anim.is_regular_stance
            && anim.is_no_input
            && anim.is_generally_pushing
            && anim.transitioning_on_off_board
            && anim.skater_off_board
            && anim.running_out
    );
    assert!(
        out.concrete.offboard.as_ref().unwrap().is_jumping
            && out
                .concrete
                .offboard
                .as_ref()
                .unwrap()
                .is_starting_standing_jump
    );
}
#[test]
fn failed_dot_preserves_three_prefix_stores_without_reading_stance() {
    let mut out = output();
    let mut source = Source::default();
    let mut numeric = Numeric {
        fail: true,
        ..Numeric::default()
    };
    assert!(matches!(
        out.try_publish_update5_animation(&mut source, &mut numeric),
        Err(Update5AnimationError::Numeric(()))
    ));
    assert_eq!(source.calls, 4);
    let anim = out.concrete.animation.as_ref().unwrap();
    assert!(anim.is_carving);
    assert_eq!(anim.input_kickturn.to_bits(), 3.0_f32.to_bits());
    assert_eq!(anim.centre_of_mass_to_deck.to_bits(), 0.0_f32.to_bits());
    assert!(!anim.is_mirrored);
}
#[test]
fn locked_initial_guard_is_sampled_before_carving_but_asserted_after_read() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut source = Source::default();
    let mut numeric = Numeric::default();
    assert!(
        out.try_publish_update5_animation(&mut source, &mut numeric)
            .is_err()
    );
    assert_eq!(source.calls, 2);
    assert!(numeric.args.is_none());
}
