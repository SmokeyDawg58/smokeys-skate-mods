# Linux

The game and the dedicated server both build for 64-bit Linux. A release's Linux zip holds
both: `skate2-pc` (the game, which also hosts with `--server`) and `skate2-server` (the server
alone). Built from the same `release-secret.txt` and version as the Windows game, they play
together with it on one server ([auto-update.md](auto-update.md#the-release-key-and-the-build-secret)),
and each updates itself from the Linux zip.

## Playing

**The game needs:**

- An x86-64 processor with AVX2 (x86-64-v3), as the Windows build does.
- A Vulkan driver for a GPU with BC texture compression (Mesa's RADV or ANV, NVIDIA's driver).
- glibc 2.35 or newer (the build is made on Ubuntu 22.04), and ALSA's `libasound.so.2`, which
  PipeWire and PulseAudio systems have.
- An X11 or Wayland session; where `WAYLAND_DISPLAY` names a socket that does not exist, the
  window opens on X11. The first start's file picker uses the desktop portal
  (`xdg-desktop-portal`), and its messages `zenity`.

Unzip the Linux zip into a folder of your own and run `./skate2-pc` from that folder: settings and
saves stay in the folder the game runs from (`config.json`, `online-config.json`, `saves/`). The
first start asks for your copy's `.xex`, as on Windows. Where no picker can open, write the
game's folder into `config.json` as `"game_root"`; the game prints where that file is.

**Controllers:** SDL3 reads every pad: Xbox, PlayStation, Switch and others. An Xbox pad gives
XInput's values; a PlayStation pad gives what DS4Windows gave on Windows.

**File names:** the original game names its files in any case (the Xbox 360's file system
ignores it). Where a file the game asks for is missing, the Linux build looks it up ignoring
ASCII case (`skate2_formats::host_path`), so a copy of the game with differently cased names
plays.

**Steam Deck:** in Desktop Mode, unzip the release, run `skate2-pc` once from its folder to pick
the `.xex`, then add it to Steam as a non-Steam game; Steam starts it in its own folder. In Game
Mode Steam hands the Deck's controls to the game as an Xbox pad.

**Proton or Wine** run the Windows game as well; it then updates from the Windows zip.

## Hosting

`./skate2-server [ip:port]` needs only glibc, and runs on any x86-64 processor: it is the one
for a VPS. `./skate2-pc --server [ip:port]` runs the same server from the game, which needs
`libasound.so.2` (`apt install libasound2`, `libasound2t64` on Ubuntu 24.04 and later) and an
AVX2 processor even when it only hosts. Stop the server and run `--update` on either to put the
newest release in place.

## Building

On Ubuntu 22.04 (native, or in WSL on Windows):

```sh
sudo apt install build-essential pkg-config cmake libasound2-dev libudev-dev curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
./build-release.sh            # dist/linux/skate2-pc and dist/linux/skate2-server
./build-release.sh server     # the server only
```

SDL3 builds from source with CMake. It reads only the pads, so it builds as a console build
(`build-from-source-unix-console`) with no X11 or Wayland development files; `libudev-dev` gives
it udev, which it loads when it runs. SDL leaves SIGINT and SIGTERM to the game
(`SDL_NO_SIGNAL_HANDLERS`), so Ctrl+C, `kill` and a launcher's stop end it.

The builds go to `~/.cache/skate2-recomp` (or `CARGO_TARGET_DIR`), off a Windows drive and away
from the Windows build's `target` folder. The game builds at x86-64-v3 like the Windows game
(`.cargo/config.toml`), the server for any x86-64 processor. With `release-secret.txt` beside
`Cargo.toml` they are release builds ([auto-update.md](auto-update.md)). The copies in
`dist/linux` keep their symbols but not the release profile's line tables, which are three
quarters of the game's file; the builds in the target folder keep them.

**WSL:** `wsl --install Ubuntu-22.04 --location <folder>` puts the distribution where it has
room. The game's crates need several GB each to compile, more than WSL's default memory limit
on a 16 GB PC: raise it in `%UserProfile%\.wslconfig`:

```ini
[wsl2]
memory=12GB
swap=8GB
```

then `wsl --shutdown`. Inside it, install as above; `build-release-linux.bat` then runs
`build-release.sh` in WSL from Windows and leaves the programs in `dist\linux` for
`package-release.bat`.
