# Porting notes

## Upstream

- Repository: <https://github.com/storytold/photocraft>
- Version: **0.14.0** (`[workspace.package] version`)
- Commit: **`ff4738810134d7f7e68b49f2e4f231b686282ff3`** (release commit, 2026-10-22)
- Licence: MIT OR Apache-2.0

The source is vendored rather than consumed as a submodule so the port is buildable from this
repository alone, including offline mirrors. To sync:

```sh
git remote add upstream https://github.com/storytold/photocraft.git
git fetch upstream
# replace crates/, apps/, xtask/, assets/, contributors/, Cargo.toml, Cargo.lock
# re-apply the three upstream edits below, then build in Actions
```

The only upstream-file edits are:

1. `Cargo.toml` — add the `android-web` profile (inherits `release`, `opt-level = 2`,
   `lto = false`, `codegen-units = 16`, `strip = "debuginfo"`).
2. `apps/photocraft-web/index.html` — select `data-cargo-profile="android-web"`, drop the
   `--converge` wasm-opt flag, add `viewport-fit=cover` and `color-scheme: dark`.
3. Nothing else. The Android bridge is injected into the built `index.html`; no Rust source is
   patched.

## Why WebAssembly instead of `cargo-ndk`

PhotoCraft is a desktop application. Its shell (`apps/photocraft`) depends on X11/Wayland
clipboard and tablet APIs, native file dialogs, single-instance IPC, desktop menus and a localhost
control server. A native Android target would mean replacing that shell and its platform services
before the first pixel is drawn — a different app, not a 1:1 port.

Upstream already ships a supported second target: `apps/photocraft-web`, which runs
`PhotocraftApp` through eframe's web runner. It uses the same document model, engine, codecs and
egui UI. Android's WebView is Chromium, so that target runs unmodified on the device. This port
therefore:

- builds the **upstream web target** on Linux in CI;
- packages the resulting wasm/JS/HTML in the APK;
- replaces only the browser integration points (file picker, downloads, dialogs, lifecycle).

That gives a 1:1 functional replica, not a reduced "mobile edition".

### The "Linux base"

The core is compiled on Ubuntu Linux in GitHub Actions. It is not recompiled for Android and it
does not use the NDK. WebAssembly is the portable output of that Linux build; Android supplies the
runtime. This is the same model as the official PhotoCraft web deployment, with the browser's
network and download assumptions replaced by native Android APIs.

## Android shell details

### Local origin and MIME types

`WebViewAssetLoader` serves `assets/www/` at
`https://appassets.androidplatform.net/assets/www/`. `LocalAssetHandler` overrides the default
`AssetsPathHandler` because the platform MIME table does not know `.wasm`, and a wrong MIME type
breaks `WebAssembly.instantiateStreaming`. It also sets `Cache-Control: no-store` so an APK update
cannot keep a stale module, and rejects traversal.

### Downloads

The web build's `download()` creates a `blob:` URL, clicks a detached `<a download>`, and revokes
the URL immediately. WebView cannot read `blob:` URLs through `DownloadListener`. The bridge:

1. patches `HTMLAnchorElement.prototype.click` for `download` anchors with `blob:`/`data:` hrefs;
2. delays `URL.revokeObjectURL` so the fetch has time to start;
3. streams the blob in 48 KiB chunks as base64 to `DownloadBridge`;
4. writes to `MediaStore.Downloads` with `RELATIVE_PATH=Download/PhotoCraft` and `IS_PENDING`,
   then clears `IS_PENDING` and shows a toast.

No `WRITE_EXTERNAL_STORAGE` permission is needed on the supported API range (29+).

### Files

`onShowFileChooser` uses `ACTION_OPEN_DOCUMENT` with `*/*` and `EXTRA_ALLOW_MULTIPLE` when the
page asks for multiple files. PhotoCraft's accepted list includes extensions (`.pcraft`, `.psdt`,
`.xcf`, `.kra`, `.cr3`, `.afphoto`, …) that Android MIME filters often hide; `*/*` lets the user
reach them, and the engine still validates the bytes. The URI grant is read-only and does not
persist beyond the page's read.

### Rendering and touch

The page is loaded with `?webgl` so wgpu uses the WebGL2 backend, which is the most widely
available WebView path. If initialization fails, `android-bridge.js` watches the page's own error
text and reloads once with `?cpu` (PhotoCraft's CPU canvas). The bridge sets `touch-action: none`
and disables context menus so browser gestures do not fight the editor; egui receives the pointer
events. The upstream web shell already has pen pressure/tilt support and a touch-twin filter for
tap events.

### Insets, rotation and lifecycle

- Target SDK 35 means Android 15 enforces edge-to-edge. `MainActivity` takes the system
  bar/cutout/IME insets as padding on the root, so panels are never under the status bar.
- `configChanges` keeps orientation changes from recreating the WebView; egui resizes its canvas.
- `FLAG_KEEP_SCREEN_ON` prevents the screen from sleeping during edits.
- `onRenderProcessGone` destroys the dead WebView and shows a restart screen rather than letting
  Android kill the app.
- Back shows a confirmation; there is no browser history to pop.

### Offline guarantee

`shouldInterceptRequest` allows only the local asset origin (plus `blob:`/`data:`/`about:` which
are in-process), returns 403 for everything else, and `shouldOverrideUrlLoading` sends external
links to the system browser. The app has no server URL, no analytics and no update checker.

## Build knobs

| Knob | Default | Effect |
|---|---|---|
| `android-web` Cargo profile | `opt-level=2`, no LTO | Fast CI build; larger wasm |
| `data-wasm-opt` | `z` | Trunk runs wasm-opt once (no `--converge`) |
| `PHOTOCRAFT_VERSION_NAME` | from `Cargo.toml` | `versionName` in the APK |
| `PHOTOCRAFT_VERSION_CODE` | `GITHUB_RUN_NUMBER` | Monotonic `versionCode` |
| `ANDROID_KEYSTORE_*` secrets | ephemeral key | Stable release signing |

To reduce APK size later, switch the profile back to upstream's `wasm-release` (fat LTO) and/or
re-enable wasm-opt convergence; both cost CI minutes but do not change behaviour.

## Testing checklist

There is no local SDK/NDK build by design. After an Actions run:

- [ ] APK installs on Android 10+ and launches past the splash.
- [ ] `File → Open` shows Android's picker and opens a JPEG and a PSD.
- [ ] A brush stroke and an adjustment layer render at a reasonable frame rate.
- [ ] `File → Save` / export writes to `Downloads/PhotoCraft/` and the toast appears.
- [ ] Rotating the device does not reload the page or lose the document.
- [ ] Back shows the exit confirmation; external links open the browser.
- [ ] Airplane mode does not change any of the above.
- [ ] A device without WebGL2 falls back to the CPU canvas (`?cpu`).

## Known gaps

- The web target has no TCP control server, native menus, printing or drag-and-drop from other
  apps. Those are upstream web limitations, not removed here.
- `rfd`'s web picker returns file contents, so File → Open works, but a folder picker is not
  representable (`FileDialogRequest::Folder` returns `None` upstream).
- Saving does not prompt for a location; exports go to `Downloads/PhotoCraft/`. This mirrors the
  browser's download behaviour and is intentional.
- A very large document can exhaust the WebView renderer; the shell recovers but cannot prevent
  the OOM.
