// Kotlin Multiplatform library of kraken-ffi: JVM, Android, iOS, macOS and Linux, on the bindings of
// UbiqueInnovation/uniffi-kotlin-multiplatform-bindings. The plugin builds the Rust crate itself
// for every target and generates the bindings from its library:
//
//   gradle jvmTest                       the tests on the JVM (library of the host)
//   gradle linuxX64Test                  Kotlin/Native on Linux x86_64
//   gradle iosSimulatorArm64Test         Kotlin/Native on the iOS simulator (macOS)
//   gradle assemble                      the libraries of every target
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

rootProject.name = "kraken-multiplatform"
