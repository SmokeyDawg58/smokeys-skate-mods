//! `sponsor_browser` (controls/sponsor_browser, the sponsor overlay's `mSponsorSelector`): a
//! window of `MAX_SPONSORS` (3 by 3) logos over the sponsor list, and its `basic_slider`
//! (`mSlider`). The ActionScript keeps the selection; the native reads it back through
//! `SetIndexPicked` when the overlay leaves.

/// `sponsor_browser.WindowRowSize`.
const ROW_SIZE: i32 = 3;
/// `sponsor_browser.MAX_SPONSORS`: `WindowRowSize * WindowColSize`.
pub const MAX_SPONSORS: i32 = 9;

/// `basic_slider`'s value and window, as `mSlider` keeps them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Slider {
    /// `mCurrentValue`.
    pub current: i32,
    /// `mWindowedValue`: the first item of the window, which places the indicator.
    pub windowed: i32,
    /// `mWindowLocalVal`.
    pub local: i32,
    /// `mNumItems`.
    pub items: i32,
    /// `mNumVisibleRows`.
    pub rows: i32,
    /// `mEnableGUI`, which `UpdateDisplay` sets while the items outnumber the rows.
    pub enabled: bool,
}

impl Slider {
    /// The constructor: no value, ten items in one row, the GUI on.
    #[must_use]
    pub const fn new() -> Self {
        Self { current: 0, windowed: 0, local: 0, items: 10, rows: 1, enabled: true }
    }

    /// `UpdateDisplay`: the scroll bar and its input on while the items outnumber the rows.
    fn update_display(&mut self) {
        self.enabled = self.items > self.rows;
    }

    /// `SetNumberOfItems(numItems, numVisibleItems)`.
    pub(crate) fn set_number_of_items(&mut self, items: i32, rows: i32) {
        self.items = items;
        self.rows = rows;
    }

    /// `SetValue(n)`: the window follows the value.
    pub(crate) fn set_value(&mut self, value: i32) {
        self.current = value;
        if self.current < self.windowed {
            self.windowed = self.current;
        } else if self.current >= self.windowed + self.rows {
            self.windowed = self.current - self.rows + 1;
        }
        self.local = self.current - self.windowed;
        self.update_display();
    }

    /// `NextValue`, only while the GUI is enabled.
    fn next_value(&mut self) {
        if !self.enabled {
            return;
        }
        self.current += 1;
        self.local += 1;
        if self.current > self.items - 1 {
            self.current = self.items - 1;
        }
        if self.local > self.rows - 1 {
            self.local = self.rows - 1;
            self.windowed += 1;
            if self.windowed > self.items - self.rows {
                self.windowed = self.items - self.rows;
            }
        }
        self.update_display();
    }

    /// `PrevValue`, only while the GUI is enabled.
    fn prev_value(&mut self) {
        if !self.enabled {
            return;
        }
        self.current -= 1;
        self.local -= 1;
        if self.current < 0 {
            self.current = 0;
        }
        if self.local < 0 {
            self.local = 0;
            self.windowed -= 1;
            if self.windowed < 0 {
                self.windowed = 0;
            }
        }
        self.update_display();
    }
}

/// A browser key, as `UpdateInput` takes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Right,
    Left,
    Down,
    Up,
}

/// The `sponsor_browser` instance.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SponsorBrowser {
    /// `CurrentValue`.
    pub current: i32,
    /// `NumItems`.
    pub items: i32,
    /// `mWindowIndex`: the highlighted thumbnail.
    pub window_index: i32,
    /// `mMinWindowIndex`: the item in the first thumbnail.
    pub min_window_index: i32,
    /// `mMaxWindowIndex`.
    pub max_window_index: i32,
    /// `mSlider`.
    pub slider: Slider,
}

