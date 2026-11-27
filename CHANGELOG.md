# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/sunsided/width-search/compare/v0.3.1...v0.4.0) - 2026-11-27

### Added

- Add six scenario examples, one per family of searches
- Add π-HIW, count-based Rollout IW over high-level states
- Add IHIW, HIW that discovers its own high-level features
- Add hierarchical IW, HIW(w_high, w_low)
- Add SIW-R, serialized IW with subgoals from sketch rules
- Add π-IW, Rollout IW guided by a policy
- Add SIW+, serialized IW with novelty partitioned by #r

### Changed

- Explain a missing task file and room borders in the examples
- Resume HIW's low-level searches lazily, as in the paper
- Assert both HIW widths up front
- Say that #r in SIW+ skips facts true at the run root

### Fixed

- Fix HIW order docs and reject infinite π-IW weight sums

## [0.3.1](https://github.com/sunsided/width-search/compare/v0.3.0...v0.3.1) - 2026-11-22

### Changed

- Link the crate to its page on the planning and search site ([#25](https://github.com/sunsided/width-search/pull/25))
- Note the set-width guarantee and the hardness of certifying width ([#24](https://github.com/sunsided/width-search/pull/24))
- Describe the audience and installation in the README ([#21](https://github.com/sunsided/width-search/pull/21))

## [0.3.0](https://github.com/sunsided/width-search/compare/v0.2.0...v0.3.0) - 2026-11-21

### Added

- Expose the additive heuristic as `SasProblem::h_add` (feature `sas-plus`); `None` marks a relaxed dead end; the value saturates at `u64::MAX` ([#17](https://github.com/sunsided/width-search/pull/17))

### Changed

- List the crate in the game-development category ([#20](https://github.com/sunsided/width-search/pull/20))

### Fixed

- Count a fact that is both a precondition and an effect condition once in `h_add`, not twice ([#18](https://github.com/sunsided/width-search/pull/18))
- `bfws` and `bfws_r` reject width 0 even when the initial state is a goal; `iw`, `bfws` and `bfws_r` document the panic ([#19](https://github.com/sunsided/width-search/pull/19))

## [0.2.0](https://github.com/sunsided/width-search/compare/v0.1.1...v0.2.0) - 2026-11-19

### Added

- Add SIW, BFWS with #r and Rollout IW, compared against LAPKT ([#14](https://github.com/sunsided/width-search/pull/14))

## [0.1.1](https://github.com/sunsided/width-search/compare/v0.1.0...v0.1.1) - 2026-11-19

### Changed

- Compare width-based search to other search algorithms in the README ([#12](https://github.com/sunsided/width-search/pull/12))
- Cover old states and the goal exception in the width section
- Explain what "width" means in the README

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
