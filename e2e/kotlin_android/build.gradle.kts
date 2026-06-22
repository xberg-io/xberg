import java.net.HttpURLConnection
import java.net.URL
import java.util.zip.ZipFile
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.library") version "9.2.0"
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
        // Host JVM unit tests: no Android device/emulator required.
        // Tests run against the published AAR and JVM-side deps via `gradle test`.
        unitTests {
            isReturnDefaultValues = true
        }
    }
}

kotlin {
    // Set JVM target for compilation. gradle.properties enables auto-detection
    // of host JDK installations so Gradle uses the available JDK version on the
    // build machine, preventing provisioning failures when the target version is not installed.
    jvmToolchain(17)
    compilerOptions {
        jvmTarget = JvmTarget.JVM_17
    }
}

// Repositories declared in settings.gradle.kts via
// dependencyResolutionManagement (FAIL_ON_PROJECT_REPOS). Re-declaring them
// here triggers Gradle "repository was added by build file" errors.

dependencies {

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
    testRuntimeOnly("org.junit.platform:junit-platform-launcher:6.1.0")

    // Kotlin stdlib test helpers
    testImplementation(kotlin("test"))

    // JNA for loading the native library from java.library.path
    testImplementation("net.java.dev.jna:jna:5.18.1")

}

// Build host JNI library for JVM unit tests (macOS/Linux/Windows).
// The generated Kotlin Bridge object calls System.loadLibrary("kreuzberg_jni") for JVM
// unit tests running on developer machines. This task builds the host-platform binary
// and stages it into src/test/resources/host-jni/<platform>/ for the test loader.
// Set alef.skipHostJni=true to disable this (e.g., in CI where only source-set validation is needed).
tasks.register("buildHostJni", Exec::class) {
    if (project.properties["alef.skipHostJni"] != "true") {
        val jniCargoPath = "../../crates/kreuzberg-jni/Cargo.toml"
        description = "Build host-platform JNI library from ../../crates/kreuzberg-jni"
        commandLine("cargo", "build", "--release", "--manifest-path", jniCargoPath)
        errorOutput = System.err
    } else {
        description = "Build host JNI (disabled via alef.skipHostJni=true)"
        commandLine("true")
    }
}

tasks.register("copyHostJni", Copy::class) {
    if (project.properties["alef.skipHostJni"] != "true") {
        description = "Copy host JNI library to test resources"
        dependsOn("buildHostJni")

        val hostPlatform = if (System.getProperty("os.name").lowercase().contains("mac")) {
            "darwin"
        } else if (System.getProperty("os.name").lowercase().contains("win")) {
            "windows"
        } else {
            "linux"
        }
        val libName = when (hostPlatform) {
            "darwin" -> "libkreuzberg_jni.dylib"
            "windows" -> "kreuzberg_jni.dll"
            else -> "libkreuzberg_jni.so"
        }

        // Cargo builds to the workspace target directory by default, even when
        // --manifest-path points at a member crate. The previous
        // `if (workspaceTarget.exists()) ... else crateTarget` dual-path was
        // evaluated at gradle configuration time, before `cargo build` finished
        // or before the workspace target dir existed, so the glob could match
        // zero files and the test runtime would fail with `UnsatisfiedLinkError`
        // at static-init time. Always read from the workspace target.
        val workspaceTarget = file("../../target/release")

        from(workspaceTarget) {
            include(libName)
        }
        into(layout.projectDirectory.dir("src/test/resources/host-jni/$hostPlatform"))
    }
}

tasks.withType<Test> {
    useJUnitPlatform()

    // Resolve the native library location (e.g., ../../target/release)
    val libPath = System.getProperty("kb.lib.path") ?: "${rootDir}/../../target/release"
    systemProperty("java.library.path", libPath)
    systemProperty("jna.library.path", libPath)

    // Resolve fixture paths (e.g. "docx/fake.docx") against test_documents/
    workingDir = file("${rootDir}/../../test_documents")

    if (project.properties["alef.skipHostJni"] != "true") {
        val hostPlatform = if (System.getProperty("os.name").lowercase().contains("mac")) {
            "darwin"
        } else if (System.getProperty("os.name").lowercase().contains("win")) {
            "windows"
        } else {
            "linux"
        }
        systemProperty(
            "java.library.path",
            project.layout.projectDirectory.dir("src/test/resources/host-jni/$hostPlatform").asFile.absolutePath
        )
        dependsOn("copyHostJni")
    }
}

tasks.matching { it.name.startsWith("processDebug") || it.name.startsWith("processRelease") }.configureEach {
    if (project.properties["alef.skipHostJni"] != "true" && name.contains("UnitTestJavaRes")) {
        dependsOn("copyHostJni")
    }
}
