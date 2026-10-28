//! PC filesystem adapter for SaveLoadManager's replay/photo requests. Native
//! auto-assigned slots, modern media exports and atomic completion.
use crate::{career_photos,gpu_renderer::RenderResult};
use std::{path::{Path,PathBuf},sync::Arc,io::Write};

#[derive(Clone)]
pub(crate) struct Request {pub slot:u32,pub replay:Option<Arc<[u8]>>,pub seconds:u32,pub location:String,
    pub video:skate2_core::ui::replay_editor::VideoSettings}
fn directory(replay:bool)->RenderResult<PathBuf> {
    let profile=skate2_core::edit_skater_profile::EditSkaterProfileStore::default_path()?;
    Ok(profile.parent().ok_or("profile has no directory")?.join("skate_reel").join(if replay {"replays"} else {"photos"}))
}
pub(crate) fn slots(replay:bool)->RenderResult<Vec<u32>> {
    let root=directory(replay)?;
    if !root.exists() {return Ok(Vec::new());}
    let mut slots=Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry=entry?;
        if entry.file_type()?.is_dir()
            && let Some(name)=entry.file_name().to_str()
            && name.len()==8
            && let Ok(id)=u32::from_str_radix(name,16)
            && id>0 {slots.push(id);}
    }
    slots.sort_unstable();Ok(slots)
}
/// AssignNextSaveIndexName825761b0: 60 replays, 100 photos; append after the
/// largest existing name, without recycling a deleted slot.
pub(crate) fn next_slot(replay:bool)->RenderResult<Option<u32>> {
    let slots=slots(replay)?;
    if slots.len()>=if replay {60} else {100} {return Ok(None);}
    Ok(Some(slots.last().copied().unwrap_or(0).checked_add(1).ok_or("media slot overflow")?))
}
fn write_new(path:&Path,data:&[u8])->RenderResult<()> {
    let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(data)?;file.sync_all()?;Ok(())
}
/// StateMemcardReplaySaveFlow82750568 / PhotoSaveFlow82750808: capture the
/// post-effect image before frontend, keep its thumbnail and report completion
/// only once every payload has reached the save directory.
pub(crate) fn save_photo(request:&Request,width:u32,height:u32,rgba:&[u8])->RenderResult<String> {
    if request.replay.is_some(){return Err("video must be rendered before its save completes".into());}
    let save=Save::begin(request,width,height,rgba)?;
    write_new(&save.pending.join("photo.jpg"),&career_photos::full_quality(width,height,rgba)?)?;
    Ok(save.commit()?.join("photo.jpg").display().to_string())
}

const FILES:&[&str]=&["thumbnail.jpg","metadata.bin","replay.sk2r","photo.jpg","replay.mp4","video.tmp.mp4","audio.f32le","encoder.log"];
struct Save {pending:PathBuf,target:PathBuf,committed:bool}
impl Save {
    fn begin(request:&Request,width:u32,height:u32,rgba:&[u8])->RenderResult<Self> {
        if width==0||height==0||rgba.len()!=width as usize*height as usize*4||request.slot==0 {return Err("invalid media capture".into());}
        let root=directory(request.replay.is_some())?;
        let target=root.join(format!("{:08x}",request.slot));
        if target.exists(){return Err("The selected media slot already exists.".into());}
        std::fs::create_dir_all(&root)?;
        let nonce=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
        let pending=root.join(format!(".pending-{}-{nonce}",std::process::id()));
        std::fs::create_dir(&pending)?;
        let save=Self{pending,target,committed:false};
        let thumbnail=career_photos::compress(160,120,&career_photos::thumbnail(width,height,rgba),15*1024)?;
        write_new(&save.pending.join("thumbnail.jpg"),&thumbnail)?;
        let mut metadata=request.seconds.to_be_bytes().to_vec();metadata.extend_from_slice(request.location.as_bytes());
        write_new(&save.pending.join("metadata.bin"),&metadata)?;
        if let Some(bytes)=&request.replay {write_new(&save.pending.join("replay.sk2r"),bytes)?;}
        Ok(save)
    }
    fn commit(mut self)->RenderResult<PathBuf> {
        if self.target.exists(){return Err("The media slot was occupied during export.".into());}
        std::fs::rename(&self.pending,&self.target)?;self.committed=true;Ok(self.target.clone())
    }
}
impl Drop for Save {
    fn drop(&mut self) {
        if !self.committed {
            // Only our exact files in this newly created pending directory;
            // never recursively remove a user save or unrelated file.
            for file in FILES {let _=std::fs::remove_file(self.pending.join(file));}
            let _=std::fs::remove_dir(&self.pending);
        }
    }
}
pub(crate) struct VideoSave {
    // Drop the process before cleaning its still-open temporary files.
    encoder:crate::media_encoder::Encoder,
    save:Option<Save>,
    pub finishing:bool,
    /// The 60 Hz steps pushed, each with its audio; every `stride`th is also a video frame.
    steps:u64,
    stride:u64,
}
impl VideoSave {
    pub fn begin(request:&Request,width:u32,height:u32,rgba:&[u8])->RenderResult<Self> {
        if request.replay.is_none(){return Err("replay recording absent".into());}
        let fps=request.video.fps;
        if fps==0||!crate::media_encoder::STEP_RATE.is_multiple_of(fps) {return Err("unsupported video frame rate".into());}
        let save=Save::begin(request,width,height,rgba)?;
        let encoder=crate::media_encoder::Encoder::new(&save.pending,width,height,fps)?;
        Ok(Self {encoder,save:Some(save),finishing:false,steps:0,stride:u64::from(crate::media_encoder::STEP_RATE/fps)})
    }
    pub fn step(&self)->u64 {self.steps}
    /// Whether the next step is one of the video's frames.
    pub fn encodes_step(&self)->bool {self.steps.is_multiple_of(self.stride)}
    pub fn push(&mut self,rgba:Option<&[u8]>,audio:&[f32])->RenderResult<()> {
        if let Some(rgba)=rgba {self.encoder.push_video(rgba)?;}
        self.encoder.push_audio(audio)?;self.steps+=1;Ok(())
    }
    pub fn finish_frames(&mut self)->RenderResult<()> {
        if !self.finishing {self.encoder.finish_frames()?;self.finishing=true;}Ok(())
    }
    pub fn poll(&mut self)->RenderResult<Option<String>> {
        if !self.finishing||!self.encoder.poll()? {return Ok(None);}
        let save=self.save.take().ok_or("video save already finished")?;
        // Keep the browser's duration in sync with the actual rendered movie.
        let metadata=save.pending.join("metadata.bin");
        let mut bytes=std::fs::read(&metadata)?;
        bytes[..4].copy_from_slice(&((self.steps as f64/f64::from(crate::media_encoder::STEP_RATE)).round() as u32).to_be_bytes());
        let mut file=std::fs::OpenOptions::new().write(true).truncate(true).open(metadata)?;
        file.write_all(&bytes)?;file.sync_all()?;drop(file);
        Ok(Some(save.commit()?.join("replay.mp4").display().to_string()))
    }
}
pub(crate) fn read_replay(slot:u32)->RenderResult<Vec<u8>> {
    Ok(std::fs::read(directory(true)?.join(format!("{slot:08x}")).join("replay.sk2r"))?)
}

