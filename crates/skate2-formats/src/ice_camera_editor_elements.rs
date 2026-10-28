//! ICELITE::RegisterElements (0x82a651e0), recovered constructor arguments.
//! This is the editor schema; ICE cinematic ids and channels differ.
use super::super::{Element, el};
pub(super) const ELEMENTS: [Element; 135] = [
    el(0, 0, false, 4, 32, -4.699999809265137, -100000.0, 100000.0, 84, 7), // EYE_X
    el(1, 0, false, 4, 32, 0.0, -100000.0, 100000.0, 84, 7), // EYE_Y
    el(2, 0, false, 4, 32, 1.600000023841858, -100000.0, 100000.0, 84, 7), // EYE_Z
    el(3, 0, false, 4, 32, 0.0, -100000.0, 100000.0, 85, 8), // LOOK_X
    el(4, 0, false, 4, 32, 0.0, -100000.0, 100000.0, 85, 8), // LOOK_Y
    el(5, 0, false, 4, 32, 0.0, -100000.0, 100000.0, 85, 8), // LOOK_Z
    el(6, 0, false, 3, 10, 0.0, -0.25, 0.25, 84, 7), // DUTCH
    el(7, 0, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_EYE
    el(8, 0, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_LOOK
    el(9, 0, false, 3, 9, 24.0, 10.0, 500.0, 84, 7), // LENS_LENGTH
    el(10, 1, false, 3, 9, 24.0, 10.0, 500.0, 97, 12), // LENS_LENGTH_AUX
    el(11, 1, false, 3, 12, 0.5, 0.05000000074505806, 100.0, 65535, 65535), // NEAR_CLIP
    el(12, 1, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_LENS
    el(13, 1, false, 3, 12, 0.05000000074505806, 0.05000000074505806, 100.0, 65535, 65535), // CLIPPING_FAR
    el(14, 1, false, 3, 12, 0.10000000149011612, 0.05000000074505806, 100.0, 65535, 65535), // CLIPPING_NEAR
    el(15, 2, false, 3, 16, 0.0, 0.0, 10000.0, 99, 17), // FOCAL_DISTANCE
    el(16, 2, false, 1, 6, 37.0, 0.0, 37.0, 99, 17), // APERTURE
    el(17, 2, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_FOCUS
    el(18, 3, false, 3, 16, 0.0, 0.0, 10000.0, 100, 22), // NEAR_FOCUS
    el(19, 3, false, 3, 16, 0.0, 0.0, 10000.0, 100, 22), // FAR_FOCUS
    el(20, 3, false, 3, 7, 0.0, 0.0, 1.0, 100, 22), // BLUR_FALLOFF
    el(21, 3, false, 3, 7, 0.0, 0.0, 1.0, 100, 22), // BLUR_INTENSITY
    el(22, 3, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_RAWFOCUS
    el(23, 4, false, 3, 7, 0.0, 0.0, 1.0, 65535, 65535), // SHAKE_AMPLITUDE
    el(24, 4, false, 3, 7, 0.0, 0.0, 1.0, 65535, 65535), // SHAKE_FREQUENCY
    el(25, 5, false, 1, 7, 100.0, 0.0, 100.0, 65535, 65535), // TIME_SCALE
    el(26, 8, false, 1, 7, 0.0, 0.0, 100.0, 65535, 65535), // LETTERBOX
    el(27, 9, false, 1, 7, 0.0, 0.0, 100.0, 65535, 65535), // FADE
    el(28, 10, false, 3, 8, 0.0, 0.0, 360.0, 65535, 65535), // LIGHT_RIG_DIR_U
    el(29, 10, false, 3, 8, 0.0, -90.0, 90.0, 65535, 65535), // LIGHT_RIG_DIR_V
    el(30, 10, false, 3, 8, 1.0, 0.0, 2.0, 65535, 65535), // LIGHT_RIG_KEY_INTENSITY
    el(31, 10, false, 3, 8, 1.0, 0.0, 2.0, 65535, 65535), // LIGHT_RIG_SEC_INTENSITY
    el(32, 10, false, 3, 8, 1.0, 0.0, 2.0, 65535, 65535), // LIGHT_RIG_FILL_INTENSITY
    el(33, 10, false, 3, 8, 1.0, 0.0, 2.0, 65535, 65535), // LIGHT_RIG_RIM_INTENSITY
    el(34, 11, false, 3, 16, 0.0, 0.0, 4.0, 65535, 65535), // TONEMAP_VALUE
    el(35, 12, false, 3, 8, 1.0, 0.0, 10.0, 65535, 65535), // MOTION_BLUR_VALUE
    el(36, 13, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // BLOOM_INTENSITY
    el(37, 13, false, 3, 8, 0.8999999761581421, 0.0, 1.0, 65535, 65535), // BLOOM_THRESHOLD
    el(38, 14, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // NOISE_INTENSITY
    el(39, 14, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // NOISE_SCALEX
    el(40, 14, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // NOISE_SCALEY
    el(41, 14, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // NOISE_SCROLLRATE
    el(42, 14, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // NOISE_SCROLLX
    el(43, 14, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // NOISE_SCROLLY
    el(44, 15, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // FISHEYE_HORIZONTAL
    el(45, 15, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // FISHEYE_MULTIPLIER
    el(46, 15, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // FISHEYE_VERTICAL
    el(47, 16, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // FOG_COLOR_X
    el(48, 16, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // FOG_COLOR_Y
    el(49, 16, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // FOG_COLOR_Z
    el(50, 16, false, 4, 32, 100.0, 0.0, 10000.0, 65535, 65535), // FOG_DISTANCE
    el(51, 16, false, 3, 8, 0.8999999761581421, 0.0, 1.0, 65535, 65535), // FOG_MAX
    el(52, 16, false, 4, 32, 100.0, 0.0, 10000.0, 65535, 65535), // FOG_START
    el(53, 17, false, 4, 32, 100.0, 0.0, 10000.0, 65535, 65535), // INTERPOLATION_FAR
    el(54, 17, false, 4, 32, 100.0, 0.0, 10000.0, 65535, 65535), // INTERPOLATION_NEAR
    el(55, 18, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // BRIGHTNESS_FAR
    el(56, 18, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // BRIGHTNESS_NEAR
    el(57, 19, false, 3, 8, 1.0, 0.0, 50.0, 65535, 65535), // CONTRAST_FAR
    el(58, 19, false, 3, 8, 0.5, 0.0, 1.0, 65535, 65535), // CONTRAST_FAR_MIDPOINT
    el(59, 19, false, 3, 8, 1.0, 0.0, 50.0, 65535, 65535), // CONTRAST_NEAR
    el(60, 19, false, 3, 8, 0.5, 0.0, 1.0, 65535, 65535), // CONTRAST_NEAR_MIDPOINT
    el(61, 20, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // GREYSCALE_FAR
    el(62, 20, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // GREYSCALE_NEAR
    el(63, 21, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // SATURATE_FAR
    el(64, 21, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // SATURATE_NEAR
    el(65, 22, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // TINT_NEAR_COLOR_X
    el(66, 22, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // TINT_NEAR_COLOR_Y
    el(67, 22, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // TINT_NEAR_COLOR_Z
    el(68, 22, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // TINT_FAR_COLOR_X
    el(69, 22, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // TINT_FAR_COLOR_Y
    el(70, 22, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // TINT_FAR_COLOR_Z
    el(71, 23, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // POST_MODULATE_NEAR
    el(72, 23, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // POST_MODULATE_FAR
    el(73, 24, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // POST_QUANTISE_NEAR
    el(74, 24, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // POST_QUANTISE_FAR
    el(75, 25, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // POST_BRIGHTNESS_NEAR
    el(76, 25, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // POST_BRIGHTNESS_FAR
    el(77, 26, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // CHARACTER_EFFECT_MASK_VALUE
    el(78, 33, false, 3, 16, 0.0, -1.0, 1.0, 65535, 65535), // SHAKE_QUAT_X
    el(79, 33, false, 3, 16, 0.0, -1.0, 1.0, 65535, 65535), // SHAKE_QUAT_Y
    el(80, 33, false, 3, 16, 0.0, -1.0, 1.0, 65535, 65535), // SHAKE_QUAT_Z
    el(81, 33, false, 3, 16, 0.0, -1.0, 1.0, 65535, 65535), // SHAKE_POS_X
    el(82, 33, false, 3, 16, 0.0, -1.0, 1.0, 65535, 65535), // SHAKE_POS_Y
    el(83, 33, false, 3, 16, 0.0, -1.0, 1.0, 65535, 65535), // SHAKE_POS_Z
    el(84, 0, true, 1, 1, 1.0, 0.0, 1.0, 65535, 65535), // CUBIC_EYE
    el(85, 0, true, 1, 1, 1.0, 0.0, 1.0, 65535, 65535), // CUBIC_LOOK
    el(86, 0, true, 1, 3, 0.0, 0.0, 4.0, 65535, 65535), // SPACE_EYE
    el(87, 0, true, 1, 3, 0.0, 0.0, 4.0, 65535, 65535), // SPACE_LOOK
    el(88, 0, true, 1, 5, 0.0, 0.0, 31.0, 65535, 65535), // AVATAR_EYE
    el(89, 0, true, 1, 5, 0.0, 0.0, 31.0, 65535, 65535), // AVATAR_LOOK
    el(90, 0, true, 1, 5, 0.0, 0.0, 31.0, 65535, 65535), // AVATAR_ORBIT
    el(91, 0, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // AVOID_DYNAMIC_OBJECTS
    el(92, 0, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // AVOID_STATIC_OBJECTS
    el(93, 0, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // PLAYER_SPACE_LOCK_TO_RENDER_OBJECT
    el(94, 0, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // PLAYER_SPACE_NOROLL
    el(95, 0, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // PLAYER_SPACE_LAG
    el(96, 0, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // PLAYER_SPACE_BUNGEE
    el(97, 1, true, 1, 1, 1.0, 0.0, 1.0, 65535, 65535), // CUBIC_LENS
    el(98, 1, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // LENS_OVERRIDE
    el(99, 2, true, 1, 1, 1.0, 0.0, 1.0, 65535, 65535), // CUBIC_FOCUS
    el(100, 3, true, 1, 1, 1.0, 0.0, 1.0, 65535, 65535), // CUBIC_RAWFOCUS
    el(101, 3, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // RAWFOCUS_OVERRIDE
    el(102, 4, true, 1, 5, 0.0, 0.0, 14.0, 65535, 65535), // SHAKE_TYPE
    el(103, 6, true, 2, 32, 0.0, 0.0, 268435455.0, 65535, 65535), // EVENT_TAG
    el(104, 7, true, 1, 16, 0.0, 0.0, 65535.0, 65535, 65535), // OVERLAY
    el(105, 9, true, 1, 3, 0.0, 0.0, 5.0, 65535, 65535), // FADE_TO_COLOR
    el(106, 11, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // TONEMAP_OVERRIDE
    el(107, 12, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // MOTION_BLUR_OVERRIDE
    el(108, 12, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // MOTION_BLUR_OBJECTAPPLY
    el(109, 13, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // BLOOM_OVERRIDE
    el(110, 15, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // FISHEYE_OVERRIDE
    el(111, 16, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // FOG_OVERRIDE
    el(112, 17, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // NEAR_FAR_OVERRIDE
    el(113, 18, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // BRIGHTNESS_OVERRIDE
    el(114, 19, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // CONTRAST_OVERRIDE
    el(115, 20, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // GREYSCALE_OVERRIDE
    el(116, 21, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // SATURATE_OVERRIDE
    el(117, 22, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // TINT_NEAR_NEGATIVE
    el(118, 22, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // TINT_FAR_NEGATIVE
    el(119, 22, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // TINT_OVERRIDE
    el(120, 26, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // CHARACTER_EFFECT_MASK_OVERRIDE
    el(121, 27, true, 1, 2, 0.0, 0.0, 3.0, 65535, 65535), // AUDIO_EFFECT_CMD
    el(122, 27, true, 1, 32, 0.0, 0.0, 65535.0, 65535, 65535), // AUDIO_EFFECT_ID
    el(123, 28, true, 1, 2, 0.0, 0.0, 3.0, 65535, 65535), // AUDIO_MUSIC_CMD
    el(124, 28, true, 1, 32, 0.0, 0.0, 65535.0, 65535, 65535), // AUDIO_MUSIC_ID
    el(125, 29, true, 1, 2, 0.0, 0.0, 3.0, 65535, 65535), // AUDIO_SPEECH_CMD
    el(126, 29, true, 1, 32, 5000.0, 5000.0, 65536.0, 65535, 65535), // AUDIO_SPEECH_ID
    el(127, 29, true, 1, 8, 0.0, 0.0, 255.0, 65535, 65535), // AUDIO_SPEECH_CHARACTER
    el(128, 30, true, 1, 2, 0.0, 0.0, 3.0, 65535, 65535), // AUDIO_POST_CMD
    el(129, 30, true, 1, 32, 5000.0, 5000.0, 65536.0, 65535, 65535), // AUDIO_POST_ID
    el(130, 31, true, 1, 4, 0.0, 0.0, 15.0, 65535, 65535), // FREEZE_CMD
    el(131, 31, true, 1, 16, 0.0, 0.0, 300.0, 65535, 65535), // FREEZE_TIME
    el(132, 32, true, 3, 16, 0.0, 0.0, 1.0, 65535, 65535), // TAKE_START
    el(133, 32, true, 1, 6, 0.0, 0.0, 63.0, 65535, 65535), // TAKE_NUMBER
    el(134, 32, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // CONTAINS_SUBTAKE
];

/// Semantic ids shared with the cinematic camera math, checked by native tag.
pub(super) const CINEMATIC_IDS: [Option<usize>; 135] = [
    Some(0), // EYE_X
    Some(1), // EYE_Y
    Some(2), // EYE_Z
    Some(3), // LOOK_X
    Some(4), // LOOK_Y
    Some(5), // LOOK_Z
    Some(6), // DUTCH
    Some(7), // TANGENT_EYE
    Some(8), // TANGENT_LOOK
    Some(9), // LENS_LENGTH
    Some(10), // LENS_LENGTH_AUX
    Some(11), // NEAR_CLIP
    Some(12), // TANGENT_LENS
    Some(13), // CLIPPING_FAR
    Some(14), // CLIPPING_NEAR
    Some(15), // FOCAL_DISTANCE
    Some(16), // APERTURE
    Some(17), // TANGENT_FOCUS
    Some(18), // NEAR_FOCUS
    Some(19), // FAR_FOCUS
    Some(20), // BLUR_FALLOFF
    Some(21), // BLUR_INTENSITY
    Some(22), // TANGENT_RAWFOCUS
    Some(23), // SHAKE_AMPLITUDE
    Some(24), // SHAKE_FREQUENCY
    Some(25), // TIME_SCALE
    Some(26), // LETTERBOX
    Some(27), // FADE
    Some(28), // LIGHT_RIG_DIR_U
    Some(29), // LIGHT_RIG_DIR_V
    Some(30), // LIGHT_RIG_KEY_INTENSITY
    Some(31), // LIGHT_RIG_SEC_INTENSITY
    Some(32), // LIGHT_RIG_FILL_INTENSITY
    Some(33), // LIGHT_RIG_RIM_INTENSITY
    Some(34), // TONEMAP_VALUE
    Some(35), // MOTION_BLUR_VALUE
    Some(36), // BLOOM_INTENSITY
    Some(37), // BLOOM_THRESHOLD
    Some(40), // NOISE_INTENSITY
    Some(41), // NOISE_SCALEX
    Some(42), // NOISE_SCALEY
    Some(43), // NOISE_SCROLLRATE
    Some(44), // NOISE_SCROLLX
    Some(45), // NOISE_SCROLLY
    Some(46), // FISHEYE_HORIZONTAL
    Some(47), // FISHEYE_MULTIPLIER
    Some(48), // FISHEYE_VERTICAL
    None, // FOG_COLOR_X
    None, // FOG_COLOR_Y
    None, // FOG_COLOR_Z
    None, // FOG_DISTANCE
    None, // FOG_MAX
    None, // FOG_START
    Some(49), // INTERPOLATION_FAR
    Some(50), // INTERPOLATION_NEAR
    Some(51), // BRIGHTNESS_FAR
    Some(52), // BRIGHTNESS_NEAR
    Some(53), // CONTRAST_FAR
    Some(54), // CONTRAST_FAR_MIDPOINT
    Some(55), // CONTRAST_NEAR
    Some(56), // CONTRAST_NEAR_MIDPOINT
    None, // GREYSCALE_FAR
    None, // GREYSCALE_NEAR
    Some(57), // SATURATE_FAR
    Some(58), // SATURATE_NEAR
    Some(59), // TINT_NEAR_COLOR_X
    Some(60), // TINT_NEAR_COLOR_Y
    Some(61), // TINT_NEAR_COLOR_Z
    Some(62), // TINT_FAR_COLOR_X
    Some(63), // TINT_FAR_COLOR_Y
    Some(64), // TINT_FAR_COLOR_Z
    None, // POST_MODULATE_NEAR
    None, // POST_MODULATE_FAR
    None, // POST_QUANTISE_NEAR
    None, // POST_QUANTISE_FAR
    None, // POST_BRIGHTNESS_NEAR
    None, // POST_BRIGHTNESS_FAR
    Some(65), // CHARACTER_EFFECT_MASK_VALUE
    Some(81), // SHAKE_QUAT_X
    Some(82), // SHAKE_QUAT_Y
    Some(83), // SHAKE_QUAT_Z
    Some(84), // SHAKE_POS_X
    Some(85), // SHAKE_POS_Y
    Some(86), // SHAKE_POS_Z
    Some(87), // CUBIC_EYE
    Some(88), // CUBIC_LOOK
    Some(89), // SPACE_EYE
    Some(90), // SPACE_LOOK
    Some(91), // AVATAR_EYE
    Some(92), // AVATAR_LOOK
    Some(93), // AVATAR_ORBIT
    Some(94), // AVOID_DYNAMIC_OBJECTS
    Some(95), // AVOID_STATIC_OBJECTS
    Some(96), // PLAYER_SPACE_LOCK_TO_RENDER_OBJECT
    Some(97), // PLAYER_SPACE_NOROLL
    Some(98), // PLAYER_SPACE_LAG
    Some(99), // PLAYER_SPACE_BUNGEE
    Some(100), // CUBIC_LENS
    Some(101), // LENS_OVERRIDE
    Some(102), // CUBIC_FOCUS
    Some(103), // CUBIC_RAWFOCUS
    Some(104), // RAWFOCUS_OVERRIDE
    Some(105), // SHAKE_TYPE
    Some(106), // EVENT_TAG
    Some(107), // OVERLAY
    Some(108), // FADE_TO_COLOR
    Some(109), // TONEMAP_OVERRIDE
    Some(110), // MOTION_BLUR_OVERRIDE
    Some(111), // MOTION_BLUR_OBJECTAPPLY
    Some(112), // BLOOM_OVERRIDE
    Some(115), // FISHEYE_OVERRIDE
    None, // FOG_OVERRIDE
    Some(116), // NEAR_FAR_OVERRIDE
    Some(117), // BRIGHTNESS_OVERRIDE
    Some(118), // CONTRAST_OVERRIDE
    None, // GREYSCALE_OVERRIDE
    Some(119), // SATURATE_OVERRIDE
    Some(120), // TINT_NEAR_NEGATIVE
    Some(121), // TINT_FAR_NEGATIVE
    Some(122), // TINT_OVERRIDE
    Some(123), // CHARACTER_EFFECT_MASK_OVERRIDE
    Some(132), // AUDIO_EFFECT_CMD
    Some(133), // AUDIO_EFFECT_ID
    Some(134), // AUDIO_MUSIC_CMD
    Some(135), // AUDIO_MUSIC_ID
    Some(136), // AUDIO_SPEECH_CMD
    Some(137), // AUDIO_SPEECH_ID
    Some(138), // AUDIO_SPEECH_CHARACTER
    Some(139), // AUDIO_POST_CMD
    Some(140), // AUDIO_POST_ID
    Some(141), // FREEZE_CMD
    Some(142), // FREEZE_TIME
    Some(143), // TAKE_START
    Some(144), // TAKE_NUMBER
    Some(145), // CONTAINS_SUBTAKE
];
