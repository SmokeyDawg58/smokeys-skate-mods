use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;
struct Source {
    calls: Vec<&'static str>,
    last_reads: usize,
    deck_reads: usize,
    deck: [bool; 2],
    fail_acceleration: bool,
}
impl Source {
    fn new() -> Self {
        Self {
            calls: vec![],
            last_reads: 0,
            deck_reads: 0,
            deck: [true, false],
            fail_acceleration: false,
        }
    }
}
fn vector(xyz: [f32; 3]) -> SkeletonRootVector {
    SkeletonRootVector { xyz, w: None }
}
impl BoardFillGroundSource for Source {
    type Error = &'static str;
    type Body = ();
    fn capture_body(&mut self) -> Result<(), Self::Error> {
        self.calls.push("body");
        Ok(())
    }
    fn time_since_wheels(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.calls.push("time");
        Ok(0.25)
    }
    fn dynamic_up(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("dynamic");
        Ok(vector([1.0, 0.0, 0.0]))
    }
    fn last_valid_normal(&mut self, (): &()) -> Result<SkeletonRootVector, Self::Error> {
        self.last_reads += 1;
        Ok(if self.last_reads == 1 {
            vector([0.0, 1.0, 0.0])
        } else {
            vector([0.0, 0.0, 1.0])
        })
    }
    fn contact_acceleration(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(vector([2.0, 3.0, 4.0]))
    }
    fn average_normal(&mut self, (): &()) -> Result<SkeletonRootVector, Self::Error> {
        Ok(vector([0.0, 1.0, 0.0]))
    }
    fn num_wheels(&mut self, (): &()) -> Result<u8, Self::Error> {
        Ok(0)
    }
    fn in_contact(&mut self, (): &(), part: usize) -> Result<bool, Self::Error> {
        match part {
            5 => {
                self.calls.push("back");
                Ok(true)
            }
            4 => panic!("back true must short-circuit fresh front capture/read"),
            6 => {
                let value = self.deck[self.deck_reads];
                self.deck_reads += 1;
                Ok(value)
            }
            _ => panic!("unexpected part"),
        }
    }
    fn deck_normal(&mut self, (): &()) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("deck normal");
        Ok(vector([0.0, 1.0, 0.0]))
    }
    fn deck_acceleration(&mut self, (): &()) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("deck acceleration");
        if self.fail_acceleration {
            Err("deck acceleration")
        } else {
            Ok(vector([0.0, 400.0, 0.0]))
        }
    }
}
#[derive(Default)]
struct Numeric {
    dots: usize,
}
impl SkeletonCollisionNumeric for Numeric {
    type Error = ();
    fn dot3(&mut self, a: [f32; 3], b: [f32; 3]) -> Result<f32, ()> {
        self.dots += 1;
        Ok((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, ()> {
        panic!()
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, ()> {
        panic!()
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out
}
#[test]
fn motion_contact_predicate_uses_truck_and_deck_not_wheel_or_skeleton_contact() {
    let mut out = output();
    for (truck, deck) in [(false, false), (true, false), (false, true), (true, true)] {
        let collision = out.concrete.collision.as_mut().unwrap();
        collision.a_truck_in_contact = truck;
        collision.deck_in_contact = deck;
        collision.anything_in_contact = true;
        collision.skeleton_in_contact = true;
        collision.wheel_in_contact = [true; 4];
        assert_eq!(out.try_motion_trucks_or_deck_contact().unwrap(), truck || deck);
    }
}
#[test]
fn fresh_normals_and_deck_validity_have_distinct_consumers() {
    let mut out = output();
    let mut source = Source::new();
    let mut numeric = Numeric::default();
    out.try_fill_board_ground(&mut source, &mut numeric)
        .unwrap();
    assert_eq!(source.last_reads, 2);
    assert_eq!(numeric.dots, 1);
    assert_eq!(
        out.concrete.ground.as_ref().unwrap().ground_force.to_bits(),
        3.0_f32.to_bits()
    );
    let collision = out.concrete.collision.as_ref().unwrap();
    assert!(
        collision.deck_in_contact && collision.a_truck_in_contact && collision.anything_in_contact
    );
    assert_eq!(collision.deck_contact_force.to_bits(), 0);
    assert!(!source.calls.contains(&"deck normal"));
    assert_eq!(
        out.ground_vector(PhysOutGroundVector::LastValidGroundUp)
            .unwrap()
            .value
            .xyz
            .map(f32::to_bits),
        [0.0_f32, 0.0, 1.0].map(f32::to_bits)
    );
}
#[test]
fn late_force_input_failure_keeps_preceding_contact_publications() {
    let mut out = output();
    let mut source = Source::new();
    source.deck = [false, true];
    source.fail_acceleration = true;
    assert!(matches!(
        out.try_fill_board_ground(&mut source, &mut Numeric::default()),
        Err(BoardGroundOutputError::Source("deck acceleration"))
    ));
    let collision = out.concrete.collision.as_ref().unwrap();
    assert!(!collision.deck_in_contact);
    assert!(collision.anything_in_contact);
    assert_eq!(collision.deck_contact_force.to_bits(), 0);
}
#[test]
fn initial_lock_still_captures_body_and_time_before_refusal() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut source = Source::new();
    assert!(matches!(
        out.try_fill_board_ground(&mut source, &mut Numeric::default()),
        Err(BoardGroundOutputError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    ));
    assert_eq!(source.calls, ["body", "time"]);
}
#[test]
fn deck_force_raw_fsel_keeps_nan_and_signed_zero_behavior() {
    for input in [0.0, -0.0] {
        assert_eq!(deck_force_from_dot(input).to_bits(), 0);
    }
    assert_eq!(
        deck_force_from_dot(f32::from_bits(0x7fc0_0042)).to_bits(),
        1.0_f32.to_bits()
    );
    assert_eq!(
        deck_force_from_dot(f32::NEG_INFINITY).to_bits(),
        1.0_f32.to_bits()
    );
    assert_eq!(deck_force_from_dot(-400.0).to_bits(), 0.5_f32.to_bits());
}
