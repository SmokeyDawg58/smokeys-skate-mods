# Juice input boot admission

Read-only source/owner audit, September 22. Confidence **high** for the native
branches below; no network attempt, implementation, or Cargo run was performed.
This extends `selected-input-admission-audit-2026-09-22.md`.

## Actual configuration and constructor

The supplied original root's loose `data/juice/Juice_m.ini` exists and was read
successfully. It has `[JUICECONFIG] enable=1`, `serverport=31001`, and these
endpoints: default `10.10.226.22`, HTC `10.107.84.18`, Madrid `10.20.129.141`,
VMC `192.168.80.13`. Gamepad/RPC/online/telemetry plugins are enabled. These are
observed original configuration values, not proposed production defaults.

`IniReaderJuiceIni82a76c38` stores enable=false at `82a76c74`, then reads
`JUICE_INI` (`83004754` points to `82209fb0`, `data\\juice\\Juice_m.ini`) at
`82a76d08`. `ReadFromIniFile82a75638` leaves constructor values when the actual
file loader returns null; otherwise it handles UTF-8 BOM then parses. An
unavailable host root/mount is not evidence that this native read returned null.
`SetFromIniFile82a75530` recognizes case-insensitive JUICECONFIG section/key
names; `SetJuiceVars82a74ce8` sets enable exactly when `ConvertAToI(value)==1`.
This is sampled during construction, not a cvar re-read on every input poll.

`JuiceBase82a776f0` constructs INI, network/core and gamepad plugin, then publishes
the single `sRef`. Core construction always reaches network initialization:
`JuiceCore::Initialize829693f0` installs the supplied network at `82969430`,
then `JuiceCom::Initialize82965570` calls it at `82965664` and retains
`mComLayerUp`; this happens even if INI enable=false. Xenon adapter
`JuiceXNet::Initialize82a74660` samples XNetStartup and WSAStartup(2.2); its
result requires both successes. Do not fabricate successful initialization.

## Start, endpoint selection and failure

Boot constructs/initializes InputManager at `82740db8..bc`, constructs Juice,
calls `JuiceManager::Start826e36b0` at `82740e38`, then installs the actual
Juice instance as external pad remap at `82740e54`.

Manager Start first samples `kEnableJuiceConnectionRetry`; initializer
`82f5ce90`, table `82fd1578`, ref `8304e694`, section Debug, title Enable Juice
Connection Retry, default=false, null callback. The actual startup.ini inspected
does not override this label. `JuiceNG::Start(bool)82a75ba0` stores it as
`mWaitForIP` (+0x6c10), then invokes Start. The inspected Start/GetIpAndPort
path does not use it to increase connection attempts.

`GetIpAndPort82a75ec8` first copies configured default endpoint/port, then:

1. Calls XNetGetDebugXnAddr and formats its IPv4 address. If formatted length>7,
   prefix `10.107.` selects HTC, then prefix `10.20.` selects Madrid.
2. Calls DmGetXboxName. Only status47841280 enters name overrides: length>=3
   and case-insensitive `vmc` selects VMC. Then length>=11 and `jcon` prefix
   selects the suffix after jcon up to '-' or '_'. The last override wins.

These need actual supplied platform identity; no invented zero address or failed
Xbox-name status. The source ignores the XNet address return code, so unknown
output cannot be treated as known zeros. The jcon copy also needs an explicit
host bounds boundary for malformed input, not silent truncation.

`JuiceNG::Start82a76e90` exits when disabled. Otherwise it obtains the endpoint
and uses an integer attempt count initialized to ONE (`82a76ec4`), not an
unbounded retry. Failed connection sleeps 2000ms at `82a76f08`, then the loop
exits. `JuiceCore::Connect82969d30` checks session=-1 and disconnected before
`JuiceCom::Connect82965aa8`; Com skips transport if mComLayerUp=false. Actual
transport `JuiceXNet::Connect82a766a8` parses IPv4 and calls
`EstablishConnection82a74500`: socket(AF_INET,SOCK_STREAM,0), blocking connect,
failure socket sentinel=-1. No application-level connect timeout is present;
OS connection delay precedes the additional two-second sleep.

Transport success is not boot completion. `InitialConnectionHandshake82969eb0`
disables registered server plugins, requests enabling each, sends InitComplete,
then pumps Com while requests are incomplete and still connected. Start then
spawns the update thread, sleeps100ms, runs SetupGamePad, initializes selected
plugins, and only then stores ready=true at `82a76fb0`. Source readiness
`8308c904` starts as zero in the image; this success store is its only write xref.
Failure does not reset an already-ready singleton, so an admitted first-start
implementation must not silently support repeated Start by clearing readiness.

## Device ownership and proposed connected packet

Gamepad ctor `82ed7d48` has device count0, capacity0 and null device storage.
Successful `SetupGamePad82a75c00` supplies two device slots, constructs24 button
channels, registers controllers0 and1 at `82a75ea8`, then Start publishes ready.
`UpdateGamePad82a74c70` returns without changing any lane while not ready;
otherwise it resolves the same singleton and calls `GamePadPlugin::Update82ed8768`.
The latter invokes a device only when index<count. Therefore neither null-remap
nor constructor device-count0 is a valid replacement for completed enabled boot.

