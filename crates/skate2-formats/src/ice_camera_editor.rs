//! Editable ICELITE 2.96 takes used by Replay2, distinct from cinematic ICE.
//! Native owners: ICELITETake, ICELITEAuthor and ICELITEController.
use super::{CameraSet, Channel, Element, Mover, Result, Spaces, Take, chunk, clamp_parameter, pack, round_int, word};
#[path = "ice_camera_editor_elements.rs"]
mod descriptors;
use descriptors::{CINEMATIC_IDS, ELEMENTS};
#[path = "ice_camera_author.rs"]
mod author;
pub use author::{Author, CameraKind};

pub const CHANNEL_COUNT: usize = 34;
pub const ELEMENT_COUNT: usize = 135;
pub const CAMERA: usize = 0;
pub const LENS: usize = 1;
pub const SHAKE: usize = 4;
pub const SPEED: usize = 5;

pub mod element {
    pub const EYE: usize = 0;
    pub const LOOK: usize = 3;
    pub const LENS: usize = 9;
    pub const AUX_LENS: usize = 10;
    pub const NEAR_CLIP: usize = 14;
    pub const SHAKE_AMPLITUDE: usize = 23;
    pub const SHAKE_FREQUENCY: usize = 24;
    pub const SPEED: usize = 25;
    pub const CUBIC_EYE: usize = 84;
    pub const CUBIC_LOOK: usize = 85;
    pub const EYE_SPACE: usize = 86;
    pub const LOOK_SPACE: usize = 87;
    pub const EYE_AVATAR: usize = 88;
    pub const LOOK_AVATAR: usize = 89;
    pub const ORBIT_AVATAR: usize = 90;
    pub const AVOID_DYNAMIC: usize = 91;
    pub const AVOID_STATIC: usize = 92;
    pub const LENS_OVERRIDE: usize = 98;
    pub const SHAKE_TYPE: usize = 102;
}

/// Mutable packed take, with the native cursor and decoded evaluation cache.
/// Mutations retain the original quantisation, key sharing and channel ordering.
#[derive(Clone, Debug)]
pub struct EditableTake {
    take: Take,
    parameter: f32,
    values: [f64; super::ELEMENT_COUNT],
    intervals: [Option<usize>; CHANNEL_COUNT],
}

impl EditableTake {
    /// `ICELITETake::NewEditBuffer` 82a5b2c8 / TakeData::Init82a55ab0.
    /// Scene connection replaces the initial one-second duration.
    #[must_use]
    pub fn new(length: f32) -> Self {
        Self::from_take(Take {
            length,
            schema: &ELEMENTS,
            channels: vec![Channel::default(); CHANNEL_COUNT],
            data: vec![Vec::new(); ELEMENT_COUNT],
        })
    }

    fn from_take(take: Take) -> Self {
        let mut out = Self { take, parameter: 0.0, values: [0.0; super::ELEMENT_COUNT], intervals: [None; CHANNEL_COUNT] };
        out.set_parameter_force(0.0);
        out
    }

