//! A skater's audio record (`AudioPresentation::FillSkaterData` 0x827f4c48's `PhysOut` values,
//! [`SkaterAudioPhysOut`]) as the replay frame's audio lists: its floats in field order, and its
//! integers, with the flags packed 32 to a word.

use crate::SkaterAudioPhysOut;

/// Walks the record's fields in one fixed order, for both directions.
trait Fields {
    fn float(&mut self, value: &mut f32);
    fn int(&mut self, value: &mut i32);
    fn word(&mut self, value: &mut u32);
    fn flag(&mut self, value: &mut bool);

    fn floats(&mut self, values: &mut [f32]) {
        for value in values {
            self.float(value);
        }
    }

    fn ints(&mut self, values: &mut [i32]) {
        for value in values {
            self.int(value);
        }
    }

    fn flags(&mut self, values: &mut [bool]) {
        for value in values {
            self.flag(value);
        }
    }
}

fn visit(record: &mut SkaterAudioPhysOut, fields: &mut impl Fields) {
    let r = record;
    fields.floats(&mut r.centre_of_mass);
    fields.floats(&mut r.centre_of_mass_velocity);
    fields.floats(&mut r.hips_mat_y);
    fields.floats(&mut r.foot_l_vel_deck_space);
    fields.floats(&mut r.foot_r_vel_deck_space);
    fields.floats(&mut r.foot_left_vel);
    fields.floats(&mut r.foot_right_vel);
    fields.floats(&mut r.torso_angular_vel);
    fields.floats(&mut r.right_foot_angular_vel);
    fields.floats(&mut r.left_foot_angular_vel);
    fields.float(&mut r.absorption_compression);
    fields.floats(&mut r.average_limb_speed);
    fields.flag(&mut r.is_wipeout_over);
    fields.flag(&mut r.is_left_foot_on_deck);
    fields.flag(&mut r.is_right_foot_on_deck);
    fields.flag(&mut r.is_grabbing_left);
    fields.flag(&mut r.is_grabbing_right);
    fields.floats(&mut r.effective_transform_y);
    fields.floats(&mut r.deck_nose_dir);
    fields.floats(&mut r.deck_position);
    for wheel in &mut r.wheel_pos {
        fields.floats(wheel);
    }
    fields.floats(&mut r.deck_velocity);
    fields.float(&mut r.skateboard_surface_speed);
    fields.float(&mut r.deck_tilt);
    fields.float(&mut r.wheel_hardness);
    fields.float(&mut r.time_in_known_air);
    fields.float(&mut r.traj_collision_time);
    fields.float(&mut r.jump_height);
    fields.word(&mut r.foot_plant_surface_id);
    fields.word(&mut r.hand_plant_surface_id);
    fields.flag(&mut r.is_skateboard_airborne);
    fields.flag(&mut r.is_starting_to_jump);
    fields.flag(&mut r.is_foot_planting);
    fields.flag(&mut r.is_left_foot_planting);
    fields.flag(&mut r.is_right_foot_planting);
    fields.flag(&mut r.hand_plant_grabbing);
    fields.int(&mut r.num_wheels_in_contact);
    fields.int(&mut r.front_truck_audio_surface_type);
    fields.int(&mut r.back_truck_audio_surface_type);
    fields.int(&mut r.deck_audio_surface_type);
    fields.float(&mut r.deck_slide_speed);
    fields.ints(&mut r.wheel_audio_surface_type);
    fields.ints(&mut r.wheel_surface_pattern);
    fields.flag(&mut r.is_front_truck_in_contact);
    fields.flag(&mut r.is_back_truck_in_contact);
    fields.flag(&mut r.is_deck_in_contact);
    fields.int(&mut r.physics_state);
    fields.flag(&mut r.is_foot_braking);
    fields.flag(&mut r.is_manual_braking);
    fields.flag(&mut r.is_physically_pushing);
    fields.flag(&mut r.is_generally_pushing);
    fields.flag(&mut r.is_generally_pushing_right_foot);
    fields.flag(&mut r.is_wiping_out);
    fields.flag(&mut r.is_manual);
    fields.flag(&mut r.is_reverting);
    fields.float(&mut r.turn);
    fields.floats(&mut r.ground_up_vector);
    fields.floats(&mut r.deck_local_xyz_angular_speed);
    fields.float(&mut r.wheel_slide);
    fields.float(&mut r.jump_fraction);
    fields.int(&mut r.grind_type);
    fields.float(&mut r.grind_impact_speed);
    fields.float(&mut r.averaged_deck_contact_force);
    fields.int(&mut r.grind_audio_surface_type);
    fields.int(&mut r.off_board_jump_landing);
    fields.int(&mut r.hippy_jump_strength);
    fields.floats(&mut r.wheel_impact_strength);
    fields.flags(&mut r.wheel_landed);
    let collision = &mut r.averaged_skeleton_collision_info;
    fields.floats(&mut collision.contact_force);
    fields.floats(&mut collision.slide_speed);
    fields.ints(&mut collision.audio_surface_type);
    fields.flag(&mut collision.is_groin_contact);
    fields.flag(&mut collision.is_face_contact);
    fields.float(&mut collision.vehicle_impact);
    fields.float(&mut collision.skater_impact);
    fields.float(&mut collision.pedestrian_impact);
    fields.flag(&mut r.is_grinding);
    fields.flag(&mut r.is_on_stairs);
    fields.flag(&mut r.is_walking_up);
    fields.flag(&mut r.is_walking_down);
    fields.flag(&mut r.push_mongo);
    fields.int(&mut r.line_score);
    fields.int(&mut r.wipedout_trick_score);
    fields.int(&mut r.banked_point_change_this_frame);
    fields.int(&mut r.audio_banked_point_change);
    fields.int(&mut r.raw_scorable_id);
    fields.word(&mut r.left_foot_surface_id);
    fields.word(&mut r.right_foot_surface_id);
    fields.flag(&mut r.is_grabbing_object);
    fields.flag(&mut r.left_foot_down);
    fields.flag(&mut r.right_foot_down);
    fields.flag(&mut r.is_unmounting);
    fields.flag(&mut r.is_air_dismount);
    fields.flag(&mut r.is_holding_skateboard);
    fields.flag(&mut r.is_jumping);
    fields.flag(&mut r.is_starting_standing_jump);
    fields.float(&mut r.momentum_meter_multiplier);
    fields.float(&mut r.overall_wipeout_duration);
    fields.float(&mut r.contact_force_total_during_wipeout);
    fields.flag(&mut r.hom_bail_in_progress);
    fields.flag(&mut r.hom_effect_enabled);
    fields.word(&mut r.hom_score);
}

