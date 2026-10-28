//! Original ContextTrickDone/UpdateBigTrickDisplay/UpdateEntityTricks,
//! trickdisplay2 AS offsets21da1/21f45/223e4; enums826127d8.
use super::{Assets, Clip, RenderResult};
use skate2_core::ui::trick_context::{ContextDisplay, Entry};

pub(super) fn refresh(assets: &Assets, root: &mut Clip, context: &ContextDisplay,
    previous: &mut [bool; 7]) -> RenderResult<()> {
    for (index, slot) in context.slots.iter().enumerate() {
        let clip = root.named(&[&format!("bigTrick_mc{index}")])?;
        if let Some(entry) = &slot.entry {
            if !previous[index] { clip.goto(assets,"intro")?; }
            if !previous[index] || matches!(entry,Entry::Big {..}) {
                let (text, backer) = label(assets,entry)?;
                for name in ["title","titleGlow"] {
                    clip.named(&["dmo1_mc",name])?.text=Some(text.encode_utf16().collect());
                }
                clip.named(&["dmo1_mc","contextBacker_mc"])?.goto(assets,backer)?;
            }
        } else if previous[index] { clip.goto(assets,"outro")?; }
        previous[index]=slot.entry.is_some();
    }
    Ok(())
}
fn label(assets: &Assets, entry: &Entry) -> RenderResult<(String,&'static str)> {
    match *entry {
        Entry::Big { index,achievement,measurement } => {
            let suffix = ["MANUAL","GRIND","POWERSLIDE","HANDPLANT","GRABTIME","AIRTIME",
                "AIRHORDISTANCE","AIRHEIGHT","AIRSTEPUP","AIRSTEPDOWN","SEQUENCESIZE",
                "GRASSGAP","STAIRGAP","HIGHGAP","VERYHIGHGAP"].get(index as usize).copied();
            let mut text=assets.localize(&suffix.map_or_else(||"ID_COMMON_UNKNOWN".into(),
                |s|format!("ID_SCORETHRESHOLDTYPE_{s}")))?;
            if measurement > 0. {
                let amount=match index {
                    4|5 => format!("{}s",measurement.floor()),
                    10 => format!("{}",measurement.floor()),
                    _ => distance(assets,measurement)?,
                };
                text.push(' ');text.push_str(&amount);
            }
            Ok((text,match achievement {0=>"bigtrick",1=>"hugetrick",_=>"personalbest"}))
        }
        Entry::Entity {entity_type,is_entity,count,context_type} => {
            // Shipped AS tests0 twice; its OFF branch is unreachable. Preserve
            // that flow. The native air producer sends Over(0).
            let action=assets.localize(if context_type==0 {"ID_TRICKDISPLAY_CONTEXTTYPE_OVER"}
                else {"ID_TRICKDISPLAY_CONTEXTTYPE_ON"})?;
            let key=if is_entity {entity_label(entity_type)} else {
                ["ID_CONTEXTENTITYGROUP_PEDESTRIAN","ID_CONTEXTENTITYGROUP_VEHICLE",
                    "ID_CONTEXTENTITYGROUP_SKATER","ID_CONTEXTENTITYGROUP_DYNAMICOBJECT",
                    "ID_CONTEXTENTITYGROUP_STATICOBJECT"].get(entity_type as usize).copied()
                    .unwrap_or("ID_COMMON_UNKNOWN").into()
            };
            let object=assets.localize(&key)?;
            let mut text=format!("{action} {object}");
            if count>1 {text.push_str(&format!(" x{count}"));}
            Ok((text,"dmo"))
        }
    }
}
fn entity_label(index: i32) -> String {
    if index==6 {return "ID_COMMON_BIG_BLACK".into();}
    if index==7 {return "ID_ONLINE_RANK_ICON_PRO_TYPE".into();}
    let names=["MALE","FEMALE","GRANNY","SECURITYGUARD","BUM","JOCK","","","SKATER",
        "CAR","SPORTSCAR","MUSCLECAR","SUV","TRUCK","BARRIER","BENCH","BOX","CHAIR",
        "DUMPSTER","GARBAGEBIN","KEG","MAILBOX","MANNIEPAD","NEWSPAPERBOX","OILDRUM",
        "PLANTER","PYLON","QUARTERPIPE","RAIL","RAMP","TABLE","AWNING","BIKERACK",
        "BILLBOARD","BOLLARD","BOULDER","BUSSHELTER","CABLECAR","CARGOCONTAINER",
        "ELECTRICALBOX","FENCE","FIREHYDRANT","FLAGPOLE","HYDROLINE","HYDROPOLE",
        "LAMPPOLE","PHONEBOOTH","SIGN"];
    if let Some(name)=names.get(index as usize) {format!("ID_CONTEXTENTITYTYPE_{name}")}
    else if (48..68).contains(&index) {format!("ID_CONTEXTENTITYTYPE_DLC{:03}",index-47)}
    else {"ID_COMMON_UNKNOWN".into()}
}
fn distance(assets: &Assets, meters: f32) -> RenderResult<String> {
    // TrickDisplay_GetFormatDistance826b1f28 formats the metres with one
    // decimal (FormatNumber precision r6=1), then FormatStringCustom with
    // ID_BIGTRICK_DISTANCE_FORMAT: ProcessDistance8255a2b8 reads the first
    // comma-separated argument and writes feet (metres * 3, one decimal).
    let metres = crate::marker_ui::localizer::format_float(assets, meters, 1);
    Ok(String::from_utf16(&crate::marker_ui::localizer::format_custom(assets, "ID_BIGTRICK_DISTANCE_FORMAT", &metres)?)?)
}
