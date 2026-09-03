plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "com.nexus.nexus_mobile"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        // TODO: Specify your own unique Application ID (https://developer.android.com/studio/build/application-id.html).
        applicationId = "com.nexus.nexus_mobile"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    buildTypes {
        release {
            // Release signing configuration is deferred to the native release milestone;
            // debug signing keeps `flutter run --release` functional during development.
            signingConfig = signingConfigs.getByName("debug")
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

dependencies {
    // AUD-040: native biometric surface (BiometricManager/BiometricPrompt).
    // androidx.biometric 1.1.0 stable provides
    // BiometricManager.Authenticators.BIOMETRIC_STRONG and the
    // crypto-based BiometricPrompt. The native security channel uses
    // these documented APIs; Keystore-backed secure storage needs no
    // extra dependency.
    implementation("androidx.biometric:biometric:1.1.0")
}

flutter {
    source = "../.."
}
