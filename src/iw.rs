use std::collections::VecDeque;

use crate::relevance::{Relevance, RelevanceTracker};
use crate::{Node, NoveltyTable, SearchResult, WidthProblem, extract_plan};

/// IW(k): breadth-first search that prunes every generated state whose
/// novelty exceeds `k`. Runs in `O(n^k)` expansions for `n` features.
///
/// # Panics
///
/// Panics if `k` is 0.
pub fn iw<P: WidthProblem>(problem: &P, k: usize) -> SearchResult<P::Action> {
    iw_from(problem, problem.initial_state(), k, |s| problem.is_goal(s)).0
}

/// IW(k) from `root` towards the states accepted by `is_goal`.
///
/// Returns the result and, if a plan was found, the state it ends in. Goal
/// states are never pruned, whatever their novelty.
pub(crate) fn iw_from<P: WidthProblem>(
    problem: &P,
    root: P::State,
    k: usize,
    is_goal: impl Fn(&P::State) -> bool,
) -> (SearchResult<P::Action>, Option<P::State>) {
    iw_plus_from(
        problem,
        root,
        k,
        is_goal,
        None::<fn(&P::State, &mut Vec<u32>)>,
    )
}

/// IW(k) from `root`, with novelty computed separately for each `#r`
/// (IW+ when `oracle` is set, plain IW otherwise).
///
/// `oracle` gives the relaxed-plan features of `root`, computed once per run;
/// `#r` of a node counts those reached on its path, as in
/// [`bfws_r`](crate::bfws_r). Returns the result and, if a plan was found,
/// the state it ends in. Goal states are never pruned.
pub(crate) fn iw_plus_from<P, F>(
    problem: &P,
    root: P::State,
    k: usize,
    is_goal: impl Fn(&P::State) -> bool,
    oracle: Option<F>,
) -> (SearchResult<P::Action>, Option<P::State>)
where
    P: WidthProblem,
    F: FnMut(&P::State, &mut Vec<u32>),
{
    let mut feats = Vec::new();
    let mut succ = Vec::new();
    let mut result = SearchResult {
        plan: None,
        expanded: 0,
        generated: 1,
    };

    problem.features(&root, &mut feats);
    let mut novelty = PartitionedNovelty::new(problem.num_features(), k, oracle);
    let root_rel = novelty.start(&root, &feats);
    if is_goal(&root) {
        result.plan = Some(Vec::new());
        return (result, Some(root));
    }

    let mut nodes = vec![Node {
        state: root,
        parent: None,
        action: None,
    }];
    let mut rels = vec![root_rel];
    let mut open = VecDeque::from([0usize]);

    while let Some(i) = open.pop_front() {
        result.expanded += 1;
        succ.clear();
        problem.successors(&nodes[i].state, &mut succ);
        for (action, state) in succ.drain(..) {
            result.generated += 1;
            let goal = is_goal(&state);
            feats.clear();
            problem.features(&state, &mut feats);
            let (novel, rel) = novelty.evaluate(rels[i].as_ref(), &feats);
            if !novel && !goal {
                continue;
            }

            nodes.push(Node {
                state,
                parent: Some(i),
                action: Some(action),
            });
            rels.push(rel);
            let j = nodes.len() - 1;
            if goal {
                result.plan = Some(extract_plan(&nodes, j));
                return (result, Some(nodes.swap_remove(j).state));
            }

            open.push_back(j);
        }
    }

    (result, None)
}

/// The novelty tables of one IW run, one per `#r` value, created on first use.
struct PartitionedNovelty<F> {
    /// Computes `#r`; disabled for plain IW, which then has one partition.
    tracker: RelevanceTracker<F>,
    /// Novelty table of each `#r`, indexed by it; `None` until first used.
    tables: Vec<Option<NoveltyTable>>,
    /// Number of features of the problem.
    num_features: usize,
    /// Width bound of every table.
    k: usize,
}

impl<F> PartitionedNovelty<F> {
    /// Empty tables for features `0..num_features` and width `k`.
    fn new(num_features: usize, k: usize, oracle: Option<F>) -> Self {
        Self {
            tracker: RelevanceTracker::new(oracle),
            tables: Vec::new(),
            num_features,
            k,
        }
    }

    /// Computes the relevant set at the root, sizes the tables for it, and
    /// records the root's tuples. Returns the root's `#r` bookkeeping.
    fn start<S>(&mut self, root: &S, feats: &[u32]) -> Option<Relevance>
    where
        F: FnMut(&S, &mut Vec<u32>),
    {
        let rel = self.tracker.recompute(root, feats);
        let partitions = rel.as_ref().map_or(1, |r| self.tracker.set_len(r) + 1);
        self.tables = std::iter::repeat_with(|| None).take(partitions).collect();
        self.evaluate(None, feats);

        rel
    }

    /// Whether a state with `feats` is novel (width at most `k`) in its
    /// partition, and its `#r` bookkeeping. `parent` is the parent's
    /// bookkeeping; `None` for the root or when `#r` is disabled.
    fn evaluate(&mut self, parent: Option<&Relevance>, feats: &[u32]) -> (bool, Option<Relevance>) {
        let rel = parent.map(|p| self.tracker.extend(p, feats));
        let r = rel.as_ref().map_or(0, |r| r.count);
        let (num_features, k) = (self.num_features, self.k);
        let table = self.tables[r].get_or_insert_with(|| NoveltyTable::new(num_features, k));

        (table.evaluate_and_insert(feats).is_some(), rel)
    }
}

/// Runs IW(1), IW(2), ... up to `max_k`; returns the width that succeeded
/// (if any) and the accumulated statistics.
pub fn iterated_width<P: WidthProblem>(
    problem: &P,
    max_k: usize,
) -> (Option<usize>, SearchResult<P::Action>) {
    let mut total = SearchResult {
        plan: None,
        expanded: 0,
        generated: 0,
    };
    for k in 1..=max_k {
        let r = iw(problem, k);
        total.expanded += r.expanded;
        total.generated += r.generated;
        if r.plan.is_some() {
            total.plan = r.plan;
            return (Some(k), total);
        }
    }
    (None, total)
}
