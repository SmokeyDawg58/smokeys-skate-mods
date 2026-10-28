//! Slappy's blog's screens: `FrontEndState_SlappysBlog`'s screen 125 (screens/slappys/slappys_blog,
//! asset 77, fingerprint 632b87a9), `FrontEndState_SlappysArticles`' 126 (articles, asset 78,
//! fingerprint d6632366) and `FrontEndState_SlappysCovers`' 127 (covers, asset 79, fingerprint
//! ad9d0792), each a template at placement 0. Each InitAction (characters 125, 59 and 44) defines
//! the class translated here and root frame 0 constructs it. The classes' selection is the
//! core's (`skate2_core::ui::slappys_blog`), whose calls each screen replays with the instance as
//! it was. `LoadMemcardTexture` requests the saved mission photo by slot, with
//! thumbnails for blog/article rows and the full image for covers.
use super::{
    assets::Assets,
    clips::{Child, Clip},
    hud_leaderboard_movie::{find, find_child},
    localizer::format_custom,
    map_movie::update_scrollbar,
    pause_movie::{bounds, configure_right_buttons},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::{
    challenge::slappys_blog::{ArticleView, BlogView, CoverView, SlappysCall, SlappysHud},
    ui::{
        apt_timeline::{DisplayEntry, IDENTITY, PlacementIdentity},
        slappys_blog::{BlogCall, BlogScreen, MAX_NUM_PANEL_OPTIONS, MAX_VISIBLE_BLOGS},
    },
};

pub(super) const BLOG: usize = 77;
pub(super) const ARTICLES: usize = 78;
pub(super) const COVERS: usize = 79;
/// `slappys_blog.mScrollbarLength`.
const SCROLLBAR_LENGTH: f32 = 435.0;
/// `mScroller`'s initialize event: `mSkin = "Yellow_Glow"`.
const SCROLLER_SKIN: &str = "Yellow_Glow";
/// The depth `attachMovie(name, depth)` places a clip at, above the authored ones.
const SCRIPT_DEPTH: i32 = 0x4000;
/// `_alpha = 40`.
const DIM_ALPHA: u32 = 102;

/// An open or closing screen.
struct Screen {
    asset: usize,
    root: Clip,
    /// `CloseScreen` has played its outro.
    closing: bool,
    /// `articles.mNumItems` and its layout's `mArticle` (`article01` to `article03`).
    article_items: i32,
    article: Option<&'static str>,
}

#[derive(Default)]
pub(super) struct SlappysMovie {
    /// The top blog state's screen last, after the screens still playing their outro.
    screens: Vec<Screen>,
    intro_complete: bool,
    /// The timelines' `Audio.PlaySound` requests (none of these movies has one).
    pub sounds: Vec<&'static str>,
}

impl SlappysMovie {
    /// The top blog state is open (its screen hides the HUD and asks for the dimmer).
    pub fn open(&self) -> bool {
        self.screens.iter().any(|screen| !screen.closing)
    }

    /// The screens to draw, the closing ones first.
    pub fn roots(&self) -> impl Iterator<Item = &Clip> {
        self.screens.iter().map(|screen| &screen.root)
    }

    /// The top state's screen: a push or a pop closes the old one (`CloseScreen`, its outro) and
    /// opens the new one (`OpenScreen`: the class constructed, then the screen's intro). Then the
    /// state's calls, in order.
    pub fn update(&mut self, assets: &Assets, hud: Option<&SlappysHud>, calls: &[SlappysCall]) -> RenderResult<()> {
        let asset = hud.map(|hud| match hud {
            SlappysHud::Blog { .. } => BLOG,
            SlappysHud::Article { .. } => ARTICLES,
            SlappysHud::Cover { .. } => COVERS,
        });
        let open = self.screens.iter().position(|screen| !screen.closing);
        if open.map(|index| self.screens[index].asset) != asset {
            if let Some(index) = open {
                let screen = &mut self.screens[index];
                screen.closing = true;
                screen.root.goto(assets, "outro")?;
            }
            if let Some(hud) = hud {
                self.screens.push(open_screen(assets, hud)?);
            }
        }
        if let (Some(hud), Some(screen)) = (hud, self.screens.last_mut().filter(|screen| !screen.closing)) {
            for &call in calls {
                apply(screen, assets, hud, call)?;
            }
        }
        self.calls()
    }

    pub(super) fn photo_requests(&self) -> std::collections::BTreeSet<(u32, bool)> {
        fn visit(clip: &Clip, requests: &mut std::collections::BTreeSet<(u32, bool)>) {
            if !clip.properties.contains_key("photo_resolved")
                && let Some(request) = photo_request(clip)
            { requests.insert(request); }
            for child in clip.children.values() { visit(&child.clip, requests); }
        }
        let mut requests = std::collections::BTreeSet::new();
        for screen in &self.screens { visit(&screen.root, &mut requests); }
        requests
    }

    pub(super) fn set_photo(&mut self, slot: u32, thumbnail: bool, texture: Option<&str>) {
        fn visit(clip: &mut Clip, slot: u32, thumbnail: bool, texture: Option<&str>) {
            if photo_request(clip) == Some((slot, thumbnail)) {
                clip.properties.insert("photo_resolved".into(), "true".into());
                if let Some(texture) = texture { clip.properties.insert("cac_texture".into(), texture.into()); }
                else { clip.properties.remove("cac_texture"); }
            }
            for child in clip.children.values_mut() { visit(&mut child.clip, slot, thumbnail, texture); }
        }
        for screen in &mut self.screens { visit(&mut screen.root, slot, thumbnail, texture); }
    }

    /// One front-end update.
    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        for screen in &mut self.screens {
            screen.root.tick(assets)?;
        }
        self.calls()
    }

    /// The open screen's `ScreenManager.IntroComplete` since the last call.
    pub fn take_intro_complete(&mut self) -> bool {
        std::mem::take(&mut self.intro_complete)
    }

    /// The frame scripts' `IntroComplete` and `OutroComplete`, which unloads its screen.
    fn calls(&mut self) -> RenderResult<()> {
        let mut index = 0;
        while let Some(screen) = self.screens.get_mut(index) {
            let mut calls = Vec::new();
            screen.root.take_calls(&mut calls);
            let mut unload = false;
            for call in calls {
                match call {
                    "slappys_blog" | "articles" | "covers" => {}
                    "IntroComplete" => self.intro_complete |= !screen.closing,
                    "OutroComplete" => unload = true,
                    _ => return Err(format!("untranslated Slappy's blog callback {call}").into()),
                }
            }
            if unload {
                self.screens.remove(index);
            } else {
                index += 1;
            }
        }
        Ok(())
    }
}

