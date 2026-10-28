use super::super::twist::tests::{Numeric, Rounding, close, pose};
use super::*;

struct Attributes {
    reads: Vec<TwistRatio>,
    fail: Option<TwistRatio>,
}
impl TwistAttributeSource for Attributes {
    type Error = &'static str;
    fn read_ratio(&mut self, ratio: TwistRatio) -> Result<f32, Self::Error> {
        self.reads.push(ratio);
        if self.fail == Some(ratio) {
            return Err("attribute");
        }
        Ok(match ratio {
            TwistRatio::Arm => 1.0,
            TwistRatio::Forearm => 0.5,
            TwistRatio::ForearmSecond => -0.5,
            TwistRatio::UpperLeg => 0.0,
        })
    }
}
#[test]
fn actual_fourteen_index_mapping_drives_eight_ordered_destinations() {
    let indices = [3, 0, 6, 1, 2, 9, 4, 11, 5, 12, 7, 8, 13, 10];
    let bind = [pose([0.0, 0.0, 0.0, 1.0]); 14];
    let mut replay = bind;
    for index in [3, 6, 9] {
        replay[index].rotation = [0.5, 0.5, 0.5, 0.5];
    }
    for index in [11, 12, 13] {
        replay[index].rotation = [-0.5, -0.5, 0.5, 0.5];
    }
    let mut attributes = Attributes {
        reads: vec![],
        fail: None,
    };
    let mut numeric = Numeric::new();
    let mut rounding = Rounding(0);
    try_compute_expressions(
        &bind,
        &mut replay,
        &indices,
        &mut attributes,
        &mut numeric,
        &mut rounding,
    )
    .unwrap();
    assert_eq!(
        attributes.reads,
        [
            TwistRatio::Arm,
            TwistRatio::Forearm,
            TwistRatio::ForearmSecond,
            TwistRatio::UpperLeg
        ]
    );
    for (index, sine, cosine) in [
        (0, 0.707_106_77, 0.707_106_77),
        (1, 0.382_683_43, 0.923_879_5),
        (2, -0.382_683_43, 0.923_879_5),
        (4, 0.0, 1.0),
        (5, -0.707_106_77, 0.707_106_77),
        (7, -0.382_683_43, 0.923_879_5),
        (8, 0.382_683_43, 0.923_879_5),
        (10, 0.0, 1.0),
    ] {
        close(replay[index].rotation, [sine, 0.0, 0.0, cosine]);
    }
    assert_eq!(numeric.norms, 8);
    assert_eq!(numeric.dots, 24);
    assert_eq!(rounding.0, 32);
}
#[test]
fn all_ratio_reads_precede_first_pose_correction() {
    let bind = [pose([0.0, 0.0, 0.0, 1.0]); 14];
    let mut replay = bind;
    let mut attributes = Attributes {
        reads: vec![],
        fail: Some(TwistRatio::ForearmSecond),
    };
    let mut numeric = Numeric::new();
    assert!(matches!(
        try_compute_expressions(
            &bind,
            &mut replay,
            &std::array::from_fn(|i| i),
            &mut attributes,
            &mut numeric,
            &mut Rounding(0)
        ),
        Err(ExpressionsError::Attribute("attribute"))
    ));
    assert_eq!(numeric.norms, 0);
    assert_eq!(numeric.dots, 0);
    assert_eq!(
        attributes.reads,
        [
            TwistRatio::Arm,
            TwistRatio::Forearm,
            TwistRatio::ForearmSecond
        ]
    );
}
#[test]
fn absent_global_is_not_attribute_default_data() {
    let mut source: Option<&PresentationTwistAttributes> = None;
    assert!(matches!(
        source.read_ratio(TwistRatio::Arm),
        Err(TwistAttributeError::MissingGlobal)
    ));
}

#[test]
fn missing_later_native_index_keeps_all_ratio_reads_and_first_twist() {
    let bind = [pose([0., 0., 0., 1.]); 14];
    let mut replay = bind;
    replay[0].rotation = [0.5; 4];
    let mut indices = std::array::from_fn(|i| i);
    indices[2] = usize::try_from(u32::MAX).unwrap();
    let mut attributes = Attributes {
        reads: vec![],
        fail: None,
    };
    let result = try_compute_expressions(
        &bind,
        &mut replay,
        &indices,
        &mut attributes,
        &mut Numeric::new(),
        &mut Rounding(0),
    );
    assert!(matches!(
        result,
        Err(ExpressionsError::Twist(TwistError::BindBone(_)))
    ));
    assert_eq!(attributes.reads.len(), 4);
    close(replay[1].rotation, [0.707_106_77, 0., 0., 0.707_106_77]);
}
