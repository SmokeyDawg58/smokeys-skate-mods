//! Tuning writes at run time (doc 16 "Tuning writes"; capability `audio_tuning` = 1): the same API
//! for mods (`sdk.audio.set_tuning(domain, patch)`) and engine systems ([`AudioTuning::set`]).
//!
//! - Domains (`skate_mods::audio_tuning`): `player` (`player_tuning`), `world` (`world_tuning`),
//!   `bus` (`bus_tuning`) and `reverb` (`bus_tuning.reverb`). A patch is a strict field merge onto
//!   the section as loaded (install + content overlays): checked when it is set (unknown fields,
//!   indices or types are errors; the merged section must read back as the typed tuning), so the
//!   mod gets the error from its command.
//! - Applied between audio passes: [`apply`] runs at the start of the pass (after a restart, before
//!   any post), rebuilds the patched sections (owners in mod-id order: the first mod to write a
//!   field owns it; a later mod's write of it is an error at `set`), replaces them in the
//!   [`Library`] and hands the new values to the systems that cache them: the local player's
//!   components (and with them the NPC skaters, which read the local player's tuning), the world
//!   host's ped tuning, the speech managers' event tuning and voice curves (their timers kept), the
//!   world bridge's engine records and tazer hold, the runtime's reverb presets, eEQChain records and
//!   FlangeSub returns. Systems that read the Library every frame (traffic engines, the speech
//!   rules, the ped models) see the new values at once.
//! - Restored when the mod stops, fails or reloads (or sets `nil`): the section goes back to the
//!   very value loaded (not a re-read), and the caches follow. After an audio restart (a new
//!   Library) the patches are applied again.
//! - Not tunable: the MixMap's layout (instances, pools), which needs a restart and a deliberate
//!   non-retail option. SFXObj_Jitter keeps the walk it was built with; a reverb preset that is
//!   already faded in keeps its values until the next preset change (the network copies a preset
//!   when it selects it).
//! - Without patches nothing runs ([`apply`] returns at once), so the game sounds exactly as before.
use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use serde_json::Value;
use skate_mods::audio_tuning::{DOMAINS, apply as merge, leaves, section_patch, valid_patch};

use super::Library;

#[derive(Resource, Default)]
pub(crate) struct AudioTuning {
    /// Owner → domain → patch (each mod's latest per domain).
    patches: BTreeMap<String, BTreeMap<String, Value>>,
    /// The patches changed since the last apply.
    dirty: bool,
    /// The content generation the patches were last applied under (a restart loads a new Library).
    applied: Option<u64>,
    /// Sections the Library holds patched values for (restored when no patch is left).
    touched: BTreeSet<&'static str>,
    /// Field (leaf path) → the mod that owns it, as of the last apply.
    owners: BTreeMap<String, String>,
    /// The patched sections' JSON as last applied (what changed is handed on, nothing else).
    last: BTreeMap<&'static str, Value>,
}

impl AudioTuning {
    /// Set (`Some`) or restore (`None`) `owner`'s patch of `domain`. Checked against the section as
    /// loaded and the other mods' fields now; applied at the next audio pass.
    pub(crate) fn set(
        &mut self,
        library: &Library,
        owner: &str,
        domain: &str,
        patch: Option<Value>,
    ) -> Result<(), String> {
        if !DOMAINS.contains(&domain) {
            return Err(format!(
                "audio tuning: no domain {domain} ({})",
                DOMAINS.join(", ")
            ));
        }
        let Some(patch) = patch else {
            if let Some(m) = self.patches.get_mut(owner) {
                if m.remove(domain).is_some() {
                    self.dirty = true;
                }
                if m.is_empty() {
                    self.patches.remove(owner);
                }
            }
            return Ok(());
        };
        if !valid_patch(domain, &patch) {
            return Err("audio tuning: the patch must be a non-empty table of fields (finite numbers, booleans, short strings)".into());
        }
        let (section, full) = section_patch(domain, &patch).ok_or("audio tuning: bad domain")?;
        let base = library.tuning_base()?;
        let mut target = base
            .get(section)
            .cloned()
            .ok_or_else(|| format!("audio tuning: this install has no {section}"))?;
        // Fields other mods own (their patches, in any domain of this section).
        let mut theirs = BTreeMap::new();
        for (o, domains) in self.patches.iter().filter(|(o, _)| *o != owner) {
            for (d, p) in domains {
                if let Some((s, full)) = section_patch(d, p).filter(|(s, _)| *s == section) {
                    let mut l = Vec::new();
                    leaves(&full, s, &mut l);
                    for leaf in l {
                        theirs.entry(leaf).or_insert_with(|| o.clone());
                    }
                }
            }
        }
        let mut conflict = None;
        merge(&mut target, &full, section, &mut |leaf| {
            if let Some(o) = theirs.get(leaf) {
                conflict.get_or_insert_with(|| {
                    format!("audio tuning field {leaf} is owned by another mod ({o})")
                });
            }
            true
        })?;
        if let Some(c) = conflict {
            return Err(c);
        }
        Library::check_tuning(section, &target)?;
        self.patches
            .entry(owner.to_owned())
            .or_default()
            .insert(domain.to_owned(), patch);
        self.dirty = true;
        Ok(())
    }

