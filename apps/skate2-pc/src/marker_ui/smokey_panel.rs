//! Compact mod news on the main menu, using the existing game font atlas.
use super::{assets::Assets, Batch, PanelPainter, panel_text};
use crate::gpu_renderer::RenderResult;

pub(super) fn draw(assets:&Assets, size:(u32,u32), version:&str, status:&str, batches:&mut Vec<Batch>) -> RenderResult<()> {
    let font=assets.font_by_file("helvetica_bold")?;
    let atlas=assets.textures.get(&font.texture).ok_or("mod panel font atlas missing")?;
    let painter=||PanelPainter {size:(1280.,720.),vertices:Vec::new()};
    let (mut shape,mut text)=(painter(),painter());
    let dark=[0.04,0.025,0.045,0.88];let pink=[0.93,0.04,0.68,1.];
    let cream=[0.96,0.92,0.94,1.];let muted=[0.75,0.70,0.74,1.];
    let edge=[0.32,0.20,0.30,0.86];
    shape.rect(966.,156.,1238.,370.,edge,edge);
    shape.rect(967.,157.,1237.,369.,dark,dark);
    shape.rect(967.,157.,971.,369.,pink,pink);
    shape.rect(981.,214.,1223.,215.,edge,edge);
    shape.rect(981.,287.,1223.,288.,edge,edge);
    for (line,x,y,h,color) in [
        ("SMOKEY'S MODS",982.,172.,16.,cream),
        (version,982.,197.,11.,muted),
        (status,1104.,197.,11.,pink),
        ("LATEST UPDATE",982.,228.,10.,pink),
        ("Debug camera & park editor",982.,246.,12.,cream),
        ("Physics & difficulty settings",982.,264.,12.,cream),
        ("TIP",982.,302.,10.,pink),
        ("Debug Camera: Mods > Keybinds",982.,320.,11.,muted),
        ("Press Y for more",982.,343.,10.,muted),
    ] {panel_text(&mut text,font,atlas,line,x,y,h,color);}
    let aspect=size.0 as f32/size.1.max(1) as f32;
    let scale=[((16./9.)/aspect).min(1.),(aspect/(16./9.)).min(1.)];
    for painter in [&mut shape,&mut text] {for v in &mut painter.vertices {v[0]*=scale[0];v[1]*=scale[1];}}
    batches.push(Batch {texture:None,wrapped:false,vertices:shape.vertices});
    batches.push(Batch {texture:Some(font.texture.clone()),wrapped:false,vertices:text.vertices});
    Ok(())
}