/// `OpenScreen`: the movie, root frame 0's class constructed, then the screen's `intro`.
fn open_screen(assets: &Assets, hud: &SlappysHud) -> RenderResult<Screen> {
    let (asset, class) = match hud {
        SlappysHud::Blog { .. } => (BLOG, "slappys_blog"),
        SlappysHud::Article { .. } => (ARTICLES, "articles"),
        SlappysHud::Cover { .. } => (COVERS, "covers"),
    };
    let mut root = Clip::new(assets, asset, 0)?;
    let mut calls = Vec::new();
    root.take_calls(&mut calls);
    if !calls.contains(&class) {
        return Err(format!("{class} root constructed no screen").into());
    }
    // The constructors' mScreen.stop().
    root.playback.stop();
    match hud {
        SlappysHud::Blog { .. } => construct_blog(&mut root, assets)?,
        SlappysHud::Article { .. } => {
            if let Some(menu) = find(&mut root, &["mButtonMenu"]) {
                configure_right_buttons(menu, assets, &[("B", "ID_COMMON_BACK"), ("A", "ID_COMMON_SELECT")])?;
            }
        }
        SlappysHud::Cover { view } => {
            if let Some(help) = find(&mut root, &["mPanel", "mButtonHelp"]) {
                if view.trigger_text == "#" {
                    configure_right_buttons(help, assets, &[("B", "ID_COMMON_BACK")])?;
                } else {
                    configure_right_buttons(help, assets, &[("B", "ID_COMMON_BACK"), ("R_Trigger", view.trigger_text), ("L_Trigger", "#")])?;
                }
            }
        }
    }
    root.goto(assets, "intro")?;
    Ok(Screen { asset, root, closing: false, article_items: 0, article: None })
}

