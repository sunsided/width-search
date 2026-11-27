use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::hash::Hash;

use crate::relevance::{Relevance, RelevanceTracker};
use crate::{GoalCount, Node, NoveltyTable, RelaxedPlan, SearchResult, extract_plan};

/// BFWS(f) with `f = ⟨w_{#g}, #g⟩`: best-first search ordered by novelty
/// (computed separately for each goal-count value `#g`, capped at `k + 1`),
/// breaking ties by `#g`. Complete on finite state spaces.
///
/// # Panics
///
/// Panics if `k` is 0.
pub fn bfws<P>(problem: &P, k: usize) -> SearchResult<P::Action>
where
    P: GoalCount,
    P::State: Hash + Eq,
{
    search(problem, k, None::<fn(&P::State, &mut Vec<u32>)>)
}

/// BFWS(f5) with `f = ⟨w_{#g,#r}, #g⟩` (Lipovetzky & Geffner, AAAI 2017):
/// like [`bfws`], but novelty is computed separately for each pair of `#g`
/// and `#r`.
///
/// `#r` counts the relaxed-plan features reached on the way to a state. The
/// relaxed plan comes from [`RelaxedPlan::relaxed_plan_features`], computed at
/// the initial state and again at every state where `#g` drops, as in
/// LAPKT's `BFWS-f5`. Features already true in the state the relaxed plan was
/// computed from are not counted. `#r` is path-dependent: it counts every
/// relevant feature true in some state since that recomputation.
///
/// Complete on finite state spaces.
///
/// # Panics
///
/// Panics if `k` is 0.
pub fn bfws_r<P>(problem: &P, k: usize) -> SearchResult<P::Action>
where
    P: RelaxedPlan,
    P::State: Hash + Eq,
{
    search(
        problem,
        k,
        Some(|s: &P::State, out: &mut Vec<u32>| problem.relaxed_plan_features(s, out)),
    )
}

/// The `#g` and `#r` of a node: what its novelty is partitioned by.
type Partition = (usize, Option<Relevance>);

/// Computes `⟨w, #g⟩` of states, owning the novelty tables of every partition.
struct Evaluator<'p, P, F> {
    /// The problem being searched.
    problem: &'p P,
    /// Width bound; novelty above it is reported as `k + 1`.
    k: usize,
    /// One novelty table per `(#g, #r)` pair, created on first use.
    tables: HashMap<(usize, usize), NoveltyTable>,
    /// Relaxed-plan sets and the oracle that computes them.
    tracker: RelevanceTracker<F>,
    /// Reused buffer for the features of a state.
    feats: Vec<u32>,
}

impl<P, F> Evaluator<'_, P, F>
where
    P: GoalCount,
    F: FnMut(&P::State, &mut Vec<u32>),
{
    /// The novelty `w` of `state` and its partition. `parent` is the
    /// parent's partition, `None` for the root. The relaxed plan is
    /// recomputed at the root and wherever `#g` drops.
    fn evaluate(&mut self, state: &P::State, parent: Option<&Partition>) -> (usize, Partition) {
        self.feats.clear();
        self.problem.features(state, &mut self.feats);
        let g = self.problem.unsatisfied_goals(state);
        let rel = match parent {
            Some((pg, Some(rel))) if g >= *pg => Some(self.tracker.extend(rel, &self.feats)),
            _ => self.tracker.recompute(state, &self.feats),
        };

        let r = rel.as_ref().map_or(0, |r| r.count);
        let (num_features, k) = (self.problem.num_features(), self.k);
        let table = self
            .tables
            .entry((g, r))
            .or_insert_with(|| NoveltyTable::new(num_features, k));
        let w = table.evaluate_and_insert(&self.feats).unwrap_or(k + 1);

        (w, (g, rel))
    }
}

/// The best-first search behind [`bfws`] and [`bfws_r`]: novelty tables are
/// partitioned by `(#g, #r)`, and the open list is ordered by `⟨w, #g⟩` with
/// ties broken by generation order. With `oracle == None`, `#r` is 0.
fn search<P, F>(problem: &P, k: usize, oracle: Option<F>) -> SearchResult<P::Action>
where
    P: GoalCount,
    P::State: Hash + Eq,
    F: FnMut(&P::State, &mut Vec<u32>),
{
    // Checked up front, not only when the first novelty table is built, so a
    // goal initial state does not hide the bad bound.
    assert!(k >= 1, "width bound must be at least 1");
    let mut eval = Evaluator {
        problem,
        k,
        tables: HashMap::new(),
        tracker: RelevanceTracker::new(oracle),
        feats: Vec::new(),
    };
    let mut seen: HashSet<P::State> = HashSet::new();
    let mut succ = Vec::new();
    let mut result = SearchResult {
        plan: None,
        expanded: 0,
        generated: 1,
    };

    let root = problem.initial_state();
    if problem.is_goal(&root) {
        result.plan = Some(Vec::new());
        return result;
    }

    let (w, part) = eval.evaluate(&root, None);
    let mut open = BinaryHeap::from([Reverse((w, part.0, 0usize))]);
    seen.insert(root.clone());
    let mut nodes = vec![Node {
        state: root,
        parent: None,
        action: None,
    }];
    let mut parts = vec![part];

    while let Some(Reverse((_, _, i))) = open.pop() {
        result.expanded += 1;
        succ.clear();
        problem.successors(&nodes[i].state, &mut succ);
        for (action, state) in succ.drain(..) {
            result.generated += 1;
            if !seen.insert(state.clone()) {
                continue;
            }

            let goal = problem.is_goal(&state);
            let (w, part) = eval.evaluate(&state, Some(&parts[i]));
            open.push(Reverse((w, part.0, nodes.len())));
            nodes.push(Node {
                state,
                parent: Some(i),
                action: Some(action),
            });
            parts.push(part);
            if goal {
                result.plan = Some(extract_plan(&nodes, nodes.len() - 1));
                return result;
            }
        }
    }

    result
}
