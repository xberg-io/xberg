import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
  `java-library`
  kotlin("jvm") version "2.3.21"
  `maven-publish`
  id("org.jlleitschuh.gradle.ktlint") version "12.1.1"
}

group = "dev.kreuzberg"
version = "5.0.0-rc.1"

repositories {
  mavenCentral()
}

dependencies {
  api("net.java.dev.jna:jna:5.18.1")
  // Jackson is on the public surface because the alef-emitted Java records
  // include `@JsonProperty` annotations for serialization round-tripping.
  api("com.fasterxml.jackson.core:jackson-annotations:2.18.2")
  api("com.fasterxml.jackson.core:jackson-databind:2.18.2")
  api("com.fasterxml.jackson.datatype:jackson-datatype-jdk8:2.18.2")
  // jspecify ships the `@Nullable` / `@NonNull` annotations referenced by the
  // alef-emitted Java facade; it must be on the api configuration so Kotlin
  // consumers see the annotations on cross-language types.
  api("org.jspecify:jspecify:1.0.0")
  implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.11.0")
  testImplementation("org.jetbrains.kotlin:kotlin-test:2.3.21")
  testImplementation("junit:junit:4.13.2")
}

java {
  sourceCompatibility = JavaVersion.VERSION_25
  targetCompatibility = JavaVersion.VERSION_25
}

// Include the alef-emitted Java facade (sibling package) so the Kotlin object
// can call into the JNA-loaded native bridge. The Kotlin backend places its
// generated files in a sub-package (`<group>.kt`) to avoid colliding with the
// Java facade that uses the canonical `<group>` package.
sourceSets {
  main {
    java {
      // Pull in the Java facade emitted by the alef Java backend so the
      // Kotlin module compiles against the same on-disk sources.
      srcDir("../java/src/main/java")
    }
  }
}

kotlin {
  compilerOptions {
    jvmTarget.set(JvmTarget.JVM_25)
  }
}

// ktlint configuration — see .editorconfig for details. We deliberately exclude
// the Java facade (which lives under `packages/java/`) and any build/generated
// directories: ktlint cannot lint pure-Java files, and the FFM/Panama bindings
// are kept in their own module.
ktlint {
  version.set("1.4.1")
  outputToConsole.set(true)
  ignoreFailures.set(false)
  filter {
    exclude { entry -> entry.file.toString().contains("/packages/java/") }
    exclude("**/build/**")
    exclude("**/generated/**")
  }
}

// JNA needs the native lib on java.library.path; default to the workspace
// `target/release` cargo output. Override with `-Pkb.lib.path=<dir>`.
tasks.withType<Test>().configureEach {
  val libPath = (project.findProperty("kb.lib.path") as String?) ?: "$rootDir/../../target/release"
  systemProperty("jna.library.path", libPath)
  systemProperty("java.library.path", libPath)
  useJUnit()
}

// Publish under a Kotlin-specific artifactId so consumers can disambiguate
// the Kotlin module from the sibling Java facade in the same Maven group.
publishing {
  publications {
    create<MavenPublication>("maven") {
      artifactId = "kreuzberg-kotlin"
      from(components["java"])
    }
  }
}
