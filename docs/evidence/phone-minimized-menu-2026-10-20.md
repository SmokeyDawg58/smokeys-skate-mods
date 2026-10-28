# The phone's minimized menu: service icons and alerts (2026-10-20)

Near a capped rail, once Mike's uncapping service is unlocked, the phone list HUD's quick menu pops
up with Mike's icon. The service challenges (`service_uncapping`, `dist_*_uncapping` and the pool
draining ones) drive it:

- `wait_for_service`: a `cMsgWorldPainterServiceNotification` whose `ServiceKey` is the
  challenge's key (`cServiceManager::WorldPainterKeyChangeMessageHandler` 0x8289d390, the services
  WorldPainter layer at the skater) moves it to `service`.
- `service` enter: with the service unlocked (`Uncapping Available`) and the hull not yet in its
  unlock state, `Service.SetServiceAvailable(data, type, true)` and `HUD2.ShowUncapHUD(true)`
  (or `ShowPoolDrainHUD`).
- `worldpainterleftservicenotification`: the icon off, the service withdrawn, back to waiting.
- `servicerequested` (the phone's services menu, `ActivateUncappingService` 0x8289d6d0) leads to
  `handle_service`: the service movie, the hull set to its unlock state, then on
  `fmvfinishedplaying` the icon off and the challenge won.

`HUD2.ShowUncapHUD` (0x82899d48) only sets the `phonelist` group's
`PhonelistHUDData::SetServiceIconStatus(eUncapping, show)` (0x826b3748), which stores the icon and
dirties the data on every call. `PhonelistHUD::Update` (0x826c06f0) answers dirty data while
minimized with `Minimize`, `SetState(0)`, and `hudphonelist.apt`'s `SetState(0)` runs
`UpdateMinimizedMenu`:

1. It asks `HUDComponents` for `GetPhoneListSecurityIconStatus`, `PoolDrainIconStatus` and
   `UncappingIconStatus` (`PhonelistHUD::GetServiceIconStatus` 0x826b5f90: the data's icon, then
   `CanShowPhoneListAlerts`), `GetPhoneListSponsorPP` (0x826b6328: the `Sponsor PinchPoint`
   setting) and `GetPhoneListAnywhereFilmPP` (0x826b6428: `Anywhere Film PinchPoint`), both also
   behind `CanShowPhoneListAlerts`.
2. `CanShowPhoneListAlerts` (0x826b5ea0): `IsSinglePlayerMode` (game mode 1 or 7), the
   `Uncapping Available` setting, and no main challenge.
3. With any service icon, `mcQuickMenu` plays `minimized`, its header reads
   `ID_PHONELIST_USE_PHONE`, `mAlert` hides, and the two slots `mIconServices0` and
   `mIconServices1` play the shown services in security, pool drain, uncapping order (`bb`,
   `sammis`, `mikes`), an unused slot `off`.
4. Without one but with an alert, the same `minimized` with `mAlert` shown, reading
   `ID_PHONELIST_ANYWHERE_ALERT`, else `ID_PHONELIST_SPONSORSHIP_ALERT`, and both slots `off`.
5. Otherwise `mcQuickMenu` plays `hidden`.

Each icon label places the service's sprite, which grows from 5% to 60% scale while fading in over
nine frames, then flashes and repeats; the alert line fades in and flashes the same way. The screen
is template 12, a 3D HUD under the FocusLeft projection.

## The port

The minimized state always played `hidden`, so no service icon or alert ever showed.

- `PhoneListData::minimized_menu` (`ui/phonelist.rs`) decides it, from the data's icons and the
  game's `AlertFacts` (`challenge::contacts::alert_facts`: `can_show_alerts` and the two settings).
- `CareerHud` carries `phone_alert_facts` and `phone_data_dirty`, the `phonelist` data's dirty
  flag taken once, so a call that leaves the icons unchanged still refreshes the menu.
- `marker_ui.rs` `update_minimized_menu` plays the labels, texts and the alert line's visibility.
