use crate::rng::SplitMix64;
use crate::{Node, SearchResult, WidthProblem, extract_plan};

/// Rollout IW(1) (Bandres, Bonet & Geffner, AAAI 2018), goal-directed.
///
/// Builds a search tree by repeated rollouts from the root. Each rollout
/// walks down through a random child not yet labeled solved, generating
/// children lazily, and stops by labeling a node solved. Novelty uses the
/// smallest depth `d[f]` at which each feature `f` was reached:
///
/// - a new node at depth `d` continues the rollout if `d < d[f]` for some
///   feature `f` it makes true, and lowers `d[f]` to `d` for all of them;
///   otherwise it is labeled solved (pruned);
/// - a node already in the tree continues if `d == d[f]` for some feature,
///   and is labeled solved otherwise (a shallower path took its features).
///
/// A node with no successors is solved; a node is solved once all of its
/// children are. The search returns as soon as a goal state is generated,
/// and gives up (`plan: None`) when the root is solved.
///
/// The paper's algorithm picks actions for a reward-maximizing agent within a
/// time budget; this version has no budget and stops at the first goal. Like
/// IW(1), it reaches every goal of width 1, but its plans need not be
/// shortest. `seed` fixes the random child choices.
pub fn rollout_iw<P: WidthProblem>(problem: &P, seed: u64) -> SearchResult<P::Action> {
    search(problem, None::<Weights<P>>, seed)
}

/// π-IW (Junyent, Jonsson & Gómez, ICAPS 2019): [`rollout_iw`] with rollouts
/// guided by a policy.
///
/// When a node is expanded, `policy` gets its state and all of its
/// `(action, successor)` pairs, and pushes one weight per successor into the
/// (empty) output vector, for example the action probabilities of a learned
/// policy. A rollout then moves to an unsolved child with probability
/// proportional to its weight. Children of weight 0 are picked, uniformly at
/// random, only once every unsolved sibling has weight 0, so π-IW explores the
/// same tree as [`rollout_iw`] in a different order and still reaches every
/// goal of width 1. Constant weights give uniform choices.
///
/// The paper also learns the policy from the planner's choices; that loop is
/// left to the caller. `seed` fixes the random choices.
///
/// # Panics
///
/// Panics if `policy` pushes a different number of weights than there are
/// successors, a weight that is negative or not finite, or weights whose sum
/// is not finite.
pub fn pi_iw<P, F>(problem: &P, policy: F, seed: u64) -> SearchResult<P::Action>
where
    P: WidthProblem,
    F: FnMut(&P::State, &[(P::Action, P::State)], &mut Vec<f64>),
{
    search(problem, Some(policy), seed)
}

/// The signature of a π-IW policy, as taken by [`pi_iw`].
type Weights<P> = fn(
    &<P as WidthProblem>::State,
    &[(<P as WidthProblem>::Action, <P as WidthProblem>::State)],
    &mut Vec<f64>,
);

/// The search behind [`rollout_iw`] (`policy == None`, uniform choices) and
/// [`pi_iw`].
fn search<P, F>(problem: &P, policy: Option<F>, seed: u64) -> SearchResult<P::Action>
where
    P: WidthProblem,
    F: FnMut(&P::State, &[(P::Action, P::State)], &mut Vec<f64>),
{
    let mut tree = Tree::new(problem, policy, seed, None);
    let root = problem.initial_state();
    if problem.is_goal(&root) {
        tree.result.plan = Some(Vec::new());
        return tree.result;
    }

    tree.add_root(root);
    while !tree.is_exhausted() {
        if let Outcome::Goal(goal) = tree.rollout() {
            tree.result.plan = Some(extract_plan(&tree.nodes, goal));
            break;
        }
    }

    tree.result
}

/// Per-node rollout bookkeeping, parallel to [`Tree::nodes`].
struct Meta {
    /// Number of actions from the root.
    depth: usize,
    /// Arena indices of the children; `None` until the node is expanded.
    children: Option<Vec<usize>>,
    /// Whether the subtree below needs no further rollouts.
    solved: bool,
    /// Whether a rollout has already run the novelty test on this node.
    visited: bool,
    /// The policy's weight for reaching this node from its parent; finite
    /// and non-negative. Unused without a policy and at the root.
    weight: f64,
}

/// How a rollout ended.
pub(crate) enum Outcome {
    /// It generated a goal state, at this arena index.
    Goal(usize),
    /// It entered a state outside the root's high-level partition, at this
    /// arena index; see [`Tree::new`].
    Boundary(usize),
    /// It labeled a node solved; nothing to report.
    Solved,
}

