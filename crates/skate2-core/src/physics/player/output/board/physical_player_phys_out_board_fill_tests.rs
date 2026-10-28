use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source flag, debug and missing-lane test conditions"
)]
struct Source {
    calls: Vec<&'static str>,
    fail: Option<&'static str>,
    effective_calls: u8,
    draw_velocity: bool,
    draw_look: bool,
    flip: bool,
    unknown_w: bool,
}
impl Source {
    fn new() -> Self {
        Self {
            calls: vec![],
            fail: None,
            effective_calls: 0,
            draw_velocity: false,
            draw_look: false,
            flip: false,
            unknown_w: false,
        }
    }
    fn call(&mut self, name: &'static str) -> Result<(), &'static str> {
        self.calls.push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(())
        }
    }
    fn transform(&self, tag: f32) -> SkeletonRootMatrix {
        let mut columns = *matrix(tag).columns();
        if self.unknown_w {
            for column in &mut columns {
                column.w = None;
            }
        }
        SkeletonRootMatrix::from_columns(columns)
    }
}
fn matrix(tag: f32) -> SkeletonRootMatrix {
    SkeletonRootMatrix::from_columns([
        SkeletonRootVector::known([tag, 2.0, 3.0, -0.0]),
        SkeletonRootVector::known([4.0; 4]),
        SkeletonRootVector::known([5.0, 6.0, tag, 8.0]),
        SkeletonRootVector::known([9.0; 4]),
    ])
}
impl BoardFillSource for Source {
    type Error = &'static str;
    fn capture_body(&mut self) -> Result<(), Self::Error> {
        self.call("body")
    }
    fn part_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.call("part")?;
        Ok(self.transform(10.0))
    }
    fn effective_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.call("effective")?;
        self.effective_calls += 1;
        Ok(self.transform(f32::from(self.effective_calls)))
    }
    fn sample_linear_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.call("linear")?;
        Ok(SkeletonRootVector {
            xyz: [0.0, 0.0, -1.0],
            w: None,
        })
    }
    fn look_direction_flipped(&mut self) -> Result<bool, Self::Error> {
        self.call("flip")?;
        Ok(self.flip)
    }
    fn draw_velocity_direction(&mut self) -> Result<bool, Self::Error> {
        self.call("draw velocity")?;
        Ok(self.draw_velocity)
    }
    fn draw_look_direction(&mut self) -> Result<bool, Self::Error> {
        self.call("draw look")?;
        Ok(self.draw_look)
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.call("overlay")
    }
}
struct Numeric {
    projection: f32,
}
impl SkeletonCollisionNumeric for Numeric {
    type Error = Infallible;
    fn dot3(&mut self, a: [f32; 3], b: [f32; 3]) -> Result<f32, Infallible> {
        assert_eq!(a.map(f32::to_bits), [5.0_f32, 6.0, 3.0].map(f32::to_bits));
        assert_eq!(b.map(f32::to_bits), [0.0_f32, 0.0, -1.0].map(f32::to_bits));
        Ok(self.projection)
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Infallible> {
        panic!("motion not entered")
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Infallible> {
        panic!("no reciprocal")
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
fn distinct_live_transforms_full_w_and_late_flip_are_consumed_in_order() {
    let mut output = output();
    let mut source = Source {
        flip: true,
        ..Source::new()
    };
    output
        .try_fill_board_transform_directions(&mut source, &mut Numeric { projection: -3.0 })
        .unwrap();
    assert_eq!(
        source.calls,
        [
            "body",
            "part",
            "effective",
            "effective",
            "linear",
            "effective",
            "effective",
            "flip",
            "draw velocity",
            "draw look"
        ]
    );
    let result = output.concrete.skateboard_reckoning.as_ref().unwrap();
    assert_eq!(result.transform, matrix(10.0));
    assert_eq!(result.effective_transform, matrix(1.0));
    assert_eq!(result.deck_nose_dir, matrix(2.0).columns()[2]);
    assert_eq!(
        result.vel_direction,
        SkeletonRootVector::known([-5.0, -6.0, -3.0, -8.0])
    );
    assert_eq!(
        result.skater_look_direction.xyz.map(f32::to_bits),
        [-4.0_f32, -2.0, -3.0].map(f32::to_bits)
    );
    assert_eq!(result.skater_look_direction.w.unwrap().to_bits(), 0);
}

#[test]
fn unordered_projection_selects_positive_sign() {
    let mut output = output();
    output
        .try_fill_board_transform_directions(
            &mut Source::new(),
            &mut Numeric {
                projection: f32::NAN,
            },
        )
        .unwrap();
    assert_eq!(
        output
            .concrete
            .skateboard_reckoning
            .as_ref()
            .unwrap()
            .vel_direction,
        matrix(3.0).columns()[2]
    );
}

#[test]
fn unknown_native_transform_w_survives_all_copies_and_direction_scales() {
    let mut output = output();
    let mut source = Source {
        unknown_w: true,
        flip: true,
        ..Source::new()
    };
    output
        .try_fill_board_transform_directions(&mut source, &mut Numeric { projection: -3.0 })
        .unwrap();
    let result = output.concrete.skateboard_reckoning.as_ref().unwrap();
    assert!(
        result
            .transform
            .columns()
            .iter()
            .all(|column| column.w.is_none())
    );
    assert!(
        result
            .effective_transform
            .columns()
            .iter()
            .all(|column| column.w.is_none())
    );
    assert_eq!(result.deck_nose_dir.w, None);
    assert_eq!(result.vel_direction.w, None);
    assert_eq!(result.skater_look_direction.w, None);
    assert_eq!(
        result.vel_direction.xyz.map(f32::to_bits),
        [-5.0_f32, -6.0, -3.0].map(f32::to_bits)
    );
}

#[test]
fn locked_output_still_captures_body_and_reads_initial_part_transform() {
    let mut output = output();
    output.lock_after_update1().unwrap();
    let mut source = Source::new();
    assert!(matches!(
        output.try_fill_board_transform_directions(&mut source, &mut Numeric { projection: 0.0 }),
        Err(BoardFillOutputError::Access(_))
    ));
    assert_eq!(source.calls, ["body", "part"]);
}

#[test]
fn missing_real_velocity_frame_retains_three_transform_publications() {
    let mut output = output();
    let mut source = Source {
        fail: Some("linear"),
        ..Source::new()
    };
    assert!(matches!(
        output.try_fill_board_transform_directions(&mut source, &mut Numeric { projection: 0.0 }),
        Err(BoardFillOutputError::Source("linear"))
    ));
    let result = output.concrete.skateboard_reckoning.as_ref().unwrap();
    assert_eq!(result.transform, matrix(10.0));
    assert_eq!(result.effective_transform, matrix(1.0));
    assert_eq!(result.deck_nose_dir, matrix(2.0).columns()[2]);
    assert_eq!(source.effective_calls, 2);
}

#[test]
fn debug_failure_occurs_after_direction_stores_and_two_fresh_transform_reads() {
    let mut output = output();
    let mut source = Source {
        fail: Some("overlay"),
        draw_velocity: true,
        draw_look: true,
        ..Source::new()
    };
    assert!(matches!(
        output.try_fill_board_transform_directions(&mut source, &mut Numeric { projection: -3.0 }),
        Err(BoardFillOutputError::Source("overlay"))
    ));
    assert_eq!(
        &source.calls[8..],
        ["draw velocity", "effective", "effective", "overlay"]
    );
    assert_eq!(
        output
            .concrete
            .skateboard_reckoning
            .as_ref()
            .unwrap()
            .vel_direction,
        SkeletonRootVector::known([-5.0, -6.0, -3.0, -8.0])
    );
}

#[test]
fn native_negative_fused_residual_preserves_intermediate_negative_zero() {
    assert_eq!(board_speed_residual(4.0, 0.25).to_bits(), 0x8000_0000);
    assert_eq!((-4.0_f32).mul_add(0.25, 1.0).to_bits(), 0);
    assert_eq!(
        board_speed_from_squared_and_estimate(4.0, 0.5).to_bits(),
        2.0_f32.to_bits()
    );
    assert_eq!(
        board_speed_from_squared_and_estimate(-0.0, f32::INFINITY).to_bits(),
        0
    );
}
