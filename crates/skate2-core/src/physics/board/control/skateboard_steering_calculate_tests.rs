use super::*;
use std::cell::RefCell;
struct Input {
    fail_last: bool,
    turn: f32,
    balance: f32,
    speed: f32,
    pushing: bool,
}
impl SteeringCalculationInput for Input {
    type Error = &'static str;
    fn scalar(&self, field: SteeringScalar) -> Result<f32, Self::Error> {
        use SteeringScalar as S;
        Ok(match field {
            S::Turn => self.turn,
            S::HardTurn => 0.0,
            S::AbsoluteSpeed => self.speed,
            S::TruckTightness => 0.6,
            S::Balance => self.balance,
            S::FlippedControls => {
                if self.fail_last {
                    return Err("late flipped");
                }
                -1.0
            }
        })
    }
    fn pushing(&self) -> Result<bool, Self::Error> {
        Ok(self.pushing)
    }
}
#[derive(Default)]
struct Attributes {
    keys: Vec<u64>,
    graphs: RefCell<Vec<(usize, u32)>>,
    manual_calls: usize,
}
impl SteeringCalculationAttributes for Attributes {
    type Error = ();
    fn steering_scalar(&mut self, key: u64) -> Result<f32, ()> {
        self.keys.push(key);
        Ok(match key {
            0xdbd1_d2af_088b_a302 => 2.0,
            0xc40b_e048_fab4_f0b3 => 0.25,
            0x6481_a86a_7e6d_5efc => 4.0,
            0xdb8f_95ac_3f0d_e414 => 0.2,
            0xe6b6_a5e8_74f4_5937 => 0.3,
            0xbe1a_0232_c2b2_358b => 0.1,
            0x3658_38a8_b87e_eda6 => 0.5,
            0xf0fc_892e_a704_30cb => 0.8,
            _ => panic!("unexpected source key"),
        })
    }
    fn manual_scalar(&mut self, key: u64) -> Result<f32, ()> {
        assert_eq!(key, 0x4c9e_095b_dc2f_1de5);
        self.manual_calls += 1;
        Ok(0.75)
    }
    fn graph(&mut self, offset: usize, input: f32) -> Result<f32, ()> {
        self.graphs.borrow_mut().push((offset, input.to_bits()));
        Ok(input)
    }
}
fn input() -> Input {
    Input {
        fail_last: false,
        turn: 0.4,
        balance: 0.0,
        speed: 2.0,
        pushing: true,
    }
}
#[test]
fn late_read_failure_retains_both_real_pointer_stores() {
    let mut input = input();
    input.fail_last = true;
    let mut attributes = Attributes::default();
    let mut push = 0.6;
    let mut damped = 0.2;
    assert!(matches!(
        try_calculate(&input, &mut attributes, Some(&mut push), Some(&mut damped)),
        Err(SteeringCalculationError::Input("late flipped"))
    ));
    assert_eq!(damped.to_bits(), 0.25_f32.to_bits());
    assert_eq!(push.to_bits(), 0.3_f32.to_bits());
    assert_eq!(attributes.manual_calls, 0);
    assert_eq!(attributes.keys.last(), Some(&0xf0fc_892e_a704_30cb));
}
#[test]
fn null_pointers_skip_only_their_native_attribute_branches() {
    let mut attributes = Attributes::default();
    try_calculate(&input(), &mut attributes, None, None).unwrap();
    assert_eq!(
        attributes.keys,
        [
            0xdbd1_d2af_088b_a302,
            0x6481_a86a_7e6d_5efc,
            0x3658_38a8_b87e_eda6,
            0xf0fc_892e_a704_30cb
        ]
    );
    assert_eq!(
        *attributes.graphs.borrow(),
        [(0, 0.5_f32.to_bits()), (0x40, 0.4_f32.to_bits())]
    );
}
#[test]
fn unordered_speed_selects_one_push_stays_nan_and_manual_gate_is_nonzero() {
    let mut input = input();
    input.speed = f32::NAN;
    input.balance = f32::NAN;
    let mut attributes = Attributes::default();
    let mut push = f32::NAN;
    let result = try_calculate(&input, &mut attributes, Some(&mut push), None).unwrap();
    assert!(result.is_nan() && push.is_nan());
    assert_eq!(attributes.manual_calls, 1);
    assert_eq!(attributes.graphs.borrow()[0], (0, 1.0_f32.to_bits()));
}
#[test]
fn absolute_turn_preserves_negative_zero_for_the_graph() {
    let mut input = input();
    input.turn = -0.0;
    let mut attributes = Attributes::default();
    try_calculate(&input, &mut attributes, None, None).unwrap();
    assert_eq!(attributes.graphs.borrow()[1], (0x40, 0x8000_0000));
}
