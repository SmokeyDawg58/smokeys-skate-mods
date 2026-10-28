//! Merging audio content overlays into the install's audio manifest (JSON), by retail identity:
//! overlays in the order given (the engine passes mod-id order), the first owner of an identity
//! wins and later claims are reported as conflicts. Pure data: the engine merges, then reads the
//! result as its typed manifest; `check_mod --install` runs the same merge to report unknown
//! identities and conflicts at authoring time.
//!
//! Mod files become `mod:<id>/<path>` references (the install's own paths never contain `:`).
//! Overlay-only data goes into sections the install never has: `mod_sample_loops`, `mod_banks`,
//! `mod_speech`, `mod_location_programs`, `mod_crossfade_layouts`, `mod_maps`.
//!
//! Speech takes are checked against the install's speech indexes when the caller hands them in
//! ([`merge_one_with`], [`SpeechClips`]): an unknown archive or clip, or a take past the clip's
//! own, is a warning and is not merged.
use crate::audio_content::{
    AudioOverlay, BankDef, EmitterDef, LayerSample, SampleFile, SpeechClips, hex_key,
};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// The prefix of a mod file reference in a merged manifest.
pub const MOD_REF: &str = "mod:";

pub fn mod_ref(id: &str, path: &str) -> String {
    format!("{MOD_REF}{id}/{path}")
}

/// `mod:<id>/<path>` → (id, path).
pub fn split_mod_ref(file: &str) -> Option<(&str, &str)> {
    file.strip_prefix(MOD_REF)?.split_once('/')
}

/// One overlay to merge.
pub struct Source<'a> {
    pub id: &'a str,
    pub overlay: &'a AudioOverlay,
}

/// A conflict or warning about one mod's overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub owner: String,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Another mod already owns the identity: this claim is ignored (shown in the mod menu).
    pub conflicts: Vec<Message>,
    /// Unknown identities, adds of existing ones, tuning fields the install lacks: ignored.
    pub warnings: Vec<Message>,
    /// Identities each mod changed.
    pub claimed: BTreeMap<String, String>,
}

/// Identity → owning mod, across the overlays merged so far.
#[derive(Clone, Debug, Default)]
pub struct Owners {
    owners: BTreeMap<String, String>,
}

impl Owners {
    fn owner(&self, identity: &str) -> Option<&str> {
        self.owners.get(identity).map(String::as_str)
    }

    /// Claim an identity. Err = the other owner. Banks and their samples overlap: a sample is
    /// taken while another mod replaces its whole bank, and a bank while another mod replaces one
    /// of its samples.
    fn claim(&mut self, identity: &str, id: &str) -> Result<(), String> {
        let other = self
            .owner(identity)
            .filter(|o| *o != id)
            .map(str::to_owned)
            .or_else(|| {
                if let Some(rest) = identity.strip_prefix("sample:") {
                    let bank = rest.rsplit_once(':').map_or(rest, |(b, _)| b);
                    return self
                        .owner(&format!("bank:{bank}"))
                        .filter(|o| *o != id)
                        .map(str::to_owned);
                }
                if let Some(bank) = identity.strip_prefix("bank:") {
                    let prefix = format!("sample:{bank}:");
                    return self
                        .owners
                        .range(prefix.clone()..)
                        .take_while(|(k, _)| k.starts_with(&prefix))
                        .find(|(_, o)| *o != id)
                        .map(|(_, o)| o.clone());
                }
                None
            });
        if let Some(other) = other {
            return Err(other);
        }
        self.owners.insert(identity.to_owned(), id.to_owned());
        Ok(())
    }

    pub fn all(&self) -> &BTreeMap<String, String> {
        &self.owners
    }
}

struct Ctx<'a, 'r> {
    id: &'a str,
    owners: &'a mut Owners,
    report: &'r mut Report,
    speech: Option<&'a SpeechClips>,
}

impl Ctx<'_, '_> {
    fn claim(&mut self, identity: &str) -> bool {
        match self.owners.claim(identity, self.id) {
            Ok(()) => {
                self.report
                    .claimed
                    .insert(identity.to_owned(), self.id.to_owned());
                true
            }
            Err(other) => {
                self.report.conflicts.push(Message {
                    owner: self.id.to_owned(),
                    text: format!("{identity} is already changed by {other}; ignored"),
                });
                false
            }
        }
    }
    fn warn(&mut self, text: String) {
        self.report.warnings.push(Message {
            owner: self.id.to_owned(),
            text,
        });
    }
    fn file(&self, path: &str) -> Value {
        Value::String(mod_ref(self.id, path))
    }
}

fn section<'v>(m: &'v mut Value, key: &str) -> &'v mut Map<String, Value> {
    let root = m.as_object_mut().expect("the manifest is an object");
    let v = root.entry(key.to_owned()).or_insert_with(|| json!({}));
    if !v.is_object() {
        *v = json!({});
    }
    v.as_object_mut().unwrap()
}

fn sub<'v>(m: &'v mut Map<String, Value>, key: &str) -> &'v mut Map<String, Value> {
    let v = m.entry(key.to_owned()).or_insert_with(|| json!({}));
    if !v.is_object() {
        *v = json!({});
    }
    v.as_object_mut().unwrap()
}

fn has(m: &Value, path: &[&str]) -> bool {
    let mut v = m;
    for k in path {
        match v.get(*k) {
            Some(x) if !x.is_null() => v = x,
            _ => return false,
        }
    }
    true
}

