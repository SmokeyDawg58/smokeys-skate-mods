# Test7 public lobby server configuration

The Test7 distribution copied an empty `server` value in `online-config.json`
from Test6, but this executable was compiled without `SKATE2_DEFAULT_SERVER`.
`ui::online_config::DEFAULT_SERVER` therefore selects `127.0.0.1:32082`.
The executable contains that loopback literal and does not contain the public
server address. An empty setting is valid syntax but reaches the local machine
rather than the public lobby service for this build.

The installed configuration and distribution generator now explicitly use
`172.237.109.212:32082`, the address used by the earlier successful Test4 public
sign-in check. The installed nickname and lobby chat preference are preserved.
This correction changes no Rust source, executable, wire protocol, authentication
checks, or physics settings. The user's current physics settings are left as
they selected them; the diagnostic enables full Skate3 physics only in its
scratch save root.

The same Test7 executable was tested through the actual Online menu and Lobby
Browser using a copied profile, separate temporary nickname, scratch
`SKATE2_SAVE_ROOT`, and nonactivating offscreen diagnostic window. Its online
trace confirms successful public sign-in, selection of Lobby Browser, and
`lobby browser lists 1 of 1 games`. It did not join that player's game or send
chat. The existing user's Test7 process was left running and untouched.

Executable SHA256 remains
`cb363a082e1aa0b790bbf81d0c3db29e49957fff73ada585c9af9864e5b3bfda`.
The configuration before correction, trace, and bounded diagnostic result are
retained outside the repository in `work/online132-test7-lobbies`. The package
validation record and ZIP include this server setting. Runtime logs and live
profile changes are not added to the distribution archive.

The live list is a point-in-time observation; lobby count and availability can
change. This check covers discovery and sign-in, not a completed online match.
