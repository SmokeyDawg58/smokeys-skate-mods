//! User_ICELITEWorldManager826dad28/826db510; post-shake camera constraints.
//! The caller provides Presentation's BF_Default=7 line batches, without a
//! simulation step. This uses the project's existing host numeric policy.
use skate2_formats::ice_camera::{self, Spaces, editor::{Author, CameraKind, SHAKE, element}};
use crate::{reckoning_numeric::{HostReckoningNumeric,try_length,try_normalize_safe},
    skeleton_root_transforms::SkeletonRootVector};

pub(super) struct Hit {pub normal:[f32;3],pub time:f32}
pub(super) struct Collision {previous:[f32;3],first:bool,clip:Option<usize>}
impl Default for Collision {fn default()->Self {Self{previous:[0.;3],first:true,clip:None}}}

impl Collision {
    pub fn resolve(&mut self,author:&mut Author,spaces:&Spaces,matrix:&mut [f32;16],hips:[f32;3],
        enabled:bool,max_distance:f32,near:f32,
        mut lines:impl FnMut(&[([f32;3],[f32;3])],f32)->Result<Vec<Option<Hit>>,String>)->Result<(),String> {
        // UpdateHardCuts82a54518 uses the shake channel, which follows the hard clips.
        let clip=author.take.current_interval(SHAKE);
        if clip!=self.clip {self.previous=[0.;3];self.first=true;self.clip=clip;}
        if !enabled||author.take.value_int(element::AVOID_STATIC)==0 {return Ok(());}
        let free=author.current_camera()==CameraKind::Free;
        let (_,anchor)=author.take.eye_and_look(spaces)?;
        let anchor=[anchor[1],anchor[2],anchor[0]];
        let eye=[matrix[12],matrix[13],matrix[14]];
        let delta=sub(eye,hips);
        // ClampDistance: ICE order is (Skate Z,X,Y).
        let length=(delta[2]*delta[2]+(delta[1]*delta[1]+delta[0]*delta[0])).sqrt();
        let mut changed=length>max_distance;
        if changed {
            let inverse=1.0/length;
            for i in 0..3 {matrix[12+i]=(delta[i]*inverse).mul_add(max_distance,hips[i]);}
        }
        let end=[matrix[12],matrix[13],matrix[14]];
        let start=self.previous;
        let delta=sub(end,start);
        if delta.iter().any(|v|v.abs()>f32::from_bits(0x3400_0000)) {
            let fatness=near*1.3;
            let mut hits=lines(&[(start,end),(hips,end)],fatness)?.into_iter();
            let movement=hits.next().ok_or("missing camera movement line result")?;
            let los=hits.next().ok_or("missing camera sight line result")?;
            let mut resolved=None;
            if self.first {
                if let Some(hit)=los.as_ref() {
                    let t=(hit.time-0.01).max(0.);
                    resolved=Some(std::array::from_fn(|i|(end[i]-hips[i]).mul_add(t,hips[i])));
                }
            } else if let Some(hit)=movement {
                let dot=(delta[0]*hit.normal[0]+delta[1]*hit.normal[1])+delta[2]*hit.normal[2];
                let projected=std::array::from_fn(|i|end[i]-hit.normal[i]*dot);
                let slide=lines(&[(start,projected)],fatness)?.into_iter().next().ok_or("missing camera slide line result")?;
                let position=if slide.is_none() {projected} else {start};
                if los.is_some() {
                    if !free {
                        let mut numeric=HostReckoningNumeric;
                        let d=sub(anchor,position);
                        let z=try_normalize_safe([d[0],d[1],d[2],0.],[0.;4],&mut numeric).map_err(|e|e.to_string())?;
                        let mut z=[z[0],z[1],z[2]];
                        if z.iter().all(|v|v.abs()<=f32::from_bits(0x3400_0000)) {z=[0.,0.,1.];}
                        let up=[0.,1.,0.];
                        let x=if sub(z,up).iter().all(|v|v.abs()<=f32::from_bits(0x3780_0000)) {[1.,0.,0.]}
                            else {ice_camera::vmx_cross(up,z)};
                        let y=ice_camera::vmx_cross(z,x);
                        for (row,axis) in [x,y,z].into_iter().enumerate() {matrix[row*4..row*4+3].copy_from_slice(&axis);matrix[row*4+3]=0.;}
                    }
                    resolved=Some(position);
                }
            }
            if let Some(position)=resolved {matrix[12..15].copy_from_slice(&position);changed=true;}
            self.previous=[matrix[12],matrix[13],matrix[14]];
        }
        if changed {
            if free {author.set_free_camera_matrix(*matrix,spaces)?;}
            else {
                let mut numeric=HostReckoningNumeric;
                let direction=SkeletonRootVector::known([matrix[8],matrix[9],matrix[10],0.]);
                let (pitch,yaw)=crate::camera_compass::runtime::math::euler_angles(direction,&mut numeric).map_err(|e|e.to_string())?;
                let mut spin=yaw*57.29578-180.;
                if spin<0. {spin+=360.;} else if spin>360. {spin-=360.;}
                let mut tilt=-pitch*57.29578;if tilt<0. {tilt+=360.;}
                let d=sub([matrix[12],matrix[13],matrix[14]],anchor);
                let size=try_length([d[0],d[1],d[2],0.],&mut numeric).map_err(|e|e.to_string())?;
                author.set_bubble((spin*182.04445) as i32 as u16,(tilt*182.04445) as i32 as u16,size,spaces)?;
            }
        }
        self.first=false;
        Ok(())
    }
}
fn sub(a:[f32;3],b:[f32;3])->[f32;3] {std::array::from_fn(|i|a[i]-b[i])}
