# Lobby stalls and local sky refinement

The owner described a friend's brief freeze followed by return to the start
screen. Their local public Update28 journal records no crash/watchdog or peer
leave reason; it cannot prove the friend's cause. The new journal preserves the
server's raw Kicked reason and peer-leave reason as well as transport failures.
The old UI mapped all ServerMessage::Kicked codes to the same notice.

Inspection confirms a two-second server game-activity timeout, separate from
the ten-second transport timeout. The game normally sends the existing Busy
message every250ms; that producer cannot run during a blocking frame/load.
The existing socket worker now bridges a short stall after a genuine Busy:
after500ms without another, at most one Busy per250ms, ending five seconds after
the last game-produced Busy. It is never armed before one, clears on LeaveGame,
Kicked or disconnection, does not synthesize frames/facts, and changes no server
policy, connection timeout, protocol or build identity. An unresponsive game
still expires after the bounded grace. This is an explicit PC loading adaptation,
not claimed recovered retail behavior or proof of the friend's particular boot.

Production encrypted in-memory client/server tests pass a three-second loading
stall, verify prolonged inactivity still kicks, and retain real host-kick and
host-authority behavior. These tests contact no public service or other players.

The local sky now has distinct broken Light Rain clouds, thicker Medium overcast,
dark Heavy thunderstorm banks/upper anvil veil, and partly clearing AfterRain.
Two layers use different dome projections and wind speeds. Their wind loop is
continuous at the4096s clock wrap; broad billow gradients and existing lightning
light the storm clouds without volumetric ray marching. This remains authored
PC weather. MSAA edge composition replaces only the original clear sky's sample
coverage, preserving geometry coverage instead of a bright silhouette halo.
Rain drops use nearest sample depth at silhouettes to prevent edge bleed.

The owner's running log also showed200% resolution scale, four times native
pixel count. The separate preview will select120 FPS cap and100% native scale;
normal game and all normal volume settings remain untouched. Performance tests
while another supersampled game instance runs cannot establish isolated120 FPS.
