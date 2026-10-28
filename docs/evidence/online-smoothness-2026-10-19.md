# Online smoothness — October 19, 2026

Why a remote skater (spectated, in the crowd or in lobby skate) could move in steady, unsmoothed
steps for long stretches, worst at locations with big airs, and why the world under the pause
menu of an online game moved without the frames between ticks. Native addresses are the debug
`sk82_na_m.xex` (Jan-17-2009).

## 1. Slow motion online

- `Simulation::OnRequestSetSimRate` (0x8289e350): in a network game (`IsNetworkGame` 0x8289e448,
  `CommonBackEnd::sGameInfo.isNetworkGame`) the rate is `kUpdateTime` (1/60 s) whatever the
  message asks, so the timer stays at 60 Hz.
- `Sk8::Behaviours::SlowMotionController::Begin` (0x82ca8e20) and `Update` (0x82ca8f48), the
  camera graph's big-air slow motion, post `cMsgSetSimRate` only when
  `SkaterManager::IsInAnOnlineGame` is false. `End` (0x82ca9170) posts `kUpdateTime`.
- `StateOnlineGame::SetupInitalGameInfo` (0x8275db08) sets `isNetworkGame` (the store at
  0x8275dba8); `GameModule::BackToSP` (0x8273da50) clears it (0x8273dac8).
- The port's `PhysicalGameInfo::network_game` was constructed false and had no writer. Online, the
  camera's slow motion therefore posted its rates and the Simulation took them, and the app paces
  its ticks by `timer_period(sim_timer_hz())`. A skater in a big air ran its own game, and sent
  its frames, at the shot's slower rate (`fps_at_scale_one` blended by the shot's timescale).
  Its watchers received fewer remote ticks a second than the playout can slow down to (a
  quarter under real time), so the moment shown passed the newest sample and held there until
  the next run came. After the slow motion ended, the arrivals' spread stayed above the playout's
  30-tick cap for the three seconds the arrival window spans, and the moment shown kept running
  ahead of the frames.
- The flag itself stays clear: other consumers stop at network-game branches the port has not
  ported (`PendingNetworkLivingWorld`, `PendingNetworkReset`, `PendingNetworkCollision`). In its
  place, `SimulationSimRate::set_online_game` makes `try_is_network_game` answer true for the
  rate owner. That owner backs both gates above. `update_network_pause` sets it every tick
  while the local skater is in an online game's world.

## 2. The playout

`RemoteSkater`'s `Playout` aims at the local tick less the earliest arrival offset in a
180-frame window and a delay, the window's spread capped at 30 ticks, plus a margin, at real
time's pace. Three flaws showed in a simulation of it:

- **A cap measured from a stale earliest.** When the two games' ticks drift apart, the window's
  earliest offset is up to three seconds old. A game that cannot keep 60 ticks a second, a
  slow-motion burst, or a stream that stopped and resumed behind (a map travel stops a game's
  completed ticks under the fade) all spread the window past the cap. The moment shown then
  runs ahead of the frames, so the skater steps from run to run.
- **No pace.** The rate controller assumed the sender's ticks come at real time. With a slower
  sender its steady state sits ahead of the target by the pace difference over its gain.
- **Running past the newest frame.** Through a gap the moment shown kept moving. When frames came
  again it was ahead of them and had to step its way back.

The changes:
- **Delay.** The cap is 45 and the margin 3 ticks.
- **Pace.** The pace is the target's own movement over the last 180 ticks, between 0.75 and
  1.25. The rate controller runs at it.
- **Newest frame.** The moment shown never passes the newest frame.
- **Re-basing.** The window drops its older arrivals when the newest 12 frames all arrive more
  than 10 ticks later than the older ones' 95th percentile, or earlier than their earliest.
  Jitter does not do that, since a run on time among them would show.
- **Reserve.** How far the 95th-percentile arrival lay behind the median is remembered, measured
  over the arrivals before the newest 12. The newest 12 are left out so that a timeline shift is
  re-based before it counts. The reserve decays by 8 ticks a minute, and the target uses at least
  the median plus it. A route that spikes every few seconds otherwise met every spike after a
  quiet three seconds with a hold.
- **Held console.** Frames that come while the receiver's own ticks are held
  (`network_resumed` false: a travel, a load, a sync point) are kept but not counted, and the
  first counted afterwards clears the window. Stamped with the held tick, they otherwise bridged
  the old and the new timeline: the re-basing never saw a shift, and the spread they made stayed
  in the delay.

The simulation drives the exact `Playout` code:
- **Sender.** Each sender tick on its clock; runs end at samples (`tick % interval == 0`) and go
  out at the sample's time.
