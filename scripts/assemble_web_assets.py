#!/usr/bin/env python3
"""Copy Trunk's dist/web into the Android APK and inject the Android bridge.

Run from the repository root after:
    cd apps/photocraft-web && trunk build --release
"""

from __future__ import annotations

import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DIST = ROOT / "dist" / "web"
ASSETS = ROOT / "android" / "app" / "src" / "main" / "assets" / "www"
BRIDGE = ROOT / "bridge" / "android-bridge.js"


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    if not DIST.is_dir():
        fail(f"{DIST} is missing; run `trunk build --release` in apps/photocraft-web first")
    index = DIST / "index.html"
    if not index.is_file():
        fail(f"{index} is missing")
    if not BRIDGE.is_file():
        fail(f"{BRIDGE} is missing")

    wasm = list(DIST.glob("*_bg.wasm"))
    if not wasm:
        fail("no *_bg.wasm in dist/web; the Rust build did not produce a module")

    if ASSETS.exists():
        shutil.rmtree(ASSETS)
    ASSETS.mkdir(parents=True)
    shutil.copytree(DIST, ASSETS, dirs_exist_ok=True)
    shutil.copy2(BRIDGE, ASSETS / "android-bridge.js")

    html = (ASSETS / "index.html").read_text(encoding="utf-8")
    if "android-bridge.js" not in html:
        tag = '<script src="./android-bridge.js"></script>'
        if "</body>" in html:
            html = html.replace("</body>", f"  {tag}\n</body>", 1)
        else:
            html += f"\n{tag}\n"
        (ASSETS / "index.html").write_text(html, encoding="utf-8")
    else:
        print("note: index.html already references the Android bridge")

    if 'src="/' in html or 'href="/' in html:
        fail("index.html contains root-absolute URLs; WebViewAssetLoader needs relative paths")

    total = sum(p.stat().st_size for p in ASSETS.rglob("*") if p.is_file())
    print(f"web assets -> {ASSETS}")
    print(f"  wasm: {wasm[0].name} ({wasm[0].stat().st_size / 1048576:.1f} MiB)")
    print(f"  total: {total / 1048576:.1f} MiB in {sum(1 for p in ASSETS.rglob('*') if p.is_file())} files")


if __name__ == "__main__":
    main()
