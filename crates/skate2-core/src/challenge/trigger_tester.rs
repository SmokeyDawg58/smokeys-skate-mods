//! A skater as a challenge trigger tester:
//! `TriggerVolumeUtils::GenerateTesterVolume` (`0x82e20f90`) and the enter/exit
//! bookkeeping of `cTriggerVolumeMessageGroup::Update` (`0x82e24020`).

use skate2_formats::collision::{Volume, VolumeType};

/// `Challenge_TriggerVolumes` cvars.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TesterTuning {
    /// `kTriggerVolumeCharacterType`: 0 capsule, 1 cylinder.
    pub cylinder: bool,
    /// `kTriggerVolumeCharacterWidth`.
    pub width: f32,
    /// `kTriggerVolumeCharacterHeightAddition`.
    pub height_addition: f32,
    /// `kTriggerVolumeCharacterHeightOffset`.
    pub height_offset: f32,
}

impl Default for TesterTuning {
    fn default() -> Self {
        Self { cylinder: true, width: 0.34, height_addition: 0.05, height_offset: -0.02 }
    }
}

/// The tester volume and its world transform (`Matrix44Affine`: three axes,
/// then the position).
#[derive(Clone, Debug, PartialEq)]
pub struct Tester {
    pub volume: Volume,
    pub transform: [f32; 16],
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// `a.yzx * b - a * b.yzx`, the permuted cross product the native forms.
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length_squared = dot(v, v);
    if length_squared == 0.0 {
        return [0.0; 3];
    }
    let inverse = 1.0 / length_squared.sqrt();
    [v[0] * inverse, v[1] * inverse, v[2] * inverse]
}

/// `GenerateTesterVolume`: the axis runs from the centre of mass through the
/// base; the volume spans the base-to-top height plus the addition, shifted by
/// the offset.
#[must_use]
pub fn generate(base: [f32; 3], top: [f32; 3], centre_of_mass: [f32; 3], tuning: TesterTuning) -> Tester {
    let down = sub(base, centre_of_mass);
    let reference = if down[0] > 0.001 { [1.0, 0.0, 0.0] } else { [0.0, 0.0, 1.0] };
    let z = normalize(down);
    let x = normalize(cross(reference, z));
    let y = cross(z, x);
    let height = dot(sub(top, base), sub(top, base)).sqrt();
    let (offset, shape) = if tuning.cylinder {
        let half = height.mul_add(0.5, tuning.height_addition);
        (tuning.height_offset + half, [half.to_bits(), tuning.width.to_bits(), 0])
    } else {
        let half = height.mul_add(0.5, -tuning.width) + tuning.height_addition;
        (tuning.height_offset + tuning.width + half, [half.to_bits(), 0, 0])
    };
    let position = [base[0] - z[0] * offset, base[1] - z[1] * offset, base[2] - z[2] * offset];
    let identity = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    let volume = Volume {
        transform: identity,
        vtable_offset: if tuning.cylinder { VolumeType::Cylinder } else { VolumeType::Capsule } as u32,
        shape_words: shape,
        radius: if tuning.cylinder { 0.0 } else { tuning.width },
        group_id: 0,
        surface_id: 0,
        flags: 1,
    };
    Tester {
        volume,
        transform: [
            x[0], x[1], x[2], 0.0, y[0], y[1], y[2], 0.0, z[0], z[1], z[2], 0.0, position[0], position[1], position[2], 1.0,
        ],
    }
}

impl Tester {
    /// The world bounding box of the tester (`Volume::GetBBox`, tight).
    #[must_use]
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let axis = [self.transform[8], self.transform[9], self.transform[10]];
        let centre = [self.transform[12], self.transform[13], self.transform[14]];
        let half = f32::from_bits(self.volume.shape_words[0]);
        let radius = if self.volume.vtable_offset == VolumeType::Cylinder as u32 {
            f32::from_bits(self.volume.shape_words[1])
        } else {
            self.volume.radius
        };
        let extent: [f32; 3] =
            std::array::from_fn(|i| axis[i].abs() * half + radius * (1.0 - axis[i] * axis[i]).max(0.0).sqrt());
        (std::array::from_fn(|i| centre[i] - extent[i]), std::array::from_fn(|i| centre[i] + extent[i]))
    }
}

/// `GenerateEnterExitData`: entries in `current` but not `previous`, then
/// entries in `previous` but not `current`, each in ascending order.
#[must_use]
pub fn enter_exit(previous: &[u32], current: &[u32]) -> (Vec<u32>, Vec<u32>) {
    let entered = current.iter().copied().filter(|i| !previous.contains(i)).collect();
    let exited = previous.iter().copied().filter(|i| !current.contains(i)).collect();
    (entered, exited)
}
