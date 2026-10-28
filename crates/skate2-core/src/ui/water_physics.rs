//! Optional PC-authored water resistance. It is off unless the player enables it.
use std::{path::PathBuf,sync::{Mutex,OnceLock,atomic::{AtomicBool,Ordering}},time::{Duration,Instant}};
use super::{online_config::strict_string_fields,weather::Mode};

fn path()->Result<PathBuf,String>{
    Ok(std::env::var_os("SKATE2_SAVE_ROOT").map_or_else(std::env::current_dir,|p|Ok(PathBuf::from(p))).map_err(|e|e.to_string())?.join("water-physics.json"))
}
fn parse(text:&str)->Result<bool,String>{
    let fields=strict_string_fields(text).ok_or("Invalid water-physics.json")?;
    let mut enabled=false;let mut seen=std::collections::BTreeSet::new();
    for (key,value) in fields {
        if !seen.insert(key.clone()){return Err(format!("Duplicate water setting {key}"));}
        match (key.as_str(),value.as_str()) {
            ("version","1")=>{},("enabled","on")=>enabled=true,("enabled","off")=>enabled=false,
            _=>return Err(format!("Invalid water setting {key}"))
        }
    }
    Ok(enabled)
}
fn state()->Result<&'static AtomicBool,String>{
    static STATE:OnceLock<Result<AtomicBool,String>>=OnceLock::new();
    STATE.get_or_init(||{
        let enabled=match std::fs::read_to_string(path()?){
            Ok(text)=>parse(&text)?,Err(e) if e.kind()==std::io::ErrorKind::NotFound=>false,Err(e)=>return Err(e.to_string()),
        };
        Ok(AtomicBool::new(enabled))
    }).as_ref().map_err(Clone::clone)
}
pub fn enabled()->bool{state().is_ok_and(|s|s.load(Ordering::Relaxed))}
pub fn toggle()->Result<(),String>{
    let state=state()?;let next=!state.load(Ordering::Relaxed);let path=path()?;
    if let Some(parent)=path.parent(){std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    std::fs::write(path,format!("{{\n  \"version\": \"1\",\n  \"enabled\": \"{}\"\n}}\n",if next{"on"}else{"off"})).map_err(|e|e.to_string())?;
    state.store(next,Ordering::Relaxed);clear_contact();Ok(())
}
#[derive(Clone,Copy,Debug)]
pub struct Contact {pub position:[f32;3],pub depths:[f32;4],pub mode:Mode,at:Instant}
fn latest()->&'static Mutex<Option<Contact>>{
    static CONTACT:OnceLock<Mutex<Option<Contact>>>=OnceLock::new();CONTACT.get_or_init(||Mutex::new(None))
}
/// Only the local rendered board reports actual sampled wheel contacts.
pub fn publish_contact(position:[f32;3],depths:[f32;4],mode:Mode){
    if !enabled() || !position.iter().chain(depths.iter()).all(|v|v.is_finite())
        || !matches!(mode,Mode::LightRain|Mode::MediumRain|Mode::HeavyRain|Mode::AfterRain){clear_contact();return;}
    if let Ok(mut contact)=latest().lock(){*contact=Some(Contact{position,depths:depths.map(|d|d.clamp(0.,0.65)),mode,at:Instant::now()});}
}
pub fn clear_contact(){if let Ok(mut contact)=latest().lock(){*contact=None;}}
pub(crate) fn contact_at(position:[f32;3])->Option<Contact>{
    if !enabled() || !position.iter().all(|v|v.is_finite()){return None;}
    let contact=latest().lock().ok().and_then(|c|*c)?;
    if contact.at.elapsed()>Duration::from_millis(200) || super::weather::current().ok()!=Some(contact.mode)
        || (position[0]-contact.position[0]).hypot(position[2]-contact.position[2])>0.8
        || position[1]>contact.position[1]+0.12 || position[1]<contact.position[1]-0.8 {return None;}
    Some(contact)
}

/// A visibly wet wheel keeps 15% of its normal contact friction.
pub(crate) fn grip_scale(depth:f32)->f32 {
    if depth.is_finite() && depth>0.003 {0.15}else{1.0}
}
/// Bounded hydrodynamic response; thresholds are authored, not retail physics.
#[derive(Default,Debug)]
pub(crate) struct Response {wet_seconds:f32,cooldown:f32}
#[derive(Clone,Copy,Debug,Default)]
pub(crate) struct Decision {pub depths:[f32;4],pub drag:f32,pub wipeout:bool}
impl Response {
    pub(crate) fn step(&mut self,depths:Option<[f32;4]>,speed:f32,dt:f32)->Decision {
        if !dt.is_finite() || !(0.0..=0.1).contains(&dt) || !speed.is_finite(){*self=Self::default();return Decision::default();}
        self.cooldown=(self.cooldown-dt).max(0.);
        let Some(depths)=depths.filter(|ds|ds.iter().all(|d|d.is_finite())) else {self.wet_seconds=0.;return Decision::default();};
        let depth=depths.iter().copied().fold(0.,f32::max);
        if depth<=0.003 {self.wet_seconds=0.;return Decision::default();}
        if speed>0.6 && depth>0.14 {self.wet_seconds+=dt;} else {self.wet_seconds=0.;}
        let wipeout=self.wet_seconds>=0.35&&self.cooldown<=0.;
        if wipeout {self.cooldown=2.;self.wet_seconds=0.;}
        // Shallow puddles reduce wheel contact grip; deeper water adds drag.
        Decision{depths,drag:((depth-0.06).max(0.)*2.5).min(0.8),wipeout}
    }

}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn preference_defaults_off_and_rejects_bad_values(){
  assert!(!parse("{}").unwrap());assert!(parse(r#"{"version":"1","enabled":"on"}"#).unwrap());
  assert!(!parse(r#"{"enabled":"off"}"#).unwrap());assert!(parse(r#"{"enabled":"maybe"}"#).is_err());
  assert!(parse(r#"{"enabled":"on","enabled":"off"}"#).is_err());
 }
 #[test]fn water_response_requires_contact_and_accumulates_before_bailing(){
  let mut r=Response::default();for _ in 0..120 {let d=r.step(None,10.,1./60.);assert_eq!(d.drag,0.);assert!(!d.wipeout);}
  for _ in 0..120 {assert!(!r.step(Some([0.001;4]),10.,1./60.).wipeout);}
  assert_eq!(grip_scale(0.),1.);assert_eq!(grip_scale(f32::NAN),1.);assert_eq!(grip_scale(0.01),0.15);
  let shallow=r.step(Some([0.01;4]),5.,1./60.);assert!(shallow.depths[0]>0.&&!shallow.wipeout);
  let first=r.step(Some([0.25;4]),5.,1./60.);assert!(first.drag>0.&&!first.wipeout);
  let mut bailed=false;for _ in 0..40 {bailed|=r.step(Some([0.25;4]),5.,1./60.).wipeout;}assert!(bailed);
  assert!(!r.step(None,5.,1./60.).wipeout);
 }
}