/// The high-level feature function of π-HIW and the sorted high-level
/// features of a tree's root: states with other features are boundaries.
pub(crate) type Partition<'a, S> = (&'a dyn Fn(&S, &mut Vec<u32>), Vec<u32>);

/// What the novelty test says about the node a rollout just entered.
enum Verdict {
    /// The node is novel: continue the rollout below it.
    Continue,
    /// The node is not novel: label it solved and end the rollout.
    Prune,
}

/// The search tree of a Rollout IW(1) run and its novelty table.
pub(crate) struct Tree<'p, P: WidthProblem, F> {
    /// The problem being searched.
    problem: &'p P,
    /// For π-HIW: the high-level partition of the root. A rollout entering
    /// a state outside it stops there; `None` for a flat search.
    partition: Option<Partition<'p, P::State>>,
    /// The π-IW policy; `None` picks children uniformly.
    policy: Option<F>,
    /// Reused buffer for the policy's weights.
    weights: Vec<f64>,
    /// Node arena: states and parent links.
    pub nodes: Vec<Node<P::State, P::Action>>,
    /// Rollout data of each node, same indices as `nodes`.
    meta: Vec<Meta>,
    /// Smallest depth at which each feature was reached; `usize::MAX` if never.
    depth_of: Vec<usize>,
    /// Random source for child selection.
    rng: SplitMix64,
    /// Reused buffer for the features of a state.
    feats: Vec<u32>,
    /// Reused buffer for successors.
    succ: Vec<(P::Action, P::State)>,
    /// Node counts so far; `plan` is filled in by [`rollout_iw`].
    pub result: SearchResult<P::Action>,
}