/// A text field's `text = label`, resolved as `SkateAptString` does.
fn set_label(root: &mut Clip, assets: &Assets, path: &[&str], label: &str) {
    if let Some(field) = find(root, path) {
        field.text = Some(assets.initial_text(label));
    }
}

/// A `#` result is the literal after it; any other label goes through `FormatStringCustom`.
fn formatted(assets: &Assets, label: &str) -> RenderResult<Vec<u16>> {
    match label.strip_prefix('#') {
        Some(literal) => Ok(literal.encode_utf16().collect()),
        None => format_custom(assets, label, ""),
    }
}

/// `gotoAndPlay(label)` where the clip exists.
fn goto(root: &mut Clip, assets: &Assets, path: &[&str], label: &str) -> RenderResult<()> {
    if let Some(clip) = find(root, path) {
        clip.goto(assets, label)?;
    }
    Ok(())
}

/// `_alpha = value` of a member, which then keeps its timeline's moves from the colour.
fn set_alpha(parent: &mut Clip, name: &str, alpha: u32) {
    if let Some(child) = find_child(parent, name) {
        child.entry.colour_scale = (child.entry.colour_scale & 0x00ff_ffff) | (alpha << 24);
        child.clip.script_transform = true;
    }
}

/// `new slappys_blog(this)`: the dummy thumbnails hidden, the scroller's length and skin
/// (`basic_slider`'s constructor `SetSkin(mSkin)`), the button texts.
fn construct_blog(root: &mut Clip, assets: &Assets) -> RenderResult<()> {
    let Some(panel) = find(root, &["mPanel"]) else { return Ok(()) };
    for i in 0..MAX_VISIBLE_BLOGS {
        if let Some(dummy) = find(panel, &[&format!("mDummyThumb{i}")]) {
            dummy.visible = false;
        }
    }
    if let Some(scroller) = find(panel, &["mScroller"]) {
        let skin = scroller.properties.get("mSkin").cloned().unwrap_or_else(|| SCROLLER_SKIN.to_owned());
        for path in [
            &["mcScrollbar", "mArrowTop"][..],
            &["mcScrollbar", "mArrowBottom"],
            &["mcScrollbar", "mcEdge"],
            &["mcScrollbar", "mGak"],
            &["mcScrollbar", "mSliderIndicator", "mIndSolid", "mBarTop"],
            &["mcScrollbar", "mSliderIndicator", "mIndSolid", "mBarMiddle"],
            &["mcScrollbar", "mSliderIndicator", "mIndSolid", "mBarBottom"],
            &["mcScrollbar", "mSliderIndicator", "mIndGlow", "mGlowTop"],
            &["mcScrollbar", "mSliderIndicator", "mIndGlow", "mGlowMiddle"],
            &["mcScrollbar", "mSliderIndicator", "mIndGlow", "mGlowBottom"],
        ] {
            if let Some(part) = find(scroller, path) {
                part.goto_and_stop(assets, &skin)?;
            }
        }
    }
    set_label(panel, assets, &["mButtonAcceptText", "labelText"], "ID_COMMON_SELECT");
    set_label(panel, assets, &["mButtonBackText", "labelText"], "ID_COMMON_BACK");
    Ok(())
}

