# width-search

[![crates.io](https://img.shields.io/crates/v/width-search.svg)](https://crates.io/crates/width-search)
[![docs.rs](https://img.shields.io/docsrs/width-search)](https://docs.rs/width-search)
[![CI](https://github.com/sunsided/width-search/actions/workflows/ci.yml/badge.svg)](https://github.com/sunsided/width-search/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/sunsided/width-search/graph/badge.svg)](https://codecov.io/gh/sunsided/width-search)
[![MSRV](https://img.shields.io/crates/msrv/width-search)](https://crates.io/crates/width-search)
[![License](https://img.shields.io/crates/l/width-search)](https://github.com/sunsided/width-search#license)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

Width-based search (Lipovetzky & Geffner): novelty tables, IW(k), iterated width, and BFWS(⟨w_{#g}, #g⟩).

Not tied to PDDL. Anything that can enumerate successors and describe states as integer features implements `WidthProblem`: planning atoms, grid cells, RAM bytes. The `sas-plus` feature adds an adapter for `sas_plus::Task`.

## Usage

Implement `WidthProblem` and run a search. Here, walking from 0 to 5 on a line has width 1:

```rust
use width_search::{WidthProblem, iw};

/// Walk along the integers 0..=5, one step at a time.
struct Line;

impl WidthProblem for Line {
    type State = u32;
    type Action = i8;

    fn initial_state(&self) -> u32 {
        0
    }

    fn successors(&self, s: &u32, out: &mut Vec<(i8, u32)>) {
        if *s < 5 {
            out.push((1, s + 1));
        }
        if *s > 0 {
            out.push((-1, s - 1));
        }
    }

    // One feature per position.
    fn features(&self, s: &u32, out: &mut Vec<u32>) {
        out.push(*s);
    }

    fn num_features(&self) -> usize {
        6
    }

    fn is_goal(&self, s: &u32) -> bool {
        *s == 5
    }
}

let plan = iw(&Line, 1).plan.expect("IW(1) solves it");
assert_eq!(plan, [1, 1, 1, 1, 1]);
```

A grid with a key and a door needs width 2, because the agent revisits cells after picking up the key:

```sh
cargo run --example key_door         # IW(1) fails, IW(2) succeeds
```

## Features

- `sas-plus`: `sas::SasProblem`, an adapter that runs width-based search on a [`sas_plus::Task`](https://docs.rs/sas-plus).

**Tested:** end to end on a Fast Downward `output.sas` (gripper: IW needs width 3 for the conjunctive goal, finding the optimal 5-step plan). Also: IW with k = #variables is complete and optimal on unit-cost random tasks; IW and BFWS plans are validated.

**Roadmap:** SIW, BFWS with `#r`, Rollout IW, node-count comparison against LAPKT.

References: Lipovetzky & Geffner, ECAI 2012 and AAAI 2017.

## MSRV

Rust 1.86. Raising it is a minor release.

## Contributing

See [CONTRIBUTING.md](https://github.com/sunsided/width-search/blob/main/CONTRIBUTING.md).

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/sunsided/width-search/blob/main/LICENSES/Apache-2.0.txt) or [MIT license](https://github.com/sunsided/width-search/blob/main/LICENSES/MIT.txt) at your option.
The examples in [`examples/`](https://github.com/sunsided/width-search/tree/main/examples) are public domain ([CC0-1.0](https://github.com/sunsided/width-search/blob/main/LICENSES/CC0-1.0.txt)), so you can copy them into anything.
Test fixtures carry their own licenses; see [`REUSE.toml`](https://github.com/sunsided/width-search/blob/main/REUSE.toml).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you shall be dual licensed as above, without any additional terms or conditions.
