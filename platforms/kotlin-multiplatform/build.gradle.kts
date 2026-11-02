import ch.ubique.uniffi.plugin.model.RustHost
import org.gradle.api.tasks.testing.logging.TestExceptionFormat
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    kotlin("multiplatform") version "2.4.0"
    id("com.android.kotlin.multiplatform.library") version "9.3.1"
    id("ch.ubique.uniffi.plugin") version "1.3.2"
}

group = "com.crypt0p1a.kraken"
version = "0.1.0"

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
    jvm {
        compilerOptions {
            jvmTarget.set(JvmTarget.JVM_17)
        }
    }

    android {
        namespace = "com.crypt0p1a.kraken.multiplatform"
        compileSdk = 36
        minSdk = 24
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

    sourceSets {
        commonMain.dependencies {
            // the exact decimals of every target: the type of kraken_api.Decimal (uniffi.toml)
            api("com.ionspin.kotlin:bignum:0.3.10")
        }
        commonTest.dependencies {
            implementation(kotlin("test"))
            implementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.11.0")
        }
        jvmTest.dependencies {
            implementation("com.squareup.okhttp3:mockwebserver:4.12.0")
        }
    }
}

tasks.withType<AbstractTestTask>().configureEach {
    testLogging {
        events("passed", "skipped", "failed")
        exceptionFormat = TestExceptionFormat.FULL
    }
}
