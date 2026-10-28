//! Production-helper checks for the recovered postphysics write and deck graphs.
#![expect(
    clippy::float_cmp,
    reason = "exact source stores and representable geometry values"
)]
use super::*;
use crate::{
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skateboard_attributes::{SkateboardAttributeClass, SkateboardAttributeRead},
};

#[derive(Default)]
struct Source {
    events: Vec<&'static str>,
    part: Option<Matrix>,
    record: Option<Matrix>,
    fail_record: bool,
    draw: bool,
}
impl FootIkSource for Source {
    type Error = &'static str;
    fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
        panic!("helper does not read flags")
    }
    fn state_timer(&mut self) -> Result<f32, Self::Error> {
        panic!("helper does not read timer")
    }
    fn minimum_deck_up_y(&mut self) -> Result<f32, Self::Error> {
        panic!("helper does not read angle threshold")
    }
    fn deck_bounds_margin(&mut self) -> Result<Vector, Self::Error> {
        self.events.push("margin");
        Ok(ZERO)
    }
    fn feet_relative_height(&mut self, wipeout: bool) -> Result<f32, Self::Error> {
        self.events
            .push(if wipeout { "bail height" } else { "height" });
        Ok(if wipeout { 0.75 } else { 0.25 })
    }
    fn record(&mut self, _: Part) -> Result<Matrix, Self::Error> {
        panic!("helper does not read record")
    }
    fn set_part(&mut self, _: Part, matrix: &Matrix) -> Result<(), Self::Error> {
        self.events.push("part");
        self.part = Some(*matrix);
        Ok(())
    }
    fn set_record(&mut self, _: Part, matrix: &Matrix) -> Result<(), Self::Error> {
        self.events.push("record");
        if self.fail_record {
            return Err("record refused");
        }
        self.record = Some(*matrix);
        Ok(())
    }
    fn draw_post_feet_ik(&mut self) -> Result<bool, Self::Error> {
        self.events.push("draw feet");
        Ok(self.draw)
    }
    fn draw_post_phys_ik(&mut self) -> Result<bool, Self::Error> {
        panic!("helper does not read caller debug control")
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.events.push("overlay");
        Ok(())
    }
}

struct Deck(f32);
impl SkateboardAttributeRead for Deck {
    type Pointer = u64;
    type Error = &'static str;
    fn lookup(&mut self, class: SkateboardAttributeClass, key: u64) -> Result<u64, Self::Error> {
        assert_eq!(class, SkateboardAttributeClass::Deck);
        Ok(key)
    }
    fn float(&mut self, key: &u64) -> Result<f32, Self::Error> {
        Ok(match *key {
            0xd76d_9db5_ee1f_d008 => 2.0,
            0xc94d_6509_1dcb_828b => self.0,
            0x1d53_bc53_9321_90e7 => 1.0,
            0x8020_3de9_f9b7_32ca => 45.0,
            _ => panic!("unexpected geometry key"),
        })
    }
    fn byte(&mut self, _: &u64) -> Result<u8, Self::Error> {
        panic!("no byte geometry")
    }
    fn integer(&mut self, _: &u64) -> Result<i32, Self::Error> {
        panic!("no integer geometry")
    }
}
fn geometry(length: f32) -> SkeletonIkGeometry {
    let mut geometry = SkeletonIkGeometry::new();
    geometry
        .try_construct(
            &mut Deck(length),
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    geometry
}

#[test]
fn failed_record_commit_retains_the_preceding_part_write() {
    let matrix = with_position(
        Matrix::known(IDENTITY),
        Vector {
            xyz: [2.0, 3.0, 4.0],
            w: None,
        },
    );
    let mut source = Source {
        fail_record: true,
        ..Source::default()
    };
    let result = commit::<_, (), ()>(&mut source, Part::LeftFoot, &matrix);
    assert!(matches!(result, Err(FootIkError::Source("record refused"))));
    assert_eq!(source.events, ["part", "record"]);
    assert_eq!(source.part.unwrap().columns()[3].xyz, [2.0, 3.0, 4.0]);
    assert_eq!(source.part.unwrap().columns()[3].w, None);
    assert!(source.record.is_none());
}

#[test]
fn independent_x_and_z_bounds_reject_before_height_and_preserve_output() {
    let geometry = geometry(2.0);
    // Width=1, total half-length=2. Each case rejects on a DIFFERENT axis.
    for xyz in [[1.0, -1.0, 0.0], [0.0, -1.0, 2.0]] {
        let bone = Vector::known([xyz[0], xyz[1], xyz[2], 0.0]);
        let mut output = Vector::known([7.0, 8.0, 9.0, 10.0]);
        let mut source = Source {
            draw: true,
            ..Source::default()
        };
        assert!(
            !intersect_deck::<_, (), ()>(
                &geometry,
                &Matrix::known(IDENTITY),
                bone,
                bone,
                ZERO,
                false,
                &mut output,
                &mut source,
            )
            .unwrap()
        );
        assert_eq!(output.xyz, [7.0, 8.0, 9.0]);
        assert_eq!(output.w, Some(10.0));
        assert_eq!(source.events, ["margin", "draw feet", "overlay"]);
    }
}

#[test]
fn selected_height_is_live_and_preserves_target_xz_in_world_space() {
    let geometry = geometry(2.0);
    let deck = with_position(
        Matrix::known(IDENTITY),
        Vector::known([10.0, 20.0, 30.0, 0.0]),
    );
    let bone = Vector::known([10.0, 19.0, 30.0, 0.0]);
    for (wipeout, height, event) in [(false, 0.25, "height"), (true, 0.75, "bail height")] {
        let mut source = Source::default();
        let mut output = ZERO;
        assert!(
            intersect_deck::<_, (), ()>(
                &geometry,
                &deck,
                bone,
                bone,
                Vector::known([0.5, 99.0, -0.5, 0.0]),
                wipeout,
                &mut output,
                &mut source,
            )
            .unwrap()
        );
        assert_eq!(output.xyz, [10.5, 20.0 + height, 29.5]);
        assert_eq!(source.events, ["margin", event, "draw feet"]);
    }
}

#[test]
fn end_slope_uses_z_extent_and_unordered_geometry_rejects_at_bounds() {
    let normal = geometry(2.0);
    let mut output = ZERO;
    let mut source = Source::default();
    let bone = Vector::known([0.0, -1.0, 1.5, 0.0]);
    assert!(
        intersect_deck::<_, (), ()>(
            &normal,
            &Matrix::known(IDENTITY),
            bone,
            bone,
            ZERO,
            false,
            &mut output,
            &mut source,
        )
        .unwrap()
    );
    // A 45-degree deck end raises the ordinary 0.25 height by 0.5.
    assert!((output.xyz[1] - 0.75).abs() < 0.000_01);

    let unordered = geometry(f32::NAN);
    source.events.clear();
    output = ZERO;
    // NaN authored length also produces NaN total bounds: BO4/BI24 rejects.
    assert!(
        !intersect_deck::<_, (), ()>(
            &unordered,
            &Matrix::known(IDENTITY),
            bone,
            bone,
            ZERO,
            false,
            &mut output,
            &mut source,
        )
        .unwrap()
    );
    assert_eq!(output.xyz, ZERO.xyz);
    assert_eq!(source.events, ["margin", "draw feet"]);
}