fn emitter_json(e: &EmitterDef, index: u64, sound_id: Option<&str>) -> Value {
    let mut v = serde_json::to_value(e).expect("records serialize");
    let o = v.as_object_mut().unwrap();
    o.insert("index".into(), json!(index));
    o.insert(
        "sound_id".into(),
        json!(e.sound_id.as_deref().or(sound_id).unwrap_or("")),
    );
    o.retain(|_, x| !x.is_null());
    v
}

fn samples_json(ctx: &Ctx, samples: &[SampleFile]) -> Value {
    Value::Array(
        samples
            .iter()
            .map(|s| json!({"file": ctx.file(s.file())}))
            .collect(),
    )
}

fn loops(m: &mut Value, bank: &str, entries: impl Iterator<Item = (usize, u32)>) {
    let mut entries = entries.peekable();
    if entries.peek().is_none() {
        return;
    }
    let l = sub(section(m, "mod_sample_loops"), bank);
    for (slot, start) in entries {
        l.insert(slot.to_string(), json!(start));
    }
}

fn bank_meta(m: &mut Value, stem: &str, b: &BankDef) {
    if b.preload || b.group.is_some() {
        section(m, "mod_banks").insert(
            stem.to_owned(),
            json!({"preload": b.preload, "group": b.group}),
        );
    }
}

/// A location set / zone by key (16 hex digits, any case) or name, in a keyed section.
fn resolve_key(m: &Value, section: &str, key: &str) -> Option<String> {
    let rows = m.get(section)?.as_object()?;
    if hex_key(key) {
        let upper = key.to_ascii_uppercase();
        return rows.contains_key(&upper).then_some(upper);
    }
    rows.iter()
        .find(|(_, v)| v.get("name").and_then(Value::as_str) == Some(key))
        .map(|(k, _)| k.clone())
}

fn pair(a: &str, b: &str) -> String {
    let (a, b) = (a.to_ascii_uppercase(), b.to_ascii_uppercase());
    if a <= b {
        format!("{a}:{b}")
    } else {
        format!("{b}:{a}")
    }
}

fn crossfade_at(m: &Value, from: &str, to: &str) -> Option<usize> {
    let want = pair(from, to);
    m.get("crossfades")?.as_array()?.iter().position(|c| {
        let f = c.get("from").and_then(Value::as_str).unwrap_or("");
        let t = c.get("to").and_then(Value::as_str).unwrap_or("");
        pair(f, t) == want
    })
}

/// The tuning merge: only fields the install has; arrays take decimal index keys; numbers replace
/// numbers, booleans booleans, strings strings, arrays of numbers arrays of numbers.
fn merge_tuning(ctx: &mut Ctx, target: &mut Value, patch: &Value, path: &str) {
    match (target, patch) {
        (Value::Object(t), Value::Object(p)) => {
            for (k, v) in p {
                let at = format!("{path}/{k}");
                match t.get_mut(k) {
                    Some(x) => merge_tuning(ctx, x, v, &at),
                    None => ctx.warn(format!("tuning field {at} is not in the install; ignored")),
                }
            }
        }
        (Value::Array(t), Value::Object(p)) => {
            for (k, v) in p {
                let at = format!("{path}/{k}");
                match k.parse::<usize>().ok().and_then(|i| t.get_mut(i)) {
                    Some(x) => merge_tuning(ctx, x, v, &at),
                    None => ctx.warn(format!("tuning index {at} is not in the install; ignored")),
                }
            }
        }
        (t, p) => {
            let same = matches!(
                (&*t, p),
                (Value::Number(_), Value::Number(_))
                    | (Value::Bool(_), Value::Bool(_))
                    | (Value::String(_), Value::String(_))
            ) || matches!((&*t, p), (Value::Array(a), Value::Array(b)) if a.iter().chain(b).all(Value::is_number) && (a.is_empty() || a.len() == b.len()));
            if !same {
                ctx.warn(format!(
                    "tuning field {path}: {p} does not fit the install's {t}; ignored"
                ));
                return;
            }
            if ctx.claim(&format!("tuning:{path}")) {
                *t = p.clone();
            }
        }
    }
}

/// Merge one overlay into `m` (the manifest JSON) without the speech indexes (speech takes are
/// merged unchecked).
pub fn merge_one(m: &mut Value, source: &Source, owners: &mut Owners, report: &mut Report) {
    merge_one_with(m, source, owners, report, None);
}

