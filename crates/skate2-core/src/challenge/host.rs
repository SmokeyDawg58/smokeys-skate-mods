//! Queries the challenge system makes of the running game outside the
//! per-frame player state: DMO locators and their collision against trigger
//! volumes.

/// The session's answers. [`NullHost`] answers for a system with no world.
pub trait Host: Send {
    /// `LWDynamicObjectLocatorMan::GetLocatorFromLuaID`: the locator's
    /// `mLocatorID` (`+0x80`).
    fn dmo_locator_from_lua_id(&mut self, lua_id: f32) -> Option<u64>;
    /// The collision test of `cChallengeDMOManager::IsDMOInVolume`
    /// (`0x825b7518`): the DMO's collision volume and transform against the
    /// trigger instance found by `cTriggerVolumeManager::GetTriggerInstance`.
    fn is_dmo_in_volume(&mut self, locator: u64, volume_id: f32) -> bool;
    /// `cTriggerVolumeManager::GetTriggerInstance` (`0x82e21cc0`) then
    /// `TriggerVolumeUtils::ComputeTriggerVolumeCenter` (`0x82e21438`): the volume's centre.
    fn trigger_volume_center(&mut self, volume_id: f32) -> Option<[f32; 3]>;
    /// VDWriter::AddRaceGate827fda90: the collision bounding volume's transform
    /// composed with the trigger instance, and the three union words at +0x44.
    fn race_gate_volume(&self, _volume_id: f32) -> Option<([f32; 16], [f32; 3])> {
        None
    }
    /// `cTriggerVolumeManager::GetTriggerInstance` (`0x82e21cc0`) and
    /// `TriggerVolumeUtils::GetTriggerBoundingVolume` (`0x82e20bb8`) of a crowd's volume: `None`
    /// while no instance of it is streamed in, `Some(None)` for an instance without a bounding
    /// volume; else the bounding volume's transform composed with the instance's and
    /// `ComputerTriggerVolumeRadius` (`0x82e21508`): a box's half extents, any other volume's
    /// radius on all three axes, W zero.
    fn crowd_volume(&self, _volume_id: f32) -> Option<Option<([f32; 16], [f32; 4])>> {
        None
    }
    /// `cLocationManager::GetLocationOrientation` (`0x82a8de48`): the orientation of the
    /// location registered under the hash, `None` when none is.
    fn location_orientation(&self, _location: u32) -> Option<[f32; 16]> {
        None
    }
    /// `NISManager::GetPhotoCount(sceneName)` (`0x828a0e70`): the scene's photo shots, 0
    /// without a NIS manager.
    fn nis_photo_count(&self, _scene: &str) -> i32 {
        0
    }
    /// `LocalizerManager::FormatString` of a HAL id without arguments, the format prefix left
    /// out; `None` without the language tables or the string.
    fn localized(&self, _label: &str) -> Option<String> {
        None
    }
}

/// No world: no locators, nothing collides.
#[derive(Debug, Default)]
pub struct NullHost;

impl Host for NullHost {
    fn dmo_locator_from_lua_id(&mut self, _: f32) -> Option<u64> {
        None
    }
    fn is_dmo_in_volume(&mut self, _: u64, _: f32) -> bool {
        false
    }
    fn trigger_volume_center(&mut self, _: f32) -> Option<[f32; 3]> {
        None
    }
}
