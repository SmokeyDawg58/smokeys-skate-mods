# Edit Skater wear styles (2026-10-17)

Report: in Edit Skater some custom board colours stopped the game with
`missing original localized UI string ID_CAC_WEAR_STYLE_OPTION_17`, and the "Option" row under the
deck's colour picker looked wrong.

## The original (`sk82_na_m.xex`)

- `FrontEndState_CaC::OnThumbnailSelect` (`0x8268aee0`), after the receipt checks: the part's
  `GetWearStyleAlternates` decides the wear styles menu (24), which opens for a colourizable or
  logoable part, or one with more than one alternate. A deck (`PARTSLOT::SKATE_DECK`, 11) of the
  worn deck's style subcategory leaves the worn deck on: the worn deck is worn again, and nothing
  is worn when the selected deck is the worn one. Then `WearPart`.
- `SetScreenState(24)` (`0x82684b40`), also on coming back to the menu (`PopScreenState`
  `0x826897d8`), from `mCurrentPart` (set by `OnMenuHilight` `0x826806f8` to the highlighted
  thumbnail, and by the style row): Colour when colourizable; unless the part is a deck,
  `mPartList = GetWearStyleAlternates(mCurrentPart)` and, with more than one, Style at
  `mCurrentWearStyleIndex = GetPartIndex` (0: the part is the list's first); Graphic when logoable.
  A deck never has the Style row.
- `CASSimManager::GetWearStyleAlternates` (`0x8259d710`): the part first; for a part with a style
  subcategory or a hat, `GetParts` (`0x8259d510`) of its slot's range under
  `PARTCONDITION_SHINYNEW`: a pack on disk, `WearStyleFilter::AcceptImpl` (`0x82589390`: same
  gender, a different part id compared in full 64 bits, the same group: the style subcategory,
  or for a hat the material variant name when the part has no subcategory) and not
  `pIsUsedInGame`.
- `ReceiveInput` (`0x8268d818`), Style row: the index steps and wraps, `mCurrentPart` becomes that
  alternate, `WearPart`. `GetStringValue` (`0x826775f0`) shows
  `ID_CAC_WEAR_STYLE_OPTION_%d` of `mCurrentWearStyleIndex + 1` (the tables hold 1 to 16).
- `GetTitleText` (`0x82649408`): `ID_CUSTOM_EDITDECK` when `mCurrentPart` is a deck, else the
  menu's title.

## The port before

It listed every part of the deck's group in database order, decks included, with no used-in-game
filter, and numbered the worn part by its place in that list: decks reached 20 parts, so
`ID_CAC_WEAR_STYLE_OPTION_17` and up. Under the original's rules the longest list (hats) is 7.

## Now

`edit_skater_frontend.rs` keeps `current_part` and `wear_style_index`; `wear_styles_menu` is
`SetScreenState(24)` on every entry; `edit_skater_shop::wear_style_alternates` is the original's
list (also used by the receipt); `CacPartDb::slot_range` gives a slot's range.
