//! FE Trick Guide's pause, loose-world scene and preview ownership.
use super::{SelectedLiveSession,SelectedLiveError,SelectedLiveStage,SelectedSceneCharacterVisual,
    career::CareerSession,scene_characters::{CareerSceneSource,SceneCharacterModels,recipe_model}};
use crate::{nis::{self,manager::NisSystem,directory::SceneDirectory},ui::{trick_guide, pause_menu::PauseMenuInput}};
use skate2_formats::ice_camera::CameraSet;
use std::sync::Arc;
fn error(e:impl std::fmt::Display)->SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::Visual,format!("Trick Guide: {e}"))
}
pub(super) struct TrickGuideSession {
    pub controller:trick_guide::Controller,
    scene:NisSystem,
    models:SceneCharacterModels,
    last_animation:String,
    change_timer:f32,
    intro:bool,
    fade_count:u32,
}
pub struct TrickGuideView<'a> {
    pub clip:[f32;16],
    pub position:[f32;3],
    pub axes:[[f32;3];3],
    pub planes:[f32;2],
    pub postfx:Option<nis::manager::IcePostfx>,
    pub characters:Vec<SelectedSceneCharacterVisual<'a>>,
}
impl<T,C> SelectedLiveSession<T,C> {
    pub(super) fn open_trick_guide(&mut self)->Result<(),SelectedLiveError> {
        let assets=self.inner.assets.as_ref().ok_or_else(||error("assets absent"))?;
        let mounts=&assets.mounts;
        let directory=SceneDirectory::parse(&mounts.read_path(nis::directory::PATH).map_err(error)?).map_err(error)?;
        let cameras=CameraSet::parse(&mounts.read_path(nis::camera::PATH).map_err(error)?).map_err(error)?;
        let source=Arc::new(CareerSceneSource::new(Arc::clone(mounts),nis::characters::twist_ratios(mounts).map_err(error)?));
        let recipe=if let Some(skater)=self.inner.local_party_skater().map_err(error)? {
            skater.recipe_load.recipe().map_err(error)?.as_ref().clone()
        } else {
            self.inner.player.as_ref().and_then(|p|p.selected.owners.recipe_load.as_ref())
                .and_then(|r|r.recipe().ok()).ok_or_else(||error("local recipe absent"))?.as_ref().clone()
        };
        source.set_local_player(Some(CareerSession::local_player_recipe(&recipe)));
        let mut scene=NisSystem::new(Arc::new(directory),Arc::new(nis::camera::IceCameras::new(Arc::new(cameras))),None,"English");
        scene.set_character_source(source);
        scene.start_trickbook().map_err(error)?;
        scene.set_trickbook_animation(trick_guide::DEFAULT_ANIMATION,1).map_err(error)?;
        let character=scene.scene_characters_mut().first_mut().ok_or_else(||error("preview character absent"))?;
        character.borrowed=false;
        character.category=if recipe.category==3 {"createacharacter"}else{"marquee"}.into();
        let model=recipe_model(character,recipe,mounts,&assets.vault,&assets.cvars,2).map_err(error)?;
        let mut models=SceneCharacterModels::with_model(character,model,2);
        models.present(scene.scene_characters_mut(),mounts,&assets.vault,&assets.cvars);
        let goofy=!self.career_stance_regular()?;
        self.trick_guide=Some(TrickGuideSession {controller:trick_guide::Controller::new(self.movie_attrib_db()?,goofy),scene,models,
            last_animation:trick_guide::DEFAULT_ANIMATION.into(),change_timer:60.,intro:false,fade_count:0});
        self.pause_menu.hide_for_replay();
        Ok(())
    }
    pub fn trick_guide_snapshot(&self)->Option<trick_guide::Snapshot> {self.trick_guide.as_ref().map(|g|g.controller.snapshot())}
    pub fn trick_guide_intro_complete(&mut self) {if let Some(g)=self.trick_guide.as_mut(){g.intro=true;}}
    pub(super) fn update_trick_guide(&mut self,input:PauseMenuInput,focused:bool)->Result<(),SelectedLiveError> {
        let g=self.trick_guide.as_mut().ok_or_else(||error("state absent"))?;
        // FrontEndManager::Update82640130 passes 16 to StateManager's update,
        // and OnUpdate_CC826605b0 decrements this only in Idle, while positive.
        if g.controller.ready && g.change_timer>0. {g.change_timer-=16.;}
        if g.fade_count<=10 {
            g.fade_count+=1;
            if g.fade_count>10 {self.frontend_fade.reveal();}
        }
        g.controller.ready=g.intro && g.fade_count>10;
        let exiting=g.controller.exit;
        g.controller.update(input,focused);
        if g.controller.exit && !exiting {self.frontend_fade.cover();}
        if g.controller.exit && self.frontend_fade.covered() {
            self.trick_guide=None;self.pause_menu.return_from_replay();self.frontend_fade.reveal();
        }
        Ok(())
    }
    /// Presentation time, independently of the paused simulation.
    pub fn trick_guide_render_frame(&mut self,dt:f32)->Result<Option<TrickGuideView<'_>>,SelectedLiveError> {
        let Some(g)=self.trick_guide.as_mut() else {return Ok(None)};
        if g.controller.ready && g.change_timer<=0. && g.last_animation!=g.controller.animation {
            g.scene.set_trickbook_animation(&g.controller.animation,g.controller.camera).map_err(error)?;
            g.last_animation=g.controller.animation.clone();g.change_timer=60.;
        }
        g.scene.update_trickbook(dt);
        let assets=self.inner.assets.as_ref().ok_or_else(||error("assets absent"))?;
        g.models.present(g.scene.scene_characters_mut(),&assets.mounts,&assets.vault,&assets.cvars);
        let view=g.scene.view();let camera=view.camera.ok_or_else(||error("camera absent"))?;
        let p=self.inner.world.as_ref().ok_or_else(||error("world absent"))?.local
            .try_ice_camera_projection(camera.matrix,camera.fov,camera.near_plane).map_err(error)?;
        let columns=p.camera_matrix.columns();
        Ok(Some(TrickGuideView {clip:p.clip_from_world,position:columns[3].xyz,axes:[columns[0].xyz,columns[1].xyz,columns[2].xyz],
            planes:[p.near_plane,p.far_plane],postfx:view.ice_postfx,characters:g.models.visuals()}))
    }
}
