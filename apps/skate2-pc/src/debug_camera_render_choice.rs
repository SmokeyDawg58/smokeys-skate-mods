//! A free-camera view must remain the camera used for every rendered frame,
//! even while the game's presentation restorer interpolates skater animation.
pub(super) fn choose<T: Copy>(current: T, restored: Option<T>, debug_camera: bool) -> T {
    if debug_camera { current } else { restored.unwrap_or(current) }
}

#[cfg(test)]
mod tests {
    use super::choose;
    #[test]
    fn free_camera_takes_priority_without_changing_normal_restore() {
        assert_eq!(choose(10, Some(20), true), 10);
        assert_eq!(choose(10, Some(20), false), 20);
        assert_eq!(choose(10, None, false), 10);
    }
}
