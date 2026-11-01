//! Shared native-menu weather preferences.
pub(crate) use skate2_core::ui::weather::*;
static RAIN_EXPOSURE:std::sync::atomic::AtomicU32=std::sync::atomic::AtomicU32::new(1f32.to_bits());
pub(crate) fn set_rain_exposure(value:f32){RAIN_EXPOSURE.store(value.to_bits(),std::sync::atomic::Ordering::Relaxed);}
pub(crate) fn rain_exposure()->f32 {f32::from_bits(RAIN_EXPOSURE.load(std::sync::atomic::Ordering::Relaxed))}

/// PC weather cloud cover, using the same cover rule as the ported surface shader.
pub(crate) fn cloud_cover()->f32 {
    let mode=render_mode().unwrap_or_default();
    if mode==Mode::AfterRain {return 0.32;}
    let [rain,snow]=mode.amounts();
    if snow>0. {1.} else {(rain*2.).min(1.)}
}

// Last rendered storm clock: the next tree draw uses the same gust phase.
static RAIN_CLOCK:std::sync::atomic::AtomicU32=std::sync::atomic::AtomicU32::new(0);
pub(crate) fn set_rain_clock(value:f32){RAIN_CLOCK.store(value.to_bits(),std::sync::atomic::Ordering::Relaxed);}
pub(crate) fn rain_clock()->f32{f32::from_bits(RAIN_CLOCK.load(std::sync::atomic::Ordering::Relaxed))}

#[derive(Clone,Copy,Default)]
pub(crate) struct Playback {pub replay:bool,pub running:bool,pub recorded:Option<ReplayWeather>}
static PLAYBACK:std::sync::Mutex<Playback>=std::sync::Mutex::new(Playback {replay:false,running:true,recorded:None});
pub(crate) fn set_playback(value:Playback){if let Ok(mut state)=PLAYBACK.lock(){*state=value;}}
pub(crate) fn playback()->Playback {PLAYBACK.lock().map_or(Playback {replay:false,running:true,recorded:None},|state|*state)}
pub(crate) fn render_mode()->Result<Mode,String> {playback().recorded.map_or_else(current,|recorded|Ok(recorded.mode))}
