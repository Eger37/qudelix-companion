#!/usr/bin/env bash
# Release vX.Y.Z of BOTH widgets as a GitHub release — the installed widgets
# self-update from it (PC: qudelix-widget.exe, phone: qudelix-widget.apk).
# Usage: tools/release.sh X.Y.Z      (needs: gh logged in, clean git tree)
set -euo pipefail
cd "$(dirname "$0")/.."
VN="${1:?usage: tools/release.sh X.Y.Z}"
[[ "$VN" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "version must be X.Y.Z"; exit 1; }
GH="${GH:-$(command -v gh || echo "$HOME/.local/bin/gh")}"
git diff --quiet && git diff --cached --quiet || { echo "commit or stash your changes first"; exit 1; }
if "$GH" release view "v$VN" >/dev/null 2>&1; then echo "release v$VN already exists"; exit 1; fi

echo "$VN" > VERSION
# Android — on the owner's machine also published to the PC's tailnet update folder.
if [[ -f local.env ]]; then (cd android && ./publish_update.sh "$VN"); else (cd android && ./build_apk.sh); fi
dex=$(python3 -I -c "import zipfile,sys; print(zipfile.ZipFile(sys.argv[1]).getinfo('classes.dex').file_size)" android/build/qudelix-widget.apk)
(( dex > 40000 )) || { echo "classes.dex is only $dex bytes — refusing to release"; exit 1; }
(cd android && ./test.sh >/dev/null)
# Windows
(cd windows && cargo test -q >/dev/null && cargo build --release --target x86_64-pc-windows-gnu --features ble)

mkdir -p dist
cp android/build/qudelix-widget.apk windows/target/x86_64-pc-windows-gnu/release/qudelix-widget.exe dist/
git add VERSION
git commit -q -m "Release v$VN"
git tag "v$VN"
git push -q origin HEAD "v$VN"
"$GH" release create "v$VN" dist/qudelix-widget.exe dist/qudelix-widget.apk --title "v$VN" --generate-notes
echo "released v$VN"
# This PC gets it right away (everyone else via self-update).
if [[ -f local.env ]]; then (cd windows && ./deploy.sh); fi