- **Link.** Each run takes the link's delay, jitter, spikes and loss.
- **Receiver.** It stamps an arrival with its completed ticks and shows at the next tick, as
  `update_online` and `online_tick` do. While its ticks are held it counts no arrivals.

Each case ran 180 s, the first 3 s left out. WAN is 110 ms, up to 30 ms jitter, 3% spikes of
120 ms and 1% loss. "Held" counts ticks whose drawn moment stopped at the newest sample while
the sender's ticks went on. "Jumps" counts steps of more than twice the sender's pace. The last
column is how far behind the newest sample the drawn moment was on average:

| Case | Held, before | Held, after | Jumps, before | Jumps, after | Behind the newest sample, before/after (ticks) |
| --- | ---: | ---: | ---: | ---: | ---: |
| LAN, 30 runs/s | 0.00% | 0.00% | 0 | 0 | 3.6 / 4.6 |
| WAN, 30 runs/s | 0.00% | 0.00% | 0 | 0 | 5.1 / 9.9 |
| WAN, 20 runs/s | 0.00% | 0.00% | 0 | 0 | 7.1 / 10.9 |
| Sender at 63 Hz | 0.00% | 0.00% | 0 | 0 | 12.2 / 14.3 |
| Sender at 57 Hz | 0.24% | 0.00% | 3 | 0 | 2.9 / 5.1 |
| Sender at 55 Hz | 0.85% | 0.00% | 13 | 0 | 2.1 / 4.0 |
| Sender at 52 Hz | 20.60% | 0.04% | 186 | 0 | 0.3 / 3.3 |
| Sender at 48 Hz | 51.36% | 8.30% | 181 | 0 | 0.0 / 0.7 |
| Sender at 45 Hz | 48.41% | 5.34% | 168 | 0 | 0.0 / 0.7 |
| Sender at 50 Hz for 4 s in every 10 | 11.20% | 0.15% | 86 | 0 | 2.8 / 13.1 |
| Receiver at 50 Hz | 0.00% | 0.00% | 0 | 0 | 29.3 / 33.6 |
| Slow-motion bursts (1.5 s at 35 Hz every 6 s) | 34.11% | 0.08% | 203 | 0 | 1.4 / 18.2 |
| Sender hitch of 250 ms every 3 s, caught up | 1.53% | 0.71% | 57 | 0 | 10.2 / 11.9 |
| Sender stopping 1.5 s every 10 s, not caught up | 28.39% | 2.18% | 219 | 5 | 1.6 / 8.2 |
| Receiver hitch of 400 ms every 3 s | 0.00% | 0.00% | 0 | 0 | 6.2 / 11.2 |
| Receiver held 1.5 s every 10 s | 0.00% | 0.00% | 0 | 17 | 13.3 / 10.9 |
| Route spikes of 200 ms every 25 s | 0.68% | 0.06% | 24 | 0 | 7.0 / 14.8 |
| Route spikes of 120 ms every 12 s | 0.88% | 0.02% | 14 | 0 | 5.7 / 10.4 |
| Bufferbloat (100–220 ms over 5 s, 2% loss) | 0.06% | 0.00% | 3 | 0 | 7.5 / 11.8 |

The jumps of the held receiver are its skaters catching up once, after its own hold.

The slow-motion bursts case is the old online behaviour, which section 1 removes. The tests in
`remote_skater.rs` cover four cases:
- a 50 Hz sender;
- a stream that stops for 90 ticks and resumes behind;
- a link that spikes every ten seconds;
- a receiver held for 90 ticks.

Each fails without the change it covers and passes with it.

## 3. The front end over an online game

`LiveSkatingClient::threaded_tick_allowed` kept the pause menu, Options and the music player on
the serial path. That split was made for single player, where nothing simulates under them
([performance-threading-2026-10-02.md](performance-threading-2026-10-02.md)). The serial path
draws each tick as it completes: `complete_tick` takes no restorer, and the window resets the
restorer every frame. An online game keeps simulating under those screens (`Net::ResumeGame`,
the facade's `network_resumed`), so there the world moved at the tick rate without the frames
between ticks. While `online_simulates_under_frontend` holds, those screens now take the
threaded path and its restored frames.

## 4. Send rates

Unchanged: a sender's run is two ticks (30 a second) while its skater plays the round (its turn
in a turn-based challenge, or a challenge everyone plays at once), three (20 a second) otherwise.
Every tick of a run carries the root, board speed, flag words and audio words; the sample (the
run's last tick, and a teleport tick and the one after it) adds the pose, twist ratios, audio
floats and game camera.