pub(crate) fn items(replay:bool)->RenderResult<Vec<skate2_core::ui::replay_editor::FootageItem>> {
    let root=directory(replay)?;
    slots(replay)?.into_iter().map(|slot| {
        let bytes=std::fs::read(root.join(format!("{slot:08x}")).join("metadata.bin"))?;
        let seconds=u32::from_be_bytes(bytes.get(..4).ok_or("saved media metadata length")?.try_into()?);
        let location=String::from_utf8(bytes[4..].to_vec())?;
        Ok(skate2_core::ui::replay_editor::FootageItem {slot,seconds,location})
    }).collect()
}
/// Called only after the native No/Yes delete confirmation. Enumerated slot
/// number and exact file names prevent deleting unrelated profile contents.
pub(crate) fn delete(replay:bool,slot:u32)->RenderResult<()> {
    if !slots(replay)?.contains(&slot) {return Err("Saved media slot no longer exists.".into());}
    let root=directory(replay)?.canonicalize()?;
    let target=root.join(format!("{slot:08x}")).canonicalize()?;
    if target.parent()!=Some(root.as_path()) {return Err("Saved media slot is outside its media directory.".into());}
    let names:&[&str]=if replay {&["thumbnail.jpg","metadata.bin","replay.sk2r","replay.mp4"]} else {&["thumbnail.jpg","metadata.bin","photo.jpg"]};
    // Check the complete directory before deleting any of its files.
    for entry in std::fs::read_dir(&target)? {
        let entry=entry?;
        if !entry.file_type()?.is_file()||!names.iter().any(|n|entry.file_name()==*n) {
            return Err("Saved media slot contains an unrecognized file.".into());
        }
    }
    for name in names {
        match std::fs::remove_file(target.join(name)) {
            Ok(())=>(),
            Err(e) if e.kind()==std::io::ErrorKind::NotFound=>(), // Legacy replays have no MP4.
            Err(e)=>return Err(e.into()),
        }
    }
    std::fs::remove_dir(target)?;Ok(())
}
pub(crate) fn texture_key(replay:bool,slot:u32)->String {format!("skate_reel/{}/{slot}",if replay {"replay"} else {"photo"})}
pub(crate) fn thumbnail_texture(replay:bool,slot:u32)->RenderResult<skate2_core::texture::NativeTexture> {
    decode_texture(directory(replay)?.join(format!("{slot:08x}")).join("thumbnail.jpg"),texture_key(replay,slot))
}
pub(crate) fn photo_key(slot:u32)->String {format!("skate_reel/full_photo/{slot}")}
pub(crate) fn photo_texture(slot:u32)->RenderResult<skate2_core::texture::NativeTexture> {
    decode_texture(directory(false)?.join(format!("{slot:08x}")).join("photo.jpg"),photo_key(slot))
}
fn decode_texture(path:PathBuf,name:String)->RenderResult<skate2_core::texture::NativeTexture> {
    use skate2_core::texture::NativeTexture;
    use skate2_formats::material::{TextureType,TextureEndian,PIXEL_FORMAT_A8R8G8B8,LinearTextureLevel};
    let bytes=std::fs::read(path)?;
    let mut decoder=jpeg_decoder::Decoder::new(std::io::Cursor::new(bytes));let decoded=decoder.decode()?;
    let info=decoder.info().ok_or("Replay JPEG has no image info")?;
    if info.pixel_format!=jpeg_decoder::PixelFormat::RGB24 {return Err("Replay JPEG is not RGB".into());}
    let (width,height)=(u32::from(info.width),u32::from(info.height));
    let data=decoded.chunks_exact(3).flat_map(|p|[p[2],p[1],p[0],255]).collect();
    Ok(NativeTexture {dictionary_index:0,guid:0,name,texture_type:TextureType::TwoDimensional,endian:TextureEndian::None,
        width,height,depth:1,mip_levels:1,pixel_format:PIXEL_FORMAT_A8R8G8B8,packed_mips:false,mip_data_offset:0,encoded_data:Vec::new(),
        linear_levels:vec![LinearTextureLevel {level:0,face:0,width,height,width_in_blocks:width,height_in_blocks:height,bytes_per_block:4,row_pitch:width*4,data}]})
}
