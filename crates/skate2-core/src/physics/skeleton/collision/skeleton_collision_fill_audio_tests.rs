use super::*;
use crate::{
    ragdoll::SkeletonPartIndex,
    skeleton_collision_attributes::{SkeletonCollisionAttributeRead, SkeletonCollisionOffset},
};
use std::{cell::RefCell, rc::Rc};
type Trace = Rc<RefCell<Vec<String>>>;
struct Attributes {
    trace: Trace,
}
impl SkeletonAudioAttributes for Attributes {
    type Error = &'static str;
    type Global = ();
    type Pointer = usize;
    fn capture(&mut self) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push("global".into());
        Ok(())
    }
    fn lookup(&mut self, (): &(), key: u64) -> Result<usize, Self::Error> {
        let i = AUDIO_KEYS.iter().position(|k| *k == key).unwrap();
        self.trace.borrow_mut().push(format!("lookup{i}"));
        Ok(i)
    }
    fn float(&mut self, p: &usize) -> Result<f32, Self::Error> {
        self.trace.borrow_mut().push(format!("float{p}"));
        Ok([2.0, 3.0, 4.0, 5.0][*p])
    }
}
struct Init;
impl SkeletonCollisionAttributeRead for Init {
    type Pointer = ();
    type Error = ();
    fn cached_offset(&mut self, _: SkeletonCollisionOffset) -> Result<[f32; 4], ()> {
        Ok([0.0; 4])
    }
    fn lookup(&mut self, _: u64) -> Result<(), ()> {
        Ok(())
    }
    fn float(&mut self, (): &()) -> Result<f32, ()> {
        Ok(0.0)
    }
}
fn data() -> SkeletonCollisionData {
    let mut d = SkeletonCollisionData::try_new(&mut Init).unwrap();
    d.specific_parts.contact = [Some(0); 2];
    d.specific_parts.recent_contact = [Some(0); 2];
    d
}
struct Services {
    trace: Trace,
    draw: bool,
    draw_reads: usize,
    fail_draw: bool,
    overlays: usize,
}
impl SkeletonAudioServices for Services {
    type Error = &'static str;
    fn velocity_delta(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        self.trace.borrow_mut().push(format!("delta{part}"));
        Ok(SkeletonRootVector {
            xyz: [2.0, 0.0, 0.0],
            w: None,
        })
    }
    fn position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        self.trace.borrow_mut().push(format!("position{part}"));
        Ok(SkeletonRootVector {
            xyz: [7.0, 0.0, 0.0],
            w: None,
        })
    }
    fn mass(&mut self, part: usize) -> Result<f32, Self::Error> {
        self.trace.borrow_mut().push(format!("mass{part}"));
        Ok(3.0)
    }
    fn draw_audio_info(&mut self) -> Result<bool, Self::Error> {
        self.draw_reads += 1;
        if self.fail_draw {
            Err("draw")
        } else {
            Ok(self.draw)
        }
    }
    fn debug_overlay_noop(&mut self) -> Result<(), Self::Error> {
        self.overlays += 1;
        Ok(())
    }
}
#[derive(Default)]
struct Record {
    positions: usize,
    flags: Vec<(AudioFlag, u8)>,
    scalars: Vec<(AudioScalar, f32)>,
    surfaces: [u32; 8],
    speeds: [f32; 8],
    forces: [f32; 8],
    bone_surface: Vec<u32>,
    bone_w: Vec<Option<f32>>,
}
struct Borrow<'a>(&'a mut Record);
impl SkeletonAudioRecord for Borrow<'_> {
    fn flag(&mut self, f: AudioFlag, v: u8) {
        self.0.flags.push((f, v));
    }
    fn scalar(&mut self, f: AudioScalar, v: f32) {
        self.0.scalars.push((f, v));
    }
    fn bone_vector(&mut self, _: usize, f: AudioBoneVector, v: SkeletonRootVector) {
        if f == AudioBoneVector::Position {
            self.0.positions += 1;
        }
        self.0.bone_w.push(v.w);
    }
    fn bone_scalar(&mut self, _: usize, _: AudioBoneScalar, _: f32) {}
    fn bone_surface(&mut self, _: usize, _: bool, v: u32) {
        self.0.bone_surface.push(v);
    }
    fn bone_flag(&mut self, _: usize, _: AudioBoneFlag, _: u8) {}
    fn slide_speed(&mut self, i: usize, v: f32) {
        self.0.speeds[i] = v;
    }
    fn contact_force(&mut self, i: usize, v: f32) {
        self.0.forces[i] = v;
    }
    fn surface(&mut self, i: usize, v: u32) {
        self.0.surfaces[i] = v;
    }
}
struct Output {
    trace: Trace,
    record: Record,
    locked: bool,
}
impl SkeletonAudioOutput for Output {
    type Error = &'static str;
    type Guard = ();
    type Record<'a> = Borrow<'a>;
    fn sample_guard(&mut self) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push("guard".into());
        Ok(())
    }
    fn begin(&mut self, (): ()) -> Result<Borrow<'_>, Self::Error> {
        self.trace.borrow_mut().push("begin".into());
        if self.locked {
            Err("locked")
        } else {
            Ok(Borrow(&mut self.record))
        }
    }
}
struct Numeric {
    calls: usize,
    fail: Option<usize>,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.calls += 1;
        if self.fail == Some(self.calls) {
            return Err("dot");
        }
        Ok((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("source FDIVS")
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("no rsqrt")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no integer")
    }
}
fn collaborators() -> (Attributes, Services, Output, Numeric) {
    let t = Trace::default();
    (
        Attributes { trace: t.clone() },
        Services {
            trace: t.clone(),
            draw: false,
            draw_reads: 0,
            fail_draw: false,
            overlays: 0,
        },
        Output {
            trace: t,
            record: Record {
                surfaces: [77; 8],
                ..Record::default()
            },
            locked: false,
        },
        Numeric {
            calls: 0,
            fail: None,
        },
    )
}
#[test]
fn all_bones_and_empty_categories_are_real_publication_preserving_old_surface() {
    let mut collision = data();
    collision.bone_data[0].contact_surface_id = 0x8000_0001;
    collision.unrideable_contact = 0x80;
    let (attributes, services, output, numeric) = &mut collaborators();
    collision
        .try_fill_audio_collision(attributes, services, output, numeric)
        .unwrap();
    assert_eq!(output.record.positions, 24);
    assert_eq!(numeric.calls, 48);
    assert_eq!(services.draw_reads, 9);
    assert_eq!(output.record.surfaces, [77; 8]);
    assert_eq!(output.record.bone_surface[0], 0x8000_0001);
    assert!(output.record.bone_w[0].is_none());
    assert_eq!(output.record.flags[0], (AudioFlag::Unrideable, 0x80));
    assert_eq!(
        &attributes.trace.borrow()[..14],
        &[
            "global", "lookup0", "global", "float0", "lookup1", "global", "float1", "lookup2",
            "global", "float2", "lookup3", "guard", "float3", "begin"
        ]
    );
}
#[test]
fn fourth_attribute_value_is_loaded_after_guard_sample_even_when_locked() {
    let mut collision = data();
    let (attributes, services, output, numeric) = &mut collaborators();
    output.locked = true;
    assert!(matches!(
        collision.try_fill_audio_collision(attributes, services, output, numeric),
        Err(SkeletonAudioError::Output("locked"))
    ));
    assert_eq!(
        &attributes.trace.borrow()[11..],
        &["guard", "float3", "begin"]
    );
    assert!(output.record.flags.is_empty());
}
#[test]
fn second_bone_dot_failure_precedes_first_position_write() {
    let mut collision = data();
    let (attributes, services, output, numeric) = &mut collaborators();
    numeric.fail = Some(2);
    assert!(matches!(
        collision.try_fill_audio_collision(attributes, services, output, numeric),
        Err(SkeletonAudioError::Numeric("dot"))
    ));
    assert_eq!(output.record.flags.len(), 3);
    assert_eq!(output.record.positions, 0);
    assert_eq!(attributes.trace.borrow().last().unwrap(), "position0");
}
#[test]
fn category_updates_accumulators_before_late_debug_failure() {
    let mut collision = data();
    collision.contact_bone[0] = Some(SkeletonPartIndex::Neck1);
    collision.slide_speed[0] = 1.0;
    collision.contact_normal[0] = SkeletonCollisionVector::known([1.0, 0.0, 0.0, 0.0]);
    collision.audio_surface_type[0] = 0xffff_fffe;
    let (attributes, services, output, numeric) = &mut collaborators();
    services.fail_draw = true;
    assert!(matches!(
        collision.try_fill_audio_collision(attributes, services, output, numeric),
        Err(SkeletonAudioError::Source("draw"))
    ));
    assert_eq!(
        collision.contact_force_total_during_wipeout.to_bits(),
        3.0f32.to_bits()
    );
    assert_eq!(
        collision.slide_force_total_during_wipeout.to_bits(),
        2.0f32.to_bits()
    );
    assert_eq!(output.record.forces[0].to_bits(), 1.0f32.to_bits());
    assert_eq!(output.record.surfaces[0], 0xffff_fffe);
}
#[test]
fn unwritten_face_stops_before_accumulator_clamping_not_at_entry() {
    let mut collision = data();
    collision.specific_parts.contact[1] = None;
    collision.contact_force_total_during_wipeout = 2.0;
    let (attributes, services, output, numeric) = &mut collaborators();
    assert!(matches!(
        collision.try_fill_audio_collision(attributes, services, output, numeric),
        Err(SkeletonAudioError::UnwrittenSpecificContact {
            recent: false,
            index: 1
        })
    ));
    assert_eq!(output.record.positions, 24);
    assert_eq!(services.draw_reads, 8);
    assert_eq!(
        collision.contact_force_total_during_wipeout.to_bits(),
        2.0f32.to_bits()
    );
}
#[test]
fn enabled_debug_requires_each_actual_overlay_and_fsel_nan_finishes_at_one() {
    let mut collision = data();
    let (attributes, services, output, numeric) = &mut collaborators();
    services.draw = true;
    collision
        .try_fill_audio_collision(attributes, services, output, numeric)
        .unwrap();
    assert_eq!(services.overlays, 44);
    assert_eq!(bound_force(f32::NAN).to_bits(), 1.0f32.to_bits());
    assert_eq!(bound_total(f32::NAN).to_bits(), 1.0f32.to_bits());
    assert_eq!(bound_total(-0.0).to_bits(), 0);
    assert_eq!(absolute(-0.0).to_bits(), 0);
}
