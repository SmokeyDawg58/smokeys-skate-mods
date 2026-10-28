//! The magazine articles of `cSignPostManager` and `cSignPostUtil`: the `progression_photoarticles`
//! a photo challenge belongs to (through its `progression_photoarticlechallenges`), the BLOG entry
//! and the coverage an article's last photo adds (`UpdatePhotoArticlesBLOGS` `0x825d91a0`), the
//! captions it picks, and what Slappy's blog, its article and its cover screens read of an
//! article.

use super::{
    signpost_progress::{self, challenge_id, challenge_key_from_id, classes::PHOTO_ARTICLES},
    system::World,
};

/// `progression_photoarticlechallenges`: the photo slot (word 0), the article id (word 1) and
/// the photo challenge's `ChallengeID` (word 2).
pub const PHOTO_ARTICLE_CHALLENGES: u64 = 0xccb3_ed0f_6c18_d140;
/// `progression_photoarticles`' photo challenges (`RefSpec[]` of `PHOTO_ARTICLE_CHALLENGES`).
const ARTICLE_PHOTOS: u64 = 0xec1c_213b_bdbf_1a68;
/// `progression_photoarticlechallenges`' captions (`char*[]`).
const CAPTIONS: u64 = 0x4cdd_3422_751f_7431;
/// `progression_photoarticlechallenges`' `Challenge` (`RefSpec` of `progression_challenges`).
const PHOTO_CHALLENGE: u64 = 0x6a65_511c_0299_be61;

/// `progression_photoarticles` layout words.
pub mod article {
    /// `TitleHALID`.
    pub const TITLE: u32 = 0;
    /// The BLOG entry's text.
    pub const BLOG_TEXT: u32 = 1;
    /// The article's text.
    pub const TEXT: u32 = 3;
    pub const ID: u32 = 4;
    /// The photos the article needs; 0 for all of its list.
    pub const PHOTO_COUNT: u32 = 5;
    /// 0 for Thrasher, 1 for The Skateboard Mag.
    pub const MAGAZINE: u32 = 6;
}

/// A photo challenge's photo slot (`progression_photoarticlechallenges` word 0).
#[must_use]
pub fn photo_slot(world: &World, photo: u64) -> u32 {
    world.db.layout_word(PHOTO_ARTICLE_CHALLENGES, photo, 0)
}

/// `GetUniquePhotoChallengeID` 0x825d0328 and
/// `GetPhotoArticleChallengeForChallenge` 0x825ca080: first matching collection.
pub fn unique_photo_slot(world: &World, challenge: u64) -> u32 {
    let Some(signpost) = super::signpost::signpost_key_for_challenge(world, challenge) else { return 0 };
    photos_of_challenge(world, signpost).first().map_or(0, |&photo| photo_slot(world, photo))
}

/// The article's photo challenges, in its list's order.
fn article_photos(world: &World, photo_article: u64) -> Vec<u64> {
    let Some(list) = world.db.attribute(PHOTO_ARTICLES, photo_article, ARTICLE_PHOTOS) else { return Vec::new() };
    (0..list.len()).map(|i| list.ref_spec(i).map_or(0, |(_, key)| key)).collect()
}

/// The signpost key of a photo challenge's challenge (its `Challenge` reference).
fn photo_challenge(world: &World, photo: u64) -> u64 {
    world.db.attribute(PHOTO_ARTICLE_CHALLENGES, photo, PHOTO_CHALLENGE).and_then(|a| a.ref_spec(0)).map_or(0, |(_, key)| key)
}

/// `cSignPostUtil::GetPhotoArticleKeyFromID` (`0x825c9980`): the first article (collection walk)
/// with the id, or 0.
fn photo_article_key_from_id(world: &World, id: u32) -> u64 {
    world.db.table_order(PHOTO_ARTICLES).into_iter().find(|&key| world.db.layout_word(PHOTO_ARTICLES, key, article::ID) == id).unwrap_or(0)
}

/// The photo challenges of a challenge's `ChallengeID`, in collection walk order.
fn photos_of_challenge(world: &World, signpost: u64) -> Vec<u64> {
    let id = challenge_id(&world.db, signpost);
    world.db.table_order(PHOTO_ARTICLE_CHALLENGES).into_iter().filter(|&photo| world.db.layout_word(PHOTO_ARTICLE_CHALLENGES, photo, 2) == id).collect()
}

/// `cSignPostManager::GetArticlePhotoCount` (`0x825c5a30`): the article's photo count, or the
/// length of its list without one.
#[must_use]
pub fn article_photo_count(world: &World, photo_article: u64) -> i32 {
    let count = world.db.layout_word(PHOTO_ARTICLES, photo_article, article::PHOTO_COUNT);
    let count = if count == 0 {
        world.db.attribute(PHOTO_ARTICLES, photo_article, ARTICLE_PHOTOS).map_or(0, |a| a.len())
    } else {
        usize::try_from(count).unwrap_or(usize::MAX)
    };
    i32::try_from(count).unwrap_or(i32::MAX)
}

