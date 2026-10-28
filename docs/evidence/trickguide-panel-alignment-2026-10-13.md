# Trick Guide panel alignment - 2026-10-13

## Status and scope

**Reverted later on 2026-10-13:** the original offsets
are kept. `trick_guide_movie.rs` is restored exactly to
its pre-adjustment version; the changes described below are historical.
See `replay-cac-followup-2026-10-13.md`. Earlier pause-input/replay fixes remain.

PC source change in `apps/skate2-pc/src/marker_ui/trick_guide_movie.rs`.
A new crop showed the lower panel protruding to the left
and overlapping the category panel. The previous audits retained those edges;
this change deliberately aligns them. **This is a PC presentation adjustment,
not a newly recovered native layout.**

No build, tests or game launch. Source syntax parsed
successfully with `rustfmt --edition 2024 --emit stdout`; output was discarded
without reformatting the file. In-game appearance remains unverified.

Backups, native decompilations, decoded metrics and the scoped review are in
`.scratch/trickguide-panel-alignment-20261013/`. Existing return/input/replay
fixes from `replay-trickguide-audit-2026-10-13.md` are preserved.

## Original-reference findings

- `AptTextureUnit` constructor **826098a8** retains GEO vertex positions.
  `SetGameTextureInfo` **82609aa0** derives UVs using the texture dimensions;
  `Draw` **82609d38** passes those positions/UVs to SimpleDraw with the supplied
  vertex transform. There is no special Trick Guide width correction here.
- Trick Guide `OnEnter` **8264e920** opens screen32. `OnLoaded` **82687c08**
  initializes categories without resizing the panels.
- `RegisterScreenTemplates` **82628098**, entry **82fdb558**, identifies
  `TrickTutorial` / `tricks/trickguide.swf`. Its third word2 is TransitionType;
  its fourth word0 is FEScreenType2D. This is not a tilted 3D panel.
- Original root/inner matrices are translations: top (809,77)+(-59,4),
  bottom (748,370)+(-3,-3). Root intro/outro animate alpha. The panel hierarchies
  have no mask that trims their overlap or equalizes their widths.
- The previous Xbox/PS3 comparison established matching APT/CONST/GEO data
  and matching decoded pixels for textures1,2,18,19. This audit also followed
  their GEO positions and UVs through the draw path. Texture padding cannot
  be used as the visible edge.

Confidence: **high** for these source/asset findings. No original-game frame
capture of this page was used to establish full runtime visual parity.

## Applied layout

Coordinates below are the original 1280x720 UI space. Right/bottom are exclusive
nontransparent texel bounds. The normal viewport mapping still applies to the
whole UI uniformly.

| Artwork | Original bounds (left, top, right, bottom) | New bounds |
| --- | --- | --- |
| Upper panel, shape20 / textures19+18 | 824,89,1188,393 | 824,89,1188,393 |
| Lower panel, shape3 / textures2+1 | 809,373,1192,632 | 824,393,1188,632 |

The lower background alone changes scale to 364/383 horizontally and 239/259
vertically. Its local translation accounts for the original transparent
padding (left64,top6). This aligns the side edges, removes the 20px overlap,
and keeps the lower edge at632 above the footer.

The description is moved to the upper heading's actual text origin, including
its authored -2px text bounds and 0.99731445 horizontal scale: X839.50537.
Its wrap width becomes332.98926, keeping the same padding on each side.
Font, height, vertical position and diagram scale are retained. The gesture
group's origin moves13 ->18.5, following the panel center's +5.5px shift;
every selection uses that new base with the original kerning/centering formula.

Alignment runs on each fresh movie link before population/intro. The changed
background/description children use the existing script-transform mechanism;
the outer panel alpha fades remain controlled by their original timelines.
Category changes do not reapply transforms cumulatively. Reopening starts
from fresh authored instances and recomputes the same alignment.

## Review limits

The scoped source diff and syntax were checked. No compiler/type check or
rendered screenshot was produced for this change.
It must not be reported as visually verified or
as 1:1 native panel positioning. No shared renderer, physics, Android or save
code was changed.
