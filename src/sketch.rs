use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::Hash;

use crate::siw::{Subgoals, serialize};
use crate::{SearchResult, WidthProblem};

/// A condition of a sketch [`Rule`] on the feature values of the state a
/// subproblem starts from. The `usize` is the feature's index.
///
/// Boolean features are 0 (false) or 1 (true), so `Zero` reads as ¬p and
/// `Positive` as p.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    /// The feature is 0 (`n = 0`, or ¬p).
    Zero(usize),
    /// The feature is above 0 (`n > 0`, or p).
    Positive(usize),
}

/// An effect of a sketch [`Rule`]: how a feature may change between the
/// start state of a subproblem and its subgoal. The `usize` is the
/// feature's index. Features no effect mentions must keep their value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// The feature ends above 0 (p).
    Positive(usize),
    /// The feature ends at 0 (¬p).
    Zero(usize),
    /// The feature ends higher than it started (`n↑`).
    Increase(usize),
    /// The feature ends lower than it started (`n↓`).
    Decrease(usize),
    /// The feature may take any value (`p?`, `n?`).
    Any(usize),
}

impl Effect {
    /// The index of the feature this effect is about.
    fn feature(self) -> usize {
        match self {
            Self::Positive(f)
            | Self::Zero(f)
            | Self::Increase(f)
            | Self::Decrease(f)
            | Self::Any(f) => f,
        }
    }

    /// Whether a change of its feature from `from` to `to` satisfies this effect.
    fn allows(self, from: u64, to: u64) -> bool {
        match self {
            Self::Positive(_) => to > 0,
            Self::Zero(_) => to == 0,
            Self::Increase(_) => to > from,
            Self::Decrease(_) => to < from,
            Self::Any(_) => true,
        }
    }
}

/// A sketch rule `C ↦ E` (Bonet & Geffner): from a state that satisfies
/// every condition in `C`, any state whose feature values changed as `E`
/// says is a subgoal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rule {
    /// Conditions on the start state; all must hold. Empty means always.
    pub conditions: Vec<Condition>,
    /// Allowed changes. Features not mentioned here must stay the same; a
    /// feature mentioned twice must satisfy both effects.
    pub effects: Vec<Effect>,
}

impl Rule {
    /// Whether the pair of feature vectors `(from, to)` is compatible with
    /// this rule.
    ///
    /// # Panics
    ///
    /// Panics if the rule names a feature index outside `from`.
    fn allows(&self, from: &[u64], to: &[u64]) -> bool {
        let conditions = self.conditions.iter().all(|c| match *c {
            Condition::Zero(f) => value(from, f) == 0,
            Condition::Positive(f) => value(from, f) > 0,
        });
        if !conditions {
            return false;
        }

        let effects = self
            .effects
            .iter()
            .all(|e| e.allows(value(from, e.feature()), value(to, e.feature())));
        let others_unchanged = (0..from.len())
            .filter(|&f| self.effects.iter().all(|e| e.feature() != f))
            .all(|f| from[f] == to[f]);

        effects && others_unchanged
    }
}

/// The value of feature `f` in `values`.
///
/// # Panics
///
/// Panics if `f` is out of range, with a message naming the sketch.
fn value(values: &[u64], f: usize) -> u64 {
    *values.get(f).unwrap_or_else(|| {
        panic!(
            "sketch rule names feature {f}, but only {} features are defined",
            values.len()
        )
    })
}

/// SIW-R (Drexler, Seipp & Geffner, KR 2021): serialized IW with subgoals
/// given by a sketch.
///
/// `features` pushes the sketch feature values of a state, one `u64` per
/// feature in a fixed order: 0 or 1 for Boolean features, counts for
/// numerical ones. These are separate from the novelty features of
/// [`WidthProblem::features`]. From the current state `s`, IW(1), IW(2), ...
/// up to `max_k` run until they reach a goal state, or a state `s' != s` such
/// that `(s, s')` is compatible with some rule in `rules`. The subplans are
/// concatenated.
///
/// With one numerical feature `#g` (unsatisfied goals) and the single rule
/// `{#g > 0} ↦ {#g↓}`, this is [`siw`](fn@crate::siw). Good sketches split a
/// problem into subproblems of low width; with an empty sketch, only goal
/// states are subgoals.
///
/// SIW-R is incomplete: it gives up (`plan: None`) when no subgoal is
/// reachable within width `max_k`, and when it reaches a subgoal state it has
/// already passed through, since the search is deterministic and would cycle
/// (the sketch is not terminating). Node counts add up over all IW runs.
///
/// # Panics
///
/// Panics if a rule names a feature index that `features` does not produce,
/// or if `features` pushes different numbers of values for different states.
pub fn siw_r<P, F>(
    problem: &P,
    rules: &[Rule],
    features: F,
    max_k: usize,
) -> SearchResult<P::Action>
where
    P: WidthProblem,
    P::State: Hash + Eq,
    F: Fn(&P::State, &mut Vec<u64>),
{
    let subgoals = SketchSubgoals {
        problem,
        rules,
        features,
        passed: HashSet::from([problem.initial_state()]),
        buf: RefCell::new(Vec::new()),
    };

    serialize(
        problem,
        max_k,
        None::<fn(&P::State, &mut Vec<u32>)>,
        subgoals,
    )
}

/// The subgoals of [`siw_r`]: goal states, and states a sketch rule allows.
struct SketchSubgoals<'p, P: WidthProblem, F> {
    /// The problem being searched, for its goal test.
    problem: &'p P,
    /// The sketch.
    rules: &'p [Rule],
    /// Computes the sketch feature values of a state.
    features: F,
    /// Every state a subproblem started from so far, the initial one included.
    passed: HashSet<P::State>,
    /// Reused buffer for the feature values of a candidate subgoal, so the
    /// per-state test does not allocate.
    buf: RefCell<Vec<u64>>,
}

