---
name: release
description: Release a new version of the Qudelix widgets (PC .exe + phone .apk) as a signed GitHub release that the installed widgets self-update from. Use when the user asks to release, ship, publish or cut a version, with or without a version number.
---

# Release

One command does everything: `tools/release.sh X.Y.Z` — checks, builds and tests
both widgets, signs them, tags, pushes, creates the GitHub release, then
publishes the tailnet build for the phone and redeploys this PC.

## Steps

1. **Pick the version.** `cat VERSION` is the current one. Unless the user named
   a version, bump the patch (1.6.30 → 1.6.31). Bump the minor for a new
   feature, and say in one line which number you picked.
2. **Clean tree on main, in sync with origin** — the script refuses otherwise.
   Commit the pending work first (ask the user if it is unclear what belongs in
   this release), then `git push origin HEAD`.
3. **Run it:** `tools/release.sh X.Y.Z` with a timeout of at least 600000 ms
   (builds both widgets and runs every test). Never pass a version that already
   has a release.
4. **Report** the release URL and the final self-test line. Point out that the
   other PCs and the phone update themselves (the PC checks ~30 s after start,
   then every 6 h).

## Notes

- Release notes are generated from the commit subjects since the last release,
  so write commit subjects that read as changelog lines.
- The script bumps and commits `VERSION` itself — do not edit it by hand.
- It needs the Ed25519 release key (`~/.config/qudelix-companion/release-ed25519.pem`
  or `secrets/`) and a logged-in `gh`. Without the key, no release can ever
  update existing installs — never generate a new one to "fix" a missing key.
- Safe to re-run after a failure: it restores `VERSION` on error and skips the
  tag if it already points at HEAD. If it failed *after* `gh release create`,
  fix the release on GitHub instead of re-running with the same version.
- Step 3 ends with the phone build and `windows/deploy.sh`; a failing deploy
  self-test ("5K off or busy") does not invalidate the release.