    /// Native 156-byte header followed by indices, parameters and element data.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let (take, used) = Take::parse_schema(bytes, &ELEMENTS, CHANNEL_COUNT)?;
        if used != bytes.len() { return Err("trailing ICELITE take data".into()); }
        if take.channels.iter().any(|c| c.keys > 199 || c.intervals > 198) {
            return Err("ICELITE edit buffer capacity exceeded".into());
        }
        Ok(Self::from_take(take))
    }

    /// `ICELITETakeData::ComputeActualSize` / SetDataPointers: portable payload
    /// has zero link/group pointers; original little-endian scalars and packed
    /// big-endian element bits are preserved.
    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = vec![0u8; 156];
        out[12..16].copy_from_slice(&self.take.length.to_le_bytes());
        out[16..20].copy_from_slice(&1u32.to_le_bytes());
        for (i, c) in self.take.channels.iter().enumerate() {
            out[20+4*i..22+4*i].copy_from_slice(&(c.intervals as u16).to_le_bytes());
            out[22+4*i..24+4*i].copy_from_slice(&(c.keys as u16).to_le_bytes());
        }
        for c in &self.take.channels {
            for &key in &c.key_indices { out.extend_from_slice(&key.to_le_bytes()); }
        }
        for c in &self.take.channels {
            for &parameter in &c.params { out.extend_from_slice(&parameter.to_le_bytes()); }
        }
        out.resize((out.len()+3)&!3, 0);
        for e in &ELEMENTS {
            let values = &self.take.data[e.id];
            let base = out.len();
            out.resize(base + ((e.bits * values.len() + 31) >> 3 & !3), 0);
            for (i, &v) in values.iter().enumerate() { encode_into(*e, &mut out[base..], i, v); }
        }
        out
    }

    #[must_use]
    pub fn length(&self) -> f32 { self.take.length }
    #[must_use]
    pub fn parameter(&self) -> f32 { self.parameter }
    #[must_use]
    pub fn value(&self, id: usize) -> f32 { self.values[id] as f32 }
    #[must_use]
    pub fn value_int(&self, id: usize) -> i32 {
        if ELEMENTS[id].kind >= 3 { round_int(self.value(id)) as i32 } else { self.values[id] as i32 }
    }
    #[must_use]
    pub fn key_value(&self, id: usize, index: usize) -> f32 { self.take.value_float(id, index) }
    #[must_use]
    pub fn key_count(&self, channel: usize) -> usize { self.take.channels[channel].keys }
    #[must_use]
    pub fn interval_count(&self, channel: usize) -> usize { self.take.channels[channel].intervals }
    #[must_use]
    pub fn interval_parameter(&self, channel: usize, interval: usize) -> f32 {
        self.take.channels[channel].interval_parameter(interval)
    }
    #[must_use]
    pub fn key_index(&self, channel: usize, interval: usize) -> usize {
        self.take.channels[channel].key_index(interval)
    }
    #[must_use]
    pub fn current_interval(&self, channel: usize) -> Option<usize> {
        self.intervals[channel]
    }

    /// `ICELITETake::SetParameter`82a5a150 and Channel::SetParameter82a55678.
    /// While the editor is on, both ends of the current interval are inclusive.
    /// This keeps an edited end key on its own side of a camera cut. A fresh
    /// lookup uses the binary search's right side of an equal packed boundary.
    pub fn set_parameter(&mut self, parameter: f32) {
        if self.parameter==clamp_parameter(parameter) {return;}
        self.set_parameter_force(parameter);
    }
    /// Key-state changes and buffer mutations force a refresh even when the
    /// cursor has not moved. Ordinary zero-delta scrubs retain unquantized edits.
    pub fn set_parameter_force(&mut self, parameter: f32) {
        self.parameter = clamp_parameter(parameter);
        for channel in 0..CHANNEL_COUNT {
            let c = &self.take.channels[channel];
            let retained = self.intervals[channel].filter(|&i| i < c.intervals).and_then(|i| {
                let (start, end) = (c.interval_parameter(i), c.interval_parameter(i+1));
                (pack(self.parameter) >= pack(start) && pack(self.parameter) <= pack(end)).then_some((i, start, end))
            });
            let located = retained.or_else(|| c.locate(self.parameter));
            self.intervals[channel] = self.take.evaluate_interval(channel, self.parameter, located, &mut self.values);
        }
    }

    /// `ICELITETake::SetValue`82a55b50 / ElementDesc::SetValue82a35b30.
    pub fn set_value(&mut self, id: usize, index: usize, value: f32) -> Result<()> {
        let e = *ELEMENTS.get(id).ok_or("ICELITE element outside schema")?;
        let target = self.take.data[id].get_mut(index).ok_or("ICELITE key outside channel")?;
        *target = quantize(e, f64::from(value));
        Ok(())
    }

    /// Author::SetKeyElementValue82a60db8 preserves the uncompressed edit
    /// value. Repeated sub-quantum edits must accumulate before encoding.
    fn edit_value(&mut self, id: usize, index: usize, value: f32) -> Result<()> {
        self.set_value(id, index, value)?;
        self.values[id] = f64::from(value);
        Ok(())
    }

    /// `ICELITEChannel::IsHardCut`82a36258: the outer ends are always hard;
    /// inner ends are hard exactly when the neighbouring intervals do not share
    /// their adjacent key. `end` selects the right edge.
    #[must_use]
    pub fn is_hard_cut(&self, channel: usize, interval: usize, end: bool) -> bool {
        let c = &self.take.channels[channel];
        let neighbour = if end { interval.checked_add(1) } else { interval.checked_sub(1) };
        let Some(neighbour) = neighbour.filter(|&n| n < c.intervals) else { return true };
        c.key_index(interval).abs_diff(c.key_index(neighbour)) > 1
    }

    /// `ICELITETake::Insert`82a5bce0. Its exclusion radius is one 30 Hz
    /// timeline frame, inclusive at both sides of every existing boundary.
    pub fn insert(&mut self, channel: usize, hard: bool) -> bool {
        let c = &self.take.channels[channel];
        let populated = c.intervals != 0;
        let gap = 1 + usize::from(hard);
        let (keys, at_key, intervals, at_interval) = if populated {
            let radius = 1.0 / (self.take.length * 30.0);
            if (0..=c.intervals).any(|i| {
                let p = c.interval_parameter(i);
                self.parameter >= p-radius && self.parameter <= p+radius
            }) { return false; }
            let i = self.current_interval(channel).unwrap();
            (gap as i32, c.key_index(i)+1, 1, i+1)
        } else if matches!(pack(self.parameter), 0 | 0xffff) {
            (2, 0, 1, 0)
        } else {
            (3+i32::from(hard), 0, 2, 0)
        };
        self.change_size(channel, keys, at_key, intervals, at_interval, gap, usize::from(hard && populated))
    }

    /// `ICELITETake::Harden`82a5c128 duplicates the shared key at one end.
    pub fn harden(&mut self, channel: usize, interval: usize, end: bool) -> bool {
        if self.interval_count(channel) == 0 || self.is_hard_cut(channel, interval, end) { return false; }
        let key = self.key_index(channel, interval) + usize::from(end);
        self.change_size(channel, 1, key, 0, interval, 0, 0)
    }

    /// `ICELITETake::Delete`82a5bf70. The native removes the current interval
    /// and one or two keys according to its hard ends and nearest boundary.
    pub fn delete(&mut self, channel: usize) -> bool {
        let Some(i) = self.current_interval(channel) else { return false };
        let c = &self.take.channels[channel];
        let first = self.is_hard_cut(channel, i, false);
        let last = self.is_hard_cut(channel, i, true);
        let nearer_end = (self.parameter-c.interval_parameter(i+1)).abs()
            <= (self.parameter-c.interval_parameter(i)).abs();
        let key = c.key_index(i) + usize::from((first && !last) || (!first && !last && nearer_end));
        self.change_size(channel, if first && last { -2 } else { -1 }, key, -1, i, 0, 0)
    }

    /// `ICELITETake::ChangeSize`82a5b368. Channel reconstruction is ordered
    /// before element reconstruction; inserted values are the evaluated cursor
    /// value, and copied values go through the descriptor's original encoder.
    #[expect(clippy::too_many_arguments, reason = "native channel splice contract")]
    fn change_size(&mut self, channel: usize, new_keys: i32, at_key: usize,
        new_intervals: i32, at_interval: usize, key_gap: usize, key_offset: usize) -> bool {
        if new_keys == 0 && new_intervals == 0 { return true; }
        let old = &self.take.channels[channel];
        let keys = old.keys as i32 + new_keys;
        let intervals = old.intervals as i32 + new_intervals;
        if !(0..=199).contains(&keys) || !(0..=198).contains(&intervals) { return false; }
        let (keys, intervals) = (keys as usize, intervals as usize);
        let after = at_interval + new_intervals.max(0) as usize;
        let mut starts = Vec::with_capacity(intervals);
        let mut indices = Vec::with_capacity(intervals);
        for i in 0..intervals {
            if i >= at_interval && i < after {
                starts.push(pack(self.parameter));
                indices.push(at_key + (i-at_interval)*key_gap + key_offset);
            } else {
                let source = if i < at_interval { i } else { (i as i32-new_intervals) as usize };
                starts.push(pack(old.interval_parameter(source)));
                let key = old.key_index(source);
                indices.push((key as i32 + if key >= at_key { new_keys } else { 0 }) as usize);
            }
        }
        if at_interval < intervals && new_intervals < 0 && at_key > old.key_index(at_interval) {
            starts[at_interval] = pack(old.interval_parameter(at_interval));
        }
        let next = Channel {
            intervals, keys,
            params: starts.into_iter().skip(1).collect(),
            key_indices: indices.into_iter().skip(1).take(intervals.saturating_sub(2)).map(|k| k as u16).collect(),
        };
        for e in ELEMENTS.iter().filter(|e| e.channel == channel) {
            let (at, delta, count) = if e.interval { (at_interval, new_intervals, intervals) } else { (at_key, new_keys, keys) };
            let after = at + delta.max(0) as usize;
            let data = &self.take.data[e.id];
            let next_data = (0..count).map(|i| {
                let v = if i < at { data[i] }
                    else if i < after { self.values[e.id] }
                    else { data[(i as i32-delta) as usize] };
                quantize(*e, v)
            }).collect();
            self.take.data[e.id] = next_data;
        }
        self.take.channels[channel] = next;
        // ChangeSize82a5b368 rebinds the populated take with SetDataPointers
        // 82a55d20, which preserves nCurrentInterval. SetParameter then keeps
        // that interval at an inclusive editor boundary. Invalidating it here
        // selected the right side of every new key: Setup* changed the left
        // camera's space but edited the right camera, and EndSetup discarded it.
        // The regular refresh below re-locates any interval removed by a splice.
        self.set_parameter_force(self.parameter);
        true
    }

    /// Copy/paste is restricted to key elements of the selected channel;
    /// interval camera types are not accidentally copied with the key.
    #[must_use]
    pub fn copy_key(&self, channel: usize, key: usize) -> Vec<(usize, f32)> {
        ELEMENTS.iter().filter(|e| e.channel == channel && !e.interval)
            .map(|e| (e.id, self.key_value(e.id, key))).collect()
    }
    pub fn paste_key(&mut self, key: usize, values: &[(usize, f32)]) -> Result<()> {
        for &(id, value) in values { self.set_value(id, key, value)?; }
        Ok(())
    }

    /// Native camera math consumes the same semantic channels under different
    /// element ids. Postprocessing also remains accessible through Playback.
    fn cinematic_values(&self) -> [f64; super::ELEMENT_COUNT] {
        let mut out = super::ELEMENTS.map(|e| f64::from(e.default));
        for (lite, id) in CINEMATIC_IDS.iter().enumerate() {
            if let Some(id) = id { out[*id] = self.values[lite]; }
        }
        out
    }

    pub fn camera(&self, shakes: &CameraSet, spaces: &Spaces, mover: &mut Mover, seconds: f32) -> Result<([f32; 16], f32)> {
        super::camera_matrix(shakes, &self.cinematic_values(), seconds, spaces, mover, self.parameter)
    }

    pub fn eye_and_look(&self, spaces: &Spaces) -> Result<([f32; 3], [f32; 3])> {
        let mut mover = Mover::default();
        let player = super::PlayerSpace { parameter: self.parameter, ..Default::default() };
        let point = |start| [self.value(start), self.value(start+1), self.value(start+2)];
        let eye = super::to_world(point(element::EYE), self.value_int(element::EYE_SPACE).into(),
            self.value_int(element::EYE_AVATAR).into(), spaces, &mut mover, player)?;
        let look = super::to_world(point(element::LOOK), self.value_int(element::LOOK_SPACE).into(),
            self.value_int(element::LOOK_AVATAR).into(), spaces, &mut mover, player)?;
        Ok((eye, look))
    }
}

