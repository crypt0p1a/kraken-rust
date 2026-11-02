import org.gradle.api.tasks.testing.logging.TestExceptionFormat
import org.gradle.internal.os.OperatingSystem
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

// Kotlin Multiplatform library of kraken-ffi: JVM, Android, iOS, macOS, Linux and Windows, on the
// bindings of UbiqueInnovation/uniffi-kotlin-multiplatform-bindings. The plugin builds the Rust
// crate itself for every target and generates the bindings from its library. Every target is
// declared whatever the host: the library is built on macOS, with all of them.
//
//   ./gradlew :kraken-multiplatform:jvmTest                 the tests on the JVM (library of the host)
//   ./gradlew :kraken-multiplatform:linuxX64Test            Kotlin/Native on Linux x86_64
//   ./gradlew :kraken-multiplatform:iosSimulatorArm64Test   Kotlin/Native on the iOS simulator (macOS)
//   ./gradlew :kraken-multiplatform:connectedAndroidDeviceTest   the common tests on Android (device / emulator)
//   ./gradlew :kraken-multiplatform:assemble -PreleaseBuild=true   the libraries of every target:
//       the JVM jar with darwin, linux and win32, the AAR with every ABI
plugins {
    alias(additionals.plugins.kotlin.multiplatform)
    alias(additionals.plugins.android.kotlin.multiplatform.library)
    alias(additionals.plugins.ubique.uniffi)
}

cargo {
    // the crate of the library: kraken-ffi, which holds the types of kraken-api and kraken-network
    packageDirectory = layout.projectDirectory.dir("../../crates/kraken-ffi")
}

// The Apple targets can only be built on macOS
tasks.matching { (it.name.contains("Ios") || it.name.contains("Macos")) && !OperatingSystem.current().isMacOsX }
    .configureEach { enabled = false }

uniffi {
    // the bindings of kraken-api and kraken-network too, in their own packages (uniffi.kraken_api,
    // uniffi.kraken_network): they are part of the library of kraken-ffi
    generateBindingsForExternalCrates = true
    generateFromLibrary()
}

kotlin {
    // the UniFFI runtime is built for Java 17, its inline functions need the same target
    jvm {
        compilerOptions {
            jvmTarget.set(JvmTarget.JVM_17)
        }
    }

    android {
        namespace = "com.crypt0p1a.kraken.multiplatform"
        // the UniFFI runtime requires compileSdk 37
        compileSdk = additionals.versions.compileSdkVersion.get().toInt()
        minSdk = additionals.versions.minSdkVersion.get().toInt()
        compilerOptions {
            jvmTarget.set(JvmTarget.JVM_17)
        }

        // JNA and the bindings are called by reflection: the rules of the apps using the AAR
        optimization {
            consumerKeepRules.publish = true
            consumerKeepRules.file("consumer-rules.pro")
        }

        // the common tests on a device / emulator: ./gradlew :kraken-multiplatform:connectedAndroidDeviceTest
        withDeviceTestBuilder {
            sourceSetTreeName = "test"
        }.configure {
            instrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        }
    }

    // Kotlin/Native: every target, whatever the host. Built on macOS, the only host of the Apple
    // targets: the Linux and Windows libraries are cross-compiled (docs/toolchains.md)
    iosArm64()
    iosSimulatorArm64()
    iosX64()
    macosArm64()
    // no macosX64: neither the UniFFI plugin nor its runtime support it
    linuxX64()
    linuxArm64()
    mingwX64()

    // the uniffi bindings hold unused expressions, flagged since Kotlin 2.4
    targets.configureEach {
        compilations.configureEach {
            compileTaskProvider.configure {
                compilerOptions.freeCompilerArgs.add("-Xwarning-level=UNUSED_EXPRESSION:disabled")
            }
        }
    }

    sourceSets {
        commonMain.dependencies {
            // the exact decimals of every target: the type of kraken_api.Decimal (uniffi.toml)
            api(libs.bignum)
        }
        commonTest.dependencies {
            implementation(kotlin("test"))
            implementation(additionals.kotlinx.coroutines.test)
        }
        jvmTest.dependencies {
            implementation(additionals.okhttp.mockwebserver)
        }
        getByName("androidDeviceTest").dependencies {
            implementation(additionals.androidx.test.runner)
        }
    }
}

tasks.withType<AbstractTestTask>().configureEach {
    testLogging {
        events("passed", "skipped", "failed")
        exceptionFormat = TestExceptionFormat.FULL
    }
}
