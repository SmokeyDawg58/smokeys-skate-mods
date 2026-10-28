use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;

struct Source {
    calls: Vec<(&'static str, usize)>,
    contact: [bool; 7],
    fail: Option<(&'static str, usize)>,
}
impl Source {
    fn record(&mut self, name: &'static str, part: usize) -> Result<(), &'static str> {
        self.calls.push((name, part));
        if self.fail == Some((name, part)) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl BoardFillContactSource for Source {
    type Error = &'static str;
    type Contacts = ();
    fn capture_contacts(&mut self) -> Result<(), Self::Error> {
        self.record("capture", 0)
    }
    fn in_contact(&mut self, (): &(), part: usize) -> Result<bool, Self::Error> {
        self.record("contact", part)?;
        Ok(self.contact[part])
    }
    fn wheel_velocity(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        self.record("velocity", part)?;
        Ok(SkeletonRootVector {
            xyz: [1.0, 2.0, 3.0],
            w: None,
        })
    }
    fn position(&mut self, (): &(), part: usize) -> Result<SkeletonRootVector, Self::Error> {
        self.record("position", part)?;
        assert!(self.contact[part]);
        Ok(SkeletonRootVector::known([5.0, 6.0, 7.0, -0.0]))
    }
    fn normal(&mut self, (): &(), part: usize) -> Result<SkeletonRootVector, Self::Error> {
        self.record("normal", part)?;
        assert!(self.contact[part]);
        Ok(SkeletonRootVector::known([8.0, 9.0, 10.0, 11.0]))
    }
    fn audio(&mut self, (): &(), part: usize) -> Result<u32, Self::Error> {
        self.record("audio", part)?;
        Ok(u32::MAX)
    }
    fn pattern(&mut self, (): &(), part: usize) -> Result<u32, Self::Error> {
        self.record("pattern", part)?;
        Ok(0x8000_0000)
    }
    fn overall_surface(&mut self, (): &()) -> Result<u32, Self::Error> {
        self.record("surface", 0)?;
        Ok(7)
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    output
}
#[test]
fn ascending_false_contacts_skip_vectors_and_keep_raw_integer_bits() {
    let mut output = output();
    let mut source = Source {
        calls: vec![],
        contact: [false; 7],
        fail: None,
    };
    output.try_fill_board_contacts(&mut source).unwrap();
    assert_eq!(
        &source.calls[..9],
        [
            ("capture", 0),
            ("contact", 0),
            ("velocity", 0),
            ("audio", 0),
            ("pattern", 0),
            ("contact", 1),
            ("velocity", 1),
            ("audio", 1),
            ("pattern", 1)
        ]
    );
    assert!(
        !source
            .calls
            .iter()
            .any(|(name, _)| *name == "position" || *name == "normal")
    );
    let collision = output.concrete.collision.as_ref().unwrap();
    assert_eq!(collision.wheel_audio_surface, [-1; 4]);
    assert_eq!(collision.wheel_surface_pattern, [i32::MIN; 4]);
    assert_eq!(
        collision.wheel_contact_normal,
        [SkeletonRootVector::known([0.0; 4]); 4]
    );
    assert_eq!(output.collision_physics_type().unwrap().value, 7);
    assert!(
        output
            .concrete
            .motion
            .as_ref()
            .unwrap()
            .wheel_linear_velocity
            .iter()
            .all(|v| v.w.is_none())
    );
}
#[test]
fn late_wheel_audio_failure_preserves_position_normal_and_prior_wheel_only() {
    let mut output = output();
    let mut source = Source {
        calls: vec![],
        contact: [true; 7],
        fail: Some(("audio", 1)),
    };
    assert!(matches!(
        output.try_fill_board_contacts(&mut source),
        Err(BoardFillContactsError::Source("audio"))
    ));
    let collision = output.concrete.collision.as_ref().unwrap();
    assert_eq!(collision.wheel_audio_surface, [-1, 0, 0, 0]);
    assert_eq!(collision.wheel_surface_pattern, [i32::MIN, 0, 0, 0]);
    assert!(collision.wheel_in_contact[1]);
    assert_eq!(
        collision.wheel_contact_position[1].w.unwrap().to_bits(),
        0x8000_0000
    );
    assert_eq!(
        collision.wheel_contact_normal[1],
        SkeletonRootVector::known([8.0, 9.0, 10.0, 11.0])
    );
    assert!(!source.calls.iter().any(|(_, part)| *part == 2));
}
#[test]
fn locked_output_reads_contact_then_live_velocity_before_assertion() {
    let mut output = output();
    output.lock_after_update1().unwrap();
    let mut source = Source {
        calls: vec![],
        contact: [true; 7],
        fail: None,
    };
    assert!(matches!(
        output.try_fill_board_contacts(&mut source),
        Err(BoardFillContactsError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    ));
    assert_eq!(
        source.calls,
        [("capture", 0), ("contact", 0), ("velocity", 0)]
    );
    assert_eq!(
        output.concrete.collision.as_ref().unwrap().wheel_in_contact,
        [false; 4]
    );
}
