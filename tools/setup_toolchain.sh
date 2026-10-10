#!/usr/bin/env bash
# Sets up a self-contained Android build toolchain under $SDK (no sudo, no apt).
# JDK 17 (Temurin) + Android cmdline-tools + platform-34 + build-tools 34.0.0.
set -euo pipefail
SDK="${SDK:-$HOME/android-sdk}"
mkdir -p "$SDK/dl"
cd "$SDK/dl"

echo "[1/5] JDK 17 (Temurin)…"
if [ ! -x "$SDK/jdk/bin/javac" ]; then
  curl -sL --retry 3 -o jdk.tgz \
    "https://api.adoptium.net/v3/binary/latest/17/ga/linux/x64/jdk/hotspot/normal/eclipse"
  rm -rf "$SDK/jdk" && mkdir -p "$SDK/jdk"
  tar xzf jdk.tgz -C "$SDK/jdk" --strip-components=1
fi
export JAVA_HOME="$SDK/jdk"
export PATH="$JAVA_HOME/bin:$PATH"
java -version 2>&1 | head -1

echo "[2/5] Android cmdline-tools…"
if [ ! -x "$SDK/cmdline-tools/latest/bin/sdkmanager" ]; then
  curl -sL --retry 3 -o cmdtools.zip \
    "https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip"
  python3 -I -c "import zipfile; zipfile.ZipFile('cmdtools.zip').extractall('.')"
  rm -rf "$SDK/cmdline-tools/latest"
  mkdir -p "$SDK/cmdline-tools/latest"
  mv cmdline-tools/* "$SDK/cmdline-tools/latest/"
fi

SM="$SDK/cmdline-tools/latest/bin/sdkmanager"
echo "[3/5] Accepting licenses…"
yes 2>/dev/null | "$SM" --sdk_root="$SDK" --licenses >/dev/null 2>&1 || true

echo "[4/5] platform-tools, platforms;android-34, build-tools;34.0.0…"
"$SM" --sdk_root="$SDK" "platform-tools" "platforms;android-34" "build-tools;34.0.0" 2>&1 | tail -3

echo "[5/5] Verify…"
ls "$SDK/build-tools/34.0.0/"{aapt2,d8,zipalign,apksigner} 2>&1
ls "$SDK/platforms/android-34/android.jar" 2>&1
echo "DONE toolchain ready at $SDK"
