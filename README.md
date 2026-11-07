<p align="center">
  <img src="assets/icon.svg" alt="hyperb" width="128">
</p>

<h1 align="center">hyperb</h1>

# What is it, and why should I care?

**hyperb** is a browser for small server-driven applications that put content first. Most apps don't need flashy gradients, rounded corners, or piles of hard-to-maintain client-side JavaScript, and few are worth a dedicated native app. With **hyperb**, the server sends a basic layout of components along with their data. A visually neutral client renders it, collects the user's input, and sends it back to the server.

It's ideal for server admin dashboards, remote controls for hardware devices, interfacing with a sandboxed WASM module, and for simple form-driven apps where content matters more than appearance: a food ordering service, a club enrollment status page, or a shipping monitor.

## How it works

**hyperb** opens `hyperb://` addresses, which can be served over HTTP or iroh, or can point to (local or HTTP) paths to WASM modules.

Applications are servers that send `metadata`, `element`, or `signal` messages. These modify the basic layout and local state of the components the browser renders. For WASM modules, the server role is played by the sandboxed program running locally.

[SPEC.md](SPEC.md) is a thin layer on top of the [hypereffect](https://hypereffect.dev) protocol. There is no client-side computation, and the browser defines one homogeneous style. The server provides only the layout skeleton and the basic behavior of the elements, and the client's only way to communicate with the server is by sending `action` messages. In the case of WASM modules they are executed locally inside the **hyperb** app, but play the role of a server.

# Demo apps
The demo apps are WASM modules (`crates/demoapps`), hosted at `hyperb://hypereffect.github.io/hyperb/apps/<app>.wasm`. They are demos with fake data, and because they don't interact with real servers, but only with a WASM sandbox, they don't represent the true powers of **hyperb**, only a small part of it.

The links below open the demo apps in the web renderer instance, but the same URLs can be open in the desktop, Android or TUI renderers.

* [`pomodoro`](https://hypereffect.github.io/hyperb/?link=hyperb://hypereffect.github.io/hyperb/apps/pomodoro.wasm): a focus timer
* [`chart`](https://hypereffect.github.io/hyperb/?link=hyperb://hypereffect.github.io/hyperb/apps/chart.wasm): generate charts from pasted data
* [`grammar`](https://hypereffect.github.io/hyperb/?link=hyperb://hypereffect.github.io/hyperb/apps/grammar.wasm): a grammar checker, using Harper
* [`restaurant`](https://hypereffect.github.io/hyperb/?link=hyperb://hypereffect.github.io/hyperb/apps/restaurant.wasm): reserve a table
* [`dbclient`](https://hypereffect.github.io/hyperb/?link=hyperb://hypereffect.github.io/hyperb/apps/dbclient.wasm): a SQL client over fake tables
* [`minecraft`](https://hypereffect.github.io/hyperb/?link=hyperb://hypereffect.github.io/hyperb/apps/minecraft.wasm): a fake Minecraft server's admin console
* [`saas`](https://hypereffect.github.io/hyperb/?link=hyperb://hypereffect.github.io/hyperb/apps/saas.wasm): a fake product's admin panel
* [`smarthome`](https://hypereffect.github.io/hyperb/?link=hyperb://hypereffect.github.io/hyperb/apps/smarthome.wasm): a simulated house

# Installation
Grab the files from the [latest release](https://github.com/hypereffect/hyperb/releases/latest), or use the commands below, which all download the latest one.

<details>
<summary>Windows</summary>

**Manually**: download the `windows-x64-setup-` `.exe` from the [releases page](https://github.com/hypereffect/hyperb/releases) and run it. It adds a Start menu entry and an uninstaller and registers the `hyperb://` and `web+hyperb://` links.

**One-liner** (PowerShell), does the same: downloads the installer and runs it.
```
$u=(irm https://api.github.com/repos/hypereffect/hyperb/releases/latest).assets.browser_download_url -match 'windows-x64-setup-v'; iwr $u -OutFile $env:TEMP\hyperb-setup.exe; Start-Process $env:TEMP\hyperb-setup.exe
```

**No installer** (PowerShell): downloads the `.zip`, unzips it to `%LOCALAPPDATA%\hyperb` and you start it with `hyperb.bat`. No Start menu entry and no `hyperb://` links.
```
$u=((irm https://api.github.com/repos/hypereffect/hyperb/releases/latest).assets.browser_download_url -match 'windows-x64-v.*\.zip$')[0]; iwr $u -OutFile $env:TEMP\hyperb.zip; Expand-Archive -Force $env:TEMP\hyperb.zip $env:LOCALAPPDATA\hyperb; & $env:LOCALAPPDATA\hyperb\hyperb.bat
```
</details>

<details>
<summary>Linux</summary>

**Flatpak** (any distro), `.flatpak`, installs it for your user:
```
curl -fL "$(curl -s https://api.github.com/repos/hypereffect/hyperb/releases/latest | grep -o 'https://[^"]*linux-x86_64[^"]*\.flatpak')" -o /tmp/hyperb.flatpak && flatpak install --user -y /tmp/hyperb.flatpak
```

**Debian / Ubuntu** (Debian 13+, Ubuntu 24.04+), `.deb`:
```
curl -fL "$(curl -s https://api.github.com/repos/hypereffect/hyperb/releases/latest | grep -o 'https://[^"]*linux-amd64[^"]*\.deb')" -o /tmp/hyperb.deb && sudo apt install /tmp/hyperb.deb
```

**Fedora and other RPM distros**, `.rpm` (needs GTK 4.12+):
```
curl -fL "$(curl -s https://api.github.com/repos/hypereffect/hyperb/releases/latest | grep -o 'https://[^"]*linux-x86_64[^"]*\.rpm')" -o /tmp/hyperb.rpm && sudo dnf install /tmp/hyperb.rpm
```

**AppImage**, saves it to `~/.local/bin` and runs it:
```
mkdir -p ~/.local/bin && curl -fL "$(curl -s https://api.github.com/repos/hypereffect/hyperb/releases/latest | grep -o 'https://[^"]*linux-x86_64[^"]*\.AppImage')" -o ~/.local/bin/hyperb.AppImage && chmod +x ~/.local/bin/hyperb.AppImage && ~/.local/bin/hyperb.AppImage
```
It needs FUSE (otherwise run it with `--appimage-extract-and-run`).

**Plain binary** (`.tar.gz`), for when you already have GTK 4.12+ installed:
```
mkdir -p ~/.local/bin && curl -fL "$(curl -s https://api.github.com/repos/hypereffect/hyperb/releases/latest | grep -o 'https://[^"]*linux-x86_64[^"]*\.tar\.gz')" | tar -xz -C ~/.local/bin hyperb-desktop && ~/.local/bin/hyperb-desktop
```

**From source** (see [Building from source](#building-from-source) for the dependencies). This installs the desktop app for your user, with its icon, and makes it the handler of `hyperb://` and `web+hyperb://` links:
```
git clone https://github.com/hypereffect/hyperb && cd hyperb && just desktop-install
```
</details>

<details>
<summary>macOS</summary>

Apple Silicon only, and the app is unsigned.

**Homebrew**, installs from the [tap](https://github.com/andunieee/homebrew-hyperb) and clears the quarantine flag for you:
```
brew install --cask andunieee/hyperb/hyperb
```

**Manually**: download the `macos-arm64` `.zip`, double-click it in Finder to unzip, and drag `hyperb.app` to `Applications`. On first launch right-click it and choose Open.

**One-liner** (Terminal), downloads and unzips it into `/Applications`. `ditto` is the archive tool that comes with macOS, and it keeps the app bundle intact where a plain `unzip` can break it:
```
curl -fL "$(curl -s https://api.github.com/repos/hypereffect/hyperb/releases/latest | grep -o 'https://[^"]*macos-arm64[^"]*\.zip')" -o /tmp/hyperb.zip && ditto -x -k /tmp/hyperb.zip /Applications
```

If macOS refuses to open it because it is from an unidentified developer, run:
```
xattr -d com.apple.quarantine /Applications/hyperb.app
```
</details>

<details>
<summary>Android</summary>

Builds from source and installs on every device connected through adb (see [Building from source](#building-from-source)):
```
just android
```
</details>

<details>
<summary>Web</summary>

Nothing to install: open <https://hypereffect.github.io/hyperb/>.
</details>

### Building locally

`just` lists the recipes. The main ones:

```
just desktop        # the GTK4 app
just tui            # the terminal app
just cli --help     # hypb: print an app's messages or state, fire actions, run JS scripts against it
just demo pomodoro  # the desktop app on a demo app
just demo-serve     # the demo apps over HTTP on 127.0.0.1:9001, for the web app
just web            # the web app with hot reload on http://127.0.0.1:3000
```
