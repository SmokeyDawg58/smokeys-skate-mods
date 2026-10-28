//! Original jacked/jacked_overlay, fingerprint f5a0ff8e, frontend screen68.
use super::{assets::Assets,clips::Clip};
use crate::gpu_renderer::RenderResult;
pub(super) struct JackedMovie {pub root:Clip,visible:bool}
impl JackedMovie{
    pub fn new(assets:&Assets)->RenderResult<Self>{
        let mut root=Clip::new(assets,13,0)?;
        root.playback.stop();root.visible=false;
        Ok(Self{root,visible:false})
    }
    pub fn update(&mut self,assets:&Assets,visible:bool)->RenderResult<()>{
        if visible==self.visible{return Ok(());}
        self.visible=visible;
        if visible{
            self.root=Clip::new(assets,13,0)?;
            super::map_movie::configure_button_items(self.root.named(&["mButtonHelp"])? ,assets,
                &[("A","ID_COMMON_CONTINUE")],0xffff_ffff)?;
            self.root.goto(assets,"intro")?;
        }else{self.root.goto(assets,"outro")?;}
        Ok(())
    }
    /// Frames 1 and 40 call `ScreenManager.EnableDimmer`, a method screenmanager.apt never
    /// defines; FE state 42's `mbDimmer` drives the dimmer.
    pub fn tick(&mut self,assets:&Assets)->RenderResult<()>{
        if !self.root.visible{return Ok(());}
        self.root.tick(assets)?;
        let mut calls=Vec::new();self.root.take_calls(&mut calls);
        for call in calls{match call{
            "JackedDimmerOn"|"JackedDimmerOff"|"JackedIntroComplete"=>{},
            "JackedOutroComplete"=>self.root.visible=false,
            _=>return Err(format!("untranslated Jacked callback {call}").into())}}
        Ok(())
    }
}
pub(super) fn action(clip:&mut Clip,offset:usize)->RenderResult<()>{
    match offset{
        0x12dc|0x1304=>{},
        0x1420=>clip.calls.push("JackedDimmerOn"),
        0x143c=>{clip.playback.stop();clip.calls.push("JackedIntroComplete");},
        0x1460=>clip.calls.push("JackedDimmerOff"),
        0x147c=>{clip.playback.stop();clip.calls.push("JackedOutroComplete");},
        _=>return Err(format!("untranslated Jacked action {offset:#x}").into())
    }Ok(())
}
