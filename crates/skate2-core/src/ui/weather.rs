//! PC weather preferences shared by the native menus, renderer and ambience mixer.
use super::online_config::{quote, strict_string_fields};
use std::{path::PathBuf, sync::{OnceLock, atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering}}};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode { #[default] Clear, LightRain, MediumRain, HeavyRain, LightSnow, HeavySnow, AfterRain }
impl Mode {
    pub fn key(self) -> &'static str { ["clear", "light_rain", "medium_rain", "heavy_rain", "light_snow", "heavy_snow", "after_rain"][self as usize] }
    pub fn label(self) -> &'static str { ["Clear", "Light Rain", "Medium Rain", "Heavy Rain", "Light Snow", "Heavy Snow", "30 min after rain"][self as usize] }
    pub fn amounts(self) -> [f32; 2] { match self { Self::Clear | Self::AfterRain => [0.,0.], Self::LightRain => [0.35,0.], Self::MediumRain => [0.65,0.], Self::HeavyRain => [1.,0.], Self::LightSnow => [0.,0.35], Self::HeavySnow => [0.,1.] } }
    fn from_index(index: u8) -> Self { [Self::Clear, Self::LightRain, Self::MediumRain, Self::HeavyRain, Self::LightSnow, Self::HeavySnow, Self::AfterRain][usize::from(index)] }
    pub fn from_recorded(index:u8)->Option<Self> { (index<7).then(||Self::from_index(index)) }
    fn parse(key: &str) -> Result<Self, String> { (0..7).map(Self::from_index).find(|m| m.key() == key).ok_or_else(|| format!("Unknown weather mode {key:?}")) }
}
/// The last weather phase actually shown by the renderer, captured alongside
/// simulation replay frames. It is independent of user audio preferences.
static RENDERED_PHASE:AtomicU32=AtomicU32::new(0);
pub fn set_rendered_phase(value:f32) {if value.is_finite() {RENDERED_PHASE.store(value.to_bits(),Ordering::Relaxed);}}
pub fn rendered_phase()->f32 {f32::from_bits(RENDERED_PHASE.load(Ordering::Relaxed))}
#[derive(Clone,Copy,Debug,PartialEq)]
pub struct ReplayWeather {pub mode:Mode,pub phase:f32}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Config { mode: Mode, rain_steps: u8 }
impl Default for Config { fn default() -> Self { Self { mode: Mode::Clear, rain_steps: 10 } } }
impl Config {
    fn packed(self) -> u16 { u16::from(self.mode as u8) | (u16::from(self.rain_steps) << 8) }
    fn unpack(value: u16) -> Self { Self { mode: Mode::from_index(value as u8), rain_steps: (value >> 8) as u8 } }
    fn text(self) -> String { format!("{{\n  \"version\": \"1\",\n  \"mode\": {},\n  \"rain_volume\": {}\n}}\n", quote(self.mode.key()), quote(&self.rain_steps.to_string())) }
    fn parse(text: &str) -> Result<Self, String> {
        let fields = strict_string_fields(text).ok_or("Invalid weather.json object")?;
        let mut result = Self::default();
        let mut seen = std::collections::BTreeSet::new();
        for (key, value) in fields {
            if !seen.insert(key.clone()) { return Err(format!("Duplicate weather setting {key}")); }
            match key.as_str() {
                "version" if value == "1" => {},
                "mode" => result.mode = Mode::parse(&value)?,
                "rain_volume" => result.rain_steps = value.parse::<u8>().ok().filter(|v| *v <= 10).ok_or("Rain volume must be 0..10")?,
                _ => return Err(format!("Unknown weather setting {key} or unsupported value {value:?}")),
            }
        }
        Ok(result)
    }
}
fn path() -> Result<PathBuf, String> {
    Ok(std::env::var_os("SKATE2_SAVE_ROOT").map_or_else(std::env::current_dir, |p| Ok(PathBuf::from(p))).map_err(|e| e.to_string())?.join("weather.json"))
}
fn state() -> Result<&'static AtomicU16, String> {
    static STATE: OnceLock<Result<AtomicU16, String>> = OnceLock::new();
    STATE.get_or_init(|| {
        let mut config = match std::fs::read_to_string(path()?) {
            Ok(text) => Config::parse(&text)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
            Err(e) => return Err(e.to_string()),
        };
        // Diagnostic override is subprocess-only, never changes ordinary preferences.
        if let Ok(key) = std::env::var("SKATE2_WEATHER") { config.mode = Mode::parse(&key)?; }
        Ok(AtomicU16::new(config.packed()))
    }).as_ref().map_err(Clone::clone)
}
fn config() -> Result<Config, String> { Ok(Config::unpack(state()?.load(Ordering::Relaxed))) }
fn save(value: Config) -> Result<(), String> {
    let path = path()?;
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    std::fs::write(path, value.text()).map_err(|e| e.to_string())?;
    state()?.store(value.packed(), Ordering::Relaxed);
    Ok(())
}
pub fn current() -> Result<Mode, String> { Ok(config()?.mode) }
pub fn rain_steps() -> Result<u8, String> { Ok(config()?.rain_steps) }
pub fn rain_volume() -> Result<f32, String> { Ok(f32::from(rain_steps()?) * 0.1) }
pub fn cycle(forward: bool) -> Result<(), String> {
    let mut value = config()?;
    let order=[Mode::Clear,Mode::LightRain,Mode::MediumRain,Mode::HeavyRain,Mode::AfterRain,Mode::LightSnow,Mode::HeavySnow];
    let index=order.iter().position(|m|*m==value.mode).unwrap_or(0);
    value.mode=order[(index+if forward {1}else{6})%order.len()];
    save(value)?;
    crate::diagnostics::test_log::event("weather_mode", format_args!("mode={} local_only=true", value.mode.key()));
    Ok(())
}
pub fn adjust_rain(forward: bool) -> Result<(), String> {
    let mut value = config()?;
    value.rain_steps = if forward { value.rain_steps.saturating_add(1).min(10) } else { value.rain_steps.saturating_sub(1) };
    save(value)
}
static VISIBLE: AtomicBool = AtomicBool::new(false);
pub fn set_visible(value: bool) { VISIBLE.store(value, Ordering::Relaxed); }
pub fn visible() -> bool { VISIBLE.load(Ordering::Relaxed) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_round_trip_and_reject_invalid_preferences() {
        for mode in (0..7).map(Mode::from_index) {
            for rain_steps in 0..=10 {
                let value = Config { mode, rain_steps };
                assert_eq!(Config::parse(&value.text()).unwrap(), value);
                assert_eq!(Config::unpack(value.packed()), value);
            }
        }
        for text in [r#"{"mode":"fog"}"#, r#"{"rain_volume":"11"}"#, r#"{"rain_volume":"-1"}"#, r#"{"rain_volume":2}"#, r#"{"mode":"clear","mode":"heavy_rain"}"#, r#"{"version":"2"}"#, r#"{"mode":"clear"}oops"#] { assert!(Config::parse(text).is_err(), "{text}"); }
    }
}
