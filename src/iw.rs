use std::collections::VecDeque;

use crate::{Node, NoveltyTable, SearchResult, WidthProblem, extract_plan};

/// IW(k): breadth-first search that prunes every generated state whose
/// novelty exceeds `k`. Runs in `O(n^k)` expansions for `n` features.
pub fn iw<P: WidthProblem>(problem: &P, k: usize) -> SearchResult<P::Action> {
    let mut table = NoveltyTable::new(problem.num_features(), k);
    let mut feats = Vec::new();
    let mut succ = Vec::new();
    let mut result = SearchResult {
        plan: None,
        expanded: 0,
        generated: 1,
    };

    let root = problem.initial_state();
    problem.features(&root, &mut feats);
    table.evaluate_and_insert(&feats);
    if problem.is_goal(&root) {
        result.plan = Some(Vec::new());
        return result;
    }
    let mut nodes = vec![Node {
        state: root,
        parent: None,
        action: None,
    }];
    let mut open = VecDeque::from([0usize]);

    while let Some(i) = open.pop_front() {
        result.expanded += 1;
        succ.clear();
        problem.successors(&nodes[i].state, &mut succ);
        for (action, state) in succ.drain(..) {
            result.generated += 1;
            let goal = problem.is_goal(&state);
            feats.clear();
            problem.features(&state, &mut feats);
            if table.evaluate_and_insert(&feats).is_none() && !goal {
                continue;
            }
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
            open.push_back(j);
        }
    }
    result
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
