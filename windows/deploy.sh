#!/usr/bin/env bash
# Build the Windows widget, replace the running copy on the Desktop, restart it,
# and self-test the 5K control path (Refresh: connect → read → release).
set -euo pipefail
cd "$(dirname "$0")"

source ../local.env  # WIN_HOME, WIN_HOME_WIN (see ../local.env.example)
DESK="$WIN_HOME/Desktop"
LOG="$WIN_HOME/AppData/Roaming/qudelix-widget/qudelix.log"

cargo build --release --target x86_64-pc-windows-gnu --features ble 2>&1 | grep -E "^error|Finished"
EXE=target/x86_64-pc-windows-gnu/release/qudelix-widget.exe

# Deploy like a user install: the built-in installer (`--install`) closes the
# running widget, copies the exe to %LOCALAPPDATA%\Programs\Qudelix Widget, makes
# the Start-menu shortcut + autostart and starts the installed copy.
cp "$EXE" "$DESK/Qudelix/qudelix-5k.exe"   # the 5K → USB/BT shortcuts run this copy
STAGE="$WIN_HOME/AppData/Roaming/qudelix-widget/stage.exe"
cp "$EXE" "$STAGE"
N0=$(wc -l < "$LOG")
# Via `cmd start`: launched directly, Windows would let the installed widget inherit
# WSL's pipe handles and this script would wait for the widget to exit.
(cd /mnt/c && timeout 10 cmd.exe /c start "" "$WIN_HOME_WIN\\AppData\\Roaming\\qudelix-widget\\stage.exe" --install >/dev/null 2>&1) || true
for _ in $(seq 1 30); do tail -n +$((N0 + 1)) "$LOG" | grep -q "\[install\] --install" && break; sleep 1; done
tail -n +$((N0 + 1)) "$LOG" | grep "\[install\] --install" || { echo "install did not finish"; exit 1; }
rm -f "$DESK/Qudelix-Widget.exe"            # pre-installer location (migrated)
# (Not `tasklist | grep -q`: with pipefail, grep's early exit SIGPIPEs tasklist → false negative.)
running() { local t; t="$(cd /mnt/c && tasklist.exe)"; [[ "${t,,}" == *qudelix-widget.exe* ]]; }
for _ in 1 2 3 4 5; do running && break; sleep 2; done
running || { echo "widget did not start"; exit 1; }
sleep 2

N=$(wc -l < "$LOG")
"$DESK/Qudelix/qudelix-5k.exe" --refresh
sleep 6
SINCE="$(tail -n +$((N + 1)) "$LOG")"
if [[ "$SINCE" == *"session open (registered)"* ]]; then
  echo "self-test OK: v$(tr -d '[:space:]' < ../VERSION) running, 5K control works"
elif [[ "$SINCE" == *"open failed: busy"* ]]; then
  echo "self-test SKIPPED: v$(tr -d '[:space:]' < ../VERSION) running, but the 5K reports its control session held elsewhere (close phone app or power-cycle the 5K)"
else
  echo "self-test FAILED — log since start:"; tail -n +$((N + 1)) "$LOG" | grep -v " RX "
  exit 1
fi
