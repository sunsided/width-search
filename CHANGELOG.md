# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/sunsided/width-search/releases/tag/v0.1.0) - 2026-11-19

### Added

- Add the unsafe-forbidden badge to the README
- Add release automation with release-plz
- Add CONTRIBUTING.md
- Add git hooks and Copilot review instructions
- Add crate keywords, categories and README badges
- Add benchmarks and a novelty-table fuzz target
- Add cargo-deny, Dependabot and typos configuration
- Add test coverage with Codecov
- Add Taskfile and harden the CI workflow
- Add README
- Add an optional adapter for sas-plus tasks
- Add a key-and-door example
- Add best-first width search
- Add IW(k) and iterated width
- Add a novelty table

### Changed

- Upgrade sas-plus to 0.2 and criterion to 0.8 ([#8](https://github.com/sunsided/width-search/pull/8))
- Upgrade criterion to 0.8 and raise the MSRV to 1.86
- Clarify description of WidthProblem implementation ([#6](https://github.com/sunsided/width-search/pull/6))
- Upgrade sas-plus to 0.2 and evaluate axioms in the initial state
- Give release PRs an imperative title
- Clarify description of WidthProblem implementation
- Print usage when the commit-msg check gets no file
- Group Dependabot requirement updates and drop dead doc references
- Fail the publish check on unexpected crates.io responses
- Document the novelty table's full memory footprint
- Build fuzz targets for the host triple in the Taskfile
- Install actionlint from its release instead of install-action
- Document every item and clear the pedantic findings
- Enable pedantic clippy, doc-comment lints and forbid unsafe code
- Use the README as crate docs and doctest its example
- Move integration tests into tests/it and add property tests
- Adopt REUSE for licensing
- Test width-based search on Fast Downward translator output
- Initial commit

### Security

- Harden the actionlint download in CI