/// One class call on the open screen.
fn apply(screen: &mut Screen, assets: &Assets, hud: &SlappysHud, call: SlappysCall) -> RenderResult<()> {
    match (hud, call) {
        (SlappysHud::Blog { view }, SlappysCall::Blog(call, blog)) => blog_call(&mut screen.root, assets, view, call, &blog),
        (SlappysHud::Article { view, .. }, SlappysCall::ArticleState) => article_state(screen, assets, view),
        (SlappysHud::Article { .. }, SlappysCall::Article(index)) => {
            select_article(screen, assets, index)
        }
        (SlappysHud::Cover { view }, SlappysCall::Cover(state)) => cover_state(&mut screen.root, assets, view, state),
        _ => Ok(()),
    }
}

/// The `slappys_blog` calls, with the instance as it was then.
fn blog_call(root: &mut Clip, assets: &Assets, view: &BlogView, call: BlogCall, blog: &BlogScreen) -> RenderResult<()> {
    let Some(panel) = find(root, &["mPanel"]) else { return Ok(()) };
    match call {
        BlogCall::SetState { previous, previous_items } => {
            if previous != blog.state {
                destroy_blog_thumbs(panel, previous_items);
            }
            let label = if blog.state == 0 { "slappys" } else { "magazine" };
            panel.goto(assets, label)?;
            update_banner(panel, assets, view)?;
            update_blog_scrollbar(panel, assets, blog)?;
            update_blog(panel, assets, view, blog)?;
            update_selected_blog(panel, assets, view, blog)?;
            if blog.state == 0 {
                update_selected_blog_panel(panel, assets, blog)?;
            }
            update_button_help(panel, view, blog);
        }
        BlogCall::UpdateBlog => update_blog(panel, assets, view, blog)?,
        BlogCall::UpdateSelectedBlog => update_selected_blog(panel, assets, view, blog)?,
        BlogCall::UpdateSelectedBlogPanel => update_selected_blog_panel(panel, assets, blog)?,
        BlogCall::UpdateButtonHelp => update_button_help(panel, view, blog),
        BlogCall::ScrollerValue => update_blog_scrollbar(panel, assets, blog)?,
    }
    Ok(())
}

/// The row clips: `mBlog0` to `mBlog2` in Slappy's own state, `mBlogLong0` to `mBlogLong2` in a
/// magazine's (`mBlogStr`).
fn row_name(blog: &BlogScreen, slot: i32) -> String {
    if blog.state == 0 { format!("mBlog{slot}") } else { format!("mBlogLong{slot}") }
}

/// `DestroyBlogThumbs`: every attached thumbnail hidden, destroyed and removed.
fn destroy_blog_thumbs(panel: &mut Clip, items: i32) {
    for index in 0..items {
        let name = format!("mThumbnail{index}");
        panel.children.retain(|_, child| !child.entry.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(&name)));
    }
}

/// `UpdateBanner`: the header's label for the banner, then its title.
fn update_banner(panel: &mut Clip, assets: &Assets, view: &BlogView) -> RenderResult<()> {
    goto(panel, assets, &["mHeader"], view.banner)?;
    if view.banner != "none" {
        set_label(panel, assets, &["mHeader", view.banner, "mTextTitle", "labelText"], view.title);
    }
    Ok(())
}

/// `UpdateBlogScrollbar` and `mScroller.SetValue`: `SetNumberOfItems(mNumItems, 3)` and the
/// indicator at the scroller's window.
fn update_blog_scrollbar(panel: &mut Clip, assets: &Assets, blog: &BlogScreen) -> RenderResult<()> {
    let Some(scroller) = find(panel, &["mScroller"]) else { return Ok(()) };
    let count = usize::try_from(blog.scroller.items).unwrap_or(0);
    let rows = usize::try_from(blog.scroller.rows).unwrap_or(0);
    let start = usize::try_from(blog.scroller.windowed).unwrap_or(0);
    update_scrollbar(scroller, assets, count, rows, start, SCROLLBAR_LENGTH)
}

