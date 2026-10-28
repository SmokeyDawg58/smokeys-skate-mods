use super::*;
use std::{cell::RefCell, rc::Rc};

type Events = Rc<RefCell<Vec<String>>>;
struct Attributes {
    events: Events,
    value: f32,
    fail_load: bool,
}
impl SkateboardAttributeRead for Attributes {
    type Pointer = ();
    type Error = &'static str;
    fn lookup(&mut self, class: SkateboardAttributeClass, key: u64) -> Result<(), Self::Error> {
        assert_eq!(class, SkateboardAttributeClass::Deck);
        assert_eq!(key, 0x8d30_65ce_d858_a7df);
        self.events.borrow_mut().push("lookup".into());
        Ok(())
    }
    fn float(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("float".into());
        if self.fail_load {
            Err("float")
        } else {
            Ok(self.value)
        }
    }
    fn byte(&mut self, (): &()) -> Result<u8, Self::Error> {
        panic!("not a byte lookup")
    }
    fn integer(&mut self, (): &()) -> Result<i32, Self::Error> {
        panic!("not an integer lookup")
    }
}
struct Services {
    events: Events,
    fail: Option<&'static str>,
    free_wheel: bool,
    angular_drag: f32,
    materials: [Option<SkateboardStandardMaterial>; 7],
    top_flags: [u32; 3],
    deck_flags: [u32; 3],
    counts: Vec<i32>,
    count_reads: usize,
}
impl Services {
    fn event(&mut self, event: String) -> Result<(), &'static str> {
        let refused = self.fail == Some(event.as_str());
        self.events.borrow_mut().push(event);
        if refused { Err("service") } else { Ok(()) }
    }
}
impl SkateboardSetStandardServices for Services {
    type Error = &'static str;
    type Body = ();
    type Assembly = ();
    type Parts = ();
    type PartBody = ();
    type Inertia = ();
    type Materials = ();
    fn capture_body(&mut self) -> Result<(), Self::Error> {
        self.event("body".into())
    }
    fn clear_free_wheel(&mut self, (): &()) -> Result<(), Self::Error> {
        self.event("clear".into())?;
        self.free_wheel = false;
        Ok(())
    }
    fn capture_assembly(&mut self, (): &()) -> Result<(), Self::Error> {
        self.event("assembly".into())
    }
    fn capture_parts(&mut self, (): &()) -> Result<(), Self::Error> {
        self.event("parts".into())
    }
    fn capture_part_body(&mut self, (): &(), ordinal: usize) -> Result<(), Self::Error> {
        assert_eq!(ordinal, 6);
        self.event("deck-body".into())
    }
    fn capture_inertia(&mut self, (): &()) -> Result<(), Self::Error> {
        self.event("inertia".into())
    }
    fn store_angular_drag(&mut self, (): &(), value: f32) -> Result<(), Self::Error> {
        self.event("drag".into())?;
        self.angular_drag = value;
        Ok(())
    }
    fn capture_standard_materials(&mut self, (): &()) -> Result<(), Self::Error> {
        self.event("materials".into())
    }
    fn store_material(
        &mut self,
        (): &(),
        ordinal: usize,
        (): &(),
        selected: SkateboardStandardMaterial,
    ) -> Result<(), Self::Error> {
        self.event(format!("material{ordinal}"))?;
        self.materials[ordinal] = Some(selected);
        Ok(())
    }
    fn enable_top_volume(&mut self, (): &(), index: usize) -> Result<(), Self::Error> {
        self.event(format!("top{index}"))?;
        self.top_flags[index] |= 1;
        Ok(())
    }
    fn deck_volume_count(&mut self, (): &()) -> Result<i32, Self::Error> {
        self.event(format!("count{}", self.count_reads))?;
        let index = self.count_reads.min(self.counts.len() - 1);
        self.count_reads += 1;
        Ok(self.counts[index])
    }
    fn enable_deck_volume(&mut self, (): &(), index: usize) -> Result<(), Self::Error> {
        self.event(format!("child{index}"))?;
        self.deck_flags[index] |= 1;
        Ok(())
    }
}
fn fixture() -> (Attributes, Services) {
    let events = Rc::new(RefCell::new(Vec::new()));
    (
        Attributes {
            events: Rc::clone(&events),
            value: 0.25,
            fail_load: false,
        },
        Services {
            events,
            fail: None,
            free_wheel: true,
            angular_drag: -7.0,
            materials: [None; 7],
            top_flags: [0x80, 0x42, 0x24],
            deck_flags: [0x40; 3],
            counts: vec![3],
            count_reads: 0,
        },
    )
}

