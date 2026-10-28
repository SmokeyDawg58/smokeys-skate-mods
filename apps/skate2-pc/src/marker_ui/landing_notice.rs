//! Original Skate 3 Clean/Sketchy lettering and timeline beside S2's score.
//! The external build input contains original glyph pixels and APT transforms.
use super::{Batch,Clip,Matrix,RenderResult,concatenate,vertex};
use skate2_core::{texture::NativeTexture,ui::trick_backend::{LandingGrade,PhysicsFrame}};
use skate2_formats::material::{LinearTextureLevel,TextureEndian,TextureType,PIXEL_FORMAT_A8R8G8B8};
use std::sync::{Arc,OnceLock};
const TEXTURE: &str = "skate3-landing-notifier";
const INPUT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"),"/skate3-landing-notifier.bin"));
struct Data {width:u32,height:u32,clean:Vec<Vec<[f32;20]>>,sketchy:Vec<Vec<[f32;20]>>,origins:[[f32;2];2],rgba:Vec<u8>}
// The imported timelines use different original placements (Sketchy is about
// 125 stage units farther right). Remove that placement once per timeline,
// keeping the original frame-to-frame glyph animation and colours intact.
fn timeline_origin(frames:&[Vec<[f32;20]>])->[f32;2]{
    let mut bounds=[f32::INFINITY,f32::INFINITY,f32::NEG_INFINITY,f32::NEG_INFINITY];
    for q in frames.iter().flatten().filter(|q|q[19]>0.) {
        for p in [0,4,8,12] {
            bounds[0]=bounds[0].min(q[p]);bounds[1]=bounds[1].min(q[p+1]);
            bounds[2]=bounds[2].max(q[p]);bounds[3]=bounds[3].max(q[p+1]);
        }
    }
    if bounds.iter().all(|v|v.is_finite()){[bounds[0],(bounds[1]+bounds[3])*0.5]}else{[0.;2]}
}
fn align_beside(vertices:&mut [[f32;10]],anchor:[f32;4],size:(u32,u32)){
    if vertices.is_empty(){return;}
    let bounds=vertices.iter().fold([f32::INFINITY,f32::INFINITY,f32::NEG_INFINITY,f32::NEG_INFINITY],|mut b,v|{
        let p=[v[0]/v[3],v[1]/v[3]];for i in 0..2{b[i]=b[i].min(p[i]);b[i+2]=b[i+2].max(p[i]);}b});
    // Align in projected coordinates: the HUD's perspective also shears Y with X.
    let shift=[anchor[2]+16./size.0.max(1) as f32-bounds[0],(anchor[1]+anchor[3]-bounds[1]-bounds[3])*0.5];
    for v in vertices.iter_mut(){v[0]+=shift[0]*v[3];v[1]+=shift[1]*v[3];}
}
fn keep_on_screen(vertices:&mut [[f32;10]],size:(u32,u32)){
    if vertices.is_empty(){return;}
    let margin=[16./size.0.max(1) as f32,16./size.1.max(1) as f32];
    for axis in 0..2 {
        let (min,max)=vertices.iter().map(|v|v[axis]/v[3])
            .fold((f32::INFINITY,f32::NEG_INFINITY),|(a,b),p|(a.min(p),b.max(p)));
        let low=-1.+margin[axis];let high=1.-margin[axis];
        let shift=if min<low {low-min}else if max>high {high-max}else{0.};
        for v in vertices.iter_mut(){v[axis]+=shift*v[3];}
    }
}
struct Reader<'a> {bytes:&'a [u8],at:usize}
impl Reader<'_> {
    fn take(&mut self,n:usize)->Result<&[u8],String>{
        let end=self.at.checked_add(n).ok_or("Landing HUD size overflow")?;
        let value=self.bytes.get(self.at..end).ok_or("Truncated landing HUD")?;
        self.at=end;Ok(value)
    }
    fn word(&mut self)->Result<u32,String>{Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))}
    fn frames(&mut self,count:u32)->Result<Vec<Vec<[f32;20]>>,String>{
        if count>120{return Err("Landing HUD frame limit".into());}
        (0..count).map(|_|{
            let count=self.word()?;if count>64{return Err("Landing HUD glyph limit".into());}
            (0..count).map(|_|{
                let mut quad=[0.;20];
                for value in &mut quad {*value=f32::from_le_bytes(self.take(4)?.try_into().unwrap());
                    if !value.is_finite(){return Err("Nonfinite landing HUD geometry".into());}}
                Ok(quad)
            }).collect()
        }).collect()
    }
}
fn decode(bytes:&[u8])->Result<Data,String>{
    let mut r=Reader{bytes,at:0};if r.take(8)?!=b"S3LN0001"{return Err("Landing HUD format".into());}
    let width=r.word()?;let height=r.word()?;
    if width==0||height==0||width>1024||height>1024{return Err("Landing HUD atlas dimensions".into());}
    let clean_count=r.word()?;let sketchy_count=r.word()?;
    let clean=r.frames(clean_count)?;let sketchy=r.frames(sketchy_count)?;
    let rgba=r.take((width*height*4) as usize)?.to_vec();
    if r.at!=bytes.len(){return Err("Trailing landing HUD data".into());}
    let origins=[timeline_origin(&clean),timeline_origin(&sketchy)];
    Ok(Data{width,height,clean,sketchy,origins,rgba})
}
fn preview_grade()->Option<LandingGrade>{
    static PREVIEW:OnceLock<Option<LandingGrade>>=OnceLock::new();
    *PREVIEW.get_or_init(||match std::env::var("SKATE2_DIAGNOSTIC_LANDING_NOTICE").as_deref(){
        Ok("clean")=>Some(LandingGrade::Clean),Ok("sketchy")=>Some(LandingGrade::Sketchy),_=>None})
}
pub(super) struct LandingNotice {data:Option<Arc<Data>>,active:Option<(LandingGrade,usize)>}
impl LandingNotice {
    pub fn new()->RenderResult<Self>{
        static DATA:OnceLock<Result<Option<Arc<Data>>,String>>=OnceLock::new();
        let data=DATA.get_or_init(||if INPUT.is_empty(){Ok(None)}else{decode(INPUT).map(|data|Some(Arc::new(data)))});
        Ok(Self{data:data.clone().map_err(|e|->Box<dyn std::error::Error>{e.into()})?,active:None})
    }
    pub fn texture(&self)->Option<NativeTexture>{
        let data=self.data.as_ref()?;let(width,height)=(data.width,data.height);
        let pixels=data.rgba.chunks_exact(4).flat_map(|p|[p[2],p[1],p[0],p[3]]).collect();
        Some(NativeTexture{dictionary_index:0,guid:0,name:TEXTURE.into(),texture_type:TextureType::TwoDimensional,endian:TextureEndian::None,
            width,height,depth:1,mip_levels:1,pixel_format:PIXEL_FORMAT_A8R8G8B8,packed_mips:false,mip_data_offset:0,encoded_data:Vec::new(),
            linear_levels:vec![LinearTextureLevel{level:0,face:0,width,height,width_in_blocks:width,height_in_blocks:height,bytes_per_block:4,row_pitch:width*4,data:pixels}]})
    }
    pub fn observe(&mut self,frame:&PhysicsFrame,shown:bool){
        if !shown||frame.just_wiped_out||frame.interrupted||frame.reactivated {self.active=None;return;}
        if let Some(grade)=frame.skate3_landing_grade {
            self.active=Some((grade,0));
            skate2_core::diagnostics::test_log::event("skate3_landing_notice",format_args!("grade={grade:?} finished={} hud={}",frame.finished_sequence_score,frame.hud_score));
        }
    }
    pub fn tick(&mut self){
        if let (Some(data),Some((grade,frame)))=(&self.data,self.active){
            let length=match grade{LandingGrade::Clean=>data.clean.len(),LandingGrade::Sketchy=>data.sketchy.len()};
            self.active=if frame+1<length{Some((grade,frame+1))}else{None};
        }
    }
    pub fn shown(&self)->bool{
        self.data.is_some() && (self.active.is_some() || preview_grade().is_some())
    }
    pub fn draw(&self,root:&Clip,matrix:Matrix,size:(u32,u32),batches:&mut Vec<Batch>){
        self.draw_at(root,matrix,size,None,batches);
    }
    pub fn draw_at(&self,root:&Clip,matrix:Matrix,size:(u32,u32),anchor:Option<[f32;4]>,batches:&mut Vec<Batch>){
        let Some(data)=&self.data else{return;};
        // Explicit visual-only diagnostic. It never produces a landing or score.
        let Some((grade,index))=self.active.or_else(||preview_grade().map(|g|(g,12))) else{return;};
        let frames=match grade{LandingGrade::Clean=>&data.clean,LandingGrade::Sketchy=>&data.sketchy};
        let Some(quads)=frames.get(index) else{return;};
        if !root.visible{return;}
        // Share the multiplier's actual authored transform and aspect projection.
        // Keep the grade level with the multiplier, immediately to its right.
        let Some(multiplier)=root.children.values().find(|child|child.entry.name.as_deref()==Some("multiplier_mc")) else{return;};
        let matrix=concatenate(matrix,multiplier.entry.matrix);
        let origin=data.origins[usize::from(grade==LandingGrade::Sketchy)];
        let mut vertices=Vec::with_capacity(quads.len()*6);
        for quad in quads {
            let colour=[quad[16],quad[17],quad[18],quad[19]];
            for i in [0,1,2,0,2,3] {let p=i*4;vertices.push(vertex(matrix,
                [quad[p]-origin[0]+100.,quad[p+1]-origin[1]+4.],
                [quad[p+2],quad[p+3]],colour,size,false));}
        }
        if let Some(anchor)=anchor {align_beside(&mut vertices,anchor,size);}
        keep_on_screen(&mut vertices,size);
        if !vertices.is_empty(){batches.push(Batch{texture:Some(TEXTURE.into()),wrapped:false,vertices});}
    }
}
