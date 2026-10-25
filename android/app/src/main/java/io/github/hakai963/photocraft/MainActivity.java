package io.github.hakai963.photocraft;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.content.res.ColorStateList;
import android.graphics.Color;
import android.graphics.Insets;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.util.Log;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.view.Window;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import android.view.WindowManager;
import android.webkit.ConsoleMessage;
import android.webkit.JsPromptResult;
import android.webkit.JsResult;
import android.webkit.PermissionRequest;
import android.webkit.ValueCallback;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceError;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Button;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.ProgressBar;
import android.widget.TextView;
import android.widget.Toast;

import androidx.webkit.WebViewAssetLoader;

import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;

/**
 * Native Android shell around PhotoCraft's own Rust/egui web build.
 *
 * <p>The editor is not reimplemented here: {@code www/} is the upstream {@code photocraft-web}
 * application, compiled from the same Rust core and rendered by the same egui UI. This activity
 * supplies the things a WebView cannot provide by itself: the asset origin, the Android file
 * picker, MediaStore downloads, JavaScript dialogs, safe-area insets, and a recoverable error
 * screen. All resources are served locally; remote requests are blocked.</p>
 */
public final class MainActivity extends Activity {

    private static final String TAG = "PhotoCraft";
    private static final String ASSET_HOST = "appassets.androidplatform.net";
    private static final String START_URL =
            "https://" + ASSET_HOST + "/assets/www/index.html?webgl&mobile";
    private static final int REQUEST_FILE = 4001;

    private WebView webView;
    private WebViewAssetLoader assetLoader;
    private DownloadBridge downloadBridge;
    private ValueCallback<Uri[]> fileCallback;
    private View loadingView;
    private View errorView;
    private TextView errorMessage;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        FrameLayout root = new FrameLayout(this);
        root.setBackgroundColor(0xFF262626);
        setContentView(root);
        applyWindowInsets(root);

        assetLoader = new WebViewAssetLoader.Builder()
                .setDomain(ASSET_HOST)
                .addPathHandler("/assets/", new LocalAssetHandler(this))
                .build();

