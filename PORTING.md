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

The port keeps the upstream core and editor, and changes only the shell and the chrome around
the canvas:

1. `Cargo.toml` — add the `android-web` profile (inherits `release`, `opt-level = 2`,
   `lto = false`, `codegen-units = 16`, `strip = "debuginfo"`).
2. `apps/photocraft-web/index.html` — select `data-cargo-profile="android-web"`, drop the
   `--converge` wasm-opt flag, add `viewport-fit=cover` and `color-scheme: dark`.
3. `apps/photocraft-web/src/web.rs` — read `?mobile` and enable the touch chrome before starting
   the eframe app.
4. `crates/ui-egui/src/mobile_ui.rs` (new) and the small branch in `lib.rs` — the Android-only
   top bar, bottom tool strip, brush-size stepper, panel sheets, command menu, tool picker and
   New Document sheet. Desktop builds never set the flag, so this module draws nothing there.
5. `crates/ui-egui/src/canvas.rs` — the welcome screen's **New document** button opens the mobile
   New Document sheet instead of the 790 pt desktop dialog.

Everything behind those entry points — the engine, document model, canvas, tools, panels, filters
and dialogs — is upstream code. The bridge and Android shell are port-owned and injected during
the build.

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

### Touch-first Android chrome

The page is loaded with `?webgl&mobile`, so the web shell enables `mobile_ui` in addition to the
WebGL2 backend. On a phone-width screen that replaces the desktop rails with:

- a 54 pt top bar: hamburger menu, document title, color swatch, Undo, Redo and Panels;
- the upstream options bar underneath it (horizontally scrollable, so every tool setting remains
  reachable);
- a large bottom strip with Tools, Move, Brush, Eraser, Hand, Zoom, Type, Crop, Marquee and Lasso;
- a 50 pt Brush Size stepper above that strip while a brush-like tool is active;
- full-width, resizable panel sheets (Layers, Color, History, Properties, Brushes, Navigator,
  Character) with large group chips and 38–46 pt tabs;
- a modal command menu with New / Open / Save / Export quick actions and grouped Edit, Select,
  Image, Layer and View items;
- a three-column tool picker containing every upstream tool;
- a phone-sized New Document sheet (presets, name, width/height, background) that calls the same
  `file.new` command as the desktop dialog.

All actions go through the normal `menus::invoke` / `app.run` paths, so they get the same
enablement rules, history, dialogs and status messages as the desktop. The layout uses egui
points; on Android a CSS pixel is a dp, so the 44–52 pt controls meet the usual touch-target size.
The desktop web build is unchanged because it never receives `?mobile`.

The browser first-launch shim that used to set the interface scale to 75/80% has been replaced by
a one-time migration back to 100%. The scale preference is still editable in Preferences.

### Rendering and touch

The page is loaded with `?webgl&mobile`; wgpu uses the WebGL2 backend, the most widely available
WebView path. If initialization fails, `android-bridge.js` watches the page's own error text and
reloads once with `?cpu&mobile` (PhotoCraft's CPU canvas). The bridge sets `touch-action: none`
and disables context menus so browser gestures do not fight the editor; egui receives the pointer
events. Two-finger pinch/twist reach the upstream canvas gesture code (Enhanced Controls › Enable
Gestures is on by default), and the Hand/Zoom tools cover one-finger navigation. The upstream web
shell also has pen pressure/tilt support and a touch-twin filter for tap events.

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

- [ ] APK installs on Android 10+ and launches past the splash into the mobile top/bottom bars.
- [ ] Menu → Open shows Android's picker and opens a JPEG and a PSD.
- [ ] Menu → New opens the mobile New Document sheet and creates a document.
- [ ] Panels opens the Layers sheet; Color opens the Color sheet; both scroll and resize.
- [ ] Tools opens the full tool picker; Move/Brush/Eraser/Hand/Zoom switch tools.
- [ ] The Brush Size stepper changes size without leaving the canvas.
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
- The touch chrome covers the main workspace, panels, tools and commands. A few deep editors
  (Preferences, some filter dialogs) still use the upstream desktop two-column sizing; the mobile
  New Document sheet replaces the widest of those.
- Saving does not prompt for a location; exports go to `Downloads/PhotoCraft/`. This mirrors the
  browser's download behaviour and is intentional.
- A very large document can exhaust the WebView renderer; the shell recovers but cannot prevent
  the OOM.
