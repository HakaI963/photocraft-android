package io.github.hakai963.photocraft;

import android.app.Activity;
import android.content.ContentResolver;
import android.content.ContentValues;
import android.net.Uri;
import android.os.Environment;
import android.provider.MediaStore;
import android.util.Base64;
import android.util.Log;
import android.webkit.JavascriptInterface;
import android.widget.Toast;

import java.io.OutputStream;

/**
 * Receives the web build's exports.
 *
 * <p>PhotoCraft saves by creating a {@code blob:} URL and clicking an {@code <a download>}. WebView
 * cannot hand blob URLs to {@code DownloadListener}, so {@code android-bridge.js} patches the anchor
 * click, streams the blob here in base64 chunks, and this class writes it into
 * {@code Downloads/PhotoCraft/} through MediaStore. No storage permission is required on API 29+.</p>
 */
public final class DownloadBridge {

    private static final String TAG = "PhotoCraftDownloads";
    private static final String FOLDER = Environment.DIRECTORY_DOWNLOADS + "/PhotoCraft";
    private static final int MAX_NAME_LENGTH = 180;

    private final Activity activity;
    private final Object lock = new Object();

    private OutputStream output;
    private Uri uri;
    private String displayName;
    private boolean active;

    DownloadBridge(Activity activity) {
        this.activity = activity;
    }

    @JavascriptInterface
    public boolean beginDownload(String name, String mime) {
        synchronized (lock) {
            if (active) {
                abortLocked();
            }
            displayName = sanitize(name);
            String safeMime = sanitizeMime(mime);
            try {
                ContentResolver resolver = activity.getContentResolver();
                ContentValues values = new ContentValues();
                values.put(MediaStore.MediaColumns.DISPLAY_NAME, displayName);
                values.put(MediaStore.MediaColumns.MIME_TYPE, safeMime);
                values.put(MediaStore.MediaColumns.RELATIVE_PATH, FOLDER);
                values.put(MediaStore.MediaColumns.IS_PENDING, 1);
                uri = resolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values);
                if (uri == null) {
                    failLocked("MediaStore refused the download");
                    return false;
                }
                output = resolver.openOutputStream(uri, "w");
                if (output == null) {
                    failLocked("could not open the output stream");
                    return false;
                }
                active = true;
                return true;
            } catch (Exception e) {
                failLocked(e.toString());
                return false;
            }
        }
    }

    @JavascriptInterface
    public void writeChunk(String base64) {
        synchronized (lock) {
            if (!active || output == null || base64 == null) {
                return;
            }
            try {
                output.write(Base64.decode(base64, Base64.NO_WRAP));
            } catch (Exception e) {
                failLocked(e.toString());
            }
        }
    }

    @JavascriptInterface
    public String finishDownload() {
        synchronized (lock) {
            if (!active || output == null || uri == null) {
                return null;
            }
            String savedName = displayName;
            Uri savedUri = uri;
            try {
                output.flush();
                output.close();
                output = null;
                ContentValues values = new ContentValues();
                values.put(MediaStore.MediaColumns.IS_PENDING, 0);
                activity.getContentResolver().update(savedUri, values, null, null);
                active = false;
                uri = null;
                toast("Saved to Downloads/PhotoCraft/" + savedName);
                return savedUri.toString();
            } catch (Exception e) {
                failLocked(e.toString());
                return null;
            }
        }
    }

    @JavascriptInterface
    public void abortDownload() {
        synchronized (lock) {
            abortLocked();
        }
    }

    @JavascriptInterface
    public void toast(String message) {
        if (message == null) {
            return;
        }
        activity.runOnUiThread(() -> {
            if (!activity.isFinishing() && !activity.isDestroyed()) {
                Toast.makeText(activity, message, Toast.LENGTH_SHORT).show();
            }
        });
    }

    @JavascriptInterface
    public void logError(String message) {
        Log.e(TAG, String.valueOf(message));
    }

    void close() {
        synchronized (lock) {
            abortLocked();
        }
    }

    private void failLocked(String message) {
        Log.e(TAG, message);
        toast(activity.getString(R.string.save_failed));
        abortLocked();
    }

    private void abortLocked() {
        if (output != null) {
            try {
                output.close();
            } catch (Exception ignored) {
                // Best effort: the pending MediaStore row is deleted below.
            }
            output = null;
        }
        if (uri != null) {
            try {
                activity.getContentResolver().delete(uri, null, null);
            } catch (Exception ignored) {
                // The row may already be gone (process death, revoked permission).
            }
            uri = null;
        }
        active = false;
    }

    private static String sanitize(String name) {
        String value = name == null ? "" : name;
        int slash = Math.max(value.lastIndexOf('/'), value.lastIndexOf('\\'));
        if (slash >= 0) {
            value = value.substring(slash + 1);
        }
        value = value.replaceAll("[\\\\/:*?\"<>|\\p{Cntrl}]", "_").trim();
        if (value.isEmpty() || ".".equals(value) || "..".equals(value)) {
            value = "photocraft-export";
        }
        if (value.length() > MAX_NAME_LENGTH) {
            value = value.substring(0, MAX_NAME_LENGTH);
        }
        return value;
    }

    private static String sanitizeMime(String mime) {
        if (mime == null || mime.isEmpty() || !mime.contains("/")) {
            return "application/octet-stream";
        }
        return mime;
    }
}
