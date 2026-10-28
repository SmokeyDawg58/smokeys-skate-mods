//! ICELITEAuthor/Controller's editable camera, clip and head operations.
use super::{EditableTake, CAMERA, LENS, SHAKE, SPEED, element as e};
use super::super::{Result, Spaces, clamp, math};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CameraKind { Follow = 0, Tripod = 1, Free = 2, Game = 3 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyState { Scrub, Start, End }

#[derive(Clone, Debug)]
pub struct Author {
    pub take: EditableTake,
    pub start: f32,
    pub end: f32,
    channel: usize,
    state: KeyState,
    saved_parameter: f32,
    saved_limits: [f32;2],
    advanced: bool,
    orbit: bool,
    spin: u16,
    tilt: i16,
    spin_delta: i16,
    tilt_delta: i16,
    distance: f32,
    height: f32,
}

impl Author {
    /// Controller::Reset82a5d310 followed by SetupScrub82a63008. Recorded
    /// footage and camera cuts are retained; these are controller fields.
    pub fn reset_instant(&mut self) -> Result<()> {
        self.spin=0;self.tilt=0;self.spin_delta=0;self.tilt_delta=0;
        self.distance=0.;self.height=0.;self.channel=CAMERA;
        self.set_state(KeyState::Scrub);Ok(())
    }
    /// SceneLite::Init8287df38 creates the game camera and all four editable
    /// channels, then places the playhead at the last recorded frame.
    pub fn new(length: f32) -> Result<Self> {
        let mut take = EditableTake::new(length.max(1.5));
        take.insert(CAMERA, false);
        for (id, value) in [(e::EYE_AVATAR, 2.), (e::LOOK_AVATAR, 3.), (e::ORBIT_AVATAR, 2.)] {
            take.set_value(id, 0, value)?;
        }
        for id in 0..6 { for key in 0..2 { take.set_value(id, key, 0.)?; } }
        for channel in [SPEED, LENS, SHAKE] { take.insert(channel, false); }
        take.set_parameter(1.0);
        Ok(Self { take, start: 0., end: 1., channel: CAMERA, state: KeyState::Scrub,
            saved_parameter: 0., saved_limits:[0.,1.], advanced: false, orbit: false, spin: 0, tilt: 0,
            spin_delta: 0, tilt_delta: 0, distance: 0., height: 0. })
    }

    /// ICELITEAuthor::SetState82a622a8: edit-key states temporarily move the
    /// cursor to the selected endpoint, restoring its saved parameter on exit.
    fn set_state(&mut self, state: KeyState) {
        if self.state == state { return; }
        if self.state != KeyState::Scrub {
            self.take.set_parameter(self.saved_parameter);
            [self.start,self.end]=self.saved_limits;
        }
        self.state = state;
        if state != KeyState::Scrub {
            self.saved_parameter = self.take.parameter();
            self.saved_limits=[self.start,self.end];
            if let Some(interval) = self.take.current_interval(self.channel) {
                let end = usize::from(state == KeyState::End);
                self.take.set_parameter_force(self.take.interval_parameter(self.channel, interval+end));
                // 82a625e4..62634: shift the temporary window to contain the
                // edited endpoint, then restore both limits on leaving the state.
                let p=self.take.parameter();
                if p<self.start {let delta=self.start-p;self.start=p;self.end-=delta;}
                else if p>self.end {let delta=p-self.end;self.end=p;self.start+=delta;}
            }
        }
    }

    fn selected_key(&self) -> Result<usize> {
        let i = self.take.current_interval(self.channel).ok_or("ICELITE edit channel has no interval")?;
        Ok(self.take.key_index(self.channel, i)+usize::from(self.state == KeyState::End))
    }
    fn key_value(&self, id: usize) -> Result<f32> { Ok(self.take.value(id)) }
    fn set_key(&mut self, id: usize, value: f32) -> Result<()> { self.take.edit_value(id, self.selected_key()?, value) }
    fn change_key(&mut self, id: usize, delta: f32) -> Result<()> {
        // SetKeyElementValueChange82a60b40 clamps to the descriptor limits.
        let d = super::ELEMENTS[id];
        let value = if d.kind >= 3 { self.key_value(id)?+delta }
            else { (self.take.key_value(id,self.selected_key()?) as i32 + delta as i32) as f32 };
        self.set_key(id, clamp(value, d.low, d.high))
    }
    fn copy_key(&self) -> Result<Vec<(usize, f32)>> { Ok(self.take.copy_key(self.channel, self.selected_key()?)) }
    fn paste_key(&mut self, values: &[(usize, f32)]) -> Result<()> {
        for &(id,value) in values { self.set_key(id,value)?; } Ok(())
    }

    #[must_use]
    pub fn cursor_frame(&self) -> i32 { ((self.take.parameter()*self.take.length())*30.0+0.0005) as i32 }
    #[must_use]
    pub fn start_frame(&self) -> i32 { ((self.start*self.take.length())*30.0+0.0005) as i32 }
    #[must_use]
    pub fn end_frame(&self) -> i32 { ((self.end*self.take.length())*30.0+0.0005) as i32 }
    #[must_use]
    pub fn frame_count(&self) -> i32 { (self.take.length()*30.0+0.0005) as i32 }

    /// MoveStartHead82a5ea60 / MoveEndHead82a5eae0: minimum span 0.1 of
    /// the whole take; moving either head also moves the presentation cursor.
    pub fn move_start(&mut self, delta: f32) {
        self.start = clamp((self.start+delta).min(self.end-0.1), 0., 1.);
        self.take.set_parameter(self.start);
    }
    pub fn move_end(&mut self, delta: f32) {
        self.end = clamp((self.end+delta).max(self.start+0.1), 0., 1.);
        self.take.set_parameter(self.end);
    }
    pub fn move_cursor(&mut self, delta: f32) {
        self.take.set_parameter(clamp(self.take.parameter()+delta, self.start, self.end));
    }
    pub fn advance(&mut self, seconds: f32) {
        let rate = self.take.value(e::SPEED)*0.01;
        self.move_cursor((rate/self.take.length())*seconds);
    }

    /// The native author's jump (82a5da50/82a5db80), before the controller's
    /// trim clamp and +0.0001 nudge on a forward jump.
    fn jump_key(&mut self, forward: bool) {
        let candidate = |channel| {
            let i = self.take.current_interval(channel).unwrap_or(0);
            if forward {
                let p = self.take.interval_parameter(channel, i+1);
                if self.take.parameter()+0.00001 > p && i+1 < self.take.key_count(channel) {
                    self.take.interval_parameter(channel, i+2)
                } else { p }
            } else {
                let p = self.take.interval_parameter(channel, i);
                if self.take.parameter()-0.00001 < p && i>0 {
                    self.take.interval_parameter(channel, i-1)
                } else { p }
            }
        };
        let mut p = candidate(self.channel);
        if self.channel != CAMERA && self.channel != 32 {
            p = if forward { p.min(candidate(CAMERA)) } else { p.max(candidate(CAMERA)) };
        }
        self.take.set_parameter(p);
    }
    pub fn jump_previous(&mut self) { self.jump_key(false); if self.take.parameter()<self.start { self.take.set_parameter(self.start); } }
    pub fn jump_next(&mut self) { self.jump_key(true); self.move_cursor(0.0001); }

    /// InsertKeys82a5ffb0: camera, speed, shake and lens are cut in that order.
    pub fn insert_clip(&mut self) -> bool {
        self.channel = CAMERA;
        if !self.take.insert(CAMERA, true) { return false; }
        for channel in [SPEED, SHAKE, LENS] { self.take.insert(channel, true); }
        let previous = self.take.parameter();
        self.move_cursor(0.0001);
        if self.take.parameter() >= 1. { self.take.set_parameter(previous); }
        true
    }

    /// Backend::GetCamera8255df80: camera identity comes from eye/look
    /// spaces and avatars, independent of whether that clip contains soft keys.
    #[must_use]
    pub fn camera_kind(&self, interval: usize) -> CameraKind {
        if self.take.key_value(e::EYE_SPACE, interval) as i32 == 0 {
            if self.take.key_value(e::LOOK_AVATAR, interval) as i32 == 4 { CameraKind::Follow } else { CameraKind::Game }
        } else if self.take.key_value(e::EYE_SPACE, interval) as i32 == 1 && self.take.key_value(e::LOOK_SPACE, interval) as i32 == 1 {
            CameraKind::Free
        } else { CameraKind::Tripod }
    }
    #[must_use]
    pub fn current_camera(&self) -> CameraKind { self.camera_kind(self.take.current_interval(CAMERA).unwrap_or(0)) }

    #[must_use]
    pub fn has_soft_keys(&self) -> bool {
        self.take.current_interval(CAMERA).is_some_and(|i|
            !self.take.is_hard_cut(CAMERA, i, false) || !self.take.is_hard_cut(CAMERA, i, true))
    }

    /// RemoveCurrentKey82a63538 keeps effect cuts aligned when removing a
    /// hard cut; a soft-key removal copies the resulting start key across the join.
    fn remove_current(&mut self, soft: bool) -> Result<()> {
        self.channel = CAMERA;
        if soft {
            let state = self.state;
            let p = self.take.parameter();
            if !self.take.delete(CAMERA) { return Err("ICELITE soft key could not be removed".into()); }
            self.set_state(KeyState::Start);
            let values = self.copy_key()?;
            self.take.set_parameter(p);
            self.paste_key(&values)?;
            self.set_state(KeyState::End);
            self.set_state(state);
        } else {
            for channel in [CAMERA, SPEED, SHAKE, LENS] { self.take.delete(channel); }
        }
        Ok(())
    }

    /// ClearSoftMarkers82a63718. `both` cleans on both sides of the cursor;
    /// `clean_only` removes the separating hard cut as well.
    pub fn clear_soft_keys(&mut self, both: bool, clean_only: bool) -> Result<()> {
        self.channel = CAMERA;
        let state = self.state;
        loop {
            self.set_state(KeyState::End);
            let i = self.take.current_interval(CAMERA).ok_or("ICELITE camera interval absent")?;
            if self.take.is_hard_cut(CAMERA, i, true) { break; }
            self.remove_current(true)?;
        }
        if both {
            if clean_only { self.remove_current(false)?; }
            loop {
                self.set_state(KeyState::Start);
                let i = self.take.current_interval(CAMERA).ok_or("ICELITE camera interval absent")?;
                if self.take.is_hard_cut(CAMERA, i, false) { break; }
                self.remove_current(true)?;
            }
        }
        self.set_state(state);
        Ok(())
    }

    /// DeleteMarker82a64ce0; caller confirms a camera reset when required.
    pub fn delete_marker(&mut self, index: usize, reset: bool, spaces: &Spaces) -> Result<()> {
        if self.take.interval_count(CAMERA)<=1 { return Ok(()); }
        let saved = self.take.parameter();
        let soft = !self.take.is_hard_cut(CAMERA, index, false);
        if Some(index)>self.take.current_interval(CAMERA) { self.jump_next(); }
        if reset { self.clear_soft_keys(true, true)?; } else { self.remove_current(soft)?; }
        self.take.set_parameter(saved);
        if reset && self.setup_camera(CameraKind::Game, false, false, spaces)? { self.end_setup()?; }
        Ok(())
    }

    /// Transform both endpoints before changing a camera interval's reference
    /// space (SwitchEyeSpace82a61150 / SwitchLookSpace82a61390).
    fn switch_space(&mut self, interval: usize, look: bool, space: i32, avatar: i32, spaces: &Spaces) -> Result<bool> {
        let space_id = if look { e::LOOK_SPACE } else { e::EYE_SPACE };
        let avatar_id = if look { e::LOOK_AVATAR } else { e::EYE_AVATAR };
        let (old_space,old_avatar) = (self.take.key_value(space_id,interval) as i32,self.take.key_value(avatar_id,interval) as i32);
        if (old_space,old_avatar)==(space,avatar) { return Ok(false); }
        let start = if look { e::LOOK } else { e::EYE };
        let first = self.take.key_index(CAMERA, interval);
        let from = space_matrix(spaces, old_space, old_avatar)?;
        let to = inverse(space_matrix(spaces, space, avatar)?);
        for key in first..first+2 {
            let point = std::array::from_fn(|i| self.take.key_value(start+i,key));
            let point = transform(transform(point,from),to);
            for (i,v) in point.into_iter().enumerate() { self.take.set_value(start+i,key,v)?; }
        }
        Ok(true)
    }

    /// SetupOrbit/Tripod/Free/GameCam82a638d0..82a640f0. Basic edits insert
    /// a temporary hard interval which EndSetup merges into the clip; advanced
    /// edits insert a shared soft key or edit the existing marker's start key.
    pub fn setup_camera(&mut self, camera: CameraKind, advanced: bool, edit: bool, spaces: &Spaces) -> Result<bool> {
        self.channel = CAMERA;
        let Some(i) = self.take.current_interval(CAMERA) else { return Ok(false) };
        self.advanced = advanced;
        self.orbit = matches!(camera, CameraKind::Follow | CameraKind::Tripod);
        self.height = 0.;
        if edit {
            self.set_state(KeyState::Start);
        } else {
            if !self.take.insert(CAMERA, !advanced) {
                self.advanced=false; self.orbit=false; return Ok(false);
            }
            if !advanced { self.take.harden(CAMERA,i,false); self.take.harden(CAMERA,i,true); }
            let avatar = self.take.value_int(e::EYE_AVATAR);
            let old_look = self.take.value_int(e::LOOK_SPACE);
            let eye_space = i32::from(matches!(camera, CameraKind::Tripod | CameraKind::Free));
            let eye_avatar = if matches!(camera,CameraKind::Follow|CameraKind::Game) { 2 } else { avatar };
            let mut changed = self.switch_space(i,false,eye_space,eye_avatar,spaces)?;
            self.take.set_value(e::AVOID_STATIC,i,if camera==CameraKind::Game { 0. } else { 1. })?;
            if changed { self.take.set_value(e::EYE_SPACE,i,eye_space as f32)?; }
            let look_space = i32::from(camera==CameraKind::Free);
            if old_look != look_space {
                self.switch_space(i,true,look_space,eye_avatar,spaces)?;
                self.take.set_value(e::LOOK_SPACE,i,look_space as f32)?;
                changed=true;
            }
            match camera {
                CameraKind::Follow | CameraKind::Game => {
                    for (id,v) in [(e::EYE_AVATAR,2.),(e::LOOK_AVATAR,if camera==CameraKind::Game {3.} else {4.}),(e::ORBIT_AVATAR,2.)] {
                        self.take.set_value(id,i,v)?;
                    }
                }
                CameraKind::Tripod => self.take.set_value(e::LOOK_AVATAR,i,4.)?,
                CameraKind::Free => (),
            }
            self.set_state(KeyState::End);
            if camera==CameraKind::Free && changed {
                for id in [e::EYE_AVATAR,e::LOOK_AVATAR,e::ORBIT_AVATAR] { self.take.set_value(id,i,0.)?; }
            } else if camera==CameraKind::Game {
                for id in 0..6 { self.set_key(id,0.)?; }
            } else if changed {
                for id in 3..6 { self.set_key(id,0.)?; }
            }
        }
        self.to_bubble(spaces)?;
        Ok(true)
    }

    /// EndSetup82a63120; all copy/paste and endpoint jumps retain native order.
    pub fn end_setup(&mut self) -> Result<()> {
        self.channel = CAMERA;
        let values = self.copy_key()?;
        if self.advanced {
            self.set_state(KeyState::Scrub);
            self.move_cursor(0.0001);
            let i=self.take.current_interval(CAMERA).ok_or("ICELITE camera interval absent")?;
            if self.take.is_hard_cut(CAMERA,i,true) {
                self.set_state(KeyState::End); self.paste_key(&values)?;
            }
            self.set_state(KeyState::Start);
            self.jump_key(false);
            let i=self.take.current_interval(CAMERA).ok_or("ICELITE camera interval absent")?;
            if self.take.is_hard_cut(CAMERA,i,false) {
                self.set_state(KeyState::Start); self.paste_key(&values)?;
                self.set_state(KeyState::End);
            }
            self.jump_key(true);
            self.move_cursor(0.0001);
        } else {
            self.set_state(KeyState::Start); self.paste_key(&values)?;
            self.set_state(KeyState::End); self.jump_key(true);
            self.set_state(KeyState::Start); self.paste_key(&values)?;
            self.set_state(KeyState::End); self.paste_key(&values)?;
            self.take.delete(CAMERA);
        }
        self.advanced=false;
        self.set_state(KeyState::Scrub);
        Ok(())
    }

    /// SetSpeed82a646d8 sets both ends of the selected speed interval.
    pub fn set_speed(&mut self, value: u32) -> Result<()> {
        self.set_channel_pair(SPEED, &[(e::SPEED,value as f32)], None)
    }
    /// SetShakeType82a64458 isolates both boundaries before setting the pair.
    pub fn set_shake(&mut self, kind: u8) -> Result<()> {
        let (native, strength) = match kind { 1=>(3,0.35),2=>(3,1.),3=>(4,1.),_=>(0,0.) };
        self.set_channel_pair(SHAKE,&[(e::SHAKE_FREQUENCY,strength),(e::SHAKE_AMPLITUDE,strength)],Some((e::SHAKE_TYPE,native)))
    }
    fn set_channel_pair(&mut self, channel:usize, values:&[(usize,f32)], interval_value:Option<(usize,i32)>) -> Result<()> {
        let state=self.state;
        self.channel=channel; self.set_state(KeyState::End);
        let i=self.take.current_interval(channel).ok_or("ICELITE effect interval absent")?;
        if let Some((id,value))=interval_value {
            self.take.harden(channel,i,false); self.take.harden(channel,i,true);
            self.take.set_value(id,i,value as f32)?;
        }
        for &(id,v) in values { self.set_key(id,v)?; }
        let copied=self.copy_key()?;
        self.set_state(KeyState::Start); self.paste_key(&copied)?;
        self.set_state(KeyState::End); self.channel=CAMERA; self.set_state(state);
        Ok(())
    }
    pub fn zoom(&mut self, delta:f32) -> Result<()> {
        self.channel=CAMERA;
        self.set_key(e::LENS,clamp(self.key_value(e::LENS)?+delta,24.,75.))?;
        self.channel=LENS; self.set_key(e::NEAR_CLIP,0.1)?; self.channel=CAMERA;
        Ok(())
    }
    pub fn reset_camera_settings(&mut self) -> Result<()> {
        self.channel=CAMERA; self.set_key(e::LENS,24.)?;
        self.channel=LENS; self.set_key(e::NEAR_CLIP,0.1)?;
        self.channel=CAMERA; self.set_speed(100)
    }

    fn key_point(&self, look:bool) -> Result<[f32;3]> {
        let id=if look {e::LOOK} else {e::EYE};
        // ToBubble82a606f0 / FromBubble82a61a28 read the selected stored
        // endpoint. Incremental edits above instead read ElementValue::mValue.
        let key=self.selected_key()?;
        Ok(std::array::from_fn(|axis|self.take.key_value(id+axis,key)))
    }
    fn set_point(&mut self, look:bool, point:[f32;3]) -> Result<()> {
        let id=if look {e::LOOK} else {e::EYE};
        for (axis,v) in point.into_iter().enumerate() { self.set_key(id+axis,v)?; } Ok(())
    }
    fn point_frame(&self, spaces:&Spaces, look:bool) -> Result<Matrix> {
        space_matrix(spaces,self.take.value_int(if look {e::LOOK_SPACE} else {e::EYE_SPACE}),
            self.take.value_int(if look {e::LOOK_AVATAR} else {e::EYE_AVATAR}))
    }
    /// ToBubble82a606f0: binary turn angles are calculated in world space.
    fn to_bubble(&mut self, spaces:&Spaces) -> Result<()> {
        let eye=transform(self.key_point(false)?,self.point_frame(spaces,false)?);
        let look=transform(self.key_point(true)?,self.point_frame(spaces,true)?);
        let [x,y,z]=std::array::from_fn(|i|eye[i]-look[i]);
        self.spin=math::atan(x,y);
        self.tilt=math::atan((x*x+y*y).sqrt(),z) as i16;
        self.distance=(z.mul_add(z,x*x)+y*y).sqrt();
        Ok(())
    }
    /// FromBubble82a61a28. Orbit changes rotate the look offset about the
    /// authored orbit avatar before deriving the eye; free changes keep the eye.
    fn from_bubble(&mut self, spaces:&Spaces) -> Result<()> {
        let (sin,cos)=math::sin_cos(self.spin);
        let (tilt_sin,tilt_cos)=math::sin_cos(self.tilt as u16);
        let offset=[self.distance*tilt_cos*cos,self.distance*tilt_cos*sin,self.distance*tilt_sin];
        if self.orbit {
            let mut look=self.key_point(true)?;
            let orbit=self.take.value_int(e::ORBIT_AVATAR);
            let look_space=self.take.value_int(e::LOOK_SPACE);
            let look_avatar=self.take.value_int(e::LOOK_AVATAR);
            if orbit>0 && matches!(look_space,0|2) {
                let pivot=space_matrix(spaces,look_space,orbit-1)?;
                if orbit-1!=look_avatar { look=transform(transform(look,self.point_frame(spaces,true)?),inverse(pivot)); }
                let eye=self.key_point(false)?;
                let dot=(eye[1]-look[1]).mul_add(look[1],(eye[0]-look[0])*look[0]);
                let radius=(look[2].mul_add(look[2],look[1].mul_add(look[1],look[0]*look[0]))).sqrt();
                let flat=(look[1].mul_add(look[1],look[0]*look[0])).sqrt();
                let spin=math::atan(look[0],look[1]).wrapping_add(self.spin_delta as u16);
                let tilt=math::atan(flat,look[2]);
                let tilt=if dot>=0. {tilt.wrapping_add(self.tilt_delta as u16)} else {tilt.wrapping_sub(self.tilt_delta as u16)};
                let (s,c)=math::sin_cos(spin); let (ts,tc)=math::sin_cos(tilt);
                look=[radius*tc*c,radius*tc*s,radius*ts];
                if orbit-1!=look_avatar { look=transform(transform(look,pivot),inverse(self.point_frame(spaces,true)?)); }
                self.set_point(true,look)?;
            }
            let world=transform(look,self.point_frame(spaces,true)?);
            let eye=std::array::from_fn(|i|world[i]+offset[i]);
            self.set_point(false,transform(eye,inverse(self.point_frame(spaces,false)?)))?;
        } else {
            let eye=transform(self.key_point(false)?,self.point_frame(spaces,false)?);
            let look=std::array::from_fn(|i|eye[i]-offset[i]);
            self.set_point(true,transform(look,inverse(self.point_frame(spaces,true)?)))?;
        }
        Ok(())
    }

    /// Collision feedback, ICELITEController::SetFreeCameraMatrix82a61928.
    /// The free camera's authored coordinates are already world coordinates.
    pub fn set_free_camera_matrix(&mut self,skate:[f32;16],spaces:&Spaces)->Result<()> {
        let eye=[skate[14],skate[12],skate[13]];
        let forward=[skate[10],skate[8],skate[9]];
        let look=std::array::from_fn(|i|forward[i].mul_add(self.distance,eye[i]));
        self.set_point(false,eye)?;self.set_point(true,look)?;self.to_bubble(spaces)
    }
    /// SetBubble82a64b58 resets both rotation deltas before FromBubble.
    pub fn set_bubble(&mut self,spin:u16,tilt:u16,size:f32,spaces:&Spaces)->Result<()> {
        self.distance=size;self.spin=spin;self.tilt=tilt as i16;
        self.spin_delta=0;self.tilt_delta=0;self.from_bubble(spaces)
    }

    /// ChangeKeyElementValue82a64818: binary-angle rounding and 85-degree
    /// signed tilt clamp; movement is transformed into the edited point's space.
    fn change_camera(&mut self, element:i32, delta:f32, spaces:&Spaces) -> Result<()> {
        if delta==0. { return Ok(()); }
        let handled=matches!(element,0|1|3|4);
        if handled {
            let angle=(delta*182.04445) as i32 as i16;
            let angle=if self.orbit {i32::from(angle)} else {-i32::from(angle)};
            let old_spin=self.spin; let old_tilt=self.tilt;
            if element==0 { self.spin=(i32::from(self.spin as i16)+angle) as u16; }
            if element==1 { self.tilt=(i32::from(self.tilt)+angle).clamp(-15473,15473) as i16; }
            self.spin_delta=self.spin.wrapping_sub(old_spin) as i16;
            self.tilt_delta=self.tilt.wrapping_sub(old_tilt);
            let x=if element==3 {delta} else {0.};
            let y=if element==4 {delta} else {0.};
            let (sin,cos)=math::sin_cos(self.spin);
            let world=[-(cos*y+sin*x),cos*x-sin*y,0.];
            let mut frame=inverse(self.point_frame(spaces,self.orbit)?); frame[3]=[0.,0.,0.,1.];
            let local=transform(world,frame);
            let id=if self.orbit {e::LOOK} else {e::EYE};
            self.change_key(id,local[0])?; self.change_key(id+1,local[1])?;
        }
        if element<0 { self.distance=clamp(self.distance+delta,0.5,1000.); }
        if handled || element<0 { self.from_bubble(spaces)?; }
        else { self.change_key(element as usize,delta)?; }
        Ok(())
    }
    pub fn orbit_position(&mut self, x:f32,y:f32,distance:f32,spaces:&Spaces) -> Result<()> {
        let z=self.key_value(e::LOOK+2)?;
        self.change_camera(-1,distance,spaces)?;
        self.change_camera(0,x,spaces)?; self.change_camera(1,y,spaces)?;
        self.set_point(true,[0.,0.,z])
    }
    pub fn free_position(&mut self, x:f32,y:f32,tx:f32,ty:f32,spaces:&Spaces) -> Result<()> {
        for (id,v) in [(0,x),(1,y),(3,tx),(4,ty)] { self.change_camera(id,v,spaces)?; } Ok(())
    }
    pub fn height(&mut self, delta:f32,spaces:&Spaces) -> Result<()> {
        self.height+=delta;
        if self.height>=1. {self.height=1.;return Ok(());}
        if self.height<=-1. {self.height=-1.;return Ok(());}
        if delta!=0. {self.change_camera(5,delta,spaces)?;self.to_bubble(spaces)?;} Ok(())
    }
    /// UpdateFreeCamHeight82a65020 translates eye and look together. Its
    /// accumulated offset has the native +/-10 limit, separate from orbit's
    /// +/-1 look-only adjustment. Hitting either limit suppresses that step.
    pub fn free_height(&mut self,delta:f32,spaces:&Spaces)->Result<()> {
        self.height+=delta;
        if self.height>=10. {self.height=10.;return Ok(());}
        if self.height<=-10. {self.height=-10.;return Ok(());}
        if delta!=0. {
            self.change_camera(2,delta,spaces)?;
            self.change_camera(5,delta,spaces)?;
        }
        Ok(())
    }
}

type Matrix=[[f32;4];4];
const IDENTITY:Matrix=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]];