/// `cSignPostManager::AllPhotosComplete` (`0x825d01c0`): the article's photo challenges
/// (`progression_photoarticlechallenges` with its id) are all complete, or as many as its photo
/// count asks.
fn all_photos_complete(world: &World, photo_article: u64) -> bool {
    let db = &world.db;
    let id = db.layout_word(PHOTO_ARTICLES, photo_article, article::ID);
    let needed = db.layout_word(PHOTO_ARTICLES, photo_article, article::PHOTO_COUNT).cast_signed();
    let mut all = true;
    let mut complete = 0;
    for photo in db.table_order(PHOTO_ARTICLE_CHALLENGES) {
        if db.layout_word(PHOTO_ARTICLE_CHALLENGES, photo, 1) != id {
            continue;
        }
        let done = world.signpost.is_challenge_complete(challenge_key_from_id(db, db.layout_word(PHOTO_ARTICLE_CHALLENGES, photo, 2)));
        if needed != 0 {
            complete += i32::from(done);
        } else if !done {
            all = false;
        }
    }
    all && !(needed > 0 && complete < needed)
}

/// `cSignPostManager::PickBLOGPhotoCaptions` (`0x825c8168`): each of the article's photo
/// challenges whose challenge is complete and that has captions picks one,
/// `(int)Simulation::GetRandom(1, captions + 1)` with `Sk8::Random::msLocked` cleared around it.
fn pick_photo_captions(world: &mut World, photo_article: u64) {
    for photo in article_photos(world, photo_article) {
        let challenge = photo_challenge(world, photo);
        let captions = world.db.attribute(PHOTO_ARTICLE_CHALLENGES, photo, CAPTIONS).map_or(0, |a| a.len());
        if captions == 0 || !world.signpost.completed.contains_key(&challenge) {
            continue;
        }
        let Some(random) = world.random.as_mut() else {
            eprintln!("career: PickBLOGPhotoCaptions without the Simulation RNG");
            continue;
        };
        random.unlock_for_commands();
        #[expect(clippy::cast_precision_loss, reason = "the native (float) conversion")]
        let value = random.try_range(1.0, (captions + 1) as f32);
        random.lock_after_commands();
        match value {
            #[expect(clippy::cast_possible_truncation, reason = "the native (int) conversion")]
            Ok(value) => {
                if let Some(completed) = world.signpost.completed.get_mut(&challenge) {
                    completed.photo_caption = value as i32;
                }
            }
            Err(e) => eprintln!("career: PickBLOGPhotoCaptions: {e}"),
        }
    }
}

/// `cSignPostManager::UpdatePhotoArticlesBLOGS` (`0x825d91a0`) after `ChallengeComplete`: each
/// article the challenge photographs for (`GetPhotoArticleCountForChallenge` `0x825c9e48`,
/// `GetPhotoArticleForChallenge` `0x825c9f40`) whose photos are all complete becomes the
/// completion results' `photoArticle`; one without a BLOG entry gets a new one, which unlocks the
/// coverage and picks the captions (outside the initialisation).
pub(crate) fn update_photo_articles_blogs(world: &mut World, signpost: u64) {
    for photo in photos_of_challenge(world, signpost) {
        let id = world.db.layout_word(PHOTO_ARTICLE_CHALLENGES, photo, 1);
        let photo_article = photo_article_key_from_id(world, id);
        if !all_photos_complete(world, photo_article) {
            continue;
        }
        let key = photo_article_key_from_id(world, world.db.layout_word(PHOTO_ARTICLES, photo_article, article::ID));
        if signpost_progress::add_photo_article_blog(world, key) {
            world.signpost.completion.coverage_unlocked = true;
            pick_photo_captions(world, key);
        }
        world.signpost.completion.photo_article = photo_article;
    }
}

/// `cSignPostManager::GetArticlePhotoChallenge(article, order)` (`0x825cfdc0`): the article's
/// photo challenges whose challenge is complete, by completion order (`SortByOrder`), the
/// `order`th (from 1); 0 past the last.
#[must_use]
pub fn article_photo_challenge(world: &World, photo_article: u64, order: i32) -> u64 {
    let mut photos: Vec<(i32, u64)> = article_photos(world, photo_article)
        .into_iter()
        .filter_map(|photo| world.signpost.completed.get(&photo_challenge(world, photo)).map(|completed| (completed.order, photo)))
        .collect();
    photos.sort_by_key(|&(order, _)| order);
    usize::try_from(order - 1).ok().and_then(|index| photos.get(index)).map_or(0, |&(_, photo)| photo)
}

/// `cSignPostManager::GetBLOGPhotoCaption(photo)` (`0x825c8498`): over the completed challenges
/// with the photo challenge's `ChallengeID`, the last one's caption: its picked caption (from 1),
/// else the first caption; none without captions.
#[must_use]
pub fn blog_photo_caption(world: &World, photo: u64) -> Option<String> {
    let db = &world.db;
    let id = db.layout_word(PHOTO_ARTICLE_CHALLENGES, photo, 2);
    let captions = db.attribute(PHOTO_ARTICLE_CHALLENGES, photo, CAPTIONS);
    let length = captions.as_ref().map_or(0, |a| a.len());
    let mut caption = None;
    for (&signpost, completed) in &world.signpost.completed {
        if challenge_id(db, signpost) != id {
            continue;
        }
        let picked = usize::try_from(completed.photo_caption - 1).ok().filter(|&index| index < length);
        if let Some(index) = picked.or((length != 0).then_some(0)) {
            caption = captions.as_ref().and_then(|a| a.text(index)).map(str::to_owned);
        }
    }
    caption
}
