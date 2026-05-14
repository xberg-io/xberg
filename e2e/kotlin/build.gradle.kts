import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    kotlin("jvm") version "2.3.21"
    java
}

group = "dev.kreuzberg"
version = "0.1.0"

java {
    sourceCompatibility = JavaVersion.VERSION_25
    targetCompatibility = JavaVersion.VERSION_25
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_25)
    }
}

repositories {
    mavenCentral()
}

dependencies {
    testImplementation(files("../../packages/kotlin/build/libs/kreuzberg-5.0.0-rc.1.jar"))
    testImplementation("net.java.dev.jna:jna:5.18.1")
    testImplementation("com.fasterxml.jackson.core:jackson-annotations:2.18.2")
    testImplementation("com.fasterxml.jackson.core:jackson-databind:2.18.2")
    testImplementation("com.fasterxml.jackson.datatype:jackson-datatype-jdk8:2.18.2")
    testImplementation("org.jspecify:jspecify:1.0.0")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.11.0")
    testImplementation("org.junit.jupiter:junit-jupiter-api:6.0.3")
    testImplementation("org.junit.jupiter:junit-jupiter-engine:6.0.3")

    testImplementation("com.fasterxml.jackson.core:jackson-databind:2.18.2")
    testImplementation("com.fasterxml.jackson.datatype:jackson-datatype-jdk8:2.18.2")
    testImplementation(kotlin("test"))
}

tasks.test {
    useJUnitPlatform()
    val libPath = System.getProperty("kb.lib.path") ?: "${rootDir}/../../target/release"
    systemProperty("java.library.path", libPath)
    systemProperty("jna.library.path", libPath)
    // Resolve fixture paths (e.g. "docx/fake.docx") against test_documents/.
    workingDir = file("${rootDir}/../../test_documents")
}
