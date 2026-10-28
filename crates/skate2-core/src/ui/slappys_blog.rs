//! The selection of Slappy's blog (screens/slappys/slappys_blog, the `slappys_blog` class) and
//! of a magazine article (screens/slappys/articles, `articles`), whose ActionScript keeps it and
//! reports a choice through `Game.OnMenuSelect`. Each change records the class calls it made
//! with the screen as it was then, which the movie replays.

use super::sponsor_browser::Slider;

/// `slappys_blog.mMaxVisibleBlogs`.
pub const MAX_VISIBLE_BLOGS: i32 = 3;
/// `slappys_blog.mMaxNumPanelOptions`.
pub const MAX_NUM_PANEL_OPTIONS: i32 = 3;

/// A key as the screens' `UpdateInput` takes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Right,
    Left,
    Down,
    Up,
    /// `AptNext` (A).
    Next,
}

/// A call of the `slappys_blog` class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlogCall {
    /// `SetState(state)`: the thumbnails destroyed when the state changes, the panel's label,
    /// `UpdateBanner`, `UpdateBlogScrollbar`, `UpdateBlog`, `UpdateSelectedBlog`, the panel's
    /// `UpdateSelectedBlogPanel` (Slappy's own state only) and `UpdateButtonHelp`.
    SetState { previous: i32, previous_items: i32 },
    UpdateBlog,
    UpdateSelectedBlog,
    UpdateSelectedBlogPanel,
    UpdateButtonHelp,
    /// `mScroller.SetValue(mCurrentIndex)`.
    ScrollerValue,
}

/// The `slappys_blog` instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlogScreen {
    /// `mState`: the screen state (`eSlappysScreenMode`).
    pub state: i32,
    /// `mBlogState`: the panel (0) or the list (1).
    pub blog_state: i32,
    /// `mCurrentIndex`.
    pub current: i32,
    /// `mMinWindowIndex` and `mMaxWindowIndex`.
    pub min_window: i32,
    pub max_window: i32,
    /// `mNumItems`.
    pub num_items: i32,
    /// `mPanel.mScroller`.
    pub scroller: Slider,
}

impl Default for BlogScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl BlogScreen {
    /// The constructor: state 0 and no items.
    #[must_use]
    pub const fn new() -> Self {
        Self { state: 0, blog_state: 0, current: 0, min_window: 0, max_window: 0, num_items: 0, scroller: Slider::new() }
    }

    fn call(&self, calls: &mut Vec<(BlogCall, Self)>, call: BlogCall) {
        calls.push((call, *self));
    }

    /// `SetState(state)` with `Slappys_GetNumItems`: Slappy's own state selects the panel without
    /// items and the list with them; a magazine's selects its list.
    pub fn set_state(&mut self, state: i32, num_items: i32, calls: &mut Vec<(BlogCall, Self)>) {
        let (previous, previous_items) = (self.state, self.num_items);
        self.state = state;
        if !(0..=2).contains(&state) {
            return;
        }
        self.current = 0;
        self.min_window = 0;
        self.max_window = MAX_VISIBLE_BLOGS - 1;
        self.num_items = num_items;
        self.blog_state = i32::from(state != 0 || num_items > 0);
        self.scroller.set_number_of_items(num_items, MAX_VISIBLE_BLOGS);
        self.scroller.set_value(self.current);
        self.call(calls, BlogCall::SetState { previous, previous_items });
    }

    /// `SetBlogState(state)`.
    fn set_blog_state(&mut self, blog_state: i32, calls: &mut Vec<(BlogCall, Self)>) {
        self.blog_state = blog_state;
        self.call(calls, BlogCall::UpdateSelectedBlog);
        self.call(calls, BlogCall::UpdateSelectedBlogPanel);
        self.call(calls, BlogCall::UpdateButtonHelp);
    }

    /// `NextBlog`: one row on, the window a row down past its last, then the scroller.
    fn next_blog(&mut self, calls: &mut Vec<(BlogCall, Self)>) {
        self.current += 1;
        if self.current > self.num_items - 1 {
            self.current = self.num_items - 1;
        } else if self.current > self.max_window {
            self.min_window += 1;
            self.max_window += 1;
            self.call(calls, BlogCall::UpdateBlog);
        }
        self.scroller.set_value(self.current);
        self.call(calls, BlogCall::ScrollerValue);
    }

    /// `PrevBlog`.
    fn prev_blog(&mut self, calls: &mut Vec<(BlogCall, Self)>) {
        self.current -= 1;
        if self.current < 0 {
            self.current = 0;
        } else if self.current < self.min_window {
            self.min_window -= 1;
            self.max_window -= 1;
            self.call(calls, BlogCall::UpdateBlog);
        }
        self.scroller.set_value(self.current);
        self.call(calls, BlogCall::ScrollerValue);
    }

