use std::cell::RefCell;
use std::collections::HashMap;

use crate::rng::SplitMix64;
use crate::rollout::{Outcome, Tree};
use crate::{SearchResult, WidthProblem, extract_plan};

/// π-HIW(1, 1) (Junyent, Jonsson & Gómez, ICAPS 2021), goal-directed:
/// hierarchical IW with count-based Rollout IW at the high level and π-IW at
/// the low level.
///
/// `high` gives the high-level feature ids (`< num_high`) of a state, as in
/// [`hiw`](super::hiw). Each high-level node owns a resumable π-IW tree,
/// guided by `policy` as in [`pi_iw`](crate::pi_iw) and confined to the
/// node's high-level features: a rollout that enters a state with other
/// high-level features stops there and hands that state up as a high-level
/// successor. The high level runs the paper's count-based Rollout IW
/// (Algorithm 3):
///
/// - **select** an open high-level node with probability proportional to
///   `exp(1 / (tau · (c + 1)))`, where `c` counts how often a node with the
///   same high-level features was rolled out from, so rarely visited
///   high-level states are preferred;
/// - **roll out** from it: ask the node for its next successor; a node whose
///   tree is exhausted leaves the open list. A successor is kept if it
///   reaches some high-level feature at a smaller high-level depth than
///   before; it then takes over those features, and an earlier node left
///   with no such feature is removed from the open list with its
///   descendants. The rollout continues from the kept successor, and ends
///   at a pruned one.
///
/// The search returns at the first goal generated and gives up
/// (`plan: None`) when no high-level node is open. The paper's agent
/// maximizes reward within a node budget, learns the policy and a value
/// function, and backpropagates rewards and counts; this version has none of
/// that, and uses width 1 at both levels. Small `tau` makes selection favor
/// rare high-level states strongly; the paper uses 0.005. `seed` fixes all
/// random choices.
///
/// # Panics
///
/// Panics if `tau` is not finite and positive, if `high` pushes an id not
/// below `num_high`, or if `policy` breaks the contract of
/// [`pi_iw`](crate::pi_iw).
pub fn pi_hiw<P, H, F>(
    problem: &P,
    high: H,
    num_high: usize,
    policy: F,
    tau: f64,
    seed: u64,
) -> SearchResult<P::Action>
where
    P: WidthProblem,
    H: Fn(&P::State, &mut Vec<u32>),
    F: FnMut(&P::State, &[(P::Action, P::State)], &mut Vec<f64>),
{
    assert!(
        tau.is_finite() && tau > 0.0,
        "tau must be finite and positive"
    );
    let root = problem.initial_state();
    if problem.is_goal(&root) {
        return SearchResult {
            plan: Some(Vec::new()),
            expanded: 0,
            generated: 1,
        };
    }

    let policy = RefCell::new(policy);
    let shared = |s: &P::State, succ: &[(P::Action, P::State)], out: &mut Vec<f64>| {
        (policy.borrow_mut())(s, succ, out);
    };
    let mut search = HighSearch {
        problem,
        high: &high,
        policy: shared,
        tau,
        rng: SplitMix64::new(seed),
        nodes: Vec::new(),
        open: Vec::new(),
        counts: HashMap::new(),
        depth_of: vec![usize::MAX; num_high],
        owner: vec![None; num_high],
    };
    search.add_node(root, None, 0);

    let goal = search.run();
    let mut result = search.totals();
    result.plan = goal.map(|(h, j)| search.plan(h, j));
    result
}

/// One high-level node of π-HIW.
struct HighNode<'p, P: WidthProblem, F> {
    /// The node's low-level π-IW tree, confined to its high-level features.
    tree: Tree<'p, P, F>,
    /// The node's sorted high-level features.
    features: Vec<u32>,
    /// The parent high-level node and the index, in its tree, of the
    /// boundary node this node starts from; `None` for the root.
    origin: Option<(usize, usize)>,
    /// Number of high-level steps from the root.
    depth: usize,
    /// High-level features this node still reaches first (`T` in the paper).
    novel: Vec<u32>,
    /// High-level nodes started from this one.
    children: Vec<usize>,
    /// Whether the node is in the open list.
    open: bool,
}