/// `_width` and `_height` of a member.
fn size(child: &Child, assets: &Assets) -> RenderResult<(f32, f32)> {
    let r = bounds(&child.clip, assets)?.ok_or("blog thumbnail has no bounds")?;
    Ok(((r.right - r.left) * child.entry.matrix.a.abs(), (r.bottom - r.top) * child.entry.matrix.d.abs()))
}

/// `UpdateBlog`: the thumbnails outside the window removed; each window row shows its item's
/// text (`short` beside a photo article's thumbnail, attached at the item's depth over its
/// slot's dummy, else `long`) and goes `off`, and hides past the last item.
fn update_blog(panel: &mut Clip, assets: &Assets, view: &BlogView, blog: &BlogScreen) -> RenderResult<()> {
    for index in 0..blog.num_items {
        if index < blog.min_window || index >= blog.min_window + MAX_VISIBLE_BLOGS {
            let name = format!("mThumbnail{index}");
            panel.children.retain(|_, child| !child.entry.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(&name)));
        }
    }
    for slot in 0..MAX_VISIBLE_BLOGS {
        let index = blog.min_window + slot;
        let row_name = row_name(blog, slot);
        let item = usize::try_from(index).ok().filter(|_| index < blog.num_items).and_then(|index| view.rows.get(index));
        let Some(item) = item else {
            if let Some(row) = find(panel, &[&row_name]) {
                row.visible = false;
            }
            continue;
        };
        let text = format_custom(assets, &item.text, "")?;
        if let Some(row) = find(panel, &[&row_name]) {
            row.visible = true;
            goto(row, assets, &["mBlogText"], if item.photo_slot.is_some() { "short" } else { "long" })?;
            if let Some(field) = find(row, &["mBlogText", "labelText"]) {
                field.text = Some(text);
            }
        }
        if let Some(photo) = item.photo_slot {
            attach_thumbnail(panel, assets, index, slot)?;
            if let Some(thumb) = find(panel, &[&format!("mThumbnail{index}")]) {
                request_photo(thumb, photo, true);
            }
        }
        goto(panel, assets, &[&row_name], "off")?;
    }
    Ok(())
}

/// `attachMovie("Image_Renderer_Component", "mThumbnail" + index, index)` once, then its size,
/// `LoadMemcardTexture(slot, 4)`, shown, at its slot's dummy.
fn attach_thumbnail(panel: &mut Clip, assets: &Assets, index: i32, slot: i32) -> RenderResult<()> {
    let name = format!("mThumbnail{index}");
    let dummy_name = format!("mDummyThumb{slot}");
    let Some(dummy) = find_child(panel, &dummy_name) else { return Ok(()) };
    let (dummy_width, dummy_height) = size(dummy, assets)?;
    let (x, y) = (dummy.entry.matrix.tx, dummy.entry.matrix.ty);
    if find_child(panel, &name).is_none() {
        let character = assets.movies[BLOG]
            .movie
            .imports
            .iter()
            .find(|import| import.name == "Image_Renderer_Component")
            .map(|import| import.character_id)
            .ok_or("slappys_blog imports no Image_Renderer_Component")?;
        let clip = Clip::new(assets, BLOG, character)?;
        let entry = DisplayEntry {
            identity: PlacementIdentity { character: clip.character, frame: 0 },
            matrix: IDENTITY,
            colour_scale: u32::MAX,
            colour_bias: 0,
            ratio: 0.,
            name: Some(name.clone()),
            clip_depth: -1,
            events: Vec::new(),
            blend_mode: None,
        };
        panel.children.insert(SCRIPT_DEPTH + index, Child { entry, clip });
        let thumb = find_child(panel, &name).ok_or("attached thumbnail absent")?;
        let r = bounds(&thumb.clip, assets)?.ok_or("thumbnail has no bounds")?;
        thumb.entry.matrix.d = dummy_height / (r.bottom - r.top);
        thumb.entry.matrix.a = dummy_width / (r.right - r.left);
        thumb.clip.script_transform = true;
        thumb.clip.properties.remove("cac_texture");
    }
    let thumb = find_child(panel, &name).ok_or("attached thumbnail absent")?;
    thumb.clip.visible = true;
    thumb.entry.matrix.tx = x;
    thumb.entry.matrix.ty = y;
    thumb.clip.script_transform = true;
    Ok(())
}

