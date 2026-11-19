#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod bfws;
mod iw;
mod novelty;
#[cfg(feature = "sas-plus")]
pub mod sas;

pub use bfws::bfws;
pub use iw::{iterated_width, iw};
pub use novelty::NoveltyTable;

pub trait WidthProblem {
    type State: Clone;
    type Action: Clone;

    fn initial_state(&self) -> Self::State;
    /// Push all `(action, successor)` pairs of `state` into `out`.
    fn successors(&self, state: &Self::State, out: &mut Vec<(Self::Action, Self::State)>);
    /// Push the feature ids (`< num_features()`) true in `state` into `out`.
    fn features(&self, state: &Self::State, out: &mut Vec<u32>);
    fn num_features(&self) -> usize;
    fn is_goal(&self, state: &Self::State) -> bool;
}

/// Goal-directed information used by BFWS.
pub trait GoalCount: WidthProblem {
    /// Number of goal conditions not yet satisfied (`#g`).
    fn unsatisfied_goals(&self, state: &Self::State) -> usize;
}

#[derive(Clone, Debug)]
pub struct SearchResult<A> {
    pub plan: Option<Vec<A>>,
    pub expanded: usize,
    pub generated: usize,
}

pub(crate) struct Node<S, A> {
    pub state: S,
    pub parent: Option<usize>,
    pub action: Option<A>,
}

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
