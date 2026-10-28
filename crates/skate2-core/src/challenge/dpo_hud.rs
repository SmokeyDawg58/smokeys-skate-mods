//! `Sk8::HUD::DPOHUDData`, created with a group that has the `dpo` component, and the `HUD2`
//! natives that write it. Each native finds its group's data with
//! `HUDGroup::GetHUDData<DPOHUDData>` (`0x82898690`), which asserts on a group without the
//! component (`HUDGroup.h` line 90) and then reads the bucket sentinel; such a group is left
//! alone here. Spot Battle's group has it: its cumulative locations set the turn score and the
//! best score (`turn_hud::Kind::Dpo` shows them).

use super::{
    binding::{get_float_or, get_int_or, get_string},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    system::World,
};

type R = LuaResult<Vec<Value>>;

fn none() -> R {
    Ok(Vec::new())
}

/// `DPOHUDData::kHUDName`.
pub const COMPONENT: &str = "dpo";

/// `eDPOFormat`.
pub mod format {
    pub const INTS: i32 = 0;
    pub const POINTS: i32 = 1;
    pub const TIME: i32 = 2;
    pub const DISTANCE: i32 = 3;
}

/// A parameter as `SetParams` formatted it: the value and the format type of the moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DpoParam {
    pub format: i32,
    pub value: f32,
}

/// `DPOHUDData` (`0x1a4` bytes; constructor `0x826b3dd0`, `Reset` `0x826b1cb8`).
#[derive(Clone, Debug, PartialEq)]
pub struct DpoHud {
    /// `mFormatType` (`eDPOFormat`).
    pub format_type: i32,
    /// `mStatus` (`eDPOStatus`: None, Close, Failed, Success).
    pub status: i32,
    pub status_dirty: bool,
    /// `mIcon` (`eDPOIcon`).
    pub icon: i32,
    pub first_param: f32,
    pub second_param: f32,
    /// `formattedFirstParam`, empty until a first parameter is formatted.
    pub formatted_first: Option<DpoParam>,
    /// `formattedSecondParam`.
    pub formatted_second: Option<DpoParam>,
    /// `mTitle`: a HAL id, or `#` and literal text.
    pub title: String,
    pub dirty: bool,
}

impl Default for DpoHud {
    fn default() -> Self {
        Self {
            format_type: format::INTS,
            status: 0,
            status_dirty: true,
            icon: 0,
            first_param: -1.0,
            second_param: -1.0,
            formatted_first: None,
            formatted_second: None,
            title: "#".to_owned(),
            dirty: true,
        }
    }
}

impl DpoHud {
    /// `SetParams(firstParam, secondParam)` (`0x826b3338`): a changed parameter is formatted
    /// by the current format type and dirties the data.
    #[expect(clippy::float_cmp, reason = "the native compares the parameters exactly")]
    pub fn set_params(&mut self, first: f32, second: f32) {
        if self.first_param != first {
            self.first_param = first;
            self.formatted_first = Some(DpoParam { format: self.format_type, value: first });
            self.dirty = true;
        }
        if self.second_param != second {
            self.second_param = second;
            self.formatted_second = Some(DpoParam { format: self.format_type, value: second });
            self.dirty = true;
        }
    }
}

fn dpo<'w>(world: &'w mut World, call: &NativeCall) -> Option<&'w mut DpoHud> {
    let group = get_string(call, 0).unwrap_or_default();
    world.hud.group_mut(&group).and_then(|g| g.dpo.as_mut())
}

/// `HUD2.DPOSetFormatType(group, format = 0)` (`0x8289a508`).
fn set_format_type(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let value = get_int_or(call, 1, 0);
    if let Some(data) = dpo(world, call)
        && data.format_type != value
    {
        data.format_type = value;
        data.dirty = true;
    }
    none()
}

/// `HUD2.DPOSetStatus(group, status = 0)` (`0x8289a5a8`).
fn set_status(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let value = get_int_or(call, 1, 0);
    if let Some(data) = dpo(world, call)
        && data.status != value
    {
        data.status = value;
        data.dirty = true;
        data.status_dirty = true;
    }
    none()
}

/// `HUD2.DPOSetParams(group, first = 0, second = 0)` (`0x8289a650`).
fn set_params(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let first = get_float_or(call, 1, 0.0);
    let second = get_float_or(call, 2, 0.0);
    if let Some(data) = dpo(world, call) {
        data.set_params(first, second);
    }
    none()
}

/// `HUD2.DPOSetTitle(group, title)` (`0x8289a720`): `SetTitle` (`0x826b36e0`) copies it and
/// dirties the data.
fn set_title(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(title) = get_string(call, 1) else { return none() };
    if let Some(data) = dpo(world, call) {
        data.title = title.chars().take(127).collect();
        data.dirty = true;
    }
    none()
}

/// `HUD2.DPOSetIcon(group, icon = 0)` (`0x8289a7c8`).
fn set_icon(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let value = get_int_or(call, 1, 0);
    if let Some(data) = dpo(world, call)
        && data.icon != value
    {
        data.icon = value;
        data.dirty = true;
    }
    none()
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("HUD2", "DPOSetFormatType", set_format_type),
        ("HUD2", "DPOSetStatus", set_status),
        ("HUD2", "DPOSetParams", set_params),
        ("HUD2", "DPOSetTitle", set_title),
        ("HUD2", "DPOSetIcon", set_icon),
    ]
}
