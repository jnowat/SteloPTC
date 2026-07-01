plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

android {
    compileSdk = 35
    ndkVersion = "27.2.12479018"
    namespace = "com.steloptc.app"

    defaultConfig {
        applicationId = "com.steloptc.app"
        minSdk = 24
        targetSdk = 35
        versionCode = 26
        versionName = "1.1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    signingConfigs {
        create("release") {
            // Configured via CI secrets decoded by build-android.yml.
            // All four env vars must be set — the build fails if any are absent.
            storeFile = System.getenv("ANDROID_KEY_STORE_PATH")
                ?.let { file(it) }
                ?: error("ANDROID_KEY_STORE_PATH is not set — cannot sign release APK")
            storePassword = System.getenv("ANDROID_KEY_STORE_PASSWORD")
                ?: error("ANDROID_KEY_STORE_PASSWORD is not set — cannot sign release APK")
            keyAlias = System.getenv("ANDROID_KEY_ALIAS")
                ?: error("ANDROID_KEY_ALIAS is not set — cannot sign release APK")
            keyPassword = System.getenv("ANDROID_KEY_PASSWORD")
                ?: error("ANDROID_KEY_PASSWORD is not set — cannot sign release APK")
        }
    }

    buildTypes {
        getByName("debug") {
            isDebuggable = true
            isMinifyEnabled = false
        }
        getByName("release") {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            signingConfig = signingConfigs.getByName("release")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        buildConfig = true
    }
}

rust {
    rootDirRel = "../../../"
}

dependencies {
    implementation("androidx.webkit:webkit:1.11.0")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.core:core-ktx:1.13.1")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.6.1")
}
