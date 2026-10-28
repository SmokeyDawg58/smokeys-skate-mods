//! FrontEndManager EnableBackground826066c8 / UpdateFade82614f10.
//! Stepped by the frontend input clock, never by uncapped render frequency.
#[derive(Default)]
pub(crate) struct FrontendFade {
    alpha: f32,
    direction: i8,
    delay: i32,
    black_presented: bool,
}
impl FrontendFade {
    pub(crate) fn cover(&mut self) {
        self.direction = 1;
        self.delay = 0;
        self.black_presented = false;
    }
    /// `EnableBackground(1, bSnap = 1)`: black at once.
    pub(crate) fn cover_snap(&mut self) {
        self.cover();
        self.alpha = 1.;
    }
    pub(crate) fn reveal(&mut self) {
        self.direction = -1;
        self.delay = 10; // EnableBackground callers' fadeoutcount.
    }
    pub(crate) fn update(&mut self, speed: f32) {
        if self.direction < 0 { self.delay -= 1; }
        if self.delay <= 0 {
            self.delay = 0;
            self.alpha = (self.alpha + f32::from(self.direction) * speed).clamp(0., 1.);
            if self.alpha == 0. { self.direction = 0; }
        }
    }
    pub(crate) fn alpha(&self) -> f32 { self.alpha }
    /// `mbFadeBackground`: a fade has started and not yet faded out.
    pub(crate) const fn fading(&self) -> bool { self.direction != 0 }
    // PC loading is synchronous in places. Do not start it until a fully
    // covered frame has actually reached the swapchain (timeouts don't count).
    pub(crate) fn presented(&mut self, alpha: f32) {
        if alpha == 1. && self.alpha == 1. { self.black_presented = true; }
    }
    pub(crate) fn covered(&self) -> bool { self.alpha == 1. && self.black_presented }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_fade_waits_for_black_and_delays_reveal() {
        let mut fade = FrontendFade::default();
        fade.cover();
        for _ in 0..35 { fade.update(0.028); }
        assert!(fade.alpha() < 1.);
        fade.update(0.028);
        assert_eq!(fade.alpha(), 1.);
        assert!(!fade.covered());
        fade.presented(0.98);
        assert!(!fade.covered());
        fade.presented(1.);
        assert!(fade.covered());
        for _ in 0..120 { fade.update(0.028); }
        assert_eq!(fade.alpha(), 1.);
        fade.reveal();
        for _ in 0..9 { fade.update(0.028); }
        assert_eq!(fade.alpha(), 1.);
        fade.update(0.028);
        assert_eq!(fade.alpha(), 1. - 0.028);
        for _ in 0..35 { fade.update(0.028); }
        assert_eq!(fade.alpha(), 0.);
    }
}