fn space_matrix(spaces:&Spaces, space:i32, avatar:i32) -> Result<Matrix> {
    if space==1 {return Ok(IDENTITY);}
    if space==3 {return Ok(spaces.scene.unwrap_or(IDENTITY));}
    if !matches!(space,0|2) {return Err(format!("ICELITE unknown coordinate space {space}"));}
    let mut out=IDENTITY;
    let (position,orientation)=if avatar>0 {
        let i=(avatar-1) as usize;
        (spaces.avatars.get(i).copied().flatten(),spaces.avatar_orientations.get(i).copied().flatten())
    } else {(spaces.anchor,spaces.anchor_orientation)};
    let position=position.ok_or("ICELITE camera avatar position unavailable")?;
    if space==0 {
        let orientation=orientation.ok_or("ICELITE camera avatar orientation unavailable")?;
        for (r,row) in orientation.into_iter().enumerate() {out[r][..3].copy_from_slice(&row);}
    }
    out[3][..3].copy_from_slice(&position); Ok(out)
}

/// GetWorldTransformationMatrix82a5e0e0 calls the original rigid inverse.
fn inverse(m:Matrix) -> Matrix {
    let mut out=IDENTITY;
    for row in 0..3 {for column in 0..3 {out[row][column]=m[column][row];}}
    for axis in 0..3 {out[3][axis]=-m[3][0].mul_add(m[axis][0],m[3][2].mul_add(m[axis][2],m[3][1]*m[axis][1]));}
    out
}
fn transform(v:[f32;3],m:Matrix) -> [f32;3] {
    std::array::from_fn(|c|v[0].mul_add(m[0][c],v[2].mul_add(m[2][c],v[1]*m[1][c]))+m[3][c])
}

