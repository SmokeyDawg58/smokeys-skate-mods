//! Unregistered child of `phys_out::com`: discriminate early capture from reread.
use super::{PhysOutComSource, PhysicalPlayerPhysOut, SkeletonRootVector};
use crate::physical_player_phys_out::{PhysOutSystemVector, construct::PhysOutAllocation};

struct Source {
    velocity: SkeletonRootVector,
    reads: Vec<&'static str>,
}
impl PhysOutComSource for Source {
    type Error = ();
    fn centre_of_mass_velocity(&mut self) -> Result<SkeletonRootVector, ()> {
        self.reads.push("velocity");
        Ok(self.velocity)
    }
    fn centre_of_mass(&mut self) -> Result<SkeletonRootVector, ()> {
        self.reads.push("centre");
        // Source collaborator changes its current value after native585b0.
        self.velocity = SkeletonRootVector::known([99.0; 4]);
        Ok(SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0]))
    }
}
#[test]
fn capture_return_and_published_velocity_are_same_early_value_with_unknown_w() {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    let early = SkeletonRootVector {
        xyz: [7.0, -0.0, 11.0],
        w: None,
    };
    let mut source = Source {
        velocity: early,
        reads: Vec::new(),
    };
    let captured = output
        .try_publish_skeleton_com_capture(&mut source)
        .unwrap();
    assert_eq!(source.reads, ["velocity", "centre"]);
    assert_eq!(captured.xyz.map(f32::to_bits), early.xyz.map(f32::to_bits));
    assert_eq!(captured.w, None);
    let published = output
        .system_vector(PhysOutSystemVector::CentreOfMassVelocity)
        .unwrap()
        .value;
    assert_eq!(published.xyz.map(f32::to_bits), early.xyz.map(f32::to_bits));
    assert_eq!(published.w, None);
    assert_eq!(
        source.velocity.xyz.map(f32::to_bits),
        [99.0f32.to_bits(); 3]
    );
}