impl<'p, P, F> Tree<'p, P, F>
where
    P: WidthProblem,
    F: FnMut(&P::State, &[(P::Action, P::State)], &mut Vec<f64>),
{
    /// An empty tree for `problem`, guided by `policy` if set, and confined
    /// to a high-level `partition` if set.
    pub fn new(
        problem: &'p P,
        policy: Option<F>,
        seed: u64,
        partition: Option<Partition<'p, P::State>>,
    ) -> Self {
        Self {
            problem,
            partition,
            policy,
            weights: Vec::new(),
            nodes: Vec::new(),
            meta: Vec::new(),
            depth_of: vec![usize::MAX; problem.num_features()],
            rng: SplitMix64::new(seed),
            feats: Vec::new(),
            succ: Vec::new(),
            result: SearchResult {
                plan: None,
                expanded: 0,
                generated: 1,
            },
        }
    }

    /// Adds the root at depth 0 and records its features at depth 0.
    pub fn add_root(&mut self, root: P::State) {
        self.feats.clear();
        self.problem.features(&root, &mut self.feats);
        for &f in &self.feats {
            self.depth_of[f as usize] = 0;
        }

        self.push(root, None, None, 0, 0.0);
        self.meta[0].visited = true;
    }

    /// Whether every node is solved, so no rollout is left to run.
    pub fn is_exhausted(&self) -> bool {
        self.meta[0].solved
    }

    /// One rollout from the root. A boundary node it enters is labeled
    /// solved, so each is reported once.
    pub fn rollout(&mut self) -> Outcome {
        let mut n = 0;
        loop {
            if self.meta[n].children.is_none() {
                if let Some(goal) = self.expand(n) {
                    return Outcome::Goal(goal);
                }
            }

            let Some(child) = self.pick_unsolved_child(n) else {
                // No unsolved children: nothing left to explore below n.
                self.solve(n);
                return Outcome::Solved;
            };

            n = child;
            if self.is_boundary(n) {
                self.solve(n);
                return Outcome::Boundary(n);
            }

            if let Verdict::Prune = self.novelty(n) {
                self.solve(n);
                return Outcome::Solved;
            }
        }
    }

    /// Whether node `n` lies outside the root's high-level partition.
    fn is_boundary(&mut self, n: usize) -> bool {
        let Some((high, root_features)) = &self.partition else {
            return false;
        };

        self.feats.clear();
        high(&self.nodes[n].state, &mut self.feats);
        self.feats.sort_unstable();
        self.feats.dedup();
        self.feats != *root_features
    }

    /// Generates all children of `n`; returns the first goal child, if any.
    fn expand(&mut self, n: usize) -> Option<usize> {
        self.result.expanded += 1;
        let mut succ = std::mem::take(&mut self.succ);
        succ.clear();
        self.problem.successors(&self.nodes[n].state, &mut succ);
        self.ask_policy(n, &succ);

        let depth = self.meta[n].depth + 1;
        let mut children = Vec::with_capacity(succ.len());
        let mut goal = None;
        for (i, (action, state)) in succ.drain(..).enumerate() {
            self.result.generated += 1;
            let is_goal = self.problem.is_goal(&state);
            let weight = self.weights.get(i).copied().unwrap_or(1.0);
            let child = self.push(state, Some(n), Some(action), depth, weight);
            children.push(child);
            if is_goal {
                goal.get_or_insert(child);
            }
        }

        self.succ = succ;
        self.meta[n].children = Some(children);
        goal
    }

    /// Fills [`Tree::weights`] with the policy's weights for the successors
    /// `succ` of node `n`; leaves it empty without a policy.
    ///
    /// # Panics
    ///
    /// Panics if the policy breaks the contract of [`pi_iw`].
    fn ask_policy(&mut self, n: usize, succ: &[(P::Action, P::State)]) {
        self.weights.clear();
        let Some(policy) = self.policy.as_mut() else {
            return;
        };

        policy(&self.nodes[n].state, succ, &mut self.weights);
        assert_eq!(
            self.weights.len(),
            succ.len(),
            "policy must push one weight per successor"
        );
        assert!(
            self.weights.iter().all(|w| w.is_finite() && *w >= 0.0),
            "policy weights must be finite and non-negative"
        );
        // An infinite sum would make every weighted draw pick the last child.
        assert!(
            self.weights.iter().sum::<f64>().is_finite(),
            "policy weights must have a finite sum"
        );
    }

    /// Runs the depth-aware novelty test on `n`, updating the depth table
    /// for a new node.
    fn novelty(&mut self, n: usize) -> Verdict {
        let depth = self.meta[n].depth;
        self.feats.clear();
        self.problem.features(&self.nodes[n].state, &mut self.feats);

        if self.meta[n].visited {
            let kept = self
                .feats
                .iter()
                .any(|&f| self.depth_of[f as usize] == depth);
            return if kept {
                Verdict::Continue
            } else {
                Verdict::Prune
            };
        }

        self.meta[n].visited = true;
        let mut novel = false;
        for &f in &self.feats {
            let d = &mut self.depth_of[f as usize];
            if depth < *d {
                *d = depth;
                novel = true;
            }
        }

        if novel {
            Verdict::Continue
        } else {
            Verdict::Prune
        }
    }

    /// A child of `n` not labeled solved, if any: drawn by policy weight with
    /// a policy (uniformly when all unsolved children have weight 0), and
    /// uniformly without one.
    fn pick_unsolved_child(&mut self, n: usize) -> Option<usize> {
        if self.policy.is_some() {
            if let Some(child) = self.pick_weighted_child(n) {
                return Some(child);
            }
        }

        self.pick_uniform_child(n)
    }

    /// An unsolved child of `n` drawn with probability proportional to its
    /// weight; `None` if no unsolved child has a positive weight.
    fn pick_weighted_child(&mut self, n: usize) -> Option<usize> {
        let unit = self.rng.unit();
        let meta = &self.meta;
        let candidates = || {
            meta[n]
                .children
                .as_deref()
                .unwrap_or_default()
                .iter()
                .copied()
                .filter(|&c| !meta[c].solved && meta[c].weight > 0.0)
        };
        let total: f64 = candidates().map(|c| meta[c].weight).sum();
        let last = candidates().next_back()?;

        let mut target = unit * total;
        for c in candidates() {
            target -= meta[c].weight;
            if target < 0.0 {
                return Some(c);
            }
        }

        // Rounding left `target` at or just above 0: take the last candidate.
        Some(last)
    }

    /// A uniformly random child of `n` not labeled solved, if any.
    fn pick_uniform_child(&mut self, n: usize) -> Option<usize> {
        let count_unsolved = |t: &Self| {
            let children = t.meta[n].children.as_deref().unwrap_or_default();
            children.iter().filter(|&&c| !t.meta[c].solved).count()
        };
        let unsolved = count_unsolved(self);
        if unsolved == 0 {
            return None;
        }

        let pick = self.rng.below(unsolved);
        self.meta[n]
            .children
            .as_deref()
            .unwrap_or_default()
            .iter()
            .copied()
            .filter(|&c| !self.meta[c].solved)
            .nth(pick)
    }

    /// Labels `n` solved and propagates the label to every ancestor whose
    /// children are now all solved.
    fn solve(&mut self, mut n: usize) {
        self.meta[n].solved = true;
        while let Some(p) = self.nodes[n].parent {
            let children = self.meta[p].children.as_deref().unwrap_or_default();
            if !children.iter().all(|&c| self.meta[c].solved) {
                return;
            }

            self.meta[p].solved = true;
            n = p;
        }
    }

    /// Appends a node and returns its index.
    fn push(
        &mut self,
        state: P::State,
        parent: Option<usize>,
        action: Option<P::Action>,
        depth: usize,
        weight: f64,
    ) -> usize {
        self.nodes.push(Node {
            state,
            parent,
            action,
        });
        self.meta.push(Meta {
            depth,
            children: None,
            solved: false,
            visited: false,
            weight,
        });

        self.nodes.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_problems::Line;

    /// The goal has width 1, so every seed reaches it, and along a line the
    /// only plan that survives the depth test is the shortest one.
    #[test]
    fn reaches_a_width_one_goal_for_every_seed() {
        let line = Line { len: 6 };
        for seed in 0..50 {
            let plan = rollout_iw(&line, seed).plan.expect("width-1 goal");
            assert_eq!(plan, [1; 6], "seed {seed}");
        }
    }

    /// A policy that only weights the step towards the goal walks straight
    /// there: every rollout follows it, so only positions 0 to 5 are expanded.
    #[test]
    fn a_greedy_policy_walks_straight_to_the_goal() {
        let line = Line { len: 6 };
        let towards_goal = |_: &u32, succ: &[(i8, u32)], out: &mut Vec<f64>| {
            out.extend(succ.iter().map(|&(a, _)| if a > 0 { 1.0 } else { 0.0 }));
        };
        for seed in 0..20 {
            let r = pi_iw(&line, towards_goal, seed);
            assert_eq!(r.plan.as_deref(), Some(&[1; 6][..]), "seed {seed}");
            assert_eq!(r.expanded, 6, "seed {seed}");
        }
    }

    /// A policy that weights only the step away from the goal still reaches
    /// it: once the weighted children are solved, the weight-0 ones are tried.
    #[test]
    fn zero_weight_children_are_tried_last() {
        let line = Line { len: 6 };
        let away = |_: &u32, succ: &[(i8, u32)], out: &mut Vec<f64>| {
            out.extend(succ.iter().map(|&(a, _)| if a < 0 { 1.0 } else { 0.0 }));
        };
        for seed in 0..20 {
            let plan = pi_iw(&line, away, seed).plan.expect("width-1 goal");
            assert_eq!(plan, [1; 6], "seed {seed}");
        }
    }

    /// A negative weight breaks the policy contract.
    #[test]
    #[should_panic(expected = "finite and non-negative")]
    fn rejects_negative_weights() {
        let negative = |_: &u32, succ: &[(i8, u32)], out: &mut Vec<f64>| {
            out.extend(succ.iter().map(|_| -1.0));
        };
        pi_iw(&Line { len: 3 }, negative, 0);
    }

    /// Weights that are finite one by one but overflow when summed break the
    /// policy contract: position 1 has two successors of weight `f64::MAX`.
    #[test]
    #[should_panic(expected = "finite sum")]
    fn rejects_weights_with_an_infinite_sum() {
        let huge = |_: &u32, succ: &[(i8, u32)], out: &mut Vec<f64>| {
            out.extend(succ.iter().map(|_| f64::MAX));
        };
        pi_iw(&Line { len: 3 }, huge, 0);
    }

    /// A policy must push exactly one weight per successor.
    #[test]
    #[should_panic(expected = "one weight per successor")]
    fn rejects_a_missing_weight() {
        pi_iw(
            &Line { len: 3 },
            |_: &u32, _: &[(i8, u32)], _: &mut Vec<f64>| {},
            0,
        );
    }

    /// Without a goal the search ends once the root is solved.
    #[test]
    fn gives_up_when_the_root_is_solved() {
        let r = rollout_iw(&NoGoal(Line { len: 6 }), 3);
        assert!(r.plan.is_none());
        assert_eq!(r.expanded, 7, "every position expanded once");
    }

    /// A [`Line`] whose goal is never reached.
    struct NoGoal(Line);

    impl WidthProblem for NoGoal {
        type State = u32;
        type Action = i8;

        fn initial_state(&self) -> u32 {
            self.0.initial_state()
        }
        fn successors(&self, s: &u32, out: &mut Vec<(i8, u32)>) {
            self.0.successors(s, out);
        }
        fn features(&self, s: &u32, out: &mut Vec<u32>) {
            self.0.features(s, out);
        }
        fn num_features(&self) -> usize {
            self.0.num_features()
        }
        fn is_goal(&self, _: &u32) -> bool {
            false
        }
    }
}