No Juice socket/identity adapter exists in apps or crates: searches for
TcpStream/TcpSocket/connect_timeout/WSAStartup/winsock/XNetGetDebug/DmGetXboxName
found none. `windows_pad_platform.rs` supplies actual XInput only;
`windows_selected_input.rs` already exposes poll->SIM cache->PRES cache and SIM
drain over the same core runtime. `input_cvar_ini` has different section/case/
duplicate grammar; reuse only its recovered integer conversion, not its parser.

Proposed first packet: one retained Juice boot occurrence holding actual parsed
configuration, shared ready state, connection prefix and supplied transport;
reuse the existing JuiceTrickInput allocation rather than creating another
per-Actor global. Explicit platform methods provide file result, initialization,
debug identity, connection outcome and sleep. Preserve constructor/start order
and false initialization/connection outcomes. A successful transport retains the
socket and returns typed HandshakePending before claiming Start complete or
publishing ready. Disabled or genuinely failed first Start can complete, publish
the same remap handle through InputDeviceManager::set_external_remap, and move
that owner into existing SelectedDeviceInputRuntime. Its remap samples shared
readiness and preserves input only on the proven false branch.

SelectedPlayerStartup currently constructs JuiceTrickInput in its physical
completion method. Minimal shared change supplies/clones the boot-owned instance
there; existing ActorInputRuntimeServices, cache/map owners, queued command
delivery and SelectedPlayerTick remain the consumers. Do not replace the existing
controlled 120-frame test's remap without recording its changed hardware policy.

For first executable testing, an explicit user-selectable host configuration
override of the ORIGINAL enable setting to0 can use the recovered disabled
branch and avoid developer-server startup latency. It must be labeled an
override, applied before construction/Start, never described as corpus default,
and must not bypass constructor network initialization. Keeping original enable1
requires real supplied endpoint identity and transport; do not assume connection
failure or suppress success. No such override or network operation was performed
in this audit. Focused tests should verify lazy disabled branch, one failed
attempt plus exact sleep, no ready publication on handshake pending, foreign
remap identity rejection, and unchanged lane bits on the installed not-ready
remap. Original-data validation should explicitly assert original enable1 before
testing any separately labeled override.

## Draft implementation and connected validation

The authorized draft packet adds `juice_input_boot.rs`, `juice_input_ini.rs`,
their focused tests and `juice_input_boot_corpus.rs`. Shared changes are isolated
in `artifacts/juice-input-boot-2026-09-22.patch`. Cvar294 holds the recovered retry
reference. Its native initializer is before the selected ghost-default row;
manager output remains sorted by section/title. The integer parser is reused
without changing the original cvar parser.

One shared Arc/mutex occurrence retains configuration, source enable value,
explicit host override, initialization result, retry sample, endpoint, successful
socket and phase. All remap handles refer to that occurrence; a foreign handle
fails. It shares the existing JuiceTrickInput allocation supplied to selected
startup, whose physical constructor now clones that same allocation. The
represented constructor fields are not a claim that all Juice plugins, RPC
transport and thread services have been constructed.

The enabled successful-transport boundary remains incomplete, including Com
NotifyConnect plugin callbacks (`82969540`) and the later protocol handshake;
the `HandshakePending` label denotes this remaining connected-protocol work,
not completed callbacks. No ready state or installed remap is published there.
Unknown names, unterminated/truncated address buffers and malformed INI fields
have explicit host admission errors; no unknown original bytes become defaults.

The additional original-data queued120 mode reads the real loose INI and asserts
source enable1, then applies the explicit DisableBeforeStart option. The injected
test initialization result is false, and tests assert the service was called;
this is a controlled platform result, not actual PC network initialization. Any
endpoint, socket or sleep call is an error in that disabled fixture. The same
startup Juice/map/backend/PauseMan flows through existing device cache, command
queue, RNG and SelectedPlayerTick. The old fixture mode remains unchanged.
Hardware still supplies its existing held-Dpad-up button1; disabled Juice leaves
the original converted channel value intact instead of the fixture remap's0.75.
Both are explicitly controlled input modes, not neutral or full application boot.

Author checks: standalone rustfmt and shared-patch apply-check pass. Cargo and
original-data execution remain root-owned. Independent source/code review
approved the connected packet, including malformed key/section LF/comment
refusal and releasing the owner lock around platform calls. The platform
observer regression verifies readiness can be read during those calls.
The subsequent authorized Windows adapter must supply actual WSA initialization
and retain its cleanup lifetime with the session/socket. No development network
connection is authorized or needed for these checks.

Root execution: Action47-focus-3 fmt/Clippy31.85s,4710units+6docs pass.
Earlier retry tests corrected INI true to numeric1 per existing ConvertAToI;
production converter unchanged. Initial original20/22 stopped at stale first
focus command count. Corrected source-backed count/RNG oracle independently
approved; original-focus-count run20/22 in6.83s completesframes0..9 under BOTH
remap policies, next Action49 atframe10. No platform socket attempted.