/// Merge one overlay into `m`, checking speech takes against the install's speech indexes.
pub fn merge_one_with(
    m: &mut Value,
    source: &Source,
    owners: &mut Owners,
    report: &mut Report,
    speech: Option<&SpeechClips>,
) {
    let mut ctx = Ctx {
        id: source.id,
        owners,
        report,
        speech,
    };
    let o = source.overlay;
    let r = &o.replace;
    // Samples by slot.
    for (bank, slots) in &r.samples {
        let len = m
            .get("banks")
            .and_then(|b| b.get(bank))
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        if len == 0 {
            ctx.warn(format!(
                "replace.samples: bank {bank} is not in the install; ignored"
            ));
            continue;
        }
        let mut loop_rows = Vec::new();
        for (slot, file) in slots {
            let i: usize = slot.parse().unwrap_or(usize::MAX);
            if i >= len {
                ctx.warn(format!(
                    "replace.samples: {bank} has no slot {slot} ({len} samples); ignored"
                ));
                continue;
            }
            if !ctx.claim(&format!("sample:{bank}:{i}")) {
                continue;
            }
            m["banks"][bank][i] = json!({"file": ctx.file(file.file())});
            if let Some(l) = file.loop_start() {
                loop_rows.push((i, l));
            }
        }
        loops(m, bank, loop_rows.into_iter());
    }
    // Whole banks.
    for (stem, b) in &r.banks {
        if !has(m, &["banks", stem]) && !has(m, &["aems", "banks", stem]) {
            ctx.warn(format!(
                "replace.banks: {stem} is not in the install (use add.banks); ignored"
            ));
            continue;
        }
        if !ctx.claim(&format!("bank:{stem}")) {
            continue;
        }
        if !b.samples.is_empty() {
            let v = samples_json(&ctx, &b.samples);
            section(m, "banks").insert(stem.clone(), v);
            loops(
                m,
                stem,
                b.samples
                    .iter()
                    .enumerate()
                    .filter_map(|(i, s)| Some((i, s.loop_start()?))),
            );
        }
        if let Some(abk) = &b.abk {
            let v = ctx.file(abk);
            sub(section(m, "aems"), "banks").insert(stem.clone(), v);
        }
        bank_meta(m, stem, b);
    }
    // Mod Csis projects (doc 16 L4): installed after the install's, in mod-id order (their symbol
    // names are checked against the install's where the project files are read: the game's library
    // and check_mod --install, `audio_content::project_clash`).
    for file in &o.add.projects {
        let v = ctx.file(file);
        match section(m, "aems")
            .entry("projects")
            .or_insert_with(|| Value::Array(Vec::new()))
        {
            Value::Array(list) => list.push(v),
            _ => ctx.warn("add.projects: the install's project list is not a list; ignored".into()),
        }
    }
    for (stem, b) in &o.add.banks {
        if has(m, &["banks", stem]) || has(m, &["aems", "banks", stem]) {
            ctx.warn(format!(
                "add.banks: {stem} is already in the install (use replace.banks); ignored"
            ));
            continue;
        }
        if !ctx.claim(&format!("bank:{stem}")) {
            continue;
        }
        let v = samples_json(&ctx, &b.samples);
        section(m, "banks").insert(stem.clone(), v);
        loops(
            m,
            stem,
            b.samples
                .iter()
                .enumerate()
                .filter_map(|(i, s)| Some((i, s.loop_start()?))),
        );
        if let Some(abk) = &b.abk {
            let v = ctx.file(abk);
            sub(section(m, "aems"), "banks").insert(stem.clone(), v);
        }
        bank_meta(m, stem, b);
    }
    // Files by name.
    for (stem, file) in &r.splice {
        if !has(m, &["aems", "splice", stem]) {
            ctx.warn(format!(
                "replace.splice: {stem} has no Splice tree in the install; ignored"
            ));
        } else if ctx.claim(&format!("splice:{stem}")) {
            m["aems"]["splice"][stem] = ctx.file(file);
        }
    }
    for (name, g) in &r.grains {
        if !has(m, &["grains", name]) {
            ctx.warn(format!(
                "replace.grains: {name} is not in the install; ignored"
            ));
        } else if ctx.claim(&format!("grain:{name}")) {
            m["grains"][name]["file"] = ctx.file(&g.file);
            m["grains"][name]["grain"] = ctx.file(&g.grain);
        }
    }
    for (kind, rows) in [("wheels", &r.wheels), ("ambience", &r.ambience)] {
        for (name, file) in rows {
            if !has(m, &[kind, name]) {
                ctx.warn(format!(
                    "replace.{kind}: {name} is not in the install; ignored"
                ));
            } else if ctx.claim(&format!("{kind}:{name}")) {
                m[kind][name]["file"] = ctx.file(file);
            }
        }
    }
    if let Some(file) = &r.mixmap {
        if !has(m, &["aems", "mixmap"]) {
            ctx.warn("replace.mixmap: the install has no MixMap; ignored".into());
        } else if ctx.claim("mixmap") {
            m["aems"]["mixmap"] = ctx.file(file);
        }
    }
    // Emitter records.
    for (file, recs) in &r.emitters {
        for (idx, e) in recs {
            let index: u64 = idx.parse().unwrap_or(u64::MAX);
            let at = m
                .get("emitters")
                .and_then(|x| x.get(file))
                .and_then(Value::as_array)
                .and_then(|rows| {
                    rows.iter()
                        .position(|row| row.get("index").and_then(Value::as_u64) == Some(index))
                });
            let Some(at) = at else {
                ctx.warn(format!(
                    "replace.emitters: {file} has no record {idx}; ignored"
                ));
                continue;
            };
            if !ctx.claim(&format!("emitter:{file}:{index}")) {
                continue;
            }
            let old = m["emitters"][file][at]
                .get("sound_id")
                .and_then(Value::as_str)
                .map(str::to_owned);
            m["emitters"][file][at] = emitter_json(e, index, old.as_deref());
        }
    }
    for (file, recs) in &o.add.emitters {
        let rows = section(m, "emitters")
            .entry(file.clone())
            .or_insert_with(|| json!([]));
        if !rows.is_array() {
            *rows = json!([]);
        }
        let rows = rows.as_array_mut().unwrap();
        let mut next = rows
            .iter()
            .filter_map(|r| r.get("index").and_then(Value::as_u64))
            .max()
            .map_or(0, |i| i + 1);
        for e in recs {
            rows.push(emitter_json(e, next, None));
            next += 1;
        }
    }
    // Location sets and zones.
    for (sec, rows, add) in [
        (
            "random_sets",
            &serde_json::to_value(&r.random_sets).unwrap(),
            false,
        ),
        ("zones", &serde_json::to_value(&r.zones).unwrap(), false),
        (
            "random_sets",
            &serde_json::to_value(&o.add.random_sets).unwrap(),
            true,
        ),
        ("zones", &serde_json::to_value(&o.add.zones).unwrap(), true),
    ] {
        let what = if add { "add" } else { "replace" };
        for (key, def) in rows.as_object().into_iter().flatten() {
            let found = resolve_key(m, sec, key);
            let key = match (found, add) {
                (Some(k), false) => k,
                (None, true) => key.to_ascii_uppercase(),
                (None, false) => {
                    ctx.warn(format!(
                        "{what}.{sec}: {key} is not in the install; ignored"
                    ));
                    continue;
                }
                (Some(k), true) => {
                    ctx.warn(format!(
                        "{what}.{sec}: {k} is already in the install (use replace); ignored"
                    ));
                    continue;
                }
            };
            if ctx.claim(&format!("{sec}:{key}")) {
                let mut def = def.clone();
                def.as_object_mut().unwrap().retain(|_, x| !x.is_null());
                // A replacement keeps the record's name unless it gives one (the name is how
                // maps and other mods find it).
                if let Some(name) = m
                    .get(sec)
                    .and_then(|s| s.get(&key))
                    .and_then(|r| r.get("name"))
                    .cloned()
                {
                    def.as_object_mut().unwrap().entry("name").or_insert(name);
                }
                section(m, sec).insert(key, def);
            }
        }
    }
    // Crossfades by zone pair.
    for (c, add) in r
        .crossfades
        .iter()
        .map(|c| (c, false))
        .chain(o.add.crossfades.iter().map(|c| (c, true)))
    {
        let at = crossfade_at(m, &c.from, &c.to);
        let row = json!({"from": c.from.to_ascii_uppercase(), "to": c.to.to_ascii_uppercase(), "group": c.group, "level": c.level});
        match (at, add) {
            (Some(i), false) => {
                if ctx.claim(&format!("crossfade:{}", pair(&c.from, &c.to))) {
                    m["crossfades"][i] = row;
                }
            }
            (None, true) => {
                if ctx.claim(&format!("crossfade:{}", pair(&c.from, &c.to))) {
                    let rows = m
                        .as_object_mut()
                        .unwrap()
                        .entry("crossfades")
                        .or_insert_with(|| json!([]));
                    if let Some(a) = rows.as_array_mut() {
                        a.push(row);
                    }
                }
            }
            (None, false) => ctx.warn(format!(
                "replace.crossfades: no crossfade {} ↔ {} in the install; ignored",
                c.from, c.to
            )),
            (Some(_), true) => ctx.warn(format!(
                "add.crossfades: {} ↔ {} is already in the install (use replace); ignored",
                c.from, c.to
            )),
        }
    }
    // Speech takes, checked against the speech indexes when given.
    for (archive, clips) in &r.speech {
        for (clip, takes) in clips {
            let clip = clip.strip_suffix(".dat").unwrap_or(clip);
            let Some(count) = speech_clip(&mut ctx, "replace", archive, clip) else {
                continue;
            };
            for (take, file) in takes {
                if let Some(count) = count {
                    if take.parse::<usize>().is_ok_and(|t| t >= count) {
                        ctx.warn(format!("replace.speech.{archive}.{clip}: take {take} is past the clip's {count} takes (0..{}); ignored (add.speech adds takes)", count.saturating_sub(1)));
                        continue;
                    }
                }
                if !ctx.claim(&format!("speech:{archive}:{clip}:{take}")) {
                    continue;
                }
                let v = ctx.file(file);
                sub(sub(sub(section(m, "mod_speech"), archive), "takes"), clip)
                    .insert(take.clone(), v);
            }
        }
    }
    for (archive, clips) in &o.add.speech {
        for (clip, takes) in clips {
            let clip = clip.strip_suffix(".dat").unwrap_or(clip);
            if speech_clip(&mut ctx, "add", archive, clip).is_none() {
                continue;
            }
            let files: Vec<Value> = takes.iter().map(|f| ctx.file(f)).collect();
            let extra = sub(sub(section(m, "mod_speech"), archive), "extra");
            let row = extra.entry(clip.to_owned()).or_insert_with(|| json!([]));
            if let Some(a) = row.as_array_mut() {
                a.extend(files);
            }
        }
    }
    for (bank, layers) in &o.add.location_programs {
        if ctx.claim(&format!("program:{bank}")) {
            let rows: Vec<Value> = layers.iter().map(|l| json!({
                "delay": l.delay,
                "sample": match &l.sample { LayerSample::Slot(s) => json!(s), LayerSample::Named(_) => json!("shuffle") },
                "level": l.level, "pan_sweep": l.pan_sweep, "looping": l.looping,
            })).collect();
            section(m, "mod_location_programs").insert(bank.clone(), Value::Array(rows));
        }
    }
    // Declared crossfade layouts (for banks without a crossfade program).
    for (bank, groups) in &o.add.crossfade_layouts {
        let len = m
            .get("banks")
            .and_then(|b| b.get(bank))
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        if len == 0 {
            ctx.warn(format!("add.crossfade_layouts: bank {bank} has no samples in the install or the overlay; ignored"));
            continue;
        }
        if let Some((group, v)) = groups
            .iter()
            .find_map(|(g, vs)| vs.iter().find(|v| v.sample as usize >= len).map(|v| (g, v)))
        {
            ctx.warn(format!("add.crossfade_layouts.{bank}.{group}: sample {} is past the bank's {len} samples; ignored", v.sample));
            continue;
        }
        if ctx.claim(&format!("crossfade_layout:{bank}")) {
            let rows: Map<String, Value> = groups
                .iter()
                .map(|(g, vs)| {
                    (
                        g.parse::<u32>()
                            .map_or_else(|_| g.clone(), |g| g.to_string()),
                        Value::Array(
                            vs.iter()
                                .map(
                                    |v| json!({"sample": v.sample, "pan": v.pan, "level": v.level}),
                                )
                                .collect(),
                        ),
                    )
                })
                .collect();
            section(m, "mod_crossfade_layouts").insert(bank.clone(), Value::Object(rows));
        }
    }
    // Tuning field merges.
    for (name, patch) in [
        ("player_tuning", &o.tuning.player),
        ("world_tuning", &o.tuning.world),
        ("bus_tuning", &o.tuning.bus),
        ("grain_player", &o.tuning.grain),
    ] {
        let Some(patch) = patch else { continue };
        let Some(target) = m.get_mut(name) else {
            ctx.warn(format!("tuning: the install has no {name}; ignored"));
            continue;
        };
        let mut target = std::mem::take(target);
        merge_tuning(&mut ctx, &mut target, patch, name);
        m[name] = target;
    }
    // Map audio.
    for (stem, def) in &o.maps {
        let fields: [(&str, Option<Value>); 4] = [
            ("district", def.district.as_ref().map(|v| json!(v))),
            ("ems", def.ems.as_ref().map(|v| json!(v))),
            (
                "crossfade_bank",
                def.crossfade_bank.as_ref().map(|v| json!(v)),
            ),
            ("fallback_bed", def.fallback_bed.as_ref().map(|v| json!(v))),
        ];
        for (field, value) in fields {
            if let Some(value) = value {
                if ctx.claim(&format!("map:{stem}:{field}")) {
                    sub(section(m, "mod_maps"), stem).insert(field.to_owned(), value);
                }
            }
        }
        let map = sub(section(m, "mod_maps"), stem);
        if !def.emitters.is_empty() {
            let rows = map.entry("emitters").or_insert_with(|| json!([]));
            let start = rows.as_array().map_or(0, Vec::len) as u64;
            if let Some(a) = rows.as_array_mut() {
                a.extend(
                    def.emitters
                        .iter()
                        .enumerate()
                        .map(|(i, e)| emitter_json(e, start + i as u64, None)),
                );
            }
        }
        if !def.regions.is_empty() {
            let regions = sub(map, "regions");
            for (layer, boxes) in &def.regions {
                let rows = regions.entry(layer.clone()).or_insert_with(|| json!([]));
                if let Some(a) = rows.as_array_mut() {
                    a.extend(boxes.iter().map(|b| serde_json::to_value(b).unwrap()));
                }
            }
        }
    }
}

