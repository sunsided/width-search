use std::collections::VecDeque;

use crate::{Node, NoveltyTable, SearchResult, WidthProblem, extract_plan};

/// Hierarchical IW, HIW(`w_high`, `w_low`) (Junyent, Jonsson & Gómez,
/// ICAPS 2021): IW over high-level features, where each high-level state is
/// explored by its own low-level IW search.
///
/// `high` pushes the high-level feature ids (`< num_high`) of a state; the
/// low level uses [`WidthProblem::features`]. A high-level node is a state
/// whose high-level features differ from its parent node's. The high level
/// is a breadth-first IW(`w_high`) over these nodes, with novelty computed on
/// high-level features. Expanding a high-level node runs IW(`w_low`) from its
/// state, with a novelty table of its own:
///
/// - a generated state with the same high-level features as the node is
///   kept or pruned by low-level novelty, as in [`iw`](fn@crate::iw);
/// - a generated state with different high-level features is not expanded
///   at the low level; it becomes a new high-level node if its high-level
///   features are novel (width at most `w_high`), and is pruned otherwise.
///
/// The search returns at the first goal state generated, at either level,
/// and gives up (`plan: None`) once every high-level node is expanded.
/// Without high-level features, HIW(`w_high`, `w_low`) is IW(`w_low`).
///
/// The paper shows that if every high-level feature is splitting (the
/// states sharing a value form a connected graph), HIW(`w_high`, `w_low`)
/// is a restricted IW(`w_high + w_low`), using `w_high`-tuples of high-level
/// and `w_low`-tuples of low-level features. So good high-level features let
/// it solve problems of width `w_high + w_low` while it expands at most
/// `O(n_high^w_high · n_low^w_low)` states. Its search is resumed lazily in
/// the paper; this version expands each high-level node to the end, in
/// breadth-first order.
///
/// # Panics
///
/// Panics if `w_high` or `w_low` is 0, or `high` pushes an id not below
/// `num_high`.
pub fn hiw<P, H>(
    problem: &P,
    high: H,
    num_high: usize,
    w_high: usize,
    w_low: usize,
) -> SearchResult<P::Action>
where
    P: WidthProblem,
    H: Fn(&P::State, &mut Vec<u32>),
{
    assert!(w_high >= 1 && w_low >= 1, "width bound must be at least 1");
    let mut search = Search {
        problem,
        high,
        high_table: NoveltyTable::new(num_high, w_high),
        w_low,
        nodes: Vec::new(),
        open: VecDeque::new(),
        feats: Vec::new(),
        succ: Vec::new(),
        result: SearchResult {
            plan: None,
            expanded: 0,
            generated: 1,
        },
    };

    let root = problem.initial_state();
    let root_high = search.high_features(&root);
    search.high_table.evaluate_and_insert(&root_high);
    if problem.is_goal(&root) {
        search.result.plan = Some(Vec::new());
        return search.result;
    }

    search.nodes.push(Node {
        state: root,
        parent: None,
        action: None,
    });
    search.open.push_back((0, root_high));
    while let Some((n, features)) = search.open.pop_front() {
        if let Some(goal) = search.explore(n, &features) {
            search.result.plan = Some(extract_plan(&search.nodes, goal));
            break;
        }
    }

    search.result
}

/// The state of one HIW run.
struct Search<'p, P: WidthProblem, H> {
    /// The problem being searched.
    problem: &'p P,
    /// Computes the high-level features of a state.
    high: H,
    /// Novelty of high-level feature tuples, shared by the whole run.
    high_table: NoveltyTable,
    /// Width bound of every low-level search.
    w_low: usize,
    /// Node arena of all low-level trees; parent links cross between trees,
    /// so a plan is one path back to the root.
    nodes: Vec<Node<P::State, P::Action>>,
    /// High-level nodes not yet expanded, breadth-first: the arena index of
    /// each one's state and its sorted high-level features.
    open: VecDeque<(usize, Vec<u32>)>,
    /// Reused buffer for the low-level features of a state.
    feats: Vec<u32>,
    /// Reused buffer for successors.
    succ: Vec<(P::Action, P::State)>,
    /// Node counts so far; `plan` is filled in by [`hiw`].
    result: SearchResult<P::Action>,
}