#[cfg(test)]
mod replay_camera_tests {
    use super::*;
    fn spaces()->Spaces {
        let mut s=Spaces {anchor:Some([0.;3]),anchor_orientation:Some([[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]),..Default::default()};
        for (i,p) in [[0.;3],[-4.,0.,2.],[-1.,0.,2.],[0.,0.,2.]].into_iter().enumerate() {
            s.avatars[i]=Some(p);s.avatar_orientations[i]=s.anchor_orientation;
        }
        s
    }
    fn free()->(Author,Spaces) {
        let s=spaces();let mut a=Author::new(10.).unwrap();a.take.set_parameter(0.5);
        // PrepareFreeCam then SetupFreeCam, as on the first camera selection.
        assert!(a.setup_camera(CameraKind::Free,false,false,&s).unwrap());
        a.reset_camera_settings().unwrap();a.end_setup().unwrap();
        assert!(a.setup_camera(CameraKind::Free,false,false,&s).unwrap());
        (a,s)
    }
    fn close(a:[f32;3],b:[f32;3]) {
        for (a,b) in a.into_iter().zip(b) {assert!((a-b).abs()<0.002,"{a} != {b}");}
    }
    #[test]
    fn freecam_translation_and_yaw_keep_the_other_camera_components() {
        let (mut a,s)=free();
        let (eye,look)=a.take.eye_and_look(&s).unwrap();
        a.free_position(0.,0.,0.2,0.,&s).unwrap();
        let (moved,target)=a.take.eye_and_look(&s).unwrap();
        close(moved,[eye[0],eye[1]-0.2,eye[2]]);
        close(std::array::from_fn(|i|target[i]-moved[i]),std::array::from_fn(|i|look[i]-eye[i]));
        a.free_position(2.,0.,0.,0.,&s).unwrap();
        let (rotated_eye,rotated_look)=a.take.eye_and_look(&s).unwrap();
        close(rotated_eye,moved);
        assert!((rotated_look[1]-target[1]).abs()>0.05);
        close([rotated_look[2],0.,0.],[target[2],0.,0.]);
    }
    #[test]
    fn freecam_height_translates_both_points_beyond_the_orbit_limit() {
        let (mut a,s)=free();let (eye,look)=a.take.eye_and_look(&s).unwrap();
        for _ in 0..20 {a.free_height(0.2,&s).unwrap();}
        let (high,target)=a.take.eye_and_look(&s).unwrap();
        close(high,[eye[0],eye[1],eye[2]+4.]);close(target,[look[0],look[1],look[2]+4.]);
        for _ in 0..100 {a.free_height(0.2,&s).unwrap();}
        let capped=a.take.eye_and_look(&s).unwrap();
        assert_eq!(a.height,10.);a.free_height(0.2,&s).unwrap();assert_eq!(a.take.eye_and_look(&s).unwrap(),capped);
        a.free_height(-0.2,&s).unwrap();assert!(a.take.eye_and_look(&s).unwrap().0[2]<capped.0[2]);
    }
    #[test]
    fn basic_freecam_commit_preserves_pose_and_zoom_across_the_clip() {
        let (mut a,s)=free();
        a.free_position(12.,3.,0.2,0.1,&s).unwrap();a.free_height(0.2,&s).unwrap();a.zoom(15.).unwrap();
        let before=a.take.eye_and_look(&s).unwrap();a.end_setup().unwrap();
        assert_eq!(a.take.interval_count(CAMERA),1);
        for p in [0.,0.25,0.75,1.] {
            a.take.set_parameter_force(p);assert_eq!(a.current_camera(),CameraKind::Free);
            let (eye,look)=a.take.eye_and_look(&s).unwrap();close(eye,before.0);close(look,before.1);
            assert!((a.take.value(e::LENS)-39.).abs()<1.);
        }
    }
    #[test]
    fn clip_effects_and_advanced_keys_survive_serialization() {
        let (mut a,s)=free();a.end_setup().unwrap();a.take.set_parameter(0.5);assert!(a.insert_clip());
        a.take.set_parameter(0.75);a.set_speed(40).unwrap();a.set_shake(2).unwrap();
        assert!(a.setup_camera(CameraKind::Free,true,false,&s).unwrap());
        a.free_height(0.2,&s).unwrap();a.end_setup().unwrap();
        let mut restored=EditableTake::parse(&a.take.serialize()).unwrap();
        assert_eq!(restored.interval_count(CAMERA),3);
        for (p,speed,shake) in [(0.25,100,0),(0.6,40,3),(0.9,40,3)] {
            restored.set_parameter_force(p);assert_eq!(restored.value_int(e::SPEED),speed);
            assert_eq!(restored.value_int(e::SHAKE_TYPE),shake);
        }
    }
    #[test]
    fn all_basic_camera_modes_keep_the_edited_interval_when_prepared_and_committed() {
        let s=spaces();let mut a=Author::new(10.).unwrap();
        for camera in [CameraKind::Follow,CameraKind::Tripod,CameraKind::Free,CameraKind::Game] {
            a.take.set_parameter(0.4);a.clear_soft_keys(true,false).unwrap();
            assert!(a.setup_camera(camera,false,false,&s).unwrap());
            a.reset_camera_settings().unwrap();a.end_setup().unwrap();
            assert!(a.setup_camera(camera,false,false,&s).unwrap());
            assert_eq!(a.current_camera(),camera);
            a.zoom(5.).unwrap();a.end_setup().unwrap();
            for p in [0.1,0.8] {a.take.set_parameter(p);assert_eq!(a.current_camera(),camera);}
        }
    }
    #[test]
    fn existing_advanced_key_can_be_edited_deleted_then_a_hard_clip_reset() {
        let (mut a,s)=free();a.end_setup().unwrap();a.take.set_parameter(0.4);
        assert!(a.setup_camera(CameraKind::Free,true,false,&s).unwrap());
        a.free_height(0.2,&s).unwrap();a.end_setup().unwrap();
        let boundary=a.take.interval_parameter(CAMERA,1);
        a.take.set_parameter(boundary+0.0001);
        assert!(a.setup_camera(CameraKind::Free,true,true,&s).unwrap());
        let before=a.key_point(false).unwrap();a.free_height(0.2,&s).unwrap();a.end_setup().unwrap();
        a.take.set_parameter(boundary+0.0001);
        assert!((a.take.value(e::EYE+2)-before[2]-0.2).abs()<0.002);
        a.delete_marker(1,false,&s).unwrap();assert_eq!(a.take.interval_count(CAMERA),1);
        a.take.set_parameter(0.6);assert!(a.insert_clip());
        a.delete_marker(1,true,&s).unwrap();assert_eq!(a.take.interval_count(CAMERA),1);
        assert_eq!(a.current_camera(),CameraKind::Game);
    }
}
