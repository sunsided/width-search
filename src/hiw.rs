use std::collections::VecDeque;

use crate::{Node, NoveltyTable, SearchResult, WidthProblem, extract_plan, sorted_set};

/// Incremental HIW: discovers high-level features from pruned nodes.
mod incremental;

/// π-HIW: count-based Rollout IW over high-level states, π-IW below.
mod policy;

pub use incremental::ihiw;
pub use policy::pi_hiw;

/// Hierarchical IW, HIW(`w_high`, `w_low`) (Junyent, Jonsson & Gómez,
/// ICAPS 2021): IW over high-level features, where each high-level state is
/// explored by its own low-level IW search.
///
/// `high` pushes the high-level feature ids (`< num_high`) of a state; the
/// low level uses [`WidthProblem::features`]. A high-level node is a state
/// whose high-level features differ from its parent node's. The high level
/// is an IW(`w_high`) over these nodes, with novelty computed on
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
/// `O(n_high^w_high · n_low^w_low)` states.
///
/// As in the paper, low-level searches are resumed lazily. Each high-level
/// node keeps its low-level search; asking it for a successor resumes that
/// search until it meets the next new high-level node, which is returned,
/// or runs out. The high level asks its nodes for successors one at a time,
/// round-robin: a node that returned a successor goes to the back of the
/// queue, behind that successor. So a goal under an early high-level
/// successor is found before the rest of its parent's low-level search runs.
/// The order is not breadth-first: a node two high-level steps from the root
/// can be generated before its parent's later siblings, and then claim
/// high-level tuples first, so those siblings are pruned.
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
    run(problem, high, num_high, w_high, w_low, false).result
}

/// What one HIW run leaves behind, for [`ihiw`].
pub(crate) struct Run<P: WidthProblem> {
    /// Plan and node counts.
    pub result: SearchResult<P::Action>,
    /// The node arena of every low-level tree, linked by parent indices.
    pub nodes: Vec<Node<P::State, P::Action>>,
    /// Every pruned state with the arena index of its parent; empty unless
    /// requested.
    pub pruned: Vec<(usize, P::State)>,
}

/// The search behind [`hiw`]; also logs pruned states when `log_pruned` is
/// set, which [`ihiw`] needs.
///
/// # Panics
///
/// As [`hiw`].
pub(crate) fn run<P, H>(
    problem: &P,
    high: H,
    num_high: usize,
    w_high: usize,
    w_low: usize,
    log_pruned: bool,
) -> Run<P>
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
        log_pruned,
        pruned: Vec::new(),
        feats: Vec::new(),
        result: SearchResult {
            plan: None,
            expanded: 0,
            generated: 1,
        },
    };

    let root = problem.initial_state();
    let root_high = sorted_set(|out| (search.high)(&root, out));
    search.high_table.evaluate_and_insert(&root_high);
    if problem.is_goal(&root) {
        search.result.plan = Some(Vec::new());
        return search.finish();
    }

    search.nodes.push(Node {
        state: root,
        parent: None,
        action: None,
    });
    let mut open = VecDeque::from([LowSearch::new(&mut search, 0, root_high)]);
    while let Some(mut low) = open.pop_front() {
        match low.next_high(&mut search) {
            Step::Goal(goal) => {
                search.result.plan = Some(extract_plan(&search.nodes, goal));
                break;
            }
            Step::High(node, features) => {
                open.push_back(LowSearch::new(&mut search, node, features));
                open.push_back(low);
            }
            Step::Done => {}
        }
    }

    search.finish()
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
    /// Whether pruned states go into `pruned`.
    log_pruned: bool,
    /// Pruned states with their parents' arena indices, if `log_pruned`.
    pruned: Vec<(usize, P::State)>,
    /// Reused buffer for the low-level features of a state.
    feats: Vec<u32>,
    /// Node counts so far; `plan` is filled in by [`hiw`].
    result: SearchResult<P::Action>,
}

