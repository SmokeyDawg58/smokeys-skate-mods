//! `DisableTricks` instance and timer, source82ca1f78/4c00/4cf8.
#[derive(Default)]
pub(crate) struct Instance {
    pub(crate) timer: f32,
    pub(crate) completed: bool,
}
impl Instance {
    pub(crate) fn begin<E>(
        &mut self,
        length: f32,
        mut set: impl FnMut(bool) -> Result<(), E>,
    ) -> Result<(), E> {
        set(false)?;
        self.timer = length;
        self.completed = false;
        Ok(())
    }
    pub(crate) fn update<E>(
        &mut self,
        dt: f32,
        mut set: impl FnMut(bool) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.completed {
            return Ok(());
        }
        self.timer -= dt;
        // Raw409800d8: unordered also skips expiration.
        if self.timer.partial_cmp(&0.0) != Some(std::cmp::Ordering::Less) {
            return Ok(());
        }
        set(true)?;
        self.completed = true;
        Ok(())
    }
}
#[cfg(test)]
#[path = "motion_disable_tricks_tests.rs"]
mod tests;
