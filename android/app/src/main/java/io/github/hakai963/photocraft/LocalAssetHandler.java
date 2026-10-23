package io.github.hakai963.photocraft;

import android.content.Context;
import android.webkit.WebResourceResponse;

import androidx.webkit.WebViewAssetLoader;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Locale;
import java.util.Map;

/**
 * Serves {@code android/app/src/main/assets/www} through {@link WebViewAssetLoader}.
 *
 * <p>{@link WebViewAssetLoader.AssetsPathHandler} guesses MIME types with a small platform table
 * that does not know {@code .wasm}; a wrong MIME type makes {@code instantiateStreaming} fail (or
 * forces a slower fallback). This handler sets the exact types the web build needs and adds
 * no-store headers so an app update cannot keep a stale wasm module alive.</p>
 */
final class LocalAssetHandler implements WebViewAssetLoader.PathHandler {

    private final Context context;

    LocalAssetHandler(Context context) {
        this.context = context.getApplicationContext();
    }

    @Override
    public WebResourceResponse handle(String path) {
        String clean = path == null ? "" : path;
        while (clean.startsWith("/")) {
            clean = clean.substring(1);
        }
        if (clean.isEmpty()) {
            clean = "index.html";
        }
        // The asset manager is read-only, but reject traversal attempts before joining paths.
        if (clean.contains("..") || clean.contains("\\") || clean.indexOf('\0') >= 0) {
            return notFound();
        }

        String assetPath = "www/" + clean;
        try {
            InputStream stream = context.getAssets().open(assetPath);
            String mime = mimeType(clean);
            Map<String, String> headers = new HashMap<>();
            headers.put("Cache-Control", "no-store");
            headers.put("X-Content-Type-Options", "nosniff");
            if ("application/wasm".equals(mime)) {
                headers.put("Cross-Origin-Resource-Policy", "same-origin");
            }
            String encoding = isText(mime) ? "utf-8" : null;
            return new WebResourceResponse(mime, encoding, 200, "OK", headers, stream);
        } catch (IOException e) {
            return notFound();
        }
    }

    static WebResourceResponse notFound() {
        return textResponse(404, "Not Found", "Not found.\n");
    }

    static WebResourceResponse forbidden() {
        return textResponse(403, "Forbidden", "PhotoCraft is offline; remote requests are blocked.\n");
    }

    private static WebResourceResponse textResponse(int status, String reason, String body) {
        Map<String, String> headers = new HashMap<>();
        headers.put("Cache-Control", "no-store");
        headers.put("X-Content-Type-Options", "nosniff");
        return new WebResourceResponse(
                "text/plain",
                "utf-8",
                status,
                reason,
                headers,
                new ByteArrayInputStream(body.getBytes(StandardCharsets.UTF_8)));
    }

    private static boolean isText(String mime) {
        return mime.startsWith("text/")
                || mime.contains("javascript")
                || mime.contains("json")
                || mime.contains("xml")
                || mime.contains("svg");
    }

    private static String mimeType(String path) {
        String lower = path.toLowerCase(Locale.ROOT);
        int dot = lower.lastIndexOf('.');
        String extension = dot >= 0 ? lower.substring(dot + 1) : "";
        switch (extension) {
            case "html":
            case "htm":
                return "text/html";
            case "js":
            case "mjs":
                return "text/javascript";
            case "wasm":
                return "application/wasm";
            case "json":
            case "map":
                return "application/json";
            case "css":
                return "text/css";
            case "svg":
                return "image/svg+xml";
            case "png":
                return "image/png";
            case "jpg":
            case "jpeg":
                return "image/jpeg";
            case "webp":
                return "image/webp";
            case "gif":
                return "image/gif";
            case "ico":
                return "image/x-icon";
            case "ttf":
                return "font/ttf";
            case "otf":
                return "font/otf";
            case "woff":
                return "font/woff";
            case "woff2":
                return "font/woff2";
            case "txt":
            case "md":
                return "text/plain";
            case "xml":
                return "application/xml";
            default:
                return "application/octet-stream";
        }
    }
}
