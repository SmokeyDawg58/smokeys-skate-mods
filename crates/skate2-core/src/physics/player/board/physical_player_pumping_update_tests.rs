use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;

#[derive(Default)]
struct Attributes {
    reads: Vec<u64>,
    fail: Option<u64>,
}
impl PumpingAttributeRead for Attributes {
    type Pointer = u64;
    type Error = &'static str;
    fn lookup(&mut self, key: u64) -> Result<u64, Self::Error> {
        self.reads.push(key);
        if self.fail == Some(key) {
            Err("selected attribute failure")
        } else {
            Ok(key)
        }
    }
    fn scalar(&mut self, key: &u64, _: u64) -> Result<f32, Self::Error> {
        Ok(match *key {
            0xa898_f4cd_23f5_d9b0 => 0.0,
            0x4bae_22d0_e9c5_851c | 0x1ad7_b592_6a86_dc10 | 0x4747_cb3d_449b_8780 => 10.0,
            0xca49_d85f_a629_f7e7 | 0xced7_9378_3567_47bd => 4.0,
            _ => 1.0,
        })
    }
    fn keyed_graph(&mut self, key: u64, _: f32) -> Result<f32, Self::Error> {
        self.lookup(key)?;
        Ok(1.0)
    }
    fn layout_graph(&mut self, _: usize, _: f32) -> Result<f32, Self::Error> {
        Ok(1.0)
    }
}
#[derive(Default)]
struct Debug {
    deck: usize,
    pump: usize,
    fail_pump: bool,
}
impl PumpingDebug for Debug {
    type Error = &'static str;
    fn draw_deck_angle(&mut self, _: f32, _: f32, _: f32, _: f32) -> Result<(), Self::Error> {
        self.deck += 1;
        Ok(())
    }
    fn draw_pump(&mut self) -> Result<(), Self::Error> {
        self.pump += 1;
        if self.fail_pump {
            Err("late debug failure")
        } else {
            Ok(())
        }
    }
}
fn vector(xyz: [f32; 3]) -> SkeletonRootVector {
    SkeletonRootVector { xyz, w: None }
}

#[test]
fn first_record_is_retained_before_debug_failure_without_attribute_reads() {
    let mut owner = PhysicalPlayerPumpingOwner::new();
    owner.try_construct(true).unwrap();
    let mut attributes = Attributes::default();
    let mut debug = Debug {
        fail_pump: true,
        ..Debug::default()
    };
    let position = vector([3.0, 2.0, 1.0]);
    let normal = vector([0.0, 1.0, 0.0]);
    let result = owner.try_update(
        position,
        normal,
        vector([0.0, 2.0, 0.0]),
        0.0,
        0.5,
        &mut attributes,
        &mut HostReckoningNumeric,
        &mut debug,
    );
    assert!(matches!(
        result,
        Err(PumpingUpdateError::Debug("late debug failure"))
    ));
    assert!(attributes.reads.is_empty());
    let state = owner.value.as_ref().unwrap();
    assert!(state.record_valid);
    assert_eq!(
        state.last_position.xyz.map(f32::to_bits),
        position.xyz.map(f32::to_bits)
    );
    assert!(state.last_position.w.is_none());
    assert!(state.last_normal.w.is_none());
    assert_eq!(state.last_height.to_bits(), 2.0f32.to_bits());
    assert_eq!(debug.deck, 0);
    assert_eq!(debug.pump, 1);
}

#[test]
fn repeated_curvature_uses_same_history_and_caps_acceleration_after_calculate() {
    let mut owner = PhysicalPlayerPumpingOwner::new();
    owner.try_construct(true).unwrap();
    let mut attributes = Attributes::default();
    let mut debug = Debug::default();
    owner
        .try_update(
            vector([0.0; 3]),
            vector([0.0, 1.0, 0.0]),
            vector([0.0; 3]),
            0.0,
            0.5,
            &mut attributes,
            &mut HostReckoningNumeric,
            &mut debug,
        )
        .unwrap();
    owner
        .try_update(
            vector([0.0, 1.0, 0.0]),
            vector([1.0, 0.0, 0.0]),
            vector([1.0, 0.0, 0.0]),
            0.0,
            0.5,
            &mut attributes,
            &mut HostReckoningNumeric,
            &mut debug,
        )
        .unwrap();
    let state = owner.value.as_ref().unwrap();
    assert_eq!(state.angular_speed.to_bits(), 2.0f32.to_bits());
    assert_eq!(state.pump_amount.to_bits(), 2.0f32.to_bits());
    assert_eq!(state.pump_time.to_bits(), 0.5f32.to_bits());
    assert_eq!(state.acceleration.to_bits(), 2.0f32.to_bits());
    assert!(attributes.reads.contains(&0x1ad7_b592_6a86_dc10));
    assert!(!attributes.reads.contains(&0x4747_cb3d_449b_8780));
    assert_eq!(debug.deck, 1);
    assert_eq!(debug.pump, 2);
}

#[test]
fn late_cap_failure_preserves_calculation_and_old_record_for_retry() {
    let mut owner = PhysicalPlayerPumpingOwner::new();
    owner.try_construct(true).unwrap();
    let state = owner.value.as_mut().unwrap();
    state.record_valid = true;
    state.last_normal = vector([0.0, 1.0, 0.0]);
    let old_position = state.last_position;
    let mut attributes = Attributes {
        fail: Some(0xca49_d85f_a629_f7e7),
        ..Attributes::default()
    };
    let mut debug = Debug::default();
    let result = owner.try_update(
        vector([0.0, 1.0, 0.0]),
        vector([1.0, 0.0, 0.0]),
        vector([1.0, 0.0, 0.0]),
        0.0,
        0.5,
        &mut attributes,
        &mut HostReckoningNumeric,
        &mut debug,
    );
    assert!(matches!(
        result,
        Err(PumpingUpdateError::Attribute("selected attribute failure"))
    ));
    let state = owner.value.as_ref().unwrap();
    assert_eq!(state.acceleration.to_bits(), 20.0f32.to_bits());
    assert_eq!(
        state.last_position.known_lanes().unwrap().map(f32::to_bits),
        old_position.known_lanes().unwrap().map(f32::to_bits)
    );
    assert_eq!(state.last_height.to_bits(), 0);
    assert_eq!(debug.pump, 0);
}
