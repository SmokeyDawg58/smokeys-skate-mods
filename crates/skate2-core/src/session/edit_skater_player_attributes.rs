//! Actor::ReloadCharacterData825f4690 and SkaterAnim::SetCharacterData82cae1d0.
//! Registered below actor_animation_construction to mutate the SAME fields.
use super::ActorAnimationConstruction;
use crate::{
    actor_construction::ActorConstruction, play_animation::FastString30,
    players_backend_settings::PlayersBackendSettings,
};

impl<K, W> ActorAnimationConstruction<K, W> {
    /// Preserves source null-pro behavior: no string write for Normal/default.
    pub(crate) fn try_set_character_data(
        &mut self,
        stance: u32,
        gestures: [u32; 4],
        pro: Option<FastString30>,
    ) -> Result<(), &'static str> {
        let player = self
            .player
            .as_mut()
            .ok_or("SkaterAnim player fields unavailable")?;
        player.natural_stance = stance;
        if stance != player.last_stance {
            player.flags_known_mask |= 0xc0000000;
            if player.relative_stance != 0 {
                if stance == 1 {
                    player.relative_stance = 0;
                    player.flags_bits &= 0x3fffffff;
                } else {
                    player.flags_bits |= 0xc0000000;
                    player.relative_stance = u32::from(stance != 0);
                }
            } else {
                player.relative_stance = 1;
                if stance == 0 {
                    player.flags_bits &= 0x3fffffff;
                } else {
                    player.flags_bits |= 0xc0000000;
                }
            }
            player.last_stance = player.natural_stance;
        }
        player.gestures = gestures;
        if let Some(pro) = pro {
            player.pro_skater = pro;
        }
        Ok(())
    }
}

/// Actual actor player ID is queried, including when caller applies ghost row8.
pub(crate) fn reload_character_data(
    actor: &ActorConstruction,
    settings: &PlayersBackendSettings,
) -> Result<(), String> {
    apply_character_data(actor, settings, false)
}

/// A new game's Actor normally constructs mProSkater="Default" at 82cc0ec8
/// before GetCACSettings. The host retains its first Actor across games;
/// restore that default when the new character supplies no pro/style string.
/// Ordinary ReloadCharacterData keeps the native null-string behavior.
pub(crate) fn reload_character_data_for_spawn(
    actor: &ActorConstruction,
    settings: &PlayersBackendSettings,
) -> Result<(), String> {
    apply_character_data(actor, settings, true)
}

fn apply_character_data(
    actor: &ActorConstruction,
    settings: &PlayersBackendSettings,
    spawning: bool,
) -> Result<(), String> {
    let player = actor
        .prefix()
        .map_err(|e| e.to_string())?
        .fields()
        .ok_or("Actor fields unavailable")?
        .player_index;
    let regular = settings
        .try_stance_byte(player)
        .map_err(|e| e.to_string())?
        != 0;
    let character = settings.try_character(player).map_err(|e| e.to_string())?;
    let mut gestures = [0, 1, 2, 3];
    let pro = match crate::actor_construction::animation_start::pro_skater_name(character) {
        Some(name) => Some(std::str::from_utf8(name).expect("native ASCII literal")),
        None if character == u64::MAX => {
            gestures = settings.try_gestures(player).map_err(|e| e.to_string())?;
            match settings.try_style(player).map_err(|e| e.to_string())? {
                0 => None,
                1 => Some("Loose"),
                2 => Some("Aggressive"),
                _ => return Err("invalid CAC animation style".into()),
            }
        }
        _ => None,
    };
    let pro = pro.or_else(|| spawning.then_some("Default"))
        .map(FastString30::encode)
        .transpose()
        .map_err(|e| format!("CAC style string: {e:?}"))?;
    let animation = actor
        .retained_animation()
        .map_err(|e| e.to_string())?
        .ok_or("Actor SkaterAnim unavailable")?;
    animation
        .lock()
        .map_err(|_| "Actor animation lock poisoned")?
        .try_set_character_data(u32::from(!regular), gestures, pro)
        .map_err(str::to_owned)?;
    Ok(())
}
