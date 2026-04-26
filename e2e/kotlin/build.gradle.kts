import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    kotlin("jvm") version "2.1.10"
}

group = "dev.kreuzberg"
version = "0.1.0"

java {
    sourceCompatibility = JavaVersion.VERSION_21
    targetCompatibility = JavaVersion.VERSION_21
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_21)
    }
}

repositories {
    mavenCentral()
}

dependencies {
    testImplementation(files("../../packages/kotlin/build/libs/kreuzberg-0.1.0.jar"))
    testImplementation("org.junit.jupiter:junit-jupiter-api:5.11.4")
    testImplementation("org.junit.jupiter:junit-jupiter-engine:5.11.4")
    testImplementation("com.fasterxml.jackson.core:jackson-databind:2.18.2")
    testImplementation("com.fasterxml.jackson.datatype:jackson-datatype-jdk8:2.18.2")
}

tasks.test {
    useJUnitPlatform()
    environment("java.library.path", "../../target/release")
}