    /// `UpdateInput(keycode)`: in Slappy's own state right and left move between the panel and
    /// the list, up and down step the panel's options or the list; a magazine's state steps its
    /// list. Each move sounds `FE_SOUND_BLOG_NAV`; A sounds `FE_SOUND_CORE_A_BUTTON` and returns
    /// `Game.OnMenuSelect`'s menu (the panel 0, the list 1) and index.
    pub fn update_input(&mut self, key: Key, sounds: &mut Vec<&'static str>, calls: &mut Vec<(BlogCall, Self)>) -> Option<(i32, i32)> {
        const NAV: &str = "FE_SOUND_BLOG_NAV";
        match (self.state, key) {
            (0, Key::Right) => {
                if self.blog_state == 0 && self.num_items > 0 {
                    sounds.push(NAV);
                    self.current += self.min_window;
                    if self.current >= self.num_items {
                        self.current = self.num_items - 1;
                    }
                    self.set_blog_state(1, calls);
                }
            }
            (0, Key::Left) => {
                if self.blog_state == 1 {
                    sounds.push(NAV);
                    self.current -= self.min_window;
                    self.set_blog_state(0, calls);
                }
            }
            (0, Key::Down) => {
                if self.blog_state == 0 {
                    sounds.push(NAV);
                    self.current += 1;
                    if self.current >= MAX_NUM_PANEL_OPTIONS {
                        self.current = MAX_NUM_PANEL_OPTIONS - 1;
                    }
                    self.call(calls, BlogCall::UpdateSelectedBlogPanel);
                } else if self.blog_state == 1 {
                    sounds.push(NAV);
                    self.next_blog(calls);
                    self.call(calls, BlogCall::UpdateSelectedBlog);
                    self.call(calls, BlogCall::UpdateButtonHelp);
                }
            }
            (0, Key::Up) => {
                if self.blog_state == 0 {
                    sounds.push(NAV);
                    self.current -= 1;
                    if self.current < 0 {
                        self.current = 0;
                    }
                    self.call(calls, BlogCall::UpdateSelectedBlogPanel);
                } else if self.blog_state == 1 {
                    sounds.push(NAV);
                    self.prev_blog(calls);
                    self.call(calls, BlogCall::UpdateSelectedBlog);
                    self.call(calls, BlogCall::UpdateButtonHelp);
                }
            }
            (1 | 2, Key::Down) => {
                sounds.push(NAV);
                self.next_blog(calls);
                self.call(calls, BlogCall::UpdateSelectedBlog);
                self.call(calls, BlogCall::UpdateButtonHelp);
            }
            (1 | 2, Key::Up) => {
                sounds.push(NAV);
                self.prev_blog(calls);
                self.call(calls, BlogCall::UpdateSelectedBlog);
                self.call(calls, BlogCall::UpdateButtonHelp);
            }
            (0..=2, Key::Next) if self.current >= 0 => {
                sounds.push("FE_SOUND_CORE_A_BUTTON");
                return Some((if self.state == 0 { self.blog_state } else { 1 }, self.current));
            }
            _ => {}
        }
        None
    }
}

/// The `articles` instance: `mCurrentIndex` over `mNumItems` photos, laid out by their count
/// (`article01` for four, `article02` for five, `article03` for three).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArticleScreen {
    pub current: i32,
    pub num_items: i32,
}

impl ArticleScreen {
    /// `SetState(0)` with `Articles_GetNumItems`: the first photo selected.
    #[must_use]
    pub const fn new(num_items: i32) -> Self {
        Self { current: 0, num_items }
    }

    /// `UpdateInput(keycode)`: right and left step a photo, down and up a row (two photos in
    /// the four-photo layout, else three), each sounding `FE_SOUND_BLOG_NAV` and selecting the
    /// photo (`SetSelectedArticle`); A sounds `FE_SOUND_CORE_A_BUTTON` and returns
    /// `Game.OnMenuSelect(0, mCurrentIndex)`'s index. Whether the selection was set.
    pub fn update_input(&mut self, key: Key, sounds: &mut Vec<&'static str>) -> (bool, Option<i32>) {
        let row = if self.num_items == 4 { 2 } else { 3 };
        match key {
            Key::Right | Key::Down => {
                self.current += if key == Key::Right { 1 } else { row };
                if self.current >= self.num_items {
                    self.current = self.num_items - 1;
                }
            }
            Key::Left | Key::Up => {
                self.current -= if key == Key::Left { 1 } else { row };
                if self.current < 0 {
                    self.current = 0;
                }
            }
            Key::Next => {
                sounds.push("FE_SOUND_CORE_A_BUTTON");
                return (false, Some(self.current));
            }
        }
        sounds.push("FE_SOUND_BLOG_NAV");
        (true, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slappys_state_moves_between_the_panel_and_the_window_of_rows() {
        let mut screen = BlogScreen::new();
        let (mut sounds, mut calls) = (Vec::new(), Vec::new());
        screen.set_state(0, 5, &mut calls);
        assert_eq!((screen.blog_state, screen.current, screen.max_window), (1, 0, 2));
        for _ in 0..4 {
            screen.update_input(Key::Down, &mut sounds, &mut calls);
        }
        assert_eq!((screen.current, screen.min_window, screen.max_window), (4, 2, 4));
        screen.update_input(Key::Left, &mut sounds, &mut calls);
        assert_eq!((screen.blog_state, screen.current), (0, 2));
        screen.update_input(Key::Right, &mut sounds, &mut calls);
        assert_eq!((screen.blog_state, screen.current), (1, 4));
        assert_eq!(screen.update_input(Key::Next, &mut sounds, &mut calls), Some((1, 4)));
    }

    #[test]
    fn article_rows_follow_the_layout() {
        let mut article = ArticleScreen::new(4);
        let mut sounds = Vec::new();
        article.update_input(Key::Down, &mut sounds);
        assert_eq!(article.current, 2);
        article.update_input(Key::Down, &mut sounds);
        assert_eq!(article.current, 3);
        let mut article = ArticleScreen::new(5);
        article.update_input(Key::Down, &mut sounds);
        assert_eq!(article.update_input(Key::Next, &mut sounds), (false, Some(3)));
    }
}
