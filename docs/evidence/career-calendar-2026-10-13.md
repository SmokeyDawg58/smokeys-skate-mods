# First-career statistics clock, October 13, 2026

Solo, PC only. Source fix; no compilation or runtime test in this task.

## Report and cause

After the first skater-creation flow, the game showed:

```text
controlled tick after 1 completed frames:
Failed { stage: Career, detail: "\"statistics save requires host calendar\"" }
```

`LiveSkatingClient::stage_tick` installed the local-calendar callback on each
tick. However, `SelectedLiveSession::set_career_calendar` only wrote it into an
already-existing `CareerSession`. The call was discarded while the career was
absent. Both `CareerSession::new` callers created a challenge world whose
`hom_local_calendar` was `None`.

The editor return path can call `start_career(false)`, then
`close_editor_shop`, then `save_career` for a nonempty purchase receipt in the
same call. There is no intervening `stage_tick` to install the clock. This is
a concrete path to the reported error. Other saves and HoM metadata on the
first tick of a newly created challenge world had the same missing service.

## Reference evidence

Live IDA was unavailable for this task.
Used the previously exported original decompilations, not a new live audit:

- `.scratch/prop-hom-20261010/825698b8.c`:
  `Sk8::BE::HOMResultsData::Reset` calls `GetLocalTime` and copies year, month,
  day, hour, minute and second into its six time fields.
- `.scratch/prop-hom-20261010/82569948.c`:
  `HOMResultsData::Init` calls that reset before copying the bail output and
  accumulating injuries.
- The existing `stats_backend::save_statistics` / `hom_save::record` retain
  recorded bail timestamps and require a real host calendar for empty record
  slots. The PC's existing `local_calendar::now` supplies the same six local
  calendar components.

This is a host-service lifetime repair. It changes neither the statistics
record format nor reconstructed gameplay behavior. Confidence: **high** for
the source cause and repair; the reported runtime route remains unverified.

## Changes

- The long-lived `SelectedLiveSession` retains the clock callback even when no
  career exists. Replacing the callback also updates an existing career.
- The PC installs it immediately after session construction, before boot or
  any frontend processing, instead of relying on a later simulation tick.
- `CareerSession::new` accepts and installs the retained callback before its
  caller loads profile data, starts challenge flow, or saves. Both callers
  pass it: new/continued Career and Party Play.
- Returning to the main menu, Free Skate, or changing Party Play challenges
  drops the career without clearing the retained host provider. The callback
  is sampled when needed; no startup date is cached.

## Review and limits

- Reviewed all `CareerSession::new` callers and all calendar writes; both
  construction paths receive the retained provider.
- Reviewed the career destruction sites; they leave the session provider
  intact.
- `rustfmt --edition 2024 --emit stdout --config skip_children=true` parsed
  all three changed source files successfully without rewriting them.
- Reviewed the source diff against task backups. No tests added or run, no
  compiler or game launched. The existing release executable is unchanged;
  the next build is required to use the fix.
- No save files or Android files were edited. Backups of the three source
  files and existing documentation are in
  `.scratch/career-calendar-20261013/before/`.

Pending runtime check after rebuilding: finish a fresh profile's creation
and enter Career, save, return to the main menu and continue. Use a separate
save root when checking a fresh profile; a real save must be preserved.