impl<P, F> SketchSubgoals<'_, P, F>
where
    P: WidthProblem,
    F: Fn(&P::State, &mut Vec<u64>),
{
    /// The sketch feature values of `state`, in a new vector.
    fn values(&self, state: &P::State) -> Vec<u64> {
        let mut out = Vec::new();
        (self.features)(state, &mut out);
        out
    }

    /// Whether some rule allows the change from `from_values` to the
    /// feature values of `state`, computed into the reused buffer.
    fn allows(&self, from_values: &[u64], state: &P::State) -> bool {
        let mut buf = self.buf.borrow_mut();
        buf.clear();
        (self.features)(state, &mut buf);
        allowed(self.rules, from_values, &buf)
    }
}

impl<P, F> Subgoals<P::State> for SketchSubgoals<'_, P, F>
where
    P: WidthProblem,
    P::State: Hash + Eq,
    F: Fn(&P::State, &mut Vec<u64>),
{
    /// The start state and its sketch feature values.
    type Anchor = (P::State, Vec<u64>);

    fn anchor(&mut self, from: &P::State) -> Self::Anchor {
        (from.clone(), self.values(from))
    }

    fn reached(&self, (from, from_values): &Self::Anchor, state: &P::State) -> bool {
        self.problem.is_goal(state) || (state != from && self.allows(from_values, state))
    }

    /// A subgoal passed before means the deterministic search would cycle.
    fn advance(&mut self, state: &P::State) -> bool {
        self.passed.insert(state.clone())
    }
}

/// Whether some rule allows the change from `from` to `to`.
///
/// # Panics
///
/// Panics if the two vectors differ in length, or a rule names a feature
/// outside them.
fn allowed(rules: &[Rule], from: &[u64], to: &[u64]) -> bool {
    assert_eq!(
        from.len(),
        to.len(),
        "sketch features must push the same number of values for every state"
    );
    rules.iter().any(|r| r.allows(from, to))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Walking from 0 to `len` on a line, one step either way. The only
    /// sketch feature is the position.
    struct Line {
        /// The goal position, also the largest one.
        len: u32,
    }

    impl WidthProblem for Line {
        type State = u32;
        type Action = i8;

        fn initial_state(&self) -> u32 {
            0
        }
        fn successors(&self, s: &u32, out: &mut Vec<(i8, u32)>) {
            if *s > 0 {
                out.push((-1, s - 1));
            }
            if *s < self.len {
                out.push((1, s + 1));
            }
        }
        fn features(&self, s: &u32, out: &mut Vec<u32>) {
            out.push(*s);
        }
        fn num_features(&self) -> usize {
            self.len as usize + 1
        }
        fn is_goal(&self, s: &u32) -> bool {
            *s == self.len
        }
    }

    /// The position as the single sketch feature.
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "matches the sketch feature closure signature"
    )]
    fn position(s: &u32, out: &mut Vec<u64>) {
        out.push(u64::from(*s));
    }

    /// `{} ↦ {n↑}`: every step to the right is a subgoal, one IW(1) run each.
    #[test]
    fn an_increasing_sketch_steps_to_the_goal() {
        let rules = [Rule {
            conditions: vec![],
            effects: vec![Effect::Increase(0)],
        }];
        let r = siw_r(&Line { len: 6 }, &rules, position, 1);

        assert_eq!(r.plan.as_deref(), Some(&[1; 6][..]));
    }

    /// `{} ↦ {n?}` accepts any other state, so the search steps back and
    /// forth between 0 and 1; it gives up when 0 comes around again.
    #[test]
    fn a_cycling_sketch_gives_up() {
        let rules = [Rule {
            conditions: vec![],
            effects: vec![Effect::Any(0)],
        }];
        let r = siw_r(&Line { len: 6 }, &rules, position, 1);

        assert!(r.plan.is_none());
    }

    /// A condition that never holds leaves only the goal as a subgoal.
    #[test]
    fn unmet_conditions_leave_only_the_goal() {
        let rules = [Rule {
            conditions: vec![Condition::Zero(0), Condition::Positive(0)],
            effects: vec![Effect::Any(0)],
        }];
        let r = siw_r(&Line { len: 4 }, &rules, position, 1);

        assert_eq!(r.plan.as_deref(), Some(&[1; 4][..]));
    }

    /// The effect kinds on single values, and unmentioned features.
    #[test]
    fn rule_compatibility() {
        let rule = |effects| Rule {
            conditions: vec![],
            effects,
        };

        assert!(rule(vec![Effect::Positive(0)]).allows(&[0, 5], &[2, 5]));
        assert!(!rule(vec![Effect::Positive(0)]).allows(&[0, 5], &[2, 6]));
        assert!(rule(vec![Effect::Zero(0)]).allows(&[3], &[0]));
        assert!(rule(vec![Effect::Decrease(0)]).allows(&[3], &[2]));
        assert!(!rule(vec![Effect::Decrease(0)]).allows(&[3], &[3]));
        assert!(rule(vec![]).allows(&[3, 1], &[3, 1]));
    }

    /// A rule naming a feature that does not exist is a sketch bug.
    #[test]
    #[should_panic(expected = "names feature 3")]
    fn rejects_unknown_features() {
        let rules = [Rule {
            conditions: vec![Condition::Positive(3)],
            effects: vec![],
        }];
        siw_r(&Line { len: 2 }, &rules, position, 1);
    }
}
