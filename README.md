# width-search

[![crates.io](https://img.shields.io/crates/v/width-search.svg)](https://crates.io/crates/width-search)
[![docs.rs](https://img.shields.io/docsrs/width-search)](https://docs.rs/width-search)
[![CI](https://github.com/sunsided/width-search/actions/workflows/ci.yml/badge.svg)](https://github.com/sunsided/width-search/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/sunsided/width-search/graph/badge.svg)](https://codecov.io/gh/sunsided/width-search)
[![MSRV](https://img.shields.io/crates/msrv/width-search)](https://crates.io/crates/width-search)
[![License](https://img.shields.io/crates/l/width-search)](https://github.com/sunsided/width-search#license)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

Width-based search (Lipovetzky & Geffner): novelty tables, IW(k), iterated width, serialized IW (SIW), BFWS(⟨w_{#g}, #g⟩), BFWS with relaxed-plan counts `#r`, and Rollout IW(1).

Anything that can enumerate successors and describe states as integer features implements `WidthProblem`: planning atoms, grid cells, RAM bytes. The `sas-plus` feature adds an adapter for `sas_plus::Task`.

Part of the [planning and search crates](https://sunsided.github.io/planning-and-search/); see also this crate's [overview page](https://sunsided.github.io/planning-and-search/crates/width-search/).

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

## What "width" means

A state's **novelty** is the size of the smallest set of its features that no earlier state in the search had all at once. A state that reaches a never-seen cell has novelty 1. A state whose cell and key flag were each seen before, but never together, has novelty 2. A state whose every feature combination was seen before has no novelty value; it counts as greater than any `k`.

**IW(k)** is breadth-first search that drops every non-goal state with novelty greater than `k`. Goal states are always kept. So it remembers only feature combinations of size up to `k`, and with `n` features there are at most `O(n^k)` of those.

A problem's **width** is, roughly, the smallest `k` for which IW(k) finds a shortest plan. Formally (Lipovetzky & Geffner, 2012), it is the smallest `k` such that a chain of feature sets of size at most `k` leads from the initial state to the goal. Each set in the chain is reached optimally by extending an optimal plan for the previous set by one action. Intuitively, width counts how many facts you must track *jointly* to avoid losing your way:

- Walking a line or reaching a cell on an open grid has width 1. Position alone tells you whether a state is new.
- The `key_door` example has width 2. After taking the key, the agent walks back over cells it has visited. Position alone calls those states old and IW(1) prunes them. The pair (cell, has key) shows they are new.

Width is a property of the problem *and* of the features you choose. Better features can lower it. Many puzzle and planning domains have width 1 or 2 for a single goal fact, which is why low-`k` searches go so far. Conjunctive goals ("collect every key") usually have higher width, which is the case BFWS handles by also counting satisfied goals (`#g`).

## How it compares

Classic searches rank states by distance: the cost so far (BFS, Dijkstra), plus an estimate of the cost to go (A*). Width-based search ranks them by **novelty** instead. A state is worth keeping only if it makes some feature value, or some combination of up to `k` feature values, true for the first time. You don't need a heuristic or a distance metric. You only need features, and the search explores "new things" first.

| Algorithm       | Needs                                 | Optimal?                                                  | Memory / cost                         | Shines when                                                           |
|-----------------|---------------------------------------|-----------------------------------------------------------|---------------------------------------|-----------------------------------------------------------------------|
| BFS             | successors                            | yes, unit costs                                           | whole reachable space                 | small state spaces                                                    |
| Dijkstra        | successors, non-negative edge costs   | yes                                                       | whole reachable space                 | weighted graphs, no good heuristic                                    |
| A*              | successors, non-negative costs, admissible heuristic (consistent when it keeps a closed set) | yes | grows with heuristic error            | pathfinding on maps, where distance heuristics are good               |
| Greedy best-first | successors, heuristic               | no                                                        | heuristic-dependent                   | fast "any plan" with an informative heuristic                         |
| MCTS            | simulator, reward / rollout policy    | no (anytime)                                              | tree of visited nodes                 | adversarial or stochastic games, large branching                      |
| **IW(k)**       | successors, features                  | shortest plan (unit costs) if the problem's width is ≤ k | at most `O(n^k)` expansions for `n` features | huge state spaces with low width, no heuristic available        |
| **SIW**         | successors, features, goal count      | no; incomplete                                            | one IW(k) per subgoal                 | goals that can be reached one at a time                               |
| **BFWS**        | successors, features, goal count      | no                                                        | novelty tables plus open list         | conjunctive goals (collect all keys, reach all switches)              |
| **BFWS `#r`**   | as BFWS, plus relaxed-plan features   | no                                                        | as BFWS, plus a relaxed plan per drop in `#g` | planning tasks where a relaxed plan says which facts matter     |
| **Rollout IW(1)** | successors, features                | no                                                        | tree of rollouts                      | simulators where breadth-first layers are too wide (Atari)            |

**Pros**

- No heuristic to design. Features (atoms, cell positions, inventory flags, RAM bytes) are usually easy to write down.
- Bounded effort. IW(k) expands at most `O(n^k)` states however big the raw state space is, so it is predictable on combinatorial spaces where BFS blows up.
- Many benchmark and puzzle domains have width 1 or 2 per goal atom, so IW(1) and IW(2) solve them quickly.
- It works on black-box simulators: it never inspects the transition model, only the features of the states it generates.

**Cons**

- IW(k) is incomplete when the problem's width exceeds `k`. It prunes the state that leads to the goal and reports failure. `iterated_width` retries with larger `k`, at a cost that grows steeply with `k`.
- It ignores action costs. Use Dijkstra or A* when you need the cheapest path on a weighted graph.
- Results depend on your features. Coarse features prune too much, and redundant ones waste effort.
- It is not built for adversarial or stochastic settings. Use minimax or MCTS there.

### In games

Good fits:

- **Puzzle and level validation.** Check that a generated level is solvable (keys, doors, switches, block pushing) and get a witness plan.
- **Goal-oriented NPC planning (GOAP-style).** Features are world-state facts, and you don't need a hand-tuned heuristic per goal.
- **Automated playtesting on a simulator.** Width-based search plays Atari games from the emulator RAM bytes alone (Lipovetzky, Ramirez & Geffner, IJCAI 2015).
- **Exploration bots.** Novelty pruning visits new rooms, items and states before it revisits old ones.

Poor fits:

- Shortest paths on a weighted grid or navmesh. A* is faster and optimal there (or JPS on uniform-cost grids).
- Opponent moves, chance, or continuous physics without a discrete feature encoding.

## Features

- `sas-plus`: `sas::SasProblem`, an adapter that runs width-based search on a [`sas_plus::Task`](https://docs.rs/sas-plus), with goal counts and relaxed plans for BFWS.

**Tested:** end to end on a Fast Downward `output.sas` (gripper: IW needs width 3 for the conjunctive goal, finding the optimal 5-step plan). Also: IW with k = #variables is complete and optimal on unit-cost random tasks; plans of every search are validated on random tasks, and both BFWS variants find one exactly when the task is solvable.

## Searches

- `iw(problem, k)` and `iterated_width(problem, max_k)`: IW(k), and IW(1), IW(2), ... until one succeeds.
- `siw(problem, max_k)`: a simplified serialized IW. Runs iterated width from the current state until `#g` drops, appends that subplan, and repeats. LAPKT's SIW tracks goal atoms instead: it keeps every achieved goal, and accepts a new one only if the full goal stays reachable without deleting achieved goals. This version accepts any drop in `#g`, so its plans can be longer. SIW is fast when goals can be reached one at a time. It gives up when no subgoal is reachable within `max_k`, either in a dead end left by an earlier subplan or because the next subgoal needs more width.
- `bfws(problem, k)`: BFWS(⟨w_{#g}, #g⟩). Novelty is computed per value of `#g`.
- `bfws_r(problem, k)`: BFWS(⟨w_{#g,#r}, #g⟩), LAPKT's `BFWS-f5`. `#r` counts how many facts of a relaxed plan the path to a state has reached. The relaxed plan is recomputed at every state where `#g` drops. Implement `RelaxedPlan` to use it; `SasProblem` does, with `h_add` best supporters.
- `rollout_iw(problem, seed)`: Rollout IW(1) (Bandres, Bonet & Geffner, AAAI 2018). It runs depth-first rollouts and prunes a node unless it reaches some feature at a smaller depth than before. The paper uses it to pick actions within a time budget. This version runs until it finds a goal or the root is solved. It reaches every width-1 goal, but its plans need not be shortest.

## Compared with LAPKT

Nodes expanded on the test fixtures, this crate / [LAPKT](https://github.com/LAPKT-dev/LAPKT-Public) 0.2.1 (Tarski grounder). The searches after IW(1) use width bound 2, the LAPKT default:

| task            | IW(1)   | IW(1), IW(2)  | SIW           | BFWS          | BFWS `#r` / f5 |
|-----------------|---------|---------------|---------------|---------------|----------------|
| gripper-2       | 6 / 6   | 13 / 13       | 13 / 13       | 7 / 4         | 7 / 4          |
| sussman-anomaly | 7 / 7   | 16 / 16       | 17 / 12       | 8 / 8         | 11 / 6         |
| gripper-4       | 10 / 10 | 72 / 72       | 71 / 71       | 29 / 43       | 27 / 33        |
| bw-large-a      | 49 / 49 | 1913 / 1914   | 74 / 16       | 70 / 48       | 66 / 102       |
| logistics-e     | 46 / 46 | 12033 / 12033 | 40238 / 39547 | 14915 / 17999 | 4416 / 4399    |

LAPKT planners: `IW_Planner --iw_bound 1` and `--iw_bound 2` (the latter runs IW(1), then IW(2)), `SIW_Planner`, and `BFWS` with `--search_type BFWS-goalcount-only` and `BFWS-f5`. `scripts/lapkt-compare.sh` reruns them, and `tests/it/lapkt_counts.rs` pins this crate's side.

IW agrees almost exactly, although LAPKT computes novelty over STRIPS atoms and this crate over Fast Downward facts. The other searches differ for three reasons. LAPKT's SIW keeps achieved goals and rejects goals whose achievement would block the rest, so it finds 3- and 7-step plans on sussman-anomaly and bw-large-a where this crate's finds 5 and 14. Tie-breaking differs: LAPKT breaks ties by path cost, this crate by generation order. And LAPKT's relaxed plans come from its own heuristic code, so `#r` counts different facts. LAPKT reports pruned nodes separately; its generated count plus pruned count is close to this crate's `generated`.

References: Lipovetzky & Geffner, ECAI 2012 and AAAI 2017; Lipovetzky, Ramirez & Geffner, IJCAI 2015; Bandres, Bonet & Geffner, AAAI 2018.

## MSRV

Rust 1.86. Raising it is a minor release.

## Contributing

See [CONTRIBUTING.md](https://github.com/sunsided/width-search/blob/main/CONTRIBUTING.md).

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/sunsided/width-search/blob/main/LICENSES/Apache-2.0.txt) or [MIT license](https://github.com/sunsided/width-search/blob/main/LICENSES/MIT.txt) at your option.
The examples in [`examples/`](https://github.com/sunsided/width-search/tree/main/examples) are public domain ([CC0-1.0](https://github.com/sunsided/width-search/blob/main/LICENSES/CC0-1.0.txt)), so you can copy them into anything.
Test fixtures carry their own licenses; see [`REUSE.toml`](https://github.com/sunsided/width-search/blob/main/REUSE.toml).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you shall be dual licensed as above, without any additional terms or conditions.
