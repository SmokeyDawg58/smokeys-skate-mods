//! PC mod control for the free camera. This does not remap stock controls.
use std::{path::PathBuf, sync::{Mutex, OnceLock}};

pub const DEFAULT_CHORD:u16=0x0088; // Right stick click + D-pad Right.
pub const TELEPORT:u16=0x2000; // B, reserved while the camera is active.
const ALLOWED:u16=0xd3cf; // D-pad, sticks, bumpers and A/X/Y; not B/Start/Back.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bindings { pub toggle:u16 }
impl Default for Bindings { fn default()->Self { Self { toggle:DEFAULT_CHORD } } }
impl Bindings {
    pub const fn chord(self)->u16 {self.toggle}
    pub fn label(self)->String {
        button_items(self.toggle).map(|(_,name)|name).collect::<Vec<_>>().join(" + ")
    }
    pub fn icons(self)->Vec<&'static str> {button_items(self.toggle).map(|(icon,_)|icon).collect()}
}
fn button_items(mask:u16)->impl Iterator<Item=(&'static str,&'static str)> {
    const BUTTONS:[(u16,&str,&str);11]=[
        (0x80,"R3_Click","Right Stick Click"),(0x40,"L3_Click","Left Stick Click"),
        (0x100,"L_Bumper","LB"),(0x200,"R_Bumper","RB"),
        (1,"DPad_Up","D-pad Up"),(2,"DPad_Down","D-pad Down"),
        (4,"DPad_Left","D-pad Left"),(8,"DPad_Right","D-pad Right"),
        (0x1000,"A","A"),(0x4000,"X","X"),(0x8000,"Y","Y")];
    BUTTONS.into_iter().filter(move |(bit,_,_)|mask & bit!=0).map(|(_,icon,name)|(icon,name))
}
pub fn valid(mask:u16)->bool {
    mask!=0 && mask & !ALLOWED==0 && mask.count_ones()<=2
        && mask & 0x0100 == 0 // LB opens the phone, including Object Dropper and Contacts.
        && mask != 0x0004 && mask != 0x8000 // Do not shadow LB + Left or LB + Y.
        && mask & 3 != 3 && mask & 12 != 12
}
fn path()->Result<PathBuf,String> {
    Ok(std::env::var_os("SKATE2_SAVE_ROOT").map_or_else(std::env::current_dir,|p|Ok(PathBuf::from(p))).map_err(|e|e.to_string())?.join("debug-camera-keybinds.json"))
}
fn parse(text:&str)->Result<Bindings,String> {
    let fields=super::online_config::strict_string_fields(text).ok_or("Invalid debug camera settings")?;
    let mut chord=None; let mut version=None;
    for (key,value) in fields {
        match key.as_str() {
            "version" if version.is_none()=>version=Some(value),
            "activation" if chord.is_none()=>chord=Some(u16::from_str_radix(&value,16).map_err(|_|"Invalid activation")?),
            _=>return Err(format!("Invalid debug camera setting: {key}")),
        }
    }
    if version.as_deref()!=Some("2") {return Err("Invalid debug camera settings version".into());}
    let toggle=chord.ok_or("Missing activation")?;
    if !valid(toggle){return Err("Invalid debug camera chord".into());}
    Ok(Bindings{toggle})
}
fn state()->&'static Mutex<Bindings> {
    static STATE:OnceLock<Mutex<Bindings>>=OnceLock::new();
    STATE.get_or_init(||Mutex::new(path().ok().and_then(|p|std::fs::read_to_string(p).ok()).and_then(|s|parse(&s).ok()).unwrap_or_default()))
}
pub fn current()->Bindings {state().lock().map_or_else(|_|Bindings::default(),|v|*v)}
pub fn save(toggle:u16)->Result<(),String> {
    if !valid(toggle){return Err("That shortcut conflicts with another control".into());}
    let file=path()?;if let Some(parent)=file.parent(){std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    std::fs::write(file,format!("{{\n  \"version\": \"2\",\n  \"activation\": \"{toggle:04x}\"\n}}\n")).map_err(|e|e.to_string())?;
    *state().lock().map_err(|e|e.to_string())?=Bindings{toggle};Ok(())
}

#[cfg(test)] mod tests {use super::*;
    #[test]fn defaults_icons_and_round_trip(){
        let b=Bindings::default();assert_eq!(b.chord(),0x88);
        assert_eq!(b.icons(),["R3_Click","DPad_Right"]);
        assert_eq!(parse(r#"{"version":"2","activation":"0088"}"#).unwrap(),b);
    }
    #[test]fn rejects_reserved_controls(){
        assert!(!valid(0x104));assert!(!valid(0x8100));assert!(!valid(TELEPORT));
        assert!(!valid(0x10));assert!(!valid(0x83));assert!(valid(0x44));
    }
}
