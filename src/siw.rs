use crate::iw::iw_plus_from;
use crate::{GoalCount, RelaxedPlan, SearchResult, WidthProblem};

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
    serialize(
        problem,
        max_k,
        None::<fn(&P::State, &mut Vec<u32>)>,
        GoalCountDrop(problem),
    )
}

/// SIW+ (Lipovetzky & Geffner, ECAI 2014): [`siw`] with IW+ in place of IW.
///
/// IW+ computes novelty separately for each `#r`: the number of relaxed-plan
/// features reached on the path from the root of the current IW run, as in
/// [`bfws_r`](crate::bfws_r). Features already true at that root never
/// count, even if the path makes them false and true again. The relaxed plan
/// comes from
/// [`RelaxedPlan::relaxed_plan_features`] and is computed once at the root of
/// every IW run. A state is pruned only if it is not novel among the states
/// with the same `#r`, so IW+ keeps more states than IW and can reach a
/// subgoal at a lower width, at the price of more nodes per run.
///
/// Subgoals are found as in [`siw`]: any drop in `#g` counts, and the search
/// gives up when no subgoal is reachable within width `max_k`. LAPKT's SIW+
/// differs in four ways, so node counts and plans differ too: it tracks goal
/// atoms (see [`siw`]), it skips states already generated in the current run,
/// it stops widening early when a run pruned nothing by novelty, and its `#r`
/// counts add effects on the path, so a root fact that an action adds again
/// counts too.
pub fn siw_plus<P: RelaxedPlan>(problem: &P, max_k: usize) -> SearchResult<P::Action> {
    serialize(
        problem,
        max_k,
        Some(|s: &P::State, out: &mut Vec<u32>| problem.relaxed_plan_features(s, out)),
        GoalCountDrop(problem),
    )
}

/// How a serialized search picks its subgoals.
pub(crate) trait Subgoals<S> {
    /// What the test needs to know about a subproblem's start state,
    /// computed once per subproblem.
    type Anchor;

    /// Summarizes `from`, the state a subproblem starts from.
    fn anchor(&mut self, from: &S) -> Self::Anchor;

    /// Whether `state` solves the subproblem that started at `anchor`.
    fn reached(&self, anchor: &Self::Anchor, state: &S) -> bool;

    /// Records that the search moved on to the subgoal `state`; returns
    /// `false` to make the search give up instead.
    fn advance(&mut self, _state: &S) -> bool {
        true
    }
}

/// The subgoals of [`siw`] and [`siw_plus`]: a goal state, or any state with
/// fewer unsatisfied goals than the start of the subproblem.
struct GoalCountDrop<'p, P>(&'p P);

impl<P: GoalCount> Subgoals<P::State> for GoalCountDrop<'_, P> {
    /// `#g` of the start state.
    type Anchor = usize;

    fn anchor(&mut self, from: &P::State) -> usize {
        self.0.unsatisfied_goals(from)
    }

    fn reached(&self, g: &usize, state: &P::State) -> bool {
        self.0.is_goal(state) || self.0.unsatisfied_goals(state) < *g
    }
}

/// The loop behind [`siw`], [`siw_plus`] and [`siw_r`](crate::siw_r):
/// reaches one subgoal after another and concatenates the subplans.
///
/// Each IW run is IW+ when `oracle` is set, plain IW otherwise. `subgoals`
/// decides which states end a subproblem; only [`siw_r`](crate::siw_r)'s
/// subgoals also stop the search when it moves on to a subgoal it passed
/// before (a cycling sketch).
pub(crate) fn serialize<P, F, G>(
    problem: &P,
    max_k: usize,
    mut oracle: Option<F>,
    mut subgoals: G,
) -> SearchResult<P::Action>
where
    P: WidthProblem,
    F: FnMut(&P::State, &mut Vec<u32>),
    G: Subgoals<P::State>,
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
        let anchor = subgoals.anchor(&state);
        let is_subgoal = |s: &P::State| subgoals.reached(&anchor, s);
        let Some((subplan, next)) = reach_subgoal(
            problem,
            &state,
            max_k,
            oracle.as_mut(),
            is_subgoal,
            &mut total,
        ) else {
            return total;
        };
        if !subgoals.advance(&next) {
            return total;
        }

        plan.extend(subplan);
        state = next;
    }

    total.plan = Some(plan);
    total
}

/// Iterated width from `state` until `is_subgoal` holds; returns the subplan
/// and the state it reaches. Adds every run's counts to `total`.
/// Each run is IW+ with `oracle`'s relaxed plan when it is set.
fn reach_subgoal<P, F>(
    problem: &P,
    state: &P::State,
    max_k: usize,
    mut oracle: Option<&mut F>,
    is_subgoal: impl Fn(&P::State) -> bool + Copy,
    total: &mut SearchResult<P::Action>,
) -> Option<(Vec<P::Action>, P::State)>
where
    P: WidthProblem,
    F: FnMut(&P::State, &mut Vec<u32>),
{
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