impl<P, H> Search<'_, P, H>
where
    P: WidthProblem,
    H: Fn(&P::State, &mut Vec<u32>),
{
    /// Ends the run, handing over what [`Run`] keeps.
    fn finish(self) -> Run<P> {
        Run {
            result: self.result,
            nodes: self.nodes,
            pruned: self.pruned,
        }
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

        let high = sorted_set(|out| (self.high)(state, out));
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

/// The resumable low-level IW search of one high-level node.
struct LowSearch<P: WidthProblem> {
    /// The node's sorted high-level features; states with others leave it.
    features: Vec<u32>,
    /// Low-level novelty of this search only.
    table: NoveltyTable,
    /// Arena indices of low-level nodes not yet expanded, breadth-first.
    open: VecDeque<usize>,
    /// Arena index of the node whose successors are being processed.
    parent: usize,
    /// Successors of `parent` not yet processed, in reverse order, so a
    /// search paused mid-expansion resumes with the next one.
    pending: Vec<(P::Action, P::State)>,
}

impl<P: WidthProblem> LowSearch<P> {
    /// A search from the high-level node at arena index `root`, with sorted
    /// high-level features `features`; records the root's features.
    fn new<H>(search: &mut Search<'_, P, H>, root: usize, features: Vec<u32>) -> Self {
        let mut table = NoveltyTable::new(search.problem.num_features(), search.w_low);
        search.feats.clear();
        search
            .problem
            .features(&search.nodes[root].state, &mut search.feats);
        table.evaluate_and_insert(&search.feats);

        Self {
            features,
            table,
            open: VecDeque::from([root]),
            parent: root,
            pending: Vec::new(),
        }
    }

    /// The high-level successor function: resumes the search until it
    /// generates a goal or a novel high-level node, or runs out.
    fn next_high<H>(&mut self, search: &mut Search<'_, P, H>) -> Step
    where
        H: Fn(&P::State, &mut Vec<u32>),
    {
        loop {
            if self.pending.is_empty() && !self.expand_next(search) {
                return Step::Done;
            }

            while let Some((action, state)) = self.pending.pop() {
                search.result.generated += 1;
                let Some(next) = search.classify(&state, &self.features, &mut self.table) else {
                    if search.log_pruned {
                        search.pruned.push((self.parent, state));
                    }
                    continue;
                };

                let j = search.push(state, self.parent, action);
                match next {
                    Next::Goal => return Step::Goal(j),
                    Next::Low => self.open.push_back(j),
                    Next::High(features) => return Step::High(j, features),
                }
            }
        }
    }

    /// Expands the next low-level node into `pending`; `false` if none is
    /// left.
    fn expand_next<H>(&mut self, search: &mut Search<'_, P, H>) -> bool {
        let Some(i) = self.open.pop_front() else {
            return false;
        };

        search.result.expanded += 1;
        self.parent = i;
        search
            .problem
            .successors(&search.nodes[i].state, &mut self.pending);
        self.pending.reverse();
        true
    }
}

/// What a high-level successor call found.
enum Step {
    /// A goal node, at this arena index.
    Goal(usize),
    /// A new high-level node at this arena index, with these sorted features.
    High(usize, Vec<u32>),
    /// Nothing more: the low-level search is exhausted.
    Done,
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
pub(crate) mod tests {
    use super::*;

    /// The corridor of the paper: positions `0..=len`, a key at `key_at`
    /// (the far end in the paper), and the goal at position 0 with the key. The low-level feature is the
    /// position only, so revisiting position 0 with the key is not novel and
    /// IW(1) fails.
    pub(crate) struct Corridor {
        /// The last position.
        pub len: u32,
        /// Where the key lies; picked up on entering that position.
        pub key_at: u32,
        /// Whether holding the key is a low-level feature too (id `len + 1`),
        /// as atoms of a planning task would make it.
        pub key_feature: bool,
    }

    impl Corridor {
        /// A corridor with the key at its far end, as in the paper.
        pub fn new(len: u32) -> Self {
            Self {
                len,
                key_at: len,
                key_feature: false,
            }
        }
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
                out.push((-1, (p - 1, key || p - 1 == self.key_at)));
            }
            if p < self.len {
                out.push((1, (p + 1, key || p + 1 == self.key_at)));
            }
        }
        fn features(&self, &(p, key): &(u32, bool), out: &mut Vec<u32>) {
            out.push(p);
            if key && self.key_feature {
                out.push(self.len + 1);
            }
        }
        fn num_features(&self) -> usize {
            self.len as usize + 2
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
        let corridor = Corridor::new(5);
        let r = hiw(&corridor, key, 1, 1, 1);

        assert!(crate::iw(&corridor, 1).plan.is_none());
        assert_eq!(
            r.plan.as_deref(),
            Some(&[1, 1, 1, 1, 1, -1, -1, -1, -1, -1][..])
        );
        assert_eq!(r.expanded, 10, "positions 0..5 without, 5..1 with the key");
    }

    /// With the key near the start of a long corridor, the root's low-level
    /// search pauses at the key state; the search under it reaches the goal
    /// before positions 3 to 10 without the key are ever expanded.
    #[test]
    fn low_level_searches_pause_at_new_high_level_states() {
        let corridor = Corridor {
            len: 10,
            key_at: 2,
            key_feature: false,
        };
        let r = hiw(&corridor, key, 1, 1, 1);

        assert_eq!(r.plan.as_deref(), Some(&[1, 1, -1, -1][..]));
        assert_eq!(
            r.expanded, 4,
            "positions 0 and 1, then 2 and 1 with the key"
        );
    }

    /// A high-level feature that never changes leaves one high-level node,
    /// so HIW(1, 1) is IW(1) and fails on the corridor too.
    #[test]
    fn a_constant_high_level_feature_is_iw() {
        let corridor = Corridor::new(3);
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
        hiw(&Corridor::new(0), key, 1, 0, 1);
    }

    /// Width 0 at the low level is rejected even when the root is a goal.
    #[test]
    #[should_panic(expected = "width bound must be at least 1")]
    fn rejects_low_width_zero() {
        hiw(&Corridor::new(0), key, 1, 1, 0);
    }
}