impl<P, H> Search<'_, P, H>
where
    P: WidthProblem,
    H: Fn(&P::State, &mut Vec<u32>),
{
    /// The high-level features of `state`, sorted and deduplicated so that
    /// two states compare equal exactly when they share the same set.
    fn high_features(&self, state: &P::State) -> Vec<u32> {
        let mut out = Vec::new();
        (self.high)(state, &mut out);
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Expands the high-level node at arena index `root`, whose high-level
    /// features are `features`, by a low-level IW search from its state.
    /// Queues the novel high-level nodes it meets; returns the arena index
    /// of a goal node if one was generated.
    fn explore(&mut self, root: usize, features: &[u32]) -> Option<usize> {
        let mut table = NoveltyTable::new(self.problem.num_features(), self.w_low);
        self.feats.clear();
        self.problem
            .features(&self.nodes[root].state, &mut self.feats);
        table.evaluate_and_insert(&self.feats);

        let mut open = VecDeque::from([root]);
        let mut succ = std::mem::take(&mut self.succ);
        while let Some(i) = open.pop_front() {
            self.result.expanded += 1;
            succ.clear();
            self.problem.successors(&self.nodes[i].state, &mut succ);
            for (action, state) in succ.drain(..) {
                self.result.generated += 1;
                if let Some(next) = self.classify(&state, features, &mut table) {
                    let j = self.push(state, i, action);
                    match next {
                        Next::Goal => return Some(j),
                        Next::Low => open.push_back(j),
                        Next::High(high) => self.open.push_back((j, high)),
                    }
                }
            }
        }

        self.succ = succ;
        None
    }

    /// Where a generated `state` goes, seen from a high-level node with
    /// high-level features `features` and low-level novelty table `table`;
    /// `None` if it is pruned. Updates the novelty tables it consults.
    fn classify(
        &mut self,
        state: &P::State,
        features: &[u32],
        table: &mut NoveltyTable,
    ) -> Option<Next> {
        if self.problem.is_goal(state) {
            return Some(Next::Goal);
        }

        let high = self.high_features(state);
        if high != features {
            return self
                .high_table
                .evaluate_and_insert(&high)
                .map(|_| Next::High(high));
        }

        self.feats.clear();
        self.problem.features(state, &mut self.feats);
        table.evaluate_and_insert(&self.feats).map(|_| Next::Low)
    }

    /// Appends a node reached from arena index `parent` and returns its index.
    fn push(&mut self, state: P::State, parent: usize, action: P::Action) -> usize {
        self.nodes.push(Node {
            state,
            parent: Some(parent),
            action: Some(action),
        });

        self.nodes.len() - 1
    }
}

/// What happens to a generated state that survived pruning.
enum Next {
    /// It is a goal: the search ends.
    Goal,
    /// It is novel at the low level: the current low-level search expands it.
    Low,
    /// It starts a new high-level node with these sorted high-level features.
    High(Vec<u32>),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The corridor of the paper: positions `0..=len`, a key at the far end,
    /// and the goal at position 0 with the key. The low-level feature is the
    /// position only, so revisiting position 0 with the key is not novel and
    /// IW(1) fails.
    struct Corridor {
        /// The last position, where the key lies.
        len: u32,
    }

    impl WidthProblem for Corridor {
        /// Position and whether the key is held.
        type State = (u32, bool);
        type Action = i8;

        fn initial_state(&self) -> (u32, bool) {
            (0, false)
        }
        fn successors(&self, &(p, key): &(u32, bool), out: &mut Vec<(i8, (u32, bool))>) {
            if p > 0 {
                out.push((-1, (p - 1, key)));
            }
            if p < self.len {
                out.push((1, (p + 1, key || p + 1 == self.len)));
            }
        }
        fn features(&self, &(p, _): &(u32, bool), out: &mut Vec<u32>) {
            out.push(p);
        }
        fn num_features(&self) -> usize {
            self.len as usize + 1
        }
        fn is_goal(&self, &(p, key): &(u32, bool)) -> bool {
            p == 0 && key
        }
    }

    /// The key flag as the one high-level feature.
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "matches the high-level feature closure signature"
    )]
    fn key(&(_, key): &(u32, bool), out: &mut Vec<u32>) {
        if key {
            out.push(0);
        }
    }

    /// HIW(1, 1) with the key as high-level feature solves the corridor in
    /// two low-level searches, one per key value, where IW(1) fails.
    #[test]
    fn the_key_splits_the_corridor() {
        let corridor = Corridor { len: 5 };
        let r = hiw(&corridor, key, 1, 1, 1);

        assert!(crate::iw(&corridor, 1).plan.is_none());
        assert_eq!(
            r.plan.as_deref(),
            Some(&[1, 1, 1, 1, 1, -1, -1, -1, -1, -1][..])
        );
        assert_eq!(r.expanded, 10, "positions 0..5 without, 5..1 with the key");
    }

    /// A high-level feature that never changes leaves one high-level node,
    /// so HIW(1, 1) is IW(1) and fails on the corridor too.
    #[test]
    fn a_constant_high_level_feature_is_iw() {
        let corridor = Corridor { len: 3 };
        let constant = |_: &(u32, bool), out: &mut Vec<u32>| out.push(0);
        let r = hiw(&corridor, constant, 1, 1, 1);

        let iw = crate::iw(&corridor, 1);
        assert!(r.plan.is_none());
        assert_eq!((r.expanded, r.generated), (iw.expanded, iw.generated));
    }

    /// Width 0 at the high level is rejected up front, before any table is
    /// built.
    #[test]
    #[should_panic(expected = "width bound must be at least 1")]
    fn rejects_high_width_zero() {
        hiw(&Corridor { len: 0 }, key, 1, 0, 1);
    }

    /// Width 0 at the low level is rejected even when the root is a goal.
    #[test]
    #[should_panic(expected = "width bound must be at least 1")]
    fn rejects_low_width_zero() {
        hiw(&Corridor { len: 0 }, key, 1, 1, 0);
    }
}
