# PhotoCraft for Android

<p align="center">
  <img src="https://raw.githubusercontent.com/storytold/photocraft/main/assets/app-icon/hicolor/256x256/apps/ai.storyteller.photocraft.png" width="112" height="112" alt="PhotoCraft icon">
</p>

<p align="center">
  <b>The same PhotoCraft Rust core and the same egui editor, running on Android.</b><br>
  Layers, masks, adjustment layers, layer styles, type, vectors, brushes, RAW/HEIC import and real PSD files — offline, on a phone or tablet.
</p>

<p align="center">
  <a href="https://github.com/HakaI963/photocraft-android/actions/workflows/android.yml"><img alt="Build Android APK" src="https://github.com/HakaI963/photocraft-android/actions/workflows/android.yml/badge.svg"></a>
  <a href="https://github.com/HakaI963/photocraft-android/releases"><img alt="Download APK" src="https://img.shields.io/badge/download-APK-2ea44f?logo=android&logoColor=white"></a>
  <img alt="Android 10+" src="https://img.shields.io/badge/Android-10%2B-3ddc84?logo=android&logoColor=white">
  <img alt="100% offline" src="https://img.shields.io/badge/network-100%25%20offline-4b9fd5">
</p>

> **Unofficial Android port.** PhotoCraft is created by [Storytold / ArtCraft](https://github.com/storytold/photocraft). This repository packages the upstream editor for Android and is not affiliated with Adobe or with the upstream project. Upstream is MIT / Apache-2.0; see [ATTRIBUTION.md](ATTRIBUTION.md).

---

## What this is

This is **not** a reimplementation or a “PhotoCraft-inspired” app. The APK contains the actual
upstream `photocraft-web` build: the same Rust image engine, the same document model, the same
PSD/RAW/HEIC codecs, and the same egui interface. The Android layer only supplies what a browser
cannot:

| Android layer | What it does |
|---|---|
| **WebView + `WebViewAssetLoader`** | Serves the Rust/egui build from `https://appassets.androidplatform.net` — local files, no server and no network. |
| **SAF file picker** | File → Open gets Android's document picker. PSD, PSDT, XCF, KRA, RAW, HEIC and every other format stay selectable (`*/*`; the editor validates bytes). |
| **MediaStore bridge** | The web build saves by creating a `blob:` download. A small JS bridge streams that blob to `Downloads/PhotoCraft/` in base64 chunks. No storage permission on Android 10+. |
| **Lifecycle / recovery** | Keeps the screen awake while editing, handles insets and rotation without reloading, reports JS errors, and recovers from a WebView renderer crash instead of closing. |
| **CPU fallback** | Starts with WebGL2 (`?webgl`). If the device's WebGL context cannot start, the bridge reloads once with PhotoCraft's CPU canvas (`?cpu`). |

Everything that edits pixels runs in the Rust core compiled to WebAssembly. There is **no NDK
build, no JNI image pipeline, and no second implementation to drift from upstream**.

### Architecture

```
                    GitHub Actions (Ubuntu Linux)
  ┌────────────────────────────────────────────────────────────┐
  │  vendored PhotoCraft 0.14.0 (Rust, 1,500+ source files)     │
  │        │ cargo + wasm32-unknown-unknown + Trunk            │
  │        ▼                                                   │
  │  dist/web: index.html + JS glue + photo engine .wasm       │
  │        │ scripts/assemble_web_assets.py                    │
  │        ▼                                                   │
  │  android/app/src/main/assets/www  +  android-bridge.js     │
  │        │ Gradle + AGP (Java shell, no NDK)                 │
  │        ▼                                                   │
  │  signed release APK  ──►  GitHub Release                   │
  └────────────────────────────────────────────────────────────┘

                    Android device
  ┌────────────────────────────────────────────────────────────┐
  │  MainActivity (Java)                                       │
  │    ├─ WebView ── local https asset origin ── Rust core     │
  │    ├─ SAF picker ── File → Open / Place                      │
  │    ├─ DownloadBridge ── MediaStore Downloads/PhotoCraft     │
  │    └─ error screen / renderer recovery / back handling      │
  └────────────────────────────────────────────────────────────┘
```

The core is **built on Linux** in CI and shipped as a portable WebAssembly module. Android
provides the runtime (Chromium WebView), storage and file picker; the editor itself is upstream
PhotoCraft, not a rewrite.

## Install

1. Open **[Releases](https://github.com/HakaI963/photocraft-android/releases)**.
2. Download `PhotoCraft-Android-<version>.apk` (and `SHA256SUMS.txt` if you want to verify it).
3. Allow installation from your browser/file manager when Android asks.
4. Open **PhotoCraft**, then **File → Open** (or the Open button) and pick an image.

Requirements: **Android 10 (API 29) or newer**, a 64-bit device, and a WebView with WebGL2
(the preinstalled Android System WebView / Chrome). A tablet, foldable or DeX-style display is
recommended: the editor is the desktop UI, and its panels are dense on a phone.

The app is fully offline. It declares `INTERNET` only because Chromium expects the permission for
an `https://` origin; every request that is not the local asset origin is answered with 403 by
`MainActivity.LocalWebViewClient`, and no analytics, fonts, or update checks are performed.

## Features

Same as upstream PhotoCraft 0.14.0, because it is the same build:

- Layers, groups, masks, clipping masks, blend modes, opacity and lock flags
- Adjustment layers (Curves, Levels, Hue/Saturation, Vibrance, Color Balance, …)
- Layer styles (drop shadow, stroke, glow, bevel, color overlay, …)
- Type layers with live text, vectors, shapes and pen paths
- Brushes, eraser, gradients, selections, transform, crop and canvas resize
- Real **PSD / PSB / PSDT** read and write, plus PNG, JPEG, TIFF, WebP, OpenRaster, Krita, XCF,
  Affinity files, and camera RAW / HEIC import (the upstream web build's codec set)
- Undo history, presets, document tabs, recovery-friendly WebView storage
- 100% local processing — no upload, account, subscription or telemetry

See the [upstream README](https://github.com/storytold/photocraft#features) for the full,
current feature matrix.

## Build

**All builds run in GitHub Actions. There is no local Gradle or NDK build in this port.**

Push to `main` (or run **Actions → Build PhotoCraft Android → Run workflow**). The workflow:

1. installs JDK 17, Android SDK 35, Rust stable + `wasm32-unknown-unknown`, and Trunk;
2. compiles the vendored upstream workspace with the `android-web` Cargo profile;
3. copies `dist/web` into the APK assets and injects `bridge/android-bridge.js`;
4. builds `:app:assembleRelease` with Gradle 8.11.1 / AGP 8.7.3 (Java 17, no NDK);
5. verifies the signature with `apksigner`, uploads the APK artifact, and creates or updates
   the `android-v<version>` GitHub Release.

### Why the `android-web` profile?

Upstream's browser profile uses fat LTO at `opt-level = "z"` and is tuned for a 24 MiB download
budget. A CI-only Android APK has no such budget, and a four-core runner spends most of an hour on
that LTO. `android-web` inherits `release`, uses `opt-level = 2`, disables LTO, and allows parallel
codegen units. The resulting wasm is larger but the build is much faster and the editor runs the
same code.

### Signing

The workflow uses repository secrets when present:

| Secret | Meaning |
|---|---|
| `ANDROID_KEYSTORE_BASE64` | Base64 of the release `.jks` |
| `ANDROID_KEYSTORE_PASSWORD` | Keystore password |
| `ANDROID_KEY_ALIAS` | Key alias (optional, defaults to `photocraft`) |
| `ANDROID_KEY_PASSWORD` | Key password (optional, defaults to the keystore password) |

Without them the workflow still produces an installable APK signed with an **ephemeral** key, and
prints a warning. Ephemeral keys cannot upgrade an earlier install, so configure the secrets for
releases you intend to keep.

## Repository layout

| Path | Contents |
|---|---|
| `crates/`, `apps/`, `assets/`, `contributors/` | Vendored upstream PhotoCraft 0.14.0 (commit `ff47388`) |
| `apps/photocraft-web/` | Upstream browser runner; `index.html` selects the `android-web` profile |
| `bridge/android-bridge.js` | Blob→MediaStore downloads, touch page rules, CPU fallback |
| `scripts/assemble_web_assets.py` | Copies Trunk output into the APK and injects the bridge |
| `scripts/make_icons.py` | Reproducible launcher/splash icons from the upstream icon |
| `android/` | Java WebView shell, Gradle project, manifest, resources |
| `.github/workflows/android.yml` | The only supported build path |
| `PORTING.md` | Detailed port notes, upstream sync and limitations |

## Known limitations

These come from the upstream **web** target, not from the Android shell:

- The UI is desktop-first; touch works (the web build has pointer/touch handling) but panels are
  small on a phone. Use a tablet for serious editing.
- Very large documents are limited by the WebView's memory. The manifest requests `largeHeap`, and
  the shell reports a renderer crash instead of dying silently, but WASM still needs the pixels in
  memory.
- Desktop-only features (native menu bar, localhost control server, printing, some OS
  integrations) are absent in the browser build.
- First launch spends a few seconds compiling the wasm module. Later launches use the WebView
  cache.
- HEIC/HEIF import is enabled (`--features heif`); camera RAW support is the upstream pure-Rust
  set. A format that upstream's web build cannot decode is not added by this port.

## Credits and licences

PhotoCraft is © the PhotoCraft contributors, licensed **MIT OR Apache-2.0**. The vendored source,
icon, fonts and attribution notices are kept in this repository. The Android shell, bridge and
build scripts are released under the same terms. See [LICENSE-MIT](LICENSE-MIT),
[LICENSE-APACHE](LICENSE-APACHE), [NOTICE](NOTICE) and [ATTRIBUTION.md](ATTRIBUTION.md).

PhotoCraft is an independent clean-room project and is not affiliated with Adobe. Adobe,
Photoshop and PSD are trademarks of Adobe Inc.
