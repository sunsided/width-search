use crate::iw::iw_plus_from;
use crate::{GoalCount, RelaxedPlan, SearchResult};

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
    serialize(problem, max_k, None::<fn(&P::State, &mut Vec<u32>)>)
}

/// SIW+ (Lipovetzky & Geffner, ECAI 2014): [`siw`] with IW+ in place of IW.
///
/// IW+ computes novelty separately for each `#r`: the number of relaxed-plan
/// features reached on the path from the root of the current IW run, as in
/// [`bfws_r`](crate::bfws_r). The relaxed plan comes from
/// [`RelaxedPlan::relaxed_plan_features`] and is computed once at the root of
/// every IW run. A state is pruned only if it is not novel among the states
/// with the same `#r`, so IW+ keeps more states than IW and can reach a
/// subgoal at a lower width, at the price of more nodes per run.
///
/// Subgoals are found as in [`siw`]: any drop in `#g` counts, and the search
/// gives up when no subgoal is reachable within width `max_k`. LAPKT's SIW+
/// differs in three ways, so node counts and plans differ too: it tracks goal
/// atoms (see [`siw`]), it skips states already generated in the current run,
/// and it stops widening early when a run pruned nothing by novelty.
pub fn siw_plus<P: RelaxedPlan>(problem: &P, max_k: usize) -> SearchResult<P::Action> {
    serialize(
        problem,
        max_k,
        Some(|s: &P::State, out: &mut Vec<u32>| problem.relaxed_plan_features(s, out)),
    )
}

/// The loop behind [`siw`] and [`siw_plus`]: reaches one subgoal after
/// another and concatenates the subplans. Each IW run is IW+ when `oracle`
/// is set, plain IW otherwise.
fn serialize<P, F>(problem: &P, max_k: usize, mut oracle: Option<F>) -> SearchResult<P::Action>
where
    P: GoalCount,
    F: FnMut(&P::State, &mut Vec<u32>),
{
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
        let Some((subplan, next)) =
            reach_subgoal(problem, &state, max_k, oracle.as_mut(), &mut total)
        else {
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
/// Each run is IW+ with `oracle`'s relaxed plan when it is set.
fn reach_subgoal<P, F>(
    problem: &P,
    state: &P::State,
    max_k: usize,
    mut oracle: Option<&mut F>,
    total: &mut SearchResult<P::Action>,
) -> Option<(Vec<P::Action>, P::State)>
where
    P: GoalCount,
    F: FnMut(&P::State, &mut Vec<u32>),
{
    let g = problem.unsatisfied_goals(state);
    let is_subgoal = |s: &P::State| problem.is_goal(s) || problem.unsatisfied_goals(s) < g;

    for k in 1..=max_k {
        let run_oracle = oracle.as_deref_mut();
        let (r, end) = iw_plus_from(problem, state.clone(), k, is_subgoal, run_oracle);
        total.expanded += r.expanded;
        total.generated += r.generated;
        if let (Some(subplan), Some(end)) = (r.plan, end) {
            return Some((subplan, end));
        }
    }

    None
}