#[derive(Default)]
struct Pack {
    floats: Vec<f32>,
    words: Vec<u32>,
    flags: Vec<u32>,
    flag_count: u32,
}

impl Fields for Pack {
    fn float(&mut self, value: &mut f32) {
        self.floats.push(*value);
    }

    fn int(&mut self, value: &mut i32) {
        self.words.push(value.cast_unsigned());
    }

    fn word(&mut self, value: &mut u32) {
        self.words.push(*value);
    }

    fn flag(&mut self, value: &mut bool) {
        if self.flag_count % 32 == 0 {
            self.flags.push(0);
        }
        if *value {
            *self.flags.last_mut().expect("pushed") |= 1 << (self.flag_count % 32);
        }
        self.flag_count += 1;
    }
}

struct Unpack<'a> {
    floats: std::slice::Iter<'a, f32>,
    words: std::slice::Iter<'a, u32>,
    flags: &'a [u32],
    flag_count: usize,
    short: bool,
}

impl Fields for Unpack<'_> {
    fn float(&mut self, value: &mut f32) {
        match self.floats.next() {
            Some(&next) => *value = next,
            None => self.short = true,
        }
    }

    fn int(&mut self, value: &mut i32) {
        match self.words.next() {
            Some(&next) => *value = next.cast_signed(),
            None => self.short = true,
        }
    }

    fn word(&mut self, value: &mut u32) {
        match self.words.next() {
            Some(&next) => *value = next,
            None => self.short = true,
        }
    }

    fn flag(&mut self, value: &mut bool) {
        match self.flags.get(self.flag_count / 32) {
            Some(word) => *value = word & (1 << (self.flag_count % 32)) != 0,
            None => self.short = true,
        }
        self.flag_count += 1;
    }
}

