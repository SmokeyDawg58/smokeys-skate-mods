//! Original hudminimap.apt and MiniMap::Show826bcc50.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;

pub(super) struct MiniMapMovie {
    pub root: Clip,
    visible: bool,
    no_skate: bool,
    pub sounds: Vec<&'static str>,
}
impl MiniMapMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        Ok(Self {root:Clip::new(assets,26,0)?,visible:false,no_skate:false,sounds:Vec::new()})
    }
    pub fn show(&mut self, assets: &Assets, visible: bool) -> RenderResult<()> {
        if self.visible != visible {
            self.visible=visible;
            self.root.goto(assets,if visible {"hudintro"} else {"hudoutro"})?;
        }
        Ok(())
    }
    pub fn no_skate_zone(&mut self, assets:&Assets, enabled:bool)->RenderResult<()> {
        if self.no_skate != enabled {
            self.no_skate=enabled;
            self.root.named(&["mMiniMap"])?.goto(assets,if enabled {"mongocorp"} else {"normal"})?;
        }
        Ok(())
    }
    pub fn tick(&mut self, assets:&Assets)->RenderResult<()> {
        self.root.tick(assets)?;
        self.root.take_calls(&mut self.sounds);
        Ok(())
    }
}
pub(super) fn action(clip:&mut Clip,offset:usize)->RenderResult<()> {
    match offset {
        0x2de8=>{clip.visible=false;clip.playback.stop();},
        0x2e18=>{}, // Verified class/method definitions, represented above.
        0x3018=>{clip.visible=true;clip.calls.push("PlayMiniMapTrans");},
        0x3048=>clip.calls.push("PlayMiniMapTrans"),
        0x3068=>{clip.visible=false;clip.playback.stop();},
        _=>return Err(format!("untranslated minimap action {offset:#x}").into()),
    }
    Ok(())
}

impl super::MarkerUi {
    pub(super) fn draw_minimap(&self,size:(u32,u32),matrix:skate2_formats::apt::Matrix,
        batches:&mut Vec<crate::gpu_renderer::marker_ui::Batch>)->RenderResult<()> {
        use super::{flatten_projected,Colour,quad,concatenate};
        use crate::gpu_renderer::marker_ui::Batch;
        if !self.minimap.root.visible {return Ok(());}
        let start=batches.len();
        // Flatten to original stage coordinates, then apply the screen's
        // FocusRight82608748 to ALL vertices, including native custom icons.
        flatten_projected(&self.assets,&self.minimap.root,matrix,Colour::default(),
            (1280,720),batches,true,None)?;
        let component=self.minimap.root.children.values().find(|child|
            child.clip.properties.get("_type").map(String::as_str)==Some("MiniMapComponent"))
            .ok_or("minimap custom component absent")?;
        if component.clip.visible {
            let transform=concatenate(matrix,component.entry.matrix);
            for icon in self.minimap_icons.iter().chain(&self.minimap_frame.dynamic_icons) {
                let Some((offset,alpha))=skate2_core::ui::minimap::placement(icon,
                    self.minimap_frame.position,self.minimap_heading) else {continue};
                // An unregistered texture draws the native's "missingtexture" (0x8262ea90),
                // which none of the loaded arenas holds.
                let Ok(texture) = self.assets.texture(&icon.texture) else {
                    continue;
                };
                let (sin,cos)=skate2_core::ui::minimap::sin_cos(icon.rotation);
                let icon_matrix=skate2_formats::apt::Matrix {
                    a:cos,b:-sin,c:sin,d:cos,tx:offset[0],ty:offset[1],
                };
                let half=16.*icon.scale; // kFEMapEventIconSize32, AddIcon scale1
                batches.push(Batch {texture:Some(texture.name.clone()),wrapped:false,
                    vertices:quad(concatenate(transform,icon_matrix),[-half,-half,half,half],
                        [0.,0.,1.,1.],[1.,1.,1.,alpha],(1280,720),true)});
            }
        }
        super::project_focus_right(size,&mut batches[start..]);
        Ok(())
    }
}
