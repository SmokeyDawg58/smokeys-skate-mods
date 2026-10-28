//! Original BigBlack blocking geometry (82e7b4d0) and geotest helpers.
use crate::ai::vmx::{self,add,sub,scale,dot3,sqrt_guarded,reciprocal};
fn flat(mut v:[f32;4])->[f32;4]{v[1]=0.;v[3]=0.;v}
fn unit(v:[f32;4])->[f32;4]{scale(v,vmx::rsqrt(dot3(v,v)))}
/// CircleRayIntersection82e559b8: strictly interior projected point, nearest intersection first.
fn circle_ray(centre:[f32;4],radius:f32,point:[f32;4],direction:[f32;4])->Result<Option<[f32;4]>,String>{
    let start=flat(point);let end=flat(add(point,direction));let c=flat(centre);
    let d=sub(end,start);let towards=sub(c,start);let squared=dot3(d,d);
    if !(squared>0.){return Err("native CircleRayIntersection direction assertion1157".into());}
    let t=reciprocal(squared)*dot3(towards,d);
    if 0.>t{return Ok(None);}
    let projection=vmx::madd(d,[t;4],start);let delta=sub(c,projection);
    let distance=dot3(delta,delta);
    if !(0.>distance-radius*radius){return Ok(None);}
    let (axis,half_chord)=if distance>f32::from_bits(0x3727_c5ac){
        let v=sub(projection,c);(unit([-v[2],0.,v[0],0.]),sqrt_guarded(radius*radius-dot3(v,v)))
    }else{(unit(towards),radius)};
    let mut a=add(projection,scale(axis,half_chord));let b=sub(projection,scale(axis,half_chord));
    if dot3(sub(a,point),sub(a,point))>dot3(sub(b,point),sub(b,point)){a=b;}
    a[1]=centre[1];Ok(Some(a))
}
/// CircleCircleIntersection82e55dd0 solves x first, selects each z by circle2 residual,
/// then orders the two points by cross product. Preserve the original z-division.
fn circle_circle(c1:[f32;4],r1:f32,c2:[f32;4],r2:f32)->Option<[[f32;4];2]>{
    let d=sub(c2,c1);let dx=d[0];let dz=d[2];let xx=dx*dx;let zz=dz*dz;
    if xx+zz>(r1+r2)*(r1+r2){return None;}
    let rr=r1*r1;let rhs=((rr-r2*r2)+zz)+xx;
    let a=reciprocal(zz)*xx+1.;let minus_b=reciprocal(zz)*(dx*rhs);
    let cc=reciprocal(zz*4.)*(rhs*rhs)-rr;
    let disc=minus_b*minus_b-(a*4.)*cc;
    let root=sqrt_guarded(if 0.>disc{0.}else{disc});
    let xs=[reciprocal(a*2.)*(minus_b+root),reciprocal(a*2.)*(minus_b-root)];
    let mut points=xs.map(|x|{
        let z=sqrt_guarded(rr-x*x);
        let residual=|z:f32| (r2-sqrt_guarded((x-dx).mul_add(x-dx,(z-dz)*(z-dz)))).abs();
        let z=if residual(-z)>residual(z){z}else{-z};
        [x+c1[0],c2[1],z+c1[2],0.]
    });
    let a=sub(points[0],c2);let b=sub(points[1],c2);
    if a[2]*b[0]-a[0]*b[2]>0.{points.swap(0,1);}
    Some(points)
}
/// CirclePointTangent82e56340 uses an auxiliary circle at the midpoint.
fn circle_tangent(c:[f32;4],r:f32,p:[f32;4])->Option<[[f32;4];2]>{
    let d=sub(c,p);let squared=dot3(d,d);
    if r*r>squared{return None;}
    circle_circle(c,r,vmx::madd(d,[0.5;4],p),sqrt_guarded(squared)*0.5)
}
/// ComputeBlockPoint82e7b4d0. The original compares squared lengths with minRadius
/// (and 4*minRadius), without squaring that parameter.
pub(crate) fn block_point(protectee:[f32;4],threat:[f32;4],predicted:[f32;4],
    imminent:f32,min_radius:f32)->Result<Option<[f32;4]>,String>{
    let delta=sub(protectee,threat);let distance=dot3(delta,delta);
    if distance<=imminent*imminent{
        return Ok((distance>min_radius*4.).then(||add(threat,scale(unit(delta),min_radius))));
    }
    let mut point=if let Some(point)=circle_ray(protectee,imminent,threat,sub(predicted,threat))?{point}
        else if let Some([a,b])=circle_tangent(protectee,imminent,threat){
            if dot3(sub(a,predicted),sub(a,predicted))>dot3(sub(b,predicted),sub(b,predicted)){b}else{a}
        }else{return Ok(None);};
    if dot3(sub(point,threat),sub(point,threat))<min_radius{
        point=add(point,scale(unit(sub(protectee,point)),min_radius));
    }
    Ok(Some(point))
}
