# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.2](https://github.com/garykellygrimm-a11y/Redactify/compare/redactify-core-v0.4.1...redactify-core-v0.4.2) - 2026-07-21

### Added

- *(core)* manifest dispositions — reviewed vs unreviewed runs
- *(core)* derive Serialize on Finding
- *(core)* load user-defined rules from TOML with fail-fast validation
- *(core)* add audit manifest and error type

### Fixed

- *(ci)* restore release-plz's release memory; lockstep versions
- *(core)* derive Debug on Rule

### Other

- release
- Apply Rust format
- bump versions to 0.4.0 for milestone 4
- Merge pull request #6 from garykellygrimm-a11y/dependabot/cargo/toml-1.1.3spec-1.1.0
- *(deps)* bump sha2 from 0.10.9 to 0.11.0
- Corrected formatting issue.
- Removed .gitkeep file and added markdown of manifest content.
- Created redactify-cli
- adjusted phone regex
- Created finding.rs and rules.rs. Setting them both with rule sets and tests.
- Ran a format all agaist repository.
- Adjusted lib.rs with test stub
- Removed stale .gitkeep files
- Developed ci.yml with a proper lint and test for rust. Created main.rs and populated it with a print function.
- Initialized crate directories to generate the necessary libraries.
- initialize repository layout

## [0.4.0](https://github.com/garykellygrimm-a11y/Redactify/releases/tag/redactify-core-v0.4.0) - 2026-07-21

### Added

- *(core)* manifest dispositions — reviewed vs unreviewed runs
- *(core)* derive Serialize on Finding
- *(core)* load user-defined rules from TOML with fail-fast validation
- *(core)* add audit manifest and error type

### Fixed

- *(core)* derive Debug on Rule

### Other

- Apply Rust format
- bump versions to 0.4.0 for milestone 4
- Merge pull request #6 from garykellygrimm-a11y/dependabot/cargo/toml-1.1.3spec-1.1.0
- *(deps)* bump sha2 from 0.10.9 to 0.11.0
- Corrected formatting issue.
- Removed .gitkeep file and added markdown of manifest content.
- Created redactify-cli
- adjusted phone regex
- Created finding.rs and rules.rs. Setting them both with rule sets and tests.
- Ran a format all agaist repository.
- Adjusted lib.rs with test stub
- Removed stale .gitkeep files
- Developed ci.yml with a proper lint and test for rust. Created main.rs and populated it with a print function.
- Initialized crate directories to generate the necessary libraries.
- initialize repository layout
