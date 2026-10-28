//! Audio content hot swap (doc 16 "L1"): when the running mods' audio content changes (a mod
//! starts, stops, fails, reloads, or its `audio.json` / files change while it runs), the new
//! [`Library`] is swapped in **without restarting the native runtime**: only what changed is
//! replaced, in place, and everything else keeps playing.
//!
//! What is swapped in place, and why it is exact (the runtime then holds exactly what a restart
//! would have loaded for it):
//! - **AEMS banks** the runtime holds whose content changed (program, WAVs by slot, rebuilt
//!   headers, volume group): `Runtime::replace_bank` keeps the bank's id and its place among each
//!   class's constructors; its instances go (their voices released) and every post its poster
//!   still holds is re-bound to the new bank with the post's payload, so a held sound (a
//!   traffic engine, an emitter, the player's rolling) continues on the new content. A bank that
//!   left the audio is unloaded. Banks not loaded load from the new library on use.
//! - **Overlay-preloaded banks**: the new set loads now.
//! - **Splice trees** (pops, landings, foley, the ped ring) and **wheel streams**: replaced in place
//!   (sounding Splice sounds of a replaced tree stop, as after a release).
//! - **Mod Csis projects** (L4): a new one is installed, one that went is taken out of the lookups
//!   (`Registry::uninstall`), a changed one both; the banks bound to them are replaced (re-bound)
//!   or unloaded.
//! - **Tuning sections** from overlays: handed to the systems that cache them (as a tuning write).
//! - **Speech** takes / index: the speech host reloads its data (the lines speaking stop).
//! - **The world layer** (beds, zones, crossfades, regions, `.ems` records, sets, map audio): the
//!   map-keyed state rebuilds only when it changed (`AudioContent::world_generation`).
//!
//! What still restarts the runtime (the fallback: it cannot be swapped exactly in place):
//! - the **MixMap** file (its controller graph and every instance's state are built from it);
//! - the **rolling bed's** grain recordings or grain tuning (the bed is built at start);
//! - the **install's own projects** (never changed by overlays; a guard);
//! - a Splice tree a library lost (a guard) and any failure while swapping (logged).
use bevy::prelude::*;
use serde_json::Value;

use super::{Library, Native};

