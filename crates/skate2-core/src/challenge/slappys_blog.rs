//! What Slappy's blog (`FrontEndState_SlappysBlog`, FE state 61), its magazine articles
//! (`FrontEndState_SlappysArticles`, 62) and covers (`FrontEndState_SlappysCovers`, 63) read
//! through the `Game.Slappys_*`, `Articles_*` and `Covers_*` natives (`GameCmdTranslator`).

use super::{
    keys::class,
    photo_articles::{self, article},
    signpost,
    signpost_progress::{blog_type, classes::PHOTO_ARTICLES},
    system::World,
};
use crate::ui::slappys_blog::{ArticleScreen, BlogCall, BlogScreen};

/// `FrontEndState_SlappysBlog::eSlappysScreenMode`.
pub mod mode {
    pub const SLAPPYS: i32 = 0;
    pub const THRASHER: i32 = 1;
    pub const SBM: i32 = 2;
}

/// A `cBLOGData` of `mSlappysBlogList`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlogEntry {
    /// `eBLOGTypes`.
    pub kind: u32,
    /// `progressionData`: the challenge's signpost key, the article's or the message's key.
    pub key: u64,
}

/// A row of the blog list as its screen reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlogRow {
    /// `GetItemText` (`0x826a66d0`): the genderized label `FormatStringCustom` formats.
    pub text: String,
    /// `ShowPhoto` (`0x82693610`) and `GetPhotoSlot` (`0x82693630`): a photo article's first photo.
    pub photo_slot: Option<u32>,
    /// `ShowAccept` (`0x82693730`).
    pub accept: bool,
}

/// What the blog screen reads in a mode (`mMenuStack.back()`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlogView {
    pub mode: i32,
    /// `GetBannerType` (`0x82693530`).
    pub banner: &'static str,
    /// `GetTitleText` (`0x826935b8`).
    pub title: &'static str,
    pub rows: Vec<BlogRow>,
}

/// What the article screen reads (`mPhotoArticleKey`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArticleView {
    /// `GetBannerType` (`0x82693788`).
    pub banner: &'static str,
    /// `GetTitleText` (`0x82693898`): the article's `TitleHALID`.
    pub title: String,
    /// `GetPhotoSlot` (`0x826938e0`) of each photo (`GetNumItems` `0x826937f8`).
    pub photo_slots: Vec<u32>,
    /// `GetArticleText` (`0x826a1c80`): the genderized label, or `#`.
    pub text: String,
}

/// What the cover screen reads (`mPhotoArticleKey`, `mCurrentIndex`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverView {
    /// `UpdateCover`'s screen state: Thrasher's (0) or the Skateboard Mag's (1) cover with one
    /// photo, else their full screen (2, 3).
    pub state: i32,
    /// `GetPhotoSlot` (`0x82693b48`).
    pub photo_slot: u32,
    /// `GetButtonHelpTextTrigger` (`0x8268fd60`).
    pub trigger_text: &'static str,
    /// `Game.GetProfileName` (`0x82603d68`): `#` and the profile's name, a literal.
    pub profile_name: String,
}

/// The blog state on top of the front-end stack (only its screen is open) and what the screen
/// reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlappysHud {
    Blog { view: BlogView },
    Article { view: ArticleView, screen: ArticleScreen },
    Cover { view: CoverView },
}

/// A call of the top screen's class, in order, which the screen replays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlappysCall {
    /// A `slappys_blog` call with the instance as it was then.
    Blog(BlogCall, BlogScreen),
    /// `articles::SetState(0)` (`OnLoaded`'s `SetScreenState(0)`).
    ArticleState,
    /// `articles::SetSelectedArticle(index)`.
    Article(i32),
    /// `covers::SetState(state)` (`UpdateCover`'s `SetScreenState`).
    Cover(i32),
}

/// `cSignPostManager::GetBLOGDataInOrder(order)` (`0x825c8108`): the last entry in map order
/// with that order.
fn blog_in_order(world: &World, order: i32) -> Option<BlogEntry> {
    world.signpost.progress.blog.iter().filter(|(_, blog)| blog.order == order).map(|(&(kind, key), _)| BlogEntry { kind, key }).last()
}

/// The article of a photo-article entry is Thrasher's (`MAGAZINE` 0) or the Skateboard Mag's (1).
fn magazine(world: &World, photo_article: u64) -> u32 {
    world.db.layout_word(PHOTO_ARTICLES, photo_article, article::MAGAZINE)
}