/// The high level of one π-HIW run.
struct HighSearch<'p, P: WidthProblem, F> {
    /// The problem being searched.
    problem: &'p P,
    /// Computes the high-level features of a state.
    high: &'p dyn Fn(&P::State, &mut Vec<u32>),
    /// The policy, shared by every low-level tree.
    policy: F,
    /// Selection temperature.
    tau: f64,
    /// Random source for selection and the trees' seeds.
    rng: SplitMix64,
    /// All high-level nodes, root first.
    nodes: Vec<HighNode<'p, P, F>>,
    /// Indices of the open high-level nodes.
    open: Vec<usize>,
    /// Rollouts started from each set of high-level features.
    counts: HashMap<Vec<u32>, usize>,
    /// Smallest high-level depth at which each high-level feature was reached.
    depth_of: Vec<usize>,
    /// The node that reached each high-level feature at that depth.
    owner: Vec<Option<usize>>,
}

impl<P, F> HighSearch<'_, P, F>
where
    P: WidthProblem,
    F: FnMut(&P::State, &[(P::Action, P::State)], &mut Vec<f64>) + Copy,
{
    /// Runs rollouts from selected open nodes until a goal is generated
    /// (returned as high-level node and index in its tree) or none is open.
    fn run(&mut self) -> Option<(usize, usize)> {
        while !self.open.is_empty() {
            let mut n = self.select();
            loop {
                let features = self.nodes[n].features.clone();
                *self.counts.entry(features).or_default() += 1;
                match self.successor(n) {
                    Step::Goal(j) => return Some((n, j)),
                    Step::Exhausted => {
                        self.close(n);
                        break;
                    }
                    Step::Boundary(j) => {
                        let Some(s) = self.admit(n, j) else {
                            break;
                        };
                        n = s;
                    }
                }
            }
        }

        None
    }

    /// An open node drawn with probability proportional to
    /// `exp(1 / (tau · (c + 1)))`, computed relative to the largest exponent
    /// so that small `tau` cannot overflow.
    fn select(&mut self) -> usize {
        let exponent = |n: usize| {
            let c = self
                .counts
                .get(&self.nodes[n].features)
                .copied()
                .unwrap_or(0);
            // shortcut: counts beyond 2^53 lose precision here, far past any run.
            #[expect(clippy::cast_precision_loss, reason = "counts stay far below 2^53")]
            let c = c as f64;
            1.0 / (self.tau * (c + 1.0))
        };
        let max = self
            .open
            .iter()
            .map(|&n| exponent(n))
            .fold(f64::MIN, f64::max);
        let weights: Vec<f64> = self
            .open
            .iter()
            .map(|&n| (exponent(n) - max).exp())
            .collect();

        let mut target = self.rng.unit() * weights.iter().sum::<f64>();
        for (&n, w) in self.open.iter().zip(&weights) {
            target -= w;
            if target < 0.0 {
                return n;
            }
        }

        *self
            .open
            .last()
            .expect("select is only called with open nodes")
    }

    /// The high-level successor function: runs rollouts in node `n`'s tree
    /// until one reaches a goal or a boundary, or the tree is exhausted.
    fn successor(&mut self, n: usize) -> Step {
        let tree = &mut self.nodes[n].tree;
        while !tree.is_exhausted() {
            match tree.rollout() {
                Outcome::Goal(j) => return Step::Goal(j),
                Outcome::Boundary(j) => return Step::Boundary(j),
                Outcome::Solved => {}
            }
        }

        Step::Exhausted
    }

    /// Decides on the boundary node `j` of node `n`'s tree: if it reaches
    /// some high-level feature at a smaller depth than before, it becomes a
    /// new open node (returned) that takes over those features.
    fn admit(&mut self, n: usize, j: usize) -> Option<usize> {
        let state = self.nodes[n].tree.nodes[j].state.clone();
        let features = self.high_features(&state);
        let depth = self.nodes[n].depth + 1;
        let novel: Vec<u32> = features
            .iter()
            .copied()
            .filter(|&f| depth < self.depth_of[f as usize])
            .collect();
        if novel.is_empty() {
            return None;
        }

        let s = self.add_node(state, Some((n, j)), depth);
        self.nodes[n].children.push(s);
        for &f in &novel {
            self.take_over(f, s, depth);
        }

        self.nodes[s].novel = novel;
        Some(s)
    }

    /// Makes node `s` the owner of high-level feature `f` at `depth`; the
    /// previous owner loses `f`, and leaves the open list with its
    /// descendants if it has no feature left (`PruneOther`).
    fn take_over(&mut self, f: u32, s: usize, depth: usize) {
        let f_index = f as usize;
        self.depth_of[f_index] = depth;
        if let Some(o) = self.owner[f_index].replace(s) {
            self.nodes[o].novel.retain(|&g| g != f);
            if self.nodes[o].novel.is_empty() && o != 0 {
                self.close_subtree(o);
            }
        }
    }

    /// Adds a high-level node for `state`, open, and returns its index.
    fn add_node(&mut self, state: P::State, origin: Option<(usize, usize)>, depth: usize) -> usize {
        let features = self.high_features(&state);
        let seed = self.rng.next_u64();
        let mut tree = Tree::new(
            self.problem,
            Some(self.policy),
            seed,
            Some((self.high, features.clone())),
        );
        tree.add_root(state);
        if origin.is_none() {
            for &f in &features {
                self.depth_of[f as usize] = 0;
                self.owner[f as usize] = Some(0);
            }
        }

        self.nodes.push(HighNode {
            tree,
            novel: if origin.is_none() {
                features.clone()
            } else {
                Vec::new()
            },
            features,
            origin,
            depth,
            children: Vec::new(),
            open: true,
        });
        let index = self.nodes.len() - 1;
        self.open.push(index);
        index
    }

    /// Removes node `n` from the open list.
    fn close(&mut self, n: usize) {
        if std::mem::replace(&mut self.nodes[n].open, false) {
            self.open.retain(|&o| o != n);
        }
    }

    /// Removes node `n` and all of its descendants from the open list.
    fn close_subtree(&mut self, n: usize) {
        let mut stack = vec![n];
        while let Some(m) = stack.pop() {
            self.close(m);
            stack.extend(self.nodes[m].children.iter().copied());
        }
    }

    /// The sorted, deduplicated high-level features of `state`.
    fn high_features(&self, state: &P::State) -> Vec<u32> {
        let mut out = Vec::new();
        (self.high)(state, &mut out);
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The plan to node `j` of high-level node `h`'s tree: the paths through
    /// the trees of `h`'s ancestors, then through `h`'s own.
    fn plan(&self, mut h: usize, j: usize) -> Vec<P::Action> {
        let mut pieces = vec![extract_plan(&self.nodes[h].tree.nodes, j)];
        while let Some((parent, boundary)) = self.nodes[h].origin {
            pieces.push(extract_plan(&self.nodes[parent].tree.nodes, boundary));
            h = parent;
        }

        pieces.into_iter().rev().flatten().collect()
    }

    /// Node counts over all trees. A tree's root was already generated in
    /// its parent's tree, so it counts only for the first tree.
    fn totals(&self) -> SearchResult<P::Action> {
        let mut total = SearchResult {
            plan: None,
            expanded: 0,
            generated: 1,
        };
        for node in &self.nodes {
            total.expanded += node.tree.result.expanded;
            total.generated += node.tree.result.generated - 1;
        }

        total
    }
}

/// What a high-level successor call found.
enum Step {
    /// A goal, at this index in the node's tree.
    Goal(usize),
    /// A boundary state, at this index in the node's tree.
    Boundary(usize),
    /// Nothing: the node's tree is exhausted.
    Exhausted,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hiw::tests::Corridor;

    /// Equal weights for every successor.
    fn uniform<S, A>(_: &S, succ: &[(A, S)], out: &mut Vec<f64>) {
        out.extend(succ.iter().map(|_| 1.0));
    }

    /// The key flag as the one high-level feature.
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "matches the high-level feature closure signature"
    )]
    fn key(&(_, key): &(u32, bool), out: &mut Vec<u32>) {
        if key {
            out.push(0);
        }
    }

    /// With the key as high-level feature, π-HIW reaches the goal of the
    /// corridor, which Rollout IW(1) over positions cannot, for every seed.
    #[test]
    fn the_key_splits_the_corridor() {
        let corridor = Corridor::new(5);
        for seed in 0..20 {
            let r = pi_hiw(&corridor, key, 1, uniform, 0.005, seed);
            assert_eq!(
                r.plan.as_deref(),
                Some(&[1, 1, 1, 1, 1, -1, -1, -1, -1, -1][..]),
                "seed {seed}"
            );
        }
    }

    /// Without high-level features there is one high-level node, and the
    /// search ends when its tree is exhausted.
    #[test]
    fn without_high_level_features_it_terminates() {
        let none = |_: &(u32, bool), _: &mut Vec<u32>| {};
        let r = pi_hiw(&Corridor::new(5), none, 0, uniform, 1.0, 3);

        assert!(r.plan.is_none());
    }

    /// A temperature of 0 is rejected.
    #[test]
    #[should_panic(expected = "tau must be finite and positive")]
    fn rejects_zero_tau() {
        pi_hiw(&Corridor::new(2), key, 1, uniform, 0.0, 0);
    }
}
