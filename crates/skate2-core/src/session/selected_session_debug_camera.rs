//! Smokey's opt-in free camera. Camera movement uses the bundled Skate 3
//! ManualCam implementation; teleport goes through the normal player UI queue.
use super::{SelectedLiveError, SelectedLiveSession, SelectedVisualSnapshot};
use crate::{input_pad::XboxPadSample, nis::manager::{FrozenFocus,NisCamera}, ui::debug_camera_keybinds};
use skate3_core::{camera::{CameraFrame,ManualCam,ManualCamSettings}, input::gameplay_map::GameplayActions, math::Basis3};

#[path = "selected_session_debug_camera_controls.rs"]
mod controls;

#[derive(Default)]
pub(super) struct DebugCamera { cam: ManualCam, buttons: u16, releasing: bool, arming: bool, controls: controls::Controls }

impl<T,C> SelectedLiveSession<T,C> {
    pub(super) fn debug_camera_holds_input(&self)->bool { self.debug_camera.cam.active || self.debug_camera.releasing }

    /// Called only in ordinary free roam, before the gameplay input is sent.
    pub(super) fn try_open_debug_camera(&mut self,pad:Option<XboxPadSample>,allowed:bool)->bool {
        let buttons=pad.map_or(0,|p|p.buttons);
        let chord=debug_camera_keybinds::current().chord();
        let hit=buttons & chord==chord && self.debug_camera.buttons & chord!=chord;
        self.debug_camera.buttons=buttons;
        if !allowed || !hit || self.debug_camera_holds_input() {return false;}
        let Some(m)=self.game_camera_matrix() else {return false;};
        let basis=Basis3{columns:[[m[0],m[1],m[2]],[m[4],m[5],m[6]],[m[8],m[9],m[10]]]};
        let p=[m[12],m[13],m[14],1.0];
        let frame=CameraFrame{basis,position:p,previous_basis:basis,previous_position:p,linear_velocity:[0.;4],angular_velocity:[0.;4],shake_translation:[0.;4],discontinuity:false,field_of_view_degrees:55.,opacity:1.,blur:0.};
        self.debug_camera.cam.set_manual_cam_mode(true,Some(&frame));
        self.debug_camera.arming=true;
        self.debug_camera.controls.reset_boost();
        true
    }

    pub(super) fn update_debug_camera(&mut self,pad:Option<XboxPadSample>,dt:f32)->Result<(),SelectedLiveError> {
        let Some(pad)=pad else { return Ok(()); };
        let bindings=debug_camera_keybinds::current();
        let prev=self.debug_camera.buttons;
        self.debug_camera.buttons=pad.buttons;
        if self.debug_camera.cam.active {
            // Let the opening chord go before treating the D-pad as camera input.
            if self.debug_camera.arming {
                if pad.buttons & bindings.chord()==0 { self.debug_camera.arming=false; }
                return Ok(());
            }
            let toggle=pad.buttons & bindings.chord()==bindings.chord() && prev & bindings.chord()!=bindings.chord();
            let teleport=pad.buttons & debug_camera_keybinds::TELEPORT!=0 && prev & debug_camera_keybinds::TELEPORT==0;
            if toggle || teleport {
                let spawn=self.debug_camera.cam.spawn_matrix();
                self.debug_camera.cam.set_manual_cam_mode(false,None);
                self.debug_camera.releasing=true;
                if teleport { self.teleport_local_player(std::array::from_fn(|i|spawn[i/4][i%4]))?; }
            } else {
                let axis=|v:i16| (f32::from(v)/32767.).clamp(-1.,1.);
                let input=self.debug_camera.controls.advance(pad.buttons,pad.right_trigger,dt);
                let actions=GameplayActions::from_values([axis(pad.left_x),axis(pad.left_y),input.fast,axis(pad.right_x),axis(pad.right_y),0.,input.down,input.up,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.]);
                let settings=ManualCamSettings { fast_multiplier:input.fast_multiplier, vertical_speed:input.vertical_speed, ..ManualCamSettings::default() };
                self.debug_camera.cam.update_manual_cam(&actions,settings,dt);
            }
        } else if self.debug_camera.releasing && pad.buttons==0 && pad.left_trigger<13 && pad.right_trigger<13 && [pad.left_x,pad.left_y,pad.right_x,pad.right_y].iter().all(|v|i32::from(*v).abs()<5900) {
            self.debug_camera.releasing=false;
        }
        Ok(())
    }

    pub(super) fn debug_camera_active(&self)->bool { self.debug_camera.cam.active }
    pub(super) fn debug_camera_visual(&self)->Result<SelectedVisualSnapshot<'_>,SelectedLiveError> {
        let rows=self.debug_camera.cam.spawn_matrix();
        let matrix=std::array::from_fn(|i|rows[i/4][i%4]);
        let mut visual=self.visual_snapshot(Some(NisCamera{matrix:Some(matrix),fov:self.debug_camera.controls.fov(),near_plane:0.2}),Some(FrozenFocus::At([matrix[12],matrix[13],matrix[14]])))?;
        visual.debug_camera_override=true;
        Ok(visual)
    }
}