impl SponsorBrowser {
    /// The constructor: no items, the window at the start, and the slider as `basic_slider`'s
    /// constructor leaves it (ten items, one row, the GUI on).
    #[must_use]
    pub fn new() -> Self {
        Self { max_window_index: MAX_SPONSORS - 1, slider: Slider::new(), ..Self::default() }
    }

    /// `Init(numItems, swfArray)`: `SetNumItems`, the selection and window back to the start.
    pub fn init(&mut self, items: i32) {
        self.items = items;
        // SetNumItems: the slider's length, SetNumberOfItems(NumItems, MAX_SPONSORS) and
        // SetValue(CurrentValue).
        self.slider.set_number_of_items(items, MAX_SPONSORS);
        self.slider.set_value(self.current);
        self.current = 0;
        self.window_index = 0;
        self.min_window_index = 0;
        self.max_window_index = MAX_SPONSORS - 1;
    }

    /// `GetSelectedIndex`.
    #[must_use]
    pub const fn selected_index(&self) -> i32 {
        self.min_window_index + self.window_index
    }

    /// `NextValue`: one item on, the window a row down past its last item, then the slider.
    fn next_value(&mut self) {
        self.current += 1;
        if self.current > self.items - 1 {
            self.current = self.items - 1;
        } else if self.current > self.max_window_index {
            self.window_index += 1 - ROW_SIZE;
            self.min_window_index += ROW_SIZE;
            self.max_window_index += ROW_SIZE;
        } else {
            self.window_index += 1;
        }
        self.slider.next_value();
    }

    /// `PrevValue`: one item back, the window a row up before its first item, then the slider.
    fn prev_value(&mut self) {
        self.current -= 1;
        if self.current < 0 {
            self.current = 0;
        } else if self.current < self.min_window_index {
            self.window_index += ROW_SIZE - 1;
            self.min_window_index -= ROW_SIZE;
            self.max_window_index -= ROW_SIZE;
        } else {
            self.window_index -= 1;
        }
        self.slider.prev_value();
    }

    /// `UpdateInput`: right and left step one item, down and up a row (three steps); each sounds
    /// `FE_SOUND_SPONSOR_NAV`, which the caller plays.
    pub fn update_input(&mut self, key: Key) {
        match key {
            Key::Right => self.next_value(),
            Key::Left => self.prev_value(),
            Key::Down => {
                for _ in 0..ROW_SIZE {
                    self.next_value();
                }
            }
            Key::Up => {
                for _ in 0..ROW_SIZE {
                    self.prev_value();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_scroll_the_window_and_clamp_at_the_ends() {
        let mut browser = SponsorBrowser::new();
        browser.init(14);
        assert_eq!((browser.window_index, browser.min_window_index, browser.slider.enabled), (0, 0, true));
        browser.update_input(Key::Down);
        browser.update_input(Key::Down);
        assert_eq!((browser.current, browser.window_index, browser.min_window_index), (6, 6, 0));
        browser.update_input(Key::Down);
        assert_eq!((browser.current, browser.window_index, browser.min_window_index), (9, 6, 3));
        assert_eq!(browser.selected_index(), 9);
        browser.update_input(Key::Down);
        assert_eq!((browser.current, browser.window_index, browser.min_window_index), (12, 6, 6));
        browser.update_input(Key::Right);
        browser.update_input(Key::Right);
        assert_eq!((browser.current, browser.selected_index()), (13, 13));
        assert_eq!(browser.slider.windowed, 5);
        browser.update_input(Key::Up);
        assert_eq!((browser.current, browser.window_index, browser.min_window_index), (10, 4, 6));
    }

    #[test]
    fn a_window_that_holds_every_item_leaves_the_slider_off() {
        let mut browser = SponsorBrowser::new();
        browser.init(7);
        assert!(!browser.slider.enabled);
        browser.update_input(Key::Down);
        browser.update_input(Key::Down);
        browser.update_input(Key::Right);
        assert_eq!((browser.current, browser.selected_index()), (6, 6));
    }
}
