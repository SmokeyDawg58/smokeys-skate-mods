//! Authored, installed main-world destinations for the selected free-roam Map.
use crate::{attribute_runtime::AttributeRuntimeVault, ui::map_menu::{MapCategory, MapLocation}};
use skate2_formats::location::LocationTable;
use std::sync::Arc;

pub(crate) const LOCATION_CLASS: u64 = 0xddcd_6bc2_3255_e508;
pub(crate) const GROUP_CLASS: u64 = 0xd897_e7db_651e_58dc;
pub(crate) const ICON_CLASS: u64 = 0xf788_504a_8d92_2ced;
pub(crate) const DISTRICT_CLASS: u64 = 0x79d0_2084_0118_7a51;
const LOCATIONS: u64 = 0xf1c6_d5fa_4a23_951e;

#[derive(Default)]
pub(crate) struct MapCatalog {
    pub(crate) locations: Vec<MapLocation>,
    pub(crate) minimap: Vec<crate::ui::minimap::Icon>,
    transforms: Vec<(u64, [f32; 16])>,
}

impl MapCatalog {
    /// FEStuffMenu::PopulateMenus8263e170 / PopulateMenuItemsFromLocationGroup
    /// 82632cc0, FELocation GetName82615b68/GetImage82615950 and
    /// MapBackEndManager::TeleportToFELocation82565350.
    pub(crate) fn load(vault: &Arc<AttributeRuntimeVault>, table: &LocationTable) -> Result<Self, String> {
        let mut result = Self::default();
        for (group, category) in [
            (0x0229_d702_2a15_e01e, MapCategory::Landmarks),
            (0x10b1_6dc9_c4fe_004f, MapCategory::Plazas),
            (0x1757_7ab7_dec3_54e2, MapCategory::Skateparks),
            (0xb632_afae_7d12_0e40, MapCategory::SpawnPoints),
        ] {
            let group = vault.instance(GROUP_CLASS, group);
            let icon_key = group.try_ref_spec_collection(0x7862_55b8_ff42_6a3d)
                .map_err(|e| e.to_string())?.ok_or("Map group icon reference missing")?;
            let icon_instance = vault.instance(ICON_CLASS, icon_key);
            let visible = icon_instance.try_cached_layout_field(4, 1).map_err(|e| e.to_string())?
                .is_some_and(|value| value.bytes()[0] != 0);
            let maximum_range = f32::from_bits(icon_instance.try_scalar_word(0x2cf2_5b33_b439_ddcd, 0)
                .map_err(|e| e.to_string())?.unwrap_or(0));
            let icon = icon_instance.try_cached_layout_text(0)
                .map_err(|e| e.to_string())?.ok_or("Map icon name missing")?;
            let icon = String::from_utf8(icon.bytes().to_vec()).map_err(|e| e.to_string())?;
            let members = group.try_reference_members(LOCATIONS, false)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "Map location group has no Locations array".to_owned())?;
            for index in 0..members.len() {
                let member = members.get(index).expect("bounded reference index");
                let key = u64::from_be_bytes(member[8..16].try_into().expect("RefSpec collection"));
                let location = vault.instance(LOCATION_CLASS, key);
                if location.is_null() { return Err(format!("Map location {key:016x} is absent")); }
                let text = |offset| -> Result<String, String> {
                    let bytes = location.try_cached_layout_text(offset).map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("Map location {key:016x} missing text +{offset}"))?;
                    String::from_utf8(bytes.bytes().to_vec()).map_err(|e| e.to_string())
                };
                let spawn_name = text(4)?;
                // Additional worlds/DLC without a BAM transform are not installed
                // main-world travel targets. No coordinates or unlocks are invented.
                let Some(spawn) = table.find(&spawn_name) else { continue; };
                result.locations.push(MapLocation {
                    key,
                    localization_key: text(8)?,
                    category,
                    world_position: [spawn.transform[12], spawn.transform[14]],
                    image: text(12)?,
                    icon: format!("{icon}.Texture"),
                });
                result.transforms.push((key, spawn.transform));
                // MapBackEndManager::AddLocationIcon82577850 reuses this same
                // group icon, authored location and installed BAM transform.
                // Its zero-X/Z sentinel suppresses only the icon, not the
                // already admitted travel destination.
                if spawn.transform[12] == 0. && spawn.transform[14] == 0. { continue; }
                result.minimap.push(crate::ui::minimap::Icon {
                    texture: format!("{icon}.Texture"),
                    position: [spawn.transform[12], spawn.transform[14]],
                    rotation: 0., scale: 1., visible, fade_at_edge: true,
                    maximum_range, pinch_point: false,
                });
            }
        }
        if result.locations.is_empty() { return Err("Map has no installed main-world destinations".to_owned()); }
        Ok(result)
    }

    pub(crate) fn transform(&self, key: u64) -> Option<[f32; 16]> {
        self.transforms.iter().find(|(id, _)| *id == key).map(|(_, transform)| *transform)
    }
}
