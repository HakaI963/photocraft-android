plugins {
    id("com.android.application")
}

android {
    namespace = "io.github.hakai963.photocraft"
    compileSdk = 35

    defaultConfig {
        applicationId = "io.github.hakai963.photocraft"
        minSdk = 29
        targetSdk = 35
        versionCode = (System.getenv("PHOTOCRAFT_VERSION_CODE") ?: "1").toInt()
        versionName = System.getenv("PHOTOCRAFT_VERSION_NAME") ?: "0.14.0"
    }

    signingConfigs {
        val storePath = System.getenv("ANDROID_KEYSTORE_PATH")
        if (!storePath.isNullOrBlank()) {
            create("release") {
                storeFile = file(storePath)
                storePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("ANDROID_KEY_ALIAS")
                keyPassword = System.getenv("ANDROID_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            isShrinkResources = false
            signingConfig = signingConfigs.findByName("release") ?: signingConfigs.getByName("debug")
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    buildFeatures {
        buildConfig = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    packaging {
        resources {
            excludes += setOf(
                "META-INF/DEPENDENCIES",
                "META-INF/LICENSE",
                "META-INF/LICENSE.txt",
                "META-INF/NOTICE",
                "META-INF/NOTICE.txt",
            )
        }
    }

    androidResources {
        // The wasm module is read through the asset loader; compressing it would make WebView
        // decompress the whole 30+ MB blob into memory before compiling it.
        noCompress += "wasm"
    }
}

dependencies {
    implementation("androidx.webkit:webkit:1.12.1")
}