/// `UpdateBlogList` (`0x826ac3e8`): the entries by order, newest first; Thrasher's and the
/// Skateboard Mag's modes keep their magazine's photo articles.
#[must_use]
pub fn blog_list(world: &World, mode: i32) -> Vec<BlogEntry> {
    let count = i32::try_from(world.signpost.progress.blog.len()).unwrap_or(i32::MAX);
    let mut list = Vec::new();
    for order in 1..=count {
        let Some(entry) = blog_in_order(world, order) else { continue };
        let keep = match mode {
            mode::SLAPPYS => true,
            mode::THRASHER => entry.kind == blog_type::PHOTO_ARTICLE && magazine(world, entry.key) == 0,
            mode::SBM => entry.kind == blog_type::PHOTO_ARTICLE && magazine(world, entry.key) == 1,
            _ => false,
        };
        if keep {
            list.insert(0, entry);
        }
    }
    list
}

/// `GetGenderizedHALID` (`0x826a1910`): a label ending in `_M` ends in `_F` for a female player;
/// one shorter than two characters is `Missing`.
#[must_use]
pub fn genderized(label: &str, male: bool) -> String {
    if label.len() < 2 {
        return "Missing".to_owned();
    }
    match label.strip_suffix("_M") {
        Some(stem) if !male => format!("{stem}_F"),
        _ => label.to_owned(),
    }
}

/// A blog row: the entry's text (an activated challenge's word 6, a completed one's word 5, a
/// photo article's BLOG text, a message's `HALText`), genderized.
fn row(world: &World, entry: BlogEntry, male: bool) -> BlogRow {
    let db = &world.db;
    let label = match entry.kind {
        blog_type::CHALLENGE_ACTIVATE => db.layout_text(class::PROGRESSION_CHALLENGES, entry.key, 6).map(str::to_owned),
        blog_type::CHALLENGE_COMPLETE => db.layout_text(class::PROGRESSION_CHALLENGES, entry.key, 5).map(str::to_owned),
        blog_type::PHOTO_ARTICLE => db.layout_text(PHOTO_ARTICLES, entry.key, article::BLOG_TEXT).map(str::to_owned),
        _ => signpost::message_string_property(db, entry.key, "HALText"),
    };
    let photo = entry.kind == blog_type::PHOTO_ARTICLE;
    BlogRow {
        text: genderized(label.as_deref().unwrap_or_default(), male),
        photo_slot: photo.then(|| photo_articles::photo_slot(world, photo_articles::article_photo_challenge(world, entry.key, 1))),
        accept: photo,
    }
}

/// The blog screen's reads in `mode`.
#[must_use]
pub fn blog_view(world: &World, mode: i32, list: &[BlogEntry], male: bool) -> BlogView {
    let (banner, title) = match mode {
        mode::SLAPPYS => ("slappys", "ID_SLAPPYS_BLOG_TITLE"),
        mode::THRASHER => ("thrasher", "#"),
        mode::SBM => ("sbm", "#"),
        _ => ("none", "#"),
    };
    let rows = if (mode::SLAPPYS..=mode::SBM).contains(&mode) { list.iter().map(|&entry| row(world, entry, male)).collect() } else { Vec::new() };
    BlogView { mode, banner, title, rows }
}

/// The article screen's reads for an article.
#[must_use]
pub fn article_view(world: &World, photo_article: u64, male: bool) -> ArticleView {
    let db = &world.db;
    let count = photo_articles::article_photo_count(world, photo_article);
    let photo_slots = (1..=count).map(|order| photo_articles::photo_slot(world, photo_articles::article_photo_challenge(world, photo_article, order))).collect();
    let text = db.layout_text(PHOTO_ARTICLES, photo_article, article::TEXT).map_or_else(|| "#".to_owned(), |text| genderized(text, male));
    ArticleView {
        banner: if magazine(world, photo_article) == 1 { "sbm" } else { "thrasher" },
        title: db.layout_text(PHOTO_ARTICLES, photo_article, article::TITLE).unwrap_or_default().to_owned(),
        photo_slots,
        text,
    }
}

/// The cover screen's reads for an article's photo (`SetArticleKeyAndPhotoIndex` `0x826939a8`
/// and `UpdateCover` `0x8269d428`).
#[must_use]
pub fn cover_view(world: &World, photo_article: u64, index: i32) -> CoverView {
    let photo = photo_articles::article_photo_challenge(world, photo_article, index);
    let photos = photo_articles::article_photo_count(world, photo_article);
    let state = match (magazine(world, photo_article) == 1, photos == 1) {
        (true, true) => 1,
        (true, false) => 3,
        (false, true) => 0,
        (false, false) => 2,
    };
    CoverView {
        state,
        photo_slot: photo_articles::photo_slot(world, photo),
        trigger_text: if photos == 1 { "#" } else { "ID_COMMON_CYCLE" },
        profile_name: String::new(),
    }
}

/// `cSignPostManager::ResetNewBLOGCount` (`0x825c7d70`): every entry is old.
pub fn reset_new_blog_count(world: &mut World) {
    for blog in world.signpost.progress.blog.values_mut() {
        blog.new = false;
    }
}
