import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    `java-library`
    kotlin("jvm") version "2.1.10"
    `maven-publish`
}

group = "dev.kreuzberg"
version = "4.9.5"

repositories {
    mavenCentral()
}

dependencies {
    api("net.java.dev.jna:jna:5.14.0")
    // Jackson is on the public surface because the alef-emitted Java records
    // include `@JsonProperty` annotations for serialization round-tripping.
    api("com.fasterxml.jackson.core:jackson-annotations:2.18.2")
    api("com.fasterxml.jackson.core:jackson-databind:2.18.2")
    api("com.fasterxml.jackson.datatype:jackson-datatype-jdk8:2.18.2")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.9.0")
    testImplementation("org.jetbrains.kotlin:kotlin-test:2.1.10")
    testImplementation("junit:junit:4.13.2")
}

java {
    sourceCompatibility = JavaVersion.VERSION_21
    targetCompatibility = JavaVersion.VERSION_21
}

// Include the alef-emitted Java facade (sibling package) so the Kotlin object
// can call into the JNA-loaded native bridge. The Kotlin backend places its
// generated files in a sub-package (`<group>.kt`) to avoid colliding with the
// Java facade that uses the canonical `<group>` package.
sourceSets {
    main {
        java {
            srcDir("../java/src/main/java")
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_21)
    }
}

// JNA needs the native lib on java.library.path; default to the workspace
// `target/release` cargo output. Override with `-Pkb.lib.path=<dir>`.
tasks.withType<Test>().configureEach {
    val libPath = (project.findProperty("kb.lib.path") as String?) ?: "${rootDir}/../../target/release"
    systemProperty("jna.library.path", libPath)
    systemProperty("java.library.path", libPath)
    useJUnit()
}

publishing {
    publications {
        create<MavenPublication>("maven") {
            from(components["java"])
        }
    }
}
