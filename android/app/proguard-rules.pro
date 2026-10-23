# Keep the JavaScript bridge methods; R8 is disabled for release today, but this keeps the
# interface safe if minification is enabled later.
-keepclassmembers class io.github.hakai963.photocraft.DownloadBridge {
    @android.webkit.JavascriptInterface <methods>;
}