/// `UpdateSelectedBlog`: in Slappy's own panel every row `off` and every thumbnail at 40, else
/// the current item's row `on` and its thumbnail at 100, the others `off` and at 40; then
/// `UpdateButtonHelp`.
fn update_selected_blog(panel: &mut Clip, assets: &Assets, view: &BlogView, blog: &BlogScreen) -> RenderResult<()> {
    for slot in 0..MAX_VISIBLE_BLOGS {
        let index = blog.min_window + slot;
        let selected = !(blog.state == 0 && blog.blog_state == 0) && index == blog.current;
        goto(panel, assets, &[&row_name(blog, slot)], if selected { "on" } else { "off" })?;
        set_alpha(panel, &format!("mThumbnail{index}"), if selected { 0xff } else { DIM_ALPHA });
    }
    update_button_help(panel, view, blog);
    Ok(())
}

/// `UpdateSelectedBlogPanel`: in the panel the current option `on` and the others `off`, in the
/// list every option `off`.
fn update_selected_blog_panel(panel: &mut Clip, assets: &Assets, blog: &BlogScreen) -> RenderResult<()> {
    for option in 0..MAX_NUM_PANEL_OPTIONS {
        let on = blog.blog_state == 0 && option == blog.current;
        goto(panel, assets, &[&format!("mButton{option}")], if on { "on" } else { "off" })?;
    }
    Ok(())
}

/// `UpdateButtonHelp`: A shown in the panel, and in the list over an item `Slappys_ShowAccept`
/// allows.
fn update_button_help(panel: &mut Clip, view: &BlogView, blog: &BlogScreen) {
    let accept = match blog.blog_state {
        0 => true,
        _ => blog.num_items > 0 && usize::try_from(blog.current).ok().and_then(|index| view.rows.get(index)).is_some_and(|row| row.accept),
    };
    if !matches!(blog.blog_state, 0 | 1) {
        return;
    }
    for name in ["mButtonAccept", "mButtonAcceptText"] {
        if let Some(clip) = find(panel, &[name]) {
            clip.visible = accept;
        }
    }
}

/// `articles::SetState(0)`: the old photos destroyed, the layout for the photo count
/// (`article01` four, `article02` five, `article03` three), `UpdateBanner`, `UpdateArticle` (the
/// photos' `LoadMemcardTexture(slot, 4)` and the article's text) and `SetSelectedArticle(0)`.
fn article_state(screen: &mut Screen, assets: &Assets, view: &ArticleView) -> RenderResult<()> {
    let items = i32::try_from(view.photo_slots.len()).unwrap_or(i32::MAX);
    screen.article_items = items;
    screen.article = match items {
        4 => Some("article01"),
        5 => Some("article02"),
        3 => Some("article03"),
        _ => screen.article,
    };
    if let Some(label) = screen.article.filter(|_| matches!(items, 3..=5)) {
        goto(&mut screen.root, assets, &["mArticles"], label)?;
    }
    goto(&mut screen.root, assets, &["mHeader"], view.banner)?;
    set_label(&mut screen.root, assets, &["mHeader", view.banner, "mTextTitle", "labelText"], &view.title);
    let text = formatted(assets, &view.text)?;
    if let Some(field) = find(&mut screen.root, &["mArticles", "mArticleText", "labelText"]) {
        field.text = Some(text);
    }
    if let Some(article) = screen.article {
        let member = format!("mArticle{}", &article[article.len() - 2..]);
        for (index, &slot) in view.photo_slots.iter().enumerate() {
            if let Some(image) = find(&mut screen.root, &["mArticles", &member, &format!("mPhoto{index}"), "mImage"]) {
                request_photo(image, slot, true);
            }
        }
    }
    select_article(screen, assets, 0)
}