/// What a swap changes, or why it cannot be done in place.
#[derive(Debug, Default)]
pub(crate) struct Plan {
    /// Reasons for the restart fallback (empty: swap in place).
    pub restart: Vec<String>,
    pub replace: Vec<String>,
    pub unload: Vec<String>,
    pub splice: Vec<String>,
    pub wheels: bool,
    /// Mod projects to take out (file, token) and to install (file).
    pub projects_out: Vec<(String, u64)>,
    pub projects_in: Vec<String>,
    pub resident: bool,
    pub tuning: Vec<(&'static str, Value, Value)>,
    pub speech: bool,
    pub world: bool,
}

/// Compare the library the runtime runs on with the new one.
pub(crate) fn plan(old: &Library, new: &Library, native: &Native) -> Plan {
    let mut p = Plan::default();
    if old.mixmap_key() != new.mixmap_key() {
        p.restart.push("the MixMap file changed".into());
    }
    if old.grain_key() != new.grain_key() {
        p.restart
            .push("the rolling bed's grain recordings or tuning changed".into());
    }
    let (old_retail, _) = old.project_files();
    let (new_retail, new_mods) = new.project_files();
    if old_retail != new_retail {
        p.restart.push("the install's Csis projects changed".into());
    }
    // Mod projects: the runtime's installed list against the new one.
    for (file, stamp, token) in &native.mod_projects {
        if !new_mods.iter().any(|(f, s)| f == file && s == stamp) {
            p.projects_out.push((file.clone(), *token));
        }
    }
    for (file, stamp) in &new_mods {
        if !native
            .mod_projects
            .iter()
            .any(|(f, s, _)| f == file && s == stamp)
        {
            p.projects_in.push(file.clone());
        }
    }
    let project_banks: Vec<String> = {
        let rt = native.shared.lock();
        let ids: Vec<usize> = rt.as_ref().map_or(Vec::new(), |rt| {
            p.projects_out
                .iter()
                .flat_map(|(_, t)| rt.eval.banks_using_project(*t))
                .collect()
        });
        native
            .bank_ids()
            .into_iter()
            .filter(|(_, id)| ids.contains(id))
            .map(|(s, _)| s)
            .collect()
    };
    for (stem, _) in native.bank_ids() {
        match (old.bank_key(&stem), new.bank_key(&stem)) {
            (_, None) => p.unload.push(stem),
            (a, Some(b))
                if a.as_ref() != Some(&b)
                    || project_banks.contains(&stem)
                    || !p.projects_in.is_empty() && new.is_mod_bank(&stem) =>
            {
                p.replace.push(stem)
            }
            _ => {}
        }
    }
    if let Ok(rt) = native.shared.lock() {
        for b in &rt.splice.banks {
            match (old.splice_key(&b.name), new.splice_key(&b.name)) {
                (_, None) => p
                    .restart
                    .push(format!("the Splice tree {} left the audio", b.name)),
                (a, Some(n)) if a.as_ref() != Some(&n) => p.splice.push(b.name.clone()),
                _ => {}
            }
        }
    }
    p.wheels = old.wheels_key() != new.wheels_key();
    p.resident = old.resident_banks() != new.resident_banks();
    let (a, b) = (old.tuning_sections(), new.tuning_sections());
    for section in super::library::TUNING_SECTIONS {
        let (x, y) = (
            a.get(section).cloned().unwrap_or(Value::Null),
            b.get(section).cloned().unwrap_or(Value::Null),
        );
        if x != y {
            p.tuning.push((section, x, y));
        }
    }
    p.speech = ["livingworld", "maincast"]
        .iter()
        .any(|a| old.speech_key(a) != new.speech_key(a));
    p.world = old.world_key() != new.world_key();
    p
}

/// Apply a plan without restart reasons to the running runtime (the new library is not yet a
/// resource). Errors are logged; the caller restarts on `Err`.
pub(crate) fn apply(world: &mut World, plan: &Plan, new: &Library) -> Result<(), String> {
    let mut native = world
        .remove_resource::<Native>()
        .ok_or("no native runtime")?;
    let result = apply_native(&mut native, plan, new);
    world.insert_resource(native);
    result?;
    if plan.speech {
        if let Some(native) = world.get_resource::<Native>().map(|n| n.shared.clone()) {
            if let (Some(mut speech), Ok(mut rt)) = (
                world.get_resource_mut::<super::world_speech::WorldSpeech>(),
                native.lock(),
            ) {
                speech.reload_content(&mut rt);
            }
        }
    }
    Ok(())
}

fn apply_native(native: &mut Native, plan: &Plan, new: &Library) -> Result<(), String> {
    // Banks bound to a project that goes come out first; the project after them.
    for stem in &plan.unload {
        native.unload_bank(stem);
    }
    {
        let mut rt = native.shared.lock().map_err(|_| "audio lock poisoned")?;
        for (file, token) in &plan.projects_out {
            rt.eval.uninstall_project(*token);
            info!("Game audio: swap: Csis project {file} out");
        }
    }
    native
        .mod_projects
        .retain(|(_, _, t)| !plan.projects_out.iter().any(|(_, x)| x == t));
    for file in &plan.projects_in {
        let bytes = new.read(file).map_err(|e| format!("{file}: {e}"))?;
        let project =
            skate_audio::formats::Project::parse(file, &bytes).map_err(|e| e.to_string())?;
        let token = native
            .shared
            .lock()
            .map_err(|_| "audio lock poisoned")?
            .install_project(&project);
        native
            .mod_projects
            .push((file.clone(), new.stamp(file), token));
        info!("Game audio: swap: Csis project {file} in");
    }
    for stem in &plan.replace {
        native.replace_bank(new, stem)?;
    }
    if plan.resident {
        native.set_resident(new);
    }
    if !plan.splice.is_empty() || plan.wheels {
        let mut rt = native.shared.lock().map_err(|_| "audio lock poisoned")?;
        let rt = &mut *rt;
        for stem in &plan.splice {
            let (bank, pcm) = new
                .splice_bank(stem)
                .ok_or_else(|| format!("Splice tree {stem} does not load"))?;
            rt.splice.replace_bank(stem, bank, pcm, &mut rt.mixer);
        }
        if plan.wheels {
            rt.load_streams(
                super::player_audio::WHEEL_STREAMS
                    .iter()
                    .map(|n| new.wheels_pcm(n))
                    .collect(),
            );
        }
    }
    // A decoded bank the prefetch worker holds may be of the old content.
    native.prefetch.clear();
    Ok(())
}