/// The record's world positions (the centre of mass, the deck and its wheels) as offsets from
/// the skater's root position: they travel as those, which stay small and change slowly.
#[must_use]
pub fn relative_to_root(mut record: SkaterAudioPhysOut, root: [f32; 3]) -> SkaterAudioPhysOut {
    for position in positions(&mut record) {
        for (value, root) in position.iter_mut().zip(root) {
            *value -= root;
        }
    }
    record
}

/// [`relative_to_root`]'s offsets as world positions again, about the root position shown.
#[must_use]
pub fn at_root(mut record: SkaterAudioPhysOut, root: [f32; 3]) -> SkaterAudioPhysOut {
    for position in positions(&mut record) {
        for (value, root) in position.iter_mut().zip(root) {
            *value += root;
        }
    }
    record
}

fn positions(record: &mut SkaterAudioPhysOut) -> impl Iterator<Item = &mut [f32; 4]> {
    [&mut record.centre_of_mass, &mut record.deck_position].into_iter().chain(record.wheel_pos.iter_mut())
}

/// The record's floats, and its integers followed by its packed flags.
#[must_use]
pub fn flatten(record: &SkaterAudioPhysOut) -> (Vec<f32>, Vec<u32>) {
    let mut pack = Pack::default();
    let mut copy = record.clone();
    visit(&mut copy, &mut pack);
    let mut words = pack.words;
    words.extend(pack.flags);
    (pack.floats, words)
}

/// The record again; `None` when the lists are shorter than a record of this build's.
#[must_use]
pub fn unflatten(floats: &[f32], words: &[u32]) -> Option<SkaterAudioPhysOut> {
    let (_, layout) = flatten(&SkaterAudioPhysOut::default());
    let flag_words = layout.len() - integer_count();
    let integers = words.len().checked_sub(flag_words)?;
    let mut record = SkaterAudioPhysOut::default();
    let mut unpack = Unpack { floats: floats.iter(), words: words[..integers].iter(), flags: &words[integers..], flag_count: 0, short: false };
    visit(&mut record, &mut unpack);
    (!unpack.short).then_some(record)
}

fn integer_count() -> usize {
    let mut pack = Pack::default();
    visit(&mut SkaterAudioPhysOut::default(), &mut pack);
    pack.words.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_round_trip() {
        let mut record = SkaterAudioPhysOut::default();
        record.centre_of_mass = [1.0, 2.0, 3.0, 0.0];
        record.wheel_pos[3] = [-4.0, 5.5, 6.0, 1.0];
        record.is_manual = true;
        record.wheel_landed = [true, false, true, false];
        record.averaged_skeleton_collision_info.is_face_contact = true;
        record.raw_scorable_id = -7;
        record.hom_score = 1234;
        record.hom_effect_enabled = true;
        let (floats, words) = flatten(&record);
        assert_eq!(unflatten(&floats, &words), Some(record.clone()));
        assert_eq!(unflatten(&floats[..10], &words), None);
        let root = [100.0, -40.0, 900.0];
        let relative = relative_to_root(record.clone(), root);
        assert_eq!(relative.wheel_pos[3], [-104.0, 45.5, -894.0, 1.0]);
        assert_eq!(at_root(relative, root), record);
    }
}
