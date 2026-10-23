/*
 * Android bridge for the PhotoCraft web build.
 *
 * Injected into dist/web/index.html by scripts/assemble_web_assets.py. It does three jobs:
 *   1. turns the web build's blob downloads into MediaStore files (WebView's DownloadListener
 *      cannot read blob: URLs),
 *   2. gives the desktop-oriented UI a touch-friendly page (no browser scroll/zoom/context menu),
 *   3. falls back from WebGL2 to PhotoCraft's CPU canvas if the GL context cannot start.
 *
 * It is a no-op outside the Android shell, so the same file can be opened in a desktop browser.
 */
(function () {
    'use strict';

    var api = window.PhotoCraftAndroid;
    if (!api || window.__photocraftAndroidBridge) {
        return;
    }
    window.__photocraftAndroidBridge = true;

    var CHUNK = 48 * 1024;
    var saving = Promise.resolve();

    function log(message) {
        try {
            api.logError(String(message));
        } catch (ignored) {
            // The bridge is optional; never break the editor because logging failed.
        }
    }

    function toBase64(bytes) {
        var binary = '';
        var step = 0x8000;
        for (var i = 0; i < bytes.length; i += step) {
            binary += String.fromCharCode.apply(null, bytes.subarray(i, i + step));
        }
        return btoa(binary);
    }

    function writeArrayBuffer(buffer, name, mime) {
        if (!api.beginDownload(name, mime || 'application/octet-stream')) {
            api.toast('Could not create the export file.');
            return Promise.resolve();
        }
        var bytes = new Uint8Array(buffer);
        for (var offset = 0; offset < bytes.length; offset += CHUNK) {
            api.writeChunk(toBase64(bytes.subarray(offset, offset + CHUNK)));
        }
        api.finishDownload();
        return Promise.resolve();
    }

    function writeAll(blob, name) {
        // Blob.stream is Chrome 76+. Android 10 shipped with a WebView around that version, but
        // some OEM images never update it, so keep a FileReader path for older engines.
        if (typeof blob.stream !== 'function') {
            return new Promise(function (resolve, reject) {
                var reader = new FileReader();
                reader.onload = function () {
                    resolve(reader.result);
                };
                reader.onerror = function () {
                    reject(reader.error || new Error('FileReader failed'));
                };
                reader.readAsArrayBuffer(blob);
            }).then(function (buffer) {
                return writeArrayBuffer(buffer, name, blob.type);
            }).catch(function (error) {
                api.abortDownload();
                api.toast('Export failed: ' + error);
                log(error);
            });
        }
        if (!api.beginDownload(name, blob.type || 'application/octet-stream')) {
            api.toast('Could not create the export file.');
            return Promise.resolve();
        }
        var reader = blob.stream().getReader();
        function pump() {
            return reader.read().then(function (result) {
                if (result.done) {
                    api.finishDownload();
                    return;
                }
                var value = result.value;
                for (var offset = 0; offset < value.length; offset += CHUNK) {
                    api.writeChunk(toBase64(value.subarray(offset, offset + CHUNK)));
                }
                return pump();
            });
        }
        return pump().catch(function (error) {
            api.abortDownload();
            api.toast('Export failed: ' + error);
            log(error);
        });
    }

    function saveBlob(name, blob) {
        // One export at a time: MediaStore writing is a single stream on the Java side.
        saving = saving.then(function () {
            return writeAll(blob, name);
        }, function () {
            return writeAll(blob, name);
        });
        return saving;
    }

    // The web shell revokes a blob URL immediately after clicking the anchor. Keep it alive long
    // enough for fetch() to read it; two minutes is far longer than a save takes to start.
    var nativeRevoke = URL.revokeObjectURL.bind(URL);
    URL.revokeObjectURL = function (url) {
        setTimeout(function () {
            try {
                nativeRevoke(url);
            } catch (ignored) {
                // Already revoked.
            }
        }, 120000);
    };

    var nativeClick = HTMLAnchorElement.prototype.click;
    HTMLAnchorElement.prototype.click = function () {
        try {
            var href = this.href;
            var name = this.download;
            if (name && href && (href.indexOf('blob:') === 0 || href.indexOf('data:') === 0)) {
                // Start the read before returning: the caller may revoke the URL right away.
                fetch(href).then(function (response) {
                    return response.blob();
                }).then(function (blob) {
                    return saveBlob(name, blob);
                }).catch(function (error) {
                    api.toast('Export failed: ' + error);
                    log(error);
                });
                return;
            }
        } catch (error) {
            log(error);
        }
        return Reflect.apply(nativeClick, this, arguments);
    };

    // WebView has no browser chrome. Stop long-press selection, pull-to-refresh and page zoom
    // from fighting the editor; egui receives the pointer events itself.
    document.addEventListener('contextmenu', function (event) {
        event.preventDefault();
    }, true);
    document.addEventListener('dragstart', function (event) {
        event.preventDefault();
    }, true);
    var style = document.createElement('style');
    style.textContent =
        'html,body{overscroll-behavior:none;touch-action:none;-webkit-user-select:none;user-select:none}' +
        '#photocraft_canvas{touch-action:none;outline:none}' +
        'input,textarea{-webkit-user-select:text;user-select:text}';
    (document.head || document.documentElement).appendChild(style);

    // Report failures that would otherwise only exist in logcat.
    window.addEventListener('error', function (event) {
        if (event && (event.message || event.error)) {
            log(event.message || event.error);
        }
    });
    window.addEventListener('unhandledrejection', function (event) {
        if (event && event.reason) {
            log(event.reason);
        }
    });

    // PhotoCraft starts the WebGL2 backend with ?webgl. Some OEM WebViews expose WebGL but fail
    // to create a context; in that case the page writes "failed to start" into its own loading
    // element. Reload once with ?cpu, which uses the CPU canvas and always works.
    var cpuFallbackDone = false;
    function fallBackToCpu() {
        if (cpuFallbackDone || location.search.indexOf('cpu') >= 0) {
            return;
        }
        cpuFallbackDone = true;
        location.replace(location.pathname + '?cpu');
    }

    function watchLoading() {
        var loading = document.getElementById('photocraft_loading');
        if (!loading) {
            return;
        }
        var observer = new MutationObserver(function () {
            if (/failed to start/i.test(loading.textContent || '')) {
                observer.disconnect();
                fallBackToCpu();
            }
        });
        observer.observe(loading, { childList: true, subtree: true, characterData: true });
        setTimeout(function () {
            observer.disconnect();
            var stillLoading = document.getElementById('photocraft_loading');
            if (stillLoading && stillLoading.isConnected) {
                fallBackToCpu();
            }
        }, 60000);
    }

    // The script is injected at the end of <body>, so the loading element already exists. Do
    // not wait for DOMContentLoaded: Trunk's module uses top-level await, and a hung wasm init
    // can hold that event forever, which would also disable the CPU fallback timer.
    if (document.getElementById('photocraft_loading')) {
        watchLoading();
    } else {
        document.addEventListener('DOMContentLoaded', watchLoading, { once: true });
    }
})();