/// `SetSelectedArticle(index)`: the photo `on`, the others `off`.
fn select_article(screen: &mut Screen, assets: &Assets, index: i32) -> RenderResult<()> {
    let Some(article) = screen.article else { return Ok(()) };
    let member = format!("mArticle{}", &article[article.len() - 2..]);
    for photo in 0..screen.article_items {
        let label = if photo == index { "on" } else { "off" };
        goto(&mut screen.root, assets, &["mArticles", &member, &format!("mPhoto{photo}")], label)?;
    }
    Ok(())
}

/// `covers::SetState(state)`: a single photo's cover (`thrasher_cover`, `sbm_cover`) with
/// `UpdateCover`, or the full screen with the magazine's logos and `UpdateFullscreen` (the
/// profile name, an empty caption, both auto-sized); each loads the photo
/// (`LoadMemcardTexture(Covers_GetPhotoSlot(), 5)`).
fn cover_state(root: &mut Clip, assets: &Assets, view: &CoverView, state: i32) -> RenderResult<()> {
    match state {
        0 => goto(root, assets, &["mPanel"], "thrasher_cover")?,
        1 => goto(root, assets, &["mPanel"], "sbm_cover")?,
        2 | 3 => {
            goto(root, assets, &["mPanel"], "fullscreen")?;
            goto(root, assets, &["mPanel", "mLogos"], if state == 2 { "thrasher" } else { "sbm" })?;
            if let Some(field) = find(root, &["mPanel", "mPlayername", "labelText"]) {
                field.text = Some(assets.initial_text(&view.profile_name));
                field.properties.insert("text_autosize".into(), "left".into());
            }
            if let Some(field) = find(root, &["mPanel", "mCaption", "labelText"]) {
                field.text = Some(Vec::new());
                field.properties.insert("text_autosize".into(), "left".into());
            }
        }
        _ => {}
    }
    if let Some(image) = find(root, &["mPanel", "mImage"]) {
        request_photo(image, view.photo_slot, false);
    }
    Ok(())
}

fn request_photo(clip: &mut Clip, slot: u32, thumbnail: bool) {
    clip.properties.insert("photo_slot".into(), slot.to_string());
    clip.properties.insert("photo_thumbnail".into(), thumbnail.to_string());
    clip.properties.remove("photo_resolved");
}

fn photo_request(clip: &Clip) -> Option<(u32, bool)> {
    Some((clip.properties.get("photo_slot")?.parse().ok()?,
        clip.properties.get("photo_thumbnail")? == "true"))
}

/// The movies' InitActions define the guarded global classes translated above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (BLOG, 125, 0x6d9c) | (ARTICLES, 59, 0x1ebc) | (COVERS, 44, 0x135c))
}

/// Timeline actions besides `Stop`: root frame 0 constructs the class; the intro's first frame
/// asks `ScreenManager.EnableDimmer(true)`, which screenmanager.apt never defines; the intro's
/// last frame reports `IntroComplete` and the outro's `OutroComplete` (and `EnableDimmer(false)`).
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match (clip.asset, offset) {
        (BLOG, 0x6d80) => clip.calls.push("slappys_blog"),
        (ARTICLES, 0x1e88) => clip.calls.push("articles"),
        (COVERS, 0x1340) => clip.calls.push("covers"),
        (BLOG, 0x7b88) | (ARTICLES, 0x24fc) => {}
        (BLOG, 0x7ba4) | (ARTICLES, 0x2518) | (COVERS, 0x16c4) => {
            clip.playback.stop();
            clip.calls.push("IntroComplete");
        }
        (BLOG, 0x7bd0) | (ARTICLES, 0x2544) | (COVERS, 0x16f0) => {
            clip.playback.stop();
            clip.calls.push("OutroComplete");
        }
        _ => return Err(format!("untranslated Slappy's blog APT action {}:{} {offset:#x}", clip.asset, clip.character).into()),
    }
    Ok(())
}
