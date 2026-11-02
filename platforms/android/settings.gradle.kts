// Android library (AAR) of the Kotlin bindings of kraken-ffi, with the libraries of every ABI:
//
//   scripts/build.sh android && scripts/bindings.sh kotlin
//   gradle assembleRelease                 build/outputs/aar/kraken-android-release.aar
//   gradle connectedDebugAndroidTest       the tests of platforms/kotlin, on a device / emulator
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "kraken-android"
