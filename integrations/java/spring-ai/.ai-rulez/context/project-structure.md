---
priority: medium
---

# Project Structure

## Build System

- Maven wrapper: `./mvnw verify` (no Taskfile, no Gradle)
- Java 25 with `--enable-preview`
- Enforcer plugin rejects lower Maven versions

## Coordinates

- Group: `dev.xberg`, Artifact: `xberg-spring-ai-document-reader`
- Package: `dev.xberg.springai`

## Public API

- Single class: `XbergDocumentReader implements DocumentReader` (Spring AI)
- Builder pattern for configuration

## Key Dependencies

- `dev.xberg:xberg` (Java binding, version property `xberg.version`)
- `org.springframework.ai:spring-ai-commons`

## Quality Gates (`./mvnw verify`)

- Checkstyle, PMD/CPD, Spotless formatter, JaCoCo 80% line coverage

## Publishing

- Profile `publish`: GPG signing + Central Publishing Plugin (OSSRH)
