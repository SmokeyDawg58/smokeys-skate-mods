use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out
}
struct Hand {
    reads: usize,
}
impl HandPlantFillSource for Hand {
    type Error = ();
    fn position(&mut self) -> Result<SkeletonRootVector, ()> {
        Ok(SkeletonRootVector::known([1.0, 2.0, 3.0, -0.0]))
    }
    fn apex_time(&mut self) -> Result<f32, ()> {
        Ok(-0.0)
    }
    fn animation_flags(&mut self) -> Result<u32, ()> {
        let flags = [0x8000_0000, 0, 0x2000_0000, 0, 0][self.reads];
        self.reads += 1;
        Ok(flags)
    }
    fn ik_flags(&mut self) -> Result<u32, ()> {
        Ok(0x8000_0000)
    }
    fn surface(&mut self) -> Result<u32, ()> {
        Ok(55)
    }
}
#[test]
fn hand_rereads_flags_preserves_low_bits_and_w() {
    let mut out = output();
    out.concrete
        .air
        .as_mut()
        .unwrap()
        .anim_hand_plant_info_struct
        .flags = 0x5abc_def0;
    let mut source = Hand { reads: 0 };
    out.try_fill_handplant(&mut source).unwrap();
    let air = out.concrete.air.as_ref().unwrap();
    assert_eq!(air.anim_hand_plant_info_struct.flags, 0xaabc_def0);
    assert_eq!(
        air.anim_hand_plant_info_struct
            .position
            .w
            .unwrap()
            .to_bits(),
        0x8000_0000
    );
    assert_eq!(source.reads, 5);
    assert!(!air.found_hand_plant_pos);
    assert!(air.hand_plant_grabbing);
}
struct Foot {
    valid: bool,
    surface: Option<u32>,
    planting: bool,
    part: i32,
    reads: usize,
}
impl FootPlantFillSource for Foot {
    type Error = &'static str;
    fn results_valid(&mut self) -> Result<bool, Self::Error> {
        Ok(self.valid)
    }
    fn time_to_land(&mut self) -> Result<f32, Self::Error> {
        self.reads += 1;
        Ok(2.0)
    }
    fn total_time(&mut self) -> Result<f32, Self::Error> {
        Ok(3.0)
    }
    fn landing(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn planting(&mut self) -> Result<bool, Self::Error> {
        Ok(self.planting)
    }
    fn foot_part(&mut self) -> Result<i32, Self::Error> {
        Ok(self.part)
    }
    fn surface(&mut self) -> Result<u32, Self::Error> {
        self.surface.ok_or("surface")
    }
}
#[test]
fn invalid_results_return_even_when_output_locked() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut source = Foot {
        valid: false,
        surface: None,
        planting: false,
        part: -1,
        reads: 0,
    };
    out.try_fill_footplant(&mut source).unwrap();
    assert_eq!(source.reads, 0);
}
#[test]
fn missing_surface_retains_prior_active_stores() {
    let mut out = output();
    let mut source = Foot {
        valid: true,
        surface: None,
        planting: true,
        part: 15,
        reads: 0,
    };
    assert!(matches!(
        out.try_fill_footplant(&mut source),
        Err(PlantFillError::Source("surface"))
    ));
    let air = out.concrete.air.as_ref().unwrap();
    assert_eq!(air.time_to_land_foot_plant.to_bits(), 2.0_f32.to_bits());
    assert_eq!(
        air.foot_plant_total_absorb_time.to_bits(),
        3.0_f32.to_bits()
    );
    assert!(air.is_landing_foot_plant && air.is_foot_planting);
    assert!(!air.is_left_foot_planting);
}
#[test]
fn active_foot_selection_uses_native_toe_parts() {
    for (part, left, right) in [(15, true, false), (19, false, true), (14, false, false)] {
        let mut out = output();
        let mut source = Foot {
            valid: true,
            surface: Some(44),
            planting: true,
            part,
            reads: 0,
        };
        out.try_fill_footplant(&mut source).unwrap();
        let air = out.concrete.air.as_ref().unwrap();
        assert_eq!(
            (air.is_left_foot_planting, air.is_right_foot_planting),
            (left, right)
        );
        assert_eq!(air.foot_plant_surface_id, 44);
    }
}
