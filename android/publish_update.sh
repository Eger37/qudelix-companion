#!/usr/bin/env bash
# Bump versionCode, build, and publish the APK to the PC's update folder. The
# Windows widget serves that folder to the tailnet on port 8787
# (windows/src/update_server.rs); the phone checks it every ~15 min (and on
# "Check for update now") and installs newer builds.
set -euo pipefail
cd "$(dirname "$0")"

source ../local.env  # WIN_HOME, UPDATE_HOST (see ../local.env.example)
UPDATES="$WIN_HOME/AppData/Roaming/qudelix-widget/updates"

# Bump PATCH of the shared semantic version (../VERSION); bump MINOR/MAJOR by hand.
# Usage: ./publish_update.sh            → bump PATCH
#        ./publish_update.sh 1.2.0      → set an explicit version (MINOR/MAJOR bumps)
if [[ $# -ge 1 ]]; then
  VN="$1"
else
  IFS=. read -r V_MAJ V_MIN V_PAT <<< "$(tr -d "[:space:]" < ../VERSION)"
  VN="$V_MAJ.$V_MIN.$(( V_PAT + 1 ))"
fi
echo "$VN" > ../VERSION
./build_apk.sh
IFS=. read -r V_MAJ V_MIN V_PAT <<< "$VN"
VC=$(( V_MAJ * 1000000 + V_MIN * 1000 + V_PAT ))

mkdir -p "$UPDATES"
cp build/qudelix-widget.apk "$UPDATES/qudelix-widget.apk.tmp"
mv "$UPDATES/qudelix-widget.apk.tmp" "$UPDATES/qudelix-widget.apk"
printf '{"versionCode": %s, "versionName": "%s", "apk": "qudelix-widget.apk"}\n' "$VC" "$VN" > "$UPDATES/version.json"
cp build/qudelix-widget.apk "$WIN_HOME/Desktop/Qudelix-Widget.apk" 2>/dev/null || true   # handy for a first install

echo "published v$VN (code $VC) → http://$UPDATE_HOST:8787/version.json"
echo "NOTE: rebuild/redeploy the Windows widget (windows/deploy.sh) so it shows v$VN too."
