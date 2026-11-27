use std::collections::HashSet;

use super::run;
use crate::rng::SplitMix64;
use crate::{Node, SearchResult, WidthProblem};

/// Incremental HIW, IHIW(`w_high`, `w_low`) (Junyent, Jonsson & Gómez,
/// ICAPS 2021, Algorithms 1 and 2): [`hiw`](super::hiw) that picks its own
/// high-level features from the problem's features.
///
/// It runs a series of HIW searches. The first has no high-level features,
/// so it is IW(`w_low`). After a failed search, its pruned states join a
/// pool. IHIW draws pruned states from the pool at random until one yields
/// candidate features, then promotes one random candidate to a high-level
/// feature and searches again. A high-level feature `f` here means "feature
/// `f` is true", so the high-level features of a state are its features that
/// were promoted so far.
///
/// The candidates of a pruned state `n` with parent `p` (Algorithm 1) are the
/// features true in both `n` and `p` that are true in no state on the path
/// above `p`, the root included: a feature that changed once, just before
/// the branch was pruned, likely splits the state space (like picking up a
/// key). `p` must not be the root, and `n` must differ from `p` in some
/// feature; otherwise `n` has no candidates.
///
/// The search returns the first plan found and gives up (`plan: None`) when
/// the pool runs out of states with new candidates. It promotes at most one
/// feature per round, so it runs at most `num_features() + 1` searches. The
/// paper restructures the previous search tree to reuse it; this version
/// starts every search afresh, which costs nodes but finds the same plans.
/// Node counts add up over all searches, and every search's tree is kept
/// until the end, since pruned states from any round may be drawn. `seed`
/// fixes the random draws.
///
/// # Panics
///
/// Panics if `w_high` or `w_low` is 0.
pub fn ihiw<P: WidthProblem>(
    problem: &P,
    w_high: usize,
    w_low: usize,
    seed: u64,
) -> SearchResult<P::Action> {
    assert!(w_high >= 1, "width bound must be at least 1");
    let mut rng = SplitMix64::new(seed);
    let mut chosen = vec![false; problem.num_features()];
    let mut total = SearchResult {
        plan: None,
        expanded: 0,
        generated: 0,
    };
    let mut trees = Vec::new();
    let mut pool = Vec::new();
    let mut candidates = Vec::new();

    loop {
        let high = |s: &P::State, out: &mut Vec<u32>| {
            problem.features(s, out);
            out.retain(|&f| chosen[f as usize]);
        };
        let run = run(problem, high, chosen.len(), w_high, w_low, true);
        total.expanded += run.result.expanded;
        total.generated += run.result.generated;
        if run.result.plan.is_some() {
            total.plan = run.result.plan;
            return total;
        }

        let round = trees.len();
        pool.extend(
            run.pruned
                .into_iter()
                .map(|(parent, state)| (round, parent, state)),
        );
        trees.push(run.nodes);

        let Some(feature) = next_feature(
            problem,
            &trees,
            &mut pool,
            &mut candidates,
            &chosen,
            &mut rng,
        ) else {
            return total;
        };
        chosen[feature as usize] = true;
    }
}

/// Algorithm 2's choice of the next high-level feature: a random candidate
/// not chosen yet, refilling the candidates from random pruned states of the
/// `pool` (round, parent index, state) while none is left. `None` once the
/// pool is empty too.
fn next_feature<P: WidthProblem>(
    problem: &P,
    trees: &[Vec<Node<P::State, P::Action>>],
    pool: &mut Vec<(usize, usize, P::State)>,
    candidates: &mut Vec<u32>,
    chosen: &[bool],
    rng: &mut SplitMix64,
) -> Option<u32> {
    loop {
        candidates.retain(|&f| !chosen[f as usize]);
        if !candidates.is_empty() {
            return Some(candidates.swap_remove(rng.below(candidates.len())));
        }

        if pool.is_empty() {
            return None;
        }

        let (round, parent, state) = pool.swap_remove(rng.below(pool.len()));
        *candidates = feature_candidates(problem, &trees[round], parent, &state);
    }
}

/// Algorithm 1: the candidate high-level features of a pruned `state` whose
/// parent is `nodes[parent]`, sorted.
fn feature_candidates<P: WidthProblem>(
    problem: &P,
    nodes: &[Node<P::State, P::Action>],
    parent: usize,
    state: &P::State,
) -> Vec<u32> {
    // The paper's `Depth(n) > 2` counts the nodes on the branch, root
    // included: the parent must not be the root.
    if nodes[parent].parent.is_none() {
        return Vec::new();
    }

    let own = sorted_features(problem, state);
    let from_parent = sorted_features(problem, &nodes[parent].state);
    let mut common: Vec<u32> = own
        .iter()
        .copied()
        .filter(|f| from_parent.binary_search(f).is_ok())
        .collect();
    if common.len() == own.len() {
        return Vec::new();
    }

    let mut above = HashSet::new();
    let mut i = nodes[parent].parent;
    while let Some(n) = i {
        above.extend(sorted_features(problem, &nodes[n].state));
        i = nodes[n].parent;
    }

    common.retain(|f| !above.contains(f));
    common
}

/// The features of `state`, sorted and deduplicated.
fn sorted_features<P: WidthProblem>(problem: &P, state: &P::State) -> Vec<u32> {
    let mut out = Vec::new();
    problem.features(state, &mut out);
    out.sort_unstable();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hiw::tests::Corridor;

    /// The paper's corridor, with holding the key as a feature of its own.
    fn corridor(len: u32) -> Corridor {
        Corridor {
            key_feature: true,
            ..Corridor::new(len)
        }
    }

    /// IW(1) fails on the corridor; IHIW(1, 1) finds the key feature in the
    /// first failed search and solves it in the second, for every seed.
    #[test]
    fn discovers_the_key_as_high_level_feature() {
        let problem = corridor(5);
        assert!(crate::iw(&problem, 1).plan.is_none());

        for seed in 0..20 {
            let r = ihiw(&problem, 1, 1, seed);
            assert_eq!(
                r.plan.as_deref(),
                Some(&[1, 1, 1, 1, 1, -1, -1, -1, -1, -1][..]),
                "seed {seed}"
            );
        }
    }

    /// Only the state pruned after picking up the key has a candidate: the
    /// key, true in it and its parent but nowhere above.
    #[test]
    fn candidates_are_features_that_changed_once_before_the_prune() {
        let problem = corridor(5);
        let run = run(
            &problem,
            |_: &(u32, bool), _: &mut Vec<u32>| {},
            7,
            1,
            1,
            true,
        );
        let found: Vec<Vec<u32>> = run
            .pruned
            .iter()
            .map(|(parent, state)| feature_candidates(&problem, &run.nodes, *parent, state))
            .collect();

        assert_eq!(found.iter().filter(|c| !c.is_empty()).count(), 1);
        assert!(found.contains(&vec![6]), "the key feature, id len + 1");
    }

    /// Without any splitting feature to find, IHIW gives up once the pool
    /// is empty instead of looping.
    #[test]
    fn gives_up_without_candidates() {
        let r = ihiw(&Corridor::new(5), 1, 1, 0);

        assert!(r.plan.is_none());
    }
}
