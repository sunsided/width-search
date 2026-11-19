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
    let mut tree = Tree::new(problem, seed);
    let root = problem.initial_state();
    if problem.is_goal(&root) {
        tree.result.plan = Some(Vec::new());
        return tree.result;
    }

    tree.add_root(root);
    while !tree.meta[0].solved {
        if let Some(goal) = tree.rollout() {
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
}

/// What the novelty test says about the node a rollout just entered.
enum Verdict {
    /// The node is novel: continue the rollout below it.
    Continue,
    /// The node is not novel: label it solved and end the rollout.
    Prune,
}

/// The search tree of a Rollout IW(1) run and its novelty table.
struct Tree<'p, P: WidthProblem> {
    /// The problem being searched.
    problem: &'p P,
    /// Node arena: states and parent links.
    nodes: Vec<Node<P::State, P::Action>>,
    /// Rollout data of each node, same indices as `nodes`.
    meta: Vec<Meta>,
    /// Smallest depth at which each feature was reached; `usize::MAX` if never.
    depth_of: Vec<usize>,
    /// Pseudo-random state for child selection (`SplitMix64`).
    rng: u64,
    /// Reused buffer for the features of a state.
    feats: Vec<u32>,
    /// Reused buffer for successors.
    succ: Vec<(P::Action, P::State)>,
    /// Node counts so far; `plan` is filled in by [`rollout_iw`].
    result: SearchResult<P::Action>,
}

impl<'p, P: WidthProblem> Tree<'p, P> {
    /// An empty tree for `problem`.
    fn new(problem: &'p P, seed: u64) -> Self {
        Self {
            problem,
            nodes: Vec::new(),
            meta: Vec::new(),
            depth_of: vec![usize::MAX; problem.num_features()],
            rng: seed,
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
    fn add_root(&mut self, root: P::State) {
        self.feats.clear();
        self.problem.features(&root, &mut self.feats);
        for &f in &self.feats {
            self.depth_of[f as usize] = 0;
        }

        self.push(root, None, None, 0);
        self.meta[0].visited = true;
    }

    /// One rollout from the root; returns the index of a goal node if one
    /// was generated.
    fn rollout(&mut self) -> Option<usize> {
        let mut n = 0;
        loop {
            if self.meta[n].children.is_none() {
                if let Some(goal) = self.expand(n) {
                    return Some(goal);
                }
            }

            let Some(child) = self.pick_unsolved_child(n) else {
                // No unsolved children: nothing left to explore below n.
                self.solve(n);
                return None;
            };

            n = child;
            if let Verdict::Prune = self.novelty(n) {
                self.solve(n);
                return None;
            }
        }
    }

    /// Generates all children of `n`; returns the first goal child, if any.
    fn expand(&mut self, n: usize) -> Option<usize> {
        self.result.expanded += 1;
        let mut succ = std::mem::take(&mut self.succ);
        succ.clear();
        self.problem.successors(&self.nodes[n].state, &mut succ);

        let depth = self.meta[n].depth + 1;
        let mut children = Vec::with_capacity(succ.len());
        let mut goal = None;
        for (action, state) in succ.drain(..) {
            self.result.generated += 1;
            let is_goal = self.problem.is_goal(&state);
            let child = self.push(state, Some(n), Some(action), depth);
            children.push(child);
            if is_goal {
                goal.get_or_insert(child);
            }
        }

        self.succ = succ;
        self.meta[n].children = Some(children);
        goal
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

    /// A uniformly random child of `n` not labeled solved, if any.
    fn pick_unsolved_child(&mut self, n: usize) -> Option<usize> {
        let count_unsolved = |t: &Self| {
            let children = t.meta[n].children.as_deref().unwrap_or_default();
            children.iter().filter(|&&c| !t.meta[c].solved).count()
        };
        let unsolved = count_unsolved(self);
        if unsolved == 0 {
            return None;
        }

        // shortcut: modulo bias is negligible for branching factors far below 2^64.
        let pick = usize::try_from(self.next_random() % unsolved as u64).expect("below a usize");
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
        });

        self.nodes.len() - 1
    }

    /// The next `SplitMix64` output.
    fn next_random(&mut self) -> u64 {
        self.rng = self.rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Walking from 0 to `len` on a line, one step either way.
    struct Line {
        /// The goal position, also the largest one.
        len: u32,
    }

    impl WidthProblem for Line {
        type State = u32;
        type Action = i8;

        fn initial_state(&self) -> u32 {
            0
        }
        fn successors(&self, s: &u32, out: &mut Vec<(i8, u32)>) {
            if *s > 0 {
                out.push((-1, s - 1));
            }
            if *s < self.len {
                out.push((1, s + 1));
            }
        }
        fn features(&self, s: &u32, out: &mut Vec<u32>) {
            out.push(*s);
        }
        fn num_features(&self) -> usize {
            self.len as usize + 1
        }
        fn is_goal(&self, s: &u32) -> bool {
            *s == self.len
        }
    }

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
