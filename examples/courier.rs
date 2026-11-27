//! A bike courier delivers packages across town, guided by a sketch.
//!
//! The courier carries one package at a time. Instead of a goal count, a
//! *sketch* says what progress looks like, in terms of two features: `u`, the
//! number of undelivered packages, and `h`, whether the courier holds one.
//!
//! - `{¬h, u > 0} ↦ {h}`: with empty hands, picking up a package is progress;
//! - `{h} ↦ {¬h, u↓}`: holding a package, delivering it is progress.
//!
//! Features a rule does not mention must stay unchanged, so dropping a
//! package at the wrong address is not progress. Each step of the sketch has
//! width 1, so SIW-R solves every subproblem with IW(1). Run:
//! `cargo run --example courier`.

use width_search::{
    Condition, Effect, GoalCount, Rule, SearchResult, WidthProblem, bfws, iterated_width, siw,
    siw_r,
};

/// The places in town.
const PLACES: &[&str] = &[
    "depot", "market", "bakery", "park", "library", "station", "harbor", "school", "museum",
];

/// Two-way streets between [`PLACES`], by index.
const STREETS: &[(usize, usize)] = &[
    (0, 1),
    (1, 2),
    (1, 3),
    (3, 4),
    (2, 5),
    (5, 6),
    (4, 7),
    (7, 8),
    (6, 8),
    (0, 4),
];

/// The packages: pick-up place and address, by index into [`PLACES`].
const PACKAGES: &[(usize, usize)] = &[(0, 6), (0, 8), (2, 7), (5, 3)];

/// Where a package is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Place {
    /// At this place, by index.
    At(usize),
    /// On the courier's bike.
    Carried,
}

/// The courier's position and every package's place.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct State {
    /// Where the courier is, by index.
    courier: usize,
    /// Each package's place, indexed like [`PACKAGES`].
    packages: Vec<Place>,
}

/// A courier action.
#[derive(Clone, Copy, Debug)]
enum Action {
    /// Ride to the neighboring place with this index.
    Ride(usize),
    /// Pick up the package with this index.
    Pick(usize),
    /// Hand over the carried package here.
    Drop,
}

/// The town as a [`WidthProblem`].
struct Town;

impl Town {
    /// Feature id from an index; the town is small, so every id fits.
    fn id(index: usize) -> u32 {
        u32::try_from(index).expect("fewer than 2^32 features")
    }

    /// The sketch features of `s`: undelivered packages, then 1 if the
    /// courier holds a package.
    fn sketch_features(s: &State, out: &mut Vec<u64>) {
        let undelivered = s
            .packages
            .iter()
            .zip(PACKAGES)
            .filter(|&(p, &(_, to))| *p != Place::At(to))
            .count();
        out.push(u64::try_from(undelivered).expect("few packages"));
        out.push(u64::from(s.packages.contains(&Place::Carried)));
    }
}

impl WidthProblem for Town {
    type State = State;
    type Action = Action;

    fn initial_state(&self) -> State {
        State {
            courier: 0,
            packages: PACKAGES.iter().map(|&(from, _)| Place::At(from)).collect(),
        }
    }

    fn successors(&self, s: &State, out: &mut Vec<(Action, State)>) {
        for &(a, b) in STREETS {
            let next = if a == s.courier {
                b
            } else if b == s.courier {
                a
            } else {
                continue;
            };
            out.push((
                Action::Ride(next),
                State {
                    courier: next,
                    ..s.clone()
                },
            ));
        }

        match s.packages.iter().position(|&p| p == Place::Carried) {
            Some(i) => {
                let mut next = s.clone();
                next.packages[i] = Place::At(s.courier);
                out.push((Action::Drop, next));
            }
            None => {
                for (i, &p) in s.packages.iter().enumerate() {
                    if p == Place::At(s.courier) {
                        let mut next = s.clone();
                        next.packages[i] = Place::Carried;
                        out.push((Action::Pick(i), next));
                    }
                }
            }
        }
    }

    // The courier's place; per package, its place or "carried".
    fn features(&self, s: &State, out: &mut Vec<u32>) {
        let places = PLACES.len();
        out.push(Self::id(s.courier));
        for (i, p) in s.packages.iter().enumerate() {
            let place = match p {
                Place::At(at) => *at,
                Place::Carried => places,
            };
            out.push(Self::id(places + i * (places + 1) + place));
        }
    }

    fn num_features(&self) -> usize {
        PLACES.len() + PACKAGES.len() * (PLACES.len() + 1)
    }

    fn is_goal(&self, s: &State) -> bool {
        self.unsatisfied_goals(s) == 0
    }
}

impl GoalCount for Town {
    fn unsatisfied_goals(&self, s: &State) -> usize {
        let mut values = Vec::new();
        Self::sketch_features(s, &mut values);
        usize::try_from(values[0]).expect("few packages")
    }
}

/// Prints one row of the comparison table.
fn report(name: &str, r: &SearchResult<Action>) {
    let plan = r
        .plan
        .as_ref()
        .map_or("no plan".to_string(), |p| format!("{} steps", p.len()));
    println!("{name:<22} {plan:>10} {:>8} expanded", r.expanded);
}

/// The sketch from the module docs, over feature 0 (`u`) and 1 (`h`).
fn sketch() -> Vec<Rule> {
    let (u, h) = (0, 1);
    vec![
        Rule {
            conditions: vec![Condition::Zero(h), Condition::Positive(u)],
            effects: vec![Effect::Positive(h)],
        },
        Rule {
            conditions: vec![Condition::Positive(h)],
            effects: vec![Effect::Zero(h), Effect::Decrease(u)],
        },
    ]
}

/// Compares SIW-R with the sketch against goal-count searches.
fn main() {
    let (_, r) = iterated_width(&Town, 2);
    report("IW(1), IW(2)", &r);
    report("SIW, max width 1", &siw(&Town, 1));
    report("BFWS(<w_#g, #g>), k=1", &bfws(&Town, 1));
    let r = siw_r(&Town, &sketch(), Town::sketch_features, 1);
    report("SIW-R, IW(1) per rule", &r);

    println!("\nSIW-R route:");
    for a in r.plan.expect("the sketch has width 1") {
        match a {
            Action::Ride(to) => println!("  ride to {}", PLACES[to]),
            Action::Pick(i) => println!("  pick up package {i}"),
            Action::Drop => println!("  deliver"),
        }
    }
}
