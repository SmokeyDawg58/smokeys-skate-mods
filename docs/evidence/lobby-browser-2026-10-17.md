# Lobby Browser (2026-10-17)

A PC addition: a list of the dedicated server's public unranked games, opened from the bottom
of the Online options. The original game finds games only through PlayNow matchmaking
([matchmaking.md](../online/matchmaking.md)); nothing in it lists games. Behaviour and protocol:
[implementation.md](../online/implementation.md#the-lobby-browser). Addresses are
`sk82_na_m.xex` VAs (IDA, read-only).

## Player limit

- `Sk8::Flow::MaxPlayerFilter` (constructor 0x8276bfb0, attribute 5, `max_players`) filters
  matchmaking by `mMaxPlayers`: 8 for unranked games; in ranked play 6 for Own the Spot and
  Hall of Meat, 4 for 1-Up and S.K.A.T.E. The server now makes every game with
  `protocol::MAX_PLAYERS` (8); it held 6 before (protocol 10, `transport::PROTOCOL_VERSION`).
- `HubStateOnlineHost::OnEvent` 0x8268b940 formats the hub's Player List title as
  `"#%s (%d/%d)"` with a literal total of 5, counting the other players. The port's total is
  `MAX_PLAYERS - 1` (7).
- `kOnlineColours` holds eight colours, and the in-game leaderboards author eight rows.
- `ranked_results_overall` (asset 112): `normal` has room for six list rows above
  `txtNextLabel`, and `mSkateCol` authors six letter rows (`mcSkate0` to `mcSkate5`, 38 apart).
  `full`, the layout `SetupNewRank` and `SetupNewBadge` take (`gotoAndPlay("full")` on `mWindow`
  and `mContent` before the screen's text is set), moves the list 40 up and `txtNextLabel`,
  `txtNextTimer` and `mButtonMenu` 80 down, and recreates the labels and column headers. A
  board of seven or eight players takes `full` with `mRankUp` hidden, and the letters column
  gains `mcSkate6` and `mcSkate7`, instances of the same letters character 38 apart.

## What the screen reuses

- `unranked/unranked_custom_match` (asset 105): its timeline (intro to `IntroComplete` 0x874c,
  outro to `OutroComplete` 0x878c, the fades of depths 1, 5, 10, 42, 46 and 49), its panel and
  top glare (depth 49, shape 32, at most 30% white), `mTitle`, `mContent.mButtonHelp` and
  `mOptions.mMatch` (`txtTitle` and the loading wheel). `mGamerTag`, `txtHelp`, `mPreviewImage`
  and the three selectors are hidden.
- The panel (shape 4 of the sprite at depth 1, 704x512 at 292,106: three textures 512 high, one
  texel a unit) has, from the top: the outer frame, the title strip with its rounded corners
  (12 to 64), the gamer tag's header band (64 to 146), a light line (146 to 149), the content
  with the location preview's frame drawn into it (x 400 to 656, y 178 to 328), a dark line, the
  help band (358 to 468), the inner card's bottom edge and the bottom strip. The browser draws
  it in `shape_bands`: the top 64, the line and three rows under it (146 to 152) at 64, the
  content rows 158 to 162 (the most uniform, clear of the preview's frame) stretched over 70 to
  469, and the bottom (469 to 512). The title strip, and so the title, is where the custom
  match screen has it; the list fills 67 to 469 (the stage's 173 to 575).
- `background_scaler` `mcShading` (character 51) for each card: 8x32 corners, 1x32 and 8x1
  edges, a 1x1 core; black with soft edges, solid from 32 under its top to 30 over its bottom.
  The browser draws its pieces at half their height, so that a 130-high card is solid from 16 to
  115, under its title line (12, its letters from about 14, over 97% black), image (42 to 114,
  8 under the title line) and status line; three cards with four spaces of 3 fill the list.
- `menu_picker` `mIndicator` (character 53, labels `normal` and `dark`; its frame actions at
  0x3a00 and 0x3a0c are `stop`), placed beside a card's title as the picker places it beside an
  item's text (41 left of the item, the text 10 in).
- `basic_slider`'s vertical slider (character 53: `mVerticalBar`, `mcScrollbar`; every piece's
  frame action is `stop`), sized by `map_movie::update_scrollbar`.
- The custom match screen's `image_renderer` (import character 8; bounds 434x237) for a card's
  picture. The location small images (`<image>_sm`) are 128x128 textures shown at 16:9, as the
  custom match screen shows them.
- Text: the title's HelveticaPresto (character 29) and HelveticaGlow (character 28) fields, and
  `txtHelp`'s Helvetica LT Std (character 7, colour 0xffb4b19b); `menu_picker`'s item colours
  0xfffffeec (selected), 0xffdae8e1 (deselected) and glow 0xffffff99.

## Strings

- Native: `ID_RANKED_MATCHMAKING` "Searching for Game Session...", `ID_ONLINE_MATCHING_SKATERS`
  "Matching up with skaters...", `ID_JOIN_ONLINE_GAME_FAILED_POPUP_TITLE` "The Server is Full"
  and `_MESSAGE` "Couldn't connect: the server is full!", `ID_ONLINE_GAMESESSION_NOTEXIST` "The
  game session you are attempting to join no longer exists.",
  `ID_SIDEKICK_ONLINE_SESSION_FULL_ERROR` "Unable to join game session.  Please try again.",
  `ID_EANATION`, `ID_COMMON_BACK`, `ID_COMMON_OK`.
- Literal: "Lobby Browser", "Join", "Refresh", "In Game", "Pre-Game", "No online games right
  now.".

## Online options

- The native screen has seven rows. The port leaves out Ranked, the ranked leaderboard and the
  Skate profile (the server runs no ranking or profile services), and adds Edit Nickname above
  the native rows and the Lobby Browser below them: six rows at the picker's own pitch
  (`dummy0`'s height, 31.4 units, 37.3 under `mMenu`'s 1.1875 scale).
- The board (shape 2, one 512x512 texture at the root sprite's 0.95 scale) is drawn without two
  row pitches of its middle (`shape_bands`, heights 208 to 286, 78 units), where its texture's
  rows above and below the cut match: one pitch for the seventh native row, and one so that the
  space under the last row's baseline (about 25 on the stage, 62 on the native screen) is nearer
  the space over the first row's letters (about 15).

## Validation

- `crates/skate2-net` tests: listing and joining, an eight-player game reported full, a list
  request before the Hello, and the list rate. `ui/lobby_browser.rs` tests: scrolling and
  wrapping, joining and its notice, a new list keeping the selection, input before the intro
  and the list.
- rustfmt parse of the changed files. Not compiled or run when written.
- The panel's bands, the half-height card fades and the six-row Online options compile with no
  new warnings (`cargo check -p skate2-pc`). Not run when written.