    /// A mod stopped, failed or was reloaded: its patches go (restored at the next pass).
    pub(crate) fn clear_owner(&mut self, owner: &str) {
        if self.patches.remove(owner).is_some() {
            self.dirty = true;
        }
    }

    /// The fields `owner` owns (`sdk.audio.info`).
    pub(crate) fn owned(&self, owner: &str) -> Vec<String> {
        self.owners
            .iter()
            .filter(|(_, o)| *o == owner)
            .map(|(l, _)| l.clone())
            .collect()
    }

    /// The domain (or a path inside it) as the game uses it now: the section as loaded with the
    /// patches applied (`sdk.audio.tuning`, `sdk.engine.inspect(key, 'audio_tuning:…')`).
    pub(crate) fn read(&self, library: &Library, query: &str) -> Value {
        let mut parts = query.split('/');
        let Some((section, field)) = parts.next().and_then(skate_mods::audio_tuning::section)
        else {
            return Value::Null;
        };
        let Ok(base) = library.tuning_base() else {
            return Value::Null;
        };
        let Some(mut v) = base.get(section).cloned() else {
            return Value::Null;
        };
        let mut claims = BTreeMap::new();
        self.merged_into(section, &mut v, &mut claims);
        let mut at = &v;
        for key in field.into_iter().chain(parts) {
            at = match at {
                Value::Object(m) => m.get(key).unwrap_or(&Value::Null),
                Value::Array(a) => key
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| a.get(i))
                    .unwrap_or(&Value::Null),
                _ => &Value::Null,
            };
        }
        let out = at.clone();
        // A command result rides in every snapshot until replaced: keep it bounded.
        if serde_json::to_vec(&out).is_ok_and(|b| b.len() > 256 * 1024) {
            return serde_json::json!({"error": "more than 256 KiB: read a path inside the domain"});
        }
        out
    }

    /// Apply every owner's patches of `section` (mod-id order; the first owner of a field wins).
    fn merged_into(&self, section: &str, v: &mut Value, claims: &mut BTreeMap<String, String>) {
        for (owner, domains) in &self.patches {
            for d in DOMAINS {
                let Some((s, full)) = domains.get(d).and_then(|p| section_patch(d, p)) else {
                    continue;
                };
                if s != section {
                    continue;
                }
                let result = merge(v, &full, s, &mut |leaf| match claims.get(leaf) {
                    Some(o) => o == owner,
                    None => {
                        claims.insert(leaf.to_owned(), owner.clone());
                        true
                    }
                });
                if let Err(e) = result {
                    // The section changed under the patch (an audio content change): skipped.
                    warn!("Game audio: mod {owner}: {e}; its {d} tuning is left out");
                }
            }
        }
    }

    fn idle(&self, generation: u64) -> bool {
        !self.dirty
            && (self.applied == Some(generation)
                || (self.patches.is_empty() && self.touched.is_empty()))
    }
}

