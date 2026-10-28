//! LooseWorldPresentation eTrickGuide827e3510 / location827d0d90.
use crate::{gpu_renderer::{GpuRenderer,GpuScene,RenderResult,CameraRelativeClip,
    character::CharacterGhostState,character_palette::SelectedCharacterGpuBinding},
    render_inspection::Inspection,render_environment::EnvironmentLighting,
    render_window::RenderWindowClient,selected_character_material};
use skate2_formats::mount::MountSet;
use std::time::Instant;
pub(super) struct Room {
    room:Inspection,
    bindings:Vec<SelectedCharacterGpuBinding>,
    lighting:crate::render_irradiance::SkaterLighting,
    previous:Instant,
}
impl Room {
    pub fn load(mounts:&MountSet,now:Instant)->RenderResult<Self> {
        let assets=crate::edit_skater_room::EditSkaterRoom::load_paths(mounts,
            "data/content/world/models/DIST_trickguide_Textures.rx2","data/content/world/models/DIST_trickguide.rx2")?;
        let room=Inspection::from_assets(mounts,assets.scene,assets.textures,
            EnvironmentLighting::load_location(mounts,0x6568_f346_4306_6684)?)?;
        let lighting=crate::render_irradiance::SkaterLighting::new(room.environment().default_irradiance);
        Ok(Self {room,bindings:vec![],lighting,previous:now})
    }
    pub fn initialize(&mut self,renderer:&GpuRenderer)->RenderResult<GpuScene> {self.room.initialize(renderer)}
    pub fn update(&mut self,renderer:&GpuRenderer,scene:&mut GpuScene,mounts:&MountSet,
        now:Instant,view:&skate2_core::TrickGuideView<'_>)->RenderResult<()> {
        let character=view.characters.first().ok_or("Trick Guide preview has no renderable")?;
        let clip=super::window_clip_from_world(view.clip,renderer.dimensions());
        self.room.set_loose_world_view(clip,view.position,view.axes,crate::render_shadows::ShadowCasterInputs {
            camera_position:view.position,camera_forward:view.axes[2],skater_hips:character.hips_position,
            skater_bounds:character.world_bounds},self.room.environment().csm);
        self.room.frame(renderer,scene,now)?;
        if let Some((_,postfx))=scene.postfx.as_mut() {
            postfx.camera_planes=Some(view.planes);
            if let Some(ice)=view.postfx {super::ice_postfx(postfx,&ice,0.,view.planes);}
        }
        let dt=now.saturating_duration_since(self.previous).as_secs_f32();self.previous=now;
        self.lighting.update_frontend(self.room.light_probe_at(character.hips_position),dt);
        let camera=CameraRelativeClip::new(&clip,view.position);
        let ghost=CharacterGhostState {opacity:1.,ghosted:false};
        let materials=character.materials.ok_or("Trick Guide materials absent")?;
        if self.bindings.is_empty() {
            let created=selected_character_material::create_objects(renderer,mounts,character.palette,materials,&camera,ghost,
                self.room.environment(),self.room.frame_environment(),self.lighting.coefficients)?;
            scene.objects.clear();for (binding,object) in created {self.bindings.push(binding);scene.objects.push(object);}
        } else {
            for (binding,object) in self.bindings.iter().zip(&mut scene.objects) {
                selected_character_material::update_object(renderer,binding,object,character.palette,materials,&camera,ghost,
                    self.room.environment(),self.room.frame_environment(),self.lighting.coefficients)?;
            }
        }Ok(())
    }
}