/// `ICELITEElementDesc::Encode`82a54d60. The default divides the bit range
/// into separate positive and negative quantisation spans.
fn encoded_integer(e: Element, value: f64) -> u32 {
    if e.kind != 3 { return value as i64 as u32; }
    let slots = ((1u64 << e.bits)-1) as f32;
    let high = round_int((e.high-e.default)*slots/(e.high-e.low)) as f32;
    let low = slots-high;
    let v = super::clamp(value as f32, e.low, e.high);
    let delta = v-e.default;
    let q = if delta < 0.0 && e.default-e.low > 0.0 {
        low*delta/(e.default-e.low)+low
    } else if delta > 0.0 && e.high-e.default > 0.0 {
        high*delta/(e.high-e.default)+low
    } else { low };
    round_int(q) as u32
}

fn encode_into(e: Element, data: &mut [u8], index: usize, value: f64) {
    if e.kind == 4 {
        data[index*4..index*4+4].copy_from_slice(&(value as f32).to_le_bytes());
        return;
    }
    let raw = encoded_integer(e, value);
    for shift in 0..e.bits {
        let bit = index*e.bits+shift;
        let mask = 1u8 << (7-bit%8);
        data[bit/8] = (data[bit/8] & !mask) | (((raw >> (e.bits-1-shift)) as u8 & 1) << (7-bit%8));
    }
}

