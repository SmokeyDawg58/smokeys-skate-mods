//! Bounded-memory H.264/AAC MP4 encoding. No shell, network service or screen
//! recording: each frame comes from the game's high-resolution render target.
use crate::gpu_renderer::RenderResult;
use std::{fs::File,io::{BufWriter,Write},path::{Path,PathBuf},process::{Child,ChildStdin,Command,Stdio}};

/// The export's step: one restored replay frame and its audio each 60th of a second.
pub(crate) const STEP_RATE:u32=60;
pub(crate) const AUDIO_FRAMES:usize=48_000/STEP_RATE as usize;
enum Stage { Frames, DrainingVideo, Muxing, Complete }
pub(crate) struct Encoder {
    executable:PathBuf,
    directory:PathBuf,
    child:Option<Child>,
    input:Option<ChildStdin>,
    audio:Option<BufWriter<File>>,
    stage:Stage,
    dimensions:(u32,u32),
    pub frames:u64,
}
fn executable()->RenderResult<PathBuf> {
    if let Some(path)=std::env::var_os("SKATE2_FFMPEG") {return Ok(path.into());}
    let name=if cfg!(windows){"ffmpeg.exe"}else{"ffmpeg"};
    if let Some(parent)=std::env::current_exe()?.parent() {
        let path=parent.join(name);if path.is_file(){return Ok(path);}
    }
    let profile=skate2_core::edit_skater_profile::EditSkaterProfileStore::default_path()?;
    if let Some(root)=profile.parent().and_then(Path::parent) {
        let path=root.join("tools/ffmpeg").join(name);if path.is_file(){return Ok(path);}
    }
    Ok(name.into()) // An installed encoder on PATH is also supported.
}
fn command(executable:&Path,directory:&Path)->RenderResult<Command> {
    let mut command=Command::new(executable);
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW.
    }
    let log=std::fs::OpenOptions::new().create(true).append(true).open(directory.join("encoder.log"))?;
    command.args(["-hide_banner","-loglevel","error","-nostdin","-n"]).stdout(Stdio::null()).stderr(log);
    Ok(command)
}
impl Encoder {
    pub fn new(directory:&Path,width:u32,height:u32,fps:u32)->RenderResult<Self> {
        let executable=executable()?;
        let mut command=command(&executable,directory)?;
        command.args(["-f","rawvideo","-pixel_format","rgba","-video_size",&format!("{width}x{height}"),
            "-framerate",&fps.to_string(),"-i","pipe:0","-an","-c:v","libx264","-preset","fast","-crf","18","-threads","2",
            "-vf","scale=in_range=full:out_range=limited:out_color_matrix=bt709,format=yuv420p",
            "-color_range","tv","-colorspace","bt709","-color_primaries","bt709","-color_trc","bt709","-g",&(fps*2).to_string()])
            .arg(directory.join("video.tmp.mp4")).stdin(Stdio::piped());
        let mut child=command.spawn().map_err(|e|format!("Could not start the MP4 encoder ({e}). Install FFmpeg with H.264/AAC support, or place ffmpeg.exe beside skate2-pc.exe."))?;
        let input=child.stdin.take();
        let mut encoder=Self {executable,directory:directory.into(),child:Some(child),input,audio:None,stage:Stage::Frames,dimensions:(width,height),frames:0};
        encoder.audio=Some(BufWriter::new(std::fs::OpenOptions::new().write(true).create_new(true).open(directory.join("audio.f32le"))?));
        Ok(encoder)
    }
    fn failure(&self,context:&str)->String {
        let log=std::fs::read_to_string(self.directory.join("encoder.log")).unwrap_or_default();
        let tail:Vec<_>=log.lines().rev().take(6).collect();
        format!("{context}\n{}",tail.into_iter().rev().collect::<Vec<_>>().join("\n"))
    }
    pub fn push_video(&mut self,rgba:&[u8])->RenderResult<()> {
        if !matches!(self.stage,Stage::Frames)||rgba.len()!=self.dimensions.0 as usize*self.dimensions.1 as usize*4 {
            return Err("invalid video export frame".into());
        }
        if let Err(e)=self.input.as_mut().ok_or("encoder input closed")?.write_all(rgba) {
            return Err(self.failure(&format!("Video encoding failed: {e}")).into());
        }
        self.frames+=1;Ok(())
    }
    /// One step's audio, which the mux joins as one continuous stream whatever the video rate.
    pub fn push_audio(&mut self,audio:&[f32])->RenderResult<()> {
        if !matches!(self.stage,Stage::Frames)||audio.len()!=AUDIO_FRAMES*2 {return Err("invalid audio export frame".into());}
        let bytes:Vec<_>=audio.iter().flat_map(|s|s.clamp(-1.,1.).to_le_bytes()).collect();
        self.audio.as_mut().ok_or("audio export closed")?.write_all(&bytes)?;Ok(())
    }
    pub fn finish_frames(&mut self)->RenderResult<()> {
        if !matches!(self.stage,Stage::Frames)||self.frames==0 {return Err("no video frames were rendered".into());}
        self.input.take(); // EOF lets x264 drain delayed frames.
        if let Some(mut audio)=self.audio.take() {audio.flush()?;audio.get_ref().sync_all()?;}
        self.stage=Stage::DrainingVideo;Ok(())
    }
    /// Nonblocking, including the final MP4 index pass. The UI can still cancel.
    pub fn poll(&mut self)->RenderResult<bool> {
        if matches!(self.stage,Stage::Frames){return Ok(false);}
        if matches!(self.stage,Stage::Complete){return Ok(true);}
        let Some(status)=self.child.as_mut().ok_or("encoder process absent")?.try_wait()? else {return Ok(false)};
        self.child.take();
        if !status.success(){return Err(self.failure(&format!("MP4 encoding failed ({status})")).into());}
        if matches!(self.stage,Stage::DrainingVideo) {
            let mut command=command(&self.executable,&self.directory)?;
            command.arg("-i").arg(self.directory.join("video.tmp.mp4"))
                .args(["-f","f32le","-ar","48000","-ac","2","-i"]).arg(self.directory.join("audio.f32le"))
                .args(["-map","0:v:0","-map","1:a:0","-c:v","copy","-c:a","aac","-b:a","192k","-movflags","+faststart"])
                .arg(self.directory.join("replay.mp4")).stdin(Stdio::null());
            self.child=Some(command.spawn()?);self.stage=Stage::Muxing;return Ok(false);
        }
        std::fs::OpenOptions::new().write(true).open(self.directory.join("replay.mp4"))?.sync_all()?;
        for file in ["video.tmp.mp4","audio.f32le","encoder.log"] {std::fs::remove_file(self.directory.join(file))?;}
        self.stage=Stage::Complete;Ok(true)
    }
}
impl Drop for Encoder {
    fn drop(&mut self) {
        if let Some(mut child)=self.child.take() {let _=child.kill();self.input.take();let _=child.wait();}
    }
}