#[test]
fn source_pointer_reads_scalar_load_and_each_material_store_are_ordered() {
    let (mut a, mut s) = fixture();
    try_set_standard(&mut a, &mut s).unwrap();
    assert_eq!(
        &*s.events.borrow(),
        &[
            "body",
            "clear",
            "lookup",
            "assembly",
            "float",
            "parts",
            "deck-body",
            "inertia",
            "drag",
            "assembly",
            "materials",
            "parts",
            "material6",
            "assembly",
            "parts",
            "material0",
            "assembly",
            "parts",
            "material1",
            "assembly",
            "parts",
            "material2",
            "assembly",
            "parts",
            "material3",
            "assembly",
            "parts",
            "material4",
            "assembly",
            "parts",
            "material5",
            "body",
            "top0",
            "top1",
            "top2",
            "count0",
            "child0",
            "count1",
            "child1",
            "count2",
            "child2",
            "count3",
        ]
    );
    assert!(!s.free_wheel);
    assert_eq!(
        s.angular_drag.to_bits(),
        (0.25_f32 * f32::from_bits(0x426f_ffff)).to_bits()
    );
    assert_eq!(
        s.materials,
        [
            Some(SkateboardStandardMaterial::Wheel),
            Some(SkateboardStandardMaterial::Wheel),
            Some(SkateboardStandardMaterial::Wheel),
            Some(SkateboardStandardMaterial::Wheel),
            Some(SkateboardStandardMaterial::Truck),
            Some(SkateboardStandardMaterial::Truck),
            Some(SkateboardStandardMaterial::Deck)
        ]
    );
    assert_eq!(s.top_flags, [0x81, 0x43, 0x25]);
    assert_eq!(s.deck_flags, [0x41; 3]);
}

#[test]
fn late_failures_retain_the_specific_native_prefix() {
    let (mut a, mut s) = fixture();
    a.fail_load = true;
    assert_eq!(
        try_set_standard(&mut a, &mut s),
        Err(SkateboardSetStandardError::Attribute("float"))
    );
    assert!(!s.free_wheel);
    assert_eq!(s.angular_drag.to_bits(), (-7.0_f32).to_bits());
    assert_eq!(
        &*s.events.borrow(),
        &["body", "clear", "lookup", "assembly", "float"]
    );

    let (mut a, mut s) = fixture();
    s.fail = Some("material3");
    assert!(try_set_standard(&mut a, &mut s).is_err());
    assert!(s.materials[6].is_some());
    assert!(s.materials[..3].iter().all(Option::is_some));
    assert_eq!(s.materials[3..6], [None; 3]);
    assert_eq!(s.top_flags, [0x80, 0x42, 0x24]);

    let (mut a, mut s) = fixture();
    s.fail = Some("top1");
    assert!(try_set_standard(&mut a, &mut s).is_err());
    assert!(s.materials.iter().all(Option::is_some));
    assert_eq!(s.top_flags, [0x81, 0x42, 0x24]);
    assert_eq!(s.deck_flags, [0x40; 3]);

    let (mut a, mut s) = fixture();
    s.fail = Some("count1");
    assert!(try_set_standard(&mut a, &mut s).is_err());
    assert_eq!(s.deck_flags, [0x41, 0x40, 0x40]);
}

#[test]
fn actual_count_is_reread_after_each_child_and_nonpositive_counts_skip_children() {
    for initial in [-1, 0] {
        let (mut a, mut s) = fixture();
        s.counts = vec![initial];
        try_set_standard(&mut a, &mut s).unwrap();
        assert_eq!(s.deck_flags, [0x40; 3]);
        assert_eq!(s.count_reads, 1);
    }
    let (mut a, mut s) = fixture();
    s.counts = vec![3, 1];
    try_set_standard(&mut a, &mut s).unwrap();
    assert_eq!(s.deck_flags, [0x41, 0x40, 0x40]);
    assert_eq!(s.count_reads, 2);
}

#[test]
fn damping_preserves_negative_zero_and_nonfinite_source_values() {
    for bits in [0x8000_0000, 0x7f80_0000, 0xff80_0000, 0x7fc1_2345] {
        let (mut a, mut s) = fixture();
        a.value = f32::from_bits(bits);
        try_set_standard(&mut a, &mut s).unwrap();
        let expected = a.value * f32::from_bits(0x426f_ffff);
        if expected.is_nan() {
            assert!(s.angular_drag.is_nan());
        } else {
            assert_eq!(s.angular_drag.to_bits(), expected.to_bits());
        }
    }
}
