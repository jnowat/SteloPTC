# ProGuard rules for SteloPTC Android release build
# Add project-specific ProGuard rules here.
# https://developer.android.com/studio/build/shrink-code

# Tauri WebView bridge — keep all public API classes
-keep class app.tauri.** { *; }
-keep class com.steloptc.app.** { *; }

# Keep Rust JNI bridge
-keep class * implements java.lang.Runnable { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}

# WebView JavaScript interface
-keepattributes JavascriptInterface
-keepclassmembers class * {
    @android.webkit.JavascriptInterface <methods>;
}

# Kotlin serialization
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.SerializationKt
-keep,includedescriptorclasses class com.steloptc.app.**$$serializer { *; }
-keepclassmembers class com.steloptc.app.** {
    *** Companion;
}
-keepclasseswithmembers class com.steloptc.app.** {
    kotlinx.serialization.KSerializer serializer(...);
}

# Keep source file names and line numbers for crash reports
-keepattributes SourceFile,LineNumberTable
-renamesourcefileattribute SourceFile
