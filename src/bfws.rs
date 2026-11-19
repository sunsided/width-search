use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::hash::Hash;

use crate::{GoalCount, Node, NoveltyTable, SearchResult, extract_plan};

/// BFWS(f) with `f = ⟨w_{#g}, #g⟩`: best-first search ordered by novelty
/// (computed separately for each goal-count value `#g`, capped at `k + 1`),
/// breaking ties by `#g`. Complete on finite state spaces.
pub fn bfws<P>(problem: &P, k: usize) -> SearchResult<P::Action>
where
    P: GoalCount,
    P::State: Hash + Eq,
{
    let mut tables: HashMap<usize, NoveltyTable> = HashMap::new();
    let mut seen: HashSet<P::State> = HashSet::new();
    let mut feats = Vec::new();
    let mut succ = Vec::new();
    let mut result = SearchResult {
        plan: None,
        expanded: 0,
        generated: 1,
    };

    let mut novelty = |state: &P::State, feats: &mut Vec<u32>| -> (usize, usize) {
        let g = problem.unsatisfied_goals(state);
        feats.clear();
        problem.features(state, feats);
        let table = tables
            .entry(g)
            .or_insert_with(|| NoveltyTable::new(problem.num_features(), k));
        (table.evaluate_and_insert(feats).unwrap_or(k + 1), g)
    };

    let root = problem.initial_state();
    if problem.is_goal(&root) {
        result.plan = Some(Vec::new());
        return result;
    }
    let (w, g) = novelty(&root, &mut feats);
    seen.insert(root.clone());
    let mut nodes = vec![Node {
        state: root,
        parent: None,
        action: None,
    }];
    let mut open = BinaryHeap::from([Reverse((w, g, 0usize))]);

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
            let (w, g) = novelty(&state, &mut feats);
            nodes.push(Node {
                state,
                parent: Some(i),
                action: Some(action),
            });
            let j = nodes.len() - 1;
            if goal {
                result.plan = Some(extract_plan(&nodes, j));
                return result;
            }
            open.push(Reverse((w, g, j)));
        }
    }
    result
}