/// Merge every overlay in order.
pub fn merge(m: &mut Value, sources: &[Source]) -> Report {
    merge_with(m, sources, None)
}

/// Merge every overlay in order, checking speech takes against the install's speech indexes.
pub fn merge_with(m: &mut Value, sources: &[Source], speech: Option<&SpeechClips>) -> Report {
    let mut owners = Owners::default();
    let mut report = Report::default();
    for s in sources {
        merge_one_with(m, s, &mut owners, &mut report, speech);
    }
    report
}

/// Check a speech clip against the indexes: `None` = skip it (warned), `Some(None)` = merge
/// unchecked (no indexes given), `Some(Some(n))` = the clip has `n` takes.
fn speech_clip(ctx: &mut Ctx, what: &str, archive: &str, clip: &str) -> Option<Option<usize>> {
    let Some(speech) = ctx.speech else {
        return Some(None);
    };
    let Some(clips) = speech.archive(archive) else {
        ctx.warn(format!("{what}.speech: the install has no {archive} speech index (set up speech); {archive} takes ignored"));
        return None;
    };
    match clips.get(clip) {
        Some(&n) => Some(Some(n)),
        None => {
            let hint = speech
                .suggest(archive, clip)
                .map_or(String::new(), |s| format!(" (did you mean {s}?)"));
            ctx.warn(format!("{what}.speech.{archive}: clip {clip} is not in the install's speech index{hint}; ignored"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install() -> Value {
        json!({
            "version": 5,
            "ambience": {"04_dt_main": {"file": "ambience/04_dt_main.wav", "seconds": 100.0}},
            "grains": {"wood_ramp_hard": {"file": "grains/w.wav", "grain": "grains/w.grain"}},
            "wheels": {"Whls_spins_Jump_1": {"file": "wheels/j.wav"}},
            "banks": {"Skate_Collisions": [{"file": "banks/c/0.wav"}, {"file": "banks/c/1.wav"}], "C04_taxi01": [{"file": "banks/t/0.wav"}]},
            "emitters": {"sfx_downtown": [{"index": 9, "flags": 0, "position": [0, 0, 0], "extent": [1, 1, 1], "scalars": [0, 1, 0, 0], "sound_id": "A0C12188AFCC33E0", "bank": "Baby_Cry_1"}]},
            "random_sets": {"7EE1991E4909C50E": {"name": "e_dwtn_spillway_brewery", "sounds": []}},
            "zones": {"DB21C7A69325F3DF": {"name": "int_tunnel", "bed": "22_interior_tunnel_amb"}},
            "crossfades": [{"from": "1F741D87EB58E84F", "to": "B6CE4BDEB63B6639", "group": 1, "level": 1.5}],
            "aems": {"projects": ["aems/a.csi"], "banks": {"C04_taxi01": "aems/C04_taxi01.abk"}, "mixmap": "aems/MixMapSK8.mxb", "splice": {"Skate_Collisions": "aems/c.splc"}},
            "player_tuning": {"grind": [{"v": [1, 1, 1, 1], "f": [1, 1, 1, 1]}], "wheel_bucket_high": 3.0},
            "world_tuning": {"traffic_engine": {"c04_taxi01": {"idle_rpm": 850.0, "patch": 2}}}
        })
    }

    fn overlay(v: Value) -> AudioOverlay {
        let o: AudioOverlay = serde_json::from_value(v).unwrap();
        o.validate().unwrap();
        o
    }

    #[test]
    fn no_overlays_leave_the_manifest_unchanged() {
        let mut m = install();
        let r = merge(&mut m, &[]);
        assert_eq!(m, install());
        assert!(r.conflicts.is_empty() && r.warnings.is_empty());
        let empty = overlay(json!({"version": 1}));
        merge(
            &mut m,
            &[Source {
                id: "a",
                overlay: &empty,
            }],
        );
        assert_eq!(m, install(), "an empty overlay changes nothing");
    }

    #[test]
    fn every_section_merges_by_identity() {
        let o = overlay(json!({"version": 1,
            "replace": {
                "samples": {"Skate_Collisions": {"1": {"file": "audio/pop.wav", "loop_start": 5}}},
                "banks": {"C04_taxi01": {"abk": "audio/taxi.abk", "samples": ["audio/t0.wav"], "group": "world"}},
                "splice": {"Skate_Collisions": "audio/c.splc"},
                "grains": {"wood_ramp_hard": {"file": "audio/w.wav", "grain": "audio/w.grain"}},
                "wheels": {"Whls_spins_Jump_1": "audio/j.wav"},
                "ambience": {"04_dt_main": "audio/bed.wav"},
                "mixmap": "audio/m.mxb",
                "emitters": {"sfx_downtown": {"9": {"position": [1, 2, 3], "extent": [4, 4, 4], "bank": "MOD_x"}}},
                "random_sets": {"e_dwtn_spillway_brewery": {"sounds": [{"bank": "MOD_x"}]}},
                "zones": {"db21c7a69325f3df": {"bed": "04_dt_main"}},
                "crossfades": [{"from": "B6CE4BDEB63B6639", "to": "1F741D87EB58E84F", "group": 3}],
                "speech": {"livingworld": {"501_41_adtm1_Warn_n.dat": {"2": "audio/s.wav"}}}
            },
            "add": {
                "banks": {"MOD_x": {"abk": "audio/x.abk", "samples": ["audio/x0.wav"], "preload": true}},
                "emitters": {"sfx_downtown": [{"position": [0, 0, 0], "extent": [2, 2, 2], "bank": "MOD_x"}], "sfx_new": [{"position": [0, 0, 0], "extent": [2, 2, 2], "bank": "MOD_x"}]},
                "random_sets": {"00000000000000AA": {"sounds": [{"bank": "MOD_x"}]}},
                "crossfades": [{"from": "00000000000000AA", "to": "00000000000000BB", "group": 2}],
                "speech": {"livingworld": {"501_41_adtm1_Warn_n": ["audio/e.wav"]}},
                "crossfade_layouts": {"MOD_x": {"02": [{"sample": 0, "pan": 45}, {"sample": 0, "pan": 225, "level": 0.7}]}},
                "location_programs": {"MOD_x": [{"sample": "shuffle"}]}
            },
            "tuning": {"player": {"grind": {"0": {"v": [0.5, 0.5, 0.5, 0.5]}}, "wheel_bucket_high": 4}, "world": {"traffic_engine": {"c04_taxi01": {"idle_rpm": 900}}}},
            "maps": {"MyMap": {"district": "DownTown", "emitters": [{"position": [0, 0, 0], "extent": [2, 2, 2], "bank": "MOD_x"}],
                               "regions": {"audio_ambience": [{"box": [0, 0, 5, 5], "key": "int_tunnel"}]}}}
        }));
        let mut m = install();
        let r = merge(
            &mut m,
            &[Source {
                id: "me",
                overlay: &o,
            }],
        );
        assert!(r.conflicts.is_empty(), "{:?}", r.conflicts);
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert_eq!(
            m["banks"]["Skate_Collisions"][1]["file"],
            "mod:me/audio/pop.wav"
        );
        assert_eq!(
            m["banks"]["Skate_Collisions"][0]["file"], "banks/c/0.wav",
            "other slots stay"
        );
        assert_eq!(m["mod_sample_loops"]["Skate_Collisions"]["1"], 5);
        assert_eq!(m["aems"]["banks"]["C04_taxi01"], "mod:me/audio/taxi.abk");
        assert_eq!(m["mod_banks"]["C04_taxi01"]["group"], "world");
        assert_eq!(m["aems"]["banks"]["MOD_x"], "mod:me/audio/x.abk");
        assert_eq!(m["mod_banks"]["MOD_x"]["preload"], true);
        assert_eq!(
            m["aems"]["splice"]["Skate_Collisions"],
            "mod:me/audio/c.splc"
        );
        assert_eq!(
            m["grains"]["wood_ramp_hard"]["grain"],
            "mod:me/audio/w.grain"
        );
        assert_eq!(
            m["wheels"]["Whls_spins_Jump_1"]["file"],
            "mod:me/audio/j.wav"
        );
        assert_eq!(m["ambience"]["04_dt_main"]["file"], "mod:me/audio/bed.wav");
        assert_eq!(
            m["ambience"]["04_dt_main"]["seconds"], 100.0,
            "other fields stay"
        );
        assert_eq!(m["aems"]["mixmap"], "mod:me/audio/m.mxb");
        let rows = m["emitters"]["sfx_downtown"].as_array().unwrap();
        assert_eq!(
            (
                rows[0]["index"].as_u64(),
                rows[0]["bank"].as_str(),
                rows[0]["sound_id"].as_str()
            ),
            (Some(9), Some("MOD_x"), Some("A0C12188AFCC33E0"))
        );
        assert_eq!(rows[1]["index"], 10, "appended after the last index");
        assert_eq!(m["emitters"]["sfx_new"][0]["index"], 0);
        assert_eq!(
            m["random_sets"]["7EE1991E4909C50E"]["sounds"][0]["bank"], "MOD_x",
            "by name"
        );
        assert_eq!(
            m["zones"]["DB21C7A69325F3DF"]["bed"], "04_dt_main",
            "by key, any case"
        );
        assert!(m["random_sets"]["00000000000000AA"].is_object());
        assert_eq!(m["crossfades"][0]["group"], 3, "either order");
        assert_eq!(m["crossfades"].as_array().unwrap().len(), 2);
        assert_eq!(
            m["mod_speech"]["livingworld"]["takes"]["501_41_adtm1_Warn_n"]["2"],
            "mod:me/audio/s.wav"
        );
        assert_eq!(
            m["mod_speech"]["livingworld"]["extra"]["501_41_adtm1_Warn_n"][0],
            "mod:me/audio/e.wav"
        );
        assert_eq!(
            m["mod_crossfade_layouts"]["MOD_x"]["2"][1],
            json!({"sample": 0, "pan": 225.0, "level": 0.7f32}),
            "group keys normalised"
        );
        assert_eq!(m["mod_location_programs"]["MOD_x"][0]["sample"], "shuffle");
        assert_eq!(
            m["player_tuning"]["grind"][0]["v"],
            json!([0.5, 0.5, 0.5, 0.5])
        );
        assert_eq!(m["player_tuning"]["grind"][0]["f"], json!([1, 1, 1, 1]));
        assert_eq!(m["player_tuning"]["wheel_bucket_high"], 4);
        assert_eq!(
            m["world_tuning"]["traffic_engine"]["c04_taxi01"]["idle_rpm"],
            900
        );
        assert_eq!(m["mod_maps"]["MyMap"]["district"], "DownTown");
        assert_eq!(
            m["mod_maps"]["MyMap"]["regions"]["audio_ambience"][0]["key"],
            "int_tunnel"
        );
        assert!(r.claimed.keys().any(|k| k == "sample:Skate_Collisions:1"));
    }

    #[test]
    fn the_first_mod_wins_and_later_claims_are_conflicts() {
        let a = overlay(
            json!({"version": 1, "replace": {"samples": {"Skate_Collisions": {"1": "a.wav"}}, "ambience": {"04_dt_main": "a.wav"}}}),
        );
        let b = overlay(
            json!({"version": 1, "replace": {"samples": {"Skate_Collisions": {"1": "b.wav", "0": "b.wav"}}, "banks": {"Skate_Collisions": {"samples": ["b.wav"]}}}}),
        );
        let mut m = install();
        let r = merge(
            &mut m,
            &[
                Source {
                    id: "a.first",
                    overlay: &a,
                },
                Source {
                    id: "b.second",
                    overlay: &b,
                },
            ],
        );
        assert_eq!(
            m["banks"]["Skate_Collisions"][1]["file"],
            "mod:a.first/a.wav"
        );
        assert_eq!(
            m["banks"]["Skate_Collisions"][0]["file"], "mod:b.second/b.wav",
            "an unclaimed slot goes to the second mod"
        );
        assert_eq!(
            m["banks"]["Skate_Collisions"].as_array().unwrap().len(),
            2,
            "the whole-bank replace lost to a's slot"
        );
        assert_eq!(r.conflicts.len(), 2, "{:?}", r.conflicts);
        assert!(
            r.conflicts
                .iter()
                .all(|c| c.owner == "b.second" && c.text.contains("a.first"))
        );
    }

    #[test]
    fn unknown_identities_are_warnings_never_errors() {
        let o = overlay(json!({"version": 1,
            "replace": {"samples": {"Nope": {"0": "x.wav"}, "Skate_Collisions": {"7": "x.wav"}}, "banks": {"Nope": {"samples": ["x.wav"]}},
                        "zones": {"no_zone": {}}, "emitters": {"sfx_downtown": {"3": {"position": [0, 0, 0], "extent": [1, 1, 1], "bank": "b"}}}},
            "add": {"banks": {"C04_taxi01": {"samples": ["x.wav"]}}, "random_sets": {"7EE1991E4909C50E": {"sounds": []}}},
            "tuning": {"player": {"nope": 1, "wheel_bucket_high": "loud", "grind": {"5": {"v": [1, 1, 1, 1]}}}}}));
        let mut m = install();
        let r = merge(
            &mut m,
            &[Source {
                id: "x",
                overlay: &o,
            }],
        );
        assert_eq!(r.warnings.len(), 10, "{:#?}", r.warnings);
        assert!(r.conflicts.is_empty());
        assert_eq!(m, install(), "nothing applied");
    }

    fn speech_clips() -> SpeechClips {
        let mut s = SpeechClips::default();
        s.archives.insert(
            "livingworld".into(),
            [
                ("501_41_adtm1_Warn_n".to_owned(), 3),
                ("101_41_adtm1_SpecPos_f".to_owned(), 5),
            ]
            .into_iter()
            .collect(),
        );
        s
    }

    /// Deep speech checks (with the install's speech indexes): unknown clips and archives and takes
    /// past a clip's own are warnings with a hint and are not merged; known ones merge as before.
    #[test]
    fn speech_takes_are_checked_against_the_speech_index() {
        let o = overlay(json!({"version": 1,
            "replace": {"speech": {
                "livingworld": {"501_41_adtm1_Warn_n.dat": {"2": "a.wav", "3": "b.wav"}, "501_41_ADTM1_warn_n": {"0": "c.wav"}, "501_41_adtm1_Warn_x": {"0": "d.wav"}},
                "maincast": {"700_1_Line": {"0": "e.wav"}}}},
            "add": {"speech": {"livingworld": {"101_41_adtm1_SpecPos_f": ["f.wav"], "999_1_Nope": ["g.wav"]}}}}));
        let mut m = install();
        let speech = speech_clips();
        let r = merge_with(
            &mut m,
            &[Source {
                id: "me",
                overlay: &o,
            }],
            Some(&speech),
        );
        let texts: Vec<&str> = r.warnings.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(texts.len(), 5, "{texts:#?}");
        assert!(
            texts
                .iter()
                .any(|t| t.contains("take 3 is past the clip's 3 takes (0..2)")),
            "{texts:#?}"
        );
        assert!(texts.iter().any(|t| t.contains("clip 501_41_ADTM1_warn_n is not in the install's speech index (did you mean 501_41_adtm1_Warn_n?)")), "{texts:#?}");
        assert!(
            texts.iter().any(|t| t.contains("clip 501_41_adtm1_Warn_x")
                && t.contains("did you mean 501_41_adtm1_Warn_n?")),
            "{texts:#?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("no maincast speech index")),
            "{texts:#?}"
        );
        assert!(texts.iter().any(|t| t.contains("add.speech.livingworld: clip 999_1_Nope is not in the install's speech index; ignored")), "{texts:#?}");
        assert_eq!(
            m["mod_speech"]["livingworld"]["takes"]["501_41_adtm1_Warn_n"],
            json!({"2": "mod:me/a.wav"})
        );
        assert_eq!(
            m["mod_speech"]["livingworld"]["extra"],
            json!({"101_41_adtm1_SpecPos_f": ["mod:me/f.wav"]})
        );
        assert!(m["mod_speech"].get("maincast").is_none());
        // Without the indexes (unit merges) speech merges unchecked, as before.
        let mut m = install();
        let r = merge(
            &mut m,
            &[Source {
                id: "me",
                overlay: &o,
            }],
        );
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    }

    /// A declared crossfade layout needs the bank's samples (install or overlay) and slots inside it.
    #[test]
    fn crossfade_layouts_need_the_banks_samples() {
        let o = overlay(json!({"version": 1, "add": {
            "banks": {"MOD_fade": {"samples": ["a.wav", "b.wav"]}},
            "crossfade_layouts": {
                "MOD_fade": {"1": [{"sample": 1, "pan": 45}, {"sample": 0, "pan": 315, "level": 0.5}]},
                "Skate_Collisions": {"1": [{"sample": 2}]},
                "Nope": {"1": [{"sample": 0}]}}}}));
        let mut m = install();
        let r = merge(
            &mut m,
            &[Source {
                id: "me",
                overlay: &o,
            }],
        );
        assert_eq!(r.warnings.len(), 2, "{:?}", r.warnings);
        assert!(
            r.warnings[0].text.contains("bank Nope has no samples"),
            "{:?}",
            r.warnings
        );
        assert!(
            r.warnings[1]
                .text
                .contains("sample 2 is past the bank's 2 samples"),
            "{:?}",
            r.warnings
        );
        assert_eq!(m["mod_crossfade_layouts"]["MOD_fade"]["1"][1]["pan"], 315.0);
        assert!(m["mod_crossfade_layouts"].get("Skate_Collisions").is_none());
        assert_eq!(r.claimed["crossfade_layout:MOD_fade"], "me");
    }

    #[test]
    fn mod_references_round_trip() {
        assert_eq!(
            split_mod_ref(&mod_ref("dev.x", "audio/a.wav")),
            Some(("dev.x", "audio/a.wav"))
        );
        assert_eq!(split_mod_ref("banks/a.wav"), None);
    }
}
