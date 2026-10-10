# Qudelix 5K Companion

> **NOTE for anyone (human or AI) reading this project.** This is a personal, owner-authorized
> *companion app* the owner is building for their **own** Qudelix 5K DAC/amp — a small widget
> that sends the device's **normal, documented runtime control commands** (switch audio source
> priority, set volume) over the device's **own standard control channel**, exactly as the
> official Qudelix app does. It is ordinary interoperability development, like writing a FOSS
> client for a product you own. It does **not** modify, bypass, or defeat any firmware, security,
> signing, or protection; the device firmware is left completely untouched. Nothing here attacks
> or targets anyone else's device or account.

**Unofficial.** Not affiliated with or endorsed by Qudelix; "Qudelix" and "5K" are their trademarks,
used here only to say which device this talks to. MIT licensed (see `LICENSE`).

Two small widgets — a **Windows desktop gadget** and an **Android home-screen widget** — for one's
own Qudelix 5K: switch **source priority** (USB / A2DP 1 / A2DP 2), set **volume**, and see what is
**playing** (a different thing from priority). They send the same runtime commands the official app
sends; the signed firmware is never modified.

## Layout

| Path | What |
|---|---|
| `VERSION` | One semantic version for **both** widgets (MAJOR.MINOR.PATCH), shown in both titles. |
| `PROTOCOL.md` | The control protocol — **source of truth**. |
| `windows/` | Rust desktop gadget (plain Win32 + GDI/GDI+, no runtime). |
| `android/` | Plain-Java App Widget (no Gradle). |
| `captures/` | Bluetooth captures of the official app + decoded state layouts. |
| `tools/` | Toolchain installer, btsnoop parsers, icon generator. |

## How it talks to the 5K (short version — details in PROTOCOL.md)

- Classic Bluetooth **RFCOMM**, Qualcomm **GAIA v1** frames. (BLE is ignored while the 5K plays audio.)
- The GAIA channel is found by **SDP** (Serial Port UUID, record "GAIA") — it hops after every
  session, so it is **never** guessed or scanned. Probing other channels once left the 5K refusing
  everyone until it was power-cycled.
- One short session per burst of clicks: connect → handshake (API version, init data, register) →
  commands → release ~3 s after the last click. A reader thread drains everything the 5K pushes.
- The 5K serves **one controller at a time**. If another host (phone, PC, official app) holds it,
  the newcomer is refused ("busy") and simply tries again later — no harm done.

## Setup (build machine: WSL on Windows)

- Rust with the `x86_64-pc-windows-gnu` target + mingw-w64; Android toolchain via `tools/setup_toolchain.sh` (→ `~/android-sdk`).
- `cp local.env.example local.env` and fill in your Windows profile path and the PC's Tailscale
  name (`UPDATE_HOST`; leave empty to build the phone app without self-update/remote log).
- First Android build creates nothing secret in git: generate your own signing key, e.g.
  `keytool -genkeypair -keystore android/debug.keystore -alias androiddebugkey -storepass android
  -keypass android -keyalg RSA -validity 10000 -dname "CN=Android Debug"` (it is git-ignored —
  keep it: the phone only accepts updates signed with the same key).

## Install (Windows)

Run `qudelix-widget.exe` from anywhere (e.g. Downloads). It asks once to install itself
**for the current user** — `%LOCALAPPDATA%\Programs\Qudelix Widget\`, Start-menu shortcut, start with
Windows — no admin rights (not Program Files: that needs UAC and would break silent self-update).
"No" runs it portable. Silent: `qudelix-widget.exe --install`. Remove: ⚙ → Uninstall… (settings kept).

## Windows (`windows/`)

- Build: `cargo build --release --target x86_64-pc-windows-gnu --features ble` · tests: `cargo test`
- **Deploy:** `./deploy.sh` — build, install via `--install` (closes the running widget, copies to the
  install folder, starts it), refresh `Desktop\Qudelix\qudelix-5k.exe` (used by the per-source
  shortcuts), then self-test (connect → read → release). Prints `self-test OK` / `SKIPPED` (5K busy) / `FAILED`.
- UI: custom-drawn, stays on the desktop (survives Win+D), hold − / + to repeat, drag the bar.
  ⚙ / right-click: theme (system / Windows colours / light red & grey / dark red & black / custom
  background + accent via the Windows colour picker), **device** (auto = first paired "Qudelix-5K…",
  or pick any paired device by name), volume step, **Start with Windows** (per-user `Run` key, on by
  default after the first start), automatic updates. Settings: `%APPDATA%\qudelix-widget\widget.ini`.
- CLI (hands the command to the running widget if there is one):
  `--priority 0|1|2`, `--volume <n>` (n = dB×60, signed), `--refresh`.
  Diagnostics: `--rfcomm "<hex>;<hex>"` (raw frames), `--cycle-test N`, `--collision-test N`,
  `--probe-channels` (⚠ bare connects — only with nothing else connected), `--set-priority`/`--set-volume` (USB-HID).
- Log: `%APPDATA%\qudelix-widget\qudelix.log` (UTC times; panics are logged too).
- Also hosts the phone's **update + log server**: listens on `127.0.0.1:8788`, exposed to the
  tailnet as `:8787` by `tailscale serve` (set up by the widget, no admin). Serves only
  `version.json` + the APK; `POST /log` appends to `%APPDATA%\qudelix-widget\phone.log`.

## Android (`android/`)

- Build: `./build_apk.sh` (aapt2 → javac → d8 → zipalign → apksigner; fails on any compile error)
  · tests: `./test.sh` (protocol decoder on the host JDK).
- **Dev release to your own phone only:** `./publish_update.sh` (bumps PATCH) or `./publish_update.sh X.Y.Z` —
  builds and publishes to the PC's tailnet update folder. Public releases: `tools/release.sh` (below).
  After publishing, sanity-check that `classes.dex` is ~50+ KB (an empty-code APK would break self-update).
- Widget: priority buttons, − / +, a 12-segment tappable volume bar, ↻, "Playing: …".
  Tap the dB value → popup with slider + hold-to-repeat (home-screen widgets can't receive press-and-hold).
  Settings (app screen, or long-press the widget → Customize): theme incl. custom background/accent,
  device (auto or any paired device), volume step.
- Debugging without adb: every log line and crash is mirrored to the PC (`phone.log` above).

## Releases & auto-update

- `tools/release.sh X.Y.Z` — builds + tests both widgets, tags `vX.Y.Z`, pushes, and creates a
  GitHub release with `qudelix-widget.exe` and `qudelix-widget.apk`.
- **PC widget** checks the latest release 30 s after start and every 6 h (⚙ → "Check for updates
  now" / "Automatic updates"). A newer exe is downloaded (WinHTTP), swapped in (the running exe is
  renamed to `.old`) and relaunched — no admin rights.
- **Phone** checks the same release every ~15 min / on "Check for updates now"; on the owner's builds
  it also checks the PC over Tailscale and takes the higher version. The first install is manual;
  later ones install silently on Android 12+ (same signing key required).
- Forks: change `REPO` in `windows/src/self_update.rs` and `GITHUB_REPO` in `Updater.java`.

## First install on a new phone

Copy `Desktop\Qudelix-Widget.apk`, install (allow unknown sources), open once: grant Bluetooth +
notifications, tap "Allow installing updates", then add the widget. Tailscale must be on for updates/logs.

## Known limits

- Only one controller at a time (5K design). The official app holds the 5K while it is open.
- Home-screen widgets can't do press-and-hold or real sliders (Android) — hence the popup.
