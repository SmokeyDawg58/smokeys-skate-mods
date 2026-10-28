//! Original hud2/bbtimer, native BigBlackTimerHUD states and authored placement.
use super::{assets::Assets,clips::Clip};
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::turn_hud::TurnHudView;
pub(super) const ASSET:usize=103;
pub(super) struct BigBlackTimerMovie{pub root:Clip,visible:Option<bool>}
impl BigBlackTimerMovie{
    pub fn new(assets:&Assets)->RenderResult<Self>{
        let mut root=Clip::new(assets,ASSET,0)?;
        root.named(&["mcTextTimer","mcTimer","labelText"])?.text=Some(assets.initial_text("#"));
        Ok(Self{root,visible:None})
    }
    pub fn tick(&mut self,assets:&Assets)->RenderResult<()>{self.root.tick(assets)}
    pub fn set_state(&mut self,assets:&Assets,state:i32,view:&TurnHudView)->RenderResult<()>{
        match state{
            1|2=>{
                let visible=state==1;
                if self.visible!=Some(visible){self.root.goto(assets,if visible{"hudintro"}else{"hudoutro"})?;self.visible=Some(visible);}
            },
            3=>{
                let TurnHudView::Timer(text)=view else{return Err("BigBlack timer state without data".into());};
                let text=assets.initial_text(&format!("#{}",super::hud_timer_movie::format_time(assets,text.minutes,text.seconds)?));
                let timer=self.root.named(&["mcTextTimer","mcTimer"])?;
                timer.named(&["labelText"])?.text=Some(text);
                // labelTextGlow is absent from authored character6; Flash's write is inert.
                // This movie does not read the colour element returned by GetData.
            },
            4=>self.root.named(&["mcTextTimer"])?.goto(assets,"flash")?,
            5=>self.root.named(&["mcTextTimer"])?.goto_and_stop(assets,"normal")?,
            _=>{},
        }Ok(())
    }
}
/// First-frame DoInitAction defines the guarded `bbtimer` class, whose
/// constructor and methods are supplied by BigBlackTimerMovie above.
/// AptCharacterAnimation::ExecuteInitAction829f0e20 selects it by sprite ID.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 8, 0x1d38))
}
pub(super) fn action(clip:&mut Clip,offset:usize)->RenderResult<()>{
    match offset{
        0x1d10=>clip.visible=false,
        0x1fa4=>clip.visible=true,
        0x1fcc=>{clip.visible=false;clip.playback.stop();},
        _=>return Err(format!("untranslated BigBlack timer action {offset:#x}").into()),
    }Ok(())
}
