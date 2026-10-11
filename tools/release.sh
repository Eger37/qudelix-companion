#!/usr/bin/env bash
# Release vX.Y.Z of BOTH widgets as a GitHub release — the installed widgets
# self-update from it (PC: qudelix-widget.exe, phone: qudelix-widget.apk).
# Usage: tools/release.sh X.Y.Z      (needs: gh logged in, clean tree on main, in sync with origin)
# Order: check → build + test everything → tag + GitHub release → only then the
# owner's extras (tailnet build for the phone, this PC). Safe to re-run after a failure.
set -euo pipefail
cd "$(dirname "$0")/.."
VN="${1:?usage: tools/release.sh X.Y.Z}"
[[ "$VN" =~ ^(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})$ ]] || { echo "version must be X.Y.Z (0-999, no leading zeros)"; exit 1; }
GH="${GH:-$(command -v gh || echo "$HOME/.local/bin/gh")}"
"$GH" auth status >/dev/null 2>&1 || { echo "gh is missing or not logged in"; exit 1; }
[[ "$(git branch --show-current)" == main ]] || { echo "release from main only"; exit 1; }
[[ -z "$(git status --porcelain)" ]] || { echo "commit or stash your changes first (untracked files too)"; exit 1; }
git fetch -q origin
[[ "$(git rev-parse HEAD)" == "$(git rev-parse origin/main)" ]] || { echo "main is not in sync with origin/main"; exit 1; }
if "$GH" release view "v$VN" >/dev/null 2>&1; then echo "release v$VN already exists"; exit 1; fi

OLD_VERSION="$(cat VERSION)"
trap 'echo "$OLD_VERSION" > VERSION; echo "release failed — VERSION restored"' ERR
echo "$VN" > VERSION
# Build paths must not carry this machine's home directory into the exe.
export RUSTFLAGS="--remap-path-prefix=$HOME=~"

# 1. Build + test. The public APK has no PC name baked in (PUBLIC_BUILD=1).
(cd android && PUBLIC_BUILD=1 ./build_apk.sh)
dex=$(python3 -I -c "import zipfile,sys; print(zipfile.ZipFile(sys.argv[1]).getinfo('classes.dex').file_size)" android/build/qudelix-widget.apk)
(( dex > 40000 )) || { echo "classes.dex is only $dex bytes — refusing to release"; false; }
(cd android && ./test.sh)
(cd windows && cargo test --locked -q && cargo build --locked --release --target x86_64-pc-windows-gnu --features ble)
mkdir -p dist
cp android/build/qudelix-widget.apk windows/target/x86_64-pc-windows-gnu/release/qudelix-widget.exe dist/

# 2. Commit, tag, push, release.
git add VERSION
git diff --cached --quiet || git commit -q -m "Release v$VN"   # VERSION may already be bumped
if [[ "$(git rev-parse -q --verify "refs/tags/v$VN^{commit}" || true)" != "$(git rev-parse HEAD)" ]]; then
  git tag "v$VN"
fi
git push -q origin HEAD "v$VN"
"$GH" release create "v$VN" dist/qudelix-widget.exe dist/qudelix-widget.apk --title "v$VN" --generate-notes
trap - ERR
echo "released v$VN"

# 3. Owner's machine only: the tailnet build for the phone (seeds "Debug over
#    Tailscale" with this PC's name) and this PC right away.
if [[ -f local.env ]]; then
  (cd android && ./publish_update.sh "$VN")
  (cd windows && ./deploy.sh) || echo "NOTE: deploy self-test failed (5K off or busy?) — the release itself is fine"
fi
