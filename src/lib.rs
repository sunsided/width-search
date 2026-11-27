#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

/// Best-first width search: BFWS(⟨w_{#g}, #g⟩) and BFWS(⟨w_{#g,#r}, #g⟩).
mod bfws;
/// IW(k) and iterated width.
mod iw;
/// The novelty table shared by all searches.
mod novelty;
/// `#r`: relaxed-plan features reached on the path to a node.
mod relevance;
/// Rollout IW(1): depth-first rollouts with depth-aware novelty.
mod rollout;
/// Adapter that runs width-based search on a [`sas_plus::Task`].
#[cfg(feature = "sas-plus")]
pub mod sas;
/// Serialized IW and SIW+: one IW (or IW+) search per achieved goal.
mod siw;

pub use bfws::{bfws, bfws_r};
pub use iw::{iterated_width, iw};
pub use novelty::NoveltyTable;
pub use rollout::rollout_iw;
pub use siw::{siw, siw_plus};

/// A search problem described by successors and integer features.
///
/// Width-based search never inspects states directly. It only needs the
/// successors of a state, the goal test, and the features that are true in a
/// state, numbered `0..num_features()`. Novelty is computed over those
/// features, so their choice decides what counts as "new".
pub trait WidthProblem {
    /// A search state. Cloned when a node is stored.
    type State: Clone;
    /// The label of a transition; plans are sequences of actions.
    type Action: Clone;

    /// The state the search starts from.
    fn initial_state(&self) -> Self::State;
    /// Push all `(action, successor)` pairs of `state` into `out`.
    fn successors(&self, state: &Self::State, out: &mut Vec<(Self::Action, Self::State)>);
    /// Push the feature ids (`< num_features()`) true in `state` into `out`.
    fn features(&self, state: &Self::State, out: &mut Vec<u32>);
    /// Number of distinct features; every id from [`WidthProblem::features`]
    /// must be below it.
    fn num_features(&self) -> usize;
    /// Whether `state` satisfies the goal.
    fn is_goal(&self, state: &Self::State) -> bool;
}

/// Goal-directed information used by BFWS.
pub trait GoalCount: WidthProblem {
    /// Number of goal conditions not yet satisfied (`#g`).
    fn unsatisfied_goals(&self, state: &Self::State) -> usize;
}

/// Relaxed-plan information used by [`bfws_r`] to compute `#r`.
pub trait RelaxedPlan: GoalCount {
    /// Push into `out` the features a relaxed plan from `state` makes true,
    /// for example the add effects of its actions (LAPKT's choice).
    ///
    /// Push nothing when the goal is unreachable even under the relaxation.
    /// Features already true in `state` are ignored by the search.
    fn relaxed_plan_features(&self, state: &Self::State, out: &mut Vec<u32>);
}

/// The outcome of a search: a plan, if one was found, and node counts.
#[derive(Clone, Debug)]
pub struct SearchResult<A> {
    /// The actions from the initial state to a goal, or `None` if the search
    /// gave up (pruning) or exhausted the state space.
    pub plan: Option<Vec<A>>,
    /// Number of nodes whose successors were generated.
    pub expanded: usize,
    /// Number of states generated, the initial state included.
    pub generated: usize,
}

/// A search node, stored in an arena and linked to its parent by index.
pub(crate) struct Node<S, A> {
    /// The state this node represents.
    pub state: S,
    /// Arena index of the parent node; `None` only for the root.
    pub parent: Option<usize>,
    /// The action that led from the parent here; `None` only for the root.
    pub action: Option<A>,
}

/// Follows parent links from node `i` back to the root and returns the
/// actions along the way, in execution order.
pub(crate) fn extract_plan<S, A: Clone>(nodes: &[Node<S, A>], mut i: usize) -> Vec<A> {
    let mut plan = Vec::new();
    while let Some(p) = nodes[i].parent {
        plan.push(
            nodes[i]
                .action
                .clone()
                .expect("non-root node has an action"),
        );
        i = p;
    }

    plan.reverse();
    plan
}
