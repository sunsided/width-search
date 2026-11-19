# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
