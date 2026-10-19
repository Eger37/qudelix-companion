#!/usr/bin/env bash
# Unit-test the pure-Java protocol code (DevState, Proto) on the host JDK and check the
# widget layout (tools/check_widget_layout.py) — no device/emulator.
set -euo pipefail
cd "$(dirname "$0")"
JDK="${ANDROID_SDK:-$HOME/android-sdk}/jdk/bin"
OUT=build/test
rm -rf "$OUT"; mkdir -p "$OUT"
"$JDK/javac" -encoding UTF-8 -d "$OUT" src/com/qudelixwidget/app/DevState.java src/com/qudelixwidget/app/Proto.java test/DevStateTest.java
"$JDK/java" -cp "$OUT" com.qudelixwidget.app.DevStateTest
# Widget layout: RemoteViews-allowed classes only, ids present, fits the minimum height.
python3 -I ../tools/check_widget_layout.py .
