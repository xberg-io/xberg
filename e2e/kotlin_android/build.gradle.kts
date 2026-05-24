import com.android.build.api.dsl.ManagedVirtualDevice
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.library") version "8.13.0"
    kotlin("android") version "2.3.21"
}

group = "dev.kreuzberg"
version = "0.1.0"

android {
    namespace = "dev.kreuzberg.e2e"
    compileSdk = 35

    defaultConfig {
        minSdk = 21
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets {
        getByName("test") {
            // Include the AAR-bundled Java facade as test sources
            java.srcDir("../../packages/kotlin-android/src/main/java")
            // Include the AAR-bundled Kotlin wrapper as test sources
            kotlin.srcDir("../../packages/kotlin-android/src/main/kotlin")
        }
    }

    testOptions {
        // Gradle Managed Virtual Devices for on-device instrumented tests.
        // Run: ./gradlew pixel6api34DebugAndroidTest
        managedDevices {
            devices {
                create<ManagedVirtualDevice>("pixel6api34") {
                    device = "Pixel 6"
                    apiLevel = 34
                    systemImageSource = "aosp"
                }
            }
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_17
    }
}

// Repositories declared in settings.gradle.kts via
// dependencyResolutionManagement (FAIL_ON_PROJECT_REPOS). Re-declaring them
// here triggers Gradle "repository was added by build file" errors.

dependencies {
    // JNA for loading the native library from java.library.path
    testImplementation("net.java.dev.jna:jna:5.18.1")

    // Jackson for JSON assertion helpers
    testImplementation("com.fasterxml.jackson.core:jackson-annotations:2.18.2")
    testImplementation("com.fasterxml.jackson.core:jackson-databind:2.18.2")
    testImplementation("com.fasterxml.jackson.datatype:jackson-datatype-jdk8:2.18.2")

    // jackson-module-kotlin registers constructors/properties for Kotlin data
    // classes, which have no default constructor and cannot be deserialized by
    // plain Jackson without this module.
    testImplementation("com.fasterxml.jackson.module:jackson-module-kotlin:2.18.2")

    // jspecify for null-safety annotations on wrapped types
    testImplementation("org.jspecify:jspecify:1.0.0")

    // Kotlin coroutines for async test helpers
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.11.0")

    // JUnit 5 API and engine
    testImplementation("org.junit.jupiter:junit-jupiter-api:6.1.0")
    testImplementation("org.junit.jupiter:junit-jupiter-engine:6.1.0")


    // Kotlin stdlib test helpers
    testImplementation(kotlin("test"))
}

tasks.withType<Test> {
    useJUnitPlatform()

    // Resolve the native library location (e.g., ../../target/release)
    val libPath = System.getProperty("kb.lib.path") ?: "${rootDir}/../../target/release"
    systemProperty("java.library.path", libPath)
    systemProperty("jna.library.path", libPath)

    // Resolve fixture paths (e.g. "docx/fake.docx") against test_documents/
    workingDir = file("${rootDir}/../../test_documents")
}
