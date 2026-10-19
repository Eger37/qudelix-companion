#!/usr/bin/env python3
"""Static checks of the Android home-screen widget layout (run by android/test.sh).

Catches what broke the widget before, without a phone:
  * a view class RemoteViews refuses (a plain <View> → launcher shows "Can't load widget");
  * an R.id the widget code uses but the layout lacks (crash when building the views);
  * content taller than the widget's minimum height (rows squeezed / cut off).

Usage: check_widget_layout.py <android dir>
"""
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

A = "{http://schemas.android.com/apk/res/android}"

# Classes RemoteViews can inflate (android.widget.RemoteViews docs; API 31 adds the
# compound buttons). Anything else, including plain View, fails the whole widget.
ALLOWED = {
    "FrameLayout", "LinearLayout", "RelativeLayout", "GridLayout",
    "AnalogClock", "Button", "Chronometer", "ImageButton", "ImageView", "ProgressBar",
    "TextView", "ViewFlipper", "ListView", "GridView", "StackView", "AdapterViewFlipper",
    "ViewStub", "CheckBox", "Switch", "RadioButton", "RadioGroup",
}

failures = []


def fail(msg):
    failures.append(msg)
    print("FAIL  " + msg)


def ok(msg):
    print("ok    " + msg)


def dp(value):
    """'14dp' / '11sp' → number, else None."""
    m = re.fullmatch(r"([0-9.]+)(dp|sp)", value or "")
    return float(m.group(1)) if m else None


def min_height(el):
    """Smallest height (dp) this view can take: fixed dp, else minHeight for weighted
    (0dp) views, else for wrap_content the tallest child / one line of text."""
    h = el.get(A + "layout_height")
    if dp(h) is not None and dp(h) > 0:
        return dp(h)
    if h == "0dp":
        return dp(el.get(A + "minHeight")) or 0.0
    kids = list(el)
    if kids:
        pad = 2 * (dp(el.get(A + "padding")) or 0.0)
        if el.get(A + "orientation") == "vertical":
            return pad + sum(min_height(k) + margins(k) for k in kids)
        return pad + max(min_height(k) + margins(k) for k in kids)
    size = dp(el.get(A + "textSize")) or dp(el.get(A + "autoSizeMaxTextSize")) or 14.0
    return size * 1.35  # one line incl. font padding


def margins(el):
    return (dp(el.get(A + "layout_marginTop")) or 0.0) + (dp(el.get(A + "layout_marginBottom")) or 0.0)


def main():
    root_dir = Path(sys.argv[1])
    layout = root_dir / "res/layout/widget.xml"
    info = root_dir / "res/xml/widget_info.xml"
    code = (root_dir / "src/com/qudelixwidget/app/QudelixWidget.java").read_text(encoding="utf-8")

    tree = ET.parse(layout).getroot()

    bad = sorted({el.tag for el in tree.iter() if el.tag not in ALLOWED})
    if bad:
        fail(f"widget.xml uses classes RemoteViews refuses: {', '.join(bad)}")
    else:
        ok("widget.xml: only RemoteViews-allowed classes")

    ids = {el.get(A + "id", "").split("/")[-1] for el in tree.iter() if el.get(A + "id")}
    used = set(re.findall(r"R\.id\.(\w+)", code))
    missing = sorted(used - ids)
    if missing:
        fail(f"QudelixWidget uses ids missing from widget.xml: {', '.join(missing)}")
    else:
        ok(f"widget.xml has all {len(used)} ids the widget code uses")

    pad = 2 * (dp(tree.get(A + "padding")) or 0.0)
    need = pad + sum(min_height(k) + margins(k) for k in tree)
    have = dp(ET.parse(info).getroot().get(A + "minResizeHeight"))
    if have is None or need > have + 0.5:
        fail(f"content needs ~{need:.0f}dp but minResizeHeight is {have}dp")
    else:
        ok(f"content fits the minimum height (~{need:.0f}dp of {have:.0f}dp)")

    print("all passed" if not failures else f"{len(failures)} failed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
