//! Native linked ReplayEditorTweaks; the frontend reads current targets.
use super::{InputCvarReferences,InputCvarError};
impl InputCvarReferences {
    pub(crate) fn try_replay_camera_collision(&self)->Result<(bool,f32),InputCvarError> {
        Ok((self.try_current_boolean(1272)?,self.try_current_float(1271)?))
    }
    pub(crate) fn try_replay_editor(&self) -> Result<crate::ui::replay_editor::Tuning,InputCvarError> {
        Ok(crate::ui::replay_editor::Tuning {
            marker_threshold:self.try_current_float(1259)?,scrub_max:self.try_current_float(1260)?,
            scrub_smoothing:self.try_current_float(1261)?,zoom_delta:self.try_current_float(1262)?,
            free_ls:self.try_current_float(1263)?,free_rs:self.try_current_float(1264)?,free_height:self.try_current_float(1265)?,
            orbit_ls:self.try_current_float(1266)?,orbit_rs:self.try_current_float(1267)?,orbit_height:self.try_current_float(1268)?,
            orbit_multiplier:self.try_current_float(1269)?,frame_step:self.try_current_boolean(1270)?,
        })
    }
}