        webView = new WebView(this);
        webView.setBackgroundColor(0xFF262626);
        configureWebView(webView);
        downloadBridge = new DownloadBridge(this);
        webView.addJavascriptInterface(downloadBridge, "PhotoCraftAndroid");
        webView.setWebViewClient(new LocalWebViewClient());
        webView.setWebChromeClient(new PhotoCraftChromeClient());
        root.addView(webView, new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));

        loadingView = createLoadingView();
        root.addView(loadingView, new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));

        errorView = createErrorView();
        errorView.setVisibility(View.GONE);
        root.addView(errorView, new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));

        webView.loadUrl(START_URL);
    }

    private void configureWebView(WebView view) {
        WebSettings settings = view.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setDatabaseEnabled(true);
        settings.setAllowFileAccess(false);
        settings.setAllowContentAccess(true);
        settings.setAllowFileAccessFromFileURLs(false);
        settings.setAllowUniversalAccessFromFileURLs(false);
        settings.setJavaScriptCanOpenWindowsAutomatically(false);
        settings.setSupportMultipleWindows(false);
        settings.setSupportZoom(false);
        settings.setBuiltInZoomControls(false);
        settings.setDisplayZoomControls(false);
        settings.setTextZoom(100);
        settings.setLoadsImagesAutomatically(true);
        settings.setMediaPlaybackRequiresUserGesture(false);
        settings.setGeolocationEnabled(false);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            settings.setSafeBrowsingEnabled(true);
        }
        view.setVerticalScrollBarEnabled(false);
        view.setHorizontalScrollBarEnabled(false);
        view.setOverScrollMode(View.OVER_SCROLL_NEVER);
        WebView.setWebContentsDebuggingEnabled(BuildConfig.DEBUG);
    }

    private void applyWindowInsets(View root) {
        Window window = getWindow();
        window.setStatusBarColor(Color.TRANSPARENT);
        window.setNavigationBarColor(0xFF1E1E1E);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            window.setNavigationBarContrastEnforced(false);
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            window.setDecorFitsSystemWindows(false);
            WindowInsetsController controller = window.getInsetsController();
            if (controller != null) {
                controller.setSystemBarsAppearance(
                        0,
                        WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
                                | WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS);
            }
            root.setOnApplyWindowInsetsListener((view, insets) -> {
                Insets bars = insets.getInsets(
                        WindowInsets.Type.systemBars()
                                | WindowInsets.Type.displayCutout()
                                | WindowInsets.Type.ime());
                view.setPadding(bars.left, bars.top, bars.right, bars.bottom);
                return WindowInsets.CONSUMED;
            });
        }
    }

    private View createLoadingView() {
        LinearLayout layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL);
        layout.setGravity(Gravity.CENTER);
        layout.setBackgroundColor(0xFF262626);
        layout.setClickable(true);
        layout.setFocusable(true);

        ProgressBar progress = new ProgressBar(this);
        progress.setIndeterminate(true);
        progress.setIndeterminateTintList(ColorStateList.valueOf(0xFF4B9FD5));
        layout.addView(progress, new LinearLayout.LayoutParams(dp(48), dp(48)));

        TextView title = new TextView(this);
        title.setText(R.string.loading);
        title.setTextColor(0xFFE0E0E0);
        title.setTextSize(16);
        title.setGravity(Gravity.CENTER);
        LinearLayout.LayoutParams titleParams = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        titleParams.topMargin = dp(18);
        layout.addView(title, titleParams);

        TextView hint = new TextView(this);
        hint.setText(R.string.loading_hint);
        hint.setTextColor(0xFF8C8C8C);
        hint.setTextSize(12);
        hint.setGravity(Gravity.CENTER);
        hint.setPadding(dp(32), dp(8), dp(32), 0);
        layout.addView(hint);

        return layout;
    }

    private View createErrorView() {
        LinearLayout layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL);
        layout.setGravity(Gravity.CENTER);
        layout.setBackgroundColor(0xFF262626);
        layout.setPadding(dp(28), dp(28), dp(28), dp(28));
        layout.setClickable(true);

        TextView title = new TextView(this);
        title.setText(R.string.error_title);
        title.setTextColor(0xFFE0E0E0);
        title.setTextSize(20);
        title.setGravity(Gravity.CENTER);
        layout.addView(title);

        errorMessage = new TextView(this);
        errorMessage.setTextColor(0xFFB8B8B8);
        errorMessage.setTextSize(14);
        errorMessage.setGravity(Gravity.CENTER);
        LinearLayout.LayoutParams messageParams = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        messageParams.topMargin = dp(12);
        layout.addView(errorMessage, messageParams);

        Button retry = new Button(this);
        retry.setText(R.string.error_retry);
        retry.setOnClickListener(v -> {
            errorView.setVisibility(View.GONE);
            loadingView.setVisibility(View.VISIBLE);
            if (webView != null) {
                webView.loadUrl(START_URL);
            } else {
                recreate();
            }
        });
        LinearLayout.LayoutParams retryParams = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        retryParams.topMargin = dp(24);
        layout.addView(retry, retryParams);

        return layout;
    }

    private void showError(String message) {
        runOnUiThread(() -> {
            if (isFinishing() || isDestroyed()) {
                return;
            }
            errorMessage.setText(message);
            errorView.setVisibility(View.VISIBLE);
            loadingView.setVisibility(View.GONE);
        });
    }

    private void hideLoading() {
        runOnUiThread(() -> loadingView.setVisibility(View.GONE));
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }

    private final class LocalWebViewClient extends WebViewClient {
        @Override
        public WebResourceResponse shouldInterceptRequest(WebView view, WebResourceRequest request) {
            Uri uri = request.getUrl();
            String host = uri.getHost();
            if (ASSET_HOST.equals(host)) {
                WebResourceResponse response = assetLoader.shouldInterceptRequest(uri);
                return response != null ? response : LocalAssetHandler.notFound();
            }
            String scheme = uri.getScheme();
            if ("blob".equals(scheme) || "data".equals(scheme) || "about".equals(scheme)
                    || "javascript".equals(scheme)) {
                return null;
            }
            // The editor is offline. Anything that is not a local asset gets a small, honest 403
            // instead of a slow DNS failure or an accidental network request.
            return LocalAssetHandler.forbidden();
        }

        @Override
        public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
            return handleNavigation(request.getUrl());
        }

        @SuppressWarnings("deprecation")
        @Override
        public boolean shouldOverrideUrlLoading(WebView view, String url) {
            return handleNavigation(Uri.parse(url));
        }

        private boolean handleNavigation(Uri uri) {
            if (uri == null) {
                return true;
            }
            String scheme = uri.getScheme();
            if ("https".equals(scheme) && ASSET_HOST.equals(uri.getHost())) {
                return false;
            }
            if ("about".equals(scheme) || "blob".equals(scheme) || "data".equals(scheme)) {
                return false;
            }
            try {
                startActivity(new Intent(Intent.ACTION_VIEW, uri));
            } catch (ActivityNotFoundException e) {
                Toast.makeText(MainActivity.this, R.string.file_picker_unavailable, Toast.LENGTH_SHORT).show();
            }
            return true;
        }

        @Override
        public void onPageFinished(WebView view, String url) {
            hideLoading();
        }

        @Override
        public void onReceivedError(WebView view, WebResourceRequest request, WebResourceError error) {
            if (request.isForMainFrame()) {
                showError(String.valueOf(error.getDescription()));
            }
        }

        @SuppressWarnings("deprecation")
        @Override
        public void onReceivedError(WebView view, int errorCode, String description, String failingUrl) {
            // The deprecated callback also fires for subresources on some WebViews. Only the
            // editor page itself failing is worth replacing the whole screen.
            if (failingUrl != null && failingUrl.startsWith(START_URL)) {
                showError(description + " (" + errorCode + ")");
            }
        }

        @Override
        public void onReceivedHttpError(WebView view, WebResourceRequest request, WebResourceResponse errorResponse) {
            if (request.isForMainFrame() && errorResponse.getStatusCode() >= 400) {
                showError("HTTP " + errorResponse.getStatusCode() + " " + errorResponse.getReasonPhrase());
            }
        }

        @Override
        public boolean onRenderProcessGone(WebView view, android.webkit.RenderProcessGoneDetail detail) {
            Log.e(TAG, "WebView renderer gone, didCrash=" + detail.didCrash());
            if (webView != null) {
                try {
                    webView.destroy();
                } catch (Throwable ignored) {
                    // The renderer is already gone; destroy can throw on some OEM builds.
                }
                webView = null;
            }
            showError(getString(R.string.error_renderer));
            return true;
        }
    }

    private final class PhotoCraftChromeClient extends WebChromeClient {
        @Override
        public boolean onShowFileChooser(WebView view, ValueCallback<Uri[]> callback,
                                         FileChooserParams params) {
            if (fileCallback != null) {
                fileCallback.onReceiveValue(null);
            }
            fileCallback = callback;

            // The editor accepts many formats (PSD, PSDT, XCF, KRA, RAW, HEIC, …) whose MIME
            // types Android does not know. Asking for */* keeps every file visible; PhotoCraft
            // validates the actual bytes after the picker returns.
            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            intent.setType("*/*");
            if (params.getMode() == FileChooserParams.MODE_OPEN_MULTIPLE) {
                intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true);
            }
            intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            try {
                startActivityForResult(intent, REQUEST_FILE);
            } catch (ActivityNotFoundException e) {
                fileCallback = null;
                callback.onReceiveValue(null);
                Toast.makeText(MainActivity.this, R.string.file_picker_unavailable, Toast.LENGTH_LONG).show();
            }
            return true;
        }

        @Override
        public boolean onJsAlert(WebView view, String url, String message, JsResult result) {
            showJsDialog(message, result, null);
            return true;
        }

        @Override
        public boolean onJsConfirm(WebView view, String url, String message, JsResult result) {
            showJsDialog(message, result, null);
            return true;
        }

        @Override
        public boolean onJsPrompt(WebView view, String url, String message, String defaultValue,
                                 JsPromptResult result) {
            EditText input = new EditText(MainActivity.this);
            input.setText(defaultValue == null ? "" : defaultValue);
            input.setSelection(input.getText().length());
            showJsDialog(message, result, input);
            return true;
        }

        private void showJsDialog(String message, JsResult result, EditText input) {
            if (isFinishing() || isDestroyed()) {
                result.cancel();
                return;
            }
            AlertDialog.Builder builder = new AlertDialog.Builder(MainActivity.this)
                    .setMessage(message)
                    .setPositiveButton(android.R.string.ok, (dialog, which) -> {
                        if (result instanceof JsPromptResult prompt) {
                            prompt.confirm(input == null ? "" : input.getText().toString());
                        } else {
                            result.confirm();
                        }
                    })
                    .setNegativeButton(android.R.string.cancel, (dialog, which) -> result.cancel())
                    .setOnCancelListener(dialog -> result.cancel());
            if (input != null) {
                builder.setView(input);
            }
            builder.show();
        }

        @Override
        public void onPermissionRequest(PermissionRequest request) {
            // The editor needs no camera or microphone. Deny rather than silently granting.
            request.deny();
        }

        @Override
        public boolean onConsoleMessage(ConsoleMessage consoleMessage) {
            Log.d(TAG + "Web", consoleMessage.message() + " ("
                    + consoleMessage.sourceId() + ":" + consoleMessage.lineNumber() + ")");
            return true;
        }
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        if (requestCode == REQUEST_FILE) {
            Uri[] result = null;
            if (resultCode == RESULT_OK) {
                result = WebChromeClient.FileChooserParams.parseResult(resultCode, data);
            }
            if (fileCallback != null) {
                fileCallback.onReceiveValue(result);
                fileCallback = null;
            }
            return;
        }
        super.onActivityResult(requestCode, resultCode, data);
    }

    @SuppressWarnings("deprecation")
    @Override
    public void onBackPressed() {
        if (webView != null && webView.canGoBack()) {
            webView.goBack();
            return;
        }
        if (isFinishing() || isDestroyed()) {
            return;
        }
        new AlertDialog.Builder(this)
                .setTitle(R.string.exit_title)
                .setMessage(R.string.exit_message)
                .setPositiveButton(R.string.exit_confirm, (dialog, which) -> finish())
                .setNegativeButton(R.string.cancel, null)
                .show();
    }

    @Override
    protected void onPause() {
        if (webView != null) {
            webView.onPause();
        }
        super.onPause();
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (webView != null) {
            webView.onResume();
        }
    }

    @Override
    protected void onDestroy() {
        if (downloadBridge != null) {
            downloadBridge.close();
        }
        if (webView != null) {
            webView.removeJavascriptInterface("PhotoCraftAndroid");
            webView.loadUrl("about:blank");
            webView.destroy();
            webView = null;
        }
        super.onDestroy();
    }
}
