//! Native six-byte replay quaternion, RawDiscretize82808c38/82808af0.
use crate::reckoning_sin_cos::{HostReckoningSinCosNumeric,try_cosine,sine::try_sine};

/// QuatToAxisAngle82e7c5a8 and QuatCompressAxisAngle82e7c7c8.
pub(crate) fn compress(q:[f32;4])->Result<[u8;6],String>{
    use crate::reckoning_numeric::{HostReckoningNumeric,ReckoningNumeric,try_acos,try_asin,
        length_from_squared_and_estimate};
    let numeric=&mut HostReckoningNumeric;
    let w=if q[3]<0.{-q[3]}else{q[3]};
    let mut half=try_acos(w,numeric).map_err(|e|format!("{e:?}"))?;
    let (yaw,pitch)=if half>=1.5707964_f32/(65535_f32+1.){
        let squared=-(w*w-1.);
        let estimate=numeric.reciprocal_sqrt_estimate(squared).map_err(|e|format!("{e:?}"))?;
        let length=length_from_squared_and_estimate(squared,estimate);
        let yaw=crate::camera_compass::runtime::math::camera_atan2_pending(q[0],q[2],numeric)
            .map_err(|e|format!("{e:?}"))?.finish();
        let pitch=try_asin(q[1]/length,numeric).map_err(|e|format!("{e:?}"))?;
        (yaw,pitch)
    }else{half=0.;(0.,0.)};
    let values=[yaw.mul_add(f32::from_bits(0x3e22f983),0.5).mul_add(65535.,0.5),
        pitch.mul_add(f32::from_bits(0x3ea2f983),0.5).mul_add(65535.,0.5),
        (half*f32::from_bits(0x3f22f983)).mul_add(65535.,0.5)];
    let mut bytes=[0;6];
    for (index,value) in values.into_iter().enumerate(){
        // FCTIDZ follows VRFIM, then stores the low two bytes. Exceptional
        // conversion is a host boundary, not Rust's saturating float cast.
        let value=value.floor();
        if !value.is_finite()||value < i64::MIN as f32||value >= -(i64::MIN as f32){
            return Err("post-adjust quaternion quantization outside FCTIDZ host range".into());
        }
        bytes[index*2..index*2+2].copy_from_slice(&((value as i64) as u16).to_be_bytes());
    }
    Ok(bytes)
}

pub(crate) fn decompress(bytes:[u8;6])->Result<[f32;4],String>{
    let word=|index|u16::from_be_bytes([bytes[index],bytes[index+1]]) as f32;
    let yaw=(word(0)*0.000015259022_f32-0.5)*6.2831855_f32;
    let pitch=(word(2)*0.000015259022_f32-0.5)*3.1415927_f32;
    let half_angle=(word(4)*0.000015259022_f32)*1.5707964_f32;
    let numeric=&mut HostReckoningSinCosNumeric;
    // AnimSinCos2(pitch,yaw)825e9a30 uses separate XMVectorCos/Sin,
    // not XMVectorSinCos: preserve their distinct power dependencies.
    let sin_half=try_sine(half_angle,numeric).map_err(|e|format!("{e:?}"))?;
    let cos_yaw=try_cosine(yaw,numeric).map_err(|e|format!("{e:?}"))?;
    let sin_yaw=try_sine(yaw,numeric).map_err(|e|format!("{e:?}"))?;
    let cos_pitch=try_cosine(pitch,numeric).map_err(|e|format!("{e:?}"))?;
    let sin_pitch=try_sine(pitch,numeric).map_err(|e|format!("{e:?}"))?;
    let x=(cos_pitch*sin_yaw)*sin_half;
    let y=sin_pitch*sin_half;
    let z=(cos_pitch*cos_yaw)*sin_half;
    let w=try_cosine(half_angle,numeric).map_err(|e|format!("{e:?}"))?;
    Ok([x,y,z,w])
}
