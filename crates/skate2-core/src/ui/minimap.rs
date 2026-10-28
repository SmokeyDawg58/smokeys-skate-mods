//! MiniMap826bcdd8 / MapIconManager8262ea20 presentation facts.
//! Career icons are supplied by their owners, never synthesized from destinations.

#[derive(Clone, Debug)]
pub struct Icon {
    pub texture: String,
    pub position: [f32; 2],
    pub rotation: f32,
    pub scale: f32,
    pub visible: bool,
    pub fade_at_edge: bool,
    pub maximum_range: f32,
    pub pinch_point: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub position: [f32; 2],
    pub look: [f32; 2],
    pub velocity: [f32; 3],
    pub wiping_out: bool,
    pub district_key: u64,
    pub district_name: Option<String>,
    /// `cActivityManager::GetMainChallenge`, which `FrontEndManager::UpdateDistrictZoneName`
    /// (`0x82638820`) reads.
    pub main_challenge: bool,
    pub no_skate_zone: bool,
    pub dynamic_icons: Vec<Icon>,
}

#[derive(Default)]
pub struct Direction([f32; 2]);
impl Direction {
    /// Render826bcdd8 runs this once per presentation, not per simulation tick.
    /// Constructor826b5164..5194 explicitly zeros all direction lanes.
    pub fn update(&mut self, frame: &Frame) -> f32 {
        let speed = length(frame.velocity);
        let mut next = frame.look;
        let moving = speed > 1.; // kMinimapSpeedThresh82f5b278
        if moving {
            next = [frame.velocity[0], frame.velocity[2]];
            let length = length([next[0],0.,next[1]]);
            // Fast purely vertical motion leaves the previous heading intact.
            if length <= 0.001 {return heading([-self.0[0],-self.0[1]]);}
            next = next.map(|v| v / length);
        }
        if moving || !frame.wiping_out {
            let smooth = 0.85_f32; // kMinimapSmoothness82f5b2d8
            self.0 = std::array::from_fn(|i| self.0[i].mul_add(smooth,next[i]*(1.-smooth)));
            if moving {
                let length = length([self.0[0],0.,self.0[1]]);
                if length > 0.001 { self.0 = self.0.map(|v| v / length); }
            }
        }
        // Math::Angle::GetHeading on the NEGATED direction, RenderIcons826b7650.
        heading([-self.0[0],-self.0[1]])
    }
}

/// Original radius/fade constants82f58cb0..82f59130; world X/Z is in metres.
pub fn placement(icon: &Icon, player: [f32; 2], heading: f32) -> Option<([f32; 2], f32)> {
    if !icon.visible { return None; }
    let mut delta = [icon.position[0]-player[0], icon.position[1]-player[1]];
    use crate::reckoning_numeric::{HostReckoningNumeric,ReckoningNumeric,length_from_squared_and_estimate};
    let squared=delta[1].mul_add(delta[1],delta[0]*delta[0]);
    let estimate=HostReckoningNumeric.reciprocal_sqrt_estimate(squared).expect("host estimate");
    let distance = length_from_squared_and_estimate(squared,estimate);
    let radius = 71.281097_f32;
    let mut alpha = 1.;
    if distance > radius {
        let scale=(1./distance)*radius;
        delta = delta.map(|v| scale*v);
        if icon.fade_at_edge {
            let disappear = if icon.pinch_point {10000.} else if icon.maximum_range>0. {icon.maximum_range} else {1000.};
            let far = if icon.pinch_point {5000.} else {350.};
            alpha = if distance>disappear {0.} else {1.-(distance-60.)/(far-60.)};
        }
    }
    if alpha <= 0. { return None; }
    let (sin,cos) = sin_cos(heading);
    Some(([cos.mul_add(delta[0],0.)-sin*delta[1], sin.mul_add(delta[0],cos*delta[1])],alpha))
}

fn length(v:[f32;3])->f32 {
    crate::reckoning_numeric::try_length([v[0],v[1],v[2],0.],
        &mut crate::reckoning_numeric::HostReckoningNumeric).expect("host VMX length")
}
pub fn sin_cos(angle:f32)->(f32,f32) {
    crate::reckoning_sin_cos::try_sin_cos(angle,&mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric)
        .expect("infallible native sin/cos polynomial")
}
/// Math::Angle::GetHeading826b6ee8: normalize X/Z then select an asin
/// quadrant, including the original Normalize825ece78 at each addition.
pub fn heading(direction:[f32;2])->f32 {
    use crate::reckoning_numeric::{HostReckoningNumeric,try_normalize_unsafe,try_normalize_angle,try_asin};
    let mut n=HostReckoningNumeric;
    let v=try_normalize_unsafe([direction[0],0.,direction[1],0.],&mut n).expect("host normalize");
    let (base,argument)=if v[0]>=0. {
        if v[2]>=0. {(0.,v[0])} else {(f32::from_bits(0x3fc90fdb),-v[2])}
    } else if v[2]<=0. {(f32::from_bits(0x40490fdb),-v[0])}
      else {(f32::from_bits(0x4096cbe4),v[2])};
    let base=try_normalize_angle(base,&mut n).expect("host angle");
    let clamped=if -1.-argument>=0. {-1.} else {argument};
    let clamped=if 1.-clamped>=0. {clamped} else {1.};
    let angle=try_asin(clamped,&mut n).expect("host asin");
    let angle=try_normalize_angle(angle,&mut n).expect("host angle");
    try_normalize_angle(base+angle,&mut n).expect("host angle")
}

/// Entity::GetHeading825f4880 uses EulerXYZFromNormalizedVector, which has
/// its own short-vector guard. Reuse that existing owner instead of Angle's
/// unsafe normalization (used only by the minimap's smoothed direction).
pub(crate) fn entity_heading(forward:[f32;3])->Result<f32,String> {
    crate::camera_compass::runtime::math::euler_angles(
        crate::skeleton_root_transforms::SkeletonRootVector {xyz:forward,w:Some(0.)},
        &mut crate::reckoning_numeric::HostReckoningNumeric)
        .map(|(_,heading)|heading).map_err(|e|format!("{e:?}"))
}