fn quantize(e: Element, value: f64) -> f64 {
    let mut data = [0u8; 4];
    encode_into(e, &mut data, 0, value);
    e.decode(&data, 0)
}

/// The editor loads its own original `data/scene/ICELite.bin` for recorded
/// camera shakes (Sk8::ICELITE::Load82879568). Remap by semantic descriptor
/// to share the cinematic shake evaluator, retaining original group ordering.
pub fn parse_shakes(bytes: &[u8]) -> Result<CameraSet> {
    let (root, body, end) = chunk(bytes, 0)?;
    if root != 0x8000_0000 || end != bytes.len() { return Err("invalid ICELITE root".into()); }
    let mut out = CameraSet { groups: Default::default(), order: Vec::new() };
    let mut at = 0;
    while at < body.len() {
        let (context, data, next) = chunk(body, at)?; at = next;
        let mut group_at = 0;
        while group_at < data.len() {
            let (_, group, next) = chunk(data, group_at)?; group_at = next;
            let key = word(group, 0)?;
            let count = word(group, 4)? as usize;
            if count > group.len()/156 { return Err("invalid ICELITE take count".into()); }
            let mut takes = Vec::with_capacity(count);
            let mut take_at = 8;
            for _ in 0..count {
                let (lite, size) = Take::parse_schema(&group[take_at..], &ELEMENTS, CHANNEL_COUNT)?;
                take_at += size;
                let mut take = Take { length: lite.length, schema: &super::ELEMENTS,
                    channels: vec![Channel::default(); super::CHANNEL_COUNT],
                    data: vec![Vec::new(); super::ELEMENT_COUNT] };
                for (lite_id, id) in CINEMATIC_IDS.iter().enumerate() {
                    if let Some(id) = *id {
                        take.data[id] = lite.data[lite_id].clone();
                        take.channels[super::ELEMENTS[id].channel] = lite.channels[ELEMENTS[lite_id].channel].clone();
                    }
                }
                takes.push(take);
            }
            if take_at != group.len() { return Err("trailing ICELITE group data".into()); }
            if out.groups.insert((context,key), takes).is_some() { return Err("duplicate ICELITE group".into()); }
            out.order.push((context,key));
        }
    }
    Ok(out)
}
