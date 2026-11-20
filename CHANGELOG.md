# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Expose the additive heuristic as `SasProblem::h_add` (feature `sas-plus`); `None` marks a relaxed dead end; the value saturates at `u64::MAX`

### Fixed

- Count a fact that is both a precondition and an effect condition once in `h_add`, not twice

## [0.2.0](https://github.com/sunsided/width-search/compare/v0.1.1...v0.2.0) - 2026-11-19

### Added

- Add SIW, BFWS with #r and Rollout IW, compared against LAPKT ([#14](https://github.com/sunsided/width-search/pull/14))

## [0.1.1](https://github.com/sunsided/width-search/compare/v0.1.0...v0.1.1) - 2026-11-19

### Changed

- Compare width-based search to other search algorithms in the README ([#12](https://github.com/sunsided/width-search/pull/12))
- Cover old states and the goal exception in the width section
- Explain what "width" means in the README
- Compare width-based search to other search algorithms in the README

### Fixed

- Fix algorithm preconditions in the README comparison

## [0.1.0](https://github.com/sunsided/width-search/releases/tag/v0.1.0) - 2026-11-19

Initial release.

### Added

- `NoveltyTable`: novelty of feature tuples up to size k (bitsets for sizes 1 and 2, hashing above).
- `iw` and `iterated_width`: IW(k) and iterated width.
- `bfws`: BFWS(⟨w_{#g}, #g⟩), complete on finite state spaces.
- `WidthProblem` and `GoalCount` traits, independent of PDDL.
- `sas-plus` feature: `sas::SasProblem`, an adapter for sas-plus 0.2 tasks, with axioms evaluated in the initial state.
- A key-and-door example where IW(1) fails and IW(2) succeeds.

MSRV: Rust 1.86.
