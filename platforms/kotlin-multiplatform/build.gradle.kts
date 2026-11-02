import ch.ubique.uniffi.plugin.model.RustHost
import org.gradle.api.tasks.testing.logging.TestExceptionFormat
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

// Kotlin Multiplatform library of kraken-ffi: JVM, Android, iOS, macOS and Linux, on the bindings of
// UbiqueInnovation/uniffi-kotlin-multiplatform-bindings. The plugin builds the Rust crate itself
// for every target and generates the bindings from its library:
//
//   ./gradlew :kraken-multiplatform:jvmTest                 the tests on the JVM (library of the host)
//   ./gradlew :kraken-multiplatform:linuxX64Test            Kotlin/Native on Linux x86_64
//   ./gradlew :kraken-multiplatform:iosSimulatorArm64Test   Kotlin/Native on the iOS simulator (macOS)
//   ./gradlew :kraken-multiplatform:assemble                the libraries of every target
plugins {
    alias(additionals.plugins.kotlin.multiplatform)
    alias(additionals.plugins.android.kotlin.multiplatform.library)
    alias(additionals.plugins.ubique.uniffi)
}

cargo {
    // the crate of the library: kraken-ffi, which holds the types of kraken-api and kraken-network
    packageDirectory = layout.projectDirectory.dir("../../crates/kraken-ffi")
}

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
    }

    // Kotlin/Native: the targets the Rust crate can be built for on the host, without a cross
    // compiler for ring
    when (RustHost.current.platform) {
        RustHost.Platform.MacOS -> {
            iosArm64()
            iosSimulatorArm64()
            iosX64()
            macosArm64()
        }
        RustHost.Platform.Linux -> when (RustHost.current.arch) {
            RustHost.Arch.X64 -> linuxX64()
            RustHost.Arch.Arm64 -> linuxArm64()
        }
        RustHost.Platform.Windows -> {}
    }

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
    }
}

tasks.withType<AbstractTestTask>().configureEach {
    testLogging {
        events("passed", "skipped", "failed")
        exceptionFormat = TestExceptionFormat.FULL
    }
}
