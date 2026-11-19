use crate::iw::iw_from;
use crate::{GoalCount, SearchResult};

/// Serialized IW (SIW): reaches the goal one subgoal at a time.
///
/// From the current state it runs IW(1), IW(2), ... up to `max_k` until a
/// state with fewer unsatisfied goals (`#g`) is found, or a goal state. The
/// plans of these searches are concatenated; each search starts with an
/// empty novelty table.
///
/// This is a simplified SIW. LAPKT's SIW (after Lipovetzky & Geffner,
/// ECAI 2012) tracks goal atoms: a subgoal state must keep every goal achieved
/// so far, and a newly achieved goal is accepted only if the full goal stays
/// reachable in the delete relaxation without actions that delete achieved
/// goals. [`GoalCount`] exposes only the number of unsatisfied goals, so here
/// any drop in `#g` counts. A subgoal may then undo earlier goals or commit to
/// one that must be undone later, which gives longer plans (blocksworld).
///
/// SIW is incomplete: it gives up (`plan: None`) when no subgoal is reachable
/// within width `max_k`. That happens in dead ends left by earlier subplans,
/// but also in live states whose next subgoal needs a width above `max_k`.
/// Node counts add up over all IW runs, failed ones included.
pub fn siw<P: GoalCount>(problem: &P, max_k: usize) -> SearchResult<P::Action> {
    // The initial state counts once, even if no IW run is needed.
    let mut total = SearchResult {
        plan: None,
        expanded: 0,
        generated: 0,
    };
    let mut state = problem.initial_state();
    if problem.is_goal(&state) {
        total.generated = 1;
    }

    let mut plan = Vec::new();

    while !problem.is_goal(&state) {
        let Some((subplan, next)) = reach_subgoal(problem, &state, max_k, &mut total) else {
            return total;
        };
        plan.extend(subplan);
        state = next;
    }

    total.plan = Some(plan);
    total
}

/// Iterated width from `state` until `#g` drops or the goal holds; returns
/// the subplan and the state it reaches. Adds every run's counts to `total`.
fn reach_subgoal<P: GoalCount>(
    problem: &P,
    state: &P::State,
    max_k: usize,
    total: &mut SearchResult<P::Action>,
) -> Option<(Vec<P::Action>, P::State)> {
    let g = problem.unsatisfied_goals(state);
    let is_subgoal = |s: &P::State| problem.is_goal(s) || problem.unsatisfied_goals(s) < g;

    for k in 1..=max_k {
        let (r, end) = iw_from(problem, state.clone(), k, is_subgoal);
        total.expanded += r.expanded;
        total.generated += r.generated;
        if let (Some(subplan), Some(end)) = (r.plan, end) {
            return Some((subplan, end));
        }
    }

    None
}
