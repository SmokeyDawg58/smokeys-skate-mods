//! Optional replay-save branch of the existing controller regression. Use an
//! isolated SKATE2_SAVE_ROOT; all actions go through the actual editor input.
use super::PcSession;
use crate::gpu_renderer::RenderResult;
use skate2_core::{XboxPadSample,ui::replay_editor::Snapshot};

pub(super) struct Probe {
    pub enabled:bool,
    pub done:bool,
    stage:u8,
    ticks:u64,
    baseline:Option<[f32;3]>,
}
impl Default for Probe {
    fn default()->Self {Self {enabled:std::env::var_os("SKATE2_DIAGNOSTIC_REPLAY_MEDIA").is_some(),done:false,stage:0,ticks:0,baseline:None}}
}
impl Probe {
    fn next(&mut self) {self.stage+=1;self.ticks=0;}
    fn restored(&self,snapshot:&Snapshot)->RenderResult<()> {
        let now=[snapshot.parameter,snapshot.start_head,snapshot.end_head];
        if self.baseline.is_none_or(|old|old.into_iter().zip(now).any(|(a,b)|(a-b).abs()>0.00001)) {
            return Err(format!("Media export changed cursor or trim: {:?} -> {now:?}",self.baseline).into());
        }
        Ok(())
    }
    /// Returns the ordinary controller buttons and whether exit was requested.
    pub fn input(&mut self,session:&PcSession,editor:&Snapshot,sample:&mut XboxPadSample)->RenderResult<bool> {
        self.ticks+=1;
        if self.ticks>7200 {return Err(format!("Replay media test timed out at stage {}: {:?}",self.stage,editor.popup).into());}
        // One-frame presses, with release frames between every action.
        if !self.ticks.is_multiple_of(20) {return Ok(false);}
        if let Some(popup)=&editor.popup
            && popup.description.starts_with("#Choose the video's") {
            sample.buttons=0x1000;return Ok(false); // The selected 1080p / 60 FPS.
        }
        if let Some(popup)=&editor.popup {
            if popup.description.starts_with('#')&&!popup.description.starts_with("#Rendering")
                &&!popup.description.starts_with("#Finishing")&&!popup.description.starts_with("#Find your")
                &&popup.description!="#" {return Err(format!("Replay media notice: {} {}",popup.title,popup.description).into());}
        }
        match self.stage {
            0=>{
                self.baseline=Some([editor.parameter,editor.start_head,editor.end_head]);
                sample.buttons=0x4000;self.next(); // X: photo (Replay2 screen0).
            }
            1|6|10|14=>{
                let description=match self.stage {1=>"ID_REPLAY_EDITOR_POPUP_PHOTOSAVE",14=>"ID_REPLAY_EDITOR_QUIT",_=>"ID_REPLAY_EDITOR_POPUP_REPLAYSAVE"};
                let popup=editor.popup.as_ref().ok_or("Media confirmation absent")?;
                if popup.description!=description {return Err(format!("Wrong media confirmation: {}",popup.description).into());}
                if popup.selected!=1 {sample.buttons=0x0002;} else {
                    sample.buttons=0x1000;self.next();
                    if self.stage==15 {self.done=true;return Ok(true);}
                }
            }
            2=>{
                if editor.popup.as_ref().is_some_and(|p|p.title=="#Photo saved") {
                    self.restored(editor)?;
                    let slots=crate::replay_media::slots(false)?;
                    let slot=*slots.last().ok_or("No saved photo")?;
                    let image=crate::replay_media::photo_texture(slot)?;
                    if image.width!=1920||image.height!=1080 {return Err(format!("Wrong photo size: {}x{}",image.width,image.height).into());}
                    eprintln!("REPLAY_MEDIA_TEST photo saved: {}x{} slot={slot}",image.width,image.height);
                    sample.buttons=0x1000;self.next();
                }
            }
            3|8|12=>{
                if !session.replay_export_active()&&!editor.saving&&editor.popup.is_none() {
                    self.restored(editor)?;
                    if self.stage==8 {
                        if !crate::replay_media::slots(true)?.is_empty() {return Err("Cancelled export published a replay".into());}
                        eprintln!("REPLAY_MEDIA_TEST cancellation restored editor and published no slot");
                    }
                    sample.buttons=0x2000;self.next(); // B: editor menu.
                }
            }
            4|9|13=>{
                let option=if self.stage==13 {"ID_REPLAYEDITOR_EXIT"} else {"ID_REPLAYEDITOR_BASIC_SAVE_REPLAY"};
                let popup=editor.popup.as_ref().ok_or("Media editor menu absent")?;
                let index=popup.options.iter().position(|s|s==option).ok_or("Media menu option absent")?;
                if popup.selected!=index {sample.buttons=0x0002;} else {
                    sample.buttons=0x1000;
                    // Stage 5 is intentionally skipped: stage 6 confirms save.
                    if self.stage==4 {self.stage=5;}
                    self.next();
                }
            }
            7=>{
                if session.replay_export_active() {sample.buttons=0x2000;self.next();}
            }
            11=>{
                if editor.popup.as_ref().is_some_and(|p|p.title=="#Replay saved") {
                    self.restored(editor)?;
                    let slot=*crate::replay_media::slots(true)?.last().ok_or("No saved replay")?;
                    let archive=crate::replay_media::read_replay(slot)?;
                    if archive.is_empty(){return Err("Saved editable replay is empty".into());}
                    eprintln!("REPLAY_MEDIA_TEST MP4 and editable recording saved: slot={slot}, archive={} bytes",archive.len());
                    sample.buttons=0x1000;self.next();
                }
            }
            _=>(),
        }
        Ok(false)
    }
}