/// The start of the audio pass (`content::frame`, after a restart): apply the changed patches
/// between passes, restore sections nobody patches any more, and hand the new values to the
/// systems that cache them. Returns at once without changes.
pub(super) fn apply(world: &mut World) {
    let generation = world
        .get_resource::<super::AudioContent>()
        .map_or(0, |c| c.generation);
    let Some(t) = world.get_resource::<AudioTuning>() else {
        return;
    };
    if t.idle(generation) {
        return;
    }
    world.resource_scope(|world, mut t: Mut<AudioTuning>| {
        if t.applied != Some(generation) {
            // A new Library (restart): it holds the loaded sections.
            t.touched.clear();
            t.last.clear();
        }
        let Some(mut library) = world.get_resource_mut::<Library>() else {
            t.dirty = false;
            return;
        };
        let wanted: BTreeSet<&'static str> = t
            .patches
            .values()
            .flat_map(|d| d.keys())
            .filter_map(|d| skate_mods::audio_tuning::section(d).map(|s| s.0))
            .collect();
        let mut changed: Vec<(&'static str, Value, Value)> = Vec::new();
        let mut claims = BTreeMap::new();
        for section in super::library::TUNING_SECTIONS {
            let Some(base) = library
                .tuning_base()
                .ok()
                .and_then(|b| b.get(section).cloned())
            else {
                continue;
            };
            if wanted.contains(section) {
                let mut v = base.clone();
                t.merged_into(section, &mut v, &mut claims);
                let prev = t.last.get(section).cloned().unwrap_or_else(|| base.clone());
                if prev == v && t.touched.contains(section) {
                    continue;
                }
                match library.set_tuning(section, &v) {
                    Ok(()) => {
                        t.touched.insert(section);
                        t.last.insert(section, v.clone());
                        changed.push((section, prev, v));
                    }
                    Err(e) => warn!("Game audio: tuning {section}: {e}; the loaded values stay"),
                }
            } else if t.touched.remove(section) {
                library.restore_tuning(section);
                let prev = t.last.remove(section).unwrap_or_else(|| base.clone());
                changed.push((section, prev, base));
            }
        }
        t.owners = claims;
        t.dirty = false;
        t.applied = Some(generation);
        drop(library);
        if !changed.is_empty() {
            info!(
                "Game audio: tuning applied ({:?}; {} fields from mods)",
                changed.iter().map(|c| c.0).collect::<Vec<_>>(),
                t.owners.len()
            );
            retune(world, &changed);
        }
    });
}

/// Hand changed sections (section, before, after) to the systems that cache them; within the
/// bus section only the parts that changed (a FlangeSub preset re-applied restarts its LFOs).
pub(super) fn retune(world: &mut World, changed: &[(&'static str, Value, Value)]) {
    let part = |section: &str, field: &str| {
        changed
            .iter()
            .any(|(s, a, b)| *s == section && (field.is_empty() || a.get(field) != b.get(field)))
    };
    world.resource_scope(|world, library: Mut<Library>| {
        let library = &*library;
        if part("player_tuning", "") {
            if let Some(mut native) = world.get_resource_mut::<super::Native>() {
                if let Some(p) = native.player.as_mut() {
                    p.retune(library);
                }
            }
            let tuning = world
                .get_resource::<super::Native>()
                .and_then(|n| n.player.as_ref().map(|p| p.tuning.clone()));
            if let Some(mut host) = world.get_resource_mut::<super::world_sources::WorldHost>() {
                host.retune(library, tuning.as_ref());
            }
            world.resource_scope(|world, mut api: Mut<super::mod_audio::AudioApi>| {
                api.retune_tags(world.get_resource::<super::Native>());
            });
        }
        if part("world_tuning", "") {
            let tuning = world
                .get_resource::<super::Native>()
                .and_then(|n| n.player.as_ref().map(|p| p.tuning.clone()));
            if let Some(mut host) = world.get_resource_mut::<super::world_sources::WorldHost>() {
                host.retune(library, tuning.as_ref());
            }
            if let Some(mut speech) = world.get_resource_mut::<super::world_speech::WorldSpeech>() {
                speech.retune(library);
            }
            if let Some(mut bridge) = world.get_resource_mut::<super::world_bridge::Bridge>() {
                bridge.retune();
            }
        }
        let (reverb, eq_buses, flange) = (
            part("bus_tuning", "reverb"),
            part("bus_tuning", "eq_buses"),
            part("bus_tuning", "flange"),
        );
        if reverb || eq_buses || flange {
            if let Some(native) = world.get_resource::<super::Native>() {
                let (presets, eq) = library.bus_tuning();
                let returns = library.flange_presets();
                if let Ok(mut rt) = native.shared.lock() {
                    if reverb {
                        rt.mixer.buses.env.presets = presets;
                    }
                    if eq_buses {
                        rt.mixer.buses.eq.set_records(&eq);
                    }
                    if let (true, Some([a, b])) = (flange, returns) {
                        rt.mixer.buses.flange.set_presets(a, b);
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn library() -> Library {
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"));
        Library::load(root).unwrap_or_else(|_| panic!("missing private data: no audio install"))
    }

    fn world(library: Library) -> World {
        let mut world = World::new();
        let native = super::super::Native::start(&library)
            .unwrap_or_else(|e| panic!("missing private data: {e}"));
        world.insert_resource(native);
        world.insert_resource(library);
        world.init_resource::<super::super::AudioContent>();
        world.init_resource::<AudioTuning>();
        world.init_resource::<super::super::mod_audio::AudioApi>();
        world.init_resource::<super::super::world_sources::WorldHost>();
        world.init_resource::<super::super::world_speech::WorldSpeech>();
        world.init_resource::<super::super::world_bridge::Bridge>();
        world
    }

    fn set(
        world: &mut World,
        owner: &str,
        domain: &str,
        patch: Option<Value>,
    ) -> Result<(), String> {
        world.resource_scope(|world, mut t: Mut<AudioTuning>| {
            t.set(world.resource::<Library>(), owner, domain, patch)
        })
    }

    /// The first numeric top-level field of a section and a changed value of the same type.
    fn number_field(library: &Library, section: &str) -> (String, Value) {
        let base = library.tuning_base().unwrap();
        let Some(Value::Object(m)) = base.get(section) else {
            panic!("missing private data: no {section}")
        };
        m.iter()
            .find_map(|(k, v)| match (v.as_u64(), v.as_f64()) {
                (Some(i), _) => Some((k.clone(), json!(if i > 0 { i - 1 } else { 1 }))),
                (None, Some(f)) => Some((k.clone(), json!(f + 1.0))),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing private data: no number in {section}"))
    }

    /// The typed world tuning as sorted debug lines (it holds HashMaps).
    fn world_lines(library: &Library) -> Vec<String> {
        let mut l: Vec<String> = format!("{:#?}", library.world_tuning())
            .lines()
            .map(str::to_owned)
            .collect();
        l.sort();
        l
    }

    fn presets(world: &World) -> Vec<(u64, skate_audio::bus::env::Preset)> {
        let n = world.resource::<super::super::Native>();
        let rt = n.shared.lock().unwrap();
        let mut v: Vec<_> = rt
            .mixer
            .buses
            .env
            .presets
            .iter()
            .map(|(k, p)| (*k, *p))
            .collect();
        v.sort_by_key(|x| x.0);
        v
    }

    /// Tuning writes (data-gated): checked when set (unknown fields, types, other mods' fields are
    /// errors), applied only at the pass boundary, reaching the Library and the systems that cache
    /// it (the local player's components, the runtime's reverb presets); readable as the game uses
    /// them; two mods own different fields; removing the mods restores the very values loaded,
    /// and the runtime's presets and the player's tuning are the retail ones again; a restart
    /// (new Library) gets the patches again.
    #[test]
    #[ignore = "needs the private install data"]
    fn tuning_writes_apply_between_passes_reach_the_systems_and_restore_exactly() {
        let mut world = world(library());
        let taxi = |w: &World| {
            w.resource::<Library>()
                .world_tuning()
                .engine("c04_taxi01")
                .unwrap_or_else(|| panic!("missing private data: no c04_taxi01"))
        };
        let original = taxi(&world);
        let original_world = world_lines(world.resource::<Library>());
        let original_player = world
            .resource::<super::super::Native>()
            .player
            .as_ref()
            .map(|p| p.tuning.clone());
        let original_presets = presets(&world);
        let (field, changed) = number_field(world.resource::<Library>(), "player_tuning");
        let preset_key = original_presets
            .first()
            .map(|p| p.0)
            .unwrap_or_else(|| panic!("missing private data: no reverb presets"));
        let preset_hex = format!("{preset_key:016X}");
        // Checked when set.
        assert!(
            set(&mut world, "dev.a", "world", Some(json!({"nope": 1})))
                .unwrap_err()
                .contains("not in the install")
        );
        assert!(
            set(
                &mut world,
                "dev.a",
                "world",
                Some(json!({"traffic_engine": {"c04_taxi01": {"idle_rpm": "fast"}}}))
            )
            .unwrap_err()
            .contains("does not fit")
        );
        assert!(set(&mut world, "dev.a", "mixmap", Some(json!({"a": 1}))).is_err());
        set(
            &mut world,
            "dev.a",
            "world",
            Some(json!({"traffic_engine": {"c04_taxi01": {"idle_rpm": 1234}}})),
        )
        .unwrap();
        set(
            &mut world,
            "dev.a",
            "player",
            Some(json!({ field.clone(): changed })),
        )
        .unwrap();
        set(
            &mut world,
            "dev.a",
            "reverb",
            Some(json!({ preset_hex.clone(): {"0": original_presets[0].1.0[0] + 0.25} })),
        )
        .unwrap();
        assert!(
            set(
                &mut world,
                "dev.b",
                "world",
                Some(json!({"traffic_engine": {"c04_taxi01": {"idle_rpm": 99}}}))
            )
            .unwrap_err()
            .contains("owned by another mod")
        );
        set(
            &mut world,
            "dev.b",
            "world",
            Some(json!({"traffic_engine": {"c04_taxi01": {"max_rpm": 5555}}})),
        )
        .unwrap();
        assert_eq!(
            taxi(&world),
            original,
            "nothing changes before the pass boundary"
        );
        apply(&mut world);
        let tuned = taxi(&world);
        assert_eq!(
            (tuned.idle_rpm, tuned.max_rpm, tuned.patch),
            (1234.0, 5555.0, original.patch)
        );
        let player = world
            .resource::<super::super::Native>()
            .player
            .as_ref()
            .map(|p| p.tuning.clone());
        assert_ne!(
            player, original_player,
            "the player's components took the new tuning"
        );
        assert_eq!(player, Some(world.resource::<Library>().player_tuning()));
        let p = presets(&world);
        assert_eq!(
            p[0].1.0[0],
            original_presets[0].1.0[0] + 0.25,
            "the runtime's reverb preset"
        );
        assert_eq!(&p[1..], &original_presets[1..]);
        {
            let t = world.resource::<AudioTuning>();
            let l = world.resource::<Library>();
            assert_eq!(
                t.read(l, "world/traffic_engine/c04_taxi01/idle_rpm"),
                json!(1234)
            );
            assert_eq!(
                t.read(l, &format!("reverb/{preset_hex}/0"))
                    .as_f64()
                    .map(|v| v as f32),
                Some(original_presets[0].1.0[0] + 0.25)
            );
            assert!(
                t.owned("dev.a")
                    .contains(&"world_tuning/traffic_engine/c04_taxi01/idle_rpm".to_owned())
            );
            assert_eq!(
                t.owned("dev.b"),
                ["world_tuning/traffic_engine/c04_taxi01/max_rpm"]
            );
        }
        apply(&mut world);
        assert_eq!(taxi(&world), tuned, "an idle pass changes nothing");
        // nil restores one domain; the mod stopping restores the rest.
        set(&mut world, "dev.b", "world", None).unwrap();
        apply(&mut world);
        assert_eq!(taxi(&world).max_rpm, original.max_rpm);
        world.resource_mut::<AudioTuning>().clear_owner("dev.a");
        apply(&mut world);
        assert_eq!(taxi(&world), original);
        assert_eq!(
            world_lines(world.resource::<Library>()),
            original_world,
            "the loaded world tuning, exactly"
        );
        assert_eq!(
            world
                .resource::<super::super::Native>()
                .player
                .as_ref()
                .map(|p| p.tuning.clone()),
            original_player
        );
        assert_eq!(
            presets(&world),
            original_presets,
            "the retail presets in the runtime"
        );
        assert!(world.resource::<AudioTuning>().owned("dev.a").is_empty());
        // A restart (a new Library, the content generation bumped): the patch is applied again.
        set(
            &mut world,
            "dev.a",
            "world",
            Some(json!({"traffic_engine": {"c04_taxi01": {"idle_rpm": 777}}})),
        )
        .unwrap();
        apply(&mut world);
        world.insert_resource(library());
        world
            .resource_mut::<super::super::AudioContent>()
            .generation += 1;
        assert_eq!(
            taxi(&world),
            original,
            "the new Library holds the loaded values"
        );
        apply(&mut world);
        assert_eq!(
            taxi(&world).idle_rpm,
            777.0,
            "applied again after the restart"
        );
    }
}
