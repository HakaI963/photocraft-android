# PhotoCraft for Android

The complete PhotoCraft editor — the same Rust core and editor canvas as the desktop app — packaged for Android with a touch-first layout.

## Install

1. Download `PhotoCraft-Android-0.14.0.apk` below.
2. Allow installs from this source when Android asks.
3. Open PhotoCraft, then tap **☰ → Open** (or the Open button on the welcome screen) and choose an image.

Requires Android 10 or newer. Fully offline; no account or network connection is used.

## What's inside

- **Touch-first Android chrome** built for phones and tablets:
  - large top bar with menu, undo/redo, color and panels
  - bottom tool strip with Move, Brush, Eraser, Hand, Zoom, Type, Crop, Marquee and Lasso
  - full-width, resizable **Layers / Color / History / Properties / Brushes / Navigator** sheets
  - a command menu with large New / Open / Save / Export buttons and grouped actions
  - a mobile **New Document** sheet with presets, size fields and background choice
  - a large **Brush Size** stepper above the tool strip while painting
- The Android build now starts at **100% interface scale**; old 75/80% first-launch settings are migrated once. UI scale remains adjustable in Preferences → Interface.
- Layers, masks, adjustment layers and layer styles
- Type, vectors, brushes, selections, transform and crop
- PSD / PSB / PSDT read and write, plus PNG, JPEG, TIFF, WebP, OpenRaster, Krita, XCF, Affinity
- Camera RAW and HEIC import
- Exports are saved to **Downloads/PhotoCraft/**
- WebGL2 by default, automatic CPU fallback if the GPU path cannot start

This is an unofficial port of [storytold/photocraft](https://github.com/storytold/photocraft)
(PhotoCraft 0.14.0, commit `ff47388`), MIT / Apache-2.0. It is built entirely in GitHub Actions
from the vendored upstream source; see the repository's `PORTING.md` for details and limitations.

Verify the download with the attached `SHA256SUMS.txt`.
