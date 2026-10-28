# Shared selected mounted startup composition

Extracts the production sequence previously in `actor_player_startup_corpus`
into the existing retained `SelectedPlayerStartup`. `new_mounted` constructs
native empty recipe/load/settings/profile/face fields with supplied mounts,
Vault, cvars, local skater owner, animation manager allocations, Juice and Pause.
The selected index, marquee and spawn/flags are explicit host inputs. The
continuation rejects a foreign world's skater owner or mismatched local index.
It never constructs a second world, backend, controller or input-history owner.

`try_initialize_mounted` retains attempted state before fallible work, then
reuses offline CreatePlayer, selected loader SetMarquee, actual Vault schema
projection, mounted recipe completion and attribute continuation. It requires
the actual returned Register action, not completed registration. Global
animation initialization then uses the supplied animation source, followed by
the existing Actor/input/physical-animation constructor. All partial owners
remain in the same startup on failure. A failed attempt is not retried.

`try_initialize_mounted_presentation` is a separate continuation over the same
retained recipe and supplied source. This preserves the caller's intervening
registration boundary. It does not force graph asset loads before actual
InitPlayer. The component corpus still explicitly loads its diagnostic graph
branch between physical animation and presentation; native-roster mode does not.

Native behavior is reused from reviewed producers: CreatePlayer8277b168,
GetDefaultMarquee82cfdbe8 constant provenance only, PlayerLoader attribute/recipe
completion, Simulation.InitAnimation828a5640, Actor825fef00..ff16c and existing
presentation startup. This extraction does not infer new native scheduling,
profile validity, job/thread identity, registration completion or physical
allocation.

The corpus now calls the shared builder and retains the existing loader phase,
backend type/peer, finalized instance identity, registry scene identity, recipe
name, presentation and listener assertions. Its mounted animation source uses
the SAME MountSet Arc as recipe/input/presentation instead of opening anim.big
again through a diagnostic helper. Actual job manager initialization remains
the caller's action; production supplies its own callback, while the corpus's
thread1 and selected default marquee/spawn remain explicitly labeled policies.

Files: `selected_player_mounted_startup.rs` plus artifact shared patch
`artifacts/selected-mounted-startup-2026-09-22.patch` for existing startup and
corpus. No registered edits or Cargo by author. Original-data validation and
whole-session integration remain root responsibilities.

Independent source/code review approved the retained owner composition, corpus
extraction and separate presentation continuation. Author and reviewer patch
apply checks passed; no author or reviewer Cargo run.
