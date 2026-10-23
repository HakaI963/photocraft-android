#!/usr/bin/env python3
"""Generate Android launcher and splash assets from PhotoCraft's upstream icon.

The source is `assets/app-icon/hicolor/512x512/apps/ai.storyteller.photocraft.png`
(MIT/Apache-2.0 PhotoCraft artwork; see ATTRIBUTION.md). Keeping the generator in the
repository makes the icon set reproducible after an upstream sync.
"""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "assets/app-icon/hicolor/512x512/apps/ai.storyteller.photocraft.png"
RES = ROOT / "android/app/src/main/res"

DENSITIES = {
    "mdpi": 48,
    "hdpi": 72,
    "xhdpi": 96,
    "xxhdpi": 144,
    "xxxhdpi": 192,
}


def resized(image: Image.Image, size: int) -> Image.Image:
    return image.resize((size, size), Image.LANCZOS)


def circular(image: Image.Image) -> Image.Image:
    size = image.size[0]
    mask = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(mask)
    draw.ellipse((0, 0, size - 1, size - 1), fill=255)
    out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    out.paste(image, (0, 0), mask)
    return out


def main() -> None:
    source = Image.open(SOURCE).convert("RGBA")

    for density, size in DENSITIES.items():
        folder = RES / f"mipmap-{density}"
        folder.mkdir(parents=True, exist_ok=True)
        icon = resized(source, size)
        icon.save(folder / "ic_launcher.png", optimize=True)
        circular(icon).save(folder / "ic_launcher_round.png", optimize=True)

    # Adaptive icon: 108dp canvas with the artwork inside the 72dp safe zone.
    foreground_size = 432  # 108dp at xxxhdpi
    artwork_size = 288  # 72dp safe zone
    foreground = Image.new("RGBA", (foreground_size, foreground_size), (0, 0, 0, 0))
    art = resized(source, artwork_size)
    offset = (foreground_size - artwork_size) // 2
    foreground.paste(art, (offset, offset), art)
    foreground_dir = RES / "drawable-xxxhdpi"
    foreground_dir.mkdir(parents=True, exist_ok=True)
    foreground.save(foreground_dir / "ic_launcher_foreground.png", optimize=True)

    for name in ("ic_launcher.xml", "ic_launcher_round.xml"):
        anydpi = RES / "mipmap-anydpi-v26"
        anydpi.mkdir(parents=True, exist_ok=True)
        (anydpi / name).write_text(
            """<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background" />
    <foreground android:drawable="@drawable/ic_launcher_foreground" />
</adaptive-icon>
""",
            encoding="utf-8",
        )

    splash_dir = RES / "drawable-xxhdpi"
    splash_dir.mkdir(parents=True, exist_ok=True)
    resized(source, 288).save(splash_dir / "splash_icon.png", optimize=True)
    print("generated launcher and splash icons from", SOURCE.relative_to(ROOT))


if __name__ == "__main__":
    main()
